"""
Domain to Service Mapping
ドメインパターンからサービス名・カテゴリを判定

Features:
- デフォルト辞書: data/default_domain_services.json（コード管理）
- 学習データ: /var/lib/is20s/domain_services.json（デバイス上）
- 起動時にメモリロード、変更時のみファイル書き込み（SDカード保護）
- UnknownDomainsCache: 完全オンメモリ（永続化なし、API経由で取得）
"""
import json
import logging
from collections import OrderedDict
from datetime import datetime
from pathlib import Path
from threading import Lock
from typing import Any, Dict, List, Optional, Tuple

logger = logging.getLogger(__name__)

# パス設定
APP_DIR = Path(__file__).parent
DEFAULT_SERVICES_FILE = APP_DIR.parent / "data" / "default_domain_services.json"
DATA_DIR = Path("/var/lib/is20s")
DOMAIN_SERVICES_FILE = DATA_DIR / "domain_services.json"


class UnknownDomainsCache:
    """
    未検出ドメインのFIFOキャッシュ

    特徴:
    - 最大300件保持（超えたら古いものから削除）
    - ドメインごとに出現回数・最終検出時刻を記録
    - API経由で未検出一覧を取得可能
    """

    MAX_SIZE = 300

    def __init__(self):
        self._lock = Lock()
        # OrderedDict: domain -> {count, first_seen, last_seen, sample_ips}
        self._cache: OrderedDict[str, Dict[str, Any]] = OrderedDict()

    def add(self, domain: str, src_ip: str = None, room_no: str = None) -> None:
        """未検出ドメインを追加"""
        if not domain:
            return

        now = datetime.now().isoformat()

        with self._lock:
            if domain in self._cache:
                # 既存: カウント増加、最終検出時刻更新
                entry = self._cache[domain]
                entry["count"] += 1
                entry["last_seen"] = now
                if src_ip and src_ip not in entry["sample_ips"]:
                    if len(entry["sample_ips"]) < 5:
                        entry["sample_ips"].append(src_ip)
                if room_no and room_no not in entry["rooms"]:
                    entry["rooms"].append(room_no)
                # 最近使用したものを末尾に移動
                self._cache.move_to_end(domain)
            else:
                # 新規追加
                self._cache[domain] = {
                    "count": 1,
                    "first_seen": now,
                    "last_seen": now,
                    "sample_ips": [src_ip] if src_ip else [],
                    "rooms": [room_no] if room_no else [],
                }
                # 最大サイズ超過時は古いものを削除
                while len(self._cache) > self.MAX_SIZE:
                    self._cache.popitem(last=False)

    def get_all(self) -> List[Dict[str, Any]]:
        """全未検出ドメインを取得（カウント降順）"""
        with self._lock:
            result = []
            for domain, info in self._cache.items():
                result.append({
                    "domain": domain,
                    "count": info["count"],
                    "first_seen": info["first_seen"],
                    "last_seen": info["last_seen"],
                    "sample_ips": info["sample_ips"],
                    "rooms": info["rooms"],
                })
            # カウント降順でソート
            result.sort(key=lambda x: -x["count"])
            return result

    def get_count(self) -> int:
        """未検出ドメイン数を取得"""
        with self._lock:
            return len(self._cache)

    def get_total_hits(self) -> int:
        """総ヒット数を取得"""
        with self._lock:
            return sum(info["count"] for info in self._cache.values())

    def clear(self) -> None:
        """キャッシュをクリア"""
        with self._lock:
            self._cache.clear()

    def remove(self, domain: str) -> bool:
        """特定のドメインを削除"""
        with self._lock:
            if domain in self._cache:
                del self._cache[domain]
                return True
            return False


# 未検出キャッシュシングルトン
_unknown_cache: Optional[UnknownDomainsCache] = None


def get_unknown_cache() -> UnknownDomainsCache:
    """UnknownDomainsCacheシングルトンを取得"""
    global _unknown_cache
    if _unknown_cache is None:
        _unknown_cache = UnknownDomainsCache()
    return _unknown_cache


def record_unknown_domain(domain: str, src_ip: str = None, room_no: str = None) -> None:
    """未検出ドメインを記録（便利関数）"""
    get_unknown_cache().add(domain, src_ip, room_no)


def _load_default_services() -> Dict[str, Dict[str, str]]:
    """デフォルト辞書をJSONファイルから読み込み"""
    try:
        if DEFAULT_SERVICES_FILE.exists():
            with DEFAULT_SERVICES_FILE.open(encoding="utf-8") as f:
                data = json.load(f)
                services = data.get("services", {})
                logger.info("Loaded %d default services from %s", len(services), DEFAULT_SERVICES_FILE)
                return services
        else:
            logger.warning("Default services file not found: %s", DEFAULT_SERVICES_FILE)
            return {}
    except Exception as e:
        logger.error("Failed to load default services: %s", e)
        return {}

