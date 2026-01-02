# IS22 AI Event Log 設計書

## 改訂履歴
| 日付 | バージョン | 変更内容 |
|------|----------|----------|
| 2026-01-02 | 1.0.0 | 初版作成 |

---

## 1. 概要

### 1.1 目的
AI Event Logは、IS22 CamServerで巡回取得した画像をis21 Edge AIに送信し、検知結果をUI表示・DB保存・BQ同期する機能である。

### 1.2 システム構成
```
[カメラ] → [is22 PollingOrchestrator]
              ↓ snapshot取得
         [is22 AI Client]
              ↓ POST /v1/analyze
         [is21 Edge AI] (192.168.3.240:9000)
              ↓ 解析結果
         [is22 Detection Log]
              ↓ 検知あり時のみ保存
         [MySQL detection_logs] → [BQ Sync] → [BigQuery]
              ↓
         [WebSocket通知] → [Frontend EventLogPane]
```

### 1.3 スコープ
#### 本ターン対象
- is21連携（画像送信・結果受信）
- 検知ログDB保存
- UI表示（EventLogPane改修）
- BQ同期基盤

#### 次ターン対象（スコープ外）
- mobes2.0系連携（AIチャット・サマリー）
- go2rtx動画再生

---

## 2. データフロー

### 2.1 画像解析フロー
```
1. PollingOrchestrator: カメラからスナップショット取得
2. SnapshotService: 画像をローカル保存
3. AiClient: is21へ画像+コンテキスト送信
4. is21: YOLOv5s + PAR解析
5. AiClient: レスポンス受信
6. DetectionLogService:
   - primary_event != "none" の場合のみDB保存
   - BQ Sync Queue登録
7. RealtimeHub: WebSocket通知 (event_log)
8. Frontend: EventLogPane更新
```

### 2.2 is21 API仕様

#### エンドポイント
```
POST http://192.168.3.240:9000/v1/analyze
Content-Type: multipart/form-data
```

#### リクエストパラメータ
| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| camera_id | string | ○ | カメラID |
| captured_at | string | ○ | ISO8601形式タイムスタンプ |
| schema_version | string | ○ | "2025-12-29.1" |
| infer_image | file | ○ | JPEG/PNG画像（現在のフレーム） |
| prev_image | file | - | JPEG/PNG画像（前回のフレーム、差分分析用） |
| preset_id | string | ○ | 使用プリセットID (例: "parking", "balanced") |
| preset_version | string | ○ | プリセットバージョン (例: "1.0.0") |
| output_schema | string | - | 期待する出力形式 (例: "parking", "person_detailed") |
| return_bboxes | bool | - | bbox返却 (default: true) |
| enable_frame_diff | bool | - | 差分分析有効化 (default: true, prev_image送信時) |
| hints_json | string | - | camera_context JSON |

#### hints_json (camera_context)
```json
{
  "location_type": "entrance|corridor|parking|outdoor|indoor",
  "distance": "close|medium|far",
  "expected_objects": ["human", "vehicle"],
  "excluded_objects": ["cat"],
  "busy_hours": ["08:00-09:00", "18:00-19:00"],
  "quiet_hours": ["22:00-06:00"],
  "previous_frame": {
    "captured_at": "2026-01-02T00:30:00Z",
    "person_count": 2,
    "primary_event": "human"
  }
}
```

