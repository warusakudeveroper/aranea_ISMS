import asyncio
import logging
from typing import Any, Dict, Optional
import httpx

logger = logging.getLogger(__name__)


class HttpClient:
    def __init__(self, timeout: int = 8, max_retry: int = 3):
        self.timeout = timeout
        self.max_retry = max_retry
        self._client = httpx.AsyncClient()

    async def post_json(self, url: str, payload: Dict[str, Any], headers: Optional[Dict[str, str]] = None) -> httpx.Response:
        last_exc = None
        for attempt in range(1, self.max_retry + 1):
            try:
                res = await self._client.post(url, json=payload, headers=headers, timeout=self.timeout)
                res.raise_for_status()
                return res
            except (httpx.RequestError, httpx.HTTPStatusError) as exc:
                last_exc = exc
                logger.warning("post_json failed attempt=%s url=%s err=%s", attempt, url, exc)
                if attempt < self.max_retry:
                    await asyncio.sleep(min(2 ** attempt, 5))
        if last_exc:
            raise last_exc
        raise RuntimeError("post_json failed without exception")

    async def close(self) -> None:
        await self._client.aclose()


__all__ = ["HttpClient"]
