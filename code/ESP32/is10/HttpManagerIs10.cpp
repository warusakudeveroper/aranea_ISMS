/**
 * HttpManagerIs10.cpp
 *
 * IS10専用HTTPサーバー実装
 * AraneaWebUIを継承し、Router Inspector固有機能を追加
 */

#include "HttpManagerIs10.h"
#include "Is10Keys.h"
#include <SPIFFS.h>
#include <WiFi.h>

// ========================================
// 初期化
// ========================================
void HttpManagerIs10::begin(SettingManager* settings, RouterConfig* routers, int* routerCount, int port) {
  routers_ = routers;
  routerCount_ = routerCount;

  // ルーター設定読み込み
  loadRouters();

  // 基底クラス初期化（共通エンドポイント登録）
  AraneaWebUI::begin(settings, port);

  Serial.printf("[HTTP-IS10] Server started on port %d\n", port);
}

// ========================================
// is10固有エンドポイント登録
// ========================================
void HttpManagerIs10::registerTypeSpecificEndpoints() {
  server_->on("/api/is10/inspector", HTTP_POST, [this]() { handleSaveInspector(); });
  server_->on("/api/is10/router", HTTP_POST, [this]() { handleSaveRouter(); });
  server_->on("/api/is10/router/delete", HTTP_POST, [this]() { handleDeleteRouter(); });
  server_->on("/api/is10/router/clear", HTTP_POST, [this]() { handleClearRouters(); });
  server_->on("/api/is10/polling", HTTP_POST, [this]() { handlePollingControl(); });
  server_->on("/api/is10/import", HTTP_POST, [this]() { handleImportConfig(); });
}

// ========================================
// ステータス更新メソッド
// ========================================
void HttpManagerIs10::setRouterStatus(int totalRouters, int successfulPolls, unsigned long lastPollTime) {
  totalRouters_ = totalRouters;
  successfulPolls_ = successfulPolls;
  lastPollTime_ = lastPollTime;
}

void HttpManagerIs10::setMqttStatus(bool connected) {
  mqttConnected_ = connected;
}

void HttpManagerIs10::setLastStateReport(const String& time, int code) {
  lastStateReportTime_ = time;
  lastStateReportCode_ = code;
}

void HttpManagerIs10::setDroppedLogStats(uint32_t count, unsigned long lastDropMs) {
  droppedLogCount_ = count;
  lastLogDropMs_ = lastDropMs;
}

void HttpManagerIs10::setPollingEnabled(bool enabled) {
  pollingEnabled_ = enabled;
  Serial.printf("[HTTP-IS10] Polling %s\n", enabled ? "enabled" : "disabled");
}

void HttpManagerIs10::setPollingStatus(int currentRouter, bool inProgress) {
  currentRouterIndex_ = currentRouter;
  pollingInProgress_ = inProgress;
}

void HttpManagerIs10::onPollingControl(void (*callback)(bool enabled)) {
  pollingControlCallback_ = callback;
}

// ========================================
// is10固有ステータス（API用）
// ========================================
void HttpManagerIs10::getTypeSpecificStatus(JsonObject& obj) {
  // ルーターポーリング状態
  obj["totalRouters"] = totalRouters_;
  obj["successfulPolls"] = successfulPolls_;
  obj["lastPollTime"] = lastPollTime_;

  // ポーリング制御状態
  obj["pollingEnabled"] = pollingEnabled_;
  obj["pollingInProgress"] = pollingInProgress_;
  obj["currentRouterIndex"] = currentRouterIndex_;

  // MQTT状態
  obj["mqttConnected"] = mqttConnected_;

  // 最後のステートレポート
  obj["lastStateReportTime"] = lastStateReportTime_;
  obj["lastStateReportCode"] = lastStateReportCode_;

  // CelestialGlobe設定有無
  obj["celestialConfigured"] = settings_->getString(Is10Keys::kEndpoint, "").length() > 0;

  // SSH設定
  obj["sshTimeout"] = settings_->getInt(Is10Keys::kTimeout, 30000);
  obj["scanIntervalSec"] = settings_->getInt(Is10Keys::kInterval, 60);

  // ログ破棄統計（安定性のためログを捨てた回数、異常に多い場合は別問題の兆候）
  obj["droppedLogs"] = droppedLogCount_;
  obj["lastLogDropMs"] = lastLogDropMs_;
}

