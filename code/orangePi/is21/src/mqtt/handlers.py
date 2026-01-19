"""
MQTTコマンドハンドラ
IS21用のMQTTコマンド処理

コマンド:
- ping: 生存確認
- set_config: 設定変更
- inference: 推論リクエスト
- model_load: モデルロード
- status: 状態取得
"""

import json
import logging
import time
from dataclasses import dataclass, field
from typing import Any, Callable, Coroutine, Dict, Optional

from aranea_common import (
    MqttManager,
    HardwareInfo,
    build_command_topic,
    build_response_topic,
    build_status_topic,
)

logger = logging.getLogger(__name__)


@dataclass
class MqttHandlerContext:
    """ハンドラコンテキスト"""
    lacis_id: str = ""
    cic: str = ""
    tid: str = ""
    base_topic: str = "aranea"
    hardware_info: Optional[HardwareInfo] = None
    inference_callback: Optional[Callable] = None
    config_callback: Optional[Callable] = None
    status_callback: Optional[Callable] = None


class MqttCommandHandler:
    """
    MQTTコマンドハンドラ

    MqttManagerと連携してコマンドを処理
    """

    def __init__(
        self,
        mqtt: MqttManager,
        context: MqttHandlerContext,
    ):
        self.mqtt = mqtt
        self.ctx = context
        self._command_stats: Dict[str, int] = {}
        self._last_command: Optional[Dict] = None
        self._last_command_status: str = ""

    @property
    def command_topic(self) -> str:
        """コマンドトピック"""
        return build_command_topic(
            self.ctx.base_topic,
            self.ctx.tid,
            self.ctx.lacis_id,
        )

    @property
    def response_topic(self) -> str:
        """レスポンストピック"""
        return build_response_topic(
            self.ctx.base_topic,
            self.ctx.tid,
            self.ctx.lacis_id,
        )

    @property
    def status_topic(self) -> str:
        """ステータストピック"""
        return build_status_topic(
            self.ctx.base_topic,
            self.ctx.tid,
            self.ctx.lacis_id,
        )

    async def setup(self) -> bool:
        """
        ハンドラをセットアップ

        MQTTマネージャに購読を登録
        """
        if not self.mqtt.connected:
            logger.warning("[MQTT-HANDLER] MQTT not connected, skipping setup")
            return False

        await self.mqtt.subscribe(self.command_topic, self._handle_command)
        logger.info(f"[MQTT-HANDLER] Subscribed to {self.command_topic}")
        return True

    async def _handle_command(self, topic: str, payload_str: str) -> None:
        """コマンド受信ハンドラ"""
        try:
            payload = json.loads(payload_str) if payload_str else {}
        except json.JSONDecodeError:
            logger.warning(f"[MQTT-HANDLER] Invalid JSON: {payload_str[:100]}")
            await self._send_response({"status": "error", "message": "invalid_json"})
            return

        cmd_type = payload.get("type") or payload.get("cmd") or ""
        self._last_command = payload
        self._last_command_status = "processing"

        # 統計更新
        self._command_stats[cmd_type] = self._command_stats.get(cmd_type, 0) + 1

        logger.info(f"[MQTT-HANDLER] Command received: {cmd_type}")

        # コマンド別処理
        if cmd_type == "ping":
            await self._handle_ping(payload)
        elif cmd_type == "set_config":
            await self._handle_set_config(payload)
        elif cmd_type == "inference":
            await self._handle_inference(payload)
        elif cmd_type == "model_load":
            await self._handle_model_load(payload)
        elif cmd_type == "status":
            await self._handle_status(payload)
        else:
            await self._send_response({
                "status": "unsupported",
                "cmd": cmd_type,
                "message": f"Unknown command: {cmd_type}",
            })
            self._last_command_status = "unsupported"

    async def _handle_ping(self, payload: Dict) -> None:
        """pingコマンド処理"""
        status_data = self._get_status_data()
        response = {
            "type": "ping",
            "status": "ok",
            **status_data,
        }
        await self._send_response(response)
        self._last_command_status = "ok"

    async def _handle_set_config(self, payload: Dict) -> None:
        """set_configコマンド処理"""
        config_data = payload.get("config", {})

        if not config_data:
            await self._send_response({
                "type": "set_config",
                "status": "error",
                "message": "No config data provided",
            })
            self._last_command_status = "error"
            return

        # コールバックで設定を適用
        result = {}
        if self.ctx.config_callback:
            try:
                result = await self.ctx.config_callback(config_data)
            except Exception as e:
                await self._send_response({
                    "type": "set_config",
                    "status": "error",
                    "message": str(e),
                })
                self._last_command_status = "error"
                return

        await self._send_response({
            "type": "set_config",
            "status": "accepted",
            **result,
        })
        self._last_command_status = "accepted"

    async def _handle_inference(self, payload: Dict) -> None:
        """inferenceコマンド処理"""
        if not self.ctx.inference_callback:
            await self._send_response({
                "type": "inference",
                "status": "error",
                "message": "Inference not available",
            })
            self._last_command_status = "error"
            return

        try:
            # 推論リクエスト
            image_data = payload.get("image")
            options = payload.get("options", {})
            request_id = payload.get("request_id", f"mqtt_{time.time():.0f}")

            result = await self.ctx.inference_callback(image_data, options)

            await self._send_response({
                "type": "inference",
                "status": "ok",
                "request_id": request_id,
                **result,
            })
            self._last_command_status = "ok"

        except Exception as e:
            logger.error(f"[MQTT-HANDLER] Inference error: {e}")
            await self._send_response({
                "type": "inference",
                "status": "error",
                "message": str(e),
            })
            self._last_command_status = "error"

    async def _handle_model_load(self, payload: Dict) -> None:
        """model_loadコマンド処理"""
        model_name = payload.get("model")

        if not model_name:
            await self._send_response({
                "type": "model_load",
                "status": "error",
                "message": "No model name specified",
            })
            self._last_command_status = "error"
            return

        # モデルロードは現在のmain.pyで初期化時に行われる
        # 動的ロードは将来実装
        await self._send_response({
            "type": "model_load",
            "status": "not_implemented",
            "message": "Dynamic model loading not yet supported",
            "model": model_name,
        })
        self._last_command_status = "not_implemented"

    async def _handle_status(self, payload: Dict) -> None:
        """statusコマンド処理"""
        status_data = self._get_status_data()

        # カスタムステータスコールバック
        if self.ctx.status_callback:
            try:
                custom_status = await self.ctx.status_callback()
                status_data.update(custom_status)
            except Exception as e:
                logger.warning(f"[MQTT-HANDLER] Status callback error: {e}")

        await self._send_response({
            "type": "status",
            "status": "ok",
            **status_data,
        })
        self._last_command_status = "ok"

    def _get_status_data(self) -> Dict[str, Any]:
        """基本ステータスデータを取得"""
        data = {
            "lacisId": self.ctx.lacis_id,
            "cic": self.ctx.cic,
            "mqttConnected": self.mqtt.connected,
            "timestamp": time.time(),
        }

        # ハードウェア情報
        if self.ctx.hardware_info:
            try:
                hw_summary = self.ctx.hardware_info.get_summary()
                data["hardware"] = {
                    "uptime_sec": hw_summary.get("uptime_sec", 0),
                    "memory_percent": hw_summary.get("memory", {}).get("percent_used", 0),
                    "cpu_load": hw_summary.get("cpu", {}).get("load_1min", 0),
                    "cpu_temp": hw_summary.get("thermal", {}).get("cpu", 0),
                }
            except Exception:
                pass

        return data

    async def _send_response(self, data: Dict) -> None:
        """レスポンスを送信"""
        await self.mqtt.publish(self.response_topic, data)

    async def publish_status(self) -> bool:
        """ステータスを送信"""
        status_data = self._get_status_data()
        return await self.mqtt.publish(self.status_topic, status_data)

    def get_stats(self) -> Dict[str, Any]:
        """ハンドラ統計を取得"""
        return {
            "commandTopic": self.command_topic,
            "responseTopic": self.response_topic,
            "statusTopic": self.status_topic,
            "commandStats": self._command_stats,
            "lastCommand": self._last_command,
            "lastCommandStatus": self._last_command_status,
        }


