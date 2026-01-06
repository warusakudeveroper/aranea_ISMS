//! AIClient - IS21 Communication Adapter
//!
//! ## Responsibilities
//!
//! - Send inference requests to IS21
//! - Handle response parsing
//! - Connection management
//! - Preset/Context support
//! - Frame diff support

use crate::error::{Error, Result};
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// IS21 AI Client
pub struct AiClient {
    client: reqwest::Client,
    base_url: String,
    timeout: Duration,
}

/// Inference request (extended for AI Event Log)
#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeRequest {
    pub camera_id: String,
    pub captured_at: String,
    pub schema_version: String,

    // Preset info
    pub preset_id: String,
    pub preset_version: String,

    // Optional fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_context: Option<CameraContext>,

    // Frame diff options
    pub enable_frame_diff: bool,
    pub return_bboxes: bool,
}

impl Default for AnalyzeRequest {
    fn default() -> Self {
        Self {
            camera_id: String::new(),
            captured_at: String::new(),
            schema_version: "2025-12-29.1".to_string(),
            preset_id: "balanced".to_string(),
            preset_version: "1.0.0".to_string(),
            output_schema: None,
            camera_context: None,
            enable_frame_diff: true,
            return_bboxes: true,
        }
    }
}

/// Camera context for hints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_objects: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_objects: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub busy_hours: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quiet_hours: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_frame: Option<PreviousFrameInfo>,

    // v1.9 Threshold overrides (hints_json経由でis21に渡す)
    /// YOLO confidence threshold override (0.2-0.8)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conf_override: Option<f32>,

    /// NMS threshold override (0.3-0.6)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nms_threshold: Option<f32>,

    /// PAR threshold override (0.3-0.8)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub par_threshold: Option<f32>,
}

/// Previous frame metadata for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousFrameInfo {
    pub captured_at: String,
    pub person_count: i32,
    pub primary_event: String,
}

/// Bounding box from IS21 detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub label: String,
    pub conf: f32,

    #[serde(default)]
    pub par: Option<serde_json::Value>,

    #[serde(default)]
    pub details: Option<serde_json::Value>,
}

/// Person detail from PAR analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonDetail {
    pub index: i32,

    #[serde(default)]
    pub top_color: Option<ColorInfo>,

    #[serde(default)]
    pub bottom_color: Option<ColorInfo>,

    #[serde(default)]
    pub uniform_like: Option<bool>,

    #[serde(default)]
    pub body_size: Option<String>,

    #[serde(default)]
    pub body_build: Option<String>,

    #[serde(default)]
    pub posture: Option<PostureInfo>,

    #[serde(default)]
    pub height_category: Option<String>,

    #[serde(default)]
    pub meta: Option<serde_json::Value>,
}

/// Color information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorInfo {
    pub color: String,
    pub confidence: f32,
}

/// Posture information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostureInfo {
    #[serde(rename = "type")]
    pub posture_type: String,
    pub confidence: f32,
}

/// Suspicious score information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspiciousInfo {
    pub score: i32,
    pub level: String,

    #[serde(default)]
    pub factors: Vec<String>,
}

/// Frame diff analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameDiff {
    /// Whether frame diff was performed (may be absent if no previous frame)
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub person_changes: Option<PersonChanges>,

    #[serde(default)]
    pub movement_vectors: Option<Vec<MovementVector>>,

    #[serde(default)]
    pub loitering: Option<LoiteringInfo>,

    #[serde(default)]
    pub scene_change: Option<SceneChangeInfo>,

    #[serde(default)]
    pub camera_status: Option<CameraStatus>,
}

/// Person changes between frames
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonChanges {
    pub appeared: i32,
    pub disappeared: i32,
    pub moved: i32,
    pub stationary: i32,
}

/// Movement vector for tracked persons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementVector {
    pub person_index: i32,
    pub dx: f32,
    pub dy: f32,
    pub speed: String,
    pub direction: String,
}

/// Loitering detection info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoiteringInfo {
    pub detected: bool,

    #[serde(default)]
    pub duration_seconds: Option<i32>,

    #[serde(default)]
    pub person_indices: Option<Vec<i32>>,
}

/// Scene change detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneChangeInfo {
    pub significant: bool,
    pub change_ratio: f32,
}

/// Camera status from frame analysis
/// Can be a structured object or a string like "no_reference"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CameraStatus {
    /// Full camera status from frame diff analysis
    Status {
        obscured: bool,
        moved: bool,
        stable: bool,
    },
    /// Simple string status (e.g., "no_reference" when no previous frame)
    Simple(String),
}

