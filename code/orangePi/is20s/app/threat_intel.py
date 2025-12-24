"""
Threat Intelligence Module
公開ドメインリスト・Tor出口ノードを活用した脅威検知
"""
import asyncio
import json
import logging
import re
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Dict, Optional, Set

import httpx

logger = logging.getLogger(__name__)

# データ保存先
THREAT_DATA_DIR = Path("/var/lib/is20s/threat_intel")
UPDATE_INTERVAL_HOURS = 24  # 更新間隔

# データソース
SOURCES = {
    "stevenblack_unified": {
        "url": "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts",
        "type": "hosts",
        "category": "malware",
    },
    "stevenblack_fakenews": {
        "url": "https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/fakenews/hosts",
        "type": "hosts",
        "category": "fakenews",
    },
    "stevenblack_gambling": {
        "url": "https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/gambling/hosts",
        "type": "hosts",
        "category": "gambling",
    },
    "stevenblack_porn": {
        "url": "https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/porn/hosts",
        "type": "hosts",
        "category": "adult",
    },
    "tor_exit_nodes": {
        "url": "https://www.dan.me.uk/torlist/?exit",
        "type": "ip_list",
        "category": "tor",
    },
}


def parse_hosts_file(content: str) -> Set[str]:
    """hosts形式ファイルをパース（0.0.0.0 domain.com）"""
    domains = set()
    for line in content.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) >= 2 and parts[0] in ("0.0.0.0", "127.0.0.1"):
            domain = parts[1].lower()
            if domain and domain not in ("localhost", "localhost.localdomain"):
                domains.add(domain)
    return domains


