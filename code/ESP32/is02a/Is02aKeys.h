/**
 * Is02aKeys.h
 *
 * IS02A用 NVSキー定数定義
 *
 * 【重要】NVS (Preferences) のキー長制限
 * ESP32のNVS APIはキー名を最大15文字に制限している。
 * 16文字以上のキーを使用すると、保存が無視されるか切り捨てられ、
 * 「保存したはずが反映されない」という見つけにくいバグになる。
 */

#ifndef IS02A_KEYS_H
#define IS02A_KEYS_H

// ============================================================
// NVSキー定義マクロ（15文字制限をコンパイル時に強制）
// ============================================================
#define NVS_KEY(name, value) \
  static constexpr const char name[] = value; \
  static_assert(sizeof(value) - 1 <= 15, "NVS key '" value "' exceeds 15 chars")

// ============================================================
// IS02A固有のNVSキー（全て15文字以内）
// ============================================================
namespace Is02aKeys {

  // --- MQTT設定 ---
  NVS_KEY(kMqttBroker,  "is02a_broker");   // 12文字: MQTTブローカーURL
  NVS_KEY(kMqttPort,    "is02a_port");     // 10文字: MQTTポート
  NVS_KEY(kMqttUser,    "is02a_mquser");   // 12文字: MQTT認証ユーザー
  NVS_KEY(kMqttPass,    "is02a_mqpass");   // 12文字: MQTT認証パスワード
  NVS_KEY(kMqttTls,     "is02a_tls");      // 9文字:  TLS使用フラグ

  // --- BLE受信設定 ---
  NVS_KEY(kBleScanSec,  "is02a_blescan");  // 13文字: BLEスキャン時間(秒)
  NVS_KEY(kBleFilter,   "is02a_bleflt");   // 12文字: BLEフィルター（LacisIDプレフィックス）

  // --- センサー設定 ---
  NVS_KEY(kTempOffset,  "is02a_toffset");  // 13文字: 温度オフセット
  NVS_KEY(kHumOffset,   "is02a_hoffset");  // 13文字: 湿度オフセット
  NVS_KEY(kSelfIntv,    "is02a_selfint");  // 13文字: 自己計測間隔(秒)

  // --- バッチ送信設定 ---
  NVS_KEY(kBatchIntv,   "is02a_batch");    // 11文字: バッチ送信間隔(秒)
  NVS_KEY(kRelayUrl,    "is02a_relay");    // 11文字: is03aリレー先URL
  NVS_KEY(kRelayUrl2,   "is02a_relay2");   // 12文字: 冗長化リレー先URL

  // --- ステータスレポート設定 ---
  NVS_KEY(kReportIntv,  "is02a_report");   // 12文字: ステータスレポート間隔(秒)

  // --- リブートスケジューラ ---
  NVS_KEY(kRebootHour,  "is02a_rbhour");   // 12文字: リブート時刻(時) -1=無効
  NVS_KEY(kRebootMin,   "is02a_rbmin");    // 11文字: リブート時刻(分)

  // --- ノード蓄積設定（オンメモリ限定）---
  NVS_KEY(kMaxNodes,    "is02a_maxnode");  // 13文字: 最大ノード保持数

  // --- SSOT設定 ---
  NVS_KEY(kSchema,      "is02a_schema");   // 12文字: schemaVersion
  NVS_KEY(kHash,        "is02a_hash");     // 10文字: configHash
  NVS_KEY(kAppliedAt,   "is02a_applied");  // 13文字: 最終適用日時

}  // namespace Is02aKeys

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
  NVS_KEY(kGateUrl,       "gate_url");      // 8文字:  araneaDeviceGate URL

}  // namespace CommonKeys

#endif // IS02A_KEYS_H
