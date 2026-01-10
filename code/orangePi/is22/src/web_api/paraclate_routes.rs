//! Paraclate API Routes
//!
//! Phase 4: ParaclateClient (Issue #117)
//! DD03_ParaclateClient.md準拠
//!
//! ## エンドポイント
//! - GET /api/paraclate/status - 接続状態取得
//! - POST /api/paraclate/connect - 接続開始
//! - POST /api/paraclate/disconnect - 切断
//! - GET /api/paraclate/config - 設定取得
//! - PUT /api/paraclate/config - 設定更新
//! - GET /api/paraclate/queue - キュー一覧
//! - POST /api/paraclate/queue/process - キュー処理実行
//! - POST /api/paraclate/pubsub/push - Pub/Sub Push通知受信 (T4-7)
//! - POST /api/paraclate/notify - 直接通知受信 (T4-7)
//!
//! ## セキュリティ (Issue #119)
//! - 全エンドポイントでFID所属検証を実施
//! - SKIP_FID_VALIDATION=1 で検証をスキップ可能（開発用）

use axum::{
    extract::{Query, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;

use crate::paraclate_client::{
    ConfigResponse, ConfigUpdateNotification, ConnectRequest, ConnectResponse,
    FidValidator, PubSubPushMessage, QueueListResponse, QueueStatus, StatusResponse,
    UpdateConfigRequest,
};
use crate::state::AppState;
use crate::Result;

/// Paraclate APIルートを作成
pub fn paraclate_routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(get_status))
        .route("/connect", post(connect))
        .route("/disconnect", post(disconnect))
        .route("/config", get(get_config))
        .route("/config", put(update_config))
        .route("/queue", get(list_queue))
        .route("/queue/process", post(process_queue))
        // T4-7: Pub/Sub受信フロー
        .route("/pubsub/push", post(handle_pubsub_push))
        .route("/notify", post(handle_direct_notification))
}

/// TIDクエリパラメータ（将来のマルチテナント対応用）
#[derive(Debug, Deserialize)]
pub struct TidQuery {
    #[serde(default = "default_tid")]
    pub tid: String,
    #[serde(default = "default_fid")]
    pub fid: String,
}

fn default_tid() -> String {
    std::env::var("DEFAULT_TID").unwrap_or_else(|_| "T0000000000000000000".to_string())
}

fn default_fid() -> String {
    std::env::var("DEFAULT_FID").unwrap_or_else(|_| "0000".to_string())
}

/// キュー一覧クエリパラメータ
#[derive(Debug, Deserialize)]
pub struct QueueQuery {
    #[serde(default = "default_tid")]
    pub tid: String,
    #[serde(default = "default_fid")]
    pub fid: String,
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_limit() -> i32 {
    50
}

/// FID所属検証ヘルパー (Issue #119)
///
/// FIDがデバイスの登録TIDに所属しているか検証する。
/// SKIP_FID_VALIDATION=1 環境変数で検証をスキップ可能。
async fn validate_fid(state: &AppState, fid: &str) -> Result<()> {
    // 開発/テスト用スキップ
    if FidValidator::should_skip_validation() {
        tracing::debug!(fid = %fid, "FID validation skipped (SKIP_FID_VALIDATION=1)");
        return Ok(());
    }

    state
        .fid_validator
        .validate_fid(fid)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, fid = %fid, "FID validation failed");
            crate::Error::Forbidden(e.to_string())
        })
}

/// GET /api/paraclate/status
///
/// 接続状態を取得
async fn get_status(
    State(state): State<AppState>,
    Query(query): Query<TidQuery>,
) -> Result<Json<StatusResponse>> {
    // Issue #119: FID所属検証
    validate_fid(&state, &query.fid).await?;

    let status = state
        .paraclate_client
        .get_status(&query.tid, &query.fid)
        .await?;

    Ok(Json(status))
}

/// POST /api/paraclate/connect
///
/// Paraclate APPに接続
async fn connect(
    State(state): State<AppState>,
    Query(query): Query<TidQuery>,
    Json(body): Json<ConnectRequest>,
) -> Result<Json<ConnectResponse>> {
    // Issue #119: FID所属検証
    validate_fid(&state, &body.fid).await?;

    let result = state
        .paraclate_client
        .connect(&query.tid, &body.fid, &body.endpoint)
        .await?;

    Ok(Json(result))
}

/// POST /api/paraclate/disconnect
///
/// Paraclate APPから切断
async fn disconnect(
    State(state): State<AppState>,
    Query(query): Query<TidQuery>,
) -> Result<Json<serde_json::Value>> {
    // Issue #119: FID所属検証
    validate_fid(&state, &query.fid).await?;

    state
        .paraclate_client
        .disconnect(&query.tid, &query.fid)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Disconnected from Paraclate APP"
    })))
}

