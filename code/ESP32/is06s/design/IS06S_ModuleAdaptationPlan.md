# IS06S モジュール適応計画

**製品コード**: AR-IS06S
**作成日**: 2025/01/23
**目的**: global/モジュールの再利用とIS06S固有モジュールの設計

---

## 1. アーキテクチャ概要

### 1.1 ディレクトリ構成

```
code/ESP32/
├── global/src/                  ← 共通モジュール（変更なし）
│   ├── WiFiManager.h/cpp
│   ├── SettingManager.h/cpp
│   ├── DisplayManager.h/cpp
│   ├── NtpManager.h/cpp
│   ├── MqttManager.h/cpp
│   ├── AraneaWebUI.h/cpp
│   ├── HttpOtaManager.h/cpp
│   ├── Operator.h/cpp
│   ├── LacisIDGenerator.h/cpp   ← 必須
│   └── AraneaRegister.h/cpp     ← 必須
│
└── is06s/                       ← IS06S固有
    ├── is06s.ino                ← メインスケッチ
    ├── AraneaGlobalImporter.h   ← 共通モジュールインポート
    ├── Is06PinManager.h/cpp     ← PIN管理（コア）
    ├── HttpManagerIs06s.h/cpp   ← Web UI（固有API実装）
    ├── StateReporterIs06s.h/cpp ← 状態送信ペイロード構築
    └── design/                  ← 設計ドキュメント
```

---

## 2. 共通モジュール使用計画

### 2.1 WiFiManager

**使用方法**: そのまま使用

```cpp
#include "WiFiManager.h"

WiFiManager wifi;
wifi.connectWithSettings(&settings);
```

**変更不要**: is04aと同じ使用方法

**IS06S固有考慮事項**:
- 最大6 SSID対応（NVSキー: ssid1-6, pass1-6）
- APモード移行（全SSID接続失敗時）

---

### 2.2 SettingManager

**使用方法**: そのまま使用

```cpp
#include "SettingManager.h"

SettingManager settings;
settings.begin();
String ssid = settings.getString("wifi_ssid", "");
int validity = settings.getInt("ch1_validity", 3000);
```

**IS06S固有のNVSキー**:

| カテゴリ | キー | 型 | デフォルト | 説明 |
|---------|-----|-----|-----------|------|
| WiFi | ssid1-6 | string | "" | WiFi SSID |
| WiFi | pass1-6 | string | "" | WiFi パスワード |
| Network | dhcp | bool | true | DHCP有効 |
| Network | static_ip, gateway, subnet | string | "" | 静的IP設定 |
| AP | ap_ssid | string | "IS06S_SETUP" | AP SSID |
| AP | ap_pass | string | "" | AP パスワード |
| Auth | tid, lacisid, cic | string | "" | 認証情報 |
| Auth | tenant_lacisid, tenant_email, tenant_cic | string | "" | テナントプライマリ情報 |
| PIN | ch{n}_enabled | bool | true | PIN有効/無効 |
| PIN | ch{n}_type | string | "digitalOutput" | PIN type |
| PIN | ch{n}_name | string | "CH{n}" | PIN表示名 |
| PIN | ch{n}_mode | string | "" | actionMode (空=PINglobal参照) |
| PIN | ch{n}_validity | int | -1 | defaultValidity (-1=PINglobal参照) |
| PIN | ch{n}_debounce | int | -1 | defaultDebounce (-1=PINglobal参照) |
| PIN | ch{n}_expiry | int | -1 | defaultExpiry (-1=PINglobal参照) |
| PIN | ch{n}_roc | int | -1 | RateOfChange (-1=PINglobal参照) |
| **PINglobal** | dig_mode | string | "Mom" | Digital defaultActionMode |
| PINglobal | dig_validity | int | 3000 | Digital defaultValidity |
| PINglobal | dig_debounce | int | 3000 | Digital defaultDebounce |
| PINglobal | pwm_mode | string | "Slow" | PWM defaultActionMode |
| PINglobal | pwm_roc | int | 4000 | PWM RateOfChange |
| PINglobal | expiry | int | 1 | 共通 defaultexpiry |
| PINglobal | expiry_enabled | bool | false | 有効期限機能ON/OFF |

---

### 2.3 DisplayManager

**使用方法**: そのまま使用

```cpp
#include "DisplayManager.h"

DisplayManager display;
display.begin();
display.showBoot("Booting...");
display.showIs02Main(line1, cicStr, sensorLine, false);
```

**IS06S固有表示内容**:

