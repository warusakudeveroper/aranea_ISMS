# IS06S 詳細設計書

**製品コード**: AR-IS06S
**作成日**: 2025/01/23
**最終更新**: 2025/01/23
**バージョン**: 1.4

---

## 1. 設計概要

### 1.1 設計原則

| 原則 | IS06S適用 |
|------|----------|
| **SSoT** | userObjectDetailをSSoTとし、NVS/MQTT双方向同期 |
| **SOLID** | Is06PinManagerで単一責任、共通モジュールで依存逆転 |
| **MECE** | PIN機能・状態・設定を漏れなく網羅 |
| **アンアンビギュアス** | 全パラメータに型・範囲・デフォルト値を明示 |

### 1.2 設計制約（ESP32）

```
重要: ESP32では以下を遵守
- セマフォとWDTの過剰制御を避ける
- 監査系関数を入れすぎない
- コードのシンプル化
- 可能な限りシングルタスクで実装
- パーティション: min_SPIFFS使用（huge_app禁止）
- I2C処理は必ず直列実行（並列禁止）
```

---

## 2. クラス設計

### 2.1 クラス図

```
┌─────────────────────────────────────────────────────────────────┐
│                        is06s.ino (メイン)                        │
│  - setup() / loop()                                              │
│  - グローバルインスタンス管理                                      │
└────────────────────────────┬────────────────────────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Is06PinManager │  │HttpManagerIs06s │  │StateReporterIs06s│
│  (PIN制御コア)   │  │  (Web UI/API)   │  │  (状態送信)      │
├─────────────────┤  ├─────────────────┤  ├─────────────────┤
│+ begin()        │  │+ begin()        │  │+ setAuth()      │
│+ update()       │  │+ setupRoutes()  │  │+ buildPayload() │
│+ setPinState()  │  │+ handlePulse()  │  │+ sendReport()   │
│+ getPinState()  │  │+ handleStatus() │  └─────────────────┘
│+ setPinSetting()│  └─────────────────┘
│+ getPinSetting()│
│+ onStateChange()│
└─────────────────┘
         │
         │ uses
         ▼
┌─────────────────────────────────────────────────────────────────┐
│                  global/src/ (共通モジュール)                     │
├─────────────────┬─────────────────┬─────────────────────────────┤
│ WiFiManager     │ SettingManager  │ MqttManager                 │
│ DisplayManager  │ NtpManager      │ HttpOtaManager              │
│ AraneaWebUI     │ Operator        │ LacisIDGenerator (必須)     │
│                 │                 │ AraneaRegister (必須)       │
└─────────────────┴─────────────────┴─────────────────────────────┘
```

### 2.2 AraneaSettingsDefaults（初期値定義）

#### 2.2.1 責務

- **唯一のハードコード値定義場所**
- araneaSettings（NVS）の初期書き込み値
- PINglobal → PINSettings参照チェーンの最終フォールバック

#### 2.2.2 定義

```cpp
// AraneaSettingsDefaults.h
// ハードコード値は本namespaceにのみ記述可（他箇所でのハードコード禁止）

#ifndef ARANEA_SETTINGS_DEFAULTS_H
#define ARANEA_SETTINGS_DEFAULTS_H

namespace AraneaSettingsDefaults {
    // Digital Output デフォルト値
    const char* DIGITAL_VALIDITY = "3000";       // ms
    const char* DIGITAL_DEBOUNCE = "3000";       // ms
    const char* DIGITAL_ACTION_MODE = "Mom";     // Mom/Alt

    // PWM Output デフォルト値
    const char* PWM_RATE_OF_CHANGE = "4000";     // ms
    const char* PWM_ACTION_MODE = "Slow";        // Slow/Rapid

    // 共通デフォルト値
    const char* DEFAULT_EXPIRY = "1";            // day
    const char* DEFAULT_EXPIRY_ENABLED = "false";
}

#endif
```

**重要**: Is06PinManager等の実装コードでは直接数値を記述せず、必ずPINglobal経由で値を取得すること。

---

### 2.3 Is06PinManager（コアクラス）

#### 2.3.1 責務

- PIN設定の管理（PINSettings）
- PIN状態の管理（PINState）
- **PINglobalからのデフォルト値解決**
- Digital/PWM出力制御
- Digital入力検知・連動
- 状態変化コールバック

#### 2.3.2 インターフェース定義

