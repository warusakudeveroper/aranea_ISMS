//! Access Absorber Service - Core business logic

use super::repository::{AccessAbsorberRepository, ConnectionStats};
use super::types::*;
use crate::error::{Error, Result};
use chrono::{Duration, Utc};
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// In-memory cache for connection limits
#[derive(Debug, Default)]
struct LimitsCache {
    limits: HashMap<String, ConnectionLimits>,
    loaded_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Access Absorber Service
///
/// Manages camera stream connection limits with:
/// - Concurrent stream limiting
/// - Reconnection interval enforcement
/// - Priority-based preemption
/// - User-friendly error messages
pub struct AccessAbsorberService {
    repo: AccessAbsorberRepository,
    cache: Arc<RwLock<LimitsCache>>,
    /// Default session expiry in seconds (1 hour)
    default_session_expiry_secs: i64,
}

impl AccessAbsorberService {
    /// Create new service
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            repo: AccessAbsorberRepository::new(pool),
            cache: Arc::new(RwLock::new(LimitsCache::default())),
            default_session_expiry_secs: 3600,
        }
    }

    /// Initialize service and load cache
    pub async fn init(&self) -> Result<()> {
        self.refresh_cache().await?;
        info!("AccessAbsorberService initialized");
        Ok(())
    }

    /// Refresh limits cache from database
    pub async fn refresh_cache(&self) -> Result<()> {
        let limits = self.repo.get_all_limits().await?;
        let mut cache = self.cache.write().await;

        cache.limits.clear();
        for limit in limits {
            cache.limits.insert(limit.family.clone(), limit);
        }
        cache.loaded_at = Some(Utc::now());

        info!(
            limit_count = cache.limits.len(),
            "Access limits cache refreshed"
        );

        Ok(())
    }

    /// Get limits from cache (falls back to DB if not cached)
    async fn get_cached_limits(&self, family: &str) -> ConnectionLimits {
        let cache = self.cache.read().await;
        cache
            .limits
            .get(family)
            .cloned()
            .unwrap_or_else(|| {
                // Return unknown limits as fallback
                cache.limits.get("unknown").cloned().unwrap_or_default()
            })
    }

    // ========================================================================
    // Main API
    // ========================================================================

    /// Acquire a stream for a camera
    ///
    /// This is the main entry point for acquiring stream access.
    /// Returns AcquireStreamResult on success (includes token + optional preemption info),
    /// or an AbsorberError on failure.
    pub async fn acquire_stream(
        &self,
        camera_id: &str,
        purpose: StreamPurpose,
        client_id: &str,
        stream_type: StreamType,
        allow_preempt: bool,
    ) -> std::result::Result<AcquireStreamResult, AbsorberError> {
        let family = self.get_camera_family(camera_id).await;
        let limits = self.get_cached_limits(&family).await;

        debug!(
            camera_id = %camera_id,
            family = %family,
            purpose = ?purpose,
            max_concurrent = limits.max_concurrent_streams,
            "Attempting to acquire stream"
        );

        // 1. Check active sessions
        let active_sessions = self
            .repo
            .get_active_sessions(camera_id)
            .await
            .map_err(|e| AbsorberError::Internal {
                message: e.to_string(),
            })?;

        let current_count = active_sessions.len() as u8;

        // Track preemption info if it occurs
        let mut preemption_info: Option<PreemptionInfo> = None;

        // 2. Check concurrent limit
        if current_count >= limits.max_concurrent_streams {
            // Try preemption if allowed
            if allow_preempt {
                if let Some(preempted) = self
                    .try_preempt(&active_sessions, &purpose, client_id, camera_id)
                    .await?
                {
                    info!(
                        camera_id = %camera_id,
                        preempted_session = %preempted.session_id,
                        preempted_purpose = ?preempted.preempted_purpose,
                        preempted_client = %preempted.preempted_client_id,
                        "Preempted lower priority session"
                    );
                    // Store preemption info to return to caller
                    preemption_info = Some(preempted);
                    // Preemption successful, continue with acquisition
                } else {
                    return Err(self.make_concurrent_error(
                        camera_id,
                        &limits,
                        current_count,
                        &active_sessions,
                    ));
                }
            } else {
                return Err(self.make_concurrent_error(
                    camera_id,
                    &limits,
                    current_count,
                    &active_sessions,
                ));
            }
        }

        // 3. Check reconnection interval
        if limits.min_reconnect_interval_ms > 0 {
            if let Some(last_disconnect) = self
                .repo
                .get_last_disconnect(camera_id)
                .await
                .map_err(|e| AbsorberError::Internal {
                    message: e.to_string(),
                })?
            {
                let elapsed_ms = (Utc::now() - last_disconnect).num_milliseconds() as u32;
                if elapsed_ms < limits.min_reconnect_interval_ms {
                    let remaining_ms = limits.min_reconnect_interval_ms - elapsed_ms;

                    // If short wait, sleep instead of returning error
                    if remaining_ms < 1000 {
                        debug!(
                            camera_id = %camera_id,
                            wait_ms = remaining_ms,
                            "Short wait for reconnect interval"
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(remaining_ms as u64))
                            .await;
                    } else {
                        // Log the event
                        let _ = self
                            .repo
                            .log_event(
                                camera_id,
                                ConnectionEventType::ConnectBlockedInterval,
                                Some(purpose),
                                Some(client_id),
                                Some(&format!(
                                    "Interval not met: {} < {}",
                                    elapsed_ms, limits.min_reconnect_interval_ms
                                )),
                                Some(remaining_ms),
                                None,
                            )
                            .await;

                        return Err(AbsorberError::ReconnectIntervalNotMet {
                            camera_id: camera_id.to_string(),
                            family: limits.display_name.clone(),
                            required_interval_ms: limits.min_reconnect_interval_ms,
                            remaining_ms,
                        });
                    }
                }
            }
        }

        // 4. Check exclusive lock (if required and not already holding)
        if limits.require_exclusive_lock && current_count > 0 && preemption_info.is_none() {
            // Check if current client already has a session
            let client_has_session = active_sessions.iter().any(|s| s.client_id == client_id);
            if !client_has_session {
                let holder = &active_sessions[0];
                return Err(AbsorberError::ExclusiveLockFailed {
                    camera_id: camera_id.to_string(),
                    held_by: holder.client_id.clone(),
                    held_purpose: holder.purpose.to_japanese().to_string(),
                });
            }
        }

        // 5. Create session
        let session_id = self
            .repo
            .create_session(
                camera_id,
                stream_type,
                purpose,
                client_id,
                Some(self.default_session_expiry_secs),
            )
            .await
            .map_err(|e| AbsorberError::Internal {
                message: e.to_string(),
            })?;

        // 6. Log success event
        let _ = self
            .repo
            .log_event(
                camera_id,
                ConnectionEventType::ConnectSuccess,
                Some(purpose),
                Some(client_id),
                None,
                None,
                None,
            )
            .await;

        info!(
            camera_id = %camera_id,
            session_id = %session_id,
            purpose = ?purpose,
            preempted = preemption_info.is_some(),
            "Stream acquired successfully"
        );

        let token = StreamToken {
            session_id,
            camera_id: camera_id.to_string(),
            stream_type,
            purpose,
            client_id: client_id.to_string(),
            acquired_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::seconds(self.default_session_expiry_secs)),
        };

        Ok(AcquireStreamResult {
            token,
            preemption: preemption_info,
        })
    }

    /// Release a stream
    pub async fn release_stream(&self, session_id: &str) -> Result<()> {
        // Get session info before deleting
        let sessions = self.get_all_active_sessions().await?;
        let session = sessions.iter().find(|s| s.session_id == session_id);

        if let Some(session) = session {
            let camera_id = &session.camera_id;

            // Delete session
            self.repo.delete_session(session_id).await?;

            // Update last disconnect time
            self.repo.update_last_disconnect(camera_id).await?;

            // Log event
            let _ = self
                .repo
                .log_event(
                    camera_id,
                    ConnectionEventType::DisconnectNormal,
                    Some(session.purpose),
                    Some(&session.client_id),
                    None,
                    None,
                    None,
                )
                .await;

            debug!(
                session_id = %session_id,
                camera_id = %camera_id,
                "Stream released"
            );
        } else {
            // Session not found, just try to delete anyway
            self.repo.delete_session(session_id).await?;
        }

        Ok(())
    }

    /// Update session heartbeat
    pub async fn heartbeat(&self, session_id: &str) -> Result<bool> {
        self.repo.update_heartbeat(session_id).await
    }

    /// Get connection state for a camera
    pub async fn get_connection_state(&self, camera_id: &str) -> Result<CameraConnectionState> {
        let family = self.get_camera_family(camera_id).await;
        let limits = self.get_cached_limits(&family).await;
        let active_sessions = self.repo.get_active_sessions(camera_id).await?;
        let last_disconnect = self.repo.get_last_disconnect(camera_id).await?;

        let current_count = active_sessions.len() as u8;
        let available_slots = limits.max_concurrent_streams.saturating_sub(current_count);

        // Calculate next available time based on reconnection interval
        let next_available_at = if available_slots == 0 {
            None // No slots available
        } else if limits.min_reconnect_interval_ms > 0 {
            last_disconnect.map(|dt| {
                dt + Duration::milliseconds(limits.min_reconnect_interval_ms as i64)
            })
        } else {
            None
        };

        let can_connect = available_slots > 0
            && next_available_at
                .map(|t| t <= Utc::now())
                .unwrap_or(true);

        Ok(CameraConnectionState {
            camera_id: camera_id.to_string(),
            family: AccessFamily::from_str(&family),
            limits,
            active_streams: active_sessions,
            last_disconnect_at: last_disconnect,
            available_slots,
            can_connect,
            next_available_at,
        })
    }

    /// Get stream status for API response
    pub async fn get_stream_status(&self, camera_id: &str) -> Result<StreamStatusResponse> {
        let state = self.get_connection_state(camera_id).await?;

        let active_streams: Vec<ActiveStreamInfo> = state
            .active_streams
            .iter()
            .map(|s| ActiveStreamInfo {
                stream_id: s.session_id.clone(),
                stream_type: s.stream_type,
                purpose: s.purpose,
                client_id: s.client_id.clone(),
                started_at: s.started_at,
                duration_sec: s.duration_sec(),
            })
            .collect();

        let user_message = if !state.can_connect && state.available_slots == 0 {
            Some(
                AbsorberError::ConcurrentLimitReached {
                    camera_id: camera_id.to_string(),
                    family: state.limits.family.clone(),
                    max_streams: state.limits.max_concurrent_streams,
                    current_count: state.active_streams.len() as u8,
                    current_purposes: state
                        .active_streams
                        .iter()
                        .map(|s| s.purpose.to_japanese().to_string())
                        .collect(),
                }
                .to_user_message(),
            )
        } else {
            None
        };

        Ok(StreamStatusResponse {
            camera_id: camera_id.to_string(),
            family: state.family.as_str().to_string(),
            limits: state.limits,
            current_state: CurrentStreamState {
                active_streams,
                available_slots: state.available_slots,
                can_connect: state.can_connect,
                next_available_at: state.next_available_at,
            },
            user_message,
        })
    }

    /// Cleanup expired sessions (call periodically)
    pub async fn cleanup_expired(&self) -> Result<u64> {
        self.repo.cleanup_expired_sessions().await
    }

    /// Get connection statistics for a camera
    pub async fn get_stats(
        &self,
        camera_id: &str,
        since_hours: i64,
    ) -> Result<ConnectionStats> {
        let since = Utc::now() - Duration::hours(since_hours);
        self.repo.get_event_stats(camera_id, since).await
    }

    /// Get all limits (for settings UI)
    pub async fn get_all_limits(&self) -> Result<Vec<ConnectionLimits>> {
        self.repo.get_all_limits().await
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Get camera family (from cache or DB)
    async fn get_camera_family(&self, camera_id: &str) -> String {
        // Try to get from DB
        if let Ok(limits) = self.repo.get_effective_limits(camera_id, "unknown").await {
            return limits.family;
        }
        "unknown".to_string()
    }

    /// Get all active sessions across all cameras
    async fn get_all_active_sessions(&self) -> Result<Vec<ActiveStream>> {
        // This is a simplified version - in production, you might want pagination
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT camera_id
            FROM camera_stream_sessions
            WHERE status = 'active'
            "#,
        )
        .fetch_all(self.repo.pool())
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        let mut all_sessions = Vec::new();
        for (camera_id,) in rows {
            if let Ok(sessions) = self.repo.get_active_sessions(&camera_id).await {
                all_sessions.extend(sessions);
            }
        }

        Ok(all_sessions)
    }

    /// Try to preempt a lower priority session
    /// Returns PreemptionInfo on successful preemption
    async fn try_preempt(
        &self,
        active_sessions: &[ActiveStream],
        new_purpose: &StreamPurpose,
        new_client_id: &str,
        camera_id: &str,
    ) -> std::result::Result<Option<PreemptionInfo>, AbsorberError> {
        // Find lowest priority session that can be preempted
        let preemptable = active_sessions
            .iter()
            .filter(|s| new_purpose.can_preempt(&s.purpose))
            .max_by_key(|s| s.purpose.priority()); // Highest priority value = lowest priority

        if let Some(session) = preemptable {
            let session_id = session.session_id.clone();
            let old_client = session.client_id.clone();
            let old_purpose = session.purpose;

            // Delete the session
            self.repo
                .delete_session(&session_id)
                .await
                .map_err(|e| AbsorberError::Internal {
                    message: e.to_string(),
                })?;

            // Update last disconnect
            let _ = self.repo.update_last_disconnect(camera_id).await;

            // Log preemption event
            let _ = self
                .repo
                .log_event(
                    camera_id,
                    ConnectionEventType::DisconnectPreempted,
                    Some(old_purpose),
                    Some(&old_client),
                    Some(&format!("Preempted by {:?}", new_purpose)),
                    None,
                    Some(new_client_id),
                )
                .await;

            // Calculate exit delay based on preempted purpose
            let exit_delay_sec = match old_purpose {
                StreamPurpose::SuggestPlay => 5,  // Suggest gets 5 seconds to show feedback
                StreamPurpose::HealthCheck => 0, // Immediate
                _ => 3,                           // Default 3 seconds
            };

            Ok(Some(PreemptionInfo {
                session_id,
                camera_id: camera_id.to_string(),
                preempted_purpose: old_purpose,
                preempted_client_id: old_client,
                preempted_by_purpose: *new_purpose,
                preempted_by_client_id: new_client_id.to_string(),
                exit_delay_sec,
            }))
        } else {
            Ok(None)
        }
    }

    /// Create concurrent limit error
    fn make_concurrent_error(
        &self,
        camera_id: &str,
        limits: &ConnectionLimits,
        current_count: u8,
        active_sessions: &[ActiveStream],
    ) -> AbsorberError {
        // Log the event (fire and forget)
        let camera_id_clone = camera_id.to_string();
        let repo = &self.repo;
        let purposes: Vec<String> = active_sessions
            .iter()
            .map(|s| s.purpose.to_japanese().to_string())
            .collect();

        AbsorberError::ConcurrentLimitReached {
            camera_id: camera_id.to_string(),
            family: limits.display_name.clone(),
            max_streams: limits.max_concurrent_streams,
            current_count,
            current_purposes: purposes,
        }
    }
}
