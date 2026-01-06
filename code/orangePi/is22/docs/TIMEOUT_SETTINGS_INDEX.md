# タイムアウト設定機能 設計インデックス

**作成日**: 2026-01-06
**最終更新**: 2026-01-06

## ドキュメント構成

```
TIMEOUT_SETTINGS_INDEX.md (本ファイル)
├─ TIMEOUT_SETTINGS_DESIGN.md (概要設計)
├─ TIMEOUT_SETTINGS_DETAILED_DESIGN.md (詳細設計)
└─ TIMEOUT_SETTINGS_TASKS.md (タスクリスト・テスト計画)
```

---

## 1. 概要設計

**ファイル**: [TIMEOUT_SETTINGS_DESIGN.md](./TIMEOUT_SETTINGS_DESIGN.md)

**内容**:
- 目的・背景
- 機能要件（提案1・提案2）
- 非機能要件
- 設計原則との整合性（SSoT, SOLID, MECE, アンアンビギュアス）
- 依存関係と実装順序
- テスト計画概要
- リスクと対策
- 受け入れ基準

**対象読者**: プロジェクトマネージャー、レビュアー、実装者

**MECEチェック**: ✅ 完了
**アンアンビギュアスチェック**: ✅ 完了

---

## 2. 詳細設計

**ファイル**: [TIMEOUT_SETTINGS_DETAILED_DESIGN.md](./TIMEOUT_SETTINGS_DETAILED_DESIGN.md)

**内容**:

### Phase 1: グローバルタイムアウト設定
- 1.1 データベース設計（settings.polling拡張）
- 1.2 Backend API設計
  - GET /api/settings/timeouts
  - PUT /api/settings/timeouts
- 1.3 SnapshotService修正
- 1.4 PollingOrchestrator修正
- 1.5 Frontend実装（SettingsModal）

### Phase 2: カメラ別カスタムタイムアウト設定
- 2.1 データベース設計（camerasテーブル拡張）
- 2.2 Backend修正
  - Camera構造体
  - ConfigStore
  - SnapshotService（タイムアウト適用ロジック）
  - Camera更新API
- 2.3 Frontend実装（CameraDetailModal）

### Phase 3: 統合テスト計画
- テストシナリオ一覧（7ケース）
- パフォーマンステスト
- UI/UXテスト

**対象読者**: 実装者、テスター

**実装レベル**: コードレベルの詳細設計
**SQLファイル**: マイグレーション含む

---

## 3. タスクリスト・テスト計画

**ファイル**: [TIMEOUT_SETTINGS_TASKS.md](./TIMEOUT_SETTINGS_TASKS.md)

**内容**:

### Phase 1タスク（11タスク、推定3時間10分）
- Backend API実装（3タスク）
- SnapshotService修正（1タスク）
- PollingOrchestrator修正（1タスク）
- Frontend実装（2タスク）
- ビルド・デプロイ・テスト（4タスク）

### Phase 2タスク（11タスク、推定2時間30分）
- DB Migration（2タスク）
- Backend修正（4タスク）
- Frontend修正（3タスク）
- ビルド・デプロイ・テスト（4タスク）

### Phase 3タスク（4タスク、推定1時間35分）
- フォールバック動作確認（1タスク）
- 実カメラ検証（1タスク）
- パフォーマンステスト（1タスク）
- UI/UXテスト（2タスク）

**合計推定時間**: 約7時間15分

**対象読者**: 実装者、プロジェクトマネージャー

**チェックリスト形式**: ✅ タスクごとに進捗管理可能

---

## 4. 関連ファイル

### 影響を受けるソースファイル

**Backend**:
- `src/web_api/routes.rs` - API実装
- `src/snapshot_service/mod.rs` - タイムアウト適用
- `src/polling_orchestrator/mod.rs` - 設定読み込み
- `src/config_store/types.rs` - Camera構造体
- `src/config_store/repository.rs` - カメラ読み込み

