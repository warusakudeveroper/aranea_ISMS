"""
MqttManager - MQTTクライアント管理
ESP32 MqttManager.cppのPython移植版

aiomqttベースの非同期MQTTクライアント
- TLS接続対応
- 自動再接続
- コマンドハンドラ登録
"""

import asyncio
import json
import logging
import ssl
from dataclasses import dataclass, field
from typing import Any, Awaitable, Callable, Dict, Optional, Union
from urllib.parse import urlparse

logger = logging.getLogger(__name__)

# aiomqttのインポート（インストールされていない場合のフォールバック）
try:
    import aiomqtt
    AIOMQTT_AVAILABLE = True
except ImportError:
    AIOMQTT_AVAILABLE = False
    logger.warning("[MQTT] aiomqtt not installed. MQTT functionality disabled.")


@dataclass
class MqttConfig:
    """MQTT接続設定"""
    broker: str = ""
    port: int = 8883
    username: str = ""
    password: str = ""
    client_id: str = ""
    use_tls: bool = True
    keepalive: int = 60
    reconnect_interval: float = 5.0
    max_reconnect_attempts: int = 0  # 0 = unlimited


@dataclass
class MqttStats:
    """MQTT統計情報"""
    messages_received: int = 0
    messages_sent: int = 0
    reconnect_count: int = 0
    last_connect_time: float = 0.0
    last_message_time: float = 0.0
    errors: int = 0


MessageHandler = Callable[[str, str], Awaitable[None]]