| 行 | 内容 |
|----|------|
| 1 | デバイスID（CIC） |
| 2 | IPアドレス / APモード表示 |
| 3 | PIN状態サマリ（CH1-6） |
| 4 | RSSI / Uptime |

**表示例**:
```
CIC: 123456
IP: 192.168.77.100
CH: ●○○○○●
RSSI: -65 Up: 01:23
```

---

### 2.4 NtpManager

**使用方法**: そのまま使用

```cpp
#include "NtpManager.h"

NtpManager ntp;
ntp.sync();
String timestamp = ntp.getIso8601();
```

**IS06S固有考慮事項**:
- 状態レポートのタイムスタンプに使用
- expiryDate判定に使用

---

### 2.5 MqttManager

**使用方法**: is10と同様のパターン

```cpp
#include "MqttManager.h"

MqttManager mqtt;
mqtt.begin(brokerUrl, deviceId, apiKey);
mqtt.subscribe("device/" + lacisId + "/command", onCommand);
mqtt.publish("device/" + lacisId + "/state", stateJson);
```

**IS06S MQTTトピック**:

| トピック | 方向 | ペイロード |
|---------|------|----------|
| device/{lacisId}/command | Subscribe | コマンドJSON |
| device/{lacisId}/state | Publish | 状態JSON |
| device/{lacisId}/ack | Publish | ACK JSON |
| device/{lacisId}/settings | Subscribe | 設定同期JSON |

**コマンドペイロード例**:
```json
{
    "command": "setPinState",
    "channel": "CH1",
    "state": "on",
    "validity": 5000
}
```

---

### 2.6 AraneaWebUI

**使用方法**: 継承して固有API実装

```cpp
#include "AraneaWebUI.h"

class HttpManagerIs06s : public AraneaWebUI {
public:
    void begin(SettingManager* settings, int port) override;
    void setupRoutes() override;

private:
    void handleStatus();
    void handlePinState();
    void handlePinSetting();
    void handleGlobalSettings();
};
```

**継承元機能（そのまま使用）**:
- 基本Web UIテンプレート
- 認証処理
- 静的ファイル配信

**IS06S固有エンドポイント**:

| エンドポイント | メソッド | 機能 |
|---------------|---------|------|
| /api/status | GET | 全PIN状態取得 |
| /api/pin/{ch}/state | GET/POST | PIN状態取得/変更 |
| /api/pin/{ch}/setting | GET/POST | PIN設定取得/変更 |
| /api/settings | GET/POST | globalSettings |
| /pincontrol | GET | PIN操作ページ |
| /pinsetting | GET | PIN設定ページ |

---

### 2.7 HttpOtaManager

**使用方法**: そのまま使用

```cpp
#include "HttpOtaManager.h"

HttpOtaManager ota;
ota.begin(hostname, "");
ota.onStart([]() { display.showBoot("OTA..."); });
ota.onProgress([](int p) { display.showProgress(p); });
```

**パーティション注意**:
- **min_SPIFFS使用必須**
- huge_app禁止（OTA領域消失）

---

### 2.8 Operator

**使用方法**: そのまま使用

```cpp
#include "Operator.h"

Operator op;
op.setMode(OperatorMode::PROVISION);
// ...設定完了後
op.setMode(OperatorMode::RUN);
```

**IS06Sモード遷移**:

```
BOOT → PROVISION → RUN
         │
         └──→ AP_MODE (WiFi接続失敗時)
```

---

### 2.9 LacisIDGenerator（必須）

**使用方法**: lacisID生成（IoT動作に必須）

```cpp
#include "LacisIDGenerator.h"

LacisIDGenerator lacisGen;
// IS06S: ProductType=006, ProductCode=0200
String myLacisId = lacisGen.generate("006", "0200");
String myMac = lacisGen.getStaMac12Hex();
```

**LacisIDフォーマット**:
```
[Prefix=3][ProductType=3桁][MAC=12桁][ProductCode=4桁] = 20文字
※ SSoT: CLAUDE.md準拠
例: 300612345678ABCD0200
```

**重要**: lacisIDはIoT動作の必須要件。AraneaRegisterでのCIC取得に使用。

---

### 2.10 AraneaRegister（必須）

**使用方法**: CIC取得（IoT動作に必須）

