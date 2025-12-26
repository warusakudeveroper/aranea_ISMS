# AraneaSDK CLI テストレポート

**テスト日時**: 2025-12-26
**テスト環境**: macOS, Node.js v22.19.0
**パッケージ**: aranea-sdk-cli@0.1.0

---

## テスト結果サマリー

| テスト項目 | 結果 | 備考 |
|-----------|------|------|
| インストール | ✅ PASS | `npm install -g aranea-sdk-cli` |
| バージョン確認 | ✅ PASS | 0.1.0 |
| schema list | ✅ PASS | 5件取得 |
| schema get | ⚠️ FAIL | **スキーマ内容が不正** |
| validate type | ✅ PASS | 形式チェック動作 |
| validate lacis-id | ✅ PASS | 形式チェック動作 |
| validate mac | ✅ PASS | 形式チェック動作 |
| test connection | ✅ PASS | State OK, Gate 404 (想定内) |
| test auth | ❌ FAIL | HTMLレスポンス返却 |
| simulate state-report | ⚠️ FAIL | **スキーマ不整合により不正なフィールド生成** |
| register dry-run | ✅ PASS | リクエスト生成・バリデーション動作 |

---

## 🔴 重大問題: スキーマ不整合

### 発見内容

mobes側Firestoreの `typeSettings/araneaDevice/{type}` に登録されているスキーマが、aranea_ISMS側の実装と**完全に異なる**。

| Type | mobes側 | aranea_ISMS側 (正) |
|------|---------|-------------------|
| is04a | Network Scanner | Window & Door Controller |
| is05a | Environment Sensor | 8-Channel Detector |

### 影響範囲

1. `schema get` が間違ったスキーマを返す
2. `simulate` が間違った状態フィールドを生成
3. 開発者が間違ったコードを書いてしまう

### 詳細

[SCHEMA_MISMATCH_BUG_REPORT.md](./SCHEMA_MISMATCH_BUG_REPORT.md) 参照

---

## コマンド別詳細結果

### 1. インストール

```bash
$ npm install -g aranea-sdk-cli
added 56 packages in 2s
```

**注意**: コマンド名は `aranea-sdk` (`aranea-cli` ではない)

### 2. バージョン・ヘルプ

```bash
$ aranea-sdk --version
0.1.0

$ aranea-sdk --help
Commands:
  test                接続・認証テスト
  simulate            デバイス動作のシミュレーション
  validate            入力値のバリデーション
  schema              Type スキーマの取得・表示
  register [options]  デバイス登録 (ドライラン / 実登録)
```

### 3. schema list

```bash
$ aranea-sdk schema list
登録済みType:
  aranea_ar-is01   - AraneaSDK Basic Sensor
  aranea_ar-is04a  - Network Scanner ← ❌ 間違い
  aranea_ar-is05a  - Environment Sensor ← ❌ 間違い
  aranea_ar-is06a  - Power Monitor
  aranea_ar-is10   - Router Inspector ← ✅ 概ね正しい
```

### 4. validate (形式チェック)

すべて正常動作:

```bash
# Type名検証
$ aranea-sdk validate type --type aranea_ar-is04a
✓ Type名は正しい形式です

$ aranea-sdk validate type --type ISMS_ar-is04
✗ 旧プレフィックス 'ISMS_' を使用しています
  推奨: 'aranea_ar-is04'

# LacisID検証
$ aranea-sdk validate lacis-id --lacis-id 3004AABBCCDDEEFF0001
✓ LacisIDは正しい形式です

# MACアドレス検証
$ aranea-sdk validate mac --mac AA:BB:CC:DD:EE:FF
✓ MACアドレスは正しい形式です
  正規化: AABBCCDDEEFF
```

### 5. test connection

```bash
$ aranea-sdk test connection
Gate:  ✗ (404) ← GETではなくPOST必要、想定内
State: ✓
```

### 6. test auth

```bash
$ aranea-sdk test auth --tid T999... --lacis-id 173... --cic 022029
✖ 認証テスト失敗
エラー: invalid json response body ... "
<html><hea"... is not valid JSON
```

**原因**: Gate APIがHTMLエラーページを返却（要調査）

### 7. register dry-run

```bash
$ aranea-sdk register --type aranea_ar-is04a --mac AABBCCDDEEFF --dry-run
デバイス情報:
  Type:         aranea_ar-is04a
  ProductType:  004
  LacisID:      3004AABBCCDDEEFF1201

✓ ドライラン完了 - 形式チェック通過
```

**良い点**: デフォルトで開発アカウント情報が設定済み

---

## 使用可能な機能

| 機能 | 可否 | 備考 |
|------|------|------|
| Type名検証 | ✅ | 即座に使用可能 |
| LacisID検証 | ✅ | 即座に使用可能 |
| MAC検証 | ✅ | 即座に使用可能 |
| スキーマ一覧 | ⚠️ | 動作するがスキーマ内容が不正 |
| スキーマ取得 | ⚠️ | 動作するがスキーマ内容が不正 |
| 登録ドライラン | ✅ | 形式チェックとして使用可能 |
| 実登録 | ⚠️ | 未テスト（実際に登録される） |

---

## mobes側への要望

### P0 (即座に対応必要)

1. **Firestoreスキーマ更新**: `aranea_ISMS/araneaSDK/schemas/types/` の内容で更新
   - `aranea_ar-is04a.json`
   - `aranea_ar-is05a.json`

### P1 (優先対応)

2. **test auth エラー修正**: HTMLではなくJSONエラーを返す

### P2 (改善)

3. **コマンド名統一**: `aranea-cli` と `aranea-sdk` の統一を検討
4. **環境変数サポート**: `ARANEA_CIC` 等で認証情報を設定可能に

---

## 環境設定

テスト実行に使用した認証情報:

```bash
# ~/.zshrc に追加推奨
export ARANEA_TID="T9999999999999999999"
export ARANEA_LACIS_ID="17347487748391988274"
export ARANEA_USER_ID="dev@araneadevice.dev"
export ARANEA_CIC="022029"
```

詳細は [TEST_CREDENTIALS.md](./TEST_CREDENTIALS.md) 参照

---

## 作成ファイル

| ファイル | 内容 |
|----------|------|
| `TEST_CREDENTIALS.md` | テスト認証情報ドキュメント |
| `.env.example` | 環境変数テンプレート |
| `SCHEMA_MISMATCH_BUG_REPORT.md` | スキーマ不整合バグレポート |
| `SDK_CLI_TEST_REPORT.md` | このファイル |

---

## 結論

- CLI自体は**正常に動作**
- **validate系コマンドは即座に使用可能**
- **スキーマ関連は mobes側修正待ち**
- register/simulateは修正後に再テスト推奨
