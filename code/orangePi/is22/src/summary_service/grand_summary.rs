//! GrandSummary Generator
//!
//! Phase 3: Summary/GrandSummary (Issue #116)
//! T3-6, T3-7: GrandSummary設計・実装
//!
//! ## 概要
//! 複数のhourly Summaryを統合し、シフト単位のGrandSummaryを生成する。
//!
//! ## シフト単位集計
//! GrandSummaryは「前回実行時刻〜今回実行時刻」の期間でSummaryを集計する。
//! 例: scheduled_times = ["08:00", "15:00", "22:00"] の場合
//!   - 08:00実行 → 前日22:00〜当日08:00のSummaryを集計
//!   - 15:00実行 → 当日08:00〜15:00のSummaryを集計
//!   - 22:00実行 → 当日15:00〜22:00のSummaryを集計
//!
//! ## タイムゾーン
//! - scheduled_times（09:00等）はAsia/Tokyo（JST）基準

use super::repository::SummaryRepository;
use super::types::{SummaryInsert, SummaryResult, SummaryType};
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Asia::Tokyo;
use std::collections::HashSet;
use tracing::{debug, info, warn};

/// GrandSummary生成サービス
pub struct GrandSummaryGenerator {
    repository: SummaryRepository,
}

impl GrandSummaryGenerator {
    /// 新しいGrandSummaryGeneratorを作成
    pub fn new(repository: SummaryRepository) -> Self {
        Self { repository }
    }

