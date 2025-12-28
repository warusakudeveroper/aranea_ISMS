# 詳細設計書

文書番号: DETAIL_01
作成日: 2025-12-29
ステータス: Draft

---

## Phase 0: 基盤構築

### IS21-001: RKNNランタイム環境構築

#### ディレクトリ構成
```
/opt/is21/
├── .venv/                    # Python仮想環境
├── models/                   # RKNNモデル
│   └── yolov8n_640.rknn
├── logs/                     # ログ出力
├── config/                   # 設定ファイル
│   └── schema.json          # タグスキーマ（キャッシュ）
└── src/                      # ソースコード
    ├── main.py              # FastAPIエントリポイント
    ├── inference.py         # 推論エンジン
    ├── schema.py            # スキーマ管理
    └── quality.py           # 品質判定
```

#### 環境構築手順
```bash
# 1. Python仮想環境
python3.10 -m venv /opt/is21/.venv
source /opt/is21/.venv/bin/activate

# 2. 依存パッケージ
pip install \
  rknn-toolkit2-lite==2.0.0 \
  fastapi==0.109.0 \
  uvicorn[standard]==0.27.0 \
  python-multipart==0.0.9 \
  opencv-python-headless==4.9.0.80 \
  numpy==1.26.3 \
  pydantic==2.5.3

# 3. NPU権限
sudo usermod -aG render $USER
```

#### RKNNランタイム初期化
```python
# inference.py
from rknnlite.api import RKNNLite

class InferenceEngine:
    def __init__(self, model_path: str):
        self.rknn = RKNNLite()
        ret = self.rknn.load_rknn(model_path)
        if ret != 0:
            raise RuntimeError(f"Failed to load RKNN model: {ret}")

        ret = self.rknn.init_runtime(core_mask=RKNNLite.NPU_CORE_0)
        if ret != 0:
            raise RuntimeError(f"Failed to init runtime: {ret}")

    def infer(self, img: np.ndarray) -> list:
        outputs = self.rknn.inference(inputs=[img])
        return outputs
```

---

### IS21-002: 推論モデル選定・RKNN変換

#### モデル選定基準
| モデル | 入力サイズ | 推論時間 | mAP | 採用 |
|--------|----------|---------|-----|------|
| YOLOv8n | 640x640 | ~200ms | 0.52 | ○ |
| YOLOv5s | 640x640 | ~180ms | 0.50 | △ |
| YOLOv8n | 960x960 | ~450ms | 0.55 | × |

#### RKNN変換スクリプト
```python
# convert_model.py
from rknn.api import RKNN

rknn = RKNN()

# ONNX読み込み
rknn.config(mean_values=[[0, 0, 0]], std_values=[[255, 255, 255]])
rknn.load_onnx(model='yolov8n.onnx')

# 量子化（INT8）
rknn.build(do_quantization=True, dataset='dataset.txt')

# エクスポート
rknn.export_rknn('yolov8n_640.rknn')
```

#### 検出クラスマッピング
```python
COCO_CLASSES = {
    0: "person",      # → human
    16: "dog",        # → animal
    17: "cat",        # → animal
    18: "horse",      # → animal
    19: "sheep",      # → animal
    20: "cow",        # → animal
    21: "elephant",   # → animal
    22: "bear",       # → animal
    23: "zebra",      # → animal
    2: "car",         # → vehicle
    5: "bus",         # → vehicle
    7: "truck",       # → vehicle
}
```

---

### IS22-001: Rustワークスペース構成

#### Cargo.toml（ワークスペースルート）
```toml
[workspace]
resolver = "2"
members = [
    "cam_common",
    "cam_collector",
    "cam_dispatcher",
    "cam_web",
    "cam_streamer",
]

[workspace.dependencies]
tokio = { version = "1.35", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio", "mysql", "chrono", "uuid"] }
reqwest = { version = "0.11", features = ["json", "multipart"] }
axum = { version = "0.7", features = ["ws"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1.0"
thiserror = "1.0"
```

