//! Summary API Routes
//!
//! Phase 3: Summary/GrandSummary (Issue #116)
//! T3-5: Summary API実装
//!
//! ## エンドポイント
//! - POST /api/summary/generate - 手動Summary生成
//! - POST /api/summary/force-hourly - 強制Hourly Summary生成（テスト用）
//! - POST /api/summary/force-grand - 強制GrandSummary生成（テスト用）
//! - GET /api/summary/latest - 最新Summary取得
//! - GET /api/summary/:id - Summary取得
//! - GET /api/summary/range - 期間指定Summary一覧
//! - GET /api/grand-summary/:date - 日付指定GrandSummary
//! - GET /api/grand-summary/latest - 最新GrandSummary
//! - GET /api/reports/schedule - スケジュール設定取得
//! - PUT /api/reports/schedule - スケジュール設定更新

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::realtime_hub::{HubMessage, SummaryReportMessage};
use crate::state::AppState;
use crate::summary_service::{
    ReportSchedule, ReportType, SummaryInsert, SummaryResult, SummaryType,
};

/// Summary API ルーター
pub fn summary_routes() -> Router<AppState> {
    Router::new()
        // Summary endpoints
        .route("/summary/generate", post(generate_summary))
        .route("/summary/force-hourly", post(force_generate_hourly))
        .route("/summary/force-grand", post(force_generate_grand))
        .route("/summary/latest", get(get_latest_summary))
        .route("/summary/:id", get(get_summary_by_id))
        .route("/summary/range", get(get_summaries_range))
        // GrandSummary endpoints
        .route("/grand-summary/latest", get(get_latest_grand_summary))
        .route("/grand-summary/:date", get(get_grand_summary_by_date))
        // Schedule endpoints
        .route("/reports/schedule", get(get_schedules))
        .route("/reports/schedule", put(update_schedule))
}

// ========================================
// Request/Response Types
// ========================================

