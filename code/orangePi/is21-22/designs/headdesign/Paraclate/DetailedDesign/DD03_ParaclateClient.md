# DD03: Paraclate連携API 詳細設計

作成日: 2026-01-10
対象: is22 Paraclate Server → Paraclate APP (mobes2.0) 連携
ステータス: 詳細設計

## 1. 目的と概要

### 1.1 目的
is22からParaclate APP（mobes2.0側）へのSummary送信、設定同期、接続テスト機能を実装する。
lacisOath認証を用いた安全な通信を実現する。

### 1.2 通信方向

```
┌─────────────────┐                    ┌─────────────────┐
│     is22        │                    │  Paraclate APP  │
│ (Paraclate      │  ─── HTTPS ───>    │   (mobes2.0)    │
│    Server)      │  <── Response ──   │                 │
└─────────────────┘                    └─────────────────┘
        │                                      │
        │ Summary送信                          │ Ingest処理
        │ Event送信                            │ Chat API
        │ 設定取得                             │ 設定配信
        │ 接続テスト                           │ Pub/Sub通知
```

### 1.3 スコープ
- Paraclate APP Ingest APIへのSummary/Event送信
- 設定同期（取得・適用）
- 接続テスト・状態監視
- Pub/Sub設定変更通知の受信

### 1.4 スコープ外
- Paraclate APP側のIngest処理実装（mobes2.0側）
- LLM Chat機能（mobes2.0側）
- blessing発行（mobes2.0側）

## 2. 依存関係

### 2.1 参照ドキュメント
| ドキュメント | 参照セクション |
|-------------|---------------|
| Paraclate_DesignOverview.md | lacisOath認証、Summaryペイロード |
| Paraclate_BasicDesign.md | Paraclate APP API定義 |
| 05_paraclate_integration.md | 連携詳細 |

### 2.2 内部依存
| モジュール | 用途 |
|-----------|------|
| DD01 AraneaRegister | lacisOath認証情報取得 |
| DD02 SummaryService | Summary JSONペイロード取得 |
| config_store | エンドポイント設定取得 |

### 2.3 外部依存（Paraclate APP側）
| エンドポイント | 用途 |
|---------------|------|
| POST /api/paraclate/ingest/summary | Summary受信 |
| POST /api/paraclate/ingest/event | Event受信 |
| GET /api/paraclate/config/{tid} | 設定取得 |
| POST /api/paraclate/connect | 接続テスト |

## 3. データ設計

### 3.1 設定テーブル: paraclate_config

