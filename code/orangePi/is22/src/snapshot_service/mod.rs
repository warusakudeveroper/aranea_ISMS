//! SnapshotService - Image Capture from Cameras via RTSP
//!
//! ## Responsibilities
//!
//! - RTSP snapshot capture using ffmpeg (no go2rtc dependency)
//! - go2rtc経由でのスナップショット取得（視聴者がいる場合、RTSP競合回避）
//! - Image caching for display
//! - Previous image retention for IS21 diff detection
//! - RtspManager経由でカメラへの多重アクセスを防止

use crate::config_store::Camera;
use crate::error::{Error, Result};
use crate::rtsp_manager::RtspManager;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::process::Command;

/// go2rtc API base URL (localhost)
const GO2RTC_BASE_URL: &str = "http://localhost:1984";

/// Snapshot source enum for tracking how the image was captured
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotSource {
    /// Captured via go2rtc frame.jpeg API (when active viewers exist)
    Go2rtc,
    /// Captured via ffmpeg RTSP direct connection
    Ffmpeg,
    /// Captured via HTTP snapshot URL
    Http,
}

impl SnapshotSource {
    /// Convert to string for logging/serialization
    pub fn as_str(&self) -> &'static str {
        match self {
            SnapshotSource::Go2rtc => "go2rtc",
            SnapshotSource::Ffmpeg => "ffmpeg",
            SnapshotSource::Http => "http",
        }
    }
}

/// Result of snapshot capture, including the image data and source
pub struct CaptureResult {
    /// JPEG image data
    pub data: Vec<u8>,
    /// Source of the capture (go2rtc, ffmpeg, http)
    pub source: SnapshotSource,
}

/// SnapshotService instance
pub struct SnapshotService {
    /// HTTP client for fallback snapshot URLs
    client: reqwest::Client,
    /// Directory for snapshot cache
    snapshot_dir: PathBuf,
    /// Directory for temporary files (prev image)
    temp_dir: PathBuf,
    /// ffmpeg timeout for main stream in seconds
    ffmpeg_timeout_main: u64,
    /// ffmpeg timeout for sub stream in seconds (longer for fallback)
    ffmpeg_timeout_sub: u64,
    /// RTSPアクセス制御
    rtsp_manager: Arc<RtspManager>,
}

