import asyncio
import copy
import gc
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

from .config import load_is20s_config
from .state import RuntimeState
from .forwarder import Forwarder
from .mqtt_handlers import build_mqtt_handlers
from .api import create_router
from .capture import CaptureManager, load_watch_config, save_watch_config
from .hardware_info import get_memory_info
from . import ui

# メモリ管理設定
MEMORY_CHECK_INTERVAL_SEC = 30  # メモリチェック間隔
MEMORY_THRESHOLD_PERCENT = 80  # GCトリガー閾値

logger = logging.getLogger(__name__)

cfg = load_is20s_config()
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
mqtt_client: Optional[MqttClient] = None
if cfg.mqtt.host:
    mqtt_client = MqttClient(cfg.mqtt)


register_cache_path = Path(cfg.device.data_dir) / "register.json"
config_override_path = Path(cfg.device.data_dir) / "config_override.yaml"
log_path = Path(cfg.device.log_dir) / "is20s.log"
watch_config_path = Path(cfg.device.data_dir) / "watch_config.json"

# tsharkキャプチャマネージャ（コア機能）
capture_manager = CaptureManager(str(watch_config_path))


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


config_lock = asyncio.Lock()
heartbeat_task: Optional[PeriodicTask] = None
memory_watchdog_task: Optional[asyncio.Task] = None


async def _memory_watchdog(stop_event: asyncio.Event) -> None:
    """
    メモリ監視タスク: 80%超過時にGCとキャッシュクリアを実行
    """
    gc_count = 0
    while not stop_event.is_set():
        try:
            await asyncio.wait_for(stop_event.wait(), timeout=MEMORY_CHECK_INTERVAL_SEC)
        except asyncio.TimeoutError:
            pass

        if stop_event.is_set():
            break

        try:
            mem = get_memory_info()
            if mem.usage_percent >= MEMORY_THRESHOLD_PERCENT:
                gc_count += 1
                logger.warning(
                    "Memory usage %.1f%% >= %.1f%% threshold, running GC (#%d)",
                    mem.usage_percent, MEMORY_THRESHOLD_PERCENT, gc_count
                )

                # 1. DNSキャッシュの古いエントリを削減（50%削減）
                if capture_manager.dns_cache:
                    cache_size = len(capture_manager.dns_cache.cache)
                    if cache_size > 10000:
                        # 古い半分を削除
                        to_remove = cache_size // 2
                        for _ in range(to_remove):
                            if capture_manager.dns_cache.cache:
                                capture_manager.dns_cache.cache.popitem(last=False)
                        logger.info("DNS cache reduced: %d -> %d entries", cache_size, len(capture_manager.dns_cache.cache))

                # 2. ASNキャッシュの古いエントリを削減
                if capture_manager.asn_lookup:
                    asn_cache_size = len(capture_manager.asn_lookup.cache)
                    if asn_cache_size > 10000:
                        to_remove = asn_cache_size // 2
                        for _ in range(to_remove):
                            if capture_manager.asn_lookup.cache:
                                capture_manager.asn_lookup.cache.popitem(last=False)
                        logger.info("ASN cache reduced: %d -> %d entries", asn_cache_size, len(capture_manager.asn_lookup.cache))

                # 3. PTR pending setの古いエントリを安全にクリア
                # 注意: 完全クリアは処理中のIPに影響するため、キューのみクリア
                if capture_manager._ptr_queue:
                    cleared = 0
                    while not capture_manager._ptr_queue.empty():
                        try:
                            ip = capture_manager._ptr_queue.get_nowait()
                            capture_manager._ptr_pending.discard(ip)
                            cleared += 1
                        except asyncio.QueueEmpty:
                            break
                    if cleared > 0:
                        logger.info("PTR queue cleared: %d pending IPs removed", cleared)

                # 4. 強制GC実行
                collected = gc.collect()
                logger.info("GC collected %d objects", collected)

                # 5. 結果確認
                mem_after = get_memory_info()
                logger.info(
                    "Memory after GC: %.1f%% (was %.1f%%, freed %dMB)",
                    mem_after.usage_percent, mem.usage_percent,
                    mem.used_mb - mem_after.used_mb
                )
        except Exception as e:
            logger.error("Memory watchdog error: %s", e)


