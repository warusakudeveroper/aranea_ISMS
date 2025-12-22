/**
 * Is04aKeys.h
 *
 * IS04A用 NVSキー定数定義
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

#ifndef IS04A_KEYS_H
#define IS04A_KEYS_H

// ============================================================
// NVSキー定義マクロ（15文字制限をコンパイル時に強制）
// ============================================================
#define NVS_KEY(name, value) \
  static constexpr const char name[] = value; \
  static_assert(sizeof(value) - 1 <= 15, "NVS key '" value "' exceeds 15 chars")

// ============================================================
// IS04A固有のNVSキー（全て15文字以内）
// ============================================================
namespace Is04aKeys {

  // --- 出力設定 ---
  NVS_KEY(kOut1Name,    "is04_out1_nm");  // 12文字: 出力1の名前
  NVS_KEY(kOut2Name,    "is04_out2_nm");  // 12文字: 出力2の名前
  NVS_KEY(kPulseMs,     "is04_pls_ms");   // 11文字: パルス幅(ms)
  NVS_KEY(kInterlockMs, "is04_intrlk");   // 11文字: インターロック時間(ms)

  // --- 入力設定 ---
  NVS_KEY(kIn1Pin,      "is04_in1_pin");  // 12文字: 入力1のGPIO
  NVS_KEY(kIn2Pin,      "is04_in2_pin");  // 12文字: 入力2のGPIO
  NVS_KEY(kIn1Target,   "is04_in1_tgt");  // 12文字: 入力1のターゲット出力(1 or 2)
  NVS_KEY(kIn2Target,   "is04_in2_tgt");  // 12文字: 入力2のターゲット出力(1 or 2)
  NVS_KEY(kDebounceMs,  "is04_deb_ms");   // 11文字: デバウンス時間(ms)

  // --- 出力ピン設定 ---
  NVS_KEY(kOut1Pin,     "is04_o1_pin");   // 11文字: 出力1のGPIO
  NVS_KEY(kOut2Pin,     "is04_o2_pin");   // 11文字: 出力2のGPIO

  // --- MQTT設定 ---
  NVS_KEY(kMqttUrl,     "is04_mqtt");     // 9文字: MQTTブローカーURL
  NVS_KEY(kMqttUser,    "is04_mquser");   // 11文字: MQTTユーザー
  NVS_KEY(kMqttPass,    "is04_mqpass");   // 11文字: MQTTパスワード

  // --- 動作設定 ---
  NVS_KEY(kHeartbeat,   "is04_hb_sec");   // 11文字: 心拍間隔(秒)
  NVS_KEY(kBootGrace,   "is04_boot_g");   // 11文字: 起動猶予期間(ms)

}  // namespace Is04aKeys

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

#endif // IS04A_KEYS_H
