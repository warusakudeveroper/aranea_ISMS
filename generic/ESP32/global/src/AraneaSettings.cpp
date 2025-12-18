/**
 * AraneaSettings.cpp
 *
 * 汎用araneaDevice設定クラス実装
 */

#include "AraneaSettings.h"
#include <SPIFFS.h>
#include <ArduinoJson.h>

// 静的メンバ初期化
bool AraneaSettings::_initialized = false;
TenantAuth AraneaSettings::_tenantAuth;
WiFiConfig AraneaSettings::_wifiConfig;
String AraneaSettings::_gateUrl;
String AraneaSettings::_cloudUrl;
String AraneaSettings::_relayPrimary;
String AraneaSettings::_relaySecondary;
int AraneaSettings::_relayPort;

static const char* SETTINGS_FILE = "/aranea_settings.json";

void AraneaSettings::init() {
  if (_initialized) return;

  // デフォルト設定をロード
  resetToDefaults();

  // SPIFFSから既存設定を読み込み
  if (SPIFFS.begin(true)) {
    loadFromSPIFFS();
  }

  _initialized = true;
  Serial.println("[AraneaSettings] Initialized");
}

void AraneaSettings::resetToDefaults() {
  // テナント設定
  _tenantAuth.tid = ARANEA_DEFAULT_TID;
  _tenantAuth.fid = ARANEA_DEFAULT_FID;
  _tenantAuth.lacisId = ARANEA_DEFAULT_TENANT_LACISID;
  _tenantAuth.email = ARANEA_DEFAULT_TENANT_EMAIL;
  _tenantAuth.cic = ARANEA_DEFAULT_TENANT_CIC;
  _tenantAuth.pass = ARANEA_DEFAULT_TENANT_PASS;

  // WiFi設定（ペア形式）
  _wifiConfig.pairs[0].ssid = ARANEA_DEFAULT_WIFI_SSID_1;
  _wifiConfig.pairs[0].password = ARANEA_DEFAULT_WIFI_PASS;
  _wifiConfig.pairs[1].ssid = ARANEA_DEFAULT_WIFI_SSID_2;
  _wifiConfig.pairs[1].password = ARANEA_DEFAULT_WIFI_PASS;
  _wifiConfig.pairs[2].ssid = ARANEA_DEFAULT_WIFI_SSID_3;
  _wifiConfig.pairs[2].password = ARANEA_DEFAULT_WIFI_PASS;
  _wifiConfig.pairs[3].ssid = ARANEA_DEFAULT_WIFI_SSID_4;
  _wifiConfig.pairs[3].password = ARANEA_DEFAULT_WIFI_PASS;
  _wifiConfig.pairs[4].ssid = ARANEA_DEFAULT_WIFI_SSID_5;
  _wifiConfig.pairs[4].password = ARANEA_DEFAULT_WIFI_PASS;
  _wifiConfig.pairs[5].ssid = ARANEA_DEFAULT_WIFI_SSID_6;
  _wifiConfig.pairs[5].password = ARANEA_DEFAULT_WIFI_PASS;

  // エンドポイント
  _gateUrl = ARANEA_DEFAULT_GATE_URL;
  _cloudUrl = ARANEA_DEFAULT_CLOUD_URL;

  // Relay
  _relayPrimary = ARANEA_DEFAULT_RELAY_PRIMARY;
  _relaySecondary = ARANEA_DEFAULT_RELAY_SECONDARY;
  _relayPort = ARANEA_DEFAULT_RELAY_PORT;

  Serial.println("[AraneaSettings] Reset to defaults");
}

bool AraneaSettings::loadFromSPIFFS() {
  if (!SPIFFS.exists(SETTINGS_FILE)) {
    Serial.println("[AraneaSettings] No saved settings file");
    return false;
  }

  File file = SPIFFS.open(SETTINGS_FILE, "r");
  if (!file) {
    Serial.println("[AraneaSettings] Failed to open settings file");
    return false;
  }

  String json = file.readString();
  file.close();

  return fromJson(json);
}

bool AraneaSettings::saveToSPIFFS() {
  File file = SPIFFS.open(SETTINGS_FILE, "w");
  if (!file) {
    Serial.println("[AraneaSettings] Failed to create settings file");
    return false;
  }

  String json = toJson();
  file.print(json);
  file.close();

  Serial.println("[AraneaSettings] Saved to SPIFFS");
  return true;
}

// Getters
String AraneaSettings::getTid() { return _tenantAuth.tid; }
String AraneaSettings::getFid() { return _tenantAuth.fid; }
TenantAuth AraneaSettings::getTenantAuth() { return _tenantAuth; }
WiFiConfig AraneaSettings::getWiFiConfig() { return _wifiConfig; }
String AraneaSettings::getGateUrl() { return _gateUrl; }
String AraneaSettings::getCloudUrl() { return _cloudUrl; }
String AraneaSettings::getRelayPrimary() { return _relayPrimary; }
String AraneaSettings::getRelaySecondary() { return _relaySecondary; }
int AraneaSettings::getRelayPort() { return _relayPort; }