// ========================================
// is10固有設定（API用）
// ========================================
void HttpManagerIs10::getTypeSpecificConfig(JsonObject& obj) {
  // CelestialGlobe/SSH設定
  JsonObject inspector = obj.createNestedObject("inspector");
  inspector["endpoint"] = settings_->getString(Is10Keys::kEndpoint, "");
  inspector["celestialSecret"] = settings_->getString(Is10Keys::kSecret, "");  // ブラインドなし
  inspector["scanIntervalSec"] = settings_->getInt(Is10Keys::kInterval, 60);
  inspector["reportClients"] = settings_->getBool(Is10Keys::kReportClnt, true);
  inspector["sshTimeout"] = settings_->getInt(Is10Keys::kTimeout, 30000);
  inspector["retryCount"] = settings_->getInt(Is10Keys::kRetry, 2);
  inspector["routerInterval"] = settings_->getInt(Is10Keys::kRtrIntv, 30000);

  // ルーター設定
  JsonArray routersArr = obj.createNestedArray("routers");
  for (int i = 0; i < *routerCount_; i++) {
    JsonObject r = routersArr.createNestedObject();
    r["rid"] = routers_[i].rid;
    r["ipAddr"] = routers_[i].ipAddr;
    r["sshPort"] = routers_[i].sshPort;
    r["username"] = routers_[i].username;
    r["password"] = routers_[i].password;
    r["publicKey"] = routers_[i].publicKey;
    r["enabled"] = routers_[i].enabled;
    r["osType"] = (routers_[i].osType == RouterOsType::ASUSWRT) ? 1 : 0;
  }
}

// ========================================
// is10固有タブ
// ========================================
String HttpManagerIs10::generateTypeSpecificTabs() {
  return R"TABS(
<div class="tab" data-tab="inspector" onclick="showTab('inspector')">Inspector</div>
<div class="tab" data-tab="routers" onclick="showTab('routers')">Routers</div>
)TABS";
}

// ========================================
// is10固有タブコンテンツ
// ========================================
String HttpManagerIs10::generateTypeSpecificTabContents() {
  return R"HTML(
<!-- Inspector Tab -->
<div id="tab-inspector" class="tab-content">
<div class="card">
<div class="card-title">Polling Status</div>
<div class="status-grid">
<div class="status-item"><div class="label">Status</div><div class="value" id="is-polling-status">-</div></div>
<div class="status-item"><div class="label">Progress</div><div class="value" id="is-polling-progress">-</div></div>
<div class="status-item"><div class="label">Result</div><div class="value" id="is-routers">-</div></div>
<div class="status-item"><div class="label">Last Poll</div><div class="value" id="is-poll">-</div></div>
</div>
<div class="btn-group" style="margin-top:12px">
<button class="btn btn-primary" id="btn-polling-toggle" onclick="togglePolling()">Stop Polling</button>
<button class="btn" onclick="refreshStatus()">Refresh</button>
</div>
</div>
<div class="card">
<div class="card-title">Connection Status</div>
<div class="status-grid">
<div class="status-item"><div class="label">MQTT</div><div class="value" id="is-mqtt">-</div></div>
<div class="status-item"><div class="label">State Report</div><div class="value" id="is-report">-</div></div>
</div>
</div>
<div class="card">
<div class="card-title">CelestialGlobe SSOT</div>
<div class="form-group"><label>Endpoint URL</label><input type="text" id="i-endpoint" placeholder="https://...universalIngest"></div>
<div class="form-group"><label>X-Celestial-Secret</label><input type="password" id="i-secret" placeholder="(unchanged if empty)"></div>
<div class="form-row">
<div class="form-group"><label>Scan Interval (sec)</label><input type="number" id="i-scan" min="60" max="86400"></div>
<div class="form-group"><label>Report Clients</label><select id="i-clients"><option value="true">Enabled</option><option value="false">Disabled</option></select></div>
</div>
</div>
<div class="card">
<div class="card-title">SSH Settings</div>
<div class="form-row">
<div class="form-group"><label>SSH Timeout (ms)</label><input type="number" id="i-timeout"></div>
<div class="form-group"><label>Retry Count</label><input type="number" id="i-retry"></div>
<div class="form-group"><label>Router Interval (ms)</label><input type="number" id="i-interval"></div>
</div>
<button class="btn btn-primary" onclick="saveInspector()">Save Inspector Settings</button>
</div>
</div>

<!-- Routers Tab -->
<div id="tab-routers" class="tab-content">
<div class="card">
<div class="card-title">Router Configuration</div>
<div class="btn-group">
<button class="btn btn-primary" onclick="addRouter()">+ Add Router</button>
<button class="btn btn-danger" onclick="clearAllRouters()">Clear All</button>
</div>
<div id="router-list" style="margin-top:12px"></div>
</div>
</div>

<!-- Router Modal -->
<div id="router-modal" style="display:none;position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.5);z-index:1000">
<div style="background:white;max-width:500px;margin:50px auto;padding:20px;border-radius:8px;max-height:80vh;overflow-y:auto">
<h3 style="margin-bottom:16px">Router Configuration</h3>
<input type="hidden" id="r-index">
<div class="form-row">
<div class="form-group"><label>RID</label><input type="text" id="r-rid" placeholder="e.g. 306"></div>
<div class="form-group"><label>SSH Port</label><input type="number" id="r-port" value="22"></div>
</div>
<div class="form-row">
<div class="form-group"><label>IP Address</label><input type="text" id="r-ip" placeholder="192.168.x.x"></div>
<div class="form-group"><label>OS Type</label><select id="r-ostype"><option value="0">OpenWrt</option><option value="1">ASUSWRT</option></select></div>
</div>
<div class="form-row">
<div class="form-group"><label>Username</label><input type="text" id="r-user"></div>
<div class="form-group"><label>Password</label><input type="text" id="r-pass"></div>
</div>
<div class="form-group"><label>Public Key (optional)</label><textarea id="r-key" rows="3" style="font-size:12px"></textarea></div>
<div class="form-group"><label><input type="checkbox" id="r-enabled" checked> Enabled</label></div>
<div class="btn-group">
<button class="btn btn-primary" onclick="saveRouter()">Save</button>
<button class="btn" onclick="closeRouterModal()">Cancel</button>
</div>
</div>
</div>
)HTML";
}