```sql
CREATE TABLE paraclate_config (
    config_id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32) NOT NULL,
    endpoint VARCHAR(256) NOT NULL,
    report_interval_minutes INT DEFAULT 60,
    grand_summary_times JSON DEFAULT '["09:00", "17:00", "21:00"]',
    retention_days INT DEFAULT 30,
    attunement JSON DEFAULT '{}',
    sync_source_timestamp DATETIME(3),
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    UNIQUE KEY uk_tid_fid (tid, fid),
    INDEX idx_tid (tid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

**attunementフィールド例**:
```json
{
  "autoTuningEnabled": true,
  "tuningFrequency": "weekly",
  "tuningAggressiveness": 50
}
```

### 3.2 送信キュー: paraclate_send_queue

```sql
CREATE TABLE paraclate_send_queue (
    queue_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32) NOT NULL,
    payload_type ENUM('summary', 'grand_summary', 'event', 'emergency') NOT NULL,
    payload JSON NOT NULL,
    status ENUM('pending', 'sending', 'sent', 'failed') DEFAULT 'pending',
    retry_count INT DEFAULT 0,
    max_retries INT DEFAULT 3,
    last_error TEXT,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    sent_at DATETIME(3),
    INDEX idx_status (status),
    INDEX idx_tid_fid (tid, fid),
    INDEX idx_created (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

### 3.3 接続状態キャッシュ

```rust
// メモリ上の接続状態（config_storeに永続化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub last_check_at: DateTime<Utc>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub endpoint: String,
}
```

## 4. API設計

### 4.1 Paraclate APP側API（呼び出し先）

#### POST /api/paraclate/ingest/summary
Summary送信

**Request Headers**
```
Content-Type: application/json
```

**Request Body** (DD02 SummaryPayload形式)
```json
{
  "lacisOath": {
    "lacisID": "3022AABBCCDDEEFF0000",
    "tid": "T2025120621041161827",
    "cic": "123456",
    "blessing": null
  },
  "summaryOverview": {
    "summaryID": "uuid-xxx",
    "firstDetectAt": "2026-01-10T00:00:00Z",
    "lastDetectAt": "2026-01-10T00:59:59Z",
    "detectedEvents": "42"
  },
  "cameraContext": { ... },
  "cameraDetection": [ ... ]
}
```

**Response (200 OK)**
```json
{
  "ok": true,
  "summaryId": "uuid-xxx",
  "receivedAt": "2026-01-10T01:00:05Z"
}
```

**Response (401 Unauthorized)**
```json
{
  "ok": false,
  "error": "Invalid lacisOath authentication"
}
```

**Response (403 Forbidden)**
```json
{
  "ok": false,
  "error": "tid/fid mismatch or blessing required for cross-tenant access"
}
```

#### POST /api/paraclate/ingest/event
個別イベント送信（Emergency用）

**Request Body**
```json
{
  "lacisOath": { ... },
  "event": {
    "eventId": "uuid-xxx",
    "timestamp": "2026-01-10T12:34:56Z",
    "cameraLacisId": "3801AABBCCDDEEFF0001",
    "severity": 8,
    "detectionDetail": { ... }
  }
}
```

#### GET /api/paraclate/config/{tid}
設定取得（SSoT同期用）

**Query Parameters**
| パラメータ | 必須 | 説明 |
|-----------|------|------|
| fid | ❌ | 施設ID（省略時は全fid） |

**Response**
```json
{
  "tid": "T2025120621041161827",
  "configs": [
    {
      "fid": "0150",
      "endpoint": "https://api.paraclate.com/v1",
      "reportIntervalMinutes": 60,
      "grandSummaryTimes": ["09:00", "17:00", "21:00"],
      "retentionDays": 30,
      "attunement": { ... },
      "updatedAt": "2026-01-10T00:00:00Z"
    }
  ]
}
```

#### POST /api/paraclate/connect
接続テスト

**Request Body**
```json
{
  "lacisOath": { ... },
  "testType": "ping"
}
```

**Response**
```json
{
  "ok": true,
  "latencyMs": 45,
  "serverTime": "2026-01-10T12:00:00Z"
}
```

### 4.2 is22側内部API

#### GET /api/paraclate/status
Paraclate接続状態を取得

**Response**
```json
{
  "connected": true,
  "endpoint": "https://api.paraclate.com/v1",
  "lastCheckAt": "2026-01-10T12:00:00Z",
  "lastSuccessAt": "2026-01-10T12:00:00Z",
  "pendingQueueCount": 0
}
```

#### POST /api/paraclate/connect
接続テストを実行

**Response**
```json
{
  "ok": true,
  "latencyMs": 45,
  "message": "Connection successful"
}
```

#### GET /api/paraclate/config
現在の設定を取得

**Response**
```json
{
  "endpoint": "https://api.paraclate.com/v1",
  "reportIntervalMinutes": 60,
  "grandSummaryTimes": ["09:00", "17:00", "21:00"],
  "retentionDays": 30,
  "syncSourceTimestamp": "2026-01-10T00:00:00Z"
}
```

#### PUT /api/paraclate/config
設定を更新（mobes2.0へ反映後、ローカルキャッシュ更新）

**Request**
```json
{
  "reportIntervalMinutes": 30,
  "grandSummaryTimes": ["08:00", "12:00", "18:00", "22:00"]
}
```

**Response**
```json
{
  "ok": true,
  "syncedAt": "2026-01-10T12:00:00Z"
}
```

## 5. モジュール構造

### 5.1 ディレクトリ構成

```
src/
├── paraclate_client/
│   ├── mod.rs              # モジュールエクスポート
│   ├── types.rs            # 型定義
│   ├── client.rs           # HTTPクライアント本体
│   ├── config_sync.rs      # 設定同期ロジック
│   ├── send_queue.rs       # 送信キュー管理
│   └── repository.rs       # DB操作
├── web_api/
│   └── paraclate_routes.rs # APIルート（新規追加）
```

### 5.2 型定義 (types.rs)

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Paraclate API応答
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParaclateResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_at: Option<DateTime<Utc>>,
}

