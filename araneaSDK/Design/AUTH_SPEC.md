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
