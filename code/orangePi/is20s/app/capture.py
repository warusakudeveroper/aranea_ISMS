"""
is20s tshark Capture Module
ER605ミラーポートからパケットキャプチャしメタデータを抽出・送信
"""
import asyncio
import gzip
import json
import logging
import os
import random
import re
import signal
import socket
import subprocess
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Set
from zoneinfo import ZoneInfo

import httpx

from .threat_intel import get_threat_intel
from .asn_lookup import get_asn_lookup

logger = logging.getLogger(__name__)

# ファイルログ設定
DEFAULT_LOG_DIR = "/var/lib/is20s/capture_logs"
DEFAULT_MAX_FILE_SIZE = 1 * 1024 * 1024  # 1MB per file
DEFAULT_MAX_TOTAL_SIZE = 10 * 1024 * 1024  # 10MB total

# DNSキャッシュ設定
DEFAULT_DNS_CACHE_FILE = "/var/lib/is20s/dns_cache.json"
DNS_CACHE_MAX_ENTRIES = 50000  # 最大エントリ数（LRU管理）
# NOTE: 50Kエントリ程度ならシャーディング不要（dict O(1)で十分高速、メモリ約2.5MB）
# 将来100K超えるなら、IPの第1オクテットで256分割を検討:
#   shards = [dict() for _ in range(256)]
#   shard_idx = int(ip.split('.')[0])
#   shards[shard_idx][ip] = domain

# PTR逆引き設定
PTR_QUEUE_MAX_SIZE = 100  # PTRリクエストキュー上限（溢れたら諦める）
PTR_LOOKUP_TIMEOUT = 2.0  # PTR逆引きタイムアウト（秒）
PTR_WORKER_INTERVAL = 0.1  # PTRワーカー処理間隔（秒）

# デフォルト設定
DEFAULT_WATCH_CONFIG = {
    "timezone": "Asia/Tokyo",
    "rooms": {},  # IP -> room_no マッピング
    "capture": {
        "iface": "eth0",
        "snaplen": 2048,
        "buffer_mb": 16,
        "ports": [53, 80, 443],
        "include_quic_udp_443": True,
        "include_tcp_syn_any_port": True,
        "syn_only": True,  # 軽量モード（デフォルトON）
    },
    "post": {
        "url": "",
        "headers": {},
        "batch_max_events": 500,
        "batch_max_seconds": 30,
        "gzip": True,
        "max_queue_events": 20000,
    },
    "mode": {
        "dry_run": False,
        "enabled": False,
    },
    "logging": {
        "enabled": True,
        "log_dir": "/var/lib/is20s/capture_logs",
        "max_file_size": 1048576,  # 1MB per file
        "max_total_size": 10485760,  # 10MB total
    },
}


def load_watch_config(path: str = "/etc/is20s/watch_config.json") -> Dict[str, Any]:
    """監視設定JSONをロード"""
    p = Path(path)
    if not p.exists():
        logger.warning("watch_config not found at %s, using defaults", path)
        return DEFAULT_WATCH_CONFIG.copy()
    try:
        with p.open() as f:
            cfg = json.load(f)
        # デフォルトとマージ
        merged = DEFAULT_WATCH_CONFIG.copy()
        for key, val in cfg.items():
            if isinstance(val, dict) and key in merged and isinstance(merged[key], dict):
                merged[key] = {**merged[key], **val}
            else:
                merged[key] = val
        return merged
    except Exception as e:
        logger.error("Failed to load watch_config: %s", e)
        return DEFAULT_WATCH_CONFIG.copy()


def save_watch_config(cfg: Dict[str, Any], path: str = "/etc/is20s/watch_config.json") -> bool:
    """監視設定JSONを保存"""
    p = Path(path)
    try:
        p.parent.mkdir(parents=True, exist_ok=True)
        with p.open("w") as f:
            json.dump(cfg, f, ensure_ascii=False, indent=2)
        return True
    except Exception as e:
        logger.error("Failed to save watch_config: %s", e)
        return False