/// 送信ペイロード種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum PayloadType {
    Summary,
    GrandSummary,
    Event,
    Emergency,
}

/// 送信キューアイテム
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendQueueItem {
    pub queue_id: u64,
    pub tid: String,
    pub fid: String,
    pub payload_type: PayloadType,
    pub payload: serde_json::Value,
    pub status: SendStatus,
    pub retry_count: i32,
    pub max_retries: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum SendStatus {
    Pending,
    Sending,
    Sent,
    Failed,
}

/// Paraclate設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParaclateConfig {
    pub tid: String,
    pub fid: String,
    pub endpoint: String,
    pub report_interval_minutes: i32,
    pub grand_summary_times: Vec<String>,
    pub retention_days: i32,
    pub attunement: serde_json::Value,
    pub sync_source_timestamp: Option<DateTime<Utc>>,
}

/// 接続テスト結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectTestResult {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub server_time: Option<DateTime<Utc>>,
    pub error: Option<String>,
}
```

### 5.3 HTTPクライアント (client.rs)

```rust
use crate::paraclate_client::{types::*, repository::ParaclateRepository};
use crate::aranea_register::AraneaRegisterService;

pub struct ParaclateClient {
    http_client: reqwest::Client,
    repository: ParaclateRepository,
    register_service: Arc<AraneaRegisterService>,
    config_store: Arc<ConfigStore>,
}