    /// GrandSummary生成（シフト単位でSummaryを統合）
    ///
    /// # Arguments
    /// * `tid` - テナントID
    /// * `fid` - 施設ID
    /// * `period_start` - シフト開始時刻（前回実行時刻）
    /// * `period_end` - シフト終了時刻（今回実行時刻）
    ///
    /// # シフト単位集計
    /// scheduled_times = ["08:00", "15:00", "22:00"] の場合：
    ///   - 08:00実行 → 前日22:00〜当日08:00のSummaryを集計
    ///   - 15:00実行 → 当日08:00〜15:00のSummaryを集計
    ///   - 22:00実行 → 当日15:00〜22:00のSummaryを集計
    pub async fn generate(
        &self,
        tid: &str,
        fid: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> crate::Result<SummaryResult> {
        // JSTで期間を表示用にフォーマット
        let start_jst = period_start.with_timezone(&Tokyo);
        let end_jst = period_end.with_timezone(&Tokyo);

        info!(
            tid = %tid,
            fid = %fid,
            period_start = %start_jst.format("%Y-%m-%d %H:%M"),
            period_end = %end_jst.format("%Y-%m-%d %H:%M"),
            "Generating GrandSummary (shift-based)"
        );

        // 1. 指定期間のhourly Summaryを取得
        let hourly_summaries = self
            .repository
            .get_by_period(tid, fid, SummaryType::Hourly, period_start, period_end)
            .await?;

        if hourly_summaries.is_empty() {
            return self.create_empty_grand_summary(tid, fid, period_start, period_end).await;
        }

        // 2. 統合情報を算出
        let mut total_detection_count = 0i32;
        let mut severity_max = 0i32;
        let mut all_camera_ids: HashSet<String> = HashSet::new();
        let mut included_summary_ids: Vec<u64> = Vec::new();

        for summary in &hourly_summaries {
            total_detection_count += summary.detection_count;
            severity_max = severity_max.max(summary.severity_max);

            for camera_id in &summary.camera_ids {
                all_camera_ids.insert(camera_id.clone());
            }

            included_summary_ids.push(summary.summary_id);
        }

        // 3. 統合summary_text生成（シフト単位）
        let summary_text = format!(
            "シフトサマリー（{} 〜 {}）: 合計{}件の検出（{}台のカメラ）\n{}個のhourlyサマリーを統合",
            start_jst.format("%m/%d %H:%M"),
            end_jst.format("%H:%M"),
            total_detection_count,
            all_camera_ids.len(),
            hourly_summaries.len()
        );

        // 4. summary_json生成
        let summary_json = serde_json::json!({
            "type": "grand_summary",
            "shiftStart": period_start.to_rfc3339(),
            "shiftEnd": period_end.to_rfc3339(),
            "includedSummaryIds": included_summary_ids,
            "totalDetectionCount": total_detection_count,
            "severityMax": severity_max,
            "cameraCount": all_camera_ids.len(),
            "hourlySummaryCount": hourly_summaries.len()
        });

        // 5. DBに保存（同一期間のGrandSummaryが既存の場合は更新）
        let camera_ids_vec: Vec<String> = all_camera_ids.into_iter().collect();
        let result = self
            .repository
            .upsert(SummaryInsert {
                tid: tid.to_string(),
                fid: fid.to_string(),
                summary_type: SummaryType::Daily,
                period_start,
                period_end,
                summary_text,
                summary_json: Some(summary_json),
                detection_count: total_detection_count,
                severity_max,
                camera_ids: serde_json::to_value(&camera_ids_vec)
                    .map_err(|e| crate::Error::Parse(e.to_string()))?,
                expires_at: Utc::now() + Duration::days(90),
            })
            .await?;

        info!(
            summary_id = result.summary_id,
            period = %format!("{} - {}", start_jst.format("%m/%d %H:%M"), end_jst.format("%H:%M")),
            detection_count = total_detection_count,
            included_count = included_summary_ids.len(),
            "GrandSummary generated successfully"
        );

        Ok(result)
    }

    /// 空のGrandSummaryを生成（hourly Summaryなしの場合）
    async fn create_empty_grand_summary(
        &self,
        tid: &str,
        fid: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> crate::Result<SummaryResult> {
        let start_jst = period_start.with_timezone(&Tokyo);
        let end_jst = period_end.with_timezone(&Tokyo);

        let summary_text = format!(
            "シフトサマリー（{} 〜 {}）: 検出イベントなし",
            start_jst.format("%m/%d %H:%M"),
            end_jst.format("%H:%M")
        );

        let result = self
            .repository
            .upsert(SummaryInsert {
                tid: tid.to_string(),
                fid: fid.to_string(),
                summary_type: SummaryType::Daily,
                period_start,
                period_end,
                summary_text,
                summary_json: Some(serde_json::json!({
                    "type": "grand_summary",
                    "shiftStart": period_start.to_rfc3339(),
                    "shiftEnd": period_end.to_rfc3339(),
                    "includedSummaryIds": [],
                    "totalDetectionCount": 0,
                    "severityMax": 0,
                    "cameraCount": 0,
                    "hourlySummaryCount": 0
                })),
                detection_count: 0,
                severity_max: 0,
                camera_ids: serde_json::json!([]),
                expires_at: Utc::now() + Duration::days(90),
            })
            .await?;

        debug!(
            summary_id = result.summary_id,
            period = %format!("{} - {}", start_jst.format("%m/%d %H:%M"), end_jst.format("%H:%M")),
            "Empty GrandSummary created"
        );

        Ok(result)
    }

    /// 最新のGrandSummaryを取得
    pub async fn get_latest(
        &self,
        tid: &str,
        fid: Option<&str>,
    ) -> crate::Result<Option<SummaryResult>> {
        self.repository.get_latest(tid, fid, SummaryType::Daily).await
    }

    /// 日付指定でGrandSummaryを取得
    pub async fn get_by_date(
        &self,
        tid: &str,
        fid: &str,
        date: NaiveDate,
    ) -> crate::Result<Option<SummaryResult>> {
        let period_start_jst = date
            .and_hms_opt(0, 0, 0)
            .expect("Invalid time")
            .and_local_timezone(Tokyo)
            .single()
            .expect("Invalid timezone conversion");
        let period_start = period_start_jst.with_timezone(&Utc);
        let period_end = period_start + Duration::days(1);

        let summaries = self
            .repository
            .get_by_period(tid, fid, SummaryType::Daily, period_start, period_end)
            .await?;

        Ok(summaries.into_iter().next())
    }
}

/// 次のGrandSummary実行時刻を計算（Asia/Tokyo基準）
///
/// # Arguments
/// * `scheduled_times` - 実行時刻リスト（例: ["09:00", "17:00", "21:00"]）
/// * `now` - 現在時刻（UTC）
///
/// # Returns
/// 次の実行時刻（UTC）
pub fn calculate_next_grand_summary_time(
    scheduled_times: &[String],
    now: DateTime<Utc>,
) -> crate::Result<DateTime<Utc>> {
    if scheduled_times.is_empty() {
        return Err(crate::Error::Config("No scheduled times configured".into()));
    }

    // 現在時刻をJSTに変換
    let now_jst = now.with_timezone(&Tokyo);
    let today_jst = now_jst.date_naive();

    // 本日の残り時刻をチェック（JST基準）
    for time_str in scheduled_times {
        let time = NaiveTime::parse_from_str(time_str, "%H:%M")
            .map_err(|e| crate::Error::Parse(format!("Invalid time format: {}", e)))?;

        let scheduled_jst = today_jst
            .and_time(time)
            .and_local_timezone(Tokyo)
            .single()
            .ok_or_else(|| crate::Error::Parse("Invalid timezone conversion".into()))?;

        if scheduled_jst > now_jst {
            return Ok(scheduled_jst.with_timezone(&Utc));
        }
    }

    // 翌日の最初の時刻（JST基準）
    let tomorrow_jst = today_jst + Duration::days(1);
    let first_time = NaiveTime::parse_from_str(&scheduled_times[0], "%H:%M")
        .map_err(|e| crate::Error::Parse(format!("Invalid time format: {}", e)))?;

    let next_jst = tomorrow_jst
        .and_time(first_time)
        .and_local_timezone(Tokyo)
        .single()
        .ok_or_else(|| crate::Error::Parse("Invalid timezone conversion".into()))?;

    Ok(next_jst.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_next_time_today() {
        // 2026-01-10 10:00 JST = 2026-01-10 01:00 UTC
        let now = Utc.with_ymd_and_hms(2026, 1, 10, 1, 0, 0).unwrap();
        let times = vec!["09:00".to_string(), "17:00".to_string(), "21:00".to_string()];

        let next = calculate_next_grand_summary_time(&times, now).unwrap();

        // 次は17:00 JST = 08:00 UTC
        assert_eq!(next.hour(), 8);
    }

    #[test]
    fn test_calculate_next_time_tomorrow() {
        // 2026-01-10 22:00 JST = 2026-01-10 13:00 UTC（21:00 JST過ぎ）
        let now = Utc.with_ymd_and_hms(2026, 1, 10, 13, 0, 0).unwrap();
        let times = vec!["09:00".to_string(), "17:00".to_string(), "21:00".to_string()];

        let next = calculate_next_grand_summary_time(&times, now).unwrap();

        // 次は翌日09:00 JST = 翌日00:00 UTC
        assert_eq!(next.day(), 11);
        assert_eq!(next.hour(), 0);
    }
}