/// Summary生成リクエスト
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSummaryRequest {
    pub fid: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Summary生成レスポンス
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSummaryResponse {
    pub summary_id: u64,
    pub summary_type: String,
    pub detection_count: i32,
    pub severity_max: i32,
    pub camera_ids: Vec<String>,
}

/// Summary取得レスポンス
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryResponse {
    pub summary_id: u64,
    pub tid: String,
    pub fid: String,
    pub summary_type: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub summary_text: String,
    pub summary_json: Option<serde_json::Value>,
    pub detection_count: i32,
    pub severity_max: i32,
    pub camera_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl From<SummaryResult> for SummaryResponse {
    fn from(result: SummaryResult) -> Self {
        Self {
            summary_id: result.summary_id,
            tid: result.tid,
            fid: result.fid,
            summary_type: format!("{:?}", result.summary_type).to_lowercase(),
            period_start: result.period_start,
            period_end: result.period_end,
            summary_text: result.summary_text,
            summary_json: result.summary_json,
            detection_count: result.detection_count,
            severity_max: result.severity_max,
            camera_ids: result.camera_ids,
            created_at: result.created_at,
        }
    }
}

/// 最新Summary取得クエリ
#[derive(Debug, Deserialize)]
pub struct LatestSummaryQuery {
    pub fid: Option<String>,
    #[serde(rename = "type")]
    pub summary_type: Option<String>,
}

/// Summary範囲取得クエリ
#[derive(Debug, Deserialize)]
pub struct SummaryRangeQuery {
    pub fid: Option<String>,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    #[serde(rename = "type")]
    pub summary_type: Option<String>,
    pub limit: Option<i64>,
}

/// Summary範囲レスポンス
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryRangeResponse {
    pub summaries: Vec<SummaryResponse>,
    pub total: usize,
    pub has_more: bool,
}

/// スケジュールレスポンス
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleResponse {
    pub schedule_id: u32,
    pub report_type: String,
    pub interval_minutes: Option<i32>,
    pub scheduled_times: Option<Vec<String>>,
    pub enabled: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
}

impl From<ReportSchedule> for ScheduleResponse {
    fn from(schedule: ReportSchedule) -> Self {
        Self {
            schedule_id: schedule.schedule_id,
            report_type: match schedule.report_type {
                ReportType::Summary => "summary".to_string(),
                ReportType::GrandSummary => "grand_summary".to_string(),
            },
            interval_minutes: schedule.interval_minutes,
            scheduled_times: schedule.scheduled_times,
            enabled: schedule.enabled,
            last_run_at: schedule.last_run_at,
            next_run_at: schedule.next_run_at,
        }
    }
}

/// スケジュール一覧レスポンス
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulesResponse {
    pub schedules: Vec<ScheduleResponse>,
}

/// スケジュール更新リクエスト
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScheduleRequest {
    pub report_type: String,
    pub interval_minutes: Option<i32>,
    pub scheduled_times: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

// ========================================
// Handlers
// ========================================

/// POST /api/summary/generate - 手動Summary生成
async fn generate_summary(
    State(state): State<AppState>,
    Json(req): Json<GenerateSummaryRequest>,
) -> impl IntoResponse {
    // TIDを設定から取得
    let tid = match get_config_tid(&state).await {
        Some(tid) => tid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "TID not configured"
                })),
            )
                .into_response()
        }
    };

    match state
        .summary_generator
        .generate(&tid, &req.fid, req.period_start, req.period_end)
        .await
    {
        Ok(result) => {
            let response = GenerateSummaryResponse {
                summary_id: result.summary_id,
                summary_type: format!("{:?}", result.summary_type).to_lowercase(),
                detection_count: result.detection_count,
                severity_max: result.severity_max,
                camera_ids: result.camera_ids,
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to generate summary");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "SUMMARY_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// GET /api/summary/latest - 最新Summary取得
async fn get_latest_summary(
    State(state): State<AppState>,
    Query(query): Query<LatestSummaryQuery>,
) -> impl IntoResponse {
    let tid = match get_config_tid(&state).await {
        Some(tid) => tid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "TID not configured"
                })),
            )
                .into_response()
        }
    };

    let summary_type = match query.summary_type.as_deref() {
        Some("daily") => SummaryType::Daily,
        Some("emergency") => SummaryType::Emergency,
        _ => SummaryType::Hourly,
    };

    match state
        .summary_repository
        .get_latest(&tid, query.fid.as_deref(), summary_type)
        .await
    {
        Ok(Some(result)) => Json(SummaryResponse::from(result)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error_code": "NOT_FOUND",
                "message": "No summary found"
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get latest summary");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "DATABASE_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// GET /api/summary/:id - Summary取得
async fn get_summary_by_id(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.summary_repository.get_by_id(id).await {
        Ok(Some(result)) => Json(SummaryResponse::from(result)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error_code": "NOT_FOUND",
                "message": "Summary not found"
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, summary_id = id, "Failed to get summary");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "DATABASE_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// GET /api/summary/range - 期間指定Summary一覧
async fn get_summaries_range(
    State(state): State<AppState>,
    Query(query): Query<SummaryRangeQuery>,
) -> impl IntoResponse {
    let tid = match get_config_tid(&state).await {
        Some(tid) => tid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "TID not configured"
                })),
            )
                .into_response()
        }
    };

    let summary_type = match query.summary_type.as_deref() {
        Some("daily") => Some(SummaryType::Daily),
        Some("emergency") => Some(SummaryType::Emergency),
        Some("hourly") => Some(SummaryType::Hourly),
        _ => None,
    };

    let limit = query.limit.unwrap_or(100) as u32;

    match state
        .summary_repository
        .get_range(&tid, query.fid.as_deref(), query.from, query.to, summary_type, limit)
        .await
    {
        Ok(results) => {
            let total = results.len();
            let has_more = total as u32 >= limit;
            let summaries = results.into_iter().map(SummaryResponse::from).collect();

            Json(SummaryRangeResponse {
                summaries,
                total,
                has_more,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get summaries range");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "DATABASE_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// GET /api/grand-summary/latest - 最新GrandSummary取得
async fn get_latest_grand_summary(State(state): State<AppState>) -> impl IntoResponse {
    let tid = match get_config_tid(&state).await {
        Some(tid) => tid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "TID not configured"
                })),
            )
                .into_response()
        }
    };

    match state.grand_summary_generator.get_latest(&tid, None).await {
        Ok(Some(result)) => Json(SummaryResponse::from(result)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error_code": "NOT_FOUND",
                "message": "No grand summary found"
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get latest grand summary");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "DATABASE_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// GET /api/grand-summary/:date - 日付指定GrandSummary取得
async fn get_grand_summary_by_date(
    State(state): State<AppState>,
    Path(date_str): Path<String>,
) -> impl IntoResponse {
    let tid = match get_config_tid(&state).await {
        Some(tid) => tid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "TID not configured"
                })),
            )
                .into_response()
        }
    };

    let fid = match get_config_fid(&state).await {
        Some(fid) => fid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "FID not configured"
                })),
            )
                .into_response()
        }
    };

    // YYYY-MM-DD形式のパース
    let date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error_code": "PARSE_ERROR",
                    "message": "Invalid date format. Use YYYY-MM-DD"
                })),
            )
                .into_response()
        }
    };

    match state
        .grand_summary_generator
        .get_by_date(&tid, &fid, date)
        .await
    {
        Ok(Some(result)) => Json(SummaryResponse::from(result)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error_code": "NOT_FOUND",
                "message": format!("No grand summary found for date: {}", date_str)
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, date = %date_str, "Failed to get grand summary by date");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "DATABASE_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// GET /api/reports/schedule - スケジュール設定取得
async fn get_schedules(State(state): State<AppState>) -> impl IntoResponse {
    let tid = match get_config_tid(&state).await {
        Some(tid) => tid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "TID not configured"
                })),
            )
                .into_response()
        }
    };

    let fid = match get_config_fid(&state).await {
        Some(fid) => fid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "FID not configured"
                })),
            )
                .into_response()
        }
    };

    match state.schedule_repository.get_schedules(&tid, &fid).await {
        Ok(schedules) => {
            let schedules = schedules.into_iter().map(ScheduleResponse::from).collect();
            Json(SchedulesResponse { schedules }).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get schedules");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "DATABASE_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// PUT /api/reports/schedule - スケジュール設定更新
async fn update_schedule(
    State(state): State<AppState>,
    Json(req): Json<UpdateScheduleRequest>,
) -> impl IntoResponse {
    let tid = match get_config_tid(&state).await {
        Some(tid) => tid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "TID not configured"
                })),
            )
                .into_response()
        }
    };

    let fid = match get_config_fid(&state).await {
        Some(fid) => fid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "FID not configured"
                })),
            )
                .into_response()
        }
    };

    let report_type = match req.report_type.as_str() {
        "summary" => ReportType::Summary,
        "grand_summary" => ReportType::GrandSummary,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error_code": "VALIDATION_ERROR",
                    "message": "Invalid report_type. Use 'summary' or 'grand_summary'"
                })),
            )
                .into_response()
        }
    };

    // 既存スケジュールを取得または新規作成
    let schedules = match state.schedule_repository.get_schedules(&tid, &fid).await {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "DATABASE_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    };

    let existing = schedules.iter().find(|s| s.report_type == report_type);

    let schedule = ReportSchedule {
        schedule_id: existing.map(|s| s.schedule_id).unwrap_or(0),
        tid: tid.clone(),
        fid: fid.clone(),
        report_type,
        interval_minutes: req.interval_minutes.or(existing.and_then(|s| s.interval_minutes)),
        scheduled_times: req.scheduled_times.or(existing.and_then(|s| s.scheduled_times.clone())),
        last_run_at: existing.and_then(|s| s.last_run_at),
        next_run_at: existing.and_then(|s| s.next_run_at),
        enabled: req.enabled.unwrap_or(existing.map(|s| s.enabled).unwrap_or(true)),
    };

    match state.schedule_repository.upsert(&schedule).await {
        Ok(_) => Json(serde_json::json!({
            "message": "Schedule updated successfully"
        }))
        .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to update schedule");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "DATABASE_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

// ========================================
// Helper Functions
// ========================================

/// ConfigStoreからTIDを取得
async fn get_config_tid(state: &AppState) -> Option<String> {
    state
        .config_store
        .service()
        .get_setting("aranea.tid")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.as_str().map(String::from))
}

/// ConfigStoreからFIDを取得
async fn get_config_fid(state: &AppState) -> Option<String> {
    state
        .config_store
        .service()
        .get_setting("aranea_fid")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.as_str().map(String::from))
}

