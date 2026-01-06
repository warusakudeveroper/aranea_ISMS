//! DetectionLogService - AI Event Log Persistence
//!
//! ## Responsibilities
//!
//! - Persist detection results to MySQL (detection_logs table)
//! - Save event images to filesystem
//! - Queue for BigQuery synchronization
//! - Provide query interface for AI Chat
//!
//! ## Design Reference
//!
//! - §2.5 DetectionLogService (is22_AI_EVENT_LOG_DESIGN.md)
//! - migration 008_detection_logs.sql

use crate::ai_client::{AnalyzeResponse, CameraContext};
use crate::error::{Error, Result};
use crate::models::ProcessingTimings;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPool;
use sqlx::Row;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// Detection log record (matches detection_logs table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionLog {
    pub log_id: Option<u64>,

    // Identifiers
    pub tid: String,
    pub fid: String,
    pub camera_id: String,
    pub lacis_id: Option<String>,

    // Timestamps
    pub captured_at: DateTime<Utc>,
    pub analyzed_at: DateTime<Utc>,

    // IS21 results
    pub primary_event: String,
    pub severity: i32,
    pub confidence: f32,
    pub count_hint: i32,
    pub unknown_flag: bool,

    // Detail data (JSON)
    pub tags: Vec<String>,
    pub person_details: Option<serde_json::Value>,
    pub bboxes: Option<serde_json::Value>,
    pub suspicious: Option<serde_json::Value>,

    // Frame diff
    pub frame_diff: Option<serde_json::Value>,
    pub loitering_detected: bool,

    // Preset info
    pub preset_id: String,
    pub preset_version: Option<String>,
    pub output_schema: Option<String>,

    // Context
    pub context_applied: bool,
    pub camera_context: Option<serde_json::Value>,

    // Raw data
    pub is21_log: serde_json::Value,

    // Image paths
    pub image_path_local: String,
    pub image_path_cloud: Option<String>,

    // Processing info
    pub processing_ms: Option<i32>,
    pub polling_cycle_id: Option<String>,
    pub schema_version: String,

    // Timing breakdown (ボトルネック分析用)
    pub timings: Option<ProcessingTimings>,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub synced_to_bq: bool,
    pub synced_at: Option<DateTime<Utc>>,
}

/// BQ sync queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BqSyncEntry {
    pub queue_id: Option<u64>,
    pub table_name: String,
    pub record_id: u64,
    pub operation: String,
    pub payload: serde_json::Value,
    pub retry_count: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct DetectionLogConfig {
    /// Base path for event images
    pub image_base_path: PathBuf,
    /// Cloud storage path prefix
    pub cloud_path_prefix: Option<String>,
    /// Maximum local images per camera
    pub max_images_per_camera: usize,
    /// Enable BQ sync
    pub enable_bq_sync: bool,
}

impl Default for DetectionLogConfig {
    fn default() -> Self {
        Self {
            image_base_path: PathBuf::from("/var/lib/is22/events"),
            cloud_path_prefix: None,
            max_images_per_camera: 1000,
            enable_bq_sync: true,
        }
    }
}

/// DetectionLogService instance
pub struct DetectionLogService {
    pool: MySqlPool,
    config: DetectionLogConfig,
    stats: Arc<RwLock<ServiceStats>>,
}

/// Service statistics
#[derive(Debug, Clone, Default)]
pub struct ServiceStats {
    pub logs_saved: u64,
    pub bq_queued: u64,
    pub images_saved: u64,
    pub errors: u64,
}

impl DetectionLogService {
    /// Create new DetectionLogService
    pub fn new(pool: MySqlPool, config: DetectionLogConfig) -> Self {
        Self {
            pool,
            config,
            stats: Arc::new(RwLock::new(ServiceStats::default())),
        }
    }

    /// Create with default config
    pub fn with_pool(pool: MySqlPool) -> Self {
        Self::new(pool, DetectionLogConfig::default())
    }

