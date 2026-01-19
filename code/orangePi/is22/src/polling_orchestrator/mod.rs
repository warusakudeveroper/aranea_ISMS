//! PollingOrchestrator - Subnet-Parallel Camera Polling
//!
//! ## Design Intent
//!
//! サブネットごとに独立したポーリングループを実行する並列システム。
//! - サブネット分離: 192.168.125.x と 192.168.126.x が独立して巡回
//! - 同期的処理: 各サブネット内ではカメラを順次処理
//! - タイムアウト: スナップショット取得に3秒タイムアウト（遅いカメラをスキップ）
//! - 帯域制御: サブネット内は同時に1台、サブネット間は並列
//! - AI Event Log: 常に何かしら巡回中なのでログが途切れない
//!
//! ## Flow
//!
//! ```text
//! サブネット 192.168.125.x:
//!   [Camera1取得(3s timeout)] → [is21 POST] → [Camera2取得] → ...
//! サブネット 192.168.126.x: (並列実行)
//!   [Camera1取得(3s timeout)] → [is21 POST] → [Camera2取得] → ...
//! ```

use crate::access_absorber::{AccessAbsorberService, StreamPurpose};
use crate::ai_client::{AIClient, AnalyzeRequest, AnalyzeResponse, CameraContext};
use crate::camera_status_tracker::{CameraStatusEvent, CameraStatusTracker};
use crate::config_store::{Camera, ConfigStore};
use crate::models::ProcessingTimings;
use crate::detection_log_service::{DetectionLogService, should_save_image};
use crate::event_log_service::{DetectionEvent, EventLogService};
use crate::prev_frame_cache::{FrameMeta, PrevFrameCache};
use crate::preset_loader::PresetLoader;
use crate::snapshot_service::{CaptureResult, SnapshotService, SnapshotSource};
use crate::stream_gateway::StreamGateway;
use crate::suggest_engine::SuggestEngine;
use crate::realtime_hub::{
    CooldownTickMessage, CycleStatsMessage, EventLogMessage, HubMessage, RealtimeHub,
    SnapshotUpdatedMessage,
};
use chrono::{DateTime, Utc};
use rand::Rng;
use sqlx::MySqlPool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cooldown duration between polling cycles (seconds)
const CYCLE_COOLDOWN_SEC: u32 = 15;
/// Snapshot capture timeout in milliseconds (30 seconds for very slow cameras)
const SNAPSHOT_TIMEOUT_MS: u64 = 30000;
/// Threshold for "slow camera" warning (10 seconds)
const SLOW_CAMERA_THRESHOLD_MS: u64 = 10000;

/// Filter excluded objects from IS21 response
/// Issue #104: unknown乱発防止 - IS22側でexcluded_objectsをポストフィルタリング
fn filter_excluded_objects(response: &mut AnalyzeResponse, excluded_objects: &[String]) {
    if excluded_objects.is_empty() {
        return;
    }

    // Convert to lowercase for case-insensitive comparison
    let excluded_lower: Vec<String> = excluded_objects
        .iter()
        .map(|s| s.to_lowercase())
        .collect();

    // Count original bboxes for logging
    let original_count = response.bboxes.len();

    // Filter out excluded labels from bboxes
    response.bboxes.retain(|bbox| {
        !excluded_lower.contains(&bbox.label.to_lowercase())
    });

    let filtered_count = original_count - response.bboxes.len();

    // If all bboxes were filtered out and it was unknown, change to none
    if response.bboxes.is_empty() && response.primary_event == "unknown" {
        tracing::debug!(
            camera_id = %response.camera_id,
            filtered_count = filtered_count,
            original_count = original_count,
            "All bboxes filtered by excluded_objects, changing unknown -> none"
        );
        response.primary_event = "none".to_string();
        response.unknown_flag = false;
        response.detected = false;
        response.confidence = 0.0;
        response.count_hint = 0;
    } else if filtered_count > 0 {
        tracing::debug!(
            camera_id = %response.camera_id,
            filtered_count = filtered_count,
            remaining_count = response.bboxes.len(),
            "Filtered excluded objects from IS21 response"
        );
    }
}

/// PollingOrchestrator instance
pub struct PollingOrchestrator {
    pool: MySqlPool,
    config_store: Arc<ConfigStore>,
    snapshot_service: Arc<SnapshotService>,
    ai_client: Arc<AIClient>,
    event_log: Arc<EventLogService>,
    detection_log: Arc<DetectionLogService>,
    prev_frame_cache: Arc<PrevFrameCache>,
    preset_loader: Arc<PresetLoader>,
    suggest_engine: Arc<SuggestEngine>,
    realtime_hub: Arc<RealtimeHub>,
    camera_status_tracker: Arc<CameraStatusTracker>,
    stream_gateway: Arc<StreamGateway>,
    paraclate_client: Arc<crate::paraclate_client::ParaclateClient>,
    /// AccessAbsorberService for camera brand-specific connection limits
    access_absorber: Option<Arc<AccessAbsorberService>>,
    running: Arc<RwLock<bool>>,
    /// Active subnet polling loops (to prevent duplicate spawns)
    active_subnets: Arc<RwLock<HashSet<String>>>,
    /// Default TID (tenant ID) for logging
    default_tid: String,
    /// Default FID (facility ID) for logging
    default_fid: String,
}

impl PollingOrchestrator {
    /// Create new PollingOrchestrator
    pub fn new(
        pool: MySqlPool,
        config_store: Arc<ConfigStore>,
        snapshot_service: Arc<SnapshotService>,
        ai_client: Arc<AIClient>,
        event_log: Arc<EventLogService>,
        detection_log: Arc<DetectionLogService>,
        prev_frame_cache: Arc<PrevFrameCache>,
        preset_loader: Arc<PresetLoader>,
        suggest_engine: Arc<SuggestEngine>,
        realtime_hub: Arc<RealtimeHub>,
        camera_status_tracker: Arc<CameraStatusTracker>,
        stream_gateway: Arc<StreamGateway>,
        paraclate_client: Arc<crate::paraclate_client::ParaclateClient>,
        access_absorber: Option<Arc<AccessAbsorberService>>,
        default_tid: String,
        default_fid: String,
    ) -> Self {
        Self {
            pool,
            config_store,
            snapshot_service,
            ai_client,
            event_log,
            detection_log,
            prev_frame_cache,
            preset_loader,
            suggest_engine,
            realtime_hub,
            camera_status_tracker,
            stream_gateway,
            paraclate_client,
            access_absorber,
            running: Arc::new(RwLock::new(false)),
            active_subnets: Arc::new(RwLock::new(HashSet::new())),
            default_tid,
            default_fid,
        }
    }

