# AraneaSDK Validation Tools

## 1. Overview

AraneaSDKは、開発・テスト・運用のための検証ツールを提供します。

| Tool | Purpose | Target |
|------|---------|--------|
| Schema Validator | スキーマ検証 | 両側 |
| Auth Tester | 認証テスト | 両側 |
| Payload Checker | ペイロード検証 | ファーム側 |
| E2E Test Suite | 統合テスト | 両側 |
| LacisID Validator | LacisID形式検証 | 両側 |
| CIC Generator | CIC生成（テスト用） | mobes側 |

---

## 2. Schema Validator

### 2.1 Purpose

JSON Schemaに基づいてペイロードの正当性を検証します。

### 2.2 CLI Usage

```bash
# userObject検証
aranea-sdk validate-schema \
  --schema userObject \
  --input ./payload.json

# deviceStateReport検証
aranea-sdk validate-schema \
  --schema deviceStateReport \
  --input ./report.json

# Type別state検証
aranea-sdk validate-schema \
  --schema state \
  --type ISMS_ar-is04a \
  --input ./state.json
```

### 2.3 Node.js API

```typescript
import { SchemaValidator } from '@aranea-sdk/validator';

const validator = new SchemaValidator();

// userObject検証
const result = validator.validateUserObject({
  lacis: {
    type_domain: 'araneaDevice',
    type: 'ISMS_ar-is04a',
    tid: 'T2025120608261484221',
    permission: 10,
    cic_code: '123456',
    cic_active: true
  },
  activity: {
    created_at: '2025-01-01T00:00:00Z',
    lastUpdated_at: '2025-01-01T00:00:00Z'
  },
  device: {
    macAddress: '0123456789AB',
    productType: '004',
    productCode: '0001'
  }
});

if (!result.valid) {
  console.error('Validation errors:', result.errors);
}
```

### 2.4 ESP32 Validation (Compile-time)

```cpp
// AraneaPayloadValidator.h
class AraneaPayloadValidator {
public:
  static bool validateLacisId(const String& lacisId);
  static bool validateCic(const String& cic);
  static bool validateTid(const String& tid);
  static bool validateMac(const String& mac);

  // 使用例
  // if (!AraneaPayloadValidator::validateLacisId(id)) {
  //   Serial.println("Invalid LacisID");
  // }
};

bool AraneaPayloadValidator::validateLacisId(const String& lacisId) {
  if (lacisId.length() != 20) return false;
  if (lacisId[0] != '3') return false;

  // ProductType (1-3)
  for (int i = 1; i < 4; i++) {
    if (!isDigit(lacisId[i])) return false;
  }

  // MAC (4-15)
  for (int i = 4; i < 16; i++) {
    char c = lacisId[i];
    if (!isHexadecimalDigit(c)) return false;
  }

  // ProductCode (16-19)
  for (int i = 16; i < 20; i++) {
    if (!isDigit(lacisId[i])) return false;
  }

  return true;
}
```

---

## 3. Auth Tester

### 3.1 Purpose

認証フローの動作確認を行います。

### 3.2 CLI Usage

```bash
# Device Auth テスト
aranea-sdk test-auth device \
  --tid T2025120608261484221 \
  --lacisId 30040123456789AB0001 \
  --cic 123456

# LacisOath テスト
aranea-sdk test-auth lacis-oath \
  --primary-lacisId 12767487939173857894 \
  --email info+ichiyama@neki.tech \
  --cic 263238

# Firebase Auth テスト
aranea-sdk test-auth firebase \
  --token eyJhbGciOiJSUzI1NiIs...
```

### 3.3 Output Example

```json
{
  "test": "device-auth",
  "result": "PASS",
  "details": {
    "lacisIdFormat": "VALID",
    "cicFormat": "VALID",
    "tidFormat": "VALID",
    "firestoreCheck": "FOUND",
    "cicMatch": "VALID",
    "cicActive": true,
    "permission": 10
  },
  "duration": "234ms"
}
```

### 3.4 mobes2.0 Integration

