/**
 * HttpManagerIs10.cpp
 *
 * IS10専用HTTPサーバー実装
 */

#include "HttpManagerIs10.h"
#include <SPIFFS.h>
#include <WiFi.h>

void HttpManagerIs10::begin(SettingManager* settings, int port) {
  settings_ = settings;
  server_ = new WebServer(port);

  // ルーター設定読み込み
  loadRouters();

  // エンドポイント設定
  server_->on("/", HTTP_GET, [this]() { handleRoot(); });
  server_->on("/api/config", HTTP_GET, [this]() { handleGetConfig(); });
  server_->on("/api/global", HTTP_POST, [this]() { handleSaveGlobal(); });
  server_->on("/api/lacisgen", HTTP_POST, [this]() { handleSaveLacisGen(); });
  server_->on("/api/router", HTTP_POST, [this]() { handleSaveRouter(); });
  server_->on("/api/router/delete", HTTP_POST, [this]() { handleDeleteRouter(); });
  server_->on("/api/tenant", HTTP_POST, [this]() { handleSaveTenant(); });
  server_->on("/api/wifi", HTTP_POST, [this]() { handleSaveWifi(); });
  server_->on("/api/reboot", HTTP_POST, [this]() { handleReboot(); });
  server_->on("/api/factory-reset", HTTP_POST, [this]() { handleFactoryReset(); });
  server_->onNotFound([this]() { handleNotFound(); });

  server_->begin();
  Serial.printf("[HTTP-IS10] Server started on port %d\n", port);
}

void HttpManagerIs10::handle() {
  if (server_) {
    server_->handleClient();
  }
}

void HttpManagerIs10::setDeviceInfo(const String& type, const String& lacisId,
                                     const String& cic, const String& version) {
  deviceType_ = type;
  lacisId_ = lacisId;
  cic_ = cic;
  firmwareVersion_ = version;
  fid_ = settings_->getString("fid", "0150");
}

void HttpManagerIs10::onSettingsChanged(void (*callback)()) {
  settingsChangedCallback_ = callback;
}

void HttpManagerIs10::onRebootRequest(void (*callback)()) {
  rebootCallback_ = callback;
}

// ========================================
// ルーター設定読み書き
// ========================================
void HttpManagerIs10::loadRouters() {
  routerCount_ = 0;

  if (!SPIFFS.exists("/routers.json")) {
    Serial.println("[HTTP-IS10] No routers.json found");
    return;
  }

  File file = SPIFFS.open("/routers.json", "r");
  if (!file) return;

  String json = file.readString();
  file.close();

  DynamicJsonDocument doc(8192);
  DeserializationError err = deserializeJson(doc, json);
  if (err) {
    Serial.printf("[HTTP-IS10] JSON parse error: %s\n", err.c_str());
    return;
  }

  JsonArray arr = doc.as<JsonArray>();
  for (JsonObject obj : arr) {
    if (routerCount_ >= MAX_ROUTERS) break;
    routers_[routerCount_].rid = obj["rid"] | "";
    routers_[routerCount_].ipAddr = obj["ipAddr"] | "";
    routers_[routerCount_].publicKey = obj["publicKey"] | "";
    routers_[routerCount_].sshPort = obj["sshPort"] | 22;
    routers_[routerCount_].username = obj["username"] | "";
    routers_[routerCount_].password = obj["password"] | "";
    routers_[routerCount_].enabled = obj["enabled"] | true;
    if (routers_[routerCount_].ipAddr.length() > 0) {
      routerCount_++;
    }
  }

  Serial.printf("[HTTP-IS10] Loaded %d routers\n", routerCount_);
}

