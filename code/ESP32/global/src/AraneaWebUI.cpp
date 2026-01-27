/**
 * AraneaWebUI.cpp
 *
 * Aranea デバイス共通 Web UI フレームワーク実装
 */

#include "AraneaWebUI.h"
#include <WiFi.h>
#include <SPIFFS.h>
#include <Preferences.h>
#include <esp_system.h>

// ========================================
// コンストラクタ/デストラクタ
// ========================================
AraneaWebUI::AraneaWebUI() {}

AraneaWebUI::~AraneaWebUI() {
  if (server_) {
    server_->stop();
    delete server_;
  }
}

// ========================================
// 初期化
// ========================================
void AraneaWebUI::begin(SettingManager* settings, int port) {
  settings_ = settings;
  server_ = new WebServer(port);

  // 共通エンドポイント登録
  server_->on("/", HTTP_GET, [this]() { handleRoot(); });
  server_->on("/api/status", HTTP_GET, [this]() { handleStatus(); });
  server_->on("/api/config", HTTP_GET, [this]() { handleConfig(); });
  server_->on("/api/common/network", HTTP_POST, [this]() { handleSaveNetwork(); });
  server_->on("/api/common/ap", HTTP_POST, [this]() { handleSaveAP(); });
  server_->on("/api/common/cloud", HTTP_POST, [this]() { handleSaveCloud(); });
  server_->on("/api/common/tenant", HTTP_POST, [this]() { handleSaveTenant(); });
  server_->on("/api/system/settings", HTTP_POST, [this]() { handleSaveSystem(); });
  server_->on("/api/system/reboot", HTTP_POST, [this]() { handleReboot(); });
  server_->on("/api/system/factory-reset", HTTP_POST, [this]() { handleFactoryReset(); });
  server_->on("/api/system/clear-registration", HTTP_POST, [this]() { handleClearRegistration(); });

  // SpeedDial API
  server_->on("/api/speeddial", HTTP_GET, [this]() { handleSpeedDialGet(); });
  server_->on("/api/speeddial", HTTP_POST, [this]() { handleSpeedDialPost(); });

  // WiFi操作API
  server_->on("/api/wifi/list", HTTP_GET, [this]() { handleWifiList(); });
  server_->on("/api/wifi/add", HTTP_POST, [this]() { handleWifiAdd(); });
  server_->on("/api/wifi/delete", HTTP_POST, [this]() { handleWifiDelete(); });
  server_->on("/api/wifi/move", HTTP_POST, [this]() { handleWifiMove(); });
  server_->on("/api/wifi/connect", HTTP_POST, [this]() { handleWifiConnect(); });
  server_->on("/api/wifi/scan", HTTP_POST, [this]() { handleWifiScan(); });
  server_->on("/api/wifi/scan/result", HTTP_GET, [this]() { handleWifiScanResult(); });
  server_->on("/api/wifi/reset", HTTP_POST, [this]() { handleWifiReset(); });
  server_->on("/api/wifi/auto", HTTP_POST, [this]() { handleWifiAuto(); });

  server_->onNotFound([this]() { handleNotFound(); });

  // 派生クラスの独自エンドポイント登録
  registerTypeSpecificEndpoints();

  server_->begin();
  Serial.printf("[WebUI] Server started on port %d (UI v%s)\n", port, ARANEA_UI_VERSION);
}

// ========================================
// ループ処理
// ========================================
void AraneaWebUI::handle() {
  if (server_) {
    server_->handleClient();
  }
}

// ========================================
// デバイス情報設定
// ========================================
void AraneaWebUI::setDeviceInfo(const AraneaDeviceInfo& info) {
  deviceInfo_ = info;
}

// ========================================
// コールバック設定
// ========================================
void AraneaWebUI::onSettingsChanged(void (*callback)()) {
  settingsChangedCallback_ = callback;
}

void AraneaWebUI::onRebootRequest(void (*callback)()) {
  rebootCallback_ = callback;
}

void AraneaWebUI::onDeviceNameChanged(void (*callback)()) {
  deviceNameChangedCallback_ = callback;
}

void AraneaWebUI::onClearRegistration(void (*callback)()) {
  clearRegistrationCallback_ = callback;
}

// ========================================
// CIC認証
// ========================================
bool AraneaWebUI::validateCIC(const String& cic) {
  return cic.length() > 0 && cic == deviceInfo_.cic;
}

// ========================================
// Basic Auth認証
// ========================================
bool AraneaWebUI::checkAuth() {
  String password = settings_->getString("ui_password", "");
  if (password.length() == 0) {
    return true;  // パスワード未設定なら認証不要
  }

  if (!server_->authenticate("admin", password.c_str())) {
    return false;
  }
  return true;
}

void AraneaWebUI::requestAuth() {
  server_->requestAuthentication(BASIC_AUTH, "Aranea Device");
}

// ========================================
// ルートハンドラ
// ========================================
void AraneaWebUI::handleRoot() {
  // CICパラメータがあればJSON Statusを返す（API用、認証不要）
  if (server_->hasArg("cic")) {
    String cic = server_->arg("cic");
    if (validateCIC(cic)) {
      handleStatus();
      return;
    }
    // CIC不一致は401
    server_->send(401, "application/json", "{\"ok\":false,\"error\":\"Invalid CIC\"}");
    return;
  }

  // 通常アクセスはBasic Auth認証
  if (!checkAuth()) {
    requestAuth();
    return;
  }

  // 認証成功でHTML UI表示（チャンク転送でヒープ断片化を防止）
  sendHTMLChunked();
}

