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
    # Extract observedAt from either top-level or meta.observedAt
    observed_at = payload.get('observedAt') or (payload.get('meta', {}).get('observedAt') if payload.get('meta') else None) or datetime.utcnow().isoformat() + 'Z'

    sensor = payload.get('sensor', {})
    state = payload.get('state', {})
    raw = payload.get('raw', {})
    gateway = payload.get('gateway', {})

    event = {
        'seenAt': observed_at,
        'src': 'is02',
        'mac': sensor.get('mac'),
        'rssi': None,
        'adv_hex': None,
        'manufacturer_hex': raw.get('mfgHex'),
        'lacisId': sensor.get('lacisId'),
        'productType': sensor.get('productType'),
        'productCode': sensor.get('productCode'),
        'gatewayLacisId': gateway.get('lacisId'),
        'gatewayIp': gateway.get('ip'),
        # Support both naming conventions: temperatureC/temperature, humidityPct/humidity, batteryPct/battery
        'temperatureC': state.get('temperatureC') or state.get('temperature'),
        'humidityPct': state.get('humidityPct') or state.get('humidity'),
        'batteryPct': state.get('batteryPct') or state.get('battery')
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
