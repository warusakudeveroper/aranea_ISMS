"""
System Configuration Module for Orange Pi Zero3
WiFi設定、NTP設定、キャッシュ同期機能
"""
import asyncio
import hashlib
import json
import logging
import os
import re
import shutil
import subprocess
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple
from zoneinfo import ZoneInfo

import httpx

logger = logging.getLogger(__name__)

# データディレクトリ
DATA_DIR = Path("/var/lib/is20s")
SYNC_CONFIG_FILE = DATA_DIR / "sync_config.json"
WIFI_CONFIG_FILE = DATA_DIR / "wifi_config.json"

# 同期対象キャッシュファイル
CACHE_FILES = [
    "dns_cache.json",
    "asn_cache.json",
]

# WiFi設定の最大数
MAX_WIFI_CONFIGS = 6

# デフォルトWiFi設定（cluster1-6）
DEFAULT_WIFI_CONFIGS = [
    {"ssid": "cluster1", "password": "isms12345@", "priority": 0},
    {"ssid": "cluster2", "password": "isms12345@", "priority": 1},
    {"ssid": "cluster3", "password": "isms12345@", "priority": 2},
    {"ssid": "cluster4", "password": "isms12345@", "priority": 3},
    {"ssid": "cluster5", "password": "isms12345@", "priority": 4},
    {"ssid": "cluster6", "password": "isms12345@", "priority": 5},
]