impl ParaclateClient {
    pub fn new(
        pool: Pool<MySql>,
        register_service: Arc<AraneaRegisterService>,
        config_store: Arc<ConfigStore>,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            repository: ParaclateRepository::new(pool),
            register_service,
            config_store,
        }
    }

    /// Summary送信
    ///
    /// # Arguments
    /// * `payload` - Summaryペイロード
    /// * `tid` - テナントID（scheduler/configから取得した確定値）
    /// * `fid` - 施設ID（scheduler/configから取得した確定値、推測禁止）
    pub async fn send_summary(
        &self,
        payload: serde_json::Value,
        tid: &str,
        fid: &str,
    ) -> Result<ParaclateResponse, ParaclateError> {
        self.send_payload(PayloadType::Summary, payload, tid, fid).await
    }

    /// GrandSummary送信
    ///
    /// # Arguments
    /// * `payload` - GrandSummaryペイロード
    /// * `tid` - テナントID
    /// * `fid` - 施設ID（推測禁止）
    pub async fn send_grand_summary(
        &self,
        payload: serde_json::Value,
        tid: &str,
        fid: &str,
    ) -> Result<ParaclateResponse, ParaclateError> {
        self.send_payload(PayloadType::GrandSummary, payload, tid, fid).await
    }

    /// Emergency Event送信
    ///
    /// # Arguments
    /// * `payload` - Eventペイロード
    /// * `tid` - テナントID
    /// * `fid` - 施設ID（推測禁止）
    pub async fn send_emergency(
        &self,
        payload: serde_json::Value,
        tid: &str,
        fid: &str,
    ) -> Result<ParaclateResponse, ParaclateError> {
        self.send_payload(PayloadType::Emergency, payload, tid, fid).await
    }

    /// ペイロード送信（共通処理）
    ///
    /// # Arguments
    /// * `payload_type` - 送信種別
    /// * `payload` - 送信データ（lacisOath.fidを必須で含むこと）
    /// * `tid` - テナントID（呼び出し側から明示的に渡す）
    /// * `fid` - 施設ID（呼び出し側から明示的に渡す、推測禁止）
    async fn send_payload(
        &self,
        payload_type: PayloadType,
        payload: serde_json::Value,
        tid: &str,
        fid: &str,
    ) -> Result<ParaclateResponse, ParaclateError> {
        // エンドポイント取得
        let endpoint = self.get_endpoint().await?;

        // tid/fidバリデーション（空文字やfallback値は禁止）
        if tid.is_empty() {
            return Err(ParaclateError::MissingField("tid"));
        }
        if fid.is_empty() || fid == "0000" {
            // fid="0000"（全施設）への送信は禁止（権限境界違反）
            return Err(ParaclateError::InvalidFid("fid must be explicitly specified, not 0000"));
        }

        // キューに追加（tid/fidはenqueue時点で確定、推測禁止）
        let queue_id = self.repository.enqueue(SendQueueItem {
            queue_id: 0,
            tid: tid.to_string(),
            fid: fid.to_string(),  // scheduler/configから確定値として渡される
            payload_type,
            payload: payload.clone(),
            status: SendStatus::Pending,
            retry_count: 0,
            max_retries: 3,
            last_error: None,
            created_at: Utc::now(),
            sent_at: None,
        }).await?;

        // 即時送信を試行
        let result = self.try_send(queue_id, &endpoint, &payload, payload_type).await;

        // キュー状態を更新
        match &result {
            Ok(response) if response.ok => {
                self.repository.mark_sent(queue_id).await?;
            }
            Ok(response) => {
                self.repository.mark_failed(queue_id, response.error.as_deref()).await?;
            }
            Err(e) => {
                self.repository.increment_retry(queue_id, &e.to_string()).await?;
            }
        }

        result
    }

    /// 実際のHTTP送信
    async fn try_send(
        &self,
        queue_id: u64,
        endpoint: &str,
        payload: &serde_json::Value,
        payload_type: PayloadType,
    ) -> Result<ParaclateResponse, ParaclateError> {
        let path = match payload_type {
            PayloadType::Summary | PayloadType::GrandSummary => "/ingest/summary",
            PayloadType::Event | PayloadType::Emergency => "/ingest/event",
        };

        let url = format!("{}{}", endpoint, path);

        tracing::info!("Sending {} to {}", payload_type_name(payload_type), url);

        // ステータスを送信中に更新
        self.repository.mark_sending(queue_id).await?;

        let response = self.http_client
            .post(&url)
            .json(payload)
            .send()
            .await?;

        let status = response.status();
        let body: ParaclateResponse = response.json().await?;

        if !status.is_success() {
            tracing::error!(
                "Paraclate API error: {} - {:?}",
                status,
                body.error
            );
        }

        Ok(body)
    }

    /// 接続テスト
    pub async fn test_connection(&self) -> Result<ConnectTestResult, ParaclateError> {
        let endpoint = self.get_endpoint().await?;
        let url = format!("{}/connect", endpoint);

        // lacisOath取得
        let registration = self.register_service.get_registration_status().await?;
        let lacis_id = registration.lacis_id.ok_or(ParaclateError::NotRegistered)?;
        let cic = self.config_store.get("aranea.cic").await?;
        let tid = self.config_store.get("aranea.tid").await?;

        let payload = json!({
            "lacisOath": {
                "lacisID": lacis_id,
                "tid": tid,
                "cic": cic
            },
            "testType": "ping"
        });

        let start = std::time::Instant::now();

        let response = self.http_client
            .post(&url)
            .json(&payload)
            .send()
            .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match response {
            Ok(res) => {
                let body: serde_json::Value = res.json().await?;
                let ok = body["ok"].as_bool().unwrap_or(false);

                // 接続状態を更新
                self.update_connection_status(ok, None).await?;

                Ok(ConnectTestResult {
                    ok,
                    latency_ms: Some(latency_ms),
                    server_time: body["serverTime"]
                        .as_str()
                        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    error: body["error"].as_str().map(String::from),
                })
            }
            Err(e) => {
                self.update_connection_status(false, Some(&e.to_string())).await?;

                Ok(ConnectTestResult {
                    ok: false,
                    latency_ms: None,
                    server_time: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// エンドポイント取得
    async fn get_endpoint(&self) -> Result<String, ParaclateError> {
        self.config_store
            .get("paraclate.endpoint")
            .await
            .map_err(|_| ParaclateError::EndpointNotConfigured)
    }

    /// 接続状態を更新
    async fn update_connection_status(
        &self,
        connected: bool,
        error: Option<&str>,
    ) -> Result<(), ParaclateError> {
        let now = Utc::now().to_rfc3339();

        self.config_store.set("paraclate.connected", &connected.to_string()).await?;
        self.config_store.set("paraclate.last_check_at", &now).await?;

        if connected {
            self.config_store.set("paraclate.last_success_at", &now).await?;
        }

        if let Some(e) = error {
            self.config_store.set("paraclate.last_error", e).await?;
        }

        Ok(())
    }
}
```

### 5.4 設定同期 (config_sync.rs)

```rust
pub struct ConfigSyncService {
    client: Arc<ParaclateClient>,
    repository: ParaclateRepository,
    config_store: Arc<ConfigStore>,
}

impl ConfigSyncService {
    /// mobes2.0から設定を取得して同期
    pub async fn sync_from_mobes(&self) -> Result<(), ParaclateError> {
        let tid = self.config_store.get("aranea.tid").await?;
        let endpoint = self.config_store.get("paraclate.endpoint").await?;

        let url = format!("{}/config/{}", endpoint, tid);

        let response = self.client.http_client
            .get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ParaclateError::ConfigSyncFailed);
        }

        let body: serde_json::Value = response.json().await?;
        let configs = body["configs"].as_array()
            .ok_or(ParaclateError::InvalidResponse)?;

        for config in configs {
            let fid = config["fid"].as_str().unwrap_or("0000");
            let updated_at = config["updatedAt"].as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            // ローカルのタイムスタンプと比較
            let local = self.repository.get_config(&tid, fid).await?;
            let should_update = match (local.as_ref().and_then(|c| c.sync_source_timestamp), updated_at) {
                (Some(local_ts), Some(remote_ts)) => remote_ts > local_ts,
                (None, Some(_)) => true,
                _ => false,
            };

            if should_update {
                let pc = ParaclateConfig {
                    tid: tid.clone(),
                    fid: fid.to_string(),
                    endpoint: config["endpoint"].as_str().unwrap_or(&endpoint).to_string(),
                    report_interval_minutes: config["reportIntervalMinutes"].as_i64().unwrap_or(60) as i32,
                    grand_summary_times: config["grandSummaryTimes"]
                        .as_array()
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_else(|| vec!["09:00".to_string(), "17:00".to_string(), "21:00".to_string()]),
                    retention_days: config["retentionDays"].as_i64().unwrap_or(30) as i32,
                    attunement: config["attunement"].clone(),
                    sync_source_timestamp: updated_at,
                };

                self.repository.upsert_config(&pc).await?;

                tracing::info!("Config synced for tid={}, fid={}", tid, fid);
            }
        }

        Ok(())
    }

    /// ローカル設定をmobes2.0へ送信（SSoT違反注意）
    pub async fn push_to_mobes(&self, config: &ParaclateConfig) -> Result<(), ParaclateError> {
        // 注意: SSoTはmobes2.0側なので、この操作は慎重に
        // 通常は設定変更はmobes2.0側UIから行い、Pub/Subで通知を受ける

        let endpoint = self.config_store.get("paraclate.endpoint").await?;
        let url = format!("{}/config/{}", endpoint, config.tid);

        let payload = json!({
            "fid": config.fid,
            "reportIntervalMinutes": config.report_interval_minutes,
            "grandSummaryTimes": config.grand_summary_times,
            "retentionDays": config.retention_days,
            "attunement": config.attunement
        });

        let response = self.client.http_client
            .put(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ParaclateError::ConfigPushFailed);
        }

        Ok(())
    }
}
```

### 5.5 送信キュー処理 (send_queue.rs)

```rust
pub struct SendQueueProcessor {
    client: Arc<ParaclateClient>,
    repository: ParaclateRepository,
}

impl SendQueueProcessor {
    /// バックグラウンドでキューを処理
    pub async fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                if let Err(e) = self.process_pending().await {
                    tracing::error!("Queue processing error: {}", e);
                }

                // 10秒間隔で処理
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });
    }

    /// 保留中のアイテムを処理
    async fn process_pending(&self) -> Result<(), ParaclateError> {
        let pending_items = self.repository
            .get_pending_items(10)
            .await?;

        for item in pending_items {
            // リトライ上限チェック
            if item.retry_count >= item.max_retries {
                self.repository.mark_failed(
                    item.queue_id,
                    Some("Max retries exceeded"),
                ).await?;
                continue;
            }

            // 指数バックオフ
            let backoff_seconds = 2u64.pow(item.retry_count as u32) * 5;
            let since_last = Utc::now() - item.created_at;
            if since_last < chrono::Duration::seconds(backoff_seconds as i64) {
                continue;
            }

            // 再送信
            let endpoint = self.client.get_endpoint().await?;
            let result = self.client.try_send(
                item.queue_id,
                &endpoint,
                &item.payload,
                item.payload_type,
            ).await;

            match result {
                Ok(response) if response.ok => {
                    self.repository.mark_sent(item.queue_id).await?;
                    tracing::info!("Queue item {} sent successfully", item.queue_id);
                }
                Ok(response) => {
                    self.repository.increment_retry(
                        item.queue_id,
                        response.error.as_deref().unwrap_or("Unknown error"),
                    ).await?;
                }
                Err(e) => {
                    self.repository.increment_retry(
                        item.queue_id,
                        &e.to_string(),
                    ).await?;
                }
            }
        }

        Ok(())
    }
}
```

## 6. 処理フロー

### 6.1 Summary送信フロー

```
┌─────────────────────────────────────────────────────────────┐
│           SummaryScheduler (DD02)                            │
│           Summary生成完了                                     │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ ParaclateClient.      │
              │   send_summary()      │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ paraclate_send_queue  │
              │ INSERT (pending)      │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ try_send()            │
              │ HTTP POST → mobes2.0  │
              └───────────────────────┘
                          │
           ┌──────────────┴──────────────┐
           │                             │
        [成功]                        [失敗]
           │                             │
           ▼                             ▼
    ┌─────────────┐            ┌───────────────────┐
    │ status=sent │            │ status=pending    │
    │ sent_at更新  │            │ retry_count++     │
    └─────────────┘            │ last_error更新    │
                               └───────────────────┘
                                         │
                                         ▼
                               ┌───────────────────┐
                               │ SendQueueProcessor│
                               │ 10秒後再試行      │
                               │ 指数バックオフ    │
                               └───────────────────┘