impl CameraStatus {
    /// Check if this is "no_reference" status
    pub fn is_no_reference(&self) -> bool {
        matches!(self, CameraStatus::Simple(s) if s == "no_reference")
    }

    /// Get as structured status if available
    pub fn as_status(&self) -> Option<(bool, bool, bool)> {
        match self {
            CameraStatus::Status { obscured, moved, stable } => Some((*obscured, *moved, *stable)),
            CameraStatus::Simple(_) => None,
        }
    }
}

/// Performance metrics from IS21 (Phase 4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceInfo {
    /// Total inference time in milliseconds
    pub inference_ms: i64,

    /// YOLO detection time in milliseconds
    #[serde(default)]
    pub yolo_ms: i64,

    /// PAR (Person Attribute Recognition) time in milliseconds
    #[serde(default)]
    pub par_ms: i64,
}

/// Preset applied info from IS21 (Phase 4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetAppliedInfo {
    /// LacisID used for preset lookup
    #[serde(default)]
    pub lacis_id: Option<String>,

    /// Preset ID that was applied
    pub preset_id: String,

    /// Preset version that was applied
    pub preset_version: String,

    /// Whether preset was loaded from cache
    #[serde(default)]
    pub from_cache: bool,
}

/// Inference response (matches IS21 /v1/analyze response - full schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeResponse {
    pub schema_version: String,
    pub camera_id: String,
    pub captured_at: String,

    // Preset echo-back
    #[serde(default)]
    pub preset_id: Option<String>,

    #[serde(default)]
    pub preset_version: Option<String>,

    #[serde(default)]
    pub output_schema: Option<String>,

    // Analysis status
    pub analyzed: bool,
    pub detected: bool,

    // Primary detection
    pub primary_event: String,

    #[serde(default)]
    pub tags: Vec<String>,

    pub confidence: f32,
    pub severity: i32,

    #[serde(default)]
    pub unknown_flag: bool,

    #[serde(default)]
    pub count_hint: i32,

    // Detailed results
    #[serde(default)]
    pub bboxes: Vec<BBox>,

    #[serde(default)]
    pub person_details: Option<Vec<PersonDetail>>,

    #[serde(default)]
    pub suspicious: Option<SuspiciousInfo>,

    #[serde(default)]
    pub frame_diff: Option<FrameDiff>,

    // Phase 4: Performance metrics from IS21
    #[serde(default)]
    pub performance: Option<PerformanceInfo>,

    // Phase 4: Preset applied info from IS21
    #[serde(default)]
    pub preset_applied: Option<PresetAppliedInfo>,
}

impl AiClient {
    /// Create new AI client
    pub fn new(base_url: String) -> Self {
        Self::with_timeout(base_url, Duration::from_secs(30))
    }

