# Phase 3: Summary/GrandSummary 実装タスク

対応DD: DD02_Summary_GrandSummary.md
依存関係: Phase 2（CameraRegistry）

---

## 概要

検出イベントを定期的に集計し、Summary（間隔ベース）およびGrandSummary（時刻指定ベース）を生成・保存する。

---

## Summary種別

| 種別 | トリガー | デフォルト | 用途 |
|------|---------|-----------|------|
| Summary | 時間間隔経過 | 60分 | 期間内の検出イベント要約 |
| GrandSummary | 指定時刻到達 | 09:00, 17:00, 21:00 | 複数Summaryの統合・1日の総括 |
| Emergency | 異常検出時 | severity >= 閾値 | 即時報告 |

---

## タスク一覧

### T3-1: SummaryOverview設計・データモデル

**状態**: ⬜ NOT_STARTED
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- `ai_summary_cache`にsummary_jsonカラム追加
- `scheduled_reports`テーブル作成
- SummaryPayload型定義

**成果物**:
- `migrations/019_summary_service.sql`
- `src/summary_service/types.rs`

**summaryID形式**（CONSISTENCY_CHECK P1-1対応）:
- DBのsummary_id（BIGINT AUTO_INCREMENT）を正本
- Paraclate送信時は`SUM-{id}`形式

**検証方法**:
- マイグレーション実行成功
- 型定義コンパイル

---

### T3-2: summary_generator.rs 実装

**状態**: ⬜ NOT_STARTED
**優先度**: P0（ブロッカー）
**見積もり規模**: L

**内容**:
- `SummaryGenerator`クラス実装
- 検出ログ集計→summary_text/summary_json生成

**主要メソッド**:
- `generate()`: 期間指定でSummary生成
- `generate_summary_text()`: テンプレートベースのテキスト生成
- `build_summary_json()`: Paraclate送信用JSONペイロード構築

**成果物**:
- `src/summary_service/generator.rs`
- `src/summary_service/payload_builder.rs`

**検証方法**:
- 空期間→空Summary
- 検出あり→集計正確

---

### T3-3: ai_summary_cache リポジトリ

**状態**: ⬜ NOT_STARTED
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- Summary保存・取得・検索
- expires_at管理

**主要メソッド**:
- `insert()`
- `get_by_id()`
- `get_by_period()`
- `get_latest()`
- `update_summary_json()`

**成果物**:
- `src/summary_service/repository.rs`

**検証方法**:
- CRUD操作テスト

---

### T3-4: 定時実行スケジューラ

**状態**: ⬜ NOT_STARTED
**優先度**: P0（ブロッカー）
**見積もり規模**: L

**内容**:
- `SummaryScheduler`実装（1分間隔チェック）
- next_run_at管理
- interval_minutes/scheduled_times対応

**タイムゾーン対応**:
- scheduled_times（09:00等）はAsia/Tokyo基準で解釈
- UTCとの変換を正確に実装

**成果物**:
- `src/summary_service/scheduler.rs`

**検証方法**:
- スケジュール時刻到達→自動実行
- next_run_at正確更新

---

### T3-5: Summary API実装

**状態**: ⬜ NOT_STARTED
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- Summary生成/取得/一覧API

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/summary/generate` | POST | 手動Summary生成 |
| `/api/summary/latest` | GET | 最新Summary取得 |
| `/api/summary/:id` | GET | 特定Summary取得 |
| `/api/summary/range` | GET | 期間指定一覧 |

**成果物**:
- `src/web_api/summary_routes.rs`

**検証方法**:
- API呼び出しテスト

---

### T3-6: GrandSummary設計

**状態**: ⬜ NOT_STARTED
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- 複数hourly Summaryの統合ロジック
- includedSummaryIds管理
- 日次境界はAsia/Tokyo基準

**成果物**:
- GrandSummary JSON仕様
- 統合アルゴリズム

**検証方法**:
- hourly 24件→daily 1件統合

---

### T3-7: grand_summary.rs 実装

**状態**: ⬜ NOT_STARTED
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- `GrandSummaryGenerator`クラス実装

**主要メソッド**:
- `generate()`: 日付指定でGrandSummary生成

**成果物**:
- `src/summary_service/grand_summary.rs`

**検証方法**:
- 統合値正確
- includedSummaryIds正確

---

### T3-8: Summary→GrandSummary統合テスト

**状態**: ⬜ NOT_STARTED
**優先度**: P1（品質改善）
**見積もり規模**: M

**内容**:
- E2E統合テスト実装
- スケジューラ→Summary→GrandSummary→送信フロー

**成果物**:
- 統合テストコード
- テスト手順書

**検証方法**:
- フルフロー動作確認

---

## Summary JSONペイロード形式

```json
{
  "lacisOath": {
    "lacisID": "{is22_lacis_id}",
    "tid": "{tenant_id}",
    "cic": "{cic}",
    "blessing": null
  },
  "summaryOverview": {
    "summaryID": "SUM-{db_summary_id}",
    "firstDetectAt": "{timestamp}",
    "lastDetectAt": "{timestamp}",
    "detectedEvents": "{count}"
  },
  "cameraContext": { ... },
  "cameraDetection": [ ... ]
}
```

---

## スケジュール管理API

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/reports/schedule` | GET | スケジュール取得 |
| `/api/reports/schedule` | PUT | スケジュール更新 |

---

## テストチェックリスト

- [ ] T3-1: マイグレーション実行確認
- [ ] T3-2: 空期間Summaryテスト
- [ ] T3-2: 検出ありSummaryテスト
- [ ] T3-3: CRUD操作テスト
- [ ] T3-4: スケジューラ実行テスト（JST基準）
- [ ] T3-5: API呼び出しテスト
- [ ] T3-6: GrandSummary統合テスト
- [ ] T3-7: includedSummaryIds正確性テスト
- [ ] T3-8: フルフローE2Eテスト

---

## E2E統合テスト（Phase完了時）

| テストID | 内容 | 確認項目 |
|---------|------|---------|
| E2 | カメラ検出→ログ記録→Summary生成 | Phase 2,3 |
| E3 | Summary→Paraclate送信→BQ同期 | Phase 3,4,5 |

---

## 完了条件

1. Phase 2完了が前提
2. 全タスク（T3-1〜T3-8）が✅ COMPLETED
3. テストチェックリスト全項目パス
4. E2テスト実行可能

---

## Issue連携

**Phase Issue**: #116
**親Issue**: #113

全タスクは#116で一括管理。個別タスクのサブIssue化は必要に応じて実施。

---

## 更新履歴

| 日付 | 更新内容 |
|------|---------|
| 2026-01-10 | 初版作成 |
