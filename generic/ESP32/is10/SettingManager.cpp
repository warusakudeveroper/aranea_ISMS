#include "SettingManager.h"
#include "AraneaSettings.h"

// NVSの名前空間
static const char* NVS_NAMESPACE = "isms";

// デフォルト値 - リレーエンドポイント（フルURL形式）
static const char* DEFAULT_RELAY_PRIMARY = "http://" ARANEA_DEFAULT_RELAY_PRIMARY ":8080/api/events";
static const char* DEFAULT_RELAY_SECONDARY = "http://" ARANEA_DEFAULT_RELAY_SECONDARY ":8080/api/events";

// デフォルト値 - araneaDeviceGate URL（AraneaSettings.hから）
#define DEFAULT_GATE_URL        ARANEA_DEFAULT_GATE_URL

// デフォルト値 - テナント情報（AraneaSettings.hから）
#define DEFAULT_TID             ARANEA_DEFAULT_TID
#define DEFAULT_TENANT_LACISID  ARANEA_DEFAULT_TENANT_LACISID
#define DEFAULT_TENANT_EMAIL    ARANEA_DEFAULT_TENANT_EMAIL
#define DEFAULT_TENANT_CIC      ARANEA_DEFAULT_TENANT_CIC
// TENANT_PASSは廃止（認証はlacisId + userId + cicの3要素）

// デフォルト値 - is05チャンネル設定
// GPIO: ch1=19, ch2=18, ch3=5, ch4=17, ch5=16, ch6=4, ch7=2, ch8=15
static const int DEFAULT_IS05_PINS[8] = {19, 18, 5, 17, 16, 4, 2, 15};
static const char* DEFAULT_IS05_NAMES[8] = {"ch1", "ch2", "ch3", "ch4", "ch5", "ch6", "ch7", "ch8"};
static const char* DEFAULT_IS05_MEANING = "open";
static const char* DEFAULT_IS05_DID = "00000000";

// デフォルト値 - is04設定
static const char* DEFAULT_IS04_DIDS = "00001234,00005678";
static const int DEFAULT_IS04_OPEN_PIN = 12;           // OPEN出力ピン (12 or 14)
static const char* DEFAULT_IS04_PHYS1_ACTION = "open"; // 物理入力1のアクション
static const char* DEFAULT_IS04_PHYS2_ACTION = "close";// 物理入力2のアクション
static const int DEFAULT_IS04_PULSE_MS = 3000;
static const int DEFAULT_IS04_INTERLOCK_MS = 200;

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
  // tenant_passは廃止（認証はlacisId + userId + cicの3要素）

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

  // is04設定
  if (!hasKey("is04_dids")) {
    setString("is04_dids", DEFAULT_IS04_DIDS);
    Serial.println("[SETTING] Set default is04_dids");
  }
  if (!hasKey("is04_open_pin")) {
    setInt("is04_open_pin", DEFAULT_IS04_OPEN_PIN);
    Serial.println("[SETTING] Set default is04_open_pin");
  }
  if (!hasKey("is04_phys1_action")) {
    setString("is04_phys1_action", DEFAULT_IS04_PHYS1_ACTION);
    Serial.println("[SETTING] Set default is04_phys1_action");
  }
  if (!hasKey("is04_phys2_action")) {
    setString("is04_phys2_action", DEFAULT_IS04_PHYS2_ACTION);
    Serial.println("[SETTING] Set default is04_phys2_action");
  }
  if (!hasKey("is04_pulse_ms")) {
    setInt("is04_pulse_ms", DEFAULT_IS04_PULSE_MS);
    Serial.println("[SETTING] Set default is04_pulse_ms");
  }
  if (!hasKey("is04_interlock_ms")) {
    setInt("is04_interlock_ms", DEFAULT_IS04_INTERLOCK_MS);
    Serial.println("[SETTING] Set default is04_interlock_ms");
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
