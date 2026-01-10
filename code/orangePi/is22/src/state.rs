//! Application state
//!
//! Holds all shared components and state

use crate::admission_controller::AdmissionController;
use crate::ai_client::AIClient;
use crate::aranea_register::AraneaRegisterService;
use crate::auto_attunement::AutoAttunementService;
use crate::camera_brand::CameraBrandService;
use crate::config_store::ConfigStore;
use crate::detection_log_service::DetectionLogService;
use crate::event_log_service::EventLogService;
use crate::inference_stats_service::InferenceStatsService;
use crate::ipcam_scan::IpcamScan;
use crate::overdetection_analyzer::OverdetectionAnalyzer;
use crate::polling_orchestrator::PollingOrchestrator;
use crate::prev_frame_cache::PrevFrameCache;
use crate::preset_loader::PresetLoader;
use crate::realtime_hub::RealtimeHub;
use crate::snapshot_service::SnapshotService;
use crate::stream_gateway::StreamGateway;
use crate::suggest_engine::SuggestEngine;
use crate::summary_service::{
    GrandSummaryGenerator, ScheduleRepository, SummaryGenerator, SummaryRepository,
};
use sqlx::MySqlPool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Database URL
    pub database_url: String,
    /// IS21 AI server URL
    pub is21_url: String,
    /// go2rtc URL
    pub go2rtc_url: String,
    /// Server port
    pub port: u16,
    /// Server host
    pub host: String,
    /// Snapshot cache directory (for CameraGrid display)
    pub snapshot_dir: PathBuf,
    /// Temporary directory (for prev images for IS21 diff detection)
    pub temp_dir: PathBuf,
    /// araneaDeviceGate URL (Phase 1: AraneaRegister)
    pub aranea_gate_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "mysql://root:mijeos12345@@localhost/camserver".to_string()),
            is21_url: std::env::var("IS21_URL")
                .unwrap_or_else(|_| "http://192.168.3.240:9000".to_string()),
            go2rtc_url: std::env::var("GO2RTC_URL")
                .unwrap_or_else(|_| "http://localhost:1984".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            host: std::env::var("HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            snapshot_dir: std::env::var("SNAPSHOT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/var/lib/is22/snapshots")),
            temp_dir: std::env::var("TEMP_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/var/lib/is22/temp")),
            aranea_gate_url: std::env::var("ARANEA_GATE_URL").ok(),
        }
    }
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Database pool
    pub pool: MySqlPool,
    /// Application config
    pub config: AppConfig,
    /// ConfigStore (SSoT)
    pub config_store: Arc<ConfigStore>,
    /// AdmissionController
    pub admission: Arc<AdmissionController>,
    /// AIClient (IS21 adapter)
    pub ai_client: Arc<AIClient>,
    /// EventLogService (legacy in-memory ring buffer)
    pub event_log: Arc<EventLogService>,
    /// DetectionLogService (MySQL persistence for AI Event Log)
    pub detection_log: Arc<DetectionLogService>,
    /// PrevFrameCache (frame diff support)
    pub prev_frame_cache: Arc<PrevFrameCache>,
    /// PresetLoader (AI analysis presets)
    pub preset_loader: Arc<PresetLoader>,
    /// SuggestEngine
    pub suggest: Arc<SuggestEngine>,
    /// StreamGateway (go2rtc adapter)
    pub stream: Arc<StreamGateway>,
    /// RealtimeHub (WebSocket/SSE)
    pub realtime: Arc<RealtimeHub>,
    /// IpcamScan (camera discovery)
    pub ipcam_scan: Arc<IpcamScan>,
    /// CameraBrandService (OUI/RTSP template management)
    pub camera_brand: Arc<CameraBrandService>,
    /// SnapshotService (RTSP -> ffmpeg -> cache)
    pub snapshot_service: Arc<SnapshotService>,
    /// System health status
    pub system_health: Arc<RwLock<SystemHealth>>,
    /// PollingOrchestrator (camera polling)
    pub polling: Arc<PollingOrchestrator>,
    /// InferenceStatsService (Issue #106: 推論統計・分析)
    pub inference_stats: Arc<InferenceStatsService>,
    /// AutoAttunementService (Issue #106: 自動閾値調整)
    pub auto_attunement: Arc<AutoAttunementService>,
    /// OverdetectionAnalyzer (Issue #107: 過剰検出分析)
    pub overdetection_analyzer: Arc<OverdetectionAnalyzer>,
    /// AraneaRegisterService (Phase 1: Issue #114)
    pub aranea_register: Option<Arc<AraneaRegisterService>>,
    /// SummaryGenerator (Phase 3: Issue #116)
    pub summary_generator: Arc<SummaryGenerator>,
    /// GrandSummaryGenerator (Phase 3: Issue #116)
    pub grand_summary_generator: Arc<GrandSummaryGenerator>,
    /// SummaryRepository (Phase 3: Issue #116)
    pub summary_repository: SummaryRepository,
    /// ScheduleRepository (Phase 3: Issue #116)
    pub schedule_repository: ScheduleRepository,
}

/// System health metrics
#[derive(Debug, Clone, Default)]
pub struct SystemHealth {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub overloaded: bool,
    pub last_overload_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl SystemHealth {
    /// Check and update overload status
    pub fn update(&mut self, cpu: f32, memory: f32) {
        self.cpu_percent = cpu;
        self.memory_percent = memory;

        if cpu > 85.0 || memory > 90.0 {
            self.overloaded = true;
            self.last_overload_at = Some(chrono::Utc::now());
        } else if self.overloaded {
            // Recovery with hysteresis
            if let Some(last) = self.last_overload_at {
                let elapsed = chrono::Utc::now() - last;
                if elapsed > chrono::Duration::seconds(60) && cpu < 60.0 && memory < 70.0 {
                    self.overloaded = false;
                }
            }
        }
    }
}
