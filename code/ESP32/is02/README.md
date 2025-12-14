# is02 (ESP32 DevKitC) - BLE Gateway

BLEゲートウェイデバイス。is01のBLEアドバタイズを受信し、ローカルis03（Zero3）へHTTP中継する。

## 機能

- BLEスキャン（NimBLE）
- ISMSv1ペイロードのパース
- HTTP POST（primary/secondary フォールバック）
- OLED表示

## 仕様

| 項目 | 値 |
|------|-----|
| productType | "002" |
| productCode | "0096" |
| lacisId例 | 3002AABBCCDDEEFF0096 |

## 依存ライブラリ

- NimBLE-Arduino
- ArduinoJson
- Adafruit SSD1306
- Adafruit GFX Library

## ビルド

```bash
arduino-cli compile \
  --fqbn esp32:esp32:esp32 \
  --libraries ../global \
  .
```

## 書き込み

```bash
arduino-cli upload \
  --fqbn esp32:esp32:esp32 \
  --port /dev/cu.usbserial-XXXX \
  .
```

## 設定

NVSに保存されるデフォルト設定:

| キー | デフォルト値 |
|------|-------------|
| relay_pri | http://192.168.96.201:8080/api/ingest |
| relay_sec | http://192.168.96.202:8080/api/ingest |

## OLED表示

```
┌──────────────────────┐
│ 192.168.96.100       │  IP
│ ID:...EEFF0096       │  自機lacisId末尾
│ L:...5566 -65dBm     │  最終受信センサー
│ 25.3C 65% SEND:OK    │  温湿度 + 送信状態
└──────────────────────┘
```
