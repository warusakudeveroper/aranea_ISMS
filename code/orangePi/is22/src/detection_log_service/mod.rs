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

/// Storage quota configuration
///
/// AIEventlog.md要件: 「最大保存容量などの設定を設けてユーザーの裁量範疇で画像の保存を行う」
#[derive(Debug, Clone)]
pub struct StorageQuotaConfig {
    /// カメラ毎の最大画像枚数
    pub max_images_per_camera: usize,
    /// カメラ毎の最大容量（バイト）
    pub max_bytes_per_camera: u64,
    /// 全体の最大容量（バイト）
    pub max_total_bytes: u64,
}

impl Default for StorageQuotaConfig {
    fn default() -> Self {
        Self {
            max_images_per_camera: 1000,
            max_bytes_per_camera: 500 * 1024 * 1024,  // 500MB per camera
            max_total_bytes: 10 * 1024 * 1024 * 1024, // 10GB total
        }
    }
}

/// Cleanup operation statistics
#[derive(Debug, Clone, Default)]
pub struct CleanupStats {
    /// Total images before cleanup
    pub total: usize,
    /// Images deleted
    pub deleted: usize,
    /// Images kept
    pub kept: usize,
    /// Bytes freed
    pub bytes_freed: u64,
}

impl CleanupStats {
    pub fn new(total: usize, deleted: usize, kept: usize) -> Self {
        Self {
            total,
            deleted,
            kept,
            bytes_freed: 0,
        }
    }

    pub fn with_bytes_freed(mut self, bytes: u64) -> Self {
        self.bytes_freed = bytes;
        self
    }
}

