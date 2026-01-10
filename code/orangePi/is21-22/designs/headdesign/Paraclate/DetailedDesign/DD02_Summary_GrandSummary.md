# DD02: Summary/GrandSummary 詳細設計

作成日: 2026-01-10
対象: is22 Paraclate Server
ステータス: 詳細設計

## 1. 目的と概要

### 1.1 目的
is22で検出イベントを定期的に集計し、Summary（間隔ベース）およびGrandSummary（時刻指定ベース）を生成・保存・送信する機能を実装する。

### 1.2 Summary種別

| 種別 | トリガー | デフォルト | 用途 |
|------|---------|-----------|------|
| Summary | 時間間隔経過 | 60分 | 期間内の検出イベント要約 |
| GrandSummary | 指定時刻到達 | 09:00, 17:00, 21:00 | 複数Summaryの統合・1日の総括 |
| Emergency | 異常検出時 | severity >= 閾値 | 即時報告 |

### 1.3 スコープ
- 検出ログからのSummary生成ロジック
- スケジューラーによる定期実行
- ai_summary_cacheへの保存
- Paraclate APP向けペイロード生成

### 1.4 スコープ外
- Paraclate APPへの送信（DD03で設計）
- BigQuery同期（DD04で設計）
- LLM統合（Paraclate APP側の責務）

## 2. 依存関係

### 2.1 参照ドキュメント
| ドキュメント | 参照セクション |
|-------------|---------------|
| Paraclate_DesignOverview.md | Summary JSON形式 |
| 04_summary_api.md | API要件、DBスキーマ |
| Paraclate_BasicDesign.md | 処理フロー |

### 2.2 既存実装依存
| モジュール | 用途 |
|-----------|------|
| detection_log_service | 検出ログ取得 |
| config_store | 設定値取得 |
| event_log_service | イベント履歴 |

## 3. データ設計

### 3.1 既存テーブル拡張: ai_summary_cache

```sql
-- 既存テーブルに summary_json カラムを追加
ALTER TABLE ai_summary_cache
ADD COLUMN summary_json JSON NULL AFTER summary_text;
```

現状スキーマ（確認用）:
```sql
CREATE TABLE ai_summary_cache (
    summary_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32) NOT NULL,
    summary_type ENUM('hourly', 'daily', 'emergency') NOT NULL,
    period_start DATETIME(3) NOT NULL,
    period_end DATETIME(3) NOT NULL,
    summary_text TEXT NOT NULL,
    summary_json JSON NULL,                              -- 追加
    detection_count INT NOT NULL DEFAULT 0,
    severity_max INT NOT NULL DEFAULT 0,
    camera_ids JSON NOT NULL,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    expires_at DATETIME(3) NOT NULL,
    INDEX idx_tid_fid_type (tid, fid, summary_type),
    INDEX idx_period (period_start, period_end)
);
```

### 3.2 新規テーブル: scheduled_reports

```sql
CREATE TABLE scheduled_reports (
    schedule_id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32) NOT NULL,
    report_type ENUM('summary', 'grand_summary') NOT NULL,
    interval_minutes INT NULL,
    scheduled_times JSON NULL,
    last_run_at DATETIME(3),
    next_run_at DATETIME(3),
    enabled BOOLEAN DEFAULT TRUE,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    UNIQUE KEY uk_tid_fid_type (tid, fid, report_type),
    INDEX idx_next_run (next_run_at, enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

**設定例**:
```json
// Summary (間隔ベース)
{
  "tid": "T2025120621041161827",
  "fid": "0150",
  "report_type": "summary",
  "interval_minutes": 60,
  "scheduled_times": null
}

