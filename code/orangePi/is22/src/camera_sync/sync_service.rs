//! Camera Sync Service
//!
//! Phase 8: カメラ双方向同期 (Issue #121)
//! T8-3, T8-4, T8-5, T8-9: 同期サービス

use super::repository::CameraSyncRepository;
use super::types::*;
use crate::config_store::ConfigStore;
use crate::paraclate_client::ParaclateClient;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// デフォルト同期間隔（秒）: 1時間
const DEFAULT_SYNC_INTERVAL_SECS: u64 = 3600;

/// 最小同期間隔（秒）: 5分
const MIN_SYNC_INTERVAL_SECS: u64 = 300;

/// 定期同期状態
#[derive(Debug, Clone, Default)]
pub struct PeriodicSyncState {
    /// 最終フル同期日時
    pub last_full_sync: Option<DateTime<Utc>>,
    /// 同期実行中フラグ
    pub is_running: bool,
    /// 次回同期予定時刻
    pub next_sync_at: Option<DateTime<Utc>>,
    /// 連続失敗回数
    pub consecutive_failures: u32,
    /// 最後のエラーメッセージ
    pub last_error: Option<String>,
}

/// Camera Sync Service
pub struct CameraSyncService {
    repository: CameraSyncRepository,
    config_store: Arc<ConfigStore>,
    paraclate_client: Option<Arc<ParaclateClient>>,
    /// T8-9: 定期同期状態
    periodic_sync_state: Arc<RwLock<PeriodicSyncState>>,
}

impl CameraSyncService {
    /// 新しいCameraSyncServiceを作成
    pub fn new(
        repository: CameraSyncRepository,
        config_store: Arc<ConfigStore>,
    ) -> Self {
        Self {
            repository,
            config_store,
            paraclate_client: None,
            periodic_sync_state: Arc::new(RwLock::new(PeriodicSyncState::default())),
        }
    }

    /// ParaclateClientを設定
    pub fn with_paraclate_client(mut self, client: Arc<ParaclateClient>) -> Self {
        self.paraclate_client = Some(client);
        self
    }

    // ========================================
    // T8-9: 定期同期スケジューラ
    // ========================================

    /// 定期同期を開始（バックグラウンドタスク）
    ///
    /// # Arguments
    /// * `tid` - テナントID
    /// * `fid` - 施設ID
    /// * `interval_secs` - 同期間隔（秒）。Noneの場合はデフォルト1時間
    pub async fn start_periodic_sync(
        self: Arc<Self>,
        tid: String,
        fid: String,
        interval_secs: Option<u64>,
    ) {
        let interval = interval_secs
            .map(|s| s.max(MIN_SYNC_INTERVAL_SECS))
            .unwrap_or(DEFAULT_SYNC_INTERVAL_SECS);

        info!(
            tid = %tid,
            fid = %fid,
            interval_secs = interval,
            "Starting periodic camera sync scheduler"
        );

        // 初回同期までの待機時間（起動直後は少し待つ）
        tokio::time::sleep(Duration::from_secs(30)).await;

        let mut ticker = tokio::time::interval(Duration::from_secs(interval));
        // 最初のtickはすぐに発火するのでスキップ
        ticker.tick().await;

        loop {
            ticker.tick().await;

            // 次回同期時刻を更新
            {
                let mut state = self.periodic_sync_state.write().await;
                state.next_sync_at = Some(Utc::now() + chrono::Duration::seconds(interval as i64));
            }

            // 同期実行
            self.execute_periodic_sync(&tid, &fid).await;
        }
    }

