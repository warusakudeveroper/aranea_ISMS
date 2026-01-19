# AraneaSDK Authentication Specification

## 1. Overview

araneaDeviceシステムでは、3種類の認証方式を用途に応じて使い分けます。

| 認証方式 | 用途 | 認証要素 |
|---------|------|---------|
| Device Auth | デバイス→クラウド通信 | tid + lacisId + cic |
| LacisOath | デバイス登録 | lacisId + userId + cic |
| Firebase Auth | UI/管理API | Firebase ID Token |

---

## 2. LacisID Specification

### 2.1 Format

```
[Prefix][ProductType][MAC][ProductCode]
   1文字    3文字     12文字   4文字    = 20文字固定

Prefix: 固定値 "3" (araneaDevice)
ProductType: 001-999 (デバイス種別)
MAC: 12桁HEX (WiFi STA MAC、大文字)
ProductCode: 0001-9999 (製品バリエーション)
```

### 2.2 Generation (ESP32)

```cpp
// LacisIDGenerator.cpp
String LacisIDGenerator::generate(const String& productType,
                                   const String& productCode) {
  // WiFi STA MACを取得 (12桁HEX、大文字)
  String mac12hex = getStaMac12Hex();

  // LacisID = "3" + productType(3桁) + MAC(12桁) + productCode(4桁)
  return "3" + productType + mac12hex + productCode;
}

String LacisIDGenerator::getStaMac12Hex() {
  uint8_t mac[6];
  WiFi.macAddress(mac);
  char buf[13];
  snprintf(buf, sizeof(buf), "%02X%02X%02X%02X%02X%02X",
           mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
  return String(buf);
}
```

### 2.3 Validation Rules

```typescript
// mobes2.0 側検証
function validateLacisId(lacisId: string): boolean {
  // 1. 長さチェック
  if (lacisId.length !== 20) return false;

  // 2. Prefix チェック (araneaDevice = "3")
  if (!lacisId.startsWith('3')) return false;

  // 3. 数字のみ（または16進数部分）
  if (!/^\d{20}$/.test(lacisId) &&
      !/^3\d{3}[0-9A-F]{12}\d{4}$/i.test(lacisId)) {
    return false;
  }

  return true;
}
```

---

## 3. CIC (Credential Identification Code)

### 3.1 Format

```
6桁の数字文字列: "000000" - "999999"
```

### 3.2 Generation (mobes2.0 Backend)

```typescript
// functions/src/araneaDevice/auth.ts
function generateCIC(): string {
  const randomNum = Math.floor(Math.random() * 1000000);
  return randomNum.toString().padStart(6, '0');
}
```

### 3.3 Storage

**Firestore (userObject)**:
```json
{
  "lacis": {
    "cic_code": "123456",
    "cic_active": true
  }
}
```

**ESP32 NVS**:
```cpp
// SettingManager経由で保存
settings.setString("cic", "123456");
```

### 3.4 Lifecycle

1. **発行**: araneaDeviceGate で新規登録時に発行
2. **保存**: デバイス側NVS + Firestore userObject
3. **使用**: 全ての認証済みリクエストに含める
4. **無効化**: cic_active = false で無効化可能
5. **再発行**: Tenant Primary が再登録で再発行可能

---

## 4. Device Authentication

### 4.1 Auth Object Format

```typescript
interface DeviceAuth {
  tid: string;      // Tenant ID: T{19桁} or T{12桁}{7文字} or {20桁数字}
  lacisId: string;  // 20桁固定
  cic: string;      // 6桁数字
}
```

### 4.2 Request Example

```json
{
  "auth": {
    "tid": "T2025120608261484221",
    "lacisId": "30040123456789AB0001",
    "cic": "123456"
  },
  "report": {
    "type": "ISMS_ar-is04a",
    "state": { ... }
  }
}
```

### 4.3 Validation Flow (Backend)

```typescript
async function validateDeviceAuth(auth: DeviceAuth): Promise<ValidationResult> {
  // 1. Format validation
  if (!validateLacisId(auth.lacisId)) {
    return { valid: false, error: 'INVALID_LACISID_FORMAT' };
  }

  if (!/^\d{6}$/.test(auth.cic)) {
    return { valid: false, error: 'INVALID_CIC_FORMAT' };
  }

  // 2. Firestore lookup
  const userObjectRef = db.collection('userObject').doc(auth.lacisId);
  const userObject = await userObjectRef.get();

  if (!userObject.exists) {
    return { valid: false, error: 'DEVICE_NOT_REGISTERED' };
  }

  const data = userObject.data();

  // 3. TID verification
  if (data.lacis.tid !== auth.tid) {
    return { valid: false, error: 'TID_MISMATCH' };
  }

  // 4. CIC verification
  if (data.lacis.cic_code !== auth.cic) {
    return { valid: false, error: 'INVALID_CIC' };
  }

  // 5. CIC active check
  if (data.lacis.cic_active === false) {
    return { valid: false, error: 'CIC_DISABLED' };
  }

  return { valid: true, userObject: data };
}
```

