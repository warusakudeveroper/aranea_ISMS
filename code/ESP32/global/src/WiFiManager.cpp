#include "WiFiManager.h"

// デフォルトSSIDリスト
const char* WiFiManager::SSID_LIST[SSID_COUNT] = {
  "cluster1", "cluster2", "cluster3",
  "cluster4", "cluster5", "cluster6"
};

// デフォルトパスワード
const char* WiFiManager::WIFI_PASS = "isms12345@";

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

bool WiFiManager::connect(const char* ssid, const char* pass, unsigned long timeoutMs) {
  WiFi.disconnect(true);
  delay(100);

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
