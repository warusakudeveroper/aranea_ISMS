//! AIClient - IS21 Communication Adapter
//!
//! ## Responsibilities
//!
//! - Send inference requests to IS21
//! - Handle response parsing
//! - Connection management

use crate::error::{Error, Result};
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// IS21 AI Client
pub struct AIClient {
    client: reqwest::Client,
    base_url: String,
}

/// Inference request
#[derive(Debug, Clone, Serialize)]
pub struct InferRequest {
    pub camera_id: String,
    pub schema_version: String,
    pub captured_at: String,
    pub hints: Option<serde_json::Value>,
}

/// Inference response
#[derive(Debug, Clone, Deserialize)]
pub struct InferResponse {
    pub ok: bool,
    pub frame_id: Option<String>,
    pub primary_event: Option<String>,
    pub detected: bool,
    pub severity: i32,
    pub tags: Vec<String>,
    pub unknown_flag: bool,
    pub attributes: Option<serde_json::Value>,
    pub processing_ms: i64,
}

impl AIClient {
    /// Create new AI client
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    /// Check IS21 health
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/healthz", self.base_url);
        let resp = self.client.get(&url).send().await?;
        Ok(resp.status().is_success())
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

    /// Send inference request
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
}