```cpp
// Is06PinManager.h

#ifndef IS06_PIN_MANAGER_H
#define IS06_PIN_MANAGER_H

#include <Arduino.h>
#include <functional>
#include <ArduinoJson.h>
#include "AraneaSettingsDefaults.h"

// PIN Type列挙
enum class PinType {
    DIGITAL_OUTPUT,    // digitalOutput
    PWM_OUTPUT,        // pwmOutput
    DIGITAL_INPUT,     // digitalInput
    SYSTEM,            // システムPIN（Reconnect/Reset）
    I2C                // I2C（OLED用）
};

// Action Mode列挙
enum class ActionMode {
    MOMENTARY,         // Mom - モーメンタリ
    ALTERNATE,         // Alt - オルタネート
    ROTATE,            // rotate - ローテート（PWM用）
    SLOW,              // Slow - なだらか変化（PWM用）
    RAPID              // Rapid - 急激変化（PWM用）
};

// PINglobal構造体（共通デフォルト値）
struct PinGlobalDefaults {
    // Digital defaults
    String digitalActionMode;      // "Mom"/"Alt"
    int digitalValidity;           // ms
    int digitalDebounce;           // ms

    // PWM defaults
    String pwmActionMode;          // "Slow"/"Rapid"
    int pwmRateOfChange;           // ms

    // Common defaults
    int defaultExpiry;             // day
    bool expiryEnabled;
};

// PIN設定構造体
struct PinSetting {
    bool enabled;                  // PIN機能の有効/無効（デフォルト: true）
    PinType type;
    String name;
    std::vector<String> stateName;
    ActionMode actionMode;         // 未設定時はPINglobal参照
    int defaultValidity;           // ms, 未設定(-1)時はPINglobal参照
    int defaultDebounce;           // ms, 未設定(-1)時はPINglobal参照
    int defaultExpiry;             // day, 未設定(-1)時はPINglobal参照
    int rateOfChange;              // ms (PWM用), 未設定(-1)時はPINglobal参照
    std::vector<String> allocation;  // Input連動先

    // 未設定判定用定数
    static const int UNSET = -1;
};

/*
 * enabled フィールドの動作仕様:
 * - enabled=true: 通常動作（PIN操作可能）
 * - enabled=false:
 *   - Web UI: 該当PINがグレーアウト表示、操作不可
 *   - HTTP API: set/pulseコマンドを受けてもエラー応答
 *   - MQTT: コマンド無視（ACKは返すが実行しない）
 *   - 物理入力: 連動先への信号送出を行わない
 *   - GPIO: 電気的には無効化しない（ESP32コード変更不要）
 * - 用途: リレー未接続PIN誤操作防止、メンテナンス時一時無効化
 */

// PIN状態構造体
struct PinState {
    String state;           // "on"/"off" or "0"-"100"
    int validity;           // オーバーライド有効時間(ms)
    int debounce;           // オーバーライドデバウンス(ms)
    String expiryDate;      // YYYYMMDDHHmm
    unsigned long lastChangeMs;  // 内部管理用
    bool isPulseActive;     // パルス動作中フラグ
};

// 状態変化コールバック型
using StateChangeCallback = std::function<void(const String& channel, const PinState& state)>;

class Is06PinManager {
public:
    Is06PinManager();

    // 初期化
    void begin(SettingManager* settings);

    // メインループ更新（非ブロッキング）
    void update();

    // PIN有効/無効チェック
    bool isPinEnabled(const String& channel) const;
    void setPinEnabled(const String& channel, bool enabled);

    // PIN状態操作（enabled=falseのPINへの操作は失敗を返す）
    bool setPinState(const String& channel, const String& state);
    bool setPinState(const String& channel, const String& state, int validity);
    PinState getPinState(const String& channel) const;

    // PIN設定操作
    bool setPinSetting(const String& channel, const PinSetting& setting);
    PinSetting getPinSetting(const String& channel) const;

    // PINglobal操作
    void setPinGlobalDefaults(const PinGlobalDefaults& defaults);
    PinGlobalDefaults getPinGlobalDefaults() const;

    // デフォルト値解決（PINSettings → PINglobal → araneaSettings）
    int getEffectiveValidity(const String& channel) const;
    int getEffectiveDebounce(const String& channel) const;
    int getEffectiveExpiry(const String& channel) const;
    int getEffectiveRateOfChange(const String& channel) const;
    ActionMode getEffectiveActionMode(const String& channel) const;

    // JSON変換
    void toJson(JsonObject& doc) const;
    bool fromJson(const JsonObject& doc);

    // コールバック登録
    void onStateChange(StateChangeCallback callback);

    // System PIN操作
    void handleSystemButton(int gpio, unsigned long pressedMs);

private:
    // PIN定義
    static const int PIN_CH1 = 18;
    static const int PIN_CH2 = 5;
    static const int PIN_CH3 = 15;
    static const int PIN_CH4 = 27;
    static const int PIN_CH5 = 14;
    static const int PIN_CH6 = 12;
    static const int PIN_RECONNECT = 25;
    static const int PIN_RESET = 26;

    // 内部状態
    std::map<String, PinSetting> settings_;
    std::map<String, PinState> states_;
    PinGlobalDefaults globalDefaults_;  // PINglobalからロード
    SettingManager* nvs_;
    StateChangeCallback callback_;

    // PWM制御用
    int pwmChannels_[4];  // LEDC channel
    int targetPwm_[4];
    int currentPwm_[4];
    unsigned long pwmStartMs_[4];

    // 内部メソッド
    void initGpio();
    void initPinGlobalFromNvs();  // NVSからPINglobalロード
    void updateDigitalOutput(const String& channel);
    void updatePwmOutput(const String& channel);
    void updateDigitalInput();
    void processPulse();
    void triggerAllocation(const String& inputChannel);
    int channelToGpio(const String& channel) const;
    int channelToIndex(const String& channel) const;

    // デフォルト値解決ヘルパー（ハードコード禁止）
    // PINSettings値 → PINglobal値 → araneaSettings初期値 の順で解決
    int resolveDefaultInt(int pinValue, int globalValue, const char* araneaDefault) const;
    String resolveDefaultString(const String& pinValue, const String& globalValue,
                                const char* araneaDefault) const;
};

#endif
```

