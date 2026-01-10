//! Summary Repository
//!
//! Phase 3: Summary/GrandSummary (Issue #116)
//! T3-3: ai_summary_cache リポジトリ
//!
//! ## テーブル
//! - ai_summary_cache: Summary保存
//! - scheduled_reports: スケジュール設定

use super::types::{
    ReportSchedule, ReportType, SummaryInsert, SummaryResult, SummaryType,
};
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use tracing::{debug, info};

// ============================================================
// Summary Repository
// ============================================================

/// Summary保存・取得リポジトリ
pub struct SummaryRepository {
    pool: MySqlPool,
}

impl SummaryRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// プールへの参照を取得
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    /// Summaryを保存
    pub async fn insert(&self, data: SummaryInsert) -> crate::Result<SummaryResult> {
        let result = sqlx::query(
            r#"
            INSERT INTO ai_summary_cache
                (tid, fid, summary_type, period_start, period_end,
                 summary_text, summary_json, detection_count, severity_max,
                 camera_ids, expires_at)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&data.tid)
        .bind(&data.fid)
        .bind(data.summary_type.to_string())
        .bind(data.period_start)
        .bind(data.period_end)
        .bind(&data.summary_text)
        .bind(&data.summary_json)
        .bind(data.detection_count)
        .bind(data.severity_max)
        .bind(&data.camera_ids)
        .bind(data.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        let summary_id = result.last_insert_id();

        info!(
            summary_id = summary_id,
            tid = %data.tid,
            fid = %data.fid,
            summary_type = %data.summary_type,
            detection_count = data.detection_count,
            "Summary saved"
        );

        // camera_idsをVec<String>に変換
        let camera_ids: Vec<String> = serde_json::from_value(data.camera_ids.clone())
            .unwrap_or_default();

        Ok(SummaryResult {
            summary_id,
            tid: data.tid,
            fid: data.fid,
            summary_type: data.summary_type,
            period_start: data.period_start,
            period_end: data.period_end,
            summary_text: data.summary_text,
            summary_json: data.summary_json,
            detection_count: data.detection_count,
            severity_max: data.severity_max,
            camera_ids,
            created_at: Utc::now(),
            expires_at: data.expires_at,
        })
    }

    /// summary_jsonを更新（insert後にsummaryIDを正しい値で上書き）
    pub async fn update_summary_json(
        &self,
        summary_id: u64,
        summary_json: &serde_json::Value,
    ) -> crate::Result<()> {
        sqlx::query(
            r#"
            UPDATE ai_summary_cache
            SET summary_json = ?
            WHERE summary_id = ?
            "#,
        )
        .bind(summary_json)
        .bind(summary_id)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        debug!(summary_id = summary_id, "Summary JSON updated");
        Ok(())
    }

    /// IDでSummaryを取得
    pub async fn get_by_id(&self, summary_id: u64) -> crate::Result<Option<SummaryResult>> {
        let row = sqlx::query_as::<_, SummaryRow>(
            r#"
            SELECT
                summary_id, tid, fid, summary_type, period_start, period_end,
                summary_text, summary_json, detection_count, severity_max,
                camera_ids, created_at, expires_at
            FROM ai_summary_cache
            WHERE summary_id = ?
            "#,
        )
        .bind(summary_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    /// 最新のSummaryを取得
    pub async fn get_latest(
        &self,
        tid: &str,
        fid: Option<&str>,
        summary_type: SummaryType,
    ) -> crate::Result<Option<SummaryResult>> {
        let query = if let Some(f) = fid {
            sqlx::query_as::<_, SummaryRow>(
                r#"
                SELECT
                    summary_id, tid, fid, summary_type, period_start, period_end,
                    summary_text, summary_json, detection_count, severity_max,
                    camera_ids, created_at, expires_at
                FROM ai_summary_cache
                WHERE tid = ? AND fid = ? AND summary_type = ?
                ORDER BY created_at DESC
                LIMIT 1
                "#,
            )
            .bind(tid)
            .bind(f)
            .bind(summary_type.to_string())
            .fetch_optional(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, SummaryRow>(
                r#"
                SELECT
                    summary_id, tid, fid, summary_type, period_start, period_end,
                    summary_text, summary_json, detection_count, severity_max,
                    camera_ids, created_at, expires_at
                FROM ai_summary_cache
                WHERE tid = ? AND summary_type = ?
                ORDER BY created_at DESC
                LIMIT 1
                "#,
            )
            .bind(tid)
            .bind(summary_type.to_string())
            .fetch_optional(&self.pool)
            .await
        };

        let row = query.map_err(|e| crate::Error::Database(e.to_string()))?;
        Ok(row.map(|r| r.into()))
    }

    /// 期間指定でSummaryを取得
    pub async fn get_by_period(
        &self,
        tid: &str,
        fid: &str,
        summary_type: SummaryType,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> crate::Result<Vec<SummaryResult>> {
        let rows = sqlx::query_as::<_, SummaryRow>(
            r#"
            SELECT
                summary_id, tid, fid, summary_type, period_start, period_end,
                summary_text, summary_json, detection_count, severity_max,
                camera_ids, created_at, expires_at
            FROM ai_summary_cache
            WHERE tid = ? AND fid = ? AND summary_type = ?
            AND period_start >= ? AND period_end <= ?
            ORDER BY period_start ASC
            "#,
        )
        .bind(tid)
        .bind(fid)
        .bind(summary_type.to_string())
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 期間指定でSummary一覧を取得（型問わず）
    pub async fn get_range(
        &self,
        tid: &str,
        fid: Option<&str>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        summary_type: Option<SummaryType>,
        limit: u32,
    ) -> crate::Result<Vec<SummaryResult>> {
        let (query_str, has_fid, has_type) = match (fid.is_some(), summary_type.is_some()) {
            (true, true) => (
                r#"
                SELECT
                    summary_id, tid, fid, summary_type, period_start, period_end,
                    summary_text, summary_json, detection_count, severity_max,
                    camera_ids, created_at, expires_at
                FROM ai_summary_cache
                WHERE tid = ? AND fid = ? AND summary_type = ?
                AND period_start >= ? AND period_end <= ?
                ORDER BY period_start DESC
                LIMIT ?
                "#,
                true,
                true,
            ),
            (true, false) => (
                r#"
                SELECT
                    summary_id, tid, fid, summary_type, period_start, period_end,
                    summary_text, summary_json, detection_count, severity_max,
                    camera_ids, created_at, expires_at
                FROM ai_summary_cache
                WHERE tid = ? AND fid = ?
                AND period_start >= ? AND period_end <= ?
                ORDER BY period_start DESC
                LIMIT ?
                "#,
                true,
                false,
            ),
            (false, true) => (
                r#"
                SELECT
                    summary_id, tid, fid, summary_type, period_start, period_end,
                    summary_text, summary_json, detection_count, severity_max,
                    camera_ids, created_at, expires_at
                FROM ai_summary_cache
                WHERE tid = ? AND summary_type = ?
                AND period_start >= ? AND period_end <= ?
                ORDER BY period_start DESC
                LIMIT ?
                "#,
                false,
                true,
            ),
            (false, false) => (
                r#"
                SELECT
                    summary_id, tid, fid, summary_type, period_start, period_end,
                    summary_text, summary_json, detection_count, severity_max,
                    camera_ids, created_at, expires_at
                FROM ai_summary_cache
                WHERE tid = ?
                AND period_start >= ? AND period_end <= ?
                ORDER BY period_start DESC
                LIMIT ?
                "#,
                false,
                false,
            ),
        };

        let mut query = sqlx::query_as::<_, SummaryRow>(query_str).bind(tid);

        if has_fid {
            query = query.bind(fid.unwrap());
        }
        if has_type {
            query = query.bind(summary_type.unwrap().to_string());
        }

        query = query.bind(from).bind(to).bind(limit);

        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 期限切れSummaryを削除
    pub async fn delete_expired(&self) -> crate::Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM ai_summary_cache
            WHERE expires_at < NOW()
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        let deleted = result.rows_affected();
        if deleted > 0 {
            info!(count = deleted, "Expired summaries deleted");
        }
        Ok(deleted)
    }
}

// ============================================================
// Schedule Repository
// ============================================================

/// スケジュール設定リポジトリ
pub struct ScheduleRepository {
    pool: MySqlPool,
}

impl ScheduleRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// スケジュールを保存または更新（UPSERT）
    pub async fn upsert(&self, schedule: &ReportSchedule) -> crate::Result<()> {
        let scheduled_times = schedule
            .scheduled_times
            .as_ref()
            .map(|t| serde_json::to_value(t).ok())
            .flatten();

        sqlx::query(
            r#"
            INSERT INTO scheduled_reports
                (tid, fid, report_type, interval_minutes, scheduled_times, enabled, next_run_at)
            VALUES
                (?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                interval_minutes = VALUES(interval_minutes),
                scheduled_times = VALUES(scheduled_times),
                enabled = VALUES(enabled),
                next_run_at = VALUES(next_run_at),
                updated_at = CURRENT_TIMESTAMP(3)
            "#,
        )
        .bind(&schedule.tid)
        .bind(&schedule.fid)
        .bind(schedule.report_type.to_string())
        .bind(schedule.interval_minutes)
        .bind(&scheduled_times)
        .bind(schedule.enabled)
        .bind(schedule.next_run_at)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        info!(
            tid = %schedule.tid,
            fid = %schedule.fid,
            report_type = %schedule.report_type,
            "Schedule upserted"
        );
        Ok(())
    }

    /// スケジュール一覧を取得
    pub async fn get_schedules(&self, tid: &str, fid: &str) -> crate::Result<Vec<ReportSchedule>> {
        let rows = sqlx::query_as::<_, ScheduleRow>(
            r#"
            SELECT
                schedule_id, tid, fid, report_type, interval_minutes,
                scheduled_times, last_run_at, next_run_at, enabled
            FROM scheduled_reports
            WHERE tid = ? AND fid = ?
            ORDER BY report_type
            "#,
        )
        .bind(tid)
        .bind(fid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 実行待ちスケジュールを取得
    pub async fn get_due_schedules(&self, now: DateTime<Utc>) -> crate::Result<Vec<ReportSchedule>> {
        let rows = sqlx::query_as::<_, ScheduleRow>(
            r#"
            SELECT
                schedule_id, tid, fid, report_type, interval_minutes,
                scheduled_times, last_run_at, next_run_at, enabled
            FROM scheduled_reports
            WHERE enabled = TRUE
            AND (next_run_at IS NULL OR next_run_at <= ?)
            ORDER BY next_run_at ASC
            "#,
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 実行完了を記録
    pub async fn update_last_run(
        &self,
        schedule_id: u32,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> crate::Result<()> {
        sqlx::query(
            r#"
            UPDATE scheduled_reports
            SET last_run_at = ?, next_run_at = ?, updated_at = CURRENT_TIMESTAMP(3)
            WHERE schedule_id = ?
            "#,
        )
        .bind(last_run)
        .bind(next_run)
        .bind(schedule_id)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        debug!(
            schedule_id = schedule_id,
            next_run = %next_run,
            "Schedule updated"
        );
        Ok(())
    }

    /// スケジュールを有効/無効切り替え
    pub async fn set_enabled(&self, schedule_id: u32, enabled: bool) -> crate::Result<()> {
        sqlx::query(
            r#"
            UPDATE scheduled_reports
            SET enabled = ?, updated_at = CURRENT_TIMESTAMP(3)
            WHERE schedule_id = ?
            "#,
        )
        .bind(enabled)
        .bind(schedule_id)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        info!(schedule_id = schedule_id, enabled = enabled, "Schedule enabled state changed");
        Ok(())
    }
}

// ============================================================
// Row Structures
// ============================================================

#[derive(sqlx::FromRow)]
struct SummaryRow {
    summary_id: u64,
    tid: String,
    fid: String,
    summary_type: String,
    period_start: chrono::NaiveDateTime,
    period_end: chrono::NaiveDateTime,
    summary_text: String,
    summary_json: Option<serde_json::Value>,
    detection_count: i32,
    severity_max: i32,
    camera_ids: serde_json::Value,
    created_at: chrono::NaiveDateTime,
    expires_at: chrono::NaiveDateTime,
}

impl From<SummaryRow> for SummaryResult {
    fn from(row: SummaryRow) -> Self {
        let camera_ids: Vec<String> = serde_json::from_value(row.camera_ids).unwrap_or_default();

        Self {
            summary_id: row.summary_id,
            tid: row.tid,
            fid: row.fid,
            summary_type: SummaryType::from(row.summary_type.as_str()),
            period_start: DateTime::<Utc>::from_naive_utc_and_offset(row.period_start, Utc),
            period_end: DateTime::<Utc>::from_naive_utc_and_offset(row.period_end, Utc),
            summary_text: row.summary_text,
            summary_json: row.summary_json,
            detection_count: row.detection_count,
            severity_max: row.severity_max,
            camera_ids,
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(row.created_at, Utc),
            expires_at: DateTime::<Utc>::from_naive_utc_and_offset(row.expires_at, Utc),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ScheduleRow {
    schedule_id: u32,
    tid: String,
    fid: String,
    report_type: String,
    interval_minutes: Option<i32>,
    scheduled_times: Option<serde_json::Value>,
    last_run_at: Option<chrono::NaiveDateTime>,
    next_run_at: Option<chrono::NaiveDateTime>,
    enabled: bool,
}

impl From<ScheduleRow> for ReportSchedule {
    fn from(row: ScheduleRow) -> Self {
        let scheduled_times: Option<Vec<String>> = row
            .scheduled_times
            .and_then(|v| serde_json::from_value(v).ok());

        Self {
            schedule_id: row.schedule_id,
            tid: row.tid,
            fid: row.fid,
            report_type: ReportType::from(row.report_type.as_str()),
            interval_minutes: row.interval_minutes,
            scheduled_times,
            last_run_at: row
                .last_run_at
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)),
            next_run_at: row
                .next_run_at
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)),
            enabled: row.enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_type_conversion() {
        let row = SummaryRow {
            summary_id: 1,
            tid: "T123".to_string(),
            fid: "0150".to_string(),
            summary_type: "hourly".to_string(),
            period_start: chrono::NaiveDateTime::default(),
            period_end: chrono::NaiveDateTime::default(),
            summary_text: "test".to_string(),
            summary_json: None,
            detection_count: 10,
            severity_max: 3,
            camera_ids: serde_json::json!(["cam1"]),
            created_at: chrono::NaiveDateTime::default(),
            expires_at: chrono::NaiveDateTime::default(),
        };

        let result: SummaryResult = row.into();
        assert_eq!(result.summary_type, SummaryType::Hourly);
        assert_eq!(result.camera_ids, vec!["cam1".to_string()]);
    }
}
