# araneaDeviceGate 完全挙動仕様書

## 概要

araneaDeviceGateは、araneaDeviceの登録・認証を行うCloud Functionsエンドポイント。

**URL**: `https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate`

---

## 1. 基本レスポンスパターン

### HTTPステータスコード別挙動

| 状態 | cic_code | cic_active | HTTP | 結果 |
|------|----------|------------|------|------|
| 正常（既存） | "123456" | true | 200 | 既存CIC返却 |
| 停止中 | "123456" | false | 403 | エラー（管理者連絡必要） |
| CIC削除済み | null | - | 200 | 新CIC自動発行（recovered） |
| 新規登録 | - | - | 201 | 新規登録 + CIC発行 |

---

## 2. 同一MACで別lacisIDの場合（書き換え処理）

### シナリオ
デバイスのProductType/ProductCodeが変更された場合

```
旧: MAC=6CC8408C9D80, ProductType=003, ProductCode=0096
    → lacisId = 30036CC8408C9D800096

新: MAC=6CC8408C9D80, ProductType=004, ProductCode=0096
    → lacisId = 30046CC8408C9D800096
```

### 挙動
1. MACアドレス "6CC8408C9D80" で userObject_detail を検索
2. 旧lacisId "30036CC8408C9D800096" を発見
3. 旧デバイスをハードデリート（userObject + userObject_detail）
4. 新lacisId "30046CC8408C9D800096" で新規登録
5. 新CICを発行して返却

### レスポンス例 (status 201)
```json
{
  "ok": true,
  "lacisId": "30046CC8408C9D800096",
  "result": { "created": true },
  "userObject": {
    "cic_code": "新規発行CIC",
    "cic_active": true
  }
}
```

---

## 3. 処理フローチャート

```
araneaDeviceGate リクエスト
        │
        ▼
┌─────────────────────┐
│ 同一MACで別lacisID? │
└─────────────────────┘
        │
   Yes  │  No
        ▼
┌─────────────────────┐
│ 旧デバイス削除      │
│ (ハードデリート)    │
└─────────────────────┘
        │
        ▼
┌─────────────────────┐
│ 同一lacisID存在?    │
└─────────────────────┘
        │
   Yes  │  No → 新規登録 (201)
        ▼
┌─────────────────────┐
│ cic_active = true?  │
└─────────────────────┘
        │
   Yes  │  No → 403 (deactivated)
        ▼
┌─────────────────────┐
│ cic_code 存在?      │
└─────────────────────┘
        │
   Yes  │  No → 新CIC発行 (200 + recovered)
        ▼
   200 (既存CIC返却)
```

---

## 4. 危険パターン検出と対応

### 検出する危険パターン

| パターン | 状況 | リスク |
|----------|------|--------|
| tid変更 | 同一lacisIDで異なるtid | デバイスが別テナントに移動 or 乗っ取り |
| ordinaler変更 | 同一lacisIDで異なる認証者 | 権限委譲 or 不正認証 |

### 対応フロー

```
既存デバイス検出
       │
       ▼
┌─────────────────────┐
│ tid または ordinaler │
│ が異なる?           │
└─────────────────────┘
       │
  Yes  │  No → 通常処理（既存CIC返却）
       ▼
┌─────────────────────┐
│ 1. 旧CICを無効化    │
│ 2. 新CICを発行      │
│ 3. tid/ordinaler更新│
│ 4. 監査ログ記録     │
└─────────────────────┘
       │
       ▼
   200 + ownershipChanged: true
```

### レスポンス例（所有権変更時）
```json
{
  "ok": true,
  "existing": true,
  "ownershipChanged": true,
  "lacisId": "30046CC8408C9D800096",
  "userObject": {
    "cic_code": "新規CIC",
    "cic_active": true,
    "permission": 10
  },
  "warning": "Device ownership has been transferred. Previous CIC is now invalid."
}
```

### Firestore監査ログ
```javascript
// userObject/{lacisId}.security.lastOwnershipChange
{
  "previousTid": "T旧テナント",
  "previousOrdinaler": "旧認証者lacisId",
  "previousCic": "旧CIC（無効化済み）",
  "changedAt": "2025-12-18T02:25:00Z",
  "changedBy": "新認証者lacisId",
  "reason": "tid_change"
}
```

### セキュリティ効果
- 旧CICは即座に無効化（旧テナントからのアクセス不可）
- 不正な乗っ取り試行も監査ログに記録
- 正当な認証（lacisOath）を経た場合のみ移管成功

---

## 5. 運用パターン

| 操作 | 方法 | 結果 |
|------|------|------|
| デバイス一時停止 | cic_active: false | 403エラー |
| デバイス再開 | cic_active: true | 正常動作 |
| CIC強制更新 | cic_code を削除 | 新CIC自動発行 |
| デバイス移管 | ProductType/Code変更 | 旧削除→新登録 |

---

## 6. リクエスト形式

### 登録リクエスト
```json
{
  "lacisOath": {
    "lacisId": "テナントプライマリのlacisId",
    "userId": "認証メールアドレス",
    "pass": "パスワード",
    "cic": "テナントプライマリのCIC",
    "method": "register"
  },
  "userObject": {
    "lacisID": "デバイスのlacisId",
    "tid": "テナントID",
    "typeDomain": "araneaDevice",
    "type": "ar-is10"
  },
  "deviceMeta": {
    "macAddress": "MACアドレス（12文字）",
    "productType": "製品タイプ（3桁）",
    "productCode": "製品コード（4桁）"
  }
}
```

---

## 7. デバイス側実装要件

### CICが空の場合の再登録
```cpp
if (myCic.isEmpty()) {
  // 保存済みCICがない場合は再登録を試行
  araneaReg.clearRegistration();
  // 次回起動時に再登録
}
```

### Factory Reset
```cpp
araneaReg.clearRegistration();  // NVS登録フラグをクリア
settings.clear();               // 設定をクリア
// 次回起動時に新規登録として処理
```

---

## 8. lacisID形式

```
[Prefix=3][ProductType=3桁][MAC=12桁][ProductCode=4桁] = 20文字

例: 3 + 010 + 30C92212F680 + 0001 = 301030C92212F6800001
```

---

## 更新履歴

| 日付 | 内容 |
|------|------|
| 2025-12-18 | 初版作成 - 危険パターン検出、所有権移管ロジック追加 |