### 4.4 ESP32 Implementation

```cpp
// StateReporter での認証ヘッダー設定
void StateReporter::sendReport(const String& payload) {
  HTTPClient http;
  http.begin(endpoint_);
  http.addHeader("Content-Type", "application/json");

  // ペイロードに認証情報を含める
  StaticJsonDocument<2048> doc;
  deserializeJson(doc, payload);

  doc["auth"]["tid"] = tid_;
  doc["auth"]["lacisId"] = lacisId_;
  doc["auth"]["cic"] = cic_;

  String body;
  serializeJson(doc, body);

  int httpCode = http.POST(body);
  // ...
}
```

---

## 5. LacisOath (Device Registration)

### 5.1 Auth Object Format

```typescript
interface LacisOath {
  lacisId: string;  // Tenant Primary の LacisID
  userId: string;   // Tenant Primary の Email
  cic: string;      // Tenant Primary の現在のCIC
  method: string;   // "register" | "update" | "delete"
}
```

### 5.2 Permission Requirements

- **登録実行者**: Tenant Primary (permission >= 61)
- **検証項目**:
  - lacisId が userObject に存在
  - userId が該当 lacisId のユーザー
  - cic が現在有効な CIC
  - type_domain が "User" で permission >= 61

### 5.3 Request Example

```json
{
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
}
```

### 5.4 Validation Flow (Backend)

```typescript
async function validateLacisOath(oath: LacisOath): Promise<ValidationResult> {
  // 1. Tenant Primary の userObject を取得
  const primaryRef = db.collection('userObject').doc(oath.lacisId);
  const primaryDoc = await primaryRef.get();

  if (!primaryDoc.exists) {
    return { valid: false, error: 'PRIMARY_NOT_FOUND' };
  }

  const primary = primaryDoc.data();

  // 2. Type Domain チェック (User であること)
  if (primary.lacis.type_domain !== 'User') {
    return { valid: false, error: 'NOT_USER_TYPE' };
  }

  // 3. Permission チェック (61以上)
  if (primary.lacis.permission < 61) {
    return { valid: false, error: 'INSUFFICIENT_PERMISSION' };
  }

  // 4. CIC 検証
  if (primary.lacis.cic_code !== oath.cic) {
    return { valid: false, error: 'INVALID_CIC' };
  }

  // 5. Email (userId) 検証
  // Firebase Auth または userObject から確認
  if (!await verifyUserEmail(oath.lacisId, oath.userId)) {
    return { valid: false, error: 'EMAIL_MISMATCH' };
  }

  return { valid: true, tenantId: primary.lacis.tid };
}
```

### 5.5 ESP32 Implementation

```cpp
// AraneaRegister.cpp
bool AraneaRegister::registerDevice() {
  StaticJsonDocument<1024> doc;

  // lacisOath (Tenant Primary 認証)
  doc["lacisOath"]["lacisId"] = tenantPrimaryLacisId_;
  doc["lacisOath"]["userId"] = tenantPrimaryEmail_;
  doc["lacisOath"]["cic"] = tenantPrimaryCic_;
  doc["lacisOath"]["method"] = "register";

  // userObject (登録するデバイス情報)
  doc["userObject"]["lacisID"] = deviceLacisId_;
  doc["userObject"]["tid"] = tid_;
  doc["userObject"]["typeDomain"] = "araneaDevice";
  doc["userObject"]["type"] = deviceType_;

  // deviceMeta
  doc["deviceMeta"]["macAddress"] = mac12hex_;
  doc["deviceMeta"]["productType"] = productType_;
  doc["deviceMeta"]["productCode"] = productCode_;

  String payload;
  serializeJson(doc, payload);

  HTTPClient http;
  http.begin(gateUrl_);
  http.addHeader("Content-Type", "application/json");

  int httpCode = http.POST(payload);

  if (httpCode == 200) {
    // Response から CIC を取得して保存
    String response = http.getString();
    StaticJsonDocument<512> resDoc;
    deserializeJson(resDoc, response);

    if (resDoc["ok"].as<bool>()) {
      String cic = resDoc["userObject"]["cic_code"].as<String>();
      settings_->setString("cic", cic);
      return true;
    }
  }

  return false;
}
```

