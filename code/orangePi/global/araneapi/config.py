from dataclasses import dataclass, field, is_dataclass
from pathlib import Path
from typing import Any, Dict, Optional
import os
import yaml


def _merge_dataclass(instance: Any, updates: Dict[str, Any]) -> None:
    """dict を dataclass インスタンスへ反映（ネスト対応）。"""
    for key, value in updates.items():
        if not hasattr(instance, key):
            continue
        current = getattr(instance, key)
        if is_dataclass(current) and isinstance(value, dict):
            _merge_dataclass(current, value)
        else:
            setattr(instance, key, value)


def _as_bool(value: str) -> bool:
    return str(value).lower() in ("1", "true", "yes", "on")


@dataclass
class DeviceConfig:
    name: str = "is20s"
    device_type: str = "is20s"
    product_type: str = "020"
    product_code: str = "0096"
    data_dir: str = "/var/lib/is20s"
    log_dir: str = "/var/log/is20s"
    interface: str = "end0"  # MAC取得優先インターフェース


@dataclass
class RelayConfig:
    primary_url: str = ""
    secondary_url: str = ""
    timeout_sec: int = 8
    max_retry: int = 3
    max_queue: int = 5000
    backoff_base_sec: int = 2
    backoff_max_sec: int = 60


@dataclass
class MqttConfig:
    host: str = ""
    port: int = 8883
    tls: bool = True
    user: str = ""
    password: str = ""
    base_topic: str = "aranea"
    client_id: Optional[str] = None


@dataclass
class RegisterConfig:
    gate_url: str = "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate"
    tid: str = ""
    tenant_lacisid: str = ""
    tenant_email: str = ""
    tenant_pass: str = ""
    tenant_cic: str = ""
    enabled: bool = True


@dataclass
class UiConfig:
    http_port: int = 8080


@dataclass
class StoreConfig:
    sqlite_path: str = "/var/lib/is20s/events.sqlite"
    ring_size: int = 2000
    max_db_size: int = 20000
    flush_interval_sec: int = 10
    flush_batch: int = 50


@dataclass
class AccessConfig:
    allowed_sources: list[str] = field(default_factory=list)  # IP or CIDR、空なら無制限


@dataclass
class AppConfig:
    device: DeviceConfig = field(default_factory=DeviceConfig)
    relay: RelayConfig = field(default_factory=RelayConfig)
    mqtt: MqttConfig = field(default_factory=MqttConfig)
    register: RegisterConfig = field(default_factory=RegisterConfig)
    ui: UiConfig = field(default_factory=UiConfig)
    store: StoreConfig = field(default_factory=StoreConfig)
    access: AccessConfig = field(default_factory=AccessConfig)


def _load_yaml(path: Path) -> Dict[str, Any]:
    if not path.exists():
        return {}
    with path.open("r") as f:
        return yaml.safe_load(f) or {}


def _apply_env_overrides(cfg: AppConfig, env: Dict[str, str]) -> None:
    mapping = {
        "ARANEA_TID": ("register", "tid"),
        "ARANEA_GATE_URL": ("register", "gate_url"),
        "ARANEA_TENANT_LACISID": ("register", "tenant_lacisid"),
        "ARANEA_TENANT_EMAIL": ("register", "tenant_email"),
        "ARANEA_TENANT_CIC": ("register", "tenant_cic"),
        "ARANEA_RELAY_PRIMARY": ("relay", "primary_url"),
        "ARANEA_RELAY_SECONDARY": ("relay", "secondary_url"),
        "ARANEA_RELAY_MAX_QUEUE": ("relay", "max_queue"),
        "ARANEA_RELAY_BACKOFF_BASE": ("relay", "backoff_base_sec"),
        "ARANEA_RELAY_BACKOFF_MAX": ("relay", "backoff_max_sec"),
        "ARANEA_MQTT_HOST": ("mqtt", "host"),
        "ARANEA_MQTT_PORT": ("mqtt", "port"),
        "ARANEA_MQTT_USER": ("mqtt", "user"),
        "ARANEA_MQTT_PASS": ("mqtt", "password"),
        "ARANEA_MQTT_TLS": ("mqtt", "tls"),
        "ARANEA_HTTP_PORT": ("ui", "http_port"),
        "ARANEA_DATA_DIR": ("device", "data_dir"),
        "ARANEA_LOG_DIR": ("device", "log_dir"),
        "ARANEA_DEVICE_TYPE": ("device", "device_type"),
        "ARANEA_ALLOWED_SOURCES": ("access", "allowed_sources"),
    }
    for env_key, path in mapping.items():
        if env_key not in env:
            continue
        target = cfg
        for key in path[:-1]:
            target = getattr(target, key)
        leaf = path[-1]
        value = env[env_key]
        if isinstance(getattr(target, leaf), bool):
            setattr(target, leaf, _as_bool(value))
        elif isinstance(getattr(target, leaf), int):
            setattr(target, leaf, int(value))
        elif isinstance(getattr(target, leaf), list):
            items = [v.strip() for v in str(value).split(",") if v.strip()]
            setattr(target, leaf, items)
        else:
            setattr(target, leaf, value)


def load_config(path: Optional[str] = None, env: Dict[str, str] = os.environ) -> AppConfig:
    """
    設定をロードして AppConfig を返す。
    優先順位: 引数 path > IS20S_CONFIG/ARANEA_CONFIG 環境変数 > デフォルト値。
    """
    cfg = AppConfig()
    cfg_path = path or env.get("IS20S_CONFIG") or env.get("ARANEA_CONFIG")
    if cfg_path:
        loaded = _load_yaml(Path(cfg_path))
        _merge_dataclass(cfg, loaded)

    # ENV上書き（最優先）
    _apply_env_overrides(cfg, env)

    # 既定ディレクトリ作成
    Path(cfg.device.data_dir).mkdir(parents=True, exist_ok=True)
    Path(cfg.device.log_dir).mkdir(parents=True, exist_ok=True)
    Path(cfg.store.sqlite_path).parent.mkdir(parents=True, exist_ok=True)

    # data_dir 配下の override も読む（UI からの変更保存先）
    override_path = Path(cfg.device.data_dir) / "config_override.yaml"
    if override_path.exists():
        override = _load_yaml(override_path)
        _merge_dataclass(cfg, override)
        # overrideでdata_dir/log_dirが変わった場合に備え再作成
        Path(cfg.device.data_dir).mkdir(parents=True, exist_ok=True)
        Path(cfg.device.log_dir).mkdir(parents=True, exist_ok=True)
        Path(cfg.store.sqlite_path).parent.mkdir(parents=True, exist_ok=True)

    # ENVをもう一度適用して最優先にする
    _apply_env_overrides(cfg, env)
    return cfg


def update_config_from_dict(cfg: AppConfig, updates: Dict[str, Any]) -> None:
    _merge_dataclass(cfg, updates)


def save_config(cfg: AppConfig, path: Path) -> None:
    from dataclasses import asdict

    path.parent.mkdir(parents=True, exist_ok=True)
    data = asdict(cfg)
    with path.open("w") as f:
        yaml.safe_dump(data, f, sort_keys=False, allow_unicode=True)


__all__ = [
    "AppConfig",
    "DeviceConfig",
    "RelayConfig",
    "MqttConfig",
    "RegisterConfig",
    "UiConfig",
    "StoreConfig",
    "AccessConfig",
    "load_config",
    "update_config_from_dict",
    "save_config",
]
