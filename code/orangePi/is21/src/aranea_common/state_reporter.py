"""
StateReporter - 状態報告モジュール
ESP32 StateReporter.cppのPython移植版

定期状態報告機能:
- stateEndpointへのHTTP POST
- HardwareInfo連携
- デバイス固有状態
"""

import asyncio
import json
import logging
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any, Callable, Dict, Optional

import httpx

from .hardware_info import HardwareInfo

logger = logging.getLogger(__name__)


@dataclass
class StateReporterConfig:
    """状態報告設定"""
    state_endpoint: str = ""
    interval_sec: int = 300  # 5分
    timeout_sec: float = 10.0
    retry_count: int = 3
    retry_delay_sec: float = 5.0
    enabled: bool = True


@dataclass
class ReportStats:
    """報告統計"""
    total_reports: int = 0
    successful_reports: int = 0
    failed_reports: int = 0
    last_report_time: float = 0.0
    last_success_time: float = 0.0
    last_error: str = ""


@dataclass
class DeviceState:
    """デバイス状態"""
    status: str = "running"
    lacis_id: str = ""
    cic: str = ""
    device_specific: Dict[str, Any] = field(default_factory=dict)


class StateReporter:
    """
    状態報告管理

    ESP32 StateReporter.cppのPython移植版
    定期的にstateEndpointへデバイス状態を報告
    """

    def __init__(
        self,
        hardware_info: Optional[HardwareInfo] = None,
    ):
        self.hardware_info = hardware_info or HardwareInfo()
        self.config = StateReporterConfig()
        self.stats = ReportStats()
        self.device_state = DeviceState()

        self._running: bool = False
        self._report_task: Optional[asyncio.Task] = None
        self._state_callback: Optional[Callable[[], Dict[str, Any]]] = None
        self._lock = asyncio.Lock()

    def begin(
        self,
        state_endpoint: str,
        lacis_id: str,
        cic: str,
        interval_sec: int = 300,
    ) -> bool:
        """
        状態報告を初期化

        Args:
            state_endpoint: 報告先エンドポイントURL
            lacis_id: デバイスのLacisID
            cic: デバイスのCICコード
            interval_sec: 報告間隔（秒）

        Returns:
            初期化成功: True
        """
        if not state_endpoint:
            logger.warning("[STATE] No state endpoint configured")
            self.config.enabled = False
            return False

        self.config.state_endpoint = state_endpoint
        self.config.interval_sec = interval_sec
        self.device_state.lacis_id = lacis_id
        self.device_state.cic = cic

        logger.info(f"[STATE] Initialized: endpoint={state_endpoint}, interval={interval_sec}s")
        return True

    def set_status(self, status: str) -> None:
        """デバイスステータスを設定"""
        self.device_state.status = status

    def set_device_specific(self, data: Dict[str, Any]) -> None:
        """デバイス固有データを設定"""
        self.device_state.device_specific = data

    def set_state_callback(self, callback: Callable[[], Dict[str, Any]]) -> None:
        """
        状態取得コールバックを設定

        コールバックは報告時に呼び出され、デバイス固有のデータを返す
        """
        self._state_callback = callback

    async def start(self) -> None:
        """定期報告を開始"""
        if self._running:
            return

        if not self.config.enabled:
            logger.info("[STATE] Reporter disabled, not starting")
            return

        self._running = True
        self._report_task = asyncio.create_task(self._report_loop())
        logger.info("[STATE] Started periodic reporting")

    async def stop(self) -> None:
        """定期報告を停止"""
        self._running = False

        if self._report_task:
            self._report_task.cancel()
            try:
                await self._report_task
            except asyncio.CancelledError:
                pass

        logger.info("[STATE] Stopped periodic reporting")

    async def _report_loop(self) -> None:
        """報告ループ"""
        # 初回は少し遅延
        await asyncio.sleep(10)

        while self._running:
            try:
                await self.report_now()
            except Exception as e:
                logger.error(f"[STATE] Report loop error: {e}")
                self.stats.last_error = str(e)

            await asyncio.sleep(self.config.interval_sec)

    async def report_now(self) -> bool:
        """
        即座に状態を報告

        Returns:
            報告成功: True, 失敗: False
        """
        async with self._lock:
            return await self._send_report()

    async def _send_report(self) -> bool:
        """報告送信（内部）"""
        if not self.config.state_endpoint:
            return False

        # ペイロード構築
        payload = self._build_payload()

        self.stats.total_reports += 1
        self.stats.last_report_time = time.time()

        for attempt in range(self.config.retry_count):
            try:
                async with httpx.AsyncClient(timeout=self.config.timeout_sec) as client:
                    response = await client.post(
                        self.config.state_endpoint,
                        json=payload,
                        headers={"Content-Type": "application/json"},
                    )

                    if response.status_code in (200, 201, 202, 204):
                        self.stats.successful_reports += 1
                        self.stats.last_success_time = time.time()
                        logger.debug(f"[STATE] Report sent successfully (attempt {attempt + 1})")
                        return True

                    logger.warning(f"[STATE] Report failed: HTTP {response.status_code}")
                    self.stats.last_error = f"HTTP {response.status_code}"

            except httpx.TimeoutException:
                logger.warning(f"[STATE] Report timeout (attempt {attempt + 1})")
                self.stats.last_error = "timeout"
            except httpx.RequestError as e:
                logger.warning(f"[STATE] Report error: {e}")
                self.stats.last_error = str(e)
            except Exception as e:
                logger.error(f"[STATE] Unexpected error: {e}")
                self.stats.last_error = str(e)

            if attempt < self.config.retry_count - 1:
                await asyncio.sleep(self.config.retry_delay_sec)

        self.stats.failed_reports += 1
        return False

    def _build_payload(self) -> Dict[str, Any]:
        """報告ペイロードを構築"""
        # ハードウェア情報
        hw_summary = self.hardware_info.get_summary()

        # デバイス固有状態
        device_specific = self.device_state.device_specific.copy()
        if self._state_callback:
            try:
                callback_data = self._state_callback()
                device_specific.update(callback_data)
            except Exception as e:
                logger.warning(f"[STATE] State callback error: {e}")

        # タイムスタンプ
        timestamp = datetime.now(timezone.utc).isoformat()

        return {
            "lacisId": self.device_state.lacis_id,
            "cic": self.device_state.cic,
            "timestamp": timestamp,
            "state": {
                "status": self.device_state.status,
                "uptime_sec": hw_summary.get("uptime_sec", 0),
                "hardware": {
                    "memory_percent": hw_summary.get("memory", {}).get("percent_used", 0),
                    "cpu_load": hw_summary.get("cpu", {}).get("load_1min", 0),
                    "cpu_temp": hw_summary.get("thermal", {}).get("cpu", 0),
                },
                "device_specific": device_specific,
            },
        }

    def get_stats(self) -> Dict[str, Any]:
        """統計情報を取得"""
        return {
            "enabled": self.config.enabled,
            "endpoint": self.config.state_endpoint,
            "intervalSec": self.config.interval_sec,
            "totalReports": self.stats.total_reports,
            "successfulReports": self.stats.successful_reports,
            "failedReports": self.stats.failed_reports,
            "lastReportTime": self.stats.last_report_time,
            "lastSuccessTime": self.stats.last_success_time,
            "lastError": self.stats.last_error,
        }


