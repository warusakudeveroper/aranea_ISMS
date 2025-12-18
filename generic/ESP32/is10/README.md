# ar-is10 - OpenWrt Router Inspector

OpenWrt/AsusWRT系ルーターにSSHアクセスして情報を収集し、クラウドエンドポイントへ送信するESP32デバイス。

## 概要

| 項目 | 値 |
|-----|-----|
| デバイスタイプ | `ar-is10` |
| モデル名 | `Openwrt_RouterInspector` |
| ProductType | `010` |
| ProductCode | `0001` |
| ファームウェアバージョン | `1.0.0` |
| 対象ボード | ESP32-DevKitC (4MB Flash) |
| 使用ライブラリ | AraneaGlobalGeneric |

## 機能一覧

### コア機能
- **SSH経由ルーター情報収集**: 最大20台のOpenWrt/AsusWRTルーターを監視
- **定期ポーリング**: 設定間隔でルーター情報を自動取得
- **クラウド送信**: 取得データをエンドポイントへJSON POST
- **Web管理UI**: ブラウザから全設定を変更可能

### 取得可能なルーター情報
| 情報 | 取得コマンド |
|-----|-------------|
| WAN IP | `ifconfig eth0` / `ifconfig wan` |
| LAN IP | `ifconfig br-lan` / `ifconfig lan` |
| 2.4GHz SSID | `uci show wireless \| grep ssid` |
| 5GHz SSID | `uci show wireless \| grep ssid` |
| 接続クライアント数 | `cat /tmp/dhcp.leases` |
| MACアドレス | `/sys/class/net/br-lan/address` |

### ネットワーク機能
- WiFi STA接続（6つのSSIDを順次試行）
- APモードフォールバック（接続失敗時）
- NTP時刻同期
- AraneaDeviceGate登録（CIC取得）

### OTA更新
- **ArduinoOTA**: mDNS経由（ポート3232）
- **HTTP OTA**: Web API経由（`/api/ota/*`）

## ハードウェア仕様

### ピン配置
| ピン | 機能 | 説明 |
|-----|------|------|
| GPIO21 | I2C SDA | OLEDディスプレイ用 |
| GPIO22 | I2C SCL | OLEDディスプレイ用 |
| GPIO25 | BTN_WIFI | WiFi再接続ボタン（3秒長押し） |
| GPIO26 | BTN_RESET | ファクトリーリセットボタン（3秒長押し） |

### ディスプレイ
- 128x64 OLED (SSD1306)
- I2Cアドレス: 0x3C

## タイミング設定

| パラメータ | デフォルト値 | 説明 |
|-----------|-------------|------|
| ROUTER_POLL_INTERVAL_MS | 30000 (30秒) | ルーターポーリング間隔 |
| SSH_TIMEOUT_MS | 30000 (30秒) | SSHタイムアウト |
| SSH_RETRY_COUNT | 2 | リトライ回数 |
| DISPLAY_UPDATE_MS | 1000 (1秒) | ディスプレイ更新間隔 |
| BUTTON_HOLD_MS | 3000 (3秒) | ボタン長押し判定時間 |
| AP_MODE_TIMEOUT_MS | 300000 (5分) | APモードタイムアウト |

## 設定ストレージ

### NVS (Non-Volatile Storage)
名前空間: `isms`

| キー | 型 | 説明 |
|-----|-----|------|
| `is10_endpoint` | String | レポート送信先URL |
| `is10_ssh_timeout` | int | SSHタイムアウト(ms) |
| `is10_retry_count` | int | リトライ回数 |
| `is10_router_interval` | int | ポーリング間隔(ms) |
| `is10_lacis_prefix` | String | ルーターLacisIDプレフィックス |
| `is10_router_ptype` | String | ルーターProductType |
| `is10_router_pcode` | String | ルーターProductCode |
| `gate_url` | String | AraneaDeviceGate URL |
| `cloud_url` | String | Cloud Report URL |
| `relay_pri` | String | Relay Primary URL |
| `relay_sec` | String | Relay Secondary URL |
| `tid` | String | テナントID |
| `fid` | String | 施設ID |
| `tenant_lacisid` | String | テナントプライマリLacisID |
| `tenant_email` | String | テナントEmail (userId) |
| `tenant_cic` | String | テナントCIC |
| `wifi_ssid1`〜`wifi_ssid6` | String | WiFi SSID（6個） |
| `wifi_pass1`〜`wifi_pass6` | String | WiFiパスワード（6個） |

### SPIFFS ファイル
| ファイル | 内容 |
|---------|------|
| `/routers.json` | ルーター設定（最大20台） |
| `/aranea_settings.json` | AraneaSettings設定 |

## Web管理UI

アクセス: `http://<device-ip>/`

### タブ構成

#### 1. Global Settings
- Report Endpoint URL（POST先）
- SSH Timeout / Retry Count / Router Poll Interval
- AraneaDeviceGate URL
- Cloud Report URL
- Relay Primary/Secondary

#### 2. Routers
ルーター設定（最大20台）
- RID (Resource ID)
- IPアドレス / SSHポート
- ユーザー名 / パスワード
- 公開鍵（オプション）
- 有効/無効フラグ

#### 3. LacisID Gen
ルーターのLacisID生成設定
- Prefix（デフォルト: 4）
- ProductType
- ProductCode
- Generator

