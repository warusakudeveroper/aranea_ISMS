//! ParaclateClient Repository
//!
//! Phase 4: ParaclateClient (Issue #117)
//! DD03_ParaclateClient.md準拠
//!
//! ## 概要
//! Paraclate設定・送信キュー・接続ログのDB操作

use crate::paraclate_client::types::{
    calculate_retry_delay, defaults, ConnectionEventType, ConnectionLog, ConnectionLogInsert,
    ConnectionStatus, ParaclateConfig, ParaclateConfigInsert, ParaclateConfigUpdate, PayloadType,
    QueueStats, QueueStatus, SendQueueInsert, SendQueueItem, SendQueueUpdate,
};
use chrono::{DateTime, Duration, Utc};
use sqlx::{MySqlPool, Row};

// ============================================================
// Config Repository
// ============================================================

/// Paraclate設定リポジトリ
pub struct ConfigRepository {
    pool: MySqlPool,
}

impl Clone for ConfigRepository {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

impl ConfigRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// 設定を取得（tid/fid指定）
    pub async fn get(&self, tid: &str, fid: &str) -> Result<Option<ParaclateConfig>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                config_id, tid, fid, endpoint,
                report_interval_minutes, grand_summary_times, retention_days, attunement,
                sync_source_timestamp, last_sync_at, connection_status, last_error,
                created_at, updated_at
            FROM paraclate_config
            WHERE tid = ? AND fid = ?
            "#,
        )
        .bind(tid)
        .bind(fid)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| self.row_to_config(&r)))
    }

    /// 設定を取得（config_id指定）
    pub async fn get_by_id(&self, config_id: u32) -> Result<Option<ParaclateConfig>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                config_id, tid, fid, endpoint,
                report_interval_minutes, grand_summary_times, retention_days, attunement,
                sync_source_timestamp, last_sync_at, connection_status, last_error,
                created_at, updated_at
            FROM paraclate_config
            WHERE config_id = ?
            "#,
        )
        .bind(config_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| self.row_to_config(&r)))
    }

    /// 設定を挿入
    pub async fn insert(&self, insert: ParaclateConfigInsert) -> Result<u32, sqlx::Error> {
        let grand_summary_times = insert
            .grand_summary_times
            .unwrap_or_else(defaults::grand_summary_times);
        let attunement = insert.attunement.unwrap_or(serde_json::json!({}));

        let result = sqlx::query(
            r#"
            INSERT INTO paraclate_config
                (tid, fid, endpoint, report_interval_minutes, grand_summary_times, retention_days, attunement)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&insert.tid)
        .bind(&insert.fid)
        .bind(&insert.endpoint)
        .bind(insert.report_interval_minutes.unwrap_or(defaults::REPORT_INTERVAL_MINUTES))
        .bind(serde_json::to_string(&grand_summary_times).unwrap())
        .bind(insert.retention_days.unwrap_or(defaults::RETENTION_DAYS))
        .bind(attunement.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id() as u32)
    }

    /// 設定を更新
    pub async fn update(
        &self,
        tid: &str,
        fid: &str,
        update: ParaclateConfigUpdate,
    ) -> Result<bool, sqlx::Error> {
        let mut query = String::from("UPDATE paraclate_config SET updated_at = NOW(3)");
        let mut binds: Vec<String> = vec![];

        if let Some(endpoint) = &update.endpoint {
            query.push_str(", endpoint = ?");
            binds.push(endpoint.clone());
        }
        if let Some(interval) = update.report_interval_minutes {
            query.push_str(", report_interval_minutes = ?");
            binds.push(interval.to_string());
        }
        if let Some(times) = &update.grand_summary_times {
            query.push_str(", grand_summary_times = ?");
            binds.push(serde_json::to_string(times).unwrap());
        }
        if let Some(days) = update.retention_days {
            query.push_str(", retention_days = ?");
            binds.push(days.to_string());
        }
        if let Some(att) = &update.attunement {
            query.push_str(", attunement = ?");
            binds.push(att.to_string());
        }
        if let Some(ts) = update.sync_source_timestamp {
            query.push_str(", sync_source_timestamp = ?");
            binds.push(ts.to_rfc3339());
        }

        query.push_str(" WHERE tid = ? AND fid = ?");

        let mut q = sqlx::query(&query);
        for bind in &binds {
            q = q.bind(bind);
        }
        q = q.bind(tid).bind(fid);

        let result = q.execute(&self.pool).await?;
        Ok(result.rows_affected() > 0)
    }

    /// 接続状態を更新
    pub async fn update_connection_status(
        &self,
        tid: &str,
        fid: &str,
        status: ConnectionStatus,
        error: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE paraclate_config
            SET connection_status = ?, last_error = ?, last_sync_at = IF(? = 'connected', NOW(3), last_sync_at)
            WHERE tid = ? AND fid = ?
            "#,
        )
        .bind(status.to_string())
        .bind(error)
        .bind(status.to_string())
        .bind(tid)
        .bind(fid)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 全設定を取得（有効なもののみ）
    pub async fn get_all_connected(&self) -> Result<Vec<ParaclateConfig>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                config_id, tid, fid, endpoint,
                report_interval_minutes, grand_summary_times, retention_days, attunement,
                sync_source_timestamp, last_sync_at, connection_status, last_error,
                created_at, updated_at
            FROM paraclate_config
            WHERE connection_status = 'connected'
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| self.row_to_config(r)).collect())
    }

    /// 設定を削除
    pub async fn delete(&self, tid: &str, fid: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM paraclate_config WHERE tid = ? AND fid = ?")
            .bind(tid)
            .bind(fid)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 設定が存在しない場合、scan_subnetsから自動作成
    ///
    /// ## ロジック
    /// 1. 既存configがあればそれを返す
    /// 2. なければscan_subnetsでtid/fidの組み合わせを検証
    /// 3. 有効なら同tidの既存configからendpointを取得してconfig作成
    pub async fn ensure_config(&self, tid: &str, fid: &str) -> Result<Option<ParaclateConfig>, sqlx::Error> {
        // 既存configチェック
        if let Some(config) = self.get(tid, fid).await? {
            return Ok(Some(config));
        }

        // scan_subnetsでtid/fid検証
        let subnet_valid: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT fid
            FROM scan_subnets
            WHERE tid = ? AND fid = ?
            LIMIT 1
            "#,
        )
        .bind(tid)
        .bind(fid)
        .fetch_optional(&self.pool)
        .await?;

        if subnet_valid.is_none() {
            // 無効なtid/fid組み合わせ（scan_subnetsに存在しない）
            tracing::debug!(tid = %tid, fid = %fid, "tid/fid not found in scan_subnets, skipping auto-create");
            return Ok(None);
        }

        // 同じtidの既存configからendpointを取得
        let existing_endpoint: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT endpoint
            FROM paraclate_config
            WHERE tid = ? AND connection_status = 'connected'
            LIMIT 1
            "#,
        )
        .bind(tid)
        .fetch_optional(&self.pool)
        .await?;

        // デフォルトエンドポイント（mobes2.0 Cloud Functions）
        let endpoint = existing_endpoint
            .map(|e| e.0)
            .unwrap_or_else(|| "https://asia-northeast1-mobesorder.cloudfunctions.net".to_string());

        // Config自動作成
        tracing::info!(tid = %tid, fid = %fid, endpoint = %endpoint, "Auto-creating paraclate_config from scan_subnets");

        let config_id = self.insert(ParaclateConfigInsert {
            tid: tid.to_string(),
            fid: fid.to_string(),
            endpoint,
            report_interval_minutes: None,
            grand_summary_times: None,
            retention_days: None,
            attunement: None,
        }).await?;

        // 作成したconfigを返す
        self.get_by_id(config_id).await
    }

    fn row_to_config(&self, row: &sqlx::mysql::MySqlRow) -> ParaclateConfig {
        let grand_summary_json: String = row.get("grand_summary_times");
        let grand_summary_times: Vec<String> =
            serde_json::from_str(&grand_summary_json).unwrap_or_else(|_| defaults::grand_summary_times());

        let attunement_json: String = row.get("attunement");
        let attunement: serde_json::Value =
            serde_json::from_str(&attunement_json).unwrap_or(serde_json::json!({}));

        let status_str: String = row.get("connection_status");

        ParaclateConfig {
            config_id: row.get("config_id"),
            tid: row.get("tid"),
            fid: row.get("fid"),
            endpoint: row.get("endpoint"),
            report_interval_minutes: row.get("report_interval_minutes"),
            grand_summary_times,
            retention_days: row.get("retention_days"),
            attunement,
            sync_source_timestamp: row.get("sync_source_timestamp"),
            last_sync_at: row.get("last_sync_at"),
            connection_status: ConnectionStatus::from(status_str.as_str()),
            last_error: row.get("last_error"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

// ============================================================
// Send Queue Repository
// ============================================================

/// 送信キューリポジトリ
pub struct SendQueueRepository {
    pool: MySqlPool,
}

impl Clone for SendQueueRepository {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

impl SendQueueRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// キュー項目を挿入
    pub async fn insert(&self, insert: SendQueueInsert) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO paraclate_send_queue
                (tid, fid, payload_type, payload, reference_id, max_retries)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&insert.tid)
        .bind(&insert.fid)
        .bind(insert.payload_type.to_string())
        .bind(insert.payload.to_string())
        .bind(insert.reference_id)
        .bind(insert.max_retries.unwrap_or(defaults::MAX_RETRIES))
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id())
    }

    /// キュー項目を取得
    pub async fn get(&self, queue_id: u64) -> Result<Option<SendQueueItem>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                queue_id, tid, fid, payload_type, payload, reference_id,
                status, retry_count, max_retries, next_retry_at,
                last_error, http_status_code, created_at, sent_at
            FROM paraclate_send_queue
            WHERE queue_id = ?
            "#,
        )
        .bind(queue_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| self.row_to_item(&r)))
    }

    /// 送信待ちキューを取得（リトライ対象含む）
    pub async fn get_pending(&self, tid: &str, fid: &str, limit: i32) -> Result<Vec<SendQueueItem>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                queue_id, tid, fid, payload_type, payload, reference_id,
                status, retry_count, max_retries, next_retry_at,
                last_error, http_status_code, created_at, sent_at
            FROM paraclate_send_queue
            WHERE tid = ? AND fid = ?
              AND (status = 'pending' OR (status = 'failed' AND retry_count < max_retries AND (next_retry_at IS NULL OR next_retry_at <= NOW(3))))
            ORDER BY created_at ASC
            LIMIT ?
            "#,
        )
        .bind(tid)
        .bind(fid)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| self.row_to_item(r)).collect())
    }

    /// キュー項目のステータスを更新
    pub async fn update_status(&self, queue_id: u64, update: SendQueueUpdate) -> Result<bool, sqlx::Error> {
        let mut query = String::from("UPDATE paraclate_send_queue SET status = ?");
        let mut params: Vec<Option<String>> = vec![Some(update.status.to_string())];

        if let Some(count) = update.retry_count {
            query.push_str(", retry_count = ?");
            params.push(Some(count.to_string()));
        }
        if let Some(next) = update.next_retry_at {
            query.push_str(", next_retry_at = ?");
            // MySQL datetime(3) format: "YYYY-MM-DD HH:MM:SS.mmm"
            params.push(Some(next.format("%Y-%m-%d %H:%M:%S%.3f").to_string()));
        }
        if let Some(error) = &update.last_error {
            query.push_str(", last_error = ?");
            params.push(Some(error.clone()));
        }
        if let Some(code) = update.http_status_code {
            query.push_str(", http_status_code = ?");
            params.push(Some(code.to_string()));
        }
        if let Some(sent) = update.sent_at {
            query.push_str(", sent_at = ?");
            // MySQL datetime(3) format: "YYYY-MM-DD HH:MM:SS.mmm"
            params.push(Some(sent.format("%Y-%m-%d %H:%M:%S%.3f").to_string()));
        }

        query.push_str(" WHERE queue_id = ?");

        let mut q = sqlx::query(&query);
        for param in &params {
            q = q.bind(param.as_deref());
        }
        q = q.bind(queue_id);

        let result = q.execute(&self.pool).await?;
        Ok(result.rows_affected() > 0)
    }

    /// 送信済みに更新
    pub async fn mark_sent(&self, queue_id: u64) -> Result<bool, sqlx::Error> {
        self.update_status(
            queue_id,
            SendQueueUpdate {
                status: QueueStatus::Sent,
                retry_count: None,
                next_retry_at: None,
                last_error: None,
                http_status_code: Some(200),
                sent_at: Some(Utc::now()),
            },
        )
        .await
    }

    /// 失敗に更新（リトライスケジュール）
    pub async fn mark_failed(
        &self,
        queue_id: u64,
        error: &str,
        http_code: Option<i32>,
        retry_count: i32,
    ) -> Result<bool, sqlx::Error> {
        let next_retry = Utc::now() + Duration::from_std(calculate_retry_delay(retry_count)).unwrap();

        self.update_status(
            queue_id,
            SendQueueUpdate {
                status: QueueStatus::Failed,
                retry_count: Some(retry_count),
                next_retry_at: Some(next_retry),
                last_error: Some(error.to_string()),
                http_status_code: http_code,
                sent_at: None,
            },
        )
        .await
    }

    /// キュー統計を取得
    pub async fn get_stats(&self, tid: &str, fid: &str) -> Result<QueueStats, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                CAST(COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) AS SIGNED) as pending,
                CAST(COALESCE(SUM(CASE WHEN status = 'sending' THEN 1 ELSE 0 END), 0) AS SIGNED) as sending,
                CAST(COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0) AS SIGNED) as failed,
                CAST(COALESCE(SUM(CASE WHEN status = 'sent' AND sent_at >= DATE(NOW()) THEN 1 ELSE 0 END), 0) AS SIGNED) as sent_today
            FROM paraclate_send_queue
            WHERE tid = ? AND fid = ?
            "#,
        )
        .bind(tid)
        .bind(fid)
        .fetch_one(&self.pool)
        .await?;

        Ok(QueueStats {
            pending: row.get::<i64, _>("pending") as u64,
            sending: row.get::<i64, _>("sending") as u64,
            failed: row.get::<i64, _>("failed") as u64,
            sent_today: row.get::<i64, _>("sent_today") as u64,
        })
    }

    /// 古いキュー項目を削除
    pub async fn cleanup_old(&self, days: i32) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM paraclate_send_queue
            WHERE status IN ('sent', 'skipped')
              AND created_at < DATE_SUB(NOW(), INTERVAL ? DAY)
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// 一覧取得
    pub async fn list(
        &self,
        tid: &str,
        fid: &str,
        status: Option<QueueStatus>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<SendQueueItem>, sqlx::Error> {
        let query = if let Some(s) = status {
            sqlx::query(
                r#"
                SELECT
                    queue_id, tid, fid, payload_type, payload, reference_id,
                    status, retry_count, max_retries, next_retry_at,
                    last_error, http_status_code, created_at, sent_at
                FROM paraclate_send_queue
                WHERE tid = ? AND fid = ? AND status = ?
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(tid)
            .bind(fid)
            .bind(s.to_string())
            .bind(limit)
            .bind(offset)
        } else {
            sqlx::query(
                r#"
                SELECT
                    queue_id, tid, fid, payload_type, payload, reference_id,
                    status, retry_count, max_retries, next_retry_at,
                    last_error, http_status_code, created_at, sent_at
                FROM paraclate_send_queue
                WHERE tid = ? AND fid = ?
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(tid)
            .bind(fid)
            .bind(limit)
            .bind(offset)
        };

        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows.iter().map(|r| self.row_to_item(r)).collect())
    }

    fn row_to_item(&self, row: &sqlx::mysql::MySqlRow) -> SendQueueItem {
        let payload_str: String = row.get("payload");
        let payload: serde_json::Value =
            serde_json::from_str(&payload_str).unwrap_or(serde_json::json!({}));

        let type_str: String = row.get("payload_type");
        let status_str: String = row.get("status");

        SendQueueItem {
            queue_id: row.get("queue_id"),
            tid: row.get("tid"),
            fid: row.get("fid"),
            payload_type: PayloadType::from(type_str.as_str()),
            payload,
            reference_id: row.get("reference_id"),
            status: QueueStatus::from(status_str.as_str()),
            retry_count: row.get("retry_count"),
            max_retries: row.get("max_retries"),
            next_retry_at: row.get("next_retry_at"),
            last_error: row.get("last_error"),
            http_status_code: row.get("http_status_code"),
            created_at: row.get("created_at"),
            sent_at: row.get("sent_at"),
        }
    }
}

// ============================================================
// Connection Log Repository
// ============================================================

/// 接続ログリポジトリ
pub struct ConnectionLogRepository {
    pool: MySqlPool,
}

impl Clone for ConnectionLogRepository {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

impl ConnectionLogRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// ログを挿入
    pub async fn insert(&self, insert: ConnectionLogInsert) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO paraclate_connection_log
                (tid, fid, event_type, event_detail, error_code, http_status_code)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&insert.tid)
        .bind(&insert.fid)
        .bind(insert.event_type.to_string())
        .bind(&insert.event_detail)
        .bind(&insert.error_code)
        .bind(insert.http_status_code)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id())
    }

    /// 最近のログを取得
    pub async fn get_recent(
        &self,
        tid: &str,
        fid: &str,
        limit: i32,
    ) -> Result<Vec<ConnectionLog>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                log_id, tid, fid, event_type, event_detail,
                error_code, http_status_code, created_at
            FROM paraclate_connection_log
            WHERE tid = ? AND fid = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(tid)
        .bind(fid)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| self.row_to_log(r)).collect())
    }

    /// 古いログを削除
    pub async fn cleanup_old(&self, days: i32) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM paraclate_connection_log
            WHERE created_at < DATE_SUB(NOW(), INTERVAL ? DAY)
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    fn row_to_log(&self, row: &sqlx::mysql::MySqlRow) -> ConnectionLog {
        let type_str: String = row.get("event_type");
        let event_type = match type_str.as_str() {
            "connect" => ConnectionEventType::Connect,
            "disconnect" => ConnectionEventType::Disconnect,
            "sync" => ConnectionEventType::Sync,
            "retry" => ConnectionEventType::Retry,
            _ => ConnectionEventType::Error,
        };

        ConnectionLog {
            log_id: row.get("log_id"),
            tid: row.get("tid"),
            fid: row.get("fid"),
            event_type,
            event_detail: row.get("event_detail"),
            error_code: row.get("error_code"),
            http_status_code: row.get("http_status_code"),
            created_at: row.get("created_at"),
        }
    }
}
