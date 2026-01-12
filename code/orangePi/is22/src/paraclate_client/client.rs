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
//!
//! ## mobes2.0 Cloud Run Endpoints
//! E2Eテスト結果に基づき実際のエンドポイントを使用

use crate::config_store::ConfigStore;
use crate::paraclate_client::{
    repository::{ConfigRepository, ConnectionLogRepository, SendQueueRepository},
    types::{
        AIChatContext, AIChatRequest, AIChatResponse, ChatMessage, ConnectionEventType,
        ConnectionLogInsert, ConnectionStatus, EventPayload, EventResponse, LacisOath,
        ParaclateConfigInsert, ParaclateError, PayloadType, QueueItemResponse, QueueListResponse,
        QueueStats, QueueStatus, SendQueueInsert, SendQueueUpdate, StatusResponse,
        ConfigResponse, ConnectResponse, EventSendResult,
    },
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// mobes2.0 Cloud Run エンドポイント定数
/// E2Eテスト結果に基づく実際のURL
mod endpoints {
    pub const CONNECT: &str = "https://paraclateconnect-vm44u3kpua-an.a.run.app";
    pub const INGEST_SUMMARY: &str = "https://paraclateingestsummary-vm44u3kpua-an.a.run.app";
    pub const INGEST_EVENT: &str = "https://paraclateingestevent-vm44u3kpua-an.a.run.app";
    pub const GET_CONFIG: &str = "https://paraclategetconfig-vm44u3kpua-an.a.run.app";
    /// Phase 8: カメラメタデータ同期 (Issue #121)
    /// mobes2.0チーム回答に基づき訂正 (2026-01-12)
    pub const CAMERA_METADATA: &str = "https://asia-northeast1-mobesorder.cloudfunctions.net/paraclateCameraMetadata";
    /// AI Chat エンドポイント
    /// Paraclate_DesignOverview.md準拠: AIアシスタント機能
    pub const AI_CHAT: &str = "https://asia-northeast1-mobesorder.cloudfunctions.net/paraclateAIChat";
}

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
            // mobes2.0 Cloud RunはHTTPS必須、リダイレクトでPOSTがGETに変わる問題を回避
            .redirect(reqwest::redirect::Policy::none())
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
    /// ConfigStoreから is22 デバイスの LacisID/TID/CIC を取得
    /// AraneaRegister (Phase 1) で登録されたキーを参照
    async fn get_lacis_oath(&self, _tid: &str) -> Result<LacisOath, ParaclateError> {
        // AraneaRegister で定義されたconfig keys
        const LACIS_ID_KEY: &str = "aranea.lacis_id";
        const TID_KEY: &str = "aranea.tid";
        const CIC_KEY: &str = "aranea.cic";

        info!("get_lacis_oath: Fetching credentials from config_store");

        let lacis_id_result = self
            .config_store
            .service()
            .get_setting(LACIS_ID_KEY)
            .await;

        let lacis_id = match lacis_id_result {
            Ok(Some(v)) => {
                let id = v.as_str().map(|s| s.to_string());
                info!(lacis_id = ?id, "get_lacis_oath: Got lacis_id");
                id.ok_or_else(|| ParaclateError::Auth("LacisID value is not a string".to_string()))?
            }
            Ok(None) => {
                error!("get_lacis_oath: LacisID not found in config");
                return Err(ParaclateError::Auth("LacisID not configured. Run AraneaRegister first.".to_string()));
            }
            Err(e) => {
                error!(error = %e, "get_lacis_oath: Failed to get lacis_id");
                return Err(ParaclateError::Config(format!("Failed to get lacis_id: {}", e)));
            }
        };

        // TIDも設定から取得（AraneaRegisterで登録済み）
        let tid_result = self
            .config_store
            .service()
            .get_setting(TID_KEY)
            .await;

        let tid = match tid_result {
            Ok(Some(v)) => {
                let t = v.as_str().map(|s| s.to_string());
                info!(tid = ?t, "get_lacis_oath: Got tid");
                t.ok_or_else(|| ParaclateError::Auth("TID value is not a string".to_string()))?
            }
            Ok(None) => {
                error!("get_lacis_oath: TID not found in config");
                return Err(ParaclateError::Auth("TID not configured. Run AraneaRegister first.".to_string()));
            }
            Err(e) => {
                error!(error = %e, "get_lacis_oath: Failed to get tid");
                return Err(ParaclateError::Config(format!("Failed to get tid: {}", e)));
            }
        };

        let cic_result = self
            .config_store
            .service()
            .get_setting(CIC_KEY)
            .await;

        let cic = match cic_result {
            Ok(Some(v)) => {
                let c = v.as_str().map(|s| s.to_string());
                info!(cic = ?c, "get_lacis_oath: Got cic");
                c.ok_or_else(|| ParaclateError::Auth("CIC value is not a string".to_string()))?
            }
            Ok(None) => {
                error!("get_lacis_oath: CIC not found in config");
                return Err(ParaclateError::Auth("CIC not configured. Run AraneaRegister first.".to_string()));
            }
            Err(e) => {
                error!(error = %e, "get_lacis_oath: Failed to get cic");
                return Err(ParaclateError::Config(format!("Failed to get cic: {}", e)));
            }
        };

        info!(
            lacis_id = %lacis_id,
            tid = %tid,
            cic = %cic,
            "get_lacis_oath: Successfully retrieved all credentials"
        );

        Ok(LacisOath {
            lacis_id,
            tid,
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

        let oath = match self.get_lacis_oath(tid).await {
            Ok(o) => o,
            Err(e) => {
                error!(error = %e, "connect: Failed to get LacisOath");
                return Err(e);
            }
        };

        // mobes2.0 Connect エンドポイント
        // 引数endpointは設定保存用に使用
        let url = endpoints::CONNECT;

        // mobes2.0 ペイロード形式
        let connect_body = serde_json::json!({
            "fid": fid,
            "payload": {
                "deviceType": "is22",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        // INFO: 実際に送信されるリクエスト内容を確認
        info!(
            url = %url,
            lacis_id = %oath.lacis_id,
            tid = %oath.tid,
            cic = %oath.cic,
            body = %connect_body,
            "Preparing Connect request"
        );

        let mut req = self.http.post(url).json(&connect_body);
        let headers = oath.to_headers();
        for (key, value) in &headers {
            info!(header_key = %key, header_value = %value, "Adding header");
            req = req.header(key, value);
        }

        info!("Sending POST request to {}", url);

        match req.send().await {
            Ok(response) => {
                let status = response.status();
                let response_headers = response.headers().clone();
                info!(
                    status = %status,
                    headers = ?response_headers,
                    "Received response"
                );

                if status.is_success() {
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
                            http_status_code: Some(status.as_u16() as i32),
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
                    // エラーレスポンスの詳細をログ出力
                    let error_body = response.text().await.unwrap_or_default();
                    info!(status = %status, body = %error_body, "Error response body");
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
    /// T5-2: LacisFiles連携（mobes2.0回答に基づく実装）
    /// snapshotをBase64エンコードしてJSON bodyに含めて送信
    /// mobes2.0側でLacisFilesに自動保存
    /// 成功時はstoragePath（恒久参照子）を返す
    pub async fn send_event_with_snapshot(
        &self,
        tid: &str,
        fid: &str,
        mut event_payload: EventPayload,
        snapshot_data: Option<Vec<u8>>,
        log_id: u64,
    ) -> Result<EventSendResult, ParaclateError> {
        use base64::Engine;

        let config = self
            .config_repo
            .get(tid, fid)
            .await
            .map_err(|e| ParaclateError::Database(format!("Failed to get config: {}", e)))?
            .ok_or_else(|| ParaclateError::Config("Config not found".to_string()))?;

        // snapshotがあればBase64エンコードしてpayloadに含める
        if let Some(data) = &snapshot_data {
            let base64_encoded = base64::engine::general_purpose::STANDARD.encode(data);
            event_payload.snapshot_base64 = Some(base64_encoded);
            event_payload.snapshot_mime_type = Some("image/jpeg".to_string());
            debug!(
                log_id = log_id,
                snapshot_size = data.len(),
                "Snapshot attached as Base64"
            );
        }

        if config.connection_status != ConnectionStatus::Connected {
            // オフライン時はキューに追加（snapshot含むpayload）
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

        // mobes2.0 Cloud Run エンドポイント
        let url = endpoints::INGEST_EVENT;

        // mobes2.0 ペイロード形式: { fid, payload: { event: {...} } }
        // snapshotはevent_payload.snapshot_base64に含まれる
        let wrapped_payload = serde_json::json!({
            "fid": fid,
            "payload": {
                "event": &event_payload
            }
        });

        let mut req = self.http.post(url)
            .header("Content-Type", "application/json")
            .json(&wrapped_payload);

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

            // mobes2.0 Cloud Run エンドポイントを選択
            let url = match item.payload_type {
                PayloadType::Summary | PayloadType::GrandSummary => endpoints::INGEST_SUMMARY,
                PayloadType::Event | PayloadType::Emergency => endpoints::INGEST_EVENT,
            };

            // mobes2.0 ペイロード形式: { fid, payload: {...} }
            let wrapped_payload = serde_json::json!({
                "fid": fid,
                "payload": item.payload
            });

            let mut req = self.http.post(url).json(&wrapped_payload);
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

    // ========================================
    // Phase 8: カメラメタデータ同期 (Issue #121)
    // ========================================

    /// カメラメタデータをmobes2.0へ送信
    ///
    /// IS22で管理するカメラのメタデータ（名前、場所、コンテキスト等）を
    /// mobes2.0側のparaclateCameraMetadata APIへ送信する
    pub async fn send_camera_metadata(
        &self,
        tid: &str,
        fid: &str,
        payload: &crate::camera_sync::CameraMetadataPayload,
    ) -> Result<CameraMetadataResponse, ParaclateError> {
        info!(
            tid = %tid,
            fid = %fid,
            camera_count = payload.cameras.len(),
            sync_type = ?payload.sync_type,
            "Sending camera metadata to mobes2.0"
        );

        let oath = self.get_lacis_oath(tid).await?;
        let url = endpoints::CAMERA_METADATA;

        // mobes2.0 ペイロード形式: { fid, payload: { cameras, syncType, updatedAt } }
        let wrapped_payload = serde_json::json!({
            "fid": fid,
            "payload": payload
        });

        debug!(payload = %serde_json::to_string(&wrapped_payload).unwrap_or_default(), "Camera metadata payload");

        let mut req = self.http.post(url)
            .header("Content-Type", "application/json")
            .json(&wrapped_payload);

        for (key, value) in oath.to_headers() {
            req = req.header(&key, &value);
        }

        match req.send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    let body: serde_json::Value = response.json().await.map_err(|e| {
                        ParaclateError::Http(format!("Failed to parse response: {}", e))
                    })?;

                    let synced_count = body.get("syncedCount")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;

                    info!(
                        tid = %tid,
                        fid = %fid,
                        synced_count = synced_count,
                        "Camera metadata sent successfully"
                    );

                    Ok(CameraMetadataResponse {
                        success: true,
                        synced_count,
                        error: None,
                    })
                } else {
                    let error_body = response.text().await.unwrap_or_default();
                    let error_msg = format!("HTTP {}: {}", status.as_u16(), error_body);

                    warn!(
                        tid = %tid,
                        fid = %fid,
                        status = %status,
                        error = %error_msg,
                        "Camera metadata send failed"
                    );

                    Ok(CameraMetadataResponse {
                        success: false,
                        synced_count: 0,
                        error: Some(error_msg),
                    })
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                error!(tid = %tid, fid = %fid, error = %error_msg, "Camera metadata send error");

                Err(ParaclateError::Http(error_msg))
            }
        }
    }

    /// カメラ削除通知をmobes2.0へ送信
    pub async fn send_camera_deleted(
        &self,
        tid: &str,
        fid: &str,
        payload: &crate::camera_sync::CameraDeletedPayload,
    ) -> Result<CameraMetadataResponse, ParaclateError> {
        info!(
            tid = %tid,
            fid = %fid,
            deleted_count = payload.deleted_cameras.len(),
            "Sending camera deletion notification to mobes2.0"
        );

        let oath = self.get_lacis_oath(tid).await?;
        let url = endpoints::CAMERA_METADATA;

        // 削除通知も同じエンドポイント（syncType=partialで判別）
        let wrapped_payload = serde_json::json!({
            "fid": fid,
            "payload": {
                "deletedCameras": payload.deleted_cameras,
                "syncType": "partial",
                "updatedAt": payload.updated_at
            }
        });

        let mut req = self.http.post(url)
            .header("Content-Type", "application/json")
            .json(&wrapped_payload);

        for (key, value) in oath.to_headers() {
            req = req.header(&key, &value);
        }

        match req.send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    info!(
                        tid = %tid,
                        fid = %fid,
                        "Camera deletion notification sent successfully"
                    );

                    Ok(CameraMetadataResponse {
                        success: true,
                        synced_count: payload.deleted_cameras.len(),
                        error: None,
                    })
                } else {
                    let error_body = response.text().await.unwrap_or_default();
                    let error_msg = format!("HTTP {}: {}", status.as_u16(), error_body);

                    warn!(
                        tid = %tid,
                        fid = %fid,
                        status = %status,
                        "Camera deletion notification failed"
                    );

                    Ok(CameraMetadataResponse {
                        success: false,
                        synced_count: 0,
                        error: Some(error_msg),
                    })
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                error!(tid = %tid, fid = %fid, error = %error_msg, "Camera deletion notification error");

                Err(ParaclateError::Http(error_msg))
            }
        }
    }
}

/// カメラメタデータAPIレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraMetadataResponse {
    pub success: bool,
    pub synced_count: usize,
    pub error: Option<String>,
}

impl ParaclateClient {
    // ========================================
    // AI Chat (Paraclate APP Integration)
    // ========================================

    /// AIチャット送信
    ///
    /// Paraclate_DesignOverview.md準拠:
    /// AIアシスタントタブからの質問に関してもParaclate APPからのレスポンスでチャットボット機能を行う
    ///
    /// ## 機能
    /// - 検出特徴の人物のカメラ間での移動などを横断的に把握
    /// - カメラ不調などの傾向も把握
    /// - 過去の記録を参照する、過去の記録範囲を対話的にユーザーと会話
    pub async fn send_ai_chat(
        &self,
        tid: &str,
        fid: &str,
        message: &str,
        context: Option<AIChatContext>,
        conversation_history: Option<Vec<ChatMessage>>,
    ) -> Result<AIChatResponse, ParaclateError> {
        info!(tid = %tid, fid = %fid, message_len = message.len(), "Sending AI chat to Paraclate APP");

        let oath = self.get_lacis_oath(tid).await?;
        let url = endpoints::AI_CHAT;

        // リクエストペイロード構築
        // AI Chat APIはmessageをトップレベルで期待する（payloadラッパー不要）
        let request = AIChatRequest {
            tid: tid.to_string(),
            fid: fid.to_string(),
            message: message.to_string(),
            conversation_history,
            context,
        };

        debug!(
            url = %url,
            payload = %serde_json::to_string(&request).unwrap_or_default(),
            "AI chat request"
        );

        let mut req = self.http.post(url)
            .header("Content-Type", "application/json")
            .json(&request);

        for (key, value) in oath.to_headers() {
            req = req.header(&key, &value);
        }

        let start = std::time::Instant::now();

        match req.send().await {
            Ok(response) => {
                let status = response.status();
                let elapsed_ms = start.elapsed().as_millis() as u32;

                if status.is_success() {
                    let body: serde_json::Value = response.json().await.map_err(|e| {
                        ParaclateError::Http(format!("Failed to parse AI chat response: {}", e))
                    })?;

                    // デバッグ: mobes2.0の生レスポンスをログ出力
                    debug!(
                        tid = %tid,
                        fid = %fid,
                        raw_response = %serde_json::to_string(&body).unwrap_or_default(),
                        "Raw AI chat response from mobes2.0"
                    );

                    // レスポンス解析
                    // mobes2.0は response.text にAI応答を返す
                    // { "success": true, "response": { "text": "...", "suggestions": [] } }
                    let message = body.get("response")
                        .and_then(|r| r.get("text"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        // フォールバック: トップレベルのmessage/text
                        .or_else(|| body.get("message").and_then(|v| v.as_str()).map(|s| s.to_string()))
                        .or_else(|| body.get("text").and_then(|v| v.as_str()).map(|s| s.to_string()));

                    // 会話IDを取得（将来的に会話継続に使用可能）
                    let conversation_id = body.get("conversationId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    if let Some(conv_id) = &conversation_id {
                        debug!(conversation_id = %conv_id, "AI chat conversation ID received");
                    }

                    let related_data = body.get("relatedData")
                        .or_else(|| body.get("related_data"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok());

                    info!(
                        tid = %tid,
                        fid = %fid,
                        elapsed_ms = elapsed_ms,
                        message_len = message.as_ref().map(|m| m.len()).unwrap_or(0),
                        has_message = message.is_some(),
                        "AI chat response received"
                    );

                    Ok(AIChatResponse {
                        ok: true,
                        message,
                        related_data,
                        processing_time_ms: Some(elapsed_ms),
                        error: None,
                    })
                } else {
                    let error_body = response.text().await.unwrap_or_default();
                    let error_msg = format!("HTTP {}: {}", status.as_u16(), error_body);

                    warn!(
                        tid = %tid,
                        fid = %fid,
                        status = %status,
                        error = %error_msg,
                        "AI chat request failed"
                    );

                    Ok(AIChatResponse {
                        ok: false,
                        message: None,
                        related_data: None,
                        processing_time_ms: Some(elapsed_ms),
                        error: Some(error_msg),
                    })
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                error!(tid = %tid, fid = %fid, error = %error_msg, "AI chat connection error");

                Err(ParaclateError::Http(error_msg))
            }
        }
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
        // mobes2.0形式: Authorization: LacisOath <base64-json>
        assert!(headers.iter().any(|(k, v)| {
            k == "Authorization" && v.starts_with("LacisOath ")
        }));
    }

    #[test]
    fn test_lacis_oath_headers_with_blessing() {
        let oath = LacisOath {
            lacis_id: "3022AABBCCDDEEFF0000".to_string(),
            tid: "T123".to_string(),
            cic: "123456".to_string(),
            blessing: Some("test_blessing".to_string()),
        };

        let headers = oath.to_headers();
        assert!(headers.iter().any(|(k, _)| k == "Authorization"));
        assert!(headers.iter().any(|(k, v)| {
            k == "X-Lacis-Blessing" && v == "test_blessing"
        }));
    }

    #[test]
    fn test_mobes_endpoints() {
        assert!(endpoints::CONNECT.contains("paraclateconnect"));
        assert!(endpoints::INGEST_SUMMARY.contains("paraclateingestsummary"));
        assert!(endpoints::INGEST_EVENT.contains("paraclateingestevent"));
        assert!(endpoints::GET_CONFIG.contains("paraclategetconfig"));
    }
}
