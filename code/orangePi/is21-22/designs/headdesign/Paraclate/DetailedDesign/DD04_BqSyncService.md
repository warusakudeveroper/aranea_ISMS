# DD04: BQ同期サービス 詳細設計

作成日: 2026-01-10
対象: is22 Paraclate Server
ステータス: 詳細設計

## 1. 目的と概要

### 1.1 目的
is22の検出ログおよびSummaryをBigQueryへ同期し、長期保存・分析基盤を構築する。
非ブロッキングのバッチ処理により、本体処理への影響を最小化する。

### 1.2 同期対象

| ソーステーブル | BQテーブル | 同期タイミング |
|--------------|-----------|---------------|
| detection_logs | semantic_events | INSERT時にキュー追加 |
| ai_summary_cache | summaries | Summary生成時にキュー追加 |
| is21_logs | inference_logs | 推論完了時にキュー追加 |

### 1.3 スコープ
- bq_sync_queueへのエンキュー
- バッチ同期処理（非ブロッキング）
- リトライ・失敗管理
- 同期状態監視API

### 1.4 スコープ外
- BigQuery側のテーブル設計（mobes2.0側の責務）
- データ分析クエリ（Paraclate APP側）

## 2. 依存関係

### 2.1 参照ドキュメント
| ドキュメント | 参照セクション |
|-------------|---------------|
| Paraclate_BasicDesign.md | BQ同期設計 |
| 04_summary_api.md | bq_sync_queueスキーマ |

### 2.2 内部依存
| モジュール | 用途 |
|-----------|------|
| DD02 SummaryService | Summary生成→キューイング |
| detection_log_service | 検出ログ保存→キューイング |
| config_store | BQ接続設定取得 |

### 2.3 外部依存
| サービス | 用途 |
|---------|------|
| Google BigQuery | データ投入先 |
| Service Account | 認証 |

## 3. データ設計

### 3.1 既存テーブル: bq_sync_queue

```sql
CREATE TABLE bq_sync_queue (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    table_name VARCHAR(64) NOT NULL,
    record_id BIGINT UNSIGNED NOT NULL,
    operation ENUM('INSERT', 'UPDATE', 'DELETE') NOT NULL,
    payload JSON NOT NULL,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    synced_at DATETIME(3),
    sync_status ENUM('pending', 'syncing', 'completed', 'failed') DEFAULT 'pending',
    retry_count INT DEFAULT 0,
    max_retries INT DEFAULT 5,
    last_error TEXT,
    INDEX idx_status (sync_status),
    INDEX idx_table_status (table_name, sync_status),
    INDEX idx_created (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

### 3.2 ソーステーブルへの拡張

```sql
-- detection_logs に synced_to_bq カラム追加
ALTER TABLE detection_logs
ADD COLUMN synced_to_bq BOOLEAN DEFAULT FALSE,
ADD INDEX idx_synced (synced_to_bq);

-- ai_summary_cache に synced_to_bq カラム追加
ALTER TABLE ai_summary_cache
ADD COLUMN synced_to_bq BOOLEAN DEFAULT FALSE,
ADD INDEX idx_synced (synced_to_bq);
```

### 3.3 BigQuery側テーブル（参考）

**semantic_events** (mobes2.0のBQプロジェクト)
```sql
CREATE TABLE `project.dataset.semantic_events` (
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
```

**summaries** (mobes2.0のBQプロジェクト)
```sql
CREATE TABLE `project.dataset.summaries` (
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

## 4. API設計

### 4.1 is22側内部API

#### GET /api/bq-sync/status
BQ同期状態を取得

**Response**
```json
{
  "queueStats": {
    "pending": 42,
    "syncing": 5,
    "completed": 12345,
    "failed": 3
  },
  "lastSyncAt": "2026-01-10T12:00:00Z",
  "lastErrorAt": "2026-01-10T11:30:00Z",
  "lastError": "BigQuery insert failed: quota exceeded",
  "tableStats": {
    "detection_logs": {
      "pending": 30,
      "synced": 10000
    },
    "ai_summary_cache": {
      "pending": 12,
      "synced": 500
    }
  }
}
```

#### POST /api/bq-sync/retry
失敗したアイテムを再試行

**Request**
```json
{
  "ids": [123, 456, 789]
}
```

**Response**
```json
{
  "ok": true,
  "retriedCount": 3
}
```

#### POST /api/bq-sync/flush
保留中のアイテムを即時同期

**Response**
```json
{
  "ok": true,
  "flushedCount": 42
}
```

#### DELETE /api/bq-sync/failed
失敗アイテムをクリア

**Response**
```json
{
  "ok": true,
  "clearedCount": 3
}
```

## 5. モジュール構造

### 5.1 ディレクトリ構成

```
src/
├── bq_sync_service/
│   ├── mod.rs              # モジュールエクスポート
│   ├── types.rs            # 型定義
│   ├── enqueuer.rs         # キューイング処理
│   ├── processor.rs        # バッチ同期処理
│   ├── bq_client.rs        # BigQueryクライアント
│   └── repository.rs       # DB操作
├── web_api/
│   └── bq_sync_routes.rs   # APIルート（新規追加）
```

### 5.2 型定義 (types.rs)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 同期対象テーブル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncTable {
    DetectionLogs,
    AiSummaryCache,
    Is21Logs,
}

impl SyncTable {
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncTable::DetectionLogs => "detection_logs",
            SyncTable::AiSummaryCache => "ai_summary_cache",
            SyncTable::Is21Logs => "is21_logs",
        }
    }

    pub fn bq_table(&self) -> &'static str {
        match self {
            SyncTable::DetectionLogs => "semantic_events",
            SyncTable::AiSummaryCache => "summaries",
            SyncTable::Is21Logs => "inference_logs",
        }
    }
}