void HttpManagerIs10::saveRouters() {
  DynamicJsonDocument doc(8192);
  JsonArray arr = doc.to<JsonArray>();

  for (int i = 0; i < routerCount_; i++) {
    JsonObject obj = arr.createNestedObject();
    obj["rid"] = routers_[i].rid;
    obj["ipAddr"] = routers_[i].ipAddr;
    obj["publicKey"] = routers_[i].publicKey;
    obj["sshPort"] = routers_[i].sshPort;
    obj["username"] = routers_[i].username;
    obj["password"] = routers_[i].password;
    obj["enabled"] = routers_[i].enabled;
  }

  File file = SPIFFS.open("/routers.json", "w");
  if (file) {
    serializeJson(doc, file);
    file.close();
    Serial.printf("[HTTP-IS10] Saved %d routers\n", routerCount_);
  }
}

// ========================================
// API ハンドラ
// ========================================
void HttpManagerIs10::handleRoot() {
  server_->send(200, "text/html", generateHTML());
}

void HttpManagerIs10::handleGetConfig() {
  DynamicJsonDocument doc(16384);

  // デバイス情報
  JsonObject device = doc.createNestedObject("device");
  device["type"] = deviceType_;
  device["lacisId"] = lacisId_;
  device["cic"] = cic_;
  device["version"] = firmwareVersion_;
  device["fid"] = fid_;
  device["ip"] = WiFi.localIP().toString();
  device["mac"] = WiFi.macAddress();

  // グローバル設定
  JsonObject global = doc.createNestedObject("global");
  global["endpoint"] = settings_->getString("is10_endpoint", "");
  global["timeout"] = settings_->getInt("is10_timeout", 30000);
  global["retryCount"] = settings_->getInt("is10_retry", 2);
  global["interval"] = settings_->getInt("is10_interval", 30000);

  // LacisID生成設定
  JsonObject lacisGen = doc.createNestedObject("lacisGen");
  lacisGen["prefix"] = settings_->getString("lacis_prefix", "4");
  lacisGen["productType"] = settings_->getString("lacis_ptype", "");
  lacisGen["productCode"] = settings_->getString("lacis_pcode", "");
  lacisGen["generator"] = settings_->getString("lacis_gen", "DeviceWithMac");

  // テナント設定
  JsonObject tenant = doc.createNestedObject("tenant");
  tenant["tid"] = settings_->getString("tid", "");
  tenant["lacisId"] = settings_->getString("tenant_lacisid", "");
  tenant["email"] = settings_->getString("tenant_email", "");
  tenant["cic"] = settings_->getString("tenant_cic", "");
  // パスワードは表示しない

  // WiFi設定（6ペア形式）
  JsonArray wifiArr = doc.createNestedArray("wifi");
  const char* defaultSSIDs[] = {"cluster1", "cluster2", "cluster3", "cluster4", "cluster5", "cluster6"};
  for (int i = 0; i < 6; i++) {
    JsonObject wifiPair = wifiArr.createNestedObject();
    String ssidKey = "wifi_ssid" + String(i + 1);
    wifiPair["ssid"] = settings_->getString(ssidKey.c_str(), defaultSSIDs[i]);
    // パスワードは表示しない（セキュリティ）
  }

  // ルーター設定
  JsonArray routersArr = doc.createNestedArray("routers");
  for (int i = 0; i < routerCount_; i++) {
    JsonObject r = routersArr.createNestedObject();
    r["rid"] = routers_[i].rid;
    r["ipAddr"] = routers_[i].ipAddr;
    r["sshPort"] = routers_[i].sshPort;
    r["username"] = routers_[i].username;
    r["enabled"] = routers_[i].enabled;
    // publicKey, passwordは長いので省略
    r["hasPublicKey"] = routers_[i].publicKey.length() > 0;
  }

  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}

