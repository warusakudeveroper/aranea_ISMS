//! Summary Service Module
//!
//! Phase 3: Summary/GrandSummary (Issue #116)
//! DD02_Summary_GrandSummary.md準拠
//!
//! ## 概要
//! 検出イベントを定期的に集計し、Summary（間隔ベース）および
//! GrandSummary（時刻指定ベース）を生成・保存する。
//!
//! ## Summary種別
//! | 種別 | トリガー | デフォルト | 用途 |
//! |------|---------|-----------|------|
//! | Summary | 時間間隔経過 | 60分 | 期間内の検出イベント要約 |
//! | GrandSummary | 指定時刻到達 | 09:00, 17:00, 21:00 | 複数Summaryの統合・1日の総括 |
//! | Emergency | 異常検出時 | severity >= 閾値 | 即時報告 |
//!
//! ## モジュール構成
//! - `types`: 型定義・定数
//! - `repository`: DB永続化
//! - `generator`: Summary生成ロジック
//! - `grand_summary`: GrandSummary生成ロジック
//! - `scheduler`: 定時実行スケジューラ
//! - `payload_builder`: Paraclate送信用ペイロード構築
//!
//! ## 使用例
//! ```rust,ignore
//! use is22::summary_service::{SummaryGenerator, SummaryScheduler};
//!
//! // Summary手動生成
//! let result = generator.generate(tid, fid, period_start, period_end).await?;
//!
//! // スケジューラ起動
//! scheduler.start().await;
//! ```

pub mod generator;
pub mod grand_summary;
pub mod payload_builder;
pub mod repository;
pub mod scheduler;
pub mod types;

// Re-exports
pub use generator::SummaryGenerator;
pub use grand_summary::GrandSummaryGenerator;
pub use payload_builder::PayloadBuilder;
pub use repository::{ScheduleRepository, SummaryRepository};
pub use scheduler::SummaryScheduler;
pub use types::*;