---

## 6. Firebase Authentication (UI/API)

### 6.1 Token Format

```
Authorization: Bearer <Firebase ID Token>
```

### 6.2 Permission Mapping

| Permission | Role | Capabilities |
|-----------|------|--------------|
| 10+ | Staff | 状態表示のみ |
| 41+ | Manager | 設定変更可 |
| 61+ | Tenant Primary | デバイス登録・コマンド発行 |
| 71+ | System Authority | クロステナント操作 |
| 100 | Lacis | システムSU |

### 6.3 Backend Validation

```typescript
async function validateFirebaseAuth(req: Request): Promise<AuthResult> {
  const authHeader = req.headers.authorization;

  if (!authHeader?.startsWith('Bearer ')) {
    throw new Error('NO_AUTH_HEADER');
  }

  const token = authHeader.split('Bearer ')[1];

  try {
    const decoded = await admin.auth().verifyIdToken(token);
    const userObject = await getUserObject(decoded.uid);

    return {
      valid: true,
      uid: decoded.uid,
      permission: userObject.lacis.permission,
      tid: userObject.lacis.tid
    };
  } catch (error) {
    throw new Error('INVALID_TOKEN');
  }
}
```

---

## 7. MQTT Authentication

### 7.1 Connection Credentials

```
Endpoint: wss://aranea-mqtt-bridge-*.run.app
Username: {lacisId}
Password: {cic}
```

### 7.2 Topic ACL

```
# デバイスが購読可能
aranea/{lacisId}/cmd
aranea/{lacisId}/config

# デバイスが発行可能
aranea/{lacisId}/state
aranea/{lacisId}/event
```

### 7.3 ESP32 Implementation

```cpp
// MqttManager.cpp
bool MqttManager::begin(const String& url,
                        const String& lacisId,
                        const String& cic) {
  esp_mqtt_client_config_t config = {};
  config.broker.address.uri = url.c_str();
  config.credentials.username = lacisId.c_str();
  config.credentials.authentication.password = cic.c_str();

  // TLS 設定
  config.broker.verification.use_global_ca_store = true;

  client_ = esp_mqtt_client_init(&config);
  return esp_mqtt_client_start(client_) == ESP_OK;
}
```

---

## 8. Error Codes

### 8.1 Authentication Errors

| Code | Message | Cause |
|------|---------|-------|
| AUTH001 | INVALID_LACISID_FORMAT | LacisID 形式不正 |
| AUTH002 | INVALID_CIC_FORMAT | CIC 形式不正 |
| AUTH003 | DEVICE_NOT_REGISTERED | 未登録デバイス |
| AUTH004 | TID_MISMATCH | TID 不一致 |
| AUTH005 | INVALID_CIC | CIC 不一致 |
| AUTH006 | CIC_DISABLED | CIC 無効化済み |
| AUTH007 | PRIMARY_NOT_FOUND | Tenant Primary 不明 |
| AUTH008 | INSUFFICIENT_PERMISSION | 権限不足 |
| AUTH009 | EMAIL_MISMATCH | Email 不一致 |
| AUTH010 | TOKEN_EXPIRED | トークン期限切れ |

### 8.2 HTTP Response Example

```json
{
  "ok": false,
  "error": {
    "code": "AUTH005",
    "message": "INVALID_CIC",
    "details": "CIC mismatch for lacisId: 30040123456789AB0001"
  }
}
```

---

## 9. Security Best Practices

### 9.1 ESP32 Side

1. **CIC 保護**: NVSに保存、ログ出力しない
2. **HTTPS 強制**: HTTP は使用しない
3. **証明書検証**: TLS 証明書を検証する
4. **タイムアウト**: 適切なタイムアウト設定 (3-10秒)

### 9.2 Backend Side

1. **Rate Limiting**: IP/デバイス単位でレート制限
2. **Audit Logging**: 全認証試行をログ記録
3. **CIC Rotation**: 定期的なCIC更新を推奨
4. **IP Whitelist**: 必要に応じてIP制限

### 9.3 共通

1. **最小権限原則**: 必要最小限の権限のみ付与
2. **監査証跡**: 全操作をBigQueryに記録
3. **異常検知**: 連続失敗の検知とアラート

