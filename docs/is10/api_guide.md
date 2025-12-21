# IS10 Router Inspector API ガイド

IS10 (Router Inspector) HTTP API完全ガイド

---

## 目次

1. [概要](#1-概要)
2. [認証](#2-認証)
3. [共通API](#3-共通api)
4. [IS10固有API](#4-is10固有api)
5. [OTA API](#5-ota-api)
6. [運用フロー](#6-運用フロー)
7. [エラーハンドリング](#7-エラーハンドリング)

---

## 1. 概要

### 1.1 アーキテクチャ

IS10はAraneaWebUI基底クラスを継承し、共通機能に加えてルーター監視固有の機能を提供します。

```
AraneaWebUI (基底クラス)
├── 共通タブ: Status / Network / Cloud / Tenant / System
├── 共通API: /api/status, /api/config, /api/common/*, /api/system/*
│
└── HttpManagerIs10 (派生クラス)
    ├── 固有タブ: Inspector / Routers
    └── 固有API: /api/is10/*
```

### 1.2 エンドポイント一覧

| カテゴリ | エンドポイント | メソッド | 概要 |
|---------|---------------|----------|------|
| 共通 | `/api/status` | GET | デバイスステータス取得 |
| 共通 | `/api/config` | GET | 全設定取得 |
| 共通 | `/api/common/network` | POST | ネットワーク設定保存 |
| 共通 | `/api/common/ap` | POST | APモード設定保存 |
| 共通 | `/api/common/cloud` | POST | クラウド設定保存 |
| 共通 | `/api/common/tenant` | POST | テナント設定保存 |
| 共通 | `/api/system/settings` | POST | システム設定保存 |
| 共通 | `/api/system/reboot` | POST | デバイス再起動 |
| 共通 | `/api/system/factory-reset` | POST | ファクトリリセット |
| IS10 | `/api/is10/inspector` | POST | Inspector設定保存 |
| IS10 | `/api/is10/router` | POST | ルーター追加/更新 |
| IS10 | `/api/is10/router/delete` | POST | ルーター削除（indexベース） |
| IS10 | `/api/is10/router/clear` | POST | 全ルータークリア |
| IS10 | `/api/is10/polling` | POST | ポーリング制御 |
| IS10 | `/api/is10/import` | POST | 設定一括インポート |
| OTA | `/api/ota/status` | GET | OTAステータス |
| OTA | `/api/ota/info` | GET | パーティション情報 |
| OTA | `/api/ota/update` | POST | ファームウェア更新 |

### 1.3 デフォルト設定

- **HTTPポート**: 80
- **SSHタイムアウト**: 30,000ms（ASUSWRT: 60,000ms以上推奨）
- **スキャン間隔**: 60秒
- **ルーター間インターバル**: 30,000ms

---

## 2. 認証

### 2.1 Basic認証

Web UIパスワードが設定されている場合、Basic認証が必要です。

```bash
curl -u admin:password http://192.168.x.x/api/status
```

- **Username**: `admin`（固定）
- **Password**: System設定の`uiPassword`

### 2.2 CICパラメータ認証

MQTT経由のコマンド用に、URLパラメータで認証可能です。

```bash
curl "http://192.168.x.x/api/status?cic=397815"
```

---

## 3. 共通API

### 3.1 GET /api/status

デバイスの現在状態を取得します。

#### リクエスト

```bash
curl http://192.168.x.x/api/status
```

#### レスポンス

```json
{
  "ok": true,
  "uiVersion": "1.0.0",
  "device": {
    "type": "ar-is10",
    "lacisId": "3010F8B3B7496DEC0001",
    "cic": "397815",
    "mac": "F8B3B7496DEC",
    "hostname": "ar-is10-496DEC"
  },
  "network": {
    "ip": "192.168.3.91",
    "ssid": "cluster1",
    "rssi": -52,
    "gateway": "192.168.3.1",
    "apMode": false
  },
  "system": {
    "ntpTime": "2025-12-21T15:04:27Z",
    "ntpSynced": true,
    "uptime": 120,
    "uptimeHuman": "2m 0s",
    "heap": 204328,
    "heapLargest": 90100,
    "cpuFreq": 240,
    "flashSize": 4194304
  },
  "firmware": {
    "version": "1.2.1",
    "uiVersion": "1.0.0",
    "buildDate": "Dec 21 2025 23:38:54",
    "modules": "WiFi,NTP,SSH,MQTT,HTTP"
  },
  "cloud": {
    "registered": true,
    "mqttConnected": false,
    "lastStateReport": "",
    "lastStateReportCode": 0
  },
  "typeSpecific": {
    "totalRouters": 16,
    "successfulPolls": 16,
    "lastPollTime": 1703170800000,
    "pollingEnabled": true,
    "pollingInProgress": false,
    "currentRouterIndex": 0,
    "mqttConnected": false,
    "lastStateReportTime": "2025-12-21T15:04:27Z",
    "lastStateReportCode": 200,
    "celestialConfigured": false,
    "sshTimeout": 30000,
    "scanIntervalSec": 60
  }
}
```

#### typeSpecificフィールド（IS10固有）

| フィールド | 型 | 説明 |
|-----------|-----|------|
| totalRouters | int | 登録ルーター数 |
| successfulPolls | int | 最新サイクルの成功数 |
| lastPollTime | long | 最終ポーリング時刻（ms） |
| pollingEnabled | bool | ポーリング有効フラグ |
| pollingInProgress | bool | ポーリング実行中フラグ |
| currentRouterIndex | int | 現在処理中のルーターインデックス |
| celestialConfigured | bool | CelestialGlobe設定済みフラグ |
| sshTimeout | int | SSHタイムアウト（ms） |
| scanIntervalSec | int | スキャン間隔（秒） |

### 3.2 GET /api/config

全設定を取得します。

#### レスポンス

```json
{
  "device": {
    "type": "ar-is10",
    "lacisId": "3010F8B3B7496DEC0001",
    "cic": "397815",
    "mac": "F8B3B7496DEC",
    "hostname": "ar-is10-496DEC",
    "version": "1.2.1",
    "uiVersion": "1.0.0"
  },
  "network": {
    "wifi": [
      {"ssid": "cluster1"},
      {"ssid": "cluster2"},
      {"ssid": ""},
      {"ssid": ""},
      {"ssid": ""},
      {"ssid": ""}
    ],
    "hostname": "ar-is10-496DEC",
    "ntpServer": "pool.ntp.org"
  },
  "ap": {
    "ssid": "ar-is10-496DEC",
    "password": "",
    "timeout": 300
  },
  "cloud": {
    "gateUrl": "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate",
    "stateReportUrl": "",
    "relayPrimary": "http://192.168.96.201:8080/api/events",
    "relaySecondary": "http://192.168.96.202:8080/api/events"
  },
  "tenant": {
    "tid": "T2025120621041161827",
    "fid": "",
    "lacisId": "18217487937895888001",
    "email": "user@example.com",
    "cic": "204965"
  },
  "system": {
    "deviceName": "ar-is10-496DEC",
    "uiPassword": "",
    "rebootEnabled": false,
    "rebootTime": "03:00"
  },
  "typeSpecific": {
    "inspector": {
      "endpoint": "",
      "celestialSecret": "********",
      "scanIntervalSec": 60,
      "reportClients": true,
      "sshTimeout": 30000,
      "retryCount": 2,
      "routerInterval": 30000
    },
    "routers": [
      {
        "rid": "101",
        "ipAddr": "192.168.125.171",
        "sshPort": 65305,
        "username": "HALE_admin",
        "enabled": true,
        "osType": 1,
        "hasPublicKey": false
      }
    ]
  }
}
```

### 3.3 POST /api/common/network

ネットワーク設定を保存します。

#### リクエスト

```bash
curl -X POST http://192.168.x.x/api/common/network \
  -H "Content-Type: application/json" \
  -d '{
    "wifi": [
      {"ssid": "network1", "password": "pass1"},
      {"ssid": "network2", "password": "pass2"},
      {"ssid": "", "password": ""},
      {"ssid": "", "password": ""},
      {"ssid": "", "password": ""},
      {"ssid": "", "password": ""}
    ],
    "hostname": "ar-is10-496DEC",
    "ntpServer": "pool.ntp.org"
  }'
```

#### レスポンス

```json
{"ok": true}
```

### 3.4 POST /api/common/ap

APモード設定を保存します。

#### リクエスト

```bash
curl -X POST http://192.168.x.x/api/common/ap \
  -H "Content-Type: application/json" \
  -d '{
    "ssid": "ar-is10-496DEC",
    "password": "ap_password",
    "timeout": 300
  }'
```

### 3.5 POST /api/system/reboot

デバイスを再起動します。

#### リクエスト

```bash
curl -X POST http://192.168.x.x/api/system/reboot \
  -H "Content-Type: application/json" \
  -d '{}'
```

#### レスポンス

```json
{"ok": true, "message": "Rebooting..."}
```

> **注意**: レスポンス送信後、約1秒後にデバイスが再起動します。

### 3.6 POST /api/system/factory-reset

ファクトリリセットを実行します。

**削除対象:**
- NVS `isms` namespace（WiFi設定、デバイス設定等）
- NVS `aranea` namespace（CIC、登録情報等）
- SPIFFS全ファイル（ルーター設定等）

#### リクエスト

```bash
curl -X POST http://192.168.x.x/api/system/factory-reset \
  -H "Content-Type: application/json" \
  -d '{}'
```

#### レスポンス

```json
{"ok": true, "message": "Factory reset complete. All settings and SPIFFS cleared."}
```

> **注意**: 実行後、デバイスは初期APモードで起動します。

---

## 4. IS10固有API

### 4.1 POST /api/is10/polling

ポーリングの開始/停止を制御します。

#### リクエスト

```bash
# ポーリング停止
curl -X POST http://192.168.x.x/api/is10/polling \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'

# ポーリング開始
curl -X POST http://192.168.x.x/api/is10/polling \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

#### レスポンス

```json
{
  "ok": true,
  "pollingEnabled": false
}
```

### 4.2 POST /api/is10/router/clear

全ルーター設定をクリアします。

#### リクエスト

```bash
curl -X POST http://192.168.x.x/api/is10/router/clear \
  -H "Content-Type: application/json" \
  -d '{}'
```

#### レスポンス

```json
{
  "ok": true,
  "message": "All routers cleared"
}
```

### 4.3 POST /api/is10/import

設定を一括インポートします。**既存のルーター設定は上書き（置換）されます。**

#### リクエスト

```bash
curl -X POST http://192.168.x.x/api/is10/import \
  -H "Content-Type: application/json" \
  -d '{
    "inspector": {
      "endpoint": "https://api.example.com/report",
      "celestialSecret": "secret-token",
      "scanIntervalSec": 60,
      "reportClients": true,
      "sshTimeout": 30000,
      "retryCount": 2,
      "routerInterval": 30000
    },
    "routers": [
      {
        "rid": "101",
        "ipAddr": "192.168.125.171",
        "sshPort": 65305,
        "username": "admin",
        "password": "password123",
        "enabled": true,
        "osType": 1
      },
      {
        "rid": "102",
        "ipAddr": "192.168.125.172",
        "sshPort": 22,
        "username": "root",
        "publicKey": "ssh-rsa AAAA...",
        "enabled": true,
        "osType": 0
      }
    ]
  }'
```

#### パラメータ詳細

**inspector セクション（オプション）:**

| フィールド | 型 | 必須 | デフォルト | 説明 |
|-----------|-----|------|-----------|------|
| endpoint | string | - | "" | CelestialGlobe報告先URL |
| celestialSecret | string | - | "" | X-Celestial-Secretヘッダ値 |
| scanIntervalSec | int | - | 60 | ポーリング間隔（秒） |
| reportClients | bool | - | true | クライアントリスト送信フラグ |
| sshTimeout | int | - | 30000 | SSHタイムアウト（ms） |
| retryCount | int | - | 2 | リトライ回数 |
| routerInterval | int | - | 30000 | ルーター間インターバル（ms） |

**routers セクション（オプション）:**

| フィールド | 型 | 必須 | デフォルト | 説明 |
|-----------|-----|------|-----------|------|
| rid | string | ○ | - | ルーターID（一意識別子） |
| ipAddr | string | ○ | - | IPアドレス |
| sshPort | int | - | 22 | SSHポート |
| username | string | ○ | - | SSHユーザー名 |
| password | string | - | "" | SSHパスワード |
| publicKey | string | - | "" | SSH公開鍵 |
| enabled | bool | - | true | 有効フラグ |
| osType | int | - | 0 | 0=OpenWrt, 1=ASUSWRT |

#### レスポンス

```json
{
  "ok": true,
  "importedSections": 2,
  "routerCount": 16
}
```

### 4.4 POST /api/is10/router

ルーターを追加または更新します。

**動作:**
- `rid`が既存ルーターと一致する場合 → 更新（上書き）
- `rid`が新規の場合 → 追加
- `index`を指定した場合 → 指定インデックスのルーターを更新

#### リクエスト

```bash
# 新規追加（ridが存在しない場合）
curl -X POST http://192.168.x.x/api/is10/router \
  -H "Content-Type: application/json" \
  -d '{
    "rid": "103",
    "ipAddr": "192.168.125.173",
    "sshPort": 65305,
    "username": "admin",
    "password": "password123",
    "enabled": true,
    "osType": 1
  }'

# 更新（同一ridで再送信）
curl -X POST http://192.168.x.x/api/is10/router \
  -H "Content-Type: application/json" \
  -d '{
    "rid": "103",
    "ipAddr": "192.168.125.199",
    "sshPort": 22,
    "username": "newadmin",
    "enabled": false,
    "osType": 0
  }'
```

| パラメータ | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| rid | string | ○ | ルーターID（一意識別子、更新時の検索キー） |
| ipAddr | string | ○ | IPアドレス |
| sshPort | int | - | SSHポート（デフォルト: 22） |
| username | string | ○ | SSHユーザー名 |
| password | string | - | SSHパスワード |
| publicKey | string | - | SSH公開鍵 |
| enabled | bool | - | 有効フラグ（デフォルト: true） |
| osType | int | - | 0=OpenWrt, 1=ASUSWRT |
| index | int | - | 更新対象インデックス（省略時はrid検索） |

#### レスポンス

```json
{"ok": true}
```

### 4.5 POST /api/is10/router/delete

ルーターをインデックス指定で削除します。

#### リクエスト

```bash
# インデックス16のルーターを削除
curl -X POST http://192.168.x.x/api/is10/router/delete \
  -H "Content-Type: application/json" \
  -d '{"index": 16}'
```

| パラメータ | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| index | int | ○ | 削除するルーターのインデックス（0始まり） |

#### レスポンス

```json
{"ok": true}
```

### 4.6 POST /api/is10/inspector

Inspector設定のみを保存します。

#### リクエスト

```bash
curl -X POST http://192.168.x.x/api/is10/inspector \
  -H "Content-Type: application/json" \
  -d '{
    "endpoint": "https://api.example.com/report",
    "celestialSecret": "secret-token",
    "scanIntervalSec": 120,
    "reportClients": true,
    "sshTimeout": 60000,
    "retryCount": 3,
    "routerInterval": 45000
  }'
```

#### レスポンス

```json
{"ok": true}
```

---

## 5. OTA API

### 5.1 GET /api/ota/status

OTAステータスを取得します。

#### レスポンス

```json
{
  "status": "idle",
  "progress": 0
}
```

| status値 | 説明 |
|----------|------|
| idle | 待機中 |
| in_progress | 更新中 |
| success | 成功 |
| error | エラー |

### 5.2 GET /api/ota/info

OTAパーティション情報を取得します。

#### レスポンス

```json
{
  "running": "ota_0",
  "nextUpdate": "ota_1",
  "rollbackAvailable": false
}
```

### 5.3 POST /api/ota/update

ファームウェアを更新します（multipart/form-data）。

```bash
curl -X POST http://192.168.x.x/api/ota/update \
  -F "firmware=@is10.ino.bin"
```

---

## 6. 運用フロー

### 6.1 新規デバイスのプロビジョニング

```bash
# 1. 疎通確認
curl http://192.168.x.x/api/status

# 2. 全設定をインポート
curl -X POST http://192.168.x.x/api/is10/import \
  -H "Content-Type: application/json" \
  -d @config.json

# 3. 設定反映確認
curl http://192.168.x.x/api/status | jq '.typeSpecific.totalRouters'

# 4. 必要に応じて再起動
curl -X POST http://192.168.x.x/api/system/reboot -d '{}'
```

### 6.2 ルーター設定の更新

```bash
# 現在の設定を取得
curl http://192.168.x.x/api/config > current_config.json

# 編集後、インポートで一括更新
curl -X POST http://192.168.x.x/api/is10/import \
  -H "Content-Type: application/json" \
  -d @updated_config.json
```

### 6.3 ポーリング制御

```bash
# メンテナンス時：ポーリング停止
curl -X POST http://192.168.x.x/api/is10/polling -d '{"enabled": false}'

# 作業完了後：ポーリング再開
curl -X POST http://192.168.x.x/api/is10/polling -d '{"enabled": true}'

# 状態確認
curl http://192.168.x.x/api/status | jq '.typeSpecific.pollingEnabled'
```

### 6.4 ファクトリリセット

```bash
# 全設定削除＋再起動
curl -X POST http://192.168.x.x/api/system/factory-reset -d '{}'

# デバイスはAPモードで起動
# SSID: ar-is10-XXXXXX に接続して再設定
```

---

## 7. エラーハンドリング

### 7.1 共通エラーレスポンス

```json
{
  "ok": false,
  "error": "Error message"
}
```

### 7.2 HTTPステータスコード

| コード | 説明 |
|--------|------|
| 200 | 成功 |
| 400 | リクエスト不正（JSONパースエラー等） |
| 401 | 認証失敗 |
| 404 | エンドポイント不明 |
| 500 | サーバーエラー |

### 7.3 よくあるエラー

| エラー | 原因 | 対処 |
|--------|------|------|
| "No body" | POSTボディなし | `-d '{}'`を追加 |
| "Invalid JSON" | JSONパースエラー | JSON形式を確認 |
| "Not Found" | 存在しないAPI | エンドポイント確認 |
| "Unauthorized" | 認証失敗 | Basic認証またはcicパラメータ |
| "rid is required" | ルーター追加時にridが空 | ridフィールドを指定 |
| "ipAddr is required" | ルーター追加時にipAddrが空 | ipAddrフィールドを指定 |
| "Invalid ipAddr: must be valid IPv4" | IPアドレス形式不正 | 正しいIPv4形式を指定 |
| "Invalid index" | 削除時のインデックスが範囲外 | 有効なインデックスを指定 |
| "Max routers reached" | ルーター数が上限(32)に到達 | 不要なルーターを削除 |
| "Invalid tid" | TIDの形式不正 | T + 19桁数字（計20文字）|
| "Invalid fid" | FIDの形式不正 | 4桁の数字 |
| "Invalid lacisId" | LacisIDの形式不正 | 20桁の数字 |

### 7.4 バリデーションルール

#### テナント設定 (/api/common/tenant)

| フィールド | ルール | 例 |
|-----------|--------|-----|
| tid | `T` + 数字19桁（計20文字） | `T2025120608261484221` |
| fid | 数字4桁 | `9000` |
| lacisId | 数字20桁 | `12767487939173857894` |

#### ルーター設定 (/api/is10/router)

| フィールド | ルール | NG例 |
|-----------|--------|------|
| ipAddr | 有効なIPv4アドレス | `999.999.999.999` (範囲外) |
|  | 4オクテット必須 | `192.168.10` (3オクテット) |
|  | CIDR表記不可 | `192.168.0.0/16` |
|  | 連続ドット不可 | `192.168.96..` |
|  | 先頭ゼロ不可 | `192.168.01.1` |
|  | 英字/記号不可 | `山田.AAA.25.@@@` |

> **注意**: 空文字列はバリデーションをスキップし、保存されません（任意フィールド扱い）

---

## 8. 注意事項

### セキュリティ

- `celestialSecret`は取得時に`********`でマスクされます
- `password`フィールドは取得時に空文字でマスクされます
- 本番環境では必ず`uiPassword`を設定してください

### パフォーマンス

- `/api/is10/import`は大量データ処理のため16KBのJSONバッファを使用
- SSHタイムアウトはASUSWRTルーター向けに60秒以上を推奨
- ポーリング中のAPI呼び出しは正常に処理されます

### 制限事項

- 最大ルーター数: 32台（MAX_ROUTERS定数）
- WiFi設定: 最大6件
- ホスト名: 最大31文字