// GrandSummary (時刻指定ベース)
{
  "tid": "T2025120621041161827",
  "fid": "0150",
  "report_type": "grand_summary",
  "interval_minutes": null,
  "scheduled_times": ["09:00", "17:00", "21:00"]
}
```

### 3.3 Summary JSONペイロード形式（SSoT: Paraclate_DesignOverview.md）

```json
{
  "lacisOath": {
    "lacisID": "{is22_lacis_id}",
    "tid": "{tenant_id}",
    "cic": "{client_identification_code}",
    "blessing": null
  },
  "summaryOverview": {
    "summaryID": "{summary_id}",
    "firstDetectAt": "{timestamp_iso8601}",
    "lastDetectAt": "{timestamp_iso8601}",
    "detectedEvents": "{count}"
  },
  "cameraContext": {
    "{camera_lacis_id}": {
      "cameraName": "{name}",
      "cameraContext": "{context_description}",
      "fid": "{fid}",
      "rid": "{rid}",
      "preset": "{preset_name}"
    }
  },
  "cameraDetection": [
    {
      "timestamp": "{detection_timestamp}",
      "cameraLacisId": "{camera_lacis_id}",
      "detectionDetail": "{detail_json_string}"
    }
  ]
}
```

## 4. API設計

### 4.1 Summary生成・取得API

#### POST /api/summary/generate
手動でSummary生成をトリガー

**Request**
```json
{
  "fid": "0150",
  "periodStart": "2026-01-10T00:00:00Z",
  "periodEnd": "2026-01-10T01:00:00Z"
}
```

**Response (201 Created)**
```json
{
  "summaryId": 12345,
  "summaryType": "hourly",
  "detectionCount": 42,
  "severityMax": 3,
  "cameraIds": ["3801AABBCCDDEEFF0001", "3801AABBCCDDEEFF0002"]
}
```

#### GET /api/summary/latest
最新のSummaryを取得

**Query Parameters**
| パラメータ | 必須 | デフォルト | 説明 |
|-----------|------|-----------|------|
| fid | ❌ | 全施設 | 施設ID |
| type | ❌ | hourly | summary_type |

**Response**
```json
{
  "summaryId": 12345,
  "tid": "T2025120621041161827",
  "fid": "0150",
  "summaryType": "hourly",
  "periodStart": "2026-01-10T00:00:00Z",
  "periodEnd": "2026-01-10T01:00:00Z",
  "summaryText": "1時間の検出概要...",
  "detectionCount": 42,
  "severityMax": 3,
  "cameraIds": ["3801AABBCCDDEEFF0001"],
  "createdAt": "2026-01-10T01:00:05Z"
}
```

#### GET /api/summary/:id
特定のSummaryを取得

**Response**: 上記と同形式

#### GET /api/summary/range
期間指定でSummary一覧を取得

**Query Parameters**
| パラメータ | 必須 | 説明 |
|-----------|------|------|
| fid | ❌ | 施設ID |
| from | ✅ | 開始日時(ISO8601) |
| to | ✅ | 終了日時(ISO8601) |
| type | ❌ | hourly/daily/emergency |
| limit | ❌ | 最大件数(デフォルト100) |

**Response**
```json
{
  "summaries": [
    { /* Summary object */ },
    { /* Summary object */ }
  ],
  "total": 24,
  "hasMore": false
}
```

### 4.2 GrandSummary API

#### GET /api/grand-summary/:date
日付指定でGrandSummaryを取得

**Path Parameters**
- date: YYYY-MM-DD形式

**Response**
```json
{
  "summaryId": 999,
  "summaryType": "daily",
  "date": "2026-01-10",
  "summaryText": "1日の総括...",
  "includedSummaryIds": [12341, 12342, 12343],
  "detectionCount": 256,
  "severityMax": 5,
  "cameraIds": ["3801AABBCCDDEEFF0001", "3801AABBCCDDEEFF0002"]
}
```

#### GET /api/grand-summary/latest
最新のGrandSummaryを取得

### 4.3 スケジュール管理API

#### GET /api/reports/schedule
スケジュール設定を取得

**Response**
```json
{
  "schedules": [
    {
      "scheduleId": 1,
      "reportType": "summary",
      "intervalMinutes": 60,
      "scheduledTimes": null,
      "enabled": true,
      "lastRunAt": "2026-01-10T12:00:00Z",
      "nextRunAt": "2026-01-10T13:00:00Z"
    },
    {
      "scheduleId": 2,
      "reportType": "grand_summary",
      "intervalMinutes": null,
      "scheduledTimes": ["09:00", "17:00", "21:00"],
      "enabled": true,
      "lastRunAt": "2026-01-10T09:00:00Z",
      "nextRunAt": "2026-01-10T17:00:00Z"
    }
  ]
}
```

#### PUT /api/reports/schedule
スケジュール設定を更新

**Request**
```json
{
  "reportType": "summary",
  "intervalMinutes": 30,
  "enabled": true
}
```

## 5. モジュール構造

### 5.1 ディレクトリ構成

```
src/
├── summary_service/
│   ├── mod.rs              # モジュールエクスポート
│   ├── types.rs            # 型定義
│   ├── generator.rs        # Summary生成ロジック
│   ├── grand_summary.rs    # GrandSummary生成ロジック
│   ├── scheduler.rs        # スケジューラー
│   ├── repository.rs       # DB操作
│   └── payload_builder.rs  # Paraclate送信用ペイロード構築
├── web_api/
│   └── summary_routes.rs   # APIルート（新規追加）
```

### 5.2 型定義 (types.rs)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Summary種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum SummaryType {
    Hourly,
    Daily,
    Emergency,
}

/// Summary生成結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResult {
    pub summary_id: u64,
    pub tid: String,
    pub fid: String,
    pub summary_type: SummaryType,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub summary_text: String,
    pub summary_json: Option<serde_json::Value>,
    pub detection_count: i32,
    pub severity_max: i32,
    pub camera_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// スケジュール設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSchedule {
    pub schedule_id: u32,
    pub tid: String,
    pub fid: String,
    pub report_type: ReportType,
    pub interval_minutes: Option<i32>,
    pub scheduled_times: Option<Vec<String>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum ReportType {
    Summary,
    GrandSummary,
}

/// Summaryペイロード（Paraclate送信用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryPayload {
    pub lacis_oath: LacisOath,
    pub summary_overview: SummaryOverview,
    pub camera_context: HashMap<String, CameraContextItem>,
    pub camera_detection: Vec<CameraDetection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LacisOath {
    #[serde(rename = "lacisID")]
    pub lacis_id: String,
    pub tid: String,
    pub cic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blessing: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryOverview {
    #[serde(rename = "summaryID")]
    pub summary_id: String,
    pub first_detect_at: String,
    pub last_detect_at: String,
    pub detected_events: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraContextItem {
    pub camera_name: String,
    pub camera_context: String,
    pub fid: String,
    pub rid: String,
    pub preset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraDetection {
    pub timestamp: String,
    pub camera_lacis_id: String,
    pub detection_detail: String,
}
```

