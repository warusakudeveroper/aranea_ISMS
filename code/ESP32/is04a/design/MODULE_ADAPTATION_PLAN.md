# is04a モジュール適応計画

**作成日**: 2025/12/22
**目的**: global/モジュールの再利用と固有モジュールの設計

---

## 1. アーキテクチャ概要

```
code/ESP32/
├── global/src/              ← 共通モジュール（変更なし）
│   ├── WiFiManager.h/cpp
│   ├── SettingManager.h/cpp
│   ├── DisplayManager.h/cpp
│   ├── NtpManager.h/cpp
│   ├── MqttManager.h/cpp
│   ├── AraneaWebUI.h/cpp
│   ├── HttpOtaManager.h/cpp
│   ├── Operator.h/cpp
│   ├── LacisIDGenerator.h/cpp  ← 必須
│   └── AraneaRegister.h/cpp    ← 必須
│
└── is04a/                   ← is04a固有
    ├── is04a.ino            ← メインスケッチ
    ├── AraneaGlobalImporter.h ← 共通モジュールインポート
    ├── HttpManagerIs04a.h/cpp ← Web UI（固有API実装）
    ├── StateReporterIs04a.h/cpp ← 状態送信ペイロード構築
    ├── PulseController.h/cpp ← パルス出力制御
    └── design/              ← 設計ドキュメント
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

**変更不要**: is04（ISMS版）と同じ使用方法

---

### 2.2 SettingManager

**使用方法**: そのまま使用

```cpp
#include "SettingManager.h"

SettingManager settings;
settings.begin();
String ssid = settings.getString("wifi_ssid", "");
int pulseMs = settings.getInt("pulse_ms", 3000);
```

**is04a固有のNVSキー**:
- `state_report_url`: 状態送信先URL
- `mqtt_broker`: MQTTブローカーURL
- `mqtt_user`, `mqtt_pass`: MQTT認証
- `api_key`: 認証用APIキー
- `output1_name`, `output2_name`: 出力名
- `pulse_ms`, `interlock_ms`: パルス設定
- `input1_action`, `input2_action`: 入力アクション

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

**表示内容の変更**:
- CIC表示 → デバイスID表示に変更
- sensorLine: パルス状態表示

---

### 2.4 NtpManager

**使用方法**: そのまま使用

```cpp
#include "NtpManager.h"

NtpManager ntp;
ntp.sync();
String timestamp = ntp.getIso8601();
```

---

### 2.5 MqttManager

**使用方法**: is10と同様のパターン

```cpp
#include "MqttManager.h"

MqttManager mqtt;
mqtt.begin(brokerUrl, deviceId, apiKey);
mqtt.subscribe("device/" + deviceId + "/command", onCommand);
mqtt.publish("device/" + deviceId + "/ack", ackJson);
```

**is04との違い**:
- is04: esp_mqtt_client直接使用 + AraneaGateway認証
- is04a: MqttManagerラッパー使用 + APIキー認証

---

### 2.6 AraneaWebUI

**使用方法**: 継承して固有API実装

```cpp
#include "AraneaWebUI.h"

class HttpManagerIs04a : public AraneaWebUI {
public:
    void begin(SettingManager* settings, int port) override;
    void setupRoutes() override;

private:
    void handleStatus();
    void handleConfig();
    void handlePulse();
};
```

**固有エンドポイント**:
- `/api/pulse`: パルス実行API
- `/api/status`: 現在の入出力状態

---

### 2.7 HttpOtaManager

**使用方法**: そのまま使用

```cpp
#include "HttpOtaManager.h"

HttpOtaManager ota;
ota.begin(hostname, "");
ota.onStart([]() { /* ... */ });
ota.onProgress([](int p) { /* ... */ });
```

---

### 2.8 Operator

**使用方法**: そのまま使用

```cpp
#include "Operator.h"

Operator op;
op.setMode(OperatorMode::PROVISION);
// ...
op.setMode(OperatorMode::RUN);
```

---

### 2.9 LacisIDGenerator（必須）

**使用方法**: lacisID生成（IoT動作に必須）

```cpp
#include "LacisIDGenerator.h"

LacisIDGenerator lacisGen;
String myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
String myMac = lacisGen.getStaMac12Hex();
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
    myTid, DEVICE_TYPE, myLacisId, myMac, PRODUCT_TYPE, PRODUCT_CODE
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

## 3. is04a固有モジュール

### 3.1 PulseController

**役割**: パルス出力の状態管理