class WiFiConfigManager:
    """
    WiFi複数設定管理クラス

    is10と同様に最大6件のWiFi設定を保持し、
    先頭から順に接続を試行するローテーション機能を提供
    """

    def __init__(self, config_file: Path = WIFI_CONFIG_FILE):
        self.config_file = config_file
        self.configs: List[Dict[str, Any]] = []
        self._load()

    def _load(self) -> None:
        """設定ファイルから読み込み"""
        if self.config_file.exists():
            try:
                with self.config_file.open() as f:
                    data = json.load(f)
                    self.configs = data.get("wifi_configs", [])[:MAX_WIFI_CONFIGS]
                    logger.info("Loaded %d WiFi configs from file", len(self.configs))
            except Exception as e:
                logger.warning("Failed to load WiFi config: %s", e)
                self.configs = []

        # 設定がない場合はデフォルトを使用
        if not self.configs:
            self.configs = DEFAULT_WIFI_CONFIGS.copy()
            self._save()
            logger.info("Initialized with default WiFi configs")

    def _save(self) -> bool:
        """設定ファイルに保存"""
        try:
            self.config_file.parent.mkdir(parents=True, exist_ok=True)
            with self.config_file.open("w") as f:
                json.dump({"wifi_configs": self.configs}, f, indent=2)
            logger.info("Saved %d WiFi configs", len(self.configs))
            return True
        except Exception as e:
            logger.error("Failed to save WiFi config: %s", e)
            return False

    def get_all(self) -> List[Dict[str, Any]]:
        """すべてのWiFi設定を取得（パスワードはマスク）"""
        return [
            {
                "ssid": c.get("ssid", ""),
                "password": "***" if c.get("password") else "",
                "priority": c.get("priority", i),
                "index": i,
            }
            for i, c in enumerate(self.configs)
        ]

    def get_all_raw(self) -> List[Dict[str, Any]]:
        """すべてのWiFi設定を取得（パスワード含む・内部用）"""
        return self.configs.copy()

    def add(self, ssid: str, password: str) -> Dict[str, Any]:
        """WiFi設定を追加"""
        if len(self.configs) >= MAX_WIFI_CONFIGS:
            return {"ok": False, "error": f"Maximum {MAX_WIFI_CONFIGS} configs allowed"}

        if not ssid:
            return {"ok": False, "error": "SSID required"}

        # 既存のSSIDチェック
        for c in self.configs:
            if c.get("ssid") == ssid:
                return {"ok": False, "error": f"SSID '{ssid}' already exists"}

        new_config = {
            "ssid": ssid,
            "password": password,
            "priority": len(self.configs),
        }
        self.configs.append(new_config)

        if self._save():
            return {"ok": True, "message": f"Added WiFi config for '{ssid}'", "index": len(self.configs) - 1}
        else:
            self.configs.pop()
            return {"ok": False, "error": "Failed to save config"}

    def update(self, index: int, ssid: str = None, password: str = None) -> Dict[str, Any]:
        """WiFi設定を更新"""
        if index < 0 or index >= len(self.configs):
            return {"ok": False, "error": f"Invalid index: {index}"}

        if ssid is not None:
            # 他のSSIDと重複チェック
            for i, c in enumerate(self.configs):
                if i != index and c.get("ssid") == ssid:
                    return {"ok": False, "error": f"SSID '{ssid}' already exists at index {i}"}
            self.configs[index]["ssid"] = ssid

        if password is not None:
            self.configs[index]["password"] = password

        if self._save():
            return {"ok": True, "message": f"Updated WiFi config at index {index}"}
        else:
            return {"ok": False, "error": "Failed to save config"}

    def delete(self, index: int) -> Dict[str, Any]:
        """WiFi設定を削除"""
        if index < 0 or index >= len(self.configs):
            return {"ok": False, "error": f"Invalid index: {index}"}

        deleted = self.configs.pop(index)

        # priority再計算
        for i, c in enumerate(self.configs):
            c["priority"] = i

        if self._save():
            return {"ok": True, "message": f"Deleted WiFi config for '{deleted.get('ssid', '')}'"}
        else:
            self.configs.insert(index, deleted)
            return {"ok": False, "error": "Failed to save config"}

    def reorder(self, old_index: int, new_index: int) -> Dict[str, Any]:
        """WiFi設定の優先順位を変更"""
        if old_index < 0 or old_index >= len(self.configs):
            return {"ok": False, "error": f"Invalid old_index: {old_index}"}
        if new_index < 0 or new_index >= len(self.configs):
            return {"ok": False, "error": f"Invalid new_index: {new_index}"}

        config = self.configs.pop(old_index)
        self.configs.insert(new_index, config)

        # priority再計算
        for i, c in enumerate(self.configs):
            c["priority"] = i

        if self._save():
            return {"ok": True, "message": "Reordered WiFi configs"}
        else:
            return {"ok": False, "error": "Failed to save config"}

    def reset_to_defaults(self) -> Dict[str, Any]:
        """デフォルト設定にリセット"""
        self.configs = DEFAULT_WIFI_CONFIGS.copy()
        if self._save():
            return {"ok": True, "message": "Reset to default WiFi configs"}
        else:
            return {"ok": False, "error": "Failed to save config"}

    async def auto_connect(self, timeout_per_ssid: int = 60) -> Dict[str, Any]:
        """
        保存されたWiFi設定を順番に試行して接続

        Args:
            timeout_per_ssid: 各SSIDあたりの接続タイムアウト（秒）

        Returns:
            接続結果
        """
        if not self.configs:
            return {"ok": False, "error": "No WiFi configs available"}

        # 現在の接続状態チェック
        current = get_wifi_status()
        if current.get("connected"):
            return {
                "ok": True,
                "message": f"Already connected to {current.get('ssid')}",
                "ssid": current.get("ssid"),
                "already_connected": True,
            }

        tried = []
        for i, config in enumerate(self.configs):
            ssid = config.get("ssid", "")
            password = config.get("password", "")

            if not ssid:
                continue

            logger.info("Trying WiFi config %d: %s", i, ssid)
            tried.append(ssid)

            result = await connect_wifi(ssid, password)

            if result.get("ok"):
                logger.info("Successfully connected to %s", ssid)
                return {
                    "ok": True,
                    "message": f"Connected to {ssid}",
                    "ssid": ssid,
                    "tried": tried,
                    "index": i,
                }
            else:
                logger.warning("Failed to connect to %s: %s", ssid, result.get("error"))

        return {
            "ok": False,
            "error": "Failed to connect to any configured WiFi",
            "tried": tried,
        }


# WiFiConfigManagerシングルトン
_wifi_config_manager: Optional[WiFiConfigManager] = None


def get_wifi_config_manager() -> WiFiConfigManager:
    """WiFiConfigManagerシングルトンを取得"""
    global _wifi_config_manager
    if _wifi_config_manager is None:
        _wifi_config_manager = WiFiConfigManager()
    return _wifi_config_manager


# ========== WiFi設定 ==========

