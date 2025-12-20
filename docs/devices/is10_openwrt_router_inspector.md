# IS10 - Openwrt Router Inspector (ar-is10)

## 概要

OpenWrt/AsusWRT系ルーターにSSHアクセスして情報収集し、エンドポイントへPOSTするデバイス。

## デバイス情報

| 項目 | 値 |
|------|-----|
| Type Domain | araneaDevice |
| Device Type | ar-is10 |
| Model Name | Openwrt_RouterInspector |
| Product Type | 010 |
| Product Code | 0001 |
| Hardware | ESP32-DevKitC (4MB) |
| FID | 0150 |

## テナント情報

| 項目 | 値 |
|------|-----|
| TID | T2025120621041161827 |
| テナントプライマリLacisID | 18217487937895888001 |
| テナントプライマリEmail | soejim@mijeos.com |

## 機能

### 基本機能
- WiFi接続（STA）/ APモードフォールバック
- AraneaGateway登録（CIC取得）
- HTTP設定UI
- OTA更新

### Router Inspector機能
- 複数ルーター（最大20台）へのSSH接続
- ルーター情報取得
- エンドポイントへのデータPOST

### 取得情報
- WAN側IP / LAN側IP
- SSID (2.4GHz / 5.0GHz / Mixed)
- 接続クライアント数
- クライアント詳細（ホスト名、MAC、帯域、接続時間）
- ベンダー情報、機種名

## 設定構造

### グローバル設定
```json
{
  "globals": {
    "endpoint": "{endpointURL}",
    "timeout": 30000,
    "retryCount": 2,
    "interval": 30000
  },
  "OpenwrtlacisIDgen": {
    "prefix": "4",
    "productType": "{ルーターのProductType}",
    "generator": "DeviceWithMac",
    "productCode": "{ルーターのProductCode}"
  }
}
```

### ルーター設定
```json
{
  "{fid}": {
    "{rid01}": {
      "ipAddr": "192.168.x.x",
      "publicKey": "ssh-rsa ...",
      "sshPort": 22,
      "username": "admin",
      "password": "****"
    }
  }
}
```

## ピン定義

| ピン | GPIO | 機能 |
|------|------|------|
| I2C_SDA | 21 | OLED SDA |
| I2C_SCL | 22 | OLED SCL |
| BTN_WIFI | 25 | WiFi再接続ボタン（3秒長押し） |
| BTN_RESET | 26 | ファクトリーリセットボタン（3秒長押し） |

## 動作モード

### STAモード（通常）
1. WiFi接続
2. AraneaGateway登録
3. NTP同期
4. ルーターポーリング開始

### APモード（設定用）
- WiFi接続失敗時に自動移行
- SSID: `ar-is10-{MAC下6桁}`
- パスワード: なし
- 5分タイムアウトでSTA再試行

## OpenWrt取得コマンド（参考）

```bash
# WAN IP
ifconfig eth0 | grep inet

# LAN IP
ifconfig br-lan | grep inet

# SSID
uci show wireless | grep ssid

# DHCPリース（接続デバイス）
cat /tmp/dhcp.leases

# WiFi接続クライアント
ubus call hostapd.wlan0 get_clients

# ネットワーク情報
route -n
```

## ディレクトリ構造

```
generic/ESP32/
├── global/
│   ├── src/           # 共通モジュール
│   └── library.properties
└── is10/
    ├── is10.ino       # メインスケッチ
    └── src -> ../global/src  # シンボリックリンク
```

## 依存関係

- AraneaGlobal共通モジュール
- ArduinoJson
- WiFi (ESP32組み込み)
- SPIFFS (ESP32組み込み)
- HTTPClient (ESP32組み込み)

## deviceStateReport

mobesへの状態報告で送信するJSONペイロード。

### 送信タイミング
- 起動後1回（初回登録）
- 60秒毎のheartbeat
- WebUIでdeviceName変更時（即時送信）

### state フィールド

| フィールド | 型 | 説明 |
|-----------|-----|------|
| deviceName | string | デバイス名（最大64文字）。WebUIで設定、未設定時はホスト名 |
| IPAddress | string | 現在のIPアドレス |
| MacAddress | string | MACアドレス（大文字12桁HEX） |
| RSSI | int | WiFi信号強度（dBm） |
| routerCount | int | 設定済みルーター数 |
| successCount | int | 成功したポーリング数 |
| failCount | int | 失敗したポーリング数 |
| lastPollResult | string | 最後のポーリング結果 |
| mqttConnected | bool | MQTT接続状態 |
| heap | int | 空きヒープメモリ（bytes） |

### deviceName と userObject.name 連携

mobes側では `state.deviceName` を参照して `userObject.name` を更新できます。

**運用ルール（mobes側）:**
- `userObject.name` が自動生成名（"ar-is10-XXXXXX"形式）の場合のみ、`state.deviceName` で上書き
- 管理者が手動で名前を設定した後は、デバイスからの `state.deviceName` では上書きしない
- これにより、現地デバイスとクラウド管理画面の双方向設定が安全に動作

### sanitize ルール

deviceNameは送信前に以下の正規化を行う:
1. trim（前後空白除去）
2. 制御文字（0x00-0x1F, 0x7F）除去
3. 連続空白を1つに圧縮
4. 最大64文字で切り捨て

## TODO

- [x] SSH クライアント実装（LibSSH-ESP32）
- [x] ルーター情報解析パーサー
- [x] HTTP設定UIの拡張
- [x] Celestial Globe連携
- [x] deviceStateReport deviceName連携

## 注意事項

- ESP32でのSSH実装はリソース制約が厳しい
- JSON解析は軽量に保つ（ESP32のメモリ制約）
- ルーター設定はSPIFFSに保存