class DnsCache:
    """
    DNSキャッシュ: IP → ドメイン のマッピングを保持（LRU管理）
    DNSクエリから学習し、TCP SYN時に逆引きに使用
    """

    def __init__(self, cache_file: str = DEFAULT_DNS_CACHE_FILE, max_entries: int = DNS_CACHE_MAX_ENTRIES):
        self.cache_file = Path(cache_file)
        self.max_entries = max_entries
        self.cache: Dict[str, str] = {}  # IP -> domain
        self._access_order: List[str] = []  # LRU用アクセス順序
        self._load()

    def _load(self) -> None:
        """キャッシュファイルから読み込み"""
        if self.cache_file.exists():
            try:
                with self.cache_file.open() as f:
                    self.cache = json.load(f)
                self._access_order = list(self.cache.keys())
                logger.info("DNS cache loaded: %d entries", len(self.cache))
            except Exception as e:
                logger.warning("Failed to load DNS cache: %s", e)
                self.cache = {}
                self._access_order = []
        else:
            self.cache = {}
            self._access_order = []

    def save(self) -> None:
        """キャッシュファイルに保存"""
        try:
            self.cache_file.parent.mkdir(parents=True, exist_ok=True)
            with self.cache_file.open("w") as f:
                json.dump(self.cache, f, ensure_ascii=False)
        except Exception as e:
            logger.warning("Failed to save DNS cache: %s", e)

    def add(self, ip: str, domain: str) -> None:
        """IP→ドメインのマッピングを追加"""
        if not ip or not domain:
            return
        # 既存エントリの場合はLRU更新
        if ip in self.cache:
            try:
                self._access_order.remove(ip)
            except ValueError:
                pass
        self.cache[ip] = domain
        self._access_order.append(ip)
        # 最大エントリ数を超えたらLRUで削除
        while len(self.cache) > self.max_entries and self._access_order:
            oldest_ip = self._access_order.pop(0)
            self.cache.pop(oldest_ip, None)

    def add_multiple(self, ips: str, domain: str) -> None:
        """複数IP（カンマ区切り）を一括登録"""
        if not ips or not domain:
            return
        for ip in ips.split(","):
            ip = ip.strip()
            if ip:
                self.add(ip, domain)

    def get(self, ip: str) -> Optional[str]:
        """IPからドメインを取得（LRU更新）"""
        domain = self.cache.get(ip)
        if domain:
            # アクセス順序を更新（LRU）
            try:
                self._access_order.remove(ip)
                self._access_order.append(ip)
            except ValueError:
                pass
        return domain

    def get_stats(self) -> Dict[str, Any]:
        """統計情報を取得"""
        return {
            "entries": len(self.cache),
            "max_entries": self.max_entries,
            "cache_file": str(self.cache_file),
        }


def generate_event_id(room_no: str, tz: ZoneInfo) -> str:
    """
    イベントID生成: {roomNo}{yyyymmddhhmmss}-{rand4}
    例: 10120251224153005-0834
    """
    now = datetime.now(tz)
    ts = now.strftime("%Y%m%d%H%M%S")
    rand4 = f"{random.randint(0, 9999):04d}"
    return f"{room_no}{ts}-{rand4}"


def expand_segment_ips(rooms: Dict[str, str]) -> Dict[str, str]:
    """
    CIDR記法およびIP末尾0を/24セグメント全体（1-254）に展開

    対応形式:
    - 192.168.3.0/24=network → 192.168.3.1~254 をすべて登録
    - 192.168.3.0=network → 同上（末尾0も/24として扱う）
    - 192.168.3.10/32=room → 192.168.3.10 単体
    - 192.168.3.10=room → 同上（単体IP）

    注意: セグメント指定は大量トラフィックになるため、少量・小規模環境以外では非推奨
    """
    expanded = {}
    for ip_spec, room in rooms.items():
        # CIDR記法をパース
        if "/" in ip_spec:
            ip_part, cidr = ip_spec.rsplit("/", 1)
            try:
                prefix_len = int(cidr)
            except ValueError:
                logger.warning("Invalid CIDR notation: %s, treating as single IP", ip_spec)
                expanded[ip_part] = room
                continue

            if prefix_len == 32:
                # /32 = 単一IP
                expanded[ip_part] = room
            elif prefix_len == 24:
                # /24 = セグメント展開
                parts = ip_part.split(".")
                if len(parts) == 4:
                    prefix = ".".join(parts[:3]) + "."
                    for i in range(1, 255):
                        expanded[f"{prefix}{i}"] = room
                    logger.warning("Segment expansion (CIDR): %s -> %s1-254 (room=%s)", ip_spec, prefix, room)
                else:
                    expanded[ip_part] = room
            else:
                # その他のCIDRは単一IPとして扱う（将来拡張可能）
                logger.warning("Unsupported CIDR prefix /%d: %s, treating as single IP", prefix_len, ip_spec)
                expanded[ip_part] = room
        elif ip_spec.endswith(".0"):
            # 従来形式: 末尾0 → セグメント展開
            prefix = ip_spec[:-1]  # "192.168.125."
            for i in range(1, 255):
                expanded[f"{prefix}{i}"] = room
            logger.warning("Segment expansion: %s -> %s1-254 (room=%s)", ip_spec, prefix, room)
        else:
            # 単一IP
            expanded[ip_spec] = room
    return expanded


