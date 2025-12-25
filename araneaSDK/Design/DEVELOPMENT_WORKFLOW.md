# AraneaSDK Development Workflow

## 1. Overview

mobes2.0とaranea_ISMSの連携開発ワークフローです。

---

## 2. Team Structure

```
┌─────────────────────────────────────────────────────────────┐
│                    AraneaDevice Development                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌─────────────────┐         ┌─────────────────┐           │
│   │   mobes2.0      │ ◄─────► │   aranea_ISMS   │           │
│   │   Team          │   SDK   │   Team          │           │
│   │                 │         │                 │           │
│   │ - Backend       │         │ - Firmware      │           │
│   │ - API           │         │ - Hardware      │           │
│   │ - Schema        │         │ - Testing       │           │
│   │ - AI/ML         │         │ - Deployment    │           │
│   └─────────────────┘         └─────────────────┘           │
│                                                              │
│              ┌─────────────────────────────┐                │
│              │      AraneaSDK              │                │
│              │  (Shared Reference)         │                │
│              │                             │                │
│              │  - Schemas                  │                │
│              │  - Validation Tools         │                │
│              │  - Test Suites              │                │
│              │  - Documentation            │                │
│              └─────────────────────────────┘                │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Development Phases

### Phase 1: Design

```
1. 要件定義
   ├── ハードウェア仕様
   ├── 機能要件
   └── 通信要件

2. 設計書作成
   ├── DESIGN.md (デバイス設計)
   ├── MODULE_ADAPTATION_PLAN.md (モジュール適用計画)
   └── API設計 (必要に応じて)

3. SDK更新
   ├── Type定義追加
   ├── Schema定義追加
   └── ドキュメント更新
```

### Phase 2: Implementation

```
1. ファームウェア開発
   ├── コア機能実装
   ├── 通信機能実装
   └── UI実装

2. バックエンド対応 (mobes2.0)
   ├── typeSettings登録
   ├── API対応
   └── BigQuery対応

3. SDK Validation追加
   ├── スキーマ検証追加
   └── テストケース追加
```

### Phase 3: Testing

```
1. ユニットテスト
   ├── ファームウェア単体テスト
   └── API単体テスト

2. 統合テスト
   ├── 通信テスト
   ├── 認証テスト
   └── データ整合性テスト

3. E2Eテスト
   ├── 登録→レポート→コマンド
   └── 障害復旧テスト
```

### Phase 4: Deployment

```
1. ステージング環境テスト
2. 本番デプロイ
3. モニタリング設定
4. ドキュメント最終化
```

---

## 4. Repository Workflow

### 4.1 Branch Strategy

```
main
├── develop
│   ├── feature/is07-new-sensor
│   ├── feature/sdk-validation-update
│   └── fix/is04a-pulse-issue
└── release/v1.2.0
```

### 4.2 Commit Convention

```
feat(is04a): パルス制御機能追加
fix(sdk): LacisID検証の正規表現修正
docs(sdk): TYPE_REGISTRY.md更新
test(e2e): 登録フローテスト追加
chore(deps): ArduinoJson 7.0.0にアップデート
```

### 4.3 PR Template

```markdown
## 概要
<!-- 変更内容の簡潔な説明 -->

## 変更タイプ
- [ ] 新機能
- [ ] バグ修正
- [ ] ドキュメント
- [ ] リファクタリング
- [ ] テスト

## 関連Issue
<!-- #123 など -->

## テスト
- [ ] ユニットテスト追加/更新
- [ ] SDK Validation通過
- [ ] E2Eテスト実施

## SDK同期
- [ ] Schema更新必要
- [ ] Type登録必要
- [ ] ドキュメント更新必要

## チェックリスト
- [ ] コードレビュー依頼
- [ ] mobes2.0側との同期確認
- [ ] 破壊的変更の有無確認
```

---

## 5. SDK Synchronization

### 5.1 Schema Sync Flow

```
aranea_ISMS                    AraneaSDK                    mobes2.0
    │                              │                           │
    │ 1. Schema変更提案            │                           │
    │ ────────────────────────────►│                           │
    │                              │                           │
    │                              │ 2. Schema検証            │
    │                              │ ────────────────────────►│
    │                              │                           │
    │                              │ 3. 承認/フィードバック   │
    │                              │ ◄────────────────────────│
    │                              │                           │
    │ 4. SDK更新通知               │                           │
    │ ◄────────────────────────────│                           │
    │                              │                           │
    │ 5. ファーム実装              │ 6. typeSettings更新       │
    │                              │ ────────────────────────►│
    │                              │                           │
```

### 5.2 Sync Check CLI

```bash
# SDK同期状態確認
aranea-sdk check-sync

# 出力例
Checking SDK synchronization...

Schema Sync:
  userObject.schema.json    ✓ In sync (v1.2.0)
  deviceStateReport.json    ✓ In sync (v1.1.0)
  ISMS_ar-is04a.json        ! Out of sync (local: v1.2.0, remote: v1.1.0)

Type Registry:
  ISMS_ar-is01              ✓ Registered
  ISMS_ar-is04a             ✓ Registered
  ISMS_ar-is07              ✗ Not registered

Recommendations:
  - Push ISMS_ar-is04a schema update to mobes2.0
  - Register ISMS_ar-is07 type
```

---

## 6. Testing Workflow

### 6.1 Local Testing

```bash
# 1. ESP32でファームウェアビルド
cd code/ESP32/is04a
pio run

# 2. 書き込み
pio run --target upload

# 3. シリアルモニタ
pio device monitor