#### レスポンス形式
```json
{
  "schema_version": "2025-12-29.1",
  "camera_id": "cam-xxx",
  "captured_at": "2026-01-02T00:34:11Z",
  "preset_id": "parking",
  "preset_version": "1.0.0",
  "output_schema": "parking",
  "analyzed": true,
  "detected": true,
  "primary_event": "human|animal|vehicle|none|unknown",
  "tags": ["outfit.dress", "gender.female", "carry.bag"],
  "confidence": 0.85,
  "severity": 1,
  "unknown_flag": false,
  "count_hint": 1,
  "bboxes": [
    {
      "x1": 0.0, "y1": 0.05, "x2": 0.59, "y2": 0.82,
      "label": "person",
      "conf": 0.85,
      "par": { "tags": [...], "meta": {...} },
      "details": { "top_color": {...}, "bottom_color": {...} }
    }
  ],
  "person_details": [
    {
      "index": 0,
      "top_color": { "color": "white", "confidence": 0.70 },
      "bottom_color": { "color": "red", "confidence": 0.74 },
      "uniform_like": true,
      "body_size": "large",
      "body_build": "heavy",
      "posture": { "type": "crouching", "confidence": 0.6 },
      "height_category": "tall",
      "meta": { "gender": "female", "age_group": "adult" }
    }
  ],
  "suspicious": {
    "score": 28,
    "level": "normal|low|medium|high|critical",
    "factors": ["time.night[0:00] (+10)", "crouching_posture (+10)"]
  },
  "frame_diff": {
    "enabled": true,
    "person_changes": {
      "appeared": 1,
      "disappeared": 0,
      "moved": 1,
      "stationary": 0
    },
    "movement_vectors": [
      { "index": 0, "dx": 0.05, "dy": -0.02, "magnitude": 0.054 }
    ],
    "loitering": {
      "detected": false,
      "loiterers": []
    },
    "scene_change": {
      "level": "minor",
      "background_changed": false
    },
    "camera_status": {
      "frozen": false,
      "tampered": false,
      "shifted": false
    }
  },
  "context_applied": true,
  "processing_ms": { "total": 265, "yolo": 38, "par": 27, "frame_diff": 25 }
}
```

---

## 3. データベース設計

### 3.1 detection_logs テーブル
**保存条件**: `primary_event != "none"` の場合のみ

| カラム | 型 | 説明 | BQ同期 |
|--------|-----|------|--------|
| log_id | BIGINT | 主キー | ○ |
| tid | VARCHAR(32) | テナントID | ○ |
| fid | VARCHAR(32) | 施設ID | ○ |
| camera_id | VARCHAR(64) | カメラID | ○ |
| lacis_id | VARCHAR(32) | カメラlacisID | ○ |
| captured_at | DATETIME(3) | キャプチャ日時 | ○ |
| analyzed_at | DATETIME(3) | 解析完了日時 | ○ |
| primary_event | VARCHAR(32) | 主イベント | ○ |
| severity | INT | 重要度 (0-3) | ○ |
| confidence | DECIMAL(5,4) | 信頼度 | ○ |
| count_hint | INT | 検出数 | ○ |
| unknown_flag | BOOLEAN | 未知フラグ | ○ |
| tags | JSON | タグ配列 | ○ |
| person_details | JSON | PAR詳細 | ○ |
| bboxes | JSON | 検出ボックス | ○ |
| suspicious | JSON | 不審スコア | ○ |
| frame_diff | JSON | 差分分析結果 | ○ |
| loitering_detected | BOOLEAN | 滞在検知フラグ | ○ |
| preset_id | VARCHAR(32) | 使用プリセットID | ○ |
| preset_version | VARCHAR(16) | プリセットバージョン | ○ |
| output_schema | VARCHAR(32) | 出力スキーマ名 | ○ |
| context_applied | BOOLEAN | コンテキスト適用 | ○ |
| camera_context | JSON | 送信コンテキスト | ○ |
| is21_log | JSON | 完全レスポンス | ○ |
| camera_response | JSON | カメラレスポンス | ○ |
| image_path_local | VARCHAR(512) | ローカルパス | ○ |
| image_path_cloud | VARCHAR(512) | クラウドパス | ○ |
| processing_ms | INT | 処理時間 | ○ |
| schema_version | VARCHAR(32) | スキーマVer | ○ |

### 3.2 BQ同期
- **同期方式**: bq_sync_queue経由の非同期バッチ
- **同期間隔**: 60秒ごと or キュー100件達成
- **カラム完全一致**: ローカルDBとBQのカラム定義を同一にする

