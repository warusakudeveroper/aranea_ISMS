# NOTE (outdated)

## 修正履歴
- Before: 初期データフロー設計。is04a/is05a固有API分離、バックオフ/WDT非介入、force送信など最新実装が未反映。
- After: 現行との差分を明示し、最新経路（typeSpecific API、Heartbeat間隔制御、クラウド/Zero3送信先）を反映するTODOを追記。本文に2025-01時点の実装整合メモ（is04a/is05aの送信間隔・バックオフ・APIパス）を追加。

# TODO
- is04a/is05aの最新HTTP API・typeSpecificフック・バックオフ/force送信方針を追記
- Heartbeat/状態送信間隔とクラウド・Zero3両経路の優先順位を現行実装に合わせる
- Gate登録/認証手順を最新仕様で再検証し、URLを更新

---

## 2025-01 現行整合メモ（実装基準）
- 共通: HTTPタイムアウト3s。3連続失敗で30sバックオフ。送信毎に`yield()`でWDT回避。StaticJsonDocument採用＋String::reserveで断片化抑止。
- is04a: `/api/status` `/api/config` は共通+typeSpecific。固有APIは `/api/is04a/pulse`, `/api/is04a/trigger`。状態送信は心拍/入力で最小1s間隔、パルス開始/終了時は `force=true` で即時送信（Trigger1 on/off）。ローカル(relay pri/sec)→クラウドの順で送信。
- is05a: `/api/channel`(GET/POST), `/api/pulse`(POST), `/api/webhook/config|test`。状態送信は1s間隔制限、バックオフ同様。Webhook/StateReporterとも3sタイムアウト・30sバックオフ。
- Gate/Cloud: araneaDeviceGateで登録→CIC取得後、deviceStateReportへ送信（URLはAraneaSettingsデフォルトのCloud Functions）。Gate準備中のためPENDING_TEST継続。

# データフロー仕様書

**必須参照ドキュメント** - デバイス間通信のデータ形式と流れ

---

## 1. システム全体のデータフロー図

```
┌─────────────────────────────────────────────────────────────────────┐
│                         離島ローカルネットワーク                      │
│                                                                     │
│  ┌─────────┐    BLE Advertise     ┌─────────┐                      │
│  │  is01   │ ──────────────────── │  is03-1 │ ─┐                   │
│  │ (電池式) │                      │ (Zero3) │  │  LAN              │
│  │ 温湿度   │ ──────────────────── │         │  ├──── ブラウザ      │
│  └─────────┘    BLE Advertise     ├─────────┤  │   確認画面        │
│                                   │  is03-2 │ ─┘                   │
│                                   │ (Zero3) │   冗長化             │
│                                   └─────────┘                      │
│                                        ▲                           │
│  ┌─────────┐    BLE Scan              │ HTTP POST                  │
│  │  is02   │ ─────────────────────────┘                            │
│  │ (中継)  │                                                       │
│  └─────────┘                                                       │
│       │                                                            │
│       │ (将来) HTTP/MQTT                                           │
└───────┼────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────┐
│    mobes2.0     │
│   (クラウド)     │
│   Firebase      │
└─────────────────┘
```

---

## 2. BLE Advertise Payload（is01 → is02/is03）

### 2.1 Manufacturer Specific Data 構造

| Offset | Size | Field | Type | 説明 |
|--------|------|-------|------|------|
| 0 | 1 | ver | uint8 | ペイロードバージョン (0x01) |
| 1 | 2 | productType | uint16_le | 製品タイプ (001-007) |
| 3 | 2 | productCode | uint16_le | 製品コード (0096) |
| 5 | 6 | mac | bytes | MACアドレス (6bytes) |
| 11 | 4 | ts | uint32_le | タイムスタンプ (epoch秒 or 起床回数) |
| 15 | 2 | temp | int16_le | 温度 (×100, 例: 2350 = 23.50℃) |
| 17 | 2 | hum | uint16_le | 湿度 (×100, 例: 6520 = 65.20%) |
| 19 | 1 | battery | uint8 | バッテリー残量 (0-100%) |
| 20 | 1 | flags | uint8 | フラグ (bit0:reed, bit1:button等) |

**合計: 21 bytes** (BLE Advertising 31byte制限内)