/// 同期操作種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "UPPERCASE")]
pub enum SyncOperation {
    Insert,
    Update,
    Delete,
}

/// 同期ステータス
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum SyncStatus {
    Pending,
    Syncing,
    Completed,
    Failed,
}

/// 同期キューアイテム
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncQueueItem {
    pub id: u64,
    pub table_name: String,
    pub record_id: u64,
    pub operation: SyncOperation,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
    pub sync_status: SyncStatus,
    pub retry_count: i32,
    pub max_retries: i32,
    pub last_error: Option<String>,
}

/// キュー統計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending: u64,
    pub syncing: u64,
    pub completed: u64,
    pub failed: u64,
}

/// テーブル別統計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStats {
    pub pending: u64,
    pub synced: u64,
}

/// BQ同期設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BqSyncConfig {
    pub project_id: String,
    pub dataset_id: String,
    pub batch_size: usize,
    pub interval_seconds: u64,
    pub enabled: bool,
}
```

### 5.3 キューイング処理 (enqueuer.rs)

```rust
use crate::bq_sync_service::{types::*, repository::BqSyncRepository};

pub struct BqSyncEnqueuer {
    repository: BqSyncRepository,
}

impl BqSyncEnqueuer {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self {
            repository: BqSyncRepository::new(pool),
        }
    }

    /// 検出ログをキューに追加
    pub async fn enqueue_detection_log(
        &self,
        log_id: u64,
        log_data: &DetectionLog,
    ) -> Result<u64, BqSyncError> {
        let payload = self.build_detection_payload(log_data)?;

        self.repository.enqueue(SyncQueueItem {
            id: 0,
            table_name: SyncTable::DetectionLogs.as_str().to_string(),
            record_id: log_id,
            operation: SyncOperation::Insert,
            payload,
            created_at: Utc::now(),
            synced_at: None,
            sync_status: SyncStatus::Pending,
            retry_count: 0,
            max_retries: 5,
            last_error: None,
        }).await
    }

    /// Summaryをキューに追加
    pub async fn enqueue_summary(
        &self,
        summary_id: u64,
        summary_data: &SummaryResult,
    ) -> Result<u64, BqSyncError> {
        let payload = self.build_summary_payload(summary_data)?;

        self.repository.enqueue(SyncQueueItem {
            id: 0,
            table_name: SyncTable::AiSummaryCache.as_str().to_string(),
            record_id: summary_id,
            operation: SyncOperation::Insert,
            payload,
            created_at: Utc::now(),
            synced_at: None,
            sync_status: SyncStatus::Pending,
            retry_count: 0,
            max_retries: 5,
            last_error: None,
        }).await
    }

    /// 検出ログ用ペイロード構築
    fn build_detection_payload(
        &self,
        log: &DetectionLog,
    ) -> Result<serde_json::Value, BqSyncError> {
        Ok(json!({
            "event_id": format!("det_{}", log.id),
            "tid": log.tid,
            "fid": log.fid,
            "camera_lacis_id": log.camera_lacis_id,
            "detected_at": log.detected_at.to_rfc3339(),
            "severity": log.severity,
            "detection_type": log.detection_type,
            "detection_detail": log.detection_detail,
            "tags": log.tags,
            "vehicle_ocr": log.vehicle_ocr,
            "face_features": log.face_features,
            "snapshot_url": log.snapshot_url,
            "source_device": "is22"
        }))
    }

    /// Summary用ペイロード構築
    fn build_summary_payload(
        &self,
        summary: &SummaryResult,
    ) -> Result<serde_json::Value, BqSyncError> {
        Ok(json!({
            "summary_id": format!("sum_{}", summary.summary_id),
            "tid": summary.tid,
            "fid": summary.fid,
            "summary_type": format!("{:?}", summary.summary_type).to_lowercase(),
            "period_start": summary.period_start.to_rfc3339(),
            "period_end": summary.period_end.to_rfc3339(),
            "summary_text": summary.summary_text,
            "summary_json": summary.summary_json,
            "detection_count": summary.detection_count,
            "severity_max": summary.severity_max,
            "camera_ids": summary.camera_ids,
            "created_at": summary.created_at.to_rfc3339()
        }))
    }
}
```

### 5.4 バッチ同期処理 (processor.rs)

```rust
use crate::bq_sync_service::{types::*, repository::BqSyncRepository, bq_client::BqClient};