void HttpManagerIs10::handleSaveGlobal() {
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"error\":\"No body\"}");
    return;
  }

  StaticJsonDocument<512> doc;
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
    return;
  }

  if (doc.containsKey("endpoint")) settings_->setString("is10_endpoint", doc["endpoint"]);
  if (doc.containsKey("timeout")) settings_->setInt("is10_timeout", doc["timeout"]);
  if (doc.containsKey("retryCount")) settings_->setInt("is10_retry", doc["retryCount"]);
  if (doc.containsKey("interval")) settings_->setInt("is10_interval", doc["interval"]);

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs10::handleSaveLacisGen() {
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"error\":\"No body\"}");
    return;
  }

  StaticJsonDocument<512> doc;
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
    return;
  }

  if (doc.containsKey("prefix")) settings_->setString("lacis_prefix", doc["prefix"]);
  if (doc.containsKey("productType")) settings_->setString("lacis_ptype", doc["productType"]);
  if (doc.containsKey("productCode")) settings_->setString("lacis_pcode", doc["productCode"]);
  if (doc.containsKey("generator")) settings_->setString("lacis_gen", doc["generator"]);

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs10::handleSaveRouter() {
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"error\":\"No body\"}");
    return;
  }

  DynamicJsonDocument doc(4096);
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
    return;
  }

  int index = doc["index"] | -1;
  bool isNew = (index < 0 || index >= routerCount_);

  if (isNew) {
    if (routerCount_ >= MAX_ROUTERS) {
      server_->send(400, "application/json", "{\"error\":\"Max routers reached\"}");
      return;
    }
    index = routerCount_++;
  }

  routers_[index].rid = doc["rid"] | "";
  routers_[index].ipAddr = doc["ipAddr"] | "";
  if (doc.containsKey("publicKey") && doc["publicKey"].as<String>().length() > 0) {
    routers_[index].publicKey = doc["publicKey"] | "";
  }
  routers_[index].sshPort = doc["sshPort"] | 22;
  routers_[index].username = doc["username"] | "";
  if (doc.containsKey("password") && doc["password"].as<String>().length() > 0) {
    routers_[index].password = doc["password"] | "";
  }
  routers_[index].enabled = doc["enabled"] | true;

  saveRouters();
  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs10::handleDeleteRouter() {
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"error\":\"No body\"}");
    return;
  }

  StaticJsonDocument<128> doc;
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
    return;
  }

  int index = doc["index"] | -1;
  if (index < 0 || index >= routerCount_) {
    server_->send(400, "application/json", "{\"error\":\"Invalid index\"}");
    return;
  }

  // シフト
  for (int i = index; i < routerCount_ - 1; i++) {
    routers_[i] = routers_[i + 1];
  }
  routerCount_--;

  saveRouters();
  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs10::handleSaveTenant() {
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"error\":\"No body\"}");
    return;
  }

  StaticJsonDocument<512> doc;
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
    return;
  }

  if (doc.containsKey("tid")) settings_->setString("tid", doc["tid"]);
  if (doc.containsKey("lacisId")) settings_->setString("tenant_lacisid", doc["lacisId"]);
  if (doc.containsKey("email")) settings_->setString("tenant_email", doc["email"]);
  if (doc.containsKey("cic")) settings_->setString("tenant_cic", doc["cic"]);
  if (doc.containsKey("password") && doc["password"].as<String>().length() > 0) {
    settings_->setString("tenant_pass", doc["password"]);
  }

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs10::handleSaveWifi() {
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"error\":\"No body\"}");
    return;
  }

  DynamicJsonDocument doc(2048);
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
    return;
  }

  // 6ペア形式で保存
  if (doc.containsKey("pairs") && doc["pairs"].is<JsonArray>()) {
    JsonArray pairs = doc["pairs"].as<JsonArray>();
    for (int i = 0; i < 6 && i < (int)pairs.size(); i++) {
      JsonObject pair = pairs[i];
      String ssidKey = "wifi_ssid" + String(i + 1);
      String passKey = "wifi_pass" + String(i + 1);

      if (pair.containsKey("ssid")) {
        settings_->setString(ssidKey.c_str(), pair["ssid"].as<String>());
      }
      if (pair.containsKey("password") && pair["password"].as<String>().length() > 0) {
        settings_->setString(passKey.c_str(), pair["password"].as<String>());
      }
    }
  }

  Serial.println("[HTTP-IS10] WiFi settings saved");
  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs10::handleReboot() {
  server_->send(200, "application/json", "{\"ok\":true,\"message\":\"Rebooting...\"}");
  delay(500);
  if (rebootCallback_) rebootCallback_();
  ESP.restart();
}

