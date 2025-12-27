import asyncio
import copy
import logging
import shutil
from contextlib import asynccontextmanager
from dataclasses import asdict
from pathlib import Path
from typing import Optional, Dict, Any

from fastapi import FastAPI

from araneapi import setup_logging
from araneapi.lacis_id import generate_lacis_id, get_mac12
from araneapi.sqlite_store import EventStore
from araneapi.device_register import register_device
from araneapi.mqtt_client import MqttClient
from araneapi.config import save_config, update_config_from_dict
from araneapi.tasks import PeriodicTask
from araneapi.system_info import uptime_seconds

from .config import load_is03a_config
from .state import RuntimeState
from .forwarder import Forwarder
from .mqtt_handlers import build_mqtt_handlers
from .api import create_router
from .ui import create_router as create_ui_router
from .event_hub import EventHub
from .ingest import handle_ingest
from .ble_scan import ble_scan_task

logger = logging.getLogger(__name__)

cfg = load_is03a_config()
setup_logging(level="INFO", log_dir=cfg.device.log_dir)

state = RuntimeState()
interfaces = [cfg.device.interface, "end0", "eth0", "wlan0"]
state.mac12 = get_mac12(interfaces)
state.lacis_id = generate_lacis_id(cfg.device.product_type, cfg.device.product_code, interfaces)

store = EventStore(
    db_path=cfg.store.sqlite_path,
    ring_size=cfg.store.ring_size,
    max_db_size=cfg.store.max_db_size,
    flush_batch=cfg.store.flush_batch,
    flush_interval_sec=cfg.store.flush_interval_sec,
)
forwarder = Forwarder(cfg.relay, state)
event_hub = EventHub(queue_size=200)
mqtt_client: Optional[MqttClient] = None
if cfg.mqtt.host:
    mqtt_client = MqttClient(cfg.mqtt)

register_cache_path = Path(cfg.device.data_dir) / "register.json"
config_override_path = Path(cfg.device.data_dir) / "config_override.yaml"
log_path = Path(cfg.device.log_dir) / "is03a.log"

config_lock = asyncio.Lock()
heartbeat_task: Optional[PeriodicTask] = None
ble_task: Optional[asyncio.Task] = None


async def _register_once() -> None:
    if not cfg.register.enabled:
        return
    try:
        result = await register_device(
            cfg.register,
            device_type=cfg.device.device_type or cfg.device.name,
            lacis_id=state.lacis_id,
            mac12=state.mac12 or "",
            product_type=cfg.device.product_type,
            product_code=cfg.device.product_code,
            cache_path=register_cache_path,
            force=False,
        )
        if result:
            state.cic = result.get("cic") or state.cic
            state.state_endpoint = result.get("stateEndpoint") or state.state_endpoint
            state.mqtt_endpoint = result.get("mqttEndpoint") or state.mqtt_endpoint
    except Exception as exc:  # noqa: BLE001
        logger.warning("register failed: %s", exc)


async def _mqtt_watchdog(stop_event: asyncio.Event) -> None:
    while not stop_event.is_set():
        if mqtt_client:
            state.mqtt_connected = mqtt_client.connected
        try:
            await asyncio.wait_for(stop_event.wait(), timeout=2)
        except asyncio.TimeoutError:
            continue


async def _start_heartbeat():
    global heartbeat_task
    if heartbeat_task:
        await heartbeat_task.stop()
    heartbeat_task = PeriodicTask(60, _publish_status)
    await heartbeat_task.start()


async def _publish_status() -> None:
    if not mqtt_client or not mqtt_client.connected:
        return
    payload = {
        "status": "ok",
        "lacisId": state.lacis_id,
        "cic": state.cic,
        "uptimeSec": int(uptime_seconds() or 0),
        "queued": forwarder.queue.qsize(),
    }
    topic = f"{cfg.mqtt.base_topic}/{cfg.register.tid}/device/{state.lacis_id}/status"
    await mqtt_client.publish(topic, payload)


