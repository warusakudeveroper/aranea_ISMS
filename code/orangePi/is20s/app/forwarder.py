import asyncio
import logging
from datetime import datetime, timezone
from typing import Any, Dict, Optional

from araneapi.config import RelayConfig
from araneapi.http_client import HttpClient

from .state import RuntimeState

logger = logging.getLogger(__name__)


class Forwarder:
    def __init__(self, cfg: RelayConfig, state: RuntimeState):
        self.cfg = cfg
        self.state = state
        self.http_client = HttpClient(timeout=cfg.timeout_sec, max_retry=cfg.max_retry)
        self.queue: asyncio.Queue[Dict[str, Any]] = asyncio.Queue()
        self._worker: Optional[asyncio.Task] = None
        self._stop_event = asyncio.Event()
        self._backoff_until: Optional[float] = None

    async def start(self) -> None:
        self._stop_event.clear()
        self._worker = asyncio.create_task(self._run())

    async def stop(self) -> None:
        self._stop_event.set()
        if self._worker:
            await self._worker
        await self.http_client.close()

    async def update_config(self, cfg: RelayConfig) -> None:
        """Forwarder設定を動的更新。"""
        self.cfg = cfg
        await self.http_client.close()
        self.http_client = HttpClient(timeout=cfg.timeout_sec, max_retry=cfg.max_retry)
        self._backoff_until = None

    async def enqueue(self, event: Dict[str, Any]) -> None:
        # キュー上限超過時は最古を捨てて新規を入れる
        while self.queue.qsize() >= self.cfg.max_queue > 0:
            try:
                self.queue.get_nowait()
                self.queue.task_done()
            except asyncio.QueueEmpty:
                break
        await self.queue.put(event)

    async def _run(self) -> None:
        while not self._stop_event.is_set():
            if self._backoff_until:
                now = asyncio.get_event_loop().time()
                if now < self._backoff_until:
                    await asyncio.sleep(min(1, self._backoff_until - now))
                    continue
                self._backoff_until = None
            try:
                event = await asyncio.wait_for(self.queue.get(), timeout=1.0)
            except asyncio.TimeoutError:
                continue
            try:
                await self._forward_event(event)
            except Exception as exc:  # noqa: BLE001
                logger.error("Forwarder error: %s", exc, exc_info=True)
                self.state.forward_failures += 1
            finally:
                self.queue.task_done()

    async def _forward_event(self, event: Dict[str, Any]) -> None:
        payload = event.get("payload", event)
        targets = [self.cfg.primary_url, self.cfg.secondary_url]
        sent_any = False
        for url in targets:
            if not url:
                continue
            try:
                await self.http_client.post_json(url, payload)
                sent_any = True
            except Exception as exc:  # noqa: BLE001
                logger.warning("Forwarder send failed url=%s err=%s", url, exc)
                continue

        if sent_any:
            self.state.last_forward_ok_at = datetime.now(timezone.utc).isoformat()
            self.state.forward_failures = 0
            self._backoff_until = None
        else:
            self.state.forward_failures += 1
            delay = min(self.cfg.backoff_max_sec, self.cfg.backoff_base_sec * (2 ** max(self.state.forward_failures - 1, 0)))
            self._backoff_until = asyncio.get_event_loop().time() + delay


__all__ = ["Forwarder"]
