# is03-relay

Orange Pi Zero3用のBLEアドバタイズ受信・保存・表示アプリケーション

## 概要

is03-relayは、is01（BLEビーコン）からのアドバタイズを受信し、メモリとSQLiteに保存し、HTTP/WebSocketで表示するアプリケーションです。

## 機能

- BLEアドバタイズの常時スキャン・受信
- メモリリング（最新2000件保持）
- SQLiteへのバッチ書き込み（10秒/50件で自動flush）
- HTTP API（ステータス、イベント取得）
- WebSocket（リアルタイムイベント配信）→ **廃止（ブラウザ負荷軽減のためHTTPポーリングに統一）**
- ブラウザUI（イベント一覧表示）
- systemdによる自動起動・自動復旧

## 構成

```
/opt/is03-relay/
├── venv/                    # Python仮想環境
├── app/
│   ├── main.py             # FastAPI起動・lifespan管理
│   ├── db.py               # メモリリング・SQLite管理
│   ├── ble.py              # BLEスキャナー
│   ├── web.py              # HTTP/WebSocketルーター
│   └── static/
│       └── index.html      # ブラウザUI
/var/lib/is03-relay/
└── relay.sqlite            # SQLiteデータベース
/etc/is03-relay/
└── config.json             # デバイス設定
/etc/systemd/system/
└── is03-relay.service      # systemdサービス
```

## セットアップ

### 1. 必要なパッケージのインストール

```bash
sudo apt update
sudo apt -y install bluetooth bluez python3-venv python3-pip sqlite3 jq curl
sudo systemctl enable --now bluetooth
```

### 2. ディレクトリ作成

```bash
sudo mkdir -p /opt/is03-relay/app/static /var/lib/is03-relay /etc/is03-relay
sudo chown -R isms:isms /opt/is03-relay /var/lib/is03-relay
```

### 3. Python仮想環境とパッケージ

```bash
python3 -m venv /opt/is03-relay/venv
/opt/is03-relay/venv/bin/pip install --upgrade pip
/opt/is03-relay/venv/bin/pip install -r requirements.txt
```

### 4. アプリケーションファイルの配置

`app/` ディレクトリの内容を `/opt/is03-relay/app/` にコピーします。

### 5. systemdサービスの設定

```bash
sudo cp is03-relay.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now is03-relay
```

### 6. 動作確認

```bash
sudo systemctl status is03-relay
curl http://localhost:8080/api/status
```

ブラウザで `http://<デバイスIP>:8080` にアクセス

## API仕様

### GET /api/status

システムステータスを取得

**レスポンス例:**
```json
{
  "ok": true,
  "hostname": "is03-1",
  "uptimeSec": 3745,
  "bluetooth": "ok",
  "db": "ok",
  "ringSize": 2000,
  "queuedWrites": 27,
  "lastFlushAt": "2025-12-14T03:47:32.730877Z"
}
```

### GET /api/events?limit=200

メモリリングからイベントを取得（最新順、デフォルト200件、最大2000件）

**レスポンス例:**
```json
[
  {
    "seenAt": "2025-12-14T03:48:27.968578Z",
    "src": "ble",
    "mac": "E9A428A7F133",
    "rssi": -58,
    "adv_hex": null,
    "manufacturer_hex": "{76: b'\\x12\\x02\\x00\\x03'}",
    "lacisId": "3003E9A428A7F1330096",
    "productType": "003",
    "productCode": "0096"
  }
]
```

### POST /api/ingest

is02からのセンサーデータを受信（primary endpoint）

**リクエスト例:**
```json
{
  "gateway": {
    "lacisId": "30020011223344550096",
    "ip": "192.168.97.69",
    "rssi": -45
  },
  "sensor": {
    "lacisId": "30016CC8408C52440096",
    "mac": "6CC8408C5244",
    "productType": "001",
    "productCode": "0096"
  },
  "observedAt": "2025-12-14T05:40:00.000000Z",
  "state": {
    "temperatureC": 25.5,
    "humidityPct": 60.0,
    "batteryPct": 95
  },
  "raw": {
    "mfgHex": "49531234567890ABCDEF"
  }
}
```

**レスポンス:**
```json
{"ok": true}
```

### POST /api/events

is02からのセンサーデータを受信（alternative endpoint、/api/ingestと同じ）

### POST /api/debug/publishSample

デバッグ用のサンプルイベントを投入（BLE無しでもテスト可能）

**レスポンス:**
```json
{"ok": true}
```

### WS /ws

WebSocket接続でリアルタイムイベント受信

### GET /

ブラウザUI（イベント一覧とWebSocket接続状態を表示）

## デバイス登録

araneaDeviceGateへの登録例:

```bash
# MACアドレス取得
MAC_RAW=$(ip link show end0 | grep link/ether | awk '{print $2}')
MAC12=$(echo $MAC_RAW | tr -d ':' | tr '[:lower:]' '[:upper:]')
LACIS_ID="3003${MAC12}0096"

# 登録リクエスト
curl -X POST "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate" \
  -H "Content-Type: application/json" \
  -H "X-LacisOath-Version: 1" \
  -d '{
    "lacisOath": {
      "lacisId": "<TENANT_PRIMARY_LACISID>",
      "userId": "info+ichiyama@neki.tech",
      "pass": "dJBU^TpG%j$5",
      "cic": "263238",
      "method": "register"
    },
    "userObject": {
      "tid": "T2025120608261484221",
      "typeDomain": "araneaDevice",
      "type": "ISMS_ar-is03",
      "permission": 11
    },
    "deviceMeta": {
      "macAddress": "'$MAC12'",
      "productType": "003",
      "productCode": "0096"
    }
  }'
```

## デバイス情報

### is03-1
- IP: 192.168.96.201/23
- hostname: is03-1
- MAC: 02:00:a8:46:fa:c5
- lacisId: 30030200A846FAC50096
- cic_code: 285135

### is03-2
- IP: 192.168.96.202/23
- hostname: is03-2
- MAC: 02:00:a8:72:27:42
- lacisId: 30030200A87227420096
- cic_code: 537533

## トラブルシューティング

### Bluetoothが起動しない

```bash
sudo systemctl status bluetooth
# ハードウェアが無い場合は "bluetooth: ng" となるが、アプリは動作する
```

### サービスが起動しない

```bash
sudo systemctl status is03-relay --no-pager -l
sudo journalctl -u is03-relay -n 50
```

### データベースの確認

```bash
sqlite3 /var/lib/is03-relay/relay.sqlite "SELECT COUNT(*) FROM events;"
sqlite3 /var/lib/is03-relay/relay.sqlite "SELECT * FROM events ORDER BY id DESC LIMIT 5;"
```

## ライセンス

市山水産株式会社 - ISMS プロジェクト
