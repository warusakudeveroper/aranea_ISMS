"""
is20s Web UI - AraneaWebUI準拠
ESP32共通UIフレームワークと同一スタイル
"""
from fastapi import APIRouter, HTTPException, Request
from fastapi.responses import HTMLResponse


# AraneaWebUI共通CSS（ESP32 AraneaWebUI.cpp準拠）
COMMON_CSS = """
:root{--primary:#4c4948;--primary-light:#6b6a69;--accent:#3182ce;--accent-hover:#2b6cb0;--bg:#f7fafc;--card:#fff;--border:#e2e8f0;--text:#2d3748;--text-muted:#718096;--success:#38a169;--danger:#e53e3e;--warning:#d69e2e}
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;background:var(--bg);color:var(--text);line-height:1.5}
.header{background:var(--primary);color:#fff;padding:12px 20px;display:flex;align-items:center;gap:12px;box-shadow:0 2px 4px rgba(0,0,0,.1)}
.header h1{font-size:18px;font-weight:600}
.header .version{font-size:11px;opacity:.7;margin-left:auto}
.device-banner{background:linear-gradient(135deg,#f8f9fa 0%,#e9ecef 100%);border-bottom:1px solid var(--border);padding:16px 20px}
.device-banner-inner{max-width:960px;margin:0 auto}
.device-model{font-size:22px;font-weight:700;color:var(--primary);line-height:1.2}
.device-name{font-size:16px;font-weight:600;color:var(--accent);margin-top:4px}
.device-name .unnamed{color:var(--text-muted);font-weight:400;font-style:italic}
.device-context{font-size:13px;color:var(--text-muted);margin-top:6px}
.container{max-width:960px;margin:0 auto;padding:16px}
.tabs{display:flex;gap:2px;background:var(--card);border-radius:8px 8px 0 0;padding:4px 4px 0;border:1px solid var(--border);border-bottom:none;flex-wrap:wrap}
.tab{padding:10px 16px;cursor:pointer;border-radius:6px 6px 0 0;font-size:13px;font-weight:500;color:var(--text-muted);transition:all .2s;border-bottom:2px solid transparent}
.tab:hover{background:var(--bg)}
.tab.active{color:var(--accent);border-bottom-color:var(--accent);background:var(--bg)}
.tab-content{display:none;background:var(--card);border:1px solid var(--border);border-top:none;border-radius:0 0 8px 8px;padding:20px}
.tab-content.active{display:block}
.card{background:var(--card);border:1px solid var(--border);border-radius:8px;padding:16px;margin-bottom:16px}
.card-title{font-size:14px;font-weight:600;color:var(--text);margin-bottom:12px;padding-bottom:8px;border-bottom:1px solid var(--border)}
.status-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:12px}
.status-item{background:var(--bg);padding:12px;border-radius:6px}
.status-item .label{font-size:11px;color:var(--text-muted);text-transform:uppercase;letter-spacing:.5px}
.status-item .value{font-size:14px;font-weight:500;word-break:break-all}
.status-item .value.good{color:var(--success)}
.status-item .value.warn{color:var(--warning)}
.status-item .value.bad{color:var(--danger)}
.form-group{margin-bottom:14px}
.form-group label{display:block;font-size:12px;font-weight:500;color:var(--text-muted);margin-bottom:4px}
.form-group input,.form-group select,.form-group textarea{width:100%;padding:8px 12px;border:1px solid var(--border);border-radius:6px;font-size:14px;transition:border-color .2s}
.form-group input:focus,.form-group select:focus,.form-group textarea:focus{outline:none;border-color:var(--accent);box-shadow:0 0 0 2px rgba(49,130,206,.1)}
.form-row{display:flex;gap:12px}
.form-row .form-group{flex:1}
.btn{padding:8px 16px;border:none;border-radius:6px;font-size:13px;font-weight:500;cursor:pointer;transition:all .2s}
.btn-primary{background:var(--accent);color:#fff}
.btn-primary:hover{background:var(--accent-hover)}
.btn-danger{background:var(--danger);color:#fff}
.btn-sm{padding:5px 10px;font-size:12px}
.btn-group{display:flex;gap:8px;margin-top:16px}
.toast{position:fixed;bottom:20px;right:20px;background:var(--success);color:#fff;padding:12px 20px;border-radius:6px;display:none;z-index:1000;box-shadow:0 4px 12px rgba(0,0,0,.15)}
.live-box{background:var(--bg);border:1px solid var(--border);border-radius:6px;padding:12px;max-height:200px;overflow:auto;font-family:monospace;font-size:12px}
table{width:100%;border-collapse:collapse;font-size:13px}
th,td{padding:8px 10px;border-bottom:1px solid var(--border);text-align:left}
th{background:var(--bg);font-weight:500;color:var(--text-muted);position:sticky;top:0}
@keyframes rowSlideIn{from{opacity:0;transform:translateY(-12px)}to{opacity:1;transform:translateY(0)}}
@keyframes rowHighlight{0%{background-color:#cce5ff}100%{background-color:transparent}}
.new-row{animation:rowSlideIn .5s ease-out}
.new-row td{animation:rowHighlight 2s ease-out}
@media(max-width:600px){.form-row{flex-direction:column}.tabs{gap:0}.tab{padding:8px 12px;font-size:12px}}
.cap-tbl td,.cap-tbl th{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;padding:6px 4px;font-size:12px}
.cap-tbl td:first-child{overflow:visible}
.cap-tbl .col-dns,.cap-tbl .col-sni{max-width:100px}
.stats-chart-container{display:flex;flex-direction:column;gap:8px}
.stats-bar-row{display:flex;align-items:center;gap:8px;cursor:pointer;padding:4px;border-radius:4px;transition:background .2s}
.stats-bar-row:hover{background:var(--bg)}
.stats-bar-label{min-width:60px;font-size:12px;font-weight:500;text-align:right}
.stats-bar-wrap{flex:1;height:24px;background:var(--bg);border-radius:4px;overflow:hidden;position:relative}
.stats-bar{height:100%;display:flex;transition:width .6s ease-out;border-radius:4px}
.stats-bar-segment{height:100%;display:flex;align-items:center;justify-content:center;color:#fff;font-size:10px;font-weight:500;overflow:hidden;white-space:nowrap;transition:width .6s ease-out}
.stats-bar-segment:last-child{border-radius:0 4px 4px 0}
.stats-loading{display:flex;align-items:center;justify-content:center;padding:20px;color:var(--text-muted)}
.stats-spinner{width:20px;height:20px;border:2px solid var(--border);border-top-color:var(--primary);border-radius:50%;animation:spin 1s linear infinite;margin-right:8px}
@keyframes spin{to{transform:rotate(360deg)}}
.stats-bar-count{min-width:40px;font-size:11px;color:var(--text-muted);text-align:right}
.stats-legend{display:flex;flex-wrap:wrap;gap:8px;margin-top:8px;font-size:11px}
.stats-legend-item{display:flex;align-items:center;gap:4px}
.stats-legend-color{width:12px;height:12px;border-radius:2px}
.stats-cat-colors{--cat-streaming:#2563eb;--cat-sns:#7c3aed;--cat-messenger:#0891b2;--cat-ec:#059669;--cat-cloud:#6366f1;--cat-ad:#ea580c;--cat-tracker:#ca8a04;--cat-game:#dc2626;--cat-ai:#8b5cf6;--cat-other:#94a3b8}
"""

# AraneaロゴSVG（共通）
LOGO_SVG = '<svg viewBox="0 0 211 280" fill="currentColor" style="height:32px;width:auto"><path d="M187.43,209.6c-9.57-12.63-22.63-21.14-37.43-26.9-.48-.19-.83-.68-1.25-1.03.18-.1.34-.24.53-.3,4.99-1.36,10.33-2,14.92-4.2,8.82-4.24,17.97-8.39,25.62-14.33,13.42-10.42,18.72-25.75,21.18-42.04.22-1.47-1.51-4.13-2.93-4.69-1.46-.57-4.68.21-5.34,1.39-1.64,2.93-2.31,6.41-3.27,9.7-2.52,8.58-6.23,16.58-12.64,22.92-9.65,9.53-22.03,13.38-34.93,15.91-.71.14-1.43.2-2.14.3.4-.56.72-1.21,1.22-1.66,10.74-9.61,19.39-20.71,25.21-34.02,3.39-7.74,6.01-15.8,4.83-24.06-1.43-10.01-4.28-19.85-7.01-29.62-.44-1.59-3.39-3.38-5.24-3.46-3.09-.14-4.89,2.36-4.14,5.46,1.21,5.07,3.12,9.96,4.33,15.03,2.1,8.85,2.15,17.77-1.36,26.28-4.66,11.3-12.37,20.46-21.34,28.61-.18.16-.39.29-.6.41-.07.04-.14.09-.21.13-.01-.04-.03-.08-.05-.13-.03-.08-.07-.17-.11-.26-.09-.21-.18-.43-.17-.63.7-16.98-4.61-31.8-16.06-44.34-5.64-6.17-12.28-11.01-20.72-12.31-.3-.05-.58-.04-.88-.07V0h-3.88v91.63c-6.1.46-11.46,3.54-16.2,7.54-14.48,12.21-21.36,28.05-21.42,46.93,0,1.1-.13,2.2-.2,3.29-.82-.67-1.64-1.32-2.44-2.02-.62-.54-1.22-1.11-1.8-1.69-8.13-8.05-15.3-16.89-18.81-27.91-4.08-12.79-1.03-25.16,3.39-37.39.62-1.73-.56-5.65-1.86-6.16-1.75-.69-5.94.35-6.43,1.66-2.87,7.52-5.66,15.21-7.15,23.1-2.65,14.01,1.13,27.04,8.01,39.32,5.79,10.34,13.35,19.2,22.24,26.98.27.23.34.69.5,1.04-.2.04-.43.16-.6.1-8.81-2.92-18.05-4.97-26.34-8.98-14.94-7.23-20.95-21.2-24.38-36.5-.82-3.66-2.65-6.13-6.65-5.1-3.15.81-3.9,3.44-3.17,6.5.86,3.62,1.17,7.48,2.64,10.82,3.33,7.55,6.17,15.71,11.21,22.01,5.05,6.3,11.95,11.75,19.11,15.57,8.64,4.61,18.41,7.11,27.69,10.51.48.18.99.29,1.48.43-.44.36-.86.77-1.34,1.06-9.09,5.45-18.61,10.29-27.16,16.48-8.89,6.44-15.63,15.05-18.7,25.97-2.97,10.55-2.84,21.32-1.37,32.02.24,1.73,3.05,4.21,4.85,4.37,3.25.29,4.38-2.62,4.39-5.65,0-3.93-.18-7.87-.07-11.79.26-9.53,2.11-18.64,8.68-25.99,8.54-9.56,19.02-16.44,30.82-21.37.64-.27,1.27-.55,1.93-.75.07-.02.14-.03.21-.02.11,0,.22.03.34.05.08.02.15.04.23.05.04,0,.08,0,.11.02-.13.19-.26.38-.38.57-.13.19-.26.38-.4.56-5.8,7.69-12.1,15.05-17.26,23.14-5.94,9.32-5.42,19.94-2.72,30.15,2.19,8.27,5.6,16.22,8.51,24.3,1.06,2.94,3.06,5.41,6.23,3.91,1.6-.75,3.42-4.11,2.96-5.64-2.32-7.78-6.07-15.18-7.89-23.04-1.3-5.6-1.93-12.29-.06-17.49,4.47-12.45,13.46-22.02,23.8-30.2.1-.08.21-.15.32-.22.34-.2.71-.36,1.06-.55.04.28.07.56.11.84s.08.56.16.83c.99,3.58,1.92,7.19,3.06,10.73.44,1.35,2.28,2.94,1.93,3.74-2.68,6.12-1.17,15.61,3.72,19.66.94.78,2.57.71,3.89,1.03.31-1.37.84-2.73.87-4.11.03-1.83-1.04-4-.37-5.42,1-2.1,2.94-4.97,4.8-5.22,5.27-.71,10.7-.27,16.07-.26.46,0,1.05-.05,1.36.2,3.42,2.78,4.13,6.26,2.98,10.46-.35,1.27.23,2.91.77,4.21.13.3,2.12.28,2.84-.2,5.11-3.46,7.51-11.9,5.34-17.73-.44-1.19-.94-3.09-.36-3.81,3.44-4.21,4.96-8.92,4.75-14.28,0-.23.18-.47.28-.7.22.1.5.14.66.3,5.79,5.89,11.96,11.45,17.23,17.77,5.67,6.78,10.49,14.65,8.36,23.91-2.27,9.84-6.16,19.31-8.83,29.07-.47,1.72,1.07,4.71,2.61,5.97.8.65,4.89-.74,5.37-2.01,3.76-9.9,7.59-19.84,10.23-30.06,2.42-9.39,1.48-19.11-3.95-27.39-5.14-7.83-11.4-14.94-17.16-22.36-.12-.15-.19-.34-.27-.53-.03-.06-.05-.12-.08-.18h.12c.3-.03.63-.12.84,0,7.16,3.94,14.78,7.26,21.3,12.07,5.66,4.16,10.89,9.45,14.84,15.24,5.01,7.34,5.49,16.31,5.12,25.11v7.14c0,3.24.99,6.03,4.67,6.13,3.71.1,4.9-2.86,4.89-5.9-.04-8.59.75-17.42-.95-25.71-1.48-7.23-4.78-14.7-9.24-20.58z"/></svg>'