```cpp
// PulseController.h
class PulseController {
public:
    void begin(int out1Pin, int out2Pin);
    bool startPulse(int outputNum, int durationMs);
    void update();  // loop()で呼び出し
    bool isActive() const;

    // コールバック
    void onPulseStart(std::function<void(int, int)> cb);
    void onPulseEnd(std::function<void(int)> cb);

private:
    int out1Pin_, out2Pin_;
    bool active_;
    int activePin_;
    unsigned long startMs_;
    unsigned long durationMs_;
};
```

**is04からの移植元**:
- `startPulseWithSource()`
- `updatePulse()`
- `endPulse()`

---

### 3.2 StateReporterIs04a

**役割**: 状態送信ペイロードの構築

```cpp
// StateReporterIs04a.h
class StateReporterIs04a {
public:
    void setAuth(const String& tid, const String& lacisId, const String& cic);
    void setReportUrl(const String& url);

    String buildPayload(bool out1, bool out2, bool in1, bool in2,
                        const String& ip, int rssi, const String& timestamp);
    bool sendReport(const String& payload);

private:
    String tid_;
    String lacisId_;
    String cic_;
    String reportUrl_;
};
```

**ペイロード構造（is04と同一）**:
```json
{
    "auth": {
        "tid": "T2025...",
        "lacisId": "3004AABBCCDDEEFF0001",
        "cic": "123456"
    },
    "report": {
        "lacisId": "3004AABBCCDDEEFF0001",
        "type": "ar-is04a",
        "observedAt": "2025-12-22T08:00:00Z",
        "state": { ... }
    }
}
```

**重要**: auth.tid, auth.lacisId, auth.cic はIoT動作に必須。

---

### 3.3 HttpManagerIs04a

**役割**: Web UI固有実装

```cpp
// HttpManagerIs04a.h
class HttpManagerIs04a : public AraneaWebUI {
public:
    void begin(SettingManager* settings, int port);
    void setupRoutes();

    // is04a固有
    void setPulseController(PulseController* pulse);
    void setStateReporter(StateReporterIs04a* reporter);

private:
    PulseController* pulse_;
    StateReporterIs04a* reporter_;

    void handlePulse();
    void handleStatus();
};
```

---

## 4. 移植元コード分析

### 4.1 is04.ino からの移植

| 機能 | 移植先 | 変更点 |
|------|--------|--------|
| GPIO初期化 | is04a.ino | なし |
| デバウンス処理 | is04a.ino | なし |
| パルス制御 | PulseController | クラス化 |
| MQTT処理 | MqttManager使用 | ラッパー経由 |
| 状態送信 | StateReporterIs04a | ペイロード簡略化 |
| Web UI | HttpManagerIs04a | AraneaWebUI継承 |

### 4.2 変更・簡略化するコード

- クラウドURL固定値 → 設定可能に
- ISMS専用設定（is04_dids等） → 汎用名に変更
- テナント情報 → NVSから設定可能に

**削除禁止**:
- AraneaRegister関連 → **必須（CIC取得）**
- LacisIDGenerator関連 → **必須（lacisID生成）**
- TID/CIC認証 → **必須（IoT動作）**

---

## 5. 開発優先順位

### P0: 必須（初期動作）
1. GPIO入出力
2. パルス出力（非ブロッキング）
3. WiFi接続
4. Web UI（設定画面）

### P1: 基本機能
5. HTTP状態送信
6. OLED表示
7. OTA更新

### P2: 拡張機能
8. MQTT受信/制御
9. 設定永続化完全対応

---

## 6. テスト計画

### 6.1 単体テスト

| テスト項目 | 確認内容 |
|-----------|----------|
| GPIO出力 | TRG_OUT1/2がHIGH/LOW切替 |
| GPIO入力 | PHYS_IN1/2のデバウンス動作 |
| パルス | 指定時間後にLOWに戻る |
| WiFi | SSID/PASS設定で接続 |

### 6.2 統合テスト

| テスト項目 | 確認内容 |
|-----------|----------|
| 手動トリガー | 物理入力→パルス出力→状態送信 |
| Web制御 | /api/pulse→パルス出力 |
| MQTT制御 | コマンド受信→パルス出力→ACK送信 |

---

## 7. ファイル一覧（予定）

```
is04a/
├── is04a.ino                    # メインスケッチ
├── AraneaGlobalImporter.h       # 共通モジュールインポート
├── PulseController.h            # パルス制御
├── PulseController.cpp
├── StateReporterIs04a.h         # 状態送信
├── StateReporterIs04a.cpp
├── HttpManagerIs04a.h           # Web UI
├── HttpManagerIs04a.cpp
└── design/
    ├── DESIGN.md                # 設計書
    └── MODULE_ADAPTATION_PLAN.md # 本ドキュメント
```

---

## 8. 参照

- **is04 (ISMS版)**: `archive_ISMS/ESP32/is04/is04.ino`
- **is10 (参考実装)**: `code/ESP32/is10/`
- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
