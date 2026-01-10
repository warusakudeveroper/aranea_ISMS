//! IpcamScan Connector for CameraRegistry
//!
//! Phase 2: CameraRegistry (Issue #115)
//! T2-3: RTSP管理連携
//!
//! ## 概要
//! ipcam_scanモジュールとcamera_registryの連携を担当。
//! 承認済みカメラのみParaclate登録を許可する。
//!
//! ## カメラステータスフロー
//! ```text
//! [ipcam_scan]                  [camera_registry]
//! Discovered
//!     ↓ (verification)
//! Verified
//!     ↓ (approve_device)
//! Approved ─────────────────→ register_camera()
//!     ↓ (credentials set)            ↓
//! pending_auth                  Registered (with CIC)
//!     ↓ (activate_camera)
//! Active
//! ```

use super::service::CameraRegistryService;
use super::types::CameraRegisterRequest;
use crate::config_store::ConfigStore;
use sqlx::MySqlPool;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// IpcamScan連携コネクター
pub struct IpcamConnector {
    pool: MySqlPool,
    config_store: Arc<ConfigStore>,
}

impl IpcamConnector {
    /// 新しいIpcamConnectorを作成
    pub fn new(pool: MySqlPool, config_store: Arc<ConfigStore>) -> Self {
        Self { pool, config_store }
    }

    /// カメラが登録可能かチェック
    ///
    /// ## 登録条件
    /// 1. camerasテーブルに存在
    /// 2. ipcam_scan側で承認済み（approved）またはアクティブ
    /// 3. MACアドレスが設定済み
    pub async fn can_register(&self, camera_id: &str) -> crate::Result<bool> {
        let row: Option<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT c.status, c.mac_address
            FROM cameras c
            WHERE c.camera_id = ? AND c.deleted_at IS NULL
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        match row {
            Some((status, mac)) => {
                let has_mac = mac.is_some();
                let valid_status = matches!(
                    status.as_str(),
                    "active" | "pending_auth" | "approved"
                );

                if !has_mac {
                    debug!(camera_id = %camera_id, "Camera has no MAC address");
                    return Ok(false);
                }

                if !valid_status {
                    debug!(
                        camera_id = %camera_id,
                        status = %status,
                        "Camera status not valid for registration"
                    );
                    return Ok(false);
                }

                Ok(true)
            }
            None => {
                debug!(camera_id = %camera_id, "Camera not found");
                Ok(false)
            }
        }
    }

    /// 承認済みカメラを登録
    ///
    /// ## 前提条件
    /// - can_register()がtrueを返すこと
    pub async fn register_approved_camera(
        &self,
        service: &CameraRegistryService,
        camera_id: &str,
    ) -> crate::Result<Option<super::types::CameraRegisterResponse>> {
        // 登録可能性チェック
        if !self.can_register(camera_id).await? {
            warn!(camera_id = %camera_id, "Camera not eligible for Paraclate registration");
            return Ok(None);
        }

        // カメラ情報取得
        let camera_info = self.get_camera_info(camera_id).await?;
        let Some(info) = camera_info else {
            return Ok(None);
        };

        info!(
            camera_id = %camera_id,
            mac = %info.mac_address,
            "Registering approved camera to Paraclate"
        );

        // 登録リクエスト作成
        let request = CameraRegisterRequest {
            camera_id: camera_id.to_string(),
            mac_address: info.mac_address,
            product_code: info.product_code.unwrap_or_else(|| "0000".to_string()),
            fid: info.fid,
            rid: None, // TODO: rid対応
        };

        // 登録実行
        let result = service.register_camera(request).await?;
        Ok(Some(result))
    }

