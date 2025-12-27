import asyncio
from bleak import BleakScanner
from datetime import datetime
import app.db as db

async def ble_scan_task():
    def callback(device, advertising_data):
        mac = device.address.replace(':', '').upper()
        rssi = advertising_data.rssi
        
        # 暫定lacisId生成: 3 + 003 + MAC12 + 0096
        lacisId = f"3003{mac}0096"
        
        event = {
            'seenAt': datetime.utcnow().isoformat() + 'Z',
            'src': 'ble',
            'mac': mac,
            'rssi': rssi,
            'adv_hex': str(advertising_data.service_data) if advertising_data.service_data else None,
            'manufacturer_hex': str(advertising_data.manufacturer_data) if advertising_data.manufacturer_data else None,
            'lacisId': lacisId,
            'productType': '003',
            'productCode': '0096'
        }
        db.enqueue_event(event)
    
    scanner = BleakScanner(callback)
    await scanner.start()
    # スキャンは継続的に実行
    while True:
        await asyncio.sleep(60)
