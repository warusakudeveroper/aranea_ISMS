# AraneaSDK Type Registry

## 1. Overview

araneaDeviceのType登録・管理に関する仕様です。

---

## 2. Type Hierarchy

```
TypeDomain: araneaDevice
├── Type: ISMS_ar-is01 (温湿度センサー)
├── Type: ISMS_ar-is02 (WiFiゲートウェイ)
├── Type: ISMS_ar-is03 (Orange Pi Zero3)
├── Type: ISMS_ar-is04a (2ch入出力コントローラー)
├── Type: ISMS_ar-is05a (8ch検出器)
├── Type: ISMS_ar-is06a (6ch出力)
├── Type: ISMS_ar-is10 (ルーター検査)
└── Type: ISMS_ar-is20s (ネットワーク監視)
```

---

## 3. ProductType Registry

### 3.1 Master Table

| ProductType | Device | Type Name | Description | Status |
|-------------|--------|-----------|-------------|--------|
| 001 | is01 | ISMS_ar-is01 | 電池式温湿度センサー (BLE) | Active |
| 002 | is02 | ISMS_ar-is02 | WiFiゲートウェイ (BLE→WiFi) | Active |
| 003 | is03 | ISMS_ar-is03 | Orange Pi Zero3 (ローカルリレー) | Active |
| 004 | is04 | ISMS_ar-is04a | 2ch入出力コントローラー | Active |
| 005 | is05 | ISMS_ar-is05a | 8ch検出器 | Active |
| 006 | is06 | ISMS_ar-is06a | 6ch出力 (PWM対応) | Active |
| 007 | is07 | ISMS_ar-is07 | (Reserved) | Reserved |
| 008 | is08 | ISMS_ar-is08 | (Reserved) | Reserved |
| 009 | is09 | ISMS_ar-is09 | (Reserved) | Reserved |
| 010 | is10 | ISMS_ar-is10 | ルーター検査デバイス | Active |
| 020 | is20 | ISMS_ar-is20s | ネットワーク監視 (Orange Pi) | Active |

### 3.2 ProductCode Convention

| ProductCode | Meaning | Example |
|-------------|---------|---------|
| 0001 | 標準版 | is04a-0001 |
| 0002 | バリエーション2 | is10t (Tapo対応) |
| 0096 | ISMS専用カスタム | 全デバイス共通 |
| 0100+ | OEM/カスタム版 | 顧客専用 |

---

## 4. Type Registration Process

### 4.1 New Type Registration Flow

```
1. 設計書作成 (aranea_ISMS/code/ESP32/{device}/design/DESIGN.md)
      ↓
2. ProductType申請 (SDK経由でmobes2.0に登録依頼)
      ↓
3. mobes2.0側でtypeSettings作成
      ↓
4. Schema定義 (stateSchema, configSchema, commandSchema)
      ↓
5. SDK validation追加
      ↓
6. ファームウェア実装
      ↓
7. E2Eテスト実施
      ↓
8. リリース
```

### 4.2 Registration Request Format

```json
{
  "request": "register-type",
  "type": {
    "name": "ISMS_ar-is07",
    "displayName": "New Sensor Device",
    "description": "新しいセンサーデバイスの説明",
    "productType": "007",
    "productCode": "0001"
  },
  "schemas": {
    "stateSchema": {
      "type": "object",
      "properties": {
        "value": { "type": "number" }
      },
      "required": ["value"]
    },
    "configSchema": {
      "type": "object",
      "properties": {
        "interval": { "type": "integer", "minimum": 1000 }
      }
    }
  },
  "capabilities": ["sensor", "battery"],
  "semanticTags": ["温度", "湿度"]
}
```

### 4.3 CLI Registration

```bash
# Type登録申請
aranea-sdk register-type \
  --name ISMS_ar-is07 \
  --display-name "New Sensor Device" \
  --product-type 007 \
  --schema-file ./schemas/is07.json

# 登録状況確認
aranea-sdk check-type ISMS_ar-is07
```

---

## 5. Type Definitions

### 5.1 ISMS_ar-is01 (温湿度センサー)

```yaml
name: ISMS_ar-is01
displayName: Temperature & Humidity Sensor
description: 電池式温湿度センサー（BLEアドバタイズ）
productType: "001"
productCodes: ["0001", "0096"]

features:
  - BLE Advertise (送信のみ)
  - DeepSleep運用
  - CR2032 × 1
  - OTA不可

stateSchema:
  Temperature:
    type: number
    unit: "°C"
    range: [-20, 60]
  Humidity:
    type: number
    unit: "%"
    range: [0, 100]
  BatteryLevel:
    type: integer
    unit: "%"
    range: [0, 100]

configSchema:
  measureInterval:
    type: integer
    default: 60000
    unit: "ms"

capabilities:
  - temperature
  - humidity
  - battery

semanticTags:
  - 温度
  - 湿度
  - 電池
  - BLE
```

