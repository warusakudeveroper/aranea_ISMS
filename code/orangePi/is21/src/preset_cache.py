"""
IS21 Preset Cache Manager
プリセット設定をメモリ上にキャッシュし、SDアクセスを最小化

設計原則:
- 1 IS21 対 複数 IS22 構成を考慮
- SDからの毎回読み込み禁止 → 起動時にメモリキャッシュ展開
- /opt/is21/data/presets/{lacis_id}.json で永続化
"""
import os
import json
import logging
from datetime import datetime
from typing import Dict, Optional, Any
from threading import RLock

logger = logging.getLogger("is21.preset_cache")

# プリセット保存ディレクトリ
PRESET_DIR = "/opt/is21/data/presets"

# デフォルトプリセット設定
DEFAULT_PRESET = {
    "preset_id": "balanced",
    "preset_version": "1.0.0",
    "detection_config": {
        "confidence_threshold": 0.33,
        "nms_threshold": 0.40,
        "enabled_events": ["person", "vehicle", "animal"],
        "severity_boost": {},
        "excluded_objects": [],
        "person_detection": {
            "par_enabled": True,
            "par_max_persons": 10,
            "par_threshold": 0.5
        }
    }
}


class PresetCache:
    """
    プリセットをメモリ上にキャッシュするシングルトンクラス

    Key: lacis_id (IS22のlacisID)
    Value: PresetData dict
    """
    _instance = None
    _initialized = False

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __init__(self):
        if PresetCache._initialized:
            return

        self._cache: Dict[str, Dict[str, Any]] = {}
        self._lock = RLock()
        self._load_all_presets()
        PresetCache._initialized = True
        logger.info(f"PresetCache initialized with {len(self._cache)} presets")

    def _ensure_dir(self) -> None:
        """プリセットディレクトリを確保"""
        os.makedirs(PRESET_DIR, exist_ok=True)

    def _load_all_presets(self) -> None:
        """起動時に全プリセットをメモリに読み込み"""
        self._ensure_dir()

        try:
            for filename in os.listdir(PRESET_DIR):
                if filename.endswith(".json"):
                    lacis_id = filename[:-5]  # Remove .json
                    filepath = os.path.join(PRESET_DIR, filename)
                    try:
                        with open(filepath, "r", encoding="utf-8") as f:
                            preset_data = json.load(f)
                            self._cache[lacis_id] = preset_data
                            logger.info(f"Loaded preset for lacis_id={lacis_id}")
                    except json.JSONDecodeError as e:
                        logger.error(f"Invalid JSON in {filepath}: {e}")
                    except Exception as e:
                        logger.error(f"Failed to load {filepath}: {e}")
        except FileNotFoundError:
            logger.info("No preset directory found, starting fresh")

    def get(self, lacis_id: str) -> Optional[Dict[str, Any]]:
        """
        lacis_idに対応するプリセットを取得

        Args:
            lacis_id: IS22のlacisID

        Returns:
            プリセットデータ、なければNone
        """
        with self._lock:
            return self._cache.get(lacis_id)

    def get_or_default(self, lacis_id: str) -> Dict[str, Any]:
        """
        プリセットを取得、なければデフォルトを返す

        Args:
            lacis_id: IS22のlacisID

        Returns:
            プリセットデータ（常に非None）
        """
        preset = self.get(lacis_id)
        if preset:
            return preset
        return DEFAULT_PRESET.copy()

    def save(
        self,
        lacis_id: str,
        preset_id: str,
        preset_version: str,
        detection_config: Dict[str, Any]
    ) -> Dict[str, Any]:
        """
        プリセットを保存（SDとメモリ両方）

        Args:
            lacis_id: IS22のlacisID
            preset_id: プリセットID (e.g., "parking_lot", "entrance")
            preset_version: バージョン (e.g., "1.2.0")
            detection_config: 検知設定

        Returns:
            保存されたプリセットデータ
        """
        self._ensure_dir()

        preset_data = {
            "lacis_id": lacis_id,
            "preset_id": preset_id,
            "preset_version": preset_version,
            "detection_config": detection_config,
            "updated_at": datetime.now().isoformat()
        }

        filepath = os.path.join(PRESET_DIR, f"{lacis_id}.json")

        with self._lock:
            # SD保存
            try:
                with open(filepath, "w", encoding="utf-8") as f:
                    json.dump(preset_data, f, ensure_ascii=False, indent=2)
                logger.info(f"Saved preset to {filepath}")
            except Exception as e:
                logger.error(f"Failed to save preset to {filepath}: {e}")
                raise

            # メモリキャッシュ更新
            self._cache[lacis_id] = preset_data

        return preset_data

    def delete(self, lacis_id: str) -> bool:
        """
        プリセットを削除

        Args:
            lacis_id: IS22のlacisID

        Returns:
            削除成功したらTrue
        """
        filepath = os.path.join(PRESET_DIR, f"{lacis_id}.json")

        with self._lock:
            # SD削除
            try:
                if os.path.exists(filepath):
                    os.remove(filepath)
                    logger.info(f"Deleted preset file {filepath}")
            except Exception as e:
                logger.error(f"Failed to delete {filepath}: {e}")
                return False

            # メモリキャッシュ削除
            if lacis_id in self._cache:
                del self._cache[lacis_id]

        return True

    def list_all(self) -> Dict[str, Dict[str, Any]]:
        """
        全プリセットを取得

        Returns:
            {lacis_id: preset_data, ...}
        """
        with self._lock:
            return self._cache.copy()

    def get_stats(self) -> Dict[str, Any]:
        """
        キャッシュ統計を取得
        """
        with self._lock:
            return {
                "total_presets": len(self._cache),
                "preset_ids": list(self._cache.keys()),
                "preset_versions": {
                    k: v.get("preset_version", "unknown")
                    for k, v in self._cache.items()
                }
            }


# シングルトンインスタンス
preset_cache = PresetCache()


def apply_preset_to_inference(
    preset: Dict[str, Any],
    conf_threshold: float,
    nms_threshold: float
) -> tuple:
    """
    プリセット設定を推論パラメータに適用

    Args:
        preset: プリセットデータ
        conf_threshold: デフォルト信頼度閾値
        nms_threshold: デフォルトNMS閾値

    Returns:
        (conf_threshold, nms_threshold, excluded_objects, severity_boost)
    """
    config = preset.get("detection_config", {})

    return (
        config.get("confidence_threshold", conf_threshold),
        config.get("nms_threshold", nms_threshold),
        config.get("excluded_objects", []),
        config.get("severity_boost", {})
    )
