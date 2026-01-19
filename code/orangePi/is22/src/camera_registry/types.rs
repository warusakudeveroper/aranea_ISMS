//! CameraRegistry Type Definitions
//!
//! Phase 2: CameraRegistry (Issue #115)
//! DD05_CameraRegistry.md準拠

use serde::{Deserialize, Serialize};

/// LacisIDプレフィックス (araneaDevice共通)
pub const PREFIX: &str = "3";

/// ProductType: is801 paraclateCamera
pub const PRODUCT_TYPE: &str = "801";

/// MACアドレス長 (16進12桁)
pub const MAC_LENGTH: usize = 12;

/// ProductCode長 (4桁)
pub const PRODUCT_CODE_LENGTH: usize = 4;

/// LacisID全体長 (20桁)
pub const LACIS_ID_LENGTH: usize = 20;

/// デフォルトProductCode (Generic)
pub const DEFAULT_PRODUCT_CODE: &str = "0000";

/// デバイスタイプ (araneaDeviceGate登録用)
/// TYPE_REGISTRY.md準拠: ar-is801ParaclateCamera
pub const DEVICE_TYPE: &str = "ar-is801ParaclateCamera";

/// ProductCode定義
pub mod product_codes {
    pub const GENERIC: &str = "0000";
    pub const TAPO: &str = "0001";
    pub const HIKVISION: &str = "0002";
    pub const DAHUA: &str = "0003";
    pub const REOLINK: &str = "0004";
    pub const AXIS: &str = "0005";
}

/// カメラ登録リクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraRegisterRequest {
    /// カメラID (cameras.camera_id)
    pub camera_id: String,

    /// カメラのMACアドレス
    pub mac_address: String,

    /// ブランド別ProductCode
    #[serde(default = "default_product_code")]
    pub product_code: String,

    /// 施設ID (オプション)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fid: Option<String>,

    /// 部屋/エリアID (オプション)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rid: Option<String>,
}

fn default_product_code() -> String {
    DEFAULT_PRODUCT_CODE.to_string()
}

/// カメラ登録レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraRegisterResponse {
    /// 登録成功フラグ
    pub ok: bool,

    /// 生成されたLacisID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lacis_id: Option<String>,

    /// 取得したCIC
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cic: Option<String>,

    /// エラーメッセージ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl CameraRegisterResponse {
    pub fn success(lacis_id: String, cic: String) -> Self {
        Self {
            ok: true,
            lacis_id: Some(lacis_id),
            cic: Some(cic),
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            lacis_id: None,
            cic: None,
            error: Some(msg.into()),
        }
    }
}

/// カメラ登録状態
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationState {
    Pending,
    Registered,
    Failed,
    Cleared,
}

impl std::fmt::Display for RegistrationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Registered => write!(f, "registered"),
            Self::Failed => write!(f, "failed"),
            Self::Cleared => write!(f, "cleared"),
        }
    }
}

impl From<&str> for RegistrationState {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "registered" => Self::Registered,
            "failed" => Self::Failed,
            "cleared" => Self::Cleared,
            _ => Self::Pending,
        }
    }
}

/// カメラ登録情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraRegistration {
    pub camera_id: String,
    pub lacis_id: String,
    pub tid: String,
    pub fid: Option<String>,
    pub rid: Option<String>,
    pub cic: String,
    pub state: RegistrationState,
    pub registered_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
}

/// カメラ登録ステータスレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraRegistrationStatus {
    pub camera_id: String,
    pub registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lacis_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rid: Option<String>,
    pub state: RegistrationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<CameraRegistration> for CameraRegistrationStatus {
    fn from(reg: CameraRegistration) -> Self {
        Self {
            camera_id: reg.camera_id,
            registered: reg.state == RegistrationState::Registered,
            lacis_id: Some(reg.lacis_id),
            cic: Some(reg.cic),
            tid: Some(reg.tid),
            fid: reg.fid,
            rid: reg.rid,
            state: reg.state,
            registered_at: reg.registered_at,
        }
    }
}

/// araneaDeviceGate登録ペイロード
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateRegisterPayload {
    pub lacis_oath: LacisOath,
    pub user_object: UserObject,
    pub device_meta: DeviceMeta,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LacisOath {
    pub lacis_id: String,
    pub user_id: String,
    pub cic: String,
    pub method: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserObject {
    #[serde(rename = "lacisID")]  // AUTH_SPEC Section 5.3: userObject uses capital "ID"
    pub lacis_id: String,
    pub tid: String,
    pub type_domain: String,
    #[serde(rename = "type")]
    pub device_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceMeta {
    pub mac_address: String,
    pub product_type: String,
    pub product_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_lacis_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_name: Option<String>,
}

/// araneaDeviceGate登録レスポンス
/// 実際のレスポンス構造:
/// {
///   "ok": true,
///   "existing": true/false,
///   "lacisId": "...",
///   "userObject": { "cic_code": "...", "cic_active": true, "permission": 10 },
///   "stateEndpoint": "..."
/// }
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateRegisterResponse {
    /// 登録成功フラグ
    pub ok: Option<bool>,
    /// 既存デバイスかどうか
    pub existing: Option<bool>,
    /// 登録されたLacisID
    pub lacis_id: Option<String>,
    /// デバイス情報 (CICを含む)
    pub user_object: Option<GateUserObject>,
    /// 状態報告エンドポイント
    pub state_endpoint: Option<String>,
    /// MQTTエンドポイント
    pub mqtt_endpoint: Option<String>,
    /// エラーメッセージ
    pub error: Option<String>,
}

/// Gate userObject (CIC情報を含む)
#[derive(Debug, Clone, Deserialize)]
pub struct GateUserObject {
    /// CICコード
    pub cic_code: Option<String>,
    /// CIC有効フラグ
    pub cic_active: Option<bool>,
    /// 権限レベル
    pub permission: Option<i32>,
}

impl GateRegisterResponse {
    /// CICコードを取得 (userObject.cic_codeから抽出)
    pub fn get_cic(&self) -> Option<String> {
        self.user_object.as_ref().and_then(|u| u.cic_code.clone())
    }

    /// 登録成功かどうか
    pub fn is_success(&self) -> bool {
        self.ok.unwrap_or(false) && self.get_cic().is_some()
    }
}