async def _restart_mqtt_if_needed(prev_mqtt_cfg, new_mqtt_cfg) -> None:
    global mqtt_client
    if prev_mqtt_cfg == new_mqtt_cfg:
        return
    if mqtt_client:
        await mqtt_client.stop()
        mqtt_client = None
    if new_mqtt_cfg.host:
        mqtt_client = MqttClient(new_mqtt_cfg)
        handlers = build_mqtt_handlers(cfg, state.lacis_id, state, forwarder, mqtt_client, apply_config_updates)
        for topic, handler in handlers.items():
            mqtt_client.add_handler(topic, handler)
        await mqtt_client.start()
        await _start_heartbeat()


async def apply_config_updates(updates: Dict[str, Any]) -> Dict[str, Any]:
    global config_override_path, register_cache_path, log_path
    async with config_lock:
        old_register_cache = register_cache_path
        prev_mqtt_cfg = copy.deepcopy(cfg.mqtt)
        update_config_from_dict(cfg, updates)

        register_cache_path = Path(cfg.device.data_dir) / "register.json"
        config_override_path = Path(cfg.device.data_dir) / "config_override.yaml"
        log_path = Path(cfg.device.log_dir) / "is03a.log"

        save_config(cfg, config_override_path)

        try:
            if old_register_cache != register_cache_path and old_register_cache.exists() and not register_cache_path.exists():
                register_cache_path.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(old_register_cache, register_cache_path)
        except Exception as exc:  # noqa: BLE001
            logger.warning("register cache migration failed: %s", exc)

        await forwarder.update_config(cfg.relay)

        if "store" in updates:
            st = updates["store"]
            await store.update_settings(
                ring_size=st.get("ring_size"),
                max_db_size=st.get("max_db_size"),
                flush_batch=st.get("flush_batch"),
                flush_interval_sec=st.get("flush_interval_sec"),
            )
        if not store._flush_task or store._flush_task.done():
            await store.start_flush_task()

        await _restart_mqtt_if_needed(prev_mqtt_cfg, cfg.mqtt)

        requires_restart = []
        if "store" in updates or ("device" in updates and ("data_dir" in updates["device"] or "log_dir" in updates["device"])):
            requires_restart.append("store/data_dir/log_dir")
        return {"persisted": str(config_override_path), "requiresRestart": requires_restart, "config": asdict(cfg)}


app = FastAPI()


@asynccontextmanager
async def lifespan(app: FastAPI):
    await store.init_db()
    await store.start_flush_task(cfg.store.flush_interval_sec)
    await forwarder.start()
    await _register_once()

    watchdog_stop = asyncio.Event()
    watchdog_task = asyncio.create_task(_mqtt_watchdog(watchdog_stop))

    if mqtt_client:
        handlers = build_mqtt_handlers(cfg, state.lacis_id, state, forwarder, mqtt_client, apply_config_updates)
        for topic, handler in handlers.items():
            mqtt_client.add_handler(topic, handler)
        await mqtt_client.start()
        await _start_heartbeat()

    global ble_task
    ble_task = asyncio.create_task(ble_scan_task(store, forwarder, event_hub, cfg.device.product_type, cfg.device.product_code))

    try:
        yield
    finally:
        watchdog_stop.set()
        await watchdog_task
        if heartbeat_task:
            await heartbeat_task.stop()
        if ble_task:
            ble_task.cancel()
            try:
                await ble_task
            except asyncio.CancelledError:
                pass
        if mqtt_client:
            await mqtt_client.stop()
        await forwarder.stop()
        await store.stop_flush_task()


app.router.lifespan_context = lifespan
app.include_router(create_router(store, forwarder, state, cfg, apply_config_updates, lambda: log_path, event_hub))
app.include_router(create_ui_router(cfg.access.allowed_sources))


@app.get("/health")
async def health():
    return {"ok": True}


__all__ = ["app"]