// ========================================
// HTMLチャンク転送（ヒープ断片化防止）
// ========================================
void AraneaWebUI::sendHTMLChunked() {
  // Chunked Transfer Encodingを使用して大きなHTMLを分割送信
  server_->setContentLength(CONTENT_LENGTH_UNKNOWN);
  server_->send(200, "text/html", "");

  // HTML Head部分
  server_->sendContent(F("<!DOCTYPE html>\n<html><head>\n<meta charset=\"UTF-8\">\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>Aranea Device Manager</title>\n"));
  yield();

  // CSS
  server_->sendContent(generateCSS());
  yield();

  // Body開始 + Header
  String bodyStart = F("</head><body>\n<div class=\"header\">\n<div class=\"logo\">");
  bodyStart += generateLogoSVG();
  bodyStart += F("</div>\n<h1>Aranea Device Manager</h1>\n<span class=\"version\">");
  bodyStart += deviceInfo_.deviceType + " v" + deviceInfo_.firmwareVersion;
  bodyStart += F("</span>\n</div>\n");
  server_->sendContent(bodyStart);
  yield();

  // Device Banner
  String banner = F("<div class=\"device-banner\">\n<div class=\"device-banner-inner\">\n<div class=\"device-model\">");
  banner += deviceInfo_.deviceType + " " + deviceInfo_.modelName;
  banner += F("</div>\n<div class=\"device-name\" id=\"device-name-display\">");
  String deviceName = settings_->getString("device_name", "");
  if (deviceName.length() > 0) {
    banner += deviceName;
  } else {
    banner += F("<span class=\"unnamed\">(未設定)</span>");
  }
  banner += F("</div>\n<div class=\"device-context\">");
  banner += deviceInfo_.contextDesc;
  banner += F("</div>\n</div>\n</div>\n");
  server_->sendContent(banner);
  yield();

  // Container + Tabs
  server_->sendContent(F("<div class=\"container\">\n<div class=\"tabs\">\n<div class=\"tab active\" data-tab=\"status\" onclick=\"showTab('status')\">Status</div>\n<div class=\"tab\" data-tab=\"network\" onclick=\"showTab('network')\">Network</div>\n<div class=\"tab\" data-tab=\"cloud\" onclick=\"showTab('cloud')\">Cloud</div>\n<div class=\"tab\" data-tab=\"tenant\" onclick=\"showTab('tenant')\">Tenant</div>\n<div class=\"tab\" data-tab=\"system\" onclick=\"showTab('system')\">System</div>\n"));
  yield();

  // デバイス固有タブ
  server_->sendContent(generateTypeSpecificTabs());
  server_->sendContent(F("</div>\n"));
  yield();

  // Status Tab
  server_->sendContent(F("<!-- Status Tab -->\n<div id=\"tab-status\" class=\"tab-content active\">\n<div class=\"card\">\n<div class=\"card-title\">Device Information</div>\n<div class=\"status-grid\">\n<div class=\"status-item\"><div class=\"label\">Type</div><div class=\"value\" id=\"d-type\">-</div></div>\n<div class=\"status-item\"><div class=\"label\">LacisID</div><div class=\"value\" id=\"d-lacis\">-</div></div>\n<div class=\"status-item\"><div class=\"label\">CIC</div><div class=\"value\" id=\"d-cic\">-</div></div>\n<div class=\"status-item\"><div class=\"label\">MAC</div><div class=\"value\" id=\"d-mac\">-</div></div>\n<div class=\"status-item\"><div class=\"label\">Hostname</div><div class=\"value\" id=\"d-host\">-</div></div>\n<div class=\"status-item\"><div class=\"label\">Version</div><div class=\"value\" id=\"d-ver\">-</div></div>\n</div>\n</div>\n"));
  yield();

  server_->sendContent(F("<div class=\"card\">\n<div class=\"card-title\">Live Status</div>\n<div class=\"status-grid\">\n<div class=\"status-item\"><div class=\"label\">IP Address</div><div class=\"value\" id=\"s-ip\">-</div></div>\n<div class=\"status-item\"><div class=\"label\">SSID</div><div class=\"value\" id=\"s-ssid\">-</div></div>\n<div class=\"status-item\"><div class=\"label\">RSSI</div><div class=\"value\" id=\"s-rssi\">-</div></div>\n<div class=\"status-item\"><div class=\"label\">NTP Time</div><div class=\"value\" id=\"s-ntp\">-</div></div>\n<div class=\"status-item\"><div class=\"label\">Uptime</div><div class=\"value\" id=\"s-uptime\">-</div></div>\n<div class=\"status-item\"><div class=\"label\">Free Heap</div><div class=\"value\" id=\"s-heap\">-</div></div>\n<div class=\"status-item\"><div class=\"label\">Chip Temp</div><div class=\"value\" id=\"s-temp\">-</div></div>\n</div>\n</div>\n</div>\n"));
  yield();

  // Network Tab
  server_->sendContent(F("<!-- Network Tab -->\n<div id=\"tab-network\" class=\"tab-content\">\n<div class=\"card\">\n<div class=\"card-title\" style=\"display:flex;justify-content:space-between;align-items:center\">WiFi Settings (STA Mode)<button class=\"btn btn-sm\" onclick=\"scanWifi()\" id=\"scan-btn\">Scan</button></div>\n<div id=\"wifi-list\"></div>\n<div class=\"form-row\" style=\"margin-top:12px\">\n<div class=\"form-group\"><label>Hostname</label><input type=\"text\" id=\"n-hostname\"></div>\n<div class=\"form-group\"><label>NTP Server</label><input type=\"text\" id=\"n-ntp\"></div>\n</div>\n<button class=\"btn btn-primary\" onclick=\"saveNetwork()\">Save Network</button>\n</div>\n"));
  yield();

  // WiFi Scan Modal + AP Settings
  server_->sendContent(F("<!-- WiFi Scan Modal -->\n<div id=\"scan-modal\" class=\"modal\" style=\"display:none\">\n<div class=\"modal-content\">\n<div class=\"modal-header\"><span>Available Networks</span><button onclick=\"closeScanModal()\">&times;</button></div>\n<div id=\"scan-results\" style=\"max-height:300px;overflow-y:auto\"></div>\n</div>\n</div>\n<div class=\"card\">\n<div class=\"card-title\">AP Mode Settings</div>\n<div class=\"form-row\">\n<div class=\"form-group\"><label>AP SSID</label><input type=\"text\" id=\"ap-ssid\"></div>\n<div class=\"form-group\"><label>AP Password</label><input type=\"password\" id=\"ap-pass\" placeholder=\"(open if empty)\"></div>\n<div class=\"form-group\"><label>Timeout (sec)</label><input type=\"number\" id=\"ap-timeout\" min=\"0\"></div>\n</div>\n<button class=\"btn btn-primary\" onclick=\"saveAP()\">Save AP</button>\n</div>\n</div>\n"));
  yield();

  // Cloud Tab
  server_->sendContent(F("<!-- Cloud Tab -->\n<div id=\"tab-cloud\" class=\"tab-content\">\n<div class=\"card\">\n<div class=\"card-title\">AraneaDeviceGate</div>\n<div class=\"form-group\"><label>Gate URL</label><input type=\"text\" id=\"c-gate\" placeholder=\"https://...araneaDeviceGate\"></div>\n<div class=\"form-group\"><label>State Report URL</label><input type=\"text\" id=\"c-state\" placeholder=\"https://...deviceStateReport\"></div>\n</div>\n<div class=\"card\">\n<div class=\"card-title\">Relay Endpoints (Zero3)</div>\n<div class=\"form-row\">\n<div class=\"form-group\"><label>Primary</label><input type=\"text\" id=\"c-relay1\"></div>\n<div class=\"form-group\"><label>Secondary</label><input type=\"text\" id=\"c-relay2\"></div>\n</div>\n<button class=\"btn btn-primary\" onclick=\"saveCloud()\">Save Cloud</button>\n</div>\n</div>\n"));
  yield();

  // Tenant Tab
  server_->sendContent(F("<!-- Tenant Tab -->\n<div id=\"tab-tenant\" class=\"tab-content\">\n<div class=\"card\">\n<div class=\"card-title\">Tenant Authentication</div>\n<div class=\"form-row\">\n<div class=\"form-group\"><label>Tenant ID (TID)</label><input type=\"text\" id=\"t-tid\"></div>\n<div class=\"form-group\"><label>Facility ID (FID)</label><input type=\"text\" id=\"t-fid\"></div>\n</div>\n<div class=\"form-row\">\n<div class=\"form-group\"><label>Primary LacisID</label><input type=\"text\" id=\"t-lacis\"></div>\n<div class=\"form-group\"><label>CIC</label><input type=\"text\" id=\"t-cic\"></div>\n</div>\n<div class=\"form-group\"><label>Email</label><input type=\"email\" id=\"t-email\"></div>\n<button class=\"btn btn-primary\" onclick=\"saveTenant()\">Save Tenant</button>\n</div>\n</div>\n"));
  yield();

  // System Tab
  server_->sendContent(F("<!-- System Tab -->\n<div id=\"tab-system\" class=\"tab-content\">\n<div class=\"card\">\n<div class=\"card-title\">Device Identity (LacisOath)</div>\n<div class=\"form-group\"><label>Device Name</label><input type=\"text\" id=\"s-device-name\" placeholder=\"例: 本社1F-ルーター監視\"></div>\n<p style=\"font-size:12px;color:var(--text-muted);margin-top:4px\">この名前はMQTT/クラウドで userobject.name として使用されます</p>\n</div>\n<div class=\"card\">\n<div class=\"card-title\">Security</div>\n<div class=\"form-group\"><label>UI Password (Basic Auth)</label><input type=\"password\" id=\"s-pass\" placeholder=\"(no auth if empty)\"></div>\n</div>\n<div class=\"card\">\n<div class=\"card-title\">Scheduled Reboot</div>\n<div class=\"form-row\">\n<div class=\"form-group\"><label><input type=\"checkbox\" id=\"s-reboot-en\"> Enable daily reboot</label></div>\n<div class=\"form-group\"><label>Time</label><input type=\"time\" id=\"s-reboot-time\" value=\"03:00\"></div>\n</div>\n<button class=\"btn btn-primary\" onclick=\"saveSystem()\">Save System</button>\n</div>\n<div class=\"card\">\n<div class=\"card-title\">Actions</div>\n<div class=\"btn-group\">\n<button class=\"btn btn-primary\" onclick=\"reboot()\">Reboot Now</button>\n<button class=\"btn btn-danger\" onclick=\"factoryReset()\">Factory Reset</button>\n</div>\n</div>\n</div>\n"));
  yield();

  // デバイス固有タブコンテンツ
  server_->sendContent(generateTypeSpecificTabContents());
  yield();

  // Container終了 + Toast
  server_->sendContent(F("</div>\n<div id=\"toast\" class=\"toast\"></div>\n"));
  yield();

  // JavaScript
  server_->sendContent(generateJS());
  yield();

  // HTML終了
  server_->sendContent(F("</body></html>"));

  // チャンク転送終了
  server_->sendContent("");
}

// ========================================
// ステータスAPI
// ========================================
void AraneaWebUI::handleStatus() {
  yield();
  DynamicJsonDocument doc(4096);

  doc["ok"] = true;
  doc["uiVersion"] = ARANEA_UI_VERSION;

  // デバイス情報
  JsonObject device = doc.createNestedObject("device");
  device["type"] = deviceInfo_.deviceType;
  device["lacisId"] = deviceInfo_.lacisId;
  device["cic"] = deviceInfo_.cic;
  device["mac"] = deviceInfo_.mac;
  device["hostname"] = deviceInfo_.hostname;

  // ネットワーク状態
  AraneaNetworkStatus net = getNetworkStatus();
  JsonObject network = doc.createNestedObject("network");
  network["ip"] = net.ip;
  network["ssid"] = net.ssid;
  network["rssi"] = net.rssi;
  network["gateway"] = net.gateway;
  network["apMode"] = net.apMode;

  // システム状態
  AraneaSystemStatus sys = getSystemStatus();
  JsonObject system = doc.createNestedObject("system");
  system["ntpTime"] = sys.ntpTime;
  system["ntpSynced"] = sys.ntpSynced;
  system["uptime"] = sys.uptime;
  system["uptimeHuman"] = formatUptime(sys.uptime);
  system["heap"] = sys.heap;
  system["heapLargest"] = sys.heapLargest;
  system["cpuFreq"] = sys.cpuFreq;
  system["flashSize"] = sys.flashSize;
  system["chipTemp"] = sys.chipTemp;

  // ファームウェア情報
  JsonObject firmware = doc.createNestedObject("firmware");
  firmware["version"] = deviceInfo_.firmwareVersion;
  firmware["uiVersion"] = ARANEA_UI_VERSION;
  firmware["buildDate"] = deviceInfo_.buildDate;
  firmware["modules"] = deviceInfo_.modules;

  // クラウド状態
  AraneaCloudStatus cloud = getCloudStatus();
  JsonObject cloudObj = doc.createNestedObject("cloud");
  cloudObj["registered"] = cloud.registered;
  cloudObj["mqttConnected"] = cloud.mqttConnected;
  cloudObj["lastStateReport"] = cloud.lastStateReport;
  cloudObj["lastStateReportCode"] = cloud.lastStateReportCode;

  // デバイス固有ステータス（派生クラスで追加）
  JsonObject typeSpecific = doc.createNestedObject("typeSpecific");
  getTypeSpecificStatus(typeSpecific);

  yield();
  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}

