#pragma once
#include <Arduino.h>

/**
 * RebootScheduler
 *
 * 定期再起動スケジューラ
 * - 指定時刻（時）に毎日再起動
 * - NTPで時刻同期後に動作
 */
class RebootScheduler {
public:
  /**
   * 初期化
   * @param rebootHour 再起動時刻（0-23、-1で無効）
   */
  void begin(int rebootHour = -1);

  /**
   * 再起動時刻設定
   * @param hour 再起動時刻（0-23、-1で無効）
   */
  void setRebootHour(int hour);

  /**
   * 再起動時刻取得
   */
  int getRebootHour() const { return rebootHour_; }

  /**
   * チェック処理（loop()で呼び出し）
   * 指定時刻になったら再起動実行
   */
  void check();

  /**
   * 有効かどうか
   */
  bool isEnabled() const { return rebootHour_ >= 0 && rebootHour_ <= 23; }

  /**
   * 次回再起動までの秒数（概算）
   */
  int secondsUntilReboot();

private:
  int rebootHour_ = -1;
  bool rebootedToday_ = false;
  int lastCheckedHour_ = -1;
};
