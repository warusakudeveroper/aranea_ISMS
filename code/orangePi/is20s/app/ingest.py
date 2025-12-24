from datetime import datetime, timezone
from typing import Dict, Any

from araneapi.sqlite_store import EventStore


def _normalize_event(payload: Dict[str, Any]) -> Dict[str, Any]:
    """
    ESP32系（is02a→is03リレー）のペイロード互換でイベントを組み立てる。
    """
    sensor = payload.get("sensor", {})
    gateway = payload.get("gateway", {})
    state = payload.get("state", {})
    raw = payload.get("raw", {})

    seen_at = (
        payload.get("observedAt")
        or payload.get("seenAt")
        or payload.get("meta", {}).get("observedAt")
        or datetime.now(timezone.utc).isoformat()
    )

    event = {
        "seenAt": seen_at,
        "src": payload.get("src") or "ingest",
        "mac": sensor.get("mac"),
        "rssi": payload.get("rssi") or gateway.get("rssi"),
        "adv_hex": raw.get("advHex") or raw.get("adv_hex"),
        "manufacturer_hex": raw.get("mfgHex") or raw.get("manufacturer_hex"),
        "lacisId": sensor.get("lacisId") or payload.get("lacisId"),
        "productType": sensor.get("productType"),
        "productCode": sensor.get("productCode"),
        "gatewayLacisId": gateway.get("lacisId"),
        "gatewayIp": gateway.get("ip"),
        "temperatureC": state.get("temperatureC") or state.get("temperature"),
        "humidityPct": state.get("humidityPct") or state.get("humidity"),
        "batteryPct": state.get("batteryPct") or state.get("battery"),
        "payload": payload,
    }
    return event


async def handle_ingest(payload: Dict[str, Any], store: EventStore, forwarder: "Forwarder") -> Dict[str, Any]:
    """
    ローカルリレー用の受信処理。
    - RingBuffer/SQLite に格納（ESP32 is03互換フォーマット）
    - Forwarder キューに投入（元ペイロードで送信）
    """
    event = _normalize_event(payload)
    store.enqueue(event)
    await forwarder.enqueue(event)
    return {"ok": True}


# Forwarder を type hint するための遅延インポート用
from .forwarder import Forwarder  # noqa: E402  # isort:skip

__all__ = ["handle_ingest"]