#### cam_common/src/lib.rs
```rust
pub mod db;
pub mod models;
pub mod config;
pub mod error;

pub use db::DbPool;
pub use config::Config;
pub use error::{Error, Result};
```

#### ディレクトリ構成
```
/opt/camserver/
├── Cargo.toml
├── cam_common/
│   └── src/
│       ├── lib.rs
│       ├── db.rs           # DB接続プール
│       ├── models.rs       # 共通モデル
│       ├── config.rs       # 設定読み込み
│       └── error.rs        # エラー型
├── cam_collector/
│   └── src/
│       ├── main.rs
│       ├── rtsp.rs         # RTSP取得
│       ├── diff.rs         # 差分判定
│       └── worker.rs       # ワーカー
├── cam_dispatcher/
│   └── src/
│       ├── main.rs
│       ├── job.rs          # ジョブキュー
│       ├── inference.rs    # is21通信
│       ├── event.rs        # イベント化
│       ├── drive.rs        # Drive保存
│       └── notify.rs       # 通知
├── cam_web/
│   └── src/
│       ├── main.rs
│       ├── routes/
│       │   ├── mod.rs
│       │   ├── cameras.rs
│       │   ├── events.rs
│       │   └── media.rs
│       └── sse.rs          # SSE配信
└── cam_streamer/
    └── src/
        ├── main.rs
        ├── hls.rs          # HLS変換
        └── session.rs      # セッション管理
```

---

### IS22-002: MariaDB DDL適用

#### 初期化スクリプト
```bash
#!/bin/bash
# /opt/camserver/scripts/init_db.sh

mysql -u root -p << 'EOF'
CREATE DATABASE IF NOT EXISTS camserver
  CHARACTER SET utf8mb4
  COLLATE utf8mb4_unicode_ci;

CREATE USER IF NOT EXISTS 'camserver'@'localhost'
  IDENTIFIED BY 'your_password_here';

GRANT ALL PRIVILEGES ON camserver.* TO 'camserver'@'localhost';
FLUSH PRIVILEGES;
EOF

mysql -u camserver -p camserver < /opt/camserver/migrations/001_initial.sql
```

#### マイグレーション管理
```
/opt/camserver/migrations/
├── 001_initial.sql           # 特記仕様1のDDL
├── 002_inference_jobs.sql    # 推論ジョブキュー
└── 003_indexes.sql           # 追加インデックス
```

---

### IS22-003: collector RTSP取得

#### ffmpegコマンド設計
```rust
// rtsp.rs
use tokio::process::Command;

pub async fn capture_frame(
    rtsp_url: &str,
    output_path: &str,
    timeout_sec: u32,
) -> Result<()> {
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-rtsp_transport", "tcp",
            "-i", rtsp_url,
            "-frames:v", "1",
            "-q:v", "2",
            "-f", "image2",
            output_path,
        ])
        .env("FFREPORT", "")
        .kill_on_drop(true)
        .status()
        .await?;

    if !status.success() {
        return Err(Error::RtspCapture(status.code()));
    }
    Ok(())
}
```

#### タイムアウト設計
| 接続種別 | 接続タイムアウト | 読み取りタイムアウト |
|---------|----------------|-------------------|
| ローカル | 3秒 | 5秒 |
| VPN | 5秒 | 10秒 |

---

## Phase 1: コア機能

### IS21-003: FastAPI推論サーバ

#### main.py
```python
from fastapi import FastAPI, File, UploadFile, HTTPException
from fastapi.responses import JSONResponse
import uvicorn

from inference import InferenceEngine
from schema import SchemaManager
from quality import assess_quality

app = FastAPI(title="is21 Inference Server")
engine: InferenceEngine = None
schema_mgr: SchemaManager = None

@app.on_event("startup")
async def startup():
    global engine, schema_mgr
    engine = InferenceEngine("/opt/is21/models/yolov8n_640.rknn")
    schema_mgr = SchemaManager("/opt/is21/config/schema.json")

@app.get("/healthz")
async def healthz():
    return {"status": "ok"}

@app.post("/v1/analyze")
async def analyze(
    image: UploadFile = File(...),
    camera_id: str = None,
):
    if schema_mgr.schema is None:
        raise HTTPException(409, "Schema not loaded")

    # 画像読み込み
    contents = await image.read()
    img = cv2.imdecode(np.frombuffer(contents, np.uint8), cv2.IMREAD_COLOR)

    # 品質判定
    quality_tags = assess_quality(img)

    # 推論実行
    detections = engine.infer(img)

    # タグ生成
    tags = generate_tags(detections, schema_mgr.schema)
    tags.extend(quality_tags)

    # 結果構築
    result = build_response(detections, tags, schema_mgr.schema)
    return JSONResponse(result)

@app.put("/v1/schema")
async def update_schema(schema: dict):
    schema_mgr.update(schema)
    return {"ok": True}
```

