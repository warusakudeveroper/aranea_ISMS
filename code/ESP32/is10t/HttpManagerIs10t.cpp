/**
 * HttpManagerIs10t.cpp
 *
 * IS10T (Tapo Monitor) HTTPサーバー実装
 */

#include "HttpManagerIs10t.h"
#include "TapoPollerIs10t.h"
#include <ArduinoJson.h>

void HttpManagerIs10t::begin(SettingManager* settings, TapoPollerIs10t* poller, int port) {
    poller_ = poller;
    AraneaWebUI::begin(settings, port);
}

void HttpManagerIs10t::setPollingStatus(bool polling, uint8_t currentCamera) {
    pollingInProgress_ = polling;
    currentCameraIndex_ = currentCamera;
}

void HttpManagerIs10t::getTypeSpecificStatus(JsonObject& obj) {
    obj["polling"] = pollingInProgress_;
    obj["currentCamera"] = currentCameraIndex_;

    if (poller_) {
        const TapoPollSummary& summary = poller_->getSummary();
        obj["totalCameras"] = summary.total;
        obj["onlineCameras"] = summary.online;
        obj["offlineCameras"] = summary.offline;

        unsigned long elapsed = (millis() - summary.lastPollMs) / 1000;
        obj["lastPollAgo"] = elapsed;
    }
}

void HttpManagerIs10t::getTypeSpecificConfig(JsonObject& obj) {
    // Tapo固有設定は現在なし（カメラ設定は別API）
}

String HttpManagerIs10t::generateTypeSpecificTabs() {
    return R"TABS(
<div class="tab" data-tab="cameras" onclick="showTab('cameras')">Cameras</div>
)TABS";
}

String HttpManagerIs10t::generateTypeSpecificTabContents() {
    return R"HTML(
<!-- Cameras Tab -->
<div id="tab-cameras" class="tab-content">
<div class="card">
<div class="card-title">Camera Status</div>
<div class="status-grid">
<div class="status-item"><div class="label">Total</div><div class="value" id="cam-total">-</div></div>
<div class="status-item"><div class="label">Online</div><div class="value" id="cam-online">-</div></div>
<div class="status-item"><div class="label">Offline</div><div class="value" id="cam-offline">-</div></div>
<div class="status-item"><div class="label">Polling</div><div class="value" id="cam-polling">-</div></div>
</div>
</div>
<div class="card">
<div class="card-title">Camera List</div>
<div id="camera-list"></div>
</div>
<div class="card">
<div class="card-title">Add Camera</div>
<div class="form-group"><label>Host (IP Address)</label><input type="text" id="cam-host" placeholder="192.168.1.100"></div>
<div class="form-group"><label>ONVIF Port</label><input type="number" id="cam-port" value="2020"></div>
<div class="form-group"><label>Username</label><input type="text" id="cam-user"></div>
<div class="form-group"><label>Password</label><input type="password" id="cam-pass"></div>
<button class="btn" onclick="addCamera()">Add Camera</button>
</div>
</div>
)HTML";
}

String HttpManagerIs10t::generateTypeSpecificJS() {
    return R"JS(
function loadCameras() {
  fetch('/api/cameras').then(r=>r.json()).then(data=>{
    let online=0, offline=0;
    let html = '<table style="width:100%;border-collapse:collapse">';
    html += '<tr style="background:var(--bg)"><th style="padding:8px;text-align:left">#</th><th>Host</th><th>Model</th><th>Name</th><th>MAC</th><th>Status</th><th>Actions</th></tr>';
    data.cameras.forEach((c,i)=>{
      if(c.online) online++; else offline++;
      let st = c.online ? '<span style="color:#4caf50">Online</span>' : '<span style="color:#f44336">Offline</span>';
      html += '<tr style="border-bottom:1px solid var(--border)">';
      html += '<td style="padding:8px">'+(i+1)+'</td>';
      html += '<td>'+c.host+'</td>';
      html += '<td>'+(c.model||'-')+'</td>';
      html += '<td>'+(c.deviceName||'-')+'</td>';
      html += '<td>'+(c.mac||'-')+'</td>';
      html += '<td>'+st+'</td>';
      html += '<td><button class="btn" style="padding:4px 8px" onclick="deleteCamera('+i+')">Del</button></td>';
      html += '</tr>';
    });
    html += '</table>';
    if(data.cameras.length==0) html='<p>No cameras configured</p>';
    document.getElementById('camera-list').innerHTML = html;
    document.getElementById('cam-total').textContent = data.cameras.length;
    document.getElementById('cam-online').textContent = online;
    document.getElementById('cam-offline').textContent = offline;
  });
  // Update polling status from /api/status (typeSpecific配下)
  fetch('/api/status').then(r=>r.json()).then(d=>{
    let ts = d.typeSpecific || {};
    let st = ts.polling ? 'Polling ('+(ts.currentCamera+1)+'/'+ts.totalCameras+')' : 'Idle';
    if(ts.lastPollAgo !== undefined) st += ' ('+ts.lastPollAgo+'s ago)';
    document.getElementById('cam-polling').textContent = st;
  });
}
function addCamera() {
  let data = {
    host: document.getElementById('cam-host').value,
    port: parseInt(document.getElementById('cam-port').value)||2020,
    username: document.getElementById('cam-user').value,
    password: document.getElementById('cam-pass').value
  };
  if(!data.host){alert('Host is required');return;}
  fetch('/api/cameras', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(data)})
    .then(r=>r.json()).then(res=>{
      if(res.success){loadCameras();document.getElementById('cam-host').value='';document.getElementById('cam-user').value='';document.getElementById('cam-pass').value='';}
      else alert(res.error||'Failed');
    });
}
function deleteCamera(idx) {
  if(!confirm('Delete camera '+(idx+1)+'?')) return;
  fetch('/api/cameras/'+idx, {method:'DELETE'}).then(r=>r.json()).then(()=>loadCameras());
}
setInterval(loadCameras, 5000);
loadCameras();
)JS";
}