    /// Save detection result from IS21
    ///
    /// Flow:
    /// 1. Save image to filesystem
    /// 2. Insert record to detection_logs
    /// 3. Queue for BQ sync
    pub async fn save_detection(
        &self,
        tid: &str,
        fid: &str,
        lacis_id: Option<&str>,
        response: &AnalyzeResponse,
        image_data: &[u8],
        camera_context: Option<&CameraContext>,
        processing_ms: Option<i32>,
        timings: Option<&ProcessingTimings>,
        polling_cycle_id: Option<&str>,
    ) -> Result<u64> {
        let now = Utc::now();

        // 1. Save image to filesystem
        let image_path = self
            .save_image(&response.camera_id, &response.captured_at, image_data)
            .await?;

        // 2. Prepare detection log
        let log = self.build_detection_log(
            tid,
            fid,
            lacis_id,
            response,
            camera_context,
            &image_path,
            processing_ms,
            timings,
            polling_cycle_id,
            now,
        )?;

        // 3. Insert to database
        let log_id = self.insert_detection_log(&log).await?;

        // 4. Queue for BQ sync if enabled
        if self.config.enable_bq_sync {
            self.queue_for_bq(log_id, &log).await?;
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.logs_saved += 1;
            stats.images_saved += 1;
        }

        tracing::info!(
            log_id = log_id,
            camera_id = %response.camera_id,
            primary_event = %response.primary_event,
            "Detection log saved"
        );

        Ok(log_id)
    }