# 4. SDK Validation実行
aranea-sdk validate-schema --type state --device-type ISMS_ar-is04a --input ./test-state.json
```

### 6.2 Integration Testing

```bash
# 1. 登録テスト
aranea-sdk test-auth lacis-oath \
  --primary-lacisId ${TENANT_PRIMARY_LACISID} \
  --email ${TENANT_PRIMARY_EMAIL} \
  --cic ${TENANT_PRIMARY_CIC}

# 2. 通信テスト
aranea-sdk test-comm http \
  --endpoint ${STATE_URL} \
  --payload ./test-payload.json

# 3. E2Eテスト
aranea-sdk e2e-test --scenario device-registration
```

### 6.3 CI/CD Integration

```yaml
# .github/workflows/test.yml
name: AraneaDevice Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  sdk-validation:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install SDK
        run: npm install -g @aranea-sdk/cli
      - name: Validate Schemas
        run: aranea-sdk validate-schema --all
      - name: Run E2E Tests
        run: aranea-sdk e2e-test --all
        env:
          ARANEA_ENV: staging
```

---

## 7. Documentation Workflow

### 7.1 Documentation Structure

```
araneaSDK/Design/
├── INDEX.md                 # ドキュメント索引
├── ARCHITECTURE.md          # システムアーキテクチャ
├── AUTH_SPEC.md             # 認証仕様
├── SCHEMA_SPEC.md           # スキーマ仕様
├── API_REFERENCE.md         # API リファレンス
├── DEVICE_IMPLEMENTATION.md # 実装ガイド
├── VALIDATION_TOOLS.md      # 検証ツール
├── TYPE_REGISTRY.md         # Type登録
└── DEVELOPMENT_WORKFLOW.md  # 開発ワークフロー (this)
```

### 7.2 Documentation Update Process

```
1. 変更が必要なドキュメントを特定
2. 変更内容を作成
3. SDK同期チェック実行
4. PR作成（mobes2.0/aranea_ISMS両方）
5. レビュー・承認
6. マージ
```

### 7.3 Documentation Sync

```bash
# ドキュメント同期状態確認
aranea-sdk check-docs

# ドキュメント差分確認
aranea-sdk diff-docs --source aranea_ISMS --target mobes2.0
```

---

## 8. Release Process

### 8.1 Version Numbering

```
MAJOR.MINOR.PATCH

MAJOR: 破壊的変更
MINOR: 新機能追加
PATCH: バグ修正

例:
  1.0.0 → 1.0.1 (バグ修正)
  1.0.1 → 1.1.0 (新機能追加)
  1.1.0 → 2.0.0 (破壊的変更)
```

### 8.2 Release Checklist

```markdown
## Pre-Release Checklist

### Code
- [ ] 全テスト通過
- [ ] コードレビュー完了
- [ ] 静的解析クリア

### SDK
- [ ] Schema更新完了
- [ ] Type登録完了
- [ ] Validation追加完了

### Documentation
- [ ] CHANGELOG更新
- [ ] API Reference更新
- [ ] 実装ガイド更新

### Deployment
- [ ] ステージング環境テスト完了
- [ ] 本番環境準備
- [ ] ロールバック手順確認

### Communication
- [ ] リリースノート作成
- [ ] チーム通知
```

### 8.3 Release Commands

```bash
# バージョン更新
aranea-sdk version bump --type minor

# リリース作成
aranea-sdk release create v1.2.0

# リリースノート生成
aranea-sdk release notes v1.2.0
```

---

## 9. Issue & Bug Tracking

### 9.1 Issue Template

```markdown
## 問題の説明
<!-- 問題の詳細な説明 -->

## 再現手順
1.
2.
3.

## 期待される動作
<!-- 本来あるべき動作 -->

## 実際の動作
<!-- 現在の動作 -->

## 環境
- Device: is04a
- Firmware: 1.0.0
- SDK: 0.1.0

## ログ
```
<!-- 関連するログ -->
```

## スクリーンショット
<!-- 必要に応じて -->
```

### 9.2 Bug Fix Workflow

```
1. Issue作成
2. 再現確認
3. 原因調査
4. 修正PR作成
5. SDK Validation実行
6. レビュー・マージ
7. リリース
8. Issue クローズ
```

---

## 10. Communication Channels

### 10.1 Sync Meetings

```
週次ミーティング:
- 進捗共有
- SDK変更確認
- 課題ディスカッション

月次レビュー:
- リリース計画
- アーキテクチャレビュー
- 技術負債確認
```

### 10.2 Async Communication

```
GitHub Issues:
- バグ報告
- 機能要望
- ドキュメント改善

GitHub Discussions:
- 設計ディスカッション
- ベストプラクティス共有
- Q&A
```

---

## 11. Access Control

### 11.1 SDK Tool Access

```yaml
# lacisOath ユーザーとしてアクセス
credentials:
  type: lacisOath
  lacisId: ${DEVELOPER_LACISID}
  email: ${DEVELOPER_EMAIL}
  cic: ${DEVELOPER_CIC}

permissions:
  - read:schemas
  - read:types
  - write:schemas (with approval)
  - execute:validation
  - execute:tests
```

### 11.2 Environment Access

| Environment | Purpose | Access |
|-------------|---------|--------|
| Development | ローカル開発 | 全開発者 |
| Staging | 統合テスト | 全開発者 |
| Production | 本番環境 | 承認者のみ |

---

## 12. Troubleshooting Guide

### 12.1 Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Schema validation failed | スキーマ不一致 | SDK同期確認 |
| Auth failed | CIC期限切れ | CIC再発行 |
| Type not found | 未登録Type | Type登録申請 |
| Sync out of date | SDK未更新 | SDK更新 |

### 12.2 Debug Commands

```bash
# 詳細ログ有効化
aranea-sdk --debug <command>

# 接続テスト
aranea-sdk test-conn --verbose

# 設定確認
aranea-sdk config show

# キャッシュクリア
aranea-sdk cache clear
```