/// Image file metadata for storage management
#[derive(Debug, Clone)]
struct ImageMeta {
    /// File size in bytes
    size: u64,
    /// Last modified time
    modified: std::time::SystemTime,
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct DetectionLogConfig {
    /// Base path for event images
    pub image_base_path: PathBuf,
    /// Cloud storage path prefix
    pub cloud_path_prefix: Option<String>,
    /// Storage quota settings
    pub quota: StorageQuotaConfig,
    /// Enable BQ sync
    pub enable_bq_sync: bool,
}

impl Default for DetectionLogConfig {
    fn default() -> Self {
        Self {
            image_base_path: PathBuf::from("/var/lib/is22/events"),
            cloud_path_prefix: None,
            quota: StorageQuotaConfig::default(),
            enable_bq_sync: true,
        }
    }
}

/// Determine if an image should be saved based on detection result
///
/// AIEventlog.md要件:
/// - 「何もない」なら画像の保存もログも出さない
/// - unknownは分析用に保存（後で90%削除）
///
/// Truth table:
/// | severity | primary_event | should_save |
/// |----------|---------------|-------------|
/// | 0        | "none"        | false       |
/// | 0        | "unknown"     | true        |
/// | 0        | other event   | true        |
/// | 1+       | any           | true        |
pub fn should_save_image(primary_event: &str, severity: i32, unknown_flag: bool) -> bool {
    // 「何もない」(none)は保存しない
    if primary_event == "none" {
        return false;
    }

    // severity > 0 は常に保存
    if severity > 0 {
        return true;
    }

    // unknown判定は保存（後で90%削除）
    if unknown_flag || primary_event == "unknown" {
        return true;
    }

    // それ以外のイベントがあれば保存
    true
}

/// DetectionLogService instance
pub struct DetectionLogService {
    pool: MySqlPool,
    config: Arc<RwLock<DetectionLogConfig>>,
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
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(ServiceStats::default())),
        }
    }

    /// Create with default config
    pub fn with_pool(pool: MySqlPool) -> Self {
        Self::new(pool, DetectionLogConfig::default())
    }

    /// Get current configuration (for API access)
    pub async fn config(&self) -> DetectionLogConfig {
        self.config.read().await.clone()
    }

    /// Update storage quota settings at runtime
    pub async fn update_quota(
        &self,
        max_images_per_camera: Option<usize>,
        max_bytes_per_camera: Option<u64>,
        max_total_bytes: Option<u64>,
    ) {
        let mut config = self.config.write().await;
        if let Some(v) = max_images_per_camera {
            config.quota.max_images_per_camera = v;
        }
        if let Some(v) = max_bytes_per_camera {
            config.quota.max_bytes_per_camera = v;
        }
        if let Some(v) = max_total_bytes {
            config.quota.max_total_bytes = v;
        }
        tracing::info!(
            max_images = config.quota.max_images_per_camera,
            max_bytes = config.quota.max_bytes_per_camera,
            max_total = config.quota.max_total_bytes,
            "Storage quota config updated"
        );
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
        let enable_bq_sync = self.config.read().await.enable_bq_sync;
        if enable_bq_sync {
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
        let image_base_path = self.config.read().await.image_base_path.clone();
        let camera_dir = image_base_path.join(camera_id);
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
    // Storage Management (T1-3)
    // ========================================

    /// Enforce storage quota for a camera
    ///
    /// AIEventlog.md要件: 「最大保存容量などの設定を設けてユーザーの裁量範疇で画像の保存を行う」
    ///
    /// Deletion priority (oldest first):
    /// 1. Delete oldest images when count exceeds max_images_per_camera
    /// 2. Delete oldest images when capacity exceeds max_bytes_per_camera
    ///
    /// Returns CleanupStats with deletion results.
    pub async fn enforce_storage_quota(&self, camera_id: &str) -> Result<CleanupStats> {
        // Read config values and release lock before async operations
        let (camera_dir, max_images, max_bytes) = {
            let config = self.config.read().await;
            (
                config.image_base_path.join(camera_id),
                config.quota.max_images_per_camera,
                config.quota.max_bytes_per_camera,
            )
        };

        // If directory doesn't exist, nothing to clean
        if !camera_dir.exists() {
            return Ok(CleanupStats::default());
        }

        // Get all images with metadata (size, modified time)
        let mut images = self.list_images_with_metadata(&camera_dir).await?;

        let total_count = images.len();
        if total_count == 0 {
            return Ok(CleanupStats::default());
        }

        // Sort by modified time (oldest first for deletion)
        images.sort_by_key(|(_, meta)| meta.modified);
        let mut deleted_count = 0;
        let mut bytes_freed = 0u64;

        // Calculate current total bytes
        let total_bytes: u64 = images.iter().map(|(_, meta)| meta.size).sum();

        // Determine how many to delete
        // 1. Count-based: delete if exceeds max_images_per_camera
        let count_excess = if total_count > max_images {
            total_count - max_images
        } else {
            0
        };

        // 2. Capacity-based: estimate bytes to free
        let bytes_excess = if total_bytes > max_bytes {
            total_bytes - max_bytes
        } else {
            0
        };

        // Delete from oldest until both quotas are satisfied
        let mut accumulated_bytes = 0u64;
        for (path, meta) in images.iter() {
            // Check if we still need to delete
            let need_count_delete = deleted_count < count_excess;
            let need_bytes_delete = accumulated_bytes < bytes_excess;

            if !need_count_delete && !need_bytes_delete {
                break;
            }

            // Delete the file
            if let Err(e) = fs::remove_file(path).await {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to delete image during quota enforcement"
                );
                continue;
            }

            deleted_count += 1;
            bytes_freed += meta.size;
            accumulated_bytes += meta.size;

            tracing::debug!(
                camera_id = %camera_id,
                path = %path.display(),
                size = meta.size,
                "Deleted image for quota enforcement"
            );
        }

        let stats = CleanupStats {
            total: total_count,
            deleted: deleted_count,
            kept: total_count - deleted_count,
            bytes_freed,
        };

        if deleted_count > 0 {
            tracing::info!(
                camera_id = %camera_id,
                total = total_count,
                deleted = deleted_count,
                bytes_freed = bytes_freed,
                "Storage quota enforced"
            );
        }

        Ok(stats)
    }

    /// Get total storage usage across all cameras
    pub async fn get_total_storage_bytes(&self) -> Result<u64> {
        let base_path = self.config.read().await.image_base_path.clone();

        if !base_path.exists() {
            return Ok(0);
        }

        let mut total_bytes = 0u64;
        let mut entries = fs::read_dir(&base_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                // Sum up all files in camera directory
                let mut camera_entries = fs::read_dir(&path).await?;
                while let Some(file_entry) = camera_entries.next_entry().await? {
                    if let Ok(meta) = file_entry.metadata().await {
                        total_bytes += meta.len();
                    }
                }
            }
        }

        Ok(total_bytes)
    }

    /// Enforce global storage quota
    ///
    /// If total storage exceeds max_total_bytes, delete oldest images across all cameras.
    pub async fn enforce_global_quota(&self) -> Result<CleanupStats> {
        // Read config values and release lock before any async operations
        let (max_total_bytes, base_path) = {
            let config = self.config.read().await;
            (config.quota.max_total_bytes, config.image_base_path.clone())
        };

        let total_bytes = self.get_total_storage_bytes().await?;

        if total_bytes <= max_total_bytes {
            return Ok(CleanupStats::default());
        }

        let bytes_to_free = total_bytes - max_total_bytes;

        // Collect all images across all cameras with metadata
        let mut all_images = Vec::new();

        if base_path.exists() {
            let mut entries = fs::read_dir(base_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_dir() {
                    let images = self.list_images_with_metadata(&path).await?;
                    all_images.extend(images);
                }
            }
        }

        let total_count = all_images.len();
        if total_count == 0 {
            return Ok(CleanupStats::default());
        }

        // Sort by modified time (oldest first)
        all_images.sort_by_key(|(_, meta)| meta.modified);

        let mut deleted_count = 0;
        let mut bytes_freed = 0u64;

        for (path, meta) in all_images.iter() {
            if bytes_freed >= bytes_to_free {
                break;
            }

            if let Err(e) = fs::remove_file(path).await {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to delete image during global quota enforcement"
                );
                continue;
            }

            deleted_count += 1;
            bytes_freed += meta.size;
        }

        let stats = CleanupStats {
            total: total_count,
            deleted: deleted_count,
            kept: total_count - deleted_count,
            bytes_freed,
        };

        if deleted_count > 0 {
            tracing::info!(
                total = total_count,
                deleted = deleted_count,
                bytes_freed = bytes_freed,
                target_freed = bytes_to_free,
                "Global storage quota enforced"
            );
        }

        Ok(stats)
    }

    /// Manual unknown image cleanup (administrator operation only)
    ///
    /// WARNING: Rule 5準拠 - 自動実行禁止
    /// 「情報は如何なる場合においても等価であり、優劣をつけて自己判断で不要と判断した情報を
    /// 握り潰すような仕様としてはならない」
    ///
    /// AIEventlog.md要件: 「unknown乱発によって画像の保存ストーム状態」対策
    /// ただし、管理者の明示的操作のみで実行可能。
    ///
    /// 動作:
    /// - confirmed=false: プレビューのみ（削除対象件数を返却、実際の削除なし）
    /// - confirmed=true: 最新10%保持、古い90%削除
    ///
    /// v5修正: 分散サンプリング → 最新10%保持（時系列で新しいものを優先保持）
    pub async fn manual_cleanup_unknown_images(
        &self,
        camera_id: &str,
        confirmed: bool,
    ) -> Result<CleanupStats> {
        // Query unknown images from database for this camera (newest first)
        let rows = sqlx::query(
            r#"
            SELECT log_id, image_path_local, captured_at
            FROM detection_logs
            WHERE camera_id = ?
              AND (primary_event = 'unknown' OR unknown_flag = TRUE)
              AND image_path_local != ''
            ORDER BY captured_at DESC
            "#,
        )
        .bind(camera_id)
        .fetch_all(&self.pool)
        .await?;

        let total_count = rows.len();
        if total_count == 0 {
            return Ok(CleanupStats::default());
        }

        // Calculate how many to keep (latest 10%)
        let keep_count = (total_count as f64 * 0.10).ceil() as usize;
        let keep_count = keep_count.max(1); // Keep at least 1
        let delete_count = total_count.saturating_sub(keep_count);

        // Preview mode: return stats without deleting
        if !confirmed {
            tracing::info!(
                camera_id = %camera_id,
                total = total_count,
                to_delete = delete_count,
                to_keep = keep_count,
                "Unknown cleanup preview (confirmed=false, no deletion)"
            );
            return Ok(CleanupStats {
                total: total_count,
                deleted: 0,        // Preview: nothing deleted yet
                kept: keep_count,  // Will keep latest 10%
                bytes_freed: 0,
            });
        }

        // Confirmed=true: Actually delete oldest 90%
        let mut deleted_count = 0;
        let mut bytes_freed = 0u64;

        // Skip the newest keep_count images, delete the rest
        for row in rows.iter().skip(keep_count) {
            let image_path: String = row.try_get("image_path_local")?;
            let log_id: u64 = row.try_get("log_id")?;

            // Get file size before deleting
            let path = std::path::Path::new(&image_path);
            let file_size = if path.exists() {
                match fs::metadata(path).await {
                    Ok(meta) => meta.len(),
                    Err(_) => 0,
                }
            } else {
                0
            };

            // Delete the image file
            if path.exists() {
                if let Err(e) = fs::remove_file(path).await {
                    tracing::warn!(
                        path = %path.display(),
                        log_id = log_id,
                        error = %e,
                        "Failed to delete unknown image"
                    );
                    continue;
                }
            }

            // Clear image_path_local in database (keep the log record)
            sqlx::query(
                r#"
                UPDATE detection_logs
                SET image_path_local = ''
                WHERE log_id = ?
                "#,
            )
            .bind(log_id)
            .execute(&self.pool)
            .await?;

            deleted_count += 1;
            bytes_freed += file_size;

            tracing::debug!(
                camera_id = %camera_id,
                log_id = log_id,
                path = %image_path,
                "Deleted unknown image (manual cleanup, keep latest 10%)"
            );
        }

        let stats = CleanupStats {
            total: total_count,
            deleted: deleted_count,
            kept: keep_count,
            bytes_freed,
        };

        tracing::info!(
            camera_id = %camera_id,
            total = total_count,
            deleted = deleted_count,
            kept = keep_count,
            bytes_freed = bytes_freed,
            "Manual unknown images cleanup completed (confirmed=true)"
        );

        Ok(stats)
    }

    /// Manual cleanup unknown images across all cameras
    ///
    /// WARNING: Rule 5準拠 - 管理者の明示的操作のみ
    pub async fn manual_cleanup_all_unknown_images(&self, confirmed: bool) -> Result<CleanupStats> {
        // Get all distinct camera IDs with unknown images
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT camera_id
            FROM detection_logs
            WHERE primary_event = 'unknown' OR unknown_flag = TRUE
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut total_stats = CleanupStats::default();

        for row in rows {
            let camera_id: String = row.try_get("camera_id")?;
            let stats = self.manual_cleanup_unknown_images(&camera_id, confirmed).await?;

            total_stats.total += stats.total;
            total_stats.deleted += stats.deleted;
            total_stats.kept += stats.kept;
            total_stats.bytes_freed += stats.bytes_freed;
        }

        if confirmed && total_stats.deleted > 0 {
            tracing::info!(
                total = total_stats.total,
                deleted = total_stats.deleted,
                kept = total_stats.kept,
                bytes_freed = total_stats.bytes_freed,
                "All unknown images manual cleanup completed"
            );
        } else if !confirmed {
            tracing::info!(
                total = total_stats.total,
                to_delete = total_stats.total - total_stats.kept,
                to_keep = total_stats.kept,
                "All unknown images cleanup preview (no deletion)"
            );
        }

        Ok(total_stats)
    }

    /// List images in directory with metadata
    async fn list_images_with_metadata(&self, dir: &std::path::Path) -> Result<Vec<(PathBuf, ImageMeta)>> {
        let mut images = Vec::new();
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process image files
            if let Some(ext) = path.extension() {
                if ext == "jpg" || ext == "jpeg" || ext == "png" {
                    if let Ok(meta) = entry.metadata().await {
                        let modified = meta
                            .modified()
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        images.push((path, ImageMeta {
                            size: meta.len(),
                            modified,
                        }));
                    }
                }
            }
        }

        Ok(images)
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
        assert_eq!(config.quota.max_images_per_camera, 1000);
        assert_eq!(config.quota.max_bytes_per_camera, 500 * 1024 * 1024);
        assert_eq!(config.quota.max_total_bytes, 10 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_cleanup_stats() {
        let stats = CleanupStats::new(100, 90, 10);
        assert_eq!(stats.total, 100);
        assert_eq!(stats.deleted, 90);
        assert_eq!(stats.kept, 10);
        assert_eq!(stats.bytes_freed, 0);

        let stats = stats.with_bytes_freed(1024 * 1024);
        assert_eq!(stats.bytes_freed, 1024 * 1024);
    }

    #[test]
    fn test_storage_quota_config_default() {
        let quota = StorageQuotaConfig::default();
        assert_eq!(quota.max_images_per_camera, 1000);
        assert_eq!(quota.max_bytes_per_camera, 500 * 1024 * 1024);
        assert_eq!(quota.max_total_bytes, 10 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_should_save_image_none_no_save() {
        // BE-02: event="none", unknown=false → 画像保存されない
        assert!(!should_save_image("none", 0, false));
    }

    #[test]
    fn test_should_save_image_none_unknown_no_save() {
        // BE-03: event="none", unknown=true → 画像保存されない（none優先）
        assert!(!should_save_image("none", 0, true));
    }

    #[test]
    fn test_should_save_image_unknown_save() {
        // BE-04: event="unknown", unknown=true → 画像保存される
        assert!(should_save_image("unknown", 0, true));
    }

    #[test]
    fn test_should_save_image_severity_always_save() {
        // BE-05: severity=1, any → 画像保存される
        assert!(should_save_image("none", 1, false));
        assert!(should_save_image("human", 2, false));
        assert!(should_save_image("vehicle", 3, false));
    }

    #[test]
    fn test_should_save_image_human_save() {
        // 通常検出は保存
        assert!(should_save_image("human", 0, false));
        assert!(should_save_image("vehicle", 0, false));
    }

    #[test]
    fn test_cleanup_stats_default() {
        let stats = CleanupStats::default();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.deleted, 0);
        assert_eq!(stats.kept, 0);
        assert_eq!(stats.bytes_freed, 0);
    }

    #[test]
    fn test_image_meta() {
        let meta = ImageMeta {
            size: 1024,
            modified: std::time::SystemTime::UNIX_EPOCH,
        };
        assert_eq!(meta.size, 1024);
    }

    #[test]
    fn test_storage_quota_custom_values() {
        let quota = StorageQuotaConfig {
            max_images_per_camera: 500,
            max_bytes_per_camera: 100 * 1024 * 1024,
            max_total_bytes: 5 * 1024 * 1024 * 1024,
        };
        assert_eq!(quota.max_images_per_camera, 500);
        assert_eq!(quota.max_bytes_per_camera, 100 * 1024 * 1024);
        assert_eq!(quota.max_total_bytes, 5 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_should_save_image_camera_events() {
        // カメライベントは保存
        assert!(should_save_image("camera_lost", 0, false));
        assert!(should_save_image("camera_recovered", 0, false));
    }

    #[test]
    fn test_should_save_anomaly() {
        // 異常検知は保存
        assert!(should_save_image("anomaly", 3, false));
    }

    #[test]
    fn test_keep_count_calculation() {
        // v5修正: 最新10%保持のロジック検証
        // 100枚 → 10枚保持、90枚削除
        let total = 100usize;
        let keep = (total as f64 * 0.10).ceil() as usize;
        assert_eq!(keep, 10);

        // 15枚 → 2枚保持（ceil(1.5) = 2）
        let total = 15usize;
        let keep = (total as f64 * 0.10).ceil() as usize;
        assert_eq!(keep, 2);

        // 1枚 → 1枚保持（最低1枚）
        let total = 1usize;
        let keep = (total as f64 * 0.10).ceil() as usize;
        let keep = keep.max(1);
        assert_eq!(keep, 1);

        // 5枚 → 1枚保持（ceil(0.5) = 1）
        let total = 5usize;
        let keep = (total as f64 * 0.10).ceil() as usize;
        assert_eq!(keep, 1);
    }

    #[test]
    fn test_cleanup_stats_preview_mode() {
        // BE-08a: confirmed=false → プレビューのみ、deleted=0
        let stats = CleanupStats {
            total: 100,
            deleted: 0,      // プレビューモード: 削除なし
            kept: 10,        // 最新10%保持予定
            bytes_freed: 0,
        };
        assert_eq!(stats.deleted, 0);
        assert_eq!(stats.kept, 10);
        // to_delete = total - kept
        assert_eq!(stats.total - stats.kept, 90);
    }

    #[test]
    fn test_cleanup_stats_execute_mode() {
        // BE-08: confirmed=true → 実際に削除
        let stats = CleanupStats {
            total: 100,
            deleted: 90,     // 実行モード: 90%削除
            kept: 10,        // 最新10%保持
            bytes_freed: 90 * 1024 * 1024,  // 90MB freed
        };
        assert_eq!(stats.deleted, 90);
        assert_eq!(stats.kept, 10);
        assert_eq!(stats.bytes_freed, 90 * 1024 * 1024);
    }
}