def build_mqtt_handlers(
    mqtt: MqttManager,
    lacis_id: str,
    cic: str,
    tid: str,
    hardware_info: Optional[HardwareInfo] = None,
    inference_callback: Optional[Callable] = None,
    config_callback: Optional[Callable] = None,
    status_callback: Optional[Callable] = None,
    base_topic: str = "aranea",
) -> MqttCommandHandler:
    """
    MQTTコマンドハンドラを構築

    Args:
        mqtt: MqttManagerインスタンス
        lacis_id: デバイスのLacisID
        cic: デバイスのCICコード
        tid: テナントID
        hardware_info: HardwareInfoインスタンス（オプション）
        inference_callback: 推論コールバック（オプション）
        config_callback: 設定変更コールバック（オプション）
        status_callback: カスタムステータスコールバック（オプション）
        base_topic: MQTTベーストピック

    Returns:
        MqttCommandHandler インスタンス
    """
    context = MqttHandlerContext(
        lacis_id=lacis_id,
        cic=cic,
        tid=tid,
        base_topic=base_topic,
        hardware_info=hardware_info,
        inference_callback=inference_callback,
        config_callback=config_callback,
        status_callback=status_callback,
    )

    return MqttCommandHandler(mqtt, context)


__all__ = ["MqttCommandHandler", "MqttHandlerContext", "build_mqtt_handlers"]
