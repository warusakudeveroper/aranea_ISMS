//! Access Absorber - Camera brand-specific connection limit management
//!
//! ## Purpose
//!
//! Manages RTSP stream connection limits per camera brand/model, providing:
//! - Concurrent stream limiting
//! - Reconnection interval enforcement
//! - User-friendly error messages
//! - Connection state visibility
//!
//! ## Design Reference
//!
//! See: `camBrand/accessAbsorber/SPECIFICATION.md`

mod repository;
mod service;
mod types;

pub use repository::{AccessAbsorberRepository, ConnectionStats};
pub use service::AccessAbsorberService;
pub use types::*;
