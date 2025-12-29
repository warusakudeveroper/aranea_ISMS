"""
CelestialGlobe Ingest Client for IS-20S

IS-20S統合 最終版合意連携仕様書 Phase A 実装
- NDJSON形式でトラフィックデータを送信
- X-Aranea-* ヘッダーによる認証
- バッチ送信（5分間隔、最大10,000行）
- 指数バックオフによるリトライ

Note: ridキーはlacisOathのuserObjectスキーマ共通仕様に準拠
Note: CICはaraneaDeviceGateから取得（自己生成ではない）
"""

import asyncio
import gzip
import json
import logging
import time
import uuid
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

import httpx

logger = logging.getLogger(__name__)

# データディレクトリ
DATA_DIR = Path("/var/lib/is20s")
INGEST_CONFIG_FILE = DATA_DIR / "ingest_config.json"
IP_ROOM_MAPPING_FILE = DATA_DIR / "ip_room_mapping.json"
DOMAIN_SERVICES_FILE = DATA_DIR / "domain_services.json"


@dataclass
class IngestConfig:
    """Ingest設定"""
    # エンドポイント
    endpoint_url: str = "https://us-central1-mobesorder.cloudfunctions.net/celestialGlobe_ingest"
    fid: str = ""

    # デバイス認証情報（araneaDeviceGateから取得）
    lacis_id: str = ""
    mac_address: str = ""  # 正規化済み12文字 (例: AABBCCDDEEFF)
    cic: str = ""          # CIC（araneaDeviceGateから取得、自己生成ではない）
    state_endpoint: str = ""   # deviceStateReport URL
    mqtt_endpoint: str = ""    # MQTT WebSocket URL（双方向デバイスのみ）

    # バッチ設定
    batch_interval_sec: int = 300  # 5分
    max_packets_per_batch: int = 10000
    max_payload_bytes: int = 5 * 1024 * 1024  # 5MB

    # リトライ設定
    max_retries: int = 5
    backoff_base_sec: float = 1.0
    backoff_max_sec: float = 30.0
    timeout_sec: int = 60

    # 有効化フラグ
    enabled: bool = False

    # 辞書バージョン
    dict_version: str = ""

    # 登録済みフラグ
    registered: bool = False

    def to_dict(self) -> Dict[str, Any]:
        return {
            "endpoint_url": self.endpoint_url,
            "fid": self.fid,
            "lacis_id": self.lacis_id,
            "mac_address": self.mac_address,
            "cic": self.cic,
            "state_endpoint": self.state_endpoint,
            "mqtt_endpoint": self.mqtt_endpoint,
            "batch_interval_sec": self.batch_interval_sec,
            "max_packets_per_batch": self.max_packets_per_batch,
            "max_payload_bytes": self.max_payload_bytes,
            "max_retries": self.max_retries,
            "backoff_base_sec": self.backoff_base_sec,
            "backoff_max_sec": self.backoff_max_sec,
            "timeout_sec": self.timeout_sec,
            "enabled": self.enabled,
            "dict_version": self.dict_version,
            "registered": self.registered,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "IngestConfig":
        return cls(
            endpoint_url=data.get("endpoint_url", cls.endpoint_url),
            fid=data.get("fid", ""),
            lacis_id=data.get("lacis_id", ""),
            mac_address=data.get("mac_address", ""),
            cic=data.get("cic", ""),
            state_endpoint=data.get("state_endpoint", ""),
            mqtt_endpoint=data.get("mqtt_endpoint", ""),
            batch_interval_sec=data.get("batch_interval_sec", 300),
            max_packets_per_batch=data.get("max_packets_per_batch", 10000),
            max_payload_bytes=data.get("max_payload_bytes", 5 * 1024 * 1024),
            max_retries=data.get("max_retries", 5),
            backoff_base_sec=data.get("backoff_base_sec", 1.0),
            backoff_max_sec=data.get("backoff_max_sec", 30.0),
            timeout_sec=data.get("timeout_sec", 60),
            enabled=data.get("enabled", False),
            dict_version=data.get("dict_version", ""),
            registered=data.get("registered", False),
        )


@dataclass
class Is20sTrafficPacket:
    """
    トラフィックパケット（最終仕様書準拠）

    Note: ridキーはlacisOathのuserObjectスキーマ共通仕様に準拠
    （仕様書上はroom_noだが、lacisOath系との整合性のためridを使用）
    """
    ts: str                          # ISO8601タイムスタンプ
    src_ip: str                      # 送信元IP
    dst_ip: str                      # 宛先IP
    port: int                        # 宛先ポート
    protocol: str                    # TCP or UDP
    rid: str                         # 部屋ID（lacisOath userObject準拠）

    # オプションフィールド
    domain: Optional[str] = None
    bytes: Optional[int] = None
    direction: str = "outbound"
    dns_type: Optional[str] = None

    # サマリー（デバイス側解決）
    summary: Optional[Dict[str, Any]] = None

    def to_dict(self) -> Dict[str, Any]:
        """NDJSON出力用の辞書に変換"""
        result = {
            "ts": self.ts,
            "src_ip": self.src_ip,
            "dst_ip": self.dst_ip,
            "port": self.port,
            "protocol": self.protocol,
            "rid": self.rid,  # lacisOath userObject準拠
        }

        if self.domain:
            result["domain"] = self.domain
        if self.bytes is not None:
            result["bytes"] = self.bytes
        if self.direction:
            result["direction"] = self.direction
        if self.dns_type:
            result["dns_type"] = self.dns_type
        if self.summary:
            result["summary"] = self.summary

        return result


class IpRoomMapper:
    """IP→部屋ID（rid）マッピング"""

    def __init__(self, mapping_file: Path = IP_ROOM_MAPPING_FILE):
        self.mapping_file = mapping_file
        self.mappings: Dict[str, str] = {}  # IP -> rid
        self.ranges: List[tuple] = []       # [(start_ip, end_ip, rid), ...]
        self._load()

    def _load(self) -> None:
        """マッピングファイルを読み込み"""
        if not self.mapping_file.exists():
            logger.info("IP room mapping file not found, using empty mapping")
            return

        try:
            with self.mapping_file.open() as f:
                data = json.load(f)

            # 個別IPマッピング
            if "mappings" in data:
                self.mappings = data["mappings"]

            # IPレンジマッピング
            if "ranges" in data:
                for range_spec, rid in data["ranges"].items():
                    if "-" in range_spec:
                        start, end = range_spec.split("-", 1)
                        self.ranges.append((
                            self._ip_to_int(start.strip()),
                            self._ip_to_int(end.strip()),
                            rid
                        ))

            logger.info("Loaded IP room mapping: %d direct, %d ranges",
                       len(self.mappings), len(self.ranges))
        except Exception as e:
            logger.warning("Failed to load IP room mapping: %s", e)

    def _ip_to_int(self, ip: str) -> int:
        """IPアドレスを整数に変換"""
        parts = ip.split(".")
        return (int(parts[0]) << 24) + (int(parts[1]) << 16) + (int(parts[2]) << 8) + int(parts[3])

    def get_rid(self, ip: str) -> str:
        """IPアドレスから部屋ID（rid）を取得"""
        # 直接マッピングをチェック
        if ip in self.mappings:
            return self.mappings[ip]

        # レンジマッピングをチェック
        ip_int = self._ip_to_int(ip)
        for start, end, rid in self.ranges:
            if start <= ip_int <= end:
                return rid

        # 見つからない場合はUNK
        return "UNK"

    def update(self, data: Dict[str, Any]) -> None:
        """マッピングを更新"""
        if "mappings" in data:
            self.mappings = data["mappings"]
        if "ranges" in data:
            self.ranges = []
            for range_spec, rid in data["ranges"].items():
                if "-" in range_spec:
                    start, end = range_spec.split("-", 1)
                    self.ranges.append((
                        self._ip_to_int(start.strip()),
                        self._ip_to_int(end.strip()),
                        rid
                    ))

        # ファイルに保存
        try:
            self.mapping_file.parent.mkdir(parents=True, exist_ok=True)
            with self.mapping_file.open("w") as f:
                json.dump(data, f, indent=2)
        except Exception as e:
            logger.error("Failed to save IP room mapping: %s", e)


class DomainServiceResolver:
    """ドメイン→サービス解決"""

    def __init__(self, services_file: Path = DOMAIN_SERVICES_FILE):
        self.services_file = services_file
        self.services: Dict[str, Dict[str, Any]] = {}
        self.dict_version: str = ""
        self._load()

    def _load(self) -> None:
        """サービス辞書を読み込み"""
        if not self.services_file.exists():
            logger.info("Domain services file not found")
            return

        try:
            with self.services_file.open() as f:
                data = json.load(f)

            self.services = data.get("services", data)
            self.dict_version = data.get("version", datetime.now().strftime("%Y.%m.%d"))
            logger.info("Loaded %d domain services, version=%s",
                       len(self.services), self.dict_version)
        except Exception as e:
            logger.warning("Failed to load domain services: %s", e)

    def resolve(self, domain: str) -> Optional[Dict[str, Any]]:
        """ドメインからサービス情報を解決"""
        if not domain:
            return None

        # 完全一致
        if domain in self.services:
            svc = self.services[domain]
            return {
                "service_id": svc.get("id", svc.get("name")),
                "service_category": svc.get("category", svc.get("type")),
                "threat_level": svc.get("threat_level", 0),
                "dict_version": self.dict_version,
            }

        # サブドメイン検索（末尾から順に）
        parts = domain.split(".")
        for i in range(1, len(parts)):
            parent = ".".join(parts[i:])
            if parent in self.services:
                svc = self.services[parent]
                return {
                    "service_id": svc.get("id", svc.get("name")),
                    "service_category": svc.get("category", svc.get("type")),
                    "threat_level": svc.get("threat_level", 0),
                    "dict_version": self.dict_version,
                }

        # 未知ドメイン
        return {
            "service_id": None,
            "service_category": "unknown",
            "threat_level": 1,  # 未知は要注意
            "dict_version": self.dict_version,
        }


class BatchCollector:
    """バッチコレクター（5分間隔、最大10,000パケット）"""

    def __init__(
        self,
        interval_sec: int = 300,
        max_packets: int = 10000,
        max_bytes: int = 5 * 1024 * 1024
    ):
        self.interval_sec = interval_sec
        self.max_packets = max_packets
        self.max_bytes = max_bytes
        self.packets: List[Dict[str, Any]] = []
        self.last_flush = time.time()
        self._current_size = 0

    def add(self, packet: Is20sTrafficPacket) -> Optional[List[Dict[str, Any]]]:
        """
        パケットを追加。フラッシュ条件を満たしたらバッチを返す。
        """
        packet_dict = packet.to_dict()
        packet_size = len(json.dumps(packet_dict))

        self.packets.append(packet_dict)
        self._current_size += packet_size

        # フラッシュ条件チェック
        if self._should_flush():
            return self.flush()

        return None

    def _should_flush(self) -> bool:
        """フラッシュ条件をチェック"""
        if len(self.packets) >= self.max_packets:
            return True
        if self._current_size >= self.max_bytes:
            return True
        if time.time() - self.last_flush >= self.interval_sec:
            return True
        return False

    def flush(self) -> List[Dict[str, Any]]:
        """バッチをフラッシュして返す"""
        result = self.packets
        self.packets = []
        self._current_size = 0
        self.last_flush = time.time()
        return result

    def pending_count(self) -> int:
        """保留中のパケット数"""
        return len(self.packets)

    def force_flush_if_pending(self) -> Optional[List[Dict[str, Any]]]:
        """保留パケットがあればフラッシュ"""
        if self.packets:
            return self.flush()
        return None


@dataclass
class TenantPrimaryAuth:
    """テナントPrimary認証情報（lacisOath 3要素）"""
    lacis_id: str    # テナントPrimaryのlacisId
    user_id: str     # テナントPrimaryのuserId（email）
    cic: str         # テナントPrimaryのCIC


@dataclass
class AraneaRegisterResult:
    """araneaDeviceGate登録結果"""
    ok: bool = False
    cic_code: str = ""           # 6桁CIC
    state_endpoint: str = ""     # deviceStateReport URL
    mqtt_endpoint: str = ""      # MQTT WebSocket URL（双方向デバイスのみ）
    error: str = ""


class AraneaRegister:
    """
    araneaDeviceGate APIを使用してデバイスをmobes2.0に登録
    登録成功時にCICを取得し、ファイルに保存

    【AraneaRegister.cpp と同等のPython実装】
    """

    DEFAULT_GATE_URL = "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate"
    REGISTER_FILE = DATA_DIR / "aranea_register.json"

    def __init__(self, gate_url: str = ""):
        self.gate_url = gate_url or self.DEFAULT_GATE_URL
        self.tenant_auth: Optional[TenantPrimaryAuth] = None
        self._saved_data: Dict[str, Any] = {}
        self._load()

    def _load(self) -> None:
        """保存済みデータを読み込み"""
        if self.REGISTER_FILE.exists():
            try:
                with self.REGISTER_FILE.open() as f:
                    self._saved_data = json.load(f)
                logger.info("[ARANEA] Loaded saved registration data")
            except Exception as e:
                logger.warning("[ARANEA] Failed to load registration data: %s", e)

    def _save(self) -> None:
        """データをファイルに保存"""
        try:
            self.REGISTER_FILE.parent.mkdir(parents=True, exist_ok=True)
            with self.REGISTER_FILE.open("w") as f:
                json.dump(self._saved_data, f, indent=2)
        except Exception as e:
            logger.error("[ARANEA] Failed to save registration data: %s", e)

    def set_tenant_primary(self, auth: TenantPrimaryAuth) -> None:
        """テナントPrimary認証情報を設定"""
        self.tenant_auth = auth

    def is_registered(self) -> bool:
        """登録済みかチェック"""
        return self._saved_data.get("registered", False)

    def get_saved_cic(self) -> str:
        """保存されたCICを取得"""
        return self._saved_data.get("cic", "")

    def get_saved_state_endpoint(self) -> str:
        """保存されたstateEndpointを取得"""
        return self._saved_data.get("state_endpoint", "")

    def get_saved_mqtt_endpoint(self) -> str:
        """保存されたmqttEndpointを取得"""
        return self._saved_data.get("mqtt_endpoint", "")

    def clear_registration(self) -> None:
        """登録データをクリア（再登録を強制）"""
        self._saved_data = {}
        if self.REGISTER_FILE.exists():
            self.REGISTER_FILE.unlink()
        logger.info("[ARANEA] Registration cleared")

    async def register_device(
        self,
        tid: str,
        device_type: str,
        lacis_id: str,
        mac_address: str,
        product_type: str,
        product_code: str,
    ) -> AraneaRegisterResult:
        """
        デバイス登録

        Args:
            tid: テナントID
            device_type: デバイスタイプ (例: "aranea_ar-is20s")
            lacis_id: デバイスのlacisId
            mac_address: MACアドレス (12桁HEX)
            product_type: プロダクトタイプ (3桁)
            product_code: プロダクトコード (4桁)

        Returns:
            AraneaRegisterResult
        """
        result = AraneaRegisterResult()

        # 既に登録済みの場合は保存データを返す
        if self.is_registered():
            saved_cic = self.get_saved_cic()
            if saved_cic:
                result.ok = True
                result.cic_code = saved_cic
                result.state_endpoint = self.get_saved_state_endpoint()
                result.mqtt_endpoint = self.get_saved_mqtt_endpoint()
                logger.info("[ARANEA] Already registered, using saved CIC: %s", saved_cic)
                return result
            else:
                # CICが空の場合は再登録を試行
                logger.info("[ARANEA] Registered flag set but CIC is empty, re-registering...")
                self.clear_registration()

        if not self.tenant_auth:
            result.error = "Tenant primary auth not set"
            return result

        # JSON構築（AraneaRegister.cppと同じ形式）
        payload = {
            "lacisOath": {
                "lacisId": self.tenant_auth.lacis_id,
                "userId": self.tenant_auth.user_id,
                "cic": self.tenant_auth.cic,
                "method": "register",
            },
            "userObject": {
                "lacisID": lacis_id,
                "tid": tid,
                "typeDomain": "araneaDevice",
                "type": device_type,
            },
            "deviceMeta": {
                "macAddress": mac_address,
                "productType": product_type,
                "productCode": product_code,
            },
        }

        logger.info("[ARANEA] Registering device...")
        logger.info("[ARANEA] URL: %s", self.gate_url)
        logger.debug("[ARANEA] Payload: %s", json.dumps(payload))

        # HTTP POST
        try:
            async with httpx.AsyncClient(timeout=15.0) as client:
                resp = await client.post(
                    self.gate_url,
                    json=payload,
                    headers={"Content-Type": "application/json"},
                )

                logger.info("[ARANEA] Response code: %d", resp.status_code)
                logger.debug("[ARANEA] Response body: %s", resp.text)

                if resp.status_code in (200, 201):
                    data = resp.json()
                    if data.get("ok"):
                        result.ok = True
                        result.cic_code = data.get("userObject", {}).get("cic_code", "")
                        result.state_endpoint = data.get("stateEndpoint", "")
                        result.mqtt_endpoint = data.get("mqttEndpoint", "")

                        # ファイルに保存
                        self._saved_data = {
                            "cic": result.cic_code,
                            "state_endpoint": result.state_endpoint,
                            "mqtt_endpoint": result.mqtt_endpoint,
                            "registered": True,
                        }
                        self._save()

                        logger.info("[ARANEA] Registered! CIC: %s", result.cic_code)
                        logger.info("[ARANEA] State endpoint: %s", result.state_endpoint)
                        if result.mqtt_endpoint:
                            logger.info("[ARANEA] MQTT endpoint: %s", result.mqtt_endpoint)
                    else:
                        result.error = data.get("error", "Unknown error")
                        logger.error("[ARANEA] Registration failed: %s", result.error)

                elif resp.status_code == 409:
                    result.error = "Device already registered (409)"
                    logger.warning("[ARANEA] Device already registered (conflict)")

                else:
                    try:
                        data = resp.json()
                        result.error = data.get("error", f"HTTP {resp.status_code}")
                    except Exception:
                        result.error = f"HTTP {resp.status_code}: {resp.text}"
                    logger.error("[ARANEA] Error %d: %s", resp.status_code, result.error)

        except Exception as e:
            result.error = f"HTTP error: {e}"
            logger.error("[ARANEA] HTTP error: %s", e)

        return result


class IngestClient:
    """
    CelestialGlobe Ingest Client

    IS-20S統合 最終版合意連携仕様書 Phase A 実装
    """

    def __init__(self, config: IngestConfig):
        self.config = config
        self.batch_collector = BatchCollector(
            interval_sec=config.batch_interval_sec,
            max_packets=config.max_packets_per_batch,
            max_bytes=config.max_payload_bytes,
        )
        self.ip_mapper = IpRoomMapper()
        self.service_resolver = DomainServiceResolver()

        self._http_client: Optional[httpx.AsyncClient] = None
        self._worker: Optional[asyncio.Task] = None
        self._stop_event = asyncio.Event()
        self._send_queue: asyncio.Queue[List[Dict[str, Any]]] = asyncio.Queue()

        # 統計
        self.stats = {
            "packets_collected": 0,
            "batches_sent": 0,
            "batches_failed": 0,
            "last_send_at": None,
            "last_error": None,
        }

    async def start(self) -> None:
        """クライアントを開始"""
        if not self.config.enabled:
            logger.info("IngestClient is disabled")
            return

        self._stop_event.clear()
        self._http_client = httpx.AsyncClient(timeout=self.config.timeout_sec)
        self._worker = asyncio.create_task(self._run())
        logger.info("IngestClient started")

    async def stop(self) -> None:
        """クライアントを停止"""
        self._stop_event.set()

        # 残りのパケットをフラッシュ
        remaining = self.batch_collector.force_flush_if_pending()
        if remaining:
            await self._send_batch(remaining)

        if self._worker:
            self._worker.cancel()
            try:
                await self._worker
            except asyncio.CancelledError:
                pass

        if self._http_client:
            await self._http_client.aclose()

        logger.info("IngestClient stopped")

    async def add_packet(
        self,
        src_ip: str,
        dst_ip: str,
        port: int,
        protocol: str,
        domain: Optional[str] = None,
        bytes_count: Optional[int] = None,
        dns_type: Optional[str] = None,
    ) -> None:
        """パケットを追加"""
        if not self.config.enabled:
            return

        # 部屋ID解決
        rid = self.ip_mapper.get_rid(src_ip)

        # サービス解決
        summary = self.service_resolver.resolve(domain) if domain else None

        packet = Is20sTrafficPacket(
            ts=datetime.now(timezone.utc).isoformat(),
            src_ip=src_ip,
            dst_ip=dst_ip,
            port=port,
            protocol=protocol,
            rid=rid,
            domain=domain,
            bytes=bytes_count,
            direction="outbound",
            dns_type=dns_type,
            summary=summary,
        )

        self.stats["packets_collected"] += 1

        # バッチに追加
        batch = self.batch_collector.add(packet)
        if batch:
            await self._send_queue.put(batch)

    async def _run(self) -> None:
        """バックグラウンドワーカー"""
        while not self._stop_event.is_set():
            try:
                # キューからバッチを取得（タイムアウト付き）
                try:
                    batch = await asyncio.wait_for(
                        self._send_queue.get(),
                        timeout=self.config.batch_interval_sec
                    )
                    await self._send_batch(batch)
                except asyncio.TimeoutError:
                    # タイムアウト時は保留パケットをフラッシュ
                    batch = self.batch_collector.force_flush_if_pending()
                    if batch:
                        await self._send_batch(batch)
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error("IngestClient worker error: %s", e, exc_info=True)
                await asyncio.sleep(1)

    async def _send_batch(self, batch: List[Dict[str, Any]]) -> bool:
        """バッチを送信"""
        if not batch:
            return True

        # NDJSON生成
        ndjson_lines = [json.dumps(p, separators=(",", ":")) for p in batch]
        ndjson_body = "\n".join(ndjson_lines)

        # gzip圧縮
        compressed = gzip.compress(ndjson_body.encode())

        # CIC確認（araneaDeviceGateから取得済みのもの）
        if not self.config.cic:
            logger.error("CIC not set - device must be registered with araneaDeviceGate first")
            self.stats["batches_failed"] += 1
            self.stats["last_error"] = "CIC not set"
            return False

        # ヘッダー生成
        timestamp = datetime.now(timezone.utc).isoformat()
        nonce = str(uuid.uuid4())

        headers = {
            "Content-Type": "application/x-ndjson",
            "Content-Encoding": "gzip",
            "X-Aranea-LacisId": self.config.lacis_id,
            "X-Aranea-Mac": self.config.mac_address,
            "X-Aranea-CIC": self.config.cic,  # araneaDeviceGateから取得したCIC
            "X-Aranea-SourceType": "ar-is20s",
            "X-Aranea-Timestamp": timestamp,
            "X-Aranea-Nonce": nonce,
        }

        url = f"{self.config.endpoint_url}?fid={self.config.fid}&source=araneaDevice"

        # リトライ付き送信
        for attempt in range(self.config.max_retries):
            try:
                resp = await self._http_client.post(
                    url,
                    content=compressed,
                    headers=headers,
                )

                if resp.status_code == 200:
                    self.stats["batches_sent"] += 1
                    self.stats["last_send_at"] = timestamp
                    self.stats["last_error"] = None
                    logger.info("Batch sent: %d packets", len(batch))
                    return True

                elif resp.status_code == 429:
                    # レート制限
                    retry_after = int(resp.headers.get("Retry-After", 60))
                    logger.warning("Rate limited, waiting %d seconds", retry_after)
                    await asyncio.sleep(retry_after)
                    continue

                elif resp.status_code in (500, 502, 503, 504):
                    # サーバーエラー、リトライ
                    delay = min(
                        self.config.backoff_max_sec,
                        self.config.backoff_base_sec * (2 ** attempt)
                    )
                    logger.warning("Server error %d, retrying in %.1f seconds",
                                  resp.status_code, delay)
                    await asyncio.sleep(delay)
                    continue

                else:
                    # 400, 401, 403 などはリトライしない
                    error_msg = f"HTTP {resp.status_code}: {resp.text}"
                    logger.error("Batch send failed (no retry): %s", error_msg)
                    self.stats["batches_failed"] += 1
                    self.stats["last_error"] = error_msg
                    return False

            except Exception as e:
                delay = min(
                    self.config.backoff_max_sec,
                    self.config.backoff_base_sec * (2 ** attempt)
                )
                logger.warning("Send error: %s, retrying in %.1f seconds", e, delay)
                await asyncio.sleep(delay)

        # 全リトライ失敗
        self.stats["batches_failed"] += 1
        self.stats["last_error"] = "Max retries exceeded"
        logger.error("Batch send failed after %d retries", self.config.max_retries)
        return False

    def update_config(self, config: IngestConfig) -> None:
        """設定を更新"""
        self.config = config
        self.batch_collector = BatchCollector(
            interval_sec=config.batch_interval_sec,
            max_packets=config.max_packets_per_batch,
            max_bytes=config.max_payload_bytes,
        )

    def get_stats(self) -> Dict[str, Any]:
        """統計情報を取得"""
        return {
            **self.stats,
            "pending_packets": self.batch_collector.pending_count(),
            "enabled": self.config.enabled,
        }


# 設定ファイル管理
def load_ingest_config() -> IngestConfig:
    """設定ファイルから読み込み"""
    if INGEST_CONFIG_FILE.exists():
        try:
            with INGEST_CONFIG_FILE.open() as f:
                data = json.load(f)
            return IngestConfig.from_dict(data)
        except Exception as e:
            logger.warning("Failed to load ingest config: %s", e)
    return IngestConfig()


def save_ingest_config(config: IngestConfig) -> bool:
    """設定ファイルに保存"""
    try:
        INGEST_CONFIG_FILE.parent.mkdir(parents=True, exist_ok=True)
        with INGEST_CONFIG_FILE.open("w") as f:
            json.dump(config.to_dict(), f, indent=2)
        return True
    except Exception as e:
        logger.error("Failed to save ingest config: %s", e)
        return False


# シングルトン
_ingest_client: Optional[IngestClient] = None


def get_ingest_client() -> IngestClient:
    """IngestClientシングルトンを取得"""
    global _ingest_client
    if _ingest_client is None:
        config = load_ingest_config()
        _ingest_client = IngestClient(config)
    return _ingest_client


__all__ = [
    "IngestConfig",
    "IngestClient",
    "Is20sTrafficPacket",
    "IpRoomMapper",
    "DomainServiceResolver",
    "BatchCollector",
    "TenantPrimaryAuth",
    "AraneaRegisterResult",
    "AraneaRegister",
    "load_ingest_config",
    "save_ingest_config",
    "get_ingest_client",
]
