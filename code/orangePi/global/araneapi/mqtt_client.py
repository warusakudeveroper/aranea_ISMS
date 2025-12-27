import asyncio
import json
import logging
import ssl
from typing import Awaitable, Callable, Dict, List, Optional, Tuple
from asyncio_mqtt import Client, MqttError

from .config import MqttConfig

logger = logging.getLogger(__name__)


Handler = Callable[[str, str], Awaitable[None]]


def _match(filter_topic: str, topic: str) -> bool:
    if filter_topic.endswith("/#"):
        prefix = filter_topic[:-2]
        return topic.startswith(prefix)
    return filter_topic == topic


class MqttClient:
    def __init__(self, cfg: MqttConfig):
        self.cfg = cfg
        self._handlers: List[Tuple[str, Handler]] = []
        self._task: Optional[asyncio.Task] = None
        self._client: Optional[Client] = None
        self.connected: bool = False
        self._stop_event = asyncio.Event()

    def add_handler(self, topic_filter: str, handler: Handler) -> None:
        self._handlers.append((topic_filter, handler))

    async def start(self) -> None:
        self._stop_event.clear()
        self._task = asyncio.create_task(self._runner())

    async def stop(self) -> None:
        self._stop_event.set()
        if self._task:
            await self._task
        if self._client:
            await self._client.disconnect()
        self.connected = False

    async def publish(self, topic: str, payload: Dict) -> None:
        if not self._client:
            logger.warning("MQTT publish skipped (not connected)")
            return
        await self._client.publish(topic, json.dumps(payload))

    async def _runner(self) -> None:
        while not self._stop_event.is_set():
            try:
                tls_ctx = ssl.create_default_context() if self.cfg.tls else None
                client_id = self.cfg.client_id or f"is20s-{self.cfg.base_topic}"
                async with Client(
                    hostname=self.cfg.host,
                    port=self.cfg.port,
                    username=self.cfg.user or None,
                    password=self.cfg.password or None,
                    tls_context=tls_ctx,
                    client_id=client_id,
                ) as client:
                    self._client = client
                    self.connected = True
                    for topic, _ in self._handlers:
                        await client.subscribe(topic)
                    async with client.unfiltered_messages() as messages:
                        async for msg in messages:
                            await self._dispatch(msg.topic, msg.payload.decode())
            except MqttError as exc:
                self.connected = False
                self._client = None
                logger.warning("MQTT disconnected: %s", exc)
                await asyncio.sleep(3)
            except Exception as exc:  # noqa: BLE001
                self.connected = False
                self._client = None
                logger.error("MQTT loop error: %s", exc, exc_info=True)
                await asyncio.sleep(3)

    async def _dispatch(self, topic: str, payload: str) -> None:
        for filter_topic, handler in self._handlers:
            if _match(filter_topic, topic):
                try:
                    await handler(topic, payload)
                except Exception as exc:  # noqa: BLE001
                    logger.error("MQTT handler error topic=%s err=%s", topic, exc, exc_info=True)


__all__ = ["MqttClient"]
