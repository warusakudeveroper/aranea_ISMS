"""
ASN Lookup Module
Team Cymru DNS を使用したIPからASN検索

Usage:
    from .asn_lookup import ASNLookup
    lookup = ASNLookup()
    asn, org = await lookup.lookup("8.8.8.8")  # Returns (15169, "GOOGLE")
"""

import asyncio
import logging
import socket
from typing import Dict, Optional, Tuple
from datetime import datetime, timedelta
import json
from pathlib import Path

from collections import OrderedDict

from .asn_services import get_service_by_asn, ASN_SERVICE_MAP

logger = logging.getLogger(__name__)

# キャッシュファイル
ASN_CACHE_FILE = Path("/var/lib/is20s/asn_cache.json")
CACHE_TTL_HOURS = 24 * 7  # 1週間キャッシュ
MAX_CACHE_SIZE = 50000  # 最大キャッシュエントリ数（メモリリーク防止）


class ASNLookup:
    """IP→ASN検索（Team Cymru DNS + ローカルキャッシュ）"""

    def __init__(self):
        self.cache: OrderedDict[str, Tuple[int, str, float]] = OrderedDict()  # ip -> (asn, org, timestamp) LRU管理
        self._load_cache()
        self.stats = {"hits": 0, "misses": 0, "errors": 0}

    def _load_cache(self):
        """キャッシュファイルを読み込み（LRU、上限付き）"""
        if ASN_CACHE_FILE.exists():
            try:
                data = json.loads(ASN_CACHE_FILE.read_text())
                now = datetime.now().timestamp()
                ttl_sec = CACHE_TTL_HOURS * 3600
                # 期限切れでないエントリのみ読み込み、タイムスタンプ順でソート
                valid_entries = []
                for ip, entry in data.items():
                    if now - entry[2] < ttl_sec:
                        valid_entries.append((ip, tuple(entry)))
                # タイムスタンプ順（古い順）でソートしてOrderedDictに追加
                valid_entries.sort(key=lambda x: x[1][2])
                for ip, entry in valid_entries[-MAX_CACHE_SIZE:]:  # 最新のMAX_CACHE_SIZE件のみ
                    self.cache[ip] = entry
                logger.info("Loaded %d ASN cache entries (max: %d)", len(self.cache), MAX_CACHE_SIZE)
            except Exception as e:
                logger.warning("Failed to load ASN cache: %s", e)

    def _save_cache(self):
        """キャッシュをファイルに保存"""
        try:
            ASN_CACHE_FILE.parent.mkdir(parents=True, exist_ok=True)
            ASN_CACHE_FILE.write_text(json.dumps(self.cache))
        except Exception as e:
            logger.warning("Failed to save ASN cache: %s", e)

    def _reverse_ip(self, ip: str) -> str:
        """IPアドレスを逆順に（DNS PTR形式）"""
        parts = ip.split(".")
        return ".".join(reversed(parts))

    async def _dns_lookup(self, ip: str) -> Optional[Tuple[int, str]]:
        """
        Team Cymru DNS を使用してASN検索
        Query: [reversed-ip].origin.asn.cymru.com
        Response: "ASN | IP | BGP Prefix | CC | Registry"
        """
        try:
            reversed_ip = self._reverse_ip(ip)
            query = f"{reversed_ip}.origin.asn.cymru.com"

            # 非同期DNS検索
            loop = asyncio.get_event_loop()
            answers = await loop.run_in_executor(
                None, lambda: socket.gethostbyname_ex(query)
            )
            # TXT recordを取得するためにnslookupを使う
            # 実際にはgetaddrinfo では TXT を取れないので別の方法が必要

        except Exception as e:
            logger.debug("DNS lookup failed for %s: %s", ip, e)

        # フォールバック: dnspython または直接ソケット
        try:
            import subprocess
            result = subprocess.run(
                ["dig", "+short", f"{self._reverse_ip(ip)}.origin.asn.cymru.com", "TXT"],
                capture_output=True, text=True, timeout=5
            )
            if result.returncode == 0 and result.stdout.strip():
                # "15169 | 8.8.8.0/24 | US | arin | 1992-12-01"
                line = result.stdout.strip().strip('"')
                parts = line.split("|")
                if len(parts) >= 1:
                    asn = int(parts[0].strip())
                    # ASN名を取得
                    org = await self._get_asn_name(asn)
                    return (asn, org)
        except Exception as e:
            logger.debug("dig lookup failed for %s: %s", ip, e)

        return None

    async def _get_asn_name(self, asn: int) -> str:
        """ASN番号から組織名を取得"""
        try:
            import subprocess
            result = subprocess.run(
                ["dig", "+short", f"AS{asn}.asn.cymru.com", "TXT"],
                capture_output=True, text=True, timeout=5
            )
            if result.returncode == 0 and result.stdout.strip():
                # "15169 | US | arin | 2000-03-30 | GOOGLE"
                line = result.stdout.strip().strip('"')
                parts = line.split("|")
                if len(parts) >= 5:
                    return parts[4].strip()
        except Exception:
            pass
        return f"AS{asn}"

    async def lookup(self, ip: str) -> Tuple[Optional[int], Optional[str]]:
        """
        IPアドレスからASN情報を検索
        Returns: (asn, organization) or (None, None)
        """
        if not ip or not self._is_public_ip(ip):
            return (None, None)

        # キャッシュチェック（LRU更新）
        if ip in self.cache:
            asn, org, ts = self.cache[ip]
            self.stats["hits"] += 1
            # LRU: 末尾に移動
            self.cache.move_to_end(ip)
            return (asn, org)

        # DNS検索
        self.stats["misses"] += 1
        result = await self._dns_lookup(ip)

        if result:
            asn, org = result
            self.cache[ip] = (asn, org, datetime.now().timestamp())
            # キャッシュサイズ制限（LRU削除）
            while len(self.cache) > MAX_CACHE_SIZE:
                self.cache.popitem(last=False)  # 最も古いエントリを削除
            # 定期的にキャッシュ保存（1000件ごと、SDカード保護）
            if len(self.cache) % 1000 == 0:
                self._save_cache()
            return (asn, org)

        self.stats["errors"] += 1
        return (None, None)

    def _is_public_ip(self, ip: str) -> bool:
        """プライベートIPかどうか判定"""
        if ip.startswith("192.168.") or ip.startswith("10."):
            return False
        if ip.startswith("172."):
            parts = ip.split(".")
            if len(parts) >= 2:
                second = int(parts[1])
                if 16 <= second <= 31:
                    return False
        if ip.startswith("127.") or ip.startswith("0."):
            return False
        return True

    def get_service(self, asn: int) -> Tuple[Optional[str], Optional[str]]:
        """ASNからサービス名とカテゴリを取得"""
        return get_service_by_asn(asn)

    async def lookup_service(self, ip: str) -> Tuple[Optional[str], Optional[str], Optional[int]]:
        """
        IPからサービス情報を直接取得
        Returns: (service_name, category, asn) or (None, None, None)
        """
        asn, org = await self.lookup(ip)
        if asn:
            service, category = self.get_service(asn)
            if service:
                return (service, category, asn)
            # ASNマップにない場合は組織名を返す
            return (org, "unknown", asn)
        return (None, None, None)

    def get_stats(self) -> dict:
        """統計情報を取得"""
        return {
            "cache_size": len(self.cache),
            "hits": self.stats["hits"],
            "misses": self.stats["misses"],
            "errors": self.stats["errors"],
            "known_asns": len(ASN_SERVICE_MAP),
        }

    def save(self):
        """キャッシュを明示的に保存"""
        self._save_cache()


# シングルトン
_instance: Optional[ASNLookup] = None


def get_asn_lookup() -> ASNLookup:
    """ASNLookupシングルトンを取得"""
    global _instance
    if _instance is None:
        _instance = ASNLookup()
    return _instance


__all__ = ["ASNLookup", "get_asn_lookup"]
