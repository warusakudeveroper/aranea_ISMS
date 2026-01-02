/**
 * CelestialSenderIs10.h
 *
 * CelestialGlobe SSOT送信モジュール（is10固有）
 *
 * 責務:
 * - Universal Ingest JSONペイロード構築
 * - devices[]/clients[]配列構築
 * - HTTP POST送信
 */

#ifndef CELESTIAL_SENDER_IS10_H
#define CELESTIAL_SENDER_IS10_H

#include <Arduino.h>
#include "SshPollerIs10.h"

// 前方宣言
class SettingManager;
class NtpManager;

class CelestialSenderIs10 {
public:
  CelestialSenderIs10();

  /**
   * 初期化
   */
  void begin(SettingManager* settings, NtpManager* ntp, SshPollerIs10* sshPoller);

  /**
   * 認証情報設定
   */
  void setAuth(const String& tid, const String& lacisId,
               const String& cic, const String& fid);

  /**
   * source設定（CelestialGlobe payload.source用）
   */
  void setSource(const String& source) { source_ = source; }

  /**
   * デバイスMAC設定（X-Aranea-Macヘッダー用、12桁HEX大文字）
   */
  void setDeviceMac(const String& mac) { deviceMac_ = mac; }

  /**
   * CelestialGlobeへ送信
   * @return 成功フラグ
   */
  bool send();

  /**
   * 設定済みチェック（endpoint/secretが設定されているか）
   */
  bool isConfigured();

private:
  SettingManager* settings_ = nullptr;
  NtpManager* ntp_ = nullptr;
  SshPollerIs10* sshPoller_ = nullptr;

  // 認証情報
  String tid_;
  String lacisId_;
  String cic_;
  String fid_;
  String source_;
  String deviceMac_;  // X-Aranea-Macヘッダー用（12桁HEX大文字）

  // バックオフ制御
  int consecutiveFailures_ = 0;
  unsigned long lastFailTime_ = 0;

  // MACアドレスをコロン区切り形式に変換
  String formatMacWithColons(const String& mac);
};

#endif // CELESTIAL_SENDER_IS10_H
