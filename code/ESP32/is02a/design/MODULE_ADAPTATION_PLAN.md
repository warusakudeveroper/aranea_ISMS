# is02a モジュール適応計画

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
└── is02a/                   ← is02a固有
    ├── is02a.ino            ← メインスケッチ
    ├── AraneaGlobalImporter.h ← 共通モジュールインポート
    ├── HttpManagerIs02a.h/cpp ← Web UI（固有API実装）
    ├── BleScanner.h/cpp     ← BLEスキャン管理
    ├── RelayManager.h/cpp   ← HTTP中継管理
    ├── SensorManager.h/cpp  ← HT-30自己計測
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
String relayUrl = settings.getString("relay_url", "");
int batchInterval = settings.getInt("batch_interval_sec", 30);
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
- sensorLine: 受信デバイス数/最終受信時刻

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

class HttpManagerIs02a : public AraneaWebUI {
public:
    void begin(SettingManager* settings, int port) override;
    void setupRoutes() override;

private:
    void handleStatus();
    void handleConfig();
    void handleBuffer();
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

## 3. is02a固有モジュール

### 3.1 BleScanner

**役割**: BLEスキャンとISMSv1パース

```cpp
// BleScanner.h
class BleScanner {
public:
    void begin();
    void startScan();
    void stopScan();

    bool hasNewData() const;
    std::vector<ISMSv1Payload> getBuffer();
    void clearBuffer();

    // コールバック
    void onReceive(std::function<void(const ISMSv1Payload&)> cb);

private:
    std::vector<ISMSv1Payload> buffer_;
    BLEScan* pBLEScan_;

    ISMSv1Payload parseISMSv1(const String& raw);
};

struct ISMSv1Payload {
    String lacisId;
    String cic;
    float temperature;
    float humidity;
    int batteryPct;
    unsigned long receivedAt;
    int rssi;

    bool isValid() const;
};
```

**is02からの移植元**:
- BLEスキャンコールバック
- ISMSv1パース処理
- バッファ管理

---

### 3.2 RelayManager

**役割**: HTTP中継とバッチ送信

```cpp
// RelayManager.h
class RelayManager {
public:
    void begin(const String& primaryUrl, const String& secondaryUrl);
    void setAuth(const String& tid, const String& lacisId, const String& cic);

    void addEvent(const ISMSv1Payload& payload);
    void setSelfData(float temp, float hum);

    bool sendBatch();  // バッチ送信実行
    int getBufferedCount() const;

private:
    String primaryUrl_;
    String secondaryUrl_;
    String tid_;
    String lacisId_;
    String cic_;

    std::vector<ISMSv1Payload> eventBuffer_;
    float selfTemp_;
    float selfHum_;

    String buildPayload();
    bool httpPost(const String& url, const String& payload);
};
```

**ペイロード構造**:
```json
{
    "auth": {
        "tid": "T2025...",
        "lacisId": "3002AABBCCDDEEFF0001",
        "cic": "123456"
    },
    "gateway": {
        "lacisId": "...",
        "cic": "..."
    },
    "events": [...],
    "self": {
        "temperature": 26.0,
        "humidity": 60.0
    }
}
```

**重要**: auth.tid, auth.lacisId, auth.cic はIoT動作に必須。

---

### 3.3 SensorManager

**役割**: HT-30自己計測（is01aと共通設計）

```cpp
// SensorManager.h
class SensorManager {
public:
    bool begin();
    bool read();

    float getTemperature() const;
    float getHumidity() const;
    bool isValid() const;

private:
    float temperature_;
    float humidity_;
    bool valid_;
};
```

---

### 3.4 HttpManagerIs02a

**役割**: Web UI固有実装

```cpp
// HttpManagerIs02a.h
class HttpManagerIs02a : public AraneaWebUI {
public:
    void begin(SettingManager* settings, int port);
    void setupRoutes();

    void setBleScanner(BleScanner* scanner);
    void setRelayManager(RelayManager* relay);

private:
    BleScanner* scanner_;
    RelayManager* relay_;

    void handleStatus();
    void handleBuffer();
};
```

---

## 4. 移植元コード分析

### 4.1 is02.ino からの移植

| 機能 | 移植先 | 変更点 |
|------|--------|--------|
| BLEスキャン | BleScanner | クラス化 |
| ISMSv1パース | BleScanner | そのまま |
| バッファ管理 | RelayManager | バッチ送信対応 |
| HTTP送信 | RelayManager | 冗長化対応 |
| HT-30計測 | SensorManager | クラス化 |
| Web UI | HttpManagerIs02a | AraneaWebUI継承 |

### 4.2 変更・簡略化するコード

- 送信先URL固定値 → NVSから設定可能に
- cluster1-6試行 → WiFi設定を汎用化
- ISMS専用設定 → 汎用名に変更

**削除禁止**:
- AraneaRegister関連 → **必須（CIC取得）**
- LacisIDGenerator関連 → **必須（lacisID生成）**
- ISMSv1パース → **必須（BLE通信プロトコル）**
- TID/CIC認証 → **必須（IoT動作）**

---

## 5. 開発優先順位

### P0: 必須（初期動作）
1. WiFi接続
2. BLEスキャン
3. ISMSv1パース
4. OLED表示

### P1: 基本機能
5. LacisID生成【必須】
6. AraneaRegister連携【必須】
7. HTTP中継（バッチ送信）
8. HT-30自己計測

### P2: 拡張機能
9. Web UI実装
10. OTA更新
11. 冗長化（複数URL対応）
12. リトライ処理

---

## 6. テスト計画

### 6.1 単体テスト

| テスト項目 | 確認内容 |
|-----------|----------|
| BLEスキャン | is01アドバタイズ検出 |
| ISMSv1パース | 40バイトペイロード解析 |
| HT-30計測 | 温湿度値取得 |
| WiFi | SSID/PASS設定で接続 |

### 6.2 統合テスト

| テスト項目 | 確認内容 |
|-----------|----------|
| is01受信→中継 | BLE受信→HTTP POST |
| バッチ送信 | 30秒間隔でまとめて送信 |
| 冗長化 | プライマリ失敗→セカンダリへ |
| CIC取得 | AraneaRegister連携 |

---

## 7. ファイル一覧（予定）

```
is02a/
├── is02a.ino                    # メインスケッチ
├── AraneaGlobalImporter.h       # 共通モジュールインポート
├── BleScanner.h                 # BLEスキャン
├── BleScanner.cpp
├── RelayManager.h               # HTTP中継
├── RelayManager.cpp
├── SensorManager.h              # HT-30計測
├── SensorManager.cpp
├── HttpManagerIs02a.h           # Web UI
├── HttpManagerIs02a.cpp
└── design/
    ├── DESIGN.md                # 設計書
    └── MODULE_ADAPTATION_PLAN.md # 本ドキュメント
```

---

## 8. BLEとWiFiの共存

### 8.1 ESP32制約

- BLE + WiFi同時動作は可能だが、パフォーマンス低下あり
- BLEスキャン中のWiFi送信は遅延する場合あり

### 8.2 対策

```cpp
// バッチ送信時はBLEスキャンを一時停止
void sendBatchWithBlePause() {
    bleScanner.stopScan();
    delay(10);

    relay.sendBatch();

    bleScanner.startScan();
}
```

---

## 9. 参照

- **is01 (BLE送信側)**: `code/ESP32/is01a/`
- **is02 (ISMS版)**: `archive_ISMS/ESP32/is02/is02.ino`
- **is03 (受信サーバー)**: `code/ESP32/is03/`
- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