// ========================================
// 設定API
// ========================================
void AraneaWebUI::handleConfig() {
  if (!checkAuth()) { requestAuth(); return; }
  yield();
  DynamicJsonDocument doc(8192);

  // デバイス情報
  JsonObject device = doc.createNestedObject("device");
  device["type"] = deviceInfo_.deviceType;
  device["lacisId"] = deviceInfo_.lacisId;
  device["cic"] = deviceInfo_.cic;
  device["mac"] = deviceInfo_.mac;
  device["hostname"] = deviceInfo_.hostname;
  device["version"] = deviceInfo_.firmwareVersion;
  device["uiVersion"] = ARANEA_UI_VERSION;

  // ネットワーク設定
  JsonObject network = doc.createNestedObject("network");
  JsonArray wifiArr = network.createNestedArray("wifi");
  for (int i = 0; i < 6; i++) {
    JsonObject pair = wifiArr.createNestedObject();
    String ssidKey = "wifi_ssid" + String(i + 1);
    pair["ssid"] = settings_->getString(ssidKey.c_str(), "");
  }
  network["hostname"] = settings_->getString("hostname", deviceInfo_.hostname);
  network["ntpServer"] = settings_->getString("ntp_server", "pool.ntp.org");

  // AP設定
  JsonObject ap = doc.createNestedObject("ap");
  ap["ssid"] = settings_->getString("ap_ssid", deviceInfo_.hostname);
  ap["password"] = settings_->getString("ap_pass", "").length() > 0 ? "********" : "";
  ap["timeout"] = settings_->getInt("ap_timeout", 300);

  // クラウド設定
  JsonObject cloud = doc.createNestedObject("cloud");
  cloud["gateUrl"] = settings_->getString("gate_url", "");
  cloud["stateReportUrl"] = settings_->getString("state_report_url", "");
  cloud["relayPrimary"] = settings_->getString("relay_pri", "");
  cloud["relaySecondary"] = settings_->getString("relay_sec", "");

  // テナント設定
  JsonObject tenant = doc.createNestedObject("tenant");
  tenant["tid"] = settings_->getString("tid", "");
  tenant["fid"] = settings_->getString("fid", "");
  tenant["lacisId"] = settings_->getString("tenant_lacisid", "");
  tenant["email"] = settings_->getString("tenant_email", "");
  tenant["cic"] = settings_->getString("tenant_cic", "");

  // システム設定
  JsonObject system = doc.createNestedObject("system");
  system["deviceName"] = settings_->getString("device_name", "");
  system["uiPassword"] = settings_->getString("ui_password", "").length() > 0 ? "********" : "";
  system["rebootEnabled"] = settings_->getBool("reboot_enabled", false);
  system["rebootTime"] = settings_->getString("reboot_time", "03:00");

  // デバイス固有設定（派生クラスで追加）
  JsonObject typeSpecific = doc.createNestedObject("typeSpecific");
  getTypeSpecificConfig(typeSpecific);

  yield();
  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}

// ========================================
// ネットワーク設定保存
// ========================================
void AraneaWebUI::handleSaveNetwork() {
  if (!checkAuth()) { requestAuth(); return; }
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No body\"}");
    return;
  }

  DynamicJsonDocument doc(2048);
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid JSON\"}");
    return;
  }

  // WiFi設定（6ペア）
  if (doc.containsKey("wifi") && doc["wifi"].is<JsonArray>()) {
    JsonArray pairs = doc["wifi"].as<JsonArray>();
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

  if (doc.containsKey("hostname")) {
    settings_->setString("hostname", doc["hostname"].as<String>());
  }
  if (doc.containsKey("ntpServer")) {
    settings_->setString("ntp_server", doc["ntpServer"].as<String>());
  }

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

// ========================================
// AP設定保存
// ========================================
void AraneaWebUI::handleSaveAP() {
  if (!checkAuth()) { requestAuth(); return; }
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

  if (doc.containsKey("ssid")) {
    settings_->setString("ap_ssid", doc["ssid"].as<String>());
  }
  if (doc.containsKey("password")) {
    String pass = doc["password"].as<String>();
    if (pass.length() > 0 && pass != "********") {
      settings_->setString("ap_pass", pass);
    }
  }
  if (doc.containsKey("timeout")) {
    settings_->setInt("ap_timeout", doc["timeout"].as<int>());
  }

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

// ========================================
// クラウド設定保存
// ========================================
void AraneaWebUI::handleSaveCloud() {
  if (!checkAuth()) { requestAuth(); return; }
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No body\"}");
    return;
  }

  StaticJsonDocument<1024> doc;
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid JSON\"}");
    return;
  }

  if (doc.containsKey("gateUrl")) settings_->setString("gate_url", doc["gateUrl"].as<String>());
  if (doc.containsKey("stateReportUrl")) settings_->setString("state_report_url", doc["stateReportUrl"].as<String>());
  if (doc.containsKey("relayPrimary")) settings_->setString("relay_pri", doc["relayPrimary"].as<String>());
  if (doc.containsKey("relaySecondary")) settings_->setString("relay_sec", doc["relaySecondary"].as<String>());

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

// ========================================
// テナント設定保存
// ========================================
void AraneaWebUI::handleSaveTenant() {
  if (!checkAuth()) { requestAuth(); return; }
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

  // バリデーション
  if (doc.containsKey("tid")) {
    String tid = doc["tid"].as<String>();
    if (tid.length() > 0 && !validateTid(tid)) {
      server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid tid: must be T + 19 digits (20 chars total)\"}");
      return;
    }
  }
  if (doc.containsKey("fid")) {
    String fid = doc["fid"].as<String>();
    if (fid.length() > 0 && !validateFid(fid)) {
      server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid fid: must be exactly 4 digits\"}");
      return;
    }
  }
  if (doc.containsKey("lacisId")) {
    String lacisId = doc["lacisId"].as<String>();
    if (lacisId.length() > 0 && !validateLacisId(lacisId)) {
      server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid lacisId: must be exactly 20 digits\"}");
      return;
    }
  }

  // 保存
  if (doc.containsKey("tid")) settings_->setString("tid", doc["tid"].as<String>());
  if (doc.containsKey("fid")) settings_->setString("fid", doc["fid"].as<String>());
  if (doc.containsKey("lacisId")) settings_->setString("tenant_lacisid", doc["lacisId"].as<String>());
  if (doc.containsKey("email")) settings_->setString("tenant_email", doc["email"].as<String>());
  if (doc.containsKey("cic")) settings_->setString("tenant_cic", doc["cic"].as<String>());

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

// ========================================
// システム設定保存
// ========================================
void AraneaWebUI::handleSaveSystem() {
  if (!checkAuth()) { requestAuth(); return; }
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

  bool deviceNameChanged = false;
  if (doc.containsKey("deviceName")) {
    String oldName = settings_->getString("device_name", "");
    String newName = doc["deviceName"].as<String>();
    if (oldName != newName) {
      settings_->setString("device_name", newName);
      deviceNameChanged = true;
      Serial.printf("[WebUI] deviceName changed: \"%s\" -> \"%s\"\n", oldName.c_str(), newName.c_str());
    }
  }
  if (doc.containsKey("uiPassword")) {
    String pass = doc["uiPassword"].as<String>();
    if (pass.length() > 0 && pass != "********") {
      settings_->setString("ui_password", pass);
    }
  }
  if (doc.containsKey("rebootEnabled")) {
    settings_->setBool("reboot_enabled", doc["rebootEnabled"].as<bool>());
  }
  if (doc.containsKey("rebootTime")) {
    settings_->setString("reboot_time", doc["rebootTime"].as<String>());
  }

  if (settingsChangedCallback_) settingsChangedCallback_();

  // deviceName変更時は即時deviceStateReport送信トリガー（SSOT）
  if (deviceNameChanged && deviceNameChangedCallback_) {
    deviceNameChangedCallback_();
  }

  server_->send(200, "application/json", "{\"ok\":true}");
}

// ========================================
// 再起動
// ========================================
void AraneaWebUI::handleReboot() {
  if (!checkAuth()) { requestAuth(); return; }

  Serial.println("[WebUI] Reboot requested via API");

  // Connection: closeヘッダを追加してレスポンス送信
  server_->sendHeader("Connection", "close");
  server_->send(200, "application/json", "{\"ok\":true,\"message\":\"Rebooting...\"}");

  // ネットワークスタックに送信時間を与える
  for (int i = 0; i < 10; i++) {
    yield();
    delay(100);
  }

  // クライアント接続を明示的にクローズ
  if (server_->client().connected()) {
    server_->client().flush();
    server_->client().stop();
  }

  delay(100);

  if (rebootCallback_) rebootCallback_();
  ESP.restart();
}

// ========================================
// ファクトリーリセット
// ========================================
void AraneaWebUI::handleFactoryReset() {
  if (!checkAuth()) { requestAuth(); return; }

  Serial.println("[WebUI] Factory reset requested");

  // isms namespace クリア
  settings_->clear();
  Serial.println("[WebUI] NVS isms namespace cleared");

  // aranea namespace クリア
  Preferences araneaPrefs;
  araneaPrefs.begin("aranea", false);
  araneaPrefs.clear();
  araneaPrefs.end();
  Serial.println("[WebUI] NVS aranea namespace cleared");

  // SPIFFS全ファイル削除
  File root = SPIFFS.open("/");
  File file = root.openNextFile();
  while (file) {
    String fileName = file.name();
    file.close();
    SPIFFS.remove("/" + fileName);
    Serial.printf("[WebUI] Deleted: %s\n", fileName.c_str());
    file = root.openNextFile();
  }
  root.close();
  Serial.println("[WebUI] SPIFFS cleared");

  // Connection: closeヘッダを追加してレスポンス送信
  server_->sendHeader("Connection", "close");
  server_->send(200, "application/json", "{\"ok\":true,\"message\":\"Factory reset complete. All settings and SPIFFS cleared.\"}");

  // ネットワークスタックに送信時間を与える
  for (int i = 0; i < 10; i++) {
    yield();
    delay(100);
  }

  // クライアント接続を明示的にクローズ
  if (server_->client().connected()) {
    server_->client().flush();
    server_->client().stop();
  }

  delay(100);
  ESP.restart();
}

