//! LostCamTracker - DHCP追随によるカメラ自動復旧機能
//!
//! ## 設計原則: カメラに負荷をかけない
//!
//! Tapo等の低処理性能カメラはONVIF/RTSP認証試行で一時ロストするため、
//! このモジュールは**ARPスキャンのみ**を使用する。
//!
//! ### 禁止事項
//! - ONVIF Probe送信
//! - RTSP認証試行
//! - ポートスキャン
//! - クレデンシャル総当たり
//!
//! ### ARPスキャンがカメラに負荷をかけない理由
//! - ARPはOSI Layer 2（データリンク層）で動作
//! - カメラのアプリケーション層に到達しない
//! - カメラのCPU/メモリを消費しない

mod arp_scanner;
mod types;

pub use arp_scanner::{ArpScanner, ArpEntry};
pub use types::*;

use crate::config_store::ConfigStore;
use crate::event_log_service::{DetectionEvent, EventLogService};
use crate::Error;
use chrono::{DateTime, Utc};
use sqlx::{MySql, Pool};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// LostCamTracker設定
#[derive(Debug, Clone)]
pub struct LostCamTrackerConfig {
    /// 機能有効/無効
    pub enabled: bool,
    /// カメラ異常からARPスキャンまでの閾値（分）
    pub threshold_minutes: u32,
    /// 見つからない場合の再試行間隔（分）
    pub retry_minutes: u32,
}

impl Default for LostCamTrackerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_minutes: 30,
            retry_minutes: 60,
        }
    }
}

/// ロストカメラ情報
#[derive(Debug, Clone)]
pub struct LostCamera {
    pub camera_id: String,
    pub name: String,
    pub ip_address: String,
    pub lacis_id: Option<String>,
    pub last_healthy_at: Option<DateTime<Utc>>,
    pub error_duration_minutes: i64,
    pub arp_scan_attempted_at: Option<DateTime<Utc>>,
    pub arp_scan_result: Option<String>,
    pub subnet_type: SubnetType,
    pub auto_track_supported: bool,
}

/// サブネット種別
#[derive(Debug, Clone, PartialEq)]
pub enum SubnetType {
    /// ローカルサブネット（is22から直接ARPスキャン可能）
    Local,
    /// ARPスキャナデバイス管理サブネット（is20s等経由）
    ArpScannerDevice,
    /// リモート/VPN越し（ARP不可、手動スキャン必須）
    Remote,
}

/// LostCamTrackerサービス
pub struct LostCamTrackerService {
    pool: Pool<MySql>,
    config_store: Arc<ConfigStore>,
    event_log_service: Arc<EventLogService>,
    arp_scanner: ArpScanner,
    config: RwLock<LostCamTrackerConfig>,
    /// is22のローカルサブネット（ARPスキャン可能）
    local_subnets: Vec<String>,
}

impl LostCamTrackerService {
    /// 新しいLostCamTrackerServiceを作成
    pub fn new(
        pool: Pool<MySql>,
        config_store: Arc<ConfigStore>,
        event_log_service: Arc<EventLogService>,
        local_subnets: Vec<String>,
    ) -> Self {
        Self {
            pool,
            config_store,
            event_log_service,
            arp_scanner: ArpScanner::new(),
            config: RwLock::new(LostCamTrackerConfig::default()),
            local_subnets,
        }
    }

    /// 設定をDBから読み込み
    pub async fn load_config(&self) -> Result<(), Error> {
        // settings テーブルから読み込み (setting_key='lost_cam_tracker', setting_json=JSON)
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT setting_json FROM settings WHERE setting_key = 'lost_cam_tracker'"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Internal(format!("Failed to load config: {}", e)))?;

        if let Some((setting_json,)) = row {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&setting_json) {
                let mut config = self.config.write().await;
                if let Some(enabled) = json.get("enabled").and_then(|v| v.as_bool()) {
                    config.enabled = enabled;
                }
                if let Some(threshold) = json.get("threshold_minutes").and_then(|v| v.as_u64()) {
                    config.threshold_minutes = threshold as u32;
                }
                if let Some(retry) = json.get("retry_minutes").and_then(|v| v.as_u64()) {
                    config.retry_minutes = retry as u32;
                }

                tracing::info!(
                    enabled = config.enabled,
                    threshold_minutes = config.threshold_minutes,
                    retry_minutes = config.retry_minutes,
                    "LostCamTracker config loaded from settings table"
                );
            }
        } else {
            tracing::info!(
                "LostCamTracker settings not found in DB, using defaults"
            );
        }