### 5.3 Summary生成ロジック (generator.rs)

```rust
use crate::detection_log_service::DetectionLogService;
use crate::summary_service::{types::*, repository::SummaryRepository};

pub struct SummaryGenerator {
    detection_log_service: Arc<DetectionLogService>,
    repository: SummaryRepository,
    camera_service: Arc<CameraService>,
}

impl SummaryGenerator {
    /// Summary生成（期間指定）
    pub async fn generate(
        &self,
        tid: &str,
        fid: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<SummaryResult, SummaryError> {
        // 1. 期間内の検出ログを取得
        let logs = self.detection_log_service
            .get_logs_by_period(tid, fid, period_start, period_end)
            .await?;

        if logs.is_empty() {
            return self.create_empty_summary(tid, fid, period_start, period_end).await;
        }

        // 2. カメラごとに集計
        let mut camera_stats: HashMap<String, CameraStats> = HashMap::new();
        let mut first_detect: Option<DateTime<Utc>> = None;
        let mut last_detect: Option<DateTime<Utc>> = None;
        let mut severity_max = 0;

        for log in &logs {
            let stats = camera_stats
                .entry(log.camera_lacis_id.clone())
                .or_insert_with(|| CameraStats::default());

            stats.detection_count += 1;
            stats.severity_max = stats.severity_max.max(log.severity);
            stats.detections.push(log.clone());

            // 最初/最後の検出時刻を更新
            if first_detect.is_none() || log.detected_at < first_detect.unwrap() {
                first_detect = Some(log.detected_at);
            }
            if last_detect.is_none() || log.detected_at > last_detect.unwrap() {
                last_detect = Some(log.detected_at);
            }

            severity_max = severity_max.max(log.severity);
        }

        // 3. カメラコンテキストを取得
        let camera_ids: Vec<String> = camera_stats.keys().cloned().collect();
        let cameras = self.camera_service.get_cameras_by_lacis_ids(&camera_ids).await?;

        // 4. summary_textを生成（テンプレートベース）
        let summary_text = self.generate_summary_text(
            &logs,
            &camera_stats,
            period_start,
            period_end,
        );

        // 5. summary_jsonを構築
        let summary_json = self.build_summary_json(
            tid,
            fid,
            &logs,
            &cameras,
            first_detect.unwrap_or(period_start),
            last_detect.unwrap_or(period_end),
        ).await?;

        // 6. DBに保存
        let result = self.repository.insert(SummaryInsert {
            tid: tid.to_string(),
            fid: fid.to_string(),
            summary_type: SummaryType::Hourly,
            period_start,
            period_end,
            summary_text,
            summary_json: Some(summary_json),
            detection_count: logs.len() as i32,
            severity_max,
            camera_ids: serde_json::to_value(&camera_ids)?,
            expires_at: Utc::now() + chrono::Duration::days(30),
        }).await?;

        Ok(result)
    }

    /// テンプレートベースのサマリーテキスト生成
    fn generate_summary_text(
        &self,
        logs: &[DetectionLog],
        camera_stats: &HashMap<String, CameraStats>,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> String {
        let duration_hours = (period_end - period_start).num_hours();
        let total_count = logs.len();
        let camera_count = camera_stats.len();

        format!(
            "{}時間の検出サマリー: 合計{}件の検出（{}台のカメラ）。\n{}",
            duration_hours,
            total_count,
            camera_count,
            self.format_camera_breakdown(camera_stats)
        )
    }

    fn format_camera_breakdown(&self, stats: &HashMap<String, CameraStats>) -> String {
        stats.iter()
            .map(|(id, s)| format!("- {}: {}件 (最大severity: {})", id, s.detection_count, s.severity_max))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Paraclate送信用JSONペイロードを構築
    async fn build_summary_json(
        &self,
        tid: &str,
        fid: &str,
        logs: &[DetectionLog],
        cameras: &[Camera],
        first_detect: DateTime<Utc>,
        last_detect: DateTime<Utc>,
    ) -> Result<serde_json::Value, SummaryError> {
        // lacisOath取得（DD01のAraneaRegisterServiceから）
        let registration = self.get_registration_info().await?;

        // cameraContextを構築
        let mut camera_context = HashMap::new();
        for camera in cameras {
            camera_context.insert(
                camera.lacis_id.clone(),
                CameraContextItem {
                    camera_name: camera.name.clone(),
                    camera_context: camera.context.clone().unwrap_or_default(),
                    fid: camera.fid.clone(),
                    rid: camera.rid.clone().unwrap_or_default(),
                    preset: camera.preset.clone().unwrap_or_default(),
                },
            );
        }

        // cameraDetectionを構築
        let camera_detection: Vec<CameraDetection> = logs.iter()
            .map(|log| CameraDetection {
                timestamp: log.detected_at.to_rfc3339(),
                camera_lacis_id: log.camera_lacis_id.clone(),
                detection_detail: serde_json::to_string(&log.detection_detail)
                    .unwrap_or_default(),
            })
            .collect();

        let payload = SummaryPayload {
            lacis_oath: LacisOath {
                lacis_id: registration.lacis_id,
                tid: tid.to_string(),
                cic: registration.cic,
                blessing: None,
            },
            summary_overview: SummaryOverview {
                summary_id: uuid::Uuid::new_v4().to_string(),
                first_detect_at: first_detect.to_rfc3339(),
                last_detect_at: last_detect.to_rfc3339(),
                detected_events: logs.len().to_string(),
            },
            camera_context,
            camera_detection,
        };

        Ok(serde_json::to_value(payload)?)
    }
}
```

