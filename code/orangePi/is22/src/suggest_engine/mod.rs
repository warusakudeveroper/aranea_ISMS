//! SuggestEngine - "What to watch now" decision
//!
//! ## Responsibilities
//!
//! - Score-based camera selection
//! - GlobalSuggestState management
//! - Real-time distribution trigger

use crate::config_store::SuggestPolicy;
use crate::event_log_service::DetectionEvent;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global suggest state (shared by all clients)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalSuggestState {
    pub active: bool,
    pub camera_id: Option<String>,
    pub stream_profile: StreamProfile,
    pub reason_event_id: Option<u64>,
    pub started_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub policy: Option<SuggestPolicyType>,
    pub current_score: i32,
}

/// Stream profile
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StreamProfile {
    #[default]
    Sub,
    Main,
}

/// Suggest policy type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestPolicyType {
    HighSeverity,
    HazardDetected,
    UnknownFlag,
    ScheduledRound,
    Manual,
}

/// SuggestEngine instance
pub struct SuggestEngine {
    state: Arc<RwLock<GlobalSuggestState>>,
    policy: Arc<RwLock<SuggestPolicy>>,
}

impl SuggestEngine {
    /// Create new SuggestEngine
    pub fn new(policy: SuggestPolicy) -> Self {
        Self {
            state: Arc::new(RwLock::new(GlobalSuggestState::default())),
            policy: Arc::new(RwLock::new(policy)),
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> GlobalSuggestState {
        let state = self.state.read().await;
        state.clone()
    }

    /// Calculate score for an event
    pub fn calculate_score(event: &DetectionEvent, camera_weight: i32) -> i32 {
        let mut score = 0;

        // Severity
        score += event.severity * 20;

        // Tag bonuses
        if event.tags.iter().any(|t| t.starts_with("hazard.")) {
            score += 100;
        }
        if event.unknown_flag {
            score += 50;
        }
        if event.primary_event == "human" {
            score += 30;
        }
        if event.primary_event == "animal" {
            score += 20;
        }
        if event.primary_event == "camera_issue" {
            score += 80;
        }

        // Camera weight
        score = score * camera_weight / 5;

        // Time decay (1 point per second)
        let elapsed = (Utc::now() - event.captured_at).num_seconds() as i32;
        score -= elapsed;

        score.max(0)
    }

    /// Process new event and update suggest state
    pub async fn process_event(&self, event: &DetectionEvent, camera_weight: i32) -> bool {
        let new_score = Self::calculate_score(event, camera_weight);
        let policy = self.policy.read().await;

        // Check minimum threshold
        if new_score < policy.min_score_threshold {
            return false;
        }

        let mut state = self.state.write().await;

        // Hysteresis check
        let should_update = if state.active {
            (new_score as f32) > (state.current_score as f32 * policy.hysteresis_ratio)
        } else {
            true
        };

        if should_update {
            let now = Utc::now();
            state.active = true;
            state.camera_id = Some(event.camera_id.clone());
            state.stream_profile = StreamProfile::Sub;
            state.reason_event_id = Some(event.event_id);
            state.started_at = Some(now);
            state.expires_at = Some(now + Duration::seconds(policy.ttl_sec as i64));
            state.current_score = new_score;

            // Determine policy type
            state.policy = if event.tags.iter().any(|t| t.starts_with("hazard.")) {
                Some(SuggestPolicyType::HazardDetected)
            } else if event.unknown_flag {
                Some(SuggestPolicyType::UnknownFlag)
            } else if event.severity >= 2 {
                Some(SuggestPolicyType::HighSeverity)
            } else {
                None
            };

            tracing::info!(
                camera_id = %event.camera_id,
                score = new_score,
                "Suggest state updated"
            );

            true
        } else {
            false
        }
    }

    /// Check and expire suggest if TTL reached
    pub async fn check_expiration(&self) -> bool {
        let mut state = self.state.write().await;

        if state.active {
            if let Some(expires_at) = state.expires_at {
                if Utc::now() > expires_at {
                    *state = GlobalSuggestState::default();
                    tracing::info!("Suggest state expired");
                    return true;
                }
            }
        }

        false
    }

    /// Clear suggest manually
    pub async fn clear(&self) {
        let mut state = self.state.write().await;
        *state = GlobalSuggestState::default();
        tracing::info!("Suggest state cleared");
    }

    /// Set manual suggest
    pub async fn set_manual(&self, camera_id: String, duration_sec: i64) {
        let now = Utc::now();
        let mut state = self.state.write().await;

        state.active = true;
        state.camera_id = Some(camera_id);
        state.stream_profile = StreamProfile::Sub;
        state.reason_event_id = None;
        state.started_at = Some(now);
        state.expires_at = Some(now + Duration::seconds(duration_sec));
        state.policy = Some(SuggestPolicyType::Manual);
        state.current_score = 0;

        tracing::info!("Manual suggest set");
    }

    /// Update policy
    pub async fn update_policy(&self, policy: SuggestPolicy) {
        let mut p = self.policy.write().await;
        *p = policy;
    }
}