#### 2.3.3 状態遷移図（Digital Output - Momentary）

```
                    ┌─────────────────┐
                    │     OFF         │
                    │  (state="off")  │
                    └────────┬────────┘
                             │
                             │ trigger (HTTP/MQTT/Input)
                             │ isPulseActive = true
                             │ lastChangeMs = millis()
                             ▼
                    ┌─────────────────┐
                    │      ON         │
                    │  (state="on")   │
                    │  パルス動作中    │
                    └────────┬────────┘
                             │
                             │ millis() - lastChangeMs >= validity
                             │ isPulseActive = false
                             ▼
                    ┌─────────────────┐
                    │     OFF         │
                    │  (state="off")  │
                    └─────────────────┘
```

#### 2.3.4 状態遷移図（Digital Output - Alternate）

```
                    ┌─────────────────┐
                    │     OFF         │◄──────────────┐
                    │  (state="off")  │               │
                    └────────┬────────┘               │
                             │                        │
                             │ trigger               │ trigger
                             ▼                        │
                    ┌─────────────────┐               │
                    │      ON         │───────────────┘
                    │  (state="on")   │
                    └─────────────────┘
```

#### 2.3.5 状態遷移図（PWM Output - Slow）

```
     ┌──────────────────────────────────────────────────────────┐
     │                                                          │
     │   0% ─── 10% ─── 20% ─── ... ─── 90% ─── 100%           │
     │   ◄────────────────────────────────────────────►         │
     │              rateOfChange ms で変化                      │
     │                                                          │
     └──────────────────────────────────────────────────────────┘
```

#### 2.3.6 System PIN動作（Reconnect/Reset）

**GPIO25 (Reconnect)**: 5,000ms長押し
```
入力開始 → OLED表示"Reconnect" + カウントダウン表示
         → 5,000ms経過 → WiFi再接続実行 + NTP同期
         → OLED表示"Reconnecting..."
         → 完了後 → OLED通常表示に復帰
```

**GPIO26 (Reset)**: 15,000ms長押し
```
入力開始 → OLED表示"Reset" + カウントダウン表示
         → 15,000ms経過 → NVS/SPIFFSクリア実行
         → OLED表示"Resetting..."
         → 完了後 → 自動リブート → APモードで起動
```

**handleSystemButton実装ポイント**:
- 入力開始時点からOLED表示を開始（カウントダウン）
- 途中で入力が離された場合はキャンセル
- DisplayManagerへのコールバックでOLED更新

### 2.4 HttpManagerIs06s（Web UI/API）

#### 2.4.1 責務

- AraneaWebUI継承
- IS06S固有エンドポイント実装
- PIN操作API
- PIN設定API

#### 2.4.2 インターフェース定義

