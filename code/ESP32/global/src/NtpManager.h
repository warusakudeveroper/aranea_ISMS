#pragma once
#include <Arduino.h>
#include <time.h>

/**
 * NtpManager
 *
 * NTP時刻同期
 * - WiFi接続後に同期
 * - 失敗しても動作は継続
 */
class NtpManager {
public:
  /**
   * NTP同期実行
   * @return 成功時true
   */
  bool sync();

  /**
   * 現在時刻をISO8601形式で取得
   * @return ISO8601文字列（例："2025-01-15T10:30:00Z"）
   */
  String getIso8601();

  /**
   * 現在時刻をepoch秒で取得
   * @return epoch秒（同期失敗時は0）
   */
  time_t getEpoch();

  /**
   * 同期済みか確認
   * @return 同期済みならtrue
   */
  bool isSynced();

private:
  bool synced_ = false;

  static const char* NTP_SERVER1;
  static const char* NTP_SERVER2;
  static const long GMT_OFFSET_SEC;
  static const int DAYLIGHT_OFFSET_SEC;
};
