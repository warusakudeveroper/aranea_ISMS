# AraneaSDK API Reference

## 1. Endpoints Overview

| Endpoint | Method | Auth | Purpose |
|----------|--------|------|---------|
| `/araneaDeviceGate` | POST | LacisOath | デバイス登録・CIC発行 |
| `/deviceStateReport` | POST | DeviceAuth | 単発状態レポート |
| `/deviceStateReport/batch` | POST | DeviceAuth | バッチ状態レポート |
| `/araneaDeviceGetState/{lacisId}` | GET | Firebase | 状態取得 |
| `/araneaDeviceCommand/{lacisId}` | POST | Firebase | コマンド発行 |
| `/araneaDeviceConfig/{lacisId}` | POST | Firebase | 設定更新 |

Base URL: `https://us-central1-mobesorder.cloudfunctions.net`

---

## 2. araneaDeviceGate

### 2.1 Overview

デバイスの初回登録とCIC発行を行います。

- **URL**: `/araneaDeviceGate`
- **Method**: `POST`
- **Auth**: LacisOath (Tenant Primary)
- **Content-Type**: `application/json`

### 2.2 Request

```typescript
interface AraneaDeviceGateRequest {
  lacisOath: {
    lacisId: string;      // Tenant Primary の LacisID
    userId: string;       // Tenant Primary の Email
    cic: string;          // Tenant Primary の CIC
    method: 'register' | 'update' | 'delete';
  };

  userObject: {
    lacisID: string;      // 登録するデバイスの LacisID
    tid: string;          // Tenant ID
    typeDomain: 'araneaDevice';
    type: string;         // "ISMS_ar-is04a" など
  };

  deviceMeta: {
    macAddress: string;   // 12桁HEX
    productType: string;  // 3桁
    productCode: string;  // 4桁
  };

  userObjectDetail?: {
    firmware?: {
      version: string;
      buildDate: string;
    };
    config?: Record<string, any>;
  };
}
```

### 2.3 Response

```typescript
interface AraneaDeviceGateResponse {
  ok: boolean;

  userObject?: {
    cic_code: string;     // 発行されたCIC (6桁)
    lacisId: string;
    tid: string;
    type: string;
  };

  stateEndpoint?: string;  // 状態レポート送信先URL
  mqttEndpoint?: string;   // MQTT接続先URL (双方向デバイスのみ)

  error?: {
    code: string;
    message: string;
  };
}
```

### 2.4 Example

**Request**:
```bash
curl -X POST \
  https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate \
  -H "Content-Type: application/json" \
  -d '{
    "lacisOath": {
      "lacisId": "12767487939173857894",
      "userId": "info+ichiyama@neki.tech",
      "cic": "263238",
      "method": "register"
    },
    "userObject": {
      "lacisID": "30040123456789AB0001",
      "tid": "T2025120608261484221",
      "typeDomain": "araneaDevice",
      "type": "ISMS_ar-is04a"
    },
    "deviceMeta": {
      "macAddress": "0123456789AB",
      "productType": "004",
      "productCode": "0001"
    }
  }'
```

**Response (Success)**:
```json
{
  "ok": true,
  "userObject": {
    "cic_code": "456789",
    "lacisId": "30040123456789AB0001",
    "tid": "T2025120608261484221",
    "type": "ISMS_ar-is04a"
  },
  "stateEndpoint": "https://us-central1-mobesorder.cloudfunctions.net/deviceStateReport",
  "mqttEndpoint": "wss://aranea-mqtt-bridge-1010551946141.asia-northeast1.run.app"
}
```

**Response (Error)**:
```json
{
  "ok": false,
  "error": {
    "code": "AUTH008",
    "message": "INSUFFICIENT_PERMISSION"
  }
}
```

---

## 3. deviceStateReport

### 3.1 Overview

デバイスの状態レポートを送信します。

- **URL**: `/deviceStateReport`
- **Method**: `POST`
- **Auth**: DeviceAuth (tid + lacisId + cic)
- **Content-Type**: `application/json`

### 3.2 Request

```typescript
interface DeviceStateReportRequest {
  auth: {
    tid: string;          // Tenant ID
    lacisId: string;      // デバイスの LacisID
    cic: string;          // デバイスの CIC
  };

  report: {
    type: string;         // "ISMS_ar-is04a"
    state: Record<string, any>;
    observedAt?: string;  // ISO8601 (省略時サーバー時刻)
    meta?: {
      firmwareVersion?: string;
      rssi?: number;
      heap?: number;
      uptime?: number;
    };
  };
}
```

### 3.3 Response

