/**
 * AraneaWebUI.h
 *
 * Aranea デバイス共通 Web UI フレームワーク
 * 全デバイス（is02, is04, is05, is10等）で共通利用可能
 *
 * 機能:
 * - 共通タブ: Status / Network / Cloud / Tenant / System
 * - CICパラメータ付きステータスAPI（巡回ポーリング対応）
 * - デバイス固有タブの拡張ポイント
 * - ロゴ・CSS・JS の PROGMEM 埋め込み
 */

#ifndef ARANEA_WEB_UI_H
#define ARANEA_WEB_UI_H

#include <Arduino.h>
#include <WebServer.h>
#include <ArduinoJson.h>
#include "SettingManager.h"

// UI/APIバージョン
#define ARANEA_UI_VERSION "1.0.0"

// ========================================
// デバイス情報構造体（Statusタブ用）
// ========================================
struct AraneaDeviceInfo {
  String deviceType;      // e.g. "ar-is10"
  String modelName;       // e.g. "Router Inspector" - ヘッダー直下表示用
  String contextDesc;     // 機能コンテキスト説明 e.g. "ルーターの状態監視とクライアント情報収集"
  String lacisId;
  String cic;
  String mac;
  String hostname;
  String firmwareVersion;
  String buildDate;
  String modules;         // カンマ区切り e.g. "WiFi,NTP,SSH,MQTT"
};

// ========================================
// ネットワーク状態構造体
// ========================================
struct AraneaNetworkStatus {
  String ip;
  String ssid;
  int rssi;
  String gateway;
  String subnet;
  bool apMode;
  String apSsid;
};

// ========================================
// システム状態構造体
// ========================================
struct AraneaSystemStatus {
  String ntpTime;         // ISO8601
  bool ntpSynced;
  unsigned long uptime;   // seconds
  size_t heap;
  size_t heapLargest;
  int cpuFreq;            // MHz
  size_t flashSize;
};

// ========================================
// クラウド接続状態構造体
// ========================================
struct AraneaCloudStatus {
  bool registered;
  bool mqttConnected;
  String lastStateReport;
  int lastStateReportCode;
  int schemaVersion;
};

// ========================================
// AraneaWebUI 基底クラス
// ========================================
class AraneaWebUI {
public:
  AraneaWebUI();
  virtual ~AraneaWebUI();

  /**
   * 初期化
   * @param settings SettingManager インスタンス
   * @param port HTTPポート（デフォルト80）
   */
  virtual void begin(SettingManager* settings, int port = 80);

  /**
   * ループ処理（WebServer.handleClient）
   */
  virtual void handle();

  /**
   * デバイス情報設定（派生クラスから呼び出し）
   */
  void setDeviceInfo(const AraneaDeviceInfo& info);

  /**
   * WebServerインスタンス取得（OTA等で使用）
   */
  WebServer* getServer() { return server_; }

  /**
   * 設定変更コールバック
   */
  void onSettingsChanged(void (*callback)());

  /**
   * 再起動コールバック
   */
  void onRebootRequest(void (*callback)());

protected:
  // ========================================
  // 派生クラスでオーバーライド可能
  // ========================================

  /**
   * デバイス固有のステータス取得
   * 派生クラスでオーバーライドしてtypeSpecificフィールドを追加
   */
  virtual void getTypeSpecificStatus(JsonObject& obj) {}

  /**
   * デバイス固有のタブHTML生成
   * 派生クラスでオーバーライドして独自タブを追加
   */
  virtual String generateTypeSpecificTabs() { return ""; }

  /**
   * デバイス固有のタブコンテンツHTML生成
   */
  virtual String generateTypeSpecificTabContents() { return ""; }

  /**
   * デバイス固有のJavaScript生成
   */
  virtual String generateTypeSpecificJS() { return ""; }

  /**
   * デバイス固有のAPIエンドポイント登録
   * 派生クラスでオーバーライドして独自APIを追加
   */
  virtual void registerTypeSpecificEndpoints() {}

  /**
   * デバイス固有の設定取得（/api/config用）
   */
  virtual void getTypeSpecificConfig(JsonObject& obj) {}

  // ========================================
  // 共通ハンドラ
  // ========================================
  void handleRoot();
  void handleStatus();
  void handleConfig();
  void handleSaveNetwork();
  void handleSaveAP();
  void handleSaveCloud();
  void handleSaveTenant();
  void handleSaveSystem();
  void handleReboot();
  void handleFactoryReset();
  void handleNotFound();

  // ========================================
  // HTML/CSS/JS 生成
  // ========================================
  String generateHTML();
  String generateCSS();
  String generateJS();
  String generateLogoSVG();

  // ========================================
  // ステータス取得ヘルパー
  // ========================================
  AraneaNetworkStatus getNetworkStatus();
  AraneaSystemStatus getSystemStatus();
  AraneaCloudStatus getCloudStatus();
  String formatUptime(unsigned long seconds);

  // ========================================
  // メンバ変数
  // ========================================
  WebServer* server_ = nullptr;
  SettingManager* settings_ = nullptr;
  AraneaDeviceInfo deviceInfo_;

  void (*settingsChangedCallback_)() = nullptr;
  void (*rebootCallback_)() = nullptr;

  // CIC認証用
  bool validateCIC(const String& cic);

  // Basic Auth認証
  bool checkAuth();
  void requestAuth();
};

#endif // ARANEA_WEB_UI_H
