import json
import logging
from pathlib import Path
from typing import Dict, Optional

import httpx

from .config import RegisterConfig

logger = logging.getLogger(__name__)


def load_register_cache(path: Path) -> Optional[Dict[str, str]]:
    try:
        if path.exists():
            return json.loads(path.read_text())
    except Exception as exc:  # noqa: BLE001
        logger.warning("load_register_cache failed: %s", exc)
    return None


def save_register_cache(path: Path, data: Dict[str, str]) -> None:
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(data, ensure_ascii=False, indent=2))
    except Exception as exc:  # noqa: BLE001
        logger.warning("save_register_cache failed: %s", exc)


async def register_device(
    cfg: RegisterConfig,
    device_type: str,
    lacis_id: str,
    mac12: str,
    product_type: str,
    product_code: str,
    cache_path: Optional[Path] = None,
    force: bool = False,
) -> Optional[Dict[str, str]]:
    """
    AraneaDeviceGate に登録し CIC / stateEndpoint / mqttEndpoint を取得する。
    gate_url/tid が未設定、enabled=False の場合は何もしない。
    cache_path が指定され、force=False の場合はキャッシュを優先使用。
    """
    if cache_path and not force:
        cached = load_register_cache(cache_path)
        if cached and cached.get("cic"):
            logger.info("register_device: using cached registration")
            return cached

    if not cfg.enabled or not cfg.gate_url or not cfg.tid:
        logger.info("register_device: gate disabled or config incomplete")
        return None
    if not (cfg.tenant_lacisid and cfg.tenant_email and cfg.tenant_cic):
        logger.error("register_device: tenant credentials incomplete (lacisId/email/cic required)")
        return None

    payload = {
        "lacisOath": {
            "lacisId": cfg.tenant_lacisid,
            "userId": cfg.tenant_email,
            "cic": cfg.tenant_cic,
            "method": "register",
        },
        "userObject": {
            "lacisID": lacis_id,
            "tid": cfg.tid,
            "typeDomain": "araneaDevice",
            "type": device_type,
        },
        "deviceMeta": {
            "macAddress": mac12,
            "productType": product_type,
            "productCode": product_code,
        },
    }

    try:
        async with httpx.AsyncClient() as client:
            res = await client.post(cfg.gate_url, json=payload, timeout=15)
        if res.status_code // 100 != 2:
            logger.error("register_device: gate responded %s body=%s", res.status_code, res.text)
            return None
        data = res.json()
        if not data.get("ok", False):
            logger.error("register_device: ok=false body=%s", data)
            return None

        # ESP32のAraneaRegister実装互換: userObject.cic_code, stateEndpoint, mqttEndpoint
        cic = (
            data.get("userObject", {}).get("cic_code")
            or data.get("cic")
            or data.get("device", {}).get("cic")
        )
        state_ep = data.get("stateEndpoint") or data.get("state_ep")
        mqtt_ep = data.get("mqttEndpoint") or data.get("mqtt_ep")

        result = {
            "cic": cic or "",
            "stateEndpoint": state_ep or "",
            "mqttEndpoint": mqtt_ep or "",
        }
        logger.info("register_device: success cic=%s state=%s mqtt=%s", cic, state_ep, mqtt_ep)

        if cache_path and result.get("cic"):
            save_register_cache(cache_path, result)
        return result
    except (httpx.RequestError, json.JSONDecodeError) as exc:
        logger.error("register_device: request failed: %s", exc)
        return None


__all__ = ["register_device", "load_register_cache", "save_register_cache"]