// ========================================
// is10固有JavaScript
// ========================================
String HttpManagerIs10::generateTypeSpecificJS() {
  return R"JS(
<script>
// IS10固有のレンダリング
function renderTypeSpecific(){
  const ts=cfg.typeSpecific||{};
  const ins=ts.inspector||{};
  document.getElementById('i-endpoint').value=ins.endpoint||'';
  document.getElementById('i-secret').value=ins.celestialSecret||'';
  document.getElementById('i-scan').value=ins.scanIntervalSec||60;
  document.getElementById('i-clients').value=ins.reportClients?'true':'false';
  document.getElementById('i-timeout').value=ins.sshTimeout||30000;
  document.getElementById('i-retry').value=ins.retryCount||2;
  document.getElementById('i-interval').value=ins.routerInterval||30000;
  renderRouterList();
}
function renderRouterList(){
  const list=document.getElementById('router-list');
  list.innerHTML='';
  const routers=cfg.typeSpecific?.routers||[];
  const osTypes=['OpenWrt','ASUSWRT'];
  routers.forEach((r,i)=>{
    const osLabel=osTypes[r.osType||0];
    list.innerHTML+=`<div style="background:var(--bg);padding:12px;border-radius:6px;margin-bottom:8px;display:flex;justify-content:space-between;align-items:center">
      <div><div style="font-weight:500">RID: ${r.rid} ${r.enabled?'':'(disabled)'} <span style="color:var(--accent)">[${osLabel}]</span></div>
      <div style="color:var(--text-muted);font-size:13px">${r.ipAddr}:${r.sshPort} - ${r.username}</div></div>
      <div><button class="btn btn-primary btn-sm" onclick="editRouter(${i})">Edit</button>
      <button class="btn btn-danger btn-sm" onclick="deleteRouter(${i})">Del</button></div>
    </div>`;
  });
}
let pollingEnabled=true;
function refreshTypeSpecificStatus(s){
  const ts=s.typeSpecific||{};
  const total=ts.totalRouters||0;
  const success=ts.successfulPolls||0;
  pollingEnabled=ts.pollingEnabled!==false;
  const inProgress=ts.pollingInProgress||false;
  const currentIdx=ts.currentRouterIndex||0;
  // Polling status
  const statusEl=document.getElementById('is-polling-status');
  const progressEl=document.getElementById('is-polling-progress');
  const btn=document.getElementById('btn-polling-toggle');
  if(!pollingEnabled){
    statusEl.textContent='Stopped';statusEl.className='value warn';
    progressEl.textContent='-';
    btn.textContent='Start Polling';btn.className='btn btn-primary';
  }else if(inProgress){
    statusEl.textContent='Running';statusEl.className='value good';
    progressEl.textContent=`Router ${currentIdx+1}/${total}`;
    btn.textContent='Stop Polling';btn.className='btn btn-danger';
  }else{
    statusEl.textContent='Idle';statusEl.className='value';
    progressEl.textContent='Waiting...';
    btn.textContent='Stop Polling';btn.className='btn btn-danger';
  }
  // Results
  document.getElementById('is-routers').textContent=`${success}/${total}`;
  document.getElementById('is-routers').className='value '+(success===total&&total>0?'good':'warn');
  const lastPoll=ts.lastPollTime||0;
  document.getElementById('is-poll').textContent=lastPoll>0?new Date(lastPoll).toLocaleTimeString():'-';
  document.getElementById('is-mqtt').textContent=ts.mqttConnected?'Connected':'Disconnected';
  document.getElementById('is-mqtt').className='value '+(ts.mqttConnected?'good':'warn');
  const code=ts.lastStateReportCode||0;
  document.getElementById('is-report').textContent=code>0?`HTTP ${code}`:'-';
  document.getElementById('is-report').className='value '+(code===200?'good':code>0?'bad':'');
}
async function togglePolling(){
  const newState=!pollingEnabled;
  await post('/api/is10/polling',{enabled:newState});
  toast(newState?'Polling started':'Polling stopped');
  refreshStatus();
}
async function clearAllRouters(){
  if(!confirm('Clear all router configurations?'))return;
  await post('/api/is10/router/clear',{});
  toast('All routers cleared');
  load();
}
async function saveInspector(){
  await post('/api/is10/inspector',{
    endpoint:document.getElementById('i-endpoint').value,
    celestialSecret:document.getElementById('i-secret').value,
    scanIntervalSec:parseInt(document.getElementById('i-scan').value),
    reportClients:document.getElementById('i-clients').value==='true',
    sshTimeout:parseInt(document.getElementById('i-timeout').value),
    retryCount:parseInt(document.getElementById('i-retry').value),
    routerInterval:parseInt(document.getElementById('i-interval').value)
  });
  toast('Inspector settings saved');
}
function addRouter(){
  document.getElementById('r-index').value=-1;
  document.getElementById('r-rid').value='';
  document.getElementById('r-ip').value='';
  document.getElementById('r-port').value='22';
  document.getElementById('r-user').value='';
  document.getElementById('r-pass').value='';
  document.getElementById('r-key').value='';
  document.getElementById('r-ostype').value='0';
  document.getElementById('r-enabled').checked=true;
  document.getElementById('router-modal').style.display='block';
}
function editRouter(i){
  const r=cfg.typeSpecific?.routers[i];
  if(!r)return;
  document.getElementById('r-index').value=i;
  document.getElementById('r-rid').value=r.rid;
  document.getElementById('r-ip').value=r.ipAddr;
  document.getElementById('r-port').value=r.sshPort;
  document.getElementById('r-user').value=r.username;
  document.getElementById('r-pass').value=r.password||'';
  document.getElementById('r-key').value=r.publicKey||'';
  document.getElementById('r-ostype').value=r.osType||0;
  document.getElementById('r-enabled').checked=r.enabled;
  document.getElementById('router-modal').style.display='block';
}
function closeRouterModal(){document.getElementById('router-modal').style.display='none'}
async function saveRouter(){
  await post('/api/is10/router',{
    index:parseInt(document.getElementById('r-index').value),
    rid:document.getElementById('r-rid').value,
    ipAddr:document.getElementById('r-ip').value,
    sshPort:parseInt(document.getElementById('r-port').value),
    username:document.getElementById('r-user').value,
    password:document.getElementById('r-pass').value,
    publicKey:document.getElementById('r-key').value,
    osType:parseInt(document.getElementById('r-ostype').value),
    enabled:document.getElementById('r-enabled').checked
  });
  closeRouterModal();
  toast('Router saved');
  load();
}
async function deleteRouter(i){
  if(!confirm('Delete this router?'))return;
  await post('/api/is10/router/delete',{index:i});
  toast('Router deleted');
  load();
}
</script>
)JS";
}

