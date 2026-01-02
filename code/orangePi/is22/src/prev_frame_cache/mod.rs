//! PrevFrameCache - Previous Frame Cache for Frame Diff Analysis
//!
//! ## Responsibilities
//!
//! - Store previous frame with metadata per camera
//! - Provide efficient in-memory access
//! - Track capture timestamps for IS21 context
//! - Persistence fallback to filesystem
//!
//! ## Design Reference
//!
//! - §2.4 PrevFrameCache (is22_AI_EVENT_LOG_DESIGN.md)

use crate::ai_client::PreviousFrameInfo;
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// Frame metadata stored with the image
#[derive(Debug, Clone)]
pub struct FrameMeta {
    /// Capture timestamp
    pub captured_at: DateTime<Utc>,
    /// Primary event from IS21 analysis
    pub primary_event: String,
    /// Person count from IS21 (count_hint)
    pub person_count: i32,
    /// Severity level
    pub severity: i32,
    /// Image size in bytes
    pub size_bytes: usize,
}

impl FrameMeta {
    /// Create new frame metadata
    pub fn new(
        captured_at: DateTime<Utc>,
        primary_event: String,
        person_count: i32,
        severity: i32,
        size_bytes: usize,
    ) -> Self {
        Self {
            captured_at,
            primary_event,
            person_count,
            severity,
            size_bytes,
        }
    }

    /// Create metadata for no detection
    pub fn no_detection(captured_at: DateTime<Utc>, size_bytes: usize) -> Self {
        Self {
            captured_at,
            primary_event: "none".to_string(),
            person_count: 0,
            severity: 0,
            size_bytes,
        }
    }

    /// Convert to IS21 PreviousFrameInfo format
    pub fn to_previous_frame_info(&self) -> PreviousFrameInfo {
        PreviousFrameInfo {
            captured_at: self.captured_at.to_rfc3339(),
            person_count: self.person_count,
            primary_event: self.primary_event.clone(),
        }
    }
}

/// Cached frame entry
struct CacheEntry {
    /// Image data (JPEG bytes)
    data: Vec<u8>,
    /// Frame metadata
    meta: FrameMeta,
}

/// PrevFrameCache configuration
#[derive(Debug, Clone)]
pub struct PrevFrameCacheConfig {
    /// Maximum number of cameras to cache in memory
    pub max_cameras: usize,
    /// Base directory for file persistence
    pub persist_dir: PathBuf,
    /// Enable file persistence
    pub enable_persistence: bool,
    /// Maximum frame age in seconds before considered stale
    pub max_age_secs: i64,
}

impl Default for PrevFrameCacheConfig {
    fn default() -> Self {
        Self {
            max_cameras: 100,
            persist_dir: PathBuf::from("/var/lib/is22/temp"),
            enable_persistence: true,
            max_age_secs: 300, // 5 minutes
        }
    }
}

/// PrevFrameCache service
pub struct PrevFrameCache {
    /// In-memory cache: camera_id -> CacheEntry
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Configuration
    config: PrevFrameCacheConfig,
}

impl PrevFrameCache {
    /// Create new PrevFrameCache
    pub fn new(config: PrevFrameCacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(PrevFrameCacheConfig::default())
    }

    /// Store frame as previous for a camera
    ///
    /// This should be called after IS21 analysis completes
    pub async fn store(
        &self,
        camera_id: &str,
        data: Vec<u8>,
        meta: FrameMeta,
    ) -> Result<()> {
        let size = data.len();

        // Store in memory cache
        {
            let mut cache = self.cache.write().await;

            // Check capacity and evict if needed
            if cache.len() >= self.config.max_cameras && !cache.contains_key(camera_id) {
                self.evict_oldest(&mut cache);
            }

            cache.insert(camera_id.to_string(), CacheEntry { data: data.clone(), meta: meta.clone() });
        }

        // Persist to file if enabled
        if self.config.enable_persistence {
            self.persist(camera_id, &data, &meta).await?;
        }

        tracing::trace!(
            camera_id = %camera_id,
            size = size,
            captured_at = %meta.captured_at,
            primary_event = %meta.primary_event,
            "Stored prev frame"
        );

        Ok(())
    }