// ========================================
// 登録クリア（再登録用）
// ========================================
void AraneaWebUI::handleClearRegistration() {
  if (!checkAuth()) { requestAuth(); return; }

  Serial.println("[WebUI] Clear registration requested");

  if (clearRegistrationCallback_) {
    clearRegistrationCallback_();
    server_->send(200, "application/json", "{\"ok\":true,\"message\":\"Registration cleared. Reboot to re-register.\"}");
  } else {
    server_->send(500, "application/json", "{\"ok\":false,\"error\":\"Clear registration callback not set\"}");
  }
}

// ========================================
// 404
// ========================================
void AraneaWebUI::handleNotFound() {
  server_->send(404, "application/json", "{\"ok\":false,\"error\":\"Not Found\"}");
}

// ========================================
// SpeedDial API - GET
// ========================================
void AraneaWebUI::handleSpeedDialGet() {
  if (!checkAuth()) { requestAuth(); return; }

  String text = generateSpeedDialText();
  server_->send(200, "text/plain; charset=utf-8", text);
}

// ========================================
// SpeedDial API - POST
// ========================================
void AraneaWebUI::handleSpeedDialPost() {
  if (!checkAuth()) { requestAuth(); return; }

  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No body\"}");
    return;
  }

  String body = server_->arg("plain");
  std::vector<String> errors;

  // INIパース
  String currentSection = "";
  std::vector<String> currentLines;

  int start = 0;
  while (start < (int)body.length()) {
    int end = body.indexOf('\n', start);
    if (end < 0) end = body.length();

    String line = body.substring(start, end);
    line.trim();
    start = end + 1;

    // 空行・コメントスキップ
    if (line.length() == 0 || line[0] == '#' || line[0] == ';') continue;

    // セクションヘッダ
    if (line[0] == '[' && line.indexOf(']') > 0) {
      // 前のセクションを適用
      if (currentSection.length() > 0 && currentLines.size() > 0) {
        if (!applySpeedDialSection(currentSection, currentLines)) {
          errors.push_back("Section [" + currentSection + "] apply failed");
        }
      }

      currentSection = line.substring(1, line.indexOf(']'));
      currentLines.clear();
      continue;
    }

    // key=value行
    if (line.indexOf('=') > 0) {
      currentLines.push_back(line);
    }
  }

  // 最後のセクションを適用
  if (currentSection.length() > 0 && currentLines.size() > 0) {
    if (!applySpeedDialSection(currentSection, currentLines)) {
      errors.push_back("Section [" + currentSection + "] apply failed");
    }
  }

  if (settingsChangedCallback_) settingsChangedCallback_();

  DynamicJsonDocument doc(512);
  doc["ok"] = errors.size() == 0;
  if (errors.size() > 0) {
    JsonArray errArr = doc.createNestedArray("errors");
    for (const auto& e : errors) errArr.add(e);
  }

  String response;
  serializeJson(doc, response);
  server_->send(errors.size() == 0 ? 200 : 400, "application/json", response);
}

// ========================================
// SpeedDial INIテキスト生成
// ========================================
String AraneaWebUI::generateSpeedDialText() {
  String text = "";

  // [Device] セクション（読み取り専用情報）
  text += "[Device]\n";
  text += "type=" + deviceInfo_.deviceType + "\n";
  text += "lacisId=" + deviceInfo_.lacisId + "\n";
  text += "cic=" + deviceInfo_.cic + "\n";
  text += "mac=" + deviceInfo_.mac + "\n";
  text += "version=" + deviceInfo_.firmwareVersion + "\n";
  text += "uiVersion=" + String(ARANEA_UI_VERSION) + "\n";
  text += "\n";

  // [WiFi] セクション（6スロット）
  text += "[WiFi]\n";
  for (int i = 0; i < 6; i++) {
    String ssid = settings_->getString(("wifi_ssid" + String(i + 1)).c_str(), "");
    if (ssid.length() > 0) {
      text += "ssid" + String(i + 1) + "=" + ssid + "\n";
      text += "pass" + String(i + 1) + "=********\n";
    }
  }
  text += "hostname=" + settings_->getString("hostname", deviceInfo_.hostname) + "\n";
  text += "\n";

  // [NTP] セクション
  text += "[NTP]\n";
  text += "server=" + settings_->getString("ntp_server", "pool.ntp.org") + "\n";
  text += "\n";

  // [Cloud] セクション
  text += "[Cloud]\n";
  text += "gateUrl=" + settings_->getString("gate_url", "") + "\n";
  text += "stateReportUrl=" + settings_->getString("state_report_url", "") + "\n";
  text += "relayPrimary=" + settings_->getString("relay_pri", "") + "\n";
  text += "relaySecondary=" + settings_->getString("relay_sec", "") + "\n";
  text += "\n";

  // [Tenant] セクション
  text += "[Tenant]\n";
  text += "tid=" + settings_->getString("tid", "") + "\n";
  text += "fid=" + settings_->getString("fid", "") + "\n";
  text += "lacisId=" + settings_->getString("tenant_lacisid", "") + "\n";
  text += "email=" + settings_->getString("tenant_email", "") + "\n";
  text += "cic=" + settings_->getString("tenant_cic", "") + "\n";
  text += "\n";

  // [System] セクション
  text += "[System]\n";
  text += "deviceName=" + settings_->getString("device_name", "") + "\n";
  text += "rebootEnabled=" + String(settings_->getBool("reboot_enabled", false) ? "true" : "false") + "\n";
  text += "rebootTime=" + settings_->getString("reboot_time", "03:00") + "\n";
  text += "\n";

  // デバイス固有セクション（派生クラスで実装）
  text += generateTypeSpecificSpeedDial();

  return text;
}

// ========================================
// SpeedDialセクション適用
// ========================================
bool AraneaWebUI::applySpeedDialSection(const String& section, const std::vector<String>& lines) {
  // key=value をパースするラムダ
  auto parseLine = [](const String& line, String& key, String& value) -> bool {
    int eq = line.indexOf('=');
    if (eq <= 0) return false;
    key = line.substring(0, eq);
    value = line.substring(eq + 1);
    key.trim();
    value.trim();
    return true;
  };

  if (section == "Device") {
    // 読み取り専用、無視
    return true;
  }

  if (section == "WiFi") {
    for (const auto& line : lines) {
      String key, value;
      if (!parseLine(line, key, value)) continue;

      // ssid1-6, pass1-6, hostname
      for (int i = 1; i <= 6; i++) {
        if (key == "ssid" + String(i)) {
          settings_->setString(("wifi_ssid" + String(i)).c_str(), value);
        } else if (key == "pass" + String(i)) {
          if (value.length() > 0 && value != "********") {
            settings_->setString(("wifi_pass" + String(i)).c_str(), value);
          }
        }
      }
      if (key == "hostname") {
        settings_->setString("hostname", value);
      }
    }
    return true;
  }

  if (section == "NTP") {
    for (const auto& line : lines) {
      String key, value;
      if (!parseLine(line, key, value)) continue;
      if (key == "server") {
        settings_->setString("ntp_server", value);
      }
    }
    return true;
  }

  if (section == "Cloud") {
    for (const auto& line : lines) {
      String key, value;
      if (!parseLine(line, key, value)) continue;
      if (key == "gateUrl") settings_->setString("gate_url", value);
      else if (key == "stateReportUrl") settings_->setString("state_report_url", value);
      else if (key == "relayPrimary") settings_->setString("relay_pri", value);
      else if (key == "relaySecondary") settings_->setString("relay_sec", value);
    }
    return true;
  }

  if (section == "Tenant") {
    for (const auto& line : lines) {
      String key, value;
      if (!parseLine(line, key, value)) continue;

      // バリデーション付き保存
      if (key == "tid") {
        if (value.length() > 0 && !validateTid(value)) {
          Serial.printf("[WebUI] Invalid TID: %s\n", value.c_str());
          continue;
        }
        settings_->setString("tid", value);
      }
      else if (key == "fid") {
        if (value.length() > 0 && !validateFid(value)) {
          Serial.printf("[WebUI] Invalid FID: %s\n", value.c_str());
          continue;
        }
        settings_->setString("fid", value);
      }
      else if (key == "lacisId") {
        if (value.length() > 0 && !validateLacisId(value)) {
          Serial.printf("[WebUI] Invalid lacisId: %s\n", value.c_str());
          continue;
        }
        settings_->setString("tenant_lacisid", value);
      }
      else if (key == "email") settings_->setString("tenant_email", value);
      else if (key == "cic") settings_->setString("tenant_cic", value);
    }
    return true;
  }

  if (section == "System") {
    for (const auto& line : lines) {
      String key, value;
      if (!parseLine(line, key, value)) continue;

      if (key == "deviceName") {
        String oldName = settings_->getString("device_name", "");
        if (oldName != value) {
          settings_->setString("device_name", value);
          if (deviceNameChangedCallback_) deviceNameChangedCallback_();
        }
      }
      else if (key == "rebootEnabled") {
        settings_->setBool("reboot_enabled", value == "true" || value == "1");
      }
      else if (key == "rebootTime") {
        settings_->setString("reboot_time", value);
      }
    }
    return true;
  }

  // デバイス固有セクション（派生クラスで処理）
  return applyTypeSpecificSpeedDial(section, lines);
}

// ========================================
// WiFi API - リスト取得
// ========================================
void AraneaWebUI::handleWifiList() {
  if (!checkAuth()) { requestAuth(); return; }

  DynamicJsonDocument doc(1024);
  doc["ok"] = true;
  JsonArray arr = doc.createNestedArray("wifi");

  for (int i = 0; i < 6; i++) {
    String ssid = settings_->getString(("wifi_ssid" + String(i + 1)).c_str(), "");
    String pass = settings_->getString(("wifi_pass" + String(i + 1)).c_str(), "");

    JsonObject obj = arr.createNestedObject();
    obj["slot"] = i + 1;
    obj["ssid"] = ssid;
    obj["hasPassword"] = pass.length() > 0;
  }

  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}

