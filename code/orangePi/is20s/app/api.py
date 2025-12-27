import socket
from dataclasses import asdict
from pathlib import Path
from typing import Any, Dict, List, Callable, Optional

from fastapi import APIRouter, HTTPException, Request
from fastapi.responses import JSONResponse

from araneapi.sqlite_store import EventStore
from araneapi.system_info import system_status
from araneapi.config import AppConfig

from .ingest import handle_ingest
from .forwarder import Forwarder
from .state import RuntimeState
from .capture import CaptureManager, load_watch_config, save_watch_config
from .hardware_info import get_hardware_status_dict
from .system_config import (
    get_wifi_status,
    get_available_networks,
    connect_wifi,
    get_ntp_status,
    set_ntp_server,
    set_timezone,
    get_sync_manager,
    get_wifi_config_manager,
    MAX_WIFI_CONFIGS,
)


def _tail_log(log_path: Path, lines: int) -> List[str]:
    if not log_path.exists():
        return []
    content = log_path.read_text(errors="ignore").splitlines()
    return content[-lines:]


def _ip_allowed(client_ip: str, allow_list: List[str]) -> bool:
    if not allow_list:
        return True
    from ipaddress import ip_address, ip_network

    try:
        addr = ip_address(client_ip)
    except ValueError:
        return False

    for entry in allow_list:
        try:
            if "/" in entry:
                if addr in ip_network(entry, strict=False):
                    return True
            else:
                if addr == ip_address(entry):
                    return True
        except ValueError:
            continue
    return False


