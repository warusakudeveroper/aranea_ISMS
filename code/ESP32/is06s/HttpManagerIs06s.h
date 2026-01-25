/**
 * HttpManagerIs06s.h
 *
 * IS06S HTTP API & Web UI マネージャー
 *
 * AraneaWebUIを継承し、PIN制御機能を追加
 *
 * 【APIエンドポイント】
 * - GET /api/status          - 全体ステータス（typeSpecific含む）
 * - GET /api/pin/{ch}/state  - PIN状態取得
 * - POST /api/pin/{ch}/state - PIN状態設定
 * - GET /api/pin/{ch}/setting - PIN設定取得
 * - POST /api/pin/{ch}/setting - PIN設定変更
 * - GET /api/settings        - 全設定取得
 * - POST /api/settings       - 全設定保存
 *
 * 【Web UIタブ】
 * - PINControl: リアルタイムPIN操作
 * - PINSettings: PIN設定変更
 */

#ifndef HTTP_MANAGER_IS06S_H
#define HTTP_MANAGER_IS06S_H

#include <Arduino.h>
#include <functional>
#include "AraneaWebUI.h"
#include "Is06PinManager.h"

// MQTT状態取得用コールバック型
using MqttStatusCallback = std::function<bool()>;

// PIN状態変更通知コールバック型
// (int channel, int newState) - channelは1-6、newStateは0/1またはPWM値
using PinStateChangeCallback = std::function<void(int channel, int newState)>;

class HttpManagerIs06s : public AraneaWebUI {
public:
  HttpManagerIs06s();
  virtual ~HttpManagerIs06s();

  /**
   * 初期化
   * @param settings SettingManager参照
   * @param pinManager Is06PinManager参照
   * @param port HTTPポート（デフォルト80）
   */
  void begin(SettingManager* settings, Is06PinManager* pinManager, int port = 80);

  /**
   * MQTT状態取得コールバック設定
   * @param callback MQTTが接続されているかを返す関数
   */
  void setMqttStatusCallback(MqttStatusCallback callback) { mqttStatusCallback_ = callback; }

  /**
   * MQTTデバッグ情報取得コールバック設定
   * @param enabledCb mqttEnabledを返す関数
   * @param urlCb mqttUrlを返す関数
   */
  void setMqttDebugCallbacks(std::function<bool()> enabledCb, std::function<String()> urlCb) {
    mqttEnabledCallback_ = enabledCb;
    mqttUrlCallback_ = urlCb;
  }

  /**
   * PIN状態変更通知コールバック設定
   * API/WebUI経由での状態変更をOLED表示等に通知
   * @param callback (channel, newState)を受け取る関数
   */
  void setPinStateChangeCallback(PinStateChangeCallback callback) {
    pinStateChangeCallback_ = callback;
  }

protected:
  // ========================================
  // AraneaWebUI オーバーライド
  // ========================================

  /**
   * typeSpecificステータス追加（PIN状態）
   */
  void getTypeSpecificStatus(JsonObject& obj) override;

  /**
   * 固有タブ（PINControl, PINSettings）
   */
  String generateTypeSpecificTabs() override;

  /**
   * 固有タブコンテンツ
   */
  String generateTypeSpecificTabContents() override;

  /**
   * 固有JavaScript
   */
  String generateTypeSpecificJS() override;

  /**
   * 固有APIエンドポイント登録
   */
  void registerTypeSpecificEndpoints() override;

  /**
   * 固有設定取得
   */
  void getTypeSpecificConfig(JsonObject& obj) override;

  /**
   * クラウド接続状態取得（MQTT状態を含む）
   */
  AraneaCloudStatus getCloudStatus() override;

private:
  MqttStatusCallback mqttStatusCallback_ = nullptr;
  std::function<bool()> mqttEnabledCallback_ = nullptr;
  std::function<String()> mqttUrlCallback_ = nullptr;
  PinStateChangeCallback pinStateChangeCallback_ = nullptr;
  Is06PinManager* pinManager_ = nullptr;

  // ========================================
  // APIハンドラ
  // ========================================
  void handlePinStateGet();
  void handlePinStatePost();
  void handlePinSettingGet();
  void handlePinSettingPost();
  void handlePinToggle();
  void handlePinAll();
  void handleSettingsGet();
  void handleSettingsPost();
  void handleDebugGpio();   // GPIO診断API

  // ========================================
  // ヘルパー
  // ========================================
  int extractChannelFromUri();
  void sendJsonResponse(int code, const String& message);
  void sendJsonSuccess(const String& message = "OK");
  void sendJsonError(int code, const String& message);

  // JSON構築
  void buildPinStateJson(JsonObject& obj, int channel);
  void buildPinSettingJson(JsonObject& obj, int channel);
  void buildAllPinsJson(JsonArray& arr);
};

#endif // HTTP_MANAGER_IS06S_H