// ========================================
// Force Generate APIs (テスト用)
// ========================================

/// 強制生成リクエスト
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceGenerateRequest {
    /// 対象時間（時間単位）
    #[serde(default = "default_hours")]
    pub hours: u32,
}

fn default_hours() -> u32 {
    1
}

/// 強制生成レスポンス
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceGenerateResponse {
    pub summary_id: u64,
    pub summary_type: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub detection_count: i32,
    pub severity_max: i32,
    pub camera_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub message: String,
}

/// POST /api/summary/force-hourly - 強制Hourly Summary生成
///
/// 直近N時間のサマリーを強制生成（スケジュール無視）
/// テスト・デバッグ用途
async fn force_generate_hourly(
    State(state): State<AppState>,
    Json(req): Json<ForceGenerateRequest>,
) -> impl IntoResponse {
    let tid = match get_config_tid(&state).await {
        Some(tid) => tid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "TID not configured"
                })),
            )
                .into_response()
        }
    };

    let fid = match get_config_fid(&state).await {
        Some(fid) => fid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "FID not configured"
                })),
            )
                .into_response()
        }
    };

    // 直近N時間を計算
    let now = Utc::now();
    let period_start = now - chrono::Duration::hours(req.hours as i64);
    let period_end = now;

    tracing::info!(
        tid = %tid,
        fid = %fid,
        hours = req.hours,
        period_start = %period_start,
        period_end = %period_end,
        "Force generating hourly summary"
    );

    match state
        .summary_generator
        .generate(&tid, &fid, period_start, period_end)
        .await
    {
        Ok(result) => {
            // Paraclate APPへ送信（Ingest API）
            if let Some(json) = &result.summary_json {
                match state
                    .paraclate_client
                    .send_summary(&result.tid, &result.fid, json.clone(), result.summary_id)
                    .await
                {
                    Ok(queue_id) => {
                        tracing::info!(
                            queue_id = queue_id,
                            summary_id = result.summary_id,
                            "Force-hourly summary queued for Paraclate APP"
                        );
                    }
                    Err(e) => {
                        // 送信失敗してもSummary生成自体は成功扱い
                        tracing::warn!(
                            error = %e,
                            summary_id = result.summary_id,
                            "Failed to queue force-hourly summary for Paraclate APP"
                        );
                    }
                }
            }

            // Paraclate LLM APIでリッチなサマリーテキストを取得
            let llm_summary_text = match state
                .paraclate_client
                .send_ai_chat(
                    &tid,
                    &fid,
                    &format!("直近{}時間の定時サマリーを生成してください", req.hours),
                    None,
                    None,
                )
                .await
            {
                Ok(response) => {
                    if let Some(text) = response.message {
                        tracing::info!(
                            summary_id = result.summary_id,
                            llm_text_len = text.len(),
                            "LLM summary received from Paraclate (force-hourly)"
                        );
                        Some(text)
                    } else {
                        tracing::warn!(
                            summary_id = result.summary_id,
                            "No message in LLM response, using local summary"
                        );
                        None
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        summary_id = result.summary_id,
                        "Failed to get LLM summary, using local summary"
                    );
                    None
                }
            };

            // チャットへ報告（RealtimeHub経由）
            // LLMサマリーがあればそれを使用、なければローカル生成のサマリーを使用
            let final_summary_text = llm_summary_text.unwrap_or_else(|| result.summary_text.clone());
            state.realtime.broadcast(HubMessage::SummaryReport(SummaryReportMessage {
                report_type: "summary".to_string(),
                summary_id: result.summary_id,
                period_start: result.period_start.to_rfc3339(),
                period_end: result.period_end.to_rfc3339(),
                detection_count: result.detection_count,
                severity_max: result.severity_max,
                camera_count: result.camera_ids.len(),
                summary_text: final_summary_text,
                created_at: Utc::now().to_rfc3339(),
            })).await;

            let response = ForceGenerateResponse {
                summary_id: result.summary_id,
                summary_type: "hourly".to_string(),
                period_start: result.period_start,
                period_end: result.period_end,
                detection_count: result.detection_count,
                severity_max: result.severity_max,
                camera_ids: result.camera_ids,
                created_at: result.created_at,
                message: format!(
                    "直近{}時間のサマリーを生成しました（検出: {}件）",
                    req.hours, result.detection_count
                ),
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to force generate hourly summary");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "SUMMARY_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// POST /api/summary/force-grand - 強制GrandSummary生成
///
/// 直近6時間のGrandSummaryを強制生成（スケジュール無視）
/// テスト・デバッグ用途
async fn force_generate_grand(
    State(state): State<AppState>,
    Json(req): Json<ForceGenerateRequest>,
) -> impl IntoResponse {
    let tid = match get_config_tid(&state).await {
        Some(tid) => tid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "TID not configured"
                })),
            )
                .into_response()
        }
    };

    let fid = match get_config_fid(&state).await {
        Some(fid) => fid,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "CONFIG_ERROR",
                    "message": "FID not configured"
                })),
            )
                .into_response()
        }
    };

    // デフォルトは6時間
    let hours = if req.hours == 0 { 6 } else { req.hours };

    // 直近N時間を計算
    let now = Utc::now();
    let period_start = now - chrono::Duration::hours(hours as i64);
    let period_end = now;

    tracing::info!(
        tid = %tid,
        fid = %fid,
        hours = hours,
        period_start = %period_start,
        period_end = %period_end,
        "Force generating GrandSummary"
    );

    // GrandSummaryは既存のHourlySummaryを集計する
    // まず対象期間のHourlySummaryを取得
    let summaries = match state
        .summary_repository
        .get_by_period(&tid, &fid, SummaryType::Hourly, period_start, period_end)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get hourly summaries for grand summary");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "DATABASE_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response();
        }
    };

    // 集計
    let total_detection_count: i32 = summaries.iter().map(|s| s.detection_count).sum();
    let max_severity: i32 = summaries.iter().map(|s| s.severity_max).max().unwrap_or(0);
    let all_camera_ids: std::collections::HashSet<String> = summaries
        .iter()
        .flat_map(|s| s.camera_ids.clone())
        .collect();
    let included_summary_ids: Vec<u64> = summaries.iter().map(|s| s.summary_id).collect();

    // GrandSummary用のサマリーテキスト生成
    let summary_text = format!(
        "直近{}時間のGrandSummary: {}件の検出（{}件のHourlySummaryを集計）",
        hours,
        total_detection_count,
        summaries.len()
    );

    // summary_jsonを構築
    let summary_json = serde_json::json!({
        "type": "grand_summary",
        "periodHours": hours,
        "includedSummaryIds": included_summary_ids,
        "totalDetectionCount": total_detection_count,
        "severityMax": max_severity,
        "cameraCount": all_camera_ids.len(),
        "hourlySummaryCount": summaries.len()
    });

    // DBに保存
    let insert_data = SummaryInsert {
        tid: tid.clone(),
        fid: fid.clone(),
        summary_type: SummaryType::Daily,  // GrandSummaryはDailyタイプとして保存
        period_start,
        period_end,
        summary_text: summary_text.clone(),
        summary_json: Some(summary_json.clone()),
        detection_count: total_detection_count,
        severity_max: max_severity,
        camera_ids: serde_json::json!(all_camera_ids.iter().cloned().collect::<Vec<_>>()),
        expires_at: Utc::now() + chrono::Duration::days(90),  // 90日間保持
    };
    match state
        .summary_repository
        .insert(insert_data)
        .await
    {
        Ok(result) => {
            // Paraclate APPへ送信（Ingest API）
            match state
                .paraclate_client
                .send_grand_summary(&tid, &fid, summary_json.clone(), result.summary_id)
                .await
            {
                Ok(queue_id) => {
                    tracing::info!(
                        queue_id = queue_id,
                        summary_id = result.summary_id,
                        "Force-grand summary queued for Paraclate APP"
                    );
                }
                Err(e) => {
                    // 送信失敗してもGrandSummary生成自体は成功扱い
                    tracing::warn!(
                        error = %e,
                        summary_id = result.summary_id,
                        "Failed to queue force-grand summary for Paraclate APP"
                    );
                }
            }

            // Paraclate LLM APIでリッチなGrandSummaryテキストを取得
            let llm_summary_text = match state
                .paraclate_client
                .send_ai_chat(
                    &tid,
                    &fid,
                    &format!("直近{}時間のシフトサマリー（GrandSummary）を生成してください", hours),
                    None,
                    None,
                )
                .await
            {
                Ok(response) => {
                    if let Some(text) = response.message {
                        tracing::info!(
                            summary_id = result.summary_id,
                            llm_text_len = text.len(),
                            "LLM grand summary received from Paraclate (force-grand)"
                        );
                        Some(text)
                    } else {
                        tracing::warn!(
                            summary_id = result.summary_id,
                            "No message in LLM response, using local grand summary"
                        );
                        None
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        summary_id = result.summary_id,
                        "Failed to get LLM grand summary, using local summary"
                    );
                    None
                }
            };

            // チャットへ報告（RealtimeHub経由）
            // LLMサマリーがあればそれを使用、なければローカル生成のサマリーを使用
            let final_summary_text = llm_summary_text.unwrap_or_else(|| summary_text.clone());
            state.realtime.broadcast(HubMessage::SummaryReport(SummaryReportMessage {
                report_type: "grand_summary".to_string(),
                summary_id: result.summary_id,
                period_start: period_start.to_rfc3339(),
                period_end: period_end.to_rfc3339(),
                detection_count: total_detection_count,
                severity_max: max_severity,
                camera_count: all_camera_ids.len(),
                summary_text: final_summary_text,
                created_at: Utc::now().to_rfc3339(),
            })).await;

            let response = ForceGenerateResponse {
                summary_id: result.summary_id,
                summary_type: "grand_summary".to_string(),
                period_start,
                period_end,
                detection_count: total_detection_count,
                severity_max: max_severity,
                camera_ids: all_camera_ids.into_iter().collect(),
                created_at: Utc::now(),
                message: format!(
                    "直近{}時間のGrandSummaryを生成しました（検出: {}件, Hourly: {}件）",
                    hours, total_detection_count, summaries.len()
                ),
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to save grand summary");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error_code": "DATABASE_ERROR",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}
