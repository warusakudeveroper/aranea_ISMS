# IS06S API リファレンスガイド

**バージョン**: 1.0.0
**対象デバイス**: aranea_ar-is06s
**最終更新**: 2026-01-25

---

## 概要

IS06Sは6チャンネルのリレー・スイッチコントローラーです。
- CH1-4: Digital/PWM出力
- CH5-6: Digital入出力

---

## ベースURL

```
http://{device_ip}/api/
```

---

## エンドポイント一覧

| メソッド | パス | 説明 |
|----------|------|------|
| GET | /api/status | デバイス全体ステータス |
| GET | /api/pin/all | 全PIN状態・設定 |
| GET | /api/pin/{ch}/state | 指定PINの状態 |
| POST | /api/pin/{ch}/state | PIN状態設定 |
| GET | /api/pin/{ch}/setting | 指定PINの設定 |
| POST | /api/pin/{ch}/setting | PIN設定変更 |
| POST | /api/pin/{ch}/toggle | PINトグル |
| GET | /api/settings | グローバル設定取得 |
| POST | /api/settings | グローバル設定保存 |
| POST | /api/ota/upload | OTAファームウェア更新 |
| GET | /api/ota/status | OTA状態取得 |
| GET | /api/ota/info | OTAパーティション情報 |
| POST | /api/ota/confirm | OTA確認 |

---

## 1. デバイスステータス

### GET /api/status

デバイスの全体ステータスを取得します。

**レスポンス例**:
```json
{
  "ok": true,
  "uiVersion": "1.6.0",
  "device": {
    "type": "aranea_ar-is06s",
    "lacisId": "30066CC84054CB480200",
    "cic": "858628",
    "mac": "6CC84054CB48",
    "hostname": "is06s-CB48"
  },
  "network": {
    "ip": "192.168.77.32",
    "ssid": "sorapia_facility_wifi",
    "rssi": -70,
    "gateway": "192.168.77.1",
    "apMode": false
  },
  "system": {
    "ntpTime": "2026-01-25T06:00:00Z",
    "ntpSynced": true,
    "uptime": 600,
    "uptimeHuman": "10m 0s",
    "heap": 150000,
    "heapLargest": 90000,
    "cpuFreq": 240,
    "flashSize": 4194304,
    "chipTemp": 52.5
  },
  "firmware": {
    "version": "1.0.0"
  },
  "cloud": {
    "registered": true,
    "mqttConnected": true
  },
  "typeSpecific": {
    "pins": [...]
  }
}
```

---

## 2. PIN制御

### GET /api/pin/all

全PINの状態と設定を取得します。

**レスポンス例**:
```json
{
  "pins": [
    {
      "channel": 1,
      "enabled": true,
      "state": 0,
      "pwm": 0,
      "name": "メインリレー",
      "type": "digitalOutput",
      "actionMode": "Mom",
      "stateName": ["on:解錠", "off:施錠"],
      "validity": 3000,
      "debounce": 3000,
      "rateOfChange": 4000,
      "expiryDate": "",
      "expiryEnabled": false,
      "allocation": []
    },
    ...
  ]
}
```

### GET /api/pin/{ch}/state

指定チャンネルの状態を取得します。

**パラメータ**:
- `ch`: チャンネル番号 (1-6)

**レスポンス例**:
```json
{
  "channel": 1,
  "enabled": true,
  "state": 1,
  "pwm": 0,
  "name": "メインリレー",
  "type": "digitalOutput",
  "actionMode": "Mom",
  "stateName": ["on:解錠", "off:施錠"]
}
```

### POST /api/pin/{ch}/state

指定チャンネルの状態を設定します。

**パラメータ**:
- `ch`: チャンネル番号 (1-6)

