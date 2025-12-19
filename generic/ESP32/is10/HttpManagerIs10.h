/**
 * HttpManagerIs10.h
 *
 * IS10 (Openwrt_RouterInspector) 専用HTTPサーバー
 * Chakra UI風モダンデザイン
 */

#ifndef HTTP_MANAGER_IS10_H
#define HTTP_MANAGER_IS10_H

#include <Arduino.h>
#include <WebServer.h>
#include <ArduinoJson.h>
#include "SettingManager.h"
#include "RouterTypes.h"

// グローバル設定構造体
struct GlobalSetting {
  String endpoint;
  unsigned long timeout;
  int retryCount;
  unsigned long interval;
};

// LacisID生成設定
struct LacisIdGenSetting {
  String prefix;
  String productType;
  String productCode;
  String generator;
};

class HttpManagerIs10 {
public:
  /**
   * 初期化
   * @param settings SettingManagerインスタンス
   * @param routers 外部のルーター配列へのポインタ
   * @param routerCount 外部のルーター数へのポインタ
   * @param port HTTPポート
   */
  void begin(SettingManager* settings, RouterConfig* routers, int* routerCount, int port = 80);
  void handle();
  void setDeviceInfo(const String& type, const String& lacisId,
                     const String& cic, const String& version);

  // 設定取得
  GlobalSetting getGlobalSetting();
  LacisIdGenSetting getLacisIdGenSetting();
  RouterConfig getRouter(int index);
  int getRouterCount();

  // コールバック
  void onSettingsChanged(void (*callback)());
  void onRebootRequest(void (*callback)());

  // WebServer取得（HttpOtaManager連携用）
  WebServer* getServer() { return server_; }

private:
  WebServer* server_ = nullptr;
  SettingManager* settings_ = nullptr;

  String deviceType_;
  String lacisId_;
  String cic_;
  String firmwareVersion_;
  String fid_;

  void (*settingsChangedCallback_)() = nullptr;
  void (*rebootCallback_)() = nullptr;

  // 外部ルーター配列への参照
  RouterConfig* routers_ = nullptr;
  int* routerCount_ = nullptr;

  // ハンドラ
  void handleRoot();
  void handleGetConfig();
  void handleSaveGlobal();
  void handleSaveLacisGen();
  void handleSaveRouter();
  void handleDeleteRouter();
  void handleSaveTenant();
  void handleSaveWifi();
  void handleReboot();
  void handleFactoryReset();
  void handleNotFound();

  // 設定読み書き
  void loadRouters();
  void saveRouters();
  void loadGlobalSettings();
  void saveGlobalSettings();

  // HTML生成
  String generateHTML();
  String generateCSS();
  String generateJS();
};

#endif // HTTP_MANAGER_IS10_H