class MqttManager:
    """
    MQTTクライアント管理

    ESP32 MqttManager.cppのPython移植版
    aiomqttを使用した非同期実装
    """

    def __init__(self):
        self.connected: bool = False
        self.config: MqttConfig = MqttConfig()
        self.stats: MqttStats = MqttStats()

        self._client: Optional[Any] = None
        self._handlers: Dict[str, MessageHandler] = {}
        self._subscriptions: Dict[str, int] = {}  # topic -> qos
        self._running: bool = False
        self._reconnect_task: Optional[asyncio.Task] = None
        self._listener_task: Optional[asyncio.Task] = None
        self._lock: asyncio.Lock = asyncio.Lock()

    async def begin(
        self,
        url: str,
        username: str = "",
        password: str = "",
        client_id: str = None,
        use_tls: bool = True,
        keepalive: int = 60,
    ) -> bool:
        """
        MQTT接続を開始

        Args:
            url: MQTTブローカーURL (例: "mqtt.example.com:8883")
            username: 認証ユーザー名
            password: 認証パスワード
            client_id: クライアントID（Noneの場合は自動生成）
            use_tls: TLS使用フラグ
            keepalive: キープアライブ間隔（秒）

        Returns:
            接続成功: True, 失敗: False
        """
        if not AIOMQTT_AVAILABLE:
            logger.error("[MQTT] aiomqtt not available. Install with: pip install aiomqtt")
            return False

        # URL解析
        parsed = urlparse(url if "://" in url else f"mqtt://{url}")
        broker = parsed.hostname or url.split(":")[0]
        port = parsed.port or (8883 if use_tls else 1883)

        self.config = MqttConfig(
            broker=broker,
            port=port,
            username=username,
            password=password,
            client_id=client_id or f"is21-{asyncio.get_event_loop().time():.0f}",
            use_tls=use_tls,
            keepalive=keepalive,
        )

        self._running = True

        try:
            await self._connect()
            return True
        except Exception as e:
            logger.error(f"[MQTT] Initial connection failed: {e}")
            # 自動再接続を開始
            self._start_reconnect_task()
            return False

    async def _connect(self) -> None:
        """内部接続処理"""
        import time

        tls_context = None
        if self.config.use_tls:
            tls_context = ssl.create_default_context()
            tls_context.check_hostname = True
            tls_context.verify_mode = ssl.CERT_REQUIRED

        logger.info(f"[MQTT] Connecting to {self.config.broker}:{self.config.port}...")

        self._client = aiomqtt.Client(
            hostname=self.config.broker,
            port=self.config.port,
            username=self.config.username if self.config.username else None,
            password=self.config.password if self.config.password else None,
            identifier=self.config.client_id,
            tls_context=tls_context,
            keepalive=self.config.keepalive,
        )

        await self._client.__aenter__()
        self.connected = True
        self.stats.last_connect_time = time.time()
        logger.info(f"[MQTT] Connected to {self.config.broker}")

        # 購読を復元
        for topic, qos in self._subscriptions.items():
            await self._client.subscribe(topic, qos=qos)
            logger.debug(f"[MQTT] Resubscribed to {topic}")

        # リスナータスク開始
        self._listener_task = asyncio.create_task(self._message_listener())

    async def _message_listener(self) -> None:
        """メッセージ受信ループ"""
        import time

        try:
            async for message in self._client.messages:
                topic = str(message.topic)
                payload = message.payload.decode("utf-8") if message.payload else ""

                self.stats.messages_received += 1
                self.stats.last_message_time = time.time()

                logger.debug(f"[MQTT] Received: {topic} -> {payload[:100]}...")

                # マッチするハンドラを呼び出し
                for pattern, handler in self._handlers.items():
                    if self._topic_matches(pattern, topic):
                        try:
                            await handler(topic, payload)
                        except Exception as e:
                            logger.error(f"[MQTT] Handler error for {topic}: {e}")
                            self.stats.errors += 1
        except asyncio.CancelledError:
            pass
        except Exception as e:
            logger.error(f"[MQTT] Listener error: {e}")
            self.connected = False
            if self._running:
                self._start_reconnect_task()

    def _topic_matches(self, pattern: str, topic: str) -> bool:
        """MQTTワイルドカードマッチング"""
        pattern_parts = pattern.split("/")
        topic_parts = topic.split("/")

        for i, pat in enumerate(pattern_parts):
            if pat == "#":
                return True
            if i >= len(topic_parts):
                return False
            if pat == "+":
                continue
            if pat != topic_parts[i]:
                return False

        return len(pattern_parts) == len(topic_parts)

    def _start_reconnect_task(self) -> None:
        """再接続タスクを開始"""
        if self._reconnect_task is None or self._reconnect_task.done():
            self._reconnect_task = asyncio.create_task(self._reconnect_loop())

    async def _reconnect_loop(self) -> None:
        """自動再接続ループ"""
        attempts = 0
        while self._running and not self.connected:
            attempts += 1
            self.stats.reconnect_count += 1

            if self.config.max_reconnect_attempts > 0 and attempts > self.config.max_reconnect_attempts:
                logger.error("[MQTT] Max reconnect attempts reached")
                break

            logger.info(f"[MQTT] Reconnecting... (attempt {attempts})")

            try:
                await self._connect()
                break
            except Exception as e:
                logger.warning(f"[MQTT] Reconnect failed: {e}")
                await asyncio.sleep(self.config.reconnect_interval)

    async def stop(self) -> None:
        """接続を停止"""
        self._running = False

        if self._listener_task:
            self._listener_task.cancel()
            try:
                await self._listener_task
            except asyncio.CancelledError:
                pass

        if self._reconnect_task:
            self._reconnect_task.cancel()
            try:
                await self._reconnect_task
            except asyncio.CancelledError:
                pass

        if self._client and self.connected:
            try:
                await self._client.__aexit__(None, None, None)
            except Exception as e:
                logger.warning(f"[MQTT] Disconnect error: {e}")

        self.connected = False
        self._client = None
        logger.info("[MQTT] Stopped")

    async def publish(
        self,
        topic: str,
        payload: Union[str, dict],
        qos: int = 1,
        retain: bool = False,
    ) -> bool:
        """
        メッセージを発行

        Args:
            topic: トピック
            payload: メッセージ内容（strまたはdict）
            qos: QoSレベル (0, 1, 2)
            retain: 保持フラグ

        Returns:
            発行成功: True, 失敗: False
        """
        if not self.connected or not self._client:
            logger.warning(f"[MQTT] Not connected, cannot publish to {topic}")
            return False

        if isinstance(payload, dict):
            payload = json.dumps(payload, ensure_ascii=False)

        try:
            await self._client.publish(topic, payload, qos=qos, retain=retain)
            self.stats.messages_sent += 1
            logger.debug(f"[MQTT] Published: {topic}")
            return True
        except Exception as e:
            logger.error(f"[MQTT] Publish error: {e}")
            self.stats.errors += 1
            return False

    async def subscribe(
        self,
        topic: str,
        handler: MessageHandler,
        qos: int = 1,
    ) -> bool:
        """
        トピックを購読

        Args:
            topic: トピックパターン（ワイルドカード可: +, #）
            handler: メッセージハンドラ async def handler(topic: str, payload: str)
            qos: QoSレベル

        Returns:
            購読成功: True, 失敗: False
        """
        async with self._lock:
            self._handlers[topic] = handler
            self._subscriptions[topic] = qos

            if self.connected and self._client:
                try:
                    await self._client.subscribe(topic, qos=qos)
                    logger.info(f"[MQTT] Subscribed to {topic}")
                    return True
                except Exception as e:
                    logger.error(f"[MQTT] Subscribe error: {e}")
                    return False

            return True  # 接続時に購読される

    async def unsubscribe(self, topic: str) -> bool:
        """購読解除"""
        async with self._lock:
            self._handlers.pop(topic, None)
            self._subscriptions.pop(topic, None)

            if self.connected and self._client:
                try:
                    await self._client.unsubscribe(topic)
                    logger.info(f"[MQTT] Unsubscribed from {topic}")
                    return True
                except Exception as e:
                    logger.error(f"[MQTT] Unsubscribe error: {e}")
                    return False

            return True

    def on_message(self, topic: str) -> Callable:
        """
        デコレータ形式のハンドラ登録

        Usage:
            @mqtt.on_message("aranea/+/device/+/command/#")
            async def handle_command(topic: str, payload: str):
                ...
        """
        def decorator(func: MessageHandler) -> MessageHandler:
            # 同期的に登録（subscribe呼び出しは接続後）
            self._handlers[topic] = func
            self._subscriptions[topic] = 1
            return func
        return decorator

    def get_stats(self) -> Dict[str, Any]:
        """統計情報を取得"""
        return {
            "connected": self.connected,
            "broker": f"{self.config.broker}:{self.config.port}",
            "clientId": self.config.client_id,
            "messagesReceived": self.stats.messages_received,
            "messagesSent": self.stats.messages_sent,
            "reconnectCount": self.stats.reconnect_count,
            "errors": self.stats.errors,
            "subscriptions": list(self._subscriptions.keys()),
        }


# トピックヘルパー関数
def build_command_topic(base: str, tid: str, lacis_id: str) -> str:
    """コマンドトピックを構築"""
    return f"{base}/{tid}/device/{lacis_id}/command/#"


def build_status_topic(base: str, tid: str, lacis_id: str) -> str:
    """ステータストピックを構築"""
    return f"{base}/{tid}/device/{lacis_id}/status"


def build_response_topic(base: str, tid: str, lacis_id: str) -> str:
    """レスポンストピックを構築"""
    return f"{base}/{tid}/device/{lacis_id}/response"


__all__ = [
    "MqttManager",
    "MqttConfig",
    "MqttStats",
    "MessageHandler",
    "build_command_topic",
    "build_status_topic",
    "build_response_topic",
]