class DomainServiceManager:
    """
    ドメイン→サービスマッピング管理

    特徴:
    - メモリ上で検索（高速）
    - LRUキャッシュで繰り返しルックアップを高速化
    - 変更時のみファイル書き込み（SDカード保護）
    - スレッドセーフ
    """

    CACHE_MAX_SIZE = 10000  # LRUキャッシュ最大エントリ数

    def __init__(self, data_file: Path = DOMAIN_SERVICES_FILE):
        self.data_file = data_file
        self._lock = Lock()
        self._data: Dict[str, Dict[str, str]] = {}
        self._dirty = False
        # LRUキャッシュ: domain -> (service, category)
        self._cache: OrderedDict[str, Tuple[Optional[str], Optional[str]]] = OrderedDict()
        self._cache_hits = 0
        self._cache_misses = 0
        self._load()

    def _load(self) -> None:
        """JSONファイルからロード"""
        with self._lock:
            if self.data_file.exists():
                try:
                    with self.data_file.open() as f:
                        loaded = json.load(f)
                        # バージョン2: services直下にマップ
                        if isinstance(loaded, dict) and "services" in loaded:
                            self._data = loaded["services"]
                        else:
                            # 旧形式互換
                            self._data = loaded if isinstance(loaded, dict) else {}
                    logger.info("Loaded %d domain services from file", len(self._data))
                except Exception as e:
                    logger.warning("Failed to load domain services: %s", e)
                    self._data = {}

            # 空の場合はデフォルトJSONから読み込み
            if not self._data:
                self._data = _load_default_services()
                if self._data:
                    self._dirty = True
                    self._save_internal()
                    logger.info("Initialized with %d default domain services", len(self._data))
                else:
                    logger.error("No default services available")

    def _save_internal(self) -> bool:
        """内部保存（ロック取得済み前提）"""
        if not self._dirty:
            return True
        try:
            self.data_file.parent.mkdir(parents=True, exist_ok=True)
            data = {
                "version": 2,
                "services": self._data,
            }
            with self.data_file.open("w") as f:
                json.dump(data, f, ensure_ascii=False, indent=2)
            self._dirty = False
            logger.info("Saved %d domain services", len(self._data))
            return True
        except Exception as e:
            logger.error("Failed to save domain services: %s", e)
            return False

    def save(self) -> bool:
        """明示的に保存"""
        with self._lock:
            return self._save_internal()

    def get_service(self, domain: str) -> Tuple[Optional[str], Optional[str]]:
        """
        ドメインからサービス名・カテゴリを取得（LRUキャッシュ対応）

        後方互換性のため (service, category) を返す。
        roleも必要な場合は get_service_full() を使用。
        """
        result = self.get_service_full(domain)
        return (result[0], result[1])

    def get_service_full(self, domain: str) -> Tuple[Optional[str], Optional[str], Optional[str]]:
        """
        ドメインからサービス名・カテゴリ・ロールを取得（LRUキャッシュ対応）

        Args:
            domain: ドメイン名（例: "video.google.com"）

        Returns:
            (service_name, category, role) or (None, None, None)
            role: "primary" | "auxiliary" | None (未設定の場合はprimary扱い)
        """
        if not domain:
            return (None, None, None)

        domain_lower = domain.lower()

        with self._lock:
            # キャッシュヒットチェック
            if domain_lower in self._cache:
                self._cache_hits += 1
                # LRU: 末尾に移動
                self._cache.move_to_end(domain_lower)
                return self._cache[domain_lower]

            self._cache_misses += 1

            # ドット区切りを考慮したマッチング
            result: Tuple[Optional[str], Optional[str], Optional[str]] = (None, None, None)
            for pattern, info in self._data.items():
                if "." in pattern:
                    # パターンにドットがある場合は厳密マッチ
                    # prefix対応: www.gstatic → www.gstatic.com にマッチ
                    if (
                        domain_lower == pattern
                        or domain_lower.endswith("." + pattern)
                        or ("." + pattern + ".") in domain_lower
                        or domain_lower.startswith(pattern + ".")
                    ):
                        result = (info.get("service"), info.get("category"), info.get("role"))
                        break
                else:
                    # パターンにドットがない場合は部分マッチ
                    if pattern in domain_lower:
                        result = (info.get("service"), info.get("category"), info.get("role"))
                        break

            # キャッシュに保存
            self._cache[domain_lower] = result
            # 最大サイズ超過時は古いものを削除
            while len(self._cache) > self.CACHE_MAX_SIZE:
                self._cache.popitem(last=False)

            return result

    def _invalidate_cache(self) -> None:
        """キャッシュを無効化（データ変更時に呼び出し）"""
        self._cache.clear()
        logger.debug("Service lookup cache invalidated")

    def add(self, pattern: str, service: str, category: str = "") -> Dict[str, Any]:
        """サービスマッピングを追加"""
        if not pattern or not service:
            return {"ok": False, "error": "Pattern and service required"}

        pattern_lower = pattern.lower()
        with self._lock:
            self._data[pattern_lower] = {"service": service, "category": category}
            self._dirty = True
            self._invalidate_cache()
            self._save_internal()

        return {"ok": True, "message": f"Added: {pattern} -> {service}"}

    def update(
        self, pattern: str, service: str = None, category: str = None
    ) -> Dict[str, Any]:
        """サービスマッピングを更新"""
        pattern_lower = pattern.lower()
        with self._lock:
            if pattern_lower not in self._data:
                return {"ok": False, "error": f"Pattern not found: {pattern}"}

            if service is not None:
                self._data[pattern_lower]["service"] = service
            if category is not None:
                self._data[pattern_lower]["category"] = category

            self._dirty = True
            self._invalidate_cache()
            self._save_internal()

        return {"ok": True, "message": f"Updated: {pattern}"}

    def delete(self, pattern: str) -> Dict[str, Any]:
        """サービスマッピングを削除"""
        pattern_lower = pattern.lower()
        with self._lock:
            if pattern_lower not in self._data:
                return {"ok": False, "error": f"Pattern not found: {pattern}"}

            del self._data[pattern_lower]
            self._dirty = True
            self._invalidate_cache()
            self._save_internal()

        return {"ok": True, "message": f"Deleted: {pattern}"}

    def get_all(self) -> Dict[str, Dict[str, str]]:
        """全マッピングを取得"""
        with self._lock:
            return self._data.copy()

    def get_count(self) -> int:
        """登録数を取得"""
        with self._lock:
            return len(self._data)

    def search(self, query: str) -> List[Dict[str, Any]]:
        """パターン・サービス名で検索"""
        query_lower = query.lower()
        results = []
        with self._lock:
            for pattern, info in self._data.items():
                if (
                    query_lower in pattern
                    or query_lower in info.get("service", "").lower()
                    or query_lower in info.get("category", "").lower()
                ):
                    results.append(
                        {
                            "pattern": pattern,
                            "service": info.get("service"),
                            "category": info.get("category"),
                        }
                    )
        return results

    def export_data(self) -> Dict[str, Any]:
        """エクスポート用データを取得"""
        with self._lock:
            return {
                "version": 2,
                "services": self._data.copy(),
            }

    def import_data(self, data: Dict[str, Any], merge: bool = True) -> Dict[str, Any]:
        """
        データをインポート

        Args:
            data: インポートデータ
            merge: Trueなら既存とマージ、Falseなら上書き

        Returns:
            結果と統計
        """
        stats = {"added": 0, "updated": 0, "skipped": 0}

        if "services" not in data:
            return {"ok": False, "error": "Invalid data format", "stats": stats}

        with self._lock:
            if not merge:
                self._data = {}

            for pattern, info in data["services"].items():
                if not isinstance(info, dict):
                    stats["skipped"] += 1
                    continue

                pattern_lower = pattern.lower()
                if pattern_lower in self._data:
                    # 既存エントリの更新
                    self._data[pattern_lower].update(info)
                    stats["updated"] += 1
                else:
                    # 新規追加
                    self._data[pattern_lower] = info
                    stats["added"] += 1

            self._dirty = True
            self._save_internal()

        return {"ok": True, "stats": stats}


# シングルトンインスタンス
_domain_service_manager: Optional[DomainServiceManager] = None


def get_domain_service_manager() -> DomainServiceManager:
    """DomainServiceManagerシングルトンを取得"""
    global _domain_service_manager
    if _domain_service_manager is None:
        _domain_service_manager = DomainServiceManager()
    return _domain_service_manager


def get_service_by_domain(domain: str) -> Tuple[Optional[str], Optional[str]]:
    """
    ドメインからサービス情報を取得（便利関数）

    Returns: (service_name, category) or (None, None)
    """
    return get_domain_service_manager().get_service(domain)


def get_service_by_domain_full(domain: str) -> Tuple[Optional[str], Optional[str], Optional[str]]:
    """
    ドメインからサービス情報をフル取得（便利関数）

    Returns: (service_name, category, role) or (None, None, None)
    role: "primary" | "auxiliary" | None
    """
    return get_domain_service_manager().get_service_full(domain)


__all__ = [
    "DomainServiceManager",
    "get_domain_service_manager",
    "get_service_by_domain",
    "get_service_by_domain_full",
    "DOMAIN_SERVICES_FILE",
    "UnknownDomainsCache",
    "get_unknown_cache",
    "record_unknown_domain",
]