---

### IS22-004: 差分判定アルゴリズム

#### diff.rs
```rust
use image::{DynamicImage, GrayImage};

pub struct DiffResult {
    pub diff_ratio: f64,
    pub luma_delta: i16,
    pub should_analyze: bool,
}

pub fn compute_diff(
    current: &DynamicImage,
    previous: &DynamicImage,
    threshold: f64,
) -> DiffResult {
    let curr_gray = current.to_luma8();
    let prev_gray = previous.to_luma8();

    // diff_ratio: ピクセル差分の割合
    let diff_pixels = count_diff_pixels(&curr_gray, &prev_gray, 30);
    let total_pixels = (curr_gray.width() * curr_gray.height()) as f64;
    let diff_ratio = diff_pixels as f64 / total_pixels;

    // luma_delta: 平均輝度差
    let curr_luma = mean_luma(&curr_gray);
    let prev_luma = mean_luma(&prev_gray);
    let luma_delta = (curr_luma as i16) - (prev_luma as i16);

    DiffResult {
        diff_ratio,
        luma_delta,
        should_analyze: diff_ratio >= threshold || luma_delta.abs() > 20,
    }
}

fn count_diff_pixels(a: &GrayImage, b: &GrayImage, threshold: u8) -> u32 {
    a.pixels()
        .zip(b.pixels())
        .filter(|(pa, pb)| {
            (pa.0[0] as i16 - pb.0[0] as i16).abs() > threshold as i16
        })
        .count() as u32
}
```

---

### IS22-006: ジョブキュー

#### job.rs
```rust
use sqlx::MySqlPool;
use uuid::Uuid;

pub struct JobManager {
    pool: MySqlPool,
    worker_id: String,
}

impl JobManager {
    /// ジョブを取得（排他制御）
    pub async fn claim(&self) -> Result<Option<Job>> {
        let token = Uuid::new_v4().to_string();

        // UPDATE ... LIMIT 1 で排他取得
        let result = sqlx::query!(
            r#"
            UPDATE inference_jobs
            SET status = 'running',
                locked_by = ?,
                locked_token = ?,
                locked_at = NOW(3)
            WHERE status = 'queued'
              AND available_at <= NOW(3)
            ORDER BY priority DESC, job_id ASC
            LIMIT 1
            "#,
            self.worker_id,
            token,
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        // 取得したジョブを読み込み
        let job = sqlx::query_as!(
            Job,
            r#"
            SELECT * FROM inference_jobs
            WHERE locked_token = ?
            "#,
            token,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(job))
    }

    /// ジョブ完了
    pub async fn done(&self, job_id: i64, token: &str) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE inference_jobs
            SET status = 'done'
            WHERE job_id = ? AND locked_token = ?
            "#,
            job_id,
            token,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// ジョブ再キュー
    pub async fn requeue(&self, job_id: i64, delay_sec: u32) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE inference_jobs
            SET status = 'queued',
                locked_by = NULL,
                locked_token = NULL,
                locked_at = NULL,
                attempt = attempt + 1,
                available_at = DATE_ADD(NOW(3), INTERVAL ? SECOND)
            WHERE job_id = ?
            "#,
            delay_sec,
            job_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
```

---

### IS22-008: イベント化ロジック

