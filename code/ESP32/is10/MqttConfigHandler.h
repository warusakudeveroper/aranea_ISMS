/**
 * MqttConfigHandler.h
 *
 * MQTT configメッセージ処理モジュール（is10固有）
 *
 * 責務:
 * - MQTT configメッセージの受信・パース
 * - routers設定の適用
 * - StateReporterIs10へのSSoT状態反映
 */

#ifndef MQTT_CONFIG_HANDLER_H
#define MQTT_CONFIG_HANDLER_H

#include <Arduino.h>
#include <ArduinoJson.h>
#include "RouterTypes.h"

// 前方宣言
class SettingManager;
class NtpManager;
class HttpManagerIs10;
class StateReporterIs10;

class MqttConfigHandler {
public:
  MqttConfigHandler();

  /**
   * 初期化
   */
  void begin(SettingManager* settings, NtpManager* ntp,
             RouterConfig* routers, int* routerCount,
             HttpManagerIs10* httpMgr, StateReporterIs10* stateReporter);

  /**
   * configメッセージ処理
   * @param data JSONデータ
   * @param len データ長
   */
  void handleConfigMessage(const char* data, int len);

  /**
   * 設定適用成功時のコールバック設定
   */
  void onConfigApplied(std::function<void()> cb) { configAppliedCallback_ = cb; }

private:
  SettingManager* settings_ = nullptr;
  NtpManager* ntp_ = nullptr;
  RouterConfig* routers_ = nullptr;
  int* routerCount_ = nullptr;
  HttpManagerIs10* httpMgr_ = nullptr;
  StateReporterIs10* stateReporter_ = nullptr;

  std::function<void()> configAppliedCallback_ = nullptr;

  /**
   * ルーター設定適用
   */
  void applyRouterConfig(JsonArray routersJson);
};

#endif // MQTT_CONFIG_HANDLER_H
