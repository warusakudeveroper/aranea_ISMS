//! Pub/Sub Subscriber
//!
//! Phase 4: ParaclateClient (Issue #117)
//! T4-7: Pub/Sub受信フロー（設定変更通知）
//!
//! ## 概要
//! `paraclate-config-updates` Topicからの通知を受信し、
//! ConfigSyncServiceを通じて設定を同期する。
//!
//! ## 設計（ddreview_2 P1-4確定）
//! - Topic: `paraclate-config-updates`
//! - Payload: `{type, tid, fids[], updatedAt, actor}`
//! - config本体は配らない（通知のみ）
//! - 通知受信後、GET /config/{tid}でSSoTからpull
//!
//! ## 実装方式
//! - Push Subscription: HTTPエンドポイントで通知を受信
//! - is22がNAT背後の場合はPolling fallbackも検討
//!
//! ## セキュリティ
//! - LacisOath認証で通知の正当性を検証
//! - 署名検証またはTID/FID照合で不正通知を排除
//! - Issue #119: FidValidatorによるテナント-FID所属検証

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::config_sync::ConfigSyncService;
use super::fid_validator::FidValidator;
use super::types::ParaclateError;

/// 設定変更通知ペイロード
///
/// Pub/Sub Topic: `paraclate-config-updates`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigUpdateNotification {
    /// 通知タイプ
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    /// テナントID
    pub tid: String,
    /// 対象施設ID配列（空の場合はテナント全体）
    pub fids: Vec<String>,
    /// 更新日時
    pub updated_at: DateTime<Utc>,
    /// 更新アクター（ユーザーID or "system"）
    pub actor: String,
    /// 変更されたフィールド（オプション）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_fields: Vec<String>,
    /// 変更されたカメラ（Phase 8: camera_settings用）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_cameras: Vec<ChangedCamera>,
    /// 削除されたカメラ（Phase 8: camera_remove用）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_cameras: Vec<RemovedCamera>,
}

/// 変更されたカメラ情報（Phase 8）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedCamera {
    /// カメラのLacisID
    pub lacis_id: String,
    /// 変更されたフィールド
    #[serde(default)]
    pub changed_fields: Vec<String>,
}

/// 削除されたカメラ情報（Phase 8）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovedCamera {
    /// カメラのLacisID
    pub lacis_id: String,
    /// 削除理由
    pub reason: String,
}

/// 通知タイプ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    /// 設定更新
    ConfigUpdate,
    /// 設定削除
    ConfigDelete,
    /// 接続解除
    Disconnect,
    /// 強制同期要求
    ForceSync,
    /// カメラ個別設定変更（Phase 8追加）
    CameraSettings,
    /// カメラ削除指示（Phase 8追加）
    CameraRemove,
}

/// Google Cloud Pub/Sub Push形式のラッパー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubSubPushMessage {
    /// メッセージ本体
    pub message: PubSubMessage,
    /// サブスクリプション名
    pub subscription: String,
}

/// Pub/Subメッセージ形式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PubSubMessage {
    /// Base64エンコードされたデータ
    pub data: String,
    /// メッセージID
    pub message_id: String,
    /// 発行日時
    pub publish_time: DateTime<Utc>,
    /// 属性（オプション）
    #[serde(default)]
    pub attributes: std::collections::HashMap<String, String>,
}

/// Pub/Sub Subscriber
pub struct PubSubSubscriber {
    config_sync: Arc<ConfigSyncService>,
    /// 許可するTID（設定されている場合のみ受け付ける）
    allowed_tid: Option<String>,
    /// 許可するFID（設定されている場合のみ受け付ける）
    allowed_fids: Vec<String>,
    /// FID Validator (Issue #119)
    fid_validator: RwLock<Option<Arc<FidValidator>>>,
}

impl PubSubSubscriber {
    /// 新規作成
    pub fn new(config_sync: Arc<ConfigSyncService>) -> Self {
        Self {
            config_sync,
            allowed_tid: None,
            allowed_fids: vec![],
            fid_validator: RwLock::new(None),
        }
    }

    /// 許可TIDを設定
    pub fn with_allowed_tid(mut self, tid: String) -> Self {
        self.allowed_tid = Some(tid);
        self
    }