### 2.2 lacisId の再構成（受信側）

受信した payload から lacisId を生成:
```
lacisId = "3" + zeroPad(productType, 3) + macToHex(mac) + zeroPad(productCode, 4)
```

例:
- productType: 1, productCode: 96, mac: AA:BB:CC:DD:EE:FF
- lacisId: `3001AABBCCDDEEFF0096`

---

## 3. HTTP API（is02 → is03）

### 3.1 センサーデータ送信

**POST /api/events**

Request:
```json
{
  "sensor": {
    "mac": "AABBCCDDEEFF",
    "productType": "001",
    "productCode": "0096",
    "lacisId": "3001AABBCCDDEEFF0096"
  },
  "state": {
    "temperature": 23.5,
    "humidity": 65.2,
    "battery": 85
  },
  "meta": {
    "rssi": -65,
    "observedAt": "2025-01-15T10:30:00Z",
    "gatewayId": "3002FFEEDDCCBBAA0096"
  }
}
```

Response:
```json
{
  "ok": true,
  "eventId": "evt_12345"
}
```

### 3.2 is03 API一覧

| Method | Path | 説明 |
|--------|------|------|
| GET | /api/status | サーバー状態確認 |
| GET | /api/events | 最新イベント取得 (limit=2000) |
| POST | /api/events | イベント登録 (is02から) |
| WS | /ws | リアルタイムイベント配信 |
| POST | /api/debug/publishSample | テスト用疑似イベント発行 |

---

## 4. WebSocket イベント（is03 → ブラウザ）

### 4.1 接続

```javascript
const ws = new WebSocket('ws://192.168.96.201:8080/ws');
```

### 4.2 イベント形式

```json
{
  "type": "sensor_event",
  "data": {
    "lacisId": "3001AABBCCDDEEFF0096",
    "temperature": 23.5,
    "humidity": 65.2,
    "battery": 85,
    "rssi": -65,
    "receivedAt": "2025-01-15T10:30:00Z"
  }
}
```

---

## 5. デバイス登録フロー（is01 → mobes2.0）

### 5.1 初回アクティベート

**POST /araneaDeviceGate**

Request:
```json
{
  "deviceMeta": {
    "productType": "001",
    "productCode": "0096",
    "macAddress": "AABBCCDDEEFF",
    "firmwareVersion": "1.0.0"
  },
  "lacisId": "3001AABBCCDDEEFF0096"
}
```

Response:
```json
{
  "ok": true,
  "cic_code": "123456",
  "stateEndpoint": "https://api.example.com/deviceState"
}
```

### 5.2 CIC保存

- NVS (Preferences) に `cic_code` を保存
- 次回起動時は登録をスキップ
- OLEDに CIC を表示（現地確認用）

---

## 6. is04/is05 通信仕様

### 6.1 is05 → is03（開閉状態通知）

**POST /api/doorState**

```json
{
  "lacisId": "3005AABBCCDDEEFF0096",
  "channels": [
    { "ch": 0, "did": "DOOR0001", "state": "open" },
    { "ch": 1, "did": "DOOR0002", "state": "closed" }
  ],
  "observedAt": "2025-01-15T10:30:00Z"
}
```

### 6.2 is03 → is04（開閉制御）

**MQTT (ローカル)**: `local/aranea/{rid}/{lacisId}/cmd`

```json
{
  "action": "pulse",
  "ch": 1,
  "ms": 500
}
```

---

## 7. エラーハンドリング

### 7.1 送信失敗時のフォールバック

```
1. relay.primary (is03-1: 192.168.96.201) へ送信
2. 失敗 → relay.secondary (is03-2: 192.168.96.202) へ送信
3. 失敗 → ローカルバッファに保持、一定時間後リトライ
```

### 7.2 重複排除（is03側）

- 同一 lacisId + observedAt (±3秒) は重複として無視
- gatewayId が異なっても重複判定対象

---

## 8. データ保持ポリシー

### 8.1 is03 メモリ

- 最新 2000 件をリングバッファで保持
- 超過分は古いものから破棄

### 8.2 is03 SQLite

- バッチ書き込み: 10秒ごと or 50件ごと
- 最大保持: 20000 件
- ローテーション: 古いものから削除
