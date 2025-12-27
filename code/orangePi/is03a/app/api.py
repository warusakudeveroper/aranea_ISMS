from dataclasses import asdict
from pathlib import Path
from typing import Any, Dict, Callable
import asyncio
import json

from fastapi import APIRouter, HTTPException, Request
from fastapi.responses import StreamingResponse

from araneapi.sqlite_store import EventStore
from araneapi.system_info import system_status
from araneapi.config import AppConfig

from .ingest import handle_ingest
from .forwarder import Forwarder
from .state import RuntimeState
from .event_hub import EventHub


def _tail_log(log_path: Path, lines: int):
    if not log_path.exists():
        return []
    content = log_path.read_text(errors="ignore").splitlines()
    return content[-lines:]


def create_router(
    store: EventStore,
    forwarder: Forwarder,
    state: RuntimeState,
    cfg: AppConfig,
    apply_config_callback,
    log_path_getter: Callable[[], Path],
    event_hub: EventHub,
):
    router = APIRouter()

    @router.get("/api/status")
    async def get_status():
        sys = system_status()
        status = store.get_status()
        return {
            "ok": True,
            "hostname": sys.get("hostname"),
            "uptimeSec": int(sys.get("uptimeSec") or 0),
            "bluetooth": "ok",
            "db": "ok",
            **status,
            "device": {
                "name": cfg.device.name,
                "lacisId": state.lacis_id,
                "cic": state.cic,
                "productType": cfg.device.product_type,
                "productCode": cfg.device.product_code,
            },
            "forwarder": {
                "lastOkAt": state.last_forward_ok_at,
                "failures": state.forward_failures,
                "queued": forwarder.queue.qsize(),
            },
            "mqtt": {"connected": state.mqtt_connected},
        }

    @router.get("/api/events")
    async def get_events(limit: int = 200, request: Request = None):
        return store.get_recent(limit)

    @router.get("/api/events/stream")
    async def stream_events(request: Request):
        q = await event_hub.register()

        async def event_generator():
            try:
                while True:
                    event = await q.get()
                    yield f"data: {json.dumps(event, ensure_ascii=False)}\\n\\n"
            except asyncio.CancelledError:
                pass
            finally:
                await event_hub.unregister(q)

        return StreamingResponse(event_generator(), media_type="text/event-stream")

    @router.post("/api/ingest")
    async def post_ingest(payload: Dict[str, Any], request: Request):
        return await handle_ingest(payload, store, forwarder, event_hub)

    @router.post("/api/events")
    async def post_events(payload: Dict[str, Any], request: Request):
        """is02互換エンドポイント"""
        return await handle_ingest(payload, store, forwarder, event_hub)

    @router.get("/api/config")
    async def get_config(request: Request):
        return asdict(cfg)

    @router.post("/api/config")
    async def post_config(payload: Dict[str, Any], request: Request):
        apply_result = await apply_config_callback(payload)
        return {"ok": True, **apply_result}

    @router.get("/api/logs")
    async def get_logs(request: Request, lines: int = 200):
        lines = min(max(lines, 10), 2000)
        return {"lines": _tail_log(log_path_getter(), lines)}

    @router.post("/api/debug/publishSample")
    async def publish_sample():
        sample = {
            "seenAt": None,
            "src": "debug",
            "lacisId": state.lacis_id,
            "payload": {"debug": True},
        }
        store.enqueue(sample)
        await forwarder.enqueue(sample)
        await event_hub.broadcast(sample)
        return {"ok": True}

    return router


__all__ = ["create_router"]
