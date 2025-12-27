from pathlib import Path
import re
import uuid
from typing import Iterable, Optional


def _read_mac_from_sys(interface: str) -> Optional[str]:
    path = Path(f"/sys/class/net/{interface}/address")
    if not path.exists():
        return None
    try:
        raw = path.read_text().strip()
        if raw:
            return raw
    except Exception:
        return None
    return None


def _normalize_mac(mac: str) -> Optional[str]:
    if not mac:
        return None
    mac = mac.strip().upper().replace(":", "")
    return mac if re.fullmatch(r"[0-9A-F]{12}", mac) else None


def _get_mac_preferred(interfaces: Iterable[str]) -> str:
    for iface in interfaces:
        mac = _normalize_mac(_read_mac_from_sys(iface) or "")
        if mac:
            return mac
    # fallback: uuid.getnode()
    mac_int = uuid.getnode()
    mac = f"{mac_int:012X}"
    return mac


def generate_lacis_id(product_type: str, product_code: str, interfaces: Iterable[str]) -> str:
    """
    LacisID = 3 + productType(3桁) + MAC12HEX + productCode(4桁)
    """
    mac12 = _get_mac_preferred(interfaces)
    return f"3{product_type}{mac12}{product_code}"


def get_mac12(interfaces: Iterable[str]) -> str:
    """優先インターフェースからMAC12桁HEXを取得（小文字も大文字化）。"""
    return _get_mac_preferred(interfaces)


__all__ = ["generate_lacis_id", "get_mac12"]
