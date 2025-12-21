#include "OtaManager.h"

// 静的インスタンスポインタ（コールバック用）
static OtaManager* otaInstance = nullptr;

bool OtaManager::begin(const String& hostname, const String& password) {
  if (initialized_) return true;

  otaInstance = this;

  // ホスト名設定
  ArduinoOTA.setHostname(hostname.c_str());

  // パスワード設定（空でなければ）
  if (password.length() > 0) {
    ArduinoOTA.setPassword(password.c_str());
  }

  // 更新開始コールバック
  ArduinoOTA.onStart([this]() {
    updating_ = true;
    progress_ = 0;
    String type = (ArduinoOTA.getCommand() == U_FLASH) ? "sketch" : "filesystem";
    Serial.printf("[OTA] Start updating %s\n", type.c_str());
    if (onStartCallback_) onStartCallback_();
  });

  // 更新終了コールバック
  ArduinoOTA.onEnd([this]() {
    updating_ = false;
    progress_ = 100;
    Serial.println("\n[OTA] Update complete");
    if (onEndCallback_) onEndCallback_();
  });

  // 進捗コールバック
  ArduinoOTA.onProgress([this](unsigned int progress, unsigned int total) {
    progress_ = (progress * 100) / total;
    Serial.printf("[OTA] Progress: %u%%\r", progress_);
    if (onProgressCallback_) onProgressCallback_(progress_);
  });

  // エラーコールバック
  ArduinoOTA.onError([this](ota_error_t error) {
    updating_ = false;
    String errMsg;
    switch (error) {
      case OTA_AUTH_ERROR:    errMsg = "Auth Failed"; break;
      case OTA_BEGIN_ERROR:   errMsg = "Begin Failed"; break;
      case OTA_CONNECT_ERROR: errMsg = "Connect Failed"; break;
      case OTA_RECEIVE_ERROR: errMsg = "Receive Failed"; break;
      case OTA_END_ERROR:     errMsg = "End Failed"; break;
      default:                errMsg = "Unknown Error"; break;
    }
    Serial.printf("[OTA] Error: %s\n", errMsg.c_str());
    if (onErrorCallback_) onErrorCallback_(errMsg);
  });

  ArduinoOTA.begin();
  initialized_ = true;
  Serial.printf("[OTA] Ready: %s.local\n", hostname.c_str());
  return true;
}

void OtaManager::handle() {
  if (!initialized_) return;
  ArduinoOTA.handle();
}