    /// 許可FIDを設定
    pub fn with_allowed_fids(mut self, fids: Vec<String>) -> Self {
        self.allowed_fids = fids;
        self
    }

    /// FidValidatorを設定 (Issue #119)
    pub async fn set_fid_validator(&self, validator: Arc<FidValidator>) {
        let mut guard = self.fid_validator.write().await;
        *guard = Some(validator);
    }

    /// FID所属検証を実行 (Issue #119)
    async fn validate_fids(&self, fids: &[String]) -> Result<Vec<String>, ParaclateError> {
        // 検証スキップチェック
        if FidValidator::should_skip_validation() {
            debug!("FID validation skipped (SKIP_FID_VALIDATION=1)");
            return Ok(fids.to_vec());
        }

        let guard = self.fid_validator.read().await;
        let validator = match guard.as_ref() {
            Some(v) => v,
            None => {
                debug!("FidValidator not configured, skipping validation");
                return Ok(fids.to_vec());
            }
        };

        let mut valid_fids = Vec::new();
        let mut invalid_fids = Vec::new();

        for fid in fids {
            match validator.validate_fid(fid).await {
                Ok(()) => valid_fids.push(fid.clone()),
                Err(e) => {
                    warn!(fid = %fid, error = %e, "FID validation failed, excluding from sync");
                    invalid_fids.push(fid.clone());
                }
            }
        }

        if !invalid_fids.is_empty() {
            warn!(
                invalid_fids = ?invalid_fids,
                valid_fids = ?valid_fids,
                "Some FIDs failed validation and will be excluded"
            );
        }

        if valid_fids.is_empty() && !fids.is_empty() {
            return Err(ParaclateError::FidValidation(format!(
                "All FIDs failed validation: {:?}",
                invalid_fids
            )));
        }

        Ok(valid_fids)
    }

    /// Push通知を処理
    ///
    /// Google Cloud Pub/Sub Push Subscription形式のリクエストを処理
    pub async fn handle_push(&self, push_message: PubSubPushMessage) -> Result<(), ParaclateError> {
        // Base64デコード
        let decoded_data = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &push_message.message.data,
        )
        .map_err(|e| ParaclateError::Parse(format!("Failed to decode message: {}", e)))?;

        // JSONパース
        let notification: ConfigUpdateNotification = serde_json::from_slice(&decoded_data)
            .map_err(|e| ParaclateError::Parse(format!("Failed to parse notification: {}", e)))?;

        info!(
            message_id = %push_message.message.message_id,
            notification_type = ?notification.notification_type,
            tid = %notification.tid,
            fids = ?notification.fids,
            "Received config update notification"
        );

