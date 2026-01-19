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
use crate::paraclate_client::ParaclateClient;
use crate::realtime_hub::{HubMessage, RealtimeHub, SummaryReportMessage};
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
    /// Paraclate APPクライアント（Ingest API送信用）
    paraclate_client: Arc<ParaclateClient>,
    /// RealtimeHub（チャットへの報告用）
    realtime: Arc<RealtimeHub>,
    /// チェック間隔（秒）
    tick_interval_secs: u64,
}

impl SummaryScheduler {
    /// 新しいSummarySchedulerを作成
    pub fn new(
        schedule_repository: ScheduleRepository,
        summary_generator: Arc<SummaryGenerator>,
        grand_summary_generator: Arc<GrandSummaryGenerator>,
        paraclate_client: Arc<ParaclateClient>,
        realtime: Arc<RealtimeHub>,
    ) -> Self {
        Self {
            schedule_repository,
            summary_generator,
            grand_summary_generator,
            paraclate_client,
            realtime,
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

        // Paraclate APPへ送信（Ingest API）
        if let Some(json) = &result.summary_json {
            match self
                .paraclate_client
                .send_summary(&result.tid, &result.fid, json.clone(), result.summary_id)
                .await
            {
                Ok(queue_id) => {
                    info!(
                        queue_id = queue_id,
                        summary_id = result.summary_id,
                        "Summary queued for Paraclate APP"
                    );
                }
                Err(e) => {
                    // 送信失敗してもSummary生成自体は成功扱い
                    warn!(
                        error = %e,
                        summary_id = result.summary_id,
                        "Failed to queue summary for Paraclate APP"
                    );
                }
            }
        }

        // Paraclate LLM APIでリッチなサマリーテキストを取得
        // mobes_SummaryArchitecture_Analysis_20260114.md準拠
        let llm_summary_text = match self
            .paraclate_client
            .send_ai_chat(
                &schedule.tid,
                &schedule.fid,
                &format!("直近{}分間の定時サマリーを生成してください", interval),
                None,
                None,
            )
            .await
        {
            Ok(response) => {
                if let Some(text) = response.message {
                    info!(
                        summary_id = result.summary_id,
                        llm_text_len = text.len(),
                        "LLM summary received from Paraclate"
                    );
                    Some(text)
                } else {
                    warn!(
                        summary_id = result.summary_id,
                        "No message in LLM response, using local summary"
                    );
                    None
                }
            }
            Err(e) => {
                warn!(
                    error = %e,
                    summary_id = result.summary_id,
                    "Failed to get LLM summary, using local summary"
                );
                None
            }
        };

        // チャットへ報告（RealtimeHub経由）
        // LLMサマリーがある場合のみ送信（IS22ローカル生成サマリーは使用しない）
        if let Some(llm_text) = llm_summary_text {
            self.realtime.broadcast(HubMessage::SummaryReport(SummaryReportMessage {
                report_type: "summary".to_string(),
                summary_id: result.summary_id,
                period_start: result.period_start.to_rfc3339(),
                period_end: result.period_end.to_rfc3339(),
                detection_count: result.detection_count,
                severity_max: result.severity_max,
                camera_count: result.camera_ids.len(),
                summary_text: llm_text,
                created_at: Utc::now().to_rfc3339(),
            })).await;
        } else {
            warn!(
                summary_id = result.summary_id,
                "Skipping chat broadcast - no LLM summary available"
            );
        }

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

    /// GrandSummary実行（シフト単位集計）
    async fn execute_grand_summary(
        &self,
        schedule: &ReportSchedule,
        now: DateTime<Utc>,
    ) -> crate::Result<()> {
        // シフト開始時刻 = 前回実行時刻（初回はデフォルトで8時間前）
        let period_start = schedule.last_run_at.unwrap_or_else(|| {
            // 初回実行時は8時間前をデフォルトとする
            now - Duration::hours(8)
        });
        let period_end = now;

        info!(
            tid = %schedule.tid,
            fid = %schedule.fid,
            period_start = %period_start,
            period_end = %period_end,
            "Executing scheduled grand summary (shift-based)"
        );

        // GrandSummary生成（シフト単位）
        let result = self
            .grand_summary_generator
            .generate(&schedule.tid, &schedule.fid, period_start, period_end)
            .await?;

        // Paraclate APPへ送信（Ingest API）
        if let Some(json) = &result.summary_json {
            match self
                .paraclate_client
                .send_grand_summary(&result.tid, &result.fid, json.clone(), result.summary_id)
                .await
            {
                Ok(queue_id) => {
                    info!(
                        queue_id = queue_id,
                        summary_id = result.summary_id,
                        "GrandSummary queued for Paraclate APP"
                    );
                }
                Err(e) => {
                    // 送信失敗してもGrandSummary生成自体は成功扱い
                    warn!(
                        error = %e,
                        summary_id = result.summary_id,
                        "Failed to queue grand summary for Paraclate APP"
                    );
                }
            }
        }

        // シフト時間を計算
        let shift_hours = (period_end - period_start).num_hours();

        // Paraclate LLM APIでリッチなGrandSummaryテキストを取得
        // mobes_SummaryArchitecture_Analysis_20260114.md準拠
        let llm_summary_text = match self
            .paraclate_client
            .send_ai_chat(
                &schedule.tid,
                &schedule.fid,
                &format!("直近{}時間のシフトサマリー（GrandSummary）を生成してください", shift_hours),
                None,
                None,
            )
            .await
        {
            Ok(response) => {
                if let Some(text) = response.message {
                    info!(
                        summary_id = result.summary_id,
                        llm_text_len = text.len(),
                        "LLM grand summary received from Paraclate"
                    );
                    Some(text)
                } else {
                    warn!(
                        summary_id = result.summary_id,
                        "No message in LLM response, using local grand summary"
                    );
                    None
                }
            }
            Err(e) => {
                warn!(
                    error = %e,
                    summary_id = result.summary_id,
                    "Failed to get LLM grand summary, using local summary"
                );
                None
            }
        };

        // チャットへ報告（RealtimeHub経由）
        // LLMサマリーがある場合のみ送信（IS22ローカル生成サマリーは使用しない）
        if let Some(llm_text) = llm_summary_text {
            self.realtime.broadcast(HubMessage::SummaryReport(SummaryReportMessage {
                report_type: "grand_summary".to_string(),
                summary_id: result.summary_id,
                period_start: result.period_start.to_rfc3339(),
                period_end: result.period_end.to_rfc3339(),
                detection_count: result.detection_count,
                severity_max: result.severity_max,
                camera_count: result.camera_ids.len(),
                summary_text: llm_text,
                created_at: Utc::now().to_rfc3339(),
            })).await;
        } else {
            warn!(
                summary_id = result.summary_id,
                "Skipping chat broadcast - no LLM grand summary available"
            );
        }

        // 次回実行時刻を計算（configから最新の時刻設定を取得）
        let scheduled_times = match self.paraclate_client.get_config(&schedule.tid, &schedule.fid).await {
            Ok(Some(config)) if !config.grand_summary_times.is_empty() => {
                config.grand_summary_times
            }
            _ => schedule.scheduled_times.clone().unwrap_or_else(|| {
                vec!["09:00".to_string(), "17:00".to_string(), "21:00".to_string()]
            }),
        };
        let next_run = calculate_next_grand_summary_time(&scheduled_times, now)?;

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

        // GrandSummary時刻をconfigから取得（なければデフォルト）
        let scheduled_times = match self.paraclate_client.get_config(tid, fid).await {
            Ok(Some(config)) => {
                if config.grand_summary_times.is_empty() {
                    vec!["09:00".to_string(), "17:00".to_string(), "21:00".to_string()]
                } else {
                    config.grand_summary_times
                }
            }
            _ => vec!["09:00".to_string(), "17:00".to_string(), "21:00".to_string()],
        };
        debug!(
            tid = %tid,
            fid = %fid,
            times = ?scheduled_times,
            "GrandSummary times from config"
        );
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