// ========================================
// is10固有ハンドラ
// ========================================
void HttpManagerIs10::handleSaveInspector() {
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No body\"}");
    return;
  }

  StaticJsonDocument<512> doc;
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid JSON\"}");
    return;
  }

  if (doc.containsKey("endpoint")) settings_->setString(Is10Keys::kEndpoint, doc["endpoint"]);
  if (doc.containsKey("celestialSecret")) {
    String secret = doc["celestialSecret"].as<String>();
    if (secret.length() > 0 && secret != "********") {
      settings_->setString(Is10Keys::kSecret, secret);
    }
  }
  if (doc.containsKey("scanIntervalSec")) {
    int sec = doc["scanIntervalSec"];
    if (sec >= 60 && sec <= 86400) {
      settings_->setInt(Is10Keys::kInterval, sec);
    }
  }
  if (doc.containsKey("reportClients")) settings_->setBool(Is10Keys::kReportClnt, doc["reportClients"].as<bool>());
  if (doc.containsKey("sshTimeout")) settings_->setInt(Is10Keys::kTimeout, doc["sshTimeout"].as<int>());
  if (doc.containsKey("retryCount")) settings_->setInt(Is10Keys::kRetry, doc["retryCount"].as<int>());
  if (doc.containsKey("routerInterval")) settings_->setInt(Is10Keys::kRtrIntv, doc["routerInterval"].as<int>());

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs10::handleSaveRouter() {
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No body\"}");
    return;
  }

  yield();
  DynamicJsonDocument doc(4096);
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid JSON\"}");
    return;
  }

  int index = doc["index"] | -1;
  String rid = doc["rid"] | "";
  String ipAddr = doc["ipAddr"] | "";

  // 必須フィールドのバリデーション
  if (rid.length() == 0) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"rid is required\"}");
    return;
  }
  if (ipAddr.length() == 0) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"ipAddr is required\"}");
    return;
  }

  // IPv4形式バリデーション
  if (!AraneaWebUI::validateIPv4(ipAddr)) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid ipAddr: must be valid IPv4 (e.g. 192.168.1.1)\"}");
    return;
  }

  // ridが指定されている場合、既存ルーターを検索
  if (rid.length() > 0 && index < 0) {
    for (int i = 0; i < *routerCount_; i++) {
      if (routers_[i].rid == rid) {
        index = i;
        break;
      }
    }
  }

  bool isNew = (index < 0 || index >= *routerCount_);

  if (isNew) {
    if (*routerCount_ >= MAX_ROUTERS) {
      server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Max routers reached\"}");
      return;
    }
    index = (*routerCount_)++;
  }

  routers_[index].rid = doc["rid"] | "";
  routers_[index].ipAddr = doc["ipAddr"] | "";
  if (doc.containsKey("publicKey") && doc["publicKey"].as<String>().length() > 0) {
    routers_[index].publicKey = doc["publicKey"].as<String>();
  }
  routers_[index].sshPort = doc["sshPort"].as<int>();
  routers_[index].username = doc["username"].as<String>();
  if (doc.containsKey("password") && doc["password"].as<String>().length() > 0) {
    routers_[index].password = doc["password"].as<String>();
  }
  routers_[index].enabled = doc["enabled"] | true;
  int osTypeInt = doc["osType"] | 0;
  routers_[index].osType = (osTypeInt == 1) ? RouterOsType::ASUSWRT : RouterOsType::OPENWRT;

  saveRouters();
  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs10::handleDeleteRouter() {
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No body\"}");
    return;
  }

  StaticJsonDocument<128> doc;
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid JSON\"}");
    return;
  }

  int index = doc["index"] | -1;
  if (index < 0 || index >= *routerCount_) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid index\"}");
    return;
  }

  for (int i = index; i < *routerCount_ - 1; i++) {
    routers_[i] = routers_[i + 1];
  }
  (*routerCount_)--;

  saveRouters();
  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