#### event.rs
```rust
use chrono::{DateTime, Utc, Duration};

pub struct EventManager {
    pool: MySqlPool,
    merge_window: Duration,
    close_timeout: Duration,
}

impl EventManager {
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            pool,
            merge_window: Duration::seconds(60),
            close_timeout: Duration::seconds(120),
        }
    }

    /// フレーム処理後のイベント更新
    pub async fn process_frame(&self, frame: &AnalyzedFrame) -> Result<EventAction> {
        if !frame.detected {
            // 非検知フレームではクローズ処理のみ
            self.close_stale_events(&frame.camera_id).await?;
            return Ok(EventAction::None);
        }

        // オープン中のイベント検索
        let open_event = self.find_open_event(
            &frame.camera_id,
            &frame.primary_event,
        ).await?;

        match open_event {
            Some(event) if self.can_merge(&event, frame) => {
                // 既存イベントにマージ
                self.merge_frame(&event, frame).await?;
                Ok(EventAction::Merged(event.event_id))
            }
            _ => {
                // 新規イベント作成
                let event_id = self.create_event(frame).await?;
                Ok(EventAction::Created(event_id))
            }
        }
    }

    fn can_merge(&self, event: &Event, frame: &AnalyzedFrame) -> bool {
        let gap = frame.captured_at - event.last_seen_at;
        gap <= self.merge_window
            && event.primary_event == frame.primary_event
    }

    async fn close_stale_events(&self, camera_id: &str) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE events
            SET state = 'closed', end_at = last_seen_at
            WHERE camera_id = ?
              AND state = 'open'
              AND last_seen_at < DATE_SUB(NOW(3), INTERVAL ? SECOND)
            "#,
            camera_id,
            self.close_timeout.num_seconds(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
```

---

### IS22-013: Axum APIサーバ

#### routes/cameras.rs
```rust
use axum::{
    extract::{Path, Query, State},
    Json,
};

#[derive(Deserialize)]
pub struct CameraListQuery {
    pub enabled: Option<bool>,
}

pub async fn list_cameras(
    State(pool): State<MySqlPool>,
    Query(query): Query<CameraListQuery>,
) -> Result<Json<Vec<Camera>>> {
    let cameras = if let Some(enabled) = query.enabled {
        sqlx::query_as!(
            Camera,
            "SELECT * FROM cameras WHERE enabled = ?",
            enabled,
        )
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query_as!(Camera, "SELECT * FROM cameras")
            .fetch_all(&pool)
            .await?
    };

    Ok(Json(cameras))
}

pub async fn get_camera(
    State(pool): State<MySqlPool>,
    Path(camera_id): Path<String>,
) -> Result<Json<CameraDetail>> {
    let camera = sqlx::query_as!(
        Camera,
        "SELECT * FROM cameras WHERE camera_id = ?",
        camera_id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;

    let latest_frame = sqlx::query_as!(
        FrameSummary,
        r#"
        SELECT frame_uuid, captured_at, detected, primary_event, severity
        FROM frames
        WHERE camera_id = ?
        ORDER BY captured_at DESC
        LIMIT 1
        "#,
        camera_id,
    )
    .fetch_optional(&pool)
    .await?;

    Ok(Json(CameraDetail {
        camera,
        latest_frame,
    }))
}
```

---

## Phase 2: 付加機能

### IS22-009: Drive保存

#### drive.rs
```rust
use google_drive3::{DriveHub, api::File};

pub struct DriveUploader {
    hub: DriveHub<HttpsConnector<HttpConnector>>,
    archive_folder: String,
    quarantine_folder: String,
}

impl DriveUploader {
    pub async fn upload(
        &self,
        camera_id: &str,
        captured_at: DateTime<Utc>,
        retention: RetentionClass,
        jpeg: Vec<u8>,
    ) -> Result<String> {
        let folder_id = match retention {
            RetentionClass::Normal => &self.archive_folder,
            RetentionClass::Quarantine => &self.quarantine_folder,
            RetentionClass::Case => return Err(Error::InvalidRetention),
        };

        // 日付フォルダ確保
        let date_folder = self.ensure_date_folder(folder_id, captured_at).await?;
        let camera_folder = self.ensure_camera_folder(&date_folder, camera_id).await?;

        // ファイル名
        let file_name = format!(
            "{}_{}.jpg",
            captured_at.format("%Y%m%d_%H%M%S"),
            Uuid::new_v4()
        );

        // アップロード
        let file = File {
            name: Some(file_name),
            parents: Some(vec![camera_folder]),
            ..Default::default()
        };

        let result = self.hub
            .files()
            .create(file)
            .upload(
                std::io::Cursor::new(jpeg),
                "image/jpeg".parse().unwrap(),
            )
            .await?;

        Ok(result.1.id.unwrap())
    }
}
```

