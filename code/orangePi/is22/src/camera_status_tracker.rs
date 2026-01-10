//! Camera Status Tracker
//!
//! Tracks camera connection status changes to detect lost/recovered events.
//! Only transitions are logged to avoid spamming AI Event Log.
//!
//! ## Phase 2: CameraRegistry Integration
//! - Registration state tracking (pending/registered/failed)
//! - Combined status view for Paraclate integration

use std::collections::HashMap;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Camera connection status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraConnectionStatus {
    /// Initial state (never polled)
    Unknown,
    /// Camera is online and responding
    Online,
    /// Camera is offline or not responding
    Offline,
}

/// Camera registration state (Phase 2: CameraRegistry)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraRegistrationState {
    /// Not yet registered to Paraclate
    Pending,
    /// Successfully registered (has CIC)
    Registered,
    /// Registration failed
    Failed,
}

impl Default for CameraRegistrationState {
    fn default() -> Self {
        Self::Pending
    }
}

/// Combined camera status for Paraclate integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraFullStatus {
    pub camera_id: String,
    pub connection: CameraConnectionStatus,
    pub registration: CameraRegistrationState,
    pub lacis_id: Option<String>,
    pub cic: Option<String>,
}

/// Camera status transition event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraStatusEvent {
    /// Camera went from Online to Offline
    Lost,
    /// Camera went from Offline to Online
    Recovered,
}

/// Extended camera info for tracking
#[derive(Debug, Clone)]
struct CameraTrackerEntry {
    connection: CameraConnectionStatus,
    registration: CameraRegistrationState,
    lacis_id: Option<String>,
    cic: Option<String>,
}

impl Default for CameraTrackerEntry {
    fn default() -> Self {
        Self {
            connection: CameraConnectionStatus::Unknown,
            registration: CameraRegistrationState::Pending,
            lacis_id: None,
            cic: None,
        }
    }
}

/// Tracks camera connection status and detects transitions
pub struct CameraStatusTracker {
    /// Current status of each camera (camera_id -> status)
    statuses: RwLock<HashMap<String, CameraConnectionStatus>>,
    /// Extended camera info (Phase 2)
    extended: RwLock<HashMap<String, CameraTrackerEntry>>,
}

impl CameraStatusTracker {
    /// Create new tracker
    pub fn new() -> Self {
        Self {
            statuses: RwLock::new(HashMap::new()),
            extended: RwLock::new(HashMap::new()),
        }
    }

    /// Update camera status and return transition event if any
    ///
    /// Returns:
    /// - `Some(Lost)` if camera transitioned from Online to Offline
    /// - `Some(Recovered)` if camera transitioned from Offline to Online
    /// - `Some(Lost)` if camera's first status is Offline (initial lost)
    /// - `None` if no transition occurred
    pub async fn update_status(
        &self,
        camera_id: &str,
        is_online: bool,
    ) -> Option<CameraStatusEvent> {
        let mut statuses = self.statuses.write().await;
        let prev = statuses
            .get(camera_id)
            .cloned()
            .unwrap_or(CameraConnectionStatus::Unknown);

        let new_status = if is_online {
            CameraConnectionStatus::Online
        } else {
            CameraConnectionStatus::Offline
        };

        // Update status
        statuses.insert(camera_id.to_string(), new_status.clone());

        // Determine transition event
        match (&prev, &new_status) {
            // Online -> Offline = Lost
            (CameraConnectionStatus::Online, CameraConnectionStatus::Offline) => {
                tracing::warn!(
                    camera_id = %camera_id,
                    "Camera connection lost"
                );
                Some(CameraStatusEvent::Lost)
            }
            // Offline -> Online = Recovered
            (CameraConnectionStatus::Offline, CameraConnectionStatus::Online) => {
                tracing::info!(
                    camera_id = %camera_id,
                    "Camera connection recovered"
                );
                Some(CameraStatusEvent::Recovered)
            }
            // Unknown -> Offline = Initial Lost (first poll failed)
            (CameraConnectionStatus::Unknown, CameraConnectionStatus::Offline) => {
                tracing::warn!(
                    camera_id = %camera_id,
                    "Camera initial poll failed - marking as lost"
                );
                Some(CameraStatusEvent::Lost)
            }
            // Unknown -> Online = No event (first successful poll)
            (CameraConnectionStatus::Unknown, CameraConnectionStatus::Online) => None,
            // Same status = No event
            _ => None,
        }
    }

    /// Get current status for a camera
    pub async fn get_status(&self, camera_id: &str) -> CameraConnectionStatus {
        self.statuses
            .read()
            .await
            .get(camera_id)
            .cloned()
            .unwrap_or(CameraConnectionStatus::Unknown)
    }