```

### 6.2 設定同期フロー

```
┌─────────────────────────────────────────────────────────────┐
│  mobes2.0 AI Model Settings                                  │
│  (SSoT)                                                      │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ Pub/Sub通知 or 定期ポーリング
                          ▼
              ┌───────────────────────┐
              │ ConfigSyncService.    │
              │   sync_from_mobes()   │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ GET /config/{tid}     │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ タイムスタンプ比較     │
              │ 新しければ更新        │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ paraclate_config      │
              │ UPSERT               │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ SummaryScheduler     │
              │ スケジュール再計算   │
              └───────────────────────┘
```

## 7. エラーハンドリング

| エラー | 原因 | 対応 |
|-------|------|------|
| EndpointNotConfigured | エンドポイント未設定 | 設定UIでエンドポイント入力を促す |
| NotRegistered | デバイス未登録 | AraneaRegister完了を待つ |
| MissingField | 必須フィールド未指定 | 呼び出し側でtid/fid等を明示的に指定 |
| InvalidFid | fid不正（空、0000等） | scheduler/configから正しいfidを取得して渡す |
| AuthenticationFailed | lacisOath認証失敗 | CIC再取得、再登録を検討 |
| RateLimited | API制限 | 指数バックオフで再試行 |
| NetworkError | 接続失敗 | キューに保持、自動再試行 |
| ConfigSyncFailed | 設定取得失敗 | ローカルキャッシュを継続使用 |

## 8. セキュリティ考慮

### 8.1 通信セキュリティ
- HTTPS必須（HTTP拒否）
- TLS 1.2以上
- 証明書検証有効

### 8.2 認証
- 全リクエストにlacisOath必須
- CICは永続化して再利用
- blessingは越境時のみ（通常null）

### 8.3 監査
- 送信成功/失敗をログ出力
- キュー状態をDB保持
- 接続状態履歴を保持

## 9. テスト計画

### 9.1 ユニットテスト

| テストケース | 内容 |
|-------------|------|
| test_send_payload_success | 正常送信→status=sent |
| test_send_payload_retry | 失敗→retry_count増加 |
| test_connection_test | 接続テスト成功/失敗 |

### 9.2 結合テスト

| テストケース | 前提条件 | 期待結果 |
|-------------|---------|---------|
| Summary送信E2E | 登録済み、エンドポイント設定済み | 送信成功、DB更新 |
| 設定同期 | mobes2.0設定変更 | ローカル反映 |
| リトライ | ネットワーク断 | 3回リトライ後failed |

### 9.3 E2Eテスト

| テストケース | 手順 | 確認項目 |
|-------------|------|---------|
| 接続テスト実行 | UI「接続テスト」ボタン | 成功表示、latency表示 |
| キュー処理確認 | ネットワーク断→復旧 | 自動再送信、status=sent |

## 10. MECE/SOLID確認

### 10.1 MECE確認
- **網羅性**: 送信/設定同期/接続テスト/キュー処理を全カバー
- **重複排除**: Summary生成（DD02）と送信（DD03）を分離
- **未カバー領域**: Pub/Sub詳細はmobes2.0側の仕様待ち

### 10.2 SOLID確認
- **SRP**: Client/ConfigSync/QueueProcessor各々単一責務
- **OCP**: 新PayloadType追加はenumと分岐追加のみ
- **LSP**: ParaclateResponse共通インターフェース
- **ISP**: 送信/設定/テストAPIを分離
- **DIP**: ConfigStore/Repository抽象に依存

## 11. マイグレーション

### 11.1 SQLマイグレーション

ファイル: `migrations/020_paraclate_client.sql`

```sql
-- Migration: 020_paraclate_client
-- Description: Paraclate連携用テーブル作成

