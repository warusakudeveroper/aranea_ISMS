//! ParaclateClient Module
//!
//! Phase 4: ParaclateClient (Issue #117)
//! DD03_ParaclateClient.md準拠
//!
//! ## 概要
//! Paraclate APP（mobes2.0）との通信を担当するクライアントモジュール
//!
//! ## 主要機能
//! - HTTP通信（LacisOath認証ヘッダ付与）
//! - 設定同期（mobes2.0 → is22）
//! - Summary/Event送信キュー管理
//! - リトライ・オフライン対応
//! - Pub/Sub通知受信（T4-7）
//!
//! ## モジュール構成
//! - `types`: 型定義・定数
//! - `repository`: DB永続化（config, queue, log）
//! - `client`: HTTPクライアント実装
//! - `config_sync`: 設定同期サービス
//! - `pubsub_subscriber`: Pub/Sub通知受信（T4-7）
//!
//! ## 使用例
//! ```rust,ignore
//! use is22::paraclate_client::ParaclateClient;
//!
//! // クライアント作成
//! let client = ParaclateClient::new(pool, config_store);
//!
//! // 接続
//! let result = client.connect(tid, fid, endpoint).await?;
//!
//! // Summary送信
//! client.send_summary(tid, fid, payload, summary_id).await?;
//!
//! // キュー処理
//! client.process_queue(tid, fid).await?;
//! ```
//!
//! ## 認証
//! LacisOath認証ヘッダを全リクエストに付与:
//! - `X-Lacis-ID`: is22のLacisID
//! - `X-Lacis-TID`: テナントID
//! - `X-Lacis-CIC`: Client Identification Code
//! - `X-Lacis-Blessing`: 越境アクセス用（オプション）

pub mod client;
pub mod config_sync;
pub mod fid_validator;
pub mod pubsub_subscriber;
pub mod repository;
pub mod types;

// Re-exports
pub use client::ParaclateClient;
pub use config_sync::{ConfigSyncService, SyncResult};
pub use fid_validator::{FidValidationError, FidValidator};
pub use pubsub_subscriber::{
    ConfigUpdateNotification, NotificationType, PubSubPushMessage, PubSubSubscriber,
};
pub use repository::{ConfigRepository, ConnectionLogRepository, SendQueueRepository};
pub use types::*;
