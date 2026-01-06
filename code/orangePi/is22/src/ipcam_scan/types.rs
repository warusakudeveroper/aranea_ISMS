//! IpcamScan types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Scan mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScanMode {
    #[default]
    Discovery,
    Deep,
}

/// Job status
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Partial,
    Success,
    Failed,
    Canceled,
}

/// Camera family
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum CameraFamily {
    Tapo,
    Vigi,
    Nest,
    Axis,
    Hikvision,
    Dahua,
    Other,
    Unknown,
}

impl Default for CameraFamily {
    fn default() -> Self {
        Self::Unknown
    }
}

/// RTSP URL template per camera family
/// Used to generate both main and sub stream URLs during device registration
/// FIX-004: rtsp_sub自動登録対応
#[derive(Debug, Clone)]
pub struct RtspTemplate {
    pub main_path: &'static str,
    pub sub_path: &'static str,
    pub default_port: u16,
}

impl RtspTemplate {
    /// Get RTSP template for a camera family
    /// Based on manufacturer documentation and verified device behavior
    pub fn for_family(family: &CameraFamily) -> Self {
        match family {
            // TP-Link Tapo: /stream1 (main), /stream2 (sub)
            CameraFamily::Tapo => Self {
                main_path: "/stream1",
                sub_path: "/stream2",
                default_port: 554,
            },
            // TP-Link VIGI: Same as Tapo
            CameraFamily::Vigi => Self {
                main_path: "/stream1",
                sub_path: "/stream2",
                default_port: 554,
            },
            // Hikvision: /Streaming/Channels/101 (main), /102 (sub)
            CameraFamily::Hikvision => Self {
                main_path: "/Streaming/Channels/101",
                sub_path: "/Streaming/Channels/102",
                default_port: 554,
            },
            // Dahua: /cam/realmonitor?channel=1&subtype=0 (main), subtype=1 (sub)
            CameraFamily::Dahua => Self {
                main_path: "/cam/realmonitor?channel=1&subtype=0",
                sub_path: "/cam/realmonitor?channel=1&subtype=1",
                default_port: 554,
            },
            // Axis: /axis-media/media.amp (main), /axis-media/media.amp?videocodec=h264&resolution=640x480 (sub)
            CameraFamily::Axis => Self {
                main_path: "/axis-media/media.amp",
                sub_path: "/axis-media/media.amp?videocodec=h264&resolution=640x480",
                default_port: 554,
            },
            // Nest: Google cameras typically don't expose RTSP directly
            CameraFamily::Nest => Self {
                main_path: "/live",
                sub_path: "/live",
                default_port: 554,
            },
            // Other/Unknown: Use common /stream1, /stream2 pattern
            CameraFamily::Other | CameraFamily::Unknown => Self {
                main_path: "/stream1",
                sub_path: "/stream2",
                default_port: 554,
            },
        }
    }

    /// Generate RTSP URLs with credentials
    /// Returns (rtsp_main, rtsp_sub)
    pub fn generate_urls(
        &self,
        ip: &str,
        port: Option<u16>,
        username: &str,
        password: &str,
    ) -> (String, String) {
        let port = port.unwrap_or(self.default_port);
        // URL encode @ in password (common in generated passwords)
        let encoded_pass = password.replace("@", "%40");

        let main_url = format!(
            "rtsp://{}:{}@{}:{}{}",
            username, encoded_pass, ip, port, self.main_path
        );
        let sub_url = format!(
            "rtsp://{}:{}@{}:{}{}",
            username, encoded_pass, ip, port, self.sub_path
        );

        (main_url, sub_url)
    }
}

/// Device status for registration flow
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type, Default)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum DeviceStatus {
    #[default]
    Discovered,
    Verifying,
    Verified,
    Rejected,
    Approved,
}

