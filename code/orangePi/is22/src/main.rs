//! IS22 Camserver - mobes AIcam control Tower (mAcT)
//!
//! Main entry point for the Camserver application.

use is22_camserver::{
    admission_controller::AdmissionController,
    ai_client::AIClient,
    camera_brand::CameraBrandService,
    camera_status_tracker::CameraStatusTracker,
    config_store::ConfigStore,
    detection_log_service::DetectionLogService,
    event_log_service::EventLogService,
    ipcam_scan::IpcamScan,
    polling_orchestrator::PollingOrchestrator,
    prev_frame_cache::PrevFrameCache,
    preset_loader::PresetLoader,
    realtime_hub::RealtimeHub,
    rtsp_manager::RtspManager,
    snapshot_service::SnapshotService,
    stream_gateway::StreamGateway,
    suggest_engine::SuggestEngine,
    state::{AppConfig, AppState, SystemHealth},
    web_api,
};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Load global timeout settings from settings.polling
async fn load_global_timeout_settings(pool: &sqlx::MySqlPool) -> (u64, u64) {
    let result = sqlx::query("SELECT setting_json FROM settings WHERE setting_key = 'polling'")
        .fetch_optional(pool)
        .await;

    match result {
        Ok(Some(row)) => {
            let setting_json: String = row.get("setting_json");
            if let Ok(polling_settings) = serde_json::from_str::<serde_json::Value>(&setting_json) {
                let timeout_main = polling_settings["timeout_main_sec"].as_u64().unwrap_or(10);
                let timeout_sub = polling_settings["timeout_sub_sec"].as_u64().unwrap_or(20);

                tracing::info!(
                    timeout_main_sec = timeout_main,
                    timeout_sub_sec = timeout_sub,
                    "Loaded global timeout settings from database"
                );

                return (timeout_main, timeout_sub);
            }
        }
        Ok(None) => {
            tracing::warn!("settings.polling not found, using default timeouts (10s/20s)");
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to load timeout settings, using defaults (10s/20s)");
        }
    }

    // フォールバックデフォルト
    (10, 20)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "is22_camserver=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting IS22 Camserver v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = AppConfig::default();
    tracing::info!(
        database_url = %config.database_url,
        is21_url = %config.is21_url,
        go2rtc_url = %config.go2rtc_url,
        snapshot_dir = %config.snapshot_dir.display(),
        temp_dir = %config.temp_dir.display(),
        "Configuration loaded"
    );

    // Create database pool
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&config.database_url)
        .await?;

    tracing::info!("Database connected");

    // Initialize system health
    let system_health = Arc::new(RwLock::new(SystemHealth::default()));

    // Initialize components
    let config_store = Arc::new(ConfigStore::new(pool.clone()).await?);
    tracing::info!("ConfigStore initialized");

    let admission_policy = config_store.service().get_admission_policy().await?;
    let admission = Arc::new(AdmissionController::new(
        admission_policy,
        system_health.clone(),
    ));
    tracing::info!("AdmissionController initialized");

    let ai_client = Arc::new(AIClient::new(config.is21_url.clone()));
    let stream = Arc::new(StreamGateway::new(config.go2rtc_url.clone()));
    let event_log = Arc::new(EventLogService::new(2000));
    let realtime = Arc::new(RealtimeHub::new());

    // AI Event Log components
    let detection_log = Arc::new(DetectionLogService::with_pool(pool.clone()));
    let prev_frame_cache = Arc::new(PrevFrameCache::with_defaults());
    let preset_loader = Arc::new(PresetLoader::new());
    tracing::info!("AI Event Log components initialized (DetectionLogService, PrevFrameCache, PresetLoader)");

    let suggest_policy = config_store.service().get_suggest_policy().await?;
    let suggest = Arc::new(SuggestEngine::new(suggest_policy));

    // RTSPアクセス制御マネージャ（カメラごとの多重接続防止）
    let rtsp_manager = Arc::new(RtspManager::new());
    tracing::info!("RtspManager initialized (RTSP access control)");

    // Load global timeout settings for SnapshotService
    let (timeout_main_sec, timeout_sub_sec) = load_global_timeout_settings(&pool).await;

    let snapshot_service = Arc::new(
        SnapshotService::new(
            config.snapshot_dir.clone(),
            config.temp_dir.clone(),
            rtsp_manager.clone(),
            timeout_main_sec,
            timeout_sub_sec,
        )
        .await?
    );
    tracing::info!(
        snapshot_dir = %config.snapshot_dir.display(),
        temp_dir = %config.temp_dir.display(),
        timeout_main_sec = timeout_main_sec,
        timeout_sub_sec = timeout_sub_sec,
        "SnapshotService initialized with global timeout settings (ffmpeg direct RTSP with access control)"
    );

    let ipcam_scan = Arc::new(IpcamScan::new(pool.clone()));
    tracing::info!("IpcamScan initialized with DB persistence");

    // Initialize CameraBrandService with cache
    let camera_brand = Arc::new(CameraBrandService::new(pool.clone()));
    camera_brand.init().await?;
    tracing::info!("CameraBrandService initialized with OUI/RTSP template cache");

    // Get default TID/FID from environment or use defaults
    let default_tid = std::env::var("DEFAULT_TID")
        .unwrap_or_else(|_| "T0000000000000000000".to_string());
    let default_fid = std::env::var("DEFAULT_FID")
        .unwrap_or_else(|_| "0000".to_string());

    // Camera status tracker for lost/recovered events
    let camera_status_tracker = Arc::new(CameraStatusTracker::new());
    tracing::info!("CameraStatusTracker initialized");

    // Create polling orchestrator with AI Event Log pipeline + go2rtc integration
    let polling = Arc::new(PollingOrchestrator::new(
        pool.clone(),
        config_store.clone(),
        snapshot_service.clone(),
        ai_client.clone(),
        event_log.clone(),
        detection_log.clone(),
        prev_frame_cache.clone(),
        preset_loader.clone(),
        suggest.clone(),
        realtime.clone(),
        camera_status_tracker,
        stream.clone(), // go2rtc StreamGateway for cycle-based registration
        default_tid,
        default_fid,
    ));
    tracing::info!("PollingOrchestrator initialized with AI Event Log pipeline + go2rtc cycle registration");

    // Create application state
    let state = AppState {
        pool,
        config,
        config_store,
        admission,
        ai_client,
        event_log,
        detection_log,
        prev_frame_cache,
        preset_loader,
        suggest,
        stream,
        realtime,
        ipcam_scan,
        camera_brand,
        snapshot_service,
        system_health,
        polling: polling.clone(),
    };

    // Create router
    let app = web_api::create_router(state.clone())
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http());

    // Start cleanup task
    let admission_cleanup = state.admission.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            admission_cleanup.cleanup().await;
        }
    });

    // Start suggest expiration task
    let suggest_cleanup = state.suggest.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            suggest_cleanup.check_expiration().await;
        }
    });

    // Start credential cleanup task (#83 T2-11)
    // Clears tried_credentials older than 24 hours (runs every hour)
    let ipcam_scan_cleanup = state.ipcam_scan.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour
        loop {
            interval.tick().await;
            if let Err(e) = ipcam_scan_cleanup.cleanup_tried_credentials().await {
                tracing::error!(error = %e, "Failed to cleanup expired credentials");
            }
        }
    });

    // Start polling orchestrator (is21 AI integration)
    polling.start().await;
    tracing::info!("PollingOrchestrator started - AI integration active");

    // Start system health monitoring
    let health_monitor = state.system_health.clone();
    tokio::spawn(async move {
        use sysinfo::System;
        let mut sys = System::new_all();
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;
            sys.refresh_all();

            // Calculate average CPU usage across all cores
            let cpu = {
                let cpus = sys.cpus();
                if cpus.is_empty() {
                    0.0
                } else {
                    cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
                }
            };
            let memory = if sys.total_memory() > 0 {
                (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0
            } else {
                0.0
            };

            let mut health = health_monitor.write().await;
            health.update(cpu, memory);
        }
    });

    // Start server
    let addr = format!("{}:{}", state.config.host, state.config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