```typescript
// functions/src/araneaDevice/testAuth.ts
export async function testDeviceAuth(
  tid: string,
  lacisId: string,
  cic: string
): Promise<AuthTestResult> {
  const result: AuthTestResult = {
    test: 'device-auth',
    result: 'PASS',
    details: {}
  };

  // Format validation
  result.details.lacisIdFormat = validateLacisIdFormat(lacisId) ? 'VALID' : 'INVALID';
  result.details.cicFormat = /^\d{6}$/.test(cic) ? 'VALID' : 'INVALID';
  result.details.tidFormat = validateTidFormat(tid) ? 'VALID' : 'INVALID';

  if (result.details.lacisIdFormat === 'INVALID' ||
      result.details.cicFormat === 'INVALID' ||
      result.details.tidFormat === 'INVALID') {
    result.result = 'FAIL';
    return result;
  }

  // Firestore check
  const userObject = await db.collection('userObject').doc(lacisId).get();
  if (!userObject.exists) {
    result.details.firestoreCheck = 'NOT_FOUND';
    result.result = 'FAIL';
    return result;
  }

  result.details.firestoreCheck = 'FOUND';
  const data = userObject.data();

  // CIC validation
  result.details.cicMatch = data.lacis.cic_code === cic ? 'VALID' : 'INVALID';
  result.details.cicActive = data.lacis.cic_active !== false;
  result.details.permission = data.lacis.permission;

  if (result.details.cicMatch === 'INVALID' || !result.details.cicActive) {
    result.result = 'FAIL';
  }

  return result;
}
```

---

## 4. Payload Checker

### 4.1 Purpose

デバイスから送信されるペイロードの形式・内容を検証します。

### 4.2 CLI Usage

```bash
# deviceStateReport ペイロード検証
aranea-sdk check-payload \
  --type deviceStateReport \
  --file ./payload.json

# Type別state検証
aranea-sdk check-payload \
  --type state \
  --device-type ISMS_ar-is04a \
  --file ./state.json

# ライブキャプチャ
aranea-sdk capture-payload \
  --device 192.168.1.100 \
  --port 80 \
  --duration 60
```

### 4.3 Validation Rules

```typescript
interface PayloadCheckResult {
  valid: boolean;
  checks: {
    structure: CheckResult;
    auth: CheckResult;
    state: CheckResult;
    timestamp: CheckResult;
    meta: CheckResult;
  };
  warnings: string[];
  errors: string[];
}

interface CheckResult {
  status: 'PASS' | 'WARN' | 'FAIL';
  message: string;
  details?: any;
}
```

### 4.4 Example Output

```json
{
  "valid": true,
  "checks": {
    "structure": {
      "status": "PASS",
      "message": "All required fields present"
    },
    "auth": {
      "status": "PASS",
      "message": "Auth object valid",
      "details": {
        "tid": "T2025120608261484221",
        "lacisId": "30040123456789AB0001"
      }
    },
    "state": {
      "status": "PASS",
      "message": "State matches ISMS_ar-is04a schema"
    },
    "timestamp": {
      "status": "WARN",
      "message": "Timestamp is 5 minutes old"
    },
    "meta": {
      "status": "PASS",
      "message": "Meta fields valid"
    }
  },
  "warnings": [
    "Timestamp drift detected: 5 minutes"
  ],
  "errors": []
}
```

---

## 5. E2E Test Suite

### 5.1 Purpose

エンドツーエンドの統合テストを実行します。

### 5.2 Test Scenarios

```yaml
# e2e-tests/scenarios/device-registration.yaml
name: Device Registration Flow
steps:
  - name: Generate LacisID
    action: generate-lacisid
    params:
      productType: "004"
      productCode: "0001"
    expect:
      length: 20
      prefix: "3"

  - name: Register Device
    action: call-api
    params:
      endpoint: /araneaDeviceGate
      method: POST
      body:
        lacisOath:
          lacisId: ${TENANT_PRIMARY_LACISID}
          userId: ${TENANT_PRIMARY_EMAIL}
          cic: ${TENANT_PRIMARY_CIC}
          method: register
        userObject:
          lacisID: ${generated_lacisid}
          tid: ${TID}
          typeDomain: araneaDevice
          type: ISMS_ar-is04a
        deviceMeta:
          macAddress: ${MAC}
          productType: "004"
          productCode: "0001"
    expect:
      status: 200
      body:
        ok: true
        userObject:
          cic_code: /^\d{6}$/

  - name: Verify Firestore
    action: check-firestore
    params:
      collection: userObject
      document: ${generated_lacisid}
    expect:
      exists: true
      fields:
        lacis.type: ISMS_ar-is04a
        lacis.tid: ${TID}
```

### 5.3 CLI Usage

```bash
# 全テスト実行
aranea-sdk e2e-test --all

# 特定シナリオ実行
aranea-sdk e2e-test --scenario device-registration

# 環境指定
aranea-sdk e2e-test \
  --env production \
  --scenario state-report

# レポート出力
aranea-sdk e2e-test --all --report ./test-report.html
```

### 5.4 Test Environment Configuration

```json
// e2e-tests/env/test.json
{
  "TID": "T2025120608261484221",
  "TENANT_PRIMARY_LACISID": "12767487939173857894",
  "TENANT_PRIMARY_EMAIL": "info+ichiyama@neki.tech",
  "TENANT_PRIMARY_CIC": "263238",
  "GATE_URL": "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate",
  "STATE_URL": "https://us-central1-mobesorder.cloudfunctions.net/deviceStateReport",
  "TEST_MAC": "TESTMAC12345"
}
```