// Setters
void AraneaSettings::setTid(const String& tid) { _tenantAuth.tid = tid; }
void AraneaSettings::setFid(const String& fid) { _tenantAuth.fid = fid; }

void AraneaSettings::setTenantAuth(const TenantAuth& auth) {
  _tenantAuth = auth;
}

void AraneaSettings::setWiFiConfig(const WiFiConfig& config) {
  _wifiConfig = config;
}

void AraneaSettings::setWiFiSSID(int index, const String& ssid) {
  if (index >= 0 && index < 6) {
    _wifiConfig.pairs[index].ssid = ssid;
  }
}

void AraneaSettings::setWiFiPassword(int index, const String& pass) {
  if (index >= 0 && index < 6) {
    _wifiConfig.pairs[index].password = pass;
  }
}

void AraneaSettings::setWiFiPair(int index, const String& ssid, const String& pass) {
  if (index >= 0 && index < 6) {
    _wifiConfig.pairs[index].ssid = ssid;
    _wifiConfig.pairs[index].password = pass;
  }
}

void AraneaSettings::setGateUrl(const String& url) { _gateUrl = url; }
void AraneaSettings::setCloudUrl(const String& url) { _cloudUrl = url; }

String AraneaSettings::toJson() {
  StaticJsonDocument<2048> doc;

  // Tenant
  JsonObject tenant = doc.createNestedObject("tenant");
  tenant["tid"] = _tenantAuth.tid;
  tenant["fid"] = _tenantAuth.fid;
  tenant["lacisId"] = _tenantAuth.lacisId;
  tenant["email"] = _tenantAuth.email;
  tenant["cic"] = _tenantAuth.cic;
  tenant["pass"] = _tenantAuth.pass;

  // WiFi（ペア形式）
  JsonArray wifiPairs = doc.createNestedArray("wifi");
  for (int i = 0; i < 6; i++) {
    JsonObject pair = wifiPairs.createNestedObject();
    pair["ssid"] = _wifiConfig.pairs[i].ssid;
    pair["password"] = _wifiConfig.pairs[i].password;
  }

  // Endpoints
  JsonObject endpoints = doc.createNestedObject("endpoints");
  endpoints["gate"] = _gateUrl;
  endpoints["cloud"] = _cloudUrl;

  // Relay
  JsonObject relay = doc.createNestedObject("relay");
  relay["primary"] = _relayPrimary;
  relay["secondary"] = _relaySecondary;
  relay["port"] = _relayPort;

  String output;
  serializeJson(doc, output);
  return output;
}

bool AraneaSettings::fromJson(const String& json) {
  StaticJsonDocument<2048> doc;
  DeserializationError error = deserializeJson(doc, json);

  if (error) {
    Serial.printf("[AraneaSettings] JSON parse error: %s\n", error.c_str());
    return false;
  }

  // Tenant
  if (doc.containsKey("tenant")) {
    JsonObject tenant = doc["tenant"];
    if (tenant.containsKey("tid")) _tenantAuth.tid = tenant["tid"].as<String>();
    if (tenant.containsKey("fid")) _tenantAuth.fid = tenant["fid"].as<String>();
    if (tenant.containsKey("lacisId")) _tenantAuth.lacisId = tenant["lacisId"].as<String>();
    if (tenant.containsKey("email")) _tenantAuth.email = tenant["email"].as<String>();
    if (tenant.containsKey("cic")) _tenantAuth.cic = tenant["cic"].as<String>();
    if (tenant.containsKey("pass")) _tenantAuth.pass = tenant["pass"].as<String>();
  }

  // WiFi（ペア形式）
  if (doc.containsKey("wifi")) {
    JsonArray wifiPairs = doc["wifi"].as<JsonArray>();
    for (int i = 0; i < 6 && i < (int)wifiPairs.size(); i++) {
      JsonObject pair = wifiPairs[i];
      if (pair.containsKey("ssid")) _wifiConfig.pairs[i].ssid = pair["ssid"].as<String>();
      if (pair.containsKey("password")) _wifiConfig.pairs[i].password = pair["password"].as<String>();
    }
  }

  // Endpoints
  if (doc.containsKey("endpoints")) {
    JsonObject endpoints = doc["endpoints"];
    if (endpoints.containsKey("gate")) _gateUrl = endpoints["gate"].as<String>();
    if (endpoints.containsKey("cloud")) _cloudUrl = endpoints["cloud"].as<String>();
  }

  // Relay
  if (doc.containsKey("relay")) {
    JsonObject relay = doc["relay"];
    if (relay.containsKey("primary")) _relayPrimary = relay["primary"].as<String>();
    if (relay.containsKey("secondary")) _relaySecondary = relay["secondary"].as<String>();
    if (relay.containsKey("port")) _relayPort = relay["port"].as<int>();
  }

  Serial.println("[AraneaSettings] Loaded from JSON");
  return true;
}