---

## 4. UI設計

### 4.1 レイアウト
EventLogPane（右ペイン25%）を上下50%/50%に分割：

```
┌──────────────────────────┐
│    is21検知ログ (50%)     │ ← 検知ありログのみ表示
├──────────────────────────┤
│  AIチャット&サマリー (50%) │ ← 次ターンで実装
└──────────────────────────┘
```

### 4.2 is21検知ログ表示

#### 表示項目（1行あたり）
```
[時刻] [カメラ名] [primary_event] [severity badge]
[tags抜粋] [suspicious.level badge]
```

#### フィルタ機能
- primary_event別
- severity別
- suspicious.level別
- 時間範囲

#### クリック動作
- ログクリック → 該当カメラのスナップショット表示
- 詳細展開 → bboxes, person_details表示

### 4.3 非表示条件
- `primary_event == "none"` は非表示
- `detected == false` は非表示

---

## 5. is22実装仕様

### 5.1 モジュール構成

#### ai_client/mod.rs
```rust
pub struct AiClient {
    is21_endpoint: String,
    timeout: Duration,
}

impl AiClient {
    pub async fn analyze(&self, req: AnalyzeRequest) -> Result<AnalyzeResponse, AiError>;
}

pub struct AnalyzeRequest {
    pub camera_id: String,
    pub captured_at: DateTime<Utc>,
    pub image_data: Vec<u8>,
    pub camera_context: Option<CameraContext>,
}

pub struct AnalyzeResponse {
    pub primary_event: String,
    pub tags: Vec<String>,
    pub severity: i32,
    pub confidence: f64,
    pub bboxes: Vec<BBox>,
    pub person_details: Vec<PersonDetail>,
    pub suspicious: SuspiciousInfo,
    // ... 他フィールド
}
```

#### detection_log/mod.rs
```rust
pub struct DetectionLogService {
    db: Pool<MySql>,
    bq_queue: BqSyncQueue,
}

impl DetectionLogService {
    pub async fn save_detection(&self, response: &AnalyzeResponse, meta: &DetectionMeta) -> Result<i64, DbError>;
    pub async fn get_recent_logs(&self, limit: usize) -> Result<Vec<DetectionLog>, DbError>;
    pub async fn search_logs(&self, query: &LogQuery) -> Result<Vec<DetectionLog>, DbError>;
}
```

### 5.2 PollingOrchestrator統合

#### prev_image管理（フレーム差分用）
```rust
// polling_orchestrator/types.rs
use std::collections::HashMap;
use std::sync::RwLock;

/// 前回フレームキャッシュ（カメラ別）
pub struct PrevFrameCache {
    /// camera_id -> (captured_at, image_data, detection_summary)
    frames: RwLock<HashMap<String, PrevFrameData>>,
}

pub struct PrevFrameData {
    pub captured_at: DateTime<Utc>,
    pub image_data: Vec<u8>,
    pub person_count: i32,
    pub primary_event: String,
}

impl PrevFrameCache {
    pub fn new() -> Self {
        Self { frames: RwLock::new(HashMap::new()) }
    }

    /// 前回フレームを取得
    pub fn get(&self, camera_id: &str) -> Option<PrevFrameData> {
        self.frames.read().unwrap().get(camera_id).cloned()
    }

    /// 現在フレームを保存（次回の prev_image として使用）
    pub fn update(&self, camera_id: &str, data: PrevFrameData) {
        self.frames.write().unwrap().insert(camera_id.to_string(), data);
    }

    /// カメラ削除時にキャッシュクリア
    pub fn remove(&self, camera_id: &str) {
        self.frames.write().unwrap().remove(camera_id);
    }
}
```

