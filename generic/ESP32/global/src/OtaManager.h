#pragma once
#include <Arduino.h>
#include <ArduinoOTA.h>
#include <functional>

/**
 * OtaManager
 *
 * ArduinoOTAによるワイヤレスファームウェア更新管理
 */
class OtaManager {
public:
  /**
   * OTA初期化
   * @param hostname ホスト名（mDNS用）
   * @param password OTAパスワード（空文字で認証なし）
   * @return 成功時true
   */
  bool begin(const String& hostname = "aranea-device", const String& password = "");

  /**
   * OTA処理（loop()で呼び出し）
   * OTA更新リクエストをポーリング
   */
  void handle();

  /**
   * OTA更新中かどうか
   */
  bool isUpdating() const { return updating_; }

  /**
   * 更新進捗（0-100）
   */
  int progress() const { return progress_; }

  /**
   * コールバック設定：更新開始時
   */
  void onStart(std::function<void()> callback) { onStartCallback_ = callback; }

  /**
   * コールバック設定：更新終了時
   */
  void onEnd(std::function<void()> callback) { onEndCallback_ = callback; }

  /**
   * コールバック設定：進捗更新時
   */
  void onProgress(std::function<void(int)> callback) { onProgressCallback_ = callback; }

  /**
   * コールバック設定：エラー時
   */
  void onError(std::function<void(const String&)> callback) { onErrorCallback_ = callback; }

private:
  bool initialized_ = false;
  bool updating_ = false;
  int progress_ = 0;

  std::function<void()> onStartCallback_ = nullptr;
  std::function<void()> onEndCallback_ = nullptr;
  std::function<void(int)> onProgressCallback_ = nullptr;
  std::function<void(const String&)> onErrorCallback_ = nullptr;
};
