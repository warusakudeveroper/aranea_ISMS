//! IS22 Camserver - mobes AIcam control Tower (mAcT)
//!
//! Main entry point for the Camserver application.

use is22_camserver::{
    admission_controller::AdmissionController,
    ai_client::AIClient,
    aranea_register::AraneaRegisterService,
    auto_attunement::AutoAttunementService,
    camera_brand::CameraBrandService,
    camera_registry::CameraContextService,
    camera_status_tracker::CameraStatusTracker,
    config_store::ConfigStore,
    detection_log_service::DetectionLogService,
    event_log_service::EventLogService,
    inference_stats_service::InferenceStatsService,
    ipcam_scan::IpcamScan,
    overdetection_analyzer::OverdetectionAnalyzer,
    bq_sync_service::{BqSyncConfig, BqSyncService},
    camera_sync::{CameraSyncRepository, CameraSyncService},
    paraclate_client::{ConfigSyncService, FidValidator, ParaclateClient, PubSubSubscriber},
    polling_orchestrator::PollingOrchestrator,
    prev_frame_cache::PrevFrameCache,
    preset_loader::PresetLoader,
    realtime_hub::RealtimeHub,
    rtsp_manager::RtspManager,
    snapshot_service::SnapshotService,
    stream_gateway::StreamGateway,
    suggest_engine::SuggestEngine,
    summary_service::{
        GrandSummaryGenerator, ScheduleRepository, SummaryGenerator, SummaryRepository,
        SummaryScheduler,
    },
    state::{AppConfig, AppState, SystemHealth},
    web_api,
};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
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
    let inference_stats = Arc::new(InferenceStatsService::new(pool.clone()));
    let auto_attunement = Arc::new(AutoAttunementService::new(pool.clone(), inference_stats.clone()));
    let overdetection_analyzer = Arc::new(OverdetectionAnalyzer::new(pool.clone()));
    tracing::info!("AI Event Log components initialized (DetectionLogService, PrevFrameCache, PresetLoader, InferenceStatsService, AutoAttunementService, OverdetectionAnalyzer)");

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

    // Initialize AraneaRegisterService (Phase 1: Issue #114)
    let aranea_register = if let Some(ref gate_url) = config.aranea_gate_url {
        tracing::info!(gate_url = %gate_url, "AraneaRegisterService initializing");
        Some(Arc::new(AraneaRegisterService::new(
            gate_url.clone(),
            pool.clone(),
            config_store.clone(),
        )))
    } else {
        tracing::info!("AraneaRegisterService disabled (ARANEA_GATE_URL not set)");
        None
    };

    // Initialize Summary Service components (Phase 3: Issue #116)
    let summary_repository = SummaryRepository::new(pool.clone());
    let schedule_repository = ScheduleRepository::new(pool.clone());
    let camera_context_service = CameraContextService::new(pool.clone());
    let summary_generator = Arc::new(SummaryGenerator::new(
        detection_log.clone(),
        camera_context_service,
        summary_repository.clone(),
        config_store.clone(),
    ));
    let grand_summary_generator = Arc::new(GrandSummaryGenerator::new(summary_repository.clone()));
    tracing::info!("Summary Service initialized (SummaryGenerator, GrandSummaryGenerator, Repositories)");

    // Initialize ParaclateClient (Phase 4: Issue #117)
    let paraclate_client = Arc::new(ParaclateClient::new(
        pool.clone(),
        config_store.clone(),
    ));
    tracing::info!("ParaclateClient initialized (Phase 4)");

    // Initialize ConfigSyncService and PubSubSubscriber (Phase 4 T4-7: Issue #117)
    let config_sync_service = Arc::new(ConfigSyncService::new(
        pool.clone(),
        config_store.clone(),
    ));
    let pubsub_subscriber = Arc::new(PubSubSubscriber::new(config_sync_service));
    tracing::info!("PubSubSubscriber initialized (Phase 4 T4-7)");

    // Initialize FidValidator (Issue #119: テナント-FID所属検証)
    let fid_validator = Arc::new(FidValidator::new(
        pool.clone(),
        config_store.clone(),
    ));
    tracing::info!("FidValidator initialized (Issue #119: Tenant-FID ownership validation)");

    // Set FidValidator on PubSubSubscriber (Issue #119)
    pubsub_subscriber.set_fid_validator(fid_validator.clone()).await;
    tracing::info!("FidValidator integrated with PubSubSubscriber");

    // Initialize BqSyncService (Phase 5: Issue #118)
    // Disabled by default until credentials are configured
    let bq_sync_config = BqSyncConfig {
        enabled: std::env::var("BQ_SYNC_ENABLED").ok().map(|v| v == "true").unwrap_or(false),
        credentials_path: std::env::var("BQ_CREDENTIALS_PATH").ok(),
        ..Default::default()
    };
    let bq_sync_service = if bq_sync_config.enabled {
        tracing::info!(
            project_id = %bq_sync_config.project_id,
            dataset_id = %bq_sync_config.dataset_id,
            "BqSyncService enabled"
        );
        Some(Arc::new(BqSyncService::new(pool.clone(), bq_sync_config)))
    } else {
        tracing::info!("BqSyncService disabled (set BQ_SYNC_ENABLED=true to enable)");
        None
    };

    // Phase 8: CameraSyncService initialization (Issue #121)
    let camera_sync_repository = CameraSyncRepository::new(pool.clone());
    let camera_sync = Some(Arc::new(
        CameraSyncService::new(camera_sync_repository, config_store.clone())
            .with_paraclate_client(paraclate_client.clone())
    ));
    tracing::info!("CameraSyncService initialized");

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
        inference_stats,
        auto_attunement,
        overdetection_analyzer,
        aranea_register,
        summary_generator,
        grand_summary_generator,
        summary_repository,
        schedule_repository,
        paraclate_client,
        pubsub_subscriber,
        fid_validator,
        bq_sync_service: bq_sync_service.clone(),
        camera_sync,
    };

    // Start BqSyncService background task if enabled (Phase 5: Issue #118)
    if let Some(ref bq_sync) = bq_sync_service {
        if let Err(e) = bq_sync.start_background_sync().await {
            tracing::error!(error = %e, "Failed to start BqSyncService background task");
        }
    }

    // Start SummaryScheduler background task (Phase 3: Issue #116)
    // Note: Use state.schedule_repository.clone() here, not schedule_repository
    // because schedule_repository was moved into AppState
    let summary_scheduler = Arc::new(SummaryScheduler::new(
        state.schedule_repository.clone(),
        state.summary_generator.clone(),
        state.grand_summary_generator.clone(),
    ));
    summary_scheduler.start().await;
    tracing::info!("SummaryScheduler started (60-second tick interval)");

    // Create router with static file serving
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "/opt/is22/frontend/dist".to_string());
    let serve_dir = ServeDir::new(&static_dir)
        .not_found_service(ServeFile::new(format!("{}/index.html", static_dir)));

    let app = web_api::create_router(state.clone())
        .fallback_service(serve_dir)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http());

    tracing::info!(static_dir = %static_dir, "Static file serving enabled");

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