### 5.4 GrandSummary生成 (grand_summary.rs)

```rust
pub struct GrandSummaryGenerator {
    summary_repository: SummaryRepository,
}

impl GrandSummaryGenerator {
    /// GrandSummary生成（複数Summaryを統合）
    pub async fn generate(
        &self,
        tid: &str,
        fid: &str,
        date: NaiveDate,
    ) -> Result<SummaryResult, SummaryError> {
        let period_start = date.and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();
        let period_end = period_start + chrono::Duration::days(1);

        // 1. 当日のhourly Summaryを全取得
        let hourly_summaries = self.summary_repository
            .get_by_period(tid, fid, SummaryType::Hourly, period_start, period_end)
            .await?;

        if hourly_summaries.is_empty() {
            return self.create_empty_grand_summary(tid, fid, date).await;
        }

        // 2. 統合情報を算出
        let mut total_detection_count = 0;
        let mut severity_max = 0;
        let mut all_camera_ids: HashSet<String> = HashSet::new();
        let mut included_summary_ids: Vec<u64> = Vec::new();

        for summary in &hourly_summaries {
            total_detection_count += summary.detection_count;
            severity_max = severity_max.max(summary.severity_max);

            if let Ok(ids) = serde_json::from_value::<Vec<String>>(summary.camera_ids.clone()) {
                all_camera_ids.extend(ids);
            }

            included_summary_ids.push(summary.summary_id);
        }

        // 3. 統合summary_text生成
        let summary_text = format!(
            "{}の日次サマリー: 合計{}件の検出（{}台のカメラ）。\n{}個のhourlyサマリーを統合。",
            date.format("%Y-%m-%d"),
            total_detection_count,
            all_camera_ids.len(),
            hourly_summaries.len()
        );

        // 4. DBに保存
        let result = self.summary_repository.insert(SummaryInsert {
            tid: tid.to_string(),
            fid: fid.to_string(),
            summary_type: SummaryType::Daily,
            period_start,
            period_end,
            summary_text,
            summary_json: Some(json!({
                "includedSummaryIds": included_summary_ids,
                "date": date.to_string()
            })),
            detection_count: total_detection_count,
            severity_max,
            camera_ids: serde_json::to_value(all_camera_ids.into_iter().collect::<Vec<_>>())?,
            expires_at: Utc::now() + chrono::Duration::days(90),
        }).await?;

        Ok(result)
    }
}
```

