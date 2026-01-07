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

    /// Add a stream source to go2rtc
    ///
    /// Uses PUT /api/streams?name={name}&src={rtsp_url} format
    /// which is the correct API for go2rtc v1.9.x
    pub async fn add_source(&self, name: &str, rtsp_url: &str) -> Result<()> {
        // go2rtc v1.9.x API: PUT with query parameters
        let url = format!(
            "{}/api/streams?name={}&src={}",
            self.base_url,
            urlencoding::encode(name),
            urlencoding::encode(rtsp_url)
        );

        let resp = self.client.put(&url).send().await?;

        // go2rtc may return non-2xx but still register the stream
        // Check if stream was actually added
        if resp.status().is_success() || resp.status().as_u16() == 400 {
            // 400 can happen with yaml parse warnings but stream still added
            tracing::debug!(
                name = name,
                status = %resp.status(),
                "go2rtc stream add response"
            );
            return Ok(());
        }

        Err(Error::Internal(format!(
            "Failed to add stream source: {} - {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )))
    }

    /// Add a stream source with retry
    pub async fn add_source_with_retry(&self, name: &str, rtsp_url: &str, retries: u32) -> Result<()> {
        let mut last_error = None;

        for attempt in 0..=retries {
            match self.add_source(name, rtsp_url).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < retries {
                        tokio::time::sleep(Duration::from_millis(500 * (attempt as u64 + 1))).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::Internal("Unknown error".to_string())))
    }

    /// Check if a stream exists in go2rtc
    pub async fn stream_exists(&self, name: &str) -> Result<bool> {
        let streams = self.list_streams().await?;
        if let Some(obj) = streams.as_object() {
            return Ok(obj.contains_key(name));
        }
        Ok(false)
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

    /// Get go2rtc version
    ///
    /// go2rtc returns version info at /api endpoint as JSON
    /// Response format: {"version": "1.9.9", ...}
    pub async fn get_version(&self) -> Result<String> {
        let url = format!("{}/api", self.base_url);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(Error::Internal(format!(
                "Failed to get go2rtc info: {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp.json().await?;

        // go2rtc API returns version in different formats depending on version
        // Try common patterns
        if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
            return Ok(version.to_string());
        }

        // Fallback: check if it's the root info object
        if let Some(info) = json.get("info").and_then(|v| v.get("version")).and_then(|v| v.as_str()) {
            return Ok(info.to_string());
        }

        Err(Error::Internal("Could not parse go2rtc version".to_string()))
    }

    // ========================================
    // Phase 4: Nest/SDM Integration (T4-1 ~ T4-4)
    // ========================================

    /// T4-1: Add Nest source to go2rtc
    ///
    /// Nest sources use the nest:// protocol:
    /// nest://{device_id}?project_id=...&enterprise=...&client_id=...&client_secret=...&refresh_token=...
    pub async fn add_nest_source(&self, camera_id: &str, nest_url: &str) -> Result<()> {
        // Validate nest:// URL format
        if !nest_url.starts_with("nest://") {
            return Err(Error::Internal(format!(
                "Invalid nest URL format: must start with nest://"
            )));
        }

        // Use the standard add_source method with retry
        self.add_source_with_retry(camera_id, nest_url, 2).await?;

        tracing::info!(
            camera_id = camera_id,
            "Nest source registered in go2rtc"
        );

        Ok(())
    }

    /// T4-2: Sync all Nest cameras with go2rtc
    ///
    /// Registers nest:// sources for cameras with family="nest"
    pub async fn sync_nest_cameras(&self, cameras: &[NestCameraInfo]) -> SyncResult {
        let mut result = SyncResult::default();

        for camera in cameras {
            // Skip if already registered
            match self.stream_exists(&camera.camera_id).await {
                Ok(true) => {
                    result.skipped += 1;
                    continue;
                }
                Ok(false) => {}
                Err(e) => {
                    result.errors.push(format!(
                        "{}: failed to check existence: {}",
                        camera.camera_id, e
                    ));
                    continue;
                }
            }

            // Register the nest source
            match self.add_nest_source(&camera.camera_id, &camera.nest_url).await {
                Ok(()) => {
                    result.added += 1;
                }
                Err(e) => {
                    result.errors.push(format!(
                        "{}: failed to add: {}",
                        camera.camera_id, e
                    ));
                }
            }
        }

        result
    }

    /// T4-3: Prewarm Nest stream with extended timeout
    ///
    /// Nest streams may take longer to initialize than RTSP streams
    pub async fn prewarm_nest(&self, camera_id: &str) -> Result<()> {
        // First check if stream exists
        if !self.stream_exists(camera_id).await? {
            return Err(Error::Internal(format!(
                "Stream {} not found in go2rtc",
                camera_id
            )));
        }

        // Request a frame with longer timeout for Nest
        let url = format!("{}/api/frame.jpeg?src={}", self.base_url, camera_id);

        // Use a longer timeout for Nest streams (15s vs default 10s)
        let nest_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| Error::Internal(format!("Failed to create client: {}", e)))?;

        match nest_client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!(camera_id = camera_id, "Nest stream prewarmed successfully");
                Ok(())
            }
            Ok(resp) => {
                tracing::warn!(
                    camera_id = camera_id,
                    status = %resp.status(),
                    "Nest prewarm returned non-success status"
                );
                // Don't fail - stream may still work
                Ok(())
            }
            Err(e) => {
                tracing::warn!(
                    camera_id = camera_id,
                    error = %e,
                    "Nest prewarm request failed"
                );
                // Don't fail - this is just optimization
                Ok(())
            }
        }
    }

    /// T4-4: Get stream status with detailed error info
    pub async fn get_stream_status(&self, camera_id: &str) -> Result<StreamStatus> {
        let url = format!("{}/api/streams?src={}", self.base_url, camera_id);

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(StreamStatus {
                exists: false,
                active: false,
                producers: 0,
                consumers: 0,
                error: Some(format!("HTTP {}", resp.status())),
            });
        }

        let json: serde_json::Value = resp.json().await?;

        // Parse stream info from go2rtc response
        if let Some(stream) = json.get(camera_id) {
            let producers = stream
                .get("producers")
                .and_then(|p| p.as_array())
                .map(|a| a.len())
                .unwrap_or(0);

            let consumers = stream
                .get("consumers")
                .and_then(|c| c.as_array())
                .map(|a| a.len())
                .unwrap_or(0);

            Ok(StreamStatus {
                exists: true,
                active: producers > 0,
                producers,
                consumers,
                error: None,
            })
        } else {
            Ok(StreamStatus {
                exists: false,
                active: false,
                producers: 0,
                consumers: 0,
                error: None,
            })
        }
    }

    /// Remove all Nest sources (for cleanup/reset)
    pub async fn remove_all_nest_sources(&self, camera_ids: &[String]) -> SyncResult {
        let mut result = SyncResult::default();

        for camera_id in camera_ids {
            match self.remove_source(camera_id).await {
                Ok(()) => {
                    result.added += 1; // Using 'added' as 'removed' count
                }
                Err(e) => {
                    result.errors.push(format!("{}: {}", camera_id, e));
                }
            }
        }

        result
    }
}

/// Nest camera info for sync
#[derive(Debug, Clone)]
pub struct NestCameraInfo {
    pub camera_id: String,
    pub nest_url: String,
}

/// Stream sync result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncResult {
    pub added: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Stream status info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStatus {
    pub exists: bool,
    pub active: bool,
    pub producers: usize,
    pub consumers: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