---

## 10. Paraclate API認証（LacisOath HTTPヘッダー形式）

### 10.1 概要

Paraclate（IS21/IS22 → mobes2.0 Cloud Run）間の通信では、HTTPヘッダーベースの**LacisOath認証**を使用します。
これはセクション5のLacisOath（デバイス登録用ボディベース）とは異なる形式です。

| 項目 | LacisOath (登録用) | LacisOath (Paraclate用) |
|------|-------------------|------------------------|
| 用途 | デバイス登録 | API認証 |
| 場所 | リクエストボディ | HTTPヘッダー |
| 形式 | JSON object | Authorization ヘッダー |
| 要素 | lacisId + userId + cic + method | lacisId + tid + cic + timestamp |

### 10.2 ヘッダー形式

```
Authorization: LacisOath <base64-encoded-json>
```

### 10.3 Base64エンコード前のJSON構造

```json
{
  "lacisId": "3022E051D815448B0001",
  "tid": "T2025120621041161827",
  "cic": "605123",
  "timestamp": "2026-01-10T22:54:51Z"
}
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| lacisId | string | 送信元デバイスのLacisID（20桁） |
| tid | string | テナントID（T+19桁） |
| cic | string | デバイスのCIC（6桁数字） |
| timestamp | string | リクエスト時刻（ISO8601 UTC） |

### 10.4 対象エンドポイント

| API | URL | Method |
|-----|-----|--------|
| Connect | https://paraclateconnect-vm44u3kpua-an.a.run.app | POST |
| IngestSummary | https://paraclateingestsummary-vm44u3kpua-an.a.run.app | POST |
| IngestEvent | https://paraclateingestevent-vm44u3kpua-an.a.run.app | POST |
| GetConfig | https://paraclategetconfig-vm44u3kpua-an.a.run.app/config/{tid}?fid={fid} | GET |

### 10.5 リクエストボディ形式

Ingest系APIでは以下のラッピング形式が必須：

```json
{
  "fid": "0150",
  "payload": {
    "summary": { ... },
    "periodStart": "...",
    "periodEnd": "..."
  }
}
```

### 10.6 実装例（Rust / IS22）

```rust
// src/paraclate_client/types.rs

impl LacisOath {
    /// HTTPヘッダを生成
    ///
    /// mobes2.0 Paraclate API認証形式:
    /// Authorization: LacisOath <base64-encoded-json>
    pub fn to_headers(&self) -> Vec<(String, String)> {
        use base64::Engine;

        // 認証ペイロードを構築
        let auth_payload = serde_json::json!({
            "lacisId": self.lacis_id,
            "tid": self.tid,
            "cic": self.cic,
            "timestamp": chrono::Utc::now().to_rfc3339_opts(
                chrono::SecondsFormat::Secs, true
            )
        });

        // Base64エンコード
        let json_str = serde_json::to_string(&auth_payload)
            .unwrap_or_default();
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(json_str.as_bytes());

        let mut headers = vec![
            ("Authorization".to_string(), format!("LacisOath {}", encoded)),
        ];

        // blessingがある場合は追加ヘッダとして付与（越境アクセス用）
        if let Some(blessing) = &self.blessing {
            headers.push((
                "X-Lacis-Blessing".to_string(),
                blessing.clone()
            ));
        }

        headers
    }
}
```

### 10.7 実装例（Python / IS21）

```python
# src/aranea_common/lacis_oath.py

import base64
import json
from datetime import datetime, timezone

