from araneapi.config import AppConfig, load_config


def load_is20s_config(path: str | None = None) -> AppConfig:
    cfg = load_config(path)
    # is20s 固有のデフォルトを再確認
    cfg.device.name = cfg.device.name or "is20s"
    if not cfg.device.product_type:
        cfg.device.product_type = "020"
    if not cfg.device.product_code:
        cfg.device.product_code = "0096"
    return cfg


__all__ = ["load_is20s_config", "AppConfig"]
