/**
 * AraneaSettings.h
 *
 * ============================================================
 *  !!!!! 重要 !!!!!  IS10専用 araneaDevice設定クラス
 * ============================================================
 *
 * 【このファイルの目的】
 * 大量生産（施設展開）時に、同一ファームウェアを複数台に書き込むための
 * デフォルト設定を定義するクラス。
 *
 * 【大量生産モード（推奨）】
 * 1. このファイルの ARANEA_DEFAULT_* 定数を施設用に編集
 * 2. ビルド → 複数台に書き込み
 * 3. 各デバイスは起動時にNVSが空なら、ここのデフォルト値を自動適用
 * 4. tid/fid/tenant認証は全台共通、MACベースのlacisIdで個体識別
 *
 * 【汎用モード（開発・テスト用）】
 * 1. APモード起動（GPIO0長押し）
 * 2. Web UI（http://192.168.4.1）から設定変更
 * 3. NVS/SPIFFSに保存され、次回起動時に反映
 *
 * 【注意事項】
 * - tid/fidは施設ごとに異なる。本番展開前に必ず確認すること
 * - tenant認証（lacisId/email/cic）は契約テナントのPrimary情報
 * - WiFi SSIDは cluster1-6 がデフォルト（離島VPN環境用）
 * - パーティションスキーム: min_spiffs を使用すること（huge_app禁止）
 *
 * 【モジュール構成】
 * - SettingManager (global/src) = 汎用NVS CRUD操作
 * - AraneaSettings (device-specific) = デバイス固有のデフォルト設定
 *
 * @see docs/is10/is10_configuration_manual.md
 */

#ifndef ARANEA_SETTINGS_H
#define ARANEA_SETTINGS_H

#include <Arduino.h>

// Forward declaration
class SettingManager;

// ========================================
// IS10専用デフォルト設定
// ========================================
//
// 【施設展開時の編集箇所】
// 以下の定数を対象施設の値に変更してビルドする
//
// TID: テナントID（mobes2.0で発行、Tで始まる19桁）
// FID: 施設ID（4桁数字、テナント内で一意）
// TENANT_*: 契約テナントのPrimary認証情報
//           - mobes2.0管理画面で確認可能
//           - デバイス登録時に使用される
//
// ※本番環境では必ず施設固有の値に変更すること
// ※以下はデフォルト値（開発・テスト用）
//
#define ARANEA_DEFAULT_TID "T2025120621041161827"
#define ARANEA_DEFAULT_FID "0150"
#define ARANEA_DEFAULT_TENANT_LACISID "18217487937895888001"
#define ARANEA_DEFAULT_TENANT_EMAIL "soejim@mijeos.com"
#define ARANEA_DEFAULT_TENANT_CIC "204965"
// TENANT_PASSは廃止（認証はlacisId + userId + cicの3要素）

// ========================================
// WiFiデフォルト設定
// ========================================
// 離島VPN環境では cluster1-6 を使用
// 施設によっては独自SSIDに変更が必要
//
#define ARANEA_DEFAULT_WIFI_SSID_1 "cluster1"
#define ARANEA_DEFAULT_WIFI_SSID_2 "cluster2"
#define ARANEA_DEFAULT_WIFI_SSID_3 "cluster3"
#define ARANEA_DEFAULT_WIFI_SSID_4 "cluster4"
#define ARANEA_DEFAULT_WIFI_SSID_5 "cluster5"
#define ARANEA_DEFAULT_WIFI_SSID_6 "cluster6"
#define ARANEA_DEFAULT_WIFI_PASS "isms12345@"

// ========================================
// AraneaGateway設定（クラウドエンドポイント）
// ========================================
// これらのURLは通常変更不要
// - araneaDeviceGate: デバイス登録API（CIC発行）
// - deviceStateReport: 状態レポート送信先
//
#define ARANEA_DEFAULT_GATE_URL "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate"
#define ARANEA_DEFAULT_CLOUD_URL "https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport"

// ========================================
// Relayエンドポイント（Orange Pi Zero3）
// ========================================
// ローカルネットワーク内のZero3中継サーバー
// 施設によってはIPアドレスが異なる場合あり
//
#define ARANEA_DEFAULT_RELAY_PRIMARY "192.168.96.201"
#define ARANEA_DEFAULT_RELAY_SECONDARY "192.168.96.202"
#define ARANEA_DEFAULT_RELAY_PORT 8080

/**
 * テナント認証情報構造体
 */
struct TenantAuth {
  String tid;
  String fid;
  String lacisId;
  String email;
  String cic;
  String pass;
};

/**
 * WiFi設定構造体
 */
struct WiFiConfig {
  String ssid[6];
  String password;
};

/**
 * AraneaSettings - 静的設定クラス
 *
 * 使用例:
 *   SettingManager settings;
 *   settings.begin("isms");
 *   AraneaSettings::init();                    // SPIFFS設定初期化
 *   AraneaSettings::initDefaults(settings);   // NVSデフォルト設定
 *   String tid = AraneaSettings::getTid();
 */
class AraneaSettings {
public:
  // 初期化（SPIFFS設定をロード）
  static void init();

  /**
   * NVS（SettingManager）にデバイス固有のデフォルト値を設定
   * 未設定のキーのみデフォルト値を投入
   * @param settings SettingManagerインスタンス
   */
  static void initDefaults(SettingManager& settings);

  // SPIFFSから設定を読み込み（既存設定があれば上書き）
  static bool loadFromSPIFFS();

  // SPIFFSに設定を保存
  static bool saveToSPIFFS();

  // テナント設定
  static String getTid();
  static void setTid(const String& tid);

  static String getFid();
  static void setFid(const String& fid);

  static TenantAuth getTenantAuth();
  static void setTenantAuth(const TenantAuth& auth);

  // WiFi設定
  static WiFiConfig getWiFiConfig();
  static void setWiFiConfig(const WiFiConfig& config);
  static void setWiFiSSID(int index, const String& ssid);
  static void setWiFiPassword(const String& pass);

  // エンドポイント設定
  static String getGateUrl();
  static void setGateUrl(const String& url);

  static String getCloudUrl();
  static void setCloudUrl(const String& url);

  // Relay設定
  static String getRelayPrimary();
  static String getRelaySecondary();
  static int getRelayPort();

  // 設定リセット（デフォルトに戻す）
  static void resetToDefaults();

  // JSON出力（HTTP API用）
  static String toJson();

  // JSONから設定（HTTP API用）
  static bool fromJson(const String& json);

private:
  static bool _initialized;
  static TenantAuth _tenantAuth;
  static WiFiConfig _wifiConfig;
  static String _gateUrl;
  static String _cloudUrl;
  static String _relayPrimary;
  static String _relaySecondary;
  static int _relayPort;
};

#endif // ARANEA_SETTINGS_H