// ========================================
// WiFi API - 追加
// ========================================
void AraneaWebUI::handleWifiAdd() {
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

  String ssid = doc["ssid"] | "";
  String pass = doc["password"] | "";
  int slot = doc["slot"] | 0;

  if (ssid.length() == 0) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"SSID required\"}");
    return;
  }

  // スロット指定がなければ空きスロットを探す
  if (slot <= 0 || slot > 6) {
    for (int i = 1; i <= 6; i++) {
      String existing = settings_->getString(("wifi_ssid" + String(i)).c_str(), "");
      if (existing.length() == 0) {
        slot = i;
        break;
      }
    }
  }

  if (slot <= 0 || slot > 6) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No empty slot\"}");
    return;
  }

  settings_->setString(("wifi_ssid" + String(slot)).c_str(), ssid);
  settings_->setString(("wifi_pass" + String(slot)).c_str(), pass);

  if (settingsChangedCallback_) settingsChangedCallback_();

  DynamicJsonDocument resp(128);
  resp["ok"] = true;
  resp["slot"] = slot;
  String response;
  serializeJson(resp, response);
  server_->send(200, "application/json", response);
}

// ========================================
// WiFi API - 削除
// ========================================
void AraneaWebUI::handleWifiDelete() {
  if (!checkAuth()) { requestAuth(); return; }

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

  int slot = doc["slot"] | 0;
  if (slot < 1 || slot > 6) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid slot (1-6)\"}");
    return;
  }

  settings_->setString(("wifi_ssid" + String(slot)).c_str(), "");
  settings_->setString(("wifi_pass" + String(slot)).c_str(), "");

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

// ========================================
// WiFi API - 順序移動
// ========================================
void AraneaWebUI::handleWifiMove() {
  if (!checkAuth()) { requestAuth(); return; }

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

  int from = doc["from"] | 0;
  int to = doc["to"] | 0;

  if (from < 1 || from > 6 || to < 1 || to > 6 || from == to) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid from/to (1-6)\"}");
    return;
  }

  // スワップ
  String ssidFrom = settings_->getString(("wifi_ssid" + String(from)).c_str(), "");
  String passFrom = settings_->getString(("wifi_pass" + String(from)).c_str(), "");
  String ssidTo = settings_->getString(("wifi_ssid" + String(to)).c_str(), "");
  String passTo = settings_->getString(("wifi_pass" + String(to)).c_str(), "");

  settings_->setString(("wifi_ssid" + String(from)).c_str(), ssidTo);
  settings_->setString(("wifi_pass" + String(from)).c_str(), passTo);
  settings_->setString(("wifi_ssid" + String(to)).c_str(), ssidFrom);
  settings_->setString(("wifi_pass" + String(to)).c_str(), passFrom);

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

// ========================================
// WiFi API - 接続
// ========================================
void AraneaWebUI::handleWifiConnect() {
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

  String ssid = doc["ssid"] | "";
  String pass = doc["password"] | "";

  if (ssid.length() == 0) {
    // スロット番号で接続
    int slot = doc["slot"] | 0;
    if (slot >= 1 && slot <= 6) {
      ssid = settings_->getString(("wifi_ssid" + String(slot)).c_str(), "");
      pass = settings_->getString(("wifi_pass" + String(slot)).c_str(), "");
    }
  }

  if (ssid.length() == 0) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"SSID required\"}");
    return;
  }

  // レスポンスを先に返す
  server_->send(200, "application/json", "{\"ok\":true,\"message\":\"Connecting...\"}");
  delay(100);

  // WiFi接続開始
  WiFi.disconnect();
  delay(100);
  WiFi.begin(ssid.c_str(), pass.c_str());

  Serial.printf("[WebUI] WiFi connecting to: %s\n", ssid.c_str());
}

// ========================================
// WiFi API - スキャン開始
// ========================================
void AraneaWebUI::handleWifiScan() {
  if (!checkAuth()) { requestAuth(); return; }

  if (wifiScanInProgress_) {
    server_->send(200, "application/json", "{\"ok\":true,\"status\":\"scanning\"}");
    return;
  }

  WiFi.scanDelete();
  WiFi.scanNetworks(true);  // async scan
  wifiScanInProgress_ = true;
  wifiScanStartTime_ = millis();

  server_->send(200, "application/json", "{\"ok\":true,\"status\":\"started\"}");
}

// ========================================
// WiFi API - スキャン結果取得
// ========================================
void AraneaWebUI::handleWifiScanResult() {
  if (!checkAuth()) { requestAuth(); return; }

  int n = WiFi.scanComplete();

  if (n == WIFI_SCAN_RUNNING) {
    server_->send(200, "application/json", "{\"ok\":true,\"status\":\"scanning\"}");
    return;
  }

  wifiScanInProgress_ = false;

  if (n == WIFI_SCAN_FAILED || n < 0) {
    server_->send(200, "application/json", "{\"ok\":true,\"status\":\"failed\",\"networks\":[]}");
    return;
  }

  DynamicJsonDocument doc(4096);
  doc["ok"] = true;
  doc["status"] = "complete";
  JsonArray arr = doc.createNestedArray("networks");

  for (int i = 0; i < n && i < 20; i++) {  // 最大20件
    JsonObject net = arr.createNestedObject();
    net["ssid"] = WiFi.SSID(i);
    net["rssi"] = WiFi.RSSI(i);
    net["channel"] = WiFi.channel(i);
    net["secure"] = WiFi.encryptionType(i) != WIFI_AUTH_OPEN;
  }

  WiFi.scanDelete();

  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}

// ========================================
// WiFi API - リセット
// ========================================
void AraneaWebUI::handleWifiReset() {
  if (!checkAuth()) { requestAuth(); return; }

  // 全スロットクリア
  for (int i = 1; i <= 6; i++) {
    settings_->setString(("wifi_ssid" + String(i)).c_str(), "");
    settings_->setString(("wifi_pass" + String(i)).c_str(), "");
  }

  // デフォルトを設定（cluster1-6）
  settings_->setString("wifi_ssid1", "cluster1");
  settings_->setString("wifi_pass1", "isms12345@");
  settings_->setString("wifi_ssid2", "cluster2");
  settings_->setString("wifi_pass2", "isms12345@");
  settings_->setString("wifi_ssid3", "cluster3");
  settings_->setString("wifi_pass3", "isms12345@");
  settings_->setString("wifi_ssid4", "cluster4");
  settings_->setString("wifi_pass4", "isms12345@");
  settings_->setString("wifi_ssid5", "cluster5");
  settings_->setString("wifi_pass5", "isms12345@");
  settings_->setString("wifi_ssid6", "cluster6");
  settings_->setString("wifi_pass6", "isms12345@");

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true,\"message\":\"Reset to cluster1-6\"}");
}

// ========================================
// WiFi API - 自動接続トグル
// ========================================
void AraneaWebUI::handleWifiAuto() {
  if (!checkAuth()) { requestAuth(); return; }

  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No body\"}");
    return;
  }

  StaticJsonDocument<64> doc;
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid JSON\"}");
    return;
  }

  bool autoConnect = doc["enabled"] | true;
  settings_->setBool("wifi_auto", autoConnect);

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

// ========================================
// ステータス取得ヘルパー
// ========================================
AraneaNetworkStatus AraneaWebUI::getNetworkStatus() {
  AraneaNetworkStatus status;
  status.ip = WiFi.localIP().toString();
  status.ssid = WiFi.SSID();
  status.rssi = WiFi.RSSI();
  status.gateway = WiFi.gatewayIP().toString();
  status.subnet = WiFi.subnetMask().toString();
  status.apMode = (WiFi.getMode() == WIFI_AP || WiFi.getMode() == WIFI_AP_STA);
  status.apSsid = WiFi.softAPSSID();
  return status;
}

AraneaSystemStatus AraneaWebUI::getSystemStatus() {
  AraneaSystemStatus status;

  // NTP時刻
  struct tm timeinfo;
  if (getLocalTime(&timeinfo, 100)) {
    char buf[32];
    strftime(buf, sizeof(buf), "%Y-%m-%dT%H:%M:%SZ", &timeinfo);
    status.ntpTime = String(buf);
    status.ntpSynced = true;
  } else {
    status.ntpTime = "1970-01-01T00:00:00Z";
    status.ntpSynced = false;
  }

  status.uptime = millis() / 1000;
  status.heap = ESP.getFreeHeap();
  status.heapLargest = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
  status.cpuFreq = ESP.getCpuFreqMHz();
  status.flashSize = ESP.getFlashChipSize();
  status.chipTemp = temperatureRead();  // Internal sensor (±5°C)

  return status;
}

AraneaCloudStatus AraneaWebUI::getCloudStatus() {
  AraneaCloudStatus status;
  status.registered = deviceInfo_.cic.length() > 0;
  status.mqttConnected = false;  // 派生クラスでオーバーライド
  status.lastStateReport = "";
  status.lastStateReportCode = 0;
  status.schemaVersion = settings_->getInt("config_schema_version", 0);
  return status;
}

String AraneaWebUI::formatUptime(unsigned long seconds) {
  unsigned long days = seconds / 86400;
  unsigned long hours = (seconds % 86400) / 3600;
  unsigned long mins = (seconds % 3600) / 60;
  unsigned long secs = seconds % 60;

  if (days > 0) {
    return String(days) + "d " + String(hours) + "h " + String(mins) + "m";
  } else if (hours > 0) {
    return String(hours) + "h " + String(mins) + "m " + String(secs) + "s";
  } else {
    return String(mins) + "m " + String(secs) + "s";
  }
}