/// Connection status - ユーザーに「なぜ繋がらないか」を伝える
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    #[default]
    NotTested,           // 未試行
    Success,             // 接続成功（ONVIF/RTSP応答あり）
    AuthRequired,        // 認証が必要（401/認証エラー）
    AuthFailed,          // 認証失敗（クレデンシャル不正）
    Timeout,             // タイムアウト
    Refused,             // 接続拒否
    PortOpenOnly,        // ポートは開いているが応答なし
    Unknown,             // 不明なエラー
}

/// Detection reason - なぜカメラと判断したか
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetectionReason {
    /// OUIベンダー一致（TP-Link, Axis等）
    pub oui_match: Option<String>,
    /// 開いているカメラ関連ポート
    pub camera_ports: Vec<u16>,
    /// ONVIFプローブ結果
    pub onvif_status: ConnectionStatus,
    /// RTSPプローブ結果
    pub rtsp_status: ConnectionStatus,
    /// 推定デバイスタイプ
    pub device_type: DeviceType,
    /// ユーザー向けメッセージ
    pub user_message: String,
    /// 推奨アクション
    pub suggested_action: SuggestedAction,
}

/// 検出されたデバイスタイプ
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    #[default]
    Unknown,
    CameraConfirmed,     // カメラ確定（ONVIF/RTSP応答あり）
    CameraLikely,        // カメラの可能性高（ポート+OUI一致）
    CameraPossible,      // カメラかもしれない（ポートのみ）
    NvrLikely,           // NVRの可能性（複数ストリームポート等）
    NetworkDevice,       // ネットワーク機器（ルーター/スイッチ）
    OtherDevice,         // その他のデバイス
}

/// 推奨アクション
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SuggestedAction {
    #[default]
    None,
    SetCredentials,      // クレデンシャル設定が必要
    RetryWithAuth,       // 認証情報で再試行
    CheckNetwork,        // ネットワーク確認
    ManualCheck,         // 手動確認推奨
    Ignore,              // 無視推奨（カメラではない）
}

// ============================================================================
// カテゴリ分類 (SSoT統一型定義)
// ============================================================================

/// デバイスカテゴリ (メイン分類)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeviceCategory {
    /// A: 登録済みカメラ（IP一致）
    A,
    /// B: 登録可能カメラ（認証成功）
    B,
    /// C: 認証待ちカメラ（RTSP/ONVIF応答あり、認証必要）
    C,
    /// D: カメラ可能性あり（OUI一致等）
    D,
    /// E: 非カメラ（応答なし等）
    E,
    /// F: 通信断・迷子カメラ
    F,
    #[default]
    Unknown,
}

/// デバイスカテゴリ詳細
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeviceCategoryDetail {
    // カテゴリA
    Registered,            // A: 登録済み（IP一致）
    // カテゴリB
    Registrable,           // B: 登録可能（認証成功）
    // カテゴリC
    AuthRequired,          // C: 認証必要
    AuthFailed,            // C: 認証失敗
    // カテゴリD
    PossibleCamera,        // D: カメラ可能性あり（OUI一致）
    NetworkEquipment,      // D: ネットワーク機器
    IoTDevice,             // D: IoTデバイス
    UnknownDevice,         // D: 不明
    // カテゴリE
    NonCamera,             // E: 非カメラ
    // カテゴリF
    LostConnection,        // F: 通信断
    StrayChild,            // F: 迷子カメラ（IP変更検出）
    #[default]
    Unknown,
}

/// Credential trial status - スキャン時のクレデンシャル試行結果
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum CredentialStatus {
    #[default]
    NotTried,    // 試行なし（クレデンシャル未設定）
    Success,     // 成功（モデル情報取得済み）
    Failed,      // 失敗（全クレデンシャル不一致）
}

/// Trial credential for subnet (stored as JSON in DB)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialCredential {
    pub username: String,
    pub password: String,
    pub priority: u8,  // 1-10, 試行順序
}

