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

  // 認証成功でHTML UI表示
  server_->send(200, "text/html", generateHTML());
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
    settings_->setInt("ap_timeout", doc["timeout"]);
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

  if (doc.containsKey("gateUrl")) settings_->setString("gate_url", doc["gateUrl"]);
  if (doc.containsKey("stateReportUrl")) settings_->setString("state_report_url", doc["stateReportUrl"]);
  if (doc.containsKey("relayPrimary")) settings_->setString("relay_pri", doc["relayPrimary"]);
  if (doc.containsKey("relaySecondary")) settings_->setString("relay_sec", doc["relaySecondary"]);

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

  if (doc.containsKey("tid")) settings_->setString("tid", doc["tid"]);
  if (doc.containsKey("fid")) settings_->setString("fid", doc["fid"]);
  if (doc.containsKey("lacisId")) settings_->setString("tenant_lacisid", doc["lacisId"]);
  if (doc.containsKey("email")) settings_->setString("tenant_email", doc["email"]);
  if (doc.containsKey("cic")) settings_->setString("tenant_cic", doc["cic"]);

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
    settings_->setBool("reboot_enabled", doc["rebootEnabled"]);
  }
  if (doc.containsKey("rebootTime")) {
    settings_->setString("reboot_time", doc["rebootTime"]);
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
  server_->send(200, "application/json", "{\"ok\":true,\"message\":\"Rebooting...\"}");
  delay(500);
  if (rebootCallback_) rebootCallback_();
  ESP.restart();
}

// ========================================
// ファクトリーリセット
// ========================================
void AraneaWebUI::handleFactoryReset() {
  if (!checkAuth()) { requestAuth(); return; }
  // isms namespace クリア
  settings_->clear();

  // aranea namespace クリア
  Preferences araneaPrefs;
  araneaPrefs.begin("aranea", false);
  araneaPrefs.clear();
  araneaPrefs.end();

  // SPIFFS設定ファイル削除
  SPIFFS.remove("/routers.json");
  SPIFFS.remove("/aranea_settings.json");

  server_->send(200, "application/json", "{\"ok\":true,\"message\":\"Factory reset complete\"}");
  delay(500);
  ESP.restart();
}

// ========================================
// 404
// ========================================
void AraneaWebUI::handleNotFound() {
  server_->send(404, "application/json", "{\"ok\":false,\"error\":\"Not Found\"}");
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
function renderWifi(){
  const c=document.getElementById('wifi-list');
  c.innerHTML='';
  const w=cfg.network?.wifi||[];
  for(let i=0;i<6;i++){
    c.innerHTML+=`<div class="wifi-pair"><span class="num">${i+1}</span><div class="form-group" style="flex:2"><input type="text" id="w-ssid${i}" value="${w[i]?.ssid||''}" placeholder="SSID"></div><div class="form-group" style="flex:2"><input type="password" id="w-pass${i}" placeholder="Password"></div></div>`;
  }
}
function showTab(n){
  document.querySelectorAll('.tab').forEach(t=>t.classList.remove('active'));
  document.querySelectorAll('.tab-content').forEach(c=>c.classList.remove('active'));
  document.querySelector(`[data-tab="${n}"]`).classList.add('active');
  document.getElementById(`tab-${n}`).classList.add('active');
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
    if(typeof refreshTypeSpecificStatus==='function')refreshTypeSpecificStatus(s);
  }catch(e){}
}
window.onload=()=>{load();refreshStatus();setInterval(refreshStatus,5000)};
</script>
)JS";

  // 派生クラスの追加JS
  js += generateTypeSpecificJS();

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
</div>
</div>
</div>

<!-- Network Tab -->
<div id="tab-network" class="tab-content">
<div class="card">
<div class="card-title">WiFi Settings (STA Mode)</div>
<div id="wifi-list"></div>
<div class="form-row" style="margin-top:12px">
<div class="form-group"><label>Hostname</label><input type="text" id="n-hostname"></div>
<div class="form-group"><label>NTP Server</label><input type="text" id="n-ntp"></div>
</div>
<button class="btn btn-primary" onclick="saveNetwork()">Save Network</button>
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
