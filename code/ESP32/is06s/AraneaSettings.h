/**
 * AraneaSettings.h (is06s用)
 *
 * IS06S専用 araneaDevice設定クラス
 *
 * 【このファイルの目的】
 * 大量生産（施設展開）時に、同一ファームウェアを複数台に書き込むための
 * デフォルト設定を定義するクラス。
 *
 * 【標準運用フロー】
 * 1. 通常: APモード → Web UIからWiFi設定
 * 2. 大量展開: このファイルで初期値定義 → initDefaults()で一括設定
 *
 * IS06S仕様（6CH構成）:
 * - CH1-4 (GPIO 18,05,15,27): D/P Type (Digital/PWM切替)
 * - CH5-6 (GPIO 14,12): I/O Type (Input/Output切替)
 */

#ifndef ARANEA_SETTINGS_IS06S_H
#define ARANEA_SETTINGS_IS06S_H

#include <Arduino.h>

class SettingManager;

// ========================================
// テスト環境設定（DesignerInstructions.md参照）
// ========================================
// WiFi設定
#define ARANEA_DEFAULT_WIFI_SSID_1 "sorapia_facility_wifi"
#define ARANEA_DEFAULT_WIFI_PASS_1 "JYuDzjhi"
#define ARANEA_DEFAULT_WIFI_SSID_2 ""
#define ARANEA_DEFAULT_WIFI_PASS_2 ""
#define ARANEA_DEFAULT_WIFI_SSID_3 ""
#define ARANEA_DEFAULT_WIFI_PASS_3 ""

// テナント設定
#define ARANEA_DEFAULT_TID "T2025120621041161827"
#define ARANEA_DEFAULT_FID "0100"
#define ARANEA_DEFAULT_TENANT_LACISID "18217487937895888001"
#define ARANEA_DEFAULT_TENANT_EMAIL "soejim@mijeos.com"
#define ARANEA_DEFAULT_TENANT_CIC "204965"

// ========================================
// エンドポイント設定
// ========================================
#define ARANEA_DEFAULT_GATE_URL "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate"
#define ARANEA_DEFAULT_CLOUD_URL "https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport"

// ========================================
// Relayエンドポイント（Orange Pi Zero3）
// ========================================
#define ARANEA_DEFAULT_RELAY_PRIMARY ""
#define ARANEA_DEFAULT_RELAY_SECONDARY ""

// ========================================
// 動作設定
// ========================================
#define IS06S_DEFAULT_HEARTBEAT_SEC 60
#define IS06S_DEFAULT_BOOT_GRACE_MS 3000

/**
 * AraneaSettings - 静的設定クラス
 *
 * 使用方法:
 * setup()内で settings.begin() の後に
 * AraneaSettings::initDefaults(settings) を呼び出す
 */
class AraneaSettings {
public:
  /**
   * 初期化
   */
  static void init();

  /**
   * NVSにデフォルト値を書き込む（未設定の場合のみ）
   * @param settings SettingManager参照
   */
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

  // 動作設定
  static int getDefaultHeartbeatSec();
  static int getDefaultBootGraceMs();

private:
  static bool _initialized;
};

#endif // ARANEA_SETTINGS_IS06S_H
