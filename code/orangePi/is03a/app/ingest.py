from datetime import datetime, timezone
from typing import Dict, Any

from araneapi.sqlite_store import EventStore


def _normalize(payload: Dict[str, Any]) -> Dict[str, Any]:
    sensor = payload.get("sensor", {})
    gateway = payload.get("gateway", {})
    state = payload.get("state", {})
    raw = payload.get("raw", {})
    meta = payload.get("meta", {})

    seen_at = (
        payload.get("observedAt")
        or payload.get("seenAt")
        or meta.get("observedAt")
        or datetime.now(timezone.utc).isoformat()
    )

    return {
        "seenAt": seen_at,
        "src": payload.get("src") or payload.get("source") or "ingest",
        "mac": sensor.get("mac"),
        "rssi": payload.get("rssi") or gateway.get("rssi") or meta.get("rssi"),
        "adv_hex": raw.get("advHex") or raw.get("adv_hex"),
        "manufacturer_hex": raw.get("mfgHex") or raw.get("manufacturer_hex"),
        "lacisId": sensor.get("lacisId") or payload.get("lacisId"),
        "productType": sensor.get("productType"),
        "productCode": sensor.get("productCode"),
        "gatewayLacisId": gateway.get("lacisId") or meta.get("gatewayId"),
        "gatewayIp": gateway.get("ip"),
        "temperatureC": state.get("temperatureC") or state.get("temperature"),
        "humidityPct": state.get("humidityPct") or state.get("humidity"),
        "batteryPct": state.get("batteryPct") or state.get("battery"),
        "payload": payload,
    }


async def handle_ingest(payload: Dict[str, Any], store: EventStore, forwarder: "Forwarder", event_hub: "EventHub") -> Dict[str, Any]:
    event = _normalize(payload)
    store.enqueue(event)
    await forwarder.enqueue(event)
    await event_hub.broadcast(event)
    return {"ok": True}


# late imports for typing
from .forwarder import Forwarder  # noqa: E402  # isort:skip
from .event_hub import EventHub  # noqa: E402  # isort:skip

__all__ = ["handle_ingest"]
