import asyncio
from collections import deque
from datetime import datetime, timezone
import json
import logging
from typing import Any, Dict, List, Optional
import aiosqlite

logger = logging.getLogger(__name__)


class EventStore:
    def __init__(
        self,
        db_path: str,
        ring_size: int = 2000,
        max_db_size: int = 20000,
        flush_batch: int = 50,
        flush_interval_sec: int = 10,
    ):
        self.db_path = db_path
        self.ring = deque(maxlen=ring_size)
        self.max_db_size = max_db_size
        self.flush_batch = flush_batch
        self.flush_interval_sec = flush_interval_sec
        self.write_queue: List[Dict[str, Any]] = []
        self.last_flush_at: Optional[str] = None
        self._lock = asyncio.Lock()
        self._stop_event = asyncio.Event()
        self._flush_task: Optional[asyncio.Task] = None

    async def init_db(self) -> None:
        async with aiosqlite.connect(self.db_path) as db:
            await db.execute(
                """
                CREATE TABLE IF NOT EXISTS events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    seenAt TEXT NOT NULL,
                    src TEXT,
                    payload TEXT
                )
                """
            )
            await db.commit()

    def enqueue(self, event: Dict[str, Any]) -> None:
        seen_at = event.get("seenAt") or datetime.now(timezone.utc).isoformat()
        event = {"seenAt": seen_at, **event}
        self.ring.append(event)
        self.write_queue.append(event)

    async def flush_once(self) -> None:
        async with self._lock:
            if not self.write_queue:
                return
            batch = self.write_queue[: self.flush_batch]
            self.write_queue = self.write_queue[self.flush_batch :]

        async with aiosqlite.connect(self.db_path) as db:
            rows = [(e.get("seenAt"), e.get("src"), json.dumps(e, ensure_ascii=False)) for e in batch]
            await db.executemany("INSERT INTO events (seenAt, src, payload) VALUES (?, ?, ?)", rows)
            await db.commit()

            cursor = await db.execute("SELECT COUNT(*) FROM events")
            (count,) = await cursor.fetchone()
            if count > self.max_db_size:
                delete_count = count - self.max_db_size
                await db.execute(
                    f"DELETE FROM events WHERE id IN (SELECT id FROM events ORDER BY id ASC LIMIT {delete_count})"
                )
                await db.commit()

        self.last_flush_at = datetime.now(timezone.utc).isoformat()

    async def flush_loop(self) -> None:
        while not self._stop_event.is_set():
            await asyncio.sleep(self.flush_interval_sec)
            try:
                await self.flush_once()
            except Exception as exc:  # noqa: BLE001
                logger.error("flush_loop error: %s", exc, exc_info=True)

    async def start_flush_task(self, interval_sec: Optional[int] = None) -> None:
        self._stop_event.clear()
        if interval_sec is not None:
            self.flush_interval_sec = interval_sec
        self._flush_task = asyncio.create_task(self.flush_loop())

    async def stop_flush_task(self) -> None:
        self._stop_event.set()
        if self._flush_task:
            await self._flush_task

    def get_recent(self, limit: int = 200) -> List[Dict[str, Any]]:
        limit = min(limit, self.ring.maxlen or limit)
        return list(self.ring)[-limit:][::-1]

    def get_status(self) -> Dict[str, Any]:
        return {
            "ringSize": len(self.ring),
            "queuedWrites": len(self.write_queue),
            "lastFlushAt": self.last_flush_at,
        }

    async def update_settings(self, ring_size: Optional[int] = None, max_db_size: Optional[int] = None,
                              flush_batch: Optional[int] = None, flush_interval_sec: Optional[int] = None) -> None:
        """
        ランタイムでストア設定を更新。リングサイズ変更時は既存データを移し替える。
        フラッシュタスクは再起動して新しい間隔を適用する。
        """
        if ring_size and ring_size != (self.ring.maxlen or ring_size):
            new_ring = deque(self.ring, maxlen=ring_size)
            self.ring = new_ring
        if max_db_size:
            self.max_db_size = max_db_size
        if flush_batch:
            self.flush_batch = flush_batch
        if flush_interval_sec:
            self.flush_interval_sec = flush_interval_sec
            await self.stop_flush_task()
            await self.start_flush_task()


__all__ = ["EventStore"]