def get_wifi_status() -> Dict[str, Any]:
    """WiFi接続状態を取得（nmcli使用）"""
    result = {
        "connected": False,
        "ssid": None,
        "signal_strength": None,
        "ip_address": None,
        "interface": "wlan0",
    }

    try:
        # nmcli でwlan0の接続状態を取得
        proc = subprocess.run(
            ["nmcli", "-t", "-f", "NAME,TYPE,DEVICE", "connection", "show", "--active"],
            capture_output=True, text=True, timeout=5
        )
        if proc.returncode == 0:
            for line in proc.stdout.strip().split("\n"):
                if not line:
                    continue
                parts = line.split(":")
                if len(parts) >= 3 and parts[2] == "wlan0" and parts[1] == "802-11-wireless":
                    result["ssid"] = parts[0]
                    result["connected"] = True
                    break

        # IPアドレス取得
        proc = subprocess.run(
            ["ip", "-4", "addr", "show", "wlan0"],
            capture_output=True, text=True, timeout=5
        )
        if proc.returncode == 0:
            match = re.search(r"inet (\d+\.\d+\.\d+\.\d+)", proc.stdout)
            if match:
                result["ip_address"] = match.group(1)

        # 信号強度取得（nmcli dev wifi listから）
        if result["ssid"]:
            proc = subprocess.run(
                ["nmcli", "-t", "-f", "SSID,SIGNAL", "dev", "wifi", "list"],
                capture_output=True, text=True, timeout=10
            )
            if proc.returncode == 0:
                for line in proc.stdout.strip().split("\n"):
                    if not line:
                        continue
                    parts = line.split(":")
                    if len(parts) >= 2 and parts[0] == result["ssid"]:
                        try:
                            # Signal is 0-100%, convert to approximate dBm
                            signal_pct = int(parts[1])
                            result["signal_strength"] = signal_pct
                        except ValueError:
                            pass
                        break

    except Exception as e:
        logger.warning("Failed to get WiFi status: %s", e)

    return result


def get_available_networks() -> List[Dict[str, Any]]:
    """利用可能なWiFiネットワーク一覧を取得"""
    networks = []
    try:
        # nmcli でスキャン
        proc = subprocess.run(
            ["nmcli", "-t", "-f", "SSID,SIGNAL,SECURITY", "dev", "wifi", "list"],
            capture_output=True, text=True, timeout=30
        )
        if proc.returncode == 0:
            for line in proc.stdout.strip().split("\n"):
                if not line:
                    continue
                parts = line.split(":")
                if len(parts) >= 3:
                    ssid = parts[0]
                    if ssid:  # 空のSSIDはスキップ
                        networks.append({
                            "ssid": ssid,
                            "signal": int(parts[1]) if parts[1].isdigit() else 0,
                            "security": parts[2] if parts[2] else "Open",
                        })
    except Exception as e:
        logger.warning("Failed to scan WiFi networks: %s", e)

    return networks


async def connect_wifi(ssid: str, password: str) -> Dict[str, Any]:
    """WiFiに接続"""
    try:
        # nmcli で接続
        proc = await asyncio.create_subprocess_exec(
            "nmcli", "dev", "wifi", "connect", ssid, "password", password,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=60)

        if proc.returncode == 0:
            return {"ok": True, "message": f"Connected to {ssid}"}
        else:
            error = stderr.decode().strip() if stderr else "Unknown error"
            return {"ok": False, "error": error}

    except asyncio.TimeoutError:
        return {"ok": False, "error": "Connection timeout"}
    except Exception as e:
        return {"ok": False, "error": str(e)}


# ========== NTP設定 ==========

def get_ntp_status() -> Dict[str, Any]:
    """NTP同期状態を取得"""
    result = {
        "synchronized": False,
        "ntp_server": None,
        "current_time": datetime.now(ZoneInfo("Asia/Tokyo")).isoformat(),
        "timezone": "Asia/Tokyo",
    }

    try:
        # timedatectl で状態取得
        proc = subprocess.run(
            ["timedatectl", "show"],
            capture_output=True, text=True, timeout=5
        )
        if proc.returncode == 0:
            for line in proc.stdout.strip().split("\n"):
                if "=" in line:
                    key, value = line.split("=", 1)
                    if key == "NTPSynchronized":
                        result["synchronized"] = value.lower() == "yes"
                    elif key == "Timezone":
                        result["timezone"] = value

        # NTPサーバー取得
        proc = subprocess.run(
            ["timedatectl", "show-timesync"],
            capture_output=True, text=True, timeout=5
        )
        if proc.returncode == 0:
            for line in proc.stdout.strip().split("\n"):
                if line.startswith("ServerName="):
                    result["ntp_server"] = line.split("=", 1)[1]
                    break

    except Exception as e:
        logger.warning("Failed to get NTP status: %s", e)

    return result