        Ok(())
    }

    /// ロストカメラを検出
    pub async fn detect_lost_cameras(&self) -> Result<Vec<LostCamera>, Error> {
        let config = self.config.read().await;
        if !config.enabled {
            return Ok(vec![]);
        }

        let threshold_minutes = config.threshold_minutes as i64;

        // last_snapshot_atが閾値より古いカメラを検出
        #[derive(sqlx::FromRow)]
        struct CameraRow {
            camera_id: String,
            name: String,
            ip_address: String,
            lacis_id: Option<String>,
            last_healthy_at: Option<DateTime<Utc>>,
            arp_scan_attempted_at: Option<DateTime<Utc>>,
            arp_scan_result: Option<String>,
        }

        let cameras: Vec<CameraRow> = sqlx::query_as(
            r#"
            SELECT
                c.camera_id,
                c.name,
                c.ip_address,
                c.lacis_id,
                c.last_healthy_at,
                c.arp_scan_attempted_at,
                c.arp_scan_result
            FROM cameras c
            WHERE c.enabled = 1
              AND (
                c.last_healthy_at IS NULL
                OR c.last_healthy_at < DATE_SUB(NOW(), INTERVAL ? MINUTE)
              )
              AND c.lacis_id IS NOT NULL
              AND c.lacis_id != ''
            "#
        )
        .bind(threshold_minutes)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Internal(format!("Failed to detect lost cameras: {}", e)))?;

        let mut lost_cameras = Vec::new();
        for cam in cameras {
            let subnet_type = self.determine_subnet_type(&cam.ip_address).await;
            let auto_track_supported = subnet_type != SubnetType::Remote;

            let error_duration = cam.last_healthy_at
                .map(|t| (Utc::now() - t).num_minutes())
                .unwrap_or(i64::MAX);

            lost_cameras.push(LostCamera {
                camera_id: cam.camera_id,
                name: cam.name,
                ip_address: cam.ip_address,
                lacis_id: cam.lacis_id,
                last_healthy_at: cam.last_healthy_at,
                error_duration_minutes: error_duration,
                arp_scan_attempted_at: cam.arp_scan_attempted_at,
                arp_scan_result: cam.arp_scan_result,
                subnet_type,
                auto_track_supported,
            });
        }

        Ok(lost_cameras)
    }

    /// サブネット種別を判定
    async fn determine_subnet_type(&self, ip_address: &str) -> SubnetType {
        // IPアドレスからサブネットを抽出
        let parts: Vec<&str> = ip_address.split('.').collect();
        if parts.len() < 3 {
            return SubnetType::Remote;
        }
        let subnet_prefix = format!("{}.{}.{}", parts[0], parts[1], parts[2]);

        // ローカルサブネットか確認
        for local_subnet in &self.local_subnets {
            if local_subnet.starts_with(&subnet_prefix) {
                return SubnetType::Local;
            }
        }

        // ARPスキャナデバイスが管理するサブネットか確認
        let scanner_subnets: Vec<String> = sqlx::query_scalar(
            "SELECT subnets FROM arp_scanner_devices WHERE enabled = 1 AND subnets IS NOT NULL"
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for subnets_json in scanner_subnets {
            if let Ok(subnets) = serde_json::from_str::<Vec<String>>(&subnets_json) {
                for subnet in subnets {
                    if subnet.starts_with(&subnet_prefix) {
                        return SubnetType::ArpScannerDevice;
                    }
                }
            }
        }

        SubnetType::Remote
    }

    /// ARPスキャンを実行してIP追跡
    pub async fn track_lost_camera(&self, camera: &LostCamera) -> Result<Option<IpRelocationResult>, Error> {
        if !camera.auto_track_supported {
            tracing::debug!(
                camera_id = %camera.camera_id,
                subnet_type = ?camera.subnet_type,
                "Skipping ARP scan for non-local subnet"
            );
            return Ok(None);
        }

        // 再スキャン抑制チェック
        let config = self.config.read().await;
        if let Some(attempted_at) = camera.arp_scan_attempted_at {
            let minutes_since_scan = (Utc::now() - attempted_at).num_minutes();
            if minutes_since_scan < config.retry_minutes as i64 {
                if camera.arp_scan_result.as_deref() == Some("not_found") {
                    tracing::debug!(
                        camera_id = %camera.camera_id,
                        minutes_since_scan = minutes_since_scan,
                        "Skipping ARP scan (retry interval not elapsed)"
                    );
                    return Ok(None);
                }
            }
        }

        // lacis_idからMACを抽出
        let mac = match &camera.lacis_id {
            Some(lacis_id) if lacis_id.len() >= 16 => {
                lacis_id[4..16].to_uppercase()
            }
            _ => {
                tracing::warn!(
                    camera_id = %camera.camera_id,
                    "Cannot extract MAC from lacis_id"
                );
                return Ok(None);
            }
        };

        // ARPスキャン実行
        let subnet = format!("{}.0/24", {
            let parts: Vec<&str> = camera.ip_address.split('.').collect();
            format!("{}.{}.{}", parts[0], parts[1], parts[2])
        });

        tracing::info!(
            camera_id = %camera.camera_id,
            camera_name = %camera.name,
            subnet = %subnet,
            mac = %mac,
            "Starting ARP scan for lost camera"
        );

        let arp_result = match camera.subnet_type {
            SubnetType::Local => {
                self.arp_scanner.scan_subnet(&subnet).await?
            }
            SubnetType::ArpScannerDevice => {
                self.scan_via_arp_scanner_device(&subnet).await?
            }
            SubnetType::Remote => {
                return Ok(None);
            }
        };

        // MAC照合
        let new_ip = arp_result.iter()
            .find(|entry| entry.mac == mac)
            .map(|entry| entry.ip.clone());

        // 結果を記録
        let scan_result = if new_ip.is_some() && new_ip.as_ref() != Some(&camera.ip_address) {
            "found_updated"
        } else if new_ip.is_some() {
            "found_same_ip"
        } else {
            "not_found"
        };

        self.update_arp_scan_status(&camera.camera_id, scan_result).await?;

        // IPが変わっていれば更新
        if let Some(ref new_ip) = new_ip {
            if new_ip != &camera.ip_address {
                self.relocate_camera_ip(camera, new_ip).await?;

                return Ok(Some(IpRelocationResult {
                    camera_id: camera.camera_id.clone(),
                    camera_name: camera.name.clone(),
                    old_ip: camera.ip_address.clone(),
                    new_ip: new_ip.clone(),
                    detection_method: match camera.subnet_type {
                        SubnetType::Local => "arp_scan".to_string(),
                        SubnetType::ArpScannerDevice => "arp_scanner_device".to_string(),
                        SubnetType::Remote => "manual".to_string(),
                    },
                }));
            }
        }

        Ok(None)
    }

    /// ARPスキャナデバイス経由でスキャン
    async fn scan_via_arp_scanner_device(&self, subnet: &str) -> Result<Vec<ArpEntry>, Error> {
        // ARPスキャナデバイスからARP結果を取得
        #[derive(sqlx::FromRow)]
        struct ScannerDevice {
            api_url: String,
        }

        let devices: Vec<ScannerDevice> = sqlx::query_as(
            "SELECT api_url FROM arp_scanner_devices WHERE enabled = 1"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Internal(format!("Failed to get scanner devices: {}", e)))?;

        for device in devices {
            match self.fetch_arp_from_device(&device.api_url).await {
                Ok(entries) => {
                    // サブネットでフィルタ
                    let subnet_prefix = subnet.split('/').next().unwrap_or("").replace(".0", "");
                    let filtered: Vec<ArpEntry> = entries
                        .into_iter()
                        .filter(|e| e.ip.starts_with(&subnet_prefix.replace(".0", ".")))
                        .collect();
                    if !filtered.is_empty() {
                        return Ok(filtered);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        api_url = %device.api_url,
                        error = %e,
                        "Failed to fetch ARP from scanner device"
                    );
                }
            }
        }

        Ok(vec![])
    }

    /// ARPスキャナデバイスからARP結果を取得
    async fn fetch_arp_from_device(&self, api_url: &str) -> Result<Vec<ArpEntry>, Error> {
        let client = reqwest::Client::new();
        let response = client
            .get(api_url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("HTTP request failed: {}", e)))?;

        // 形式: ["192.168.125.45,A842A1B95323", "192.168.126.30,DC62798D08EA"]
        let entries: Vec<String> = response
            .json()
            .await
            .map_err(|e| Error::Internal(format!("Failed to parse response: {}", e)))?;

        let arp_entries: Vec<ArpEntry> = entries
            .iter()
            .filter_map(|s| {
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() == 2 {
                    Some(ArpEntry {
                        ip: parts[0].to_string(),
                        mac: parts[1].to_uppercase(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(arp_entries)
    }

    /// ARPスキャン状態を更新
    async fn update_arp_scan_status(&self, camera_id: &str, result: &str) -> Result<(), Error> {
        sqlx::query(
            "UPDATE cameras SET arp_scan_attempted_at = NOW(), arp_scan_result = ? WHERE camera_id = ?"
        )
        .bind(result)
        .bind(camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Internal(format!("Failed to update ARP scan status: {}", e)))?;

        Ok(())
    }

    /// カメラのIPを更新
    async fn relocate_camera_ip(&self, camera: &LostCamera, new_ip: &str) -> Result<(), Error> {
        // RTSP URLを更新
        let old_ip = &camera.ip_address;

        sqlx::query(
            r#"
            UPDATE cameras
            SET ip_address = ?,
                rtsp_main = REPLACE(rtsp_main, ?, ?),
                rtsp_sub = REPLACE(rtsp_sub, ?, ?),
                arp_scan_result = 'found_updated',
                ip_relocation_count = ip_relocation_count + 1,
                last_healthy_at = NULL
            WHERE camera_id = ?
            "#
        )
        .bind(new_ip)
        .bind(old_ip)
        .bind(new_ip)
        .bind(old_ip)
        .bind(new_ip)
        .bind(&camera.camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Internal(format!("Failed to update camera IP: {}", e)))?;

        // 履歴を記録
        sqlx::query(
            r#"
            INSERT INTO camera_ip_relocation_history
            (camera_id, camera_name, old_ip, new_ip, detection_method)
            VALUES (?, ?, ?, ?, ?)
            "#
        )
        .bind(&camera.camera_id)
        .bind(&camera.name)
        .bind(old_ip)
        .bind(new_ip)
        .bind("arp_scan")
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Internal(format!("Failed to record relocation history: {}", e)))?;

        tracing::info!(
            camera_id = %camera.camera_id,
            camera_name = %camera.name,
            old_ip = %old_ip,
            new_ip = %new_ip,
            "Camera IP relocated via ARP scan"
        );

        // EVENT LOG出力
        let event = DetectionEvent {
            event_id: 0, // Will be assigned by EventLogService
            camera_id: camera.camera_id.clone(),
            frame_id: Uuid::new_v4().to_string(),
            captured_at: Utc::now(),
            primary_event: format!("📍 {} - IPアドレスの変更を追跡しました。{}", camera.name, new_ip),
            severity: 2, // INFO level
            tags: vec!["system".to_string(), "ip_relocation".to_string()],
            unknown_flag: false,
            attributes: Some(serde_json::json!({
                "old_ip": old_ip,
                "new_ip": new_ip,
                "detection_method": "arp_scan"
            })),
            thumbnail_url: None,
            created_at: Utc::now(),
        };
        self.event_log_service.add_event(event).await;

        Ok(())
    }

    /// 定期実行タスク
    pub async fn run_tracking_cycle(&self) -> Result<Vec<IpRelocationResult>, Error> {
        let config = self.config.read().await;
        if !config.enabled {
            return Ok(vec![]);
        }
        drop(config);

        let lost_cameras = self.detect_lost_cameras().await?;

        if lost_cameras.is_empty() {
            return Ok(vec![]);
        }

        tracing::info!(
            count = lost_cameras.len(),
            "Detected lost cameras, starting ARP tracking"
        );

        let mut results = Vec::new();
        for camera in &lost_cameras {
            if let Some(result) = self.track_lost_camera(camera).await? {
                results.push(result);
            }
        }

        if !results.is_empty() {
            tracing::info!(
                relocated_count = results.len(),
                "Completed ARP tracking cycle"
            );
        }

        Ok(results)
    }

    /// カメラが正常復帰した際のフラグリセット
    pub async fn mark_camera_healthy(&self, camera_id: &str) -> Result<(), Error> {
        sqlx::query(
            "UPDATE cameras SET last_healthy_at = NOW(), arp_scan_attempted_at = NULL, arp_scan_result = NULL WHERE camera_id = ?"
        )
        .bind(camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Internal(format!("Failed to mark camera healthy: {}", e)))?;

        Ok(())
    }
}

/// IP変更追跡結果
#[derive(Debug, Clone)]
pub struct IpRelocationResult {
    pub camera_id: String,
    pub camera_name: String,
    pub old_ip: String,
    pub new_ip: String,
    pub detection_method: String,
}