    /// Get previous frame for a camera
    ///
    /// Returns both the image data and metadata
    pub async fn get(&self, camera_id: &str) -> Result<Option<(Vec<u8>, FrameMeta)>> {
        // Try memory cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(camera_id) {
                // Check if frame is not stale
                let age = Utc::now().signed_duration_since(entry.meta.captured_at);
                if age.num_seconds() <= self.config.max_age_secs {
                    return Ok(Some((entry.data.clone(), entry.meta.clone())));
                }
            }
        }

        // Fallback to file system
        if self.config.enable_persistence {
            if let Some((data, meta)) = self.load_from_file(camera_id).await? {
                // Cache in memory for next access
                {
                    let mut cache = self.cache.write().await;
                    cache.insert(camera_id.to_string(), CacheEntry {
                        data: data.clone(),
                        meta: meta.clone(),
                    });
                }
                return Ok(Some((data, meta)));
            }
        }

        Ok(None)
    }

    /// Get previous frame image data only
    pub async fn get_image(&self, camera_id: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.get(camera_id).await?.map(|(data, _)| data))
    }

    /// Get previous frame metadata only
    pub async fn get_meta(&self, camera_id: &str) -> Result<Option<FrameMeta>> {
        Ok(self.get(camera_id).await?.map(|(_, meta)| meta))
    }

    /// Get PreviousFrameInfo for IS21 context
    pub async fn get_previous_frame_info(&self, camera_id: &str) -> Result<Option<PreviousFrameInfo>> {
        Ok(self.get_meta(camera_id).await?.map(|m| m.to_previous_frame_info()))
    }

    /// Check if a camera has a valid (non-stale) previous frame
    pub async fn has_valid_frame(&self, camera_id: &str) -> bool {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(camera_id) {
            let age = Utc::now().signed_duration_since(entry.meta.captured_at);
            return age.num_seconds() <= self.config.max_age_secs;
        }
        false
    }

    /// Clear cache for a camera
    pub async fn clear(&self, camera_id: &str) -> Result<()> {
        {
            let mut cache = self.cache.write().await;
            cache.remove(camera_id);
        }

        // Remove persisted file
        if self.config.enable_persistence {
            let camera_dir = self.config.persist_dir.join(camera_id);
            let _ = fs::remove_dir_all(&camera_dir).await;
        }

        Ok(())
    }

    /// Clear all caches
    pub async fn clear_all(&self) -> Result<()> {
        {
            let mut cache = self.cache.write().await;
            cache.clear();
        }

        // Clear persist directory
        if self.config.enable_persistence {
            let _ = fs::remove_dir_all(&self.config.persist_dir).await;
            fs::create_dir_all(&self.config.persist_dir).await?;
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total_bytes: usize = cache.values().map(|e| e.data.len()).sum();
        CacheStats {
            camera_count: cache.len(),
            total_bytes,
        }
    }

    // ========================================
    // Internal Methods
    // ========================================

    /// Evict oldest entry from cache
    fn evict_oldest(&self, cache: &mut HashMap<String, CacheEntry>) {
        if let Some((oldest_id, _)) = cache
            .iter()
            .min_by_key(|(_, e)| e.meta.captured_at)
            .map(|(id, e)| (id.clone(), e.meta.captured_at))
        {
            cache.remove(&oldest_id);
            tracing::debug!(
                camera_id = %oldest_id,
                "Evicted oldest frame from cache"
            );
        }
    }

    /// Persist frame to file system
    async fn persist(&self, camera_id: &str, data: &[u8], meta: &FrameMeta) -> Result<()> {
        let camera_dir = self.config.persist_dir.join(camera_id);
        fs::create_dir_all(&camera_dir).await?;

        // Save image
        let image_path = camera_dir.join("prev.jpg");
        fs::write(&image_path, data).await?;

        // Save metadata as JSON
        let meta_json = serde_json::json!({
            "captured_at": meta.captured_at.to_rfc3339(),
            "primary_event": meta.primary_event,
            "person_count": meta.person_count,
            "severity": meta.severity,
            "size_bytes": meta.size_bytes,
        });
        let meta_path = camera_dir.join("prev.meta.json");
        fs::write(&meta_path, meta_json.to_string()).await?;

        Ok(())
    }

    /// Load frame from file system
    async fn load_from_file(&self, camera_id: &str) -> Result<Option<(Vec<u8>, FrameMeta)>> {
        let camera_dir = self.config.persist_dir.join(camera_id);
        let image_path = camera_dir.join("prev.jpg");
        let meta_path = camera_dir.join("prev.meta.json");

        // Check if files exist
        if !image_path.exists() {
            return Ok(None);
        }

        // Load image
        let data = fs::read(&image_path).await?;

        // Load metadata (or create default)
        let meta = if meta_path.exists() {
            let meta_str = fs::read_to_string(&meta_path).await?;
            let json: serde_json::Value = serde_json::from_str(&meta_str)?;

            let captured_at = json["captured_at"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            FrameMeta {
                captured_at,
                primary_event: json["primary_event"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                person_count: json["person_count"].as_i64().unwrap_or(0) as i32,
                severity: json["severity"].as_i64().unwrap_or(0) as i32,
                size_bytes: data.len(),
            }
        } else {
            // Create default metadata from file modification time
            let file_meta = fs::metadata(&image_path).await?;
            let modified = file_meta.modified().ok()
                .map(|t| DateTime::<Utc>::from(t))
                .unwrap_or_else(Utc::now);

            FrameMeta::no_detection(modified, data.len())
        };

        // Check if stale
        let age = Utc::now().signed_duration_since(meta.captured_at);
        if age.num_seconds() > self.config.max_age_secs {
            return Ok(None);
        }

        Ok(Some((data, meta)))
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cameras in cache
    pub camera_count: usize,
    /// Total bytes used
    pub total_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_meta_to_previous_frame_info() {
        let meta = FrameMeta::new(
            Utc::now(),
            "human".to_string(),
            2,
            1,
            12345,
        );

        let info = meta.to_previous_frame_info();
        assert_eq!(info.primary_event, "human");
        assert_eq!(info.person_count, 2);
    }

    #[test]
    fn test_default_config() {
        let config = PrevFrameCacheConfig::default();
        assert_eq!(config.max_cameras, 100);
        assert_eq!(config.max_age_secs, 300);
    }
}