// ========================================
// ルーター設定読み書き
// ========================================
void HttpManagerIs10::loadRouters() {
  if (!routers_ || !routerCount_) {
    Serial.println("[HTTP-IS10] Error: routers array not set");
    return;
  }

  *routerCount_ = 0;

  if (!SPIFFS.exists("/routers.json")) {
    Serial.println("[HTTP-IS10] No routers.json found");
    return;
  }

  yield();
  File file = SPIFFS.open("/routers.json", "r");
  if (!file) return;

  String json = file.readString();
  file.close();
  yield();

  DynamicJsonDocument doc(8192);
  DeserializationError err = deserializeJson(doc, json);
  yield();
  if (err) {
    Serial.printf("[HTTP-IS10] JSON parse error: %s\n", err.c_str());
    return;
  }

  JsonArray arr = doc.as<JsonArray>();
  for (JsonObject obj : arr) {
    if (*routerCount_ >= MAX_ROUTERS) break;
    routers_[*routerCount_].rid = obj["rid"].as<String>();
    routers_[*routerCount_].ipAddr = obj["ipAddr"].as<String>();
    routers_[*routerCount_].publicKey = obj["publicKey"].as<String>();
    routers_[*routerCount_].sshPort = obj["sshPort"].as<int>();
    routers_[*routerCount_].username = obj["username"].as<String>();
    routers_[*routerCount_].password = obj["password"].as<String>();
    routers_[*routerCount_].enabled = obj["enabled"] | true;
    int osTypeInt = obj["osType"].as<int>();
    routers_[*routerCount_].osType = (osTypeInt == 1) ? RouterOsType::ASUSWRT : RouterOsType::OPENWRT;
    if (routers_[*routerCount_].ipAddr.length() > 0) {
      (*routerCount_)++;
    }
  }

  Serial.printf("[HTTP-IS10] Loaded %d routers\n", *routerCount_);
}