**リクエストボディ**:
```json
{
  "state": 1
}
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| state | int | Digital: 0=OFF, 1=ON / PWM: 0-100 |

**レスポンス**:
```json
{
  "ok": true,
  "message": "OK"
}
```

### POST /api/pin/{ch}/toggle

指定チャンネルの状態をトグルします。

**レスポンス例**:
```json
{
  "ok": true,
  "channel": 1,
  "state": 1
}
```

### GET /api/pin/{ch}/setting

指定チャンネルの設定を取得します。

**レスポンス例**:
```json
{
  "channel": 1,
  "enabled": true,
  "name": "メインリレー",
  "type": "digitalOutput",
  "actionMode": "Mom",
  "validity": 3000,
  "debounce": 3000,
  "rateOfChange": 4000,
  "allocation": [],
  "expiryDate": "",
  "expiryEnabled": false,
  "stateName": ["on:解錠", "off:施錠"]
}
```

### POST /api/pin/{ch}/setting

指定チャンネルの設定を変更します。

**リクエストボディ**:
```json
{
  "enabled": true,
  "type": "pwmOutput",
  "name": "照明1",
  "actionMode": "Slow",
  "validity": 3000,
  "debounce": 3000,
  "rateOfChange": 4000,
  "stateName": ["0:消灯", "30:暗め", "60:中間", "100:全灯"],
  "allocation": ["CH1", "CH2"],
  "expiryDate": "202601251200",
  "expiryEnabled": false
}
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| enabled | bool | PIN有効/無効 |
| type | string | "digitalOutput", "pwmOutput", "digitalInput", "disabled" |
| name | string | 表示名 |
| actionMode | string | "Mom", "Alt", "Slow", "Rapid", "rotate" |
| validity | int | モーメンタリ動作時間 (ms) |
| debounce | int | デバウンス時間 (ms) |
| rateOfChange | int | PWM変化時間 (ms) |
| stateName | string[] | 状態表示名配列 |
| allocation | string[] | 入力連動先 (例: ["CH1", "CH2"]) |
| expiryDate | string | 有効期限 (YYYYMMDDHHMM形式) |
| expiryEnabled | bool | 有効期限機能の有効/無効 |

---

## 3. グローバル設定

### GET /api/settings

グローバル設定を取得します。

**レスポンス例**:
```json
{
  "ok": true,
  "settings": {
    "device_name": "テスト用IS06S",
    "rid": "villa1",
    "mqtt_url": "",
    "wifi": [
      {"index": 1, "ssid": "cluster1", "hasPassword": true}
    ],
    "pinGlobal": {
      "validity": 3000,
      "debounce": 3000,
      "rateOfChange": 4000
    }
  }
}
```

### POST /api/settings

グローバル設定を保存します。

**リクエストボディ**:
```json
{
  "device_name": "リレーコントローラー1",
  "rid": "villa1",
  "mqtt_url": "wss://mqtt.example.com",
  "pinGlobal": {
    "validity": 3000,
    "debounce": 3000,
    "rateOfChange": 4000
  }
}
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| device_name | string | デバイス表示名 |
| rid | string | Room ID（グループ/部屋識別） |
| mqtt_url | string | MQTTブローカーURL |
| pinGlobal | object | PIN共通デフォルト設定 |

---

## 4. OTA更新

### POST /api/ota/upload

ファームウェアをOTA更新します。

**Content-Type**: `multipart/form-data`

**フォームフィールド**:
- `firmware`: バイナリファイル (.bin)

**curlコマンド例**:
```bash
curl -X POST "http://192.168.77.32/api/ota/upload" \
  -F "firmware=@is06s.ino.bin"
```

### GET /api/ota/status

OTA更新状態を取得します。

**レスポンス例**:
```json
{
  "status": "idle",
  "progress": 0
}
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| status | string | "idle", "in_progress", "success", "error" |
| progress | int | 更新進捗 (0-100) |

### GET /api/ota/info

OTAパーティション情報を取得します。

**レスポンス例**:
```json
{
  "running_partition": "app0",
  "next_partition": "app1",
  "can_rollback": true,
  "pending_verify": false,
  "partition_size": 1966080
}
```

### POST /api/ota/confirm