### 5.5 スケジューラー (scheduler.rs)

```rust
pub struct SummaryScheduler {
    repository: SummaryRepository,
    schedule_repository: ScheduleRepository,
    summary_generator: Arc<SummaryGenerator>,
    grand_summary_generator: Arc<GrandSummaryGenerator>,
    paraclate_client: Arc<ParaclateClient>, // DD03で設計
}

impl SummaryScheduler {
    /// スケジューラー起動（バックグラウンドタスク）
    pub async fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                if let Err(e) = self.tick().await {
                    tracing::error!("Scheduler tick error: {}", e);
                }

                // 1分間隔でチェック
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
    }

    /// 1回のスケジューラーティック
    async fn tick(&self) -> Result<(), SummaryError> {
        let now = Utc::now();

        // 実行待ちのスケジュールを取得
        let due_schedules = self.schedule_repository
            .get_due_schedules(now)
            .await?;

        for schedule in due_schedules {
            match schedule.report_type {
                ReportType::Summary => {
                    self.execute_summary(&schedule, now).await?;
                }
                ReportType::GrandSummary => {
                    self.execute_grand_summary(&schedule, now).await?;
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
    ) -> Result<(), SummaryError> {
        let interval = schedule.interval_minutes.unwrap_or(60);
        let period_end = now;
        let period_start = now - chrono::Duration::minutes(interval as i64);

        // Summary生成
        let result = self.summary_generator
            .generate(&schedule.tid, &schedule.fid, period_start, period_end)
            .await?;

        // Paraclate APPへ送信
        if let Some(json) = &result.summary_json {
            self.paraclate_client.send_summary(json.clone()).await?;
        }

        // 次回実行時刻を更新
        let next_run = now + chrono::Duration::minutes(interval as i64);
        self.schedule_repository
            .update_last_run(schedule.schedule_id, now, next_run)
            .await?;

        tracing::info!(
            "Summary generated for tid={}, fid={}, count={}",
            schedule.tid, schedule.fid, result.detection_count
        );

        Ok(())
    }

    /// GrandSummary実行
    async fn execute_grand_summary(
        &self,
        schedule: &ReportSchedule,
        now: DateTime<Utc>,
    ) -> Result<(), SummaryError> {
        let date = now.date_naive();

        // GrandSummary生成
        let result = self.grand_summary_generator
            .generate(&schedule.tid, &schedule.fid, date)
            .await?;

        // Paraclate APPへ送信
        if let Some(json) = &result.summary_json {
            self.paraclate_client.send_grand_summary(json.clone()).await?;
        }

        // 次回実行時刻を計算
        let next_run = self.calculate_next_grand_summary_time(
            &schedule.scheduled_times,
            now,
        )?;

        self.schedule_repository
            .update_last_run(schedule.schedule_id, now, next_run)
            .await?;

        tracing::info!(
            "GrandSummary generated for tid={}, fid={}, date={}",
            schedule.tid, schedule.fid, date
        );

        Ok(())
    }

    /// 次のGrandSummary実行時刻を計算
    fn calculate_next_grand_summary_time(
        &self,
        times: &Option<Vec<String>>,
        now: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, SummaryError> {
        let times = times.as_ref()
            .ok_or(SummaryError::InvalidSchedule)?;

        let today = now.date_naive();

        // 本日の残り時刻をチェック
        for time_str in times {
            let time = NaiveTime::parse_from_str(time_str, "%H:%M")?;
            let scheduled = today.and_time(time);
            let scheduled_utc = scheduled.and_local_timezone(Utc).unwrap();

            if scheduled_utc > now {
                return Ok(scheduled_utc);
            }
        }

        // 翌日の最初の時刻
        let tomorrow = today + chrono::Duration::days(1);
        let first_time = NaiveTime::parse_from_str(&times[0], "%H:%M")?;
        let next = tomorrow.and_time(first_time).and_local_timezone(Utc).unwrap();

        Ok(next)
    }
}
```

