#include "NtpManager.h"

const char* NtpManager::NTP_SERVER1 = "ntp.nict.jp";
const char* NtpManager::NTP_SERVER2 = "pool.ntp.org";
const long NtpManager::GMT_OFFSET_SEC = 0;  // UTC
const int NtpManager::DAYLIGHT_OFFSET_SEC = 0;

bool NtpManager::sync() {
  Serial.println("[NTP] Syncing...");

  configTime(GMT_OFFSET_SEC, DAYLIGHT_OFFSET_SEC, NTP_SERVER1, NTP_SERVER2);

  // 同期完了を待つ（最大10秒）
  int retries = 20;
  struct tm timeinfo;
  while (!getLocalTime(&timeinfo) && retries > 0) {
    delay(500);
    retries--;
  }

  if (retries == 0) {
    Serial.println("[NTP] Sync failed");
    synced_ = false;
    return false;
  }

  synced_ = true;
  Serial.printf("[NTP] Synced: %s\n", getIso8601().c_str());
  return true;
}

String NtpManager::getIso8601() {
  struct tm timeinfo;
  if (!getLocalTime(&timeinfo)) {
    return "1970-01-01T00:00:00Z";
  }

  char buf[25];
  strftime(buf, sizeof(buf), "%Y-%m-%dT%H:%M:%SZ", &timeinfo);
  return String(buf);
}

time_t NtpManager::getEpoch() {
  struct tm timeinfo;
  if (!getLocalTime(&timeinfo)) {
    return 0;
  }
  return mktime(&timeinfo);
}

bool NtpManager::isSynced() {
  return synced_;
}