// ========================================
// バリデーションヘルパー
// ========================================

// FID: 4桁の数字
bool AraneaWebUI::validateFid(const String& fid) {
  if (fid.length() != 4) return false;
  for (size_t i = 0; i < fid.length(); i++) {
    if (!isDigit(fid[i])) return false;
  }
  return true;
}

// TID: T + 19桁の数字 (合計20文字)
bool AraneaWebUI::validateTid(const String& tid) {
  if (tid.length() != 20) return false;
  if (tid[0] != 'T') return false;
  for (size_t i = 1; i < tid.length(); i++) {
    if (!isDigit(tid[i])) return false;
  }
  return true;
}

// LacisID: 20桁の数字
bool AraneaWebUI::validateLacisId(const String& lacisId) {
  if (lacisId.length() != 20) return false;
  for (size_t i = 0; i < lacisId.length(); i++) {
    if (!isDigit(lacisId[i])) return false;
  }
  return true;
}

// IPv4アドレス: 4オクテット、各0-255、CIDR不可
bool AraneaWebUI::validateIPv4(const String& ip) {
  if (ip.length() == 0 || ip.length() > 15) return false;

  // CIDR表記チェック
  if (ip.indexOf('/') >= 0) return false;

  // 連続ドットチェック
  if (ip.indexOf("..") >= 0) return false;

  // 先頭・末尾のドットチェック
  if (ip[0] == '.' || ip[ip.length() - 1] == '.') return false;

  int dotCount = 0;
  int octetStart = 0;

  for (size_t i = 0; i <= ip.length(); i++) {
    if (i == ip.length() || ip[i] == '.') {
      // オクテット抽出
      String octet = ip.substring(octetStart, i);

      // 空オクテットチェック
      if (octet.length() == 0) return false;

      // 数字のみかチェック
      for (size_t j = 0; j < octet.length(); j++) {
        if (!isDigit(octet[j])) return false;
      }

      // 範囲チェック (0-255)
      int val = octet.toInt();
      if (val < 0 || val > 255) return false;

      // 先頭ゼロチェック（01, 001等は不正だが0は許可）
      if (octet.length() > 1 && octet[0] == '0') return false;

      if (i < ip.length()) {
        dotCount++;
        octetStart = i + 1;
      }
    }
  }

  // ドット数が3であること（4オクテット）
  return dotCount == 3;
}

// ========================================
// ロゴSVG（インライン埋め込み）
// aranea_logo_icononly.svg を最適化
// ========================================
String AraneaWebUI::generateLogoSVG() {
  return R"(<svg viewBox="0 0 211 280" fill="currentColor" style="height:32px;width:auto"><path d="M187.43,209.6c-9.57-12.63-22.63-21.14-37.43-26.9-.48-.19-.83-.68-1.25-1.03.18-.1.34-.24.53-.3,4.99-1.36,10.33-2,14.92-4.2,8.82-4.24,17.97-8.39,25.62-14.33,13.42-10.42,18.72-25.75,21.18-42.04.22-1.47-1.51-4.13-2.93-4.69-1.46-.57-4.68.21-5.34,1.39-1.64,2.93-2.31,6.41-3.27,9.7-2.52,8.58-6.23,16.58-12.64,22.92-9.65,9.53-22.03,13.38-34.93,15.91-.71.14-1.43.2-2.14.3.4-.56.72-1.21,1.22-1.66,10.74-9.61,19.39-20.71,25.21-34.02,3.39-7.74,6.01-15.8,4.83-24.06-1.43-10.01-4.28-19.85-7.01-29.62-.44-1.59-3.39-3.38-5.24-3.46-3.09-.14-4.89,2.36-4.14,5.46,1.21,5.07,3.12,9.96,4.33,15.03,2.1,8.85,2.15,17.77-1.36,26.28-4.66,11.3-12.37,20.46-21.34,28.61-.18.16-.39.29-.6.41-.07.04-.14.09-.21.13-.01-.04-.03-.08-.05-.13-.03-.08-.07-.17-.11-.26-.09-.21-.18-.43-.17-.63.7-16.98-4.61-31.8-16.06-44.34-5.64-6.17-12.28-11.01-20.72-12.31-.3-.05-.58-.04-.88-.07V0h-3.88v91.63c-6.1.46-11.46,3.54-16.2,7.54-14.48,12.21-21.36,28.05-21.42,46.93,0,1.1-.13,2.2-.2,3.29-.82-.67-1.64-1.32-2.44-2.02-.62-.54-1.22-1.11-1.8-1.69-8.13-8.05-15.3-16.89-18.81-27.91-4.08-12.79-1.03-25.16,3.39-37.39.62-1.73-.56-5.65-1.86-6.16-1.75-.69-5.94.35-6.43,1.66-2.87,7.52-5.66,15.21-7.15,23.1-2.65,14.01,1.13,27.04,8.01,39.32,5.79,10.34,13.35,19.2,22.24,26.98.27.23.34.69.5,1.04-.2.04-.43.16-.6.1-8.81-2.92-18.05-4.97-26.34-8.98-14.94-7.23-20.95-21.2-24.38-36.5-.82-3.66-2.65-6.13-6.65-5.1-3.15.81-3.9,3.44-3.17,6.5.86,3.62,1.17,7.48,2.64,10.82,3.33,7.55,6.17,15.71,11.21,22.01,5.05,6.3,11.95,11.75,19.11,15.57,8.64,4.61,18.41,7.11,27.69,10.51.48.18.99.29,1.48.43-.44.36-.86.77-1.34,1.06-9.09,5.45-18.61,10.29-27.16,16.48-8.89,6.44-15.63,15.05-18.7,25.97-2.97,10.55-2.84,21.32-1.37,32.02.24,1.73,3.05,4.21,4.85,4.37,3.25.29,4.38-2.62,4.39-5.65,0-3.93-.18-7.87-.07-11.79.26-9.53,2.11-18.64,8.68-25.99,8.54-9.56,19.02-16.44,30.82-21.37.64-.27,1.27-.55,1.93-.75.07-.02.14-.03.21-.02.11,0,.22.03.34.05.08.02.15.04.23.05.04,0,.08,0,.11.02-.13.19-.26.38-.38.57-.13.19-.26.38-.4.56-5.8,7.69-12.1,15.05-17.26,23.14-5.94,9.32-5.42,19.94-2.72,30.15,2.19,8.27,5.6,16.22,8.51,24.3,1.06,2.94,3.06,5.41,6.23,3.91,1.6-.75,3.42-4.11,2.96-5.64-2.32-7.78-6.07-15.18-7.89-23.04-1.3-5.6-1.93-12.29-.06-17.49,4.47-12.45,13.46-22.02,23.8-30.2.1-.08.21-.15.32-.22.34-.2.71-.36,1.06-.55.04.28.07.56.11.84s.08.56.16.83c.99,3.58,1.92,7.19,3.06,10.73.44,1.35,2.28,2.94,1.93,3.74-2.68,6.12-1.17,15.61,3.72,19.66.94.78,2.57.71,3.89,1.03.31-1.37.84-2.73.87-4.11.03-1.83-1.04-4-.37-5.42,1-2.1,2.94-4.97,4.8-5.22,5.27-.71,10.7-.27,16.07-.26.46,0,1.05-.05,1.36.2,3.42,2.78,4.13,6.26,2.98,10.46-.35,1.27.23,2.91.77,4.21.13.3,2.12.28,2.84-.2,5.11-3.46,7.51-11.9,5.34-17.73-.44-1.19-.94-3.09-.36-3.81,3.44-4.21,4.96-8.92,4.75-14.28,0-.23.18-.47.28-.7.22.1.5.14.66.3,5.79,5.89,11.96,11.45,17.23,17.77,5.67,6.78,10.49,14.65,8.36,23.91-2.27,9.84-6.16,19.31-8.83,29.07-.47,1.72,1.07,4.71,2.61,5.97.8.65,4.89-.74,5.37-2.01,3.76-9.9,7.59-19.84,10.23-30.06,2.42-9.39,1.48-19.11-3.95-27.39-5.14-7.83-11.4-14.94-17.16-22.36-.12-.15-.19-.34-.27-.53-.03-.06-.05-.12-.08-.18h.12c.3-.03.63-.12.84,0,7.16,3.94,14.78,7.26,21.3,12.07,5.66,4.16,10.89,9.45,14.84,15.24,5.01,7.34,5.49,16.31,5.12,25.11v7.14c0,3.24.99,6.03,4.67,6.13,3.71.1,4.9-2.86,4.89-5.9-.04-8.59.75-17.42-.95-25.71-1.48-7.23-4.78-14.7-9.24-20.58z"/></svg>)";
}

