# AraneaSDK Schema Specification

## 1. Overview

araneaDeviceシステムでは、以下のスキーマを正本として管理します。

| スキーマ | 用途 | 管理場所 |
|---------|------|---------|
| userObject | デバイス基本情報 | Firestore + SDK |
| userObject_detail | デバイス詳細情報 | Firestore + SDK |
| araneaDeviceStates | 状態キャッシュ | Firestore |
| deviceStateReport | 状態レポート | SDK |
| Type別スキーマ | デバイス固有 | typeSettings |

---

## 2. userObject Schema

### 2.1 Structure

```typescript
interface UserObject {
  // ドキュメントID = lacisId (20文字)

  lacis: {
    type_domain: 'araneaDevice';
    type: string;           // e.g., "ISMS_ar-is04a"
    tid: string;            // Tenant ID
    permission: number;     // 固定: 10
    cic_code: string;       // 6桁数字
    cic_active: boolean;    // CIC有効フラグ
  };

  activity: {
    created_at: Timestamp;
    lastUpdated_at: Timestamp;
    Ordination_at?: Timestamp;
  };

  device: {
    macAddress: string;     // 12桁HEX (大文字)
    productType: string;    // 3桁数字
    productCode: string;    // 4桁数字
  };

  fid?: string[];           // 施設ID配列 (オプション)
}
```

### 2.2 JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://aranea.sdk/schemas/userObject.json",
  "title": "araneaDevice userObject",
  "type": "object",
  "required": ["lacis", "activity", "device"],
  "properties": {
    "lacis": {
      "type": "object",
      "required": ["type_domain", "type", "tid", "permission", "cic_code", "cic_active"],
      "properties": {
        "type_domain": {
          "type": "string",
          "const": "araneaDevice"
        },
        "type": {
          "type": "string",
          "pattern": "^ISMS_[a-zA-Z0-9-]+$"
        },
        "tid": {
          "type": "string",
          "pattern": "^T[0-9]{19}$|^T[0-9]{12}[a-zA-Z0-9]{7}$|^[0-9]{20}$"
        },
        "permission": {
          "type": "integer",
          "minimum": 10,
          "maximum": 100
        },
        "cic_code": {
          "type": "string",
          "pattern": "^[0-9]{6}$"
        },
        "cic_active": {
          "type": "boolean"
        }
      }
    },
    "activity": {
      "type": "object",
      "required": ["created_at", "lastUpdated_at"],
      "properties": {
        "created_at": { "type": "string", "format": "date-time" },
        "lastUpdated_at": { "type": "string", "format": "date-time" },
        "Ordination_at": { "type": "string", "format": "date-time" }
      }
    },
    "device": {
      "type": "object",
      "required": ["macAddress", "productType", "productCode"],
      "properties": {
        "macAddress": {
          "type": "string",
          "pattern": "^[0-9A-F]{12}$"
        },
        "productType": {
          "type": "string",
          "pattern": "^[0-9]{3}$"
        },
        "productCode": {
          "type": "string",
          "pattern": "^[0-9]{4}$"
        }
      }
    },
    "fid": {
      "type": "array",
      "items": { "type": "string" }
    }
  }
}
```

---

## 3. userObject_detail Schema

### 3.1 Structure

```typescript
interface UserObjectDetail {
  // ドキュメントID = lacisId

  firmware: {
    version: string;        // e.g., "1.0.0"
    buildDate: string;      // ISO8601
    modules: string[];      // ["WiFi", "MQTT", "OTA"]
  };

  config: {
    // デバイス固有設定 (Type別)
    [key: string]: any;
  };

  status: {
    online: boolean;
    lastSeen: Timestamp;
    rssi?: number;
    heap?: number;
  };

  network: {
    ip?: string;
    ssid?: string;
    gateway?: string;
    subnet?: string;
  };
}
```

### 3.2 Type別 Config Examples

#### is04a Config
```json
{
  "config": {
    "pulseMs": 3000,
    "interlockMs": 200,
    "debounceMs": 50,
    "input1Pin": 5,
    "input2Pin": 18,
    "output1Pin": 12,
    "output2Pin": 14,
    "triggerAssign": {
      "input1": 1,
      "input2": 2
    }
  }
}
```

#### is05a Config
```json
{
  "config": {
    "channels": [
      { "ch": 1, "pin": 5, "name": "Window1", "meaning": "open", "debounceMs": 50 },
      { "ch": 2, "pin": 18, "name": "Window2", "meaning": "close", "debounceMs": 50 }
    ],
    "webhook": {
      "discord": "https://discord.com/api/webhooks/...",
      "slack": "https://hooks.slack.com/...",
      "generic": null
    },
    "rules": []
  }
}
```

---

## 4. araneaDeviceStates Schema

### 4.1 Structure

```typescript
interface AraneaDeviceStates {
  // ドキュメントID = lacisId

  lacisId: string;
  tid: string;
  fid: string[];
  type: string;

  state: Record<string, any>;   // Type別状態

