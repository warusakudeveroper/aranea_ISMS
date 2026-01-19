//! Access Absorber type definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Camera family for access control
/// Separate from CameraFamily enum in ipcam_scan to allow finer-grained control
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum AccessFamily {
    TapoPtz,
    TapoFixed,
    Vigi,
    Nvt,
    Hikvision,
    Dahua,
    Axis,
    Unknown,
}

impl Default for AccessFamily {
    fn default() -> Self {
        Self::Unknown
    }
}

impl AccessFamily {
    /// Get family string for database lookup
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TapoPtz => "tapo_ptz",
            Self::TapoFixed => "tapo_fixed",
            Self::Vigi => "vigi",
            Self::Nvt => "nvt",
            Self::Hikvision => "hikvision",
            Self::Dahua => "dahua",
            Self::Axis => "axis",
            Self::Unknown => "unknown",
        }
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::TapoPtz => "TP-Link Tapo (PTZ)",
            Self::TapoFixed => "TP-Link Tapo (固定)",
            Self::Vigi => "TP-Link VIGI",
            Self::Nvt => "NVT / JOOAN",
            Self::Hikvision => "Hikvision",
            Self::Dahua => "Dahua",
            Self::Axis => "Axis",
            Self::Unknown => "不明",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "tapo_ptz" => Self::TapoPtz,
            "tapo_fixed" => Self::TapoFixed,
            "vigi" => Self::Vigi,
            "nvt" => Self::Nvt,
            "hikvision" => Self::Hikvision,
            "dahua" => Self::Dahua,
            "axis" => Self::Axis,
            _ => Self::Unknown,
        }
    }

    /// Classify Tapo model into PTZ or Fixed
    pub fn classify_tapo_model(model: &str) -> Self {
        let model_upper = model.to_uppercase();
        // PTZ models: C2xx, C5xx
        if model_upper.starts_with("C2") || model_upper.starts_with("C5") {
            Self::TapoPtz
        } else if model_upper.starts_with("C1") {
            // Fixed models: C1xx
            Self::TapoFixed
        } else {
            // Default to PTZ (safer, more restrictive)
            Self::TapoPtz
        }
    }

    /// Determine access_family from ONVIF manufacturer and model strings
    ///
    /// This is the correct way to identify camera type - using ONVIF device info
    /// instead of OUI (which only identifies the network chip manufacturer).
    ///
    /// # Arguments
    /// * `manufacturer` - ONVIF Manufacturer string (e.g., "TP-Link", "HIKVISION")
    /// * `model` - ONVIF Model string (e.g., "VIGI C340", "Tapo C200", "DS-2CD2143G2-I")
    ///
    /// # Returns
    /// AccessFamily enum based on manufacturer/model analysis
    pub fn from_onvif_info(manufacturer: Option<&str>, model: Option<&str>) -> Self {
        let mfr = manufacturer.unwrap_or("").to_uppercase();
        let mdl = model.unwrap_or("").to_uppercase();

        // TP-Link products: distinguish VIGI vs Tapo by model string
        if mfr.contains("TP-LINK") || mfr.contains("TPLINK") || mfr.contains("TP LINK") {
            // VIGI series: professional line (max_concurrent=3)
            if mdl.contains("VIGI") {
                return Self::Vigi;
            }
            // Tapo series: consumer line (max_concurrent=1)
            if mdl.contains("TAPO") {
                return Self::classify_tapo_model(&mdl);
            }
            // Model starts with VIGI pattern (e.g., "C340" which is VIGI)
            if mdl.starts_with("C3") || mdl.starts_with("C4") || mdl.starts_with("C5") {
                // C3xx, C4xx, C5xx are typically VIGI models
                // But C5xx could also be Tapo PTZ... need more context
                // Check if it looks like a VIGI model number pattern
                if mdl.len() >= 4 && mdl.chars().nth(1).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    // Could be either VIGI C340 or Tapo C500
                    // Without "VIGI" or "TAPO" in the string, default based on model range
                    if mdl.starts_with("C3") || mdl.starts_with("C4") {
                        return Self::Vigi; // C3xx, C4xx are VIGI
                    }
                }
            }
            // Tapo model patterns: C1xx, C2xx (PTZ), C100, C200, etc.
            if mdl.starts_with("C1") {
                return Self::TapoFixed;
            }
            if mdl.starts_with("C2") {
                return Self::TapoPtz;
            }
            // Unknown TP-Link - default to more restrictive Tapo PTZ
            return Self::TapoPtz;
        }

        // Hikvision
        if mfr.contains("HIKVISION") || mfr.contains("HIK") || mdl.starts_with("DS-") {
            return Self::Hikvision;
        }

        // Dahua
        if mfr.contains("DAHUA") || mdl.starts_with("DH-") || mdl.starts_with("IPC-") {
            return Self::Dahua;
        }

        // Axis
        if mfr.contains("AXIS") {
            return Self::Axis;
        }

        // NVT / JOOAN
        if mfr.contains("NVT") || mfr.contains("JOOAN") {
            return Self::Nvt;
        }

        // Unknown manufacturer - return Unknown (most restrictive limits will apply)
        Self::Unknown
    }
}