### 5.2 ISMS_ar-is04a (接点コントローラー)

```yaml
name: ISMS_ar-is04a
displayName: Window & Door Controller
description: 2ch物理入力 + 2ch接点出力コントローラー
productType: "004"
productCodes: ["0001", "0096"]

features:
  - 2ch GPIO入力 (デバウンス付き)
  - 2ch GPIO出力 (パルス/持続)
  - トリガーアサイン
  - インターロック機能
  - Web UI
  - OTA対応

stateSchema:
  output1:
    type: object
    properties:
      active:
        type: boolean
      updatedAt:
        type: string
        format: date-time
  output2:
    type: object
    properties:
      active:
        type: boolean
      updatedAt:
        type: string
        format: date-time
  input1:
    type: object
    properties:
      active:
        type: boolean
      updatedAt:
        type: string
        format: date-time
  input2:
    type: object
    properties:
      active:
        type: boolean
      updatedAt:
        type: string
        format: date-time

configSchema:
  pulseMs:
    type: integer
    default: 3000
    range: [100, 60000]
  interlockMs:
    type: integer
    default: 200
    range: [0, 10000]
  debounceMs:
    type: integer
    default: 50
    range: [5, 10000]
  triggerAssign:
    type: object
    properties:
      input1:
        type: integer
        enum: [0, 1, 2]
      input2:
        type: integer
        enum: [0, 1, 2]

commandSchema:
  pulse:
    params:
      output:
        type: integer
        enum: [1, 2]
      durationMs:
        type: integer
        minimum: 100

capabilities:
  - output
  - input
  - pulse
  - trigger

semanticTags:
  - 窓
  - ドア
  - 接点出力
  - 物理入力
```

### 5.3 ISMS_ar-is05a (8ch検出器)

```yaml
name: ISMS_ar-is05a
displayName: 8-Channel Detector
description: 8ch入力検出器（ch7-8はI/O切替可能）
productType: "005"
productCodes: ["0001", "0096"]

features:
  - 8ch GPIO入力
  - ch7/ch8 出力切替可能
  - Webhook通知
  - トリガールール
  - Web UI
  - OTA対応

stateSchema:
  channels:
    type: array
    items:
      type: object
      properties:
        ch:
          type: integer
          range: [1, 8]
        active:
          type: boolean
        name:
          type: string
        updatedAt:
          type: string
          format: date-time

configSchema:
  channels:
    type: array
    maxItems: 8
    items:
      type: object
      properties:
        ch:
          type: integer
        pin:
          type: integer
        name:
          type: string
        meaning:
          type: string
          enum: ["open", "close"]
        debounceMs:
          type: integer
        isOutput:
          type: boolean
  webhook:
    type: object
    properties:
      discord:
        type: string
        format: uri
      slack:
        type: string
        format: uri
      generic:
        type: string
        format: uri
  rules:
    type: array
    maxItems: 8
    items:
      $ref: "#/definitions/Rule"

definitions:
  Rule:
    type: object
    properties:
      enabled:
        type: boolean
      srcMask:
        type: integer
      outMask:
        type: integer
      pulseMs:
        type: integer
      webhookMask:
        type: integer
      stateCond:
        type: integer
        enum: [0, 1, 2]
      cooldownMs:
        type: integer

capabilities:
  - detector
  - webhook
  - output
  - rules

semanticTags:
  - 検出
  - 窓
  - ドア
  - Webhook
```

---

## 6. Firestore typeSettings Structure

### 6.1 Collection Path

```
typeSettings/araneaDevice/{type}
```

### 6.2 Document Structure

```typescript
interface TypeSettings {
  // Metadata
  displayName: string;
  description: string;
  version: number;
  createdAt: Timestamp;
  updatedAt: Timestamp;

  // Product Info
  productType: string;
  productCodes: string[];

  // Schemas
  stateSchema: JSONSchema;
  configSchema: JSONSchema;
  commandSchema?: JSONSchema;

  // Capabilities & Tags
  capabilities: string[];
  semanticTags: string[];

  // Features
  features: string[];

  // Defaults
  defaultConfig: Record<string, any>;
}
```

---

## 7. Type Update Process

### 7.1 Schema Versioning

```
Version Format: MAJOR.MINOR.PATCH

MAJOR: 破壊的変更 (必須フィールド追加/削除)
MINOR: 互換性のある機能追加 (オプションフィールド追加)
PATCH: バグ修正、説明文変更
```

