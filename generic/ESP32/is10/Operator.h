#pragma once
#include <Arduino.h>

/**
 * 動作モード
 */
enum class OperatorMode {
  PROVISION,    // 初回プロビジョニング中
  RUN,          // 通常動作
  MAINTENANCE,  // メンテナンス中（OTA等）
  FAILSAFE      // フェイルセーフモード
};

/**
 * リソースロック種別
 */
enum class ResourceLock {
  I2C,          // I2Cバス（ディスプレイ、センサー）
  WIFI,         // WiFi操作
  HTTP_CLIENT,  // HTTPクライアント送信
  HTTP_SERVER,  // HTTPサーバー処理
  OTA,          // OTA更新中
  BLE           // BLEスキャン
};

/**
 * Operator
 *
 * 動作モード管理と競合制御
 * - I2C/WiFi/HTTP/OTAなど「追い越すとクラッシュしやすい」処理の排他制御
 * - フラグベースのシンプルなロック機構
 */
class Operator {
public:
  /**
   * モード設定
   */
  void setMode(OperatorMode mode);

  /**
   * 現在モード取得
   */
  OperatorMode mode() const { return mode_; }

  /**
   * モード名取得（デバッグ用）
   */
  const char* modeName() const;

  /**
   * リソースロック取得
   * @param resource ロック対象
   * @param timeoutMs タイムアウト（ms、0で即時）
   * @return 取得成功時true
   */
  bool lock(ResourceLock resource, unsigned long timeoutMs = 1000);

  /**
   * リソースロック解放
   * @param resource 解放対象
   */
  void unlock(ResourceLock resource);

  /**
   * リソースがロック中か
   */
  bool isLocked(ResourceLock resource) const;

  /**
   * 全ロック解放
   */
  void unlockAll();

  /**
   * OTA更新中フラグ（特別扱い）
   */
  bool isOtaUpdating() const { return otaUpdating_; }
  void setOtaUpdating(bool updating) { otaUpdating_ = updating; }

  /**
   * 致命的エラー発生フラグ
   */
  bool hasFatalError() const { return fatalError_; }
  void setFatalError(bool error) { fatalError_ = error; }

  /**
   * エラーメッセージ設定
   */
  void setErrorMessage(const String& msg) { errorMsg_ = msg; }
  String getErrorMessage() const { return errorMsg_; }

private:
  OperatorMode mode_ = OperatorMode::PROVISION;

  // ロックフラグ
  volatile bool lockI2C_ = false;
  volatile bool lockWiFi_ = false;
  volatile bool lockHttpClient_ = false;
  volatile bool lockHttpServer_ = false;
  volatile bool lockOta_ = false;
  volatile bool lockBle_ = false;

  // 特殊フラグ
  bool otaUpdating_ = false;
  bool fatalError_ = false;
  String errorMsg_ = "";

  // ロックポインタ取得
  volatile bool* getLockPtr(ResourceLock resource);
};

/**
 * スコープベースロック（RAII）
 * 使用例:
 *   ScopedLock lock(operator, ResourceLock::I2C);
 *   if (lock.acquired()) { ... }
 */
class ScopedLock {
public:
  ScopedLock(Operator& op, ResourceLock resource, unsigned long timeoutMs = 1000)
    : op_(op), resource_(resource), acquired_(op.lock(resource, timeoutMs)) {}

  ~ScopedLock() {
    if (acquired_) {
      op_.unlock(resource_);
    }
  }

  bool acquired() const { return acquired_; }

  // コピー禁止
  ScopedLock(const ScopedLock&) = delete;
  ScopedLock& operator=(const ScopedLock&) = delete;

private:
  Operator& op_;
  ResourceLock resource_;
  bool acquired_;
};
