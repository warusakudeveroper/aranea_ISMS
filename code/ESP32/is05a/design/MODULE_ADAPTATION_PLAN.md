# is05a モジュール適応計画

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
│   ├── AraneaWebUI.h/cpp
│   ├── HttpOtaManager.h/cpp
│   ├── Operator.h/cpp
│   ├── LacisIDGenerator.h/cpp  ← 必須
│   └── AraneaRegister.h/cpp    ← 必須
│
└── is05a/                   ← is05a固有
    ├── is05a.ino            ← メインスケッチ
    ├── AraneaGlobalImporter.h ← 共通モジュールインポート
    ├── HttpManagerIs05a.h/cpp ← Web UI（固有API実装）
    ├── StateReporterIs05a.h/cpp ← 状態送信ペイロード構築
    ├── ChannelManager.h/cpp ← 8ch入力管理
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

---

### 2.2 SettingManager

**使用方法**: そのまま使用

```cpp
#include "SettingManager.h"

SettingManager settings;
settings.begin();
int ch1Pin = settings.getInt("is05a_ch1_pin", 4);
String ch1Name = settings.getString("is05a_ch1_name", "ch1");
```

---

### 2.3 DisplayManager

**使用方法**: そのまま使用

```cpp
#include "DisplayManager.h"

DisplayManager display;
display.begin();
display.showIs02Main(line1, cicStr, sensorLine, false);
```

**表示内容**:
- Line1: IP + RSSI
- CIC: デバイスCIC
- sensorLine: 状態変化のあったチャンネル表示

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

### 2.5 LacisIDGenerator（必須）

**使用方法**: lacisID生成（IoT動作に必須）

```cpp
#include "LacisIDGenerator.h"

LacisIDGenerator lacisGen;
String myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
String myMac = lacisGen.getStaMac12Hex();
```

**重要**: lacisIDはIoT動作の必須要件。AraneaRegisterでのCIC取得に使用。

---

### 2.6 AraneaRegister（必須）

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

### 2.7 AraneaWebUI

**使用方法**: 継承して固有API実装

```cpp
#include "AraneaWebUI.h"

class HttpManagerIs05a : public AraneaWebUI {
public:
    void begin(SettingManager* settings, int port) override;
    void setupRoutes() override;

private:
    void handleStatus();
    void handleConfig();
};
```

---

### 2.8 HttpOtaManager

**使用方法**: そのまま使用

```cpp
#include "HttpOtaManager.h"

HttpOtaManager ota;
ota.begin(hostname, "");
```

---

### 2.9 Operator

**使用方法**: そのまま使用

```cpp
#include "Operator.h"

Operator op;
op.setMode(OperatorMode::PROVISION);
// ...
op.setMode(OperatorMode::RUN);
```

---

## 3. is05a固有モジュール

### 3.1 ChannelManager

**役割**: 8ch入力の状態管理

```cpp
// ChannelManager.h
class ChannelManager {
public:
    static const int NUM_CHANNELS = 8;

    void begin(SettingManager* settings);
    void sample();  // loop()で呼び出し
    bool hasChanged() const;
    void clearChanged();

    // 状態取得
    String getStateString(int ch) const;
    bool isActive(int ch) const;
    String getLastUpdatedAt(int ch) const;

    // 設定
    struct ChannelConfig {
        int pin;
        String name;
        String meaning;  // "open" or "close"
    };
    ChannelConfig getConfig(int ch) const;

private:
    SettingManager* settings_;
    int pins_[NUM_CHANNELS];
    String names_[NUM_CHANNELS];
    String meanings_[NUM_CHANNELS];

    int rawState_[NUM_CHANNELS];
    int stableState_[NUM_CHANNELS];
    int stableCount_[NUM_CHANNELS];
    String lastUpdatedAt_[NUM_CHANNELS];
    bool changed_;
};
```

**is05からの移植元**:
- `loadChannelConfig()`
- `initGPIO()`
- `sampleInputs()`
- `getStateString()`

---

### 3.2 StateReporterIs05a

**役割**: 状態送信ペイロードの構築

