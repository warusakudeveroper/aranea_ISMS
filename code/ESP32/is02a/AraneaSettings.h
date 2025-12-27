/**
 * AraneaSettings.h (is02a用)
 *
 * IS02A専用 araneaDevice設定クラス
 *
 * 【このファイルの目的】
 * 大量生産（施設展開）時に、同一ファームウェアを複数台に書き込むための
 * デフォルト設定を定義するクラス。
 *
 * 【大量生産モード】
 * 1. ARANEA_DEFAULT_* 定数を施設用に編集
 * 2. ビルド → 複数台に書き込み
 * 3. 各デバイスは起動時にNVSが空なら、デフォルト値を自動適用
 *
 * 【汎用モード】
 * APモード起動 → Web UIから設定変更 → NVS保存
 */

#ifndef ARANEA_SETTINGS_IS02A_H
#define ARANEA_SETTINGS_IS02A_H

#include <Arduino.h>

class SettingManager;

// ========================================
// デフォルト設定（施設展開時に編集）
// ========================================
#define ARANEA_DEFAULT_TID "T2025120621041161827"
#define ARANEA_DEFAULT_FID "0099"
#define ARANEA_DEFAULT_TENANT_LACISID "18217487937895888001"
#define ARANEA_DEFAULT_TENANT_EMAIL "soejim@mijeos.com"
#define ARANEA_DEFAULT_TENANT_CIC "204965"

// ========================================
// WiFiデフォルト設定（SSIDとパスワードはペア）
// ========================================
#define ARANEA_DEFAULT_WIFI_SSID_1 "H_to_facility"
#define ARANEA_DEFAULT_WIFI_PASS_1 "a,9E%JJDQ&kj"
#define ARANEA_DEFAULT_WIFI_SSID_2 ""
#define ARANEA_DEFAULT_WIFI_PASS_2 ""
#define ARANEA_DEFAULT_WIFI_SSID_3 ""
#define ARANEA_DEFAULT_WIFI_PASS_3 ""
#define ARANEA_DEFAULT_WIFI_SSID_4 ""
#define ARANEA_DEFAULT_WIFI_PASS_4 ""
#define ARANEA_DEFAULT_WIFI_SSID_5 ""
#define ARANEA_DEFAULT_WIFI_PASS_5 ""
#define ARANEA_DEFAULT_WIFI_SSID_6 ""
#define ARANEA_DEFAULT_WIFI_PASS_6 ""

// ========================================
// エンドポイント設定
// ========================================
#define ARANEA_DEFAULT_GATE_URL "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate"
#define ARANEA_DEFAULT_CLOUD_URL "https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport"

// ========================================
// Relayエンドポイント（Orange Pi Zero3）
// ========================================
#define ARANEA_DEFAULT_RELAY_PRIMARY "http://192.168.96.201:8080/api/relay"
#define ARANEA_DEFAULT_RELAY_SECONDARY "http://192.168.96.202:8080/api/relay"

// ========================================
// MQTT設定
// ========================================
#define ARANEA_DEFAULT_MQTT_BROKER ""
#define ARANEA_DEFAULT_MQTT_PORT 8883
#define ARANEA_DEFAULT_MQTT_USER ""
#define ARANEA_DEFAULT_MQTT_PASS ""
#define ARANEA_DEFAULT_MQTT_TLS true

// ========================================
// BLEスキャン設定
// ========================================
#define IS02A_DEFAULT_BLE_SCAN_SEC 5    // スキャン時間（秒）
#define IS02A_DEFAULT_BLE_FILTER ""     // LacisIDプレフィックスフィルター

// ========================================
// センサー設定（HT-30）
// ========================================
#define IS02A_DEFAULT_TEMP_OFFSET 0.0   // 温度補正値
#define IS02A_DEFAULT_HUM_OFFSET 0.0    // 湿度補正値
#define IS02A_DEFAULT_SELF_INTERVAL 60  // 自己計測間隔（秒）

// ========================================
// バッチ送信設定
// ========================================
#define IS02A_DEFAULT_BATCH_INTERVAL 300 // バッチ送信間隔（秒）5分

// ========================================
// ステータスレポート設定
// ========================================
#define IS02A_DEFAULT_REPORT_INTERVAL 300  // ステータスレポート間隔（秒）

// ========================================
// リブートスケジューラ設定
// ========================================
#define IS02A_DEFAULT_REBOOT_HOUR -1    // リブート時刻（時）-1=無効
#define IS02A_DEFAULT_REBOOT_MIN 0      // リブート時刻（分）

// ========================================
// ノード蓄積設定（オンメモリ限定 - SPIFFS禁止）
// ========================================
#define IS02A_DEFAULT_MAX_NODES 32      // 最大ノード保持数（メモリ考慮）

/**
 * AraneaSettings - 静的設定クラス
 */
class AraneaSettings {
public:
  static void init();
  static void initDefaults(SettingManager& settings);

  // テナント設定
  static String getTid();
  static String getFid();
  static String getTenantLacisId();
  static String getTenantEmail();
  static String getTenantCic();

  // エンドポイント
  static String getGateUrl();
  static String getCloudUrl();
  static String getRelayPrimary();
  static String getRelaySecondary();

  // MQTT設定
  static String getMqttBroker();
  static int getMqttPort();
  static String getMqttUser();
  static String getMqttPass();
  static bool getMqttTls();

  // BLE設定
  static int getBleScanSec();
  static String getBleFilter();

  // センサー設定
  static float getTempOffset();
  static float getHumOffset();
  static int getSelfIntervalSec();

  // バッチ送信設定
  static int getBatchIntervalSec();

  // ステータスレポート設定
  static int getReportIntervalSec();

  // リブートスケジューラ設定
  static int getRebootHour();
  static int getRebootMin();

  // ノード蓄積設定
  static int getMaxNodes();

private:
  static bool _initialized;
};

#endif // ARANEA_SETTINGS_IS02A_H