---

## 6. LacisID Validator

### 6.1 CLI Usage

```bash
# 単一検証
aranea-sdk validate-lacisid 30040123456789AB0001

# バッチ検証
aranea-sdk validate-lacisid --file ./lacisids.txt

# 生成
aranea-sdk generate-lacisid \
  --product-type 004 \
  --mac 0123456789AB \
  --product-code 0001
```

### 6.2 Output

```json
{
  "lacisId": "30040123456789AB0001",
  "valid": true,
  "parsed": {
    "prefix": "3",
    "productType": "004",
    "mac": "0123456789AB",
    "productCode": "0001"
  },
  "deviceInfo": {
    "type": "is04",
    "name": "Window Controller"
  }
}
```

---

## 7. Communication Test Tool

### 7.1 Purpose

デバイスとクラウド間の通信をテストします。

### 7.2 CLI Usage

```bash
# HTTP通信テスト
aranea-sdk test-comm http \
  --endpoint https://us-central1-mobesorder.cloudfunctions.net/deviceStateReport \
  --payload ./test-payload.json

# MQTT通信テスト
aranea-sdk test-comm mqtt \
  --broker wss://aranea-mqtt-bridge-xxx.run.app \
  --lacisId 30040123456789AB0001 \
  --cic 123456 \
  --topic aranea/30040123456789AB0001/state

# ローカルリレーテスト
aranea-sdk test-comm local-relay \
  --primary 192.168.96.201:8080 \
  --secondary 192.168.96.202:8080 \
  --payload ./test-payload.json
```

### 7.3 Output

```json
{
  "test": "http-communication",
  "endpoint": "https://us-central1-mobesorder.cloudfunctions.net/deviceStateReport",
  "result": "PASS",
  "metrics": {
    "dnsLookup": "23ms",
    "tcpConnect": "45ms",
    "tlsHandshake": "120ms",
    "serverResponse": "234ms",
    "total": "422ms"
  },
  "response": {
    "status": 200,
    "body": {
      "ok": true
    }
  }
}
```

---

## 8. Firestore Data Checker

### 8.1 Purpose

Firestoreのデータ整合性をチェックします。

### 8.2 CLI Usage

```bash
# userObject整合性チェック
aranea-sdk check-firestore userObject \
  --lacisId 30040123456789AB0001

# 全デバイス一括チェック
aranea-sdk check-firestore userObject \
  --tid T2025120608261484221 \
  --all

# araneaDeviceStates同期チェック
aranea-sdk check-firestore sync \
  --lacisId 30040123456789AB0001
```

### 8.3 Checks Performed

```typescript
interface FirestoreCheckResult {
  collection: string;
  document: string;
  checks: {
    exists: boolean;
    schemaValid: boolean;
    referenceIntegrity: boolean;
    dataConsistency: boolean;
  };
  issues: string[];
  suggestions: string[];
}
```

---

## 9. mobes2.0 AI Model Integration

### 9.1 Purpose

mobes2.0のAIモデルを活用したデータ整合性チェック。

### 9.2 Features

- 異常値検出
- パターン認識
- 予測的メンテナンス

### 9.3 API

```typescript
// mobes2.0側で提供されるAPI
interface AIValidationAPI {
  // 状態データの異常検出
  detectAnomalies(
    lacisId: string,
    stateHistory: StateRecord[]
  ): Promise<AnomalyReport>;

  // スキーマ推論
  inferSchema(
    samples: StateRecord[]
  ): Promise<InferredSchema>;

  // データ品質スコア
  calculateDataQuality(
    lacisId: string,
    timeRange: TimeRange
  ): Promise<DataQualityScore>;
}
```

---

## 10. SDK Installation

### 10.1 npm Package

```bash
npm install @aranea-sdk/tools
```

### 10.2 CLI Installation

```bash
npm install -g @aranea-sdk/cli
```

### 10.3 Configuration

```json
// ~/.aranea-sdk/config.json
{
  "defaultEnvironment": "production",
  "environments": {
    "production": {
      "gateUrl": "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate",
      "stateUrl": "https://us-central1-mobesorder.cloudfunctions.net/deviceStateReport"
    },
    "staging": {
      "gateUrl": "https://us-central1-mobesorder-staging.cloudfunctions.net/araneaDeviceGate",
      "stateUrl": "https://us-central1-mobesorder-staging.cloudfunctions.net/deviceStateReport"
    }
  },
  "credentials": {
    "tenant": {
      "lacisId": "12767487939173857894",
      "email": "info+ichiyama@neki.tech",
      "cic": "263238"
    }
  }
}
```
