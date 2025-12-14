#include "SettingManager.h"

// NVSの名前空間
static const char* NVS_NAMESPACE = "isms";

// デフォルト値
static const char* DEFAULT_RELAY_PRIMARY = "http://192.168.96.201:8080/api/ingest";
static const char* DEFAULT_RELAY_SECONDARY = "http://192.168.96.202:8080/api/ingest";

bool SettingManager::begin() {
  if (initialized_) return true;

  if (!prefs_.begin(NVS_NAMESPACE, false)) {
    Serial.println("[SETTING] Failed to open NVS");
    return false;
  }

  initialized_ = true;
  initDefaults();
  Serial.println("[SETTING] Initialized");
  return true;
}

void SettingManager::initDefaults() {
  // relay.primary が未設定ならデフォルトを投入
  if (!hasKey("relay_pri")) {
    setString("relay_pri", DEFAULT_RELAY_PRIMARY);
    Serial.println("[SETTING] Set default relay.primary");
  }

  // relay.secondary が未設定ならデフォルトを投入
  if (!hasKey("relay_sec")) {
    setString("relay_sec", DEFAULT_RELAY_SECONDARY);
    Serial.println("[SETTING] Set default relay.secondary");
  }
}

String SettingManager::getString(const String& key, const String& defValue) {
  if (!initialized_) return defValue;
  return prefs_.getString(key.c_str(), defValue);
}

void SettingManager::setString(const String& key, const String& value) {
  if (!initialized_) return;
  prefs_.putString(key.c_str(), value);
}

int SettingManager::getInt(const String& key, int defValue) {
  if (!initialized_) return defValue;
  return prefs_.getInt(key.c_str(), defValue);
}

void SettingManager::setInt(const String& key, int value) {
  if (!initialized_) return;
  prefs_.putInt(key.c_str(), value);
}

bool SettingManager::hasKey(const String& key) {
  if (!initialized_) return false;
  return prefs_.isKey(key.c_str());
}

void SettingManager::clear() {
  if (!initialized_) return;
  prefs_.clear();
  Serial.println("[SETTING] All settings cleared");
}
