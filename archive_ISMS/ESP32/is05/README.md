# is05 (ESP32 DevKitC) - 8ch Reed Switch Sensor

8チャンネルリードスイッチ入力センサー。窓/ドアの開閉状態を検出してis03へHTTP送信。

## 機能

- 8チャンネルリードスイッチ入力（オプトカプラ経由）
- デバウンス処理（20ms x 3回安定で確定）
- 状態変化時の即時HTTP送信
- 60秒間隔の心拍送信
- Web UI設定（チャンネル名、meaning、DID）
- OTA更新対応
- OLED表示（128x64）

## 仕様

| 項目 | 値 |
|------|-----|
| productType | "005" |
| productCode | "0096" |
| lacisId例 | 30056CC8408CB47C0096 |
| ファームウェアバージョン | 1.0.0 |
| パーティション | min_spiffs |
| 心拍間隔 | 60秒 |
| デバウンス | 20ms x 3回 |

## ピン配置

| チャンネル | GPIO | 備考 |
|-----------|------|------|
| ch1 (ReedSW-1) | GPIO19 | |
| ch2 (ReedSW-2) | GPIO18 | |
| ch3 (ReedSW-3) | GPIO5 | ブートストラップ注意 |
| ch4 (ReedSW-4) | GPIO17 | |
| ch5 (ReedSW-5) | GPIO16 | |
| ch6 (ReedSW-6) | GPIO4 | ブートストラップ注意 |
| ch7 (ReedSW-7) | GPIO2 | ブートストラップ注意 |
| ch8 (ReedSW-8) | GPIO15 | ブートストラップ注意 |

### その他のピン

| GPIO | 機能 | 備考 |
|------|------|------|
| GPIO25 | WiFi再接続ボタン | 3秒長押し |
| GPIO26 | ファクトリーリセットボタン | 3秒長押し |
| GPIO21 | I2C SDA | OLED |
| GPIO22 | I2C SCL | OLED |
| GPIO5 | I2C電源制御 | MOSFET経由 |

## 入力仕様

- 入力モード: INPUT_PULLUP
- activeLow: LOW=アクティブ（ON）、HIGH=非アクティブ（OFF）
- オプトカプラのオープンコレクタ出力を想定

### meaning設定

- `meaning="open"`: アクティブ時に"open"を送信
- `meaning="close"`: アクティブ時に"close"を送信

非アクティブ時はmeaningの逆を送信。

## NVS設定キー

各チャンネル（N=1〜8）:

| キー | 型 | 説明 | デフォルト |
|------|-----|------|---------|
| is05_ch{N}_pin | int | GPIOピン番号 | 19,18,5,17,16,4,2,15 |
| is05_ch{N}_name | string | チャンネル名 | "ch1"〜"ch8" |
| is05_ch{N}_meaning | string | "open" or "close" | "open" |
| is05_ch{N}_did | string | 8桁デバイスID | "00000000" |

## 送信JSONフォーマット

```json
{
  "observedAt": "2025-12-14T12:00:00Z",
  "sensor": {
    "lacisId": "30056CC8408CB47C0096",
    "mac": "6CC8408CB47C",
    "productType": "005",
    "productCode": "0096"
  },
  "state": {
    "ch1": "open",
    "ch1_lastUpdatedAt": "2025-12-14T11:55:00Z",
    "ch1_did": "00001234",
    "ch1_name": "Door1",
    ...
    "ch8": "close",
    "ch8_lastUpdatedAt": "2025-12-14T11:50:00Z",
    "ch8_did": "00005678",
    "ch8_name": "Window8",
    "rssi": "-48",
    "ipaddr": "192.168.96.50",
    "SSID": "cluster1"
  },
  "meta": {
    "observedAt": "2025-12-14T12:00:00Z",
    "direct": true
  },
  "gateway": {
    "lacisId": "30056CC8408CB47C0096",
    "ip": "192.168.96.50",
    "rssi": -48
  }
}
```

## Web UI

| URL | 説明 |
|-----|------|
| / | ログイン / メインページ |
| /settings | リレー設定、再起動時間等 |
| /wifi | WiFi設定（cluster1-6） |
| /tenant | テナント認証設定 |
| /is05 | チャンネル設定（pin/name/meaning/did） |
| /status | JSON API |

初期パスワード: `0000`

## OLED表示

```
192.168.96.50 -55dBm
CIC: 263238
Door1=open
OK | HB:42
```

- Line1: IP + RSSI
- Line2: CIC
- Line3: 最後に変化したチャンネル
- Line4: 送信結果 + 心拍カウント

## ビルド方法

### Arduino CLI（シリアル経由）

```bash
# ビルド
arduino-cli compile \
  --fqbn esp32:esp32:esp32:PartitionScheme=min_spiffs \
  --libraries code/ESP32/global \
  code/ESP32/is05

# 書き込み
arduino-cli upload \
  --fqbn esp32:esp32:esp32:PartitionScheme=min_spiffs \
  --port /dev/cu.usbserial-XXXX \
  code/ESP32/is05
```

### OTA更新

```bash
# mDNSホスト名: is05-XXXX.local (XXXXはMAC下位4文字)
arduino-cli upload \
  --fqbn esp32:esp32:esp32:PartitionScheme=min_spiffs \
  --port is05-XXXX.local \
  code/ESP32/is05
```

## 動作シーケンス

1. 起動 → WiFi接続
2. NTP時刻同期
3. AraneaGateway登録（CIC取得）
4. GPIO入力監視開始
5. 状態変化検出 → 即時HTTP送信
6. 60秒毎の心拍送信
7. Web UIでリモート設定変更

## ボタン操作

- **WiFi再接続（GPIO25）**: 3秒長押しでWiFi再接続
- **ファクトリーリセット（GPIO26）**: 3秒長押しでNVSクリア＆再起動

## トラブルシューティング

### 入力が検出されない

- GPIOピン番号が正しいか確認（/is05ページ）
- オプトカプラの配線確認
- INPUT_PULLUPで動作しない場合は外部プルアップ追加

### 状態が逆になる

- meaning設定を逆に変更（open ↔ close）

### 送信が失敗する

- WiFi接続確認
- relay_pri/relay_sec URLの確認
- is03が起動しているか確認

### ブート時にハングする

- GPIO2/4/5/15はブートストラップピン
- 起動時にLOW固定されていないか確認
- 周辺回路がハイインピーダンスになっているか確認
