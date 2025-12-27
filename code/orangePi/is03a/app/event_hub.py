import asyncio
from typing import Dict, Any, List


class EventHub:
    """SSE用の簡易ハブ。購読者ごとにQueueを持ち、満杯なら古いものを捨てる。"""

    def __init__(self, queue_size: int = 200):
        self.subscribers: List[asyncio.Queue] = []
        self.queue_size = queue_size
        self._lock = asyncio.Lock()

    async def register(self) -> asyncio.Queue:
        q: asyncio.Queue = asyncio.Queue(maxsize=self.queue_size)
        async with self._lock:
            self.subscribers.append(q)
        return q

    async def unregister(self, q: asyncio.Queue) -> None:
        async with self._lock:
            if q in self.subscribers:
                self.subscribers.remove(q)

    async def broadcast(self, event: Dict[str, Any]) -> None:
        async with self._lock:
            for q in self.subscribers[:]:
                try:
                    if q.full():
                        try:
                            q.get_nowait()
                            q.task_done()
                        except asyncio.QueueEmpty:
                            pass
                    q.put_nowait(event)
                except Exception:
                    try:
                        self.subscribers.remove(q)
                    except ValueError:
                        pass


__all__ = ["EventHub"]