```cpp
#include "AraneaRegister.h"

AraneaRegister araneaReg;

// 初期化
String gateUrl = settings.getString("gate_url", "");
araneaReg.begin(gateUrl);

// テナントプライマリ設定
TenantPrimaryAuth tenantAuth;
tenantAuth.lacisId = settings.getString("tenant_lacisid", "");
tenantAuth.userId = settings.getString("tenant_email", "");
tenantAuth.cic = settings.getString("tenant_cic", "");
araneaReg.setTenantPrimary(tenantAuth);

// デバイス登録
String myTid = settings.getString("tid", "");
AraneaRegisterResult regResult = araneaReg.registerDevice(
    myTid, "ar-is06s", myLacisId, myMac, "006", "0200"
);

if (regResult.ok) {
    myCic = regResult.cic_code;
    settings.setString("cic", myCic);
} else {
    myCic = settings.getString("cic", "");  // フォールバック
}
```

**重要**: CICはIoT動作の必須要件。取得失敗時はNVS保存済みCICを使用。

---

## 3. IS06S固有モジュール

### 3.1 Is06PinManager

**役割**: PIN制御のコアロジック

**ファイル**: `Is06PinManager.h`, `Is06PinManager.cpp`

**責務**:
1. GPIO初期化（D/P, I/O, System）
2. Digital Output制御（Momentary/Alternate）
3. PWM Output制御（Slow/Rapid）
4. Digital Input検知・連動
5. 状態変化コールバック
6. NVS永続化連携

**クラス概要**:
```cpp
class Is06PinManager {
public:
    void begin(SettingManager* settings);
    void update();  // loop()で呼び出し

    // 状態操作
    bool setPinState(const String& channel, const String& state);
    bool setPinState(const String& channel, const String& state, int validity);
    PinState getPinState(const String& channel) const;

    // 設定操作
    bool setPinSetting(const String& channel, const PinSetting& setting);
    PinSetting getPinSetting(const String& channel) const;

    // JSON変換
    void toJson(JsonObject& doc) const;
    bool fromJson(const JsonObject& doc);

    // コールバック
    void onStateChange(StateChangeCallback callback);

    // System PIN
    void handleSystemButton(int gpio, unsigned long pressedMs);
};
```

**is04からの移植元**:
| 機能 | 移植元 | 変更点 |
|------|--------|--------|
| GPIO初期化 | is04.ino | 6ch対応に拡張 |
| デバウンス | is04.ino | なし |
| パルス制御 | PulseController | 4ch対応に拡張 |
| PWM制御 | 新規 | LEDC使用 |

---

### 3.2 HttpManagerIs06s

**役割**: Web UI固有実装

**ファイル**: `HttpManagerIs06s.h`, `HttpManagerIs06s.cpp`

**責務**:
1. AraneaWebUI継承
2. IS06S固有エンドポイント実装
3. PIN操作ページ生成
4. PIN設定ページ生成

**クラス概要**:
```cpp
class HttpManagerIs06s : public AraneaWebUI {
public:
    void begin(SettingManager* settings, int port);
    void setupRoutes();

    void setPinManager(Is06PinManager* pinManager);
    void setStateReporter(StateReporterIs06s* reporter);

private:
    Is06PinManager* pinManager_;
    StateReporterIs06s* reporter_;

    // API
    void handleGetStatus();
    void handleGetPinState();
    void handlePostPinState();
    void handleGetPinSetting();
    void handlePostPinSetting();
    void handleGetGlobalSettings();
    void handlePostGlobalSettings();

    // ページ
    void handlePinControlPage();
    void handlePinSettingPage();
};
```

---

### 3.3 StateReporterIs06s

**役割**: 状態送信ペイロード構築

**ファイル**: `StateReporterIs06s.h`, `StateReporterIs06s.cpp`

**責務**:
1. 認証情報管理
2. 状態レポートペイロード構築
3. HTTP POST送信

**クラス概要**:
```cpp
class StateReporterIs06s {
public:
    void setAuth(const String& tid, const String& lacisId, const String& cic);
    void setReportUrl(const String& url);

    String buildPayload(const JsonObject& pinStates,
                        const String& ip, int rssi,
                        const String& timestamp);
    bool sendReport(const String& payload);

private:
    String tid_;
    String lacisId_;
    String cic_;
    String reportUrl_;
};
```

---

## 4. ファイル一覧（実装予定）