```cpp
// HttpManagerIs06s.h

#ifndef HTTP_MANAGER_IS06S_H
#define HTTP_MANAGER_IS06S_H

#include "AraneaWebUI.h"
#include "Is06PinManager.h"

class HttpManagerIs06s : public AraneaWebUI {
public:
    void begin(SettingManager* settings, int port) override;
    void setupRoutes() override;

    // 依存オブジェクト設定
    void setPinManager(Is06PinManager* pinManager);
    void setStateReporter(StateReporterIs06s* reporter);

private:
    Is06PinManager* pinManager_;
    StateReporterIs06s* reporter_;

    // IS06S固有ハンドラ
    void handleGetStatus();           // GET /api/status
    void handleGetPinState();         // GET /api/pin/{ch}/state
    void handlePostPinState();        // POST /api/pin/{ch}/state
    void handleGetPinSetting();       // GET /api/pin/{ch}/setting
    void handlePostPinSetting();      // POST /api/pin/{ch}/setting
    void handleGetGlobalSettings();   // GET /api/settings
    void handlePostGlobalSettings();  // POST /api/settings

    // Web UIページ
    void handlePinControlPage();      // /pincontrol
    void handlePinSettingPage();      // /pinsetting
};

#endif
```

#### 2.4.3 APIエンドポイント詳細

| エンドポイント | メソッド | リクエスト | レスポンス |
|---------------|---------|-----------|-----------|
| `/api/status` | GET | - | 全PIN状態JSON |
| `/api/pin/{ch}/state` | GET | - | 特定PIN状態 |
| `/api/pin/{ch}/state` | POST | `{"state":"on","validity":5000}` | 結果 |
| `/api/pin/{ch}/setting` | GET | - | 特定PIN設定 |
| `/api/pin/{ch}/setting` | POST | PinSetting JSON | 結果 |
| `/api/settings` | GET | - | globalSettings |
| `/api/settings` | POST | globalSettings JSON | 結果 |

### 2.5 StateReporterIs06s（状態送信）

#### 2.5.1 責務

- 認証情報管理
- 状態レポートペイロード構築
- HTTP POST送信

#### 2.5.2 インターフェース定義

```cpp
// StateReporterIs06s.h

#ifndef STATE_REPORTER_IS06S_H
#define STATE_REPORTER_IS06S_H

#include <Arduino.h>
#include <ArduinoJson.h>

class StateReporterIs06s {
public:
    // 認証情報設定
    void setAuth(const String& tid, const String& lacisId, const String& cic);
    void setReportUrl(const String& url);

    // ペイロード構築
    String buildPayload(const JsonObject& pinStates,
                        const String& ip, int rssi,
                        const String& timestamp);

    // 送信
    bool sendReport(const String& payload);

private:
    String tid_;
    String lacisId_;
    String cic_;
    String reportUrl_;
};

#endif
```

#### 2.5.3 送信ペイロード形式

```json
{
    "auth": {
        "tid": "T2025120621041161827",
        "lacisId": "30061234567890AB0001",
        "cic": "123456"
    },
    "report": {
        "lacisId": "30061234567890AB0001",
        "type": "ar-is06s",
        "observedAt": "2025-01-23T10:00:00Z",
        "state": {
            "PINState": {
                "CH1": {"state": "on", "validity": 0},
                "CH2": {"state": "50"},
                "CH3": {"state": "off"},
                "CH4": {"state": "off"},
                "CH5": {"state": "off"},
                "CH6": {"state": "on"}
            },
            "globalState": {
                "ipAddress": "192.168.77.100",
                "uptime": 3600,
                "rssi": -65,
                "heapFree": 120000
            }
        }
    }
}
```

---

## 3. シーケンス図

### 3.1 起動シーケンス

```
┌────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐
│ setup()│  │WiFiManager │  │SettingMgr  │  │Is06PinMgr  │  │HttpMgrIs06s│
└───┬────┘  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘
    │             │               │               │               │
    │ begin()     │               │               │               │
    │────────────────────────────►│               │               │
    │             │   load NVS    │               │               │
    │             │◄──────────────│               │               │
    │             │               │               │               │
    │ connect()   │               │               │               │
    │────────────►│               │               │               │
    │             │ WiFi connect  │               │               │
    │             │──────────────────────────────────────────────►│
    │             │               │               │               │
    │ begin(settings)             │               │               │
    │────────────────────────────────────────────►│               │
    │             │               │               │ initGpio()    │
    │             │               │               │──────┐        │
    │             │               │               │◄─────┘        │
    │             │               │               │               │
    │ begin(settings, port)       │               │               │
    │────────────────────────────────────────────────────────────►│
    │             │               │               │               │
    │             │               │               │ setupRoutes() │
    │             │               │               │◄──────────────│
    │             │               │               │               │
```