#### PollingOrchestrator本体
```rust
// polling_orchestrator/mod.rs
pub struct PollingOrchestrator {
    // ...
    prev_frame_cache: Arc<PrevFrameCache>,
}

async fn poll_camera(&self, camera: &Camera) -> Result<(), PollError> {
    // 1. スナップショット取得
    let snapshot = self.snapshot_service.capture(camera).await?;

    // 2. 前回フレーム取得（frame_diff用）
    let prev_frame = self.prev_frame_cache.get(&camera.camera_id);

    // 3. hints_json構築（previous_frame情報を含む）
    let mut hints = camera.camera_context.clone().unwrap_or_default();
    if let Some(ref prev) = prev_frame {
        hints.previous_frame = Some(PreviousFrameHint {
            captured_at: prev.captured_at,
            person_count: prev.person_count,
            primary_event: prev.primary_event.clone(),
        });
    }

    // 4. is21解析リクエスト構築
    let result = self.ai_client.analyze(AnalyzeRequest {
        camera_id: camera.camera_id.clone(),
        captured_at: snapshot.captured_at,
        image_data: snapshot.data.clone(),
        prev_image: prev_frame.map(|p| p.image_data),  // ← prev_image送信
        enable_frame_diff: prev_frame.is_some(),       // ← 前回フレームがあれば有効化
        preset_id: camera.preset_id.clone(),
        preset_version: camera.preset_version.clone(),
        camera_context: Some(hints),
    }).await?;

    // 5. 現在フレームを prev_frame_cache に保存（次回用）
    self.prev_frame_cache.update(&camera.camera_id, PrevFrameData {
        captured_at: snapshot.captured_at,
        image_data: snapshot.data.clone(),
        person_count: result.count_hint,
        primary_event: result.primary_event.clone(),
    });

    // 6. 検知ありの場合のみ保存
    if result.primary_event != "none" && result.detected {
        // loitering_detected フラグ抽出
        let loitering_detected = result.frame_diff
            .as_ref()
            .map(|fd| fd.loitering.detected)
            .unwrap_or(false);

        self.detection_log_service.save_detection(&result, &DetectionMeta {
            tid: camera.tid.clone(),
            fid: camera.fid.clone(),
            lacis_id: camera.lacis_id.clone(),
            image_path_local: snapshot.local_path.clone(),
            frame_diff: result.frame_diff.clone(),
            loitering_detected,
        }).await?;

        // 7. WebSocket通知
        self.realtime_hub.broadcast(WsMessage::EventLog(result.clone())).await;
    }

    Ok(())
}
```

---

## 6. camera_context設計

### 6.1 CameraDetailModalでの設定
cameras.camera_context JSONフィールドに保存：

```json
{
  "location_type": "entrance",
  "distance": "medium",
  "expected_objects": ["human"],
  "excluded_objects": [],
  "busy_hours": ["08:00-10:00", "17:00-19:00"],
  "quiet_hours": ["23:00-06:00"]
}
```

### 6.2 デフォルト値
- location_type: "indoor"
- distance: "medium"
- expected_objects: []
- excluded_objects: []
- busy_hours: []
- quiet_hours: []

### 6.3 UI設計（CameraDetailModal）
```
[カメラコンテキスト設定]
設置場所タイプ: [entrance ▼]
撮影距離: [medium ▼]
期待オブジェクト: [✓human] [✓vehicle] [ animal]
除外オブジェクト: [ cat] [ dog]
繁忙時間帯: [08:00-10:00] [+追加]
静寂時間帯: [23:00-06:00] [+追加]
```

---

## 7. テスト計画

### 7.1 is21連携テスト

| テストID | テスト内容 | 期待結果 |
|----------|-----------|----------|
| T-001 | 人物画像送信 | primary_event="human", PAR詳細取得 |
| T-002 | 動物画像送信 | primary_event="animal" |
| T-003 | 車両画像送信 | primary_event="vehicle" |
| T-004 | 空画像送信 | primary_event="none", DB非保存 |
| T-005 | camera_context適用 | context_applied=true |
| T-006 | suspicious計算 | score, level, factors取得 |
| T-007 | prev_image送信 | frame_diff.enabled=true, person_changes取得 |
| T-008 | 滞在検知 | frame_diff.loitering.detected=true, loiterers配列取得 |
| T-009 | カメラ改竄検知 | frame_diff.camera_status.tampered=true |
| T-010 | フレーム凍結検知 | frame_diff.camera_status.frozen=true |

