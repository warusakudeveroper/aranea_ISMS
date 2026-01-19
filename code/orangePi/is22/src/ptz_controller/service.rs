//! PTZ Controller Service
//!
//! ONVIF PTZ操作を管理するサービス

use super::tapo_ptz::TapoPtzClient;
use super::types::*;
use crate::config_store::ConfigStore;
use crate::error::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

/// アクティブなPTZ移動セッション
#[derive(Debug, Clone)]
struct ActiveMove {
    camera_id: String,
    direction: PtzDirection,
    started_at: std::time::Instant,
}

/// PTZコントローラーサービス
pub struct PtzService {
    config_store: Arc<ConfigStore>,
    /// カメラID -> アクティブな移動セッション
    active_moves: RwLock<HashMap<String, ActiveMove>>,
}

impl PtzService {
    /// 新規作成
    pub fn new(config_store: Arc<ConfigStore>) -> Self {
        Self {
            config_store,
            active_moves: RwLock::new(HashMap::new()),
        }
    }

    /// PTZ移動開始
    pub async fn move_ptz(
        &self,
        camera_id: &str,
        request: &PtzMoveRequest,
    ) -> Result<PtzResponse> {
        // カメラ情報取得
        let camera = self.config_store.service().get_camera(camera_id).await?
            .ok_or_else(|| Error::NotFound(format!("Camera {} not found", camera_id)))?;

        // PTZサポートチェック
        if !camera.ptz_supported {
            return Ok(PtzResponse::error("このカメラはPTZ非対応です"));
        }

        // PTZ無効化チェック
        if camera.ptz_disabled {
            return Ok(PtzResponse::error("このカメラのPTZ操作は無効化されています"));
        }

        // rotation適用（UI方向→実際の移動方向）
        let actual_direction = request.direction.apply_rotation(camera.rotation);

        // 速度の正規化 (0.0-1.0)
        let speed = request.speed.clamp(0.0, 1.0);

        // ONVIF PTZ操作実行
        let result = self.execute_ptz_move(&camera_id, actual_direction, speed, &camera).await;

        match result {
            Ok(_) => {
                // アクティブセッション登録
                {
                    let mut moves = self.active_moves.write().await;
                    moves.insert(camera_id.to_string(), ActiveMove {
                        camera_id: camera_id.to_string(),
                        direction: actual_direction,
                        started_at: std::time::Instant::now(),
                    });
                }

                // Nudgeモードの場合は自動停止
                if request.mode == PtzMode::Nudge {
                    let camera_id_clone = camera_id.to_string();
                    let duration = request.duration_ms;
                    let config_store = Arc::clone(&self.config_store);

                    tokio::spawn(async move {
                        sleep(Duration::from_millis(duration as u64)).await;
                        // 停止処理（config_storeを使用して実際のPTZ停止を実行）
                        if let Err(e) = Self::execute_ptz_stop_with_config(
                            &camera_id_clone,
                            &config_store,
                        ).await {
                            tracing::warn!(
                                camera_id = %camera_id_clone,
                                error = %e,
                                "Failed to auto-stop PTZ after nudge"
                            );
                        }
                    });
                }

                Ok(PtzResponse::success_with_message(format!(
                    "PTZ {} 移動中",
                    actual_direction.to_japanese()
                )))
            }
            Err(e) => Ok(PtzResponse::error(format!("PTZ操作失敗: {}", e))),
        }
    }

    /// PTZ停止
    pub async fn stop_ptz(&self, camera_id: &str) -> Result<PtzResponse> {
        // カメラ情報取得
        let camera = self.config_store.service().get_camera(camera_id).await?
            .ok_or_else(|| Error::NotFound(format!("Camera {} not found", camera_id)))?;

        if !camera.ptz_supported {
            return Ok(PtzResponse::error("このカメラはPTZ非対応です"));
        }

        // アクティブセッション削除
        {
            let mut moves = self.active_moves.write().await;
            moves.remove(camera_id);
        }

        // ONVIF PTZ停止実行
        let result = self.execute_ptz_stop(camera_id, &camera).await;

        match result {
            Ok(_) => Ok(PtzResponse::success_with_message("PTZ停止")),
            Err(e) => Ok(PtzResponse::error(format!("PTZ停止失敗: {}", e))),
        }
    }

