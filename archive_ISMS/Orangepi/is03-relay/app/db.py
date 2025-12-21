import aiosqlite
import asyncio
from collections import deque
from datetime import datetime
import json

DB_PATH = '/var/lib/is03-relay/relay.sqlite'
MAX_RING_SIZE = 2000
MAX_DB_SIZE = 20000
FLUSH_INTERVAL_SEC = 10
FLUSH_BATCH_SIZE = 50

inMemoryRing = deque(maxlen=MAX_RING_SIZE)
writeQueue = []
lastFlushAt = None

async def init_db():
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute('PRAGMA journal_mode=WAL')
        await db.execute('PRAGMA synchronous=NORMAL')
        await db.execute('PRAGMA temp_store=MEMORY')
        await db.execute('''
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                seenAt TEXT NOT NULL,
                src TEXT NOT NULL,
                mac TEXT NOT NULL,
                rssi INTEGER,
                adv_hex TEXT,
                manufacturer_hex TEXT,
                lacisId TEXT,
                productType TEXT,
                productCode TEXT
            )
        ''')
        await db.commit()

def enqueue_event(event):
    inMemoryRing.append(event)
    writeQueue.append(event)

async def flush_to_db():
    global lastFlushAt, writeQueue
    if not writeQueue:
        return
    batch = writeQueue[:FLUSH_BATCH_SIZE]
    writeQueue = writeQueue[FLUSH_BATCH_SIZE:]
    
    async with aiosqlite.connect(DB_PATH) as db:
        await db.executemany('''
            INSERT INTO events (seenAt, src, mac, rssi, adv_hex, manufacturer_hex, lacisId, productType, productCode)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ''', [(e['seenAt'], e['src'], e['mac'], e.get('rssi'), e.get('adv_hex'), 
               e.get('manufacturer_hex'), e.get('lacisId'), e.get('productType'), e.get('productCode')) for e in batch])
        await db.commit()
        
        cursor = await db.execute('SELECT COUNT(*) FROM events')
        count = (await cursor.fetchone())[0]
        if count > MAX_DB_SIZE:
            delete_count = count - MAX_DB_SIZE
            await db.execute(f'DELETE FROM events WHERE id IN (SELECT id FROM events ORDER BY id ASC LIMIT {delete_count})')
            await db.commit()
    
    lastFlushAt = datetime.utcnow().isoformat() + 'Z'

async def flush_task():
    while True:
        await asyncio.sleep(FLUSH_INTERVAL_SEC)
        if len(writeQueue) >= FLUSH_BATCH_SIZE:
            await flush_to_db()
        elif writeQueue:
            await flush_to_db()

def get_ring_events(limit=200):
    limit = min(limit, MAX_RING_SIZE)
    return list(inMemoryRing)[-limit:][::-1]

def get_status():
    return {
        'ringSize': len(inMemoryRing),
        'queuedWrites': len(writeQueue),
        'lastFlushAt': lastFlushAt
    }