def build_bpf_filter(rooms: Dict[str, str], cfg: Dict[str, Any]) -> str:
    """
    BPFフィルタを構築
    監視対象は rooms のIPに限定
    syn_only=True: DNS + TCP SYN のみ（軽量モード、デフォルト）
    syn_only=False: 全パケット詳細モード（高負荷）

    DNS: 応答も捕捉するため src/dst 両方を対象
    その他: src のみ（発信トラフィック）

    IP末尾0指定: 192.168.1.0 のように末尾を0にすると /24セグメント全体が対象
    """
    cap_cfg = cfg.get("capture", {})

    # セグメント展開（末尾0 → 1-254）
    expanded_rooms = expand_segment_ips(rooms)
    ips = list(expanded_rooms.keys())
    if not ips:
        return "host 0.0.0.0"  # 何もキャプチャしない

    # ネットワーク単位でBPF最適化（セグメント指定時）
    segment_nets = set()
    individual_ips = []
    for ip in ips:
        parts = ip.rsplit('.', 1)
        if len(parts) == 2:
            segment_nets.add(parts[0])

    # セグメント全体指定されている場合は net で表現（BPF最適化）
    net_filters = []
    for prefix in segment_nets:
        # このプレフィックスのIPがすべて存在するかチェック（254個）
        count = sum(1 for ip in ips if ip.startswith(prefix + "."))
        if count >= 254:
            net_filters.append(f"net {prefix}.0/24")
        else:
            for ip in ips:
                if ip.startswith(prefix + "."):
                    individual_ips.append(ip)

    if not net_filters and not individual_ips:
        individual_ips = ips

    # src host / dst host のOR
    src_parts = [f"src {n}" for n in net_filters] + [f"src host {ip}" for ip in individual_ips]
    dst_parts = [f"dst {n}" for n in net_filters] + [f"dst host {ip}" for ip in individual_ips]

    src_hosts = " or ".join(src_parts) if src_parts else "src host 0.0.0.0"
    dst_hosts = " or ".join(dst_parts) if dst_parts else "dst host 0.0.0.0"

    # syn_only モード（デフォルトON = 軽量モード）
    syn_only = cap_cfg.get("syn_only", True)

    # DNS用フィルタ（応答も捕捉するため双方向）
    dns_filter = None
    if 53 in cap_cfg.get("ports", [53]):
        dns_filter = f"(({src_hosts}) or ({dst_hosts})) and port 53"

    # その他のフィルタ（発信のみ）
    other_conditions = []

    if syn_only:
        # 軽量モード: TCP SYN のみ（接続開始時の1パケット）
        # tcp[13] & 0x12 == 0x02 (SYN=1, ACK=0)
        other_conditions.append("(tcp[13] & 0x12 == 0x02)")
        # QUIC Initial (UDP 443) - 接続開始のみ
        if cap_cfg.get("include_quic_udp_443", True):
            other_conditions.append("udp port 443")
    else:
        # 詳細モード: 全パケット（高負荷注意）
        # HTTP (tcp port 80)
        if 80 in cap_cfg.get("ports", [80]):
            other_conditions.append("tcp port 80")

        # HTTPS (tcp port 443)
        if 443 in cap_cfg.get("ports", [443]):
            other_conditions.append("tcp port 443")

        # QUIC (udp port 443)
        if cap_cfg.get("include_quic_udp_443", True):
            other_conditions.append("udp port 443")

        # TCP SYN any port
        if cap_cfg.get("include_tcp_syn_any_port", True):
            other_conditions.append("(tcp[13] & 0x12 == 0x02)")

    # フィルタを組み立て
    filters = []
    if dns_filter:
        filters.append(dns_filter)
    if other_conditions:
        other_filter = " or ".join(other_conditions)
        filters.append(f"(({src_hosts}) and ({other_filter}))")

    if not filters:
        return f"({src_hosts}) and port 53"

    bpf = " or ".join(filters)
    return bpf


