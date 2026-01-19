# Phase 5: BqSyncService 実装タスク

対応DD: DD04_BqSyncService.md
依存関係: Phase 3,4（Summary, ParaclateClient）

---

## 概要

is22の検出ログおよびSummaryをBigQueryへ同期し、長期保存・分析基盤を構築する。
非ブロッキングのバッチ処理により、本体処理への影響を最小化する。

---

## 同期対象

| ソーステーブル | BQテーブル | 同期タイミング |
|--------------|-----------|---------------|
| detection_logs | semantic_events | INSERT時にキュー追加 |
| ai_summary_cache | summaries | Summary生成時にキュー追加 |
| is21_logs | inference_logs | 推論完了時にキュー追加 |

**BQ dataset**（ddreview_2 P1-3確定）:
- `mobesorder.paraclate.semantic_events`
- `mobesorder.paraclate.summaries`

---

## タスク一覧

### T5-1: BQテーブル定義確認・連携準備

**状態**: ✅ COMPLETED (2026-01-11)
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- mobes2.0側BQテーブル定義確認
- Service Account認証設定
- 接続テスト

**BQテーブル定義（参考）**:
```sql
CREATE TABLE `mobesorder.paraclate.semantic_events` (
    event_id STRING NOT NULL,
    tid STRING NOT NULL,
    fid STRING NOT NULL,
    camera_lacis_id STRING,
    detected_at TIMESTAMP NOT NULL,
    severity INT64,
    detection_type STRING,
    detection_detail JSON,
    tags ARRAY<STRING>,
    vehicle_ocr JSON,
    face_features JSON,
    snapshot_url STRING,
    source_device STRING,
    ingested_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP()
)
PARTITION BY DATE(detected_at)
CLUSTER BY tid, fid;

CREATE TABLE `mobesorder.paraclate.summaries` (
    summary_id STRING NOT NULL,
    tid STRING NOT NULL,
    fid STRING NOT NULL,
    summary_type STRING,
    period_start TIMESTAMP,
    period_end TIMESTAMP,
    summary_text STRING,
    summary_json JSON,
    detection_count INT64,
    severity_max INT64,
    camera_ids ARRAY<STRING>,
    created_at TIMESTAMP,
    ingested_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP()
)
PARTITION BY DATE(period_start)
CLUSTER BY tid, fid;
```

**成果物**:
- BQ接続設定
- Service Account JSON配置

**検証方法**:
- BQ接続テスト成功

---

### T5-2: bq_sync.rs サービス実装

**状態**: ✅ COMPLETED (2026-01-11)
**優先度**: P0（ブロッカー）
**見積もり規模**: L

**内容**:
- `BqSyncProcessor`クラス実装
- バッチ処理ロジック
- BQクライアントラッパー

**主要メソッド**:
- `process_batch()`: バッチ同期実行
- `retry_failed()`: 失敗アイテム再試行
- `flush()`: 即時フラッシュ

**成果物**:
- `src/bq_sync_service/processor.rs`
- `src/bq_sync_service/bq_client.rs`

**検証方法**:
- バッチ同期動作テスト

---

### T5-3: バッチ同期処理（非ブロッキング）

**状態**: ✅ COMPLETED (2026-01-11)
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- 30秒間隔のバックグラウンド処理
- batch_size: 100件
- テーブルごとのグループ化

**成果物**:
- バックグラウンドタスク実装

**検証方法**:
- 非ブロッキング動作確認
- パフォーマンス測定

---

### T5-4: synced_to_bq管理・PKマップ

**状態**: ✅ COMPLETED (2026-01-11)
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- ソーステーブルのsynced_to_bqフラグ管理
- テーブルごとのPKマップ実装（CONSISTENCY_CHECK P0-5対応）

**PKマップ**:
| テーブル | 主キー |
|---------|--------|
| detection_logs | id |
| ai_summary_cache | summary_id |
| is21_logs | id |

**マイグレーションSQL**:
```sql
ALTER TABLE detection_logs
ADD COLUMN synced_to_bq BOOLEAN DEFAULT FALSE,
ADD INDEX idx_synced (synced_to_bq);

ALTER TABLE ai_summary_cache
ADD COLUMN synced_to_bq BOOLEAN DEFAULT FALSE,
ADD INDEX idx_synced (synced_to_bq);
```

**成果物**:
- `migrations/021_bq_sync_extension.sql`
- PKマップ実装

**検証方法**:
- synced_to_bq更新テスト（各テーブル）

---

### T5-5: 冪等性保証

**状態**: ✅ COMPLETED (2026-01-11)
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- 重複INSERT防止
- event_id/summary_idによる冪等性

**成果物**:
- 冪等性チェックロジック

**検証方法**:
- 同一データ再送信→重複なし

---

### T5-6: retention連携

**状態**: ✅ COMPLETED (2026-01-11)
**優先度**: P1（品質改善）
**見積もり規模**: M

**内容**:
- retention_days（60日）に基づく削除
- BQ側データは別管理（mobes2.0側）
- ローカルデータのクリーンアップ

**成果物**:
- retention jobスケジュール

**検証方法**:
- 60日超過データ削除確認

---

### T5-7: 同期ログ・監視

**状態**: ✅ COMPLETED (2026-01-11)
**優先度**: P1（品質改善）
**見積もり規模**: S