## 6. 処理フロー

### 6.1 Summary生成フロー

```
┌─────────────────────────────────────────────────────────────┐
│             スケジューラー (1分間隔)                          │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ scheduled_reports確認  │
              │ next_run_at <= now?   │
              └───────────────────────┘
                          │
           ┌──────────────┴──────────────┐
           │                             │
    [report_type=summary]      [report_type=grand_summary]
           │                             │
           ▼                             ▼
    ┌─────────────────┐          ┌─────────────────┐
    │ detection_logs  │          │ ai_summary_cache │
    │ 期間内取得       │          │ daily集計対象取得 │
    └─────────────────┘          └─────────────────┘
           │                             │
           ▼                             ▼
    ┌─────────────────┐          ┌─────────────────┐
    │ カメラ別集計     │          │ 複数Summary統合  │
    │ severity算出    │          │ 合計値算出       │
    └─────────────────┘          └─────────────────┘
           │                             │
           ▼                             ▼
    ┌─────────────────┐          ┌─────────────────┐
    │ summary_text    │          │ summary_text    │
    │ summary_json生成 │          │ summary_json生成 │
    └─────────────────┘          └─────────────────┘
           │                             │
           └──────────────┬──────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ ai_summary_cache保存   │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ Paraclate APP送信     │
              │ (DD03 ParaclateClient)│
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ bq_sync_queue追加     │
              │ (DD04 BQ同期)         │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ next_run_at更新       │
              └───────────────────────┘
```

### 6.2 Emergency報告フロー