/// Stream purpose for priority control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum StreamPurpose {
    ClickModal,
    SuggestPlay,
    Polling,
    Snapshot,
    HealthCheck,
}

impl StreamPurpose {
    /// Get priority (lower = higher priority)
    pub fn priority(&self) -> u8 {
        match self {
            Self::ClickModal => 1,
            Self::Snapshot => 2,
            Self::Polling => 3,
            Self::SuggestPlay => 4,
            Self::HealthCheck => 5,
        }
    }

    /// Japanese display name
    pub fn to_japanese(&self) -> &'static str {
        match self {
            Self::ClickModal => "モーダル再生",
            Self::SuggestPlay => "サジェスト再生",
            Self::Polling => "ポーリング",
            Self::Snapshot => "スナップショット取得",
            Self::HealthCheck => "ヘルスチェック",
        }
    }

    /// Check if this purpose can preempt another
    pub fn can_preempt(&self, other: &Self) -> bool {
        // ClickModal can preempt anything except ClickModal
        if matches!(self, Self::ClickModal) {
            return !matches!(other, Self::ClickModal);
        }
        // Snapshot can preempt Polling/HealthCheck
        if matches!(self, Self::Snapshot) {
            return matches!(other, Self::Polling | Self::HealthCheck);
        }
        // SuggestPlay can preempt HealthCheck
        if matches!(self, Self::SuggestPlay) {
            return matches!(other, Self::HealthCheck);
        }
        false
    }
}

/// Stream type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum StreamType {
    Main,
    Sub,
}

impl Default for StreamType {
    fn default() -> Self {
        Self::Main
    }
}

/// Connection limits for a camera family
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionLimits {
    pub family: String,
    pub display_name: String,
    pub max_concurrent_streams: u8,
    pub min_reconnect_interval_ms: u32,
    pub require_exclusive_lock: bool,
    pub connection_timeout_ms: u32,
    pub model_pattern: Option<String>,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            family: "unknown".to_string(),
            display_name: "不明".to_string(),
            max_concurrent_streams: 1,
            min_reconnect_interval_ms: 1000,
            require_exclusive_lock: true,
            connection_timeout_ms: 10000,
            model_pattern: None,
        }
    }
}

/// Active stream session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveStream {
    pub session_id: String,
    pub camera_id: String,
    pub stream_type: StreamType,
    pub purpose: StreamPurpose,
    pub client_id: String,
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl ActiveStream {
    /// Get duration in seconds
    pub fn duration_sec(&self) -> i64 {
        (Utc::now() - self.started_at).num_seconds()
    }
}

/// Stream token returned when acquiring a stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamToken {
    pub session_id: String,
    pub camera_id: String,
    pub stream_type: StreamType,
    pub purpose: StreamPurpose,
    pub client_id: String,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Camera connection state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConnectionState {
    pub camera_id: String,
    pub family: AccessFamily,
    pub limits: ConnectionLimits,
    pub active_streams: Vec<ActiveStream>,
    pub last_disconnect_at: Option<DateTime<Utc>>,
    pub available_slots: u8,
    pub can_connect: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_available_at: Option<DateTime<Utc>>,
}

/// Access absorber error
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "error_type", rename_all = "snake_case")]
pub enum AbsorberError {
    /// Concurrent stream limit reached
    ConcurrentLimitReached {
        camera_id: String,
        family: String,
        max_streams: u8,
        current_count: u8,
        current_purposes: Vec<String>,
    },
    /// Reconnection interval not met
    ReconnectIntervalNotMet {
        camera_id: String,
        family: String,
        required_interval_ms: u32,
        remaining_ms: u32,
    },
    /// Exclusive lock failed
    ExclusiveLockFailed {
        camera_id: String,
        held_by: String,
        held_purpose: String,
    },
    /// Connection timeout
    ConnectionTimeout {
        camera_id: String,
        timeout_ms: u32,
    },
    /// Internal error
    Internal {
        message: String,
    },
}

