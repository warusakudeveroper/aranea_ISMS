//! AraneaRegister Module
//!
//! Phase 1: AraneaRegister (Issue #114)
//! DD01_AraneaRegister.md準拠
//!
//! ## 概要
//! is22 (Paraclate Server) のaraneaDeviceGateへのデバイス登録機能を提供。
//! 登録によりlacisOath認証に必要なCIC (Client Identification Code) を取得し、
//! Paraclate APP連携およびtid/fid権限境界に準拠した運用を可能にする。
//!
//! ## モジュール構成
//! - `types`: 型定義・定数
//! - `lacis_id`: LacisID生成ロジック
//! - `repository`: DB永続化
//! - `service`: 登録サービス本体
//!
//! ## 使用例
//! ```rust,ignore
//! use is22::aranea_register::{AraneaRegisterService, RegisterRequest, TenantPrimaryAuth};
//!
//! let service = AraneaRegisterService::new(gate_url, pool, config_store);
//!
//! // 登録
//! let result = service.register_device(RegisterRequest {
//!     tenant_primary_auth: TenantPrimaryAuth {
//!         lacis_id: "18217487937895888001".to_string(),
//!         user_id: "soejim@mijeos.com".to_string(),
//!         cic: "204965".to_string(),
//!     },
//!     tid: "T2025120621041161827".to_string(),
//! }).await?;
//!
//! // 状態確認
//! let status = service.get_registration_status().await?;
//! println!("Registered: {}", status.registered);
//!
//! // クリア
//! service.clear_registration().await?;
//! ```
//!
//! ## SSoT原則
//! - config_store: 即時参照用（起動時読み込み、高速アクセス）
//! - aranea_registration (DB): 履歴・監査用

pub mod lacis_id;
pub mod repository;
pub mod service;
pub mod types;

// Re-exports
pub use lacis_id::{generate_lacis_id, get_primary_mac_address, try_generate_lacis_id};
pub use repository::AraneaRegistrationRepository;
pub use service::AraneaRegisterService;
pub use types::*;