### 3.2 PIN操作シーケンス（HTTP経由）

```
┌────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐
│ Client │  │HttpMgrIs06s│  │Is06PinMgr  │  │StateReporter│ │AraneaGate  │
└───┬────┘  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘
    │             │               │               │               │
    │ POST /api/pin/CH1/state     │               │               │
    │ {"state":"on","validity":5000}              │               │
    │────────────►│               │               │               │
    │             │               │               │               │
    │             │ setPinState("CH1","on",5000)  │               │
    │             │──────────────►│               │               │
    │             │               │               │               │
    │             │               │ digitalWrite  │               │
    │             │               │──────┐        │               │
    │             │               │◄─────┘        │               │
    │             │               │               │               │
    │             │               │ callback(CH1) │               │
    │             │               │──────────────►│               │
    │             │               │               │               │
    │             │               │               │ buildPayload()│
    │             │               │               │──────┐        │
    │             │               │               │◄─────┘        │
    │             │               │               │               │
    │             │               │               │ POST /state   │
    │             │               │               │──────────────►│
    │             │               │               │               │
    │             │◄──────────────│               │               │
    │◄────────────│               │               │               │
    │  200 OK     │               │               │               │
    │             │               │               │               │
```

### 3.3 物理入力連動シーケンス

```
┌────────┐  ┌────────────┐  ┌────────────┐
│GPIO14  │  │Is06PinMgr  │  │GPIO18      │
│(CH5)   │  │            │  │(CH1)       │
└───┬────┘  └─────┬──────┘  └─────┬──────┘
    │             │               │
    │ HIGH        │               │
    │────────────►│               │
    │             │               │
    │             │ debounce check│
    │             │──────┐        │
    │             │◄─────┘        │
    │             │               │
    │             │ allocation    │
    │             │ ["CH1"]       │
    │             │──────┐        │
    │             │◄─────┘        │
    │             │               │
    │             │ digitalWrite  │
    │             │──────────────►│
    │             │               │ HIGH
    │             │               │
    │             │ validity ms後 │
    │             │──────────────►│
    │             │               │ LOW
    │             │               │
```

---

## 4. データフロー

### 4.1 設定データフロー

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   mobes2.0  │     │    MQTT     │     │   IS06S     │
│userObject   │◄───►│   Broker    │◄───►│    NVS      │
│  Detail     │     │             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
      │                                       │
      │           SSoT (Single Source of Truth)
      └───────────────────────────────────────┘
```

### 4.2 状態データフロー

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   HTTP      │────►│             │     │             │
│   Client    │     │             │     │             │
└─────────────┘     │             │     │             │
                    │ Is06Pin     │────►│  StateReport│
┌─────────────┐     │  Manager    │     │    Is06s    │
│   MQTT      │◄───►│             │     │             │
│   Command   │     │             │     └──────┬──────┘
└─────────────┘     │             │            │
                    │             │            ▼
┌─────────────┐     │             │     ┌─────────────┐
│  Physical   │────►│             │     │ AraneaDevice│
│   Input     │     └─────────────┘     │    Gate     │
└─────────────┘                         └─────────────┘
```

### 4.3 デフォルト値参照フロー（ハードコード排除）

```
┌─────────────────────────────────────────────────────────────────────┐
│                        デフォルト値解決フロー                         │
│                                                                     │
│   ┌──────────────────┐                                              │
│   │ Is06PinManager   │                                              │
│   │ getDefaultValue()│                                              │
│   └────────┬─────────┘                                              │
│            │                                                        │
│            ▼                                                        │
│   ┌──────────────────┐  設定あり   ┌──────────────────┐            │
│   │PINSettings.CHx   │───────────►│ PINSettings値使用 │            │
│   │該当フィールド参照  │            └──────────────────┘            │
│   └────────┬─────────┘                                              │
│            │ 未設定/空欄                                             │
│            ▼                                                        │
│   ┌──────────────────┐  設定あり   ┌──────────────────┐            │
│   │PINglobal参照      │───────────►│ PINglobal値使用   │            │
│   │ Digital/PWM/common│           └──────────────────┘            │
│   └────────┬─────────┘                                              │
│            │ 未設定/空欄                                             │
│            ▼                                                        │
│   ┌──────────────────┐            ┌──────────────────┐            │
│   │araneaSettings    │───────────►│ 初期値使用        │            │
│   │(NVS初期値)        │            │（唯一のハードコード）│            │
│   └──────────────────┘            └──────────────────┘            │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

優先順位: PINSettings（個別）> PINglobal（共通）> araneaSettings（初期値）
```