void HttpManagerIs10::handleFactoryReset() {
  settings_->clear();
  SPIFFS.remove("/routers.json");
  server_->send(200, "application/json", "{\"ok\":true,\"message\":\"Factory reset...\"}");
  delay(500);
  ESP.restart();
}

void HttpManagerIs10::handleNotFound() {
  server_->send(404, "text/plain", "Not Found");
}

// ========================================
// Getter
// ========================================
GlobalSetting HttpManagerIs10::getGlobalSetting() {
  GlobalSetting gs;
  gs.endpoint = settings_->getString("is10_endpoint", "");
  gs.timeout = settings_->getInt("is10_timeout", 30000);
  gs.retryCount = settings_->getInt("is10_retry", 2);
  gs.interval = settings_->getInt("is10_interval", 30000);
  return gs;
}

LacisIdGenSetting HttpManagerIs10::getLacisIdGenSetting() {
  LacisIdGenSetting ls;
  ls.prefix = settings_->getString("lacis_prefix", "4");
  ls.productType = settings_->getString("lacis_ptype", "");
  ls.productCode = settings_->getString("lacis_pcode", "");
  ls.generator = settings_->getString("lacis_gen", "DeviceWithMac");
  return ls;
}

RouterSetting HttpManagerIs10::getRouter(int index) {
  if (index >= 0 && index < routerCount_) {
    return routers_[index];
  }
  return RouterSetting();
}

int HttpManagerIs10::getRouterCount() {
  return routerCount_;
}

// ========================================
// HTML/CSS/JS 生成
// ========================================
String HttpManagerIs10::generateCSS() {
  return R"CSS(
<style>
:root {
  --chakra-blue-500: #3182ce;
  --chakra-blue-600: #2b6cb0;
  --chakra-gray-50: #f7fafc;
  --chakra-gray-100: #edf2f7;
  --chakra-gray-200: #e2e8f0;
  --chakra-gray-600: #718096;
  --chakra-gray-700: #4a5568;
  --chakra-gray-800: #2d3748;
  --chakra-red-500: #e53e3e;
  --chakra-green-500: #38a169;
}
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  background: var(--chakra-gray-50);
  color: var(--chakra-gray-800);
  line-height: 1.5;
}
.container { max-width: 900px; margin: 0 auto; padding: 20px; }
h1 {
  font-size: 24px;
  font-weight: 700;
  margin-bottom: 8px;
  color: var(--chakra-gray-800);
}
.subtitle {
  color: var(--chakra-gray-600);
  font-size: 14px;
  margin-bottom: 24px;
}
.card {
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.12);
  padding: 20px;
  margin-bottom: 16px;
}
.card-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 16px;
  color: var(--chakra-gray-700);
  border-bottom: 1px solid var(--chakra-gray-200);
  padding-bottom: 8px;
}
.form-group { margin-bottom: 12px; }
.form-group label {
  display: block;
  font-size: 13px;
  font-weight: 500;
  color: var(--chakra-gray-700);
  margin-bottom: 4px;
}
.form-group input, .form-group select, .form-group textarea {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--chakra-gray-200);
  border-radius: 6px;
  font-size: 14px;
  transition: border-color 0.2s;
}
.form-group input:focus, .form-group select:focus, .form-group textarea:focus {
  outline: none;
  border-color: var(--chakra-blue-500);
  box-shadow: 0 0 0 1px var(--chakra-blue-500);
}
.form-row { display: flex; gap: 12px; }
.form-row .form-group { flex: 1; }
.btn {
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s;
}
.btn-primary {
  background: var(--chakra-blue-500);
  color: white;
}
.btn-primary:hover { background: var(--chakra-blue-600); }
.btn-danger {
  background: var(--chakra-red-500);
  color: white;
}
.btn-sm { padding: 4px 10px; font-size: 12px; }
.device-info {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 12px;
  font-size: 13px;
}
.info-item { background: var(--chakra-gray-100); padding: 8px 12px; border-radius: 4px; }
.info-item .label { color: var(--chakra-gray-600); font-size: 11px; }
.info-item .value { font-weight: 500; word-break: break-all; }
.router-list { margin-top: 12px; }
.router-item {
  background: var(--chakra-gray-100);
  padding: 12px;
  border-radius: 6px;
  margin-bottom: 8px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.router-item .router-info { flex: 1; }
.router-item .rid { font-weight: 600; }
.router-item .ip { color: var(--chakra-gray-600); font-size: 13px; }
.router-actions { display: flex; gap: 8px; }
.tabs { display: flex; gap: 4px; margin-bottom: 16px; border-bottom: 1px solid var(--chakra-gray-200); }
.tab {
  padding: 8px 16px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  color: var(--chakra-gray-600);
}
.tab.active { color: var(--chakra-blue-500); border-bottom-color: var(--chakra-blue-500); }
.tab-content { display: none; }
.tab-content.active { display: block; }
.toast {
  position: fixed;
  bottom: 20px;
  right: 20px;
  background: var(--chakra-green-500);
  color: white;
  padding: 12px 20px;
  border-radius: 6px;
  display: none;
}
</style>
)CSS";
}