#### 4. Tenant
テナント認証情報（3要素認証）
- Tenant ID (TID)
- Facility ID (FID)
- Primary LacisID
- Email (userId)
- CIC

#### 5. WiFi
6つのSSID/パスワードペア

#### 6. System
- Reboot
- Factory Reset

## HTTP API

### 設定取得
```
GET /api/config
```

### 設定保存
```
POST /api/global      # グローバル設定
POST /api/lacisgen    # LacisID生成設定
POST /api/router      # ルーター追加/編集
POST /api/router/delete  # ルーター削除
POST /api/tenant      # テナント設定
POST /api/wifi        # WiFi設定
```

### システム操作
```
POST /api/reboot         # 再起動
POST /api/factory-reset  # ファクトリーリセット
```

### OTA API
```
GET  /api/ota/status   # OTAステータス
GET  /api/ota/info     # パーティション情報
POST /api/ota/update   # ファームウェアアップロード
POST /api/ota/rollback # ロールバック
POST /api/ota/confirm  # ファームウェア確認
```

## データ送信フォーマット

### ルーター情報POST
```json
{
  "auth": {
    "tid": "T2025...",
    "lacisId": "30100...",
    "cic": "123456"
  },
  "report": {
    "observedAt": "2025-12-18T10:30:00Z",
    "sourceDevice": "30100...",
    "sourceType": "ar-is10",
    "router": {
      "rid": "R001",
      "lacisId": "40110...",
      "wanIp": "203.0.113.1",
      "lanIp": "192.168.1.1",
      "ssid24": "MyNetwork_2G",
      "ssid50": "MyNetwork_5G",
      "clientCount": 15,
      "online": true
    }
  }
}
```

## 認証方式

### AraneaDeviceGate認証（3要素）
パスワード認証は廃止され、以下の3要素で認証:
1. `lacisId` - テナントプライマリのLacisID
2. `userId` - テナントEmail
3. `cic` - CIC (Cloud Identification Code)

## 起動シーケンス

```
1. GPIO初期化
2. SPIFFS初期化
3. AraneaSettings初期化（SPIFFS設定ロード）
4. SettingManager初期化（NVS）
5. AraneaSettings::initDefaults()（NVSデフォルト投入）
6. DisplayManager初期化
7. LacisID生成
8. WiFi接続試行（失敗時APモード）
9. NTP同期
10. AraneaDeviceGate登録（CIC取得）
11. IS10設定読み込み
12. OtaManager初期化
13. HttpManager開始
14. HttpOtaManager開始
15. RUNモード開始
```

## ボタン操作

### WiFi再接続ボタン（GPIO25）
- 3秒長押し: WiFi切断→再接続

### ファクトリーリセットボタン（GPIO26）
- 3秒長押し: 全設定クリア→再起動
  - NVS `isms` 名前空間クリア
  - NVS `aranea` 名前空間クリア（CIC含む）
  - SPIFFSファイル削除

## ファイル構成

```
is10/
├── is10.ino              # メインスケッチ
├── AraneaSettings.h      # IS10専用設定（デフォルト値）
├── AraneaSettings.cpp    # IS10専用設定実装
├── AraneaRegister.h      # AraneaDeviceGate登録
├── AraneaRegister.cpp    # AraneaDeviceGate登録実装
├── HttpManagerIs10.h     # Web管理UI
├── HttpManagerIs10.cpp   # Web管理UI実装
└── README.md             # このファイル
```

## 依存ライブラリ

### AraneaGlobalGeneric（generic/ESP32/global）
- SettingManager - NVS設定管理
- DisplayManager - OLEDディスプレイ
- WiFiManager - WiFi接続管理
- NtpManager - NTP時刻同期
- LacisIDGenerator - LacisID生成
- OtaManager - ArduinoOTA
- HttpOtaManager - HTTP OTA
- Operator - 状態管理
- SshClient - SSH接続

### 外部ライブラリ
- ArduinoJson
- Adafruit SSD1306
- Adafruit GFX Library

## ビルド方法

```bash
# Arduino CLIでビルド
arduino-cli compile --fqbn esp32:esp32:esp32 is10.ino

# MCP経由でビルド
mcp__mcp-arduino-esp32__compile(sketch_path="is10.ino")
```

## 大量生産モード

`AraneaSettings.h` のデフォルト値を施設用に編集してビルド:

```cpp
#define ARANEA_DEFAULT_TID "T施設固有のTID"
#define ARANEA_DEFAULT_FID "施設固有のFID"
#define ARANEA_DEFAULT_TENANT_LACISID "テナントLacisID"
#define ARANEA_DEFAULT_TENANT_EMAIL "tenant@example.com"
#define ARANEA_DEFAULT_TENANT_CIC "123456"
```

## トラブルシューティング

### WiFi接続失敗
- APモード（`ar-is10-XXXXXX`）で起動
- ブラウザから設定変更

### SSH接続失敗
- ルーターのSSHポート確認
- ユーザー名/パスワード確認
- ファイアウォール設定確認

### CIC取得失敗
- AraneaDeviceGate URLを確認
- テナント認証情報を確認
- ネットワーク接続を確認

### OTA失敗
- 十分なフラッシュ空き容量を確認
- 同一ネットワーク上にいることを確認

## 変更履歴

### v1.0.0
- 初回リリース
- SSH経由ルーター情報収集
- Web管理UI
- HTTP OTA対応
