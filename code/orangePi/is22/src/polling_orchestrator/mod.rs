//! PollingOrchestrator - Sequential Camera Polling
//!
//! ## Responsibilities
//!
//! - Sequential camera polling (1 at a time)
//! - Poll scheduling based on priority
//! - Integration with SnapshotService, AIClient, and AI Event Log pipeline
//!
//! ## AI Event Log Integration
//!
//! Uses new analyze() API with:
//! - PrevFrameCache for frame diff analysis
//! - PresetLoader for camera preset configuration
//! - DetectionLogService for MySQL persistence

use crate::ai_client::{AIClient, AnalyzeRequest, AnalyzeResponse, CameraContext};
use crate::config_store::{Camera, ConfigStore};
use crate::detection_log_service::DetectionLogService;
use crate::event_log_service::{DetectionEvent, EventLogService};
use crate::prev_frame_cache::{FrameMeta, PrevFrameCache};
use crate::preset_loader::PresetLoader;
use crate::snapshot_service::SnapshotService;
use crate::suggest_engine::SuggestEngine;
use crate::realtime_hub::{EventLogMessage, HubMessage, RealtimeHub, SnapshotUpdatedMessage};
use chrono::Utc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;

/// PollingOrchestrator instance
pub struct PollingOrchestrator {
    config_store: Arc<ConfigStore>,
    snapshot_service: Arc<SnapshotService>,
    ai_client: Arc<AIClient>,
    event_log: Arc<EventLogService>,
    detection_log: Arc<DetectionLogService>,
    prev_frame_cache: Arc<PrevFrameCache>,
    preset_loader: Arc<PresetLoader>,
    suggest_engine: Arc<SuggestEngine>,
    realtime_hub: Arc<RealtimeHub>,
    running: Arc<RwLock<bool>>,
    /// Default TID (tenant ID) for logging
    default_tid: String,
    /// Default FID (facility ID) for logging
    default_fid: String,
}

impl PollingOrchestrator {
    /// Create new PollingOrchestrator
    pub fn new(
        config_store: Arc<ConfigStore>,
        snapshot_service: Arc<SnapshotService>,
        ai_client: Arc<AIClient>,
        event_log: Arc<EventLogService>,
        detection_log: Arc<DetectionLogService>,
        prev_frame_cache: Arc<PrevFrameCache>,
        preset_loader: Arc<PresetLoader>,
        suggest_engine: Arc<SuggestEngine>,
        realtime_hub: Arc<RealtimeHub>,
        default_tid: String,
        default_fid: String,
    ) -> Self {
        Self {
            config_store,
            snapshot_service,
            ai_client,
            event_log,
            detection_log,
            prev_frame_cache,
            preset_loader,
            suggest_engine,
            realtime_hub,
            running: Arc::new(RwLock::new(false)),
            default_tid,
            default_fid,
        }
    }

