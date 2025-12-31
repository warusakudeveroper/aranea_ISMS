//! StreamGateway - go2rtc Integration
//!
//! ## Responsibilities
//!
//! - go2rtc API adapter
//! - Stream URL management
//! - Prewarm functionality

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Stream info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub camera_id: String,
    pub webrtc_url: String,
    pub hls_url: String,
    pub snapshot_url: String,
}

/// StreamGateway instance
pub struct StreamGateway {
    client: reqwest::Client,
    base_url: String,
}

impl StreamGateway {
    /// Create new StreamGateway
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    /// Get stream URLs for a camera
    pub fn get_stream_urls(&self, camera_id: &str) -> StreamInfo {
        StreamInfo {
            camera_id: camera_id.to_string(),
            webrtc_url: format!("{}/api/webrtc?src={}", self.base_url, camera_id),
            hls_url: format!("{}/api/stream.m3u8?src={}", self.base_url, camera_id),
            snapshot_url: format!("{}/api/frame.jpeg?src={}", self.base_url, camera_id),
        }
    }

    /// Add a stream source
    pub async fn add_source(&self, name: &str, rtsp_url: &str) -> Result<()> {
        let url = format!("{}/api/streams", self.base_url);

        let body = serde_json::json!({
            "name": name,
            "source": rtsp_url
        });

        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            return Err(Error::Internal(format!(
                "Failed to add stream source: {}",
                resp.status()
            )));
        }

        Ok(())
    }

    /// Remove a stream source
    pub async fn remove_source(&self, name: &str) -> Result<()> {
        let url = format!("{}/api/streams?name={}", self.base_url, name);

        let resp = self.client.delete(&url).send().await?;

        if !resp.status().is_success() {
            return Err(Error::Internal(format!(
                "Failed to remove stream source: {}",
                resp.status()
            )));
        }

        Ok(())
    }

    /// Get all streams
    pub async fn list_streams(&self) -> Result<serde_json::Value> {
        let url = format!("{}/api/streams", self.base_url);

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(Error::Internal(format!(
                "Failed to list streams: {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }

    /// Prewarm a stream (prepare for fast switching)
    pub async fn prewarm(&self, camera_id: &str) -> Result<()> {
        // Request a snapshot to wake up the stream
        let url = format!("{}/api/frame.jpeg?src={}", self.base_url, camera_id);

        let _ = self.client.get(&url).send().await;

        tracing::debug!(camera_id = camera_id, "Stream prewarmed");

        Ok(())
    }

    /// Check go2rtc health
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api", self.base_url);
        let resp = self.client.get(&url).send().await?;
        Ok(resp.status().is_success())
    }
}