CREATE TABLE IF NOT EXISTS paraclate_config (
    config_id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32) NOT NULL,
    endpoint VARCHAR(256) NOT NULL,
    report_interval_minutes INT DEFAULT 60,
    grand_summary_times JSON DEFAULT '["09:00", "17:00", "21:00"]',
    retention_days INT DEFAULT 30,
    attunement JSON DEFAULT '{}',
    sync_source_timestamp DATETIME(3),
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    UNIQUE KEY uk_tid_fid (tid, fid),
    INDEX idx_tid (tid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS paraclate_send_queue (
    queue_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32) NOT NULL,
    payload_type ENUM('summary', 'grand_summary', 'event', 'emergency') NOT NULL,
    payload JSON NOT NULL,
    status ENUM('pending', 'sending', 'sent', 'failed') DEFAULT 'pending',
    retry_count INT DEFAULT 0,
    max_retries INT DEFAULT 3,
    last_error TEXT,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    sent_at DATETIME(3),
    INDEX idx_status (status),
    INDEX idx_tid_fid (tid, fid),
    INDEX idx_created (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

---

**SSoT宣言**: 設定SSoTはmobes2.0側。is22はキャッシュとして保持し、sync_source_timestampで同期管理。
**MECE確認**: 送信/設定同期/接続テスト/キュー処理を網羅、重複なし。
