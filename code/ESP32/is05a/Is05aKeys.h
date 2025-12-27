/**
 * Is05aKeys.h
 *
 * IS05A用 NVSキー定数定義
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

#ifndef IS05A_KEYS_H
#define IS05A_KEYS_H

// ============================================================
// NVSキー定義マクロ（15文字制限をコンパイル時に強制）
// ============================================================
#define NVS_KEY(name, value) \
  static constexpr const char name[] = value; \
  static_assert(sizeof(value) - 1 <= 15, "NVS key '" value "' exceeds 15 chars")

// ============================================================
// IS05A固有のNVSキー（全て15文字以内）
// ============================================================
namespace Is05aKeys {

  // --- チャンネル設定（ch1-ch8）---
  // パターン: is05_chN_xxx (最大15文字)
  // pin: is05_ch1_pin (12文字)
  // name: is05_ch1_name (13文字)
  // meaning: is05_ch1_mean (13文字)
  // debounce: is05_ch1_deb (12文字)
  // inverted: is05_ch1_inv (12文字)

  // ch7/ch8のI/O切替
  NVS_KEY(kCh7Mode,     "is05_ch7_mode");  // 13文字: "input" or "output"
  NVS_KEY(kCh8Mode,     "is05_ch8_mode");  // 13文字: "input" or "output"
  NVS_KEY(kCh7Pulse,    "is05_ch7_pls");   // 12文字: パルス幅(ms)
  NVS_KEY(kCh8Pulse,    "is05_ch8_pls");   // 12文字: パルス幅(ms)

  // --- 出力モード設定 ---
  // 出力モード: 0=momentary, 1=alternate, 2=interval
  NVS_KEY(kCh7OutMode,  "is05_ch7_om");    // 11文字: ch7出力モード
  NVS_KEY(kCh8OutMode,  "is05_ch8_om");    // 11文字: ch8出力モード
  NVS_KEY(kCh7OutDur,   "is05_ch7_dur");   // 12文字: ch7出力時間(ms)
  NVS_KEY(kCh8OutDur,   "is05_ch8_dur");   // 12文字: ch8出力時間(ms)
  NVS_KEY(kCh7IntCnt,   "is05_ch7_cnt");   // 12文字: ch7インターバル回数
  NVS_KEY(kCh8IntCnt,   "is05_ch8_cnt");   // 12文字: ch8インターバル回数

  // --- Webhook設定 ---
  NVS_KEY(kWebhookOn,   "is05_wh_on");     // 10文字: Webhook有効化フラグ
  NVS_KEY(kWhQueueSz,   "is05_wh_qsz");    // 11文字: キューサイズ
  NVS_KEY(kWhInterval,  "is05_wh_int");    // 11文字: 送信間隔(秒)

  // レガシー互換用（将来削除予定）
  NVS_KEY(kDiscordUrl,  "is05_discord");   // 12文字: Discord Webhook URL
  NVS_KEY(kSlackUrl,    "is05_slack");     // 11文字: Slack Webhook URL
  NVS_KEY(kGenericUrl,  "is05_generic");   // 12文字: Generic Webhook URL
  NVS_KEY(kWhChMask,    "is05_wh_ch");     // 10文字: レガシーチャンネルマスク

  // --- Webhookエンドポイント (ep0-ep4, 最大5件) ---
  // URL: is05_epN_url (12文字)
  // channelMask: is05_epN_ch (11文字)
  // message: is05_epN_msg (12文字)
  // enabled: is05_epN_on (11文字)
  NVS_KEY(kEp0Url,      "is05_ep0_url");
  NVS_KEY(kEp0Ch,       "is05_ep0_ch");
  NVS_KEY(kEp0Msg,      "is05_ep0_msg");
  NVS_KEY(kEp0On,       "is05_ep0_on");
  NVS_KEY(kEp1Url,      "is05_ep1_url");
  NVS_KEY(kEp1Ch,       "is05_ep1_ch");
  NVS_KEY(kEp1Msg,      "is05_ep1_msg");
  NVS_KEY(kEp1On,       "is05_ep1_on");
  NVS_KEY(kEp2Url,      "is05_ep2_url");
  NVS_KEY(kEp2Ch,       "is05_ep2_ch");
  NVS_KEY(kEp2Msg,      "is05_ep2_msg");
  NVS_KEY(kEp2On,       "is05_ep2_on");
  NVS_KEY(kEp3Url,      "is05_ep3_url");
  NVS_KEY(kEp3Ch,       "is05_ep3_ch");
  NVS_KEY(kEp3Msg,      "is05_ep3_msg");
  NVS_KEY(kEp3On,       "is05_ep3_on");
  NVS_KEY(kEp4Url,      "is05_ep4_url");
  NVS_KEY(kEp4Ch,       "is05_ep4_ch");
  NVS_KEY(kEp4Msg,      "is05_ep4_msg");
  NVS_KEY(kEp4On,       "is05_ep4_on");

  // --- QuietMode設定（おやすみモード）---
  NVS_KEY(kQuietOn,     "is05_qt_on");     // 10文字: QuietMode有効フラグ
  NVS_KEY(kQuietStart,  "is05_qt_s");      // 9文字: 開始時刻（HHMM形式）
  NVS_KEY(kQuietEnd,    "is05_qt_e");      // 9文字: 終了時刻（HHMM形式）

  // --- 動作設定 ---
  NVS_KEY(kHeartbeat,   "is05_hb_sec");    // 11文字: 心拍間隔(秒)
  NVS_KEY(kBootGrace,   "is05_boot_g");    // 11文字: 起動猶予期間(ms)
  NVS_KEY(kReportInt,   "is05_rpt_s");     // 10文字: レポート間隔(秒)

  // --- リブートスケジューラ ---
  NVS_KEY(kRebootHour,  "is05_rb_h");      // 9文字: リブート時刻（時）
  NVS_KEY(kRebootMin,   "is05_rb_m");      // 9文字: リブート時刻（分）

  // --- SSOT設定（MQTT config適用状態）---
  NVS_KEY(kSchema,      "is05_schema");    // 11文字: schemaVersion
  NVS_KEY(kHash,        "is05_hash");      // 9文字:  configHash
  NVS_KEY(kAppliedAt,   "is05_applied");   // 12文字: 最終適用日時

  // --- MQTT設定 ---
  NVS_KEY(kMqttBroker,  "is05_mqtt_br");   // 12文字: MQTTブローカー
  NVS_KEY(kMqttPort,    "is05_mqtt_pt");   // 12文字: MQTTポート
  NVS_KEY(kMqttUser,    "is05_mqtt_u");    // 11文字: MQTTユーザー
  NVS_KEY(kMqttPass,    "is05_mqtt_p");    // 11文字: MQTTパスワード
  NVS_KEY(kMqttTls,     "is05_mqtt_tls");  // 13文字: MQTT TLS有効

}  // namespace Is05aKeys

// ============================================================
// チャンネルキー生成ヘルパー
// ============================================================
namespace Is05aChannelKeys {

  // チャンネル番号からNVSキーを生成
  // 例: getChannelKey(1, "pin") -> "is05_ch1_pin"
  inline String getChannelKey(int ch, const char* suffix) {
    return String("is05_ch") + String(ch) + "_" + String(suffix);
  }

  // 各チャンネル設定のサフィックス
  static constexpr const char* kPin = "pin";
  static constexpr const char* kName = "name";
  static constexpr const char* kMeaning = "mean";
  static constexpr const char* kDebounce = "deb";
  static constexpr const char* kInverted = "inv";

}  // namespace Is05aChannelKeys

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

#endif // IS05A_KEYS_H
