//! IS21 Activation Type Definitions
//!
//! T5-1: IS22→IS21アクティベート

use serde::{Deserialize, Serialize};

/// IS21アクティベート用認証情報
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Is21ActivationAuth {
    /// IS22のLacisID（認証元）
    pub is22_lacis_id: String,
    /// テナントID
    pub tid: String,
    /// IS22のCIC
    pub cic: String,
    /// ユーザーID（オプション）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// IS21アクティベート結果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Is21ActivationResult {
    /// 成功フラグ
    pub ok: bool,
    /// IS21のLacisID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is21_lacis_id: Option<String>,
    /// IS21のCIC
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cic: Option<String>,
    /// IS21のエンドポイント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// エラーメッセージ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IS21アクティベートリクエスト（API用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Is21ActivateRequest {
    /// IS21のエンドポイント（例: http://192.168.3.240:9000）
    pub endpoint: String,
}

/// IS21ステータス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Is21Status {
    /// アクティベート済みか
    pub activated: bool,
    /// IS21のLacisID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lacis_id: Option<String>,
    /// IS21のCIC
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cic: Option<String>,
    /// IS21のエンドポイント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// 最終接続確認時刻
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
    /// オンラインか
    pub online: bool,
}

impl Default for Is21Status {
    fn default() -> Self {
        Self {
            activated: false,
            lacis_id: None,
            cic: None,
            endpoint: None,
            last_seen_at: None,
            online: false,
        }
    }
}
