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

// 最大ルーター数
#define MAX_ROUTERS 20

// ルーターOSタイプ
enum class RouterOsType {
  OPENWRT = 0,   // OpenWrt (uci, br-lan, /tmp/dhcp.leases)
  ASUSWRT = 1    // ASUSWRT (nvram, br0, /var/lib/misc/dnsmasq.leases)
};

// ルーター設定構造体
struct RouterSetting {
  String rid;
  String ipAddr;
  String publicKey;
  int sshPort;
  String username;
  String password;
  bool enabled;
  RouterOsType osType;  // OpenWrt or ASUSWRT
};

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
  void begin(SettingManager* settings, int port = 80);
  void handle();
  void setDeviceInfo(const String& type, const String& lacisId,
                     const String& cic, const String& version);

  // 設定取得
  GlobalSetting getGlobalSetting();
  LacisIdGenSetting getLacisIdGenSetting();
  RouterSetting getRouter(int index);
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

  // ルーター設定キャッシュ
  RouterSetting routers_[MAX_ROUTERS];
  int routerCount_ = 0;

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
