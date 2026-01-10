//! ParaclateClient HTTP Client
//!
//! Phase 4: ParaclateClient (Issue #117)
//! DD03_ParaclateClient.md準拠
//!
//! ## 概要
//! Paraclate APPへのHTTP通信を担当
//! - LacisOath認証ヘッダ付与
//! - Summary/Event送信
//! - 設定同期

use crate::config_store::ConfigStore;
use crate::paraclate_client::{
    repository::{ConfigRepository, ConnectionLogRepository, SendQueueRepository},
    types::*,
};
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// ParaclateClient
///
/// Paraclate APPとの通信を担当するHTTPクライアント
#[derive(Clone)]
pub struct ParaclateClient {
    http: Client,
    config_repo: ConfigRepository,
    queue_repo: SendQueueRepository,
    log_repo: ConnectionLogRepository,
    config_store: Arc<ConfigStore>,
}

impl ParaclateClient {
    /// 新規作成
    pub fn new(
        pool: sqlx::MySqlPool,
        config_store: Arc<ConfigStore>,
    ) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            config_repo: ConfigRepository::new(pool.clone()),
            queue_repo: SendQueueRepository::new(pool.clone()),
            log_repo: ConnectionLogRepository::new(pool),
            config_store,
        }
    }

    /// LacisOathを取得
    ///
    /// ConfigStoreから is22 デバイスの LacisID/CIC を取得
    /// AraneaRegister (Phase 1) で登録されたキーを参照
    async fn get_lacis_oath(&self, tid: &str) -> Result<LacisOath, ParaclateError> {
        // AraneaRegister で定義されたconfig keys
        const LACIS_ID_KEY: &str = "aranea.lacis_id";
        const CIC_KEY: &str = "aranea.cic";

        let lacis_id = self
            .config_store
            .service()
            .get_setting(LACIS_ID_KEY)
            .await
            .map_err(|e| ParaclateError::Config(format!("Failed to get lacis_id: {}", e)))?
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| ParaclateError::Auth("LacisID not configured. Run AraneaRegister first.".to_string()))?;

        let cic = self
            .config_store
            .service()
            .get_setting(CIC_KEY)
            .await
            .map_err(|e| ParaclateError::Config(format!("Failed to get cic: {}", e)))?
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| ParaclateError::Auth("CIC not configured. Run AraneaRegister first.".to_string()))?;

        Ok(LacisOath {
            lacis_id,
            tid: tid.to_string(),
            cic,
            blessing: None,
        })
    }

    /// 接続テスト
    pub async fn connect(
        &self,
        tid: &str,
        fid: &str,
        endpoint: &str,
    ) -> Result<ConnectResponse, ParaclateError> {
        info!(tid = %tid, fid = %fid, endpoint = %endpoint, "Connecting to Paraclate APP");

        let oath = self.get_lacis_oath(tid).await?;

        // 接続テストエンドポイント
        let url = format!("{}/api/health", endpoint);

        let mut req = self.http.get(&url);
        for (key, value) in oath.to_headers() {
            req = req.header(&key, &value);
        }

        match req.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    // 設定を保存/更新
                    let config = self.config_repo.get(tid, fid).await.map_err(|e| {
                        ParaclateError::Database(format!("Failed to get config: {}", e))
                    })?;

                    let config_id = if let Some(c) = config {
                        self.config_repo
                            .update_connection_status(tid, fid, ConnectionStatus::Connected, None)
                            .await
                            .map_err(|e| {
                                ParaclateError::Database(format!("Failed to update status: {}", e))
                            })?;
                        c.config_id
                    } else {
                        self.config_repo
                            .insert(ParaclateConfigInsert {
                                tid: tid.to_string(),
                                fid: fid.to_string(),
                                endpoint: endpoint.to_string(),
                                report_interval_minutes: None,
                                grand_summary_times: None,
                                retention_days: None,
                                attunement: None,
                            })
                            .await
                            .map_err(|e| {
                                ParaclateError::Database(format!("Failed to insert config: {}", e))
                            })?
                    };

                    // 接続ログ記録
                    let _ = self
                        .log_repo
                        .insert(ConnectionLogInsert {
                            tid: tid.to_string(),
                            fid: fid.to_string(),
                            event_type: ConnectionEventType::Connect,
                            event_detail: Some(format!("Connected to {}", endpoint)),
                            error_code: None,
                            http_status_code: Some(response.status().as_u16() as i32),
                        })
                        .await;

                    info!(tid = %tid, fid = %fid, "Connected to Paraclate APP");

                    Ok(ConnectResponse {
                        connected: true,
                        endpoint: endpoint.to_string(),
                        config_id,
                        error: None,
                    })
                } else {
                    let status = response.status();
                    let error_msg = format!("HTTP {}", status.as_u16());

                    self.config_repo
                        .update_connection_status(
                            tid,
                            fid,
                            ConnectionStatus::Error,
                            Some(&error_msg),
                        )
                        .await
                        .ok();

                    let _ = self
                        .log_repo
                        .insert(ConnectionLogInsert {
                            tid: tid.to_string(),
                            fid: fid.to_string(),
                            event_type: ConnectionEventType::Error,
                            event_detail: Some(error_msg.clone()),
                            error_code: Some(format!("HTTP_{}", status.as_u16())),
                            http_status_code: Some(status.as_u16() as i32),
                        })
                        .await;

                    warn!(tid = %tid, fid = %fid, status = %status, "Connection failed");

                    Ok(ConnectResponse {
                        connected: false,
                        endpoint: endpoint.to_string(),
                        config_id: 0,
                        error: Some(error_msg),
                    })
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                self.config_repo
                    .update_connection_status(tid, fid, ConnectionStatus::Error, Some(&error_msg))
                    .await
                    .ok();

                let _ = self
                    .log_repo
                    .insert(ConnectionLogInsert {
                        tid: tid.to_string(),
                        fid: fid.to_string(),
                        event_type: ConnectionEventType::Error,
                        event_detail: Some(error_msg.clone()),
                        error_code: Some("CONNECTION_ERROR".to_string()),
                        http_status_code: None,
                    })
                    .await;

                error!(tid = %tid, fid = %fid, error = %e, "Connection error");

                Err(ParaclateError::Http(error_msg))
            }
        }
    }

    /// 切断
    pub async fn disconnect(&self, tid: &str, fid: &str) -> Result<bool, ParaclateError> {
        self.config_repo
            .update_connection_status(tid, fid, ConnectionStatus::Disconnected, None)
            .await
            .map_err(|e| ParaclateError::Database(format!("Failed to update status: {}", e)))?;

        let _ = self
            .log_repo
            .insert(ConnectionLogInsert {
                tid: tid.to_string(),
                fid: fid.to_string(),
                event_type: ConnectionEventType::Disconnect,
                event_detail: Some("Manually disconnected".to_string()),
                error_code: None,
                http_status_code: None,
            })
            .await;

        info!(tid = %tid, fid = %fid, "Disconnected from Paraclate APP");

        Ok(true)
    }

    /// ステータス取得
    pub async fn get_status(&self, tid: &str, fid: &str) -> Result<StatusResponse, ParaclateError> {
        let config = self.config_repo.get(tid, fid).await.map_err(|e| {
            ParaclateError::Database(format!("Failed to get config: {}", e))
        })?;

        let queue_stats = self.queue_repo.get_stats(tid, fid).await.map_err(|e| {
            ParaclateError::Database(format!("Failed to get queue stats: {}", e))
        })?;

        if let Some(c) = config {
            Ok(StatusResponse {
                connected: c.connection_status == ConnectionStatus::Connected,
                connection_status: c.connection_status,
                endpoint: Some(c.endpoint),
                last_sync_at: c.last_sync_at,
                last_error: c.last_error,
                queue_stats,
            })
        } else {
            Ok(StatusResponse {
                connected: false,
                connection_status: ConnectionStatus::Disconnected,
                endpoint: None,
                last_sync_at: None,
                last_error: None,
                queue_stats,
            })
        }
    }

    /// 設定取得
    pub async fn get_config(
        &self,
        tid: &str,
        fid: &str,
    ) -> Result<Option<ConfigResponse>, ParaclateError> {
        let config = self.config_repo.get(tid, fid).await.map_err(|e| {
            ParaclateError::Database(format!("Failed to get config: {}", e))
        })?;

        Ok(config.map(ConfigResponse::from))
    }

    /// Summary送信
    pub async fn send_summary(
        &self,
        tid: &str,
        fid: &str,
        payload: serde_json::Value,
        summary_id: u64,
    ) -> Result<u64, ParaclateError> {
        let queue_id = self
            .queue_repo
            .insert(SendQueueInsert {
                tid: tid.to_string(),
                fid: fid.to_string(),
                payload_type: PayloadType::Summary,
                payload,
                reference_id: Some(summary_id),
                max_retries: None,
            })
            .await
            .map_err(|e| ParaclateError::Queue(format!("Failed to enqueue: {}", e)))?;

        debug!(tid = %tid, fid = %fid, queue_id = queue_id, summary_id = summary_id, "Summary enqueued");

        Ok(queue_id)
    }

    /// GrandSummary送信
    pub async fn send_grand_summary(
        &self,
        tid: &str,
        fid: &str,
        payload: serde_json::Value,
        summary_id: u64,
    ) -> Result<u64, ParaclateError> {
        let queue_id = self
            .queue_repo
            .insert(SendQueueInsert {
                tid: tid.to_string(),
                fid: fid.to_string(),
                payload_type: PayloadType::GrandSummary,
                payload,
                reference_id: Some(summary_id),
                max_retries: None,
            })
            .await
            .map_err(|e| ParaclateError::Queue(format!("Failed to enqueue: {}", e)))?;

        debug!(tid = %tid, fid = %fid, queue_id = queue_id, summary_id = summary_id, "GrandSummary enqueued");

        Ok(queue_id)
    }

    /// Event送信（snapshot付き）
    ///
    /// T4-3: LacisFiles連携
    /// multipart/form-dataでsnapshotを含めてEvent送信
    /// 成功時はstoragePath（恒久参照子）を返す
    pub async fn send_event_with_snapshot(
        &self,
        tid: &str,
        fid: &str,
        event_payload: EventPayload,
        snapshot_data: Option<Vec<u8>>,
        log_id: u64,
    ) -> Result<EventSendResult, ParaclateError> {
        let config = self
            .config_repo
            .get(tid, fid)
            .await
            .map_err(|e| ParaclateError::Database(format!("Failed to get config: {}", e)))?
            .ok_or_else(|| ParaclateError::Config("Config not found".to_string()))?;

        if config.connection_status != ConnectionStatus::Connected {
            // オフライン時はキューに追加
            let payload = serde_json::to_value(&event_payload)
                .map_err(|e| ParaclateError::Queue(format!("Failed to serialize: {}", e)))?;

            let queue_id = self
                .queue_repo
                .insert(SendQueueInsert {
                    tid: tid.to_string(),
                    fid: fid.to_string(),
                    payload_type: PayloadType::Event,
                    payload,
                    reference_id: Some(log_id),
                    max_retries: None,
                })
                .await
                .map_err(|e| ParaclateError::Queue(format!("Failed to enqueue: {}", e)))?;

            debug!(tid = %tid, fid = %fid, queue_id = queue_id, "Event enqueued (offline)");

            return Ok(EventSendResult {
                success: false,
                queue_id,
                event_id: None,
                snapshot_url: None,
                error: Some("Offline - enqueued for later".to_string()),
            });
        }

        let oath = self.get_lacis_oath(tid).await?;
        let url = format!("{}/api/paraclate/ingest/event", config.endpoint);

        // multipart/form-data を構築
        let event_json = serde_json::to_string(&event_payload)
            .map_err(|e| ParaclateError::Queue(format!("Failed to serialize: {}", e)))?;

        let mut form = reqwest::multipart::Form::new()
            .text("event", event_json);

        // snapshotがあれば添付
        if let Some(data) = snapshot_data {
            let file_name = format!("snapshot_{}.jpg", log_id);
            let part = reqwest::multipart::Part::bytes(data)
                .file_name(file_name)
                .mime_str("image/jpeg")
                .map_err(|e| ParaclateError::Http(format!("Failed to create multipart: {}", e)))?;
            form = form.part("snapshot", part);
        }

        let mut req = self.http.post(&url).multipart(form);
        for (key, value) in oath.to_headers() {
            req = req.header(&key, &value);
        }

        match req.send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    // レスポンスをパース
                    let event_response: EventResponse = response
                        .json()
                        .await
                        .map_err(|e| ParaclateError::Http(format!("Failed to parse response: {}", e)))?;

                    if event_response.ok {
                        let snapshot_url = event_response.snapshot
                            .as_ref()
                            .map(|s| s.storage_path.clone());

                        info!(
                            tid = %tid,
                            log_id = log_id,
                            event_id = ?event_response.event_id,
                            snapshot_url = ?snapshot_url,
                            "Event sent successfully"
                        );

                        Ok(EventSendResult {
                            success: true,
                            queue_id: 0, // 直接送信成功時はキューIDなし
                            event_id: event_response.event_id,
                            snapshot_url,
                            error: None,
                        })
                    } else {
                        warn!(tid = %tid, log_id = log_id, error = ?event_response.error, "Event send failed");

                        Ok(EventSendResult {
                            success: false,
                            queue_id: 0,
                            event_id: None,
                            snapshot_url: None,
                            error: event_response.error,
                        })
                    }
                } else {
                    let error_body = response.text().await.unwrap_or_default();
                    let error_msg = format!("HTTP {}: {}", status.as_u16(), error_body);

                    // 失敗時はキューに追加してリトライ
                    let payload = serde_json::to_value(&event_payload)
                        .map_err(|e| ParaclateError::Queue(format!("Failed to serialize: {}", e)))?;

                    let queue_id = self
                        .queue_repo
                        .insert(SendQueueInsert {
                            tid: tid.to_string(),
                            fid: fid.to_string(),
                            payload_type: PayloadType::Event,
                            payload,
                            reference_id: Some(log_id),
                            max_retries: None,
                        })
                        .await
                        .map_err(|e| ParaclateError::Queue(format!("Failed to enqueue: {}", e)))?;

                    warn!(
                        tid = %tid,
                        log_id = log_id,
                        status = %status,
                        queue_id = queue_id,
                        "Event send failed - enqueued for retry"
                    );

                    Ok(EventSendResult {
                        success: false,
                        queue_id,
                        event_id: None,
                        snapshot_url: None,
                        error: Some(error_msg),
                    })
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                // エラー時はキューに追加
                let payload = serde_json::to_value(&event_payload)
                    .map_err(|e| ParaclateError::Queue(format!("Failed to serialize: {}", e)))?;

                let queue_id = self
                    .queue_repo
                    .insert(SendQueueInsert {
                        tid: tid.to_string(),
                        fid: fid.to_string(),
                        payload_type: PayloadType::Event,
                        payload,
                        reference_id: Some(log_id),
                        max_retries: None,
                    })
                    .await
                    .map_err(|e| ParaclateError::Queue(format!("Failed to enqueue: {}", e)))?;

                error!(tid = %tid, log_id = log_id, error = %e, queue_id = queue_id, "Event send error - enqueued");

                Ok(EventSendResult {
                    success: false,
                    queue_id,
                    event_id: None,
                    snapshot_url: None,
                    error: Some(error_msg),
                })
            }
        }
    }

    /// Emergency送信（優先度高）
    pub async fn send_emergency(
        &self,
        tid: &str,
        fid: &str,
        payload: serde_json::Value,
    ) -> Result<u64, ParaclateError> {
        let queue_id = self
            .queue_repo
            .insert(SendQueueInsert {
                tid: tid.to_string(),
                fid: fid.to_string(),
                payload_type: PayloadType::Emergency,
                payload,
                reference_id: None,
                max_retries: Some(10), // Emergencyはリトライ回数多め
            })
            .await
            .map_err(|e| ParaclateError::Queue(format!("Failed to enqueue: {}", e)))?;

        info!(tid = %tid, fid = %fid, queue_id = queue_id, "Emergency enqueued");

        Ok(queue_id)
    }

    /// キューを処理（バックグラウンドワーカー用）
    pub async fn process_queue(&self, tid: &str, fid: &str) -> Result<u32, ParaclateError> {
        let config = self
            .config_repo
            .get(tid, fid)
            .await
            .map_err(|e| ParaclateError::Database(format!("Failed to get config: {}", e)))?
            .ok_or_else(|| ParaclateError::Config("Config not found".to_string()))?;

        if config.connection_status != ConnectionStatus::Connected {
            debug!(tid = %tid, fid = %fid, "Skipping queue processing - not connected");
            return Ok(0);
        }

        let oath = self.get_lacis_oath(tid).await?;
        let pending = self
            .queue_repo
            .get_pending(tid, fid, 10)
            .await
            .map_err(|e| ParaclateError::Database(format!("Failed to get pending: {}", e)))?;

        let mut sent_count = 0u32;

        for item in pending {
            // sending状態に更新
            self.queue_repo
                .update_status(
                    item.queue_id,
                    SendQueueUpdate {
                        status: QueueStatus::Sending,
                        retry_count: None,
                        next_retry_at: None,
                        last_error: None,
                        http_status_code: None,
                        sent_at: None,
                    },
                )
                .await
                .ok();

            let endpoint_path = match item.payload_type {
                PayloadType::Summary | PayloadType::GrandSummary => "/api/paraclate/summary",
                PayloadType::Event => "/api/paraclate/event",
                PayloadType::Emergency => "/api/paraclate/emergency",
            };

            let url = format!("{}{}", config.endpoint, endpoint_path);

            let mut req = self.http.post(&url).json(&item.payload);
            for (key, value) in oath.to_headers() {
                req = req.header(&key, &value);
            }

            match req.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        self.queue_repo.mark_sent(item.queue_id).await.ok();
                        sent_count += 1;
                        debug!(
                            queue_id = item.queue_id,
                            payload_type = %item.payload_type,
                            "Sent successfully"
                        );
                    } else {
                        let error_body = response.text().await.unwrap_or_default();
                        let error_msg = format!("HTTP {}: {}", status.as_u16(), error_body);
                        self.queue_repo
                            .mark_failed(
                                item.queue_id,
                                &error_msg,
                                Some(status.as_u16() as i32),
                                item.retry_count + 1,
                            )
                            .await
                            .ok();
                        warn!(
                            queue_id = item.queue_id,
                            status = %status,
                            "Send failed"
                        );
                    }
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    self.queue_repo
                        .mark_failed(item.queue_id, &error_msg, None, item.retry_count + 1)
                        .await
                        .ok();
                    error!(queue_id = item.queue_id, error = %e, "Send error");
                }
            }
        }

        if sent_count > 0 {
            debug!(tid = %tid, fid = %fid, sent_count = sent_count, "Queue processing completed");
        }

        Ok(sent_count)
    }

    /// キュー一覧取得
    pub async fn list_queue(
        &self,
        tid: &str,
        fid: &str,
        status: Option<QueueStatus>,
        limit: i32,
        offset: i32,
    ) -> Result<QueueListResponse, ParaclateError> {
        let items = self
            .queue_repo
            .list(tid, fid, status, limit, offset)
            .await
            .map_err(|e| ParaclateError::Database(format!("Failed to list queue: {}", e)))?;

        let stats = self
            .queue_repo
            .get_stats(tid, fid)
            .await
            .map_err(|e| ParaclateError::Database(format!("Failed to get stats: {}", e)))?;

        Ok(QueueListResponse {
            items: items.into_iter().map(QueueItemResponse::from).collect(),
            total: stats.pending + stats.sending + stats.failed,
            stats,
        })
    }

    /// 設定リポジトリへの参照を取得
    pub fn config_repo(&self) -> &ConfigRepository {
        &self.config_repo
    }

    /// キューリポジトリへの参照を取得
    pub fn queue_repo(&self) -> &SendQueueRepository {
        &self.queue_repo
    }

    /// 接続ログリポジトリへの参照を取得
    pub fn log_repo(&self) -> &ConnectionLogRepository {
        &self.log_repo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lacis_oath_headers() {
        let oath = LacisOath {
            lacis_id: "3022AABBCCDDEEFF0000".to_string(),
            tid: "T123".to_string(),
            cic: "123456".to_string(),
            blessing: None,
        };

        let headers = oath.to_headers();
        assert_eq!(headers.len(), 3);
        assert!(headers.iter().any(|(k, _)| k == "X-Lacis-ID"));
        assert!(headers.iter().any(|(k, _)| k == "X-Lacis-TID"));
        assert!(headers.iter().any(|(k, _)| k == "X-Lacis-CIC"));
    }
}
