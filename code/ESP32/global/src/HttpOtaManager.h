#pragma once
#include <Arduino.h>
#include <WebServer.h>
#include <Update.h>
#include <esp_ota_ops.h>
#include <functional>

/**
 * HttpOtaManager
 *
 * HTTP経由のOTA更新機能を提供
 * MCP Arduino-ESP32 toolsと互換性のあるAPIエンドポイントを実装
 *
 * エンドポイント:
 * - GET  /api/ota/status  - OTAステータス取得
 * - GET  /api/ota/info    - パーティション情報取得
 * - POST /api/ota/update  - ファームウェアアップロード
 * - POST /api/ota/rollback - 前バージョンへロールバック
 * - POST /api/ota/confirm  - ファームウェア正常確認
 */

enum class HttpOtaStatus {
  IDLE,
  IN_PROGRESS,
  SUCCESS,
  ERROR
};

struct HttpOtaInfo {
  String runningPartition;
  String nextPartition;
  bool canRollback;
  bool pendingVerify;
  size_t partitionSize;
};

class HttpOtaManager {
public:
  /**
   * 初期化
   * @param server WebServerインスタンス
   * @param authToken 認証トークン（空で認証なし）
   */
  void begin(WebServer* server, const String& authToken = "");

  /**
   * 現在のOTAステータス取得
   */
  HttpOtaStatus getStatus() const { return status_; }

  /**
   * 進捗取得（0-100）
   */
  int getProgress() const { return progress_; }

  /**
   * エラーメッセージ取得
   */
  String getError() const { return errorMessage_; }

  /**
   * パーティション情報取得
   */
  HttpOtaInfo getOtaInfo();

  /**
   * コールバック設定
   */
  void onStart(std::function<void()> callback) { onStartCallback_ = callback; }
  void onEnd(std::function<void()> callback) { onEndCallback_ = callback; }
  void onProgress(std::function<void(int)> callback) { onProgressCallback_ = callback; }
  void onError(std::function<void(const String&)> callback) { onErrorCallback_ = callback; }

private:
  WebServer* server_ = nullptr;
  String authToken_;

  HttpOtaStatus status_ = HttpOtaStatus::IDLE;
  int progress_ = 0;
  String errorMessage_;
  size_t uploadedSize_ = 0;
  size_t totalSize_ = 0;

  std::function<void()> onStartCallback_ = nullptr;
  std::function<void()> onEndCallback_ = nullptr;
  std::function<void(int)> onProgressCallback_ = nullptr;
  std::function<void(const String&)> onErrorCallback_ = nullptr;

  // 認証チェック
  bool checkAuth();

  // エンドポイントハンドラ
  void handleStatus();
  void handleInfo();
  void handleUpdate();
  void handleUpload();
  void handleRollback();
  void handleConfirm();

  // ヘルパー
  String statusToString(HttpOtaStatus status);
  void setError(const String& msg);
};
