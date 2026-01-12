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
    AIChatContext, AIChatResponse, CameraContextInfo, CameraDetectionCount, ChatMessage,
    ConfigResponse, ConfigUpdateNotification, ConnectRequest, ConnectResponse, FidValidator,
    PubSubPushMessage, QueueListResponse, QueueStatus, RecentDetectionsSummary, StatusResponse,
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
        // AI Chat (Paraclate_DesignOverview.md準拠)
        .route("/chat", post(handle_ai_chat))
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

// ============================================================
// AI Chat (Paraclate_DesignOverview.md準拠)
// ============================================================

/// AIチャットリクエスト（API用）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIChatApiRequest {
    /// 施設ID
    pub fid: String,
    /// ユーザーからの質問
    pub message: String,
    /// 会話履歴（オプション）
    #[serde(default)]
    pub conversation_history: Option<Vec<ChatMessageInput>>,
    /// コンテキスト自動収集を有効にするか（デフォルト: true）
    #[serde(default = "default_auto_context")]
    pub auto_context: bool,
}

fn default_auto_context() -> bool {
    true
}

/// チャットメッセージ入力（API用）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageInput {
    pub role: String,
    pub content: String,
}

/// POST /api/paraclate/chat
///
/// AIアシスタントチャット（Paraclate APP統合）
///
/// ## Paraclate_DesignOverview.md準拠
/// - AIアシスタントタブからの質問に関してもParaclate APPからのレスポンスでチャットボット機能を行う
/// - 検出特徴の人物のカメラ間での移動などを横断的に把握
/// - カメラ不調などの傾向も把握
/// - 過去の記録を参照する、過去の記録範囲を対話的にユーザーと会話
async fn handle_ai_chat(
    State(state): State<AppState>,
    Query(query): Query<TidQuery>,
    Json(body): Json<AIChatApiRequest>,
) -> Result<Json<AIChatResponse>> {
    // Issue #119: FID所属検証
    validate_fid(&state, &body.fid).await?;

    tracing::info!(
        tid = %query.tid,
        fid = %body.fid,
        message_len = body.message.len(),
        auto_context = body.auto_context,
        "Processing AI chat request"
    );

    // コンテキスト自動収集
    let context = if body.auto_context {
        Some(build_ai_context(&state, &query.tid, &body.fid).await?)
    } else {
        None
    };

    // 会話履歴を変換
    let conversation_history = body.conversation_history.map(|history| {
        history
            .into_iter()
            .map(|m| ChatMessage {
                role: m.role,
                content: m.content,
                timestamp: None,
            })
            .collect()
    });

    // Paraclate APPにチャット送信
    let response = state
        .paraclate_client
        .send_ai_chat(&query.tid, &body.fid, &body.message, context, conversation_history)
        .await?;

    Ok(Json(response))
}

/// AIコンテキストを自動構築
///
/// カメラ情報と直近検出サマリーを収集してコンテキストを生成
async fn build_ai_context(
    state: &AppState,
    tid: &str,
    fid: &str,
) -> Result<AIChatContext> {
    use chrono::{Duration, Utc};

    // カメラ一覧取得
    let cameras: Vec<_> = sqlx::query_as::<_, (String, String, Option<String>, bool, bool, Option<chrono::DateTime<Utc>>)>(
        r#"
        SELECT
            camera_id, name, camera_context, enabled, polling_enabled,
            (SELECT MAX(captured_at) FROM detection_logs WHERE detection_logs.camera_id = cameras.camera_id) as last_detection_at
        FROM cameras
        WHERE deleted_at IS NULL
          AND (tid = ? OR fid = ?)
        ORDER BY name
        LIMIT 30
        "#,
    )
    .bind(tid)
    .bind(fid)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| crate::Error::Database(e.to_string()))?;

    let camera_infos: Vec<CameraContextInfo> = cameras
        .iter()
        .map(|(id, name, context, enabled, polling_enabled, last_detection)| {
            CameraContextInfo {
                camera_id: id.clone(),
                name: name.clone(),
                location: context.clone(),
                status: if *enabled && *polling_enabled {
                    "online".to_string()
                } else {
                    "offline".to_string()
                },
                last_detection_at: *last_detection,
            }
        })
        .collect();

    // 24時間以内の検出サマリー取得
    let cutoff = Utc::now() - Duration::hours(24);

    let detection_stats: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            CAST(COUNT(*) AS SIGNED) as total,
            CAST(COALESCE(SUM(CASE WHEN LOWER(primary_event) LIKE '%human%' OR LOWER(primary_event) LIKE '%person%' THEN 1 ELSE 0 END), 0) AS SIGNED) as human_count,
            CAST(COALESCE(SUM(CASE WHEN LOWER(primary_event) LIKE '%vehicle%' OR LOWER(primary_event) LIKE '%car%' THEN 1 ELSE 0 END), 0) AS SIGNED) as vehicle_count
        FROM detection_logs
        WHERE captured_at >= ?
          AND (tid = ? OR fid = ?)
        "#,
    )
    .bind(cutoff)
    .bind(tid)
    .bind(fid)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| crate::Error::Database(e.to_string()))?;

    // カメラ別検出数
    let by_camera: Vec<(String, String, i64)> = sqlx::query_as(
        r#"
        SELECT
            dl.camera_id,
            c.name,
            CAST(COUNT(*) AS SIGNED) as count
        FROM detection_logs dl
        JOIN cameras c ON dl.camera_id = c.camera_id
        WHERE dl.captured_at >= ?
          AND (dl.tid = ? OR dl.fid = ?)
        GROUP BY dl.camera_id, c.name
        ORDER BY count DESC
        LIMIT 10
        "#,
    )
    .bind(cutoff)
    .bind(tid)
    .bind(fid)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| crate::Error::Database(e.to_string()))?;

    let camera_counts: Vec<CameraDetectionCount> = by_camera
        .into_iter()
        .map(|(camera_id, camera_name, count)| CameraDetectionCount {
            camera_id,
            camera_name,
            count: count as u32,
        })
        .collect();

    let recent_detections = RecentDetectionsSummary {
        total_24h: detection_stats.0 as u32,
        human_count: detection_stats.1 as u32,
        vehicle_count: detection_stats.2 as u32,
        by_camera: if camera_counts.is_empty() {
            None
        } else {
            Some(camera_counts)
        },
    };

    // 施設コンテキスト取得（Paraclate設定から）
    let facility_context = state
        .paraclate_client
        .get_config(tid, fid)
        .await
        .ok()
        .flatten()
        .and_then(|c| {
            c.attunement.get("facilityContext")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

    Ok(AIChatContext {
        cameras: Some(camera_infos),
        recent_detections: Some(recent_detections),
        facility_context,
    })
}
