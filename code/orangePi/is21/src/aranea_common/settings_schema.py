"""
IS21設定スキーマ定義・バリデーション

設定ファイル（is21_settings.json）の型定義と検証機能を提供
"""

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
import json
import logging

logger = logging.getLogger(__name__)


@dataclass
class DeviceSettings:
    """デバイス設定"""
    name: str = "IS21-Inference-001"
    location: str = ""
    description: str = "Paraclate Inference Server"


@dataclass
class WiFiSettings:
    """WiFi設定"""
    enabled: bool = False
    ssid: str = ""
    password: str = ""


@dataclass
class NtpSettings:
    """NTP設定"""
    enabled: bool = True
    servers: List[str] = field(default_factory=lambda: ["ntp.nict.jp", "time.google.com"])


@dataclass
class NetworkSettings:
    """ネットワーク設定"""
    wifi: WiFiSettings = field(default_factory=WiFiSettings)
    ntp: NtpSettings = field(default_factory=NtpSettings)


@dataclass
class ModelSettings:
    """モデル設定"""
    default: str = "yolov8n.rknn"
    detection_threshold: float = 0.5
    nms_threshold: float = 0.4
    max_detections: int = 20


@dataclass
class ParSettings:
    """PAR（人物属性認識）設定"""
    enabled: bool = True
    model: str = "par_model.rknn"
    min_person_height: int = 50


@dataclass
class InferenceSettings:
    """推論設定"""
    max_concurrent: int = 2
    timeout_sec: int = 30
    batch_size: int = 1
    priority_mode: str = "fifo"  # fifo, lifo, priority
    model: ModelSettings = field(default_factory=ModelSettings)
    par: ParSettings = field(default_factory=ParSettings)


@dataclass
class ApiSettings:
    """API設定"""
    host: str = "0.0.0.0"
    port: int = 9000
    cors_origins: List[str] = field(default_factory=lambda: ["*"])
    docs_enabled: bool = True


@dataclass
class MqttSettings:
    """MQTT設定"""
    enabled: bool = False
    broker_url: str = ""
    base_topic: str = "aranea"
    username: str = ""
    password: str = ""
    use_tls: bool = True
    keepalive_sec: int = 60
    reconnect_interval_sec: int = 30


@dataclass
class StateReportSettings:
    """状態報告設定"""
    enabled: bool = True
    interval_sec: int = 300
    retry_count: int = 3


@dataclass
class ParaclateSettings:
    """Paraclate連携設定"""
    enabled: bool = True
    endpoint: str = ""
    report_interval_minutes: int = 15
    grand_summary_times: List[str] = field(default_factory=lambda: ["09:00", "17:00", "21:00"])


@dataclass
class TenantSettings:
    """テナント設定"""
    tid: str = ""
    primary_lacis_id: str = ""
    primary_user_id: str = ""
    primary_cic: str = ""


@dataclass
class LoggingSettings:
    """ログ設定"""
    level: str = "INFO"
    file_enabled: bool = True
    file_path: str = "/opt/is21/logs/is21.log"
    max_size_mb: int = 100
    backup_count: int = 5


@dataclass
class IS21Settings:
    """IS21全体設定"""
    device: DeviceSettings = field(default_factory=DeviceSettings)
    network: NetworkSettings = field(default_factory=NetworkSettings)
    inference: InferenceSettings = field(default_factory=InferenceSettings)
    api: ApiSettings = field(default_factory=ApiSettings)
    mqtt: MqttSettings = field(default_factory=MqttSettings)
    state_report: StateReportSettings = field(default_factory=StateReportSettings)
    paraclate: ParaclateSettings = field(default_factory=ParaclateSettings)
    tenant: TenantSettings = field(default_factory=TenantSettings)
    logging: LoggingSettings = field(default_factory=LoggingSettings)


def _dataclass_from_dict(klass, d: dict):
    """辞書からdataclassインスタンスを生成（ネスト対応）"""
    if d is None:
        return klass()

    fieldtypes = {f.name: f.type for f in klass.__dataclass_fields__.values()}
    field_defaults = {f.name: f.default_factory() if f.default_factory is not field else f.default
                      for f in klass.__dataclass_fields__.values()}

    kwargs = {}
    for key, val in d.items():
        if key in fieldtypes:
            ftype = fieldtypes[key]
            # ネストされたdataclass
            if hasattr(ftype, '__dataclass_fields__'):
                kwargs[key] = _dataclass_from_dict(ftype, val if isinstance(val, dict) else {})
            else:
                kwargs[key] = val

    return klass(**kwargs)


