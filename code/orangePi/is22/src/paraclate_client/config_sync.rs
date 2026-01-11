//! Config Sync Service
//!
//! Phase 4: ParaclateClient (Issue #117)
//! DD03_ParaclateClient.md準拠
//!
//! ## 概要
//! mobes2.0 (Paraclate APP) から設定を同期
//! SSoTはmobes2.0側、is22はキャッシュとして保持
//!
//! ## 同期フロー
//! 1. mobes2.0から設定取得 (`GET /api/paraclate/config`)
//! 2. sync_source_timestamp比較
//! 3. 新しい場合のみローカル更新
//! 4. ScheduledReportsにも反映
//!
//! ## Phase 8拡張 (Issue #121)
//! - カメラ個別設定の同期 (`cameras` フィールド)
//! - CameraSyncRepositoryとの連携

use crate::camera_sync::{CameraParaclateSettings, CameraSyncRepository};
use crate::config_store::ConfigStore;
use crate::paraclate_client::{
    repository::ConfigRepository,
    types::*,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// 設定同期サービス
pub struct ConfigSyncService {
    http: reqwest::Client,
    config_repo: ConfigRepository,
    config_store: Arc<ConfigStore>,
    /// Phase 8: カメラ同期リポジトリ (Issue #121)
    camera_sync_repo: Option<CameraSyncRepository>,
}

impl ConfigSyncService {
    /// 新規作成
    pub fn new(pool: sqlx::MySqlPool, config_store: Arc<ConfigStore>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            config_repo: ConfigRepository::new(pool.clone()),
            config_store,
            camera_sync_repo: Some(CameraSyncRepository::new(pool)),
        }
    }

    /// CameraSyncRepositoryを設定（テスト用）
    pub fn with_camera_sync_repo(mut self, repo: CameraSyncRepository) -> Self {
        self.camera_sync_repo = Some(repo);
        self
    }

    /// 設定同期を実行
    ///
    /// mobes2.0からの設定取得と比較・更新
    pub async fn sync(&self, tid: &str, fid: &str) -> Result<SyncResult, ParaclateError> {
        // ローカル設定を取得
        let local_config = self
            .config_repo
            .get(tid, fid)
            .await
            .map_err(|e| ParaclateError::Database(format!("Failed to get config: {}", e)))?;

        let config = match local_config {
            Some(c) => c,
            None => {
                return Err(ParaclateError::Config(
                    "Config not found. Connect to Paraclate APP first.".to_string(),
                ));
            }
        };

        if config.connection_status != ConnectionStatus::Connected {
            return Err(ParaclateError::Offline(
                "Not connected to Paraclate APP".to_string(),
            ));
        }

        // LacisOathを取得
        let oath = self.get_lacis_oath(tid).await?;

        // mobes2.0から設定取得
        let url = format!("{}/api/paraclate/config", config.endpoint);
        let mut req = self.http.get(&url);
        for (key, value) in oath.to_headers() {
            req = req.header(&key, &value);
        }

        let response = req.send().await.map_err(|e| {
            ParaclateError::Http(format!("Failed to fetch config: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(ParaclateError::Http(format!(
                "Config sync failed: HTTP {}",
                status.as_u16()
            )));
        }

        let sync_response: MobesSyncResponse = response.json().await.map_err(|e| {
            ParaclateError::Http(format!("Failed to parse config response: {}", e))
        })?;

        if !sync_response.success {
            return Err(ParaclateError::Config(
                sync_response.error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let mobes_config = sync_response
            .config
            .ok_or_else(|| ParaclateError::Config("No config in response".to_string()))?;

        let mobes_updated_at = sync_response.updated_at;

        // タイムスタンプ比較
        let needs_update = match (config.sync_source_timestamp, mobes_updated_at) {
            (Some(local_ts), Some(remote_ts)) => remote_ts > local_ts,
            (None, Some(_)) => true,
            _ => false,
        };

        // Phase 8: カメラ個別設定を同期 (Issue #121)
        let synced_cameras = self.sync_camera_settings(&sync_response.cameras).await;

        if needs_update {
            // ローカル設定を更新
            let update = ParaclateConfigUpdate {
                endpoint: None,
                report_interval_minutes: Some(mobes_config.report_interval_minutes),
                grand_summary_times: Some(mobes_config.grand_summary_times.clone()),
                retention_days: Some(mobes_config.retention_days),
                attunement: Some(serde_json::to_value(&mobes_config.attunement).unwrap()),
                sync_source_timestamp: mobes_updated_at,
            };

            self.config_repo
                .update(tid, fid, update)
                .await
                .map_err(|e| ParaclateError::Database(format!("Failed to update config: {}", e)))?;

            info!(
                tid = %tid,
                fid = %fid,
                camera_settings_synced = synced_cameras,
                "Config synced from mobes2.0"
            );

            Ok(SyncResult {
                synced: true,
                updated_fields: vec![
                    "report_interval_minutes".to_string(),
                    "grand_summary_times".to_string(),
                    "retention_days".to_string(),
                    "attunement".to_string(),
                ],
                mobes_config: Some(mobes_config),
                synced_camera_count: synced_cameras,
            })
        } else {
            debug!(
                tid = %tid,
                fid = %fid,
                camera_settings_synced = synced_cameras,
                "Config already up to date"
            );

            Ok(SyncResult {
                synced: synced_cameras > 0,
                updated_fields: if synced_cameras > 0 {
                    vec!["camera_settings".to_string()]
                } else {
                    vec![]
                },
                mobes_config: None,
                synced_camera_count: synced_cameras,
            })
        }
    }

    /// カメラ個別設定を同期（Phase 8: Issue #121）
    ///
    /// mobes2.0からのカメラ設定をローカルDBに保存
    async fn sync_camera_settings(&self, camera_settings: &[MobesCameraSettings]) -> usize {
        let camera_sync_repo = match &self.camera_sync_repo {
            Some(repo) => repo,
            None => {
                warn!("CameraSyncRepository not configured, skipping camera settings sync");
                return 0;
            }
        };

        if camera_settings.is_empty() {
            debug!("No camera settings in response");
            return 0;
        }

        let mut synced_count = 0;

        for mobes_settings in camera_settings {
            // mobes2.0の設定をIS22のCameraParaclateSettingsに変換
            let settings = CameraParaclateSettings {
                sensitivity: mobes_settings.sensitivity,
                detection_zone: mobes_settings.detection_zone.clone(),
                alert_threshold: mobes_settings.alert_threshold,
                custom_preset: mobes_settings.custom_preset.clone(),
            };

            // camera_idを決定（mobesから送られてくる場合と、lacis_idから検索する場合がある）
            let camera_id = match &mobes_settings.camera_id {
                Some(id) => id.clone(),
                None => {
                    // lacis_idからcamera_idを取得（sync_stateテーブルから）
                    match camera_sync_repo.get_sync_state_by_lacis_id(&mobes_settings.lacis_id).await {
                        Ok(Some(state)) => state.camera_id,
                        Ok(None) => {
                            debug!(
                                lacis_id = %mobes_settings.lacis_id,
                                "Camera not found in sync state, skipping settings sync"
                            );
                            continue;
                        }
                        Err(e) => {
                            error!(
                                lacis_id = %mobes_settings.lacis_id,
                                error = %e,
                                "Failed to lookup camera by lacis_id"
                            );
                            continue;
                        }
                    }
                }
            };

            // 設定を保存
            match camera_sync_repo
                .upsert_paraclate_settings(&camera_id, &mobes_settings.lacis_id, &settings)
                .await
            {
                Ok(_) => {
                    debug!(
                        camera_id = %camera_id,
                        lacis_id = %mobes_settings.lacis_id,
                        "Synced camera paraclate settings"
                    );
                    synced_count += 1;
                }
                Err(e) => {
                    error!(
                        camera_id = %camera_id,
                        lacis_id = %mobes_settings.lacis_id,
                        error = %e,
                        "Failed to save camera paraclate settings"
                    );
                }
            }
        }

        if synced_count > 0 {
            info!(
                count = synced_count,
                total = camera_settings.len(),
                "Camera paraclate settings synced from mobes2.0"
            );
        }

        synced_count
    }

    /// 定期同期を開始（バックグラウンドタスク用）
    pub async fn start_periodic_sync(
        self: Arc<Self>,
        tid: String,
        fid: String,
        interval_secs: u64,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

        loop {
            interval.tick().await;

            match self.sync(&tid, &fid).await {
                Ok(result) => {
                    if result.synced {
                        info!(tid = %tid, fid = %fid, "Periodic config sync completed");
                    }
                }
                Err(e) => {
                    warn!(tid = %tid, fid = %fid, error = %e, "Periodic config sync failed");
                }
            }
        }
    }

    /// LacisOathを取得
    async fn get_lacis_oath(&self, tid: &str) -> Result<LacisOath, ParaclateError> {
        const LACIS_ID_KEY: &str = "aranea.lacis_id";
        const CIC_KEY: &str = "aranea.cic";

        let lacis_id = self
            .config_store
            .service()
            .get_setting(LACIS_ID_KEY)
            .await
            .map_err(|e| ParaclateError::Config(format!("Failed to get lacis_id: {}", e)))?
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| {
                ParaclateError::Auth("LacisID not configured. Run AraneaRegister first.".to_string())
            })?;

        let cic = self
            .config_store
            .service()
            .get_setting(CIC_KEY)
            .await
            .map_err(|e| ParaclateError::Config(format!("Failed to get cic: {}", e)))?
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| {
                ParaclateError::Auth("CIC not configured. Run AraneaRegister first.".to_string())
            })?;

        Ok(LacisOath {
            lacis_id,
            tid: tid.to_string(),
            cic,
            blessing: None,
        })
    }
}

/// 同期結果
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// 同期が行われたか
    pub synced: bool,
    /// 更新されたフィールド
    pub updated_fields: Vec<String>,
    /// mobes2.0からの設定（更新された場合）
    pub mobes_config: Option<MobesConfig>,
    /// Phase 8: 同期されたカメラ設定数 (Issue #121)
    pub synced_camera_count: usize,
}