String HttpManagerIs10::generateJS() {
  return R"JS(
<script>
let config = {};

async function loadConfig() {
  try {
    const res = await fetch('/api/config');
    config = await res.json();
    renderConfig();
  } catch(e) { console.error(e); }
}

function renderConfig() {
  // Device info
  document.getElementById('dev-type').textContent = config.device?.type || '-';
  document.getElementById('dev-lacis').textContent = config.device?.lacisId || '-';
  document.getElementById('dev-cic').textContent = config.device?.cic || '-';
  document.getElementById('dev-ip').textContent = config.device?.ip || '-';
  document.getElementById('dev-ver').textContent = config.device?.version || '-';
  document.getElementById('dev-fid').textContent = config.device?.fid || '-';

  // Global
  document.getElementById('g-endpoint').value = config.global?.endpoint || '';
  document.getElementById('g-timeout').value = config.global?.timeout || 30000;
  document.getElementById('g-retry').value = config.global?.retryCount || 2;
  document.getElementById('g-interval').value = config.global?.interval || 30000;

  // LacisGen
  document.getElementById('l-prefix').value = config.lacisGen?.prefix || '4';
  document.getElementById('l-ptype').value = config.lacisGen?.productType || '';
  document.getElementById('l-pcode').value = config.lacisGen?.productCode || '';
  document.getElementById('l-gen').value = config.lacisGen?.generator || 'DeviceWithMac';

  // Tenant
  document.getElementById('t-tid').value = config.tenant?.tid || '';
  document.getElementById('t-lacis').value = config.tenant?.lacisId || '';
  document.getElementById('t-email').value = config.tenant?.email || '';
  document.getElementById('t-cic').value = config.tenant?.cic || '';

  // WiFi (6 pairs)
  renderWifiPairs();

  // Routers
  renderRouters();
}

function renderWifiPairs() {
  const container = document.getElementById('wifi-pairs');
  container.innerHTML = '';
  const pairs = config.wifi || [];
  for (let i = 0; i < 6; i++) {
    const ssid = pairs[i]?.ssid || '';
    container.innerHTML += `
      <div class="form-row" style="margin-bottom:8px">
        <div class="form-group" style="flex:2">
          <label>SSID ${i+1}</label>
          <input type="text" id="w-ssid${i}" value="${ssid}">
        </div>
        <div class="form-group" style="flex:2">
          <label>Password ${i+1}</label>
          <input type="password" id="w-pass${i}" placeholder="(unchanged)">
        </div>
      </div>
    `;
  }
}

function renderRouters() {
  const list = document.getElementById('router-list');
  list.innerHTML = '';
  (config.routers || []).forEach((r, i) => {
    list.innerHTML += `
      <div class="router-item">
        <div class="router-info">
          <div class="rid">RID: ${r.rid} ${r.enabled ? '' : '(disabled)'}</div>
          <div class="ip">${r.ipAddr}:${r.sshPort} - ${r.username}</div>
        </div>
        <div class="router-actions">
          <button class="btn btn-primary btn-sm" onclick="editRouter(${i})">Edit</button>
          <button class="btn btn-danger btn-sm" onclick="deleteRouter(${i})">Del</button>
        </div>
      </div>
    `;
  });
}

function showTab(name) {
  document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
  document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
  document.querySelector(`[data-tab="${name}"]`).classList.add('active');
  document.getElementById(`tab-${name}`).classList.add('active');
}

async function saveGlobal() {
  await postJson('/api/global', {
    endpoint: document.getElementById('g-endpoint').value,
    timeout: parseInt(document.getElementById('g-timeout').value),
    retryCount: parseInt(document.getElementById('g-retry').value),
    interval: parseInt(document.getElementById('g-interval').value)
  });
  toast('Global settings saved');
}

async function saveLacisGen() {
  await postJson('/api/lacisgen', {
    prefix: document.getElementById('l-prefix').value,
    productType: document.getElementById('l-ptype').value,
    productCode: document.getElementById('l-pcode').value,
    generator: document.getElementById('l-gen').value
  });
  toast('LacisID Gen settings saved');
}

async function saveTenant() {
  await postJson('/api/tenant', {
    tid: document.getElementById('t-tid').value,
    lacisId: document.getElementById('t-lacis').value,
    email: document.getElementById('t-email').value,
    cic: document.getElementById('t-cic').value,
    password: document.getElementById('t-pass').value
  });
  toast('Tenant settings saved');
}

async function saveWifi() {
  const pairs = [];
  for (let i = 0; i < 6; i++) {
    pairs.push({
      ssid: document.getElementById('w-ssid' + i).value,
      password: document.getElementById('w-pass' + i).value
    });
  }
  await postJson('/api/wifi', { pairs });
  toast('WiFi settings saved');
}

function addRouter() {
  document.getElementById('r-index').value = -1;
  document.getElementById('r-rid').value = '';
  document.getElementById('r-ip').value = '';
  document.getElementById('r-port').value = '22';
  document.getElementById('r-user').value = '';
  document.getElementById('r-pass').value = '';
  document.getElementById('r-key').value = '';
  document.getElementById('r-enabled').checked = true;
  document.getElementById('router-modal').style.display = 'block';
}

function editRouter(i) {
  const r = config.routers[i];
  document.getElementById('r-index').value = i;
  document.getElementById('r-rid').value = r.rid;
  document.getElementById('r-ip').value = r.ipAddr;
  document.getElementById('r-port').value = r.sshPort;
  document.getElementById('r-user').value = r.username;
  document.getElementById('r-pass').value = '';
  document.getElementById('r-key').value = '';
  document.getElementById('r-enabled').checked = r.enabled;
  document.getElementById('router-modal').style.display = 'block';
}

async function saveRouter() {
  await postJson('/api/router', {
    index: parseInt(document.getElementById('r-index').value),
    rid: document.getElementById('r-rid').value,
    ipAddr: document.getElementById('r-ip').value,
    sshPort: parseInt(document.getElementById('r-port').value),
    username: document.getElementById('r-user').value,
    password: document.getElementById('r-pass').value,
    publicKey: document.getElementById('r-key').value,
    enabled: document.getElementById('r-enabled').checked
  });
  document.getElementById('router-modal').style.display = 'none';
  toast('Router saved');
  loadConfig();
}

async function deleteRouter(i) {
  if (!confirm('Delete this router?')) return;
  await postJson('/api/router/delete', { index: i });
  toast('Router deleted');
  loadConfig();
}

async function reboot() {
  if (!confirm('Reboot device?')) return;
  await postJson('/api/reboot', {});
  alert('Rebooting...');
}

async function factoryReset() {
  if (!confirm('Factory reset? All settings will be lost.')) return;
  await postJson('/api/factory-reset', {});
  alert('Factory resetting...');
}

async function postJson(url, data) {
  await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  });
}