```
detection_log_service
        │
        │ severity >= threshold
        ▼
┌─────────────────────┐
│ Emergency判定       │
│ suspicious.level    │
│   == critical?      │
└─────────────────────┘
        │ [YES]
        ▼
┌─────────────────────┐
│ Emergency Summary   │
│ 即時生成            │
└─────────────────────┘
        │
        ▼
┌─────────────────────┐
│ Paraclate APP       │
│ 緊急送信            │
└─────────────────────┘
```

## 7. エラーハンドリング

| エラー | 原因 | 対応 |
|-------|------|------|
| 検出ログ取得失敗 | DB接続エラー | ログ出力、次回ティックで再試行 |
| Summary保存失敗 | DB書き込みエラー | ログ出力、アラート通知 |
| Paraclate送信失敗 | ネットワークエラー | 3回リトライ後、pending状態で保持 |
| スケジュール更新失敗 | DB書き込みエラー | ログ出力、次回ティックで再計算 |

## 8. テスト計画

### 8.1 ユニットテスト

| テストケース | 内容 |
|-------------|------|
| test_generate_summary_text | テンプレート出力の妥当性 |
| test_build_summary_json | JSONペイロード構造の検証 |
| test_calculate_next_time | 次回実行時刻計算ロジック |

### 8.2 結合テスト

| テストケース | 前提条件 | 期待結果 |
|-------------|---------|---------|
| 空期間Summary | 検出ログなし | 空のSummary生成、detection_count=0 |
| 通常Summary | 検出ログ50件 | 集計完了、severity_max正確 |
| GrandSummary統合 | hourly 24件 | 合計値正確、includedSummaryIds正確 |
| スケジュール実行 | next_run_at過去 | Summary生成、next_run_at更新 |

### 8.3 E2Eテスト

| テストケース | 手順 | 確認項目 |
|-------------|------|---------|
| 定期Summary生成 | 60分待機 | 自動生成、DB保存、UI表示 |
| 手動Summary生成 | API呼び出し | 即時生成、レスポンス確認 |
| GrandSummary時刻 | 09:00到達 | 自動生成、統合正確 |

## 9. MECE/SOLID確認

### 9.1 MECE確認
- **網羅性**: Summary/GrandSummary/Emergency、生成/保存/送信/スケジュール全カバー
- **重複排除**: 生成ロジック（generator）と送信（DD03）を分離
- **未カバー領域**: LLM統合はParaclate APP側

### 9.2 SOLID確認
- **SRP**: Generator/Scheduler/Repository各々単一責務
- **OCP**: 新しいSummaryType追加時はenumと処理分岐追加のみ
- **LSP**: SummaryResult共通インターフェース
- **ISP**: 生成/取得/スケジュール管理APIを分離
- **DIP**: Repository抽象に依存

## 10. マイグレーション

### 10.1 SQLマイグレーション

ファイル: `migrations/019_summary_service.sql`

```sql
-- Migration: 019_summary_service
-- Description: Summary/GrandSummary用テーブル拡張・新規作成

-- ai_summary_cacheにsummary_jsonカラム追加
ALTER TABLE ai_summary_cache
ADD COLUMN summary_json JSON NULL AFTER summary_text;

-- scheduled_reportsテーブル作成
CREATE TABLE IF NOT EXISTS scheduled_reports (
    schedule_id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32) NOT NULL,
    report_type ENUM('summary', 'grand_summary') NOT NULL,
    interval_minutes INT NULL,
    scheduled_times JSON NULL,
    last_run_at DATETIME(3),
    next_run_at DATETIME(3),
    enabled BOOLEAN DEFAULT TRUE,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    UNIQUE KEY uk_tid_fid_type (tid, fid, report_type),
    INDEX idx_next_run (next_run_at, enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- デフォルトスケジュール挿入（初期設定用）
-- 実際のtid/fidは起動時に動的設定
```

---

**SSoT宣言**: Summary JSON形式は`Paraclate_DesignOverview.md`を正とする。
**MECE確認**: Summary/GrandSummary/Emergency全種別を網羅、生成/保存/スケジュール処理を重複なく設計。
