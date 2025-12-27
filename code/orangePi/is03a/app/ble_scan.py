import asyncio
from datetime import datetime, timezone
import logging
from typing import Dict

from bleak import BleakScanner

from .ingest import handle_ingest

logger = logging.getLogger(__name__)


async def ble_scan_task(store, forwarder, event_hub, product_type: str = "003", product_code: str = "0096"):
    def callback(device, advertising_data):
        mac = device.address.replace(':', '').upper()
        rssi = advertising_data.rssi
        lacis_id = f"3{product_type}{mac}{product_code}"
        payload: Dict = {
            "seenAt": datetime.now(timezone.utc).isoformat(),
            "src": "ble",
            "mac": mac,
            "rssi": rssi,
            "sensor": {
                "mac": mac,
                "lacisId": lacis_id,
                "productType": product_type,
                "productCode": product_code,
            },
            "raw": {
                "advHex": str(advertising_data.service_data) if advertising_data.service_data else None,
                "mfgHex": str(advertising_data.manufacturer_data) if advertising_data.manufacturer_data else None,
            },
        }
        # fire-and-forget; store/forwarder/event_hub are thread-safe enough for this use (enqueue only)
        asyncio.create_task(handle_ingest(payload, store, forwarder, event_hub))

    try:
        scanner = BleakScanner(callback)
        await scanner.start()
        while True:
            await asyncio.sleep(60)
    except Exception as exc:  # noqa: BLE001
        logger.error("BLE scan error: %s", exc, exc_info=True)
        await asyncio.sleep(5)


__all__ = ["ble_scan_task"]
