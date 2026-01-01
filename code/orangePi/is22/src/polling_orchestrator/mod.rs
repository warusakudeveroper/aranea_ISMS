//! PollingOrchestrator - Sequential Camera Polling
//!
//! ## Responsibilities
//!
//! - Sequential camera polling (1 at a time)
//! - Poll scheduling based on priority
//! - Integration with SnapshotService and AIClient

use crate::ai_client::{AIClient, InferRequest};
use crate::config_store::{Camera, ConfigStore, PollingPolicy};
use crate::event_log_service::{DetectionEvent, EventLogService};
use crate::snapshot_service::SnapshotService;
use crate::suggest_engine::SuggestEngine;
use crate::realtime_hub::{EventLogMessage, HubMessage, RealtimeHub};
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

/// PollingOrchestrator instance
pub struct PollingOrchestrator {
    config_store: Arc<ConfigStore>,
    snapshot_service: Arc<SnapshotService>,
    ai_client: Arc<AIClient>,
    event_log: Arc<EventLogService>,
    suggest_engine: Arc<SuggestEngine>,
    realtime_hub: Arc<RealtimeHub>,
    running: Arc<RwLock<bool>>,
}

impl PollingOrchestrator {
    /// Create new PollingOrchestrator
    pub fn new(
        config_store: Arc<ConfigStore>,
        snapshot_service: Arc<SnapshotService>,
        ai_client: Arc<AIClient>,
        event_log: Arc<EventLogService>,
        suggest_engine: Arc<SuggestEngine>,
        realtime_hub: Arc<RealtimeHub>,
    ) -> Self {
        Self {
            config_store,
            snapshot_service,
            ai_client,
            event_log,
            suggest_engine,
            realtime_hub,
            running: Arc::new(RwLock::new(false)),
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

        tracing::info!("Starting polling orchestrator");

        let config_store = self.config_store.clone();
        let snapshot_service = self.snapshot_service.clone();
        let ai_client = self.ai_client.clone();
        let event_log = self.event_log.clone();
        let suggest_engine = self.suggest_engine.clone();
        let realtime_hub = self.realtime_hub.clone();
        let running = self.running.clone();

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
                    if let Err(e) = Self::poll_camera(
                        &camera,
                        &snapshot_service,
                        &ai_client,
                        &event_log,
                        &suggest_engine,
                        &realtime_hub,
                        &config_store,
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

    /// Poll a single camera
    async fn poll_camera(
        camera: &Camera,
        snapshot_service: &SnapshotService,
        ai_client: &AIClient,
        event_log: &EventLogService,
        suggest_engine: &SuggestEngine,
        realtime_hub: &RealtimeHub,
        config_store: &ConfigStore,
    ) -> crate::error::Result<()> {
        // Capture snapshot
        let image_data = snapshot_service.capture(camera).await?;

        // Get schema version
        let schema_version = config_store
            .get_cached_schema_version()
            .await
            .unwrap_or_else(|| "2025-12-29.1".to_string());

        // Send to AI
        let request = InferRequest {
            camera_id: camera.camera_id.clone(),
            schema_version,
            captured_at: Utc::now().to_rfc3339(),
            hints: camera.camera_context.clone(),
        };

        let result = ai_client.infer(image_data, request).await?;

        // Only log if detection occurred
        if result.detected {
            // Convert bboxes to attributes JSON
            let attributes = if !result.bboxes.is_empty() {
                Some(serde_json::json!({
                    "bboxes": result.bboxes,
                    "confidence": result.confidence,
                    "count_hint": result.count_hint,
                }))
            } else {
                None
            };

            // Generate frame_id from captured_at timestamp
            let frame_id = result.captured_at.replace([':', '-', 'T', 'Z'], "");

            let event = DetectionEvent {
                event_id: 0, // Will be set by EventLogService
                camera_id: camera.camera_id.clone(),
                frame_id,
                captured_at: Utc::now(),
                primary_event: result.primary_event.clone(),
                severity: result.severity,
                tags: result.tags.clone(),
                unknown_flag: result.unknown_flag,
                attributes,
                thumbnail_url: None,
                created_at: Utc::now(),
            };

            // Add to event log
            let event_id = event_log.add_event(event.clone()).await;

            // Update suggest engine
            let _ = suggest_engine
                .process_event(&event, camera.suggest_policy_weight)
                .await;

            // Broadcast to clients
            realtime_hub
                .broadcast(HubMessage::EventLog(EventLogMessage {
                    event_id,
                    camera_id: camera.camera_id.clone(),
                    primary_event: event.primary_event.clone(),
                    severity: event.severity,
                    timestamp: Utc::now().to_rfc3339(),
                }))
                .await;

            tracing::info!(
                camera_id = %camera.camera_id,
                event_id = event_id,
                primary_event = %event.primary_event,
                severity = event.severity,
                "Detection event recorded"
            );
        }

        Ok(())
    }
}
