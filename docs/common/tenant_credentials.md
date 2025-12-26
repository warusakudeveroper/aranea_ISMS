# テナント認証情報（ISMS プロジェクト共通）

**注意**: このファイルには認証情報が含まれています。本番環境では適切に管理してください。

---

## 1. 本番テナント（市山水産株式会社）

### テナント情報

| 項目 | 値 |
|------|-----|
| テナント名 | 市山水産株式会社 |
| TID | `T2025120608261484221` |
| FID | `9000` |

### テナントプライマリ（permission 61）

| 項目 | 値 |
|------|-----|
| lacisID | `12767487939173857894` |
| Email (userId) | `info+ichiyama@neki.tech` |
| CIC | `263238` |

> 認証は lacisId + Email(userId) + CIC の3要素のみ（パスワード入力は廃止済み）

---

## 2. テストテナント（開発用）

### テナント情報

| 項目 | 値 |
|------|-----|
| TID | `T9999999999999999999` |
| 用途 | 開発・テスト専用 |

### テナントプライマリ（permission 61）

| 項目 | 値 |
|------|-----|
| lacisID | `17347487748391988274` |
| Email (userId) | `dev@araneadevice.dev` |
| CIC | `022029` |

> ⚠️ **重要**: 開発・テストは必ずこちらのテストテナントを使用すること

---

## 3. Type名について

### 正式名称（新規開発用）

| Type | 説明 |
|------|------|
| `aranea_ar-is01` | 温湿度センサー |
| `aranea_ar-is04a` | 2ch接点コントローラー |
| `aranea_ar-is05a` | 8ch検出器 |
| `aranea_ar-is06a` | 6ch出力 |
| `aranea_ar-is10` | ルーター検査 |

### 旧名称（後方互換・運用中デバイス）

| 旧Type | 対応する正式Type | 状態 |
|--------|-----------------|------|
| `ISMS_ar-is01` | `aranea_ar-is01` | ✅ 稼働中・サポート継続 |
| `ISMS_ar-is02` | `aranea_ar-is02` | ✅ 稼働中・サポート継続 |
| `ISMS_ar-is04` | `aranea_ar-is04a` | ✅ 稼働中・サポート継続 |
| `ISMS_ar-is05` | `aranea_ar-is05a` | ✅ 稼働中・サポート継続 |

> 旧名称は後方互換のため永続サポート。詳細は [MOBES_COMPATIBILITY_REQUEST.md](./MOBES_COMPATIBILITY_REQUEST.md) 参照。

---

## 4. 用途別サンプル

### araneaDeviceGate 登録時（新規デバイス）

```json
{
  "lacisOath": {
    "lacisId": "17347487748391988274",
    "userId": "dev@araneadevice.dev",
    "cic": "022029",
    "method": "register"
  },
  "userObject": {
    "tid": "T9999999999999999999",
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

### deviceStateReport 認証時

```json
{
  "auth": {
    "tid": "T9999999999999999999",
    "lacisId": "3004AABBCCDDEEFF0001",
    "cic": "<登録時に発行されたCIC>"
  },
  "report": {
    "type": "aranea_ar-is04a",
    "state": { ... }
  }
}
```

---

## 5. エンドポイント

| API | URL |
|-----|-----|
| araneaDeviceGate | `https://asia-northeast1-mobesorder.cloudfunctions.net/araneaDeviceGate` |
| deviceStateReport | `https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport` |

---

## 6. CICについて

- **有効期限**: なし（永続）
- **自動ローテーション**: なし（禁止）
- **無効化**: 管理者による手動操作のみ

> ⚠️ CICの自動失効・ローテーションは実装禁止。詳細は [MOBES_COMPATIBILITY_REQUEST.md](./MOBES_COMPATIBILITY_REQUEST.md) 参照。

---

## 関連ドキュメント

- [MOBES_COMPATIBILITY_REQUEST.md](./MOBES_COMPATIBILITY_REQUEST.md) - 後方互換性要求
- [araneaDeviceGate_spec.md](./araneaDeviceGate_spec.md) - DeviceGate仕様
- [AraneaRegister モジュール](../../code/ESP32/global/src/AraneaRegister.h)