// ========================================
// CSS生成
// ========================================
String AraneaWebUI::generateCSS() {
  return R"CSS(
<style>
:root{--primary:#4c4948;--primary-light:#6b6a69;--accent:#3182ce;--accent-hover:#2b6cb0;--bg:#f7fafc;--card:#fff;--border:#e2e8f0;--text:#2d3748;--text-muted:#718096;--success:#38a169;--danger:#e53e3e;--warning:#d69e2e}
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;background:var(--bg);color:var(--text);line-height:1.5}
.header{background:var(--primary);color:#fff;padding:12px 20px;display:flex;align-items:center;gap:12px;box-shadow:0 2px 4px rgba(0,0,0,.1)}
.header .logo{color:#fff}
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
.wifi-pair{display:flex;gap:8px;margin-bottom:8px;align-items:flex-end}
.wifi-pair .form-group{margin-bottom:0}
.wifi-pair .num{font-size:12px;color:var(--text-muted);min-width:20px;padding-bottom:10px}
.wifi-pair .scan-pick{padding:4px 8px;font-size:11px;cursor:pointer;background:var(--bg-secondary);border:1px solid var(--border);border-radius:4px}
.wifi-pair .scan-pick:hover{background:var(--accent);color:#fff}
.modal{position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(0,0,0,.5);z-index:2000;display:flex;align-items:center;justify-content:center}
.modal-content{background:var(--bg-card);border-radius:8px;width:90%;max-width:400px;box-shadow:0 4px 20px rgba(0,0,0,.2)}
.modal-header{display:flex;justify-content:space-between;align-items:center;padding:16px;border-bottom:1px solid var(--border);font-weight:600}
.modal-header button{background:none;border:none;font-size:20px;cursor:pointer;color:var(--text-muted)}
.scan-item{padding:12px 16px;border-bottom:1px solid var(--border);cursor:pointer;display:flex;justify-content:space-between;align-items:center}
.scan-item:hover{background:var(--bg-secondary)}
.scan-item .ssid{font-weight:500}
.scan-item .info{font-size:12px;color:var(--text-muted)}
.pin-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:12px}
.pin-card{background:var(--bg);border:1px solid var(--border);border-radius:8px;padding:12px;position:relative}
.pin-card.disabled{opacity:.5}
.pin-header{display:flex;justify-content:space-between;align-items:center;margin-bottom:8px}
.pin-name{font-weight:600;font-size:14px}
.pin-type{font-size:11px;color:var(--text-muted);text-transform:uppercase}
.pin-control{margin:12px 0}
.btn-toggle{padding:8px 20px;border:none;border-radius:6px;font-size:14px;font-weight:600;cursor:pointer;width:100%}
.btn-toggle.on{background:var(--success);color:#fff}
.btn-toggle.off{background:var(--border);color:var(--text)}
.btn-toggle:disabled{cursor:not-allowed;opacity:.6}
.input-state{display:block;text-align:center;padding:8px;background:var(--bg);border-radius:4px;font-weight:500}
.switch{position:relative;display:inline-block;width:44px;height:24px}
.switch input{opacity:0;width:0;height:0}
.slider{position:absolute;cursor:pointer;top:0;left:0;right:0;bottom:0;background-color:var(--border);transition:.3s;border-radius:24px}
.slider:before{position:absolute;content:"";height:18px;width:18px;left:3px;bottom:3px;background-color:#fff;transition:.3s;border-radius:50%}
input:checked+.slider{background-color:var(--accent)}
input:checked+.slider:before{transform:translateX(20px)}
.pin-settings{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:16px}
.pin-setting-card{background:var(--bg);border:1px solid var(--border);border-radius:8px;padding:16px}
.pin-setting-card h4{margin-bottom:12px;color:var(--primary)}
@media(max-width:600px){.form-row{flex-direction:column}.tabs{gap:0}.tab{padding:8px 12px;font-size:12px}}
</style>
)CSS";
}

// ========================================
// JavaScript生成
// ========================================
String AraneaWebUI::generateJS() {
  String js = R"JS(
<script>
let cfg={};
async function load(){
  try{
    const r=await fetch('/api/config');
    cfg=await r.json();
    render();
  }catch(e){console.error(e)}
}
function render(){
  // Device info
  document.getElementById('d-type').textContent=cfg.device?.type||'-';
  document.getElementById('d-lacis').textContent=cfg.device?.lacisId||'-';
  document.getElementById('d-cic').textContent=cfg.device?.cic||'-';
  document.getElementById('d-mac').textContent=cfg.device?.mac||'-';
  document.getElementById('d-host').textContent=cfg.device?.hostname||'-';
  document.getElementById('d-ver').textContent=(cfg.device?.version||'-')+' / UI '+(cfg.device?.uiVersion||'-');
  // Network
  renderWifi();
  document.getElementById('n-hostname').value=cfg.network?.hostname||'';
  document.getElementById('n-ntp').value=cfg.network?.ntpServer||'pool.ntp.org';
  // AP
  document.getElementById('ap-ssid').value=cfg.ap?.ssid||'';
  document.getElementById('ap-pass').value=cfg.ap?.password||'';
  document.getElementById('ap-timeout').value=cfg.ap?.timeout||300;
  // Cloud
  document.getElementById('c-gate').value=cfg.cloud?.gateUrl||'';
  document.getElementById('c-state').value=cfg.cloud?.stateReportUrl||'';
  document.getElementById('c-relay1').value=cfg.cloud?.relayPrimary||'';
  document.getElementById('c-relay2').value=cfg.cloud?.relaySecondary||'';
  // Tenant
  document.getElementById('t-tid').value=cfg.tenant?.tid||'';
  document.getElementById('t-fid').value=cfg.tenant?.fid||'';
  document.getElementById('t-lacis').value=cfg.tenant?.lacisId||'';
  document.getElementById('t-email').value=cfg.tenant?.email||'';
  document.getElementById('t-cic').value=cfg.tenant?.cic||'';
  // System
  document.getElementById('s-device-name').value=cfg.system?.deviceName||'';
  document.getElementById('s-pass').value=cfg.system?.uiPassword||'';
  document.getElementById('s-reboot-en').checked=cfg.system?.rebootEnabled||false;
  document.getElementById('s-reboot-time').value=cfg.system?.rebootTime||'03:00';
  // Type specific
  if(typeof renderTypeSpecific==='function')renderTypeSpecific();
}
let scanSlot=-1;
let scanNets=[];
function renderWifi(){
  const c=document.getElementById('wifi-list');
  c.innerHTML='';
  const w=cfg.network?.wifi||[];
  for(let i=0;i<6;i++){
    c.innerHTML+=`<div class="wifi-pair"><span class="num">${i+1}</span><div class="form-group" style="flex:2"><input type="text" id="w-ssid${i}" value="${w[i]?.ssid||''}" placeholder="SSID"></div><div class="form-group" style="flex:2"><input type="password" id="w-pass${i}" placeholder="Password"></div><button class="scan-pick" onclick="openScanForSlot(${i})">&#x25BC;</button></div>`;
  }
}
async function scanWifi(){
  const btn=document.getElementById('scan-btn');
  btn.textContent='Scanning...';
  btn.disabled=true;
  await post('/api/wifi/scan',{});
  setTimeout(pollScanResult,1000);
}
async function pollScanResult(){
  const r=await fetch('/api/wifi/scan/result');
  const d=await r.json();
  if(d.status==='scanning'){setTimeout(pollScanResult,1000);return;}
  const btn=document.getElementById('scan-btn');
  btn.textContent='Scan';btn.disabled=false;
  if(d.status==='complete'){scanNets=d.networks||[];showScanModal();}
  else{toast('Scan failed');scanNets=[];}
}
function openScanForSlot(i){scanSlot=i;if(scanNets.length>0)showScanModal();else scanWifi();}
function showScanModal(){
  const m=document.getElementById('scan-modal');
  const r=document.getElementById('scan-results');
  if(scanNets.length===0){r.innerHTML='<div style="padding:16px;text-align:center;color:var(--text-muted)">No networks found</div>';}
  else{r.innerHTML=scanNets.map(n=>`<div class="scan-item" onclick="selectSsid('${n.ssid.replace(/'/g,"\\'")}')"><div><div class="ssid">${n.ssid}</div><div class="info">${n.secure?'🔒':'🔓'} Ch${n.channel}</div></div><div class="info">${n.rssi} dBm</div></div>`).join('');}
  m.style.display='flex';
}
function closeScanModal(){document.getElementById('scan-modal').style.display='none';}
function selectSsid(ssid){
  if(scanSlot>=0&&scanSlot<6){document.getElementById('w-ssid'+scanSlot).value=ssid;document.getElementById('w-pass'+scanSlot).focus();}
  closeScanModal();
}
function showTab(n){
  document.querySelectorAll('.tab').forEach(t=>t.classList.remove('active'));
  document.querySelectorAll('.tab-content').forEach(c=>c.classList.remove('active'));
  document.querySelector(`[data-tab="${n}"]`).classList.add('active');
  document.getElementById(`tab-${n}`).classList.add('active');
  // Fire custom event for type-specific tabs
  document.dispatchEvent(new CustomEvent('tabchange',{detail:n}));
}
async function saveNetwork(){
  const wifi=[];
  for(let i=0;i<6;i++){
    wifi.push({ssid:document.getElementById('w-ssid'+i).value,password:document.getElementById('w-pass'+i).value});
  }
  await post('/api/common/network',{wifi,hostname:document.getElementById('n-hostname').value,ntpServer:document.getElementById('n-ntp').value});
  toast('Network settings saved');
}
async function saveAP(){
  await post('/api/common/ap',{ssid:document.getElementById('ap-ssid').value,password:document.getElementById('ap-pass').value,timeout:parseInt(document.getElementById('ap-timeout').value)});
  toast('AP settings saved');
}
async function saveCloud(){
  await post('/api/common/cloud',{gateUrl:document.getElementById('c-gate').value,stateReportUrl:document.getElementById('c-state').value,relayPrimary:document.getElementById('c-relay1').value,relaySecondary:document.getElementById('c-relay2').value});
  toast('Cloud settings saved');
}
async function saveTenant(){
  await post('/api/common/tenant',{tid:document.getElementById('t-tid').value,fid:document.getElementById('t-fid').value,lacisId:document.getElementById('t-lacis').value,email:document.getElementById('t-email').value,cic:document.getElementById('t-cic').value});
  toast('Tenant settings saved');
}
async function saveSystem(){
  await post('/api/system/settings',{deviceName:document.getElementById('s-device-name').value,uiPassword:document.getElementById('s-pass').value,rebootEnabled:document.getElementById('s-reboot-en').checked,rebootTime:document.getElementById('s-reboot-time').value});
  toast('System settings saved');
  // Update banner display
  const dn=document.getElementById('s-device-name').value;
  document.getElementById('device-name-display').innerHTML=dn||'<span class="unnamed">(未設定)</span>';
}
async function reboot(){if(confirm('Reboot device?')){await post('/api/system/reboot',{});alert('Rebooting...')}}
async function factoryReset(){if(confirm('Factory reset? All settings will be lost.')){await post('/api/system/factory-reset',{});alert('Resetting...')}}
async function post(u,d){await fetch(u,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(d)})}
function toast(m){const t=document.getElementById('toast');t.textContent=m;t.style.display='block';setTimeout(()=>t.style.display='none',2500)}
async function refreshStatus(){
  try{
    const r=await fetch('/api/status');
    const s=await r.json();
    document.getElementById('s-ip').textContent=s.network?.ip||'-';
    document.getElementById('s-ssid').textContent=s.network?.ssid||'-';
    document.getElementById('s-rssi').textContent=(s.network?.rssi||0)+' dBm';
    document.getElementById('s-ntp').textContent=s.system?.ntpTime||'-';
    document.getElementById('s-uptime').textContent=s.system?.uptimeHuman||'-';
    document.getElementById('s-heap').textContent=((s.system?.heap||0)/1024).toFixed(1)+' KB';
    document.getElementById('s-temp').textContent=(s.system?.chipTemp||0).toFixed(1)+' °C';
    if(typeof refreshTypeSpecificStatus==='function')refreshTypeSpecificStatus(s);
  }catch(e){}
}
window.onload=()=>{load();refreshStatus();setInterval(refreshStatus,5000)};
)JS";

  // 派生クラスの追加JS（</script>タグの前に挿入）
  js += generateTypeSpecificJS();

  js += R"JS(
</script>
)JS";

  return js;
}

