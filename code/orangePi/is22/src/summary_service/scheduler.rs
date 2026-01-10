//! Summary Scheduler
//!
//! Phase 3: Summary/GrandSummary (Issue #116)
//! T3-4: 定時実行スケジューラ
//!
//! ## 概要
//! 1分間隔でscheduled_reportsをチェックし、
//! next_run_at到達時にSummary/GrandSummaryを生成する。
//!
//! ## スケジュール種別
//! - Summary: interval_minutesベースで定期実行（デフォルト60分）
//! - GrandSummary: scheduled_timesベースで時刻指定実行（JST基準）

use super::generator::SummaryGenerator;
use super::grand_summary::{calculate_next_grand_summary_time, GrandSummaryGenerator};
use super::repository::ScheduleRepository;
use super::types::{ReportSchedule, ReportType};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::time::Duration as TokioDuration;
use tracing::{debug, error, info, warn};

/// Summary/GrandSummaryスケジューラ
pub struct SummaryScheduler {
    /// スケジュールリポジトリ
    schedule_repository: ScheduleRepository,
    /// Summary生成サービス
    summary_generator: Arc<SummaryGenerator>,
    /// GrandSummary生成サービス
    grand_summary_generator: Arc<GrandSummaryGenerator>,
    /// チェック間隔（秒）
    tick_interval_secs: u64,
}

impl SummaryScheduler {
    /// 新しいSummarySchedulerを作成
    pub fn new(
        schedule_repository: ScheduleRepository,
        summary_generator: Arc<SummaryGenerator>,
        grand_summary_generator: Arc<GrandSummaryGenerator>,
    ) -> Self {
        Self {
            schedule_repository,
            summary_generator,
            grand_summary_generator,
            tick_interval_secs: 60, // 1分間隔
        }
    }

    /// チェック間隔を設定（テスト用）
    pub fn with_tick_interval(mut self, secs: u64) -> Self {
        self.tick_interval_secs = secs;
        self
    }

    /// スケジューラ起動（バックグラウンドタスク）
    pub async fn start(self: Arc<Self>) {
        info!("Summary scheduler started");

        tokio::spawn(async move {
            loop {
                if let Err(e) = self.tick().await {
                    error!(error = %e, "Scheduler tick error");
                }

                tokio::time::sleep(TokioDuration::from_secs(self.tick_interval_secs)).await;
            }
        });
    }

    /// 1回のスケジューラティック
    pub async fn tick(&self) -> crate::Result<()> {
        let now = Utc::now();

        // 実行待ちのスケジュールを取得
        let due_schedules = self.schedule_repository.get_due_schedules(now).await?;

        if due_schedules.is_empty() {
            debug!("No due schedules");
            return Ok(());
        }

        debug!(count = due_schedules.len(), "Processing due schedules");

        for schedule in due_schedules {
            match schedule.report_type {
                ReportType::Summary => {
                    if let Err(e) = self.execute_summary(&schedule, now).await {
                        error!(
                            schedule_id = schedule.schedule_id,
                            error = %e,
                            "Failed to execute summary"
                        );
                    }
                }
                ReportType::GrandSummary => {
                    if let Err(e) = self.execute_grand_summary(&schedule, now).await {
                        error!(
                            schedule_id = schedule.schedule_id,
                            error = %e,
                            "Failed to execute grand summary"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Summary実行
    async fn execute_summary(
        &self,
        schedule: &ReportSchedule,
        now: DateTime<Utc>,
    ) -> crate::Result<()> {
        let interval = schedule.interval_minutes.unwrap_or(60);
        let period_end = now;
        let period_start = now - Duration::minutes(interval as i64);

        info!(
            tid = %schedule.tid,
            fid = %schedule.fid,
            interval = interval,
            "Executing scheduled summary"
        );

        // Summary生成
        let result = self
            .summary_generator
            .generate(&schedule.tid, &schedule.fid, period_start, period_end)
            .await?;

        // TODO: Paraclate APPへ送信（Phase 4で実装）
        // if let Some(json) = &result.summary_json {
        //     self.paraclate_client.send_summary(json.clone()).await?;
        // }

        // 次回実行時刻を更新
        let next_run = now + Duration::minutes(interval as i64);
        self.schedule_repository
            .update_last_run(schedule.schedule_id, now, next_run)
            .await?;

        info!(
            summary_id = result.summary_id,
            detection_count = result.detection_count,
            next_run = %next_run,
            "Summary executed successfully"
        );

        Ok(())
    }

    /// GrandSummary実行
    async fn execute_grand_summary(
        &self,
        schedule: &ReportSchedule,
        now: DateTime<Utc>,
    ) -> crate::Result<()> {
        // JSTで現在の日付を取得
        let date = now.date_naive();

        info!(
            tid = %schedule.tid,
            fid = %schedule.fid,
            date = %date,
            "Executing scheduled grand summary"
        );

        // GrandSummary生成
        let result = self
            .grand_summary_generator
            .generate(&schedule.tid, &schedule.fid, date)
            .await?;

        // TODO: Paraclate APPへ送信（Phase 4で実装）
        // if let Some(json) = &result.summary_json {
        //     self.paraclate_client.send_grand_summary(json.clone()).await?;
        // }

        // 次回実行時刻を計算
        let scheduled_times = schedule.scheduled_times.clone().unwrap_or_default();
        let next_run = if scheduled_times.is_empty() {
            // デフォルト: 翌日09:00
            now + Duration::days(1)
        } else {
            calculate_next_grand_summary_time(&scheduled_times, now)?
        };

        self.schedule_repository
            .update_last_run(schedule.schedule_id, now, next_run)
            .await?;

        info!(
            summary_id = result.summary_id,
            detection_count = result.detection_count,
            next_run = %next_run,
            "GrandSummary executed successfully"
        );

        Ok(())
    }

    /// デフォルトスケジュールを初期化
    pub async fn init_default_schedules(&self, tid: &str, fid: &str) -> crate::Result<()> {
        let now = Utc::now();

        // Summary（60分間隔）
        let summary_schedule = ReportSchedule {
            schedule_id: 0,
            tid: tid.to_string(),
            fid: fid.to_string(),
            report_type: ReportType::Summary,
            interval_minutes: Some(60),
            scheduled_times: None,
            last_run_at: None,
            next_run_at: Some(now + Duration::hours(1)),
            enabled: true,
        };
        self.schedule_repository.upsert(&summary_schedule).await?;

        // GrandSummary（09:00, 17:00, 21:00 JST）
        let scheduled_times = vec!["09:00".to_string(), "17:00".to_string(), "21:00".to_string()];
        let next_grand = calculate_next_grand_summary_time(&scheduled_times, now)?;

        let grand_summary_schedule = ReportSchedule {
            schedule_id: 0,
            tid: tid.to_string(),
            fid: fid.to_string(),
            report_type: ReportType::GrandSummary,
            interval_minutes: None,
            scheduled_times: Some(scheduled_times),
            last_run_at: None,
            next_run_at: Some(next_grand),
            enabled: true,
        };
        self.schedule_repository.upsert(&grand_summary_schedule).await?;

        info!(
            tid = %tid,
            fid = %fid,
            "Default schedules initialized"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 統合テストはT3-8で実装
}
