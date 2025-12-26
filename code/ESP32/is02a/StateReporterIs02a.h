/**
 * StateReporterIs02a.h
 *
 * is02a専用 状態レポート構築・送信モジュール
 *
 * Gateway + Nodesペイロード構築とMQTT/HTTP送信を管理。
 */

#ifndef STATE_REPORTER_IS02A_H
#define STATE_REPORTER_IS02A_H

#include <Arduino.h>
#include <vector>
#include "BleReceiver.h"

class SettingManager;
class NtpManager;

// 自己計測データ構造体
struct SelfSensorData {
  float temperature;
  float humidity;
  bool valid;
};

class StateReporterIs02a {
public:
  StateReporterIs02a();

  // ========================================
  // 初期化
  // ========================================
  void begin(SettingManager* settings, NtpManager* ntp, BleReceiver* bleReceiver);

  // ========================================
  // 認証情報設定
  // ========================================
  void setAuth(const String& tid, const String& lacisId, const String& cic);
  void setMac(const String& mac);
  void setIp(const String& ip);

  // ========================================
  // エンドポイント設定
  // ========================================
  void setRelayUrls(const String& primary, const String& secondary);

  // ========================================
  // 自己計測データ設定
  // ========================================
  void setSelfSensorData(float temp, float hum, bool valid);

  // ========================================
  // バッチ送信
  // ========================================

  /**
   * バッチ送信（ローカルリレー）
   * @return 成功フラグ
   */
  bool sendBatchToRelay();

  /**
   * 全ペイロード構築（MQTT/リレー共通）
   * @return JSON文字列
   */
  String buildPayload();

  // ========================================
  // 統計
  // ========================================
  int getRelaySuccessCount() const { return relaySuccessCount_; }
  int getRelayFailCount() const { return relayFailCount_; }
  String getLastBatchTime() const { return lastBatchTime_; }
  void resetStats();

private:
  SettingManager* settings_;
  NtpManager* ntp_;
  BleReceiver* bleReceiver_;

  String tid_;
  String lacisId_;
  String cic_;
  String mac_;
  String ip_;

  String relayPrimaryUrl_;
  String relaySecondaryUrl_;

  SelfSensorData selfData_;

  int relaySuccessCount_;
  int relayFailCount_;
  String lastBatchTime_;

  // バックオフ制御
  int consecutiveFailures_;
  unsigned long lastFailTime_;

  // HTTP送信
  bool postToUrl(const String& url, const String& payload);
};

#endif // STATE_REPORTER_IS02A_H