pub struct BqSyncProcessor {
    repository: BqSyncRepository,
    bq_client: Arc<BqClient>,
    config: BqSyncConfig,
}

impl BqSyncProcessor {
    pub fn new(
        pool: Pool<MySql>,
        bq_client: Arc<BqClient>,
        config: BqSyncConfig,
    ) -> Self {
        Self {
            repository: BqSyncRepository::new(pool),
            bq_client,
            config,
        }
    }

    /// バックグラウンド処理を開始
    pub async fn start(self: Arc<Self>) {
        if !self.config.enabled {
            tracing::info!("BQ sync disabled");
            return;
        }

        tokio::spawn(async move {
            loop {
                if let Err(e) = self.process_batch().await {
                    tracing::error!("BQ sync batch error: {}", e);
                }

                tokio::time::sleep(Duration::from_secs(self.config.interval_seconds)).await;
            }
        });
    }

    /// バッチ処理実行
    pub async fn process_batch(&self) -> Result<ProcessResult, BqSyncError> {
        // pending状態のアイテムを取得
        let items = self.repository
            .get_pending_items(self.config.batch_size)
            .await?;

        if items.is_empty() {
            return Ok(ProcessResult { processed: 0, failed: 0 });
        }

        tracing::info!("Processing {} BQ sync items", items.len());

        // テーブルごとにグループ化
        let mut grouped: HashMap<String, Vec<SyncQueueItem>> = HashMap::new();
        for item in items {
            grouped.entry(item.table_name.clone())
                .or_default()
                .push(item);
        }

        let mut processed = 0;
        let mut failed = 0;

        // テーブルごとにバッチ投入
        for (table_name, table_items) in grouped {
            let sync_table = match table_name.as_str() {
                "detection_logs" => SyncTable::DetectionLogs,
                "ai_summary_cache" => SyncTable::AiSummaryCache,
                "is21_logs" => SyncTable::Is21Logs,
                _ => continue,
            };

            let bq_table = sync_table.bq_table();
            let ids: Vec<u64> = table_items.iter().map(|i| i.id).collect();

            // ステータスを syncing に更新
            self.repository.mark_syncing(&ids).await?;

            // ペイロードを収集
            let rows: Vec<serde_json::Value> = table_items
                .iter()
                .map(|i| i.payload.clone())
                .collect();

            // BigQueryへ投入
            match self.bq_client.insert_rows(bq_table, &rows).await {
                Ok(_) => {
                    // 成功: completed に更新
                    self.repository.mark_completed(&ids).await?;

                    // ソーステーブルの synced_to_bq を更新
                    for item in &table_items {
                        self.update_source_synced_flag(&item.table_name, item.record_id).await?;
                    }

                    processed += table_items.len();
                    tracing::info!(
                        "BQ sync: {} rows inserted to {}",
                        table_items.len(),
                        bq_table
                    );
                }
                Err(e) => {
                    // 失敗: retry_count 増加 or failed に更新
                    for item in &table_items {
                        if item.retry_count + 1 >= item.max_retries {
                            self.repository.mark_failed(item.id, &e.to_string()).await?;
                            failed += 1;
                        } else {
                            self.repository.increment_retry(item.id, &e.to_string()).await?;
                        }
                    }

                    tracing::error!(
                        "BQ sync failed for {}: {}",
                        bq_table,
                        e
                    );
                }
            }
        }

        Ok(ProcessResult { processed, failed })
    }