    /// 定期同期を実行
    async fn execute_periodic_sync(&self, tid: &str, fid: &str) {
        // 実行中フラグを設定
        {
            let mut state = self.periodic_sync_state.write().await;
            if state.is_running {
                warn!("Periodic sync already running, skipping");
                return;
            }
            state.is_running = true;
        }

        info!(tid = %tid, fid = %fid, "Executing periodic camera sync");

        let result = self.push_all_cameras(tid, fid).await;

        // 結果を記録
        {
            let mut state = self.periodic_sync_state.write().await;
            state.is_running = false;

            match result {
                Ok(response) => {
                    state.last_full_sync = Some(Utc::now());
                    state.consecutive_failures = 0;
                    state.last_error = None;

                    info!(
                        tid = %tid,
                        fid = %fid,
                        synced = response.synced_count,
                        failed = response.failed_count,
                        "Periodic camera sync completed successfully"
                    );
                }
                Err(e) => {
                    state.consecutive_failures += 1;
                    state.last_error = Some(e.to_string());

                    warn!(
                        tid = %tid,
                        fid = %fid,
                        error = %e,
                        consecutive_failures = state.consecutive_failures,
                        "Periodic camera sync failed"
                    );
                }
            }
        }
    }

    /// 定期同期状態を取得
    pub async fn get_periodic_sync_state(&self) -> PeriodicSyncState {
        self.periodic_sync_state.read().await.clone()
    }

    /// 手動でフル同期を実行（API用）
    pub async fn trigger_full_sync(&self, tid: &str, fid: &str) -> crate::Result<SyncResponse> {
        info!(tid = %tid, fid = %fid, "Triggering manual full camera sync");

        let result = self.push_all_cameras(tid, fid).await;

        // 成功した場合は状態を更新
        if let Ok(ref response) = result {
            if response.success {
                let mut state = self.periodic_sync_state.write().await;
                state.last_full_sync = Some(Utc::now());
            }
        }

        result
    }

    // ========================================
    // IS22 → mobes2.0 同期
    // ========================================

