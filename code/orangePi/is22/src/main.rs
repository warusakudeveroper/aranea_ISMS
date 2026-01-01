//! IS22 Camserver - mobes AIcam control Tower (mAcT)
//!
//! Main entry point for the Camserver application.

use is22_camserver::{
    admission_controller::AdmissionController,
    ai_client::AIClient,
    config_store::ConfigStore,
    event_log_service::EventLogService,
    ipcam_scan::IpcamScan,
    polling_orchestrator::PollingOrchestrator,
    realtime_hub::RealtimeHub,
    snapshot_service::SnapshotService,
    stream_gateway::StreamGateway,
    suggest_engine::SuggestEngine,
    state::{AppConfig, AppState, SystemHealth},
    web_api,
};
use sqlx::mysql::MySqlPoolOptions;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    let suggest_policy = config_store.service().get_suggest_policy().await?;
    let suggest = Arc::new(SuggestEngine::new(suggest_policy));

    let snapshot_service = Arc::new(SnapshotService::new(config.go2rtc_url.clone()));
    let ipcam_scan = Arc::new(IpcamScan::new(pool.clone()));
    tracing::info!("IpcamScan initialized with DB persistence");

    // Create polling orchestrator
    let polling = Arc::new(PollingOrchestrator::new(
        config_store.clone(),
        snapshot_service.clone(),
        ai_client.clone(),
        event_log.clone(),
        suggest.clone(),
        realtime.clone(),
    ));
    tracing::info!("PollingOrchestrator initialized");

    // Create application state
    let state = AppState {
        pool,
        config,
        config_store,
        admission,
        ai_client,
        event_log,
        suggest,
        stream,
        realtime,
        ipcam_scan,
        system_health,
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
