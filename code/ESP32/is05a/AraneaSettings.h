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

// デフォルトデバウンス(ms)
#define IS05A_DEFAULT_DEBOUNCE_MS 100

// デフォルト心拍間隔(秒)
#define IS05A_DEFAULT_HEARTBEAT_SEC 60

// 起動猶予期間(ms)
#define IS05A_DEFAULT_BOOT_GRACE_MS 5000

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

  // デバイス設定
  static int getDefaultPin(int ch);
  static int getDefaultDebounceMs();
  static int getDefaultHeartbeatSec();
  static int getDefaultBootGraceMs();

private:
  static bool _initialized;
};

#endif // ARANEA_SETTINGS_IS05A_H
