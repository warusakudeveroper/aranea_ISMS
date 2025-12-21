# !!!!! 必読 !!!!! IS10 ルーター監視デバイス 構成マニュアル

## IS10 概要

IS10は、施設内のASUSWRTルーターをSSHで巡回監視し、接続クライアント情報をクラウドに報告するデバイスです。

### 主な機能

- **SSH巡回**: 複数ルーターに順次SSH接続し、接続クライアント一覧を取得
- **MQTT双方向通信**: クラウドからの設定変更をリアルタイム受信
- **deviceStateReport**: 定期的な状態レポート送信
- **CelestialGlobe SSOT**: ルーター情報のクラウド同期
- **HTTP OTA**: ファームウェアのリモート更新

## ビルド設定

### 必須パーティションスキーム

```
esp32:esp32:esp32:PartitionScheme=min_spiffs
```

**絶対禁止**: `huge_app` パーティションは使用しないこと
- 理由: OTA領域がなくなり、リモートでのファームウェア更新が不可能になる

### ビルドコマンド例

```bash
# MCP経由でコンパイル
mcp__mcp-arduino-esp32__compile --sketch_path code/ESP32/is10/is10.ino

# アップロード
mcp__mcp-arduino-esp32__upload --sketch_path code/ESP32/is10/is10.ino --port /dev/cu.usbserial-*

# OTA更新
mcp__mcp-arduino-esp32__ota_update --device_ip 192.168.x.x --firmware_path builds/is10.ino.bin
```

## 施設展開（大量生産）手順

### 1. AraneaSettings.h の編集

`code/ESP32/is10/AraneaSettings.h` を開き、以下を施設用に変更:

```cpp
// 施設固有の値に変更
#define ARANEA_DEFAULT_TID "T2025xxxxxxxxxxxxxx"  // テナントID
#define ARANEA_DEFAULT_FID "0001"                 // 施設ID

// テナントPrimary認証情報
#define ARANEA_DEFAULT_TENANT_LACISID "xxxxxxxxxxxxx"
#define ARANEA_DEFAULT_TENANT_EMAIL "xxx@example.com"
#define ARANEA_DEFAULT_TENANT_CIC "xxxxxx"
```

### 2. ビルド

```bash
mcp__mcp-arduino-esp32__compile --sketch_path code/ESP32/is10/is10.ino
```

### 3. 複数台に書き込み

```bash
# 各デバイスに順次書き込み
mcp__mcp-arduino-esp32__upload --sketch_path code/ESP32/is10/is10.ino --port /dev/cu.usbserial-xxxx
```

### 4. 動作確認

各デバイスが:
1. WiFi接続（cluster1-6を自動試行）
2. araneaDeviceGate に登録（CIC取得）
3. MQTT接続
4. ルーター巡回開始

## ルーター設定

### Web UIから設定

1. ブラウザで `http://<デバイスIP>/` にアクセス
2. 「ルーター設定」タブを選択
3. ルーター情報を入力:
   - RID: ルーター識別子
   - IPアドレス: 192.168.x.x
   - SSHポート: 通常 65305
   - ユーザー名/パスワード
   - OS種別: 1 = ASUSWRT

### MQTTから設定（推奨）

mobes2.0管理画面からルーター設定を送信すると、デバイスが自動適用。

設定JSON例:
```json
{
  "routers": [
    {
      "rid": "101",
      "ipAddr": "192.168.125.171",
      "sshPort": 65305,
      "username": "admin",
      "password": "****",
      "enabled": true,
      "osType": 1
    }
  ]
}
```

## トラブルシューティング

### ルーターに接続できない

1. **PINGで確認しない**: ルーターはWAN側からPINGに応答しない（正常動作）
2. **SSH接続で確認**: 接続成否はSSH結果でのみ判断
3. **タイムアウト**: ASUSWRTは反応が遅い。タイムアウトは60秒以上推奨

### OTA更新が失敗する

1. **パーティション確認**: `huge_app`を使っていないか確認
2. **空き容量**: SPIFFSの空き容量を確認
3. **ネットワーク**: デバイスとPCが同一ネットワーク上にあるか確認

### WiFi接続失敗

1. **SSID確認**: cluster1-6のいずれかが届いているか
2. **パスワード**: `isms12345@` がデフォルト
3. **APモード**: GPIO0長押しでAPモード起動、http://192.168.4.1 から設定

## API エンドポイント

| パス | 説明 |
|------|------|
| `/` | メインページ（設定UI） |
| `/api/status` | デバイス状態JSON |
| `/api/config` | 設定取得/更新 |
| `/api/routers` | ルーター設定 |
| `/api/ota/update` | OTAファームウェア更新 |
| `/api/ota/status` | OTA状態 |
| `/api/ota/info` | OTAパーティション情報 |

## ファイル構成

```
code/ESP32/is10/
├── is10.ino                 - メインスケッチ
├── AraneaGlobalImporter.h   - 共通モジュールインポート
├── AraneaSettings.h/cpp     - 施設設定（★編集対象）
├── HttpManagerIs10.h/cpp    - Web UI実装
├── RouterTypes.h            - ルーター設定構造体
├── SshPollerIs10.h/cpp      - SSH巡回
├── StateReporterIs10.h/cpp  - 状態レポート
├── CelestialSenderIs10.h/cpp - CelestialGlobe送信
└── MqttConfigHandler.h/cpp  - MQTT設定処理
```

## 参照

- `code/ESP32/______MUST_READ_ROLE_DIVISION______.md` - モジュール役割分担
- `code/ESP32/is10/AraneaGlobalImporter.h` - 共通モジュール一覧
- `docs/common/` - 共通ドキュメント
