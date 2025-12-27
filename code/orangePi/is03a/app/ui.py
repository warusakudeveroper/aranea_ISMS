"""
is03a Web UI - AraneaWebUI準拠
ESP32共通UIフレームワークと同一スタイル
"""
from fastapi import APIRouter, Request
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
.form-group input:focus,.form-group select:focus{outline:none;border-color:var(--accent);box-shadow:0 0 0 2px rgba(49,130,206,.1)}
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
@media(max-width:600px){.form-row{flex-direction:column}.tabs{gap:0}.tab{padding:8px 12px;font-size:12px}}
"""

# AraneaロゴSVG（共通）
LOGO_SVG = '<svg viewBox="0 0 211 280" fill="currentColor" style="height:32px;width:auto"><path d="M187.43,209.6c-9.57-12.63-22.63-21.14-37.43-26.9-.48-.19-.83-.68-1.25-1.03.18-.1.34-.24.53-.3,4.99-1.36,10.33-2,14.92-4.2,8.82-4.24,17.97-8.39,25.62-14.33,13.42-10.42,18.72-25.75,21.18-42.04.22-1.47-1.51-4.13-2.93-4.69-1.46-.57-4.68.21-5.34,1.39-1.64,2.93-2.31,6.41-3.27,9.7-2.52,8.58-6.23,16.58-12.64,22.92-9.65,9.53-22.03,13.38-34.93,15.91-.71.14-1.43.2-2.14.3.4-.56.72-1.21,1.22-1.66,10.74-9.61,19.39-20.71,25.21-34.02,3.39-7.74,6.01-15.8,4.83-24.06-1.43-10.01-4.28-19.85-7.01-29.62-.44-1.59-3.39-3.38-5.24-3.46-3.09-.14-4.89,2.36-4.14,5.46,1.21,5.07,3.12,9.96,4.33,15.03,2.1,8.85,2.15,17.77-1.36,26.28-4.66,11.3-12.37,20.46-21.34,28.61-.18.16-.39.29-.6.41-.07.04-.14.09-.21.13-.01-.04-.03-.08-.05-.13-.03-.08-.07-.17-.11-.26-.09-.21-.18-.43-.17-.63.7-16.98-4.61-31.8-16.06-44.34-5.64-6.17-12.28-11.01-20.72-12.31-.3-.05-.58-.04-.88-.07V0h-3.88v91.63c-6.1.46-11.46,3.54-16.2,7.54-14.48,12.21-21.36,28.05-21.42,46.93,0,1.1-.13,2.2-.2,3.29-.82-.67-1.64-1.32-2.44-2.02-.62-.54-1.22-1.11-1.8-1.69-8.13-8.05-15.3-16.89-18.81-27.91-4.08-12.79-1.03-25.16,3.39-37.39.62-1.73-.56-5.65-1.86-6.16-1.75-.69-5.94.35-6.43,1.66-2.87,7.52-5.66,15.21-7.15,23.1-2.65,14.01,1.13,27.04,8.01,39.32,5.79,10.34,13.35,19.2,22.24,26.98.27.23.34.69.5,1.04-.2.04-.43.16-.6.1-8.81-2.92-18.05-4.97-26.34-8.98-14.94-7.23-20.95-21.2-24.38-36.5-.82-3.66-2.65-6.13-6.65-5.1-3.15.81-3.9,3.44-3.17,6.5.86,3.62,1.17,7.48,2.64,10.82,3.33,7.55,6.17,15.71,11.21,22.01,5.05,6.3,11.95,11.75,19.11,15.57,8.64,4.61,18.41,7.11,27.69,10.51.48.18.99.29,1.48.43-.44.36-.86.77-1.34,1.06-9.09,5.45-18.61,10.29-27.16,16.48-8.89,6.44-15.63,15.05-18.7,25.97-2.97,10.55-2.84,21.32-1.37,32.02.24,1.73,3.05,4.21,4.85,4.37,3.25.29,4.38-2.62,4.39-5.65,0-3.93-.18-7.87-.07-11.79.26-9.53,2.11-18.64,8.68-25.99,8.54-9.56,19.02-16.44,30.82-21.37.64-.27,1.27-.55,1.93-.75.07-.02.14-.03.21-.02.11,0,.22.03.34.05.08.02.15.04.23.05.04,0,.08,0,.11.02-.13.19-.26.38-.38.57-.13.19-.26.38-.4.56-5.8,7.69-12.1,15.05-17.26,23.14-5.94,9.32-5.42,19.94-2.72,30.15,2.19,8.27,5.6,16.22,8.51,24.3,1.06,2.94,3.06,5.41,6.23,3.91,1.6-.75,3.42-4.11,2.96-5.64-2.32-7.78-6.07-15.18-7.89-23.04-1.3-5.6-1.93-12.29-.06-17.49,4.47-12.45,13.46-22.02,23.8-30.2.1-.08.21-.15.32-.22.34-.2.71-.36,1.06-.55.04.28.07.56.11.84s.08.56.16.83c.99,3.58,1.92,7.19,3.06,10.73.44,1.35,2.28,2.94,1.93,3.74-2.68,6.12-1.17,15.61,3.72,19.66.94.78,2.57.71,3.89,1.03.31-1.37.84-2.73.87-4.11.03-1.83-1.04-4-.37-5.42,1-2.1,2.94-4.97,4.8-5.22,5.27-.71,10.7-.27,16.07-.26.46,0,1.05-.05,1.36.2,3.42,2.78,4.13,6.26,2.98,10.46-.35,1.27.23,2.91.77,4.21.13.3,2.12.28,2.84-.2,5.11-3.46,7.51-11.9,5.34-17.73-.44-1.19-.94-3.09-.36-3.81,3.44-4.21,4.96-8.92,4.75-14.28,0-.23.18-.47.28-.7.22.1.5.14.66.3,5.79,5.89,11.96,11.45,17.23,17.77,5.67,6.78,10.49,14.65,8.36,23.91-2.27,9.84-6.16,19.31-8.83,29.07-.47,1.72,1.07,4.71,2.61,5.97.8.65,4.89-.74,5.37-2.01,3.76-9.9,7.59-19.84,10.23-30.06,2.42-9.39,1.48-19.11-3.95-27.39-5.14-7.83-11.4-14.94-17.16-22.36-.12-.15-.19-.34-.27-.53-.03-.06-.05-.12-.08-.18h.12c.3-.03.63-.12.84,0,7.16,3.94,14.78,7.26,21.3,12.07,5.66,4.16,10.89,9.45,14.84,15.24,5.01,7.34,5.49,16.31,5.12,25.11v7.14c0,3.24.99,6.03,4.67,6.13,3.71.1,4.9-2.86,4.89-5.9-.04-8.59.75-17.42-.95-25.71-1.48-7.23-4.78-14.7-9.24-20.58z"/></svg>'


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
<span class="version" id="hdr-ver">is03a</span>
</div>
<div class="device-banner">
<div class="device-banner-inner">
<div class="device-model">ar-is03a BLE Relay Gateway</div>
<div class="device-name" id="device-name-display"><span class="unnamed">(loading...)</span></div>
<div class="device-context">BLEセンサー受信・ローカル保存・リアルタイムモニター</div>
</div>
</div>
<div class="container">
<div class="tabs">
<div class="tab active" data-tab="status" onclick="showTab('status')">Status</div>
<div class="tab" data-tab="monitor" onclick="showTab('monitor')">Monitor</div>
<div class="tab" data-tab="network" onclick="showTab('network')">Network</div>
<div class="tab" data-tab="cloud" onclick="showTab('cloud')">Cloud</div>
<div class="tab" data-tab="tenant" onclick="showTab('tenant')">Tenant</div>
<div class="tab" data-tab="system" onclick="showTab('system')">System</div>
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

<!-- Monitor Tab (is03a固有) -->
<div id="tab-monitor" class="tab-content">
<div class="card">
<div class="card-title">Live (SSE)</div>
<div class="btn-group" style="margin-bottom:12px">
<button class="btn btn-primary btn-sm" onclick="refreshEvents()">Refresh</button>
<button class="btn btn-sm" onclick="publishSample()">Debug Event</button>
</div>
<div class="live-box" id="live">connecting...</div>
</div>
<div class="card">
<div class="card-title">Recent Events <span id="event-count" style="color:var(--text-muted);font-weight:normal">0</span></div>
<div class="btn-group" style="margin-bottom:12px">
<button class="btn btn-sm" onclick="refreshEvents(10)">10</button>
<button class="btn btn-sm" onclick="refreshEvents(50)">50</button>
<button class="btn btn-sm btn-primary" onclick="refreshEvents(100)">100</button>
<button class="btn btn-sm" onclick="refreshEvents(500)">500</button>
</div>
<div style="overflow:auto;max-height:400px">
<table>
<thead><tr><th>seenAt</th><th>src</th><th>mac</th><th>temp</th><th>hum</th><th>bat</th><th>rssi</th></tr></thead>
<tbody id="events-body"><tr><td colspan="7" style="color:var(--text-muted)">loading...</td></tr></tbody>
</table>
</div>
</div>
</div>

<!-- Network Tab -->
<div id="tab-network" class="tab-content">
<div class="card">
<div class="card-title">Network Status</div>
<p style="font-size:13px;color:var(--text-muted)">is03aはOrange Pi Zero3で動作しており、ネットワーク設定はOS側で管理されます。</p>
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
<button class="btn btn-primary" onclick="saveRelay()">Save Relay</button>
</div>
<div class="card">
<div class="card-title">MQTT Settings</div>
<div class="form-row">
<div class="form-group"><label>Host</label><input type="text" id="c-mqtt-host"></div>
<div class="form-group"><label>Port</label><input type="number" id="c-mqtt-port"></div>
</div>
<div class="form-group"><label>Base Topic</label><input type="text" id="c-mqtt-topic"></div>
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
<div class="form-group"><label>Device Name</label><input type="text" id="s-device-name" placeholder="例: 本社1F-BLEゲートウェイ"></div>
</div>
<div class="card">
<div class="card-title">Store Settings</div>
<p class="hint" style="font-size:12px;color:var(--text-muted);margin-bottom:10px">Ring/DB設定の変更は再起動が必要です</p>
<div class="form-row">
<div class="form-group"><label>Ring Size</label><input type="number" id="s-ring-size"></div>
<div class="form-group"><label>Flush Interval (sec)</label><input type="number" id="s-flush-int"></div>
</div>
<button class="btn btn-primary" onclick="saveSystem()">Save System</button>
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
  // Device info from status
  // Config form values
  document.getElementById('c-relay1').value=cfg.relay?.primary_url||'';
  document.getElementById('c-relay2').value=cfg.relay?.secondary_url||'';
  document.getElementById('c-mqtt-host').value=cfg.mqtt?.host||'';
  document.getElementById('c-mqtt-port').value=cfg.mqtt?.port||1883;
  document.getElementById('c-mqtt-topic').value=cfg.mqtt?.base_topic||'';
  document.getElementById('t-tid').value=cfg.register?.tid||'';
  document.getElementById('t-lacis').value=cfg.register?.tenant_lacisid||'';
  document.getElementById('t-cic').value=cfg.register?.tenant_cic||'';
  document.getElementById('t-email').value=cfg.register?.tenant_email||'';
  document.getElementById('t-gate').value=cfg.register?.gate_url||'';
  document.getElementById('s-device-name').value=cfg.device?.name||'';
  document.getElementById('s-ring-size').value=cfg.store?.ring_size||2000;
  document.getElementById('s-flush-int').value=cfg.store?.flush_interval_sec||5;
}}

async function refreshStatus(){{
  try{{
    const r=await fetch('/api/status');
    const s=await r.json();
    document.getElementById('d-type').textContent='ar-is03a';
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
    document.getElementById('hdr-ver').textContent='is03a v'+(s.device?.version||'');
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

function toJST(utcStr){{
  if(!utcStr)return '';
  const d=new Date(utcStr);
  return d.toLocaleString('ja-JP',{{timeZone:'Asia/Tokyo',hour12:false}});
}}

function fmt2(v){{
  if(v==null||v==='')return '';
  return Number(v).toFixed(2);
}}

let currentLimit=100;
async function refreshEvents(limit){{
  if(limit)currentLimit=limit;
  try{{
    const ev=await fetch('/api/events?limit='+currentLimit).then(r=>r.json());
    const tbody=document.getElementById('events-body');
    tbody.innerHTML='';
    ev.forEach(e=>{{
      const row=tbody.insertRow();
      row.innerHTML='<td>'+toJST(e.seenAt)+'</td><td>'+(e.src||'')+'</td><td>'+(e.mac||'')+'</td><td>'+fmt2(e.temperatureC)+'</td><td>'+fmt2(e.humidityPct)+'</td><td>'+(e.batteryPct??'')+'</td><td>'+(e.rssi??'')+'</td>';
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
      // Also append to table
      const tbody=document.getElementById('events-body');
      if(tbody.rows.length>=150)tbody.deleteRow(tbody.rows.length-1);
      const row=tbody.insertRow(0);
      row.innerHTML='<td>'+toJST(data.seenAt)+'</td><td>'+(data.src||'')+'</td><td>'+(data.mac||'')+'</td><td>'+fmt2(data.temperatureC)+'</td><td>'+fmt2(data.humidityPct)+'</td><td>'+(data.batteryPct??'')+'</td><td>'+(data.rssi??'')+'</td>';
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
  await post('/api/config',{{relay:{{primary_url:document.getElementById('c-relay1').value,secondary_url:document.getElementById('c-relay2').value}}}});
  toast('Relay settings saved');
}}

async function saveMqtt(){{
  await post('/api/config',{{mqtt:{{host:document.getElementById('c-mqtt-host').value,port:parseInt(document.getElementById('c-mqtt-port').value)||1883,base_topic:document.getElementById('c-mqtt-topic').value}}}});
  toast('MQTT settings saved');
}}

async function saveTenant(){{
  await post('/api/config',{{register:{{tid:document.getElementById('t-tid').value,tenant_lacisid:document.getElementById('t-lacis').value,tenant_cic:document.getElementById('t-cic').value,tenant_email:document.getElementById('t-email').value,gate_url:document.getElementById('t-gate').value}}}});
  toast('Tenant settings saved');
}}

async function saveSystem(){{
  await post('/api/config',{{device:{{name:document.getElementById('s-device-name').value}},store:{{ring_size:parseInt(document.getElementById('s-ring-size').value)||2000,flush_interval_sec:parseInt(document.getElementById('s-flush-int').value)||5}}}});
  toast('System settings saved (restart may be required)');
  const dn=document.getElementById('s-device-name').value;
  document.getElementById('device-name-display').innerHTML=dn||'<span class="unnamed">(未設定)</span>';
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

window.onload=()=>{{
  load();
  refreshStatus();
  refreshEvents();
  loadLogs(200);
  connectSSE();
  setInterval(refreshStatus,5000);
}};
</script>
</body></html>"""

    @router.get("/", response_class=HTMLResponse)
    async def index(request: Request):
        return MAIN_HTML

    @router.get("/settings", response_class=HTMLResponse)
    async def settings_redirect(request: Request):
        # System tabに自動遷移するためのリダイレクト用
        return MAIN_HTML

    @router.get("/logs", response_class=HTMLResponse)
    async def logs_redirect(request: Request):
        return MAIN_HTML

    return router


__all__ = ["create_router"]