    /// ホームポジション移動
    pub async fn go_home(&self, camera_id: &str) -> Result<PtzResponse> {
        // カメラ情報取得
        let camera = self.config_store.service().get_camera(camera_id).await?
            .ok_or_else(|| Error::NotFound(format!("Camera {} not found", camera_id)))?;

        if !camera.ptz_supported {
            return Ok(PtzResponse::error("このカメラはPTZ非対応です"));
        }

        if camera.ptz_disabled {
            return Ok(PtzResponse::error("このカメラのPTZ操作は無効化されています"));
        }

        if !camera.ptz_home_supported {
            return Ok(PtzResponse::error("このカメラはホームポジション非対応です"));
        }

        // ONVIF ホーム移動実行
        let result = self.execute_ptz_home(camera_id, &camera).await;

        match result {
            Ok(_) => Ok(PtzResponse::success_with_message("ホームポジションに移動中")),
            Err(e) => Ok(PtzResponse::error(format!("ホーム移動失敗: {}", e))),
        }
    }

    /// PTZステータス取得
    pub async fn get_status(&self, camera_id: &str) -> Result<PtzStatus> {
        let camera = self.config_store.service().get_camera(camera_id).await?
            .ok_or_else(|| Error::NotFound(format!("Camera {} not found", camera_id)))?;

        let active = {
            let moves = self.active_moves.read().await;
            moves.get(camera_id).cloned()
        };

        Ok(PtzStatus {
            camera_id: camera_id.to_string(),
            ptz_supported: camera.ptz_supported,
            ptz_disabled: camera.ptz_disabled,
            ptz_continuous: camera.ptz_continuous,
            ptz_absolute: camera.ptz_absolute,
            ptz_relative: camera.ptz_relative,
            ptz_home_supported: camera.ptz_home_supported,
            is_moving: active.is_some(),
            current_direction: active.map(|a| a.direction),
        })
    }

    /// ONVIF PTZ移動実行（内部）
    async fn execute_ptz_move(
        &self,
        camera_id: &str,
        direction: PtzDirection,
        speed: f32,
        camera: &crate::config_store::Camera,
    ) -> Result<()> {
        // ONVIF endpointが設定されていない場合はエラー
        let onvif_endpoint = camera.onvif_endpoint.as_ref()
            .ok_or_else(|| Error::Validation("ONVIFエンドポイント未設定".to_string()))?;

        // 認証情報取得
        let username = camera.rtsp_username.as_ref()
            .ok_or_else(|| Error::Validation("RTSPユーザー名未設定".to_string()))?;
        let password = camera.rtsp_password.as_ref()
            .ok_or_else(|| Error::Validation("RTSPパスワード未設定".to_string()))?;

        // Pan/Tilt速度計算
        let (pan_speed, tilt_speed) = match direction {
            PtzDirection::Up => (0.0, speed),
            PtzDirection::Down => (0.0, -speed),
            PtzDirection::Left => (-speed, 0.0),
            PtzDirection::Right => (speed, 0.0),
        };

        tracing::info!(
            camera_id = %camera_id,
            direction = ?direction,
            pan_speed = %pan_speed,
            tilt_speed = %tilt_speed,
            family = %camera.family,
            "Executing PTZ move"
        );

        // カメラファミリーに応じた制御
        match camera.family.to_lowercase().as_str() {
            "tapo" | "vigi" => {
                // Tapo/VIGI: 専用ONVIFクライアント使用
                let client = TapoPtzClient::new(onvif_endpoint, username, password);
                client.continuous_move(pan_speed, tilt_speed, 0.0).await?;
            }
            _ => {
                // 他メーカー: 未サポート（将来的にonvif-rsを使用予定）
                return Err(Error::Validation(format!(
                    "PTZ control not yet implemented for camera family: {}",
                    camera.family
                )));
            }
        }

        Ok(())
    }