impl AbsorberError {
    /// Generate user-friendly message
    pub fn to_user_message(&self) -> UserMessage {
        match self {
            Self::ConcurrentLimitReached {
                max_streams,
                current_purposes,
                ..
            } => {
                let purpose_text = current_purposes
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("他のプロセス");

                UserMessage {
                    title: "ストリーミング制限".to_string(),
                    message: format!(
                        "このカメラは並列再生数が{}です。現在{}で再生中のためストリーミングできません。",
                        max_streams, purpose_text
                    ),
                    severity: Severity::Warning,
                    show_duration_sec: None,
                    action_hint: Some("しばらく待ってから再度お試しください".to_string()),
                }
            }
            Self::ReconnectIntervalNotMet {
                family,
                required_interval_ms,
                remaining_ms,
                ..
            } => UserMessage {
                title: "接続間隔制限".to_string(),
                message: format!(
                    "{}カメラは再接続に{}秒の間隔が必要です。{}秒後に再試行してください。",
                    family,
                    required_interval_ms / 1000,
                    (remaining_ms + 999) / 1000
                ),
                severity: Severity::Info,
                show_duration_sec: Some(*remaining_ms / 1000 + 1),
                action_hint: None,
            },
            Self::ExclusiveLockFailed {
                held_purpose, ..
            } => UserMessage {
                title: "排他制御".to_string(),
                message: format!(
                    "このカメラは現在{}で使用中です。終了をお待ちください。",
                    held_purpose
                ),
                severity: Severity::Warning,
                show_duration_sec: None,
                action_hint: Some("使用中の処理が終了するまでお待ちください".to_string()),
            },
            Self::ConnectionTimeout { timeout_ms, .. } => UserMessage {
                title: "接続タイムアウト".to_string(),
                message: format!("カメラへの接続が{}秒以内に完了しませんでした。", timeout_ms / 1000),
                severity: Severity::Error,
                show_duration_sec: Some(5),
                action_hint: Some("ネットワーク接続を確認してください".to_string()),
            },
            Self::Internal { message } => UserMessage {
                title: "内部エラー".to_string(),
                message: message.clone(),
                severity: Severity::Error,
                show_duration_sec: Some(5),
                action_hint: None,
            },
        }
    }
}

/// Message severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// User-facing message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub title: String,
    pub message: String,
    pub severity: Severity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_duration_sec: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_hint: Option<String>,
}

/// Connection event type for logging
#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum ConnectionEventType {
    ConnectSuccess,
    ConnectBlockedConcurrent,
    ConnectBlockedInterval,
    ConnectTimeout,
    ConnectPreempted,
    DisconnectNormal,
    DisconnectPreempted,
    DisconnectTimeout,
    DisconnectError,
}

/// Stream status API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStatusResponse {
    pub camera_id: String,
    pub family: String,
    pub limits: ConnectionLimits,
    pub current_state: CurrentStreamState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_message: Option<UserMessage>,
}

/// Current stream state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentStreamState {
    pub active_streams: Vec<ActiveStreamInfo>,
    pub available_slots: u8,
    pub can_connect: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_available_at: Option<DateTime<Utc>>,
}

/// Active stream info for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveStreamInfo {
    pub stream_id: String,
    pub stream_type: StreamType,
    pub purpose: StreamPurpose,
    pub client_id: String,
    pub started_at: DateTime<Utc>,
    pub duration_sec: i64,
}

/// Acquire stream request
#[derive(Debug, Clone, Deserialize)]
pub struct AcquireStreamRequest {
    pub camera_id: String,
    pub purpose: StreamPurpose,
    pub client_id: String,
    #[serde(default)]
    pub stream_type: StreamType,
    /// Allow preemption of lower priority streams
    #[serde(default)]
    pub allow_preempt: bool,
}

/// Acquire stream response
#[derive(Debug, Clone, Serialize)]
pub struct AcquireStreamResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<StreamToken>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AbsorberError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preempted_session: Option<String>,
}

/// Release stream request
#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseStreamRequest {
    pub session_id: String,
}

/// Information about a preempted session (returned when preemption occurs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreemptionInfo {
    /// Session ID that was preempted
    pub session_id: String,
    /// Camera ID
    pub camera_id: String,
    /// Purpose of the preempted stream
    pub preempted_purpose: StreamPurpose,
    /// Client ID of the preempted session
    pub preempted_client_id: String,
    /// Purpose that caused the preemption
    pub preempted_by_purpose: StreamPurpose,
    /// Client ID that caused the preemption
    pub preempted_by_client_id: String,
    /// Recommended exit delay in seconds (5 for suggest)
    pub exit_delay_sec: u32,
}

/// Result of acquire_stream operation (includes preemption info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquireStreamResult {
    /// The acquired stream token
    pub token: StreamToken,
    /// Information about preempted session (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preemption: Option<PreemptionInfo>,
}
