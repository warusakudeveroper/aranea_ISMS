/**
 * AraneaSettings.h (is06a用)
 *
 * IS06A専用 araneaDevice設定クラス
 *
 * 【このファイルの目的】
 * 大量生産（施設展開）時に、同一ファームウェアを複数台に書き込むための
 * デフォルト設定を定義するクラス。
 *
 * IS06A仕様（6CH構成）:
 * - TRG1-4 (GPIO 12,14,27,15): デジタル出力 + PWM対応
 * - TRG5-6 (GPIO 05,18): I/O切替可能（出力 or 入力）
 *
 * ストラップピン注意:
 * - GPIO12(MTDI): ブート時LOW必要。内部プルダウン or 外部プルダウン推奨
 * - GPIO15(MTDO): ブート時HIGH必要。内部プルアップ or 外部プルアップ推奨
 * - GPIO5: ブート時ログレベル制御。通常問題なし
 * - is04aでGPIO12/14の運用実績あり
 */

#ifndef ARANEA_SETTINGS_IS06A_H
#define ARANEA_SETTINGS_IS06A_H

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
// is06a GPIO割り当て（6CH構成）
// ========================================
// TRG1-4: PWM対応トリガー出力
#define IS06A_GPIO_TRG1 12   // GPIO12 (ストラップ: LOW必要)
#define IS06A_GPIO_TRG2 14   // GPIO14
#define IS06A_GPIO_TRG3 27   // GPIO27
#define IS06A_GPIO_TRG4 15   // GPIO15 (ストラップ: HIGH必要)

// TRG5-6: I/O切替可能
#define IS06A_GPIO_TRG5 5    // GPIO5 (ストラップ: 軽微)
#define IS06A_GPIO_TRG6 18   // GPIO18

// ========================================
// is06a トリガー共通設定
// ========================================
#define IS06A_DEFAULT_INTERLOCK_MS 200    // インターロック時間(ms)

// ========================================
// is06a TRG1-4設定（PWM対応）
// ========================================
#define IS06A_DEFAULT_TRG_MODE "digital"  // "digital" or "pwm"
#define IS06A_DEFAULT_PULSE_MS 3000       // パルス幅(ms)
#define IS06A_DEFAULT_PWM_FREQ 1000       // PWM周波数(Hz)
#define IS06A_DEFAULT_PWM_DUTY 128        // PWMデューティ(0-255)

// ========================================
// is06a TRG5-6設定（I/O切替）
// ========================================
#define IS06A_DEFAULT_IO_MODE "output"    // "input" or "output"
#define IS06A_DEFAULT_DEBOUNCE_MS 100     // デバウンス(ms)

// ========================================
// is06a トリガー名称設定（6CH）
// ========================================
#define IS06A_DEFAULT_TRG1_NAME "TRG1"
#define IS06A_DEFAULT_TRG2_NAME "TRG2"
#define IS06A_DEFAULT_TRG3_NAME "TRG3"
#define IS06A_DEFAULT_TRG4_NAME "TRG4"
#define IS06A_DEFAULT_TRG5_NAME "TRG5"
#define IS06A_DEFAULT_TRG6_NAME "TRG6"

// ========================================
// 動作設定
// ========================================
#define IS06A_DEFAULT_HEARTBEAT_SEC 60
#define IS06A_DEFAULT_BOOT_GRACE_MS 3000

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
  static int getGpioTrg(int trgNum);  // 1-6

  // トリガー共通設定
  static int getDefaultInterlockMs();

  // TRG1-4設定（PWM対応）
  static String getDefaultTrgMode();
  static int getDefaultPulseMs();
  static int getDefaultPwmFreq();
  static int getDefaultPwmDuty();

  // TRG5-6設定（I/O切替）
  static String getDefaultIoMode();
  static int getDefaultDebounceMs();

  // トリガー名称
  static String getDefaultTrgName(int trgNum);  // 1-6

  // 動作設定
  static int getDefaultHeartbeatSec();
  static int getDefaultBootGraceMs();

private:
  static bool _initialized;
};

#endif // ARANEA_SETTINGS_IS06A_H
