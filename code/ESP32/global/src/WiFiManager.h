#pragma once
#include <Arduino.h>
#include <WiFi.h>

class SettingManager;  // 前方宣言

/**
 * WiFiManager
 *
 * cluster1〜cluster6 に順次接続試行
 * - 各SSIDにタイムアウト付きで接続試行
 * - 全滅したら最初から繰り返す
 * - リトライ上限で自動再起動
 */
class WiFiManager {
public:
  /**
   * デフォルトSSIDリスト（cluster1-6）に接続試行
   * @return 接続成功時true
   */
  bool connectDefault();

  /**
   * NVS設定からWiFi情報を読み取って接続試行
   * @param settings 設定マネージャ参照
   * @return 接続成功時true
   */
  bool connectWithSettings(SettingManager* settings);

  /**
   * 指定のSSID/PASSに接続試行
   * @param ssid SSID
   * @param pass パスワード
   * @param timeoutMs タイムアウト（ミリ秒）
   * @return 接続成功時true
   */
  bool connect(const char* ssid, const char* pass, unsigned long timeoutMs = 15000);

  /**
   * 現在のIPアドレスを取得
   * @return IPアドレス文字列
   */
  String getIP();

  /**
   * 現在のRSSI（電波強度）を取得
   * @return RSSI（dBm）
   */
  int getRSSI();

  /**
   * WiFi切断
   */
  void disconnect();

  /**
   * 接続中か確認
   * @return 接続中ならtrue
   */
  bool isConnected();

  /**
   * 失敗カウントを取得
   */
  int getFailCount() const { return failCount_; }

private:
  static const int SSID_COUNT = 6;
  static const char* SSID_LIST[SSID_COUNT];
  static const char* WIFI_PASS;

  int failCount_ = 0;
  int retryLimit_ = 30;  // デフォルト30回で再起動
};