def parse_ip_list(content: str) -> Set[str]:
    """IP一覧をパース（1行1IP）"""
    ips = set()
    for line in content.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        # IPv4形式チェック
        if re.match(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$", line):
            ips.add(line)
    return ips


class ThreatIntel:
    """脅威インテリジェンスデータ管理"""

    def __init__(self):
        self.data_dir = THREAT_DATA_DIR
        self.data_dir.mkdir(parents=True, exist_ok=True)

        # カテゴリ別ドメインセット
        self.malware_domains: Set[str] = set()
        self.adult_domains: Set[str] = set()
        self.gambling_domains: Set[str] = set()
        self.fakenews_domains: Set[str] = set()

        # Tor出口ノードIP
        self.tor_exit_ips: Set[str] = set()

        # メタデータ
        self.last_update: Optional[datetime] = None
        self.stats: Dict[str, int] = {}

        self._load_cached()

    def _meta_path(self) -> Path:
        return self.data_dir / "meta.json"

    def _load_cached(self) -> None:
        """キャッシュされたデータを読み込み"""
        meta_path = self._meta_path()
        if meta_path.exists():
            try:
                meta = json.loads(meta_path.read_text())
                self.last_update = datetime.fromisoformat(meta.get("last_update", ""))
                self.stats = meta.get("stats", {})
            except Exception as e:
                logger.warning("Failed to load threat intel meta: %s", e)

        # 各カテゴリのデータをロード
        for name, domains_set in [
            ("malware", self.malware_domains),
            ("adult", self.adult_domains),
            ("gambling", self.gambling_domains),
            ("fakenews", self.fakenews_domains),
        ]:
            path = self.data_dir / f"{name}_domains.txt"
            if path.exists():
                try:
                    domains_set.update(path.read_text().splitlines())
                    logger.info("Loaded %d %s domains", len(domains_set), name)
                except Exception as e:
                    logger.warning("Failed to load %s domains: %s", name, e)

        # Tor出口ノード
        tor_path = self.data_dir / "tor_exit_ips.txt"
        if tor_path.exists():
            try:
                self.tor_exit_ips.update(tor_path.read_text().splitlines())
                logger.info("Loaded %d Tor exit IPs", len(self.tor_exit_ips))
            except Exception as e:
                logger.warning("Failed to load Tor exit IPs: %s", e)

    def _save_meta(self) -> None:
        """メタデータを保存"""
        meta = {
            "last_update": self.last_update.isoformat() if self.last_update else None,
            "stats": self.stats,
        }
        self._meta_path().write_text(json.dumps(meta, indent=2))

    async def update(self, force: bool = False) -> Dict[str, Any]:
        """データソースを更新"""
        # 更新間隔チェック
        if not force and self.last_update:
            elapsed = datetime.now() - self.last_update
            if elapsed < timedelta(hours=UPDATE_INTERVAL_HOURS):
                return {"status": "skipped", "reason": "recently updated"}

        results = {}
        async with httpx.AsyncClient(timeout=60) as client:
            for name, source in SOURCES.items():
                try:
                    logger.info("Fetching %s from %s", name, source["url"])
                    resp = await client.get(source["url"])
                    resp.raise_for_status()
                    content = resp.text

                    if source["type"] == "hosts":
                        domains = parse_hosts_file(content)
                        category = source["category"]

                        if category == "malware":
                            self.malware_domains = domains
                            path = self.data_dir / "malware_domains.txt"
                        elif category == "adult":
                            self.adult_domains = domains
                            path = self.data_dir / "adult_domains.txt"
                        elif category == "gambling":
                            self.gambling_domains = domains
                            path = self.data_dir / "gambling_domains.txt"
                        elif category == "fakenews":
                            self.fakenews_domains = domains
                            path = self.data_dir / "fakenews_domains.txt"
                        else:
                            continue

                        path.write_text("\n".join(sorted(domains)))
                        results[name] = {"count": len(domains), "status": "ok"}
                        self.stats[name] = len(domains)

                    elif source["type"] == "ip_list":
                        ips = parse_ip_list(content)
                        self.tor_exit_ips = ips
                        path = self.data_dir / "tor_exit_ips.txt"
                        path.write_text("\n".join(sorted(ips)))
                        results[name] = {"count": len(ips), "status": "ok"}
                        self.stats[name] = len(ips)

                except Exception as e:
                    logger.error("Failed to fetch %s: %s", name, e)
                    results[name] = {"status": "error", "error": str(e)}

        self.last_update = datetime.now()
        self._save_meta()
        logger.info("Threat intel update complete: %s", results)
        return {"status": "updated", "results": results}

    def check_domain(self, domain: str) -> Optional[str]:
        """ドメインの脅威カテゴリを判定"""
        if not domain:
            return None
        d = domain.lower()

        # 完全一致チェック
        if d in self.malware_domains:
            return "malware"
        if d in self.adult_domains:
            return "adult"
        if d in self.gambling_domains:
            return "gambling"
        if d in self.fakenews_domains:
            return "fakenews"

        # サブドメインチェック（親ドメインがリストにあるか）
        parts = d.split(".")
        for i in range(len(parts) - 1):
            parent = ".".join(parts[i:])
            if parent in self.malware_domains:
                return "malware"
            if parent in self.adult_domains:
                return "adult"
            if parent in self.gambling_domains:
                return "gambling"
            if parent in self.fakenews_domains:
                return "fakenews"

        return None

    def check_ip(self, ip: str) -> Optional[str]:
        """IPの脅威カテゴリを判定"""
        if not ip:
            return None
        if ip in self.tor_exit_ips:
            return "tor"
        return None

    def check(self, domain: str = None, ip: str = None) -> Optional[str]:
        """ドメインまたはIPの脅威カテゴリを判定"""
        result = self.check_domain(domain)
        if result:
            return result
        return self.check_ip(ip)

    def get_stats(self) -> Dict[str, Any]:
        """統計情報を取得"""
        return {
            "last_update": self.last_update.isoformat() if self.last_update else None,
            "malware_domains": len(self.malware_domains),
            "adult_domains": len(self.adult_domains),
            "gambling_domains": len(self.gambling_domains),
            "fakenews_domains": len(self.fakenews_domains),
            "tor_exit_ips": len(self.tor_exit_ips),
        }


# シングルトンインスタンス
_instance: Optional[ThreatIntel] = None


def get_threat_intel() -> ThreatIntel:
    """ThreatIntelシングルトンを取得"""
    global _instance
    if _instance is None:
        _instance = ThreatIntel()
    return _instance


__all__ = ["ThreatIntel", "get_threat_intel"]