// ========================================
// HTML生成
// ========================================
String AraneaWebUI::generateHTML() {
  String html = R"HTML(<!DOCTYPE html>
<html><head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Aranea Device Manager</title>
)HTML";

  html += generateCSS();
  html += R"HTML(</head><body>
<div class="header">
<div class="logo">)HTML";
  html += generateLogoSVG();
  html += R"HTML(</div>
<h1>Aranea Device Manager</h1>
<span class="version">)HTML";
  html += deviceInfo_.deviceType + " v" + deviceInfo_.firmwareVersion;
  html += R"HTML(</span>
</div>
<div class="device-banner">
<div class="device-banner-inner">
<div class="device-model">)HTML";
  html += deviceInfo_.deviceType + " " + deviceInfo_.modelName;
  html += R"HTML(</div>
<div class="device-name" id="device-name-display">)HTML";
  // deviceNameはsettingsから取得
  String deviceName = settings_->getString("device_name", "");
  if (deviceName.length() > 0) {
    html += deviceName;
  } else {
    html += "<span class=\"unnamed\">(未設定)</span>";
  }
  html += R"HTML(</div>
<div class="device-context">)HTML";
  html += deviceInfo_.contextDesc;
  html += R"HTML(</div>
</div>
</div>
<div class="container">
<div class="tabs">
<div class="tab active" data-tab="status" onclick="showTab('status')">Status</div>
<div class="tab" data-tab="network" onclick="showTab('network')">Network</div>
<div class="tab" data-tab="cloud" onclick="showTab('cloud')">Cloud</div>
<div class="tab" data-tab="tenant" onclick="showTab('tenant')">Tenant</div>
<div class="tab" data-tab="system" onclick="showTab('system')">System</div>
)HTML";

  // デバイス固有タブ
  html += generateTypeSpecificTabs();

  html += R"HTML(</div>

<!-- Status Tab -->
<div id="tab-status" class="tab-content active">
<div class="card">
<div class="card-title">Device Information</div>
<div class="status-grid">
<div class="status-item"><div class="label">Type</div><div class="value" id="d-type">-</div></div>
<div class="status-item"><div class="label">LacisID</div><div class="value" id="d-lacis">-</div></div>
<div class="status-item"><div class="label">CIC</div><div class="value" id="d-cic">-</div></div>
<div class="status-item"><div class="label">MAC</div><div class="value" id="d-mac">-</div></div>
<div class="status-item"><div class="label">Hostname</div><div class="value" id="d-host">-</div></div>
<div class="status-item"><div class="label">Version</div><div class="value" id="d-ver">-</div></div>
</div>
</div>
<div class="card">
<div class="card-title">Live Status</div>
<div class="status-grid">
<div class="status-item"><div class="label">IP Address</div><div class="value" id="s-ip">-</div></div>
<div class="status-item"><div class="label">SSID</div><div class="value" id="s-ssid">-</div></div>
<div class="status-item"><div class="label">RSSI</div><div class="value" id="s-rssi">-</div></div>
<div class="status-item"><div class="label">NTP Time</div><div class="value" id="s-ntp">-</div></div>
<div class="status-item"><div class="label">Uptime</div><div class="value" id="s-uptime">-</div></div>
<div class="status-item"><div class="label">Free Heap</div><div class="value" id="s-heap">-</div></div>
<div class="status-item"><div class="label">Chip Temp</div><div class="value" id="s-temp">-</div></div>
</div>
</div>
</div>

<!-- Network Tab -->
<div id="tab-network" class="tab-content">
<div class="card">
<div class="card-title" style="display:flex;justify-content:space-between;align-items:center">WiFi Settings (STA Mode)<button class="btn btn-sm" onclick="scanWifi()" id="scan-btn">Scan</button></div>
<div id="wifi-list"></div>
<div class="form-row" style="margin-top:12px">
<div class="form-group"><label>Hostname</label><input type="text" id="n-hostname"></div>
<div class="form-group"><label>NTP Server</label><input type="text" id="n-ntp"></div>
</div>
<button class="btn btn-primary" onclick="saveNetwork()">Save Network</button>
</div>
<!-- WiFi Scan Modal -->
<div id="scan-modal" class="modal" style="display:none">
<div class="modal-content">
<div class="modal-header"><span>Available Networks</span><button onclick="closeScanModal()">&times;</button></div>
<div id="scan-results" style="max-height:300px;overflow-y:auto"></div>
</div>
</div>
<div class="card">
<div class="card-title">AP Mode Settings</div>
<div class="form-row">
<div class="form-group"><label>AP SSID</label><input type="text" id="ap-ssid"></div>
<div class="form-group"><label>AP Password</label><input type="password" id="ap-pass" placeholder="(open if empty)"></div>
<div class="form-group"><label>Timeout (sec)</label><input type="number" id="ap-timeout" min="0"></div>
</div>
<button class="btn btn-primary" onclick="saveAP()">Save AP</button>
</div>
</div>

<!-- Cloud Tab -->
<div id="tab-cloud" class="tab-content">
<div class="card">
<div class="card-title">AraneaDeviceGate</div>
<div class="form-group"><label>Gate URL</label><input type="text" id="c-gate" placeholder="https://...araneaDeviceGate"></div>
<div class="form-group"><label>State Report URL</label><input type="text" id="c-state" placeholder="https://...deviceStateReport"></div>
</div>
<div class="card">
<div class="card-title">Relay Endpoints (Zero3)</div>
<div class="form-row">
<div class="form-group"><label>Primary</label><input type="text" id="c-relay1"></div>
<div class="form-group"><label>Secondary</label><input type="text" id="c-relay2"></div>
</div>
<button class="btn btn-primary" onclick="saveCloud()">Save Cloud</button>
</div>
</div>

<!-- Tenant Tab -->
<div id="tab-tenant" class="tab-content">
<div class="card">
<div class="card-title">Tenant Authentication</div>
<div class="form-row">
<div class="form-group"><label>Tenant ID (TID)</label><input type="text" id="t-tid"></div>
<div class="form-group"><label>Facility ID (FID)</label><input type="text" id="t-fid"></div>
</div>
<div class="form-row">
<div class="form-group"><label>Primary LacisID</label><input type="text" id="t-lacis"></div>
<div class="form-group"><label>CIC</label><input type="text" id="t-cic"></div>
</div>
<div class="form-group"><label>Email</label><input type="email" id="t-email"></div>
<button class="btn btn-primary" onclick="saveTenant()">Save Tenant</button>
</div>
</div>

<!-- System Tab -->
<div id="tab-system" class="tab-content">
<div class="card">
<div class="card-title">Device Identity (LacisOath)</div>
<div class="form-group"><label>Device Name</label><input type="text" id="s-device-name" placeholder="例: 本社1F-ルーター監視"></div>
<p style="font-size:12px;color:var(--text-muted);margin-top:4px">この名前はMQTT/クラウドで userobject.name として使用されます</p>
</div>
<div class="card">
<div class="card-title">Security</div>
<div class="form-group"><label>UI Password (Basic Auth)</label><input type="password" id="s-pass" placeholder="(no auth if empty)"></div>
</div>
<div class="card">
<div class="card-title">Scheduled Reboot</div>
<div class="form-row">
<div class="form-group"><label><input type="checkbox" id="s-reboot-en"> Enable daily reboot</label></div>
<div class="form-group"><label>Time</label><input type="time" id="s-reboot-time" value="03:00"></div>
</div>
<button class="btn btn-primary" onclick="saveSystem()">Save System</button>
</div>
<div class="card">
<div class="card-title">Actions</div>
<div class="btn-group">
<button class="btn btn-primary" onclick="reboot()">Reboot Now</button>
<button class="btn btn-danger" onclick="factoryReset()">Factory Reset</button>
</div>
</div>
</div>
)HTML";

  // デバイス固有タブコンテンツ
  html += generateTypeSpecificTabContents();

  html += R"HTML(</div>
<div id="toast" class="toast"></div>
)HTML";

  html += generateJS();
  html += "</body></html>";

  return html;
}
