# is01 (ESP32 DevKitC) - Battery-powered BLE Sensor

電池駆動の温湿度センサー。BLEアドバタイズで送信のみ、DeepSleep運用。

## 機能

- HT-30温湿度センサー（SHT31互換）
- BLEアドバタイズ（ISMSv1ペイロード）
- DeepSleep（4分間隔）
- GPIO5によるI2C電源制御（MOSFET経由、バッテリー節約）
- OLED表示（128x64、20秒間維持）
- 初回アクティベーション（WiFi→AraneaGateway→CIC取得）

## 仕様

| 項目 | 値 |
|------|-----|
| productType | "001" |
| productCode | "0001" |
| lacisId例 | 3001AABBCCDDEEFF0001 |
| ファームウェアバージョン | 1.0.0 |
| パーティション | min_spiffs |
| DeepSleep間隔 | 4分（240秒） |
| ディスプレイ表示時間 | 20秒 |
| BLEアドバタイズ時間 | 5秒 |

## 動作シーケンス

1. DeepSleep復帰 / 初回起動
2. GPIO5 HIGH（I2C電源ON）
3. I2Cディスプレイ初期化（同期的に）
4. HT-30センサーから温湿度取得
5. ディスプレイ表示（20秒間維持）
6. BLEアドバタイズ送信（5秒間）
7. GPIO5 LOW（I2C電源OFF）
8. DeepSleep（4分間）

## アクティベーション（初回のみ）

初回起動時にWiFi接続してAraneaGatewayにデバイス登録、CICを取得。

1. WiFi接続（cluster1-6を順番に試行）
2. AraneaGateway登録リクエスト
3. CIC取得・NVS保存
4. WiFi切断

アクティベーション完了後はWiFiを使用しない（バッテリー節約）。

## ISMSv1 BLEペイロード（20バイト）

| オフセット | サイズ | 内容 |
|-----------|-------|------|
| 0 | 1 | Magic 'I' (0x49) |
| 1 | 1 | Magic 'S' (0x53) |
| 2 | 1 | Version (0x01) |
| 3 | 1 | productType |
| 4 | 1 | flags |
| 5-10 | 6 | STA MAC |
| 11-14 | 4 | bootCount (LE) |
| 15-16 | 2 | temp×100 (LE, int16) |
| 17-18 | 2 | hum×100 (LE, uint16) |
| 19 | 1 | battery % |

## 依存ライブラリ

- NimBLE-Arduino (2.x)
- SHT31
- Wire
- Preferences
- AraneaGlobal（プロジェクト共通ライブラリ）

## ビルド済みファイル

`build/` ディレクトリにビルド済みバイナリを格納:

| ファイル | 説明 |
|---------|------|
| is01.ino.bin | メインファームウェア |
| is01.ino.bootloader.bin | ブートローダー |
| is01.ino.partitions.bin | パーティションテーブル |
| partitions.csv | パーティション定義 |

## インストール方法

### Arduino CLI（シリアル経由）

```bash
# ビルド
arduino-cli compile \
  --fqbn esp32:esp32:esp32:PartitionScheme=min_spiffs \
  .

# 書き込み（シリアルポート指定）
arduino-cli upload \
  --fqbn esp32:esp32:esp32:PartitionScheme=min_spiffs \
  --port /dev/cu.usbserial-XXXX \
  .
```

### ビルド済みバイナリからの書き込み（esptool使用）

```bash
esptool.py --chip esp32 --port /dev/cu.usbserial-XXXX \
  --baud 921600 \
  write_flash \
  0x1000 build/is01.ino.bootloader.bin \
  0x8000 build/is01.ino.partitions.bin \
  0x10000 build/is01.ino.bin
```

### 複数デバイスへの書き込み

```bash
# シリアルポートリスト
PORTS="/dev/cu.usbserial-1 /dev/cu.usbserial-2 /dev/cu.usbserial-3"

# 一括書き込み
for PORT in $PORTS; do
  echo "Writing to $PORT..."
  esptool.py --chip esp32 --port $PORT --baud 921600 \
    write_flash \
    0x1000 build/is01.ino.bootloader.bin \
    0x8000 build/is01.ino.partitions.bin \
    0x10000 build/is01.ino.bin
done
```

## OLED表示

```
┌──────────────────────┐
│     CIC: 147512      │  登録済みCIC（大）
│                      │
│   25.3C 65%          │  温湿度
│                      │
│ Boot: 42             │  起動回数
└──────────────────────┘
```

未アクティベート時はCIC部分に `------` を表示。

## NVS設定

| キー | 説明 |
|------|------|
| activated | アクティベーション状態 (0/1) |
| cic | 取得したCICコード |
| wifi_ssid1-6 | WiFi SSID（アクティベーション用） |
| wifi_pass1-6 | WiFiパスワード |
| tid | テナントID |
| gate_url | araneaDeviceGate URL |
| tenant_lacisid | テナントプライマリLacisID |
| tenant_email | テナントプライマリEmail |
| tenant_pass | テナントプライマリパスワード |
| tenant_cic | テナントプライマリCIC |

## 注意事項

- **OTA非対応**: is01は電池駆動のため、ファームウェア更新はシリアル経由のみ
- **I2C電源制御**: GPIO5でMOSFET制御。DeepSleep前に必ずLOWにすること
- **WiFi使用は初回のみ**: アクティベーション完了後はWiFiを使用しない
- **CIC未取得でもBLE広告**: アクティベーション失敗時もBLE広告は出力（is02/is03で検知可能）

## トラブルシューティング

### 書き込み時に「Failed to communicate with the flash chip」

- I2Cディスプレイなど周辺機器を外して書き込み
- BOOTボタンを押しながらリセット

### DeepSleepから復帰しない

- GPIO5の配線確認（MOSFET制御）
- 電源電圧確認（バッテリー残量）

### センサーが読み取れない

- I2C配線確認（SDA/SCL）
- HT-30のI2Cアドレス確認（0x44）
- GPIO5がHIGHになっているか確認

### アクティベーションが失敗する

- WiFi設定確認（cluster1-6）
- AraneaGateway URLの確認
- テナント認証情報の確認