/// GET /api/paraclate/config
///
/// 設定を取得
async fn get_config(
    State(state): State<AppState>,
    Query(query): Query<TidQuery>,
) -> Result<Json<serde_json::Value>> {
    // Issue #119: FID所属検証
    validate_fid(&state, &query.fid).await?;

    let config = state
        .paraclate_client
        .get_config(&query.tid, &query.fid)
        .await?;

    if let Some(c) = config {
        Ok(Json(serde_json::to_value(c).unwrap()))
    } else {
        Ok(Json(serde_json::json!({
            "error": "Config not found",
            "message": "Please connect to Paraclate APP first"
        })))
    }
}

/// PUT /api/paraclate/config
///
/// 設定を更新
async fn update_config(
    State(state): State<AppState>,
    Query(query): Query<TidQuery>,
    Json(body): Json<UpdateConfigRequest>,
) -> Result<Json<ConfigResponse>> {
    // Issue #119: FID所属検証
    validate_fid(&state, &body.fid).await?;

    use crate::paraclate_client::ParaclateConfigUpdate;

    let update = ParaclateConfigUpdate {
        endpoint: None,
        report_interval_minutes: body.report_interval_minutes,
        grand_summary_times: body.grand_summary_times,
        retention_days: body.retention_days,
        attunement: body.attunement,
        sync_source_timestamp: None,
    };

    state
        .paraclate_client
        .config_repo()
        .update(&query.tid, &body.fid, update)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

    let config = state
        .paraclate_client
        .get_config(&query.tid, &body.fid)
        .await?
        .ok_or_else(|| crate::Error::NotFound("Config not found".to_string()))?;

    Ok(Json(config))
}

/// GET /api/paraclate/queue
///
/// 送信キュー一覧を取得
async fn list_queue(
    State(state): State<AppState>,
    Query(query): Query<QueueQuery>,
) -> Result<Json<QueueListResponse>> {
    // Issue #119: FID所属検証
    validate_fid(&state, &query.fid).await?;

    let status = query.status.map(|s| QueueStatus::from(s.as_str()));

    let result = state
        .paraclate_client
        .list_queue(&query.tid, &query.fid, status, query.limit, query.offset)
        .await?;

    Ok(Json(result))
}

/// POST /api/paraclate/queue/process
///
/// キューを手動処理（テスト/デバッグ用）
async fn process_queue(
    State(state): State<AppState>,
    Query(query): Query<TidQuery>,
) -> Result<Json<serde_json::Value>> {
    // Issue #119: FID所属検証
    validate_fid(&state, &query.fid).await?;

    let sent_count = state
        .paraclate_client
        .process_queue(&query.tid, &query.fid)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "sent_count": sent_count,
        "message": format!("Processed {} items from queue", sent_count)
    })))
}

// ============================================================
// T4-7: Pub/Sub受信フロー
// ============================================================

/// POST /api/paraclate/pubsub/push
///
/// Google Cloud Pub/Sub Push Subscription用エンドポイント
/// mobes2.0からの設定変更通知を受信
///
/// ## セキュリティ
/// - Push Subscriptionの署名検証（将来実装）
/// - TID/FID照合で不正通知を排除
async fn handle_pubsub_push(
    State(state): State<AppState>,
    Json(body): Json<PubSubPushMessage>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        message_id = %body.message.message_id,
        subscription = %body.subscription,
        "Received Pub/Sub push message"
    );

    match state.pubsub_subscriber.handle_push(body).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Notification processed"
        }))),
        Err(e) => {
            tracing::error!(error = %e, "Failed to process Pub/Sub push message");
            // Pub/Subには200を返してリトライを防ぐ
            // 実際のエラーはログに記録
            Ok(Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })))
        }
    }
}

/// POST /api/paraclate/notify
///
/// 直接通知エンドポイント（テスト・内部API用）
/// Pub/Subラッパーなしで直接通知を受信
///
/// ## 用途
/// - 開発環境でのテスト
/// - mobes2.0からの直接HTTP通知
/// - 手動での設定同期トリガー
async fn handle_direct_notification(
    State(state): State<AppState>,
    Json(body): Json<ConfigUpdateNotification>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        notification_type = ?body.notification_type,
        tid = %body.tid,
        fids = ?body.fids,
        actor = %body.actor,
        "Received direct notification"
    );

    match state.pubsub_subscriber.handle_direct(body).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Notification processed"
        }))),
        Err(e) => {
            tracing::error!(error = %e, "Failed to process direct notification");
            Err(crate::Error::Internal(e.to_string()))
        }
    }
}
