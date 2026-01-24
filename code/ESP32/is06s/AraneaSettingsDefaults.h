/**
 * AraneaSettingsDefaults.h
 *
 * IS06S PIN設定のデフォルト値定義
 *
 * 【設計原則】
 * - ハードコード排除: 全デフォルト値をここに集約
 * - PINglobal参照チェーン: PINSettings → PINglobal → araneaSettings
 * - NVS永続化: 初回起動時にこれらの値をNVSに書き込み
 *
 * 【参照チェーン】
 * 1. PINSettings.validity (個別PIN設定)
 * 2. PINglobal.Digital.validity (グローバル設定)
 * 3. AraneaSettingsDefaults::DIGITAL_VALIDITY (コード内デフォルト)
 */

#ifndef ARANEA_SETTINGS_DEFAULTS_H
#define ARANEA_SETTINGS_DEFAULTS_H

namespace AraneaSettingsDefaults {

// ============================================================
// Digital Output デフォルト値
// ============================================================
// モーメンタリ動作時間 (ms)
constexpr int DIGITAL_VALIDITY_MS = 3000;
constexpr const char* DIGITAL_VALIDITY = "3000";

// デバウンス時間 (ms)
constexpr int DIGITAL_DEBOUNCE_MS = 3000;
constexpr const char* DIGITAL_DEBOUNCE = "3000";

// アクションモード: "Mom" (Momentary) or "Alt" (Alternate)
constexpr const char* DIGITAL_ACTION_MODE = "Mom";

// ============================================================
// PWM Output デフォルト値
// ============================================================
// 変化率: 0-100%変化にかかる時間 (ms)
constexpr int PWM_RATE_OF_CHANGE_MS = 4000;
constexpr const char* PWM_RATE_OF_CHANGE = "4000";

// アクションモード: "Slow" or "Rapid"
constexpr const char* PWM_ACTION_MODE = "Slow";

// PWM周波数 (Hz)
constexpr int PWM_FREQUENCY = 5000;

// PWM解像度 (bits)
constexpr int PWM_RESOLUTION = 8;

// ============================================================
// 有効期限 デフォルト値
// ============================================================
// デフォルト有効期限 (日数)
constexpr int DEFAULT_EXPIRY_DAYS = 1;
constexpr const char* DEFAULT_EXPIRY = "1";

// 有効期限機能の有効/無効
constexpr const char* DEFAULT_EXPIRY_ENABLED = "false";

// ============================================================
// Digital Input デフォルト値
// ============================================================
// トリガータイプ: "Digital" or "PWM"
constexpr const char* INPUT_TRIGGER_TYPE = "Digital";

// 入力デバウンス時間 (ms)
constexpr int INPUT_DEBOUNCE_MS = 50;

// ============================================================
// PIN Enable デフォルト値
// ============================================================
// 全PINデフォルトで有効
constexpr const char* PIN_ENABLED_DEFAULT = "true";

// ============================================================
// System PIN タイミング定数
// ============================================================
// WiFi再接続トリガー (ms)
constexpr unsigned long RECONNECT_HOLD_MS = 5000;

// ファクトリーリセットトリガー (ms)
constexpr unsigned long RESET_HOLD_MS = 15000;

// APモードタイムアウト (ms)
constexpr unsigned long AP_MODE_TIMEOUT_MS = 300000;  // 5分

// ============================================================
// NVS Namespace / Key 定義
// ============================================================
namespace NVSKeys {
    // Namespace
    constexpr const char* NS_ARANEA = "aranea";
    constexpr const char* NS_PINGLOBAL = "pinglobal";
    constexpr const char* NS_PINSETTINGS = "pinsetting";

    // PINglobal keys (Digital)
    constexpr const char* PG_DIGITAL_VALIDITY = "dgValidity";
    constexpr const char* PG_DIGITAL_DEBOUNCE = "dgDebounce";
    constexpr const char* PG_DIGITAL_ACTION_MODE = "dgActMode";
    constexpr const char* PG_DIGITAL_EXPIRY = "dgExpiry";
    constexpr const char* PG_DIGITAL_EXPIRY_EN = "dgExpiryEn";

    // PINglobal keys (PWM)
    constexpr const char* PG_PWM_RATE_OF_CHANGE = "pwmRoC";
    constexpr const char* PG_PWM_ACTION_MODE = "pwmActMode";
    constexpr const char* PG_PWM_EXPIRY = "pwmExpiry";
    constexpr const char* PG_PWM_EXPIRY_EN = "pwmExpiryEn";

    // PIN enabled keys (ch1_enabled ~ ch6_enabled)
    constexpr const char* CH_ENABLED_PREFIX = "ch";
    constexpr const char* CH_ENABLED_SUFFIX = "_en";

    // PIN type keys (ch1_type ~ ch6_type)
    constexpr const char* CH_TYPE_SUFFIX = "_type";

    // PIN setting keys (ch{n}_{suffix})
    constexpr const char* CH_NAME_SUFFIX = "_name";
    constexpr const char* CH_ACTION_MODE_SUFFIX = "_mode";
    constexpr const char* CH_VALIDITY_SUFFIX = "_val";
    constexpr const char* CH_DEBOUNCE_SUFFIX = "_deb";
    constexpr const char* CH_RATE_OF_CHANGE_SUFFIX = "_roc";
    constexpr const char* CH_EXPIRY_DATE_SUFFIX = "_expDt";
    constexpr const char* CH_EXPIRY_ENABLED_SUFFIX = "_expEn";
    constexpr const char* CH_STATE_NAME_SUFFIX = "_stn";  // JSON配列として保存
    constexpr const char* CH_ALLOCATION_SUFFIX = "_alloc"; // CSV形式 "CH1,CH2"
}

// ============================================================
// PIN Type 定義
// ============================================================
namespace PinType {
    constexpr const char* DIGITAL_OUTPUT = "digitalOutput";
    constexpr const char* PWM_OUTPUT = "pwmOutput";
    constexpr const char* DIGITAL_INPUT = "digitalInput";
    constexpr const char* PIN_DISABLED = "disabled";  // ESP32マクロDISABLEDと衝突回避
}

// ============================================================
// Action Mode 定義
// ============================================================
namespace ActionMode {
    constexpr const char* MOMENTARY = "Mom";
    constexpr const char* ALTERNATE = "Alt";
    constexpr const char* SLOW = "Slow";
    constexpr const char* RAPID = "Rapid";
    constexpr const char* ROTATE = "rotate";
}

}  // namespace AraneaSettingsDefaults

#endif // ARANEA_SETTINGS_DEFAULTS_H
