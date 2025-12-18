/**
 * AraneaSettings.h
 *
 * IS10専用 araneaDevice設定クラス
 * tid, fid, テナント認証情報などを静的に管理
 *
 * 【大量生産モード】このファイルを施設用に編集してビルド
 * 【汎用モード】APモードのWeb UIから設定
 */

#ifndef ARANEA_SETTINGS_H
#define ARANEA_SETTINGS_H

#include <Arduino.h>

// ========================================
// IS10専用デフォルト設定
// ========================================
#define ARANEA_DEFAULT_TID "T2025120621041161827"
#define ARANEA_DEFAULT_FID "0150"
#define ARANEA_DEFAULT_TENANT_LACISID "18217487937895888001"
#define ARANEA_DEFAULT_TENANT_EMAIL "soejim@mijeos.com"
#define ARANEA_DEFAULT_TENANT_CIC "204965"
// TENANT_PASSは廃止（認証はlacisId + userId + cicの3要素）

// WiFiデフォルト設定
#define ARANEA_DEFAULT_WIFI_SSID_1 "cluster1"
#define ARANEA_DEFAULT_WIFI_SSID_2 "cluster2"
#define ARANEA_DEFAULT_WIFI_SSID_3 "cluster3"
#define ARANEA_DEFAULT_WIFI_SSID_4 "cluster4"
#define ARANEA_DEFAULT_WIFI_SSID_5 "cluster5"
#define ARANEA_DEFAULT_WIFI_SSID_6 "cluster6"
#define ARANEA_DEFAULT_WIFI_PASS "isms12345@"

// AraneaGateway設定
#define ARANEA_DEFAULT_GATE_URL "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate"
#define ARANEA_DEFAULT_CLOUD_URL "https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport"

// Relayエンドポイント（Zero3）
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
 *   AraneaSettings::init();  // デフォルト設定で初期化
 *   String tid = AraneaSettings::getTid();
 *   AraneaSettings::setTid("T12345...");  // 変更
 */
class AraneaSettings {
public:
  // 初期化（デフォルト設定をロード）
  static void init();

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