impl SnapshotService {
    /// Create new SnapshotService
    ///
    /// # Arguments
    /// * `snapshot_dir` - Directory to store latest snapshots (e.g., /var/lib/is22/snapshots)
    /// * `temp_dir` - Directory for temporary files (e.g., /var/lib/is22/temp)
    /// * `rtsp_manager` - RTSPアクセス制御マネージャ
    /// * `timeout_main_sec` - Timeout for main stream in seconds (from global settings)
    /// * `timeout_sub_sec` - Timeout for sub stream in seconds (from global settings)
    pub async fn new(
        snapshot_dir: PathBuf,
        temp_dir: PathBuf,
        rtsp_manager: Arc<RtspManager>,
        timeout_main_sec: u64,
        timeout_sub_sec: u64,
    ) -> Result<Self> {
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
            ffmpeg_timeout_main: timeout_main_sec,
            ffmpeg_timeout_sub: timeout_sub_sec,
            rtsp_manager,
        })
    }

    /// Capture snapshot from camera
    ///
    /// 取得優先順位:
    /// 1. go2rtc経由（視聴者がいる場合 → RTSP競合回避、高速）
    /// 2. ffmpeg経由RTSP（RtspManager経由でアクセス制御）
    /// 3. HTTP snapshot URL（フォールバック）
    ///
    /// Returns the captured JPEG bytes and the source of capture
    pub async fn capture(&self, camera: &Camera) -> Result<CaptureResult> {
        // 1. Try go2rtc first if there are active viewers (avoids RTSP conflict)
        // camera_id is already the stream name in go2rtc (e.g., cam-xxx-xxx)
        match self.try_go2rtc_snapshot(&camera.camera_id).await {
            Ok(Some(data)) => {
                tracing::debug!(
                    camera_id = %camera.camera_id,
                    size = data.len(),
                    source = "go2rtc",
                    "Snapshot captured via go2rtc (active viewers)"
                );
                return Ok(CaptureResult {
                    data,
                    source: SnapshotSource::Go2rtc,
                });
            }
            Ok(None) => {
                // No active viewers, proceed to ffmpeg
            }
            Err(e) => {
                tracing::debug!(
                    camera_id = %camera.camera_id,
                    error = %e,
                    "go2rtc snapshot check failed, falling back to ffmpeg"
                );
            }
        }

        // 2. Try ffmpeg RTSP capture with main→sub fallback
        // Main stream: 10s timeout, Sub stream: 20s timeout (fallback)
        let has_rtsp = camera.rtsp_main.is_some() || camera.rtsp_sub.is_some();

        if has_rtsp {
            // RTSPアクセス前にロック取得（他が使用中なら待機）
            let _lease = self.rtsp_manager.acquire(&camera.camera_id).await
                .map_err(|e| Error::Internal(format!(
                    "RTSP busy for camera {}: {}",
                    camera.camera_id, e
                )))?;

            // 2a. Try main stream first (10s timeout)
            if let Some(ref main_url) = camera.rtsp_main {
                match self.capture_rtsp(main_url, self.ffmpeg_timeout_main).await {
                    Ok(data) => {
                        tracing::debug!(
                            camera_id = %camera.camera_id,
                            size = data.len(),
                            source = "ffmpeg",
                            stream = "main",
                            "Snapshot captured via ffmpeg RTSP (main stream)"
                        );
                        return Ok(CaptureResult {
                            data,
                            source: SnapshotSource::Ffmpeg,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            camera_id = %camera.camera_id,
                            error = %e,
                            stream = "main",
                            "Main stream capture failed, trying sub stream"
                        );
                    }
                }
            }

            // 2b. Fallback to sub stream (20s timeout)
            if let Some(ref sub_url) = camera.rtsp_sub {
                match self.capture_rtsp(sub_url, self.ffmpeg_timeout_sub).await {
                    Ok(data) => {
                        tracing::debug!(
                            camera_id = %camera.camera_id,
                            size = data.len(),
                            source = "ffmpeg",
                            stream = "sub",
                            "Snapshot captured via ffmpeg RTSP (sub stream fallback)"
                        );
                        return Ok(CaptureResult {
                            data,
                            source: SnapshotSource::Ffmpeg,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            camera_id = %camera.camera_id,
                            error = %e,
                            stream = "sub",
                            "Sub stream capture failed, trying HTTP snapshot"
                        );
                    }
                }
            }
            // _leaseはここでDropされ、ロック解放
        }

        // 3. Fallback to HTTP snapshot URL if available (RTSPロック不要)
        if let Some(ref url) = camera.snapshot_url {
            let data = self.capture_http(url).await?;
            tracing::debug!(
                camera_id = %camera.camera_id,
                size = data.len(),
                source = "http",
                "Snapshot captured via HTTP"
            );
            return Ok(CaptureResult {
                data,
                source: SnapshotSource::Http,
            });
        }

        Err(Error::Internal(format!(
            "No RTSP or snapshot URL available for camera {}",
            camera.camera_id
        )))
    }

    /// Try to capture snapshot via go2rtc if there are active viewers
    ///
    /// go2rtcがストリームをアクティブに保持している場合（視聴者がいる場合）、
    /// frame.jpeg APIを使ってスナップショットを取得。
    /// これによりRTSP多重接続を回避し、カメラ側の接続制限エラーを防止。
    ///
    /// 検出方法: producersのrecv > 0 をチェック（MSE WebSocketはconsumersに含まれないため）
    ///
    /// Returns:
    /// - Ok(Some(data)) - 視聴者がいてスナップショット取得成功
    /// - Ok(None) - 視聴者がいない（ffmpegにフォールバックすべき）
    /// - Err - API呼び出しエラー
    async fn try_go2rtc_snapshot(&self, stream_id: &str) -> Result<Option<Vec<u8>>> {
        // Check go2rtc streams API for active viewers
        let streams_url = format!("{}/api/streams", GO2RTC_BASE_URL);
        let resp = match self.client.get(&streams_url).send().await {
            Ok(r) => r,
            Err(e) => {
                return Err(Error::Internal(format!("go2rtc API error: {}", e)));
            }
        };

        if !resp.status().is_success() {
            return Ok(None);
        }

        let streams: serde_json::Value = resp.json().await
            .map_err(|e| Error::Internal(format!("go2rtc JSON parse error: {}", e)))?;

        // Check if this stream exists
        let stream = match streams.get(stream_id) {
            Some(s) => s,
            None => return Ok(None), // Stream not registered
        };

        // Check for active producers with recv > 0 (indicates active RTSP stream)
        // MSE WebSocket viewers do NOT appear in "consumers" array.
        // Instead, when a viewer is watching, go2rtc connects to RTSP source,
        // and the producer's "recv" counter increases as data flows.
        // If recv > 0, go2rtc already has an active RTSP connection,
        // so we should use frame.jpeg API to avoid opening another connection
        // (Tapo cameras only support 1-2 concurrent RTSP connections).
        let has_active_producer = stream
            .get("producers")
            .and_then(|p| p.as_array())
            .map(|producers| {
                producers.iter().any(|prod| {
                    prod.get("recv")
                        .and_then(|r| r.as_u64())
                        .map(|recv| recv > 0)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if !has_active_producer {
            // No active RTSP connection in go2rtc, safe to use ffmpeg
            return Ok(None);
        }

        // Has active producer - use go2rtc frame API
        tracing::debug!(
            stream_id = %stream_id,
            "Using go2rtc for snapshot (active viewers detected)"
        );

        let frame_url = format!("{}/api/frame.jpeg?src={}", GO2RTC_BASE_URL, stream_id);
        let resp = self.client.get(&frame_url).send().await
            .map_err(|e| Error::Internal(format!("go2rtc frame API error: {}", e)))?;

        if !resp.status().is_success() {
            return Err(Error::Internal(format!(
                "go2rtc frame API returned {}",
                resp.status()
            )));
        }

        let bytes = resp.bytes().await
            .map_err(|e| Error::Internal(format!("go2rtc frame read error: {}", e)))?;

        Ok(Some(bytes.to_vec()))
    }

    /// Capture frame from RTSP stream using ffmpeg
    ///
    /// Uses kill_on_drop(true) to ensure proper process cleanup on timeout.
    /// When the timeout fires and the future is cancelled, the Child is dropped,
    /// which automatically sends SIGKILL to the ffmpeg process.
    /// This prevents zombie ffmpeg processes from accumulating when cameras are unresponsive.
    ///
    /// # Arguments
    /// * `rtsp_url` - RTSP URL to capture from
    /// * `timeout_secs` - Timeout in seconds for ffmpeg process
    async fn capture_rtsp(&self, rtsp_url: &str, timeout_secs: u64) -> Result<Vec<u8>> {
        use std::process::Stdio;

        // Spawn ffmpeg process with kill_on_drop enabled
        // This ensures the process is killed when Child is dropped (e.g., on timeout)
        // -rtsp_transport tcp: Use TCP for RTSP (more reliable)
        // -frames:v 1: Capture only 1 frame
        // -f image2pipe -vcodec mjpeg: Output as MJPEG to pipe
        let child = Command::new("ffmpeg")
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| Error::Internal(format!("ffmpeg spawn failed: {}", e)))?;

        // Wait for completion with timeout
        // If timeout fires, the future is cancelled, Child is dropped,
        // and kill_on_drop ensures SIGKILL is sent to the ffmpeg process
        let timeout_duration = Duration::from_secs(timeout_secs);

        match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
            Ok(Ok(output)) => {
                // Process completed within timeout
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
            Ok(Err(e)) => {
                // Process failed to complete (IO error)
                Err(Error::Internal(format!("ffmpeg execution failed: {}", e)))
            }
            Err(_) => {
                // Timeout occurred
                // The future was cancelled, Child was dropped, and kill_on_drop
                // automatically sent SIGKILL to the ffmpeg process
                tracing::warn!(
                    timeout_sec = timeout_secs,
                    rtsp_url = %rtsp_url,
                    "ffmpeg timeout, process killed via kill_on_drop"
                );

                Err(Error::Internal(format!(
                    "ffmpeg timeout ({}s)",
                    timeout_secs
                )))
            }
        }
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