    /// Start polling loop
    pub async fn start(&self) {
        {
            let mut running = self.running.write().await;
            if *running {
                tracing::warn!("Polling already running");
                return;
            }
            *running = true;
        }

        tracing::info!("Starting polling orchestrator with AI Event Log pipeline");

        let config_store = self.config_store.clone();
        let snapshot_service = self.snapshot_service.clone();
        let ai_client = self.ai_client.clone();
        let event_log = self.event_log.clone();
        let detection_log = self.detection_log.clone();
        let prev_frame_cache = self.prev_frame_cache.clone();
        let preset_loader = self.preset_loader.clone();
        let suggest_engine = self.suggest_engine.clone();
        let realtime_hub = self.realtime_hub.clone();
        let running = self.running.clone();
        let default_tid = self.default_tid.clone();
        let default_fid = self.default_fid.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5)); // Check interval

            loop {
                interval.tick().await;

                // Check if still running
                {
                    let is_running = running.read().await;
                    if !*is_running {
                        break;
                    }
                }

                // Get enabled cameras
                let cameras = config_store.get_cached_cameras().await;
                let enabled: Vec<_> = cameras
                    .into_iter()
                    .filter(|c| c.enabled && c.polling_enabled)
                    .collect();

                // Poll each camera sequentially
                for camera in enabled {
                    if let Err(e) = Self::poll_camera_with_ai_log(
                        &camera,
                        &snapshot_service,
                        &ai_client,
                        &event_log,
                        &detection_log,
                        &prev_frame_cache,
                        &preset_loader,
                        &suggest_engine,
                        &realtime_hub,
                        &config_store,
                        &default_tid,
                        &default_fid,
                    )
                    .await
                    {
                        tracing::error!(
                            camera_id = %camera.camera_id,
                            error = %e,
                            "Camera poll failed"
                        );
                    }

                    // Small delay between cameras
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            tracing::info!("Polling orchestrator stopped");
        });
    }

    /// Stop polling loop
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        tracing::info!("Stopping polling orchestrator");
    }

    /// Poll a single camera with AI Event Log pipeline
    ///
    /// Flow (AI Event Log v1.7):
    /// 1. Capture snapshot via ffmpeg (RTSP direct)
    /// 2. Save to cache for CameraGrid display
    /// 3. Get previous frame from PrevFrameCache for diff analysis
    /// 4. Build AnalyzeRequest with preset configuration
    /// 5. Send to IS21 using new analyze() API
    /// 6. Update PrevFrameCache with current frame
    /// 7. Persist to MySQL via DetectionLogService
    /// 8. Legacy: update in-memory EventLogService
    /// 9. Broadcast updates via RealtimeHub
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
        default_tid: &str,
        default_fid: &str,
    ) -> crate::error::Result<()> {
        let start_time = Instant::now();
        let captured_at = Utc::now();
        let captured_at_str = captured_at.to_rfc3339();

        // 1. Capture snapshot via ffmpeg
        let image_data = snapshot_service.capture(camera).await?;
        let image_size = image_data.len();

        // 2. Save to cache for CameraGrid display
        snapshot_service.save_cache(&camera.camera_id, &image_data).await?;

        // 3. Get previous frame from PrevFrameCache for diff analysis
        let prev_frame = prev_frame_cache.get(&camera.camera_id).await?;
        let prev_image_data = prev_frame.as_ref().map(|(data, _)| data.clone());

        // 4. Build AnalyzeRequest with preset configuration
        let preset_id = camera.preset_id.as_deref().unwrap_or("balanced");
        let mut request = preset_loader.create_request(
            preset_id,
            camera.camera_id.clone(),
            captured_at_str.clone(),
        );

        // Apply camera context if available
        if let Some(ref context_json) = camera.camera_context {
            if let Ok(context) = serde_json::from_value::<CameraContext>(context_json.clone()) {
                request.camera_context = Some(context);
            }
        }

        // Add previous frame info for context
        if let Some((_, ref meta)) = prev_frame {
            if let Some(ref mut ctx) = request.camera_context {
                ctx.previous_frame = Some(meta.to_previous_frame_info());
            }
        }

        // 5. Send to IS21 using new analyze() API
        let result = ai_client
            .analyze(image_data.clone(), prev_image_data, request)
            .await?;

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
        realtime_hub
            .broadcast(HubMessage::SnapshotUpdated(SnapshotUpdatedMessage {
                camera_id: camera.camera_id.clone(),
                timestamp: captured_at_str.clone(),
                primary_event: Some(result.primary_event.clone()),
                severity: Some(result.severity),
            }))
            .await;

        // 8. Persist to MySQL via DetectionLogService (always, even if not detected)
        // Get TID/FID from camera or use defaults
        let tid = camera.tid.as_deref().unwrap_or(default_tid);
        let fid = camera.fid.as_deref().unwrap_or(default_fid);
        let lacis_id = camera.lacis_id.as_deref();

        // Parse camera context for persistence
        let camera_context = camera.camera_context.as_ref()
            .and_then(|v| serde_json::from_value::<CameraContext>(v.clone()).ok());

        let log_id = detection_log
            .save_detection(
                tid,
                fid,
                lacis_id,
                &result,
                &image_data,
                camera_context.as_ref(),
                Some(processing_ms),
            )
            .await?;

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
            realtime_hub
                .broadcast(HubMessage::EventLog(EventLogMessage {
                    event_id,
                    camera_id: camera.camera_id.clone(),
                    primary_event: event.primary_event.clone(),
                    severity: event.severity,
                    timestamp: captured_at_str.clone(),
                }))
                .await;

            tracing::info!(
                camera_id = %camera.camera_id,
                log_id = log_id,
                event_id = event_id,
                primary_event = %result.primary_event,
                severity = result.severity,
                processing_ms = processing_ms,
                "Detection recorded to MySQL"
            );
        } else {
            tracing::debug!(
                camera_id = %camera.camera_id,
                log_id = log_id,
                processing_ms = processing_ms,
                "No detection, log saved"
            );
        }

        Ok(())
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

        // Preset info
        if let Some(ref preset_id) = result.preset_id {
            attrs.insert("preset_id".to_string(), serde_json::json!(preset_id));
        }

        if attrs.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(attrs))
        }
    }
}