void HttpManagerIs10::saveRouters() {
  if (!routers_ || !routerCount_) return;

  yield();
  DynamicJsonDocument doc(8192);
  JsonArray arr = doc.to<JsonArray>();

  for (int i = 0; i < *routerCount_; i++) {
    JsonObject obj = arr.createNestedObject();
    obj["rid"] = routers_[i].rid;
    obj["ipAddr"] = routers_[i].ipAddr;
    obj["publicKey"] = routers_[i].publicKey;
    obj["sshPort"] = routers_[i].sshPort;
    obj["username"] = routers_[i].username;
    obj["password"] = routers_[i].password;
    obj["enabled"] = routers_[i].enabled;
    obj["osType"] = (routers_[i].osType == RouterOsType::ASUSWRT) ? 1 : 0;
  }

  yield();
  File file = SPIFFS.open("/routers.json", "w");
  if (file) {
    serializeJson(doc, file);
    file.close();
    yield();
    Serial.printf("[HTTP-IS10] Saved %d routers\n", *routerCount_);
  }
}

// ========================================
// Getter
// ========================================
Is10GlobalSetting HttpManagerIs10::getGlobalSetting() {
  Is10GlobalSetting gs;
  gs.endpoint = settings_->getString(Is10Keys::kEndpoint, "");
  gs.celestialSecret = settings_->getString(Is10Keys::kSecret, "");
  gs.scanIntervalSec = settings_->getInt(Is10Keys::kInterval, 60);
  gs.reportClients = settings_->getBool(Is10Keys::kReportClnt, true);
  gs.timeout = settings_->getInt(Is10Keys::kTimeout, 30000);
  gs.retryCount = settings_->getInt(Is10Keys::kRetry, 2);
  gs.interval = settings_->getInt(Is10Keys::kRtrIntv, 30000);
  return gs;
}

RouterConfig HttpManagerIs10::getRouter(int index) {
  if (routers_ && routerCount_ && index >= 0 && index < *routerCount_) {
    return routers_[index];
  }
  return RouterConfig();
}

int HttpManagerIs10::getRouterCount() {
  return routerCount_ ? *routerCount_ : 0;
}

// ========================================
// ルーター設定永続化（MQTT config適用時用）
// ========================================
void HttpManagerIs10::persistRouters() {
  saveRouters();
}

// ========================================
// ルーター設定クリア
// ========================================
void HttpManagerIs10::handleClearRouters() {
  if (!checkAuth()) { requestAuth(); return; }

  *routerCount_ = 0;
  SPIFFS.remove("/routers.json");
  totalRouters_ = 0;
  successfulPolls_ = 0;

  Serial.println("[HTTP-IS10] All routers cleared");
  server_->send(200, "application/json", "{\"ok\":true,\"message\":\"All routers cleared\"}");
}