#### 4.3.1 対象パラメータ一覧

| パラメータ | PINSettings参照先 | PINglobal参照先 | araneaSettings初期値 |
|-----------|------------------|-----------------|---------------------|
| actionMode (Digital) | CHx.actionMode | PINglobal.Digital.defaultActionMode | "Mom" |
| defaultValidity | CHx.defaultValidity | PINglobal.Digital.defaultValidity | "3000" |
| defaultDebounce | CHx.defaultDebounce | PINglobal.Digital.defaultDebounce | "3000" |
| actionMode (PWM) | CHx.actionMode | PINglobal.PWM.defaultActionMode | "Slow" |
| RateOfChange | CHx.RateOfChange | PINglobal.PWM.defaultRateOfChange | "4000" |
| defaultexpiry | CHx.defaultexpiry | PINglobal.common.defaultexpiry | "1" |

### 4.4 Firestore スキーマ構造

#### 4.4.1 userObject (デバイス基本情報)

**パス**: `userObjects/{lacisId}`

```typescript
interface UserObject {
  lacis: {
    type_domain: "araneaDevice";         // 固定
    type: "ar-is06s";               // デバイスタイプ
    tid: string;                         // テナントID
    permission: 10;                      // 固定
    cic_code: string;                    // 6桁CIC
    cic_active: boolean;                 // CIC有効フラグ
  };
  activity: {
    created_at: Timestamp;               // 作成日時
    lastUpdated_at: Timestamp;           // 最終更新日時
    Ordination_at?: Timestamp;           // 施設割当日時
  };
  device: {
    macAddress: string;                  // 12桁HEX大文字
    productType: "006";                  // IS06S固定
    productCode: "0200";                 // 製品コード
  };
  fid?: string[];                        // 施設ID配列
}
```

#### 4.4.2 userObject_detail (デバイス詳細情報)

**パス**: `userObjects/{lacisId}/userObject_detail`

```typescript
interface UserObjectDetail {
  firmware: {
    version: string;                     // "1.0.0"
    buildDate: string;                   // ISO8601
    modules: string[];                   // ["WiFi", "MQTT", "OTA", "WebUI"]
  };
  config: {
    PINSettings: {
      CH1: DPPinSetting;
      CH2: DPPinSetting;
      CH3: DPPinSetting;
      CH4: DPPinSetting;
      CH5?: IOPinSetting;
      CH6?: IOPinSetting;
    };
    PINglobal: {
      Digital: { defaultValidity, defaultDebounce, defaultActionMode };
      PWM: { defaultRateOfChange, defaultActionMode };
      common: { defaultexpiry, defaultExpiryEnabled };
    };
    globalSettings: {
      wifiSetting: Record<string, string>;
      APmode: { APSSID, APPASS, APsubnet, APaddr, exclusiveConnection };
      network: { DHCP, Static? };
      WEBUI: { localUID, lacisOath };
    };
    "hardware components": {
      SBC: string;
      I2COLED: string;
      baseboard: string;
    };
  };
  status: {
    online: boolean;
    lastSeen: Timestamp;
    rssi?: number;
    heap?: number;
  };
  network: {
    ip?: string;
    ssid?: string;
    gateway?: string;
    subnet?: string;
  };
}
```

#### 4.4.3 araneaDeviceStates (状態キャッシュ)

**パス**: `araneaDeviceStates/{lacisId}`

```typescript
interface AraneaDeviceStates {
  lacisId: string;
  tid: string;
  fid: string[];
  type: "ar-is06s";
  state: {
    PINState: {
      CH1: { state, Validity?, Debounce?, expiryDate?, updatedAt };
      CH2: { state, Validity?, Debounce?, expiryDate?, updatedAt };
      // ...CH6
    };
    globalState: {
      ipAddress: string;
      uptime: number;
      rssi: number;
      heapFree: number;
    };
  };
  semanticTags: ["接点出力", "PWM", "リレー", "物理入力", "調光"];
  advert: {
    observedAt: Timestamp;
  };
  alive: boolean;
  lastUpdatedAt: Timestamp;
}
```

#### 4.4.4 typeSettings (Type定義)

**パス**: `typeSettings/araneaDevice/ar-is06s`