```cpp
// StateReporterIs05a.h
class StateReporterIs05a {
public:
    void setAuth(const String& tid, const String& lacisId, const String& cic);
    void setReportUrl(const String& url);

    String buildPayload(ChannelManager& channels,
                        const String& ip, int rssi, const String& timestamp);
    bool sendReport(const String& payload);

private:
    String tid_;
    String lacisId_;
    String cic_;
    String reportUrl_;
};
```

**ペイロード構造**:
```json
{
    "auth": {
        "tid": "T2025...",
        "lacisId": "3005AABBCCDDEEFF0001",
        "cic": "123456"
    },
    "report": {
        "lacisId": "3005AABBCCDDEEFF0001",
        "type": "ar-is05a",
        "observedAt": "2025-12-22T08:00:00Z",
        "state": {
            "ch1": "open",
            "ch2": "close",
            ...
        }
    }
}
```

**重要**: auth.tid, auth.lacisId, auth.cic はIoT動作に必須。

---

### 3.3 HttpManagerIs05a

**役割**: Web UI固有実装

```cpp
// HttpManagerIs05a.h
class HttpManagerIs05a : public AraneaWebUI {
public:
    void begin(SettingManager* settings, int port);
    void setupRoutes();

    void setChannelManager(ChannelManager* channels);

private:
    ChannelManager* channels_;

    void handleStatus();
    void handleConfig();
};
```

---

## 4. 移植元コード分析

### 4.1 is05.ino からの移植

| 機能 | 移植先 | 変更点 |
|------|--------|--------|
| チャンネル設定読み込み | ChannelManager | クラス化 |
| GPIO初期化 | ChannelManager | クラス化 |
| デバウンス処理 | ChannelManager | クラス化 |
| 状態送信 | StateReporterIs05a | ペイロード構築 |
| Web UI | HttpManagerIs05a | AraneaWebUI継承 |

### 4.2 変更・簡略化するコード

- クラウドURL固定値 → 設定可能に
- ISMS専用設定（is05_ch*_did等） → 汎用名に変更
- テナント情報 → NVSから設定可能に

**削除禁止**:
- AraneaRegister関連 → **必須（CIC取得）**
- LacisIDGenerator関連 → **必須（lacisID生成）**
- TID/CIC認証 → **必須（IoT動作）**

---

## 5. 開発優先順位

### P0: 必須（初期動作）
1. GPIO入力（8ch）
2. デバウンス処理
3. WiFi接続
4. Web UI（設定画面）

### P1: 基本機能
5. LacisID生成
6. AraneaRegister連携
7. HTTP状態送信
8. OLED表示
9. OTA更新

### P2: 拡張機能
10. 設定永続化完全対応
11. ティッカー表示

---

## 6. テスト計画

### 6.1 単体テスト

| テスト項目 | 確認内容 |
|-----------|----------|
| GPIO入力 | 8chすべてのHIGH/LOW検出 |
| デバウンス | 5回連続一致で確定 |
| meaning変換 | open/close正しく変換 |
| WiFi | SSID/PASS設定で接続 |

### 6.2 統合テスト

| テスト項目 | 確認内容 |
|-----------|----------|
| 状態変化送信 | リードスイッチ開閉→HTTP POST |
| 心拍送信 | 60秒間隔で定期送信 |
| CIC取得 | AraneaRegister連携 |

---

## 7. ファイル一覧（予定）

```
is05a/
├── is05a.ino                    # メインスケッチ
├── AraneaGlobalImporter.h       # 共通モジュールインポート
├── ChannelManager.h             # 8ch入力管理
├── ChannelManager.cpp
├── StateReporterIs05a.h         # 状態送信
├── StateReporterIs05a.cpp
├── HttpManagerIs05a.h           # Web UI
├── HttpManagerIs05a.cpp
└── design/
    ├── DESIGN.md                # 設計書
    └── MODULE_ADAPTATION_PLAN.md # 本ドキュメント
```

---

## 8. 参照

- **is05 (ISMS版)**: `archive_ISMS/ESP32/is05/is05.ino`
- **is10 (参考実装)**: `code/ESP32/is10/`
- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
