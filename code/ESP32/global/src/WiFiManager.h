#pragma once
#include <Arduino.h>
#include <WiFi.h>

/**
 * WiFiManager
 *
 * cluster1〜cluster6 に順次接続試行
 * - 各SSIDにタイムアウト付きで接続試行
 * - 全滅したら最初から繰り返す
 */
class WiFiManager {
public:
  /**
   * デフォルトSSIDリスト（cluster1-6）に接続試行
   * @return 接続成功時true
   */
  bool connectDefault();

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

private:
  static const int SSID_COUNT = 6;
  static const char* SSID_LIST[SSID_COUNT];
  static const char* WIFI_PASS;
};
