"""
aranea_common - araneaDevice共通モジュール for is21
ESP32 global/srcのPython移植版
"""

from .config import ConfigManager
from .lacis_id import LacisIdGenerator
from .aranea_register import AraneaRegister, TenantPrimaryAuth, RegisterResult
from .hardware_info import HardwareInfo
from .mqtt_manager import (
    MqttManager,
    MqttConfig,
    MqttStats,
    build_command_topic,
    build_status_topic,
    build_response_topic,
)
from .state_reporter import (
    StateReporter,
    StateReporterConfig,
    ReportStats,
    DeviceState,
    MqttStateReporter,
)
from .settings_schema import (
    IS21Settings,
    load_settings,
    validate_settings,
    settings_to_dict,
    get_default_settings,
)

__all__ = [
    # 基盤モジュール
    "ConfigManager",
    "LacisIdGenerator",
    "AraneaRegister",
    "TenantPrimaryAuth",
    "RegisterResult",
    "HardwareInfo",
    # 通信モジュール
    "MqttManager",
    "MqttConfig",
    "MqttStats",
    "build_command_topic",
    "build_status_topic",
    "build_response_topic",
    # 状態報告
    "StateReporter",
    "StateReporterConfig",
    "ReportStats",
    "DeviceState",
    "MqttStateReporter",
    # 設定スキーマ
    "IS21Settings",
    "load_settings",
    "validate_settings",
    "settings_to_dict",
    "get_default_settings",
]