        self.process_notification(notification).await
    }

    /// 直接通知を処理（テスト用・内部API用）
    pub async fn handle_direct(&self, notification: ConfigUpdateNotification) -> Result<(), ParaclateError> {
        info!(
            notification_type = ?notification.notification_type,
            tid = %notification.tid,
            fids = ?notification.fids,
            "Received direct config update notification"
        );

        self.process_notification(notification).await
    }

    /// 通知を処理
    async fn process_notification(&self, notification: ConfigUpdateNotification) -> Result<(), ParaclateError> {
        // TID検証
        if let Some(ref allowed_tid) = self.allowed_tid {
            if &notification.tid != allowed_tid {
                warn!(
                    expected_tid = %allowed_tid,
                    received_tid = %notification.tid,
                    "Ignoring notification for different TID"
                );
                return Ok(()); // 無視するだけでエラーにはしない
            }
        }

        // FID検証（fidsが空の場合はテナント全体なのでスキップ）
        if !self.allowed_fids.is_empty() && !notification.fids.is_empty() {
            let has_allowed_fid = notification.fids.iter().any(|fid| self.allowed_fids.contains(fid));
            if !has_allowed_fid {
                warn!(
                    allowed_fids = ?self.allowed_fids,
                    received_fids = ?notification.fids,
                    "Ignoring notification for non-matching FIDs"
                );
                return Ok(());
            }
        }

        // Issue #119: FID所属検証
        // 通知に含まれるFIDがデバイスの所属TIDに属しているか検証
        let validated_fids = if !notification.fids.is_empty() {
            match self.validate_fids(&notification.fids).await {
                Ok(fids) => fids,
                Err(e) => {
                    warn!(
                        error = %e,
                        fids = ?notification.fids,
                        "FID validation failed for notification, rejecting"
                    );
                    return Err(e);
                }
            }
        } else {
            // 空の場合はテナント全体 - 検証はスキップ
            notification.fids.clone()
        };

        // 検証済みFIDで通知を再構築
        let validated_notification = ConfigUpdateNotification {
            fids: validated_fids,
            ..notification
        };

        // 通知タイプに応じた処理
        match validated_notification.notification_type {
            NotificationType::ConfigUpdate | NotificationType::ForceSync => {
                self.handle_config_update(&validated_notification).await
            }
            NotificationType::ConfigDelete => {
                self.handle_config_delete(&validated_notification).await
            }
            NotificationType::Disconnect => {
                self.handle_disconnect(&validated_notification).await
            }
            NotificationType::CameraSettings => {
                self.handle_camera_settings(&validated_notification).await
            }
            NotificationType::CameraRemove => {
                self.handle_camera_remove(&validated_notification).await
            }
        }
    }

    /// 設定更新を処理
    async fn handle_config_update(&self, notification: &ConfigUpdateNotification) -> Result<(), ParaclateError> {
        let fids_to_sync = if notification.fids.is_empty() {
            // テナント全体の場合、許可されたFIDをすべて同期
            if self.allowed_fids.is_empty() {
                warn!("No FIDs configured for tenant-wide sync");
                return Ok(());
            }
            self.allowed_fids.clone()
        } else {
            // 指定されたFIDのうち、許可されたものを同期
            if self.allowed_fids.is_empty() {
                notification.fids.clone()
            } else {
                notification.fids.iter()
                    .filter(|fid| self.allowed_fids.contains(fid))
                    .cloned()
                    .collect()
            }
        };

        let mut success_count = 0;
        let mut error_count = 0;

        for fid in &fids_to_sync {
            match self.config_sync.sync(&notification.tid, fid).await {
                Ok(result) => {
                    if result.synced {
                        info!(
                            tid = %notification.tid,
                            fid = %fid,
                            updated_fields = ?result.updated_fields,
                            "Config synced successfully"
                        );
                    } else {
                        debug!(
                            tid = %notification.tid,
                            fid = %fid,
                            "Config already up to date"
                        );
                    }
                    success_count += 1;
                }
                Err(e) => {
                    error!(
                        tid = %notification.tid,
                        fid = %fid,
                        error = %e,
                        "Failed to sync config"
                    );
                    error_count += 1;
                }
            }
        }

        info!(
            tid = %notification.tid,
            success_count = success_count,
            error_count = error_count,
            "Config update notification processed"
        );

        if error_count > 0 && success_count == 0 {
            Err(ParaclateError::Sync(format!(
                "All {} FID sync attempts failed",
                error_count
            )))
        } else {
            Ok(())
        }
    }

    /// 設定削除を処理
    async fn handle_config_delete(&self, notification: &ConfigUpdateNotification) -> Result<(), ParaclateError> {
        warn!(
            tid = %notification.tid,
            fids = ?notification.fids,
            actor = %notification.actor,
            "Received config delete notification - local config will be marked as disconnected"
        );

        // 削除通知の場合、ローカル設定を「切断」状態にマーク
        // 実際の削除は行わない（ローカルキャッシュとして保持）
        // TODO: ConfigRepositoryにdisconnect()メソッドを追加して実装

        Ok(())
    }

    /// 切断通知を処理
    async fn handle_disconnect(&self, notification: &ConfigUpdateNotification) -> Result<(), ParaclateError> {
        warn!(
            tid = %notification.tid,
            fids = ?notification.fids,
            actor = %notification.actor,
            "Received disconnect notification from Paraclate APP"
        );

        // mobes2.0側から切断された場合の処理
        // is22側でも接続状態を更新
        // TODO: ConfigRepositoryにdisconnect()メソッドを追加して実装

        Ok(())
    }

    /// カメラ設定変更通知を処理（Phase 8: T8-6）
    async fn handle_camera_settings(&self, notification: &ConfigUpdateNotification) -> Result<(), ParaclateError> {
        info!(
            tid = %notification.tid,
            fids = ?notification.fids,
            changed_cameras = ?notification.changed_cameras.len(),
            actor = %notification.actor,
            "Received camera settings update notification"
        );

        if notification.changed_cameras.is_empty() {
            debug!("No changed cameras in notification, skipping");
            return Ok(());
        }

        // 各カメラの設定を同期（GetConfig経由で最新設定を取得）
        // TODO: CameraSyncServiceを注入してsync_camera_settings_from_mobesを呼び出す
        // 現時点ではログ出力のみ

        for camera in &notification.changed_cameras {
            info!(
                lacis_id = %camera.lacis_id,
                changed_fields = ?camera.changed_fields,
                "Camera settings changed - need to sync from mobes2.0"
            );
        }

        // GetConfigを呼び出してカメラ設定を取得・保存
        // self.config_sync.sync_camera_settings(...)

        Ok(())
    }

    /// カメラ削除通知を処理（Phase 8: T8-7）
    async fn handle_camera_remove(&self, notification: &ConfigUpdateNotification) -> Result<(), ParaclateError> {
        warn!(
            tid = %notification.tid,
            fids = ?notification.fids,
            removed_cameras = ?notification.removed_cameras.len(),
            actor = %notification.actor,
            "Received camera removal notification from mobes2.0"
        );

        if notification.removed_cameras.is_empty() {
            debug!("No removed cameras in notification, skipping");
            return Ok(());
        }

        // 各カメラを論理削除
        // TODO: CameraSyncServiceを注入してhandle_camera_remove_from_mobesを呼び出す
        // 現時点ではログ出力のみ

        for camera in &notification.removed_cameras {
            warn!(
                lacis_id = %camera.lacis_id,
                reason = %camera.reason,
                "Camera marked for removal by mobes2.0"
            );
        }

        Ok(())
    }
}

