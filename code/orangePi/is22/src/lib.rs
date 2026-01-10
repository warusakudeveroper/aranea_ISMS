//! IS22 Camserver Library
//!
//! mobes AIcam control Tower (mAcT)
//!
//! ## Architecture (11 Components)
//!
//! 1. ConfigStore - SSoT for cameras, policies, settings
//! 2. SnapshotService - Image capture from cameras
//! 3. AIClient - IS21 communication adapter
//! 4. PollingOrchestrator - Sequential camera polling
//! 5. EventLogService - Event recording (ring buffer)
//! 6. SuggestEngine - "What to watch now" decision
//! 7. StreamGatewayAdapter - go2rtc integration
//! 8. AdmissionController - Load control
//! 9. RealtimeHub - WebSocket/SSE distribution
//! 10. WebAPI - REST API endpoints
//! 11. IpcamScan - Camera auto-discovery
//!
//! ## Design Principles
//!
//! - SSoT: ConfigStore is the single source of truth
//! - SOLID: Single responsibility per module
//! - MECE: Mutually exclusive, collectively exhaustive

pub mod aranea_register;
pub mod camera_brand;
pub mod config_store;
pub mod sdm_integration;
pub mod admission_controller;
pub mod ai_client;
pub mod camera_status_tracker;
pub mod detection_log_service;
pub mod event_log_service;
pub mod suggest_engine;
pub mod stream_gateway;
pub mod realtime_hub;
pub mod web_api;
pub mod ipcam_scan;
pub mod snapshot_service;
pub mod prev_frame_cache;
pub mod preset_loader;
pub mod polling_orchestrator;
pub mod rtsp_manager;
pub mod models;
pub mod inference_stats_service;
pub mod auto_attunement;
pub mod overdetection_analyzer;
pub mod error;
pub mod state;

pub use error::{Error, Result};
pub use state::AppState;
