/**
 * Is06aKeys.h
 *
 * IS06A用 NVSキー定数定義
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
 *
 * IS06A仕様（6CH構成）:
 * - TRG1-4 (GPIO 12,14,27,15): デジタル出力 + PWM対応
 * - TRG5-6 (GPIO 05,18): I/O切替可能（出力 or 入力）
 */

#ifndef IS06A_KEYS_H
#define IS06A_KEYS_H

// ============================================================
// NVSキー定義マクロ（15文字制限をコンパイル時に強制）
// ============================================================
#define NVS_KEY(name, value) \
  static constexpr const char name[] = value; \
  static_assert(sizeof(value) - 1 <= 15, "NVS key '" value "' exceeds 15 chars")

// ============================================================
// IS06A固有のNVSキー（全て15文字以内）
// ============================================================
namespace Is06aKeys {

  // --- トリガー共通設定 ---
  NVS_KEY(kInterlockMs, "is06_intrlk");   // 11文字: インターロック時間(ms)

  // --- TRG1設定 (GPIO 12, PWM対応) ---
  NVS_KEY(kTrg1Name,    "is06_t1_nm");    // 10文字: 名称
  NVS_KEY(kTrg1Mode,    "is06_t1_mode");  // 12文字: "digital" or "pwm"
  NVS_KEY(kTrg1PulseMs, "is06_t1_pls");   // 11文字: パルス幅(ms)
  NVS_KEY(kTrg1PwmFreq, "is06_t1_freq");  // 12文字: PWM周波数(Hz)
  NVS_KEY(kTrg1PwmDuty, "is06_t1_duty");  // 12文字: PWMデューティ(0-255)

  // --- TRG2設定 (GPIO 14, PWM対応) ---
  NVS_KEY(kTrg2Name,    "is06_t2_nm");    // 10文字: 名称
  NVS_KEY(kTrg2Mode,    "is06_t2_mode");  // 12文字: "digital" or "pwm"
  NVS_KEY(kTrg2PulseMs, "is06_t2_pls");   // 11文字: パルス幅(ms)
  NVS_KEY(kTrg2PwmFreq, "is06_t2_freq");  // 12文字: PWM周波数(Hz)
  NVS_KEY(kTrg2PwmDuty, "is06_t2_duty");  // 12文字: PWMデューティ(0-255)

  // --- TRG3設定 (GPIO 27, PWM対応) ---
  NVS_KEY(kTrg3Name,    "is06_t3_nm");    // 10文字: 名称
  NVS_KEY(kTrg3Mode,    "is06_t3_mode");  // 12文字: "digital" or "pwm"
  NVS_KEY(kTrg3PulseMs, "is06_t3_pls");   // 11文字: パルス幅(ms)
  NVS_KEY(kTrg3PwmFreq, "is06_t3_freq");  // 12文字: PWM周波数(Hz)
  NVS_KEY(kTrg3PwmDuty, "is06_t3_duty");  // 12文字: PWMデューティ(0-255)

  // --- TRG4設定 (GPIO 15, PWM対応) ---
  NVS_KEY(kTrg4Name,    "is06_t4_nm");    // 10文字: 名称
  NVS_KEY(kTrg4Mode,    "is06_t4_mode");  // 12文字: "digital" or "pwm"
  NVS_KEY(kTrg4PulseMs, "is06_t4_pls");   // 11文字: パルス幅(ms)
  NVS_KEY(kTrg4PwmFreq, "is06_t4_freq");  // 12文字: PWM周波数(Hz)
  NVS_KEY(kTrg4PwmDuty, "is06_t4_duty");  // 12文字: PWMデューティ(0-255)

  // --- TRG5設定 (GPIO 05, I/O切替可能) ---
  NVS_KEY(kTrg5Name,    "is06_t5_nm");    // 10文字: 名称
  NVS_KEY(kTrg5IoMode,  "is06_t5_io");    // 10文字: "input" or "output"
  NVS_KEY(kTrg5PulseMs, "is06_t5_pls");   // 11文字: パルス幅(ms) (出力時)
  NVS_KEY(kTrg5Debounce,"is06_t5_deb");   // 11文字: デバウンス(ms) (入力時)

  // --- TRG6設定 (GPIO 18, I/O切替可能) ---
  NVS_KEY(kTrg6Name,    "is06_t6_nm");    // 10文字: 名称
  NVS_KEY(kTrg6IoMode,  "is06_t6_io");    // 10文字: "input" or "output"
  NVS_KEY(kTrg6PulseMs, "is06_t6_pls");   // 11文字: パルス幅(ms) (出力時)
  NVS_KEY(kTrg6Debounce,"is06_t6_deb");   // 11文字: デバウンス(ms) (入力時)

  // --- 動作設定 ---
  NVS_KEY(kHeartbeat,   "is06_hb_sec");   // 11文字: 心拍間隔(秒)
  NVS_KEY(kBootGrace,   "is06_boot_g");   // 11文字: 起動猶予期間(ms)

}  // namespace Is06aKeys

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

}  // namespace CommonKeys

#endif // IS06A_KEYS_H
