#include "SettingManager.h"

// NVSの名前空間
static const char* NVS_NAMESPACE = "isms";

// デフォルト値 - リレーエンドポイント
static const char* DEFAULT_RELAY_PRIMARY = "http://192.168.96.201:8080/api/events";
static const char* DEFAULT_RELAY_SECONDARY = "http://192.168.96.202:8080/api/events";

// デフォルト値 - araneaDeviceGate（us-central1-mobesorder）
static const char* DEFAULT_GATE_URL = "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate";

// デフォルト値 - テナント情報（市山水産株式会社）
static const char* DEFAULT_TID = "T2025120608261484221";
static const char* DEFAULT_TENANT_LACISID = "12767487939173857894";
static const char* DEFAULT_TENANT_EMAIL = "info+ichiyama@neki.tech";
static const char* DEFAULT_TENANT_CIC = "263238";
static const char* DEFAULT_TENANT_PASS = "dJBU^TpG%j$5";

// デフォルト値 - is05チャンネル設定
// GPIO: ch1=19, ch2=18, ch3=5, ch4=17, ch5=16, ch6=4, ch7=2, ch8=15
static const int DEFAULT_IS05_PINS[8] = {19, 18, 5, 17, 16, 4, 2, 15};
static const char* DEFAULT_IS05_NAMES[8] = {"ch1", "ch2", "ch3", "ch4", "ch5", "ch6", "ch7", "ch8"};
static const char* DEFAULT_IS05_MEANING = "open";
static const char* DEFAULT_IS05_DID = "00000000";

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

  // araneaDeviceGate URL
  if (!hasKey("gate_url")) {
    setString("gate_url", DEFAULT_GATE_URL);
    Serial.println("[SETTING] Set default gate_url");
  }

  // テナント情報
  if (!hasKey("tid")) {
    setString("tid", DEFAULT_TID);
    Serial.println("[SETTING] Set default tid");
  }
  if (!hasKey("tenant_lacisid")) {
    setString("tenant_lacisid", DEFAULT_TENANT_LACISID);
    Serial.println("[SETTING] Set default tenant_lacisid");
  }
  if (!hasKey("tenant_email")) {
    setString("tenant_email", DEFAULT_TENANT_EMAIL);
    Serial.println("[SETTING] Set default tenant_email");
  }
  if (!hasKey("tenant_cic")) {
    setString("tenant_cic", DEFAULT_TENANT_CIC);
    Serial.println("[SETTING] Set default tenant_cic");
  }
  if (!hasKey("tenant_pass")) {
    setString("tenant_pass", DEFAULT_TENANT_PASS);
    Serial.println("[SETTING] Set default tenant_pass");
  }

  // is05チャンネル設定（ch1〜ch8）
  for (int i = 1; i <= 8; i++) {
    String pinKey = "is05_ch" + String(i) + "_pin";
    String nameKey = "is05_ch" + String(i) + "_name";
    String meaningKey = "is05_ch" + String(i) + "_meaning";
    String didKey = "is05_ch" + String(i) + "_did";

    if (!hasKey(pinKey)) {
      setInt(pinKey, DEFAULT_IS05_PINS[i - 1]);
    }
    if (!hasKey(nameKey)) {
      setString(nameKey, DEFAULT_IS05_NAMES[i - 1]);
    }
    if (!hasKey(meaningKey)) {
      setString(meaningKey, DEFAULT_IS05_MEANING);
    }
    if (!hasKey(didKey)) {
      setString(didKey, DEFAULT_IS05_DID);
    }
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
  initDefaults();  // クリア後にデフォルト値を再投入
}
