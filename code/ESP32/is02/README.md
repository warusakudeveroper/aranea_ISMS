# is02 (ESP32 DevKitC) - BLE Gateway

BLEゲートウェイデバイス。is01のBLEアドバタイズを受信し、ローカルis03（Zero3）へHTTP中継する。

## 機能

- BLEスキャン（NimBLE 2.x）でis01データ受信
- ISMSv1ペイロードのパース
- HTTP POST（primary/secondary フォールバック）
- HT-30温湿度センサー（自己計測）
- OLED表示（128x64）
- OTAアップデート（ArduinoOTA）
- Web UI（設定・ステータス・WiFi・テナント管理）
- 定期再起動スケジューラ
- WiFiリトライ上限での自動再起動
- ファクトリーリセット（パスワード保護）

## 仕様

| 項目 | 値 |
|------|-----|
| productType | "002" |
| productCode | "0096" |
| lacisId例 | 3002AABBCCDDEEFF0096 |
| ファームウェアバージョン | 1.0.0 |
| パーティション | min_spiffs (OTA対応) |

## 依存ライブラリ

- NimBLE-Arduino (2.x)
- ArduinoJson
- SHT31
- Wire
- AraneaGlobal（プロジェクト共通ライブラリ）

## ビルド済みファイル

`build/` ディレクトリにビルド済みバイナリを格納:

| ファイル | 説明 |
|---------|------|
| is02.ino.bin | メインファームウェア |
| is02.ino.bootloader.bin | ブートローダー |
| is02.ino.partitions.bin | パーティションテーブル |
| partitions.csv | パーティション定義 |

## インストール方法

### Arduino CLI（シリアル経由 - 初回書き込み）

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
  0x1000 build/is02.ino.bootloader.bin \
  0x8000 build/is02.ino.partitions.bin \
  0x10000 build/is02.ino.bin
```

### OTAアップデート（WiFi経由 - 2回目以降）

```bash
# espota.py を使用（IPアドレス指定）
python3 ~/.arduino15/packages/esp32/hardware/esp32/3.2.1/tools/espota.py \
  -i <DEVICE_IP> \
  -p 3232 \
  -f build/is02.ino.bin

# 例: IP 192.168.97.69 のデバイスに書き込み
python3 ~/.arduino15/packages/esp32/hardware/esp32/3.2.1/tools/espota.py \
  -i 192.168.97.69 \
  -p 3232 \
  -f build/is02.ino.bin
```

### 複数デバイスへの一括インストール

```bash
# デバイスIPリスト
DEVICES="192.168.97.69 192.168.97.70 192.168.97.71"

# 一括OTAアップデート
for IP in $DEVICES; do
  echo "Updating $IP..."
  python3 ~/.arduino15/packages/esp32/hardware/esp32/3.2.1/tools/espota.py \
    -i $IP -p 3232 -f build/is02.ino.bin
done
```

## Web UI

デバイスのIPアドレスにブラウザでアクセス（ポート80）。

- **ログイン**: 初期パスワード `0000`
- **メインページ**: CIC、温湿度、デバイス情報表示
- **/settings**: リレーエンドポイント、再起動スケジュール、WiFiリトライ上限、ログインパスワード変更
- **/wifi**: WiFi接続設定（6つのSSID/パスワードペア）
- **/tenant**: テナント情報設定（TID, LacisID, Email, CIC, Gate URL）
- **/status**: JSON形式のステータスAPI

### ファクトリーリセット

設定ページからパスワード `isms12345@` で実行可能。全設定がクリアされます。

## NVS設定

| キー | デフォルト値 | 説明 |
|------|-------------|------|
| relay_pri | http://192.168.96.201:8080/api/events | プライマリリレーURL |
| relay_sec | http://192.168.96.202:8080/api/events | セカンダリリレーURL |
| reboot_hour | -1 (無効) | 定期再起動時刻 (0-23) |
| wifi_retry_limit | 30 | WiFiリトライ上限 |
| http_pass | 0000 | Web UIログインパスワード |
| wifi_ssid1-6 | cluster1-6 | WiFi SSID |
| wifi_pass1-6 | isms12345@ | WiFiパスワード |
| tid | - | テナントID |
| gate_url | - | araneaDeviceGate URL |

## OLED表示

```
┌──────────────────────┐
│ 192.168.97.69 -65dBm │  IP + RSSI
│     CIC: 147512      │  登録済みCIC
│   25.3C 65%          │  温湿度
│                      │  送信中: ドット表示
└──────────────────────┘
```

## トラブルシューティング

### OTAアップデートが失敗する

1. デバイスがWiFiに接続されているか確認（pingで疎通確認）
2. ファイアウォールでUDP 3232ポートが開いているか確認
3. デバイスが他の処理中でないか確認（BLEスキャンは継続動作）

### Web UIにアクセスできない

1. デバイスのIPアドレスを確認（シリアルログまたはOLED表示）
2. 同じネットワークセグメントにいるか確認
3. ポート80がブロックされていないか確認

### WiFi接続が不安定

1. WiFiリトライ上限を調整（設定ページ）
2. 複数のSSIDを登録して冗長化
3. RSSIを確認し、電波の強い位置に設置