/// Scanned device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedDevice {
    pub device_id: Uuid,
    pub ip: String,
    pub subnet: String,
    pub mac: Option<String>,
    pub oui_vendor: Option<String>,
    pub hostnames: Vec<String>,
    pub open_ports: Vec<PortStatus>,
    pub score: u32,
    pub verified: bool,
    pub status: DeviceStatus,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub family: CameraFamily,
    pub confidence: u8,
    pub rtsp_uri: Option<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    /// 検出理由とユーザーフィードバック
    #[serde(default)]
    pub detection: DetectionReason,
    /// クレデンシャル試行結果
    #[serde(default)]
    pub credential_status: CredentialStatus,
    /// 成功したクレデンシャルのユーザー名
    pub credential_username: Option<String>,
    /// 成功したクレデンシャルのパスワード
    pub credential_password: Option<String>,
    // ============================================
    // カテゴリ分類 (SSoT統一フィールド)
    // ============================================
    /// デバイスカテゴリ (A-F)
    #[serde(default)]
    pub category: DeviceCategory,
    /// カテゴリ詳細
    #[serde(default)]
    pub category_detail: DeviceCategoryDetail,
    /// 登録済みカメラ名（カテゴリA/Fの場合）
    pub registered_camera_name: Option<String>,
    /// 登録済みカメラID（カテゴリA/Fの場合）
    pub registered_camera_id: Option<String>,
    /// IP変更検出フラグ（StrayChild判定用）
    #[serde(default)]
    pub ip_changed: bool,
}

/// Port status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortStatus {
    pub port: u16,
    pub status: PortState,
}

/// Port state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortState {
    Open,
    Closed,
    Filtered,
}

/// Device filter
#[derive(Debug, Clone, Default)]
pub struct DeviceFilter {
    pub subnet: Option<String>,
    pub family: Option<CameraFamily>,
    pub verified: Option<bool>,
    pub status: Option<DeviceStatus>,
}

/// Approve device request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveDeviceRequest {
    pub display_name: String,
    pub location: String,
    pub fid: String,
    /// RTSP credentials (required for go2rtc registration)
    pub credentials: Option<ApproveCredentials>,
}

/// Credentials for camera approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveCredentials {
    pub username: String,
    pub password: String,
}

/// Approve device response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveDeviceResponse {
    pub camera_id: String,
    pub lacis_id: String,
    pub ip_address: String,
    /// RTSP main stream URL with credentials (FIX-004)
    pub rtsp_main: Option<String>,
    /// RTSP sub stream URL with credentials (FIX-004)
    pub rtsp_sub: Option<String>,
}

/// Batch verify request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchVerifyRequest {
    pub fid: String,
    pub device_ids: Vec<Uuid>,
    pub use_facility_credentials: bool,
}

/// Batch verify result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchVerifyResult {
    pub device_id: Uuid,
    pub verified: bool,
    pub rtsp_uri: Option<String>,
    pub error: Option<String>,
}

/// Facility credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacilityCredentials {
    pub fid: String,
    pub facility_name: String,
    pub username: String,
    // Note: password is not serialized for security
}

/// Create facility credentials request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFacilityCredentialsRequest {
    pub fid: String,
    pub facility_name: String,
    pub username: String,
    pub password: String,
}

/// Lost camera info (#82 T2-8)
/// Information about a registered camera that was not found during scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LostCameraInfo {
    /// Camera ID from cameras table
    pub camera_id: String,
    /// Camera name
    pub camera_name: String,
    /// Registered IP address
    pub ip_address: String,
    /// MAC address (if known)
    pub mac_address: Option<String>,
    /// Category (always F for lost)
    pub category: DeviceCategory,
    /// Detail: LostConnection or StrayChild
    pub category_detail: DeviceCategoryDetail,
    /// New IP address (if StrayChild detected via MAC match)
    pub new_ip_address: Option<String>,
}

/// Force register request (#83 T2-10)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceRegisterRequest {
    pub name: String,
    pub location: String,
    pub fid: String,
}

/// Force register response (#83 T2-10)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceRegisterResponse {
    pub camera_id: String,
    pub lacis_id: String,
    pub ip_address: String,
    pub status: String,
    pub rtsp_main: String,
    pub message: String,
}
