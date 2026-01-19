//! E2E Test Tool for IS22-mobes2.0 Paraclate Integration
//!
//! ## 使用方法
//! ```bash
//! # 全テスト実行
//! cargo run -- --is22 http://192.168.125.246:8080 --all
//!
//! # 個別テスト
//! cargo run -- --is22 http://192.168.125.246:8080 --test connect
//! cargo run -- --is22 http://192.168.125.246:8080 --test summary
//! ```

use anyhow::{anyhow, Result};
use base64::Engine;
use chrono::Utc;
use clap::Parser;
use colored::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// mobes2.0 Cloud Run エンドポイント
mod endpoints {
    pub const CONNECT: &str = "https://paraclateconnect-vm44u3kpua-an.a.run.app";
    pub const INGEST_SUMMARY: &str = "https://paraclateingestsummary-vm44u3kpua-an.a.run.app";
    pub const INGEST_EVENT: &str = "https://paraclateingestevent-vm44u3kpua-an.a.run.app";
    pub const GET_CONFIG: &str = "https://paraclategetconfig-vm44u3kpua-an.a.run.app";
}

#[derive(Parser, Debug)]
#[command(name = "paraclate-test")]
#[command(about = "E2E Test Tool for IS22-mobes2.0 Paraclate Integration")]
struct Args {
    /// IS22 server URL (e.g., http://192.168.125.246:8080)
    #[arg(long, default_value = "http://192.168.125.246:8080")]
    is22: String,

    /// Run all tests
    #[arg(long)]
    all: bool,

    /// Run specific test (connect, summary, grand_summary, event, lacis_files, malfunction, config)
    #[arg(long)]
    test: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Direct mobes2.0 test (bypass IS22)
    #[arg(long)]
    direct: bool,
}

/// テスト結果
#[derive(Debug)]
struct TestResult {
    name: String,
    success: bool,
    duration_ms: u64,
    message: String,
    details: Option<String>,
}

impl TestResult {
    fn success(name: &str, duration_ms: u64, message: &str) -> Self {
        Self {
            name: name.to_string(),
            success: true,
            duration_ms,
            message: message.to_string(),
            details: None,
        }
    }

    fn failure(name: &str, duration_ms: u64, message: &str) -> Self {
        Self {
            name: name.to_string(),
            success: false,
            duration_ms,
            message: message.to_string(),
            details: None,
        }
    }

    fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    fn print(&self) {
        let status = if self.success {
            "✅".green()
        } else {
            "❌".red()
        };
        let result = if self.success { "SUCCESS" } else { "FAILED" };
        println!(
            "{} {}: {} ({}ms)",
            status,
            self.name.bold(),
            result.to_string().color(if self.success { Color::Green } else { Color::Red }),
            self.duration_ms
        );
        if !self.message.is_empty() {
            println!("   └─ {}", self.message);
        }
        if let Some(ref details) = self.details {
            for line in details.lines() {
                println!("      {}", line.dimmed());
            }
        }
    }
}

/// 登録情報
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistrationStatus {
    registered: bool,
    lacis_id: Option<String>,
    tid: Option<String>,
    fid: Option<String>,
    cic: Option<String>,
}

/// Connect レスポンス
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectResponse {
    connected: bool,
    endpoint: String,
    config_id: Option<u32>,
    error: Option<String>,
}

/// LacisOath認証
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LacisOathPayload {
    lacis_id: String,
    tid: String,
    cic: String,
    timestamp: String,
}

impl LacisOathPayload {
    fn to_header(&self) -> String {
        let json = serde_json::to_string(self).unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(json.as_bytes());
        format!("LacisOath {}", encoded)
    }
}

/// テストランナー
struct TestRunner {
    client: Client,
    is22_url: String,
    verbose: bool,
    registration: Option<RegistrationStatus>,
}