```typescript
interface DeviceStateReportResponse {
  ok: boolean;
  duplicate?: boolean;      // 重複検出
  dedupHash?: string;
  bigQueryRowId?: string;
  error?: {
    code: string;
    message: string;
  };
}
```

### 3.4 Example

**Request**:
```bash
curl -X POST \
  https://us-central1-mobesorder.cloudfunctions.net/deviceStateReport \
  -H "Content-Type: application/json" \
  -d '{
    "auth": {
      "tid": "T2025120608261484221",
      "lacisId": "30040123456789AB0001",
      "cic": "456789"
    },
    "report": {
      "type": "ISMS_ar-is04a",
      "state": {
        "output1": { "active": true, "updatedAt": "2025-01-01T00:00:00Z" },
        "output2": { "active": false, "updatedAt": "2025-01-01T00:00:00Z" }
      },
      "observedAt": "2025-01-01T00:00:00Z",
      "meta": {
        "firmwareVersion": "1.0.0",
        "rssi": -65,
        "heap": 120000
      }
    }
  }'
```

**Response**:
```json
{
  "ok": true,
  "duplicate": false,
  "dedupHash": "a1b2c3d4e5f6...",
  "bigQueryRowId": "evt_1234567890_abc123"
}
```

---

## 4. deviceStateReport/batch

### 4.1 Overview

複数デバイスの状態レポートをバッチ送信します（ゲートウェイ用）。

- **URL**: `/deviceStateReport/batch`
- **Method**: `POST`
- **Auth**: DeviceAuth (ゲートウェイのtid + lacisId + cic)
- **Content-Type**: `application/json`

### 4.2 Request

```typescript
interface DeviceStateReportBatchRequest {
  auth: {
    tid: string;
    lacisId: string;      // ゲートウェイの LacisID
    cic: string;          // ゲートウェイの CIC
  };

  reports: Array<{
    lacisId: string;      // 各デバイスの LacisID
    type: string;
    state: Record<string, any>;
    observedAt?: string;
    advert?: {
      rssi?: number;
    };
  }>;
}
```

### 4.3 Response

```typescript
interface DeviceStateReportBatchResponse {
  ok: boolean;
  results: Array<{
    lacisId: string;
    ok: boolean;
    duplicate?: boolean;
    error?: string;
  }>;
  summary: {
    total: number;
    success: number;
    duplicate: number;
    failed: number;
  };
}
```

---

## 5. araneaDeviceGetState

### 5.1 Overview

デバイスの現在状態を取得します。

- **URL**: `/araneaDeviceGetState/{lacisId}`
- **Method**: `GET`
- **Auth**: Firebase Auth (permission >= 10)
- **Headers**: `Authorization: Bearer {idToken}`

### 5.2 Response

```typescript
interface AraneaDeviceGetStateResponse {
  ok: boolean;
  device?: {
    lacisId: string;
    tid: string;
    type: string;
    state: Record<string, any>;
    alive: boolean;
    lastUpdatedAt: string;
    config?: Record<string, any>;
  };
  error?: {
    code: string;
    message: string;
  };
}
```

### 5.3 Example

```bash
curl -X GET \
  https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGetState/30040123456789AB0001 \
  -H "Authorization: Bearer eyJhbGciOiJSUzI1NiIs..."
```

---

## 6. araneaDeviceCommand

### 6.1 Overview

デバイスにコマンドを発行します。

- **URL**: `/araneaDeviceCommand/{lacisId}`
- **Method**: `POST`
- **Auth**: Firebase Auth (permission >= 61)
- **Content-Type**: `application/json`

### 6.2 Request

```typescript
interface AraneaDeviceCommandRequest {
  command: string;        // "pulse", "setOutput", etc.
  params?: Record<string, any>;
  ttl?: number;           // コマンド有効期限 (秒)
}
```

### 6.3 Response

```typescript
interface AraneaDeviceCommandResponse {
  ok: boolean;
  commandId?: string;
  status?: 'queued' | 'delivered' | 'executed' | 'failed';
  error?: {
    code: string;
    message: string;
  };
}
```

### 6.4 Example

**Request**:
```bash
curl -X POST \
  https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceCommand/30040123456789AB0001 \
  -H "Authorization: Bearer eyJhbGciOiJSUzI1NiIs..." \
  -H "Content-Type: application/json" \
  -d '{
    "command": "pulse",
    "params": {
      "output": 1,
      "durationMs": 3000
    },
    "ttl": 60
  }'
```

---

## 7. araneaDeviceConfig

### 7.1 Overview

デバイスの設定を更新します。

