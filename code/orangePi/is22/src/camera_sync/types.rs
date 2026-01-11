//! Camera Sync Type Definitions
//!
//! Phase 8: カメラ双方向同期 (Issue #121)
//! T8-2: types.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 同期タイプ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncType {
    /// 全カメラ同期（初回・定期）
    Full,
    /// 変更分のみ（名前変更・削除）
    Partial,
    /// 単一カメラ更新
    Single,
}

/// 同期方向
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum SyncDirection {
    /// IS22 → mobes2.0
    ToMobes,
    /// mobes2.0 → IS22
    FromMobes,
}

/// 同期状態
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum MobesSyncStatus {
    /// 同期済み
    Synced,
    /// 同期保留中
    Pending,
    /// 同期失敗
    Failed,
    /// 削除済み
    Deleted,
}

impl Default for MobesSyncStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// カメラステータス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CameraStatus {
    Online,
    Offline,
    Unknown,
    Malfunction,
}

impl Default for CameraStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// 削除理由
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeleteReason {
    /// 物理的にカメラを取り外し
    PhysicalRemoval,
    /// ユーザーによる削除操作
    UserRequest,
    /// 故障による削除
    Malfunction,
    /// 機器交換
    Replacement,
    /// 管理者による削除（mobes2.0側）
    AdminRemoval,
}

/// カメラコンテキストデータ
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CameraContextData {
    /// 用途・目的
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    /// 監視対象
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitoring_target: Option<String>,
    /// 重点項目
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_focus: Option<String>,
    /// 運用上の注意事項
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operational_notes: Option<String>,
}

/// カメラメタデータエントリ（IS22→mobes送信用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraMetadataEntry {
    /// カメラのLacisID
    pub lacis_id: String,
    /// カメラID（IS22内部ID）
    pub camera_id: String,
    /// カメラ名
    pub name: String,
    /// 設置場所
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// コンテキスト情報
    pub context: CameraContextData,
    /// MACアドレス
    pub mac: String,
    /// カメラブランド
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    /// カメラモデル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ステータス
    pub status: CameraStatus,
    /// 最終確認日時
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<DateTime<Utc>>,
}

/// カメラメタデータペイロード（IS22→mobes送信）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraMetadataPayload {
    /// カメラメタデータリスト
    pub cameras: Vec<CameraMetadataEntry>,
    /// 同期タイプ
    pub sync_type: SyncType,
    /// 更新日時
    pub updated_at: DateTime<Utc>,
}

/// 削除されたカメラエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraDeletedEntry {
    /// カメラのLacisID
    pub lacis_id: String,
    /// カメラID（IS22内部ID）
    pub camera_id: String,
    /// 削除日時
    pub deleted_at: DateTime<Utc>,
    /// 削除理由
    pub reason: DeleteReason,
}

/// カメラ削除ペイロード（IS22→mobes送信）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraDeletedPayload {
    /// 削除されたカメラリスト
    pub deleted_cameras: Vec<CameraDeletedEntry>,
    /// 同期タイプ（常にPartial）
    pub sync_type: SyncType,
    /// 更新日時
    pub updated_at: DateTime<Utc>,
}

/// カメラ個別設定（mobes2.0から同期）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraParaclateSettings {
    /// 検出感度 (0.0-1.0)
    #[serde(default = "default_sensitivity")]
    pub sensitivity: f32,
    /// 検出ゾーン設定
    #[serde(default = "default_detection_zone")]
    pub detection_zone: String,
    /// アラート閾値
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: i32,
    /// カスタムプリセット名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_preset: Option<String>,
}

fn default_sensitivity() -> f32 {
    0.6
}

fn default_detection_zone() -> String {
    "full".to_string()
}

fn default_alert_threshold() -> i32 {
    3
}

impl Default for CameraParaclateSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.6,
            detection_zone: "full".to_string(),
            alert_threshold: 3,
            custom_preset: None,
        }
    }
}