    /// Build DetectionLog from AnalyzeResponse
    fn build_detection_log(
        &self,
        tid: &str,
        fid: &str,
        lacis_id: Option<&str>,
        response: &AnalyzeResponse,
        camera_context: Option<&CameraContext>,
        image_path: &str,
        processing_ms: Option<i32>,
        timings: Option<&ProcessingTimings>,
        polling_cycle_id: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<DetectionLog> {
        // Extract loitering_detected from frame_diff
        let loitering_detected = response
            .frame_diff
            .as_ref()
            .and_then(|fd| fd.loitering.as_ref())
            .map(|l| l.detected)
            .unwrap_or(false);

        // Serialize response as is21_log with IS22-side timings injected
        let mut is21_log = serde_json::to_value(response)?;

        // Inject IS22-side timings for bottleneck analysis
        if let Some(ref t) = timings {
            if let Some(obj) = is21_log.as_object_mut() {
                obj.insert("is22_timings".to_string(), serde_json::json!({
                    "total_ms": t.total_ms,
                    "snapshot_ms": t.snapshot_ms,
                    "is21_roundtrip_ms": t.is21_roundtrip_ms,
                    "is21_inference_ms": t.is21_inference_ms,
                    "is21_yolo_ms": t.is21_yolo_ms,
                    "is21_par_ms": t.is21_par_ms,
                    "save_ms": t.save_ms,
                    "network_overhead_ms": t.network_overhead_ms(),
                    "snapshot_source": t.snapshot_source.as_deref(),
                }));
            }
        }

        // Serialize optional fields
        let person_details = response
            .person_details
            .as_ref()
            .map(|pd| serde_json::to_value(pd))
            .transpose()?;

        let bboxes = if !response.bboxes.is_empty() {
            Some(serde_json::to_value(&response.bboxes)?)
        } else {
            None
        };

        let suspicious = response
            .suspicious
            .as_ref()
            .map(|s| serde_json::to_value(s))
            .transpose()?;

        let frame_diff_json = response
            .frame_diff
            .as_ref()
            .map(|fd| serde_json::to_value(fd))
            .transpose()?;

        let context_json = camera_context
            .map(|ctx| serde_json::to_value(ctx))
            .transpose()?;

        // Parse captured_at from response
        let captured_at = DateTime::parse_from_rfc3339(&response.captured_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(now);

        Ok(DetectionLog {
            log_id: None,
            tid: tid.to_string(),
            fid: fid.to_string(),
            camera_id: response.camera_id.clone(),
            lacis_id: lacis_id.map(String::from),
            captured_at,
            analyzed_at: now,
            primary_event: response.primary_event.clone(),
            severity: response.severity,
            confidence: response.confidence,
            count_hint: response.count_hint,
            unknown_flag: response.unknown_flag,
            tags: response.tags.clone(),
            person_details,
            bboxes,
            suspicious,
            frame_diff: frame_diff_json,
            loitering_detected,
            preset_id: response.preset_id.clone().unwrap_or_else(|| "balanced".to_string()),
            preset_version: response.preset_version.clone(),
            output_schema: response.output_schema.clone(),
            context_applied: camera_context.is_some(),
            camera_context: context_json,
            is21_log,
            image_path_local: image_path.to_string(),
            image_path_cloud: None,
            processing_ms,
            polling_cycle_id: polling_cycle_id.map(String::from),
            schema_version: response.schema_version.clone(),
            timings: timings.cloned(),
            created_at: now,
            synced_to_bq: false,
            synced_at: None,
        })
    }

    /// Save image to filesystem
    async fn save_image(
        &self,
        camera_id: &str,
        captured_at: &str,
        image_data: &[u8],
    ) -> Result<String> {
        // Create camera directory
        let camera_dir = self.config.image_base_path.join(camera_id);
        fs::create_dir_all(&camera_dir).await?;

        // Generate filename from timestamp
        let filename = format!(
            "{}.jpg",
            captured_at.replace([':', '-', 'T', 'Z', '.'], "")
        );
        let file_path = camera_dir.join(&filename);

        // Write image
        fs::write(&file_path, image_data).await?;

        Ok(file_path.to_string_lossy().to_string())
    }

    /// Insert detection log to database
    async fn insert_detection_log(&self, log: &DetectionLog) -> Result<u64> {
        let tags_json = serde_json::to_string(&log.tags)?;
        let person_details_json = log.person_details.as_ref().map(|v| v.to_string());
        let bboxes_json = log.bboxes.as_ref().map(|v| v.to_string());
        let suspicious_json = log.suspicious.as_ref().map(|v| v.to_string());
        let frame_diff_json = log.frame_diff.as_ref().map(|v| v.to_string());
        let context_json = log.camera_context.as_ref().map(|v| v.to_string());
        let is21_log_json = log.is21_log.to_string();

        let result = sqlx::query(
            r#"
            INSERT INTO detection_logs (
                tid, fid, camera_id, lacis_id,
                captured_at, analyzed_at,
                primary_event, severity, confidence, count_hint, unknown_flag,
                tags, person_details, bboxes, suspicious,
                frame_diff, loitering_detected,
                preset_id, preset_version, output_schema,
                context_applied, camera_context,
                is21_log,
                image_path_local, image_path_cloud,
                processing_ms, polling_cycle_id, schema_version,
                synced_to_bq
            ) VALUES (
                ?, ?, ?, ?,
                ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?,
                ?, ?, ?,
                ?, ?,
                ?,
                ?, ?,
                ?, ?, ?,
                FALSE
            )
            "#,
        )
        .bind(&log.tid)
        .bind(&log.fid)
        .bind(&log.camera_id)
        .bind(&log.lacis_id)
        .bind(log.captured_at)
        .bind(log.analyzed_at)
        .bind(&log.primary_event)
        .bind(log.severity)
        .bind(log.confidence)
        .bind(log.count_hint)
        .bind(log.unknown_flag)
        .bind(&tags_json)
        .bind(&person_details_json)
        .bind(&bboxes_json)
        .bind(&suspicious_json)
        .bind(&frame_diff_json)
        .bind(log.loitering_detected)
        .bind(&log.preset_id)
        .bind(&log.preset_version)
        .bind(&log.output_schema)
        .bind(log.context_applied)
        .bind(&context_json)
        .bind(&is21_log_json)
        .bind(&log.image_path_local)
        .bind(&log.image_path_cloud)
        .bind(log.processing_ms)
        .bind(&log.polling_cycle_id)
        .bind(&log.schema_version)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id())
    }

    /// Queue record for BQ synchronization
    async fn queue_for_bq(&self, log_id: u64, log: &DetectionLog) -> Result<()> {
        // Build BQ payload (subset of columns for BQ)
        let payload = serde_json::json!({
            "log_id": log_id,
            "tid": log.tid,
            "fid": log.fid,
            "camera_id": log.camera_id,
            "lacis_id": log.lacis_id,
            "captured_at": log.captured_at.to_rfc3339(),
            "analyzed_at": log.analyzed_at.to_rfc3339(),
            "primary_event": log.primary_event,
            "severity": log.severity,
            "confidence": log.confidence,
            "count_hint": log.count_hint,
            "tags": log.tags,
            "preset_id": log.preset_id,
            "loitering_detected": log.loitering_detected,
            "image_path_local": log.image_path_local,
            "schema_version": log.schema_version,
        });

        sqlx::query(
            r#"
            INSERT INTO bq_sync_queue (table_name, record_id, operation, payload, status)
            VALUES ('detection_logs', ?, 'INSERT', ?, 'pending')
            "#,
        )
        .bind(log_id)
        .bind(payload.to_string())
        .execute(&self.pool)
        .await?;

        {
            let mut stats = self.stats.write().await;
            stats.bq_queued += 1;
        }

        Ok(())
    }

    // ========================================
    // Query Methods
    // ========================================

    /// Get latest detection logs
    pub async fn get_latest(&self, limit: u32) -> Result<Vec<DetectionLog>> {
        let rows = sqlx::query(
            r#"
            SELECT
                log_id, tid, fid, camera_id, lacis_id,
                captured_at, analyzed_at,
                primary_event, severity, CAST(confidence AS DOUBLE) AS confidence, count_hint, unknown_flag,
                tags, person_details, bboxes, suspicious,
                frame_diff, loitering_detected,
                preset_id, preset_version, output_schema,
                context_applied, camera_context,
                is21_log,
                image_path_local, image_path_cloud,
                processing_ms, polling_cycle_id, schema_version,
                created_at, synced_to_bq, synced_at
            FROM detection_logs
            ORDER BY captured_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| self.row_to_log(row)).collect()
    }

    /// Get detection logs by camera
    pub async fn get_by_camera(&self, camera_id: &str, limit: u32) -> Result<Vec<DetectionLog>> {
        let rows = sqlx::query(
            r#"
            SELECT
                log_id, tid, fid, camera_id, lacis_id,
                captured_at, analyzed_at,
                primary_event, severity, CAST(confidence AS DOUBLE) AS confidence, count_hint, unknown_flag,
                tags, person_details, bboxes, suspicious,
                frame_diff, loitering_detected,
                preset_id, preset_version, output_schema,
                context_applied, camera_context,
                is21_log,
                image_path_local, image_path_cloud,
                processing_ms, polling_cycle_id, schema_version,
                created_at, synced_to_bq, synced_at
            FROM detection_logs
            WHERE camera_id = ?
            ORDER BY captured_at DESC
            LIMIT ?
            "#,
        )
        .bind(camera_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| self.row_to_log(row)).collect()
    }

    /// Get detection logs by time range
    pub async fn get_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<DetectionLog>> {
        let rows = sqlx::query(
            r#"
            SELECT
                log_id, tid, fid, camera_id, lacis_id,
                captured_at, analyzed_at,
                primary_event, severity, CAST(confidence AS DOUBLE) AS confidence, count_hint, unknown_flag,
                tags, person_details, bboxes, suspicious,
                frame_diff, loitering_detected,
                preset_id, preset_version, output_schema,
                context_applied, camera_context,
                is21_log,
                image_path_local, image_path_cloud,
                processing_ms, polling_cycle_id, schema_version,
                created_at, synced_to_bq, synced_at
            FROM detection_logs
            WHERE captured_at BETWEEN ? AND ?
            ORDER BY captured_at DESC
            LIMIT ?
            "#,
        )
        .bind(start)
        .bind(end)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| self.row_to_log(row)).collect()
    }

