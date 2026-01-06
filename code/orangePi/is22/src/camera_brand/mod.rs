//! Camera Brand Service
//!
//! SSoT management for camera brands, OUI entries, and RTSP templates.
//! Provides caching for high-performance OUI lookups during scanning.

mod types;
mod repository;
mod service;

pub use types::*;
pub use repository::CameraBrandRepository;
pub use service::CameraBrandService;
