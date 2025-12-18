#include "WiFiManager.h"
#include "SettingManager.h"

// デフォルトSSIDリスト
const char* WiFiManager::SSID_LIST[SSID_COUNT] = {
  "cluster1", "cluster2", "cluster3",
  "cluster4", "cluster5", "cluster6"
};

// デフォルトパスワード
const char* WiFiManager::WIFI_PASS = "isms12345@";

void WiFiManager::setHostname(const String& hostname) {
  hostname_ = hostname;
  Serial.printf("[WIFI] Hostname set: %s\n", hostname_.c_str());
}

void WiFiManager::applyHostname() {
  if (hostname_.length() > 0) {
    WiFi.setHostname(hostname_.c_str());
    Serial.printf("[WIFI] Applied hostname: %s\n", hostname_.c_str());
  }
}

bool WiFiManager::connectDefault() {
  WiFi.mode(WIFI_STA);
  WiFi.setAutoReconnect(true);

  // 無限ループで接続を試行（接続できるまで回す）
  while (true) {
    for (int i = 0; i < SSID_COUNT; i++) {
      Serial.printf("[WIFI] Trying %s...\n", SSID_LIST[i]);

      if (connect(SSID_LIST[i], WIFI_PASS, 15000)) {
        Serial.printf("[WIFI] Connected to %s\n", SSID_LIST[i]);
        Serial.printf("[WIFI] IP: %s\n", getIP().c_str());
        return true;
      }

      Serial.printf("[WIFI] Failed to connect to %s\n", SSID_LIST[i]);
    }

    Serial.println("[WIFI] All SSIDs failed, retrying...");
    delay(1000);
  }

  return false;  // ここには来ない
}

bool WiFiManager::connectWithSettings(SettingManager* settings) {
  WiFi.mode(WIFI_STA);
  WiFi.setAutoReconnect(true);

  // 設定からリトライ上限を取得
  retryLimit_ = settings ? settings->getInt("wifi_retry_limit", 30) : 30;
  failCount_ = 0;

  // NVSからカスタムSSID/パスワードを読み取る配列
  String ssidList[SSID_COUNT];
  String passList[SSID_COUNT];

  for (int i = 0; i < SSID_COUNT; i++) {
    String ssidKey = "wifi_ssid" + String(i + 1);
    String passKey = "wifi_pass" + String(i + 1);

    if (settings) {
      // NVSに設定があればそれを使用、なければデフォルト
      ssidList[i] = settings->getString(ssidKey.c_str(), SSID_LIST[i]);
      passList[i] = settings->getString(passKey.c_str(), WIFI_PASS);
    } else {
      ssidList[i] = SSID_LIST[i];
      passList[i] = WIFI_PASS;
    }
  }

  Serial.printf("[WIFI] Retry limit: %d\n", retryLimit_);

  // 接続試行ループ
  while (true) {
    for (int i = 0; i < SSID_COUNT; i++) {
      // 空のSSIDはスキップ
      if (ssidList[i].length() == 0) continue;

      Serial.printf("[WIFI] Trying %s... (fail count: %d/%d)\n",
                    ssidList[i].c_str(), failCount_, retryLimit_);

      if (connect(ssidList[i].c_str(), passList[i].c_str(), 15000)) {
        Serial.printf("[WIFI] Connected to %s\n", ssidList[i].c_str());
        Serial.printf("[WIFI] IP: %s\n", getIP().c_str());
        failCount_ = 0;  // 接続成功でリセット
        return true;
      }

      failCount_++;
      Serial.printf("[WIFI] Failed to connect to %s (fail count: %d)\n",
                    ssidList[i].c_str(), failCount_);

      // リトライ上限チェック
      if (failCount_ >= retryLimit_) {
        Serial.printf("[WIFI] Retry limit reached (%d). Rebooting...\n", retryLimit_);
        delay(1000);
        ESP.restart();
      }
    }

    Serial.println("[WIFI] All SSIDs failed, retrying...");
    delay(1000);
  }

  return false;  // ここには来ない
}

bool WiFiManager::connect(const char* ssid, const char* pass, unsigned long timeoutMs) {
  WiFi.disconnect(true);
  delay(100);

  // ホスト名を適用（WiFi.begin前に必要）
  applyHostname();

  WiFi.begin(ssid, pass);

  unsigned long startTime = millis();
  while (WiFi.status() != WL_CONNECTED) {
    if (millis() - startTime > timeoutMs) {
      WiFi.disconnect(true);
      return false;
    }
    delay(500);
    Serial.print(".");
  }
  Serial.println();

  return true;
}

String WiFiManager::getIP() {
  if (!isConnected()) return "0.0.0.0";
  return WiFi.localIP().toString();
}

int WiFiManager::getRSSI() {
  if (!isConnected()) return 0;
  return WiFi.RSSI();
}

void WiFiManager::disconnect() {
  WiFi.disconnect(true);
}

bool WiFiManager::isConnected() {
  return WiFi.status() == WL_CONNECTED;
}