async def set_ntp_server(server: str) -> Dict[str, Any]:
    """NTPサーバーを設定"""
    try:
        # /etc/systemd/timesyncd.conf を更新
        conf_path = Path("/etc/systemd/timesyncd.conf")
        content = f"""[Time]
NTP={server}
FallbackNTP=ntp.ubuntu.com time.google.com
"""
        conf_path.write_text(content)

        # systemd-timesyncd を再起動
        proc = await asyncio.create_subprocess_exec(
            "systemctl", "restart", "systemd-timesyncd",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        await asyncio.wait_for(proc.communicate(), timeout=10)

        if proc.returncode == 0:
            return {"ok": True, "message": f"NTP server set to {server}"}
        else:
            return {"ok": False, "error": "Failed to restart timesyncd"}

    except Exception as e:
        return {"ok": False, "error": str(e)}


async def set_timezone(tz: str) -> Dict[str, Any]:
    """タイムゾーンを設定"""
    try:
        proc = await asyncio.create_subprocess_exec(
            "timedatectl", "set-timezone", tz,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=10)

        if proc.returncode == 0:
            return {"ok": True, "message": f"Timezone set to {tz}"}
        else:
            error = stderr.decode().strip() if stderr else "Unknown error"
            return {"ok": False, "error": error}

    except Exception as e:
        return {"ok": False, "error": str(e)}


# ========== キャッシュ同期機能 ==========

class CacheSyncManager:
    """
    キャッシュファイル同期マネージャー

    機能:
    - エクスポート: キャッシュファイルをJSONでダウンロード
    - インポート: JSONからキャッシュを復元
    - 同期: 他のis20s機器とキャッシュを相互マージ（デイジーチェーン対応）
    """

    def __init__(self, data_dir: Path = DATA_DIR):
        self.data_dir = data_dir
        self.config_file = SYNC_CONFIG_FILE
        self.config = self._load_config()

    def _load_config(self) -> Dict[str, Any]:
        """同期設定を読み込み"""
        default = {
            "peers": [],  # [{"host": "192.168.x.x", "port": 8080}]
            "initiator_passphrase": "",
            "responder_passphrase": "",
            "last_sync": None,
            "auto_sync_enabled": False,
            "sync_interval_hours": 24,
        }
        if self.config_file.exists():
            try:
                with self.config_file.open() as f:
                    loaded = json.load(f)
                    return {**default, **loaded}
            except Exception as e:
                logger.warning("Failed to load sync config: %s", e)
        return default

    def _save_config(self) -> None:
        """同期設定を保存"""
        try:
            self.config_file.parent.mkdir(parents=True, exist_ok=True)
            with self.config_file.open("w") as f:
                json.dump(self.config, f, indent=2)
        except Exception as e:
            logger.error("Failed to save sync config: %s", e)

    def set_passphrases(self, initiator: str, responder: str) -> None:
        """パスフレーズを設定"""
        self.config["initiator_passphrase"] = initiator
        self.config["responder_passphrase"] = responder
        self._save_config()

    def add_peer(self, host: str, port: int = 8080) -> None:
        """同期先ピアを追加"""
        peer = {"host": host, "port": port}
        if peer not in self.config["peers"]:
            self.config["peers"].append(peer)
            self._save_config()

    def remove_peer(self, host: str) -> None:
        """同期先ピアを削除"""
        self.config["peers"] = [p for p in self.config["peers"] if p["host"] != host]
        self._save_config()

    def export_cache(self) -> Dict[str, Any]:
        """キャッシュファイルをエクスポート（辞書形式）"""
        export_data = {
            "version": 1,
            "exported_at": datetime.now(ZoneInfo("Asia/Tokyo")).isoformat(),
            "caches": {},
        }

        for cache_file in CACHE_FILES:
            path = self.data_dir / cache_file
            if path.exists():
                try:
                    with path.open() as f:
                        export_data["caches"][cache_file] = json.load(f)
                except Exception as e:
                    logger.warning("Failed to export %s: %s", cache_file, e)

        return export_data

    def import_cache(self, data: Dict[str, Any], merge: bool = True) -> Dict[str, Any]:
        """
        キャッシュファイルをインポート

        Args:
            data: エクスポートされたデータ
            merge: Trueの場合は既存とマージ、Falseの場合は上書き

        Returns:
            結果とマージ統計
        """
        stats = {"merged": 0, "added": 0, "skipped": 0}

        if "caches" not in data:
            return {"ok": False, "error": "Invalid data format", "stats": stats}

        for cache_file, cache_data in data["caches"].items():
            if cache_file not in CACHE_FILES:
                stats["skipped"] += 1
                continue

            path = self.data_dir / cache_file
            try:
                if merge and path.exists():
                    # 既存データとマージ
                    with path.open() as f:
                        existing = json.load(f)

                    if isinstance(existing, dict) and isinstance(cache_data, dict):
                        # 辞書の場合はキーをマージ
                        before_count = len(existing)
                        existing.update(cache_data)
                        stats["merged"] += len(existing) - before_count
                    elif isinstance(existing, list) and isinstance(cache_data, list):
                        # リストの場合は重複を除いて追加
                        existing_set = set(map(json.dumps, existing))
                        for item in cache_data:
                            item_str = json.dumps(item)
                            if item_str not in existing_set:
                                existing.append(item)
                                stats["merged"] += 1
                    else:
                        existing = cache_data
                        stats["added"] += 1

                    with path.open("w") as f:
                        json.dump(existing, f)
                else:
                    # 上書き
                    path.parent.mkdir(parents=True, exist_ok=True)
                    with path.open("w") as f:
                        json.dump(cache_data, f)
                    stats["added"] += 1

            except Exception as e:
                logger.error("Failed to import %s: %s", cache_file, e)
                stats["skipped"] += 1

        return {"ok": True, "stats": stats}

    def _hash_passphrase(self, passphrase: str) -> str:
        """パスフレーズをハッシュ化"""
        return hashlib.sha256(passphrase.encode()).hexdigest()[:32]

    async def sync_with_peer(
        self, host: str, port: int = 8080, as_initiator: bool = True
    ) -> Dict[str, Any]:
        """
        ピアと同期（イニシエータ/レスポンダー方式）

        Args:
            host: 同期先ホスト
            port: ポート番号
            as_initiator: Trueならイニシエータとして動作

        Returns:
            同期結果
        """
        passphrase = (
            self.config["initiator_passphrase"]
            if as_initiator
            else self.config["responder_passphrase"]
        )

        if not passphrase:
            return {"ok": False, "error": "Passphrase not configured"}

        auth_header = {"X-Sync-Auth": self._hash_passphrase(passphrase)}

        try:
            async with httpx.AsyncClient(timeout=60) as client:
                # 1. 相手からキャッシュを取得
                url = f"http://{host}:{port}/api/sync/export"
                resp = await client.get(url, headers=auth_header)

                if resp.status_code != 200:
                    return {"ok": False, "error": f"Failed to get cache from peer: {resp.status_code}"}

                peer_data = resp.json()

                # 2. 自分のキャッシュとマージ
                import_result = self.import_cache(peer_data, merge=True)

                # 3. マージ後のキャッシュを相手に送信
                my_data = self.export_cache()
                url = f"http://{host}:{port}/api/sync/import"
                resp = await client.post(url, json=my_data, headers=auth_header)

                if resp.status_code != 200:
                    logger.warning("Failed to push cache to peer: %s", resp.status_code)

                self.config["last_sync"] = datetime.now(ZoneInfo("Asia/Tokyo")).isoformat()
                self._save_config()

                return {
                    "ok": True,
                    "peer": host,
                    "stats": import_result.get("stats", {}),
                    "last_sync": self.config["last_sync"],
                }

        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def sync_all_peers(self) -> List[Dict[str, Any]]:
        """すべてのピアと同期（デイジーチェーン）"""
        results = []
        for peer in self.config["peers"]:
            result = await self.sync_with_peer(peer["host"], peer["port"])
            results.append(result)
        return results

    def verify_auth(self, auth_hash: str) -> bool:
        """認証ハッシュを検証"""
        initiator_hash = self._hash_passphrase(self.config["initiator_passphrase"])
        responder_hash = self._hash_passphrase(self.config["responder_passphrase"])
        return auth_hash in (initiator_hash, responder_hash)

    def get_status(self) -> Dict[str, Any]:
        """同期ステータスを取得"""
        return {
            "peers": self.config["peers"],
            "last_sync": self.config["last_sync"],
            "auto_sync_enabled": self.config["auto_sync_enabled"],
            "sync_interval_hours": self.config["sync_interval_hours"],
            "initiator_configured": bool(self.config["initiator_passphrase"]),
            "responder_configured": bool(self.config["responder_passphrase"]),
        }


# シングルトンインスタンス
_sync_manager: Optional[CacheSyncManager] = None


def get_sync_manager() -> CacheSyncManager:
    """CacheSyncManagerシングルトンを取得"""
    global _sync_manager
    if _sync_manager is None:
        _sync_manager = CacheSyncManager()
    return _sync_manager


__all__ = [
    "get_wifi_status",
    "get_available_networks",
    "connect_wifi",
    "get_ntp_status",
    "set_ntp_server",
    "set_timezone",
    "CacheSyncManager",
    "get_sync_manager",
    "WiFiConfigManager",
    "get_wifi_config_manager",
    "MAX_WIFI_CONFIGS",
]