/// 処理結果
#[derive(Debug, Clone, Serialize)]
pub struct NotificationResult {
    /// 処理成功
    pub success: bool,
    /// 処理メッセージ
    pub message: String,
    /// 同期されたFID数
    pub synced_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_type_serialization() {
        let types = vec![
            (NotificationType::ConfigUpdate, "config_update"),
            (NotificationType::ConfigDelete, "config_delete"),
            (NotificationType::Disconnect, "disconnect"),
            (NotificationType::ForceSync, "force_sync"),
            (NotificationType::CameraSettings, "camera_settings"),
            (NotificationType::CameraRemove, "camera_remove"),
        ];

        for (notification_type, expected_str) in types {
            let json = serde_json::to_string(&notification_type).unwrap();
            assert_eq!(json, format!("\"{}\"", expected_str));

            let parsed: NotificationType = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, notification_type);
        }
    }

    #[test]
    fn test_config_update_notification_deserialization() {
        let json = r#"{
            "type": "config_update",
            "tid": "T2025120621041161827",
            "fids": ["0150", "0151"],
            "updatedAt": "2026-01-11T10:00:00Z",
            "actor": "user_123",
            "changedFields": ["report_interval_minutes", "grand_summary_times"]
        }"#;

        let notification: ConfigUpdateNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notification.notification_type, NotificationType::ConfigUpdate);
        assert_eq!(notification.tid, "T2025120621041161827");
        assert_eq!(notification.fids.len(), 2);
        assert_eq!(notification.actor, "user_123");
        assert_eq!(notification.changed_fields.len(), 2);
    }

    #[test]
    fn test_config_update_notification_minimal() {
        let json = r#"{
            "type": "force_sync",
            "tid": "T2025120621041161827",
            "fids": [],
            "updatedAt": "2026-01-11T10:00:00Z",
            "actor": "system"
        }"#;

        let notification: ConfigUpdateNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notification.notification_type, NotificationType::ForceSync);
        assert!(notification.fids.is_empty());
        assert!(notification.changed_fields.is_empty());
    }
}