def generate_lacis_oath_header(lacis_id: str, tid: str, cic: str) -> dict:
    """LacisOath認証ヘッダーを生成"""
    auth_payload = {
        "lacisId": lacis_id,
        "tid": tid,
        "cic": cic,
        "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    }

    json_str = json.dumps(auth_payload, separators=(',', ':'))
    encoded = base64.b64encode(json_str.encode()).decode()

    return {
        "Authorization": f"LacisOath {encoded}"
    }
```

### 10.8 実装例（Shell / テスト用）

```bash
#!/bin/bash

LACIS_ID="3022E051D815448B0001"
TID="T2025120621041161827"
CIC="605123"
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

AUTH_PAYLOAD="{\"lacisId\":\"${LACIS_ID}\",\"tid\":\"${TID}\",\"cic\":\"${CIC}\",\"timestamp\":\"${TIMESTAMP}\"}"
OAUTH_HEADER=$(echo -n "$AUTH_PAYLOAD" | base64 -w 0)

curl -X POST 'https://paraclateconnect-vm44u3kpua-an.a.run.app' \
  -H 'Content-Type: application/json' \
  -H "Authorization: LacisOath $OAUTH_HEADER" \
  -d '{"fid":"0150","payload":{"deviceType":"is22","version":"0.1.0"}}'
```

### 10.9 Backend検証フロー（mobes2.0側）

```typescript
// functions/src/paraclate/auth.ts

interface LacisOathPayload {
  lacisId: string;
  tid: string;
  cic: string;
  timestamp: string;
}

async function validateLacisOathHeader(
  authHeader: string
): Promise<ValidationResult> {
  // 1. ヘッダー形式チェック
  if (!authHeader?.startsWith('LacisOath ')) {
    return { valid: false, error: 'AUTH_FAILED', reason: 'Authorization header required' };
  }

  // 2. Base64デコード
  const base64Part = authHeader.slice('LacisOath '.length);
  let payload: LacisOathPayload;
  try {
    const jsonStr = Buffer.from(base64Part, 'base64').toString('utf-8');
    payload = JSON.parse(jsonStr);
  } catch (e) {
    return { valid: false, error: 'AUTH_FAILED', reason: 'Invalid base64 or JSON' };
  }

  // 3. タイムスタンプ検証（5分以内）
  const requestTime = new Date(payload.timestamp);
  const now = new Date();
  const diffMs = Math.abs(now.getTime() - requestTime.getTime());
  if (diffMs > 5 * 60 * 1000) {
    return { valid: false, error: 'AUTH_FAILED', reason: 'Timestamp too old' };
  }

  // 4. Firestore検証
  const userObjectRef = db.collection('userObject').doc(payload.lacisId);
  const userObject = await userObjectRef.get();

  if (!userObject.exists) {
    return { valid: false, error: 'AUTH_FAILED', reason: 'Device not registered' };
  }

  const data = userObject.data();

  // 5. TID検証
  if (data.lacis.tid !== payload.tid) {
    return { valid: false, error: 'AUTH_FAILED', reason: 'TID mismatch' };
  }

  // 6. CIC検証
  if (data.lacis.cic_code !== payload.cic) {
    return { valid: false, error: 'AUTH_FAILED', reason: 'Invalid CIC' };
  }

  // 7. CIC有効性チェック
  if (data.lacis.cic_active === false) {
    return { valid: false, error: 'AUTH_FAILED', reason: 'CIC disabled' };
  }

  return { valid: true, lacisId: payload.lacisId, tid: payload.tid };
}
```

### 10.10 エラーレスポンス形式

認証失敗時のレスポンス：

```json
{
  "error": "Unauthorized",
  "code": "AUTH_FAILED",
  "reason": "Authorization header required",
  "timestamp": "2026-01-10T22:52:01.897Z"
}
```

| エラーコード | reason | 原因 |
|-------------|--------|------|
| AUTH_FAILED | Authorization header required | ヘッダー未設定 |
| AUTH_FAILED | Invalid base64 or JSON | Base64/JSONパースエラー |
| AUTH_FAILED | Timestamp too old | timestampが5分以上前 |
| AUTH_FAILED | Device not registered | lacisIdが未登録 |
| AUTH_FAILED | TID mismatch | tidが一致しない |
| AUTH_FAILED | Invalid CIC | cicが一致しない |
| AUTH_FAILED | CIC disabled | cicが無効化されている |

### 10.11 旧形式との比較（非推奨）

**旧形式（非推奨）**:
```
X-Lacis-ID: 3022E051D815448B0001
X-Lacis-TID: T2025120621041161827
X-Lacis-CIC: 605123
```

**新形式（必須）**:
```
Authorization: LacisOath eyJsYWNpc0lkIjoiMzAyMkUwNT...
```

旧形式は2026年1月以降のエンドポイントでは**サポートされません**。

### 10.12 認証情報の保存場所

| システム | 保存先 | キー |
|---------|-------|------|
| IS22 (Rust) | MySQL settings | aranea.lacis_id, aranea.tid, aranea.cic |
| IS21 (Python) | YAML config | lacis_id, tid, cic |
| ESP32 | NVS | lacisId, tid, cic |

### 10.13 Blessing（越境アクセス）

別テナントへのアクセスが必要な場合、追加ヘッダーを使用：

```
Authorization: LacisOath <base64-json>
X-Lacis-Blessing: <blessing-token>
```

Blessingトークンは対象テナントのTenant Primaryが発行。
