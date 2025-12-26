# AraneaSDK テスト認証情報

**作成日**: 2025-12-26
**対象**: araneaDevice開発チーム

---

## 1. アカウント体系

AraneaSDKでは2種類のテストアカウントを使い分けます。

| 用途 | 説明 |
|------|------|
| **デバイス登録テスト用** | araneaDeviceGateでのデバイス登録・CIC発行テスト |
| **API開発アクセス用** | SDK CLI、deviceStateReport等のAPI呼び出し |

---

## 2. デバイス登録テスト用テナント

ESP32デバイスの `araneaDeviceGate` 登録テストに使用。

| 項目 | 値 |
|------|-----|
| **TID** | `T2025122515345664102` |
| **テナントプライマリ lacisID** | `12857487748465437705` |
| **Email (userId)** | `test@araneasdk.mobes` |
| **CIC** | `363348` |
| **用途** | デバイス登録・認証テスト |

### 使用例: araneaDeviceGate 登録

```json
{
  "lacisOath": {
    "lacisId": "12857487748465437705",
    "userId": "test@araneasdk.mobes",
    "cic": "363348",
    "method": "register"
  },
  "userObject": {
    "tid": "T2025122515345664102",
    "typeDomain": "araneaDevice",
    "type": "aranea_ar-is04a"
  },
  "deviceMeta": {
    "macAddress": "AABBCCDDEEFF",
    "productType": "004",
    "productCode": "0001"
  }
}
```

---

## 3. API開発アクセス用アカウント

SDK CLI、API直接呼び出し等に使用。上位権限を持つ開発チーム用。

| 項目 | 値 |
|------|-----|
| **TID** | `T9999999999999999999` |
| **lacisID** | `17347487748391988274` |
| **Email (userId)** | `dev@araneadevice.dev` |
| **CIC** | `022029` |
| **用途** | SDK CLI、API開発・テスト |
| **権限** | araneaDevice開発チーム上位権限 |

### 環境変数設定

```bash
# ~/.bashrc または ~/.zshrc に追加
export ARANEA_TID="T9999999999999999999"
export ARANEA_LACIS_ID="17347487748391988274"
export ARANEA_USER_ID="dev@araneadevice.dev"
export ARANEA_CIC="022029"
```

### SDK CLI での使用

```bash
# 環境変数が設定されていれば自動認証
aranea-sdk schema get aranea_ar-is04a

# または明示的に指定
aranea-sdk simulate \
  --tid $ARANEA_TID \
  --lacisId $ARANEA_LACIS_ID \
  --cic $ARANEA_CIC \
  --type aranea_ar-is04a
```

---

## 4. 使い分けガイド

| シナリオ | 使用アカウント |
|----------|---------------|
| ESP32ファームウェア開発・テスト | デバイス登録テスト用 |
| SDK CLI でスキーマ取得 | API開発アクセス用 |
| deviceStateReport API テスト | API開発アクセス用 |
| 新デバイスType開発 | 両方（登録→API） |

---

## 5. セキュリティ注意事項

- これらの認証情報は**開発・テスト専用**です
- 本番環境（市山水産株式会社）の認証情報とは別管理
- CICは永続（ローテーションなし）
- 不正利用が疑われる場合は管理者に連絡

---

## 6. APIエンドポイント

| API | URL |
|-----|-----|
| araneaDeviceGate | `https://asia-northeast1-mobesorder.cloudfunctions.net/araneaDeviceGate` |
| deviceStateReport | `https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport` |
| schemaAPI | `https://asia-northeast1-mobesorder.cloudfunctions.net/araneaSchemaAPI` |

---

## 関連ドキュメント

- [DEVICE_IMPLEMENTATION.md](./Design/DEVICE_IMPLEMENTATION.md) - デバイス実装ガイド
- [AUTH_SPEC.md](./Design/AUTH_SPEC.md) - 認証仕様
- [tenant_credentials.md](../docs/common/tenant_credentials.md) - 本番テナント情報
