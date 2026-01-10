//! CameraRegistry Module
//!
//! Phase 2: CameraRegistry (Issue #115)
//! DD05_CameraRegistry.md準拠
//!
//! ## 概要
//! is22配下のカメラを仮想araneaDevice (is801 paraclateCamera) として管理。
//! 各カメラにlacisIDを付与し、Paraclate連携を実現する。
//!
//! ## モジュール構成
//! - `types`: 型定義・定数
//! - `lacis_id`: カメラ用LacisID生成ロジック
//! - `repository`: DB永続化
//! - `service`: 登録サービス本体
//!
//! ## LacisID形式 (カメラ用)
//! ```text
//! [Prefix=3][ProductType=801][MAC=12桁][ProductCode=4桁] = 20桁
//!            │                │         │
//!            │                │         └─ ブランド別 (Tapo=0001, Hikvision=0002等)
//!            │                └─ カメラのMACアドレス
//!            └─ is801 paraclateCamera
//! ```
//!
//! ## 使用例
//! ```rust,ignore
//! use is22::camera_registry::{CameraRegistryService, CameraRegisterRequest};
//!
//! let service = CameraRegistryService::new(gate_url, pool, config_store);
//!
//! // カメラ登録
//! let result = service.register_camera(CameraRegisterRequest {
//!     camera_id: "cam_001".to_string(),
//!     mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
//!     brand_product_code: "0001".to_string(), // Tapo
//! }).await?;
//!
//! // 登録状態確認
//! let status = service.get_registration("cam_001").await?;
//! ```

pub mod context;
pub mod ipcam_connector;
pub mod lacis_id;
pub mod repository;
pub mod service;
pub mod types;

// Re-exports
pub use context::{CameraContext, CameraContextService, ContextMapEntry, UpdateContextRequest};
pub use ipcam_connector::{IpcamConnector, RtspSyncStatus, SyncResult};
pub use lacis_id::{generate_camera_lacis_id, try_generate_camera_lacis_id};
pub use repository::CameraRegistrationRepository;
pub use service::CameraRegistryService;
pub use types::*;
