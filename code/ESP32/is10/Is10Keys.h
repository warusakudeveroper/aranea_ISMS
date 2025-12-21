/**
 * Is10Keys.h
 *
 * IS10用 NVSキー定数定義
 *
 * 【重要】NVS (Preferences) のキー長制限
 * ESP32のNVS APIはキー名を最大15文字に制限している。
 * 16文字以上のキーを使用すると、保存が無視されるか切り捨てられ、
 * 「保存したはずが反映されない」という見つけにくいバグになる。
 *
 * このファイルでは:
 * 1. 全てのNVSキーを定数として一元管理
 * 2. static_assertでコンパイル時に15文字超過を検出
 * 3. 将来の追加時も同じルールを強制
 */

#ifndef IS10_KEYS_H
#define IS10_KEYS_H

// ============================================================
// NVSキー定義マクロ（15文字制限をコンパイル時に強制）
// ============================================================
#define NVS_KEY(name, value) \
  static constexpr const char name[] = value; \
  static_assert(sizeof(value) - 1 <= 15, "NVS key '" value "' exceeds 15 chars")

// ============================================================
// IS10固有のNVSキー（全て15文字以内）
// ============================================================
namespace Is10Keys {

  // --- SSH/ポーリング設定 ---
  NVS_KEY(kEndpoint,    "is10_endpoint");   // 13文字: CelestialGlobeエンドポイント
  NVS_KEY(kTimeout,     "is10_timeout");    // 12文字: SSHタイムアウト(ms)
  NVS_KEY(kRetry,       "is10_retry");      // 10文字: リトライ回数
  NVS_KEY(kRtrIntv,     "is10_rtr_intv");   // 13文字: ルーター間インターバル(ms)
  NVS_KEY(kInterval,    "is10_interval");   // 13文字: 巡回インターバル(秒)
  NVS_KEY(kSecret,      "is10_secret");     // 11文字: CelestialGlobeシークレット

  // --- SSOT設定（MQTT config適用状態）---
  NVS_KEY(kSchema,      "is10_schema");     // 11文字: schemaVersion
  NVS_KEY(kHash,        "is10_hash");       // 9文字:  configHash
  NVS_KEY(kAppliedAt,   "is10_applied");    // 12文字: 最終適用日時
  NVS_KEY(kApplyErr,    "is10_aperr");      // 10文字: 適用エラー

  // --- レポート設定 ---
  NVS_KEY(kReportClnt,  "is10_clients");    // 12文字: クライアントリスト報告フラグ
  NVS_KEY(kReportFull,  "is10_rpt_full");   // 13文字: フル設定報告フラグ
  NVS_KEY(kScanIntv,    "is10_scan_sec");   // 12文字: スキャンインターバル(秒)

  // --- LacisID設定 ---
  NVS_KEY(kLacisPrefix, "is10_lpfx");       // 9文字:  LacisIDプレフィックス
  NVS_KEY(kRouterPtype, "is10_rptype");     // 11文字: ルータープロダクトタイプ
  NVS_KEY(kRouterPcode, "is10_rpcode");     // 11文字: ルータープロダクトコード

}  // namespace Is10Keys

// ============================================================
// 共通キー（他デバイスでも使用、将来global移動候補）
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

  // タイミング
  NVS_KEY(kSshTimeout,    "ssh_timeout");   // 11文字: SSHタイムアウト
  NVS_KEY(kPollInterval,  "poll_interval"); // 13文字: ポーリングインターバル
  NVS_KEY(kRetryCount,    "retry_count");   // 11文字: リトライ回数

}  // namespace CommonKeys

#endif // IS10_KEYS_H
