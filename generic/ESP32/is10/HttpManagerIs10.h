/**
 * HttpManagerIs10.h
 *
 * IS10 (Router Inspector) HTTPサーバー
 * AraneaWebUI 基底クラスを継承し、is10固有の機能を追加
 */

#ifndef HTTP_MANAGER_IS10_H
#define HTTP_MANAGER_IS10_H

#include <Arduino.h>
#include "AraneaWebUI.h"
#include "RouterTypes.h"

// IS10グローバル設定構造体
struct Is10GlobalSetting {
  String endpoint;           // CelestialGlobe endpoint URL
  String celestialSecret;    // X-Celestial-Secret header value
  int scanIntervalSec;       // スキャン間隔（秒）
  bool reportClients;        // クライアントリスト送信フラグ
  unsigned long timeout;     // SSHタイムアウト（ms）
  int retryCount;            // リトライ回数
  unsigned long interval;    // ルーター間インターバル（ms）
};

class HttpManagerIs10 : public AraneaWebUI {
public:
  /**
   * 初期化
   * @param settings SettingManagerインスタンス
   * @param routers 外部のルーター配列へのポインタ
   * @param routerCount 外部のルーター数へのポインタ
   * @param port HTTPポート
   */
  void begin(SettingManager* settings, RouterConfig* routers, int* routerCount, int port = 80);

  // 設定取得
  Is10GlobalSetting getGlobalSetting();
  RouterConfig getRouter(int index);
  int getRouterCount();

  // ルーターステータス更新（メインループから呼び出し）
  void setRouterStatus(int totalRouters, int successfulPolls, unsigned long lastPollTime);
  void setMqttStatus(bool connected);
  void setLastStateReport(const String& time, int code);

  /**
   * ルーター設定をSPIFFSに永続化（MQTT config適用時用）
   */
  void persistRouters();

protected:
  // AraneaWebUI オーバーライド
  void getTypeSpecificStatus(JsonObject& obj) override;
  void getTypeSpecificConfig(JsonObject& obj) override;
  String generateTypeSpecificTabs() override;
  String generateTypeSpecificTabContents() override;
  String generateTypeSpecificJS() override;
  void registerTypeSpecificEndpoints() override;

private:
  // 外部ルーター配列への参照
  RouterConfig* routers_ = nullptr;
  int* routerCount_ = nullptr;

  // ステータス情報
  int totalRouters_ = 0;
  int successfulPolls_ = 0;
  unsigned long lastPollTime_ = 0;
  bool mqttConnected_ = false;
  String lastStateReportTime_;
  int lastStateReportCode_ = 0;

  // is10固有ハンドラ
  void handleSaveInspector();
  void handleSaveRouter();
  void handleDeleteRouter();

  // ルーター設定読み書き
  void loadRouters();
  void saveRouters();
};

#endif // HTTP_MANAGER_IS10_H