# MQTT経由での状態報告ヘルパー
class MqttStateReporter:
    """
    MQTT経由の状態報告

    stateEndpointがない場合やMQTT優先の場合に使用
    """

    def __init__(
        self,
        mqtt_manager: Any,  # MqttManager
        hardware_info: Optional[HardwareInfo] = None,
    ):
        self.mqtt = mqtt_manager
        self.hardware_info = hardware_info or HardwareInfo()
        self._state_callback: Optional[Callable[[], Dict[str, Any]]] = None

        self.lacis_id: str = ""
        self.cic: str = ""
        self.status_topic: str = ""

    def begin(
        self,
        lacis_id: str,
        cic: str,
        status_topic: str,
    ) -> None:
        """初期化"""
        self.lacis_id = lacis_id
        self.cic = cic
        self.status_topic = status_topic

    def set_state_callback(self, callback: Callable[[], Dict[str, Any]]) -> None:
        """状態取得コールバックを設定"""
        self._state_callback = callback

    async def report_status(self, status: str = "running") -> bool:
        """MQTTで状態を報告"""
        if not self.mqtt.connected:
            return False

        hw_summary = self.hardware_info.get_summary()
        device_specific = {}
        if self._state_callback:
            try:
                device_specific = self._state_callback()
            except Exception:
                pass

        payload = {
            "lacisId": self.lacis_id,
            "cic": self.cic,
            "status": status,
            "uptime_sec": hw_summary.get("uptime_sec", 0),
            "hardware": {
                "memory_percent": hw_summary.get("memory", {}).get("percent_used", 0),
                "cpu_load": hw_summary.get("cpu", {}).get("load_1min", 0),
                "cpu_temp": hw_summary.get("thermal", {}).get("cpu", 0),
            },
            "device_specific": device_specific,
        }

        return await self.mqtt.publish(self.status_topic, payload)


__all__ = [
    "StateReporter",
    "StateReporterConfig",
    "ReportStats",
    "DeviceState",
    "MqttStateReporter",
]