### 7.2 DB保存テスト

| テストID | テスト内容 | 期待結果 |
|----------|-----------|----------|
| T-101 | 検知ログ保存 | detection_logsにレコード作成 |
| T-102 | BQカラム一致確認 | 全カラムBQ互換形式 |
| T-103 | none非保存確認 | primary_event="none"時DB未登録 |
| T-104 | BQキュー登録 | bq_sync_queue登録 |
| T-105 | frame_diff保存 | frame_diff JSONカラムに差分分析結果保存 |
| T-106 | loitering_detected保存 | 滞在検知時loitering_detected=TRUE |

### 7.3 UI表示テスト

| テストID | テスト内容 | 期待結果 |
|----------|-----------|----------|
| T-201 | WebSocket通知 | event_logメッセージ受信 |
| T-202 | EventLogPane表示 | 検知ログ一覧表示 |
| T-203 | フィルタ動作 | severity/event別絞り込み |
| T-204 | ログクリック | 該当スナップショット表示 |

### 7.4 回答精度チューニングPDCA

#### Plan
1. camera_contextパラメータ調整
2. location_type/distance設定最適化
3. busy_hours/quiet_hours設定

#### Do
1. 各設定パターンで画像解析実行
2. 結果をdetection_logsに保存

#### Check
1. suspicious.score分布確認
2. false positive/negative分析
3. person_details精度確認

#### Act
1. パラメータ調整
2. camera_contextデフォルト値更新
3. is21側閾値調整リクエスト

---

## 8. 依存関係

### 8.1 内部依存
- cameras テーブル（camera_id, camera_context）
- PollingOrchestrator（スナップショット取得）
- SnapshotService（画像保存）
- RealtimeHub（WebSocket通知）

### 8.2 外部依存
- is21 Edge AI (192.168.3.240:9000)
- BigQuery（将来）
- Google Drive API（将来）

---

## 9. 設定項目

### 9.1 settings テーブル
```json
{
  "ai_event_log": {
    "is21_endpoint": "http://192.168.3.240:9000/v1/analyze",
    "timeout_ms": 30000,
    "save_none_events": false,
    "min_confidence": 0.3,
    "bq_sync_interval_sec": 60,
    "bq_sync_batch_size": 100
  }
}
```

---

## 10. エラーハンドリング

| エラー | 対応 |
|--------|------|
| is21接続タイムアウト | リトライ3回、以降スキップ |
| is21 500エラー | ログ記録、次ポーリングで再試行 |
| DB保存失敗 | リトライ3回、失敗時ログ記録 |
| BQ同期失敗 | キューに残置、次バッチで再試行 |

---

## 11. 画像保存戦略

### 11.1 ディレクトリ構造
```
/var/lib/is22/
├── snapshots/              # スナップショット画像
│   ├── {camera_id}/        # カメラ別ディレクトリ
│   │   ├── 2026-01-02/     # 日付別ディレクトリ
│   │   │   ├── 103015_detected.jpg   # HHmmss_状態.jpg
│   │   │   ├── 103015_none.jpg       # 検知なし画像（オプション）
│   │   │   └── ...
│   │   └── ...
│   └── ...
├── temp/                   # 一時ファイル（送信待ち）
│   └── pending/
└── db/                     # SQLite/MySQL関連
```

### 11.2 ファイル命名規則
```
{HHmmss}_{status}.jpg

status:
- detected: 検知あり（primary_event != "none"）
- none: 検知なし（save_none_events=true時のみ）
- error: 解析エラー

例: 103015_detected.jpg → 10:30:15に撮影、検知あり
```

