# aranea_ar-is06s TYPE_MISMATCH問題報告

**報告日**: 2026-01-24
**報告者**: Claude Code (開発支援)
**デバイス**: IS06S (ar-is06s)

---

## 1. 問題概要

`deviceStateReport` API呼び出し時に `TYPE_MISMATCH` 警告が継続して発生する。
SDK経由での登録は完了しているが、警告が解消されない。

---

## 2. 実施した登録作業

### 2.1 UserTypeDefinitions登録 (成功)

```bash
npx aranea-sdk type create aranea_ar-is06s \
  --endpoint production \
  --type-domain araneaDevice \
  --display-name "Relay & Switch Controller" \
  --permission 21
```

**結果**: 登録済み (既存エラーで確認)

```
npx aranea-sdk type list --endpoint production
```
```
aranea_ar-is06s    AR-IS06S Relay & Switch   21 (Standard)   active
```

### 2.2 Schema登録 (成功)

```bash
# araneaSchemaAPI経由で登録
curl -X POST "https://asia-northeast1-mobesorder.cloudfunctions.net/araneaSchemaAPI" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "action": "push",
    "schema": {
      "type": "aranea_ar-is06s",
      "displayName": "Relay & Switch Controller",
      "productType": "006",
      ...
    }
  }'
```

**結果**:
```json
{"ok": true, "path": "araneaSDK/typeSettings/schemas/aranea_ar-is06s", "state": "development"}
```

### 2.3 Schema Promote (成功)

```bash
npx aranea-sdk schema promote --type aranea_ar-is06s -e production --force
```

**結果**: `[P]` (Production) 状態に昇格完了

```
npx aranea-sdk schema list -e production | grep is06s
```
```
[P] aranea_ar-is06s
    Relay & Switch Controller - 4ch D/P出力 + 2ch I/O コントローラー（PWM対応）
    ProductType: 006
```

---

## 3. 問題の状態

### 3.1 State Report テスト

```bash
curl -X POST "https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport" \
  -d '{
    "auth": {"tid": "T2025120621041161827", "lacisId": "30066CC84054CB480200", "cic": "858628"},
    "report": {"type": "aranea_ar-is06s", ...}
  }'
```

**レスポンス**:
```json
{
  "ok": true,
  "typeWarning": {
    "warning": "TYPE_MISMATCH",
    "receivedType": "aranea_ar-is06s",
    "suggestedType": "aranea_ar-is06a",
    "suggestions": [...]
  }
}
```

- `ok: true` → レポート自体は受理されている
- `typeWarning` → TYPE_MISMATCHが継続

### 3.2 推測される原因

| SDK登録先 | deviceStateReport参照先(推測) |
|-----------|------------------------------|
| `araneaSDK/typeSettings/schemas/{type}` | `typeSettings/araneaDevice/{type}` ? |
| `userTypeDefinitions/araneaDevice__{type}` | 別のコレクション? |

SDK登録パスと deviceStateReport の検証パスが異なる可能性がある。

---

## 4. 確認済み事項

| 項目 | 状態 |
|------|------|
| UserTypeDefinitions | ✅ `aranea_ar-is06s` 登録済み・active |
| Schema (araneaSDK/typeSettings) | ✅ Production状態 |
| State Report受理 | ✅ `ok: true` |
| MQTT疎通 | ✅ コマンド送受信確認済み |
| デバイス動作 | ✅ 完全に機能している |

---

## 5. 質問事項

1. **TYPE_MISMATCH警告を解消するために必要な追加登録手順はありますか？**

2. **deviceStateReportがType検証で参照するFirestoreコレクションはどこですか？**
   - `araneaSDK/typeSettings/schemas/` と `typeSettings/araneaDevice/` の関係は？

3. **過去の他デバイス(is04a, is05a等)はどのような手順で登録しましたか？**
   - SDK以外の登録方法がある場合、教えてください

---

## 6. 参考情報

### デバイス情報
- **LacisID**: `30066CC84054CB480200`
- **CIC**: `858628`
- **TID**: `T2025120621041161827`
- **Type**: `aranea_ar-is06s`
- **ProductType**: `006`
- **ProductCode**: `0200`

### スキーマファイル
- `araneaSDK/schemas/types/aranea_ar-is06s.json`

### 関連ドキュメント
- `araneaSDK/Design/TYPE_REGISTRY.md`
- `araneaSDK/Design/API_REFERENCE.md`
