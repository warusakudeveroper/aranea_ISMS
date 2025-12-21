/**
 * MqttManager.h
 *
 * Aranea デバイス共通 MQTT クライアント
 * ESP-IDF mqtt_client のラッパー
 *
 * 機能:
 * - WSS/MQTT over WebSocket 接続
 * - 自動再接続
 * - コールバックベースのイベント処理
 * - ESP-IDF証明書バンドルによるTLS
 */

#ifndef MQTT_MANAGER_H
#define MQTT_MANAGER_H

#include <Arduino.h>
#include <functional>
#include <mqtt_client.h>

class MqttManager {
public:
  MqttManager();
  ~MqttManager();

  /**
   * MQTT接続開始
   * @param url MQTT broker URL (wss://...)
   * @param username 認証ユーザー名（通常はlacisId）
   * @param password 認証パスワード（通常はcic）
   * @return 接続開始成功
   */
  bool begin(const String& url, const String& username, const String& password);

  /**
   * ループ処理（再接続チェック）
   * main loop()で毎回呼び出す
   */
  void handle();

  /**
   * 接続停止・クリーンアップ
   */
  void stop();

  /**
   * 接続状態取得
   */
  bool isConnected() const { return connected_; }

  /**
   * トピックへメッセージ発行
   * @param topic トピック名
   * @param payload ペイロード
   * @param qos QoS (0, 1, 2)
   * @param retain リテインフラグ
   * @return メッセージID (エラー時は-1)
   */
  int publish(const String& topic, const String& payload, int qos = 1, bool retain = false);

  /**
   * トピック購読
   * @param topic トピック名
   * @param qos QoS (0, 1, 2)
   * @return メッセージID (エラー時は-1)
   */
  int subscribe(const String& topic, int qos = 1);

  /**
   * トピック購読解除
   * @param topic トピック名
   * @return メッセージID (エラー時は-1)
   */
  int unsubscribe(const String& topic);

  /**
   * 再接続試行
   */
  void reconnect();

  // ========================================
  // コールバック設定
  // ========================================

  /**
   * 接続完了時コールバック
   * トピック購読はここで行う
   */
  void onConnected(std::function<void()> callback) { connectedCallback_ = callback; }

  /**
   * 切断時コールバック
   */
  void onDisconnected(std::function<void()> callback) { disconnectedCallback_ = callback; }

  /**
   * メッセージ受信コールバック
   * @param topic トピック名
   * @param data ペイロード
   * @param len ペイロード長
   */
  void onMessage(std::function<void(const String& topic, const char* data, int len)> callback) {
    messageCallback_ = callback;
  }

  /**
   * エラー発生コールバック
   * @param error エラーメッセージ
   */
  void onError(std::function<void(const String& error)> callback) { errorCallback_ = callback; }

  // ========================================
  // 設定
  // ========================================

  /**
   * 自動再接続間隔設定
   * @param ms ミリ秒（0で無効）
   */
  void setReconnectInterval(unsigned long ms) { reconnectIntervalMs_ = ms; }

  /**
   * ネットワークタイムアウト設定
   * @param ms ミリ秒
   */
  void setNetworkTimeout(int ms) { networkTimeoutMs_ = ms; }

  /**
   * Keep-Alive間隔設定
   * @param sec 秒
   */
  void setKeepAlive(int sec) { keepAliveSec_ = sec; }

private:
  // ESP-IDF MQTTクライアント
  esp_mqtt_client_handle_t client_ = nullptr;

  // 接続情報
  String url_;
  String username_;
  String password_;

  // 状態
  bool connected_ = false;
  bool initialized_ = false;
  unsigned long lastReconnectAttempt_ = 0;

  // 設定
  unsigned long reconnectIntervalMs_ = 5000;  // 5秒
  int networkTimeoutMs_ = 10000;              // 10秒
  int keepAliveSec_ = 30;                     // 30秒

  // コールバック
  std::function<void()> connectedCallback_ = nullptr;
  std::function<void()> disconnectedCallback_ = nullptr;
  std::function<void(const String& topic, const char* data, int len)> messageCallback_ = nullptr;
  std::function<void(const String& error)> errorCallback_ = nullptr;

  // イベントハンドラ（静的関数からインスタンスメソッドへブリッジ）
  static void eventHandler(void* handlerArgs, esp_event_base_t base,
                           int32_t eventId, void* eventData);
  void handleEvent(esp_mqtt_event_handle_t event, int32_t eventId);
};

#endif // MQTT_MANAGER_H