### 11.3 パス生成ロジック
```rust
// snapshot_service/mod.rs
fn generate_image_path(camera_id: &str, captured_at: &DateTime<Utc>, status: &str) -> PathBuf {
    let base = PathBuf::from("/var/lib/is22/snapshots");
    let date = captured_at.format("%Y-%m-%d").to_string();
    let time = captured_at.format("%H%M%S").to_string();
    base.join(camera_id)
        .join(&date)
        .join(format!("{}_{}.jpg", time, status))
}
```

### 11.4 保持期間・ローテーション
| 画像タイプ | 保持期間 | ローテーション方式 |
|-----------|----------|-------------------|
| 検知あり画像 | 30日 | 日次バッチで古いファイル削除 |
| 検知なし画像 | 7日 | 日次バッチで削除 |
| エラー画像 | 3日 | 日次バッチで削除 |

```rust
// ローテーションジョブ（日次 03:00実行）
async fn rotate_snapshots() {
    let thresholds = [
        ("detected", 30),
        ("none", 7),
        ("error", 3),
    ];
    for (status, days) in thresholds {
        delete_older_than("/var/lib/is22/snapshots", status, days).await;
    }
}
```

### 11.5 ディスク容量監視
| 閾値 | アクション |
|------|-----------|
| 70% | 警告ログ出力 |
| 80% | 検知なし画像の保存停止 |
| 90% | 緊急ローテーション実行（保持期間半減） |

```rust
// 起動時・1時間ごとにチェック
async fn check_disk_usage() -> DiskStatus {
    let usage = get_disk_usage("/var/lib/is22").await;
    match usage.percent {
        p if p >= 90 => DiskStatus::Critical,
        p if p >= 80 => DiskStatus::Warning,
        p if p >= 70 => DiskStatus::Caution,
        _ => DiskStatus::Normal,
    }
}
```

### 11.6 クラウド連携（将来）
Google Drive連携はis22_CLOUD_INTEGRATION.mdで別途設計予定：
- 検知あり画像のみアップロード
- 日次バッチでアップロード
- アップロード完了後 `image_path_cloud` 更新

---

## 12. BQ同期詳細仕様

### 12.1 モジュール構成
```rust
// bq_sync/mod.rs
pub struct BqSyncService {
    client: BigQueryClient,
    db: Pool<MySql>,
    config: BqSyncConfig,
}

pub struct BqSyncConfig {
    project_id: String,
    dataset_id: String,
    table_id: String,
    batch_size: usize,       // デフォルト: 100
    sync_interval_sec: u64,  // デフォルト: 60
    max_retries: usize,      // デフォルト: 3
}

impl BqSyncService {
    pub async fn start_sync_loop(&self);
    pub async fn process_pending_queue(&self) -> Result<SyncResult, BqError>;
    pub async fn sync_single_record(&self, record: &DetectionLog) -> Result<(), BqError>;
}
```

### 12.2 同期フロー
```
1. DetectionLogService.save_detection()
   ↓
2. INSERT INTO detection_logs
   ↓
3. INSERT INTO bq_sync_queue (table_name, record_id, payload)
   ↓
4. BqSyncService (別スレッド) がキューを監視
   ↓
5. バッチ取得 (100件 or 60秒経過)
   ↓
6. BigQuery Streaming Insert API
   ↓
7. 成功: UPDATE bq_sync_queue SET status='success'
   　 UPDATE detection_logs SET synced_to_bq=true, synced_at=NOW()
   失敗: UPDATE bq_sync_queue SET retry_count++, last_error=...
```

### 12.3 認証設定
```rust
// サービスアカウント認証
// 環境変数: GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
// または: /var/lib/is22/credentials/bq-service-account.json

let client = BigQueryClient::from_service_account_key_file(
    "/var/lib/is22/credentials/bq-service-account.json"
).await?;
```