```typescript
interface TypeSettings {
  displayName: "Relay & Switch Controller";
  description: "4ch D/P出力 + 2ch I/O コントローラー（PWM対応）";
  version: number;
  productType: "006";
  productCodes: ["0200"];
  stateSchema: JSONSchema;              // PINState構造
  configSchema: JSONSchema;             // PINSettings, globalSettings構造
  commandSchema: JSONSchema;            // set, pulse, allOff, config
  capabilities: ["output", "input", "pwm", "pulse", "trigger"];
  semanticTags: ["接点出力", "PWM", "リレー", "物理入力", "調光"];
  features: string[];
  defaultConfig: Record<string, any>;
}
```

### 4.5 NVSとFirestoreの同期フロー

```
┌─────────────────────────────────────────────────────────────────────┐
│                       設定同期フロー                                  │
│                                                                     │
│   ┌──────────┐  MQTT Subscribe  ┌──────────┐                        │
│   │  IS06S   │◄─────────────────│  Broker  │                        │
│   │   NVS    │                  │          │                        │
│   └────┬─────┘                  └────┬─────┘                        │
│        │                              │                              │
│        │ 1. 設定変更受信              │                              │
│        ▼                              │                              │
│   ┌──────────┐                        │                              │
│   │ NVS更新  │                        │                              │
│   │ (ローカル)│                        │                              │
│   └────┬─────┘                        │                              │
│        │                              │                              │
│        │ 2. ACK返信                   │                              │
│        │ ──────────────────────────►  │                              │
│        │                              │                              │
│                                       │  3. Firestore更新            │
│                                       │ ─────────────────────────►   │
│                                       │                              │
│                                  ┌────┴─────┐                        │
│                                  │ Firestore│                        │
│                                  │userObject│                        │
│                                  │_detail   │                        │
│                                  └──────────┘                        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

SSoT: userObject_detail (Firestore)
デバイス側: NVSはキャッシュとして機能、オフライン時のローカル動作を保証
```

---

## 5. エラーハンドリング

### 5.1 エラー分類

| カテゴリ | エラー | 対処 |
|---------|--------|------|
| **通信** | WiFi接続失敗 | APモード移行、再接続試行 |
| **通信** | MQTT接続失敗 | 再接続試行、ローカル動作継続 |
| **通信** | HTTP送信失敗 | リトライ(3回)、ログ記録 |
| **PIN** | 無効なチャンネル指定 | 400 Bad Request返却 |
| **PIN** | allocation違反 | 設定拒否、エラーログ |
| **System** | NVS書き込み失敗 | リトライ、警告ログ |
| **System** | I2C通信失敗 | OLED表示スキップ |

### 5.2 リカバリフロー

```
┌─────────────────────────────────────────────────────────────────┐
│                       エラー検出                                 │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
                                  ▼
                    ┌─────────────────────────┐
                    │  リカバリ可能か判定      │
                    └────────────┬────────────┘
                                 │
                 ┌───────────────┴───────────────┐
                 │                               │
                 ▼                               ▼
        ┌────────────────┐              ┌────────────────┐
        │  リカバリ可能   │              │  リカバリ不可   │
        │  リトライ実行   │              │  エラー通知     │
        └───────┬────────┘              │  ログ記録      │
                │                        │  動作継続      │
                ▼                        └────────────────┘
        ┌────────────────┐
        │  成功/失敗判定  │
        └────────────────┘
```

---

## 6. NVS設計

### 6.1 NVSキー一覧

| Namespace | キー | 型 | デフォルト | 説明 |
|-----------|-----|-----|-----------|------|
| wifi | ssid1-6 | string | "" | WiFi SSID |
| wifi | pass1-6 | string | "" | WiFi パスワード |
| network | dhcp | bool | true | DHCP有効 |
| network | static_ip | string | "" | 静的IP |
| network | gateway | string | "" | ゲートウェイ |
| network | subnet | string | "" | サブネット |
| ap | ap_ssid | string | "IS06S_SETUP" | AP SSID |
| ap | ap_pass | string | "" | AP パスワード |
| auth | tid | string | "" | テナントID |
| auth | lacisid | string | "" | 自デバイスlacisID |
| auth | cic | string | "" | 自デバイスCIC |
| auth | tenant_lacisid | string | "" | テナントプライマリlacisID |
| auth | tenant_email | string | "" | テナントプライマリEmail |
| auth | tenant_cic | string | "" | テナントプライマリCIC |
| pin | ch{n}_enabled | bool | true | PIN有効/無効 |
| pin | ch{n}_type | string | "digitalOutput" | PIN type |
| pin | ch{n}_name | string | "CH{n}" | PIN表示名 |
| pin | ch{n}_mode | string | "" | actionMode (空=PINglobal参照) |
| pin | ch{n}_validity | int | -1 | defaultValidity (-1=PINglobal参照) |
| pin | ch{n}_debounce | int | -1 | defaultDebounce (-1=PINglobal参照) |
| pin | ch{n}_expiry | int | -1 | defaultExpiry (-1=PINglobal参照) |
| pin | ch{n}_roc | int | -1 | RateOfChange (-1=PINglobal参照) |
| **pinglobal** | dig_mode | string | "Mom" | Digital defaultActionMode |
| pinglobal | dig_validity | int | 3000 | Digital defaultValidity |
| pinglobal | dig_debounce | int | 3000 | Digital defaultDebounce |
| pinglobal | pwm_mode | string | "Slow" | PWM defaultActionMode |
| pinglobal | pwm_roc | int | 4000 | PWM RateOfChange |
| pinglobal | expiry | int | 1 | 共通 defaultexpiry |
| pinglobal | expiry_enabled | bool | false | 有効期限機能ON/OFF |

