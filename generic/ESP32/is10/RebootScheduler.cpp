#include "RebootScheduler.h"
#include <time.h>

void RebootScheduler::begin(int rebootHour) {
  setRebootHour(rebootHour);
  rebootedToday_ = false;
  lastCheckedHour_ = -1;
  Serial.printf("[REBOOT] Scheduler initialized, hour=%d\n", rebootHour_);
}

void RebootScheduler::setRebootHour(int hour) {
  if (hour >= -1 && hour <= 23) {
    rebootHour_ = hour;
  }
}

void RebootScheduler::check() {
  if (!isEnabled()) return;

  // 現在時刻を取得
  time_t now = time(nullptr);
  struct tm* tm = localtime(&now);

  if (tm == nullptr || tm->tm_year < 100) {
    // 時刻未同期
    return;
  }

  int currentHour = tm->tm_hour;

  // 日付が変わったらフラグリセット
  if (currentHour < lastCheckedHour_) {
    rebootedToday_ = false;
  }
  lastCheckedHour_ = currentHour;

  // 再起動時刻チェック
  if (!rebootedToday_ && currentHour == rebootHour_) {
    Serial.printf("[REBOOT] Scheduled reboot at %02d:00\n", rebootHour_);
    rebootedToday_ = true;
    delay(1000);
    ESP.restart();
  }
}

int RebootScheduler::secondsUntilReboot() {
  if (!isEnabled()) return -1;

  time_t now = time(nullptr);
  struct tm* tm = localtime(&now);

  if (tm == nullptr || tm->tm_year < 100) {
    return -1;
  }

  int currentHour = tm->tm_hour;
  int currentMin = tm->tm_min;
  int currentSec = tm->tm_sec;

  int hoursUntil;
  if (currentHour < rebootHour_) {
    hoursUntil = rebootHour_ - currentHour;
  } else if (currentHour == rebootHour_) {
    // 今日の再起動時刻中
    return 0;
  } else {
    // 明日の再起動時刻まで
    hoursUntil = 24 - currentHour + rebootHour_;
  }

  return hoursUntil * 3600 - currentMin * 60 - currentSec;
}