    /// Get all offline cameras
    pub async fn get_offline_cameras(&self) -> Vec<String> {
        self.statuses
            .read()
            .await
            .iter()
            .filter(|(_, status)| **status == CameraConnectionStatus::Offline)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Remove camera from tracking (e.g., when deleted)
    pub async fn remove(&self, camera_id: &str) {
        self.statuses.write().await.remove(camera_id);
    }

    /// Clear all status (e.g., on restart)
    pub async fn clear(&self) {
        self.statuses.write().await.clear();
        self.extended.write().await.clear();
    }

    // ========================================
    // Phase 2: CameraRegistry Integration
    // ========================================

    /// Update camera registration state
    pub async fn update_registration(
        &self,
        camera_id: &str,
        state: CameraRegistrationState,
        lacis_id: Option<String>,
        cic: Option<String>,
    ) {
        let mut extended = self.extended.write().await;
        let entry = extended.entry(camera_id.to_string()).or_default();
        entry.registration = state;
        if lacis_id.is_some() {
            entry.lacis_id = lacis_id;
        }
        if cic.is_some() {
            entry.cic = cic;
        }

        tracing::debug!(
            camera_id = %camera_id,
            registration = ?entry.registration,
            lacis_id = ?entry.lacis_id,
            "Camera registration state updated"
        );
    }

    /// Get full camera status (connection + registration)
    pub async fn get_full_status(&self, camera_id: &str) -> CameraFullStatus {
        let connection = self.get_status(camera_id).await;
        let extended = self.extended.read().await;
        let entry = extended.get(camera_id);

        CameraFullStatus {
            camera_id: camera_id.to_string(),
            connection,
            registration: entry
                .map(|e| e.registration.clone())
                .unwrap_or_default(),
            lacis_id: entry.and_then(|e| e.lacis_id.clone()),
            cic: entry.and_then(|e| e.cic.clone()),
        }
    }

    /// Get all cameras with their full status
    pub async fn get_all_full_status(&self) -> Vec<CameraFullStatus> {
        let statuses = self.statuses.read().await;
        let extended = self.extended.read().await;

        // Collect all unique camera IDs
        let mut camera_ids: Vec<String> = statuses.keys().cloned().collect();
        for key in extended.keys() {
            if !camera_ids.contains(key) {
                camera_ids.push(key.clone());
            }
        }

        camera_ids
            .into_iter()
            .map(|camera_id| {
                let connection = statuses
                    .get(&camera_id)
                    .cloned()
                    .unwrap_or(CameraConnectionStatus::Unknown);
                let entry = extended.get(&camera_id);

                CameraFullStatus {
                    camera_id,
                    connection,
                    registration: entry
                        .map(|e| e.registration.clone())
                        .unwrap_or_default(),
                    lacis_id: entry.and_then(|e| e.lacis_id.clone()),
                    cic: entry.and_then(|e| e.cic.clone()),
                }
            })
            .collect()
    }

    /// Get registered cameras that are online
    pub async fn get_online_registered(&self) -> Vec<CameraFullStatus> {
        self.get_all_full_status()
            .await
            .into_iter()
            .filter(|s| {
                s.connection == CameraConnectionStatus::Online
                    && s.registration == CameraRegistrationState::Registered
            })
            .collect()
    }

    /// Get pending registration cameras
    pub async fn get_pending_registration(&self) -> Vec<String> {
        self.extended
            .read()
            .await
            .iter()
            .filter(|(_, entry)| entry.registration == CameraRegistrationState::Pending)
            .map(|(id, _)| id.clone())
            .collect()
    }
}

impl Default for CameraStatusTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initial_online_no_event() {
        let tracker = CameraStatusTracker::new();
        let event = tracker.update_status("cam1", true).await;
        assert!(event.is_none());
    }

    #[tokio::test]
    async fn test_initial_offline_triggers_lost() {
        let tracker = CameraStatusTracker::new();
        let event = tracker.update_status("cam1", false).await;
        assert_eq!(event, Some(CameraStatusEvent::Lost));
    }

    #[tokio::test]
    async fn test_online_to_offline_triggers_lost() {
        let tracker = CameraStatusTracker::new();
        tracker.update_status("cam1", true).await;
        let event = tracker.update_status("cam1", false).await;
        assert_eq!(event, Some(CameraStatusEvent::Lost));
    }

    #[tokio::test]
    async fn test_offline_to_online_triggers_recovered() {
        let tracker = CameraStatusTracker::new();
        tracker.update_status("cam1", false).await;
        let event = tracker.update_status("cam1", true).await;
        assert_eq!(event, Some(CameraStatusEvent::Recovered));
    }

    #[tokio::test]
    async fn test_offline_to_offline_no_event() {
        let tracker = CameraStatusTracker::new();
        tracker.update_status("cam1", false).await;
        let event = tracker.update_status("cam1", false).await;
        assert!(event.is_none());
    }

    #[tokio::test]
    async fn test_online_to_online_no_event() {
        let tracker = CameraStatusTracker::new();
        tracker.update_status("cam1", true).await;
        let event = tracker.update_status("cam1", true).await;
        assert!(event.is_none());
    }
}