def _guard(request: Request, allowed_sources: list[str]) -> None:
    """IP制限チェック"""
    if not allowed_sources:
        return
    from ipaddress import ip_address, ip_network
    ip = request.client.host if request.client else ""
    try:
        addr = ip_address(ip)
    except ValueError:
        raise HTTPException(status_code=403, detail="forbidden")
    for entry in allowed_sources:
        try:
            if "/" in entry and addr in ip_network(entry, strict=False):
                return
            if "/" not in entry and addr == ip_address(entry):
                return
        except ValueError:
            continue
    raise HTTPException(status_code=403, detail="forbidden")


def create_router(allowed_sources: list[str]) -> APIRouter:
    router = APIRouter()

    # メインHTML（全タブ含む）
    MAIN_HTML = f"""<!DOCTYPE html>
<html><head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Aranea Device Manager</title>
<style>{COMMON_CSS}</style>
</head><body>
<div class="header">
<div class="logo">{LOGO_SVG}</div>
<h1>Aranea Device Manager</h1>
<span class="version" id="hdr-ver">is20s</span>
</div>
<div class="device-banner">
<div class="device-banner-inner">
<div class="device-model">ar-is20s Network Capture Gateway</div>
<div class="device-name" id="device-name-display"><span class="unnamed">(loading...)</span></div>
<div class="device-context">パケットキャプチャ・ドメイン分類・CelestialGlobe連携</div>
</div>
</div>
<div class="container">
<div class="tabs">
<div class="tab active" data-tab="status" onclick="showTab('status')">Status</div>
<div class="tab" data-tab="capture" onclick="showTab('capture')">Capture</div>
<div class="tab" data-tab="monitor" onclick="showTab('monitor')">Monitor</div>
<div class="tab" data-tab="network" onclick="showTab('network')">Network</div>
<div class="tab" data-tab="sync" onclick="showTab('sync')">Sync</div>
<div class="tab" data-tab="cloud" onclick="showTab('cloud')">Cloud</div>
<div class="tab" data-tab="tenant" onclick="showTab('tenant')">Tenant</div>
<div class="tab" data-tab="system" onclick="showTab('system')">System</div>
<div class="tab" data-tab="speeddial" onclick="showTab('speeddial')">SpeedDial</div>
<div class="tab" data-tab="stats" onclick="showTab('stats')">Statistics</div>
</div>

<!-- Status Tab -->
<div id="tab-status" class="tab-content active">
<div class="card">
<div class="card-title">Device Information</div>
<div class="status-grid">
<div class="status-item"><div class="label">Type</div><div class="value" id="d-type">-</div></div>
<div class="status-item"><div class="label">LacisID</div><div class="value" id="d-lacis">-</div></div>
<div class="status-item"><div class="label">CIC</div><div class="value" id="d-cic">-</div></div>
<div class="status-item"><div class="label">Hostname</div><div class="value" id="d-host">-</div></div>
<div class="status-item"><div class="label">Product</div><div class="value" id="d-product">-</div></div>
</div>
</div>
<div class="card">
<div class="card-title">Hardware Status</div>
<div class="status-grid">
<div class="status-item"><div class="label">RAM Usage</div><div class="value" id="hw-ram">-</div></div>
<div class="status-item"><div class="label">CPU Usage</div><div class="value" id="hw-cpu">-</div></div>
<div class="status-item"><div class="label">Temperature</div><div class="value" id="hw-temp">-</div></div>
<div class="status-item"><div class="label">Storage</div><div class="value" id="hw-storage">-</div></div>
<div class="status-item"><div class="label">Load Avg</div><div class="value" id="hw-load">-</div></div>
</div>
</div>
<div class="card">
<div class="card-title">Network Interfaces</div>
<div class="status-grid">
<div class="status-item"><div class="label">eth0/end0</div><div class="value" id="hw-eth">-</div></div>
<div class="status-item"><div class="label">wlan0</div><div class="value" id="hw-wlan">-</div></div>
</div>
</div>
<div class="card">
<div class="card-title">Live Status</div>
<div class="status-grid">
<div class="status-item"><div class="label">Uptime</div><div class="value" id="s-uptime">-</div></div>
<div class="status-item"><div class="label">MQTT</div><div class="value" id="s-mqtt">-</div></div>
<div class="status-item"><div class="label">Forwarder Queue</div><div class="value" id="s-queue">-</div></div>
<div class="status-item"><div class="label">Ring Size</div><div class="value" id="s-ring">-</div></div>
<div class="status-item"><div class="label">Bluetooth</div><div class="value" id="s-bt">-</div></div>
<div class="status-item"><div class="label">DB</div><div class="value" id="s-db">-</div></div>
</div>
</div>
</div>

<!-- Capture Tab (tshark監視 - コア機能) -->
<div id="tab-capture" class="tab-content">
<div class="card">
<div class="card-title">Network Capture Control</div>
<div class="status-grid">
<div class="status-item"><div class="label">Status</div><div class="value" id="cap-status">-</div></div>
<div class="status-item"><div class="label">Mode</div><div class="value" id="cap-mode">-</div></div>
<div class="status-item"><div class="label">Queue</div><div class="value" id="cap-queue">-</div></div>
<div class="status-item"><div class="label">Sent</div><div class="value" id="cap-sent">-</div></div>
<div class="status-item"><div class="label">Dropped</div><div class="value" id="cap-drop">-</div></div>
<div class="status-item"><div class="label">Rooms Count</div><div class="value" id="cap-rooms">-</div></div>
</div>
<div class="btn-group">
<button class="btn btn-primary" onclick="startCapture()">Start Capture</button>
<button class="btn btn-danger" onclick="stopCapture()">Stop Capture</button>
<button class="btn" onclick="refreshCaptureStatus()">Refresh</button>
</div>
</div>
<div class="card">
<div class="card-title">Capture Settings</div>
<div class="form-row">
<div class="form-group"><label>Capture Enabled</label><select id="cap-enabled"><option value="true">true</option><option value="false">false</option></select></div>
<div class="form-group"><label>Dry Run</label><select id="cap-dryrun"><option value="false">false</option><option value="true">true</option></select></div>
</div>
<div class="form-row">
<div class="form-group">
<label style="display:flex;align-items:center;gap:8px">
<input type="checkbox" id="cap-syn-only" checked style="width:auto">
軽量モード (TCP SYNのみ)
</label>
<p style="font-size:11px;color:#e67e22;margin:4px 0 0 24px">⚠ チェックを外すと詳細モードになりますが、大量のログによりシステム負荷が発生します。<br>（詳細モードは短時間で使用してください）</p>
</div>
</div>
<div class="form-row">
<div class="form-group"><label>Post URL</label><input type="text" id="cap-post-url" placeholder="https://example.com/api/netwatch"></div>
</div>
<div class="form-row">
<div class="form-group"><label>Batch Max Events</label><input type="number" id="cap-batch-events" value="500"></div>
<div class="form-group"><label>Batch Max Seconds</label><input type="number" id="cap-batch-sec" value="30"></div>
</div>
<button class="btn btn-primary" onclick="saveCaptureConfig()">Save Capture Config</button>
</div>
<div class="card">
<div class="card-title">Room Mapping (IP → Room No)</div>
<p style="font-size:12px;color:var(--text-muted);margin-bottom:6px">1行に「IP=部屋番号」形式で入力（例: 192.168.125.10=101）</p>
<p style="font-size:11px;color:#e67e22;margin-bottom:10px">💡 192.168.125.0のように末尾を0にすると、そのセグメント(/24)全体を対象とします。<br>VLANで部屋を分けている環境で便利です。ただしトラフィック量が膨大になるため、少量・小規模な場合以外は使用しないでください。</p>
<div class="form-group"><textarea id="cap-rooms-text" rows="8" placeholder="192.168.125.10=101&#10;192.168.125.11=102&#10;192.168.126.0=201 (セグメント全体)"></textarea></div>
<button class="btn btn-primary" onclick="saveRooms()">Save Rooms</button>
<p id="cap-rooms-status" style="margin-top:8px;font-size:12px;color:var(--text-muted)"></p>
</div>
</div>

<!-- Monitor Tab -->
<div id="tab-monitor" class="tab-content">
<div class="card">
<div class="card-title">Capture Events (tshark) <span id="cap-event-count" style="color:var(--text-muted);font-weight:normal">0</span></div>
<div class="btn-group" style="margin-bottom:12px">
<button class="btn btn-primary btn-sm" onclick="refreshCaptureEvents()">Refresh</button>
<button class="btn btn-sm" onclick="updateThreatIntel()">Update Threat DB</button>
<span style="font-size:12px;color:var(--text-muted);margin-left:12px" id="cap-event-info">-</span>
<span style="font-size:11px;color:var(--text-muted);margin-left:8px" id="threat-info"></span>
</div>
<div style="margin-bottom:12px;display:flex;flex-wrap:wrap;gap:12px;align-items:center">
<div><label style="font-size:12px;color:var(--text-muted);margin-right:4px">Room:</label><select id="cap-room-filter" onchange="refreshCaptureEvents()" style="padding:4px 8px;border:1px solid var(--border);border-radius:4px;font-size:12px"><option value="">All</option></select></div>
<div><label style="font-size:12px;color:var(--text-muted);margin-right:4px">Dir:</label><select id="cap-dir-filter" onchange="refreshCaptureEvents()" style="padding:4px 8px;border:1px solid var(--border);border-radius:4px;font-size:12px"><option value="">All</option><option value="up">↑送信のみ</option><option value="down">↓受信のみ</option></select></div>
<details id="filter-dropdown" style="font-size:12px;position:relative">
<summary style="cursor:pointer;padding:4px 8px;border:1px solid var(--border);border-radius:4px;background:var(--bg-secondary)">除外フィルタ ▾</summary>
<div style="position:absolute;top:100%;left:0;z-index:100;background:var(--bg);border:1px solid var(--border);border-radius:4px;padding:8px;box-shadow:0 2px 8px rgba(0,0,0,0.15);min-width:160px">
<label style="display:flex;align-items:center;gap:4px;margin:4px 0"><input type="checkbox" id="cap-filter-local" onchange="updateBackendFilter()" style="width:auto" checked>ローカル</label>
<label style="display:flex;align-items:center;gap:4px;margin:4px 0"><input type="checkbox" id="cap-filter-ptr" onchange="updateBackendFilter()" style="width:auto" checked>PTR</label>
<label style="display:flex;align-items:center;gap:4px;margin:4px 0"><input type="checkbox" id="cap-filter-photo" onchange="updateBackendFilter()" style="width:auto" checked>写真同期</label>
<label style="display:flex;align-items:center;gap:4px;margin:4px 0"><input type="checkbox" id="cap-filter-oscheck" onchange="updateBackendFilter()" style="width:auto" checked>OS接続</label>
<label style="display:flex;align-items:center;gap:4px;margin:4px 0"><input type="checkbox" id="cap-filter-ad" onchange="updateBackendFilter()" style="width:auto" checked>広告/計測</label>
<label style="display:flex;align-items:center;gap:4px;margin:4px 0"><input type="checkbox" id="cap-filter-dns" onchange="updateBackendFilter()" style="width:auto" checked>DNS</label>
<label style="display:flex;align-items:center;gap:4px;margin:4px 0"><input type="checkbox" id="cap-filter-background" onchange="updateBackendFilter()" style="width:auto" checked>バックグラウンド</label>
<hr style="margin:6px 0;border:0;border-top:1px solid var(--border)">
<label style="display:flex;align-items:center;gap:4px;margin:4px 0"><input type="checkbox" id="cap-filter-streaming" onchange="refreshCaptureEvents()" style="width:auto">Streaming</label>
<label style="display:flex;align-items:center;gap:4px;margin:4px 0"><input type="checkbox" id="cap-filter-sns" onchange="refreshCaptureEvents()" style="width:auto">SNS</label>
<label style="display:flex;align-items:center;gap:4px;margin:4px 0"><input type="checkbox" id="cap-filter-google" onchange="refreshCaptureEvents()" style="width:auto">Google</label>
<label style="display:flex;align-items:center;gap:4px;margin:4px 0"><input type="checkbox" id="cap-filter-apple" onchange="refreshCaptureEvents()" style="width:auto">Apple</label>
</div>
</details>
<div style="flex:1;min-width:120px"><input type="text" id="cap-filter-keyword" oninput="refreshCaptureEvents()" placeholder="Filter..." style="width:100%;padding:4px 8px;border:1px solid var(--border);border-radius:4px;font-size:12px;font-family:monospace"></div>
</div>
<div style="overflow-y:auto;max-height:32000px">
<table class="cap-tbl" style="table-layout:fixed;width:100%">
<thead><tr><th style="width:58px">Time</th><th style="width:28px"></th><th style="width:42px">Room</th><th style="width:20px">↕</th><th style="width:80px">Service</th><th style="width:62px">Type</th><th style="width:92px">SrcIP</th><th style="width:120px">DstIP:Port</th><th>Domain</th></tr></thead>
<tbody id="cap-events-body"><tr><td colspan="9" style="color:var(--text-muted)">No capture events</td></tr></tbody>
</table>
</div>
</div>
<div class="card">
<div class="card-title">Live Events (ESP Relay)</div>
<div class="btn-group" style="margin-bottom:12px">
<button class="btn btn-primary btn-sm" onclick="refreshEvents()">Refresh</button>
<button class="btn btn-sm" onclick="publishSample()">Debug Event</button>
</div>
<div class="live-box" id="live">connecting...</div>
</div>
<div class="card">
<div class="card-title">Recent Events <span id="event-count" style="color:var(--text-muted);font-weight:normal">0</span></div>
<div style="overflow:auto;max-height:300px">
<table>
<thead><tr><th>seenAt</th><th>src</th><th>lacisId</th><th>temp</th><th>hum</th><th>bat</th><th>rssi</th></tr></thead>
<tbody id="events-body"><tr><td colspan="7" style="color:var(--text-muted)">loading...</td></tr></tbody>
</table>
</div>
</div>
</div>

<!-- Network Tab -->
<div id="tab-network" class="tab-content">
<div class="card">
<div class="card-title">WiFi接続状態</div>
<div class="status-grid">
<div class="status-item"><div class="label">STATUS</div><div class="value" id="wifi-status">-</div></div>
<div class="status-item"><div class="label">SSID</div><div class="value" id="wifi-ssid">-</div></div>
<div class="status-item"><div class="label">SIGNAL</div><div class="value" id="wifi-signal">-</div></div>
<div class="status-item"><div class="label">IP</div><div class="value" id="wifi-ip">-</div></div>
</div>
<p style="font-size:12px;color:var(--text-muted);margin:12px 0 8px">一時接続（設定リストに保存しません）</p>
<div class="form-row">
<div class="form-group"><label>SSID</label><input type="text" id="wifi-new-ssid" placeholder="ネットワーク名"></div>
<div class="form-group"><label>Password</label><input type="password" id="wifi-new-pass" placeholder="パスワード"></div>
</div>
<div class="btn-group">
<button class="btn" onclick="connectWifi()">一時接続</button>
<button class="btn" onclick="scanWifi()">Scan Networks</button>
</div>
<div id="wifi-networks" style="margin-top:8px;font-size:12px;color:var(--text-muted)"></div>
</div>
<div class="card">
<div class="card-title">WiFi設定一覧 (最大6件・優先順位順)</div>
<p style="font-size:12px;color:var(--text-muted);margin-bottom:10px">
保存されたWiFi設定を上から順に接続を試行します。<br>
遠隔地でのAP交換時も旧SSIDを残しておけば切り替え可能です。
</p>
<div style="overflow:auto;max-height:250px">
<table id="wifi-configs-table">
<thead><tr><th style="width:40px">#</th><th>SSID</th><th style="width:100px">Password</th><th style="width:140px">操作</th></tr></thead>
<tbody id="wifi-configs-body"><tr><td colspan="4" style="color:var(--text-muted)">loading...</td></tr></tbody>
</table>
</div>
<div class="btn-group" style="margin-top:12px">
<button class="btn btn-primary" onclick="autoConnectWifi()">Auto Connect</button>
<button class="btn btn-danger" onclick="resetWifiConfigs()">Reset to Default</button>
<button class="btn" onclick="refreshWifiConfigs()">Refresh</button>
</div>
<div style="margin-top:12px;padding:12px;background:var(--bg);border-radius:6px">
<p style="font-size:12px;color:var(--text-muted);margin-bottom:8px">新しいWiFi設定をリストに追加</p>
<div class="form-row">
<div class="form-group"><label>SSID</label><input type="text" id="wifi-cfg-ssid" placeholder="追加するSSID"></div>
<div class="form-group"><label>Password</label><input type="password" id="wifi-cfg-pass" placeholder="パスワード"></div>
</div>
<button class="btn btn-primary" onclick="addWifiConfig()">リストに追加</button>
</div>
</div>
<div class="card">
<div class="card-title">NTP / Time Settings</div>
<div class="status-grid">
<div class="status-item"><div class="label">Synchronized</div><div class="value" id="ntp-sync">-</div></div>
<div class="status-item"><div class="label">Server</div><div class="value" id="ntp-server">-</div></div>
<div class="status-item"><div class="label">Timezone</div><div class="value" id="ntp-tz">-</div></div>
<div class="status-item"><div class="label">Current Time</div><div class="value" id="ntp-time">-</div></div>
</div>
<div class="form-row" style="margin-top:12px">
<div class="form-group"><label>NTP Server</label><input type="text" id="ntp-new-server" placeholder="ntp.nict.jp"></div>
<div class="form-group"><label>Timezone</label><input type="text" id="ntp-new-tz" placeholder="Asia/Tokyo"></div>
</div>
<button class="btn btn-primary" onclick="saveNtp()">Save NTP Settings</button>
</div>
<div class="card">
<div class="card-title">Access Control</div>
<div class="form-group"><label>Allowed Sources (1行1アドレス/CIDR)</label><textarea id="n-allowed" rows="4" placeholder="192.168.0.0/24&#10;10.0.0.5"></textarea></div>
<button class="btn btn-primary" onclick="saveAccess()">Save Access</button>
</div>
</div>

<!-- Sync Tab -->
<div id="tab-sync" class="tab-content">
<div class="card">
<div class="card-title">Cache Sync Status</div>
<p style="font-size:12px;color:var(--text-muted);margin-bottom:10px">DNSキャッシュ・ASNキャッシュを他のis20sと同期できます。VPNで接続された拠点間でデータを共有し、検知精度を向上させます。</p>
<div class="status-grid">
<div class="status-item"><div class="label">Last Sync</div><div class="value" id="sync-last">-</div></div>
<div class="status-item"><div class="label">Peers</div><div class="value" id="sync-peers">0</div></div>
<div class="status-item"><div class="label">Initiator</div><div class="value" id="sync-init">-</div></div>
<div class="status-item"><div class="label">Responder</div><div class="value" id="sync-resp">-</div></div>
</div>
</div>
<div class="card">
<div class="card-title">Sync Passphrases</div>
<p style="font-size:12px;color:var(--text-muted);margin-bottom:10px">イニシエータ: 同期を開始する側のパスフレーズ<br>レスポンダー: 同期を受け付ける側のパスフレーズ（両方の機器で同じ値を設定）</p>
<div class="form-row">
<div class="form-group"><label>Initiator Passphrase</label><input type="password" id="sync-init-pass" placeholder="同期開始側パスフレーズ"></div>
<div class="form-group"><label>Responder Passphrase</label><input type="password" id="sync-resp-pass" placeholder="同期受付側パスフレーズ"></div>
</div>
<button class="btn btn-primary" onclick="saveSyncPassphrases()">Save Passphrases</button>
</div>
<div class="card">
<div class="card-title">Sync Peers</div>
<p style="font-size:12px;color:var(--text-muted);margin-bottom:10px">同期対象の他のis20s機器（デイジーチェーン可能）</p>
<div class="form-row">
<div class="form-group"><label>Peer Host</label><input type="text" id="sync-peer-host" placeholder="192.168.x.x"></div>
<div class="form-group"><label>Port</label><input type="number" id="sync-peer-port" value="8080"></div>
</div>
<div class="btn-group">
<button class="btn btn-primary" onclick="addSyncPeer()">Add Peer</button>
<button class="btn" onclick="syncWithPeer()">Sync Now</button>
<button class="btn" onclick="syncAll()">Sync All Peers</button>
</div>
<div id="sync-peer-list" style="margin-top:12px;font-size:12px"></div>
</div>
<div class="card">
<div class="card-title">Export / Import</div>
<p style="font-size:12px;color:var(--text-muted);margin-bottom:10px">キャッシュデータを手動でエクスポート/インポート</p>
<div class="btn-group">
<button class="btn btn-primary" onclick="exportCache()">Export Cache</button>
<button class="btn" onclick="document.getElementById('import-file').click()">Import Cache</button>
<input type="file" id="import-file" style="display:none" accept=".json" onchange="importCache(event)">
</div>
<div id="sync-export-status" style="margin-top:8px;font-size:12px;color:var(--text-muted)"></div>
</div>
</div>

<!-- Cloud Tab -->
<div id="tab-cloud" class="tab-content">
<div class="card">
<div class="card-title">Relay Endpoints</div>
<div class="form-row">
<div class="form-group"><label>Primary URL</label><input type="text" id="c-relay1"></div>
<div class="form-group"><label>Secondary URL</label><input type="text" id="c-relay2"></div>
</div>
<div class="form-row">
<div class="form-group"><label>Timeout (sec)</label><input type="number" id="c-relay-timeout"></div>
<div class="form-group"><label>Max Retry</label><input type="number" id="c-relay-retry"></div>
</div>
<button class="btn btn-primary" onclick="saveRelay()">Save Relay</button>
</div>
<div class="card">
<div class="card-title">MQTT Settings</div>
<div class="form-row">
<div class="form-group"><label>Host</label><input type="text" id="c-mqtt-host"></div>
<div class="form-group"><label>Port</label><input type="number" id="c-mqtt-port"></div>
</div>
<div class="form-row">
<div class="form-group"><label>User</label><input type="text" id="c-mqtt-user"></div>
<div class="form-group"><label>Password</label><input type="password" id="c-mqtt-pass"></div>
</div>
<div class="form-row">
<div class="form-group"><label>Base Topic</label><input type="text" id="c-mqtt-topic"></div>
<div class="form-group"><label>TLS</label><select id="c-mqtt-tls"><option value="true">true</option><option value="false">false</option></select></div>
</div>
<button class="btn btn-primary" onclick="saveMqtt()">Save MQTT</button>
</div>
</div>

<!-- Tenant Tab -->
<div id="tab-tenant" class="tab-content">
<div class="card">
<div class="card-title">Tenant Authentication</div>
<div class="form-row">
<div class="form-group"><label>Tenant ID (TID)</label><input type="text" id="t-tid"></div>
</div>
<div class="form-row">
<div class="form-group"><label>Primary LacisID</label><input type="text" id="t-lacis"></div>
<div class="form-group"><label>CIC</label><input type="text" id="t-cic"></div>
</div>
<div class="form-group"><label>Email</label><input type="email" id="t-email"></div>
<div class="form-group"><label>Gate URL</label><input type="text" id="t-gate"></div>
<button class="btn btn-primary" onclick="saveTenant()">Save Tenant</button>
</div>
</div>

<!-- System Tab -->
<div id="tab-system" class="tab-content">
<div class="card">
<div class="card-title">Device Settings</div>
<div class="form-row">
<div class="form-group"><label>Device Name</label><input type="text" id="s-device-name" placeholder="例: 本社1F-リレーゲートウェイ"></div>
<div class="form-group"><label>Device Type</label><input type="text" id="s-device-type"></div>
</div>
<div class="form-row">
<div class="form-group"><label>Product Type</label><input type="text" id="s-product-type"></div>
<div class="form-group"><label>Product Code</label><input type="text" id="s-product-code"></div>
</div>
<div class="form-row">
<div class="form-group"><label>Interface (MAC取得)</label><input type="text" id="s-interface"></div>
</div>
<button class="btn btn-primary" onclick="saveDevice()">Save Device</button>
</div>
<div class="card">
<div class="card-title">Store Settings</div>
<p class="hint" style="font-size:12px;color:var(--text-muted);margin-bottom:10px">Ring/DB設定の変更は再起動が必要です</p>
<div class="form-row">
<div class="form-group"><label>Ring Size</label><input type="number" id="s-ring-size"></div>
<div class="form-group"><label>Max DB Size</label><input type="number" id="s-max-db"></div>
</div>
<div class="form-row">
<div class="form-group"><label>Flush Interval (sec)</label><input type="number" id="s-flush-int"></div>
<div class="form-group"><label>Flush Batch</label><input type="number" id="s-flush-batch"></div>
</div>
<button class="btn btn-primary" onclick="saveStore()">Save Store</button>
</div>
<div class="card">
<div class="card-title">Logs</div>
<div class="btn-group" style="margin-bottom:12px">
<button class="btn btn-sm" onclick="loadLogs(200)">200 lines</button>
<button class="btn btn-sm" onclick="loadLogs(500)">500 lines</button>
</div>
<div class="live-box" id="logs" style="max-height:300px">loading...</div>
</div>
</div>

<!-- SpeedDial Tab -->
<div id="tab-speeddial" class="tab-content">
<div class="card">
<div class="card-title">SpeedDial 設定一括管理</div>
<p style="font-size:12px;color:var(--text-muted);margin-bottom:12px">
管理者から送られた設定テキストを貼り付けて一括適用できます。<br>
現在の設定をコピーして編集し、貼り付けることも可能です。
</p>
<div style="margin-bottom:16px">
<label style="font-weight:600;display:block;margin-bottom:6px">現在の設定</label>
<div style="position:relative">
<textarea id="sd-current" readonly style="width:100%;height:200px;font-family:monospace;font-size:12px;background:var(--bg);border:1px solid var(--border);border-radius:6px;padding:10px;resize:vertical">(読み込み中...)</textarea>
<button class="btn btn-sm" onclick="copySpeedDial()" style="position:absolute;top:8px;right:8px">📋 Copy</button>
</div>
<button class="btn" onclick="refreshSpeedDial()" style="margin-top:8px">🔄 Refresh</button>
</div>
<div>
<label style="font-weight:600;display:block;margin-bottom:6px">設定を貼り付けて適用</label>
<textarea id="sd-input" placeholder="ここに設定テキストを貼り付け..." style="width:100%;height:200px;font-family:monospace;font-size:12px;border:1px solid var(--border);border-radius:6px;padding:10px;resize:vertical"></textarea>
<div class="btn-group" style="margin-top:8px">
<button class="btn btn-primary" onclick="applySpeedDial()">✅ 適用</button>
<button class="btn" onclick="document.getElementById('sd-input').value=''">🗑️ クリア</button>
</div>
<div id="sd-result" style="margin-top:12px;padding:10px;border-radius:6px;display:none"></div>
</div>
</div>
<div class="card">
<div class="card-title">SpeedDial フォーマット説明</div>
<pre style="font-size:11px;background:var(--bg);padding:10px;border-radius:6px;overflow:auto;white-space:pre-wrap">[WiFi]
wifi1=SSID名,パスワード
wifi2=SSID名2,パスワード2

[NTP]
server=ntp.nict.jp
timezone=Asia/Tokyo

[Capture]
enabled=true
dry_run=true
iface=end0

[Post]
url=https://example.com/api
gzip=true</pre>
<p style="font-size:11px;color:var(--text-muted);margin-top:8px">
※ 変更したいセクションのみ含めれば部分更新可能<br>
※ WiFiは wifi1,wifi2...の順で優先度が高い
</p>
</div>
</div>

<!-- Statistics Tab -->
<div id="tab-stats" class="tab-content">
<div class="card">
<div class="card-title" style="display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:8px">
<span>Room別アクセス状況 <span id="stats-room-count" style="color:var(--text-muted);font-weight:normal"></span></span>
<div style="display:flex;align-items:center;gap:12px">
<label style="display:flex;align-items:center;gap:4px;font-size:11px;cursor:pointer">
<input type="checkbox" id="stats-primary-only" checked onchange="updateStatsFromCache()">
<span>一次通信のみ</span>
</label>
<span id="stats-aux-count" style="font-size:11px;color:var(--text-muted)"></span>
<span id="stats-period" style="font-size:11px;color:var(--text-muted);font-weight:normal"></span>
</div>
</div>
<p style="font-size:12px;color:var(--text-muted);margin-bottom:12px">部屋クリックでカテゴリ詳細を表示 | 「一次通信のみ」ON: CDN等の補助通信を除外</p>
<div id="stats-room-chart" class="stats-chart-container"></div>
</div>
<div class="card" id="stats-room-detail-card" style="display:none">
<div class="card-title">部屋 <span id="stats-room-detail-name"></span> - カテゴリ分布</div>
<div id="stats-room-detail-chart" class="stats-chart-container"></div>
</div>
<div class="card">
<div class="card-title">全体カテゴリ分布 (Top 10)</div>
<div id="stats-overall-chart" class="stats-chart-container"></div>
</div>
</div>

</div>
<div id="toast" class="toast"></div>

<script>
let cfg={{}};
let es=null;
const maxLive=80;

async function load(){{
  try{{
    const r=await fetch('/api/config');
    cfg=await r.json();
    render();
  }}catch(e){{console.error(e)}}
}}

function render(){{
  // Cloud tab
  document.getElementById('c-relay1').value=cfg.relay?.primary_url||'';
  document.getElementById('c-relay2').value=cfg.relay?.secondary_url||'';
  document.getElementById('c-relay-timeout').value=cfg.relay?.timeout_sec||8;
  document.getElementById('c-relay-retry').value=cfg.relay?.max_retry||3;
  document.getElementById('c-mqtt-host').value=cfg.mqtt?.host||'';
  document.getElementById('c-mqtt-port').value=cfg.mqtt?.port||8883;
  document.getElementById('c-mqtt-user').value=cfg.mqtt?.user||'';
  document.getElementById('c-mqtt-pass').value=cfg.mqtt?.password||'';
  document.getElementById('c-mqtt-topic').value=cfg.mqtt?.base_topic||'aranea';
  document.getElementById('c-mqtt-tls').value=String(cfg.mqtt?.tls??true);
  // Tenant tab
  document.getElementById('t-tid').value=cfg.register?.tid||'';
  document.getElementById('t-lacis').value=cfg.register?.tenant_lacisid||'';
  document.getElementById('t-cic').value=cfg.register?.tenant_cic||'';
  document.getElementById('t-email').value=cfg.register?.tenant_email||'';
  document.getElementById('t-gate').value=cfg.register?.gate_url||'';
  // System tab
  document.getElementById('s-device-name').value=cfg.device?.name||'';
  document.getElementById('s-device-type').value=cfg.device?.device_type||'is20s';
  document.getElementById('s-product-type').value=cfg.device?.product_type||'020';
  document.getElementById('s-product-code').value=cfg.device?.product_code||'0096';
  document.getElementById('s-interface').value=cfg.device?.interface||'end0';
  document.getElementById('s-ring-size').value=cfg.store?.ring_size||2000;
  document.getElementById('s-max-db').value=cfg.store?.max_db_size||100000;
  document.getElementById('s-flush-int').value=cfg.store?.flush_interval_sec||5;
  document.getElementById('s-flush-batch').value=cfg.store?.flush_batch||50;
  // Network tab
  document.getElementById('n-allowed').value=(cfg.access?.allowed_sources||[]).join('\\n');
}}

async function refreshStatus(){{
  try{{
    const r=await fetch('/api/status');
    const s=await r.json();
    document.getElementById('d-type').textContent='ar-is20s';
    document.getElementById('d-lacis').textContent=s.device?.lacisId||'-';
    document.getElementById('d-cic').textContent=s.device?.cic||'-';
    document.getElementById('d-host').textContent=s.hostname||'-';
    document.getElementById('d-product').textContent=(s.device?.productType||'')+'/'+(s.device?.productCode||'');
    document.getElementById('s-uptime').textContent=formatUptime(s.uptimeSec||0);
    const mqttOk=s.mqtt?.connected;
    document.getElementById('s-mqtt').textContent=mqttOk?'Connected':'Disconnected';
    document.getElementById('s-mqtt').className='value '+(mqttOk?'good':'warn');
    document.getElementById('s-queue').textContent=s.forwarder?.queued||0;
    document.getElementById('s-ring').textContent=s.ringSize||0;
    document.getElementById('s-bt').textContent=s.bluetooth||'-';
    document.getElementById('s-bt').className='value '+(s.bluetooth==='ok'?'good':'warn');
    document.getElementById('s-db').textContent=s.db||'-';
    document.getElementById('s-db').className='value '+(s.db==='ok'?'good':'warn');
    document.getElementById('hdr-ver').textContent='is20s v'+(s.device?.version||'');
    const dn=s.device?.name||cfg.device?.name||'';
    document.getElementById('device-name-display').innerHTML=dn||'<span class="unnamed">(未設定)</span>';
  }}catch(e){{}}
}}

function formatUptime(sec){{
  const d=Math.floor(sec/86400);
  const h=Math.floor((sec%86400)/3600);
  const m=Math.floor((sec%3600)/60);
  if(d>0)return d+'d '+h+'h '+m+'m';
  if(h>0)return h+'h '+m+'m';
  return m+'m';
}}

function showTab(n){{
  document.querySelectorAll('.tab').forEach(t=>t.classList.remove('active'));
  document.querySelectorAll('.tab-content').forEach(c=>c.classList.remove('active'));
  document.querySelector('[data-tab="'+n+'"]').classList.add('active');
  document.getElementById('tab-'+n).classList.add('active');
}}

async function refreshEvents(){{
  try{{
    const ev=await fetch('/api/events?limit=100').then(r=>r.json());
    const tbody=document.getElementById('events-body');
    tbody.innerHTML='';
    ev.forEach(e=>{{
      const row=tbody.insertRow();
      row.innerHTML='<td>'+(e.seenAt||'')+'</td><td>'+(e.src||'')+'</td><td>'+(e.lacisId||e.mac||'')+'</td><td>'+(e.temperatureC??'')+'</td><td>'+(e.humidityPct??'')+'</td><td>'+(e.batteryPct??'')+'</td><td>'+(e.rssi??'')+'</td>';
    }});
    document.getElementById('event-count').textContent=ev.length;
  }}catch(e){{}}
}}

async function publishSample(){{
  await fetch('/api/debug/publishSample',{{method:'POST'}});
  toast('Debug event published');
}}

function connectSSE(){{
  if(es)es.close();
  const liveEl=document.getElementById('live');
  liveEl.textContent='connecting...';
  es=new EventSource('/api/events/stream');
  es.onopen=()=>{{liveEl.textContent='connected\\n';}};
  es.onmessage=(ev)=>{{
    try{{
      const data=JSON.parse(ev.data);
      const lines=liveEl.textContent.split('\\n').filter(Boolean);
      lines.push(JSON.stringify(data));
      while(lines.length>maxLive)lines.shift();
      liveEl.textContent=lines.join('\\n');
      liveEl.scrollTop=liveEl.scrollHeight;
      const tbody=document.getElementById('events-body');
      if(tbody.rows.length>=150)tbody.deleteRow(tbody.rows.length-1);
      const row=tbody.insertRow(0);
      row.innerHTML='<td>'+(data.seenAt||'')+'</td><td>'+(data.src||'')+'</td><td>'+(data.lacisId||data.mac||'')+'</td><td>'+(data.temperatureC??'')+'</td><td>'+(data.humidityPct??'')+'</td><td>'+(data.batteryPct??'')+'</td><td>'+(data.rssi??'')+'</td>';
      document.getElementById('event-count').textContent=tbody.rows.length;
    }}catch(e){{}}
  }};
  es.onerror=()=>{{setTimeout(connectSSE,3000);}};
}}

async function loadLogs(lines){{
  const res=await fetch('/api/logs?lines='+lines).then(r=>r.json());
  document.getElementById('logs').textContent=(res.lines||[]).join('\\n');
}}

async function saveRelay(){{
  await post('/api/config',{{relay:{{primary_url:document.getElementById('c-relay1').value,secondary_url:document.getElementById('c-relay2').value,timeout_sec:parseInt(document.getElementById('c-relay-timeout').value)||8,max_retry:parseInt(document.getElementById('c-relay-retry').value)||3}}}});
  toast('Relay settings saved');
}}

async function saveMqtt(){{
  await post('/api/config',{{mqtt:{{host:document.getElementById('c-mqtt-host').value,port:parseInt(document.getElementById('c-mqtt-port').value)||8883,user:document.getElementById('c-mqtt-user').value,password:document.getElementById('c-mqtt-pass').value,base_topic:document.getElementById('c-mqtt-topic').value,tls:document.getElementById('c-mqtt-tls').value==='true'}}}});
  toast('MQTT settings saved');
}}

async function saveTenant(){{
  await post('/api/config',{{register:{{tid:document.getElementById('t-tid').value,tenant_lacisid:document.getElementById('t-lacis').value,tenant_cic:document.getElementById('t-cic').value,tenant_email:document.getElementById('t-email').value,gate_url:document.getElementById('t-gate').value}}}});
  toast('Tenant settings saved');
}}

async function saveDevice(){{
  await post('/api/config',{{device:{{name:document.getElementById('s-device-name').value,device_type:document.getElementById('s-device-type').value,product_type:document.getElementById('s-product-type').value,product_code:document.getElementById('s-product-code').value,interface:document.getElementById('s-interface').value}}}});
  toast('Device settings saved');
  const dn=document.getElementById('s-device-name').value;
  document.getElementById('device-name-display').innerHTML=dn||'<span class="unnamed">(未設定)</span>';
}}

async function saveStore(){{
  await post('/api/config',{{store:{{ring_size:parseInt(document.getElementById('s-ring-size').value)||2000,max_db_size:parseInt(document.getElementById('s-max-db').value)||100000,flush_interval_sec:parseInt(document.getElementById('s-flush-int').value)||5,flush_batch:parseInt(document.getElementById('s-flush-batch').value)||50}}}});
  toast('Store settings saved (restart may be required)');
}}

async function saveAccess(){{
  const allowed=document.getElementById('n-allowed').value.split('\\n').map(v=>v.trim()).filter(v=>v);
  await post('/api/config',{{access:{{allowed_sources:allowed}}}});
  toast('Access settings saved');
}}

async function post(u,d){{
  await fetch(u,{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify(d)}});
}}

function toast(m){{
  const t=document.getElementById('toast');
  t.textContent=m;
  t.style.display='block';
  setTimeout(()=>t.style.display='none',2500);
}}

// ========== Capture (tshark) Functions ==========
let capCfg={{}};
let configuredRooms=[];  // 設定済みroom一覧（ソート済み）

async function refreshCaptureStatus(){{
  try{{
    const r=await fetch('/api/capture/status');
    const s=await r.json();
    if(!s.ok)return;
    document.getElementById('cap-status').textContent=s.running?'RUNNING':'STOPPED';
    document.getElementById('cap-status').className='value '+(s.running?'good':'warn');
    document.getElementById('cap-queue').textContent=s.queue_size||0;
    document.getElementById('cap-sent').textContent=s.sent_count||0;
    document.getElementById('cap-drop').textContent=s.drop_count||0;
    document.getElementById('cap-mode').textContent=s.syn_only!==false?'軽量(SYN)':'詳細';
    document.getElementById('cap-mode').style.color=s.syn_only!==false?'var(--success)':'#e67e22';
    document.getElementById('cap-rooms').textContent=s.rooms_count||0;
  }}catch(e){{console.error('capture status error',e);}}
}}

async function loadCaptureConfig(){{
  try{{
    const r=await fetch('/api/capture/config');
    const d=await r.json();
    if(!d.ok)return;
    capCfg=d.config||{{}};
    document.getElementById('cap-enabled').value=String(capCfg.mode?.enabled??false);
    document.getElementById('cap-dryrun').value=String(capCfg.mode?.dry_run??false);
    document.getElementById('cap-syn-only').checked=capCfg.capture?.syn_only!==false;
    document.getElementById('cap-post-url').value=capCfg.post?.url||'';
    document.getElementById('cap-batch-events').value=capCfg.post?.batch_max_events||500;
    document.getElementById('cap-batch-sec').value=capCfg.post?.batch_max_seconds||30;
    // rooms辞書をテキスト形式に変換
    const rooms=capCfg.rooms||{{}};
    const lines=Object.entries(rooms).map(([ip,room])=>ip+'='+room);
    document.getElementById('cap-rooms-text').value=lines.join('\\n');
    document.getElementById('cap-rooms-status').textContent=Object.keys(rooms).length+' rooms configured';
    // 設定済みroom一覧を抽出（ユニーク＆ソート済み）
    const roomSet=new Set(Object.values(rooms));
    configuredRooms=[...roomSet].sort((a,b)=>{{
      const na=parseInt(a),nb=parseInt(b);
      if(!isNaN(na)&&!isNaN(nb))return na-nb;
      return a.localeCompare(b);
    }});
    // display_filter チェックボックス初期化
    const df=capCfg.display_filter||{{}};
    document.getElementById('cap-filter-local').checked=df.exclude_local_ip!==false;
    document.getElementById('cap-filter-ptr').checked=df.exclude_ptr!==false;
    document.getElementById('cap-filter-photo').checked=df.exclude_photo_sync!==false;
    document.getElementById('cap-filter-oscheck').checked=df.exclude_os_check!==false;
    document.getElementById('cap-filter-ad').checked=df.exclude_ad_tracker!==false;
    document.getElementById('cap-filter-dns').checked=df.exclude_dns!==false;
    document.getElementById('cap-filter-background').checked=df.exclude_background!==false;
  }}catch(e){{console.error('capture config error',e);}}
}}

async function startCapture(){{
  try{{
    const r=await fetch('/api/capture/start',{{method:'POST'}});
    const d=await r.json();
    toast(d.ok?'Capture started':'Failed to start: '+(d.error||''));
    refreshCaptureStatus();
  }}catch(e){{toast('Error: '+e);}}
}}

async function stopCapture(){{
  try{{
    const r=await fetch('/api/capture/stop',{{method:'POST'}});
    const d=await r.json();
    toast(d.ok?'Capture stopped':'Failed to stop');
    refreshCaptureStatus();
  }}catch(e){{toast('Error: '+e);}}
}}

async function saveCaptureConfig(){{
  const synOnly=document.getElementById('cap-syn-only').checked;
  const payload={{
    mode:{{
      enabled:document.getElementById('cap-enabled').value==='true',
      dry_run:document.getElementById('cap-dryrun').value==='true'
    }},
    capture:{{
      syn_only:synOnly
    }},
    post:{{
      url:document.getElementById('cap-post-url').value,
      batch_max_events:parseInt(document.getElementById('cap-batch-events').value)||500,
      batch_max_seconds:parseInt(document.getElementById('cap-batch-sec').value)||30
    }}
  }};
  try{{
    const r=await fetch('/api/capture/config',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify(payload)}});
    const d=await r.json();
    toast(d.ok?'Capture config saved (restart capture to apply)':'Failed to save');
    loadCaptureConfig();
  }}catch(e){{toast('Error: '+e);}}
}}

async function saveRooms(){{
  const text=document.getElementById('cap-rooms-text').value;
  const lines=text.split('\\n').filter(l=>l.trim());
  const rooms={{}};
  for(const line of lines){{
    const parts=line.split('=');
    if(parts.length===2){{
      const ip=parts[0].trim();
      const room=parts[1].trim();
      if(ip&&room)rooms[ip]=room;
    }}
  }}
  try{{
    const r=await fetch('/api/capture/rooms',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify(rooms)}});
    const d=await r.json();
    toast(d.ok?'Rooms saved ('+Object.keys(d.rooms||{{}}).length+' entries)':'Failed to save');
    document.getElementById('cap-rooms-status').textContent=Object.keys(d.rooms||{{}}).length+' rooms configured';
    refreshCaptureStatus();
  }}catch(e){{toast('Error: '+e);}}
}}

// サービス・カテゴリ判定はバックエンド（domain_services.py）で行う
// APIレスポンスの domain_service, domain_category フィールドを使用

// 連続した同一ドメインをグループ化（ポート・IPは無視）
function groupEvents(events){{
  if(!events||events.length===0)return[];
  const groups=[];
  let current=null;
  events.forEach(e=>{{
    const domain=e.http_host||e.tls_sni||e.resolved_domain||e.dns_qry||'';
    const key=domain?(e.room_no+'|'+domain):(e.room_no+'|'+e.dst_ip);
    if(current&&current.key===key){{
      current.count++;
      current.ids.push(e.id);
    }}else{{
      if(current)groups.push(current);
      current={{key,event:e,count:1,ids:[e.id]}};
    }}
  }});
  if(current)groups.push(current);
  return groups;
}}

let lastSeenIds=new Set();

async function updateThreatIntel(){{
  toast('Updating threat intel...');
  try{{
    const r=await fetch('/api/threat/update',{{method:'POST'}});
    const d=await r.json();
    if(d.ok){{
      toast('Threat intel updated');
      refreshThreatStatus();
    }}else{{
      toast('Update failed: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

async function refreshThreatStatus(){{
  try{{
    const r=await fetch('/api/threat/status');
    const d=await r.json();
    if(d.ok){{
      const info=document.getElementById('threat-info');
      const total=(d.malware_domains||0)+(d.adult_domains||0)+(d.gambling_domains||0);
      info.textContent='[ThreatDB: '+total.toLocaleString()+' domains, '+(d.tor_exit_ips||0)+' Tor IPs]';
    }}
  }}catch(e){{}}
}}

function showEventDetail(e){{
  const lines=[
    '📅 Time: '+(e.time||'-'),
    '🏠 Room: '+(e.room_no||'UNK'),
    '',
    '📤 SrcIP: '+(e.src_ip||'-'),
    '📥 DstIP: '+(e.dst_ip||'-')+':'+(e.dst_port||'?'),
    '',
    '🔎 DNS Query: '+(e.dns_qry||'-'),
    '🌐 HTTP Host: '+(e.http_host||'-'),
    '🔒 TLS SNI: '+(e.tls_sni||'-'),
    '📋 PTR: '+(e.resolved_domain||'-'),
    '',
    '🏷️ Service(Domain): '+(e.domain_service||'-')+' ['+(e.domain_category||'-')+']',
    '🌐 ASN: '+(e.asn?'AS'+e.asn:'-'),
    '🏢 ASN Service: '+(e.asn_service||'-')+' ['+(e.asn_category||'-')+']',
    '',
    '⚠️ Threat: '+(e.threat||'none'),
  ];
  alert(lines.join('\\n'));
}}

// バックエンドフィルタ設定を更新
async function updateBackendFilter(){{
  const displayFilter={{
    exclude_local_ip:document.getElementById('cap-filter-local').checked,
    exclude_ptr:document.getElementById('cap-filter-ptr').checked,
    exclude_photo_sync:document.getElementById('cap-filter-photo').checked,
    exclude_os_check:document.getElementById('cap-filter-oscheck').checked,
    exclude_ad_tracker:document.getElementById('cap-filter-ad').checked,
    exclude_dns:document.getElementById('cap-filter-dns').checked,
    exclude_background:document.getElementById('cap-filter-background').checked,
  }};
  try{{
    const r=await fetch('/api/capture/config',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{display_filter:displayFilter}})}});
    const d=await r.json();
    if(d.ok){{
      console.log('Backend filter updated:',displayFilter);
    }}
  }}catch(e){{console.error('Failed to update backend filter:',e);}}
  refreshCaptureEvents();
}}

async function refreshCaptureEvents(){{
  try{{
    const r=await fetch('/api/capture/events?limit=1000');
    const d=await r.json();
    if(!d.ok)return;
    captureEventsCache=d.events||[];
    updateStatsFromCache();
    document.getElementById('cap-event-count').textContent=d.count||0;
    document.getElementById('cap-event-info').textContent='Queue: '+d.total_queued+' events';
    // Room フィルタのオプション更新（設定済みroom全件 + イベント由来のroom）
    const filterEl=document.getElementById('cap-room-filter');
    const currentFilter=filterEl.value;
    const eventRooms=new Set((d.events||[]).map(e=>e.room_no).filter(r=>r&&r!=='UNK'));
    // 設定済みroom + イベント由来のroom（マージ）
    const allRooms=new Set([...configuredRooms,...eventRooms]);
    const sortedRooms=[...allRooms].sort((a,b)=>{{
      const na=parseInt(a),nb=parseInt(b);
      if(!isNaN(na)&&!isNaN(nb))return na-nb;
      return a.localeCompare(b);
    }});
    const opts=['<option value="">All</option>'];
    sortedRooms.forEach(r=>opts.push('<option value="'+r+'"'+(r===currentFilter?' selected':'')+'>'+r+'</option>'));
    filterEl.innerHTML=opts.join('');
    const tbody=document.getElementById('cap-events-body');
    tbody.innerHTML='';
    if(!d.events||d.events.length===0){{
      tbody.innerHTML='<tr><td colspan="9" style="color:var(--text-muted)">No capture events</td></tr>';
      return;
    }}
    // フィルタ適用
    let events=[...d.events].reverse();
    if(currentFilter)events=events.filter(e=>e.room_no===currentFilter);
    // 方向フィルタ
    const dirFilter=document.getElementById('cap-dir-filter').value;
    if(dirFilter==='up')events=events.filter(e=>e.room_no&&e.room_no!=='UNK');
    if(dirFilter==='down')events=events.filter(e=>!e.room_no||e.room_no==='UNK');
    // 注: ローカル/PTR/写真同期/OS接続/広告計測/DNS/バックグラウンドはバックエンドフィルタで処理済み
    // Streaming除外
    if(document.getElementById('cap-filter-streaming').checked){{
      events=events.filter(e=>{{
        const domain=(e.http_host||e.tls_sni||e.resolved_domain||e.dns_qry||'').toLowerCase();
        const streamPatterns=['youtube','googlevideo','youtubei','netflix','nflxvideo','nicovideo','twitch','abema','hulu','dazn','primevideo','disneyplus','spotify'];
        for(const p of streamPatterns){{if(domain.includes(p))return false;}}
        return true;
      }});
    }}
    // SNS除外
    if(document.getElementById('cap-filter-sns').checked){{
      events=events.filter(e=>{{
        const domain=(e.http_host||e.tls_sni||e.resolved_domain||e.dns_qry||'').toLowerCase();
        const snsPatterns=['instagram','facebook','fbcdn','twitter','x.com','twimg','threads','tiktok','bytedance','pinterest','linkedin','snapchat'];
        for(const p of snsPatterns){{if(domain.includes(p))return false;}}
        return true;
      }});
    }}
    // Google除外
    if(document.getElementById('cap-filter-google').checked){{
      events=events.filter(e=>{{
        const domain=(e.http_host||e.tls_sni||e.resolved_domain||e.dns_qry||'').toLowerCase();
        if(domain.includes('google')||domain.includes('gstatic')||domain.includes('googleapis')||domain.includes('ggpht')||domain.includes('gvt1')||domain.includes('gvt2'))return false;
        return true;
      }});
    }}
    // Apple除外
    if(document.getElementById('cap-filter-apple').checked){{
      events=events.filter(e=>{{
        const domain=(e.http_host||e.tls_sni||e.resolved_domain||e.dns_qry||'').toLowerCase();
        if(domain.includes('apple')||domain.includes('icloud')||domain.includes('itunes')||domain.includes('mzstatic')||domain.includes('mac.com'))return false;
        return true;
      }});
    }}
    // キーワードフィルタ（入力があれば絞り込み）
    const keyword=(document.getElementById('cap-filter-keyword').value||'').toLowerCase().trim();
    if(keyword){{
      events=events.filter(e=>{{
        const domain=(e.http_host||e.tls_sni||e.resolved_domain||e.dns_qry||'').toLowerCase();
        const svc=(e.domain_service||e.asn_service||'').toLowerCase();
        const cat=(e.domain_category||'').toLowerCase();
        return domain.includes(keyword)||svc.includes(keyword)||cat.includes(keyword)||(e.src_ip||'').includes(keyword)||(e.dst_ip||'').includes(keyword)||(e.room_no||'').toLowerCase().includes(keyword);
      }});
    }}
    // グループ化
    const groups=groupEvents(events);
    const currentIds=new Set();
    groups.forEach(g=>{{
      g.ids.forEach(id=>currentIds.add(id));
      const e=g.event;
      const row=tbody.insertRow();
      const time=e.time?e.time.split('T')[1]?.substring(0,8)||'':'';
      // 方向: room_no!=UNK → 送信↑、UNK → 受信↓
      const dir=e.room_no&&e.room_no!=='UNK'?'<span style="color:#38a169;font-weight:bold">↑</span>':'<span style="color:#3182ce;font-weight:bold">↓</span>';
      // サービス判定（バックエンドで解決済み→ASNフォールバック）
      const domain=e.http_host||e.tls_sni||e.resolved_domain||e.dns_qry||'';
      let svc=e.domain_service||null;
      let svcSource=e.domain_service?'domain':'';
      // ドメインで判定できなければASNを使用
      if(!svc&&e.asn_service){{
        svc=e.asn_service;
        svcSource='asn';
      }}
      // サービス表示（ASN由来は青、ドメイン由来はグレー、不明は赤）
      let svcCell;
      if(svc){{
        const svcColor=svcSource==='asn'?'#2563eb':'#718096';
        svcCell='<span style="color:'+svcColor+';font-size:10px" title="'+(svcSource==='asn'?'ASN:'+e.asn:'Domain')+'">'+svc+'</span>';
      }}else{{
        svcCell='<span style="color:#e53e3e;font-weight:bold;font-size:10px">???</span>';
      }}
      // 脅威/カテゴリフラグ
      const threat=e.threat;
      const threatColors={{'malware':'#dc2626','adult':'#9333ea','gambling':'#ea580c','fakenews':'#ca8a04','tor':'#0891b2'}};
      const categoryColors={{'Streaming':'#2563eb','SNS':'#7c3aed','Photo':'#059669','Messenger':'#0891b2','Ad':'#ea580c','Tracker':'#ca8a04','VPN':'#dc2626','Media':'#8b5cf6','Creative':'#ec4899'}};
      let threatCell='';
      if(threat){{
        threatCell='<span style="color:'+(threatColors[threat]||'#dc2626')+';font-weight:bold;font-size:10px">'+threat.toUpperCase()+'</span>';
      }}else{{
        // カテゴリ判定（バックエンドで解決済み or ポート検出）
        let category=e.domain_category||null;
        // VPNポート検出
        if(!category){{
          const port=e.dst_port;
          if(port===1194||port===443&&!svc)category='VPN?';
          else if(port===51820)category='VPN';
        }}
        if(category){{
          const catColor=categoryColors[category]||'#6b7280';
          threatCell='<span style="color:'+catColor+';font-size:10px">'+category+'</span>';
        }}
      }}
      const dstDisplay=e.dst_ip+(e.dst_port?':'+e.dst_port:'');
      // Domain統合表示（優先順位: http_host > tls_sni > dns_qry > resolved_domain）
      let domainDisp=e.http_host||e.tls_sni||e.dns_qry||(e.resolved_domain?'['+e.resolved_domain+']':'')||'';
      const domainFull=domainDisp;
      if(domainDisp.length>35)domainDisp=domainDisp.substring(0,35)+'…';
      const badge=g.count>1?'<span style="background:#3182ce;color:#fff;border-radius:10px;padding:1px 6px;font-size:10px">'+g.count+'</span>':'';
      const isNew=!lastSeenIds.has(e.id);
      if(isNew)row.className='new-row';
      // 未知サービスは薄い赤、脅威ありは濃い色
      if(threat)row.style.background=threat==='malware'?'#fef2f2':threat==='adult'?'#faf5ff':threat==='tor'?'#ecfeff':'#fffbeb';
      else if(!svc&&domain)row.style.background='#fff5f5';
      row.style.cursor='pointer';
      row.onclick=()=>showEventDetail(e);
      row.innerHTML='<td>'+time+'</td><td style="text-align:center">'+badge+'</td><td>'+e.room_no+'</td><td>'+dir+'</td><td>'+svcCell+'</td><td>'+threatCell+'</td><td>'+e.src_ip+'</td><td>'+dstDisplay+'</td><td title="'+domainFull+'">'+domainDisp+'</td>';
    }});
    lastSeenIds=currentIds;
  }}catch(e){{console.error('capture events error',e);}}
}}

// ========== Hardware Info Functions ==========
async function refreshHardware(){{
  try{{
    const r=await fetch('/api/hardware');
    const d=await r.json();
    if(!d)return;
    // RAM
    const ram=d.memory;
    if(ram){{
      document.getElementById('hw-ram').textContent=ram.used_mb+'MB / '+ram.total_mb+'MB ('+ram.usage_percent+'%)';
      document.getElementById('hw-ram').className='value '+(ram.usage_percent>80?'bad':ram.usage_percent>60?'warn':'good');
    }}
    // CPU
    const cpu=d.cpu;
    if(cpu){{
      document.getElementById('hw-cpu').textContent=cpu.usage_percent+'%';
      document.getElementById('hw-cpu').className='value '+(cpu.usage_percent>80?'bad':cpu.usage_percent>60?'warn':'good');
      document.getElementById('hw-temp').textContent=cpu.temperature_celsius?cpu.temperature_celsius+'°C':'-';
      document.getElementById('hw-temp').className='value '+(cpu.temperature_celsius>70?'bad':cpu.temperature_celsius>60?'warn':'good');
      document.getElementById('hw-load').textContent=cpu.load_average['1min']+' / '+cpu.load_average['5min']+' / '+cpu.load_average['15min'];
    }}
    // Storage
    if(d.storage&&d.storage.length>0){{
      const s=d.storage[0];
      document.getElementById('hw-storage').textContent=s.used_gb+'GB / '+s.total_gb+'GB ('+s.usage_percent+'%)';
      document.getElementById('hw-storage').className='value '+(s.usage_percent>90?'bad':s.usage_percent>80?'warn':'good');
    }}
    // Network
    if(d.network){{
      const eth=d.network.find(n=>n.name==='eth0'||n.name==='end0');
      const wlan=d.network.find(n=>n.name==='wlan0');
      if(eth){{
        document.getElementById('hw-eth').textContent=eth.ip||'Not connected';
        document.getElementById('hw-eth').className='value '+(eth.is_connected?'good':'warn');
      }}
      if(wlan){{
        document.getElementById('hw-wlan').textContent=wlan.ip||'Not connected';
        document.getElementById('hw-wlan').className='value '+(wlan.is_connected?'good':'warn');
      }}
    }}
  }}catch(e){{console.error('hardware error',e);}}
}}

// ========== WiFi Functions ==========
async function refreshWifi(){{
  try{{
    const r=await fetch('/api/wifi/status');
    const d=await r.json();
    document.getElementById('wifi-status').textContent=d.connected?'Connected':'Disconnected';
    document.getElementById('wifi-status').className='value '+(d.connected?'good':'warn');
    document.getElementById('wifi-ssid').textContent=d.ssid||'-';
    document.getElementById('wifi-signal').textContent=d.signal_strength?d.signal_strength+' dBm':'-';
    document.getElementById('wifi-ip').textContent=d.ip_address||'-';
  }}catch(e){{console.error('wifi status error',e);}}
}}

async function scanWifi(){{
  toast('Scanning networks...');
  try{{
    const r=await fetch('/api/wifi/networks');
    const d=await r.json();
    const el=document.getElementById('wifi-networks');
    if(d.networks&&d.networks.length>0){{
      const html=d.networks.map(n=>'<div style="padding:4px 0;border-bottom:1px solid var(--border);cursor:pointer" onclick="document.getElementById(\\'wifi-new-ssid\\').value=\\''+n.ssid+'\\'"><b>'+n.ssid+'</b> ('+n.signal+'%, '+n.security+')</div>').join('');
      el.innerHTML=html;
    }}else{{
      el.textContent='No networks found';
    }}
  }}catch(e){{toast('Scan error: '+e);}}
}}

async function connectWifi(){{
  const ssid=document.getElementById('wifi-new-ssid').value;
  const pass=document.getElementById('wifi-new-pass').value;
  if(!ssid){{toast('Please enter SSID');return;}}
  toast('Connecting to '+ssid+'...');
  try{{
    const r=await fetch('/api/wifi/connect',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{ssid,password:pass}})}});
    const d=await r.json();
    if(d.ok){{
      toast('Connected to '+ssid);
      refreshWifi();
      refreshHardware();
    }}else{{
      toast('Failed: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

// ========== WiFi Config CRUD Functions ==========
let wifiMaxConfigs=6;

async function refreshWifiConfigs(){{
  try{{
    const r=await fetch('/api/wifi/configs');
    const d=await r.json();
    if(!d.ok)return;
    wifiMaxConfigs=d.max_configs||6;
    const tbody=document.getElementById('wifi-configs-body');
    tbody.innerHTML='';
    if(!d.configs||d.configs.length===0){{
      tbody.innerHTML='<tr><td colspan="4" style="color:var(--text-muted)">No WiFi configs</td></tr>';
      return;
    }}
    d.configs.forEach((c,i)=>{{
      const row=tbody.insertRow();
      const upBtn=i>0?'<button class="btn btn-sm" onclick="moveWifiConfig('+i+','+(i-1)+')">↑</button>':'';
      const downBtn=i<d.configs.length-1?'<button class="btn btn-sm" onclick="moveWifiConfig('+i+','+(i+1)+')">↓</button>':'';
      row.innerHTML='<td>'+(i+1)+'</td><td>'+c.ssid+'</td><td>'+c.password+'</td><td>'+upBtn+downBtn+'<button class="btn btn-sm btn-danger" onclick="deleteWifiConfig('+i+')">Del</button></td>';
    }});
  }}catch(e){{console.error('wifi configs error',e);}}
}}

async function addWifiConfig(){{
  const ssid=document.getElementById('wifi-cfg-ssid').value;
  const pass=document.getElementById('wifi-cfg-pass').value;
  if(!ssid){{toast('SSIDを入力してください');return;}}
  try{{
    const r=await fetch('/api/wifi/configs',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{ssid,password:pass}})}});
    const d=await r.json();
    if(d.ok){{
      toast('WiFi設定を追加しました');
      document.getElementById('wifi-cfg-ssid').value='';
      document.getElementById('wifi-cfg-pass').value='';
      refreshWifiConfigs();
    }}else{{
      toast('追加失敗: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

async function deleteWifiConfig(index){{
  if(!confirm('この WiFi 設定を削除しますか?'))return;
  try{{
    const r=await fetch('/api/wifi/configs/'+index,{{method:'DELETE'}});
    const d=await r.json();
    if(d.ok){{
      toast('WiFi設定を削除しました');
      refreshWifiConfigs();
    }}else{{
      toast('削除失敗: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

async function moveWifiConfig(oldIdx,newIdx){{
  try{{
    const r=await fetch('/api/wifi/configs/reorder',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{old_index:oldIdx,new_index:newIdx}})}});
    const d=await r.json();
    if(d.ok){{
      refreshWifiConfigs();
    }}else{{
      toast('並べ替え失敗: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

async function resetWifiConfigs(){{
  if(!confirm('WiFi設定をデフォルト(cluster1-6)にリセットしますか?'))return;
  try{{
    const r=await fetch('/api/wifi/configs/reset',{{method:'POST'}});
    const d=await r.json();
    if(d.ok){{
      toast('WiFi設定をリセットしました');
      refreshWifiConfigs();
    }}else{{
      toast('リセット失敗: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

async function autoConnectWifi(){{
  toast('保存されたSSIDを順番に試行中...');
  try{{
    const r=await fetch('/api/wifi/auto-connect',{{method:'POST'}});
    const d=await r.json();
    if(d.ok){{
      toast(d.message||'接続成功');
      refreshWifi();
      refreshHardware();
    }}else{{
      toast('接続失敗: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

// ========== NTP Functions ==========
async function refreshNtp(){{
  try{{
    const r=await fetch('/api/ntp/status');
    const d=await r.json();
    document.getElementById('ntp-sync').textContent=d.synchronized?'Yes':'No';
    document.getElementById('ntp-sync').className='value '+(d.synchronized?'good':'warn');
    document.getElementById('ntp-server').textContent=d.ntp_server||'-';
    document.getElementById('ntp-tz').textContent=d.timezone||'-';
    document.getElementById('ntp-time').textContent=d.current_time||'-';
  }}catch(e){{console.error('ntp status error',e);}}
}}

async function saveNtp(){{
  const server=document.getElementById('ntp-new-server').value;
  const tz=document.getElementById('ntp-new-tz').value;
  try{{
    if(server){{
      const r=await fetch('/api/ntp/server',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{server}})}});
      const d=await r.json();
      if(!d.ok){{toast('Server error: '+(d.error||''));return;}}
    }}
    if(tz){{
      const r=await fetch('/api/ntp/timezone',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{timezone:tz}})}});
      const d=await r.json();
      if(!d.ok){{toast('Timezone error: '+(d.error||''));return;}}
    }}
    toast('NTP settings saved');
    refreshNtp();
  }}catch(e){{toast('Error: '+e);}}
}}

// ========== Sync Functions ==========
async function refreshSync(){{
  try{{
    const r=await fetch('/api/sync/status');
    const d=await r.json();
    document.getElementById('sync-last').textContent=d.last_sync||'Never';
    document.getElementById('sync-peers').textContent=(d.peers||[]).length;
    document.getElementById('sync-init').textContent=d.initiator_configured?'Configured':'Not set';
    document.getElementById('sync-init').className='value '+(d.initiator_configured?'good':'warn');
    document.getElementById('sync-resp').textContent=d.responder_configured?'Configured':'Not set';
    document.getElementById('sync-resp').className='value '+(d.responder_configured?'good':'warn');
    // Peer list
    const listEl=document.getElementById('sync-peer-list');
    if(d.peers&&d.peers.length>0){{
      listEl.innerHTML=d.peers.map(p=>'<div style="padding:4px 0;display:flex;align-items:center;justify-content:space-between"><span>'+p.host+':'+p.port+'</span><button class="btn btn-sm btn-danger" onclick="removeSyncPeer(\\''+p.host+'\\')">Remove</button></div>').join('');
    }}else{{
      listEl.textContent='No peers configured';
    }}
  }}catch(e){{console.error('sync status error',e);}}
}}

async function saveSyncPassphrases(){{
  const init=document.getElementById('sync-init-pass').value;
  const resp=document.getElementById('sync-resp-pass').value;
  if(!init&&!resp){{toast('Please enter at least one passphrase');return;}}
  try{{
    const r=await fetch('/api/sync/config',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{initiator_passphrase:init,responder_passphrase:resp}})}});
    const d=await r.json();
    if(d.ok){{
      toast('Passphrases saved');
      document.getElementById('sync-init-pass').value='';
      document.getElementById('sync-resp-pass').value='';
      refreshSync();
    }}else{{
      toast('Failed: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

async function addSyncPeer(){{
  const host=document.getElementById('sync-peer-host').value;
  const port=parseInt(document.getElementById('sync-peer-port').value)||8080;
  if(!host){{toast('Please enter peer host');return;}}
  try{{
    const r=await fetch('/api/sync/peer',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{host,port,action:'add'}})}});
    const d=await r.json();
    if(d.ok){{
      toast('Peer added');
      document.getElementById('sync-peer-host').value='';
      refreshSync();
    }}else{{
      toast('Failed: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

async function removeSyncPeer(host){{
  try{{
    const r=await fetch('/api/sync/peer',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{host,action:'remove'}})}});
    const d=await r.json();
    if(d.ok){{
      toast('Peer removed');
      refreshSync();
    }}else{{
      toast('Failed: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

async function syncWithPeer(){{
  const host=document.getElementById('sync-peer-host').value;
  const port=parseInt(document.getElementById('sync-peer-port').value)||8080;
  if(!host){{toast('Please enter peer host');return;}}
  toast('Syncing with '+host+'...');
  try{{
    const r=await fetch('/api/sync/peer',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{host,port,action:'sync'}})}});
    const d=await r.json();
    if(d.ok){{
      toast('Sync completed: '+JSON.stringify(d.stats||{{}}));
      refreshSync();
    }}else{{
      toast('Sync failed: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

async function syncAll(){{
  toast('Syncing all peers...');
  try{{
    const r=await fetch('/api/sync/all',{{method:'POST'}});
    const d=await r.json();
    if(d.ok){{
      toast('Sync all completed');
      refreshSync();
    }}else{{
      toast('Sync failed: '+(d.error||''));
    }}
  }}catch(e){{toast('Error: '+e);}}
}}

async function exportCache(){{
  try{{
    const r=await fetch('/api/sync/export');
    const d=await r.json();
    // Download as JSON
    const blob=new Blob([JSON.stringify(d,null,2)],{{type:'application/json'}});
    const url=URL.createObjectURL(blob);
    const a=document.createElement('a');
    a.href=url;
    a.download='is20s_cache_'+new Date().toISOString().slice(0,10)+'.json';
    a.click();
    URL.revokeObjectURL(url);
    toast('Cache exported');
  }}catch(e){{toast('Export error: '+e);}}
}}

async function importCache(event){{
  const file=event.target.files[0];
  if(!file)return;
  try{{
    const text=await file.text();
    const data=JSON.parse(text);
    const r=await fetch('/api/sync/import',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify(data)}});
    const d=await r.json();
    if(d.ok){{
      toast('Imported: merged='+d.stats.merged+', added='+d.stats.added);
    }}else{{
      toast('Import failed: '+(d.error||''));
    }}
  }}catch(e){{toast('Import error: '+e);}}
  event.target.value='';
}}

// ===== SpeedDial Functions =====
async function refreshSpeedDial(){{
  try{{
    const r=await fetch('/api/speeddial');
    const d=await r.json();
    if(d.ok){{
      document.getElementById('sd-current').value=d.text;
    }}else{{
      document.getElementById('sd-current').value='Error: '+(d.error||'Failed to load');
    }}
  }}catch(e){{
    document.getElementById('sd-current').value='Error: '+e;
  }}
}}

function copySpeedDial(){{
  const text=document.getElementById('sd-current').value;
  navigator.clipboard.writeText(text).then(()=>{{
    toast('設定をクリップボードにコピーしました');
  }}).catch(e=>{{
    // Fallback for older browsers
    const ta=document.getElementById('sd-current');
    ta.select();
    document.execCommand('copy');
    toast('設定をコピーしました');
  }});
}}

async function applySpeedDial(){{
  const text=document.getElementById('sd-input').value.trim();
  if(!text){{
    toast('貼り付けるテキストがありません');
    return;
  }}
  const resultEl=document.getElementById('sd-result');
  try{{
    const r=await fetch('/api/speeddial',{{
      method:'POST',
      headers:{{'Content-Type':'application/json'}},
      body:JSON.stringify({{text:text}})
    }});
    const d=await r.json();
    resultEl.style.display='block';
    if(d.ok){{
      resultEl.style.background='#d4edda';
      resultEl.style.color='#155724';
      resultEl.innerHTML='<b>✅ 適用完了</b><br>'+d.applied.join('<br>');
      toast('設定を適用しました');
      refreshSpeedDial();
      refreshWifiConfigs();
      refreshNtp();
    }}else{{
      resultEl.style.background='#f8d7da';
      resultEl.style.color='#721c24';
      let msg='<b>⚠️ エラーあり</b>';
      if(d.applied&&d.applied.length>0)msg+='<br>適用済み: '+d.applied.join(', ');
      if(d.errors)msg+='<br>エラー: '+d.errors.join(', ');
      resultEl.innerHTML=msg;
      toast('一部エラーが発生しました');
    }}
  }}catch(e){{
    resultEl.style.display='block';
    resultEl.style.background='#f8d7da';
    resultEl.style.color='#721c24';
    resultEl.innerHTML='<b>❌ 通信エラー</b><br>'+e;
    toast('通信エラー: '+e);
  }}
}}

// === Statistics ===
let statsSelectedRoom=null;
let statsData={{rooms:{{}},categories:{{}}}};
let statsCatColors={{}};
let statsColorIdx=0;
const STATS_COLORS=['#2563eb','#7c3aed','#0891b2','#059669','#6366f1','#ea580c','#ca8a04','#dc2626','#8b5cf6','#ec4899','#14b8a6','#f59e0b'];
function statsFilterEvents(events){{
  return events.filter(e=>{{
    const dst=e.dst_ip||'';
    // DNSクエリ以外の場合のみローカルIP宛を除外（DNS宛先はルーターになるため）
    if(!e.dns_qry){{
      if(dst.startsWith('192.168.')||dst.startsWith('10.')||dst.startsWith('127.'))return false;
      if(dst.match(/^172\.(1[6-9]|2[0-9]|3[01])\./))return false;
    }}
    const domain=(e.http_host||e.tls_sni||e.resolved_domain||e.dns_qry||'').toLowerCase();
    if(domain.match(/^ec2-|\.compute\.amazonaws\.com|\.in-addr\.arpa/))return false;
    const photoPatterns=['googleusercontent.com','photos.google.com','lh3.google.com','lh4.google.com','lh5.google.com','lh6.google.com','icloud.com','aaplimg.com','apple-dns.net','mzstatic.com'];
    for(const p of photoPatterns)if(domain.includes(p))return false;
    const osPatterns=['msftncsi.com','msftconnecttest.com','connectivitycheck.gstatic.com','connectivitycheck.android.com','captive.apple.com','detectportal.firefox.com'];
    for(const p of osPatterns)if(domain.includes(p))return false;
    const adPatterns=['googlesyndication','doubleclick','googleadservices','applovin','fivecdm','adtng','adnxs','adsrvr','criteo','taboola','outbrain','pubmatic','rubiconproject','openx','casalemedia','adcolony','chartboost','vungle','ironsrc','fyber','inmobi','mopub','unityads','adjust','appsflyer','branch','amplitude','segment','mixpanel','google-analytics','hotjar','mouseflow','fullstory','crazyegg','heap'];
    for(const p of adPatterns)if(domain.includes(p))return false;
    return true;
  }});
}}
function getCatColor(cat){{
  if(!statsCatColors[cat])statsCatColors[cat]=STATS_COLORS[statsColorIdx++%STATS_COLORS.length];
  return statsCatColors[cat];
}}
let statsFirstLoad=true;
let captureEventsCache=null;
function showStatsLoading(){{
  if(!statsFirstLoad)return;
  const rc=document.getElementById('stats-room-chart');
  const oc=document.getElementById('stats-overall-chart');
  if(rc&&!rc.querySelector('.stats-loading'))rc.innerHTML='<div class="stats-loading"><div class="stats-spinner"></div>読み込み中...</div>';
  if(oc&&!oc.querySelector('.stats-loading'))oc.innerHTML='<div class="stats-loading"><div class="stats-spinner"></div>読み込み中...</div>';
}}
function updateStatsFromCache(){{
  if(!captureEventsCache)return;
  const allEvents=statsFilterEvents(captureEventsCache);
  const primaryOnly=document.getElementById('stats-primary-only')?.checked??true;

  // auxiliary件数カウント
  let auxCount=0;
  const events=allEvents.filter(e=>{{
    const role=e.domain_role||'primary';
    if(role==='auxiliary'){{auxCount++;return !primaryOnly;}}
    return true;
  }});

  // auxiliary件数表示
  const auxEl=document.getElementById('stats-aux-count');
  if(auxEl)auxEl.textContent=primaryOnly?'(補助通信 '+auxCount+'件 除外中)':'';

  const rooms={{}};const categories={{}};
  // 設定済みroom全件を0件で初期化（登録順を保持）
  configuredRooms.forEach(r=>{{
    rooms[r]={{total:0,cats:{{}},order:configuredRooms.indexOf(r)}};
  }});
  // 統計期間計算用
  let minTs=null,maxTs=null;
  events.forEach(e=>{{
    const room=e.room_no||'UNK';
    const cat=e.domain_category||'Unknown';
    const svc=e.domain_service||e.http_host||e.tls_sni||e.resolved_domain||'Unknown';
    if(!rooms[room])rooms[room]={{total:0,cats:{{}},order:9999}};
    rooms[room].total++;
    if(!rooms[room].cats[cat])rooms[room].cats[cat]={{total:0,svcs:{{}}}};
    rooms[room].cats[cat].total++;
    if(!rooms[room].cats[cat].svcs[svc])rooms[room].cats[cat].svcs[svc]=0;
    rooms[room].cats[cat].svcs[svc]++;
    if(!categories[cat])categories[cat]={{total:0,svcs:{{}}}};
    categories[cat].total++;
    if(!categories[cat].svcs[svc])categories[cat].svcs[svc]=0;
    categories[cat].svcs[svc]++;
    // タイムスタンプ更新
    if(e.time){{
      const ts=new Date(e.time);
      if(!minTs||ts<minTs)minTs=ts;
      if(!maxTs||ts>maxTs)maxTs=ts;
    }}
  }});
  statsData={{rooms,categories}};
  statsFirstLoad=false;
  // 統計期間表示
  const periodEl=document.getElementById('stats-period');
  if(periodEl){{
    if(minTs&&maxTs){{
      const fmt=d=>{{
        const yy=String(d.getFullYear()).slice(-2);
        const mm=String(d.getMonth()+1).padStart(2,'0');
        const dd=String(d.getDate()).padStart(2,'0');
        const hh=String(d.getHours()).padStart(2,'0');
        const mi=String(d.getMinutes()).padStart(2,'0');
        return yy+'/'+mm+'/'+dd+' '+hh+':'+mi;
      }};
      periodEl.textContent='統計期間: '+fmt(minTs)+' - '+fmt(maxTs);
    }}else{{
      periodEl.textContent='';
    }}
  }}
  updateRoomChart();
  updateOverallChart();
  if(statsSelectedRoom)updateRoomDetail(statsSelectedRoom);
}}
function updateRoomChart(){{
  const container=document.getElementById('stats-room-chart');
  // 設定順でソート（order属性使用、未設定roomは最後）
  const rooms=Object.entries(statsData.rooms).sort((a,b)=>{{
    const orderA=a[1].order??9999;
    const orderB=b[1].order??9999;
    return orderA-orderB;
  }});
  if(rooms.length===0){{container.innerHTML='<p style="color:var(--text-muted)">監視対象Roomが設定されていません</p>';return;}}
  document.getElementById('stats-room-count').textContent='('+rooms.length+' rooms)';
  const maxVal=Math.max(...rooms.map(r=>r[1].total),1);  // 0件のみの場合の0除算防止
  const roomIds=new Set(rooms.map(r=>r[0]));
  // Remove old rows
  container.querySelectorAll('.stats-bar-row[data-room]').forEach(el=>{{
    if(!roomIds.has(el.dataset.room))el.style.opacity='0';
  }});
  setTimeout(()=>container.querySelectorAll('.stats-bar-row[data-room]').forEach(el=>{{
    if(!roomIds.has(el.dataset.room))el.remove();
  }}),300);
  rooms.forEach(([room,data])=>{{
    const pct=(data.total/maxVal*100);
    const cats=Object.entries(data.cats).sort((a,b)=>b[1].total-a[1].total);
    const top3=cats.slice(0,3);
    const otherTotal=cats.slice(3).reduce((s,c)=>s+c[1].total,0);
    let row=container.querySelector('[data-room="'+room+'"]');
    if(!row){{
      row=document.createElement('div');
      row.className='stats-bar-row';
      row.dataset.room=room;
      row.onclick=()=>showRoomDetail(room);
      row.innerHTML='<div class="stats-bar-label">'+room+'</div><div class="stats-bar-wrap"><div class="stats-bar"></div></div><div class="stats-bar-count"></div>';
      const legend=container.querySelector('.stats-legend');
      if(legend)container.insertBefore(row,legend);else container.appendChild(row);
    }}
    const bar=row.querySelector('.stats-bar');
    bar.style.width=pct+'%';
    let segsHtml='';
    if(data.total===0){{
      // 0件の場合：薄いグレーバーで「通信なし」表示
      row.style.opacity='0.6';
      segsHtml='<div class="stats-bar-segment" style="width:100%;background:#e2e8f0;color:#94a3b8;font-size:10px">通信なし</div>';
    }}else{{
      row.style.opacity='1';
      top3.forEach(([cat,cd])=>{{
        const segPct=(cd.total/data.total*100);
        segsHtml+='<div class="stats-bar-segment" style="width:'+segPct+'%;background:'+getCatColor(cat)+'" title="'+cat+': '+cd.total+'">'+cat+'</div>';
      }});
      if(otherTotal>0)segsHtml+='<div class="stats-bar-segment" style="width:'+(otherTotal/data.total*100)+'%;background:#94a3b8" title="その他: '+otherTotal+'">...</div>';
    }}
    bar.innerHTML=segsHtml;
    row.querySelector('.stats-bar-count').textContent=data.total;
  }});
  // Update legend
  let legend=container.querySelector('.stats-legend');
  if(!legend){{legend=document.createElement('div');legend.className='stats-legend';container.appendChild(legend);}}
  legend.innerHTML=Object.entries(statsCatColors).map(([c,col])=>'<div class="stats-legend-item"><div class="stats-legend-color" style="background:'+col+'"></div>'+c+'</div>').join('');
}}
function showRoomDetail(room){{
  statsSelectedRoom=room;
  document.getElementById('stats-room-detail-card').style.display='block';
  document.getElementById('stats-room-detail-name').textContent=room;
  updateRoomDetail(room);
}}
function updateRoomDetail(room){{
  const container=document.getElementById('stats-room-detail-chart');
  const data=statsData.rooms[room];
  if(!data){{container.innerHTML='<p style="color:var(--text-muted)">この部屋のアクセスデータがありません</p>';return;}}
  const cats=Object.entries(data.cats).sort((a,b)=>b[1].total-a[1].total).slice(0,5);
  const maxVal=Math.max(...cats.map(c=>c[1].total));
  const catIds=new Set(cats.map(c=>c[0]));
  container.querySelectorAll('.stats-bar-row[data-cat]').forEach(el=>{{
    if(!catIds.has(el.dataset.cat))el.style.opacity='0';
  }});
  setTimeout(()=>container.querySelectorAll('.stats-bar-row[data-cat]').forEach(el=>{{
    if(!catIds.has(el.dataset.cat))el.remove();
  }}),300);
  cats.forEach(([cat,cd],i)=>{{
    const pct=(cd.total/maxVal*100);
    const svcs=Object.entries(cd.svcs).sort((a,b)=>b[1]-a[1]);
    const top3=svcs.slice(0,3);
    const otherTotal=svcs.slice(3).reduce((s,v)=>s+v[1],0);
    const color=STATS_COLORS[i%STATS_COLORS.length];
    let row=container.querySelector('[data-cat="'+cat+'"]');
    if(!row){{
      row=document.createElement('div');
      row.className='stats-bar-row';
      row.dataset.cat=cat;
      row.innerHTML='<div class="stats-bar-label" style="min-width:80px">'+cat+'</div><div class="stats-bar-wrap"><div class="stats-bar"></div></div><div class="stats-bar-count"></div>';
      container.appendChild(row);
    }}
    const bar=row.querySelector('.stats-bar');
    bar.style.width=pct+'%';
    let segsHtml='';
    top3.forEach(([svc,cnt])=>{{
      const segPct=(cnt/cd.total*100);
      segsHtml+='<div class="stats-bar-segment" style="width:'+segPct+'%;background:'+color+'" title="'+svc+': '+cnt+'">'+svc.substring(0,15)+'</div>';
    }});
    if(otherTotal>0)segsHtml+='<div class="stats-bar-segment" style="width:'+(otherTotal/cd.total*100)+'%;background:#94a3b8" title="その他: '+otherTotal+'">...</div>';
    bar.innerHTML=segsHtml;
    row.querySelector('.stats-bar-count').textContent=cd.total;
  }});
}}
function updateOverallChart(){{
  const container=document.getElementById('stats-overall-chart');
  const cats=Object.entries(statsData.categories).sort((a,b)=>b[1].total-a[1].total).slice(0,10);
  if(cats.length===0){{container.innerHTML='<p style="color:var(--text-muted)">カテゴリ分布データがありません（フィルタ条件を確認）</p>';return;}}
  const maxVal=Math.max(...cats.map(c=>c[1].total));
  const catIds=new Set(cats.map(c=>c[0]));
  container.querySelectorAll('.stats-bar-row[data-cat]').forEach(el=>{{
    if(!catIds.has(el.dataset.cat))el.style.opacity='0';
  }});
  setTimeout(()=>container.querySelectorAll('.stats-bar-row[data-cat]').forEach(el=>{{
    if(!catIds.has(el.dataset.cat))el.remove();
  }}),300);
  cats.forEach(([cat,cd],i)=>{{
    const pct=(cd.total/maxVal*100);
    const svcs=Object.entries(cd.svcs).sort((a,b)=>b[1]-a[1]);
    const top3=svcs.slice(0,3);
    const otherTotal=svcs.slice(3).reduce((s,v)=>s+v[1],0);
    const color=STATS_COLORS[i%STATS_COLORS.length];
    let row=container.querySelector('[data-cat="'+cat+'"]');
    if(!row){{
      row=document.createElement('div');
      row.className='stats-bar-row';
      row.dataset.cat=cat;
      row.innerHTML='<div class="stats-bar-label" style="min-width:100px">'+cat+'</div><div class="stats-bar-wrap"><div class="stats-bar"></div></div><div class="stats-bar-count"></div>';
      container.appendChild(row);
    }}
    const bar=row.querySelector('.stats-bar');
    bar.style.width=pct+'%';
    let segsHtml='';
    top3.forEach(([svc,cnt])=>{{
      const segPct=(cnt/cd.total*100);
      segsHtml+='<div class="stats-bar-segment" style="width:'+segPct+'%;background:'+color+'" title="'+svc+': '+cnt+'">'+svc.substring(0,15)+'</div>';
    }});
    if(otherTotal>0)segsHtml+='<div class="stats-bar-segment" style="width:'+(otherTotal/cd.total*100)+'%;background:#94a3b8" title="その他: '+otherTotal+'">...</div>';
    bar.innerHTML=segsHtml;
    row.querySelector('.stats-bar-count').textContent=cd.total;
  }});
}}

window.onload=()=>{{
  load();
  refreshStatus();
  // 除外フィルタを外側クリックで閉じる
  document.addEventListener('click',(e)=>{{
    const dd=document.getElementById('filter-dropdown');
    if(dd&&dd.open&&!dd.contains(e.target))dd.open=false;
  }});
  refreshEvents();
  loadLogs(200);
  connectSSE();
  loadCaptureConfig();
  refreshCaptureStatus();
  refreshCaptureEvents();
  refreshThreatStatus();
  refreshHardware();
  refreshWifi();
  refreshWifiConfigs();
  refreshNtp();
  refreshSync();
  refreshSpeedDial();
  showStatsLoading();
  setInterval(refreshStatus,5000);
  setInterval(refreshCaptureStatus,5000);
  setInterval(refreshCaptureEvents,3000);
  setInterval(refreshHardware,10000);
}};
</script>
</body></html>"""

    @router.get("/", response_class=HTMLResponse)
    async def index(request: Request):
        _guard(request, allowed_sources)
        return MAIN_HTML

    @router.get("/settings", response_class=HTMLResponse)
    async def settings_redirect(request: Request):
        _guard(request, allowed_sources)
        return MAIN_HTML

    @router.get("/logs", response_class=HTMLResponse)
    async def logs_redirect(request: Request):
        _guard(request, allowed_sources)
        return MAIN_HTML

    return router


__all__ = ["create_router"]