### 7.2 Update Request

```json
{
  "request": "update-type",
  "type": "ISMS_ar-is04a",
  "changes": {
    "version": "2.0.0",
    "stateSchema": {
      "properties": {
        "output3": {
          "type": "object",
          "properties": {
            "active": { "type": "boolean" }
          }
        }
      }
    }
  },
  "migration": {
    "required": true,
    "script": "migrate-is04a-v2.js"
  }
}
```

### 7.3 Backward Compatibility Rules

1. **必須フィールドの追加**: 禁止 (MAJOR変更)
2. **オプションフィールドの追加**: 許可 (MINOR変更)
3. **フィールドの削除**: deprecated→次版で削除
4. **型の変更**: 禁止 (新フィールドで対応)

---

## 8. Type Lookup API

### 8.1 Get Type Info

```bash
# CLI
aranea-sdk get-type ISMS_ar-is04a

# API
GET /api/typeSettings/araneaDevice/ISMS_ar-is04a
```

### 8.2 List All Types

```bash
# CLI
aranea-sdk list-types --domain araneaDevice

# API
GET /api/typeSettings/araneaDevice
```

### 8.3 Response

```json
{
  "types": [
    {
      "name": "ISMS_ar-is01",
      "displayName": "Temperature & Humidity Sensor",
      "productType": "001",
      "version": "1.0.0",
      "status": "active"
    },
    {
      "name": "ISMS_ar-is04a",
      "displayName": "Window & Door Controller",
      "productType": "004",
      "version": "1.2.0",
      "status": "active"
    }
  ]
}
```

---

## 9. ESP32 Type Constants

### 9.1 AraneaTypes.h

```cpp
#ifndef ARANEA_TYPES_H
#define ARANEA_TYPES_H

namespace araneaTypes {
  // ProductType
  constexpr const char* PT_IS01 = "001";
  constexpr const char* PT_IS02 = "002";
  constexpr const char* PT_IS03 = "003";
  constexpr const char* PT_IS04 = "004";
  constexpr const char* PT_IS05 = "005";
  constexpr const char* PT_IS06 = "006";
  constexpr const char* PT_IS10 = "010";
  constexpr const char* PT_IS20 = "020";

  // Type Name
  constexpr const char* TYPE_IS01 = "ISMS_ar-is01";
  constexpr const char* TYPE_IS02 = "ISMS_ar-is02";
  constexpr const char* TYPE_IS03 = "ISMS_ar-is03";
  constexpr const char* TYPE_IS04A = "ISMS_ar-is04a";
  constexpr const char* TYPE_IS05A = "ISMS_ar-is05a";
  constexpr const char* TYPE_IS06A = "ISMS_ar-is06a";
  constexpr const char* TYPE_IS10 = "ISMS_ar-is10";
  constexpr const char* TYPE_IS20S = "ISMS_ar-is20s";

  // ProductCode
  constexpr const char* PC_STANDARD = "0001";
  constexpr const char* PC_ISMS = "0096";
}

#endif
```

---

## 10. Validation Integration

### 10.1 Type Validation in deviceStateReport

```typescript
// mobes2.0 Backend
async function validateStateReport(report: DeviceStateReport): Promise<ValidationResult> {
  // 1. Type存在確認
  const typeSettings = await getTypeSettings(report.report.type);
  if (!typeSettings) {
    return { valid: false, error: 'UNKNOWN_TYPE' };
  }

  // 2. State スキーマ検証
  const stateValid = validateAgainstSchema(
    report.report.state,
    typeSettings.stateSchema
  );

  if (!stateValid.valid) {
    return { valid: false, error: 'STATE_SCHEMA_INVALID', details: stateValid.errors };
  }

  return { valid: true };
}
```

### 10.2 Type Validation in araneaDeviceGate

```typescript
// mobes2.0 Backend
async function validateRegistration(req: RegistrationRequest): Promise<ValidationResult> {
  // 1. Type存在確認
  const typeSettings = await getTypeSettings(req.userObject.type);
  if (!typeSettings) {
    return { valid: false, error: 'UNKNOWN_TYPE' };
  }

  // 2. ProductType整合性
  const expectedProductType = typeSettings.productType;
  if (req.deviceMeta.productType !== expectedProductType) {
    return { valid: false, error: 'PRODUCT_TYPE_MISMATCH' };
  }

  // 3. ProductCode許可確認
  if (!typeSettings.productCodes.includes(req.deviceMeta.productCode)) {
    return { valid: false, error: 'PRODUCT_CODE_NOT_ALLOWED' };
  }

  return { valid: true };
}
```