    /// Generate a unique polling ID
    /// Format: {subnet_octet3}-{YYMMDD}-{HHmmss}-{rand4}
    /// Example: 125-250103-143052-7a3f
    fn generate_polling_id(subnet: &str, now: DateTime<Utc>) -> String {
        // Extract subnet third octet (e.g., "192.168.125" -> "125")
        let octet3 = subnet.split('.').nth(2).unwrap_or("0");

        // Format timestamp
        let date_str = now.format("%y%m%d").to_string();
        let time_str = now.format("%H%M%S").to_string();

        // Generate 4-char random hex
        let rand_hex: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(4)
            .map(|c| c.to_ascii_lowercase() as char)
            .collect();

        format!("{}-{}-{}-{}", octet3, date_str, time_str, rand_hex)
    }

    /// Extract subnet third octet as integer
    fn extract_subnet_octet3(subnet: &str) -> i32 {
        subnet
            .split('.')
            .nth(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// Insert a new polling cycle record (at cycle start)
    async fn insert_polling_cycle(
        pool: &MySqlPool,
        polling_id: &str,
        subnet: &str,
        cycle_number: u64,
        camera_count: u32,
        started_at: DateTime<Utc>,
    ) -> crate::error::Result<()> {
        let subnet_octet3 = Self::extract_subnet_octet3(subnet);

        sqlx::query(
            r#"
            INSERT INTO polling_cycles (
                polling_id, subnet, subnet_octet3,
                started_at, cycle_number, camera_count,
                status
            ) VALUES (?, ?, ?, ?, ?, ?, 'running')
            "#,
        )
        .bind(polling_id)
        .bind(subnet)
        .bind(subnet_octet3)
        .bind(started_at)
        .bind(cycle_number)
        .bind(camera_count)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Update polling cycle record (at cycle completion)
    async fn complete_polling_cycle(
        pool: &MySqlPool,
        polling_id: &str,
        success_count: u32,
        failed_count: u32,
        timeout_count: u32,
        duration_ms: i32,
        avg_processing_ms: Option<i32>,
    ) -> crate::error::Result<()> {
        sqlx::query(
            r#"
            UPDATE polling_cycles
            SET completed_at = NOW(3),
                success_count = ?,
                failed_count = ?,
                timeout_count = ?,
                duration_ms = ?,
                avg_processing_ms = ?,
                status = 'completed'
            WHERE polling_id = ?
            "#,
        )
        .bind(success_count)
        .bind(failed_count)
        .bind(timeout_count)
        .bind(duration_ms)
        .bind(avg_processing_ms)
        .bind(polling_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Start polling loops (subnet-parallel)
    ///
    /// ## Design
    ///
    /// - サブネットごとに独立したポーリングループを起動
    /// - 各サブネット内ではカメラを順次処理（同期的）
    /// - スナップショット取得に3秒タイムアウト（遅いカメラはスキップ）
    /// - サブネット間は完全に並列（IS21がキューイング）
    pub async fn start(&self) {
        {
            let mut running = self.running.write().await;
            if *running {
                tracing::warn!("Polling already running");
                return;
            }
            *running = true;
        }

        // Get enabled cameras and group by subnet
        let cameras = self.config_store.get_cached_cameras().await;
        let enabled: Vec<_> = cameras
            .into_iter()
            .filter(|c| c.enabled && c.polling_enabled)
            .collect();

        let subnets = Self::group_cameras_by_subnet(&enabled);

        tracing::info!(
            cooldown_sec = CYCLE_COOLDOWN_SEC,
            snapshot_timeout_ms = SNAPSHOT_TIMEOUT_MS,
            subnet_count = subnets.len(),
            total_cameras = enabled.len(),
            "Starting subnet-parallel polling orchestrator"
        );

        // Spawn a polling loop for each subnet
        for (subnet, cameras) in subnets {
            // Register subnet as active
            {
                let mut active = self.active_subnets.write().await;
                active.insert(subnet.clone());
            }

            let pool = self.pool.clone();
            let config_store = self.config_store.clone();
            let snapshot_service = self.snapshot_service.clone();
            let ai_client = self.ai_client.clone();
            let event_log = self.event_log.clone();
            let detection_log = self.detection_log.clone();
            let prev_frame_cache = self.prev_frame_cache.clone();
            let preset_loader = self.preset_loader.clone();
            let suggest_engine = self.suggest_engine.clone();
            let realtime_hub = self.realtime_hub.clone();
            let camera_status_tracker = self.camera_status_tracker.clone();
            let stream_gateway = self.stream_gateway.clone();
            let paraclate_client = self.paraclate_client.clone();
            let access_absorber = self.access_absorber.clone();
            let running = self.running.clone();
            let active_subnets = self.active_subnets.clone();
            let default_tid = self.default_tid.clone();
            let default_fid = self.default_fid.clone();

            let initial_camera_count = cameras.len();
            let subnet_clone = subnet.clone();

            tracing::info!(
                subnet = %subnet,
                initial_cameras = initial_camera_count,
                "Spawning subnet polling loop (dynamic camera detection enabled)"
            );

            tokio::spawn(async move {
                Self::run_subnet_loop(
                    pool,
                    subnet_clone.clone(),
                    config_store,
                    snapshot_service,
                    ai_client,
                    event_log,
                    detection_log,
                    prev_frame_cache,
                    preset_loader,
                    suggest_engine,
                    realtime_hub,
                    camera_status_tracker,
                    stream_gateway,
                    paraclate_client,
                    access_absorber,
                    running,
                    default_tid,
                    default_fid,
                )
                .await;

                // Remove from active subnets when loop exits
                let mut active = active_subnets.write().await;
                active.remove(&subnet_clone);
                tracing::info!(subnet = %subnet_clone, "Subnet polling loop removed from active set");
            });
        }
    }

    /// Spawn a polling loop for a new subnet if not already active
    /// Call this when a camera is added on a potentially new subnet
    pub async fn spawn_subnet_loop_if_needed(&self, ip_address: &str) {
        let subnet = Self::extract_subnet(ip_address);

        // Check if already active
        {
            let active = self.active_subnets.read().await;
            if active.contains(&subnet) {
                tracing::debug!(subnet = %subnet, "Subnet already has active polling loop");
                return;
            }
        }

        // Check if orchestrator is running
        {
            let running = self.running.read().await;
            if !*running {
                tracing::warn!(subnet = %subnet, "Orchestrator not running, cannot spawn subnet loop");
                return;
            }
        }

        // Register and spawn
        {
            let mut active = self.active_subnets.write().await;
            // Double-check after acquiring write lock
            if active.contains(&subnet) {
                return;
            }
            active.insert(subnet.clone());
        }

        let pool = self.pool.clone();
        let config_store = self.config_store.clone();
        let snapshot_service = self.snapshot_service.clone();
        let ai_client = self.ai_client.clone();
        let event_log = self.event_log.clone();
        let detection_log = self.detection_log.clone();
        let prev_frame_cache = self.prev_frame_cache.clone();
        let preset_loader = self.preset_loader.clone();
        let suggest_engine = self.suggest_engine.clone();
        let realtime_hub = self.realtime_hub.clone();
        let camera_status_tracker = self.camera_status_tracker.clone();
        let stream_gateway = self.stream_gateway.clone();
        let paraclate_client = self.paraclate_client.clone();
        let access_absorber = self.access_absorber.clone();
        let running = self.running.clone();
        let active_subnets = self.active_subnets.clone();
        let default_tid = self.default_tid.clone();
        let default_fid = self.default_fid.clone();

        let subnet_clone = subnet.clone();

        tracing::info!(
            subnet = %subnet,
            "Spawning NEW subnet polling loop for dynamically added camera"
        );

        tokio::spawn(async move {
            Self::run_subnet_loop(
                pool,
                subnet_clone.clone(),
                config_store,
                snapshot_service,
                ai_client,
                event_log,
                detection_log,
                prev_frame_cache,
                preset_loader,
                suggest_engine,
                realtime_hub,
                camera_status_tracker,
                stream_gateway,
                paraclate_client,
                access_absorber,
                running,
                default_tid,
                default_fid,
            )
            .await;

            // Remove from active subnets when loop exits
            let mut active = active_subnets.write().await;
            active.remove(&subnet_clone);
            tracing::info!(subnet = %subnet_clone, "Subnet polling loop removed from active set");
        });
    }

    /// Get list of active subnets
    pub async fn get_active_subnets(&self) -> Vec<String> {
        let active = self.active_subnets.read().await;
        active.iter().cloned().collect()
    }

    /// Extract subnet from IP address (first 3 octets)
    pub fn extract_subnet(ip: &str) -> String {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() >= 3 {
            format!("{}.{}.{}", parts[0], parts[1], parts[2])
        } else {
            "unknown".to_string()
        }
    }

    /// Group cameras by their subnet
    fn group_cameras_by_subnet(cameras: &[Camera]) -> HashMap<String, Vec<Camera>> {
        let mut groups: HashMap<String, Vec<Camera>> = HashMap::new();
        for camera in cameras {
            let subnet = camera
                .ip_address
                .as_ref()
                .map(|ip| {
                    let s = Self::extract_subnet(ip);
                    tracing::debug!(
                        camera_id = %camera.camera_id,
                        ip_address = %ip,
                        subnet = %s,
                        "Camera subnet classification"
                    );
                    s
                })
                .unwrap_or_else(|| {
                    tracing::warn!(
                        camera_id = %camera.camera_id,
                        "Camera has no ip_address, assigning to 'unknown' subnet"
                    );
                    "unknown".to_string()
                });
            groups.entry(subnet).or_default().push(camera.clone());
        }
        groups
    }

    /// Run polling loop for a specific subnet
    #[allow(clippy::too_many_arguments)]
    async fn run_subnet_loop(
        pool: MySqlPool,
        subnet: String,
        config_store: Arc<ConfigStore>,
        snapshot_service: Arc<SnapshotService>,
        ai_client: Arc<AIClient>,
        event_log: Arc<EventLogService>,
        detection_log: Arc<DetectionLogService>,
        prev_frame_cache: Arc<PrevFrameCache>,
        preset_loader: Arc<PresetLoader>,
        suggest_engine: Arc<SuggestEngine>,
        realtime_hub: Arc<RealtimeHub>,
        camera_status_tracker: Arc<CameraStatusTracker>,
        stream_gateway: Arc<StreamGateway>,
        paraclate_client: Arc<crate::paraclate_client::ParaclateClient>,
        access_absorber: Option<Arc<AccessAbsorberService>>,
        running: Arc<RwLock<bool>>,
        default_tid: String,
        default_fid: String,
    ) {
        let mut cycle_number: u64 = 0;

        loop {
            // Check if still running
            {
                let is_running = running.read().await;
                if !*is_running {
                    break;
                }
            }

            cycle_number += 1;
            let cycle_start = Instant::now();
            let cycle_started_at = Utc::now();
            let mut successful: u32 = 0;
            let mut failed: u32 = 0;
            let mut timeout_count: u32 = 0;
            let mut processing_times: Vec<i32> = Vec::new();

            // Generate polling ID for this cycle
            let polling_id = Self::generate_polling_id(&subnet, cycle_started_at);

            // Refresh camera list for this subnet (dynamically - new cameras included)
            let all_cameras = config_store.get_cached_cameras().await;
            let enabled: Vec<_> = all_cameras
                .into_iter()
                .filter(|c| {
                    c.enabled && c.polling_enabled &&
                    c.ip_address.as_ref()
                        .map(|ip| Self::extract_subnet(ip) == subnet)
                        .unwrap_or(false)
                })
                .collect();

            let camera_count = enabled.len() as u32;

            // === go2rtc Stream Registration at Cycle Start ===
            // Register all cameras with RTSP URLs to go2rtc at the beginning of each cycle.
            // This ensures go2rtc always has fresh stream registrations synced with DB.
            // - Idempotent: go2rtc handles duplicates gracefully
            // - No orphans: deleted cameras won't be re-registered next cycle
            // - IS21 lag window: registration completes during AI inference of previous cameras
            let mut go2rtc_registered: u32 = 0;
            let mut go2rtc_failed: u32 = 0;
            for camera in &enabled {
                // Use camera_id directly as stream name (already has cam- prefix)
                let stream_name = camera.camera_id.clone();

                // Get RTSP URL (prefer rtsp_main, fallback to rtsp_sub)
                let rtsp_url = camera.rtsp_main.as_ref()
                    .or(camera.rtsp_sub.as_ref())
                    .cloned();

                if let Some(url) = rtsp_url {
                    // Register to go2rtc (non-blocking, best-effort)
                    match stream_gateway.add_source(&stream_name, &url).await {
                        Ok(_) => {
                            go2rtc_registered += 1;
                            tracing::debug!(
                                camera_id = %camera.camera_id,
                                stream_name = %stream_name,
                                "go2rtc stream registered"
                            );
                        }
                        Err(e) => {
                            go2rtc_failed += 1;
                            tracing::warn!(
                                camera_id = %camera.camera_id,
                                stream_name = %stream_name,
                                error = %e,
                                "go2rtc stream registration failed"
                            );
                        }
                    }
                }
            }

            if go2rtc_registered > 0 || go2rtc_failed > 0 {
                tracing::info!(
                    subnet = %subnet,
                    cycle = cycle_number,
                    go2rtc_registered = go2rtc_registered,
                    go2rtc_failed = go2rtc_failed,
                    "go2rtc streams refreshed at cycle start"
                );
            }

            // Insert polling cycle record at start
            if let Err(e) = Self::insert_polling_cycle(
                &pool,
                &polling_id,
                &subnet,
                cycle_number,
                camera_count,
                cycle_started_at,
            )
            .await
            {
                tracing::warn!(
                    polling_id = %polling_id,
                    error = %e,
                    "Failed to insert polling cycle record"
                );
            }

            tracing::info!(
                subnet = %subnet,
                cycle = cycle_number,
                polling_id = %polling_id,
                cameras = camera_count,
                "Subnet cycle starting with 3-second countdown"
            );

            // Pre-cycle countdown: 3, 2, 1...
            // This provides stabilization time after go2rtc registration
            // and helps ensure cameras are ready before polling starts
            for countdown in (1..=3).rev() {
                realtime_hub
                    .broadcast(HubMessage::CooldownTick(CooldownTickMessage {
                        subnet: subnet.clone(),
                        seconds_remaining: countdown,
                        total_cooldown_sec: 3,
                        phase: "pre_cycle".to_string(),
                    }))
                    .await;

                tracing::debug!(
                    subnet = %subnet,
                    countdown = countdown,
                    "Pre-cycle countdown"
                );

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }

            tracing::info!(
                subnet = %subnet,
                cycle = cycle_number,
                polling_id = %polling_id,
                "Subnet cycle started after countdown"
            );

            // Poll each camera sequentially within this subnet
            for (index, camera) in enabled.iter().enumerate() {
                // Check if still running before each camera
                {
                    let is_running = running.read().await;
                    if !*is_running {
                        break;
                    }
                }

                tracing::debug!(
                    subnet = %subnet,
                    cycle = cycle_number,
                    polling_id = %polling_id,
                    camera = index + 1,
                    total = camera_count,
                    camera_id = %camera.camera_id,
                    "Polling camera"
                );

                // Poll camera with timeout
                let poll_result = Self::poll_camera_with_ai_log(
                    camera,
                    &snapshot_service,
                    &ai_client,
                    &event_log,
                    &detection_log,
                    &prev_frame_cache,
                    &preset_loader,
                    &suggest_engine,
                    &realtime_hub,
                    &config_store,
                    &paraclate_client,
                    access_absorber.as_deref(),
                    &default_tid,
                    &default_fid,
                    Some(&polling_id),
                )
                .await;

                // Track camera connection status and generate events
                let is_online = poll_result.is_ok();
                if let Some(status_event) = camera_status_tracker
                    .update_status(&camera.camera_id, is_online)
                    .await
                {
                    // Get TID/FID for camera event logging (use defaults)
                    let tid = &default_tid;
                    let fid = &default_fid;
                    let lacis_id = camera.lacis_id.as_deref();

                    match status_event {
                        CameraStatusEvent::Lost => {
                            // Log camera lost event (severity 4)
                            let error_msg = if !is_online {
                                poll_result.as_ref().err().map(|e| format!("{}", e))
                            } else {
                                None
                            };
                            if let Err(e) = detection_log
                                .save_camera_event(
                                    tid,
                                    fid,
                                    &camera.camera_id,
                                    lacis_id,
                                    "camera_lost",
                                    4,
                                    error_msg.as_deref(),
                                )
                                .await
                            {
                                tracing::error!(
                                    camera_id = %camera.camera_id,
                                    error = %e,
                                    "Failed to save camera_lost event"
                                );
                            }

                            // === Send camera status change to mobes (IS22_SummaryMessage_FixRequest.md section 10) ===
                            let status_payload = crate::paraclate_client::types::CameraStatusChangePayload {
                                tid: tid.to_string(),
                                fid: fid.to_string(),
                                event_type: "camera_status_change".to_string(),
                                camera_id: camera.camera_id.clone(),
                                camera_name: camera.name.clone(),
                                camera_lacis_id: camera.lacis_id.clone(),
                                ip_address: camera.ip_address.clone(),
                                previous_status: "online".to_string(),
                                new_status: "offline".to_string(),
                                reason: "connection_lost".to_string(),
                                detected_at: Utc::now(),
                                error_details: error_msg.clone(),
                            };
                            if let Err(e) = paraclate_client.send_camera_status_change(tid, fid, status_payload).await {
                                tracing::warn!(
                                    camera_id = %camera.camera_id,
                                    error = %e,
                                    "Failed to send camera_lost status to mobes"
                                );
                            }
                        }
                        CameraStatusEvent::Recovered => {
                            // Log camera recovered event (severity 2)
                            if let Err(e) = detection_log
                                .save_camera_event(
                                    tid,
                                    fid,
                                    &camera.camera_id,
                                    lacis_id,
                                    "camera_recovered",
                                    2,
                                    None,
                                )
                                .await
                            {
                                tracing::error!(
                                    camera_id = %camera.camera_id,
                                    error = %e,
                                    "Failed to save camera_recovered event"
                                );
                            }

                            // === Send camera status change to mobes (IS22_SummaryMessage_FixRequest.md section 10) ===
                            let status_payload = crate::paraclate_client::types::CameraStatusChangePayload {
                                tid: tid.to_string(),
                                fid: fid.to_string(),
                                event_type: "camera_status_change".to_string(),
                                camera_id: camera.camera_id.clone(),
                                camera_name: camera.name.clone(),
                                camera_lacis_id: camera.lacis_id.clone(),
                                ip_address: camera.ip_address.clone(),
                                previous_status: "offline".to_string(),
                                new_status: "online".to_string(),
                                reason: "connection_restored".to_string(),
                                detected_at: Utc::now(),
                                error_details: None,
                            };
                            if let Err(e) = paraclate_client.send_camera_status_change(tid, fid, status_payload).await {
                                tracing::warn!(
                                    camera_id = %camera.camera_id,
                                    error = %e,
                                    "Failed to send camera_recovered status to mobes"
                                );
                            }
                        }
                    }
                }

                // Handle poll result
                match poll_result {
                    Ok(processing_ms) => {
                        successful += 1;
                        if let Some(ms) = processing_ms {
                            processing_times.push(ms);
                        }
                    }
                    Err(e) => {
                        let error_str = format!("{}", e);
                        if error_str.contains("timeout") || error_str.contains("Timeout") {
                            timeout_count += 1;
                            tracing::warn!(
                                subnet = %subnet,
                                cycle = cycle_number,
                                polling_id = %polling_id,
                                camera_id = %camera.camera_id,
                                "Camera snapshot timeout (skipped)"
                            );
                            // Broadcast timeout error to frontend
                            realtime_hub
                                .broadcast(HubMessage::SnapshotUpdated(SnapshotUpdatedMessage {
                                    camera_id: camera.camera_id.clone(),
                                    timestamp: Utc::now().to_rfc3339(),
                                    primary_event: None,
                                    severity: None,
                                    processing_ms: Some(SNAPSHOT_TIMEOUT_MS),
                                    error: Some("timeout".to_string()),
                                    snapshot_source: None,
                                }))
                                .await;
                        } else {
                            failed += 1;
                            tracing::error!(
                                subnet = %subnet,
                                cycle = cycle_number,
                                polling_id = %polling_id,
                                camera_id = %camera.camera_id,
                                error = %e,
                                "Camera poll failed"
                            );
                            // Broadcast error to frontend
                            realtime_hub
                                .broadcast(HubMessage::SnapshotUpdated(SnapshotUpdatedMessage {
                                    camera_id: camera.camera_id.clone(),
                                    timestamp: Utc::now().to_rfc3339(),
                                    primary_event: None,
                                    severity: None,
                                    processing_ms: None,
                                    error: Some(error_str),
                                    snapshot_source: None,
                                }))
                                .await;
                        }
                    }
                }
            }

            // Cycle completed - calculate stats
            let cycle_duration = cycle_start.elapsed();
            let cycle_duration_sec = cycle_duration.as_secs();
            let cycle_duration_ms = cycle_duration.as_millis() as i32;
            let minutes = cycle_duration_sec / 60;
            let seconds = cycle_duration_sec % 60;
            let cycle_duration_formatted = format!("{:02}:{:02}", minutes, seconds);

            // Calculate average processing time
            let avg_processing_ms = if !processing_times.is_empty() {
                Some((processing_times.iter().sum::<i32>() as f32 / processing_times.len() as f32) as i32)
            } else {
                None
            };

            // Update polling cycle record at completion
            if let Err(e) = Self::complete_polling_cycle(
                &pool,
                &polling_id,
                successful,
                failed,
                timeout_count,
                cycle_duration_ms,
                avg_processing_ms,
            )
            .await
            {
                tracing::warn!(
                    polling_id = %polling_id,
                    error = %e,
                    "Failed to update polling cycle record"
                );
            }

            tracing::info!(
                subnet = %subnet,
                cycle = cycle_number,
                polling_id = %polling_id,
                duration_sec = cycle_duration_sec,
                duration_ms = cycle_duration_ms,
                duration = %cycle_duration_formatted,
                cameras = camera_count,
                successful = successful,
                failed = failed,
                timeout = timeout_count,
                avg_processing_ms = ?avg_processing_ms,
                "Subnet cycle completed"
            );

            // Broadcast cycle stats to frontend (per subnet)
            realtime_hub
                .broadcast(HubMessage::CycleStats(CycleStatsMessage {
                    subnet: subnet.clone(),
                    cycle_duration_sec,
                    cycle_duration_formatted: cycle_duration_formatted.clone(),
                    cameras_polled: camera_count,
                    successful,
                    failed: failed + timeout_count, // Include timeouts in failed count
                    cycle_number,
                    completed_at: Utc::now().to_rfc3339(),
                }))
                .await;

            // Cooldown period (no countdown broadcast - not meaningful for parallel subnets)
            tracing::debug!(
                subnet = %subnet,
                cooldown_sec = CYCLE_COOLDOWN_SEC,
                "Starting inter-cycle cooldown"
            );

            for remaining in (1..=CYCLE_COOLDOWN_SEC).rev() {
                // Check if still running
                {
                    let is_running = running.read().await;
                    if !*is_running {
                        break;
                    }
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }

        tracing::info!(subnet = %subnet, "Subnet polling loop stopped");
    }

    /// Stop polling loop
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        tracing::info!("Stopping polling orchestrator");
    }

    /// Poll a single camera with AI Event Log pipeline
    ///
    /// Flow (AI Event Log v1.7 + Access Absorber):
    /// 1. Capture snapshot via ffmpeg (RTSP direct) with AccessAbsorber rate limiting
    /// 2. Save to cache for CameraGrid display
    /// 3. Get previous frame from PrevFrameCache for diff analysis
    /// 4. Build AnalyzeRequest with preset configuration
    /// 5. Send to IS21 using new analyze() API
    /// 6. Update PrevFrameCache with current frame
    /// 7. Persist to MySQL via DetectionLogService
    /// 8. Legacy: update in-memory EventLogService
    /// 9. Broadcast updates via RealtimeHub
    ///
    /// Returns: Ok(Some(processing_ms)) on success, or error
    #[allow(clippy::too_many_arguments)]
    async fn poll_camera_with_ai_log(
        camera: &Camera,
        snapshot_service: &SnapshotService,
        ai_client: &AIClient,
        event_log: &EventLogService,
        detection_log: &DetectionLogService,
        prev_frame_cache: &PrevFrameCache,
        preset_loader: &PresetLoader,
        suggest_engine: &SuggestEngine,
        realtime_hub: &RealtimeHub,
        config_store: &ConfigStore,
        paraclate_client: &crate::paraclate_client::ParaclateClient,
        access_absorber: Option<&AccessAbsorberService>,
        default_tid: &str,
        default_fid: &str,
        polling_cycle_id: Option<&str>,
    ) -> crate::error::Result<Option<i32>> {
        let start_time = Instant::now();
        let captured_at = Utc::now();
        let captured_at_str = captured_at.to_rfc3339();

        // Get camera IP for logging
        let camera_ip = camera.ip_address.as_deref().unwrap_or("unknown");
        let preset_id = camera.preset_id.as_deref().unwrap_or("balanced");

        // === Phase 1: Snapshot capture (with AccessAbsorber rate limiting) ===
        let snapshot_start = Instant::now();
        let capture_result = match tokio::time::timeout(
            Duration::from_millis(SNAPSHOT_TIMEOUT_MS),
            snapshot_service.capture_with_absorber(
                camera,
                access_absorber,
                StreamPurpose::Polling,
                "polling_orchestrator",
            ),
        )
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => {
                tracing::warn!(
                    camera_id = %camera.camera_id,
                    camera_ip = %camera_ip,
                    preset_id = %preset_id,
                    error = %e,
                    "Snapshot capture failed"
                );
                return Err(e);
            }
            Err(_) => {
                tracing::warn!(
                    camera_id = %camera.camera_id,
                    camera_ip = %camera_ip,
                    preset_id = %preset_id,
                    timeout_ms = SNAPSHOT_TIMEOUT_MS,
                    "Snapshot capture timeout"
                );
                return Err(crate::error::Error::Internal(format!(
                    "Snapshot capture timeout ({}ms) for camera {}",
                    SNAPSHOT_TIMEOUT_MS, camera.camera_id
                )));
            }
        };
        let snapshot_ms = snapshot_start.elapsed().as_millis() as i32;
        let image_data = capture_result.data;
        let snapshot_source = capture_result.source;
        let image_size = image_data.len();

        // Generate image filename for logging
        let image_filename = format!(
            "{}.jpg",
            captured_at_str.replace([':', '-', 'T', 'Z', '.'], "")
        );

        // 2. Save to cache for CameraGrid display
        snapshot_service.save_cache(&camera.camera_id, &image_data).await?;

        // 3. Get previous frame from PrevFrameCache for diff analysis
        let prev_frame = prev_frame_cache.get(&camera.camera_id).await?;
        let prev_image_data = prev_frame.as_ref().map(|(data, _)| data.clone());

        // 4. Build AnalyzeRequest with preset configuration
        let mut request = preset_loader.create_request(
            preset_id,
            camera.camera_id.clone(),
            captured_at_str.clone(),
        );

        // Apply inference_config if available (merge with preset context)
        // Note: camera_context is for LLM/Paraclate, inference_config is for IS21
        if let Some(ref config_json) = camera.inference_config {
            if let Ok(camera_cfg) = serde_json::from_value::<CameraContext>(config_json.clone()) {
                // Merge: camera-specific settings override preset settings
                if let Some(ref mut preset_ctx) = request.camera_context {
                    // カメラ固有設定があればプリセットを上書き
                    if camera_cfg.location_type.is_some() {
                        preset_ctx.location_type = camera_cfg.location_type;
                    }
                    if camera_cfg.distance.is_some() {
                        preset_ctx.distance = camera_cfg.distance;
                    }
                    if camera_cfg.expected_objects.is_some() {
                        preset_ctx.expected_objects = camera_cfg.expected_objects;
                    }
                    if camera_cfg.excluded_objects.is_some() {
                        preset_ctx.excluded_objects = camera_cfg.excluded_objects;
                    }
                    if camera_cfg.conf_override.is_some() {
                        preset_ctx.conf_override = camera_cfg.conf_override;
                    }
                    if camera_cfg.nms_threshold.is_some() {
                        preset_ctx.nms_threshold = camera_cfg.nms_threshold;
                    }
                    if camera_cfg.par_threshold.is_some() {
                        preset_ctx.par_threshold = camera_cfg.par_threshold;
                    }
                } else {
                    // プリセットにcontextがなければカメラ設定をそのまま使用
                    request.camera_context = Some(camera_cfg);
                }
            }
        }

        // Debug: log final conf_override after merge
        if let Some(ref ctx) = request.camera_context {
            tracing::debug!(
                camera_id = %camera.camera_id,
                preset_id = %preset_id,
                conf_override = ?ctx.conf_override,
                has_inference_config = camera.inference_config.is_some(),
                "Final camera_context after merge"
            );
        }

        // Add previous frame info for context
        if let Some((_, ref meta)) = prev_frame {
            if let Some(ref mut ctx) = request.camera_context {
                ctx.previous_frame = Some(meta.to_previous_frame_info());
            }
        }

        // === Phase 2: IS21 inference ===
        let is21_start = Instant::now();
        let mut result = ai_client
            .analyze(image_data.clone(), prev_image_data, request)
            .await?;
        let is21_roundtrip_ms = is21_start.elapsed().as_millis() as i32;

        // Issue #104: Filter excluded objects from IS21 response
        // プリセットのexcluded_objectsに基づいてbboxesをフィルタリング
        let preset = preset_loader.get_or_default(preset_id);
        if !preset.excluded_objects.is_empty() {
            filter_excluded_objects(&mut result, &preset.excluded_objects);
        }

        // Extract IS21 internal timings
        let is21_inference_ms = result.performance.as_ref().map(|p| p.inference_ms as i32).unwrap_or(0);
        let is21_yolo_ms = result.performance.as_ref().map(|p| p.yolo_ms as i32).unwrap_or(0);
        let is21_par_ms = result.performance.as_ref().map(|p| p.par_ms as i32).unwrap_or(0);

        let processing_ms = start_time.elapsed().as_millis() as i32;

        // 6. Update PrevFrameCache with current frame
        let frame_meta = FrameMeta::new(
            captured_at,
            result.primary_event.clone(),
            result.count_hint,
            result.severity,
            image_size,
        );
        let _ = prev_frame_cache.store(&camera.camera_id, image_data.clone(), frame_meta).await;

        // 7. Broadcast snapshot update notification (triggers CameraGrid to refresh)
        let processing_ms_u64 = processing_ms as u64;
        realtime_hub
            .broadcast(HubMessage::SnapshotUpdated(SnapshotUpdatedMessage {
                camera_id: camera.camera_id.clone(),
                timestamp: captured_at_str.clone(),
                primary_event: Some(result.primary_event.clone()),
                severity: Some(result.severity),
                processing_ms: Some(processing_ms_u64),
                error: None,
                snapshot_source: Some(snapshot_source.as_str().to_string()),
            }))
            .await;

        // === Phase 3: Database save (conditional) ===
        let save_start = Instant::now();

        // Get TID/FID from camera (fallback to defaults if not set)
        let tid = camera.tid.as_deref().unwrap_or(default_tid);
        let fid = camera.fid.as_deref().unwrap_or(default_fid);
        let lacis_id = camera.lacis_id.as_deref();

        // Parse camera context for persistence
        let camera_context = camera.camera_context.as_ref()
            .and_then(|v| serde_json::from_value::<CameraContext>(v.clone()).ok());

        // Build timing breakdown with snapshot source
        let timings = ProcessingTimings {
            total_ms: processing_ms,
            snapshot_ms,
            is21_roundtrip_ms,
            is21_inference_ms,
            is21_yolo_ms,
            is21_par_ms,
            save_ms: 0, // Will be updated after save
            snapshot_source: Some(snapshot_source.as_str().to_string()),
        };

        // AIEventlog.md要件: 「何もない」なら画像の保存もログも出さない
        let log_id = if should_save_image(&result.primary_event, result.severity, result.unknown_flag) {
            detection_log
                .save_detection(
                    tid,
                    fid,
                    lacis_id,
                    camera.lacis_id.as_deref(),
                    &result,
                    &image_data,
                    camera_context.as_ref(),
                    Some(processing_ms),
                    Some(&timings),
                    polling_cycle_id,
                )
                .await?
        } else {
            tracing::debug!(
                camera_id = %camera.camera_id,
                primary_event = %result.primary_event,
                severity = result.severity,
                "Skipping image save - no detection (none)"
            );
            0 // No log_id when not saved
        };

        let save_ms = save_start.elapsed().as_millis() as i32;
        let total_ms = start_time.elapsed().as_millis() as i32;

        // 9. Update legacy in-memory EventLogService (for backward compatibility)
        if result.detected {
            let attributes = Self::build_event_attributes(&result);
            let frame_id = result.captured_at.replace([':', '-', 'T', 'Z', '.'], "");

            let event = DetectionEvent {
                event_id: 0,
                camera_id: camera.camera_id.clone(),
                frame_id,
                captured_at,
                primary_event: result.primary_event.clone(),
                severity: result.severity,
                tags: result.tags.clone(),
                unknown_flag: result.unknown_flag,
                attributes,
                thumbnail_url: None,
                created_at: captured_at,
            };

            let event_id = event_log.add_event(event.clone()).await;

            // Update suggest engine
            let _ = suggest_engine
                .process_event(&event, camera.suggest_policy_weight)
                .await;

            // Broadcast detection event via WebSocket
            // BUG-001 fix: Include lacis_id for frontend camera lookup
            realtime_hub
                .broadcast(HubMessage::EventLog(EventLogMessage {
                    event_id,
                    camera_id: camera.camera_id.clone(),
                    lacis_id: camera.lacis_id.clone().unwrap_or_default(),
                    primary_event: event.primary_event.clone(),
                    severity: event.severity,
                    timestamp: captured_at_str.clone(),
                }))
                .await;

            // === Phase: Send detection event + snapshot to mobes ===
            // mobes2.0 AI Chat がスナップショット画像を参照できるようにするため、
            // 検出イベントと画像をParaclate Ingest Event APIに送信
            if log_id > 0 {
                // llm_designers_review(important).md準拠: 検出データ完全送信
                // mobes2.0 Paraclate形式: フラット構造 + snake_case
                let event_payload = crate::paraclate_client::types::EventPayload {
                    tid: tid.to_string(),
                    fid: fid.to_string(),
                    detection_log_id: log_id,
                    camera_id: camera.lacis_id.clone().unwrap_or_default(),
                    captured_at,
                    primary_event: result.primary_event.clone(),
                    severity: result.severity,
                    confidence: result.confidence,
                    tags: result.tags.clone(),

                    // DD19: フラット構造で全フィールド送信
                    bboxes: if result.bboxes.is_empty() {
                        None
                    } else {
                        Some(serde_json::to_value(&result.bboxes).unwrap_or_default())
                    },
                    count_hint: Some(result.count_hint),
                    unknown_flag: Some(result.unknown_flag),
                    schema_version: Some(result.schema_version.clone()),
                    // DD19: person_details をJSONValueに変換
                    person_details: result.person_details.as_ref()
                        .map(|pd| serde_json::to_value(pd).unwrap_or_default()),
                    // DD19: vehicle_details をJSONValueに変換
                    vehicle_details: result.vehicle_details.as_ref()
                        .map(|vd| serde_json::to_value(vd).unwrap_or_default()),
                    suspicious: result.suspicious.as_ref()
                        .map(|s| serde_json::to_value(s).unwrap_or_default()),
                    frame_diff: result.frame_diff.as_ref()
                        .map(|fd| serde_json::to_value(fd).unwrap_or_default()),
                    loitering_detected: Some(result.frame_diff.as_ref()
                        .and_then(|fd| fd.loitering.as_ref())
                        .map(|l| l.detected)
                        .unwrap_or(false)),
                    is22_timings: Some(serde_json::json!({
                        "snapshot_ms": snapshot_ms,
                        "is21_roundtrip_ms": is21_roundtrip_ms,
                        "is21_inference_ms": is21_inference_ms,
                        "is21_yolo_ms": is21_yolo_ms,
                        "is21_par_ms": is21_par_ms,
                        "total_ms": processing_ms
                    })),
                    // PresetAppliedInfoから各フィールドを抽出
                    preset_applied: result.preset_applied.as_ref()
                        .map(|p| p.preset_id.clone()),
                    preset_id: result.preset_applied.as_ref()
                        .map(|p| p.preset_id.clone()),
                    preset_version: result.preset_applied.as_ref()
                        .map(|p| p.preset_version.clone()),

                    snapshot_base64: None, // send_event_with_snapshotで設定される
                    snapshot_mime_type: None,
                    malfunction_type: None,
                    malfunction_details: None,
                };

                // 非同期でイベント+スナップショットを送信（失敗してもポーリングは継続）
                let send_result = paraclate_client
                    .send_event_with_snapshot(tid, fid, event_payload, Some(image_data.clone()), log_id)
                    .await;

                match send_result {
                    Ok(result) => {
                        tracing::info!(
                            log_id = log_id,
                            queue_id = result.queue_id,
                            success = result.success,
                            "Detection event sent to mobes"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            log_id = log_id,
                            error = %e,
                            "Failed to send detection event to mobes (will retry via queue)"
                        );
                    }
                }
            }

            // Detailed logging with timing breakdown
            tracing::info!(
                camera_id = %camera.camera_id,
                camera_ip = %camera_ip,
                preset_id = %preset_id,
                image_filename = %image_filename,
                image_size_kb = image_size / 1024,
                log_id = log_id,
                event_id = event_id,
                primary_event = %result.primary_event,
                severity = result.severity,
                confidence = result.confidence,
                count_hint = result.count_hint,
                // Timing breakdown (ボトルネック特定用)
                total_ms = total_ms,
                snapshot_ms = snapshot_ms,
                is21_roundtrip_ms = is21_roundtrip_ms,
                is21_inference_ms = is21_inference_ms,
                is21_yolo_ms = is21_yolo_ms,
                is21_par_ms = is21_par_ms,
                save_ms = save_ms,
                // IS21 decision chain visibility
                analyzed = result.analyzed,
                detected = result.detected,
                bbox_count = result.bboxes.len(),
                "DETECTED: {}",
                result.primary_event
            );

            // TODO(autoAttunement): 検出結果をStatsCollectorに送信
            // 参照: Layout＆AIlog_Settings/AIEventLog_Redesign_v4.md Section 5.6
            // 実装時: stats_collector.record_detection(&camera.camera_id, &result);
        } else {
            // No detection - but still log with full detail for debugging "why none?"
            tracing::info!(
                camera_id = %camera.camera_id,
                camera_ip = %camera_ip,
                preset_id = %preset_id,
                image_filename = %image_filename,
                image_size_kb = image_size / 1024,
                log_id = log_id,
                primary_event = %result.primary_event,
                // Timing breakdown (ボトルネック特定用)
                total_ms = total_ms,
                snapshot_ms = snapshot_ms,
                is21_roundtrip_ms = is21_roundtrip_ms,
                is21_inference_ms = is21_inference_ms,
                is21_yolo_ms = is21_yolo_ms,
                is21_par_ms = is21_par_ms,
                save_ms = save_ms,
                // IS21 decision chain visibility - WHY was this "none"?
                analyzed = result.analyzed,
                detected = result.detected,
                bbox_count = result.bboxes.len(),
                confidence = result.confidence,
                "NO_DETECTION: analyzed={}, bboxes={}, confidence={}",
                result.analyzed,
                result.bboxes.len(),
                result.confidence
            );
        }

        Ok(Some(total_ms))
    }

    /// Build event attributes JSON from AnalyzeResponse
    fn build_event_attributes(result: &AnalyzeResponse) -> Option<serde_json::Value> {
        let mut attrs = serde_json::Map::new();

        // Basic detection info
        attrs.insert("confidence".to_string(), serde_json::json!(result.confidence));
        attrs.insert("count_hint".to_string(), serde_json::json!(result.count_hint));

        // Bounding boxes
        if !result.bboxes.is_empty() {
            attrs.insert("bboxes".to_string(), serde_json::json!(result.bboxes));
        }

        // Person details
        if let Some(ref person_details) = result.person_details {
            attrs.insert("person_details".to_string(), serde_json::json!(person_details));
        }

        // Suspicious info
        if let Some(ref suspicious) = result.suspicious {
            attrs.insert("suspicious".to_string(), serde_json::json!(suspicious));
        }

        // Frame diff
        if let Some(ref frame_diff) = result.frame_diff {
            attrs.insert("frame_diff".to_string(), serde_json::json!(frame_diff));
        }

        // Preset info (Phase 4)
        if let Some(ref preset_applied) = result.preset_applied {
            attrs.insert("preset_applied".to_string(), serde_json::json!(preset_applied));
        } else if let Some(ref preset_id) = result.preset_id {
            // Fallback to legacy preset_id field
            attrs.insert("preset_id".to_string(), serde_json::json!(preset_id));
        }

        // Performance metrics (Phase 4)
        if let Some(ref performance) = result.performance {
            attrs.insert("is21_performance".to_string(), serde_json::json!(performance));
        }

        if attrs.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(attrs))
        }
    }
}
