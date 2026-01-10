# AraneaRegister/LacisOath認証要件

作成日: 2026-01-10

## 1. 概要

Paraclate実装においてis22にAraneaRegister機能を実装する必要がある。
ESP32版AraneaRegister.cppおよびis21版aranea_register.pyを参照実装とする。

---

## 2. LacisID仕様

### 2.1 形式

```
[Prefix=3][ProductType=3桁][MAC=12桁][ProductCode=4桁] = 20文字
```

### 2.2 is22用パラメータ

| 項目 | 値 | 根拠 |
|------|-----|------|
| TypeDomain | araneaDevice | araneaDevice共通（単数形が正） |
| Type | ar-is22CamServer | Paraclate設計書 |
| Prefix | 3 | araneaDevice共通 |
| ProductType | 022 | is22用 |
| ProductCode | 0000 | 追い番なし |

### 2.3 例

```
3022C0742BFCCF950000
│││ │          ││││
│││ └──────────┴┴┴┴─ ProductCode (0000)
│││            MAC (C0742BFCCF95)
││└─ ProductType (022)
└┴── Prefix (3)
```

---

## 3. LacisOath認証構造

### 3.1 認証リクエスト形式

```json
{
  "lacisOath": {
    "lacisId": "[テナントプライマリlacisId]",
    "userId": "[テナントプライマリuserId]",
    "cic": "[テナントプライマリCIC]",
    "method": "register"
  },
  "userObject": {
    "lacisID": "[デバイスlacisId]",
    "tid": "[テナントID]",
    "typeDomain": "araneaDevice",
    "type": "[デバイスタイプ]"
  },
  "deviceMeta": {
    "macAddress": "[MACアドレス]",
    "productType": "[3桁]",
    "productCode": "[4桁]"
  }
}
```

### 3.2 テナント情報（移行後）

Paraclate_DesignOverview.mdより、以下へ移行:

```
mijeo.inc
- TID: T2025120621041161827
- テナントプライマリ: soejim@mijeos.com
- テナントプライマリlacisID: 18217487937895888001
- テナントプライマリCIC: 204965
```

---

## 4. AraneaRegisterフロー

### 4.1 ESP32版（参照実装）

```cpp
// 1. 既登録チェック
if (isRegistered()) {
  return getSavedCic();  // NVSから取得
}

// 2. JSON構築
JsonObject lacisOath = doc.createNestedObject("lacisOath");
lacisOath["lacisId"] = tenantPrimary_.lacisId;
lacisOath["userId"] = tenantPrimary_.userId;
lacisOath["cic"] = tenantPrimary_.cic;
lacisOath["method"] = "register";

// 3. HTTP POST
HTTPClient http;
http.begin(gateUrl_);
http.addHeader("Content-Type", "application/json");
int httpCode = http.POST(payload);

// 4. レスポンス解析・NVS保存
if (httpCode == 201 || httpCode == 200) {
  // CIC保存
  prefs.putString(KEY_CIC, cic_code);
  prefs.putBool(KEY_REGISTERED, true);
}
```

### 4.2 is22実装方針（Rust版）

```rust
// 1. 設定テーブルから既登録チェック
let registered = config_store.get("aranea_registered").await?;
if registered {
    return config_store.get("aranea_cic").await;
}

// 2. リクエスト構築
let payload = AraneaRegisterRequest {
    lacis_oath: LacisOath {
        lacis_id: tenant_primary.lacis_id.clone(),
        user_id: tenant_primary.user_id.clone(),
        cic: tenant_primary.cic.clone(),
        method: "register".to_string(),
    },
    user_object: UserObject {
        lacis_id: device_lacis_id.clone(),
        tid: tid.clone(),
        type_domain: "araneaDevice".to_string(),
        device_type: "ar-is22CamServer".to_string(),
    },
    device_meta: DeviceMeta {
        mac_address: mac.clone(),
        product_type: "022".to_string(),
        product_code: "0000".to_string(),
    },
};

// 3. HTTP POST
let response = client.post(ARANEA_DEVICE_GATE_URL)
    .json(&payload)
    .send()
    .await?;

// 4. レスポンス解析・DB保存
config_store.set("aranea_cic", &response.cic_code).await?;
config_store.set("aranea_registered", true).await?;
```

---

## 5. is22に必要な実装

### 5.1 新規スキーマ

```sql
-- migration: aranea_registration
CREATE TABLE aranea_registration (
    id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    lacis_id VARCHAR(20) NOT NULL UNIQUE,
    tid VARCHAR(32) NOT NULL,
    cic VARCHAR(6) NOT NULL,
    state_endpoint VARCHAR(255),
    mqtt_endpoint VARCHAR(255),
    registered_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    last_sync_at DATETIME(3)
);
```

### 5.2 新規APIエンドポイント

| エンドポイント | メソッド | 機能 |
|---------------|---------|------|
| POST /api/register/device | POST | araneaDeviceGate登録 |
| GET /api/register/status | GET | 登録状態確認 |
| DELETE /api/register | DELETE | 登録クリア（再登録用） |

### 5.3 新規モジュール

```
src/aranea_register/
├── mod.rs           # モジュール定義
├── types.rs         # LacisOath, UserObject, DeviceMeta
├── lacis_id.rs      # LacisID生成
└── gate_client.rs   # araneaDeviceGate HTTP通信
```

---

## 6. TID/FID権限管理

### 6.1 権限境界

```
TID (テナントID)
└── FID (施設ID) × N
    └── カメラ (is801 paraclateCamera) × N
```

### 6.2 is22の権限

- is22はそのTIDが持つ全FIDのアクセス権限を持つ
- 傘下カメラはFIDに所属

### 6.3 カメラのLacisID（is801）

```
形式: 3801{MAC}{ProductCode=カメラブランド割振}
例:   3801C0742BFCCF950001
```

---

## 7. Blessing（テナント越境権限）

Paraclate_DesignOverview.mdより:

> blessing: permission91以上のユーザーの権限において特定のlacisIDに対して
> tid権限越境の権能を付与する。このblessによってis22はテナント越境した
> カメラへのアクセス権を得ることができる。

**is22での使用**: テナント越境監視を行う場合のみ使用（通常運用では不要）

---

## 8. araneaDeviceGateエンドポイント

```
https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate
```

**注意**: 404エラー報告あり、本番URL要確認

---

## 9. MECE確認

- **網羅性**: LacisID、LacisOath、AraneaRegister、TID/FID権限を全調査
- **重複排除**: ESP32/is21/is22の実装パターンを分離して記述
- **未カバー領域**: araneaDeviceGate本番URLの確認は範囲外
