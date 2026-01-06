//! Camera Status Tracker
//!
//! Tracks camera connection status changes to detect lost/recovered events.
//! Only transitions are logged to avoid spamming AI Event Log.

use std::collections::HashMap;
use tokio::sync::RwLock;

/// Camera connection status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraConnectionStatus {
    /// Initial state (never polled)
    Unknown,
    /// Camera is online and responding
    Online,
    /// Camera is offline or not responding
    Offline,
}

/// Camera status transition event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraStatusEvent {
    /// Camera went from Online to Offline
    Lost,
    /// Camera went from Offline to Online
    Recovered,
}

/// Tracks camera connection status and detects transitions
pub struct CameraStatusTracker {
    /// Current status of each camera (camera_id -> status)
    statuses: RwLock<HashMap<String, CameraConnectionStatus>>,
}

impl CameraStatusTracker {
    /// Create new tracker
    pub fn new() -> Self {
        Self {
            statuses: RwLock::new(HashMap::new()),
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
