# is02 Specification

## 概要

is02はBLEゲートウェイデバイス。is01のBLEアドバタイズを受信し、ローカルis03（Zero3）へHTTP中継する。

## 基本仕様

| 項目 | 値 |
|------|-----|
| productType | "002" |
| productCode | "0096" |
| lacisId例 | 3002AABBCCDDEEFF0096 |
| 電源 | 常時電源（ACアダプタ） |
| OTA | 可能 |

## ハードウェア構成

| コンポーネント | 型番/仕様 | GPIO |
|--------------|----------|------|
| MCU | ESP32 DevKitC | - |
| OLED | SSD1306 128x64 I2C | SDA:21, SCL:22 |
| ボタン1 | タクトSW（WiFi再接続） | GPIO25 INPUT_PULLUP |
| ボタン2 | タクトSW（リブート） | GPIO26 INPUT_PULLUP |

## 起動シーケンス

```
1. Serial.begin(115200)
2. SettingManager.begin() → デフォルト設定投入
3. DisplayManager.begin() → OLED初期化、"Booting..." 表示
4. WiFiManager.connectDefault() → cluster1-6 に順次接続試行
   - 失敗時: OLED に "WiFi connecting..." 表示し続ける
   - 成功時: IP表示して次へ
5. NtpManager.sync() → NTP同期（失敗しても続行）
6. LacisIDGenerator.generate("002", "0096") → 自機lacisId生成
7. BLEスキャン開始（NimBLE）
8. loop() へ移行
```

## メインループ動作

```cpp
void loop() {
  // 1. BLE受信イベントをキューから取得
  if (eventQueue.available()) {
    SensorEvent evt = eventQueue.pop();

    // 2. 送信レート制限（同一lacisIdに対して最短3秒間隔）
    if (canSend(evt.sensorLacisId)) {
      // 3. HTTP POST（primary → secondary フォールバック）
      bool ok = HttpRelayClient.postToRelay(evt);
      updateLastSendTime(evt.sensorLacisId);
      displaySendResult(ok);
    }
  }

  // 4. OLED表示更新（1秒ごと）
  if (millis() - lastDisplayUpdate > 1000) {
    updateDisplay();
    lastDisplayUpdate = millis();
  }

  // 5. ボタン処理
  handleButtons();
}
```

## BLE受信処理（NimBLE callback）

```cpp
// AdvertisedDeviceCallbacks::onResult() 内
void onResult(NimBLEAdvertisedDevice* dev) {
  // 1. Manufacturer Data 取得
  if (!dev->haveManufacturerData()) return;
  std::string mfgData = dev->getManufacturerData();

  // 2. ISMSv1 判定
  if (!BleIsmsParser::isIsmsV1(mfgData)) return;

  // 3. パース
  IsmsPayload payload;
  if (!BleIsmsParser::parse(mfgData, &payload)) return;

  // 4. is01の lacisId 再構成
  String sensorLacisId = LacisIDGenerator::reconstructFromMac(
    payload.productType, payload.staMac);

  // 5. イベントをキューに追加（HTTPはloop側で実行）
  SensorEvent evt = {
    .sensorLacisId = sensorLacisId,
    .staMac12Hex = payload.staMac12Hex,
    .tempC = payload.tempC,
    .humPct = payload.humPct,
    .battPct = payload.battPct,
    .rssi = dev->getRSSI(),
    .rawMfgHex = payload.rawHex,
    .receivedAt = millis()
  };
  eventQueue.push(evt);  // キュー満杯時は古いものを破棄
}
```

**重要**: BLE callback 内で HTTP 通信を行わないこと（クラッシュの原因）

## HTTP中継先（デフォルト）

| 優先度 | URL |
|--------|-----|
| primary | http://192.168.96.201:8080/api/ingest |
| secondary | http://192.168.96.202:8080/api/ingest |

## 送信JSON形式（確定）

```json
{
  "gateway": {
    "lacisId": "3002AABBCCDDEEFF0096",
    "ip": "192.168.96.100",
    "rssi": -55
  },
  "sensor": {
    "lacisId": "3001112233445566770096",
    "mac": "112233445566",
    "productType": "001",
    "productCode": "0096"
  },
  "observedAt": "2025-01-15T10:30:00Z",
  "state": {
    "temperatureC": 25.34,
    "humidityPct": 65.20,
    "batteryPct": 80
  },
  "raw": {
    "mfgHex": "4953010107112233445566000000000E6097019500"
  }
}
```

## 送信レート制限

- 同一センサー（lacisId）に対して **最短3秒間隔** で送信
- 異なるセンサーは即座に送信可能
- is01の送信周期（3分）より十分短いため、データロスは発生しない

## OLED表示内容

```
┌──────────────────────┐
│ 192.168.96.100       │  ← 自機IP
│ ID:...EEFF0096       │  ← 自機lacisId末尾
│ Last:...5566 -65dBm  │  ← 最終受信センサー + RSSI
│ 25.3C 65% SEND:OK    │  ← 温湿度 + 送信結果
└──────────────────────┘
```

## エラーハンドリング

| 状況 | 動作 |
|------|------|
| WiFi切断 | 自動再接続試行、OLED に "WiFi..." 表示 |
| HTTP送信失敗（primary） | secondary へフォールバック |
| HTTP送信失敗（両方） | OLED に "SEND:NG" 表示、次回BLE受信時に再試行 |
| BLEスキャン停止 | 自動再開（NimBLE設定） |
| キュー満杯 | 古いイベントを破棄して新しいものを追加 |

## WiFi設定（ハードコード）

```cpp
const char* SSID_LIST[] = {
  "cluster1", "cluster2", "cluster3",
  "cluster4", "cluster5", "cluster6"
};
const char* WIFI_PASS = "isms12345@";
```

接続順: cluster1 → cluster2 → ... → cluster6 → cluster1（ループ）

## NVS保存キー

| キー | デフォルト値 | 説明 |
|------|-------------|------|
| relay.primary | http://192.168.96.201:8080/api/ingest | 中継先URL |
| relay.secondary | http://192.168.96.202:8080/api/ingest | フォールバックURL |
| hostname | isms-is02-XXXX | ホスト名（XXXX=MAC末尾4桁） |

## 依存ライブラリ

| ライブラリ | 用途 |
|-----------|------|
| NimBLE-Arduino | BLEスキャン（低メモリ） |
| ArduinoJson | JSON生成 |
| Adafruit_SSD1306 | OLED表示 |
| Adafruit_GFX | グラフィックス基盤 |
| HTTPClient | HTTP POST（ESP32標準） |
| Preferences | NVS保存（ESP32標準） |