/// 同期状態レコード（DBモデル）
#[derive(Debug, Clone)]
pub struct CameraSyncState {
    pub id: Option<i64>,
    pub camera_id: String,
    pub lacis_id: String,
    pub last_sync_to_mobes: Option<DateTime<Utc>>,
    pub last_sync_from_mobes: Option<DateTime<Utc>>,
    pub mobes_sync_status: MobesSyncStatus,
    pub sync_error_message: Option<String>,
    pub retry_count: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// 同期ログレコード（DBモデル）
#[derive(Debug, Clone)]
pub struct CameraSyncLog {
    pub id: Option<i64>,
    pub camera_id: String,
    pub lacis_id: Option<String>,
    pub sync_direction: SyncDirection,
    pub sync_type: String,
    pub status: String,
    pub changed_fields: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub request_payload: Option<serde_json::Value>,
    pub response_payload: Option<serde_json::Value>,
    pub created_at: Option<DateTime<Utc>>,
}

/// 同期リクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRequest {
    /// 同期タイプ
    pub sync_type: SyncType,
    /// 対象カメラID（空の場合は全カメラ）
    #[serde(default)]
    pub camera_ids: Vec<String>,
}

/// 同期レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    /// 成功フラグ
    pub success: bool,
    /// 同期されたカメラ数
    pub synced_count: usize,
    /// 失敗したカメラ数
    pub failed_count: usize,
    /// 詳細
    pub details: Vec<SyncDetail>,
}

/// 同期詳細
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncDetail {
    pub camera_id: String,
    pub lacis_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 同期状態レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusResponse {
    /// 最終フル同期日時
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_full_sync: Option<DateTime<Utc>>,
    /// 同期保留中のカメラ数
    pub pending_count: usize,
    /// 同期済みカメラ数
    pub synced_count: usize,
    /// 同期失敗カメラ数
    pub failed_count: usize,
    /// カメラ別状態
    pub cameras: Vec<CameraSyncStatusEntry>,
}

/// カメラ別同期状態
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraSyncStatusEntry {
    pub camera_id: String,
    pub lacis_id: String,
    pub mobes_sync_status: MobesSyncStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_to_mobes: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_from_mobes: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_type_serialization() {
        let sync_type = SyncType::Full;
        let json = serde_json::to_string(&sync_type).unwrap();
        assert_eq!(json, "\"full\"");

        let parsed: SyncType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SyncType::Full);
    }

    #[test]
    fn test_camera_metadata_entry_serialization() {
        let entry = CameraMetadataEntry {
            lacis_id: "3801E051D815448B0001".to_string(),
            camera_id: "cam_001".to_string(),
            name: "正面玄関カメラ".to_string(),
            location: Some("1F エントランス".to_string()),
            context: CameraContextData {
                purpose: Some("来客監視".to_string()),
                monitoring_target: Some("人物".to_string()),
                priority_focus: None,
                operational_notes: None,
            },
            mac: "E0:51:D8:15:44:8B".to_string(),
            brand: Some("HIKVISION".to_string()),
            model: None,
            status: CameraStatus::Online,
            last_seen: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("lacisId"));
        assert!(json.contains("3801E051D815448B0001"));
        assert!(json.contains("正面玄関カメラ"));
    }

    #[test]
    fn test_camera_paraclate_settings_default() {
        let settings = CameraParaclateSettings::default();
        assert_eq!(settings.sensitivity, 0.6);
        assert_eq!(settings.detection_zone, "full");
        assert_eq!(settings.alert_threshold, 3);
        assert!(settings.custom_preset.is_none());
    }

    #[test]
    fn test_delete_reason_serialization() {
        let reason = DeleteReason::PhysicalRemoval;
        let json = serde_json::to_string(&reason).unwrap();
        assert_eq!(json, "\"physical_removal\"");
    }
}
