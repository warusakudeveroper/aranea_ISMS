#line 1 "/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/aranea_ISMS/code/ESP32/is04a/AraneaSettings.h"
/**
 * AraneaSettings.h (is04a用)
 *
 * IS04A専用 araneaDevice設定クラス
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

#ifndef ARANEA_SETTINGS_IS04A_H
#define ARANEA_SETTINGS_IS04A_H

#include <Arduino.h>

class SettingManager;

// ========================================
// デフォルト設定（施設展開時に編集）
// ========================================
#define ARANEA_DEFAULT_TID "T2025120608261484221"
#define ARANEA_DEFAULT_FID "9000"
#define ARANEA_DEFAULT_TENANT_LACISID "12767487939173857894"
#define ARANEA_DEFAULT_TENANT_EMAIL "info+ichiyama@neki.tech"
#define ARANEA_DEFAULT_TENANT_CIC "263238"

// ========================================
// WiFiデフォルト設定
// ========================================
#define ARANEA_DEFAULT_WIFI_SSID_1 "cluster1"
#define ARANEA_DEFAULT_WIFI_SSID_2 "cluster2"
#define ARANEA_DEFAULT_WIFI_SSID_3 "cluster3"
#define ARANEA_DEFAULT_WIFI_PASS "isms12345@"

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
// is04a GPIO割り当て
// ========================================
// 物理入力（手動トリガー）
#define IS04A_DEFAULT_IN1_PIN 5    // GPIO5
#define IS04A_DEFAULT_IN2_PIN 18   // GPIO18

// トリガー出力（接点出力）
#define IS04A_DEFAULT_OUT1_PIN 12  // GPIO12
#define IS04A_DEFAULT_OUT2_PIN 14  // GPIO14

// ========================================
// is04a パルス設定
// ========================================
#define IS04A_DEFAULT_PULSE_MS 3000       // パルス幅(ms)
#define IS04A_DEFAULT_INTERLOCK_MS 200    // インターロック時間(ms)
#define IS04A_DEFAULT_DEBOUNCE_MS 50      // デバウンス(ms)

// ========================================
// is04a トリガーアサイン設定
// ========================================
#define IS04A_DEFAULT_IN1_TARGET 1  // 入力1 → 出力1
#define IS04A_DEFAULT_IN2_TARGET 2  // 入力2 → 出力2

// ========================================
// is04a 出力名称設定
// ========================================
#define IS04A_DEFAULT_OUT1_NAME "OPEN"
#define IS04A_DEFAULT_OUT2_NAME "CLOSE"

// ========================================
// 動作設定
// ========================================
#define IS04A_DEFAULT_HEARTBEAT_SEC 60
#define IS04A_DEFAULT_BOOT_GRACE_MS 3000

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

  // GPIO設定
  static int getDefaultIn1Pin();
  static int getDefaultIn2Pin();
  static int getDefaultOut1Pin();
  static int getDefaultOut2Pin();

  // パルス設定
  static int getDefaultPulseMs();
  static int getDefaultInterlockMs();
  static int getDefaultDebounceMs();

  // トリガーアサイン
  static int getDefaultIn1Target();
  static int getDefaultIn2Target();

  // 出力名称
  static String getDefaultOut1Name();
  static String getDefaultOut2Name();

  // 動作設定
  static int getDefaultHeartbeatSec();
  static int getDefaultBootGraceMs();

private:
  static bool _initialized;
};

#endif // ARANEA_SETTINGS_IS04A_H
