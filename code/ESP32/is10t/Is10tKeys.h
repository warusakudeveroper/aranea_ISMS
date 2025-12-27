/**
 * Is10tKeys.h
 *
 * IS10T用 NVSキー定数定義
 *
 * 【重要】NVS (Preferences) のキー長制限
 * ESP32のNVS APIはキー名を最大15文字に制限している。
 * 16文字以上のキーを使用すると、保存が無視されるか切り捨てられ、
 * 「保存したはずが反映されない」という見つけにくいバグになる。
 */

#ifndef IS10T_KEYS_H
#define IS10T_KEYS_H

// ============================================================
// NVSキー定義マクロ（15文字制限をコンパイル時に強制）
// ============================================================
#define NVS_KEY(name, value) \
  static constexpr const char name[] = value; \
  static_assert(sizeof(value) - 1 <= 15, "NVS key '" value "' exceeds 15 chars")

// ============================================================
// IS10T固有のNVSキー（全て15文字以内）
// ============================================================
namespace Is10tKeys {

  // --- CelestialGlobe設定 ---
  NVS_KEY(kEndpoint,    "is10t_endpt");    // 11文字: CelestialGlobeエンドポイント
  NVS_KEY(kSecret,      "is10t_secret");   // 12文字: CelestialGlobeシークレット

  // --- ポーリング設定 ---
  NVS_KEY(kPollIntv,    "is10t_poll");     // 10文字: ポーリングインターバル(ms)
  NVS_KEY(kTimeout,     "is10t_timeout");  // 13文字: HTTPタイムアウト(ms)

  // --- SSOT設定 ---
  NVS_KEY(kSchema,      "is10t_schema");   // 12文字: schemaVersion
  NVS_KEY(kHash,        "is10t_hash");     // 10文字: configHash
  NVS_KEY(kAppliedAt,   "is10t_applied");  // 13文字: 最終適用日時

  // --- Tapoカメラ設定 ---
  // tapo_count, tapo_X_host 等はTapoPollerIs10tで直接定義

}  // namespace Is10tKeys

// ============================================================
// 共通キー（is10と共通、CommonKeysを使用）
// ============================================================
namespace CommonKeys {

  NVS_KEY(kTid,           "tid");           // 3文字:  テナントID
  NVS_KEY(kFid,           "fid");           // 3文字:  施設ID
  NVS_KEY(kCic,           "cic");           // 3文字:  CICコード
  NVS_KEY(kDeviceName,    "device_name");   // 11文字: デバイス名
  NVS_KEY(kTenantLacis,   "tenant_lacis");  // 12文字: テナントLacisID
  NVS_KEY(kGateUrl,       "gate_url");      // 8文字:  araneaDeviceGate URL

}  // namespace CommonKeys

#endif // IS10T_KEYS_H