    /// カメラ情報取得
    async fn get_camera_info(&self, camera_id: &str) -> crate::Result<Option<CameraInfo>> {
        let row = sqlx::query_as::<_, CameraInfoRow>(
            r#"
            SELECT
                c.camera_id,
                c.mac_address,
                cb.product_code,
                c.fid,
                c.status
            FROM cameras c
            LEFT JOIN oui_entries oe ON UPPER(SUBSTRING(c.mac_address, 1, 8)) = oe.oui_prefix
            LEFT JOIN camera_brands cb ON oe.brand_id = cb.id
            WHERE c.camera_id = ? AND c.deleted_at IS NULL
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    /// ipcamscan_devicesステータスとカメラ登録状態を同期
    ///
    /// ## 処理内容
    /// - ipcamscan_devicesがapprovedなのにcamerasが未登録の場合、登録を試行
    pub async fn sync_registration_status(
        &self,
        service: &CameraRegistryService,
    ) -> crate::Result<SyncResult> {
        let mut registered = 0;
        let mut failed = 0;
        let mut skipped = 0;

        // 承認済みで未登録のカメラを取得
        let unregistered = self.get_approved_unregistered_cameras().await?;

        for camera_id in unregistered {
            match self.register_approved_camera(service, &camera_id).await {
                Ok(Some(resp)) if resp.ok => {
                    registered += 1;
                    info!(camera_id = %camera_id, "Auto-registered camera");
                }
                Ok(Some(_)) => {
                    failed += 1;
                    warn!(camera_id = %camera_id, "Failed to auto-register camera");
                }
                Ok(None) => {
                    skipped += 1;
                }
                Err(e) => {
                    failed += 1;
                    warn!(camera_id = %camera_id, error = %e, "Error auto-registering camera");
                }
            }
        }

        Ok(SyncResult {
            registered,
            failed,
            skipped,
        })
    }

    /// 承認済みで未登録のカメラ一覧を取得
    async fn get_approved_unregistered_cameras(&self) -> crate::Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT c.camera_id
            FROM cameras c
            WHERE c.status IN ('active', 'pending_auth')
            AND c.mac_address IS NOT NULL
            AND (c.registration_state IS NULL OR c.registration_state = 'pending')
            AND c.deleted_at IS NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// RTSPステータスとの同期チェック
    ///
    /// ## チェック内容
    /// - 登録済みカメラがオフラインの場合の警告
    /// - オンラインなのに未登録の場合の通知
    pub async fn check_rtsp_sync(&self) -> crate::Result<RtspSyncStatus> {
        // 登録済みでオフラインのカメラ
        let offline_registered: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT c.camera_id
            FROM cameras c
            WHERE c.registration_state = 'registered'
            AND c.status = 'offline'
            AND c.deleted_at IS NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        // オンラインで未登録のカメラ
        let online_unregistered: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT c.camera_id
            FROM cameras c
            WHERE c.status IN ('active', 'pending_auth')
            AND c.mac_address IS NOT NULL
            AND (c.registration_state IS NULL OR c.registration_state = 'pending')
            AND c.deleted_at IS NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(RtspSyncStatus {
            offline_registered: offline_registered.into_iter().map(|(id,)| id).collect(),
            online_unregistered: online_unregistered.into_iter().map(|(id,)| id).collect(),
        })
    }
}

/// カメラ情報
#[derive(Debug)]
struct CameraInfo {
    camera_id: String,
    mac_address: String,
    product_code: Option<String>,
    fid: Option<String>,
    status: String,
}

#[derive(sqlx::FromRow)]
struct CameraInfoRow {
    camera_id: String,
    mac_address: Option<String>,
    product_code: Option<String>,
    fid: Option<String>,
    status: String,
}

impl From<CameraInfoRow> for CameraInfo {
    fn from(row: CameraInfoRow) -> Self {
        Self {
            camera_id: row.camera_id,
            mac_address: row.mac_address.unwrap_or_default(),
            product_code: row.product_code,
            fid: row.fid,
            status: row.status,
        }
    }
}

/// 同期結果
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncResult {
    pub registered: usize,
    pub failed: usize,
    pub skipped: usize,
}

/// RTSPステータス同期状況
#[derive(Debug, Clone, serde::Serialize)]
pub struct RtspSyncStatus {
    /// 登録済みだがオフラインのカメラID
    pub offline_registered: Vec<String>,
    /// オンラインだが未登録のカメラID
    pub online_unregistered: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_result_default() {
        let result = SyncResult {
            registered: 0,
            failed: 0,
            skipped: 0,
        };
        assert_eq!(result.registered, 0);
    }
}