    /// ソーステーブルの同期フラグを更新
    async fn update_source_synced_flag(
        &self,
        table_name: &str,
        record_id: u64,
    ) -> Result<(), BqSyncError> {
        let query = format!(
            "UPDATE {} SET synced_to_bq = TRUE WHERE id = ?",
            table_name
        );

        sqlx::query(&query)
            .bind(record_id)
            .execute(self.repository.pool())
            .await?;

        Ok(())
    }

    /// 失敗アイテムの再試行
    pub async fn retry_failed(&self, ids: &[u64]) -> Result<u64, BqSyncError> {
        self.repository.reset_to_pending(ids).await
    }

    /// 保留中アイテムの即時フラッシュ
    pub async fn flush(&self) -> Result<ProcessResult, BqSyncError> {
        // 全pending を処理
        let mut total = ProcessResult { processed: 0, failed: 0 };

        loop {
            let result = self.process_batch().await?;
            total.processed += result.processed;
            total.failed += result.failed;

            if result.processed == 0 {
                break;
            }
        }

        Ok(total)
    }
}

#[derive(Debug, Clone)]
pub struct ProcessResult {
    pub processed: usize,
    pub failed: usize,
}
```

### 5.5 BigQueryクライアント (bq_client.rs)

```rust
use google_cloud_bigquery::client::Client;

pub struct BqClient {
    client: Client,
    project_id: String,
    dataset_id: String,
}

impl BqClient {
    pub async fn new(config: &BqSyncConfig) -> Result<Self, BqSyncError> {
        // Service Account認証
        let client = Client::new(&config.project_id).await?;

        Ok(Self {
            client,
            project_id: config.project_id.clone(),
            dataset_id: config.dataset_id.clone(),
        })
    }

    /// 行をBigQueryに挿入
    pub async fn insert_rows(
        &self,
        table_name: &str,
        rows: &[serde_json::Value],
    ) -> Result<(), BqSyncError> {
        if rows.is_empty() {
            return Ok(());
        }

        let table_ref = format!(
            "{}.{}.{}",
            self.project_id,
            self.dataset_id,
            table_name
        );

        // ストリーミング挿入
        let insert_request = InsertAllRequest {
            skip_invalid_rows: Some(false),
            ignore_unknown_values: Some(false),
            rows: rows.iter().map(|r| InsertRow {
                insert_id: None,
                json: r.clone(),
            }).collect(),
        };

        let response = self.client
            .tabledata()
            .insert_all(&self.project_id, &self.dataset_id, table_name, insert_request)
            .await?;

        // エラーチェック
        if let Some(errors) = response.insert_errors {
            if !errors.is_empty() {
                let error_msg = errors.iter()
                    .take(3)
                    .map(|e| format!("Row {}: {:?}", e.index, e.errors))
                    .collect::<Vec<_>>()
                    .join("; ");

                return Err(BqSyncError::InsertError(error_msg));
            }
        }

        Ok(())
    }

    /// 接続テスト
    pub async fn test_connection(&self) -> Result<bool, BqSyncError> {
        let datasets = self.client
            .dataset()
            .list(&self.project_id)
            .await?;

        let found = datasets.iter()
            .any(|d| d.dataset_reference.dataset_id == self.dataset_id);

        Ok(found)
    }
}
```

## 6. 処理フロー

### 6.1 キューイングフロー

```
┌─────────────────────────────────────────────────────────────┐
│           detection_log_service                              │
│           INSERT detection_log                               │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ BqSyncEnqueuer.       │
              │  enqueue_detection_log│
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ bq_sync_queue         │
              │ INSERT (pending)      │
              └───────────────────────┘
```

### 6.2 バッチ同期フロー

```
┌─────────────────────────────────────────────────────────────┐
│           BqSyncProcessor (interval: 30sec)                  │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ get_pending_items     │
              │ (batch_size: 100)     │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ テーブル別グループ化   │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ mark_syncing          │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ BqClient.insert_rows  │
              │ (Streaming Insert)    │
              └───────────────────────┘
                          │
           ┌──────────────┴──────────────┐
           │                             │
        [成功]                        [失敗]
           │                             │
           ▼                             ▼
    ┌─────────────────┐          ┌───────────────────┐
    │ mark_completed  │          │ retry_count < max │
    │ synced_to_bq=1  │          │   → increment     │
    └─────────────────┘          │ retry_count >= max│
                                 │   → mark_failed   │
                                 └───────────────────┘
