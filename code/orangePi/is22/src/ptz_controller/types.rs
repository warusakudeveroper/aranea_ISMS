//! PTZ Controller type definitions

use serde::{Deserialize, Serialize};

/// PTZ移動方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PtzDirection {
    Up,
    Down,
    Left,
    Right,
}

impl PtzDirection {
    /// rotation角度に基づいて方向を変換
    /// 例: rotation=180のとき、up→down, left→right
    pub fn apply_rotation(&self, rotation: i32) -> Self {
        let normalized = ((rotation % 360) + 360) % 360;
        match normalized {
            0 => *self,
            90 => match self {
                Self::Up => Self::Right,
                Self::Right => Self::Down,
                Self::Down => Self::Left,
                Self::Left => Self::Up,
            },
            180 => match self {
                Self::Up => Self::Down,
                Self::Down => Self::Up,
                Self::Left => Self::Right,
                Self::Right => Self::Left,
            },
            270 => match self {
                Self::Up => Self::Left,
                Self::Left => Self::Down,
                Self::Down => Self::Right,
                Self::Right => Self::Up,
            },
            _ => *self,
        }
    }

    /// 日本語表示名
    pub fn to_japanese(&self) -> &'static str {
        match self {
            Self::Up => "上",
            Self::Down => "下",
            Self::Left => "左",
            Self::Right => "右",
        }
    }
}

/// PTZ操作モード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PtzMode {
    /// チョン押し（短時間の移動）
    Nudge,
    /// 長押し（押している間連続移動）
    Continuous,
}

/// PTZ Move リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct PtzMoveRequest {
    /// Modal Lease ID（認証用）
    pub lease_id: String,
    /// 移動方向
    pub direction: PtzDirection,
    /// 操作モード
    pub mode: PtzMode,
    /// 速度 (0.0-1.0)
    #[serde(default = "default_speed")]
    pub speed: f32,
    /// Nudgeモード時の移動時間(ms)
    #[serde(default = "default_duration")]
    pub duration_ms: u32,
}

fn default_speed() -> f32 {
    0.5
}

fn default_duration() -> u32 {
    120
}

/// PTZ Stop リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct PtzStopRequest {
    /// Modal Lease ID（認証用）
    pub lease_id: String,
}

/// PTZ Home リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct PtzHomeRequest {
    /// Modal Lease ID（認証用）
    pub lease_id: String,
}

/// PTZ操作結果
#[derive(Debug, Clone, Serialize)]
pub struct PtzResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl PtzResponse {
    pub fn success() -> Self {
        Self {
            ok: true,
            error: None,
            message: None,
        }
    }

    pub fn success_with_message(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            error: None,
            message: Some(message.into()),
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(error.into()),
            message: None,
        }
    }
}

/// PTZステータス
#[derive(Debug, Clone, Serialize)]
pub struct PtzStatus {
    pub camera_id: String,
    pub ptz_supported: bool,
    pub ptz_disabled: bool,
    pub ptz_continuous: bool,
    pub ptz_absolute: bool,
    pub ptz_relative: bool,
    pub ptz_home_supported: bool,
    pub is_moving: bool,
    pub current_direction: Option<PtzDirection>,
}