    /// Get high severity logs (severity >= threshold)
    pub async fn get_high_severity(&self, threshold: i32, limit: u32) -> Result<Vec<DetectionLog>> {
        let rows = sqlx::query(
            r#"
            SELECT
                log_id, tid, fid, camera_id, lacis_id,
                captured_at, analyzed_at,
                primary_event, severity, CAST(confidence AS DOUBLE) AS confidence, count_hint, unknown_flag,
                tags, person_details, bboxes, suspicious,
                frame_diff, loitering_detected,
                preset_id, preset_version, output_schema,
                context_applied, camera_context,
                is21_log,
                image_path_local, image_path_cloud,
                processing_ms, polling_cycle_id, schema_version,
                created_at, synced_to_bq, synced_at
            FROM detection_logs
            WHERE severity >= ?
            ORDER BY captured_at DESC
            LIMIT ?
            "#,
        )
        .bind(threshold)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| self.row_to_log(row)).collect()
    }

    /// Convert database row to DetectionLog
    fn row_to_log(&self, row: sqlx::mysql::MySqlRow) -> Result<DetectionLog> {
        // Parse JSON fields
        let tags_str: String = row.try_get("tags")?;
        let tags: Vec<String> = serde_json::from_str(&tags_str)?;

        let person_details: Option<serde_json::Value> = row
            .try_get::<Option<String>, _>("person_details")?
            .map(|s| serde_json::from_str(&s))
            .transpose()?;

        let bboxes: Option<serde_json::Value> = row
            .try_get::<Option<String>, _>("bboxes")?
            .map(|s| serde_json::from_str(&s))
            .transpose()?;

        let suspicious: Option<serde_json::Value> = row
            .try_get::<Option<String>, _>("suspicious")?
            .map(|s| serde_json::from_str(&s))
            .transpose()?;

        let frame_diff: Option<serde_json::Value> = row
            .try_get::<Option<String>, _>("frame_diff")?
            .map(|s| serde_json::from_str(&s))
            .transpose()?;

        let camera_context: Option<serde_json::Value> = row
            .try_get::<Option<String>, _>("camera_context")?
            .map(|s| serde_json::from_str(&s))
            .transpose()?;

        let is21_log_str: String = row.try_get("is21_log")?;
        let is21_log: serde_json::Value = serde_json::from_str(&is21_log_str)?;

        // Get timestamps
        let captured_at: chrono::NaiveDateTime = row.try_get("captured_at")?;
        let analyzed_at: chrono::NaiveDateTime = row.try_get("analyzed_at")?;
        let created_at: chrono::NaiveDateTime = row.try_get("created_at")?;
        let synced_at: Option<chrono::NaiveDateTime> = row.try_get("synced_at")?;

        // Get confidence as f64 (CAST from DECIMAL in SQL query)
        let confidence: f64 = row.try_get("confidence")?;

        Ok(DetectionLog {
            log_id: Some(row.try_get("log_id")?),
            tid: row.try_get("tid")?,
            fid: row.try_get("fid")?,
            camera_id: row.try_get("camera_id")?,
            lacis_id: row.try_get("lacis_id")?,
            captured_at: DateTime::from_naive_utc_and_offset(captured_at, Utc),
            analyzed_at: DateTime::from_naive_utc_and_offset(analyzed_at, Utc),
            primary_event: row.try_get("primary_event")?,
            severity: row.try_get("severity")?,
            confidence: confidence as f32,
            count_hint: row.try_get("count_hint")?,
            unknown_flag: row.try_get("unknown_flag")?,
            tags,
            person_details,
            bboxes,
            suspicious,
            frame_diff,
            loitering_detected: row.try_get("loitering_detected")?,
            preset_id: row.try_get("preset_id")?,
            preset_version: row.try_get("preset_version")?,
            output_schema: row.try_get("output_schema")?,
            context_applied: row.try_get("context_applied")?,
            camera_context,
            is21_log,
            image_path_local: row.try_get("image_path_local")?,
            image_path_cloud: row.try_get("image_path_cloud")?,
            processing_ms: row.try_get("processing_ms")?,
            polling_cycle_id: row.try_get("polling_cycle_id")?,
            schema_version: row.try_get("schema_version")?,
            // Timings are stored in is21_log, extract if needed
            timings: None, // TODO: Parse from is21_log if needed
            created_at: DateTime::from_naive_utc_and_offset(created_at, Utc),
            synced_to_bq: row.try_get("synced_to_bq")?,
            synced_at: synced_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        })
    }

    /// Get service stats
    pub async fn get_stats(&self) -> ServiceStats {
        self.stats.read().await.clone()
    }

    /// Mark log as synced to BQ
    pub async fn mark_synced(&self, log_id: u64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE detection_logs
            SET synced_to_bq = TRUE, synced_at = NOW(3)
            WHERE log_id = ?
            "#,
        )
        .bind(log_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get pending BQ sync queue entries
    pub async fn get_pending_bq_sync(&self, limit: u32) -> Result<Vec<BqSyncEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT
                queue_id, table_name, record_id, operation, payload,
                retry_count, status, created_at
            FROM bq_sync_queue
            WHERE status = 'pending'
            ORDER BY created_at ASC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::new();
        for row in rows {
            let payload_str: String = row.try_get("payload")?;
            let payload: serde_json::Value = serde_json::from_str(&payload_str)?;
            let created_at: chrono::NaiveDateTime = row.try_get("created_at")?;

            entries.push(BqSyncEntry {
                queue_id: Some(row.try_get("queue_id")?),
                table_name: row.try_get("table_name")?,
                record_id: row.try_get("record_id")?,
                operation: row.try_get("operation")?,
                payload,
                retry_count: row.try_get("retry_count")?,
                status: row.try_get("status")?,
                created_at: DateTime::from_naive_utc_and_offset(created_at, Utc),
            });
        }

        Ok(entries)
    }

    /// Update BQ sync queue entry status
    pub async fn update_bq_sync_status(
        &self,
        queue_id: u64,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE bq_sync_queue
            SET status = ?,
                last_error = ?,
                retry_count = retry_count + 1,
                processed_at = IF(? = 'success', NOW(3), NULL)
            WHERE queue_id = ?
            "#,
        )
        .bind(status)
        .bind(error)
        .bind(status)
        .bind(queue_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========================================
    // Camera Status Events
    // ========================================

    /// Save camera status event (lost/recovered) without image
    ///
    /// These events are recorded for user visibility in AI Event Log.
    /// They use a minimal record format since there's no actual detection.
    pub async fn save_camera_event(
        &self,
        tid: &str,
        fid: &str,
        camera_id: &str,
        lacis_id: Option<&str>,
        event_type: &str,  // "camera_lost" or "camera_recovered"
        severity: i32,
        error_message: Option<&str>,
    ) -> Result<u64> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        // Build minimal is21_log for camera events
        let is21_log = serde_json::json!({
            "analyzed": false,
            "detected": false,
            "primary_event": event_type,
            "severity": severity,
            "camera_id": camera_id,
            "captured_at": now_str,
            "schema_version": "2025-12-29.1",
            "event_type": "camera_status",
            "message": if event_type == "camera_lost" {
                error_message.unwrap_or("カメラ接続ロスト")
            } else {
                "カメラ接続復帰"
            },
            "confidence": 1.0,
            "count_hint": 0,
            "unknown_flag": false,
            "tags": [],
            "bboxes": [],
        });

        let result = sqlx::query(
            r#"
            INSERT INTO detection_logs (
                tid, fid, camera_id, lacis_id,
                captured_at, analyzed_at,
                primary_event, severity, confidence, count_hint, unknown_flag,
                tags, preset_id, schema_version,
                is21_log, image_path_local, synced_to_bq
            ) VALUES (
                ?, ?, ?, ?,
                ?, ?,
                ?, ?, 1.0, 0, FALSE,
                '[]', 'system', '2025-12-29.1',
                ?, '', FALSE
            )
            "#,
        )
        .bind(tid)
        .bind(fid)
        .bind(camera_id)
        .bind(lacis_id)
        .bind(now)
        .bind(now)
        .bind(event_type)
        .bind(severity)
        .bind(is21_log.to_string())
        .execute(&self.pool)
        .await?;

        let log_id = result.last_insert_id();

        tracing::info!(
            log_id = log_id,
            camera_id = %camera_id,
            event_type = %event_type,
            severity = severity,
            "Camera status event saved"
        );

        Ok(log_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DetectionLogConfig::default();
        assert_eq!(config.image_base_path, PathBuf::from("/var/lib/is22/events"));
        assert!(config.enable_bq_sync);
        assert_eq!(config.max_images_per_camera, 1000);
    }
}
