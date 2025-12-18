#pragma once
#include <Arduino.h>
#include <WebServer.h>
#include <functional>

class SettingManager;  // 前方宣言

// ファクトリーリセット用パスワード
static const char* FACTORY_RESET_PASS = "isms12345@";

/**
 * HttpManager
 *
 * Web UI経由での設定変更・ステータス確認
 * - ログインパス認証（初期値: 0000）
 * - 設定の表示・変更（リレー、WiFi、テナント）
 * - デバイスステータス表示
 * - 再起動・ファクトリーリセット
 */
class HttpManager {
public:
  /**
   * 初期化
   * @param settings 設定マネージャ参照
   * @param port HTTPポート（デフォルト80）
   * @return 成功時true
   */
  bool begin(SettingManager* settings, int port = 80);

  /**
   * HTTP処理（loop()で呼び出し）
   */
  void handle();

  /**
   * ステータス情報設定用コールバック
   * JSON形式のステータス文字列を返す関数を設定
   */
  void setStatusCallback(std::function<String()> callback) {
    statusCallback_ = callback;
  }

  /**
   * デバイス情報設定
   */
  void setDeviceInfo(const String& deviceType, const String& lacisId,
                     const String& cic, const String& version);

  /**
   * センサー値更新（表示用）
   */
  void setSensorValues(float temp, float hum);

  /**
   * 初期化済みか
   */
  bool isReady() const { return initialized_; }

private:
  WebServer* server_ = nullptr;
  SettingManager* settings_ = nullptr;
  bool initialized_ = false;
  String loginPass_ = "0000";

  // デバイス情報
  String deviceType_ = "";
  String lacisId_ = "";
  String cic_ = "";
  String version_ = "1.0.0";
  float tempC_ = NAN;
  float humPct_ = NAN;

  std::function<String()> statusCallback_ = nullptr;

  // ハンドラ
  void handleRoot();
  void handleLogin();
  void handleStatus();
  void handleSettings();
  void handleSettingsPost();
  void handleWifi();
  void handleWifiPost();
  void handleTenant();
  void handleTenantPost();
  void handleIs05();
  void handleIs05Post();
  void handleIs04();
  void handleIs04Post();
  void handleReboot();
  void handleFactoryReset();
  void handleNotFound();

  // 認証チェック
  bool checkAuth();

  // HTML生成
  String generateLoginPage();
  String generateMainPage();
  String generateSettingsPage();
  String generateWifiPage();
  String generateTenantPage();
  String generateIs05Page();
  String generateIs04Page();
  String generateStatusJson();
};