    /// 全カメラメタデータをmobes2.0へプッシュ
    pub async fn push_all_cameras(&self, tid: &str, fid: &str) -> crate::Result<SyncResponse> {
        info!(tid = %tid, fid = %fid, "Starting full camera sync to mobes2.0");

        // 登録済みカメラを取得
        let cameras = self.get_registered_cameras(tid).await?;

        if cameras.is_empty() {
            return Ok(SyncResponse {
                success: true,
                synced_count: 0,
                failed_count: 0,
                details: vec![],
            });
        }

        // メタデータエントリを構築
        let entries: Vec<CameraMetadataEntry> = cameras
            .iter()
            .map(|c| self.build_metadata_entry(c))
            .collect();

        // ペイロード構築
        let payload = CameraMetadataPayload {
            cameras: entries,
            sync_type: SyncType::Full,
            updated_at: Utc::now(),
        };

        // mobes2.0へ送信
        let result = self.send_metadata_to_mobes(tid, fid, &payload).await;

        // 結果に応じて各カメラの状態を更新
        let mut synced_count = 0;
        let mut failed_count = 0;
        let mut details = Vec::new();

        match &result {
            Ok(_) => {
                for camera in &cameras {
                    if let Err(e) = self.repository.mark_synced_to_mobes(&camera.camera_id).await {
                        error!(camera_id = %camera.camera_id, error = %e, "Failed to mark synced");
                        failed_count += 1;
                        details.push(SyncDetail {
                            camera_id: camera.camera_id.clone(),
                            lacis_id: camera.lacis_id.clone().unwrap_or_default(),
                            status: "failed".to_string(),
                            error: Some(e.to_string()),
                        });
                    } else {
                        synced_count += 1;
                        details.push(SyncDetail {
                            camera_id: camera.camera_id.clone(),
                            lacis_id: camera.lacis_id.clone().unwrap_or_default(),
                            status: "synced".to_string(),
                            error: None,
                        });
                    }
                }
            }
            Err(e) => {
                // 全カメラを失敗としてマーク
                for camera in &cameras {
                    let _ = self
                        .repository
                        .mark_sync_failed(&camera.camera_id, &e.to_string())
                        .await;
                    failed_count += 1;
                    details.push(SyncDetail {
                        camera_id: camera.camera_id.clone(),
                        lacis_id: camera.lacis_id.clone().unwrap_or_default(),
                        status: "failed".to_string(),
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        info!(
            synced_count = synced_count,
            failed_count = failed_count,
            "Full camera sync completed"
        );

        Ok(SyncResponse {
            success: failed_count == 0,
            synced_count,
            failed_count,
            details,
        })
    }

    /// 単一カメラのメタデータをmobes2.0へプッシュ
    pub async fn push_single_camera(
        &self,
        tid: &str,
        fid: &str,
        camera_id: &str,
    ) -> crate::Result<SyncResponse> {
        info!(
            tid = %tid,
            fid = %fid,
            camera_id = %camera_id,
            "Pushing single camera to mobes2.0"
        );

        let camera = self.get_camera_info(camera_id).await?;
        let entry = self.build_metadata_entry(&camera);

        let payload = CameraMetadataPayload {
            cameras: vec![entry],
            sync_type: SyncType::Single,
            updated_at: Utc::now(),
        };

        let result = self.send_metadata_to_mobes(tid, fid, &payload).await;

        let (status, error) = match result {
            Ok(_) => {
                self.repository.mark_synced_to_mobes(camera_id).await?;
                ("synced".to_string(), None)
            }
            Err(e) => {
                self.repository
                    .mark_sync_failed(camera_id, &e.to_string())
                    .await?;
                ("failed".to_string(), Some(e.to_string()))
            }
        };

        Ok(SyncResponse {
            success: error.is_none(),
            synced_count: if error.is_none() { 1 } else { 0 },
            failed_count: if error.is_some() { 1 } else { 0 },
            details: vec![SyncDetail {
                camera_id: camera_id.to_string(),
                lacis_id: camera.lacis_id.unwrap_or_default(),
                status,
                error,
            }],
        })
    }

    /// カメラ削除をmobes2.0へ通知
    pub async fn notify_camera_deleted(
        &self,
        tid: &str,
        fid: &str,
        camera_id: &str,
        lacis_id: &str,
        reason: DeleteReason,
    ) -> crate::Result<()> {
        info!(
            tid = %tid,
            fid = %fid,
            camera_id = %camera_id,
            lacis_id = %lacis_id,
            reason = ?reason,
            "Notifying camera deletion to mobes2.0"
        );

        let payload = CameraDeletedPayload {
            deleted_cameras: vec![CameraDeletedEntry {
                lacis_id: lacis_id.to_string(),
                camera_id: camera_id.to_string(),
                deleted_at: Utc::now(),
                reason,
            }],
            sync_type: SyncType::Partial,
            updated_at: Utc::now(),
        };

        // mobes2.0へ送信
        self.send_delete_notification_to_mobes(tid, fid, &payload)
            .await?;

        // ログ記録
        self.repository
            .log_sync(
                camera_id,
                Some(lacis_id),
                SyncDirection::ToMobes,
                "delete",
                "success",
                None,
                None,
                Some(&serde_json::to_value(&payload).unwrap_or_default()),
                None,
            )
            .await?;

        Ok(())
    }

    // ========================================
    // mobes2.0 → IS22 同期
    // ========================================

    /// mobes2.0からカメラ個別設定を同期
    pub async fn sync_camera_settings_from_mobes(
        &self,
        lacis_id: &str,
        settings: CameraParaclateSettings,
    ) -> crate::Result<()> {
        info!(lacis_id = %lacis_id, "Syncing camera settings from mobes2.0");

        // LacisIDからカメラIDを検索
        let camera_id = self.get_camera_id_by_lacis_id(lacis_id).await?;

        // 設定を保存
        self.repository
            .upsert_paraclate_settings(&camera_id, lacis_id, &settings)
            .await?;

        // 同期状態を更新
        let mut state = self
            .repository
            .get_sync_state(&camera_id)
            .await?
            .unwrap_or_else(|| CameraSyncState {
                id: None,
                camera_id: camera_id.clone(),
                lacis_id: lacis_id.to_string(),
                last_sync_to_mobes: None,
                last_sync_from_mobes: None,
                mobes_sync_status: MobesSyncStatus::Pending,
                sync_error_message: None,
                retry_count: 0,
                created_at: None,
                updated_at: None,
            });

        state.last_sync_from_mobes = Some(Utc::now());
        self.repository.upsert_sync_state(&state).await?;

        // ログ記録
        self.repository
            .log_sync(
                &camera_id,
                Some(lacis_id),
                SyncDirection::FromMobes,
                "settings",
                "success",
                Some(&serde_json::to_value(&settings).unwrap_or_default()),
                None,
                None,
                None,
            )
            .await?;

        Ok(())
    }

    /// mobes2.0からのカメラ削除指示を処理
    pub async fn handle_camera_remove_from_mobes(
        &self,
        lacis_id: &str,
        reason: &str,
    ) -> crate::Result<()> {
        warn!(
            lacis_id = %lacis_id,
            reason = %reason,
            "Processing camera removal from mobes2.0"
        );

        // LacisIDからカメラIDを検索
        let camera_id = self.get_camera_id_by_lacis_id(lacis_id).await?;

        // カメラを論理削除
        self.mark_camera_deleted(&camera_id).await?;

        // 同期状態を更新
        let mut state = self
            .repository
            .get_sync_state(&camera_id)
            .await?
            .unwrap_or_else(|| CameraSyncState {
                id: None,
                camera_id: camera_id.clone(),
                lacis_id: lacis_id.to_string(),
                last_sync_to_mobes: None,
                last_sync_from_mobes: None,
                mobes_sync_status: MobesSyncStatus::Pending,
                sync_error_message: None,
                retry_count: 0,
                created_at: None,
                updated_at: None,
            });

        state.mobes_sync_status = MobesSyncStatus::Deleted;
        state.last_sync_from_mobes = Some(Utc::now());
        self.repository.upsert_sync_state(&state).await?;

        // ログ記録
        self.repository
            .log_sync(
                &camera_id,
                Some(lacis_id),
                SyncDirection::FromMobes,
                "delete",
                "success",
                Some(&serde_json::json!({ "reason": reason })),
                None,
                None,
                None,
            )
            .await?;

        Ok(())
    }

    // ========================================
    // 状態取得
    // ========================================

    /// 同期状態を取得
    pub async fn get_sync_status(&self) -> crate::Result<SyncStatusResponse> {
        let counts = self.repository.get_sync_counts().await?;

        // 全カメラの同期状態を取得（TODO: ページネーション）
        let cameras = self.get_all_sync_states().await?;

        // 定期同期状態から最終フル同期日時を取得
        let periodic_state = self.periodic_sync_state.read().await;

        Ok(SyncStatusResponse {
            last_full_sync: periodic_state.last_full_sync,
            pending_count: counts.pending,
            synced_count: counts.synced,
            failed_count: counts.failed,
            cameras,
        })
    }

    /// カメラ個別の同期状態を取得
    pub async fn get_camera_sync_state(&self, camera_id: &str) -> crate::Result<Option<CameraSyncStatusEntry>> {
        let state = self.repository.get_sync_state(camera_id).await?;
        Ok(state.map(|s| CameraSyncStatusEntry {
            camera_id: s.camera_id,
            lacis_id: s.lacis_id,
            mobes_sync_status: s.mobes_sync_status,
            last_sync_to_mobes: s.last_sync_to_mobes,
            last_sync_from_mobes: s.last_sync_from_mobes,
        }))
    }

    // ========================================
    // 内部ヘルパー
    // ========================================

    /// 登録済みカメラ一覧を取得
    async fn get_registered_cameras(&self, tid: &str) -> crate::Result<Vec<CameraInfo>> {
        let rows = sqlx::query_as::<_, CameraInfoRow>(
            r#"
            SELECT
                c.camera_id, c.lacis_id, c.name, c.location, c.mac_address,
                c.camera_context, c.is_online, c.last_seen,
                cb.name as brand_name, c.onvif_device_info
            FROM cameras c
            LEFT JOIN oui_entries oe ON UPPER(SUBSTRING(c.mac_address, 1, 8)) = oe.oui_prefix
            LEFT JOIN camera_brands cb ON oe.brand_id = cb.id
            WHERE c.tid = ? AND c.registration_state = 'registered'
            AND c.lacis_id IS NOT NULL AND c.deleted_at IS NULL
            "#,
        )
        .bind(tid)
        .fetch_all(self.repository.pool())
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 単一カメラ情報を取得
    async fn get_camera_info(&self, camera_id: &str) -> crate::Result<CameraInfo> {
        let row = sqlx::query_as::<_, CameraInfoRow>(
            r#"
            SELECT
                c.camera_id, c.lacis_id, c.name, c.location, c.mac_address,
                c.camera_context, c.is_online, c.last_seen,
                cb.name as brand_name, c.onvif_device_info
            FROM cameras c
            LEFT JOIN oui_entries oe ON UPPER(SUBSTRING(c.mac_address, 1, 8)) = oe.oui_prefix
            LEFT JOIN camera_brands cb ON oe.brand_id = cb.id
            WHERE c.camera_id = ?
            "#,
        )
        .bind(camera_id)
        .fetch_optional(self.repository.pool())
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?
        .ok_or_else(|| crate::Error::NotFound(format!("Camera {} not found", camera_id)))?;

        Ok(row.into())
    }

    /// LacisIDからカメラIDを取得
    async fn get_camera_id_by_lacis_id(&self, lacis_id: &str) -> crate::Result<String> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT camera_id FROM cameras WHERE lacis_id = ?",
        )
        .bind(lacis_id)
        .fetch_optional(self.repository.pool())
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        row.map(|r| r.0)
            .ok_or_else(|| crate::Error::NotFound(format!("Camera with LacisID {} not found", lacis_id)))
    }

    /// カメラを論理削除
    async fn mark_camera_deleted(&self, camera_id: &str) -> crate::Result<()> {
        sqlx::query(
            "UPDATE cameras SET deleted_at = NOW(3) WHERE camera_id = ?",
        )
        .bind(camera_id)
        .execute(self.repository.pool())
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(())
    }

    /// 全同期状態を取得
    async fn get_all_sync_states(&self) -> crate::Result<Vec<CameraSyncStatusEntry>> {
        let rows = sqlx::query_as::<_, SyncStateEntryRow>(
            r#"
            SELECT camera_id, lacis_id, mobes_sync_status,
                   last_sync_to_mobes, last_sync_from_mobes
            FROM camera_sync_state
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(self.repository.pool())
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// メタデータエントリを構築
    fn build_metadata_entry(&self, camera: &CameraInfo) -> CameraMetadataEntry {
        let context = camera
            .camera_context
            .as_ref()
            .and_then(|s| serde_json::from_str::<CameraContextData>(s).ok())
            .unwrap_or_default();

        let status = if camera.is_online.unwrap_or(false) {
            CameraStatus::Online
        } else {
            CameraStatus::Offline
        };

        CameraMetadataEntry {
            lacis_id: camera.lacis_id.clone().unwrap_or_default(),
            camera_id: camera.camera_id.clone(),
            name: camera.name.clone(),
            location: camera.location.clone(),
            context,
            mac: camera.mac_address.clone().unwrap_or_default(),
            brand: camera.brand_name.clone(),
            model: camera.model.clone(),
            status,
            last_seen: camera.last_seen,
        }
    }

    /// mobes2.0へメタデータを送信
    async fn send_metadata_to_mobes(
        &self,
        tid: &str,
        fid: &str,
        payload: &CameraMetadataPayload,
    ) -> crate::Result<()> {
        let client = match &self.paraclate_client {
            Some(c) => c,
            None => {
                warn!("ParaclateClient not configured, skipping mobes sync");
                return Ok(());
            }
        };

        // mobes2.0 paraclateCameraMetadata APIを呼び出し
        match client.send_camera_metadata(tid, fid, payload).await {
            Ok(response) => {
                if response.success {
                    info!(
                        tid = %tid,
                        fid = %fid,
                        synced_count = response.synced_count,
                        "Camera metadata sent to mobes2.0 successfully"
                    );
                    Ok(())
                } else {
                    let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                    warn!(
                        tid = %tid,
                        fid = %fid,
                        error = %error_msg,
                        "Camera metadata send failed"
                    );
                    Err(crate::Error::Api(format!("mobes2.0 API error: {}", error_msg)))
                }
            }
            Err(e) => {
                error!(tid = %tid, fid = %fid, error = %e, "Failed to send camera metadata");
                Err(crate::Error::Api(e.to_string()))
            }
        }
    }

    /// mobes2.0へ削除通知を送信
    async fn send_delete_notification_to_mobes(
        &self,
        tid: &str,
        fid: &str,
        payload: &CameraDeletedPayload,
    ) -> crate::Result<()> {
        let client = match &self.paraclate_client {
            Some(c) => c,
            None => {
                warn!("ParaclateClient not configured, skipping delete notification");
                return Ok(());
            }
        };

        // mobes2.0へ削除通知を送信
        match client.send_camera_deleted(tid, fid, payload).await {
            Ok(response) => {
                if response.success {
                    info!(
                        tid = %tid,
                        fid = %fid,
                        deleted_count = payload.deleted_cameras.len(),
                        "Camera deletion notification sent to mobes2.0 successfully"
                    );
                    Ok(())
                } else {
                    let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                    warn!(
                        tid = %tid,
                        fid = %fid,
                        error = %error_msg,
                        "Camera deletion notification failed"
                    );
                    Err(crate::Error::Api(format!("mobes2.0 API error: {}", error_msg)))
                }
            }
            Err(e) => {
                error!(tid = %tid, fid = %fid, error = %e, "Failed to send deletion notification");
                Err(crate::Error::Api(e.to_string()))
            }
        }
    }
}

// ========================================
// Row Types
// ========================================

#[derive(sqlx::FromRow)]
struct CameraInfoRow {
    camera_id: String,
    lacis_id: Option<String>,
    name: String,
    location: Option<String>,
    mac_address: Option<String>,
    camera_context: Option<String>,
    is_online: Option<bool>,
    last_seen: Option<chrono::NaiveDateTime>,
    brand_name: Option<String>,
    onvif_device_info: Option<String>,
}

struct CameraInfo {
    camera_id: String,
    lacis_id: Option<String>,
    name: String,
    location: Option<String>,
    mac_address: Option<String>,
    camera_context: Option<String>,
    is_online: Option<bool>,
    last_seen: Option<chrono::DateTime<chrono::Utc>>,
    brand_name: Option<String>,
    model: Option<String>,
}

impl From<CameraInfoRow> for CameraInfo {
    fn from(row: CameraInfoRow) -> Self {
        // onvif_device_infoからモデル名を抽出
        let model = row.onvif_device_info.as_ref().and_then(|info| {
            serde_json::from_str::<serde_json::Value>(info)
                .ok()
                .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(String::from))
        });

        Self {
            camera_id: row.camera_id,
            lacis_id: row.lacis_id,
            name: row.name,
            location: row.location,
            mac_address: row.mac_address,
            camera_context: row.camera_context,
            is_online: row.is_online,
            last_seen: row.last_seen.map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc)),
            brand_name: row.brand_name,
            model,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SyncStateEntryRow {
    camera_id: String,
    lacis_id: String,
    mobes_sync_status: String,
    last_sync_to_mobes: Option<chrono::NaiveDateTime>,
    last_sync_from_mobes: Option<chrono::NaiveDateTime>,
}

impl From<SyncStateEntryRow> for CameraSyncStatusEntry {
    fn from(row: SyncStateEntryRow) -> Self {
        let status = match row.mobes_sync_status.as_str() {
            "synced" => MobesSyncStatus::Synced,
            "pending" => MobesSyncStatus::Pending,
            "failed" => MobesSyncStatus::Failed,
            "deleted" => MobesSyncStatus::Deleted,
            _ => MobesSyncStatus::Pending,
        };

        Self {
            camera_id: row.camera_id,
            lacis_id: row.lacis_id,
            mobes_sync_status: status,
            last_sync_to_mobes: row.last_sync_to_mobes.map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc)),
            last_sync_from_mobes: row.last_sync_from_mobes.map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc)),
        }
    }
}
