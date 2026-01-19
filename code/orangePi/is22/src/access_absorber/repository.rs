//! Access Absorber Repository - Database operations

use super::types::*;
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Database row for camera_access_limits
#[derive(Debug, sqlx::FromRow)]
struct AccessLimitRow {
    pub id: i32,
    pub family: String,
    pub display_name: String,
    pub max_concurrent_streams: u8,
    pub min_reconnect_interval_ms: u32,
    pub require_exclusive_lock: bool,
    pub connection_timeout_ms: u32,
    pub model_pattern: Option<String>,
    pub notes: Option<String>,
}

/// Database row for camera_stream_sessions
#[derive(Debug, sqlx::FromRow)]
struct StreamSessionRow {
    pub session_id: String,
    pub camera_id: String,
    pub stream_type: String,
    pub purpose: String,
    pub client_id: String,
    pub started_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_heartbeat_at: DateTime<Utc>,
    pub status: String,
}

pub struct AccessAbsorberRepository {
    pub(crate) pool: MySqlPool,
}

impl AccessAbsorberRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Get pool reference (for internal use)
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    // ========================================================================
    // Connection Limits
    // ========================================================================

    /// Get connection limits for a family
    pub async fn get_limits(&self, family: &str) -> Result<Option<ConnectionLimits>> {
        let row: Option<AccessLimitRow> = sqlx::query_as(
            r#"
            SELECT id, family, display_name, max_concurrent_streams,
                   min_reconnect_interval_ms, require_exclusive_lock,
                   connection_timeout_ms, model_pattern, notes
            FROM camera_access_limits
            WHERE family = ?
            "#,
        )
        .bind(family)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(|r| ConnectionLimits {
            family: r.family,
            display_name: r.display_name,
            max_concurrent_streams: r.max_concurrent_streams,
            min_reconnect_interval_ms: r.min_reconnect_interval_ms,
            require_exclusive_lock: r.require_exclusive_lock,
            connection_timeout_ms: r.connection_timeout_ms,
            model_pattern: r.model_pattern,
        }))
    }

    /// Get all connection limits
    pub async fn get_all_limits(&self) -> Result<Vec<ConnectionLimits>> {
        let rows: Vec<AccessLimitRow> = sqlx::query_as(
            r#"
            SELECT id, family, display_name, max_concurrent_streams,
                   min_reconnect_interval_ms, require_exclusive_lock,
                   connection_timeout_ms, model_pattern, notes
            FROM camera_access_limits
            ORDER BY family
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| ConnectionLimits {
                family: r.family,
                display_name: r.display_name,
                max_concurrent_streams: r.max_concurrent_streams,
                min_reconnect_interval_ms: r.min_reconnect_interval_ms,
                require_exclusive_lock: r.require_exclusive_lock,
                connection_timeout_ms: r.connection_timeout_ms,
                model_pattern: r.model_pattern,
            })
            .collect())
    }

    /// Get effective limits for a camera (with override support)
    pub async fn get_effective_limits(
        &self,
        camera_id: &str,
        default_family: &str,
    ) -> Result<ConnectionLimits> {
        // First check if camera has access_family set
        let camera_family: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT access_family, access_limit_override
            FROM cameras
            WHERE camera_id = ?
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        let family = camera_family
            .as_ref()
            .and_then(|(f, _)| f.clone())
            .unwrap_or_else(|| default_family.to_string());

        // Get base limits
        let mut limits = self
            .get_limits(&family)
            .await?
            .unwrap_or_else(ConnectionLimits::default);

        // Apply camera-specific override if present
        if let Some((_, Some(override_json))) = camera_family {
            if let Ok(override_val) = serde_json::from_str::<serde_json::Value>(&override_json) {
                if let Some(max) = override_val
                    .get("max_concurrent_streams")
                    .and_then(|v| v.as_u64())
                {
                    limits.max_concurrent_streams = max as u8;
                }
                if let Some(interval) = override_val
                    .get("min_reconnect_interval_ms")
                    .and_then(|v| v.as_u64())
                {
                    limits.min_reconnect_interval_ms = interval as u32;
                }
                if let Some(lock) = override_val
                    .get("require_exclusive_lock")
                    .and_then(|v| v.as_bool())
                {
                    limits.require_exclusive_lock = lock;
                }
            }
        }

        Ok(limits)
    }

    // ========================================================================
    // Stream Sessions
    // ========================================================================

    /// Get active sessions for a camera
    pub async fn get_active_sessions(&self, camera_id: &str) -> Result<Vec<ActiveStream>> {
        let rows: Vec<StreamSessionRow> = sqlx::query_as(
            r#"
            SELECT session_id, camera_id, stream_type, purpose, client_id,
                   started_at, expires_at, last_heartbeat_at, status
            FROM camera_stream_sessions
            WHERE camera_id = ? AND status = 'active'
            "#,
        )
        .bind(camera_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| ActiveStream {
                session_id: r.session_id,
                camera_id: r.camera_id,
                stream_type: match r.stream_type.as_str() {
                    "sub" => StreamType::Sub,
                    _ => StreamType::Main,
                },
                purpose: match r.purpose.as_str() {
                    "click_modal" => StreamPurpose::ClickModal,
                    "suggest_play" => StreamPurpose::SuggestPlay,
                    "polling" => StreamPurpose::Polling,
                    "snapshot" => StreamPurpose::Snapshot,
                    _ => StreamPurpose::HealthCheck,
                },
                client_id: r.client_id,
                started_at: r.started_at,
                expires_at: r.expires_at,
            })
            .collect())
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        camera_id: &str,
        stream_type: StreamType,
        purpose: StreamPurpose,
        client_id: &str,
        expires_in_secs: Option<i64>,
    ) -> Result<String> {
        let session_id = format!("sess-{}", Uuid::new_v4());
        let stream_type_str = match stream_type {
            StreamType::Main => "main",
            StreamType::Sub => "sub",
        };
        let purpose_str = match purpose {
            StreamPurpose::ClickModal => "click_modal",
            StreamPurpose::SuggestPlay => "suggest_play",
            StreamPurpose::Polling => "polling",
            StreamPurpose::Snapshot => "snapshot",
            StreamPurpose::HealthCheck => "health_check",
        };

        let expires_at = expires_in_secs.map(|secs| Utc::now() + chrono::Duration::seconds(secs));

        sqlx::query(
            r#"
            INSERT INTO camera_stream_sessions
            (session_id, camera_id, stream_type, purpose, client_id, expires_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session_id)
        .bind(camera_id)
        .bind(stream_type_str)
        .bind(purpose_str)
        .bind(client_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        debug!(
            session_id = %session_id,
            camera_id = %camera_id,
            purpose = %purpose_str,
            "Created stream session"
        );

        Ok(session_id)
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM camera_stream_sessions
            WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Update session heartbeat
    pub async fn update_heartbeat(&self, session_id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE camera_stream_sessions
            SET last_heartbeat_at = NOW(3)
            WHERE session_id = ? AND status = 'active'
            "#,
        )
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Mark session as releasing
    pub async fn mark_releasing(&self, session_id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE camera_stream_sessions
            SET status = 'releasing'
            WHERE session_id = ? AND status = 'active'
            "#,
        )
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Cleanup expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<u64> {
        // First log expired sessions
        let expired_count = sqlx::query(
            r#"
            INSERT INTO camera_connection_events
            (camera_id, event_type, purpose, client_id)
            SELECT camera_id, 'disconnect_timeout', purpose, client_id
            FROM camera_stream_sessions
            WHERE status = 'active'
            AND (expires_at < NOW() OR last_heartbeat_at < DATE_SUB(NOW(), INTERVAL 2 MINUTE))
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?
        .rows_affected();

        // Then delete them
        let deleted = sqlx::query(
            r#"
            DELETE FROM camera_stream_sessions
            WHERE status = 'active'
            AND (expires_at < NOW() OR last_heartbeat_at < DATE_SUB(NOW(), INTERVAL 2 MINUTE))
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?
        .rows_affected();

        if deleted > 0 {
            info!(deleted = deleted, "Cleaned up expired stream sessions");
        }

        Ok(deleted)
    }

    // ========================================================================
    // Camera State
    // ========================================================================

    /// Get last disconnect time for a camera
    pub async fn get_last_disconnect(&self, camera_id: &str) -> Result<Option<DateTime<Utc>>> {
        let result: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
            r#"
            SELECT last_disconnect_at
            FROM cameras
            WHERE camera_id = ?
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result.and_then(|(dt,)| dt))
    }

    /// Update last disconnect time
    pub async fn update_last_disconnect(&self, camera_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE cameras
            SET last_disconnect_at = NOW(3)
            WHERE camera_id = ?
            "#,
        )
        .bind(camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Update camera access family
    pub async fn update_access_family(&self, camera_id: &str, family: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE cameras
            SET access_family = ?
            WHERE camera_id = ?
            "#,
        )
        .bind(family)
        .bind(camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    // ========================================================================
    // Event Logging
    // ========================================================================

    /// Log connection event
    pub async fn log_event(
        &self,
        camera_id: &str,
        event_type: ConnectionEventType,
        purpose: Option<StreamPurpose>,
        client_id: Option<&str>,
        blocked_reason: Option<&str>,
        wait_time_ms: Option<u32>,
        preempted_by: Option<&str>,
    ) -> Result<()> {
        let event_type_str = match event_type {
            ConnectionEventType::ConnectSuccess => "connect_success",
            ConnectionEventType::ConnectBlockedConcurrent => "connect_blocked_concurrent",
            ConnectionEventType::ConnectBlockedInterval => "connect_blocked_interval",
            ConnectionEventType::ConnectTimeout => "connect_timeout",
            ConnectionEventType::ConnectPreempted => "connect_preempted",
            ConnectionEventType::DisconnectNormal => "disconnect_normal",
            ConnectionEventType::DisconnectPreempted => "disconnect_preempted",
            ConnectionEventType::DisconnectTimeout => "disconnect_timeout",
            ConnectionEventType::DisconnectError => "disconnect_error",
        };

        let purpose_str = purpose.map(|p| match p {
            StreamPurpose::ClickModal => "click_modal",
            StreamPurpose::SuggestPlay => "suggest_play",
            StreamPurpose::Polling => "polling",
            StreamPurpose::Snapshot => "snapshot",
            StreamPurpose::HealthCheck => "health_check",
        });

        sqlx::query(
            r#"
            INSERT INTO camera_connection_events
            (camera_id, event_type, purpose, client_id, blocked_reason, wait_time_ms, preempted_by)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(camera_id)
        .bind(event_type_str)
        .bind(purpose_str)
        .bind(client_id)
        .bind(blocked_reason)
        .bind(wait_time_ms)
        .bind(preempted_by)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Get connection event statistics
    pub async fn get_event_stats(
        &self,
        camera_id: &str,
        since: DateTime<Utc>,
    ) -> Result<ConnectionStats> {
        let row: Option<(i64, i64, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                SUM(CASE WHEN event_type = 'connect_success' THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN event_type = 'connect_blocked_concurrent' THEN 1 ELSE 0 END) as concurrent_blocked,
                SUM(CASE WHEN event_type = 'connect_blocked_interval' THEN 1 ELSE 0 END) as interval_blocked,
                SUM(CASE WHEN event_type LIKE 'disconnect_preempted%' THEN 1 ELSE 0 END) as preempted
            FROM camera_connection_events
            WHERE camera_id = ? AND created_at >= ?
            "#,
        )
        .bind(camera_id)
        .bind(since)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row
            .map(|(s, c, i, p)| ConnectionStats {
                success_count: s as u32,
                concurrent_limit_hits: c as u32,
                interval_limit_hits: i as u32,
                preemption_count: p as u32,
            })
            .unwrap_or_default())
    }
}

/// Connection statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub success_count: u32,
    pub concurrent_limit_hits: u32,
    pub interval_limit_hits: u32,
    pub preemption_count: u32,
}
