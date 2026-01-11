//! Camera Sync Module
//!
//! Phase 8: カメラ双方向同期 (Issue #121)
//! DD10_CameraBidirectionalSync.md準拠
//!
//! ## 概要
//! IS22とmobes2.0（Paraclate APP）間でカメラメタデータを双方向同期する。
//!
//! ## 機能
//! - IS22→mobes2.0: カメラ名、コンテキスト、ステータス同期
//! - mobes2.0→IS22: カメラ個別設定（感度等）同期
//! - 双方向: カメラ削除通知
//!
//! ## モジュール構成
//! - `types`: 同期用型定義
//! - `repository`: DB永続化
//! - `sync_service`: 同期サービス
//! - `metadata_pusher`: IS22→mobes送信
//!
//! ## SSoT分担
//! - カメラLacisID: araneaDeviceGate (SSoT)
//! - カメラ名/コンテキスト: IS22 (SSoT)
//! - カメラ個別設定: mobes2.0 (SSoT)

pub mod repository;
pub mod sync_service;
pub mod types;

// Re-exports
pub use repository::CameraSyncRepository;
pub use sync_service::CameraSyncService;
pub use types::*;
