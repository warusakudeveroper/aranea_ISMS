//! AraneaRegister type definitions
//!
//! Phase 1: AraneaRegister (Issue #114)
//! DD01_AraneaRegister.md準拠
//!
//! ## LacisID形式 (is22用)
//! [Prefix=3][ProductType=022][MAC=12桁][ProductCode=0000] = 20桁
//!
//! ## SSoT原則
//! - config_store: 即時参照用（起動時読み込み）
//! - aranea_registration: 履歴・監査用

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================
// 定数定義 (DD01準拠)
// ============================================================

/// araneaDevice共通プレフィックス
pub const PREFIX: &str = "3";

/// is22 Paraclate Server プロダクトタイプ
pub const PRODUCT_TYPE: &str = "022";

/// 追い番なしのため固定
pub const PRODUCT_CODE: &str = "0000";

/// デバイスタイプ
pub const DEVICE_TYPE: &str = "ar-is22CamServer";

/// タイプドメイン
pub const TYPE_DOMAIN: &str = "araneaDevice";

/// LacisID総桁数
pub const LACIS_ID_LENGTH: usize = 20;

/// MACアドレス桁数（コロン除去後）
pub const MAC_LENGTH: usize = 12;

// ============================================================
// config_storeキー定義
// ============================================================

/// config_store用キープレフィックス
pub mod config_keys {
    pub const LACIS_ID: &str = "aranea.lacis_id";
    pub const TID: &str = "aranea.tid";
    pub const CIC: &str = "aranea.cic";
    pub const REGISTERED: &str = "aranea.registered";
    pub const STATE_ENDPOINT: &str = "aranea.state_endpoint";
    pub const MQTT_ENDPOINT: &str = "aranea.mqtt_endpoint";
}

// ============================================================
// 型定義
// ============================================================

/// テナントプライマリ認証情報
/// JSONキー（camelCase）とRustフィールド（snake_case）の変換を自動化
/// (CONSISTENCY_CHECK P0-6対応)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantPrimaryAuth {
    pub lacis_id: String,
    pub user_id: String,
    pub cic: String,
}

/// 登録リクエスト (API受付用)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub tenant_primary_auth: TenantPrimaryAuth,
    pub tid: String,
}

/// araneaDeviceGateへ送信するペイロード
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceGatePayload {
    pub lacis_oath: LacisOathForRegister,
    pub user_object: UserObject,
    pub device_meta: DeviceMeta,
}

/// 登録用lacisOath
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LacisOathForRegister {
    pub lacis_id: String,
    pub user_id: String,
    pub cic: String,
    pub method: String, // "register"
}

/// userObject (登録対象デバイス情報)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserObject {
    #[serde(rename = "lacisID")]
    pub lacis_id: String,
    pub tid: String,
    #[serde(rename = "typeDomain")]
    pub type_domain: String,
    #[serde(rename = "type")]
    pub device_type: String,
}

/// deviceMeta (デバイスメタ情報)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceMeta {
    pub mac_address: String,
    pub product_type: String,
    pub product_code: String,
}

/// araneaDeviceGateからのレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceGateResponse {
    pub ok: bool,
    #[serde(default)]
    pub user_object: Option<DeviceGateUserObjectResponse>,
    #[serde(default)]
    pub state_endpoint: Option<String>,
    #[serde(default)]
    pub mqtt_endpoint: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

/// レスポンス内のuserObject
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceGateUserObjectResponse {
    #[serde(rename = "lacisID")]
    pub lacis_id: Option<String>,
    pub cic_code: Option<String>,
}

/// 登録結果 (API応答用)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lacis_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mqtt_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 登録状態 (GET /api/register/status応答用)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationStatus {
    pub registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lacis_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<DateTime<Utc>>,
}

impl Default for RegistrationStatus {
    fn default() -> Self {
        Self {
            registered: false,
            lacis_id: None,
            tid: None,
            cic: None,
            registered_at: None,
            last_sync_at: None,
        }
    }
}

/// DB永続化用エンティティ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AraneaRegistration {
    pub id: Option<u32>,
    pub lacis_id: String,
    pub tid: String,
    pub cic: String,
    pub device_type: String,
    pub state_endpoint: Option<String>,
    pub mqtt_endpoint: Option<String>,
    pub registered_at: DateTime<Utc>,
    pub last_sync_at: Option<DateTime<Utc>>,
}

/// 登録クリア結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearResult {
    pub ok: bool,
    pub message: String,
}

// ============================================================
// バリデーション
// ============================================================

/// LacisIDのバリデーション（mobes2.0側バリデーション準拠）
/// 正規表現: ^[34]\d{3}[0-9A-F]{12}\d{4}$
pub fn validate_lacis_id(lacis_id: &str) -> bool {
    if lacis_id.len() != LACIS_ID_LENGTH {
        return false;
    }

    let chars: Vec<char> = lacis_id.chars().collect();

    // Prefix: 3 or 4
    if chars[0] != '3' && chars[0] != '4' {
        return false;
    }

    // ProductType: 3桁の数字
    if !chars[1..4].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // MAC: 12桁の16進数
    if !chars[4..16].iter().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }

    // ProductCode: 4桁の数字
    if !chars[16..20].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_lacis_id_valid() {
        assert!(validate_lacis_id("3022AABBCCDDEEFF0000"));
        assert!(validate_lacis_id("3022aabbccddeeff0000".to_uppercase().as_str()));
    }

    #[test]
    fn test_validate_lacis_id_invalid() {
        // 短すぎる
        assert!(!validate_lacis_id("3022AABBCCDD0000"));
        // 不正なプレフィックス
        assert!(!validate_lacis_id("1022AABBCCDDEEFF0000"));
        // MACに不正文字
        assert!(!validate_lacis_id("3022GGBBCCDDEEFF0000"));
    }

    #[test]
    fn test_constants() {
        assert_eq!(PREFIX, "3");
        assert_eq!(PRODUCT_TYPE, "022");
        assert_eq!(PRODUCT_CODE, "0000");
        assert_eq!(DEVICE_TYPE, "ar-is22CamServer");
        assert_eq!(TYPE_DOMAIN, "araneaDevice");
    }
}