```
is06s/
├── is06s.ino                    # メインスケッチ
├── AraneaGlobalImporter.h       # 共通モジュールインポート
├── Is06PinManager.h             # PIN管理ヘッダ
├── Is06PinManager.cpp           # PIN管理実装
├── HttpManagerIs06s.h           # Web UIヘッダ
├── HttpManagerIs06s.cpp         # Web UI実装
├── StateReporterIs06s.h         # 状態送信ヘッダ
├── StateReporterIs06s.cpp       # 状態送信実装
├── data/                        # SPIFFS（Web UI静的ファイル）
│   ├── index.html
│   ├── pincontrol.html
│   ├── pinsetting.html
│   └── style.css
└── design/                      # 設計ドキュメント
    ├── Design_Index.md
    ├── IS06S_BasicSpecification.md
    ├── IS06S_DetailedDesign.md
    ├── IS06S_ModuleAdaptationPlan.md  # 本ドキュメント
    ├── IS06S_TaskList.md
    └── IS06S_TestPlan.md
```

---

## 5. 依存関係図

```
┌─────────────────────────────────────────────────────────────────┐
│                         is06s.ino                                │
└─────────────────────────────┬───────────────────────────────────┘
                              │
    ┌─────────────────────────┼─────────────────────────┐
    │                         │                         │
    ▼                         ▼                         ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│Is06PinManager │    │HttpMgrIs06s   │    │StateReporter  │
│               │◄───│               │    │   Is06s       │
│               │    │               │───►│               │
└───────┬───────┘    └───────┬───────┘    └───────────────┘
        │                    │
        │      ┌─────────────┤
        │      │             │
        ▼      ▼             ▼
┌─────────────────────────────────────────────────────────────────┐
│                    global/src/ (共通モジュール)                  │
├────────────┬────────────┬────────────┬────────────┬────────────┤
│WiFiManager │SettingMgr  │DisplayMgr  │ NtpManager │MqttManager │
├────────────┼────────────┼────────────┼────────────┼────────────┤
│AraneaWebUI │HttpOtaMgr  │ Operator   │LacisIDGen  │AraneaReg   │
│            │            │            │  (必須)     │  (必須)    │
└────────────┴────────────┴────────────┴────────────┴────────────┘
```

---

## 6. 移植・実装ガイドライン

### 6.1 is04からの移植

| 機能 | 移植方法 |
|------|----------|
| GPIO初期化 | CH1-6に拡張 |
| デバウンス | そのまま |
| パルス制御 | 配列化して4ch対応 |
| 状態送信 | type名変更 (ar-is04a → ar-is06s) |
| Web UI | ページ追加（PINControl, PINSetting） |

### 6.2 新規実装

| 機能 | 実装方法 |
|------|----------|
| PWM出力 | LEDC API使用 |
| PWM Slow変化 | millis()ベースの漸進制御 |
| Input連動 | allocation配列参照 |
| 設定同期 | MQTT subscribe + NVS更新 |

### 6.3 削除禁止コード

- AraneaRegister関連 → **必須（CIC取得）**
- LacisIDGenerator関連 → **必須（lacisID生成）**
- TID/CIC認証 → **必須（IoT動作）**

---

## 7. MECE確認

### 7.1 モジュール責務分離

| 機能領域 | モジュール | 重複なし |
|---------|-----------|---------|
| PIN制御 | Is06PinManager | ✅ |
| Web UI/API | HttpManagerIs06s | ✅ |
| 状態送信 | StateReporterIs06s | ✅ |
| WiFi | WiFiManager (global) | ✅ |
| 設定 | SettingManager (global) | ✅ |
| MQTT | MqttManager (global) | ✅ |
| 表示 | DisplayManager (global) | ✅ |
| 認証 | AraneaRegister (global) | ✅ |
| OTA | HttpOtaManager (global) | ✅ |

### 7.2 ファイル分類

| カテゴリ | ファイル数 | 完全網羅 |
|---------|-----------|---------|
| メインスケッチ | 1 | ✅ |
| 固有モジュール | 6 (h+cpp x 3) | ✅ |
| インポータ | 1 | ✅ |
| 静的リソース | 4 | ✅ |
| 設計ドキュメント | 6 | ✅ |

---

## 8. 参照

- **is04a設計**: `code/ESP32/is04a/design/`
- **is05a設計**: `code/ESP32/is05a/design/`
- **共通モジュール**: `code/ESP32/global/src/`
- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`

---

## 9. 変更履歴

| 日付 | バージョン | 変更内容 | 担当 |
|------|-----------|----------|------|
| 2025/01/23 | 1.0 | 初版作成 | Claude |
| 2025/01/23 | 1.1 | ProductCodeを0200に変更、NVSキー追加（enabled, roc, pinglobal） | Claude |
