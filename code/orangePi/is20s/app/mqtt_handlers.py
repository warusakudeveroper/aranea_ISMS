import json
import logging
from typing import Dict, Callable, Awaitable, Any

from araneapi.mqtt_client import MqttClient
from araneapi.config import AppConfig
from araneapi.system_info import uptime_seconds

from .forwarder import Forwarder
from .state import RuntimeState

logger = logging.getLogger(__name__)

Handler = Callable[[str, str], Awaitable[None]]


def _response_topic(cfg: AppConfig, lacis_id: str) -> str:
    return f"{cfg.mqtt.base_topic}/{cfg.register.tid}/device/{lacis_id}/response"


def _status_topic(cfg: AppConfig, lacis_id: str) -> str:
    return f"{cfg.mqtt.base_topic}/{cfg.register.tid}/device/{lacis_id}/status"


def build_mqtt_handlers(
    cfg: AppConfig,
    lacis_id: str,
    state: RuntimeState,
    forwarder: Forwarder,
    mqtt: MqttClient,
    apply_config_callback: Callable[[Dict[str, Any]], Awaitable[Dict[str, Any]]],
) -> Dict[str, Handler]:
    """
    コマンド購読ハンドラ群を生成し、topic filter -> handler の dict を返す。
    """
    cmd_topic = f"{cfg.mqtt.base_topic}/{cfg.register.tid}/device/{lacis_id}/command/#"

    async def _publish_response(data: Dict) -> None:
        await mqtt.publish(_response_topic(cfg, lacis_id), data)

    def _status_payload() -> Dict[str, Any]:
        return {
            "status": "ok",
            "lacisId": lacis_id,
            "uptimeSec": int(uptime_seconds() or 0),
            "forwarderQueued": forwarder.queue.qsize(),
            "mqttConnected": mqtt.connected,
            "cic": state.cic,
        }

    async def handle_command(topic: str, payload_str: str) -> None:
        try:
            payload = json.loads(payload_str) if payload_str else {}
        except json.JSONDecodeError:
            logger.warning("MQTT command decode error: %s", payload_str)
            await _publish_response({"status": "error", "message": "invalid_json"})
            return

        cmd_type = payload.get("type") or payload.get("cmd")
        state.last_command = payload
        state.last_command_status = "processing"

        if cmd_type == "ping":
            resp = {"type": "ping", **_status_payload()}
            await _publish_response(resp)
            state.last_command_status = "ok"
            return

        if cmd_type == "relay_now":
            event = payload.get("event") or {"src": "mqtt-relay", "seenAt": payload.get("seenAt")}
            await forwarder.enqueue({"payload": event})
            await _publish_response({"status": "queued", "type": "relay_now"})
            state.last_command_status = "queued"
            return

        if cmd_type == "set_config":
            apply_res = await apply_config_callback(payload.get("config") or {})
            await _publish_response({"status": "accepted", "type": "set_config", **apply_res})
            state.last_command_status = "accepted"
            return

        await _publish_response({"status": "unsupported", "cmd": cmd_type})
        state.last_command_status = "unsupported"

    return {cmd_topic: handle_command}


__all__ = ["build_mqtt_handlers", "_status_topic"]
