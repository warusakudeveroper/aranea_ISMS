"""
ConfigManager - 設定管理（JSON永続化）
ESP32 SettingManagerのPython移植版
"""

import os
import json
import logging
from typing import Any, Optional
from pathlib import Path

logger = logging.getLogger(__name__)


class ConfigManager:
    """
    JSON永続化による設定管理
    - 純粋なCRUD操作のみ提供
    - デバイス固有のデフォルト値は各デバイスの設定に委譲
    """

    def __init__(self, config_dir: str = "/opt/is21/config"):
        self.config_dir = Path(config_dir)
        self.config_file = self.config_dir / "settings.json"
        self._data: dict = {}
        self._initialized = False

    def begin(self, namespace: str = "is21") -> bool:
        """
        初期化（設定ファイル読み込み）

        Args:
            namespace: 名前空間（ファイル名プレフィックス）

        Returns:
            成功時True
        """
        if self._initialized:
            return True

        self.config_file = self.config_dir / f"{namespace}_settings.json"

        try:
            self.config_dir.mkdir(parents=True, exist_ok=True)

            if self.config_file.exists():
                with open(self.config_file, "r", encoding="utf-8") as f:
                    self._data = json.load(f)
                logger.info(f"[CONFIG] Loaded settings from {self.config_file}")
            else:
                self._data = {}
                logger.info(f"[CONFIG] New settings file will be created: {self.config_file}")

            self._initialized = True
            return True
        except Exception as e:
            logger.error(f"[CONFIG] Failed to initialize: {e}")
            return False

    def _save(self) -> bool:
        """設定をファイルに保存"""
        if not self._initialized:
            return False
        try:
            with open(self.config_file, "w", encoding="utf-8") as f:
                json.dump(self._data, f, indent=2, ensure_ascii=False)
            return True
        except Exception as e:
            logger.error(f"[CONFIG] Failed to save: {e}")
            return False

    def get_string(self, key: str, default: str = "") -> str:
        """文字列設定を取得"""
        if not self._initialized:
            return default
        return str(self._data.get(key, default))

    def set_string(self, key: str, value: str) -> None:
        """文字列設定を保存"""
        if not self._initialized:
            return
        self._data[key] = value
        self._save()

    def get_int(self, key: str, default: int = 0) -> int:
        """整数設定を取得"""
        if not self._initialized:
            return default
        try:
            return int(self._data.get(key, default))
        except (ValueError, TypeError):
            return default

    def set_int(self, key: str, value: int) -> None:
        """整数設定を保存"""
        if not self._initialized:
            return
        self._data[key] = value
        self._save()

    def get_bool(self, key: str, default: bool = False) -> bool:
        """bool設定を取得"""
        if not self._initialized:
            return default
        val = self._data.get(key, default)
        if isinstance(val, bool):
            return val
        if isinstance(val, str):
            return val.lower() in ("true", "1", "yes")
        return bool(val)

    def set_bool(self, key: str, value: bool) -> None:
        """bool設定を保存"""
        if not self._initialized:
            return
        self._data[key] = value
        self._save()

    def get_dict(self, key: str, default: Optional[dict] = None) -> dict:
        """辞書設定を取得"""
        if not self._initialized:
            return default or {}
        val = self._data.get(key)
        if isinstance(val, dict):
            return val
        return default or {}

    def set_dict(self, key: str, value: dict) -> None:
        """辞書設定を保存"""
        if not self._initialized:
            return
        self._data[key] = value
        self._save()

    def has_key(self, key: str) -> bool:
        """設定が存在するか確認"""
        if not self._initialized:
            return False
        return key in self._data

    def remove(self, key: str) -> bool:
        """特定のキーを削除"""
        if not self._initialized:
            return False
        if key in self._data:
            del self._data[key]
            self._save()
            return True
        return False

    def clear(self) -> None:
        """全設定をクリア"""
        if not self._initialized:
            return
        self._data = {}
        self._save()
        logger.info("[CONFIG] Settings cleared")

    def get_all(self) -> dict:
        """全設定を取得（読み取り専用）"""
        return dict(self._data) if self._initialized else {}

    @property
    def is_initialized(self) -> bool:
        return self._initialized

    @property
    def config_path(self) -> str:
        return str(self.config_file)