// ========================================
// ポーリング制御
// ========================================
void HttpManagerIs10::handlePollingControl() {
  if (!checkAuth()) { requestAuth(); return; }
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No body\"}");
    return;
  }

  StaticJsonDocument<256> doc;
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid JSON\"}");
    return;
  }

  if (doc.containsKey("enabled")) {
    bool enabled = doc["enabled"];
    pollingEnabled_ = enabled;
    Serial.printf("[HTTP-IS10] Polling %s via API\n", enabled ? "started" : "stopped");

    // コールバック呼び出し（メインループでの処理用）
    if (pollingControlCallback_) {
      pollingControlCallback_(enabled);
    }

    String response = "{\"ok\":true,\"pollingEnabled\":";
    response += enabled ? "true" : "false";
    response += "}";
    server_->send(200, "application/json", response);
  } else {
    // ステータス取得のみ
    String response = "{\"ok\":true,\"pollingEnabled\":";
    response += pollingEnabled_ ? "true" : "false";
    response += ",\"currentRouterIndex\":";
    response += String(currentRouterIndex_);
    response += ",\"pollingInProgress\":";
    response += pollingInProgress_ ? "true" : "false";
    response += "}";
    server_->send(200, "application/json", response);
  }
}

// ========================================
// 設定一括インポート
// ========================================
void HttpManagerIs10::handleImportConfig() {
  if (!checkAuth()) { requestAuth(); return; }
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No body\"}");
    return;
  }

  yield();
  DynamicJsonDocument doc(16384);
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid JSON\"}");
    return;
  }

  int importedCount = 0;

  // Inspector設定
  if (doc.containsKey("inspector")) {
    JsonObject ins = doc["inspector"];
    if (ins.containsKey("endpoint")) settings_->setString(Is10Keys::kEndpoint, ins["endpoint"]);
    if (ins.containsKey("celestialSecret")) {
      String secret = ins["celestialSecret"].as<String>();
      if (secret.length() > 0 && secret != "********") {
        settings_->setString(Is10Keys::kSecret, secret);
      }
    }
    if (ins.containsKey("scanIntervalSec")) settings_->setInt(Is10Keys::kInterval, ins["scanIntervalSec"].as<int>());
    if (ins.containsKey("reportClients")) settings_->setBool(Is10Keys::kReportClnt, ins["reportClients"].as<bool>());
    if (ins.containsKey("sshTimeout")) settings_->setInt(Is10Keys::kTimeout, ins["sshTimeout"].as<int>());
    if (ins.containsKey("retryCount")) settings_->setInt(Is10Keys::kRetry, ins["retryCount"].as<int>());
    if (ins.containsKey("routerInterval")) settings_->setInt(Is10Keys::kRtrIntv, ins["routerInterval"].as<int>());
    importedCount++;
  }

  // ルーター設定
  if (doc.containsKey("routers") && doc["routers"].is<JsonArray>()) {
    *routerCount_ = 0;
    JsonArray arr = doc["routers"].as<JsonArray>();
    for (JsonObject obj : arr) {
      if (*routerCount_ >= MAX_ROUTERS) break;
      routers_[*routerCount_].rid = obj["rid"].as<String>();
      routers_[*routerCount_].ipAddr = obj["ipAddr"].as<String>();
      routers_[*routerCount_].publicKey = obj["publicKey"].as<String>();
      routers_[*routerCount_].sshPort = obj["sshPort"].as<int>();
      routers_[*routerCount_].username = obj["username"].as<String>();
      routers_[*routerCount_].password = obj["password"].as<String>();
      routers_[*routerCount_].enabled = obj["enabled"] | true;
      int osTypeInt = obj["osType"].as<int>();
      routers_[*routerCount_].osType = (osTypeInt == 1) ? RouterOsType::ASUSWRT : RouterOsType::OPENWRT;
      if (routers_[*routerCount_].ipAddr.length() > 0) {
        (*routerCount_)++;
      }
    }
    saveRouters();
    totalRouters_ = *routerCount_;
    successfulPolls_ = 0;
    importedCount++;
  }

  if (settingsChangedCallback_) settingsChangedCallback_();

  String response = "{\"ok\":true,\"importedSections\":";
  response += String(importedCount);
  response += ",\"routerCount\":";
  response += String(*routerCount_);
  response += "}";
  server_->send(200, "application/json", response);
  Serial.printf("[HTTP-IS10] Config imported: %d sections, %d routers\n", importedCount, *routerCount_);
}
