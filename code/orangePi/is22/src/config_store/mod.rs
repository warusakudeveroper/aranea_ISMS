//! ConfigStore - Single Source of Truth (SSoT)
//!
//! Issue #37 [IS22-GAP-011] ConfigStore SSoT implementation
//!
//! ## Responsibilities
//!
//! - Camera inventory management
//! - Policy/threshold settings
//! - Schema version management
//! - Settings persistence
//!
//! ## Design Principles
//!
//! - SSoT: All configuration reads/writes go through here
//! - No other module stores camera config locally
//! - IS21 receives config from here, doesn't manage it

mod repository;
mod service;
mod types;

pub use repository::ConfigRepository;
pub use service::ConfigService;
pub use types::*;

use sqlx::MySqlPool;
use std::sync::Arc;
use tokio::sync::RwLock;

/// ConfigStore instance
pub struct ConfigStore {
    pool: MySqlPool,
    service: ConfigService,
    /// In-memory cache for frequent reads
    cache: Arc<RwLock<ConfigCache>>,
}

impl ConfigStore {
    /// Create new ConfigStore
    pub async fn new(pool: MySqlPool) -> crate::Result<Self> {
        let repo = ConfigRepository::new(pool.clone());
        let service = ConfigService::new(repo);

        let cache = Arc::new(RwLock::new(ConfigCache::default()));

        let store = Self {
            pool,
            service,
            cache,
        };

        // Initial cache load
        store.refresh_cache().await?;

        Ok(store)
    }

    /// Get service reference
    pub fn service(&self) -> &ConfigService {
        &self.service
    }

    /// Refresh in-memory cache
    pub async fn refresh_cache(&self) -> crate::Result<()> {
        let cameras = self.service.list_cameras().await?;
        let settings = self.service.get_all_settings().await?;
        let schema_version = self.service.get_current_schema_version().await?;

        let mut cache = self.cache.write().await;
        cache.cameras = cameras;
        cache.settings = settings;
        cache.schema_version = schema_version;

        tracing::info!("ConfigStore cache refreshed: {} cameras", cache.cameras.len());

        Ok(())
    }

    /// Get cached cameras (fast read)
    pub async fn get_cached_cameras(&self) -> Vec<Camera> {
        self.cache.read().await.cameras.clone()
    }

    /// Get cached setting (fast read)
    pub async fn get_cached_setting(&self, key: &str) -> Option<serde_json::Value> {
        self.cache.read().await.settings.get(key).cloned()
    }

    /// Get cached schema version
    pub async fn get_cached_schema_version(&self) -> Option<String> {
        self.cache.read().await.schema_version.clone()
    }
}

/// In-memory cache for ConfigStore
#[derive(Default)]
struct ConfigCache {
    cameras: Vec<Camera>,
    settings: std::collections::HashMap<String, serde_json::Value>,
    schema_version: Option<String>,
}