### 6.2 NVS操作ポリシー

- 書き込み: 設定変更時のみ（頻繁な書き込み禁止）
- 読み込み: 起動時に全設定ロード
- 同期: MQTT設定変更受信時にNVS更新

### 6.3 araneaSettings初期値書き込みフロー

```
┌──────────────┐
│   初回起動    │
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────────────┐
│ NVS "pinglobal" namespace存在チェック         │
└──────┬───────────────────────────────────────┘
       │
       │ 存在しない場合
       ▼
┌──────────────────────────────────────────────┐
│ AraneaSettingsDefaultsの値をNVSに書き込み     │
│ - dig_mode = "Mom"                           │
│ - dig_validity = 3000                        │
│ - dig_debounce = 3000                        │
│ - pwm_mode = "Slow"                          │
│ - pwm_roc = 4000                             │
│ - expiry = 1                                 │
│ - expiry_enabled = false                     │
└──────┬───────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────┐
│ userObjectDetail.PINglobalと同期（MQTT）      │
└──────────────────────────────────────────────┘
```

**設計原則**:
- ハードコードは`AraneaSettingsDefaults`ネームスペースにのみ存在
- Is06PinManagerは常にPINglobal（NVS）経由で値を取得
- ファームウェア更新時もNVS値は維持（明示的リセット以外）

---

## 7. MECE確認

### 7.1 クラス責務分離確認

| 責務 | 担当クラス | 重複なし |
|------|-----------|---------|
| PIN制御 | Is06PinManager | ✅ |
| Web UI/API | HttpManagerIs06s | ✅ |
| 状態送信 | StateReporterIs06s | ✅ |
| WiFi管理 | WiFiManager (global) | ✅ |
| 設定永続化 | SettingManager (global) | ✅ |
| MQTT | MqttManager (global) | ✅ |
| 表示 | DisplayManager (global) | ✅ |
| 認証 | AraneaRegister (global) | ✅ |

### 7.2 データ構造網羅確認

| データ | 格納先 | 網羅確認 |
|--------|--------|---------|
| PIN設定 | NVS + userObjectDetail | ✅ |
| PIN状態 | メモリ + MQTT | ✅ |
| WiFi設定 | NVS + userObjectDetail | ✅ |
| 認証情報 | NVS | ✅ |
| ランタイム状態 | メモリ | ✅ |

### 7.3 アンアンビギュアス確認

- 全メソッドに引数型・戻り値型を明示
- 全NVSキーにデフォルト値を定義
- 全状態遷移に条件を明記

---

## 8. 変更履歴

| 日付 | バージョン | 変更内容 | 担当 |
|------|-----------|----------|------|
| 2025/01/23 | 1.0 | 初版作成 | Claude |
| 2025/01/23 | 1.1 | PINglobal・デフォルト値参照フロー追加、AraneaSettingsDefaults追加、NVS設計拡張 | Claude |
| 2025/01/23 | 1.2 | Firestore スキーマ構造追加（userObject, userObject_detail, araneaDeviceStates, typeSettings）、NVS-Firestore同期フロー追加 | Claude |
| 2025/01/23 | 1.3 | PinSettingにenabledフィールド追加、isPinEnabled/setPinEnabledメソッド追加、NVSキー追加 | Claude |
| 2025/01/23 | 1.4 | Type名をar-is06sに統一、ProductCodeを0200に変更 | Claude |
