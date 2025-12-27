/**
 * AraneaSettings.h (is05a用)
 *
 * IS05A専用 araneaDevice設定クラス
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

#ifndef ARANEA_SETTINGS_IS05A_H
#define ARANEA_SETTINGS_IS05A_H

#include <Arduino.h>

class SettingManager;

// ========================================
// デフォルト設定（施設展開時に編集）
// is02aから転用
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
#define ARANEA_DEFAULT_RELAY_PRIMARY "http://192.168.96.201:8080/api/events"
#define ARANEA_DEFAULT_RELAY_SECONDARY "http://192.168.96.202:8080/api/events"

// ========================================
// MQTT設定
// ========================================
#define ARANEA_DEFAULT_MQTT_BROKER "wss://aranea-mqtt-bridge-1010551946141.asia-northeast1.run.app"
#define ARANEA_DEFAULT_MQTT_PORT 443
#define ARANEA_DEFAULT_MQTT_USER ""
#define ARANEA_DEFAULT_MQTT_PASS ""
#define ARANEA_DEFAULT_MQTT_TLS true

// ========================================
// is05a チャンネルデフォルト設定
// ========================================
// GPIO割り当て
#define IS05A_DEFAULT_CH1_PIN 4
#define IS05A_DEFAULT_CH2_PIN 5
#define IS05A_DEFAULT_CH3_PIN 13
#define IS05A_DEFAULT_CH4_PIN 14
#define IS05A_DEFAULT_CH5_PIN 16
#define IS05A_DEFAULT_CH6_PIN 17
#define IS05A_DEFAULT_CH7_PIN 18
#define IS05A_DEFAULT_CH8_PIN 19

// デフォルトデバウンス(ms) 10-10000
#define IS05A_DEFAULT_DEBOUNCE_MS 100

// デフォルト心拍間隔(秒)
#define IS05A_DEFAULT_HEARTBEAT_SEC 60

// 起動猶予期間(ms)
#define IS05A_DEFAULT_BOOT_GRACE_MS 5000

// ========================================
// 出力モード設定
// ========================================
// 出力モード: momentary(0), alternate(1), interval(2)
#define IS05A_OUTPUT_MODE_MOMENTARY 0
#define IS05A_OUTPUT_MODE_ALTERNATE 1
#define IS05A_OUTPUT_MODE_INTERVAL 2

// デフォルト出力時間(ms) 500-10000
#define IS05A_DEFAULT_OUTPUT_DURATION_MS 3000

// デフォルトインターバル回数
#define IS05A_DEFAULT_INTERVAL_COUNT 3

// ========================================
// QuietMode設定（おやすみモード）
// Webhook送信を抑制する時間帯
// ========================================
// 開始時刻（時:分、例: 2200 = 22:00）
#define IS05A_DEFAULT_QUIET_START 0
// 終了時刻（時:分、例: 0700 = 07:00）
#define IS05A_DEFAULT_QUIET_END 0
// QuietMode有効フラグ
#define IS05A_DEFAULT_QUIET_ENABLED false

// ========================================
// ステータスレポート設定
// ========================================
#define IS05A_DEFAULT_REPORT_INTERVAL 300  // ステータスレポート間隔（秒）

// ========================================
// リブートスケジューラ設定
// ========================================
#define IS05A_DEFAULT_REBOOT_HOUR -1    // リブート時刻（時）-1=無効
#define IS05A_DEFAULT_REBOOT_MIN 0      // リブート時刻（分）

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

  // デバイス設定
  static int getDefaultPin(int ch);
  static int getDefaultDebounceMs();
  static int getDefaultHeartbeatSec();
  static int getDefaultBootGraceMs();

  // 出力モード設定
  static int getDefaultOutputMode();
  static int getDefaultOutputDurationMs();
  static int getDefaultIntervalCount();

  // QuietMode設定
  static int getDefaultQuietStart();
  static int getDefaultQuietEnd();
  static bool getDefaultQuietEnabled();

  // ステータスレポート設定
  static int getReportIntervalSec();

  // リブートスケジューラ設定
  static int getRebootHour();
  static int getRebootMin();

private:
  static bool _initialized;
};

#endif // ARANEA_SETTINGS_IS05A_H