void HttpManagerIs10t::registerTypeSpecificEndpoints() {
    // GET /api/cameras - カメラ一覧取得
    server_->on("/api/cameras", HTTP_GET, [this]() {
        handleGetCameras();
    });

    // POST /api/cameras - カメラ追加
    server_->on("/api/cameras", HTTP_POST, [this]() {
        handleAddCamera();
    });

    // DELETE /api/cameras/{id} - カメラ削除
    server_->on("/api/cameras/0", HTTP_DELETE, [this]() { handleDeleteCamera(); });
    server_->on("/api/cameras/1", HTTP_DELETE, [this]() { handleDeleteCamera(); });
    server_->on("/api/cameras/2", HTTP_DELETE, [this]() { handleDeleteCamera(); });
    server_->on("/api/cameras/3", HTTP_DELETE, [this]() { handleDeleteCamera(); });
    server_->on("/api/cameras/4", HTTP_DELETE, [this]() { handleDeleteCamera(); });
    server_->on("/api/cameras/5", HTTP_DELETE, [this]() { handleDeleteCamera(); });
    server_->on("/api/cameras/6", HTTP_DELETE, [this]() { handleDeleteCamera(); });
    server_->on("/api/cameras/7", HTTP_DELETE, [this]() { handleDeleteCamera(); });

    // POST /api/poll - 手動ポーリング開始
    server_->on("/api/poll", HTTP_POST, [this]() {
        handleStartPoll();
    });
}

void HttpManagerIs10t::handleGetCameras() {
    DynamicJsonDocument doc(4096);
    JsonArray cameras = doc.createNestedArray("cameras");

    if (poller_) {
        for (uint8_t i = 0; i < poller_->getCameraCount(); i++) {
            const TapoConfig& config = poller_->getConfig(i);
            const TapoStatus& status = poller_->getStatus(i);

            JsonObject cam = cameras.createNestedObject();
            cam["host"] = config.host;
            cam["port"] = config.onvifPort;
            cam["username"] = config.username;
            cam["enabled"] = config.enabled;
            cam["online"] = status.online;
            cam["model"] = status.model;
            cam["mac"] = status.getMacString();
            cam["deviceName"] = status.deviceName;
            cam["firmware"] = status.firmware;
        }
    }

    String response;
    serializeJson(doc, response);
    server_->send(200, "application/json", response);
}

void HttpManagerIs10t::handleAddCamera() {
    if (!poller_) {
        server_->send(500, "application/json", "{\"error\":\"Poller not initialized\"}");
        return;
    }

    DynamicJsonDocument doc(512);
    deserializeJson(doc, server_->arg("plain"));

    TapoConfig config;
    strncpy(config.host, doc["host"] | "", sizeof(config.host) - 1);
    config.onvifPort = doc["port"] | TAPO_ONVIF_PORT;
    strncpy(config.username, doc["username"] | "", sizeof(config.username) - 1);
    strncpy(config.password, doc["password"] | "", sizeof(config.password) - 1);
    config.enabled = true;

    if (strlen(config.host) == 0) {
        server_->send(400, "application/json", "{\"error\":\"Host is required\"}");
        return;
    }

    if (poller_->addCamera(config)) {
        server_->send(200, "application/json", "{\"success\":true}");
    } else {
        server_->send(400, "application/json", "{\"error\":\"Max cameras reached\"}");
    }
}

void HttpManagerIs10t::handleDeleteCamera() {
    if (!poller_) {
        server_->send(500, "application/json", "{\"error\":\"Poller not initialized\"}");
        return;
    }

    String uri = server_->uri();
    int idx = uri.substring(uri.lastIndexOf('/') + 1).toInt();

    if (poller_->removeCamera(idx)) {
        server_->send(200, "application/json", "{\"success\":true}");
    } else {
        server_->send(400, "application/json", "{\"error\":\"Invalid index\"}");
    }
}

void HttpManagerIs10t::handleStartPoll() {
    if (poller_) {
        poller_->startPoll();
        server_->send(200, "application/json", "{\"success\":true}");
    } else {
        server_->send(500, "application/json", "{\"error\":\"Poller not initialized\"}");
    }
}
