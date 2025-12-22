/**
 * StateReporter.h
 *
 * Aranea デバイス共通 deviceStateReport送信
 *
 * 機能:
 * - HTTP POST送信（共通処理）
 * - Heartbeat管理
 * - コールバックでペイロード構築を委譲
 */

#ifndef STATE_REPORTER_H
#define STATE_REPORTER_H

#include <Arduino.h>
#include <functional>

class StateReporter {
public:
  StateReporter();

  /**
   * 初期化
   * @param endpoint stateEndpoint URL
   */
  void begin(const String& endpoint);

  /**
   * エンドポイント更新
   */
  void setEndpoint(const String& endpoint) { endpoint_ = endpoint; }

  /**
   * ループ処理（heartbeat）
   * main loop()で毎回呼び出す
   */
  void handle();

  /**
   * レポート送信
   * @param jsonPayload JSON文字列
   * @return 成功フラグ
   */
  bool sendReport(const String& jsonPayload);

  /**
   * 初回送信済みフラグ
   */
  bool isInitialReportSent() const { return initialReportSent_; }
  void setInitialReportSent(bool sent) { initialReportSent_ = sent; }

  /**
   * Heartbeat間隔設定
   * @param ms ミリ秒（デフォルト: 60000）
   */
  void setHeartbeatInterval(unsigned long ms) { heartbeatIntervalMs_ = ms; }

  /**
   * 有効/無効設定
   */
  void setEnabled(bool enabled) { enabled_ = enabled; }
  bool isEnabled() const { return enabled_; }

  // ========================================
  // コールバック
  // ========================================

  /**
   * ペイロード構築コールバック
   * heartbeat時に呼ばれ、送信するJSONを返す
   * @return JSON文字列（空文字列で送信スキップ）
   */
  void onBuildPayload(std::function<String()> cb) { buildPayloadCallback_ = cb; }

  /**
   * 送信完了コールバック
   * @param success 成功フラグ
   * @param httpCode HTTPステータスコード
   */
  void onReportSent(std::function<void(bool success, int httpCode)> cb) {
    reportSentCallback_ = cb;
  }

private:
  String endpoint_;
  unsigned long heartbeatIntervalMs_ = 60000;
  unsigned long lastHeartbeat_ = 0;
  bool initialReportSent_ = false;
  bool enabled_ = true;

  // バックオフ制御
  int consecutiveFailures_ = 0;
  unsigned long lastFailTime_ = 0;

  std::function<String()> buildPayloadCallback_ = nullptr;
  std::function<void(bool, int)> reportSentCallback_ = nullptr;
};

#endif // STATE_REPORTER_H
