//! SnapshotService - Image Capture from Cameras
//!
//! ## Responsibilities
//!
//! - RTSP snapshot capture
//! - HTTP snapshot fallback
//! - Image caching

use crate::config_store::Camera;
use crate::error::{Error, Result};
use std::time::Duration;

/// SnapshotService instance
pub struct SnapshotService {
    client: reqwest::Client,
    go2rtc_url: String,
}

impl SnapshotService {
    /// Create new SnapshotService with go2rtc URL
    pub fn new(go2rtc_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, go2rtc_url }
    }

    /// Capture snapshot from camera
    pub async fn capture(&self, camera: &Camera) -> Result<Vec<u8>> {
        // Try HTTP snapshot first (if available)
        if let Some(ref url) = camera.snapshot_url {
            match self.capture_http(url).await {
                Ok(data) => return Ok(data),
                Err(e) => {
                    tracing::warn!(
                        camera_id = %camera.camera_id,
                        error = %e,
                        "HTTP snapshot failed, trying go2rtc"
                    );
                }
            }
        }

        // Try go2rtc frame API (works for all cameras registered in go2rtc)
        let go2rtc_snapshot_url = format!(
            "{}/api/frame.jpeg?src={}",
            self.go2rtc_url,
            camera.camera_id
        );
        self.capture_http(&go2rtc_snapshot_url).await
    }

    /// Capture via HTTP
    async fn capture_http(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.client.get(url).send().await?;

        if !resp.status().is_success() {
            return Err(Error::Internal(format!(
                "Snapshot HTTP error: {}",
                resp.status()
            )));
        }

        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }
}

// Note: SnapshotService requires go2rtc_url, so no Default impl
// Use SnapshotService::new(go2rtc_url) instead
