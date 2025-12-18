#include "SettingManager.h"

bool SettingManager::begin(const char* ns) {
  if (initialized_) return true;

  // 名前空間を保存
  strncpy(namespace_, ns, sizeof(namespace_) - 1);
  namespace_[sizeof(namespace_) - 1] = '\0';

  if (!prefs_.begin(ns, false)) {
    Serial.printf("[SETTING] Failed to open NVS namespace: %s\n", ns);
    return false;
  }

  initialized_ = true;
  Serial.printf("[SETTING] Initialized (namespace: %s)\n", ns);
  return true;
}

void SettingManager::end() {
  if (initialized_) {
    prefs_.end();
    initialized_ = false;
    Serial.println("[SETTING] Closed");
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

unsigned long SettingManager::getULong(const String& key, unsigned long defValue) {
  if (!initialized_) return defValue;
  return prefs_.getULong(key.c_str(), defValue);
}

void SettingManager::setULong(const String& key, unsigned long value) {
  if (!initialized_) return;
  prefs_.putULong(key.c_str(), value);
}

bool SettingManager::getBool(const String& key, bool defValue) {
  if (!initialized_) return defValue;
  return prefs_.getBool(key.c_str(), defValue);
}

void SettingManager::setBool(const String& key, bool value) {
  if (!initialized_) return;
  prefs_.putBool(key.c_str(), value);
}

bool SettingManager::hasKey(const String& key) {
  if (!initialized_) return false;
  return prefs_.isKey(key.c_str());
}

bool SettingManager::remove(const String& key) {
  if (!initialized_) return false;
  return prefs_.remove(key.c_str());
}

void SettingManager::clear() {
  if (!initialized_) return;
  prefs_.clear();
  Serial.printf("[SETTING] Cleared namespace: %s\n", namespace_);
}