impl TestRunner {
    fn new(is22_url: &str, verbose: bool) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            is22_url: is22_url.trim_end_matches('/').to_string(),
            verbose,
            registration: None,
        }
    }

    /// IS22の登録情報を取得
    async fn get_registration(&mut self) -> Result<&RegistrationStatus> {
        if self.registration.is_some() {
            return Ok(self.registration.as_ref().unwrap());
        }

        let url = format!("{}/api/register/status", self.is22_url);
        let resp: RegistrationStatus = self.client.get(&url).send().await?.json().await?;

        if !resp.registered {
            return Err(anyhow!("IS22 is not registered. Run device registration first."));
        }

        self.registration = Some(resp);
        Ok(self.registration.as_ref().unwrap())
    }

    /// LacisOathヘッダを生成
    fn create_auth_header(&self) -> Result<String> {
        let reg = self.registration.as_ref().ok_or(anyhow!("No registration"))?;
        let payload = LacisOathPayload {
            lacis_id: reg.lacis_id.clone().ok_or(anyhow!("No lacisId"))?,
            tid: reg.tid.clone().ok_or(anyhow!("No tid"))?,
            cic: reg.cic.clone().ok_or(anyhow!("No cic"))?,
            timestamp: Utc::now().to_rfc3339(),
        };
        Ok(payload.to_header())
    }

    /// Test 1: Connect API
    async fn test_connect(&self) -> TestResult {
        let start = Instant::now();
        let fid = self.registration.as_ref()
            .and_then(|r| r.fid.clone())
            .unwrap_or_else(|| "0150".to_string());

        let url = format!("{}/api/paraclate/connect", self.is22_url);
        let body = serde_json::json!({
            "endpoint": endpoints::CONNECT,
            "fid": fid
        });

        match self.client.post(&url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<ConnectResponse>().await {
                    Ok(data) => {
                        let duration = start.elapsed().as_millis() as u64;
                        if data.connected {
                            TestResult::success("Connect API", duration, &format!("configId={}", data.config_id.unwrap_or(0)))
                        } else {
                            TestResult::failure("Connect API", duration, &format!("Error: {}", data.error.unwrap_or_default()))
                        }
                    }
                    Err(e) => TestResult::failure("Connect API", start.elapsed().as_millis() as u64, &format!("Parse error: {}", e))
                }
            }
            Err(e) => TestResult::failure("Connect API", start.elapsed().as_millis() as u64, &format!("Request error: {}", e))
        }
    }

    /// Test 2: IngestSummary API (via IS22)
    async fn test_ingest_summary(&self) -> TestResult {
        let start = Instant::now();

        // IS22経由でSummary送信をトリガー
        let url = format!("{}/api/summary/trigger", self.is22_url);

        match self.client.post(&url).send().await {
            Ok(resp) => {
                let status = resp.status();
                let duration = start.elapsed().as_millis() as u64;

                if status.is_success() {
                    // キュー状態を確認
                    let fid = self.registration.as_ref()
                        .and_then(|r| r.fid.clone())
                        .unwrap_or_else(|| "0150".to_string());
                    let queue_url = format!("{}/api/paraclate/queue?fid={}", self.is22_url, fid);

                    if let Ok(queue_resp) = self.client.get(&queue_url).send().await {
                        if let Ok(queue_data) = queue_resp.json::<serde_json::Value>().await {
                            let pending = queue_data["stats"]["pending"].as_u64().unwrap_or(0);
                            return TestResult::success("IngestSummary", duration, &format!("Triggered, queue pending={}", pending))
                                .with_details(&format!("Queue stats: {:?}", queue_data["stats"]));
                        }
                    }
                    TestResult::success("IngestSummary", duration, "Summary trigger sent")
                } else {
                    let body = resp.text().await.unwrap_or_default();
                    TestResult::failure("IngestSummary", duration, &format!("HTTP {}: {}", status.as_u16(), body))
                }
            }
            Err(e) => TestResult::failure("IngestSummary", start.elapsed().as_millis() as u64, &format!("Request error: {}", e))
        }
    }

    /// Test 3: IngestSummary API (Direct to mobes2.0)
    async fn test_ingest_summary_direct(&self) -> TestResult {
        let start = Instant::now();
        let auth = match self.create_auth_header() {
            Ok(h) => h,
            Err(e) => return TestResult::failure("IngestSummary (Direct)", 0, &format!("Auth error: {}", e)),
        };

        let fid = self.registration.as_ref()
            .and_then(|r| r.fid.clone())
            .unwrap_or_else(|| "0150".to_string());

        // mobes2.0 API要件: summaryId がルートレベルで必須
        let summary_id = format!("e2e_summary_{}", Utc::now().timestamp_millis());
        let tid = self.registration.as_ref()
            .and_then(|r| r.tid.clone())
            .unwrap_or_default();

        // mobes2.0 API要件: deviceType + summaryId（camelCase）をルートレベルに
        let payload = serde_json::json!({
            "fid": fid,
            "deviceType": "is22",
            "summaryId": summary_id,
            "payload": {
                "tid": tid,
                "summary": {
                    "summaryType": "hourly",
                    "totalDetections": 42,
                    "cameras": [],
                    "testMode": true
                },
                "periodStart": Utc::now().format("%Y-%m-%dT%H:00:00Z").to_string(),
                "periodEnd": Utc::now().format("%Y-%m-%dT%H:59:59Z").to_string()
            }
        });

        // デバッグ: 送信するペイロードを出力
        if self.verbose {
            println!("IngestSummary payload: {}", serde_json::to_string_pretty(&payload).unwrap());
            println!("Auth header: {}", auth);
        }

        // URLにtidをパスとして追加（GetConfigと同様の形式）
        let url = format!("{}/{}", endpoints::INGEST_SUMMARY, tid);

        match self.client.post(&url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                let duration = start.elapsed().as_millis() as u64;

                if status.is_success() {
                    TestResult::success("IngestSummary (Direct)", duration, "mobes2.0 accepted")
                        .with_details(&body)
                } else {
                    TestResult::failure("IngestSummary (Direct)", duration, &format!("HTTP {}", status.as_u16()))
                        .with_details(&body)
                }
            }
            Err(e) => TestResult::failure("IngestSummary (Direct)", start.elapsed().as_millis() as u64, &format!("Request error: {}", e))
        }
    }

    /// Test 4: IngestEvent API (Direct to mobes2.0)
    async fn test_ingest_event_direct(&self) -> TestResult {
        let start = Instant::now();
        let auth = match self.create_auth_header() {
            Ok(h) => h,
            Err(e) => return TestResult::failure("IngestEvent (Direct)", 0, &format!("Auth error: {}", e)),
        };

        let fid = self.registration.as_ref()
            .and_then(|r| r.fid.clone())
            .unwrap_or_else(|| "0150".to_string());

        // mobes2.0 API要件: detection_log_id がルートレベルで必須
        let detection_log_id = format!("e2e_test_{}", Utc::now().timestamp_millis());
        let tid = self.registration.as_ref()
            .and_then(|r| r.tid.clone())
            .unwrap_or_default();

        // mobes2.0 API要件: detection_log_idはルートレベル、payload.eventは内部
        let payload = serde_json::json!({
            "fid": fid,
            "detection_log_id": detection_log_id,
            "payload": {
                "tid": tid,
                "event": {
                    "logId": 999999,
                    "cameraLacisId": "3022TEST000000000001",
                    "capturedAt": Utc::now().to_rfc3339(),
                    "primaryEvent": "test_event",
                    "severity": 1,
                    "confidence": 0.95,
                    "tags": ["e2e_test", "automated"],
                    "testMode": true
                }
            }
        });

        // デバッグ: 送信するペイロードを出力
        if self.verbose {
            println!("IngestEvent payload: {}", serde_json::to_string_pretty(&payload).unwrap());
        }

        match self.client.post(endpoints::INGEST_EVENT)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                let duration = start.elapsed().as_millis() as u64;

                if status.is_success() {
                    // eventIdを抽出
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        let event_id = json["eventId"].as_str().unwrap_or("unknown");
                        TestResult::success("IngestEvent (Direct)", duration, &format!("eventId={}", event_id))
                            .with_details(&body)
                    } else {
                        TestResult::success("IngestEvent (Direct)", duration, "mobes2.0 accepted")
                            .with_details(&body)
                    }
                } else {
                    TestResult::failure("IngestEvent (Direct)", duration, &format!("HTTP {}", status.as_u16()))
                        .with_details(&body)
                }
            }
            Err(e) => TestResult::failure("IngestEvent (Direct)", start.elapsed().as_millis() as u64, &format!("Request error: {}", e))
        }
    }

    /// Test 5: LacisFiles (snapshot Base64)
    async fn test_lacis_files(&self) -> TestResult {
        let start = Instant::now();
        let auth = match self.create_auth_header() {
            Ok(h) => h,
            Err(e) => return TestResult::failure("LacisFiles (Base64)", 0, &format!("Auth error: {}", e)),
        };

        let fid = self.registration.as_ref()
            .and_then(|r| r.fid.clone())
            .unwrap_or_else(|| "0150".to_string());

        // 小さなテスト画像（1x1 red pixel JPEG）
        let test_image_base64 = "/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgNDRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjL/wAARCAABAAEDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAn/xAAUEAEAAAAAAAAAAAAAAAAAAAAA/8QAFQEBAQAAAAAAAAAAAAAAAAAAAAX/xAAUEQEAAAAAAAAAAAAAAAAAAAAA/9oADAMBEQCEQT8AVKf/2Q==";

        // mobes2.0 API要件: detection_log_id がルートレベルで必須
        let detection_log_id = format!("e2e_snapshot_{}", Utc::now().timestamp_millis());
        let tid = self.registration.as_ref()
            .and_then(|r| r.tid.clone())
            .unwrap_or_default();

        // mobes2.0 API要件: detection_log_idはルートレベル
        let payload = serde_json::json!({
            "fid": fid,
            "detection_log_id": detection_log_id,
            "payload": {
                "tid": tid,
                "event": {
                    "logId": 999998,
                    "cameraLacisId": "3022TEST000000000001",
                    "capturedAt": Utc::now().to_rfc3339(),
                    "primaryEvent": "snapshot_test",
                    "severity": 1,
                    "confidence": 0.99,
                    "tags": ["e2e_test", "lacis_files"],
                    "snapshotBase64": test_image_base64,
                    "snapshotMimeType": "image/jpeg",
                    "testMode": true
                }
            }
        });

        match self.client.post(endpoints::INGEST_EVENT)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                let duration = start.elapsed().as_millis() as u64;

                if status.is_success() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        let storage_path = json["snapshot"]["storagePath"].as_str();
                        if let Some(path) = storage_path {
                            TestResult::success("LacisFiles (Base64)", duration, &format!("storagePath={}", path))
                                .with_details(&body)
                        } else {
                            TestResult::success("LacisFiles (Base64)", duration, "Accepted (no storagePath in response)")
                                .with_details(&body)
                        }
                    } else {
                        TestResult::success("LacisFiles (Base64)", duration, "mobes2.0 accepted")
                            .with_details(&body)
                    }
                } else {
                    TestResult::failure("LacisFiles (Base64)", duration, &format!("HTTP {}", status.as_u16()))
                        .with_details(&body)
                }
            }
            Err(e) => TestResult::failure("LacisFiles (Base64)", start.elapsed().as_millis() as u64, &format!("Request error: {}", e))
        }
    }

    /// Test 6: Camera Malfunction Report
    async fn test_malfunction(&self) -> TestResult {
        let start = Instant::now();
        let auth = match self.create_auth_header() {
            Ok(h) => h,
            Err(e) => return TestResult::failure("Malfunction Report", 0, &format!("Auth error: {}", e)),
        };

        let fid = self.registration.as_ref()
            .and_then(|r| r.fid.clone())
            .unwrap_or_else(|| "0150".to_string());

        // mobes2.0 API要件: detection_log_id がルートレベルで必須
        let detection_log_id = format!("e2e_malfunction_{}", Utc::now().timestamp_millis());
        let tid = self.registration.as_ref()
            .and_then(|r| r.tid.clone())
            .unwrap_or_default();

        // mobes2.0 API要件: detection_log_idはルートレベル
        let payload = serde_json::json!({
            "fid": fid,
            "detection_log_id": detection_log_id,
            "payload": {
                "tid": tid,
                "event": {
                    "logId": 0,
                    "cameraLacisId": "3022TEST000000000001",
                    "capturedAt": Utc::now().to_rfc3339(),
                    "primaryEvent": "camera_malfunction",
                    "severity": 3,
                    "confidence": 1.0,
                    "tags": ["malfunction", "offline", "e2e_test"],
                    "malfunctionType": "offline",
                    "malfunctionDetails": {
                        "lastSeenAt": Utc::now().to_rfc3339(),
                        "reason": "E2E test - simulated offline"
                    },
                    "testMode": true
                }
            }
        });

        match self.client.post(endpoints::INGEST_EVENT)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                let duration = start.elapsed().as_millis() as u64;

                if status.is_success() {
                    TestResult::success("Malfunction Report", duration, "mobes2.0 accepted")
                        .with_details(&body)
                } else {
                    TestResult::failure("Malfunction Report", duration, &format!("HTTP {}", status.as_u16()))
                        .with_details(&body)
                }
            }
            Err(e) => TestResult::failure("Malfunction Report", start.elapsed().as_millis() as u64, &format!("Request error: {}", e))
        }
    }

    /// Test 7: GetConfig API (GETメソッド、パスにtidとfidを含める)
    async fn test_get_config(&self) -> TestResult {
        let start = Instant::now();
        let auth = match self.create_auth_header() {
            Ok(h) => h,
            Err(e) => return TestResult::failure("GetConfig", 0, &format!("Auth error: {}", e)),
        };

        let fid = self.registration.as_ref()
            .and_then(|r| r.fid.clone())
            .unwrap_or_else(|| "0150".to_string());

        let tid = self.registration.as_ref()
            .and_then(|r| r.tid.clone())
            .unwrap_or_default();

        // GETメソッドでパスにtidを含め、fidはクエリで送信（mobes2.0 API要件）
        let url = format!("{}/{}?fid={}", endpoints::GET_CONFIG, tid, fid);

        match self.client.get(&url)
            .header("Authorization", &auth)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                let duration = start.elapsed().as_millis() as u64;

                if status.is_success() {
                    TestResult::success("GetConfig", duration, "Config retrieved")
                        .with_details(&body)
                } else if status.as_u16() == 404 {
                    // 404はConfigが未設定の正常ケース
                    TestResult::success("GetConfig", duration, "No config found (expected for new facility)")
                        .with_details(&body)
                } else {
                    TestResult::failure("GetConfig", duration, &format!("HTTP {}", status.as_u16()))
                        .with_details(&body)
                }
            }
            Err(e) => TestResult::failure("GetConfig", start.elapsed().as_millis() as u64, &format!("Request error: {}", e))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("{}", "═".repeat(60).blue());
    println!("{}", "  IS22-mobes2.0 Paraclate E2E Test Tool".bold());
    println!("{}", "═".repeat(60).blue());
    println!();
    println!("IS22 Target: {}", args.is22.cyan());
    println!("mobes2.0 Endpoints:");
    println!("  Connect:       {}", endpoints::CONNECT.dimmed());
    println!("  IngestSummary: {}", endpoints::INGEST_SUMMARY.dimmed());
    println!("  IngestEvent:   {}", endpoints::INGEST_EVENT.dimmed());
    println!("  GetConfig:     {}", endpoints::GET_CONFIG.dimmed());
    println!();

    let mut runner = TestRunner::new(&args.is22, args.verbose);

    // 登録情報を取得
    println!("{}", "Checking IS22 registration...".yellow());
    match runner.get_registration().await {
        Ok(reg) => {
            println!("  {} Registered", "✓".green());
            println!("  LacisID: {}", reg.lacis_id.as_ref().unwrap_or(&"N/A".to_string()).cyan());
            println!("  TID:     {}", reg.tid.as_ref().unwrap_or(&"N/A".to_string()));
            println!("  FID:     {}", reg.fid.as_ref().unwrap_or(&"N/A".to_string()));
            println!("  CIC:     {}", reg.cic.as_ref().unwrap_or(&"N/A".to_string()));
        }
        Err(e) => {
            println!("  {} {}", "✗".red(), e);
            return Err(e);
        }
    }
    println!();

    let mut results: Vec<TestResult> = Vec::new();

    let tests_to_run: Vec<&str> = if args.all {
        vec!["connect", "summary", "summary_direct", "event_direct", "lacis_files", "malfunction", "config"]
    } else if let Some(ref test) = args.test {
        vec![test.as_str()]
    } else {
        vec!["connect", "summary_direct", "event_direct"]
    };

    println!("{}", "Running tests...".yellow());
    println!("{}", "─".repeat(60));

    for test in &tests_to_run {
        let result = match *test {
            "connect" => runner.test_connect().await,
            "summary" => runner.test_ingest_summary().await,
            "summary_direct" => runner.test_ingest_summary_direct().await,
            "event_direct" => runner.test_ingest_event_direct().await,
            "lacis_files" => runner.test_lacis_files().await,
            "malfunction" => runner.test_malfunction().await,
            "config" => runner.test_get_config().await,
            _ => TestResult::failure(test, 0, "Unknown test"),
        };
        result.print();
        results.push(result);
    }

    println!("{}", "─".repeat(60));

    // サマリー
    let passed = results.iter().filter(|r| r.success).count();
    let failed = results.iter().filter(|r| !r.success).count();
    let total = results.len();

    println!();
    if failed == 0 {
        println!("{} All {} tests passed!", "✅".green(), total);
    } else {
        println!("{} {} passed, {} failed", "⚠️".yellow(), passed, failed);
    }

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}
