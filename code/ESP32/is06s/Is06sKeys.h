/**
 * Is06sKeys.h
 *
 * IS06S用 NVSキー定数定義
 *
 * 【重要】NVS (Preferences) のキー長制限
 * ESP32のNVS APIはキー名を最大15文字に制限している。
 * 16文字以上のキーを使用すると、保存が無視されるか切り捨てられ、
 * 「保存したはずが反映されない」という見つけにくいバグになる。
 *
 * このファイルでは:
 * 1. 全てのNVSキーを定数として一元管理
 * 2. static_assertでコンパイル時に15文字超過を検出
 * 3. PIN関連キーはAraneaSettingsDefaults.hで定義済み
 *
 * IS06S仕様（6CH構成）:
 * - CH1-4 (GPIO 18,05,15,27): D/P Type (Digital/PWM切替)
 * - CH5-6 (GPIO 14,12): I/O Type (Input/Output切替)
 */

#ifndef IS06S_KEYS_H
#define IS06S_KEYS_H

// ============================================================
// NVSキー定義マクロ（15文字制限をコンパイル時に強制）
// ============================================================
#define NVS_KEY(name, value) \
  static constexpr const char name[] = value; \
  static_assert(sizeof(value) - 1 <= 15, "NVS key '" value "' exceeds 15 chars")

// ============================================================
// IS06S固有のNVSキー（全て15文字以内）
// ============================================================
namespace Is06sKeys {

  // --- 動作設定 ---
  NVS_KEY(kHeartbeat,   "is06s_hb_sec");  // 12文字: 心拍間隔(秒)
  NVS_KEY(kBootGrace,   "is06s_boot_g");  // 12文字: 起動猶予期間(ms)

  // --- CH設定キープレフィックス ---
  // 実際のキーは AraneaSettingsDefaults.h の NVSKeys namespace で定義
  // 例: ch1_en, ch1_type, ch2_en, ch2_type, ...

}  // namespace Is06sKeys

// ============================================================
// 共通キー（他デバイスでも使用）
// ============================================================
namespace CommonKeys {

  NVS_KEY(kTid,           "tid");           // 3文字:  テナントID
  NVS_KEY(kFid,           "fid");           // 3文字:  施設ID
  NVS_KEY(kCic,           "cic");           // 3文字:  CICコード
  NVS_KEY(kDeviceName,    "device_name");   // 11文字: デバイス名
  NVS_KEY(kTenantLacis,   "tenant_lacis");  // 12文字: テナントLacisID
  NVS_KEY(kTenantEmail,   "tenant_email");  // 12文字: テナントEmail
  NVS_KEY(kTenantCic,     "tenant_cic");    // 10文字: テナントCIC

  // エンドポイント
  NVS_KEY(kRelayPri,      "relay_pri");     // 9文字:  プライマリリレー
  NVS_KEY(kRelaySec,      "relay_sec");     // 9文字:  セカンダリリレー
  NVS_KEY(kGateUrl,       "gate_url");      // 8文字:  araneaDeviceGate URL
  NVS_KEY(kCloudUrl,      "cloud_url");     // 9文字:  クラウドURL

  // WiFi設定（AraneaWebUIと共通）
  // wifi_ssid1 ~ wifi_ssid6, wifi_pass1 ~ wifi_pass6

}  // namespace CommonKeys

#endif // IS06S_KEYS_H
