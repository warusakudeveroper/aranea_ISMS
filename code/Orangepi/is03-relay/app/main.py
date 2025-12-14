from fastapi import FastAPI
from contextlib import asynccontextmanager
import asyncio
import app.db as db
import app.ble as ble
import app.web as web

@asynccontextmanager
async def lifespan(app: FastAPI):
    # 起動時
    await db.init_db()
    flush_task = asyncio.create_task(db.flush_task())
    ble_task = asyncio.create_task(ble.ble_scan_task())
    yield
    # 終了時
    flush_task.cancel()
    ble_task.cancel()

app = FastAPI(lifespan=lifespan)
app.include_router(web.router)