async def _restart_mqtt_if_needed(prev_mqtt_cfg, new_mqtt_cfg) -> None:
    global mqtt_client
    need_restart = prev_mqtt_cfg != new_mqtt_cfg
    if not need_restart:
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
        old_data_dir = cfg.device.data_dir
        old_log_dir = cfg.device.log_dir
        old_register_cache = register_cache_path
        prev_mqtt_cfg = copy.deepcopy(cfg.mqtt)
        update_config_from_dict(cfg, updates)

        # data_dir/log_dir 更新時は関連パスを差し替え
        register_cache_path = Path(cfg.device.data_dir) / "register.json"
        config_override_path = Path(cfg.device.data_dir) / "config_override.yaml"
        log_path = Path(cfg.device.log_dir) / "is20s.log"

        save_config(cfg, config_override_path)

        # register.json を旧パスから新パスへマイグレーション
        try:
            if old_register_cache != register_cache_path and old_register_cache.exists() and not register_cache_path.exists():
                register_cache_path.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(old_register_cache, register_cache_path)
        except Exception as exc:  # noqa: BLE001
            logger.warning("register cache migration failed: %s", exc)

        # Forwarder設定更新
        await forwarder.update_config(cfg.relay)

        # Store設定更新（リングサイズ変更は既存を移行）
        if "store" in updates:
            store_cfg = updates["store"]
            await store.update_settings(
                ring_size=store_cfg.get("ring_size"),
                max_db_size=store_cfg.get("max_db_size"),
                flush_batch=store_cfg.get("flush_batch"),
                flush_interval_sec=store_cfg.get("flush_interval_sec"),
            )
        # Flushタスクが止まっていた場合に再開
        if not store._flush_task or store._flush_task.done():
            await store.start_flush_task()

        # MQTT更新
        await _restart_mqtt_if_needed(prev_mqtt_cfg, cfg.mqtt)

        requires_restart = []
        if "store" in updates or ("device" in updates and ("data_dir" in updates["device"] or "log_dir" in updates["device"])):
            requires_restart.append("store/data_dir/log_dir")
        return {
            "persisted": str(config_override_path),
            "requiresRestart": requires_restart,
            "config": asdict(cfg),
        }


@asynccontextmanager
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


async def _start_heartbeat():
    global heartbeat_task
    if heartbeat_task:
        await heartbeat_task.stop()
    heartbeat_task = PeriodicTask(60, _publish_status)
    await heartbeat_task.start()


async def lifespan(app: FastAPI):
    global memory_watchdog_task

    await store.init_db()
    await store.start_flush_task(cfg.store.flush_interval_sec)
    await forwarder.start()
    await _register_once()

    watchdog_stop = asyncio.Event()
    watchdog_task = asyncio.create_task(_mqtt_watchdog(watchdog_stop))

    # メモリ監視タスク起動
    memory_stop = asyncio.Event()
    memory_watchdog_task = asyncio.create_task(_memory_watchdog(memory_stop))
    logger.info("Memory watchdog started (threshold: %d%%)", MEMORY_THRESHOLD_PERCENT)

    if mqtt_client:
        handlers = build_mqtt_handlers(cfg, state.lacis_id, state, forwarder, mqtt_client, apply_config_updates)
        for topic, handler in handlers.items():
            mqtt_client.add_handler(topic, handler)
        await mqtt_client.start()
        await _start_heartbeat()

    # tsharkキャプチャ起動（enabled=trueの場合）
    capture_warnings = await capture_manager.check_prerequisites()
    for w in capture_warnings:
        logger.warning(w)
    if capture_manager.cfg.get("mode", {}).get("enabled", False):
        await capture_manager.start()

    try:
        yield
    finally:
        # graceful shutdown: 残りのキャプチャデータを送信
        if capture_manager.running:
            await capture_manager.flush_remaining()
            await capture_manager.stop()

        # メモリ監視停止
        memory_stop.set()
        if memory_watchdog_task:
            await memory_watchdog_task

        watchdog_stop.set()
        await watchdog_task
        if heartbeat_task:
            await heartbeat_task.stop()
        if mqtt_client:
            await mqtt_client.stop()
        await forwarder.stop()
        await store.stop_flush_task()


app = FastAPI(lifespan=lifespan)
app.include_router(create_router(
    store, forwarder, state, cfg, apply_config_updates, lambda: log_path,
    capture_manager=capture_manager,
    watch_config_path=str(watch_config_path),
))
app.include_router(ui.create_router(cfg.access.allowed_sources))


@app.get("/health")
async def health():
    return {"ok": True}


__all__ = ["app"]