**Frontend**:
- `frontend/src/components/SettingsModal.tsx` - グローバル設定UI
- `frontend/src/components/CameraDetailModal.tsx` - カメラ個別設定UI
- `frontend/src/types/api.ts` - 型定義

**Database**:
- `migrations/015_camera_custom_timeouts.sql` - マイグレーション（新規）

**既存設定**:
- `settings.polling` テーブル（拡張済み）

---

## 5. The_golden_rules.md 準拠チェック

### 大原則の検証

- ✅ **#1 SSoT**: データソースは一意（settings.polling, cameras）
- ✅ **#2 SOLID**: 単一責任（SnapshotService）、依存逆転（設定注入）
- ✅ **#3 MECE**: 決定木で検証済み
- ✅ **#4 アンアンビギュアス**: 曖昧な表現を排除
- ✅ **#5 情報平等**: すべてのカメラを等価に扱う
- ✅ **#6 現場猫案件禁止**: ハードコード排除、デフォルト値はフォールバック
- ✅ **#7 優先順位**: 依存関係で定義（Phase 1→2→3）
- ✅ **#9 テストのない完了報告禁止**: 7ケースのテスト計画作成済み
- ✅ **#12 車輪の再発明禁止**: 既存ConfigStore機構を活用
- ✅ **#15 設計ドキュメント必須**: 概要・詳細・タスクリスト作成完了

### 基本手順の検証

- ✅ **手順1**: 現象確認（.62カメラのタイムアウト問題）
- ✅ **手順2**: 概要ドキュメント作成
- ✅ **手順4**: 詳細設計作成（MECE・アンアンビギュアス）
- ✅ **手順5**: タスクリスト・テスト計画作成
- 🔄 **手順6**: GitHubにIssue登録（次のステップ）
- ⏳ **手順7**: 実装前コミット・プッシュ
- ⏳ **手順8**: 実装開始
- ⏳ **手順9**: テスト実行・報告

---

## 6. クイックリファレンス

### 実装順序（依存関係）

```
1. Phase 1: グローバル設定（独立して実装可能）
   ↓
2. Phase 2: カメラ別設定（Phase 1のフォールバック先として依存）
   ↓
3. Phase 3: 統合テスト（Phase 1, 2完了後）
```

### データフロー

```
[ユーザー] → [SettingsModal] → [PUT /api/settings/timeouts] → [settings.polling]
                                                                       ↓
[PollingOrchestrator起動] → [load_global_timeout_settings] → [SnapshotService::new]
                                                                       ↓
[カメラ巡回] → [SnapshotService::capture] → タイムアウト値決定:
                                              ├─ camera.custom_timeout_enabled=1
                                              │  └─ camera.timeout_main/sub_sec
                                              └─ camera.custom_timeout_enabled=0
                                                 └─ グローバル設定
```

### 決定木（タイムアウト適用）

```
カメラXのスナップショット取得時
├─ custom_timeout_enabled = 1?
│  ├─ YES → camera.timeout_main_sec / timeout_sub_sec
│  │        (NULL の場合はグローバルへフォールバック)
│  └─ NO  → settings.polling の timeout_main_sec / timeout_sub_sec
└─ DB取得失敗時 → ハードコードデフォルト（10秒/20秒）
```

---

## 7. 次のアクション

**現在のステータス**: 設計完了、Issue登録待ち

**次に実施すること**:
1. GitHub Issue登録（本インデックスとリンク）
2. 実装前の段階をgit commit & push
3. Phase 1.1.1タスクから実装開始

**Issue作成時に含めるべき情報**:
- タイトル: `[is22] カメラタイムアウト設定機能実装（グローバル＋カメラ別）`
- ラベル: `enhancement`, `backend`, `frontend`, `database`
- マイルストーン: `is22-timeout-settings`
- リンク: 本インデックスファイル
- 推定時間: 7時間15分
- 優先度: High（125サブネットエラー率37.5%問題の対応）

---

**文書ステータス**: 設計インデックス完成
**MECEチェック**: ✅ 完了
**アンアンビギュアスチェック**: ✅ 完了
**The_golden_rules.md準拠**: ✅ 確認済み
