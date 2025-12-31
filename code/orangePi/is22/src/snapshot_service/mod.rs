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
}

impl SnapshotService {
    /// Create new SnapshotService
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
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
                        "HTTP snapshot failed, trying RTSP"
                    );
                }
            }
        }

        // Try RTSP snapshot (via go2rtc)
        if let Some(ref rtsp_url) = camera.rtsp_sub {
            // For RTSP, we use go2rtc's frame API
            // This assumes go2rtc is configured with the camera
            let go2rtc_url = format!(
                "http://localhost:1984/api/frame.jpeg?src={}",
                camera.camera_id
            );
            return self.capture_http(&go2rtc_url).await;
        }

        Err(Error::Internal(format!(
            "No snapshot source available for camera {}",
            camera.camera_id
        )))
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

impl Default for SnapshotService {
    fn default() -> Self {
        Self::new()
    }
}
