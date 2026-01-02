//! SnapshotService - Image Capture from Cameras via RTSP
//!
//! ## Responsibilities
//!
//! - RTSP snapshot capture using ffmpeg (no go2rtc dependency)
//! - Image caching for display
//! - Previous image retention for IS21 diff detection

use crate::config_store::Camera;
use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::process::Command;

/// SnapshotService instance
pub struct SnapshotService {
    /// HTTP client for fallback snapshot URLs
    client: reqwest::Client,
    /// Directory for snapshot cache
    snapshot_dir: PathBuf,
    /// Directory for temporary files (prev image)
    temp_dir: PathBuf,
    /// ffmpeg timeout in seconds
    ffmpeg_timeout: u64,
}

impl SnapshotService {
    /// Create new SnapshotService
    ///
    /// # Arguments
    /// * `snapshot_dir` - Directory to store latest snapshots (e.g., /var/lib/is22/snapshots)
    /// * `temp_dir` - Directory for temporary files (e.g., /var/lib/is22/temp)
    pub async fn new(snapshot_dir: PathBuf, temp_dir: PathBuf) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        // Create directories if they don't exist
        fs::create_dir_all(&snapshot_dir).await?;
        fs::create_dir_all(&temp_dir).await?;

        Ok(Self {
            client,
            snapshot_dir,
            temp_dir,
            ffmpeg_timeout: 10,
        })
    }

    /// Capture snapshot from camera using RTSP (ffmpeg)
    ///
    /// Returns the captured JPEG bytes
    pub async fn capture(&self, camera: &Camera) -> Result<Vec<u8>> {
        // Determine RTSP URL (prefer main stream)
        let rtsp_url = camera.rtsp_main.as_ref().or(camera.rtsp_sub.as_ref());

        if let Some(rtsp_url) = rtsp_url {
            match self.capture_rtsp(rtsp_url).await {
                Ok(data) => return Ok(data),
                Err(e) => {
                    tracing::warn!(
                        camera_id = %camera.camera_id,
                        error = %e,
                        "RTSP capture failed, trying HTTP snapshot"
                    );
                }
            }
        }

        // Fallback to HTTP snapshot URL if available
        if let Some(ref url) = camera.snapshot_url {
            return self.capture_http(url).await;
        }

        Err(Error::Internal(format!(
            "No RTSP or snapshot URL available for camera {}",
            camera.camera_id
        )))
    }

    /// Capture frame from RTSP stream using ffmpeg
    async fn capture_rtsp(&self, rtsp_url: &str) -> Result<Vec<u8>> {
        // ffmpeg command to capture 1 frame from RTSP and output as JPEG to stdout
        // -rtsp_transport tcp: Use TCP for RTSP (more reliable)
        // -frames:v 1: Capture only 1 frame
        // -f image2pipe -vcodec mjpeg: Output as MJPEG to pipe
        let output = Command::new("ffmpeg")
            .args([
                "-rtsp_transport", "tcp",
                "-i", rtsp_url,
                "-frames:v", "1",
                "-f", "image2pipe",
                "-vcodec", "mjpeg",
                "-loglevel", "error",
                "-y",
                "-",
            ])
            .output()
            .await
            .map_err(|e| Error::Internal(format!("ffmpeg execution failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Internal(format!(
                "ffmpeg failed: {}",
                stderr.trim()
            )));
        }

        if output.stdout.is_empty() {
            return Err(Error::Internal("ffmpeg returned empty output".to_string()));
        }

        Ok(output.stdout)
    }

    /// Capture via HTTP (fallback)
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

    /// Save snapshot to cache directory (latest.jpg)
    ///
    /// Returns the path to the saved file
    pub async fn save_cache(&self, camera_id: &str, data: &[u8]) -> Result<PathBuf> {
        let camera_dir = self.snapshot_dir.join(camera_id);
        fs::create_dir_all(&camera_dir).await?;

        let path = camera_dir.join("latest.jpg");
        fs::write(&path, data).await?;

        tracing::debug!(
            camera_id = %camera_id,
            path = %path.display(),
            size = data.len(),
            "Saved snapshot cache"
        );

        Ok(path)
    }

    /// Get path to cached snapshot
    pub fn get_cache_path(&self, camera_id: &str) -> PathBuf {
        self.snapshot_dir.join(camera_id).join("latest.jpg")
    }

    /// Get previous image for diff detection
    ///
    /// Returns None if no previous image exists
    pub async fn get_prev_image(&self, camera_id: &str) -> Result<Option<Vec<u8>>> {
        let path = self.temp_dir.join(camera_id).join("prev.jpg");

        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read(&path).await?;
        Ok(Some(data))
    }

    /// Save current image as previous for next diff detection
    pub async fn save_as_prev(&self, camera_id: &str, data: &[u8]) -> Result<()> {
        let camera_dir = self.temp_dir.join(camera_id);
        fs::create_dir_all(&camera_dir).await?;

        let path = camera_dir.join("prev.jpg");
        fs::write(&path, data).await?;

        Ok(())
    }

    /// Check if ffmpeg is available
    pub async fn check_ffmpeg() -> Result<String> {
        let output = Command::new("ffmpeg")
            .arg("-version")
            .output()
            .await
            .map_err(|e| Error::Internal(format!("ffmpeg not found: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Internal("ffmpeg version check failed".to_string()));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        // Extract first line (version info)
        let first_line = version.lines().next().unwrap_or("unknown");
        Ok(first_line.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_ffmpeg() {
        let result = SnapshotService::check_ffmpeg().await;
        assert!(result.is_ok(), "ffmpeg should be available");
        println!("ffmpeg version: {}", result.unwrap());
    }
}