function toast(msg) {
  const t = document.getElementById('toast');
  t.textContent = msg;
  t.style.display = 'block';
  setTimeout(() => t.style.display = 'none', 2000);
}

window.onload = loadConfig;
</script>
)JS";
}

String HttpManagerIs10::generateHTML() {
  String html = R"HTML(
<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>IS10 - Router Inspector</title>
)HTML";
  html += generateCSS();
  html += R"HTML(
</head>
<body>
<div class="container">
  <h1>ar-is10 Router Inspector</h1>
  <p class="subtitle">OpenWrt Router Information Collector</p>

  <div class="card">
    <div class="card-title">Device Information</div>
    <div class="device-info">
      <div class="info-item"><div class="label">Type</div><div class="value" id="dev-type">-</div></div>
      <div class="info-item"><div class="label">LacisID</div><div class="value" id="dev-lacis">-</div></div>
      <div class="info-item"><div class="label">CIC</div><div class="value" id="dev-cic">-</div></div>
      <div class="info-item"><div class="label">IP Address</div><div class="value" id="dev-ip">-</div></div>
      <div class="info-item"><div class="label">FID</div><div class="value" id="dev-fid">-</div></div>
      <div class="info-item"><div class="label">Version</div><div class="value" id="dev-ver">-</div></div>
    </div>
  </div>

  <div class="tabs">
    <div class="tab active" data-tab="global" onclick="showTab('global')">Global</div>
    <div class="tab" data-tab="routers" onclick="showTab('routers')">Routers</div>
    <div class="tab" data-tab="lacis" onclick="showTab('lacis')">LacisID Gen</div>
    <div class="tab" data-tab="tenant" onclick="showTab('tenant')">Tenant</div>
    <div class="tab" data-tab="wifi" onclick="showTab('wifi')">WiFi</div>
    <div class="tab" data-tab="system" onclick="showTab('system')">System</div>
  </div>

  <div id="tab-global" class="tab-content active">
    <div class="card">
      <div class="card-title">Global Settings</div>
      <div class="form-group">
        <label>Endpoint URL</label>
        <input type="text" id="g-endpoint" placeholder="https://...">
      </div>
      <div class="form-row">
        <div class="form-group">
          <label>SSH Timeout (ms)</label>
          <input type="number" id="g-timeout" value="30000">
        </div>
        <div class="form-group">
          <label>Retry Count</label>
          <input type="number" id="g-retry" value="2">
        </div>
        <div class="form-group">
          <label>Interval (ms)</label>
          <input type="number" id="g-interval" value="30000">
        </div>
      </div>
      <button class="btn btn-primary" onclick="saveGlobal()">Save Global Settings</button>
    </div>
  </div>

  <div id="tab-routers" class="tab-content">
    <div class="card">
      <div class="card-title">Router Settings (max 20)</div>
      <button class="btn btn-primary" onclick="addRouter()">+ Add Router</button>
      <div id="router-list" class="router-list"></div>
    </div>
  </div>

  <div id="tab-lacis" class="tab-content">
    <div class="card">
      <div class="card-title">OpenWrt LacisID Generation</div>
      <div class="form-row">
        <div class="form-group">
          <label>Prefix</label>
          <input type="text" id="l-prefix" value="4">
        </div>
        <div class="form-group">
          <label>Product Type</label>
          <input type="text" id="l-ptype" placeholder="e.g. 011">
        </div>
        <div class="form-group">
          <label>Product Code</label>
          <input type="text" id="l-pcode" placeholder="e.g. 0001">
        </div>
      </div>
      <div class="form-group">
        <label>Generator</label>
        <select id="l-gen">
          <option value="DeviceWithMac">DeviceWithMac</option>
        </select>
      </div>
      <button class="btn btn-primary" onclick="saveLacisGen()">Save LacisID Settings</button>
    </div>
  </div>

  <div id="tab-tenant" class="tab-content">
    <div class="card">
      <div class="card-title">Tenant Authentication</div>
      <div class="form-group">
        <label>Tenant ID (TID)</label>
        <input type="text" id="t-tid">
      </div>
      <div class="form-row">
        <div class="form-group">
          <label>Primary LacisID</label>
          <input type="text" id="t-lacis">
        </div>
        <div class="form-group">
          <label>CIC</label>
          <input type="text" id="t-cic">
        </div>
      </div>
      <div class="form-group">
        <label>Email</label>
        <input type="email" id="t-email">
      </div>
      <div class="form-group">
        <label>Password (leave empty to keep current)</label>
        <input type="password" id="t-pass">
      </div>
      <button class="btn btn-primary" onclick="saveTenant()">Save Tenant Settings</button>
    </div>
  </div>

  <div id="tab-wifi" class="tab-content">
    <div class="card">
      <div class="card-title">WiFi Settings (6 pairs)</div>
      <div id="wifi-pairs"></div>
      <button class="btn btn-primary" onclick="saveWifi()" style="margin-top:12px">Save WiFi Settings</button>
    </div>
  </div>

  <div id="tab-system" class="tab-content">
    <div class="card">
      <div class="card-title">System</div>
      <button class="btn btn-primary" onclick="reboot()">Reboot Device</button>
      <button class="btn btn-danger" onclick="factoryReset()" style="margin-left:8px">Factory Reset</button>
    </div>
  </div>
