//! AdmissionController - Load Control
//!
//! Issue #28 [IS22-GAP-002] AdmissionController implementation
//!
//! ## Responsibilities
//!
//! - Budget-based admission control
//! - Lease management with TTL/Heartbeat
//! - System health monitoring
//!
//! ## Design
//!
//! - Budget Model: Fixed capacity with reserved units
//! - Lease/Heartbeat: Server-issued, auto-cleanup
//! - Hysteresis: Prevents flapping

mod lease;
mod types;

pub use lease::LeaseManager;
pub use types::*;

use crate::config_store::AdmissionPolicy;
use crate::error::{Error, Result};
use crate::state::SystemHealth;
use std::sync::Arc;
use tokio::sync::RwLock;

/// AdmissionController instance
pub struct AdmissionController {
    policy: Arc<RwLock<AdmissionPolicy>>,
    lease_manager: LeaseManager,
    system_health: Arc<RwLock<SystemHealth>>,
}

impl AdmissionController {
    /// Create new AdmissionController
    pub fn new(
        policy: AdmissionPolicy,
        system_health: Arc<RwLock<SystemHealth>>,
    ) -> Self {
        Self {
            policy: Arc::new(RwLock::new(policy)),
            lease_manager: LeaseManager::new(),
            system_health,
        }
    }

    /// Check if a modal lease can be admitted
    pub async fn can_admit(&self, request: &LeaseRequest) -> Result<AdmissionResult> {
        // 1. System overload check
        {
            let health = self.system_health.read().await;
            if health.overloaded {
                return Ok(AdmissionResult::Rejected(RejectReason::SystemOverload));
            }
        }

        // 2. Check if user already has an active modal
        if self.lease_manager.has_active_lease(&request.user_id).await {
            return Ok(AdmissionResult::Rejected(RejectReason::UserAlreadyHasModal));
        }

        // 3. Budget check
        let policy = self.policy.read().await;
        let cost = match request.quality {
            StreamQuality::Main => policy.main_stream_cost,
            StreamQuality::Sub => policy.sub_stream_cost,
        };

        let current_usage = self.lease_manager.current_usage().await;
        let modal_budget = policy.total_stream_units
            - policy.reserved_baseline_units
            - (policy.max_ui_users * policy.sub_stream_cost); // Reserve for suggest

        if current_usage + cost > modal_budget {
            return Ok(AdmissionResult::Rejected(RejectReason::OverCapacity));
        }

        Ok(AdmissionResult::Admitted(cost as u32))
    }

    /// Request a new lease
    pub async fn request_lease(&self, request: LeaseRequest) -> Result<ModalLease> {
        let result = self.can_admit(&request).await?;

        match result {
            AdmissionResult::Admitted(cost) => {
                let policy = self.policy.read().await;
                let lease = self.lease_manager.create_lease(
                    request,
                    cost,
                    policy.modal_ttl_sec as u64,
                ).await;
                Ok(lease)
            }
            AdmissionResult::Rejected(reason) => {
                Err(match reason {
                    RejectReason::SystemOverload => Error::SystemOverload(
                        "サーバーが高負荷のためしばらく経ってからお試しください".to_string(),
                    ),
                    RejectReason::UserAlreadyHasModal => Error::Conflict(
                        "既に別のカメラを表示中です".to_string(),
                    ),
                    RejectReason::OverCapacity => Error::OverCapacity(
                        "現在多数のアクセスがありストリーム数が上限に達しています".to_string(),
                    ),
                })
            }
        }
    }

    /// Heartbeat for a lease
    pub async fn heartbeat(&self, lease_id: &uuid::Uuid) -> Result<HeartbeatResponse> {
        let policy = self.policy.read().await;
        self.lease_manager.heartbeat(lease_id, policy.modal_ttl_sec as u64).await
    }

    /// Release a lease
    pub async fn release_lease(&self, lease_id: &uuid::Uuid) -> Result<()> {
        self.lease_manager.release_lease(lease_id).await
    }

    /// Get current system status
    pub async fn get_status(&self) -> AdmissionStatus {
        let policy = self.policy.read().await;
        let current_usage = self.lease_manager.current_usage().await;
        let active_modals = self.lease_manager.active_count().await;
        let health = self.system_health.read().await;

        let modal_budget = policy.total_stream_units
            - policy.reserved_baseline_units
            - (policy.max_ui_users * policy.sub_stream_cost);

        AdmissionStatus {
            healthy: !health.overloaded,
            cpu_percent: health.cpu_percent,
            memory_percent: health.memory_percent,
            active_modals: active_modals as i32,
            modal_budget_remaining: modal_budget - current_usage,
            suggest_active: false, // TODO: Get from SuggestEngine
        }
    }

    /// Update policy
    pub async fn update_policy(&self, policy: AdmissionPolicy) {
        let mut p = self.policy.write().await;
        *p = policy;
    }

    /// Cleanup expired leases (should be called periodically)
    pub async fn cleanup(&self) {
        let policy = self.policy.read().await;
        self.lease_manager.cleanup(policy.heartbeat_grace_sec as u64).await;
    }

    /// Get lease by ID (for PTZ authentication)
    pub async fn get_lease_by_id(&self, lease_id: &uuid::Uuid) -> Option<ModalLease> {
        self.lease_manager.get_lease(lease_id).await
    }
}
