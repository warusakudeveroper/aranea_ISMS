/**
 * StateReporterIs10.h
 *
 * IS10 deviceStateReport ペイロード構築モジュール
 *
 * 責務:
 * - is10固有のJSON構築（routers, sshPoller情報含む）
 * - StateReporter (global)と連携して送信
 */

#ifndef STATE_REPORTER_IS10_H
#define STATE_REPORTER_IS10_H

#include <Arduino.h>
#include "StateReporter.h"
#include "RouterTypes.h"

// 前方宣言
class SettingManager;
class AraneaRegister;
class NtpManager;
class MqttManager;
class SshPollerIs10;
class HttpManagerIs10;

class StateReporterIs10 {
public:
  StateReporterIs10();

  /**
   * 初期化
   */
  void begin(StateReporter* reporter, SettingManager* settings, AraneaRegister* araneaReg,
             NtpManager* ntp, MqttManager* mqtt, SshPollerIs10* sshPoller,
             RouterConfig* routers, int* routerCount);

  /**
   * ペイロード構築（StateReporterのコールバックから呼ばれる）
   * @return JSON文字列
   */
  String buildPayload();

  // ========================================
  // デバイス情報設定
  // ========================================
  void setDeviceType(const String& type) { deviceType_ = type; }
  void setLacisId(const String& lacisId) { lacisId_ = lacisId; }
  void setMac(const String& mac) { mac_ = mac; }
  void setHostname(const String& hostname) { hostname_ = hostname; }
  void setTid(const String& tid) { tid_ = tid; }
  void setCic(const String& cic) { cic_ = cic; }

  // ========================================
  // SSOT状態取得/設定
  // ========================================
  int getSchemaVersion() const { return schemaVersion_; }
  String getConfigHash() const { return configHash_; }
  String getLastAppliedAt() const { return lastAppliedAt_; }
  String getLastApplyError() const { return lastApplyError_; }

  void setAppliedConfig(int schemaVersion, const String& hash, const String& appliedAt);
  void setApplyError(const String& error);

  // ========================================
  // UI連携コールバック
  // ========================================
  void setHttpManager(HttpManagerIs10* httpMgr) { httpMgr_ = httpMgr; }

  // ========================================
  // ヘルパー
  // ========================================
  static String sanitizeDeviceName(const String& raw);

private:
  // 依存モジュール
  StateReporter* reporter_ = nullptr;
  SettingManager* settings_ = nullptr;
  AraneaRegister* araneaReg_ = nullptr;
  NtpManager* ntp_ = nullptr;
  MqttManager* mqtt_ = nullptr;
  SshPollerIs10* sshPoller_ = nullptr;
  HttpManagerIs10* httpMgr_ = nullptr;
  RouterConfig* routers_ = nullptr;
  int* routerCount_ = nullptr;

  // デバイス情報
  String deviceType_;
  String lacisId_;
  String mac_;
  String hostname_;
  String tid_;
  String cic_;

  // SSOT状態
  int schemaVersion_ = 0;
  String configHash_;
  String lastAppliedAt_;
  String lastApplyError_;

  // ヘルパー
  String getDeviceName();
};

#endif // STATE_REPORTER_IS10_H
