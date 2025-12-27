import asyncio
import logging
from typing import Awaitable, Callable, Optional

logger = logging.getLogger(__name__)


class PeriodicTask:
    def __init__(self, interval_sec: int, func: Callable[[], Awaitable[None]]):
        self.interval_sec = interval_sec
        self.func = func
        self._stop_event = asyncio.Event()
        self._task: Optional[asyncio.Task] = None

    async def start(self) -> None:
        self._stop_event.clear()
        self._task = asyncio.create_task(self._runner())

    async def stop(self) -> None:
        self._stop_event.set()
        if self._task:
            await self._task

    async def _runner(self) -> None:
        while not self._stop_event.is_set():
            try:
                await self.func()
            except Exception as exc:  # noqa: BLE001
                logger.error("PeriodicTask error: %s", exc, exc_info=True)
            try:
                await asyncio.wait_for(self._stop_event.wait(), timeout=self.interval_sec)
            except asyncio.TimeoutError:
                continue


__all__ = ["PeriodicTask"]