    /// Create new AI client with custom timeout
    pub fn with_timeout(base_url: String, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url, timeout }
    }

    /// Check IS21 health
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/healthz", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Get IS21 capabilities
    pub async fn get_capabilities(&self) -> Result<serde_json::Value> {
        let url = format!("{}/v1/capabilities", self.base_url);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(Error::Internal(format!(
                "IS21 capabilities failed: {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }

    /// Send inference request with optional previous frame for diff analysis
    pub async fn analyze(
        &self,
        current_image: Vec<u8>,
        prev_image: Option<Vec<u8>>,
        request: AnalyzeRequest,
    ) -> Result<AnalyzeResponse> {
        let url = format!("{}/v1/analyze", self.base_url);

        // Build multipart form
        let mut form = Form::new()
            .part(
                "infer_image",
                Part::bytes(current_image)
                    .file_name("snapshot.jpg")
                    .mime_str("image/jpeg")?,
            )
            .text("camera_id", request.camera_id.clone())
            .text("captured_at", request.captured_at.clone())
            .text("schema_version", request.schema_version.clone())
            .text("preset_id", request.preset_id.clone())
            .text("preset_version", request.preset_version.clone())
            .text("return_bboxes", request.return_bboxes.to_string())
            .text("enable_frame_diff", request.enable_frame_diff.to_string());

        // Add output_schema if specified
        if let Some(ref output_schema) = request.output_schema {
            form = form.text("output_schema", output_schema.clone());
        }

        // Add camera_context as hints_json
        if let Some(ref context) = request.camera_context {
            let hints_json = serde_json::to_string(context)
                .map_err(|e| Error::Internal(format!("Failed to serialize camera_context: {}", e)))?;
            form = form.text("hints_json", hints_json);
        }

        // Add previous image for frame diff
        if let Some(prev_data) = prev_image {
            form = form.part(
                "prev_image",
                Part::bytes(prev_data)
                    .file_name("prev_snapshot.jpg")
                    .mime_str("image/jpeg")?,
            );
        }

        // Send request with retry
        let resp = self.send_with_retry(&url, form, 3).await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Internal(format!(
                "IS21 inference failed: {} - {}",
                status, body
            )));
        }

        let result: AnalyzeResponse = resp.json().await?;
        Ok(result)
    }

    /// Send request with retry logic
    async fn send_with_retry(
        &self,
        url: &str,
        form: Form,
        max_retries: u32,
    ) -> Result<reqwest::Response> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            if attempt > 0 {
                // Exponential backoff
                let delay = Duration::from_millis(100 * 2u64.pow(attempt));
                tokio::time::sleep(delay).await;
            }

            // Note: Form cannot be cloned, so we need to rebuild it on retry
            // For now, we only try once with the form
            // In production, consider using bytes and rebuilding the form
            match self.client.post(url).multipart(form).send().await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    last_error = Some(e);
                    // Can't retry since form is consumed
                    break;
                }
            }
        }

        Err(last_error.map(Error::from).unwrap_or_else(|| {
            Error::Internal("Request failed after retries".to_string())
        }))
    }

    /// Get current schema
    pub async fn get_schema(&self) -> Result<serde_json::Value> {
        let url = format!("{}/v1/schema", self.base_url);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(Error::Internal(format!(
                "IS21 schema fetch failed: {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }

    /// Get base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get timeout
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

// ============================================
// Legacy compatibility (deprecated, use new API)
// ============================================

/// Legacy inference request (for backward compatibility)
/// Migrate to AnalyzeRequest for full functionality
#[derive(Debug, Clone, Serialize)]
pub struct InferRequest {
    pub camera_id: String,
    pub schema_version: String,
    pub captured_at: String,
    pub hints: Option<serde_json::Value>,
}

/// Legacy inference response (for backward compatibility)
/// Migrate to AnalyzeResponse for full functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferResponse {
    pub schema_version: String,
    pub camera_id: String,
    pub captured_at: String,
    pub analyzed: bool,
    pub detected: bool,
    pub primary_event: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub confidence: f32,
    pub severity: i32,
    #[serde(default)]
    pub unknown_flag: bool,
    #[serde(default)]
    pub count_hint: i32,
    #[serde(default)]
    pub bboxes: Vec<BBox>,
}

impl AiClient {
    /// Legacy infer method for backward compatibility
    /// Migrate to analyze() for full functionality including prev_image and presets
    pub async fn infer(&self, image_data: Vec<u8>, request: InferRequest) -> Result<InferResponse> {
        let url = format!("{}/v1/analyze", self.base_url);

        let mut form = Form::new()
            .part(
                "infer_image",
                Part::bytes(image_data)
                    .file_name("snapshot.jpg")
                    .mime_str("image/jpeg")?,
            )
            .text("camera_id", request.camera_id)
            .text("schema_version", request.schema_version)
            .text("captured_at", request.captured_at);

        if let Some(hints) = request.hints {
            form = form.text("hints_json", hints.to_string());
        }

        let resp = self.client.post(&url).multipart(form).send().await?;

        if !resp.status().is_success() {
            return Err(Error::Internal(format!(
                "IS21 inference failed: {}",
                resp.status()
            )));
        }

        let result: InferResponse = resp.json().await?;
        Ok(result)
    }
}

// Keep old AIClient name for backward compatibility
pub type AIClient = AiClient;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_request_default() {
        let req = AnalyzeRequest::default();
        assert_eq!(req.preset_id, "balanced");
        assert_eq!(req.preset_version, "1.0.0");
        assert!(req.enable_frame_diff);
        assert!(req.return_bboxes);
    }

    #[test]
    fn test_camera_context_serialization() {
        let ctx = CameraContext {
            location_type: Some("parking".to_string()),
            distance: Some("far".to_string()),
            expected_objects: Some(vec!["human".to_string(), "vehicle".to_string()]),
            excluded_objects: None,
            busy_hours: None,
            quiet_hours: None,
            previous_frame: None,
            conf_override: Some(0.45),
            nms_threshold: None,
            par_threshold: None,
        };

        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("parking"));
        assert!(json.contains("human"));
        assert!(json.contains("conf_override"));
        assert!(json.contains("0.45"));
    }
}
