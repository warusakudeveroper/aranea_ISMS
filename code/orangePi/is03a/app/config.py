from araneapi.config import AppConfig, load_config


def load_is03a_config(path: str | None = None) -> AppConfig:
    cfg = load_config(path)
    cfg.device.name = cfg.device.name or "is03a"
    cfg.device.device_type = cfg.device.device_type or "is03a"
    cfg.device.product_type = cfg.device.product_type or "003"
    cfg.device.product_code = cfg.device.product_code or "0096"
    return cfg


__all__ = ["load_is03a_config", "AppConfig"]
