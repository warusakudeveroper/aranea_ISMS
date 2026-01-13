//! Chat Messages API Routes
//!
//! Backend storage for AI Assistant chat messages
//! Design: All data stored on backend, no browser-local storage

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::realtime_hub::{ChatMessageData, ChatSyncMessage, HubMessage};
use crate::state::AppState;

/// Chat message from/to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub message_id: String,
    pub role: String,  // "user" | "assistant" | "system"
    pub content: String,
    pub timestamp: String,  // ISO8601
    #[serde(default)]
    pub handled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_camera_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_preset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dismiss_at: Option<i64>,
    #[serde(default)]
    pub is_hiding: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

/// Create chat routes
pub fn chat_routes() -> Router<AppState> {
    Router::new()
        .route("/chat/messages", get(list_messages))
        .route("/chat/messages", post(create_message))
        .route("/chat/messages/bulk", post(bulk_create_messages))
        .route("/chat/messages/:message_id", put(update_message))
        .route("/chat/messages/:message_id", delete(delete_message))
        .route("/chat/messages", delete(clear_all_messages))
}

/// GET /api/chat/messages - List all chat messages
async fn list_messages(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    let result = sqlx::query(
        r#"
        SELECT message_id, role, content, timestamp, handled,
               action_type, action_camera_id, action_preset_id,
               dismiss_at, is_hiding
        FROM chat_messages
        ORDER BY timestamp ASC
        LIMIT ?
        "#,
    )
    .bind(query.limit)
    .fetch_all(&state.pool)
    .await;

    match result {
        Ok(rows) => {
            let messages: Vec<ChatMessage> = rows
                .iter()
                .map(|row| ChatMessage {
                    message_id: row.get("message_id"),
                    role: row.get("role"),
                    content: row.get("content"),
                    timestamp: row.get("timestamp"),
                    handled: row.get::<i32, _>("handled") != 0,
                    action_type: row.get("action_type"),
                    action_camera_id: row.get("action_camera_id"),
                    action_preset_id: row.get("action_preset_id"),
                    dismiss_at: row.get("dismiss_at"),
                    is_hiding: row.get::<i32, _>("is_hiding") != 0,
                })
                .collect();

            Json(serde_json::json!({
                "ok": true,
                "data": messages
            }))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to list chat messages");
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// POST /api/chat/messages - Create a new chat message
async fn create_message(
    State(state): State<AppState>,
    Json(msg): Json<ChatMessage>,
) -> impl IntoResponse {
    let result = sqlx::query(
        r#"
        INSERT INTO chat_messages (
            message_id, role, content, timestamp, handled,
            action_type, action_camera_id, action_preset_id,
            dismiss_at, is_hiding
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
            content = VALUES(content),
            handled = VALUES(handled),
            dismiss_at = VALUES(dismiss_at),
            is_hiding = VALUES(is_hiding)
        "#,
    )
    .bind(&msg.message_id)
    .bind(&msg.role)
    .bind(&msg.content)
    .bind(&msg.timestamp)
    .bind(if msg.handled { 1 } else { 0 })
    .bind(&msg.action_type)
    .bind(&msg.action_camera_id)
    .bind(&msg.action_preset_id)
    .bind(msg.dismiss_at)
    .bind(if msg.is_hiding { 1 } else { 0 })
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {
            // Broadcast to other clients
            state.realtime.broadcast(HubMessage::ChatSync(ChatSyncMessage {
                action: "created".to_string(),
                message: Some(ChatMessageData {
                    message_id: msg.message_id.clone(),
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                    timestamp: msg.timestamp.clone(),
                    handled: msg.handled,
                    action_type: msg.action_type.clone(),
                    action_camera_id: msg.action_camera_id.clone(),
                    action_preset_id: msg.action_preset_id.clone(),
                    dismiss_at: msg.dismiss_at,
                    is_hiding: msg.is_hiding,
                }),
                message_id: None,
            })).await;

            Json(serde_json::json!({
                "ok": true,
                "message_id": msg.message_id
            }))
        },
        Err(e) => {
            tracing::error!(error = %e, "Failed to create chat message");
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// POST /api/chat/messages/bulk - Bulk create/update messages (for migration)
async fn bulk_create_messages(
    State(state): State<AppState>,
    Json(messages): Json<Vec<ChatMessage>>,
) -> impl IntoResponse {
    let mut success_count = 0;
    let mut error_count = 0;

    for msg in messages {
        let result = sqlx::query(
            r#"
            INSERT IGNORE INTO chat_messages (
                message_id, role, content, timestamp, handled,
                action_type, action_camera_id, action_preset_id,
                dismiss_at, is_hiding
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&msg.message_id)
        .bind(&msg.role)
        .bind(&msg.content)
        .bind(&msg.timestamp)
        .bind(if msg.handled { 1 } else { 0 })
        .bind(&msg.action_type)
        .bind(&msg.action_camera_id)
        .bind(&msg.action_preset_id)
        .bind(msg.dismiss_at)
        .bind(if msg.is_hiding { 1 } else { 0 })
        .execute(&state.pool)
        .await;

        match result {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "success_count": success_count,
        "error_count": error_count
    }))
}

/// PUT /api/chat/messages/:message_id - Update a message
async fn update_message(
    State(state): State<AppState>,
    axum::extract::Path(message_id): axum::extract::Path<String>,
    Json(msg): Json<ChatMessage>,
) -> impl IntoResponse {
    let result = sqlx::query(
        r#"
        UPDATE chat_messages SET
            handled = ?,
            dismiss_at = ?,
            is_hiding = ?
        WHERE message_id = ?
        "#,
    )
    .bind(if msg.handled { 1 } else { 0 })
    .bind(msg.dismiss_at)
    .bind(if msg.is_hiding { 1 } else { 0 })
    .bind(&message_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {
            // Broadcast update to other clients
            state.realtime.broadcast(HubMessage::ChatSync(ChatSyncMessage {
                action: "updated".to_string(),
                message: Some(ChatMessageData {
                    message_id: message_id.clone(),
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                    timestamp: msg.timestamp.clone(),
                    handled: msg.handled,
                    action_type: msg.action_type.clone(),
                    action_camera_id: msg.action_camera_id.clone(),
                    action_preset_id: msg.action_preset_id.clone(),
                    dismiss_at: msg.dismiss_at,
                    is_hiding: msg.is_hiding,
                }),
                message_id: None,
            })).await;

            Json(serde_json::json!({
                "ok": true
            }))
        },
        Err(e) => {
            tracing::error!(error = %e, "Failed to update chat message");
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// DELETE /api/chat/messages/:message_id - Delete a message
async fn delete_message(
    State(state): State<AppState>,
    axum::extract::Path(message_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM chat_messages WHERE message_id = ?")
        .bind(&message_id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(_) => {
            // Broadcast deletion to other clients
            state.realtime.broadcast(HubMessage::ChatSync(ChatSyncMessage {
                action: "deleted".to_string(),
                message: None,
                message_id: Some(message_id.clone()),
            })).await;

            Json(serde_json::json!({
                "ok": true
            }))
        },
        Err(e) => {
            tracing::error!(error = %e, "Failed to delete chat message");
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// DELETE /api/chat/messages - Clear all messages
async fn clear_all_messages(State(state): State<AppState>) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM chat_messages")
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) => {
            // Broadcast clear to other clients
            state.realtime.broadcast(HubMessage::ChatSync(ChatSyncMessage {
                action: "cleared".to_string(),
                message: None,
                message_id: None,
            })).await;

            Json(serde_json::json!({
                "ok": true,
                "deleted_count": r.rows_affected()
            }))
        },
        Err(e) => {
            tracing::error!(error = %e, "Failed to clear chat messages");
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Database error: {}", e)
            }))
        }
    }
}
