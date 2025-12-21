# is01a モジュール適応計画

**作成日**: 2025/12/22
**目的**: global/モジュールの再利用と固有モジュールの設計

---

## 1. アーキテクチャ概要

```
code/ESP32/
├── global/src/              ← 共通モジュール（変更なし）
│   ├── WiFiManager.h/cpp       ← 初回のみ使用
│   ├── SettingManager.h/cpp
│   ├── DisplayManager.h/cpp
│   ├── NtpManager.h/cpp        ← 初回のみ使用
│   ├── Operator.h/cpp
│   ├── LacisIDGenerator.h/cpp  ← 必須
│   └── AraneaRegister.h/cpp    ← 必須
│
└── is01a/                   ← is01a固有
    ├── is01a.ino            ← メインスケッチ
    ├── AraneaGlobalImporter.h ← 共通モジュールインポート
    ├── SensorManager.h/cpp  ← HT-30センサー管理
    ├── BleAdvertiser.h/cpp  ← BLEアドバタイズ
    └── design/              ← 設計ドキュメント
```

---

## 2. 共通モジュール使用計画

### 2.1 WiFiManager（初回のみ）

**使用方法**: アクティベーション時のみ使用

```cpp
#include "WiFiManager.h"

// 初回起動時のみ
if (!settings.getInt("activated", 0)) {
    WiFiManager wifi;
    wifi.connectWithSettings(&settings);
    // AraneaRegister処理...
}
```

**通常動作**: WiFiは使用しない（電池節約）

---

### 2.2 SettingManager

**使用方法**: そのまま使用（NVS）

```cpp
#include "SettingManager.h"

SettingManager settings;
settings.begin();

// 読み込み
int activated = settings.getInt("activated", 0);
String myCic = settings.getString("cic", "");
String myLacisId = settings.getString("lacis_id", "");

// 保存
settings.setString("cic", myCic);
settings.setInt("activated", 1);
```

---

### 2.3 DisplayManager

**使用方法**: そのまま使用（I2C OLED）

```cpp
#include "DisplayManager.h"

DisplayManager display;
display.begin();  // I2C直列処理内で呼び出し
display.showBoot("Measuring...");
display.showIs01Main(tempStr, humStr, battStr);
```

**表示内容**:
- Line1: 温度
- Line2: 湿度
- Line3: 電池残量

---

### 2.4 NtpManager（初回のみ）

**使用方法**: アクティベーション時のみ使用

```cpp
#include "NtpManager.h"

// 初回起動時のみ
if (!settings.getInt("activated", 0)) {
    NtpManager ntp;
    ntp.sync();
}
```

**通常動作**: NTPは使用しない（WiFi不使用のため）

---

### 2.5 LacisIDGenerator（必須）

**使用方法**: lacisID生成（IoT動作に必須）

```cpp
#include "LacisIDGenerator.h"

LacisIDGenerator lacisGen;
String myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
String myMac = lacisGen.getStaMac12Hex();

// NVS保存
settings.setString("lacis_id", myLacisId);
```

**重要**: lacisIDはIoT動作の必須要件。BLEペイロードに含まれる。

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
tenantAuth.pass = settings.getString("tenant_pass", "");
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
    settings.setInt("activated", 1);
} else {
    myCic = settings.getString("cic", "");  // フォールバック
}
```

**重要**: CICはIoT動作の必須要件。取得失敗時はNVS保存済みCICを使用。

---

### 2.7 Operator

**使用方法**: そのまま使用

```cpp
#include "Operator.h"

Operator op;
op.setMode(OperatorMode::PROVISION);  // アクティベーション中
// ...
op.setMode(OperatorMode::RUN);        // 通常動作
```

---

## 3. is01a固有モジュール

### 3.1 SensorManager

**役割**: HT-30センサーの管理

```cpp
// SensorManager.h
class SensorManager {
public:
    bool begin();  // I2C初期化後に呼び出し
    bool read();   // 計測実行

    float getTemperature() const;
    float getHumidity() const;
    bool isValid() const;