def load_settings(data: Dict[str, Any]) -> IS21Settings:
    """
    辞書からIS21Settingsを生成

    Args:
        data: 設定辞書

    Returns:
        IS21Settings インスタンス
    """
    settings = IS21Settings()

    if "device" in data:
        settings.device = _dataclass_from_dict(DeviceSettings, data["device"])

    if "network" in data:
        net = data["network"]
        settings.network = NetworkSettings(
            wifi=_dataclass_from_dict(WiFiSettings, net.get("wifi", {})),
            ntp=_dataclass_from_dict(NtpSettings, net.get("ntp", {})),
        )

    if "inference" in data:
        inf = data["inference"]
        settings.inference = InferenceSettings(
            max_concurrent=inf.get("max_concurrent", 2),
            timeout_sec=inf.get("timeout_sec", 30),
            batch_size=inf.get("batch_size", 1),
            priority_mode=inf.get("priority_mode", "fifo"),
            model=_dataclass_from_dict(ModelSettings, inf.get("model", {})),
            par=_dataclass_from_dict(ParSettings, inf.get("par", {})),
        )

    if "api" in data:
        settings.api = _dataclass_from_dict(ApiSettings, data["api"])

    if "mqtt" in data:
        settings.mqtt = _dataclass_from_dict(MqttSettings, data["mqtt"])

    if "state_report" in data:
        settings.state_report = _dataclass_from_dict(StateReportSettings, data["state_report"])

    if "paraclate" in data:
        settings.paraclate = _dataclass_from_dict(ParaclateSettings, data["paraclate"])

    if "tenant" in data:
        settings.tenant = _dataclass_from_dict(TenantSettings, data["tenant"])

    if "logging" in data:
        settings.logging = _dataclass_from_dict(LoggingSettings, data["logging"])

    return settings


def validate_settings(settings: IS21Settings) -> List[str]:
    """
    設定を検証

    Args:
        settings: IS21Settings インスタンス

    Returns:
        エラーメッセージのリスト（空なら有効）
    """
    errors = []

    # API設定
    if settings.api.port < 1 or settings.api.port > 65535:
        errors.append(f"Invalid API port: {settings.api.port}")

    # 推論設定
    if settings.inference.max_concurrent < 1:
        errors.append("max_concurrent must be at least 1")
    if settings.inference.timeout_sec < 1:
        errors.append("timeout_sec must be at least 1")
    if not 0 <= settings.inference.model.detection_threshold <= 1:
        errors.append("detection_threshold must be between 0 and 1")

    # MQTT設定
    if settings.mqtt.enabled and not settings.mqtt.broker_url:
        errors.append("MQTT enabled but broker_url not set")

    # 状態報告
    if settings.state_report.enabled and settings.state_report.interval_sec < 10:
        errors.append("state_report interval must be at least 10 seconds")

    # ログレベル
    valid_levels = ["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"]
    if settings.logging.level.upper() not in valid_levels:
        errors.append(f"Invalid log level: {settings.logging.level}")

    return errors


def settings_to_dict(settings: IS21Settings) -> Dict[str, Any]:
    """
    IS21Settingsを辞書に変換

    Args:
        settings: IS21Settings インスタンス

    Returns:
        設定辞書
    """
    import dataclasses

    def _to_dict(obj):
        if dataclasses.is_dataclass(obj):
            return {k: _to_dict(v) for k, v in dataclasses.asdict(obj).items()}
        elif isinstance(obj, list):
            return [_to_dict(v) for v in obj]
        else:
            return obj

    return _to_dict(settings)


def get_default_settings() -> IS21Settings:
    """デフォルト設定を取得"""
    return IS21Settings()


def get_default_settings_json() -> str:
    """デフォルト設定をJSON文字列で取得"""
    return json.dumps(settings_to_dict(get_default_settings()), indent=2, ensure_ascii=False)


__all__ = [
    "IS21Settings",
    "DeviceSettings",
    "NetworkSettings",
    "WiFiSettings",
    "NtpSettings",
    "InferenceSettings",
    "ModelSettings",
    "ParSettings",
    "ApiSettings",
    "MqttSettings",
    "StateReportSettings",
    "ParaclateSettings",
    "TenantSettings",
    "LoggingSettings",
    "load_settings",
    "validate_settings",
    "settings_to_dict",
    "get_default_settings",
    "get_default_settings_json",
]