### 12.4 リトライ戦略
```rust
// Exponential Backoff with Jitter
async fn sync_with_retry(&self, records: &[BqSyncRecord]) -> Result<(), BqError> {
    let mut attempt = 0;
    let max_retries = 3;

    loop {
        match self.client.insert_all(records).await {
            Ok(_) => return Ok(()),
            Err(e) if attempt < max_retries => {
                let delay = Duration::from_millis(
                    (1 << attempt) * 1000 + rand::random::<u64>() % 1000
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 12.5 部分成功時の処理
```rust
// Streaming Insert はレコード単位で成功/失敗が返る
match client.insert_all(&batch).await {
    InsertResult::Success => { /* 全件成功 */ },
    InsertResult::PartialSuccess { succeeded, failed } => {
        // 成功分のみ status='success' に更新
        for id in succeeded {
            update_queue_status(id, "success").await;
        }
        // 失敗分は retry_count++
        for (id, error) in failed {
            increment_retry_count(id, &error).await;
        }
    },
    InsertResult::Failure(e) => { /* 全件失敗 */ },
}
```

### 12.6 BigQueryテーブル定義
```sql
-- BigQuery DDL
CREATE TABLE `{project_id}.{dataset_id}.detection_logs` (
    log_id INT64 NOT NULL,
    tid STRING NOT NULL,
    fid STRING NOT NULL,
    camera_id STRING NOT NULL,
    lacis_id STRING,
    captured_at TIMESTAMP NOT NULL,
    analyzed_at TIMESTAMP NOT NULL,
    primary_event STRING NOT NULL,
    severity INT64 NOT NULL,
    confidence FLOAT64 NOT NULL,
    count_hint INT64 NOT NULL,
    unknown_flag BOOL NOT NULL,
    tags JSON,
    person_details JSON,
    bboxes JSON,
    suspicious JSON,
    frame_diff JSON,                   -- 差分分析結果
    loitering_detected BOOL NOT NULL,  -- 滞在検知フラグ
    preset_id STRING NOT NULL,
    preset_version STRING,
    output_schema STRING,
    context_applied BOOL NOT NULL,
    camera_context JSON,
    is21_log JSON NOT NULL,
    camera_response JSON,
    image_path_local STRING NOT NULL,
    image_path_cloud STRING,
    processing_ms INT64,
    schema_version STRING NOT NULL,
    created_at TIMESTAMP NOT NULL,
    synced_at TIMESTAMP
)
PARTITION BY DATE(captured_at)
CLUSTER BY tid, fid, camera_id;
```

---

## 13. フロントエンド詳細設計

### 13.1 EventLogPane コンポーネント仕様

#### Props定義
```typescript
interface EventLogPaneProps {
  maxItems?: number;        // 表示最大件数（デフォルト: 100）
  autoScroll?: boolean;     // 新着時自動スクロール（デフォルト: true）
  onLogClick?: (log: DetectionLog) => void;  // ログクリックハンドラ
}
```

#### State定義
```typescript
interface EventLogPaneState {
  logs: DetectionLog[];
  isLoading: boolean;
  error: string | null;
  filter: EventLogFilter;
  wsStatus: 'connected' | 'connecting' | 'disconnected' | 'error';
}

interface EventLogFilter {
  primaryEvent: string[];   // ['human', 'vehicle', 'animal']
  minSeverity: number;      // 0-3
  suspiciousLevel: string[];// ['normal', 'low', 'medium', 'high', 'critical']
  timeRange: {
    start: Date | null;
    end: Date | null;
  };
  cameraIds: string[];      // 特定カメラのみ
}
```

### 13.2 EventLogItem コンポーネント
```typescript
interface EventLogItemProps {
  log: DetectionLog;
  isExpanded: boolean;
  onClick: () => void;
  onCameraClick: (cameraId: string) => void;
}

// 表示内容
// [10:30:15] 📹カメラ名 🧍人物検知 [severity badge]
// tags抜粋... [suspicious badge]
```

### 13.3 EventLogFilter コンポーネント
```typescript
interface EventLogFilterProps {
  filter: EventLogFilter;
  onChange: (filter: EventLogFilter) => void;
}