```

## 7. エラーハンドリング

| エラー | 原因 | 対応 |
|-------|------|------|
| QuotaExceeded | BQ書き込み制限超過 | バッチサイズ縮小、間隔延長 |
| AuthenticationError | Service Account無効 | 認証情報再設定 |
| TableNotFound | BQテーブル未作成 | mobes2.0側でテーブル作成を依頼 |
| InvalidRow | ペイロード形式不正 | スキップして続行、ログ出力 |
| NetworkError | ネットワーク断 | 自動リトライ |

### リトライ戦略

```rust
// 指数バックオフ
fn calculate_backoff(retry_count: i32) -> Duration {
    let base_seconds = 5;
    let max_seconds = 300; // 5分
    let seconds = (base_seconds * 2i64.pow(retry_count as u32)).min(max_seconds);
    Duration::from_secs(seconds as u64)
}
```

## 8. 監視・アラート

### 8.1 メトリクス

| メトリクス | 閾値 | アクション |
|-----------|------|-----------|
| pending_count | > 1000 | 警告ログ |
| failed_count | > 10 | アラート通知 |
| sync_latency | > 5分 | 調査開始 |

### 8.2 ヘルスチェック

```rust
pub async fn health_check(&self) -> HealthStatus {
    let stats = self.repository.get_stats().await.unwrap_or_default();

    let status = if stats.failed > 10 {
        HealthStatus::Unhealthy
    } else if stats.pending > 500 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    };

    status
}
```

## 9. テスト計画

### 9.1 ユニットテスト

| テストケース | 内容 |
|-------------|------|
| test_enqueue_detection | キュー追加成功 |
| test_build_payload | ペイロード形式検証 |
| test_batch_grouping | テーブル別グループ化 |

### 9.2 結合テスト

| テストケース | 前提条件 | 期待結果 |
|-------------|---------|---------|
| 正常同期 | BQ接続OK | completed、synced_to_bq=true |
| リトライ | BQ一時エラー | retry_count増加、再同期成功 |
| 失敗 | 5回失敗 | status=failed |

### 9.3 E2Eテスト

| テストケース | 手順 | 確認項目 |
|-------------|------|---------|
| 検出→BQ反映 | 検出ログINSERT | BQテーブルで確認 |
| Summary→BQ反映 | Summary生成 | BQテーブルで確認 |
| フラッシュ | API呼び出し | pending=0 |

## 10. MECE/SOLID確認

### 10.1 MECE確認
- **網羅性**: キューイング/バッチ処理/リトライ/監視を全カバー
- **重複排除**: ソース保存とBQ同期を分離
- **未カバー領域**: BQテーブル設計はmobes2.0側

### 10.2 SOLID確認
- **SRP**: Enqueuer/Processor/BqClient各々単一責務
- **OCP**: 新テーブル追加はSyncTableとbq_table()追加のみ
- **LSP**: SyncQueueItem共通構造
- **ISP**: キュー操作/同期処理/監視APIを分離
- **DIP**: BqClient抽象に依存

## 11. マイグレーション

### 11.1 SQLマイグレーション

ファイル: `migrations/021_bq_sync_extension.sql`

```sql
-- Migration: 021_bq_sync_extension
-- Description: BQ同期用カラム追加

-- detection_logs に synced_to_bq 追加
ALTER TABLE detection_logs
ADD COLUMN synced_to_bq BOOLEAN DEFAULT FALSE,
ADD INDEX idx_synced (synced_to_bq);

-- ai_summary_cache に synced_to_bq 追加
ALTER TABLE ai_summary_cache
ADD COLUMN synced_to_bq BOOLEAN DEFAULT FALSE,
ADD INDEX idx_synced (synced_to_bq);

-- bq_sync_queue に max_retries, last_error 追加（なければ）
ALTER TABLE bq_sync_queue
ADD COLUMN IF NOT EXISTS retry_count INT DEFAULT 0,
ADD COLUMN IF NOT EXISTS max_retries INT DEFAULT 5,
ADD COLUMN IF NOT EXISTS last_error TEXT;
```

### 11.2 設定ファイル

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

**SSoT宣言**: BQテーブル定義はmobes2.0側が正。is22はbq_sync_queueでキュー管理。
**MECE確認**: キューイング/バッチ処理/リトライ/監視を網羅、重複なし。
