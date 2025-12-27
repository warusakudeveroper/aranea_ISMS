/**
 * HttpManagerIs02a.h
 *
 * is02a専用 Web UI実装
 *
 * AraneaWebUIを継承し、BLEノード管理とMQTT設定UIを追加。
 */

#ifndef HTTP_MANAGER_IS02A_H
#define HTTP_MANAGER_IS02A_H

#include "AraneaWebUI.h"

class BleReceiver;

class HttpManagerIs02a : public AraneaWebUI {
public:
  HttpManagerIs02a();

  /**
   * 初期化（拡張版）
   */
  void begin(SettingManager* settings, BleReceiver* bleReceiver, int port = 80);

  /**
   * 自己計測データ更新
   */
  void setSelfSensorData(float temp, float hum);

  /**
   * 送信ステータス更新
   */
  void setSendStatus(int mqttSuccess, int mqttFail, int relaySuccess, int relayFail);

  /**
   * 最終バッチ送信時刻更新
   */
  void setLastBatchTime(const String& time);

  /**
   * MQTT接続状態更新
   */
  void setMqttConnected(bool connected) { mqttConnected_ = connected; }

protected:
  // AraneaWebUI オーバーライド
  AraneaCloudStatus getCloudStatus() override;
  // ========================================
  // AraneaWebUI オーバーライド
  // ========================================
  void getTypeSpecificStatus(JsonObject& obj) override;
  String generateTypeSpecificTabs() override;
  String generateTypeSpecificTabContents() override;
  String generateTypeSpecificJS() override;
  void registerTypeSpecificEndpoints() override;
  void getTypeSpecificConfig(JsonObject& obj) override;

  // SpeedDial
  String generateTypeSpecificSpeedDial() override;
  bool applyTypeSpecificSpeedDial(const String& section, const std::vector<String>& lines) override;

private:
  BleReceiver* bleReceiver_;

  // 自己計測データ
  float selfTemp_;
  float selfHum_;

  // 送信統計
  int mqttSuccessCount_;
  int mqttFailCount_;
  int relaySuccessCount_;
  int relayFailCount_;
  String lastBatchTime_;
  bool mqttConnected_;

  // APIハンドラ
  void handleNodes();
  void handleNodesClear();
  void handleSensorConfig();
  void handleMqttConfig();
  void handleRelayConfig();
};

#endif // HTTP_MANAGER_IS02A_H