- **URL**: `/araneaDeviceConfig/{lacisId}`
- **Method**: `POST`
- **Auth**: Firebase Auth (permission >= 61)
- **Content-Type**: `application/json`

### 7.2 Request

```typescript
interface AraneaDeviceConfigRequest {
  config: Record<string, any>;
  merge?: boolean;        // true: マージ更新, false: 全置換
}
```

### 7.3 Response

```typescript
interface AraneaDeviceConfigResponse {
  ok: boolean;
  version?: number;       // 新しい設定バージョン
  error?: {
    code: string;
    message: string;
  };
}
```

---

## 8. Local Relay API (Orange Pi)

### 8.1 Overview

ローカルリレー (is03/is20s) への状態レポート送信。

- **URL**: `http://{relay_ip}:8080/api/state`
- **Method**: `POST`
- **Content-Type**: `application/json`

### 8.2 Request Format

```typescript
interface LocalRelayStateRequest {
  tid: string;
  lacisId: string;
  cic: string;
  mac: string;
  state: Record<string, any>;
  timestamp: string;
}
```

### 8.3 Example

```bash
curl -X POST \
  http://192.168.96.201:8080/api/state \
  -H "Content-Type: application/json" \
  -d '{
    "tid": "T2025120608261484221",
    "lacisId": "30040123456789AB0001",
    "cic": "456789",
    "mac": "0123456789AB",
    "state": {
      "output1": { "active": true },
      "output2": { "active": false }
    },
    "timestamp": "2025-01-01T00:00:00Z"
  }'
```

---

## 9. MQTT Topics

### 9.1 Device → Cloud

| Topic | Purpose | QoS |
|-------|---------|-----|
| `aranea/{lacisId}/state` | 状態レポート | 1 |
| `aranea/{lacisId}/event` | イベント通知 | 1 |
| `aranea/{lacisId}/heartbeat` | 死活監視 | 0 |

### 9.2 Cloud → Device

| Topic | Purpose | QoS |
|-------|---------|-----|
| `aranea/{lacisId}/cmd` | コマンド配信 | 1 |
| `aranea/{lacisId}/config` | 設定更新 | 1 |

### 9.3 Payload Format

**State Report**:
```json
{
  "type": "state",
  "state": { ... },
  "timestamp": "2025-01-01T00:00:00Z"
}
```

**Command**:
```json
{
  "commandId": "cmd_123456",
  "command": "pulse",
  "params": { "output": 1, "durationMs": 3000 },
  "timestamp": "2025-01-01T00:00:00Z"
}
```

---

## 10. Error Codes

### 10.1 Authentication Errors (AUTH)

| Code | Message | Description |
|------|---------|-------------|
| AUTH001 | INVALID_LACISID_FORMAT | LacisID形式不正 |
| AUTH002 | INVALID_CIC_FORMAT | CIC形式不正 |
| AUTH003 | DEVICE_NOT_REGISTERED | 未登録デバイス |
| AUTH004 | TID_MISMATCH | TID不一致 |
| AUTH005 | INVALID_CIC | CIC不一致 |
| AUTH006 | CIC_DISABLED | CIC無効化済み |
| AUTH007 | PRIMARY_NOT_FOUND | Tenant Primary不明 |
| AUTH008 | INSUFFICIENT_PERMISSION | 権限不足 |
| AUTH009 | EMAIL_MISMATCH | Email不一致 |
| AUTH010 | TOKEN_EXPIRED | トークン期限切れ |

### 10.2 Validation Errors (VAL)

| Code | Message | Description |
|------|---------|-------------|
| VAL001 | INVALID_REQUEST_FORMAT | リクエスト形式不正 |
| VAL002 | MISSING_REQUIRED_FIELD | 必須フィールド欠如 |
| VAL003 | INVALID_TYPE | 不明なType |
| VAL004 | SCHEMA_VALIDATION_FAILED | スキーマ検証失敗 |
| VAL005 | INVALID_TIMESTAMP | タイムスタンプ形式不正 |

### 10.3 System Errors (SYS)

| Code | Message | Description |
|------|---------|-------------|
| SYS001 | INTERNAL_ERROR | 内部エラー |
| SYS002 | DATABASE_ERROR | DB接続エラー |
| SYS003 | RATE_LIMITED | レート制限 |
| SYS004 | SERVICE_UNAVAILABLE | サービス停止中 |

---

## 11. Rate Limits

| Endpoint | Limit | Window |
|----------|-------|--------|
| deviceStateReport | 60 req | /minute/device |
| deviceStateReport/batch | 10 req | /minute/gateway |
| araneaDeviceCommand | 30 req | /minute/device |
| araneaDeviceConfig | 10 req | /minute/device |