OTA更新後のファームウェア確認を行います。

---

## 5. MQTTコマンド

MQTT接続時、以下のトピックでコマンドを受信します。

**コマンドトピック**: `aranea/{tid}/{lacisId}/command`
**状態トピック**: `aranea/{tid}/{lacisId}/state`
**ACKトピック**: `aranea/{tid}/{lacisId}/ack`

### set - PIN状態設定

```json
{
  "cmd": "set",
  "ch": 1,
  "state": 1,
  "requestId": "req-001"
}
```

### pulse - パルス出力

```json
{
  "cmd": "pulse",
  "ch": 1,
  "requestId": "req-002"
}
```

### allOff - 全チャンネルOFF

```json
{
  "cmd": "allOff",
  "requestId": "req-003"
}
```

### getState - 状態取得

```json
{
  "cmd": "getState",
  "ch": 1,
  "requestId": "req-004"
}
```

### setEnabled - PIN有効/無効設定

```json
{
  "cmd": "setEnabled",
  "ch": 1,
  "enabled": true,
  "requestId": "req-005"
}
```

---

## 6. エラーレスポンス

エラー時は以下の形式で返却されます。

```json
{
  "ok": false,
  "message": "エラーメッセージ"
}
```

| HTTPコード | 説明 |
|-----------|------|
| 200 | 成功 |
| 400 | リクエスト不正 |
| 500 | サーバーエラー |

---

## 7. PIN設定値詳細

### type (PINタイプ)

| 値 | 説明 |
|----|------|
| digitalOutput | デジタル出力 |
| pwmOutput | PWM出力 |
| digitalInput | デジタル入力 |
| disabled | 無効 |

### actionMode (動作モード)

| 値 | 説明 |
|----|------|
| Mom | モーメンタリ（パルス） |
| Alt | オルタネート（トグル） |
| Slow | PWM: なだらか変化 |
| Rapid | PWM: 即座変化 |
| rotate | 入力: ローテート |

### stateName (状態表示名)

Digital出力の場合:
```json
["on:解錠", "off:施錠"]
```

PWM出力の場合:
```json
["0:消灯", "30:暗め", "60:中間", "100:全灯"]
```

---

## 8. State Report形式

デバイスからクラウドに送信されるState Report形式:

```json
{
  "auth": {
    "tid": "T2025120621041161827",
    "lacisId": "30066CC84054CB480200",
    "cic": "858628"
  },
  "report": {
    "lacisId": "30066CC84054CB480200",
    "type": "aranea_ar-is06s",
    "observedAt": "2026-01-25T06:00:00Z",
    "state": {
      "PINState": {
        "CH1": {"state": "off"},
        "CH2": {"state": "50"},
        "CH3": {"state": "off"},
        "CH4": {"state": "off"},
        "CH5": {"state": "off"},
        "CH6": {"state": "off"}
      },
      "globalState": {
        "ipAddress": "192.168.77.32",
        "uptime": 600,
        "rssi": -70,
        "heapFree": 150000
      },
      "userObject": {
        "name": "テスト用IS06S",
        "rid": "villa1"
      }
    }
  }
}
```

---

## 9. 使用例

### PIN状態をONにする

```bash
curl -X POST "http://192.168.77.32/api/pin/1/state" \
  -H "Content-Type: application/json" \
  -d '{"state": 1}'
```

### PWM値を50%に設定

```bash
curl -X POST "http://192.168.77.32/api/pin/2/state" \
  -H "Content-Type: application/json" \
  -d '{"state": 50}'
```

### rid（部屋ID）を設定

```bash
curl -X POST "http://192.168.77.32/api/settings" \
  -H "Content-Type: application/json" \
  -d '{"rid": "villa1"}'
```

### 全PIN状態を取得

```bash
curl "http://192.168.77.32/api/pin/all"
```

### OTA更新

```bash
curl -X POST "http://192.168.77.32/api/ota/upload" \
  -F "firmware=@is06s.ino.bin"
```