</div>

<!-- Router Modal -->
<div id="router-modal" style="display:none;position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.5);z-index:100">
  <div style="background:white;max-width:500px;margin:50px auto;padding:20px;border-radius:8px">
    <h3 style="margin-bottom:16px">Router Configuration</h3>
    <input type="hidden" id="r-index">
    <div class="form-row">
      <div class="form-group">
        <label>RID (Resource ID)</label>
        <input type="text" id="r-rid" placeholder="e.g. 306">
      </div>
      <div class="form-group">
        <label>SSH Port</label>
        <input type="number" id="r-port" value="22">
      </div>
    </div>
    <div class="form-group">
      <label>IP Address</label>
      <input type="text" id="r-ip" placeholder="192.168.x.x">
    </div>
    <div class="form-row">
      <div class="form-group">
        <label>Username</label>
        <input type="text" id="r-user">
      </div>
      <div class="form-group">
        <label>Password</label>
        <input type="password" id="r-pass">
      </div>
    </div>
    <div class="form-group">
      <label>Public Key (optional)</label>
      <textarea id="r-key" rows="3" style="font-size:12px"></textarea>
    </div>
    <div class="form-group">
      <label><input type="checkbox" id="r-enabled" checked> Enabled</label>
    </div>
    <div style="margin-top:16px">
      <button class="btn btn-primary" onclick="saveRouter()">Save</button>
      <button class="btn" onclick="document.getElementById('router-modal').style.display='none'">Cancel</button>
    </div>
  </div>
</div>

<div id="toast" class="toast"></div>
)HTML";
  html += generateJS();
  html += "</body></html>";
  return html;
}