def create_router(
    store: EventStore,
    forwarder: Forwarder,
    state: RuntimeState,
    cfg: AppConfig,
    apply_config_callback,
    log_path_getter: Callable[[], Path],
    capture_manager: Optional[CaptureManager] = None,
    watch_config_path: Optional[str] = None,
) -> APIRouter:
    router = APIRouter()

    @router.get("/api/status")
    async def get_status() -> Dict[str, Any]:
        sys = system_status()
        status = store.get_status()
        result = {
            "ok": True,
            "hostname": sys.get("hostname"),
            "uptimeSec": int(sys.get("uptimeSec") or 0),
            "bluetooth": "ok",
            "db": "ok",
            **status,
            "device": {
                "name": cfg.device.name,
                "lacisId": state.lacis_id,
                "cic": state.cic,
                "productType": cfg.device.product_type,
                "productCode": cfg.device.product_code,
            },
            "forwarder": {
                "lastOkAt": state.last_forward_ok_at,
                "failures": state.forward_failures,
                "queued": forwarder.queue.qsize(),
            },
            "mqtt": {"connected": state.mqtt_connected},
        }
        # キャプチャステータス追加
        if capture_manager:
            result["capture"] = capture_manager.get_status()
        return result

    @router.get("/api/events")
    async def get_events(limit: int = 200, request: Request = None):
        if request:
            client_ip = request.client.host if request.client else ""
            if not _ip_allowed(client_ip, cfg.access.allowed_sources):
                raise HTTPException(status_code=403, detail="forbidden")
        return store.get_recent(limit)

    @router.post("/api/ingest")
    async def post_ingest(payload: Dict[str, Any], request: Request):
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        return await handle_ingest(payload, store, forwarder)

    @router.get("/api/config")
    async def get_config(request: Request):
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        return asdict(cfg)

    @router.post("/api/config")
    async def post_config(payload: Dict[str, Any], request: Request):
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        apply_result = await apply_config_callback(payload)
        return {"ok": True, **apply_result}

    @router.get("/api/logs")
    async def get_logs(request: Request, lines: int = 200):
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        lines = min(max(lines, 10), 2000)
        return {"lines": _tail_log(log_path_getter(), lines)}

    @router.post("/api/debug/publishSample")
    async def publish_sample():
        sample = {
            "seenAt": None,
            "src": "debug",
            "lacisId": state.lacis_id,
            "payload": {"debug": True},
        }
        store.enqueue(sample)
        await forwarder.enqueue(sample)
        return {"ok": True}

    # ========== キャプチャ制御API ==========

    @router.get("/api/capture/status")
    async def get_capture_status(request: Request):
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        if not capture_manager:
            return {"ok": False, "error": "capture not configured"}
        return {"ok": True, **capture_manager.get_status()}

    @router.post("/api/capture/start")
    async def capture_start(request: Request):
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        if not capture_manager:
            return {"ok": False, "error": "capture not configured"}
        success = await capture_manager.start()
        return {"ok": success, **capture_manager.get_status()}

    @router.post("/api/capture/stop")
    async def capture_stop(request: Request):
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        if not capture_manager:
            return {"ok": False, "error": "capture not configured"}
        await capture_manager.stop()
        return {"ok": True, **capture_manager.get_status()}

    @router.post("/api/capture/restart")
    async def capture_restart(request: Request):
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        if not capture_manager:
            return {"ok": False, "error": "capture not configured"}
        success = await capture_manager.restart()
        return {"ok": success, **capture_manager.get_status()}

    @router.get("/api/capture/config")
    async def get_capture_config(request: Request):
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        if not capture_manager:
            return {"ok": False, "error": "capture not configured"}
        return {"ok": True, "config": capture_manager.cfg}

    @router.post("/api/capture/config")
    async def set_capture_config(payload: Dict[str, Any], request: Request):
        """監視設定を更新（rooms, mode.enabled等）"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        if not capture_manager or not watch_config_path:
            return {"ok": False, "error": "capture not configured"}

        # 現在の設定をマージして保存
        current = capture_manager.cfg.copy()
        for key, val in payload.items():
            if isinstance(val, dict) and key in current and isinstance(current[key], dict):
                current[key] = {**current[key], **val}
            else:
                current[key] = val

        if save_watch_config(current, watch_config_path):
            capture_manager.reload_config()
            return {"ok": True, "config": capture_manager.cfg}
        else:
            return {"ok": False, "error": "failed to save config"}

    @router.post("/api/capture/rooms")
    async def set_capture_rooms(payload: Dict[str, str], request: Request):
        """rooms辞書を更新（IP→部屋番号マッピング）"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        if not capture_manager or not watch_config_path:
            return {"ok": False, "error": "capture not configured"}

        current = capture_manager.cfg.copy()
        current["rooms"] = payload

        if save_watch_config(current, watch_config_path):
            capture_manager.reload_config()
            # キャプチャ中なら再起動してBPF更新
            if capture_manager.running:
                await capture_manager.restart()
            return {"ok": True, "rooms": capture_manager.cfg.get("rooms", {})}
        else:
            return {"ok": False, "error": "failed to save config"}

    @router.get("/api/capture/events")
    async def get_capture_events(request: Request, limit: int = 100):
        """表示用バッファからイベントを取得（最新N件、送信に影響されない）"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        if not capture_manager:
            return {"ok": False, "error": "capture not configured", "events": []}
        limit = min(max(limit, 1), 500)
        # 表示用バッファから取得（送信キューとは独立）
        events = capture_manager.display_buffer[-limit:] if capture_manager.display_buffer else []
        return {
            "ok": True,
            "count": len(events),
            "total_queued": len(capture_manager.display_buffer),
            "events": events,
        }

    # ========== 脅威インテリジェンスAPI ==========

    @router.get("/api/threat/status")
    async def get_threat_status(request: Request):
        """脅威インテリジェンスの統計情報を取得"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        if not capture_manager:
            return {"ok": False, "error": "capture not configured"}
        return {"ok": True, **capture_manager.threat_intel.get_stats()}

    @router.post("/api/threat/update")
    async def update_threat_intel(request: Request):
        """脅威インテリジェンスデータを更新（StevenBlack/hosts, Tor等）"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        if not capture_manager:
            return {"ok": False, "error": "capture not configured"}
        result = await capture_manager.threat_intel.update(force=True)
        return {"ok": True, **result}

    # ========== ハードウェア情報API ==========

    @router.get("/api/hardware")
    async def get_hardware(request: Request):
        """ハードウェア情報を取得（RAM、CPU、温度、ストレージ、ネットワーク）"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        return {"ok": True, **get_hardware_status_dict()}

    # ========== WiFi設定API ==========

    @router.get("/api/wifi/status")
    async def wifi_status(request: Request):
        """WiFi接続状態を取得"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        return {"ok": True, **get_wifi_status()}

    @router.get("/api/wifi/networks")
    async def wifi_networks(request: Request):
        """利用可能なWiFiネットワーク一覧を取得"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        networks = get_available_networks()
        return {"ok": True, "networks": networks}

    @router.post("/api/wifi/connect")
    async def wifi_connect(payload: Dict[str, Any], request: Request):
        """WiFiに接続"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        ssid = payload.get("ssid", "")
        password = payload.get("password", "")
        if not ssid:
            return {"ok": False, "error": "SSID required"}
        result = await connect_wifi(ssid, password)
        return result

    # ========== WiFi複数設定API ==========

    @router.get("/api/wifi/configs")
    async def wifi_configs_list(request: Request):
        """WiFi設定一覧を取得（パスワードはマスク）"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        mgr = get_wifi_config_manager()
        return {
            "ok": True,
            "configs": mgr.get_all(),
            "max_configs": MAX_WIFI_CONFIGS,
        }

    @router.post("/api/wifi/configs")
    async def wifi_configs_add(payload: Dict[str, Any], request: Request):
        """WiFi設定を追加"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        ssid = payload.get("ssid", "")
        password = payload.get("password", "")
        if not ssid:
            return {"ok": False, "error": "SSID required"}
        mgr = get_wifi_config_manager()
        return mgr.add(ssid, password)

    @router.put("/api/wifi/configs/{index}")
    async def wifi_configs_update(index: int, payload: Dict[str, Any], request: Request):
        """WiFi設定を更新"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        ssid = payload.get("ssid")
        password = payload.get("password")
        mgr = get_wifi_config_manager()
        return mgr.update(index, ssid=ssid, password=password)

    @router.delete("/api/wifi/configs/{index}")
    async def wifi_configs_delete(index: int, request: Request):
        """WiFi設定を削除"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        mgr = get_wifi_config_manager()
        return mgr.delete(index)

    @router.post("/api/wifi/configs/reorder")
    async def wifi_configs_reorder(payload: Dict[str, Any], request: Request):
        """WiFi設定の優先順位を変更"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        old_index = payload.get("old_index")
        new_index = payload.get("new_index")
        if old_index is None or new_index is None:
            return {"ok": False, "error": "old_index and new_index required"}
        mgr = get_wifi_config_manager()
        return mgr.reorder(old_index, new_index)

    @router.post("/api/wifi/configs/reset")
    async def wifi_configs_reset(request: Request):
        """WiFi設定をデフォルトにリセット"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        mgr = get_wifi_config_manager()
        return mgr.reset_to_defaults()

    @router.post("/api/wifi/auto-connect")
    async def wifi_auto_connect(request: Request):
        """保存されたWiFi設定を順番に試行して接続"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        mgr = get_wifi_config_manager()
        return await mgr.auto_connect()

    # ========== NTP設定API ==========

    @router.get("/api/ntp/status")
    async def ntp_status(request: Request):
        """NTP同期状態を取得"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        return {"ok": True, **get_ntp_status()}

    @router.post("/api/ntp/server")
    async def ntp_set_server(payload: Dict[str, Any], request: Request):
        """NTPサーバーを設定"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        server = payload.get("server", "")
        if not server:
            return {"ok": False, "error": "Server required"}
        result = await set_ntp_server(server)
        return result

    @router.post("/api/ntp/timezone")
    async def ntp_set_timezone(payload: Dict[str, Any], request: Request):
        """タイムゾーンを設定"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        tz = payload.get("timezone", "")
        if not tz:
            return {"ok": False, "error": "Timezone required"}
        result = await set_timezone(tz)
        return result

    # ========== キャッシュ同期API ==========

    @router.get("/api/sync/status")
    async def sync_status(request: Request):
        """同期ステータスを取得"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        sync_mgr = get_sync_manager()
        return {"ok": True, **sync_mgr.get_status()}

    @router.post("/api/sync/config")
    async def sync_config(payload: Dict[str, Any], request: Request):
        """同期設定を更新（パスフレーズ、ピア等）"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")
        sync_mgr = get_sync_manager()

        if "initiator_passphrase" in payload or "responder_passphrase" in payload:
            sync_mgr.set_passphrases(
                payload.get("initiator_passphrase", sync_mgr.config["initiator_passphrase"]),
                payload.get("responder_passphrase", sync_mgr.config["responder_passphrase"]),
            )
        if "add_peer" in payload:
            peer = payload["add_peer"]
            sync_mgr.add_peer(peer.get("host", ""), peer.get("port", 8080))
        if "remove_peer" in payload:
            sync_mgr.remove_peer(payload["remove_peer"])

        return {"ok": True, **sync_mgr.get_status()}

    @router.get("/api/sync/export")
    async def sync_export(request: Request):
        """キャッシュをエクスポート"""
        client_ip = request.client.host if request.client else ""
        # 同期リクエストの認証チェック
        auth_header = request.headers.get("X-Sync-Auth", "")
        sync_mgr = get_sync_manager()
        if auth_header:
            # 同期リクエスト: パスフレーズ認証
            if not sync_mgr.verify_auth(auth_header):
                raise HTTPException(status_code=403, detail="Invalid sync auth")
        else:
            # 通常のAPIリクエスト: IP制限
            if not _ip_allowed(client_ip, cfg.access.allowed_sources):
                raise HTTPException(status_code=403, detail="forbidden")

        return sync_mgr.export_cache()

    @router.post("/api/sync/import")
    async def sync_import(payload: Dict[str, Any], request: Request):
        """キャッシュをインポート"""
        client_ip = request.client.host if request.client else ""
        auth_header = request.headers.get("X-Sync-Auth", "")
        sync_mgr = get_sync_manager()
        if auth_header:
            if not sync_mgr.verify_auth(auth_header):
                raise HTTPException(status_code=403, detail="Invalid sync auth")
        else:
            if not _ip_allowed(client_ip, cfg.access.allowed_sources):
                raise HTTPException(status_code=403, detail="forbidden")

        merge = payload.get("merge", True)
        result = sync_mgr.import_cache(payload, merge=merge)
        return result

    @router.post("/api/sync/peer")
    async def sync_with_peer(payload: Dict[str, Any], request: Request):
        """特定のピアと同期"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")

        host = payload.get("host", "")
        port = payload.get("port", 8080)
        if not host:
            return {"ok": False, "error": "Host required"}

        sync_mgr = get_sync_manager()
        result = await sync_mgr.sync_with_peer(host, port)
        return result

    @router.post("/api/sync/all")
    async def sync_all(request: Request):
        """すべてのピアと同期"""
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")

        sync_mgr = get_sync_manager()
        results = await sync_mgr.sync_all_peers()
        return {"ok": True, "results": results}

    # ===== SpeedDial API =====
    @router.get("/api/speeddial")
    async def get_speeddial(request: Request):
        """
        SpeedDial: 全設定をINI形式テキストで取得
        スマホでの一括コピー＆ペースト設定用
        """
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")

        lines = []

        # Device Info
        lines.append("[Device]")
        lines.append(f"name={cfg.device.name}")
        lines.append(f"lacisid={state.lacis_id or ''}")
        lines.append(f"hostname={socket.gethostname()}")
        lines.append("")

        # WiFi Settings
        lines.append("[WiFi]")
        wifi_mgr = get_wifi_config_manager()
        for i, c in enumerate(wifi_mgr.configs):
            ssid = c.get("ssid", "")
            pwd = c.get("password", "")
            lines.append(f"wifi{i+1}={ssid},{pwd}")
        lines.append("")

        # Capture Settings
        lines.append("[Capture]")
        watch_cfg = load_watch_config()
        cap_cfg = watch_cfg.get("capture", {})
        mode_cfg = watch_cfg.get("mode", {})
        lines.append(f"enabled={str(mode_cfg.get('enabled', True)).lower()}")
        lines.append(f"dry_run={str(mode_cfg.get('dry_run', True)).lower()}")
        lines.append(f"iface={cap_cfg.get('iface', 'end0')}")
        lines.append(f"syn_only={str(cap_cfg.get('syn_only', True)).lower()}")
        lines.append("")

        # NTP Settings
        lines.append("[NTP]")
        lines.append(f"server={watch_cfg.get('ntp_server', 'ntp.nict.jp')}")
        lines.append(f"timezone={watch_cfg.get('timezone', 'Asia/Tokyo')}")
        lines.append("")

        # Sync Settings
        lines.append("[Sync]")
        sync_mgr = get_sync_manager()
        peers = sync_mgr.config.get("peers", [])
        for i, peer in enumerate(peers):
            if isinstance(peer, dict):
                lines.append(f"peer{i+1}={peer.get('host', '')}:{peer.get('port', 8080)}")
            else:
                lines.append(f"peer{i+1}={peer}")
        lines.append("")

        # Post Settings
        lines.append("[Post]")
        post_cfg = watch_cfg.get("post", {})
        lines.append(f"url={post_cfg.get('url', '')}")
        lines.append(f"gzip={str(post_cfg.get('gzip', True)).lower()}")

        text = "\n".join(lines)
        return {"ok": True, "text": text, "format": "ini"}

    @router.post("/api/speeddial")
    async def set_speeddial(request: Request):
        """
        SpeedDial: INI形式テキストから設定を適用
        スマホでの一括貼り付け設定用
        """
        client_ip = request.client.host if request.client else ""
        if not _ip_allowed(client_ip, cfg.access.allowed_sources):
            raise HTTPException(status_code=403, detail="forbidden")

        body = await request.json()
        text = body.get("text", "")
        if not text:
            return {"ok": False, "error": "No text provided"}

        # Parse INI-style text
        current_section = None
        parsed = {}
        errors = []

        for line in text.strip().split("\n"):
            line = line.strip()
            if not line or line.startswith("#"):
                continue

            # Section header
            if line.startswith("[") and line.endswith("]"):
                current_section = line[1:-1]
                if current_section not in parsed:
                    parsed[current_section] = {}
                continue

            # Key=Value pair
            if "=" in line and current_section:
                key, _, value = line.partition("=")
                parsed[current_section][key.strip()] = value.strip()

        applied = []

        # Apply WiFi settings
        if "WiFi" in parsed:
            try:
                wifi_mgr = get_wifi_config_manager()
                new_configs = []
                for key, val in parsed["WiFi"].items():
                    if key.startswith("wifi") and "," in val:
                        parts = val.split(",", 1)
                        ssid = parts[0].strip()
                        pwd = parts[1].strip() if len(parts) > 1 else ""
                        if ssid:
                            new_configs.append({
                                "ssid": ssid,
                                "password": pwd,
                                "priority": len(new_configs)
                            })
                if new_configs:
                    wifi_mgr.configs = new_configs[:MAX_WIFI_CONFIGS]
                    wifi_mgr._save()
                    applied.append(f"WiFi: {len(new_configs)} configs")
            except Exception as e:
                errors.append(f"WiFi: {str(e)}")

        # Apply NTP settings
        if "NTP" in parsed:
            try:
                ntp_cfg = parsed["NTP"]
                if "server" in ntp_cfg:
                    from system_config import set_ntp_server
                    set_ntp_server(ntp_cfg["server"])
                    applied.append(f"NTP server: {ntp_cfg['server']}")
                if "timezone" in ntp_cfg:
                    from system_config import set_timezone
                    set_timezone(ntp_cfg["timezone"])
                    applied.append(f"Timezone: {ntp_cfg['timezone']}")
            except Exception as e:
                errors.append(f"NTP: {str(e)}")

        # Apply Capture settings
        if "Capture" in parsed:
            try:
                cap_cfg = parsed["Capture"]
                watch_cfg = load_watch_config()
                changed = False

                if "enabled" in cap_cfg:
                    watch_cfg.setdefault("mode", {})["enabled"] = cap_cfg["enabled"].lower() == "true"
                    changed = True
                if "dry_run" in cap_cfg:
                    watch_cfg.setdefault("mode", {})["dry_run"] = cap_cfg["dry_run"].lower() == "true"
                    changed = True
                if "iface" in cap_cfg:
                    watch_cfg.setdefault("capture", {})["iface"] = cap_cfg["iface"]
                    changed = True
                if "syn_only" in cap_cfg:
                    watch_cfg.setdefault("capture", {})["syn_only"] = cap_cfg["syn_only"].lower() == "true"
                    changed = True

                if changed:
                    save_watch_config(watch_cfg)
                    applied.append("Capture settings")
            except Exception as e:
                errors.append(f"Capture: {str(e)}")

        # Apply Sync settings
        if "Sync" in parsed:
            try:
                sync_cfg = parsed["Sync"]
                new_peers = []
                for key, val in sync_cfg.items():
                    if key.startswith("peer") and val:
                        # Parse host:port format
                        if ":" in val:
                            host, port_str = val.rsplit(":", 1)
                            try:
                                port = int(port_str)
                            except ValueError:
                                port = 8080
                        else:
                            host = val
                            port = 8080
                        new_peers.append({"host": host, "port": port})
                if new_peers:
                    sync_mgr = get_sync_manager()
                    sync_mgr.config["peers"] = new_peers
                    sync_mgr._save_config()
                    applied.append(f"Sync: {len(new_peers)} peers")
            except Exception as e:
                errors.append(f"Sync: {str(e)}")

        # Apply Post settings
        if "Post" in parsed:
            try:
                post_cfg = parsed["Post"]
                watch_cfg = load_watch_config()
                changed = False

                if "url" in post_cfg:
                    watch_cfg.setdefault("post", {})["url"] = post_cfg["url"]
                    changed = True
                if "gzip" in post_cfg:
                    watch_cfg.setdefault("post", {})["gzip"] = post_cfg["gzip"].lower() == "true"
                    changed = True

                if changed:
                    save_watch_config(watch_cfg)
                    applied.append("Post settings")
            except Exception as e:
                errors.append(f"Post: {str(e)}")

        return {
            "ok": len(errors) == 0,
            "applied": applied,
            "errors": errors if errors else None
        }

    return router


__all__ = ["create_router"]