def build_tshark_cmd(cfg: Dict[str, Any], bpf: str) -> List[str]:
    """tsharkコマンドを構築"""
    cap_cfg = cfg.get("capture", {})
    iface = cap_cfg.get("iface", "eth0")
    snaplen = cap_cfg.get("snaplen", 2048)
    buffer_mb = cap_cfg.get("buffer_mb", 16)

    # tshark出力フィールド
    fields = [
        "frame.time_epoch",
        "ip.src",
        "ip.dst",
        "tcp.dstport",
        "udp.dstport",
        "ip.proto",
        "dns.qry.name",
        "dns.a",      # DNS A record response IP（キャッシュ用）
        "dns.aaaa",   # DNS AAAA record (IPv6)
        "dns.cname",  # CNAME alias
        "http.host",
        "http.request.uri",
        "tls.handshake.extensions_server_name",
        # GQUIC SNI (Google QUIC)
        "gquic.tag.sni",
    ]

    cmd = [
        "tshark",
        "-i", iface,
        "-n",  # 名前解決無効
        "-l",  # 行単位出力
        "-s", str(snaplen),
        "-B", str(buffer_mb),
        "-f", bpf,  # capture filter (BPF)
        "-T", "fields",
        "-E", "separator=|",
        "-E", "quote=n",
        "-E", "occurrence=f",  # 最初の値のみ
    ]
    for field in fields:
        cmd.extend(["-e", field])

    return cmd


def parse_tshark_line(
    line: str,
    rooms: Dict[str, str],
    tz: ZoneInfo,
    dns_cache: Optional[DnsCache] = None,
    ptr_queue: Optional[asyncio.Queue] = None,
    ptr_pending: Optional[set] = None,
    threat_intel: Optional["ThreatIntel"] = None,
) -> Optional[Dict[str, Any]]:
    """
    tshark出力行をパースしてイベント辞書を返す
    フォーマット: epoch|src|dst|tcp_port|udp_port|proto|dns_qry|dns_a|dns_aaaa|dns_cname|http_host|http_uri|tls_sni|gquic_sni
    """
    parts = line.strip().split("|")
    if len(parts) < 14:
        return None

    epoch_str = parts[0]
    src_ip = parts[1]
    dst_ip = parts[2]
    tcp_port = parts[3]
    udp_port = parts[4]
    proto = parts[5]
    dns_qry = parts[6]
    dns_a = parts[7]      # DNS A record response IP
    dns_aaaa = parts[8]   # DNS AAAA record (IPv6)
    dns_cname = parts[9]  # CNAME alias
    http_host = parts[10]
    http_uri = parts[11]
    tls_sni = parts[12]
    gquic_sni = parts[13] if len(parts) > 13 else ""

    if not epoch_str or not src_ip:
        return None

    # room_no取得
    room_no = rooms.get(src_ip, "UNK")

    # epoch -> ISO8601 (Asia/Tokyo)
    try:
        epoch = float(epoch_str)
        dt = datetime.fromtimestamp(epoch, tz=tz)
        iso_time = dt.isoformat()
    except (ValueError, OSError):
        epoch = 0.0
        iso_time = ""

    # dst_port
    dst_port = tcp_port or udp_port or ""

    # protocol
    protocol = "tcp" if tcp_port else ("udp" if udp_port else "")

    # DNSキャッシュ処理
    if dns_cache:
        # DNS応答があればキャッシュに追加 (IP → ドメイン、複数IP対応)
        if dns_qry and dns_a:
            dns_cache.add_multiple(dns_a, dns_qry)
        # IPv6 (AAAA) もキャッシュ
        if dns_qry and dns_aaaa:
            dns_cache.add_multiple(dns_aaaa, dns_qry)
        # CNAMEがあればそのドメインもAレコードIPに紐付け
        if dns_cname and dns_a:
            dns_cache.add_multiple(dns_a, dns_cname)

    # SNI/ドメイン決定: tls_sni -> gquic_sni -> DNSキャッシュ逆引き
    sni = tls_sni or gquic_sni or None

    # SNIがなくTCP SYNの場合、キャッシュから逆引き
    resolved_domain = None
    if not sni and not dns_qry and dst_ip and dns_cache:
        resolved_domain = dns_cache.get(dst_ip)
        # キャッシュになければPTR逆引きをキューに追加（上限付き、溢れたら諦め）
        if not resolved_domain and ptr_queue and ptr_pending is not None:
            if dst_ip not in ptr_pending:
                try:
                    ptr_queue.put_nowait(dst_ip)
                    ptr_pending.add(dst_ip)
                except asyncio.QueueFull:
                    pass  # キュー満杯なら諦める（またの機会に）

    # 脅威インテリジェンス判定
    threat_flag = None
    if threat_intel:
        check_domain = http_host or sni or dns_qry or resolved_domain
        threat_flag = threat_intel.check(domain=check_domain, ip=dst_ip)

    event = {
        "id": generate_event_id(room_no, tz),
        "epoch": epoch,
        "time": iso_time,
        "room_no": room_no,
        "src_ip": src_ip,
        "dst_ip": dst_ip,
        "dst_port": dst_port,
        "protocol": protocol,
        "dns_qry": dns_qry or None,
        "http_host": http_host or None,
        "http_uri": http_uri or None,
        "tls_sni": sni,
        "resolved_domain": resolved_domain,
        "threat": threat_flag,  # malware/adult/gambling/fakenews/tor
    }
    return event


