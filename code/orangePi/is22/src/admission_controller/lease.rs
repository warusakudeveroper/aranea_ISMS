//! Lease management

use super::types::*;
use crate::error::{Error, Result};
use chrono::{Duration, Utc};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Lease manager
pub struct LeaseManager {
    leases: RwLock<HashMap<Uuid, ModalLease>>,
    /// User -> Lease ID mapping (one lease per user)
    user_leases: RwLock<HashMap<String, Uuid>>,
}

impl LeaseManager {
    /// Create new lease manager
    pub fn new() -> Self {
        Self {
            leases: RwLock::new(HashMap::new()),
            user_leases: RwLock::new(HashMap::new()),
        }
    }

    /// Check if user has an active lease
    pub async fn has_active_lease(&self, user_id: &str) -> bool {
        let user_leases = self.user_leases.read().await;
        user_leases.contains_key(user_id)
    }

    /// Get current usage (sum of costs)
    pub async fn current_usage(&self) -> i32 {
        let leases = self.leases.read().await;
        leases.values().map(|l| l.cost as i32).sum()
    }

    /// Get active lease count
    pub async fn active_count(&self) -> usize {
        let leases = self.leases.read().await;
        leases.len()
    }

    /// Create a new lease
    pub async fn create_lease(&self, request: LeaseRequest, cost: u32, ttl_sec: u64) -> ModalLease {
        let now = Utc::now();
        let lease = ModalLease {
            lease_id: Uuid::new_v4(),
            user_id: request.user_id.clone(),
            camera_id: request.camera_id,
            quality: request.quality,
            cost,
            created_at: now,
            expires_at: now + Duration::seconds(ttl_sec as i64),
            last_heartbeat: now,
        };

        {
            let mut leases = self.leases.write().await;
            leases.insert(lease.lease_id, lease.clone());
        }

        {
            let mut user_leases = self.user_leases.write().await;
            user_leases.insert(request.user_id, lease.lease_id);
        }

        tracing::info!(
            lease_id = %lease.lease_id,
            user_id = %lease.user_id,
            camera_id = %lease.camera_id,
            "Lease created"
        );

        lease
    }

    /// Update heartbeat
    pub async fn heartbeat(&self, lease_id: &Uuid, ttl_sec: u64) -> Result<HeartbeatResponse> {
        let mut leases = self.leases.write().await;

        if let Some(lease) = leases.get_mut(lease_id) {
            let now = Utc::now();
            lease.last_heartbeat = now;
            // Extend expiration
            lease.expires_at = now + Duration::seconds(ttl_sec as i64);

            let remaining = (lease.expires_at - now).num_seconds();

            Ok(HeartbeatResponse {
                ok: true,
                remaining_sec: remaining,
            })
        } else {
            Err(Error::NotFound(format!("Lease {} not found", lease_id)))
        }
    }

    /// Release a lease
    pub async fn release_lease(&self, lease_id: &Uuid) -> Result<()> {
        let user_id = {
            let mut leases = self.leases.write().await;
            if let Some(lease) = leases.remove(lease_id) {
                tracing::info!(
                    lease_id = %lease_id,
                    user_id = %lease.user_id,
                    "Lease released"
                );
                Some(lease.user_id)
            } else {
                None
            }
        };

        if let Some(user_id) = user_id {
            let mut user_leases = self.user_leases.write().await;
            user_leases.remove(&user_id);
        }

        Ok(())
    }

    /// Cleanup expired leases
    pub async fn cleanup(&self, heartbeat_grace_sec: u64) {
        let now = Utc::now();
        let grace = Duration::seconds(heartbeat_grace_sec as i64);

        let expired_ids: Vec<(Uuid, String)> = {
            let leases = self.leases.read().await;
            leases
                .iter()
                .filter(|(_, lease)| {
                    let heartbeat_expired = now - lease.last_heartbeat > grace;
                    let ttl_expired = now > lease.expires_at;
                    heartbeat_expired || ttl_expired
                })
                .map(|(id, lease)| (*id, lease.user_id.clone()))
                .collect()
        };

        if !expired_ids.is_empty() {
            let mut leases = self.leases.write().await;
            let mut user_leases = self.user_leases.write().await;

            for (id, user_id) in &expired_ids {
                leases.remove(id);
                user_leases.remove(user_id);
                tracing::info!(lease_id = %id, "Expired lease cleaned up");
            }
        }
    }

    /// Get lease by ID
    pub async fn get_lease(&self, lease_id: &Uuid) -> Option<ModalLease> {
        let leases = self.leases.read().await;
        leases.get(lease_id).cloned()
    }
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self::new()
    }
}