    String getTemperatureString() const;  // "+25.5" 形式
    String getHumidityString() const;     // "065.0" 形式

private:
    float temperature_;
    float humidity_;
    bool valid_;
};
```

**is01からの移植元**:
- `readSensor()`
- `formatTemperature()`
- `formatHumidity()`

---

### 3.2 BleAdvertiser

**役割**: BLEアドバタイズ管理

```cpp
// BleAdvertiser.h
class BleAdvertiser {
public:
    void begin();
    void setPayload(const String& lacisId, const String& cic,
                    float temp, float hum, int battPct);
    void advertise(int seconds);
    void stop();

private:
    BLEAdvertising* pAdvertising_;
    String buildISMSv1Payload(const String& lacisId, const String& cic,
                               float temp, float hum, int battPct);
};
```

**ISMSv1ペイロード構築**:
```cpp
String BleAdvertiser::buildISMSv1Payload(...) {
    // "3" + lacisId(20) + cic(6) + temp(5) + hum(5) + batt(3) = 40 bytes
    String payload = "3";
    payload += lacisId;  // 20文字
    payload += String(cic).substring(0, 6);  // 6文字（0埋め）
    payload += formatTemp(temp);  // 5文字 ("+25.5")
    payload += formatHum(hum);    // 5文字 ("065.0")
    payload += formatBatt(batt);  // 3文字 ("085")
    return payload;
}
```

---

### 3.3 BatteryMonitor

**役割**: 電池残量監視

```cpp
// BatteryMonitor.h
class BatteryMonitor {
public:
    void begin(int adcPin);
    int readPercent();
    float readVoltage();
    bool isLow() const;      // 3.0V以下
    bool isCritical() const; // 2.7V以下

private:
    int adcPin_;
    float voltage_;
};
```

---

## 4. 移植元コード分析

### 4.1 is01.ino からの移植

| 機能 | 移植先 | 変更点 |
|------|--------|--------|
| HT-30計測 | SensorManager | クラス化 |
| BLEアドバタイズ | BleAdvertiser | クラス化 |
| ISMSv1ペイロード | BleAdvertiser | そのまま |
| DeepSleep | is01a.ino | 設定可能化 |
| 電池監視 | BatteryMonitor | クラス化 |

### 4.2 変更・簡略化するコード

- TID固定値 → NVSから設定可能に
- cluster1-6試行 → WiFi設定を汎用化
- ISMS専用設定 → 汎用名に変更

**削除禁止**:
- AraneaRegister関連 → **必須（CIC取得）**
- LacisIDGenerator関連 → **必須（lacisID生成）**
- ISMSv1ペイロード → **必須（BLE通信プロトコル）**

---

## 5. 開発優先順位

### P0: 必須（初期動作）
1. I2C初期化（直列処理）
2. HT-30計測
3. OLED表示
4. BLEアドバタイズ

### P1: 基本機能
5. LacisID生成【必須】
6. AraneaRegister連携【必須】
7. ISMSv1ペイロード構築
8. DeepSleep動作

### P2: 拡張機能
9. 電池残量監視
10. 設定永続化完全対応

---

## 6. テスト計画

### 6.1 単体テスト

| テスト項目 | 確認内容 |
|-----------|----------|
| I2C初期化 | HT-30/OLED両方認識 |
| HT-30計測 | 温湿度値取得 |
| BLEアドバタイズ | 3秒間送信確認 |
| ISMSv1ペイロード | 40バイト形式確認 |
| DeepSleep | 4分間隔で復帰 |

### 6.2 統合テスト

| テスト項目 | 確認内容 |
|-----------|----------|
| 初回アクティベーション | WiFi→AraneaReg→CIC取得 |
| BLE受信 | is02/is03で受信確認 |
| 長時間動作 | 24時間連続動作 |
| 電池寿命 | 想定寿命確認 |

---

## 7. ファイル一覧（予定）

```
is01a/
├── is01a.ino                    # メインスケッチ
├── AraneaGlobalImporter.h       # 共通モジュールインポート
├── SensorManager.h              # HT-30センサー
├── SensorManager.cpp
├── BleAdvertiser.h              # BLEアドバタイズ
├── BleAdvertiser.cpp
├── BatteryMonitor.h             # 電池監視
├── BatteryMonitor.cpp
└── design/
    ├── DESIGN.md                # 設計書
    └── MODULE_ADAPTATION_PLAN.md # 本ドキュメント
```

---

## 8. 注意事項

### 8.1 I2C処理の直列化

```cpp
// 正しい実装
void initDevices() {
    Wire.begin(SDA, SCL);
    delay(100);

    sensor.begin();  // HT-30
    delay(50);

    display.begin(); // OLED
    delay(50);
}

// 禁止: 並列処理
// 並列I2C初期化は失敗の原因
```

### 8.2 CIC未取得時の動作

```cpp
// CIC未取得でもBLEは必ず出す
String cicForPayload = myCic.isEmpty() ? "000000" : myCic;
bleAdv.setPayload(myLacisId, cicForPayload, temp, hum, batt);
bleAdv.advertise(3);  // 必ず実行
```

### 8.3 OTA非対応

- 電池駆動のためOTAは対応しない
- ファームウェア更新はUSB接続のみ
- HttpOtaManager/AraneaWebUIは使用しない

---

## 9. 参照

- **is01 (ISMS版)**: `archive_ISMS/ESP32/is01/is01.ino`
- **is02 (BLE受信側)**: `code/ESP32/is02a/`
- **is03 (受信・表示)**: `code/ESP32/is03/`
- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