class CaptureFileLogger:
    """
    キャプチャイベントをファイルに保存
    - タイムスタンプベースのファイル名
    - 最大10MB、古いファイルを自動削除
    """

    def __init__(
        self,
        log_dir: str = DEFAULT_LOG_DIR,
        max_file_size: int = DEFAULT_MAX_FILE_SIZE,
        max_total_size: int = DEFAULT_MAX_TOTAL_SIZE,
        tz: ZoneInfo = None,
    ):
        self.log_dir = Path(log_dir)
        self.max_file_size = max_file_size
        self.max_total_size = max_total_size
        self.tz = tz or ZoneInfo("Asia/Tokyo")
        self.current_file: Optional[Path] = None
        self.current_size: int = 0
        self._ensure_dir()

    def _ensure_dir(self) -> None:
        """ログディレクトリ作成"""
        self.log_dir.mkdir(parents=True, exist_ok=True)

    def _new_file_path(self) -> Path:
        """新しいログファイルパスを生成"""
        now = datetime.now(self.tz)
        ts = now.strftime("%Y%m%d_%H%M%S")
        return self.log_dir / f"capture_{ts}.ndjson"

    def _get_log_files(self) -> List[Path]:
        """ログファイル一覧を取得（古い順）"""
        files = list(self.log_dir.glob("capture_*.ndjson"))
        files.sort(key=lambda f: f.stat().st_mtime)
        return files

    def _total_size(self) -> int:
        """ログファイルの合計サイズ"""
        return sum(f.stat().st_size for f in self._get_log_files())

    def _cleanup_old_files(self) -> None:
        """古いファイルを削除して最大サイズ以下に"""
        while self._total_size() > self.max_total_size:
            files = self._get_log_files()
            if not files:
                break
            oldest = files[0]
            try:
                oldest.unlink()
                logger.info("Deleted old capture log: %s", oldest.name)
            except Exception as e:
                logger.warning("Failed to delete %s: %s", oldest, e)
                break

    def _rotate_if_needed(self) -> None:
        """必要に応じてファイルローテーション"""
        if self.current_file is None or self.current_size >= self.max_file_size:
            self.current_file = self._new_file_path()
            self.current_size = 0
            logger.info("New capture log file: %s", self.current_file.name)
            # 古いファイルをクリーンアップ
            self._cleanup_old_files()

    def write_event(self, event: Dict[str, Any]) -> bool:
        """イベントをログファイルに書き込み"""
        try:
            self._rotate_if_needed()
            line = json.dumps(event, ensure_ascii=False) + "\n"
            line_bytes = line.encode("utf-8")

            with self.current_file.open("a", encoding="utf-8") as f:
                f.write(line)

            self.current_size += len(line_bytes)
            return True
        except Exception as e:
            logger.error("Failed to write capture log: %s", e)
            return False

    def get_stats(self) -> Dict[str, Any]:
        """ログ統計情報を取得"""
        files = self._get_log_files()
        total = self._total_size()
        return {
            "file_count": len(files),
            "total_size_bytes": total,
            "total_size_mb": round(total / 1024 / 1024, 2),
            "current_file": self.current_file.name if self.current_file else None,
            "current_file_size": self.current_size,
            "log_dir": str(self.log_dir),
        }