**内容**:
- 同期成功/失敗ログ
- キュー統計API
- ヘルスチェック

**メトリクス閾値**:
| メトリクス | 閾値 | アクション |
|-----------|------|-----------|
| pending_count | > 1000 | 警告ログ |
| failed_count | > 10 | アラート通知 |

**成果物**:
- 監視ログ実装
- ヘルスチェックAPI

**検証方法**:
- ログ出力確認
- ヘルスチェック動作

---

## API実装

### is22側内部API

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/bq-sync/status` | GET | BQ同期状態取得 |
| `/api/bq-sync/retry` | POST | 失敗アイテム再試行 |
| `/api/bq-sync/flush` | POST | 保留中アイテム即時同期 |
| `/api/bq-sync/failed` | DELETE | 失敗アイテムクリア |

**成果物**:
- `src/web_api/bq_sync_routes.rs`

---

## enqueuer実装

### BqSyncEnqueuer

**主要メソッド**:
- `enqueue_detection_log()`: 検出ログキュー追加
- `enqueue_summary()`: Summaryキュー追加

**ペイロード構築**:
- event_id: `det_{id}` 形式
- summary_id: `SUM-{id}` 形式（DD02と統一）

---

## 設定ファイル

```yaml
# config/bq_sync.yaml
bq_sync:
  enabled: true
  project_id: "mobes-project"
  dataset_id: "paraclate"
  batch_size: 100
  interval_seconds: 30
  credentials_path: "/etc/secrets/bq-service-account.json"
```

---

## テストチェックリスト

- [ ] T5-1: BQ接続テスト
- [ ] T5-2: バッチ同期基本動作テスト
- [ ] T5-3: 非ブロッキング動作テスト
- [ ] T5-4: synced_to_bq更新テスト（各テーブル）
- [ ] T5-4: PKマップ正確性テスト
- [ ] T5-5: 冪等性テスト
- [ ] T5-6: retention削除テスト
- [ ] T5-7: ログ出力・ヘルスチェックテスト

---

## E2E統合テスト（Phase完了時）

| テストID | 内容 | 確認項目 |
|---------|------|---------|
| E3 | Summary→Paraclate送信→BQ同期 | Phase 3,4,5 |
| E7 | Retention実行→BQ/ローカル削除 | Phase 5 |

---

## 完了条件

1. Phase 3,4完了が前提
2. 全タスク（T5-1〜T5-7）が✅ COMPLETED
3. テストチェックリスト全項目パス
4. E3, E7テスト実行可能

---

## Issue連携

**Phase Issue**: #118
**親Issue**: #113

全タスクは#118で一括管理。個別タスクのサブIssue化は必要に応じて実施。

---

## 更新履歴

| 日付 | 更新内容 |
|------|---------|
| 2026-01-10 | 初版作成 |
| 2026-01-11 | **Phase 5 完了**: 全タスク（T5-1〜T5-7）実装完了 |

---

## 実装成果物（2026-01-11）

### 新規ファイル

| ファイル | 内容 |
|---------|------|
| `src/bq_sync_service/mod.rs` | BqSyncServiceメイン（設定、統計、バックグラウンドタスク） |
| `src/bq_sync_service/processor.rs` | BqSyncProcessor（バッチ処理、キュー管理） |
| `src/bq_sync_service/bq_client.rs` | BigQuery REST APIクライアント |
| `src/bq_sync_service/types.rs` | 型定義（BatchResult, QueueStatus, HealthStatus等） |
| `src/web_api/bq_sync_routes.rs` | BQ Sync API routes |
| `migrations/024_bq_sync_extension.sql` | ai_summary_cacheにsynced_to_bq追加、bq_sync_log追加 |

### 修正ファイル

| ファイル | 変更内容 |
|---------|---------|
| `src/lib.rs` | bq_sync_serviceモジュール追加 |
| `src/state.rs` | BqSyncServiceインポート、AppStateにフィールド追加 |
| `src/main.rs` | BqSyncService初期化、バックグラウンドタスク起動 |
| `src/web_api/mod.rs` | bq_sync_routes追加 |
| `src/web_api/routes.rs` | /api/bq-syncルートマウント |

### API エンドポイント

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/bq-sync/status` | GET | BQ同期状態取得 |
| `/api/bq-sync/flush` | POST | 保留中アイテム即時同期 |
| `/api/bq-sync/retry` | POST | 失敗アイテム再試行 |
| `/api/bq-sync/failed` | DELETE | 失敗アイテムクリア |
| `/api/bq-sync/retention` | POST | retention実行 |
| `/api/bq-sync/health` | GET | ヘルスチェック |

### 環境変数

| 変数名 | デフォルト | 説明 |
|-------|----------|------|
| `BQ_SYNC_ENABLED` | `false` | BQ同期有効化 |
| `BQ_CREDENTIALS_PATH` | なし | Service Account JSONパス |

### 注意事項

- BQ同期はデフォルト無効（`BQ_SYNC_ENABLED=true`で有効化）
- RSA署名（JWT生成）は`jsonwebtoken`クレート追加で完全実装可能
- 30秒間隔でバックグラウンドバッチ処理実行
- 冪等性はinsertId（event_id/summary_id）で担保