// UI構成
// [primary_event: ✓human ✓vehicle □animal]
// [severity: ○0 ●1 ●2 ●3]
// [suspicious: ✓normal ✓low ✓medium ✓high □critical]
// [時間: 過去1時間 ▼] [カメラ: 全て ▼]
```

### 13.4 EventLogDetailModal コンポーネント
```typescript
interface EventLogDetailModalProps {
  log: DetectionLog;
  isOpen: boolean;
  onClose: () => void;
}

// 表示内容
// - スナップショット画像（bbox overlay）
// - 全タグ一覧
// - person_details詳細
// - suspicious詳細（factors展開）
// - is21_log生データ表示（デバッグ用）
```

### 13.5 BboxOverlay コンポーネント
```typescript
interface BboxOverlayProps {
  imageUrl: string;
  bboxes: BBox[];
  showLabels?: boolean;
  highlightIndex?: number;  // 特定bboxをハイライト
}

// 画像上にbboxを描画
// ラベル + confidence表示
```

### 13.6 Zustand Store設計
```typescript
// stores/eventLogStore.ts
interface EventLogStore {
  // State
  logs: DetectionLog[];
  filter: EventLogFilter;
  wsStatus: WsStatus;
  isLoading: boolean;

  // Actions
  addLog: (log: DetectionLog) => void;
  setFilter: (filter: Partial<EventLogFilter>) => void;
  clearLogs: () => void;
  fetchInitialLogs: () => Promise<void>;

  // Computed
  filteredLogs: () => DetectionLog[];
}

const useEventLogStore = create<EventLogStore>((set, get) => ({
  logs: [],
  filter: defaultFilter,
  wsStatus: 'disconnected',
  isLoading: false,

  addLog: (log) => set((state) => ({
    logs: [log, ...state.logs].slice(0, 1000)  // 最新1000件保持
  })),

  setFilter: (partial) => set((state) => ({
    filter: { ...state.filter, ...partial }
  })),

  filteredLogs: () => {
    const { logs, filter } = get();
    return logs.filter(log => matchFilter(log, filter));
  },
}));
```

### 13.7 WebSocket接続管理
```typescript
// hooks/useEventLogWebSocket.ts
function useEventLogWebSocket() {
  const [status, setStatus] = useState<WsStatus>('disconnected');
  const addLog = useEventLogStore((s) => s.addLog);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    const connect = () => {
      setStatus('connecting');
      const ws = new WebSocket(`ws://${API_HOST}:8080/ws`);

      ws.onopen = () => setStatus('connected');
      ws.onclose = () => {
        setStatus('disconnected');
        // 自動再接続（5秒後）
        setTimeout(connect, 5000);
      };
      ws.onerror = () => setStatus('error');
      ws.onmessage = (event) => {
        const msg = JSON.parse(event.data);
        if (msg.type === 'event_log') {
          addLog(msg.payload);
        }
      };

      wsRef.current = ws;
    };

    connect();
    return () => wsRef.current?.close();
  }, []);

  return { status };
}
```

### 13.8 エラー表示・リカバリ
```typescript
// components/EventLogConnectionStatus.tsx
function EventLogConnectionStatus({ status }: { status: WsStatus }) {
  switch (status) {
    case 'connected':
      return <Badge color="green">🟢 リアルタイム接続中</Badge>;
    case 'connecting':
      return <Badge color="yellow">🟡 接続中...</Badge>;
    case 'disconnected':
      return (
        <Badge color="red">
          🔴 切断 - 5秒後に再接続
          <Button size="xs" onClick={reconnect}>今すぐ再接続</Button>
        </Badge>
      );
    case 'error':
      return <Badge color="red">❌ 接続エラー</Badge>;
  }
}
```

---

## 14. 次ターンタスク（参考）

### mobes2.0連携
- AIチャットパネル実装
- BQクエリ生成
- サマリー取得API

### 動画再生
- go2rtx WebRTC統合
- モーダル再生
- サジェスト再生