  semanticTags: string[];       // ["温度", "湿度"]

  advert: {
    gatewayLacisId?: string;    // BLE受信ゲートウェイ
    rssi?: number;
    observedAt: Timestamp;
  };

  alive: boolean;
  lastUpdatedAt: Timestamp;
}
```

### 4.2 State Examples by Type

#### ISMS_ar-is01 (温湿度センサー)
```json
{
  "state": {
    "Temperature": 22.5,
    "Humidity": 50,
    "BatteryLevel": 85
  }
}
```

#### ISMS_ar-is04a (接点コントローラー)
```json
{
  "state": {
    "output1": {
      "active": true,
      "updatedAt": "2025-01-01T00:00:00Z"
    },
    "output2": {
      "active": false,
      "updatedAt": "2025-01-01T00:00:00Z"
    },
    "input1": {
      "active": false,
      "updatedAt": "2025-01-01T00:00:00Z"
    },
    "input2": {
      "active": false,
      "updatedAt": "2025-01-01T00:00:00Z"
    }
  }
}
```

#### ISMS_ar-is05a (8ch検出器)
```json
{
  "state": {
    "channels": [
      { "ch": 1, "active": true, "name": "Window1" },
      { "ch": 2, "active": false, "name": "Window2" },
      { "ch": 3, "active": false, "name": "Door1" }
    ]
  }
}
```

---

## 5. deviceStateReport Schema

### 5.1 Request Format

```typescript
interface DeviceStateReportRequest {
  auth: {
    tid: string;
    lacisId: string;
    cic: string;
  };

  report: {
    type: string;                    // "ISMS_ar-is04a"
    state: Record<string, any>;      // Type別状態
    observedAt?: string;             // ISO8601 (省略時はサーバー時刻)
    meta?: {
      firmwareVersion?: string;
      rssi?: number;
      heap?: number;
      uptime?: number;
    };
  };
}
```

### 5.2 Response Format

```typescript
interface DeviceStateReportResponse {
  ok: boolean;
  duplicate?: boolean;              // 重複検出時
  dedupHash?: string;               // 重複排除ハッシュ
  bigQueryRowId?: string;           // BQ挿入ID
  error?: {
    code: string;
    message: string;
  };
}
```

### 5.3 Batch Format

```typescript
interface DeviceStateReportBatchRequest {
  auth: {
    tid: string;
    lacisId: string;                 // ゲートウェイのlacisId
    cic: string;
  };

  reports: Array<{
    lacisId: string;                 // 各デバイスのlacisId
    type: string;
    state: Record<string, any>;
    observedAt?: string;
  }>;
}
```

---

## 6. Type Registry Schema

### 6.1 typeSettings Structure

```
Firestore Path: typeSettings/araneaDevice/{type}
```

```typescript
interface TypeSettings {
  // ドキュメントID = type (e.g., "ISMS_ar-is04a")

  displayName: string;              // "Window Controller"
  description: string;              // 説明文
  version: number;                  // スキーマバージョン

  stateSchema: {
    type: 'object';
    properties: Record<string, JSONSchemaProperty>;
    required?: string[];
  };

  configSchema: {
    type: 'object';
    properties: Record<string, JSONSchemaProperty>;
    required?: string[];
  };

  commandSchema?: {
    type: 'object';
    properties: Record<string, JSONSchemaProperty>;
  };