---

### IS22-014: 署名付きMedia Proxy

#### media.rs
```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn generate_signed_url(
    frame_uuid: &str,
    secret: &[u8],
    expire_minutes: i64,
) -> String {
    let exp = Utc::now() + Duration::minutes(expire_minutes);
    let exp_ts = exp.timestamp();

    let data = format!("{}:{}", frame_uuid, exp_ts);
    let mut mac = HmacSha256::new_from_slice(secret).unwrap();
    mac.update(data.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());

    format!(
        "/media/frame/{}?exp={}&sig={}",
        frame_uuid, exp_ts, sig
    )
}

pub fn verify_signature(
    frame_uuid: &str,
    exp: i64,
    sig: &str,
    secret: &[u8],
) -> bool {
    // 期限チェック
    if Utc::now().timestamp() > exp {
        return false;
    }

    // 署名検証
    let data = format!("{}:{}", frame_uuid, exp);
    let mut mac = HmacSha256::new_from_slice(secret).unwrap();
    mac.update(data.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    sig == expected
}
```

---

### IS22-016: HLSストリーミング

#### hls.rs
```rust
use tokio::process::Command;

pub struct HlsSession {
    pub session_id: String,
    pub camera_id: String,
    pub rtsp_url: String,
    pub output_dir: PathBuf,
    pub started_at: DateTime<Utc>,
    pub process: Option<Child>,
}

impl HlsSession {
    pub async fn start(&mut self) -> Result<()> {
        let m3u8_path = self.output_dir.join("stream.m3u8");

        let child = Command::new("ffmpeg")
            .args([
                "-rtsp_transport", "tcp",
                "-i", &self.rtsp_url,
                "-c:v", "copy",
                "-c:a", "aac",
                "-f", "hls",
                "-hls_time", "2",
                "-hls_list_size", "5",
                "-hls_flags", "delete_segments",
                m3u8_path.to_str().unwrap(),
            ])
            .kill_on_drop(true)
            .spawn()?;

        self.process = Some(child);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut process) = self.process.take() {
            process.kill().await?;
        }

        // ディレクトリクリーンアップ
        tokio::fs::remove_dir_all(&self.output_dir).await?;

        Ok(())
    }
}
```

---

## 設定ファイル

### /opt/camserver/.env
```bash
# Database
DATABASE_URL=mysql://camserver:password@localhost/camserver

# is21 Inference Server
IS21_BASE_URL=http://192.168.1.100:8000

# Google Drive
DRIVE_SERVICE_ACCOUNT_FILE=/opt/camserver/secrets/drive-sa.json
DRIVE_ARCHIVE_FOLDER_ID=1ABCdefGHI...
DRIVE_QUARANTINE_FOLDER_ID=1XYZabc...

# Notification
WEBHOOK_URL=https://hooks.slack.com/services/...
WEBHOOK_SECRET=your-hmac-secret

# Media
MEDIA_SECRET=your-media-signing-secret
MEDIA_EXPIRE_MINUTES=30

# HLS
HLS_OUTPUT_DIR=/tmp/camserver/hls
HLS_SESSION_TTL_MINUTES=10
```

### /opt/is21/.env
```bash
# Server
HOST=0.0.0.0
PORT=8000
WORKERS=1

# Model
MODEL_PATH=/opt/is21/models/yolov8n_640.rknn
CONFIDENCE_THRESHOLD=0.25
NMS_THRESHOLD=0.45

# Schema
SCHEMA_PATH=/opt/is21/config/schema.json
```