class CaptureManager:
    """tsharkキャプチャ管理クラス"""

    DISPLAY_BUFFER_SIZE = 1000  # 表示用バッファサイズ

    def __init__(self, watch_config_path: str = "/etc/is20s/watch_config.json"):
        self.config_path = watch_config_path
        self.cfg = load_watch_config(watch_config_path)
        self.tz = ZoneInfo(self.cfg.get("timezone", "Asia/Tokyo"))
        self.process: Optional[asyncio.subprocess.Process] = None
        self.running = False
        self.queue: List[Dict[str, Any]] = []  # 送信用キュー
        self.display_buffer: List[Dict[str, Any]] = []  # 表示用リングバッファ（常時保持）
        self.drop_count = 0
        self.sent_count = 0
        self.last_send_ok_at: Optional[str] = None
        self.last_error: Optional[str] = None
        self._read_task: Optional[asyncio.Task] = None
        self._flush_task: Optional[asyncio.Task] = None
        self._ptr_task: Optional[asyncio.Task] = None  # PTR逆引きワーカー
        self._stop_event = asyncio.Event()
        # ファイルロガー初期化
        self.file_logger: Optional[CaptureFileLogger] = None
        self._init_file_logger()
        # DNSキャッシュ初期化
        self.dns_cache = DnsCache()
        self._dns_save_counter = 0
        # PTR逆引きキュー（上限付き、溢れたら諦める）
        self._ptr_queue: asyncio.Queue = asyncio.Queue(maxsize=PTR_QUEUE_MAX_SIZE)
        self._ptr_pending: Set[str] = set()  # 処理中IP（重複リクエスト防止）
        self._ptr_resolved = 0  # 解決成功数
        self._ptr_failed = 0    # 解決失敗数
        # 脅威インテリジェンス
        self.threat_intel = get_threat_intel()
        # ASN検索
        self.asn_lookup = get_asn_lookup()
        self._asn_task: Optional[asyncio.Task] = None
        # 展開済みrooms（セグメント指定時に使用）
        self._expanded_rooms: Dict[str, str] = {}

    def _init_file_logger(self) -> None:
        """ファイルロガーを初期化"""
        log_cfg = self.cfg.get("logging", {})
        if log_cfg.get("enabled", True):
            self.file_logger = CaptureFileLogger(
                log_dir=log_cfg.get("log_dir", DEFAULT_LOG_DIR),
                max_file_size=log_cfg.get("max_file_size", DEFAULT_MAX_FILE_SIZE),
                max_total_size=log_cfg.get("max_total_size", DEFAULT_MAX_TOTAL_SIZE),
                tz=self.tz,
            )
            logger.info("File logging enabled: %s", self.file_logger.log_dir)
        else:
            self.file_logger = None
            logger.info("File logging disabled")

    def reload_config(self) -> None:
        """設定を再読み込み"""
        self.cfg = load_watch_config(self.config_path)
        self.tz = ZoneInfo(self.cfg.get("timezone", "Asia/Tokyo"))
        self._init_file_logger()

    def get_status(self) -> Dict[str, Any]:
        """現在のステータスを返す"""
        rooms = self.cfg.get("rooms", {})
        expanded_count = len(self._expanded_rooms) if self._expanded_rooms else len(rooms)
        # セグメント指定があるか判定
        has_segment = any(ip.endswith(".0") for ip in rooms.keys())
        status = {
            "running": self.running,
            "enabled": self.cfg.get("mode", {}).get("enabled", False),
            "queue_size": len(self.queue),
            "drop_count": self.drop_count,
            "sent_count": self.sent_count,
            "last_send_ok_at": self.last_send_ok_at,
            "last_error": self.last_error,
            "rooms_count": len(rooms),
            "rooms_expanded_count": expanded_count,
            "has_segment_spec": has_segment,
            "dry_run": self.cfg.get("mode", {}).get("dry_run", False),
            "syn_only": self.cfg.get("capture", {}).get("syn_only", True),
        }
        # ファイルログ統計を追加
        if self.file_logger:
            status["file_log"] = self.file_logger.get_stats()
        # DNSキャッシュ統計を追加
        status["dns_cache"] = self.dns_cache.get_stats()
        # PTR逆引き統計を追加
        status["ptr"] = {
            "queue_size": self._ptr_queue.qsize(),
            "pending": len(self._ptr_pending),
            "resolved": self._ptr_resolved,
            "failed": self._ptr_failed,
        }
        # 脅威インテリジェンス統計を追加
        status["threat_intel"] = self.threat_intel.get_stats()
        # ASN統計を追加
        status["asn"] = self.asn_lookup.get_stats()
        return status

    async def check_prerequisites(self) -> List[str]:
        """
        起動前チェック: eth0にIPがないか、デフォルトルートがwlan0か
        """
        warnings = []

        # eth0のIPチェック
        try:
            result = subprocess.run(
                ["ip", "-4", "addr", "show", "eth0"],
                capture_output=True, text=True, timeout=5
            )
            if "inet " in result.stdout:
                warnings.append("WARNING: eth0にIPv4アドレスが設定されています。キャプチャ専用のためIPを外してください。")
        except Exception as e:
            warnings.append(f"WARNING: eth0チェック失敗: {e}")

        # デフォルトルートチェック
        try:
            result = subprocess.run(
                ["ip", "route", "show", "default"],
                capture_output=True, text=True, timeout=5
            )
            if "eth0" in result.stdout and "wlan0" not in result.stdout:
                warnings.append("WARNING: デフォルトルートがeth0を使用しています。wlan0経由にしてください。")
        except Exception as e:
            warnings.append(f"WARNING: ルートチェック失敗: {e}")

        return warnings

    async def start(self) -> bool:
        """キャプチャ開始"""
        if self.running:
            logger.warning("Capture already running")
            return False

        self.reload_config()

        if not self.cfg.get("mode", {}).get("enabled", False):
            logger.info("Capture is disabled in config")
            return False

        rooms = self.cfg.get("rooms", {})
        if not rooms:
            logger.warning("No rooms configured, capture will not start")
            return False

        # セグメント展開（末尾0 → 1-254）して保持
        self._expanded_rooms = expand_segment_ips(rooms)
        logger.info("Rooms: %d entries -> %d IPs (after segment expansion)",
                    len(rooms), len(self._expanded_rooms))

        # 前提チェック
        warnings = await self.check_prerequisites()
        for w in warnings:
            logger.warning(w)

        # BPF構築
        bpf = build_bpf_filter(rooms, self.cfg)
        logger.info("BPF filter: %s", bpf)

        # tsharkコマンド構築
        cmd = build_tshark_cmd(self.cfg, bpf)
        logger.info("Starting tshark: %s", " ".join(cmd))

        try:
            self.process = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            self.running = True
            self._stop_event.clear()

            # 読み取りタスク開始
            self._read_task = asyncio.create_task(self._read_stdout())
            # フラッシュタスク開始
            self._flush_task = asyncio.create_task(self._flush_loop())
            # PTR逆引きワーカー開始
            self._ptr_task = asyncio.create_task(self._ptr_worker())

            logger.info("Capture started, pid=%s", self.process.pid)
            return True
        except Exception as e:
            logger.error("Failed to start tshark: %s", e)
            self.last_error = str(e)
            return False

    async def stop(self) -> None:
        """キャプチャ停止"""
        if not self.running:
            return

        logger.info("Stopping capture...")
        self._stop_event.set()

        if self.process:
            try:
                self.process.terminate()
                await asyncio.wait_for(self.process.wait(), timeout=5)
            except asyncio.TimeoutError:
                self.process.kill()
            except Exception as e:
                logger.warning("Error stopping tshark: %s", e)
            self.process = None

        if self._read_task:
            self._read_task.cancel()
            try:
                await self._read_task
            except asyncio.CancelledError:
                pass
            self._read_task = None

        if self._flush_task:
            self._flush_task.cancel()
            try:
                await self._flush_task
            except asyncio.CancelledError:
                pass
            self._flush_task = None

        if self._ptr_task:
            self._ptr_task.cancel()
            try:
                await self._ptr_task
            except asyncio.CancelledError:
                pass
            self._ptr_task = None

        self.running = False
        # DNSキャッシュを保存
        self.dns_cache.save()
        # ASNキャッシュを保存
        self.asn_lookup.save()
        logger.info("Capture stopped")

    async def _ptr_worker(self) -> None:
        """PTR逆引きワーカー: キューからIPを取得し、非同期で逆引き"""
        logger.info("PTR worker started")
        while not self._stop_event.is_set():
            try:
                # タイムアウト付きで待機
                ip = await asyncio.wait_for(
                    self._ptr_queue.get(),
                    timeout=1.0
                )
                try:
                    # 逆引き実行（ブロッキング処理をexecutorで実行）
                    loop = asyncio.get_event_loop()
                    try:
                        result = await asyncio.wait_for(
                            loop.run_in_executor(
                                None,
                                lambda: socket.gethostbyaddr(ip)
                            ),
                            timeout=PTR_LOOKUP_TIMEOUT
                        )
                        hostname = result[0]
                        # DNSキャッシュに追加
                        self.dns_cache.add(ip, hostname)
                        self._ptr_resolved += 1
                        logger.debug("PTR resolved: %s -> %s", ip, hostname)
                    except (socket.herror, socket.gaierror, asyncio.TimeoutError):
                        # 逆引き失敗（正常ケース、多くのIPは逆引きできない）
                        self._ptr_failed += 1
                    except Exception as e:
                        logger.debug("PTR lookup error for %s: %s", ip, e)
                        self._ptr_failed += 1
                finally:
                    # 処理完了、pendingから削除
                    self._ptr_pending.discard(ip)

                # 処理間隔
                await asyncio.sleep(PTR_WORKER_INTERVAL)
            except asyncio.TimeoutError:
                continue
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error("PTR worker error: %s", e)
        logger.info("PTR worker stopped")

    async def restart(self) -> bool:
        """再起動（設定変更反映）"""
        await self.stop()
        return await self.start()

    async def _read_stdout(self) -> None:
        """tshark出力を読み取り、イベント化"""
        if not self.process or not self.process.stdout:
            return

        # 展開済みrooms（セグメント指定対応）を使用
        rooms = self._expanded_rooms if self._expanded_rooms else self.cfg.get("rooms", {})
        max_queue = self.cfg.get("post", {}).get("max_queue_events", 20000)

        while not self._stop_event.is_set():
            try:
                line = await asyncio.wait_for(
                    self.process.stdout.readline(),
                    timeout=1.0
                )
                if not line:
                    # EOF
                    if self.process.returncode is not None:
                        logger.warning("tshark exited with code %s", self.process.returncode)
                        break
                    continue

                decoded = line.decode("utf-8", errors="ignore")
                event = parse_tshark_line(
                    decoded, rooms, self.tz, self.dns_cache,
                    self._ptr_queue, self._ptr_pending, self.threat_intel
                )
                if event:
                    # ASNエンリッチメント（dst_ipからサービス判定）
                    dst_ip = event.get("dst_ip")
                    if dst_ip:
                        try:
                            svc, cat, asn = await self.asn_lookup.lookup_service(dst_ip)
                            if asn:
                                event["asn"] = asn
                                event["asn_service"] = svc
                                event["asn_category"] = cat
                        except Exception as e:
                            logger.debug("ASN lookup error for %s: %s", dst_ip, e)
                    # 送信用キュー追加
                    if len(self.queue) >= max_queue:
                        self.queue.pop(0)
                        self.drop_count += 1
                    self.queue.append(event)
                    # 表示用バッファ追加（常時保持、送信に影響されない）
                    self.display_buffer.append(event)
                    if len(self.display_buffer) > self.DISPLAY_BUFFER_SIZE:
                        self.display_buffer.pop(0)
                    # ファイルログ書き込み
                    if self.file_logger:
                        self.file_logger.write_event(event)
                    # DNSキャッシュ定期保存（100イベントごと）
                    self._dns_save_counter += 1
                    if self._dns_save_counter >= 100:
                        self.dns_cache.save()
                        self._dns_save_counter = 0
            except asyncio.TimeoutError:
                continue
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error("Error reading tshark output: %s", e)

    async def _flush_loop(self) -> None:
        """定期的にバッチ送信"""
        post_cfg = self.cfg.get("post", {})
        batch_max_events = post_cfg.get("batch_max_events", 500)
        batch_max_seconds = post_cfg.get("batch_max_seconds", 30)

        last_flush = asyncio.get_event_loop().time()

        while not self._stop_event.is_set():
            try:
                await asyncio.sleep(1)
                now = asyncio.get_event_loop().time()
                elapsed = now - last_flush

                should_flush = (
                    len(self.queue) >= batch_max_events or
                    (len(self.queue) > 0 and elapsed >= batch_max_seconds)
                )

                if should_flush:
                    await self._flush_batch()
                    last_flush = now
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error("Error in flush loop: %s", e)

    async def _flush_batch(self) -> None:
        """バッチ送信"""
        if not self.queue:
            return

        post_cfg = self.cfg.get("post", {})
        url = post_cfg.get("url", "")
        dry_run = self.cfg.get("mode", {}).get("dry_run", False)

        batch_size = min(len(self.queue), post_cfg.get("batch_max_events", 500))
        batch = self.queue[:batch_size]

        if dry_run or not url:
            logger.info("[DRY_RUN] Would send %d events to %s", len(batch), url or "(no url)")
            self.queue = self.queue[batch_size:]
            self.sent_count += len(batch)
            return

        # NDJSON作成
        ndjson = "\n".join([json.dumps(e, ensure_ascii=False) for e in batch])
        body = ndjson.encode("utf-8")

        headers = {
            "Content-Type": "application/x-ndjson; charset=utf-8",
            **post_cfg.get("headers", {}),
        }

        # gzip圧縮
        if post_cfg.get("gzip", True):
            body = gzip.compress(body)
            headers["Content-Encoding"] = "gzip"

        try:
            async with httpx.AsyncClient() as client:
                resp = await client.post(url, content=body, headers=headers, timeout=30)
                if resp.status_code // 100 == 2:
                    self.queue = self.queue[batch_size:]
                    self.sent_count += len(batch)
                    self.last_send_ok_at = datetime.now(self.tz).isoformat()
                    logger.info("Sent %d events, status=%d", len(batch), resp.status_code)
                else:
                    self.last_error = f"HTTP {resp.status_code}: {resp.text[:200]}"
                    logger.error("Send failed: %s", self.last_error)
        except Exception as e:
            self.last_error = str(e)
            logger.error("Send error: %s", e)

    async def flush_remaining(self) -> None:
        """残りのキューを送信（graceful shutdown用）"""
        if self.queue:
            logger.info("Flushing remaining %d events...", len(self.queue))
            await self._flush_batch()


__all__ = [
    "CaptureManager",
    "load_watch_config",
    "save_watch_config",
    "DEFAULT_WATCH_CONFIG",
]