    /// ONVIF PTZ停止実行
    async fn execute_ptz_stop(
        &self,
        camera_id: &str,
        camera: &crate::config_store::Camera,
    ) -> Result<()> {
        let onvif_endpoint = camera.onvif_endpoint.as_ref()
            .ok_or_else(|| Error::Validation("ONVIFエンドポイント未設定".to_string()))?;
        let username = camera.rtsp_username.as_ref()
            .ok_or_else(|| Error::Validation("RTSPユーザー名未設定".to_string()))?;
        let password = camera.rtsp_password.as_ref()
            .ok_or_else(|| Error::Validation("RTSPパスワード未設定".to_string()))?;

        tracing::info!(camera_id = %camera_id, "Executing PTZ stop");

        match camera.family.to_lowercase().as_str() {
            "tapo" | "vigi" => {
                let client = TapoPtzClient::new(onvif_endpoint, username, password);
                client.stop().await?;
            }
            _ => {
                return Err(Error::Validation(format!(
                    "PTZ control not yet implemented for camera family: {}",
                    camera.family
                )));
            }
        }

        Ok(())
    }

    /// ONVIF PTZ停止実行（Nudge自動停止用）
    ///
    /// config_storeを受け取り、実際のPTZ停止処理を実行する
    async fn execute_ptz_stop_with_config(
        camera_id: &str,
        config_store: &Arc<ConfigStore>,
    ) -> Result<()> {
        // カメラ情報取得
        let camera = config_store.service().get_camera(camera_id).await?
            .ok_or_else(|| Error::NotFound(format!("Camera {} not found", camera_id)))?;

        let onvif_endpoint = camera.onvif_endpoint.as_ref()
            .ok_or_else(|| Error::Validation("ONVIFエンドポイント未設定".to_string()))?;
        let username = camera.rtsp_username.as_ref()
            .ok_or_else(|| Error::Validation("RTSPユーザー名未設定".to_string()))?;
        let password = camera.rtsp_password.as_ref()
            .ok_or_else(|| Error::Validation("RTSPパスワード未設定".to_string()))?;

        tracing::info!(camera_id = %camera_id, "Executing PTZ auto-stop after nudge");

        match camera.family.to_lowercase().as_str() {
            "tapo" | "vigi" => {
                let client = TapoPtzClient::new(onvif_endpoint, username, password);
                client.stop().await?;
            }
            _ => {
                tracing::debug!(
                    camera_id = %camera_id,
                    family = %camera.family,
                    "PTZ stop not implemented for family, relying on camera timeout"
                );
            }
        }

        Ok(())
    }

    /// ONVIF ホーム移動実行（内部）
    async fn execute_ptz_home(
        &self,
        camera_id: &str,
        camera: &crate::config_store::Camera,
    ) -> Result<()> {
        let onvif_endpoint = camera.onvif_endpoint.as_ref()
            .ok_or_else(|| Error::Validation("ONVIFエンドポイント未設定".to_string()))?;
        let username = camera.rtsp_username.as_ref()
            .ok_or_else(|| Error::Validation("RTSPユーザー名未設定".to_string()))?;
        let password = camera.rtsp_password.as_ref()
            .ok_or_else(|| Error::Validation("RTSPパスワード未設定".to_string()))?;

        tracing::info!(camera_id = %camera_id, "Executing PTZ go home");

        match camera.family.to_lowercase().as_str() {
            "tapo" | "vigi" => {
                let client = TapoPtzClient::new(onvif_endpoint, username, password);
                client.goto_home().await?;
            }
            _ => {
                return Err(Error::Validation(format!(
                    "PTZ control not yet implemented for camera family: {}",
                    camera.family
                )));
            }
        }

        Ok(())
    }
}