  capabilities: string[];           // ["output", "input", "pulse"]
  semanticTags: string[];           // ["窓", "ドア", "接点"]
}
```

### 6.2 Example: ISMS_ar-is04a

```json
{
  "displayName": "Window & Door Controller",
  "description": "2ch物理入力 + 2ch接点出力コントローラー",
  "version": 1,

  "stateSchema": {
    "type": "object",
    "properties": {
      "output1": {
        "type": "object",
        "properties": {
          "active": { "type": "boolean" },
          "updatedAt": { "type": "string", "format": "date-time" }
        }
      },
      "output2": {
        "type": "object",
        "properties": {
          "active": { "type": "boolean" },
          "updatedAt": { "type": "string", "format": "date-time" }
        }
      },
      "input1": {
        "type": "object",
        "properties": {
          "active": { "type": "boolean" },
          "updatedAt": { "type": "string", "format": "date-time" }
        }
      },
      "input2": {
        "type": "object",
        "properties": {
          "active": { "type": "boolean" },
          "updatedAt": { "type": "string", "format": "date-time" }
        }
      }
    },
    "required": ["output1", "output2"]
  },

  "configSchema": {
    "type": "object",
    "properties": {
      "pulseMs": { "type": "integer", "minimum": 100, "maximum": 60000 },
      "interlockMs": { "type": "integer", "minimum": 0, "maximum": 10000 },
      "debounceMs": { "type": "integer", "minimum": 5, "maximum": 10000 },
      "triggerAssign": {
        "type": "object",
        "properties": {
          "input1": { "type": "integer", "enum": [0, 1, 2] },
          "input2": { "type": "integer", "enum": [0, 1, 2] }
        }
      }
    }
  },

  "commandSchema": {
    "type": "object",
    "properties": {
      "pulse": {
        "type": "object",
        "properties": {
          "output": { "type": "integer", "enum": [1, 2] },
          "durationMs": { "type": "integer", "minimum": 100 }
        },
        "required": ["output"]
      }
    }
  },

  "capabilities": ["output", "input", "pulse", "trigger"],
  "semanticTags": ["窓", "ドア", "接点出力", "物理入力"]
}
```

---

## 7. ProductType/ProductCode Registry

### 7.1 ProductType Table

| ProductType | Device | Type Name | Description |
|-------------|--------|-----------|-------------|
| 001 | is01 | ISMS_ar-is01 | 電池式温湿度センサー |
| 002 | is02 | ISMS_ar-is02 | WiFiゲートウェイ |
| 003 | is03 | ISMS_ar-is03 | Orange Pi Zero3 |
| 004 | is04 | ISMS_ar-is04a | 2ch入出力コントローラー |
| 005 | is05 | ISMS_ar-is05a | 8ch検出器 |
| 006 | is06 | ISMS_ar-is06a | 6ch出力 (PWM対応) |
| 010 | is10 | ISMS_ar-is10 | ルーター検査デバイス |
| 020 | is20 | ISMS_ar-is20s | ネットワーク監視 |

### 7.2 ProductCode Convention

| ProductCode | Meaning |
|-------------|---------|
| 0001 | 標準版 |
| 0002 | バリエーション2 (e.g., is10t = Tapo対応) |
| 0096 | ISMS専用 |
| 0100+ | カスタム版 |

---

## 8. Semantic Tags

### 8.1 Standard Tags

```typescript
const STANDARD_TAGS = [
  // 環境センサー系
  '温度', '湿度', '気圧', '照度', 'CO2',

  // 検出系
  '窓', 'ドア', '人感', '振動', '水漏れ',

  // 制御系
  '接点出力', 'リレー', 'PWM', 'パルス',

  // 入力系
  '物理入力', 'デジタル入力', 'アナログ入力',

  // 通信系
  'BLE', 'WiFi', 'LoRa', 'MQTT',

  // 電源系
  '電池', 'USB給電', 'PoE'
];
```

### 8.2 Usage

```typescript
// araneaDeviceStates
{
  "semanticTags": ["温度", "湿度", "電池"]
}

// typeSettings
{
  "semanticTags": ["窓", "ドア", "接点出力", "物理入力"]
}
```

---

## 9. Schema Validation Rules

### 9.1 LacisID Validation

```typescript
function validateLacisId(lacisId: string): ValidationResult {
  // 長さ
  if (lacisId.length !== 20) {
    return { valid: false, error: 'LENGTH_MUST_BE_20' };
  }

  // Prefix (araneaDevice = "3")
  if (lacisId[0] !== '3') {
    return { valid: false, error: 'INVALID_PREFIX' };
  }

  // ProductType (001-999)
  const productType = lacisId.substring(1, 4);
  if (!/^[0-9]{3}$/.test(productType)) {
    return { valid: false, error: 'INVALID_PRODUCT_TYPE' };
  }

  // MAC (12桁HEX)
  const mac = lacisId.substring(4, 16);
  if (!/^[0-9A-F]{12}$/i.test(mac)) {
    return { valid: false, error: 'INVALID_MAC' };
  }

  // ProductCode (0001-9999)
  const productCode = lacisId.substring(16, 20);
  if (!/^[0-9]{4}$/.test(productCode)) {
    return { valid: false, error: 'INVALID_PRODUCT_CODE' };
  }

  return { valid: true };
}
```

### 9.2 State Schema Validation

```typescript
async function validateState(
  type: string,
  state: Record<string, any>
): Promise<ValidationResult> {
  // typeSettings からスキーマ取得
  const typeSettings = await getTypeSettings(type);
  if (!typeSettings) {
    return { valid: false, error: 'UNKNOWN_TYPE' };
  }

  // JSON Schema で検証
  const ajv = new Ajv();
  const validate = ajv.compile(typeSettings.stateSchema);

  if (!validate(state)) {
    return {
      valid: false,
      error: 'SCHEMA_VALIDATION_FAILED',
      details: validate.errors
    };
  }

  return { valid: true };
}
```

---

## 10. Schema Evolution

### 10.1 Versioning Strategy

- `typeSettings.version` でバージョン管理
- 破壊的変更時はメジャーバージョンアップ
- 追加フィールドはマイナーバージョンアップ

### 10.2 Migration Rules

1. **新規フィールド追加**: デフォルト値設定
2. **フィールド削除**: deprecatedフラグ → 次バージョンで削除
3. **型変更**: 新フィールド追加 → 旧フィールド削除

### 10.3 Backward Compatibility

- 古いファームウェアからのレポートも受け入れる
- 不明フィールドは無視（ログ記録）
- 必須フィールド欠如はエラー
