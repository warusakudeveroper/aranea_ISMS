from fastapi import APIRouter, WebSocket, WebSocketDisconnect
from fastapi.responses import FileResponse
import app.db as db
from datetime import datetime
import json
import asyncio

router = APIRouter()

class WSManager:
    def __init__(self):
        self.connections = []

    async def connect(self, websocket: WebSocket):
        await websocket.accept()
        self.connections.append(websocket)

    def disconnect(self, websocket: WebSocket):
        if websocket in self.connections:
            self.connections.remove(websocket)

    async def broadcast(self, message: dict):
        for conn in self.connections[:]:
            try:
                await conn.send_json(message)
            except:
                self.connections.remove(conn)

ws_manager = WSManager()

@router.get('/api/status')
async def get_status():
    import socket
    import time
    status = db.get_status()
    hostname = socket.gethostname()

    try:
        import subprocess
        uptime_sec = float(subprocess.check_output(['cat', '/proc/uptime']).decode().split()[0])
    except:
        uptime_sec = 0

    try:
        bt_status = subprocess.check_output(['systemctl', 'is-active', 'bluetooth']).decode().strip()
        bluetooth = 'ok' if bt_status == 'active' else 'ng'
    except:
        bluetooth = 'ng'

    return {
        'ok': True,
        'hostname': hostname,
        'uptimeSec': int(uptime_sec),
        'bluetooth': bluetooth,
        'db': 'ok',
        **status
    }

@router.get('/api/events')
async def get_events(limit: int = 200):
    limit = min(limit, 2000)
    events = db.get_ring_events(limit)
    return events

async def handle_is02_post(payload: dict):
    event = {
        'seenAt': payload.get('observedAt', datetime.utcnow().isoformat() + 'Z'),
        'src': 'is02',
        'mac': payload.get('sensor', {}).get('mac') if payload.get('sensor') else None,
        'rssi': None,
        'adv_hex': None,
        'manufacturer_hex': payload.get('raw', {}).get('mfgHex') if payload.get('raw') else None,
        'lacisId': payload.get('sensor', {}).get('lacisId') if payload.get('sensor') else None,
        'productType': payload.get('sensor', {}).get('productType') if payload.get('sensor') else None,
        'productCode': payload.get('sensor', {}).get('productCode') if payload.get('sensor') else None,
        'gatewayLacisId': payload.get('gateway', {}).get('lacisId') if payload.get('gateway') else None,
        'gatewayIp': payload.get('gateway', {}).get('ip') if payload.get('gateway') else None,
        'temperatureC': payload.get('state', {}).get('temperatureC') if payload.get('state') else None,
        'humidityPct': payload.get('state', {}).get('humidityPct') if payload.get('state') else None,
        'batteryPct': payload.get('state', {}).get('batteryPct') if payload.get('state') else None
    }
    db.enqueue_event(event)
    await ws_manager.broadcast(event)
    return {'ok': True}

@router.post('/api/ingest')
async def post_ingest(payload: dict):
    return await handle_is02_post(payload)

@router.post('/api/events')
async def post_events(payload: dict):
    return await handle_is02_post(payload)

@router.post('/api/debug/publishSample')
async def publish_sample():
    event = {
        'seenAt': datetime.utcnow().isoformat() + 'Z',
        'src': 'debug',
        'mac': 'AABBCCDDEEFF',
        'rssi': -50,
        'adv_hex': None,
        'manufacturer_hex': None,
        'lacisId': '3003AABBCCDDEEFF0096',
        'productType': '003',
        'productCode': '0096'
    }
    db.enqueue_event(event)
    await ws_manager.broadcast(event)
    return {'ok': True}

@router.get('/')
async def index():
    return FileResponse('/opt/is03-relay/app/static/index.html')

@router.websocket('/ws')
async def websocket_endpoint(websocket: WebSocket):
    await ws_manager.connect(websocket)
    try:
        while True:
            await websocket.receive_text()
    except WebSocketDisconnect:
        ws_manager.disconnect(websocket)
