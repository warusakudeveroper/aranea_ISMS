
# 0. 全体アーキテクチャ（4サービス分割）

## 0-1. collector（収集）

* DBから camera 一覧を読み、周期でスナップ取得（RTSP→1枚）
* infer画像を正規化（例：横640、差分用は横320）
* **差分ゲート**で no_event 判定（推論スキップ）
* `frames` を INSERT（何もなしも含む）
* 推論が必要な場合のみ `inference_jobs` に enqueue

## 0-2. dispatcher（推論・イベント化・通知・Drive）

* `inference_jobs` をポーリングして claim（排他）
* infer画像を読み is21 に POST → 結果取得
* `frames` UPDATE（analyzed, detected, tags, severity, …）
* `frame_tags` INSERT
* `events` open/merge/close 更新、`event_tags` INSERT IGNORE
* Drive保存（full画像）→ `frames.drive_file_id` UPDATE、`drive_objects` INSERT
* 通知（観測事実のみ）・LLMエスカレーション（unknownのみ、クールダウン）

## 0-3. web（UI/API + Media Proxy）

* shadcn/uiのフロントが叩くAPIを提供（cameras、latest、events search…）
* `is22がプロキシURLを発行`（署名付き短寿命URL）
* `/media/frame/{uuid}` で Driveから画像をstream返却（またはローカルキャッシュ）

## 0-4. streamer（RTSP→HLS/WebRTC）

* ブラウザでRTSP直再生できないため、**モーダルを開いたときだけ**ストリーム生成
* “ほぼ実装”テンプレとしては **ffmpegでHLS生成**が現実的（WebRTCは別途go2rtc等が最適）
* `POST /v1/streams/start` → session作成→ HLS URL返却
* TTLで自動停止

---

# 1. Cargo Workspace（ほぼ実装テンプレの構成）

```
camserver/
  Cargo.toml            (workspace)
  crates/
    cam_common/         (共通: config/db/models/is21 client/crypto/image utils)
    cam_collector/      (bin)
    cam_dispatcher/     (bin)
    cam_web/            (bin: axum)
    cam_streamer/       (bin: axum + ffmpeg HLS)
  migrations/
    001_base.sql        (あなたのDDL)
    002_inference_jobs.sql
    003_events_last_seen.sql
```

---

# 2. DB拡張（work queue + events last_seen）

既存DDLに **推論ジョブ用テーブル**を足すのが、最小の「壊れない」実装になります。

## 2-1. migrations/002_inference_jobs.sql

```sql
USE camserver;

CREATE TABLE IF NOT EXISTS inference_jobs (
  job_id        BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  frame_id      BIGINT UNSIGNED NOT NULL,

  status        ENUM('queued','running','done','dead') NOT NULL DEFAULT 'queued',
  priority      TINYINT UNSIGNED NOT NULL DEFAULT 100,    -- 0..255
  attempt       TINYINT UNSIGNED NOT NULL DEFAULT 0,
  max_attempt   TINYINT UNSIGNED NOT NULL DEFAULT 5,

  available_at  DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

  locked_by     VARCHAR(128) NULL,
  locked_token  CHAR(36) NULL,
  locked_at     DATETIME(3) NULL,

  last_error    VARCHAR(255) NULL,
  finished_at   DATETIME(3) NULL,

  created_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  PRIMARY KEY(job_id),
  UNIQUE KEY uq_inference_jobs_frame (frame_id),
  KEY idx_inference_jobs_status_avail (status, available_at),
  KEY idx_inference_jobs_locked (locked_token),

  CONSTRAINT fk_inference_jobs_frame
    FOREIGN KEY (frame_id) REFERENCES frames(frame_id)
    ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
```

## 2-2. migrations/003_events_last_seen.sql

イベントclose判定を安定させるため、`events.last_seen_at` を追加します。

```sql
USE camserver;

ALTER TABLE events
  ADD COLUMN last_seen_at DATETIME(3) NOT NULL AFTER start_at;

-- 既存データがある場合は start_at を流し込み（必要なら）
UPDATE events SET last_seen_at = start_at WHERE last_seen_at IS NULL;
```

---

# 3. cam_common（共通クレート）要点

## 3-1. 共通依存（例）

`cam_common/Cargo.toml`（要点だけ）

```toml
[dependencies]
anyhow = "1"
serde = { version="1", features=["derive"] }
serde_json = "1"
tracing = "0.1"
sqlx = { version = "0.8", features = ["runtime-tokio", "mysql", "chrono", "json"] }
reqwest = { version="0.12", features=["json","multipart","rustls-tls"] }
tokio = { version="1", features=["full"] }
uuid = { version="1", features=["v4"] }
chrono = { version="0.4", features=["serde"] }
hmac = "0.12"
sha2 = "0.10"
base64 = "0.22"
image = "0.25"   # resize/diff（最初はこれで十分）
xxhash-rust = { version = "0.8", features=["xxh3"] }
```

---

# 4. collector（ほぼ実装テンプレ）

## 4-1. 処理フロー（実装の骨）

1. DBから enabled cameras + stream URLを読む
2. RTSPから1枚取得（ffmpeg image2pipe）
3. fullを `spool/full/` に保存
4. inferを生成して `spool/infer/` 保存、差分用 `spool/diff/` 生成
5. diff判定で `no_event` or `enqueue`
6. `frames INSERT`（必ず）
7. `inference_jobs INSERT`（推論が必要な場合のみ）

## 4-2. 重要SQL（INSERT：frames、enqueue：jobs）

### frames（初回INSERT）

> ここは **collectorが1回だけINSERT**して、dispatcherが後でUPDATEします。

```sql
INSERT INTO frames (
  frame_uuid, camera_id, captured_at,
  collector_status, error_code, error_message,
  diff_ratio, luma_delta, blur_score,
  analyzed, detected, primary_event, unknown_flag, severity,
  full_w, full_h, infer_w, infer_h,
  letterbox, letterbox_top, letterbox_left, letterbox_bottom, letterbox_right,
  infer_hash64_hex
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, 'none', 0, 0, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
```

### inference_jobs（enqueue）

```sql
INSERT INTO inference_jobs(frame_id, status, priority, available_at)
VALUES (?, 'queued', ?, NOW(3));
```

（`uq_inference_jobs_frame` があるので二重enqueueしません）

## 4-3. collector擬似実装（要点コード）

> ここでは “動く形に近い” ことを優先し、RTSP取得は `ffmpeg` spawn にしています（現場運用で強い）。

```rust
// cam_collector/src/main.rs
use anyhow::Result;
use cam_common::{db, config::CollectorConfig, image_utils, rtsp};
use chrono::{Utc, DateTime};
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    cam_common::telemetry::init_tracing();

    let cfg = CollectorConfig::from_env()?;
    let pool = db::connect(&cfg.database_url).await?;

    info!("collector start");

    loop {
        let cameras = db::list_enabled_cameras_with_primary_stream(&pool).await?;
        for cam in cameras {
            let cfg2 = cfg.clone();
            let pool2 = pool.clone();
            tokio::spawn(async move {
                if let Err(e) = run_one_camera_tick(&cfg2, &pool2, &cam).await {
                    warn!(camera_id=%cam.camera_id, err=?e, "camera tick failed");
                }
            });
        }
        tokio::time::sleep(std::time::Duration::from_secs(cfg.tick_sec)).await;
    }
}

async fn run_one_camera_tick(cfg: &CollectorConfig, pool: &sqlx::MySqlPool, cam: &db::CameraStreamRow) -> Result<()> {
    let captured_at = Utc::now();
    let frame_uuid = Uuid::new_v4();

    // 1) RTSPスナップ取得（ffmpegで1枚）
    let snap = match rtsp::grab_jpeg(&cam.url, cfg.rtsp_timeout_sec).await {
        Ok(bytes) => bytes,
        Err(e) => {
            // collector_status=timeout/error で frames insert（記録のため）
            let _ = db::insert_frame_error(pool, frame_uuid, &cam.camera_id, captured_at, &e).await;
            return Ok(());
        }
    };

    // 2) フル保存
    let full_path = cfg.spool_full_path(frame_uuid);
    tokio::fs::write(&full_path, &snap).await?;

    // 3) infer生成（横640 or override）
    let infer_w = db::get_infer_width_override(pool, &cam.camera_id).await?.unwrap_or(cfg.default_infer_width);
    let infer = image_utils::resize_to_width(&snap, infer_w)?;
    let infer_path = cfg.spool_infer_path(frame_uuid);
    tokio::fs::write(&infer_path, &infer).await?;

    // 4) diff用画像生成（横320）
    let diff_w = db::get_diff_width_override(pool, &cam.camera_id).await?.unwrap_or(cfg.default_diff_width);
    let diff_img = image_utils::to_gray_and_resize_to_width(&snap, diff_w)?;
    let prev_diff_path = cfg.spool_prev_diff_path(&cam.camera_id);
    let diff_metrics = image_utils::diff_with_prev(&diff_img, &prev_diff_path).await?;
    // prev_diffを更新
    tokio::fs::write(&prev_diff_path, &diff_img).await?;

    // 5) no_event判定
    let force_every_n = db::get_force_infer_override(pool, &cam.camera_id).await?.unwrap_or(cfg.force_infer_every_n);
    let should_force = db::should_force_infer(pool, &cam.camera_id, force_every_n).await?;
    let no_event = !should_force
        && diff_metrics.diff_ratio < cfg.diff_ratio_no_event
        && diff_metrics.luma_delta.abs() < cfg.luma_delta_no_event;

    // 6) frames insert
    let infer_hash = image_utils::xxh3_64_hex(&infer);
    let frame_id = db::insert_frame_initial(
        pool,
        frame_uuid,
        &cam.camera_id,
        captured_at,
        diff_metrics,
        &infer,
        infer_hash,
        &snap
    ).await?;

    // 7) no_eventならここで終了（何もなしを保存する方針）
    if no_event {
        // 任意: reason.diff_small を frame_tags に入れるならここでINSERT
        db::insert_frame_tag(pool, frame_id, "reason.diff_small", "investigation_only").await?;
        return Ok(());
    }

    // 8) 推論job enqueue
    db::enqueue_inference_job(pool, frame_id, cfg.default_job_priority).await?;

    Ok(())
}
```

---

# 5. dispatcher（ほぼ実装テンプレ）

## 5-1. dispatcherの中核は「job claim → analyze → DB更新 → eventize → Drive/Notify」

jobの排他取得（claim）をDBでやるのが重要です。

## 5-2. 重要SQL（claim / update / tag insert / event open-merge / close）

### (1) ジョブclaim（1回のUPDATEで確保）

```sql
UPDATE inference_jobs
SET status='running',
    locked_by=?,
    locked_token=?,
    locked_at=NOW(3),
    attempt=attempt+1
WHERE status='queued'
  AND available_at <= NOW(3)
ORDER BY priority DESC, job_id ASC
LIMIT 1;
```

次に token でジョブ取得：

```sql
SELECT j.job_id, j.frame_id, f.frame_uuid, f.camera_id, f.captured_at
FROM inference_jobs j
JOIN frames f ON f.frame_id = j.frame_id
WHERE j.locked_token = ?
LIMIT 1;
```

### (2) 推論結果で frames 更新

```sql
UPDATE frames
SET analyzed=1,
    detected=?,
    primary_event=?,
    unknown_flag=?,
    severity=?,
    confidence=?,
    count_hint=?,
    result_json=?,
    tags_json=?,
    retention_class=?
WHERE frame_id=?;
```

### (3) frame_tags 反映（シンプル版：delete→insert）

```sql
DELETE FROM frame_tags WHERE frame_id=?;
```

```sql
INSERT INTO frame_tags(frame_id, tag_id, tag_group) VALUES (?, ?, ?);
```

### (4) open event取得

```sql
SELECT event_id, primary_event, start_at, last_seen_at, retention_class, best_frame_id, severity_max, confidence_max
FROM events
WHERE camera_id=? AND state='open'
ORDER BY start_at DESC
LIMIT 1;
```

### (5) event 更新（merge）

```sql
UPDATE events
SET last_seen_at=?,
    severity_max=GREATEST(severity_max, ?),
    confidence_max=GREATEST(COALESCE(confidence_max, 0), ?),
    retention_class=?,
    updated_at=NOW()
WHERE event_id=?;
```

### (6) event 新規作成（open）

```sql
INSERT INTO events(
  event_uuid, camera_id,
  start_at, last_seen_at,
  primary_event, severity_max, confidence_max,
  best_frame_id,
  retention_class,
  state
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'open');
```

### (7) event_tags 反映（重複回避：INSERT IGNORE が便利）

```sql
INSERT IGNORE INTO event_tags(event_id, tag_id, tag_group)
VALUES (?, ?, ?);
```

### (8) close（一定時間検知なしのopen eventをまとめて閉じる）

dispatcherが1分に1回やるのが楽です（cameraごとにやらない）。

```sql
UPDATE events
SET end_at = last_seen_at,
    state='closed',
    updated_at=NOW()
WHERE state='open'
  AND last_seen_at < DATE_SUB(NOW(3), INTERVAL ? SECOND);
```

### (9) job完了/再キュー

```sql
UPDATE inference_jobs
SET status='done', finished_at=NOW(3), last_error=NULL
WHERE job_id=?;
```

失敗時（バックオフして再キュー）

```sql
UPDATE inference_jobs
SET status=IF(attempt >= max_attempt, 'dead', 'queued'),
    available_at=IF(attempt >= max_attempt, available_at, DATE_ADD(NOW(3), INTERVAL ? SECOND)),
    last_error=?,
    locked_by=NULL, locked_token=NULL, locked_at=NULL
WHERE job_id=?;
```

## 5-3. dispatcher擬似実装（要点コード）

```rust
// cam_dispatcher/src/main.rs
use anyhow::Result;
use cam_common::{db, is21, policy, drive, notify};
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    cam_common::telemetry::init_tracing();
    let cfg = cam_common::config::DispatcherConfig::from_env()?;
    let pool = db::connect(&cfg.database_url).await?;
    let is21c = is21::Client::new(cfg.is21_base_url.clone());

    info!("dispatcher start");

    // event closeループ（別タスク）
    {
        let pool2 = pool.clone();
        let grace = cfg.event_close_grace_sec;
        tokio::spawn(async move {
            loop {
                if let Err(e) = db::close_stale_open_events(&pool2, grace).await {
                    warn!(err=?e, "close stale events failed");
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });
    }

    loop {
        // 1) job claim
        let token = Uuid::new_v4().to_string();
        let claimed = db::claim_one_job(&pool, &cfg.worker_id, &token).await?;
        if !claimed {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            continue;
        }
        let job = match db::get_job_by_token(&pool, &token).await? {
            Some(j) => j,
            None => continue,
        };

        // 2) infer画像読み込み（spool/infer/<frame_uuid>.jpg）
        let infer_path = cfg.spool_infer_path(job.frame_uuid);
        let infer_jpeg = tokio::fs::read(&infer_path).await?;

        // 3) is21へ推論
        let resp = match is21c.analyze(&job.camera_id, job.captured_at, &cfg.schema_version, infer_jpeg).await {
            Ok(r) => r,
            Err(e) => {
                warn!(job_id=job.job_id, err=?e, "is21 analyze failed");
                db::requeue_or_dead_job(&pool, job.job_id, cfg.backoff_sec, &format!("{e:?}")).await?;
                continue;
            }
        };

        // 4) retention判定（タグとseverityで normal/quarantine/case を決める）
        let retention = policy::classify_retention(&resp.primary_event, &resp.tags, resp.severity, resp.unknown_flag);

        // 5) frames update + frame_tags更新
        db::update_frame_with_analysis(&pool, job.frame_id, &resp, retention).await?;
        db::replace_frame_tags(&pool, job.frame_id, &resp.tags, &cfg.schema_tag_groups).await?;

        // 6) eventize（open eventをmerge or open）
        let event_id = db::eventize(&pool, &job.camera_id, job.captured_at, &resp, retention).await?;
        // event_tagsは INSERT IGNORE で追加
        db::insert_event_tags_ignore(&pool, event_id, &resp.tags, &cfg.schema_tag_groups).await?;

        // 7) Drive保存（full画像はspool/full/<uuid>.jpg）
        if policy::should_upload_full(&resp, retention) {
            let full_path = cfg.spool_full_path(job.frame_uuid);
            let full_jpeg = tokio::fs::read(&full_path).await?;
            // ここはDrive実装を差し替えできるよう trait 化推奨
            if let Ok(up) = drive::upload_full(&cfg.drive, &job.camera_id, job.captured_at, retention, full_jpeg).await {
                db::update_frame_drive(&pool, job.frame_id, &up.drive_file_id, retention).await?;
                db::insert_drive_object(&pool, &up, job.frame_id, retention).await?;
            }
        }

        // 8) 通知（観測事実のみ）
        if policy::should_notify(&resp, retention) && notify::cooldown_ok(&pool, &job.camera_id, &resp.primary_event, cfg.notify_cooldown_sec).await? {
            let text = notify::build_text(&job.camera_id, job.captured_at, &resp, &cfg.schema_notify_labels);
            // proxyリンク発行（web側と共通化推奨）
            let link = notify::make_proxy_link(&cfg.web_base_url, job.frame_uuid, cfg.media_ttl_sec, &cfg.media_hmac_secret);
            notify::send_webhook(&cfg.webhook, text, Some(link)).await.ok();
            notify::set_cooldown(&pool, &job.camera_id, &resp.primary_event, cfg.notify_cooldown_sec).await?;
        }

        // 9) job done
        db::mark_job_done(&pool, job.job_id).await?;
    }
}
```

> ここで出てくる `notify::cooldown_ok` は、最初はメモリでOKですが、再起動耐性が欲しければ `settings` か `cooldowns` テーブルを足してください（後で増やせます）。

---

# 6. web（axum）テンプレ：API + 署名付き Media Proxy

## 6-1. エンドポイント（最小）

* `GET /api/cameras`
* `GET /api/cameras/{id}/latest`（最新frameのinferを返す or メタ）
* `POST /api/media/link`（frame_uuid→短寿命URL発行）
* `GET /media/frame/{frame_uuid}?exp=...&sig=...`（Driveからstream）

## 6-2. 署名付きURL（HMAC）実装の要点

payload: `"{frame_uuid}|{drive_file_id}|{exp_unix}"`
sig = base64url(HMAC-SHA256(secret, payload))

## 6-3. 重要SQL（latest取得）

```sql
SELECT frame_uuid, captured_at, detected, primary_event, severity, drive_file_id
FROM frames
WHERE camera_id=?
ORDER BY captured_at DESC
LIMIT 1;
```

## 6-4. media proxy（drive_file_id取得）

```sql
SELECT drive_file_id
FROM frames
WHERE frame_uuid=?
LIMIT 1;
```

---

# 7. streamer（axum + ffmpeg HLS）テンプレ（“ほぼ実装”）

> WebRTCを本気でやるなら go2rtc/MediaMTX 推奨ですが、
> ここでは **Rustだけで動く最短**として HLS を出します。

## 7-1. API

* `POST /v1/streams/start {camera_id}`

  * RTSP URLをDBから取得
  * session_id生成
  * `ffmpeg` spawn して `/var/lib/camserver/hls/{session_id}/index.m3u8` 生成
  * `hls_url = /hls/{session_id}/index.m3u8` を返す
* `POST /v1/streams/stop {session_id}`

  * ffmpeg kill + dir cleanup
* `GET /hls/*` 静的配信（axumでDirサービス）

## 7-2. ffmpeg コマンド例（低遅延寄り）

```bash
ffmpeg -nostdin -loglevel error \
  -rtsp_transport tcp -i "rtsp://..." \
  -c:v copy -an \
  -f hls -hls_time 1 -hls_list_size 3 -hls_flags delete_segments+omit_endlist \
  /var/lib/camserver/hls/<sid>/index.m3u8
```

> コピーできないカメラもあるので、その場合は `-c:v libx264 -preset veryfast -tune zerolatency` に切替（負荷増）。

---

# 8. “ほぼ実装”のための DB 関数（cam_common::db）雛形（SQL付き）

以下は **sqlxでそのまま貼れる**ことを優先した形です（要点のみ）。

```rust
// cam_common/src/db/mod.rs
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;

pub async fn connect(url: &str) -> Result<MySqlPool> {
    Ok(MySqlPool::connect(url).await?)
}

pub struct CameraStreamRow {
    pub camera_id: String,
    pub url: String,
}

pub async fn list_enabled_cameras_with_primary_stream(pool: &MySqlPool) -> Result<Vec<CameraStreamRow>> {
    // ※ stream_kind の優先などは運用に合わせて調整
    let rows = sqlx::query!(
        r#"
        SELECT c.camera_id as camera_id, s.url as url
        FROM cameras c
        JOIN camera_streams s ON s.camera_id = c.camera_id
        WHERE c.enabled=1 AND s.is_primary=1
        "#
    ).fetch_all(pool).await?;

    Ok(rows.into_iter().map(|r| CameraStreamRow {
        camera_id: r.camera_id,
        url: r.url,
    }).collect())
}

// --- frames insert（collector） ---
pub struct DiffMetrics { pub diff_ratio: f64, pub luma_delta: i32, pub blur_score: Option<f64> }

pub async fn insert_frame_initial(
    pool: &MySqlPool,
    frame_uuid: uuid::Uuid,
    camera_id: &str,
    captured_at: DateTime<Utc>,
    diff: DiffMetrics,
    infer_jpeg: &[u8],
    infer_hash16: String,
    full_jpeg: &[u8],
) -> Result<u64> {
    // サイズだけ抽出（image crateなどで）
    let (full_w, full_h) = crate::image_utils::jpeg_size(full_jpeg).unwrap_or((0,0));
    let (infer_w, infer_h) = crate::image_utils::jpeg_size(infer_jpeg).unwrap_or((0,0));

    let res = sqlx::query!(
        r#"
        INSERT INTO frames (
          frame_uuid, camera_id, captured_at,
          collector_status, error_code, error_message,
          diff_ratio, luma_delta, blur_score,
          analyzed, detected, primary_event, unknown_flag, severity,
          full_w, full_h, infer_w, infer_h,
          letterbox, letterbox_top, letterbox_left, letterbox_bottom, letterbox_right,
          infer_hash64_hex
        ) VALUES (?, ?, ?, 'ok', NULL, NULL, ?, ?, ?, 0, 0, 'none', 0, 0, ?, ?, ?, ?, 0, NULL, NULL, NULL, NULL, ?)
        "#,
        frame_uuid.to_string(),
        camera_id,
        captured_at.naive_utc(),
        diff.diff_ratio,
        diff.luma_delta,
        diff.blur_score,
        full_w as i32, full_h as i32,
        infer_w as i32, infer_h as i32,
        infer_hash16
    ).execute(pool).await?;

    Ok(res.last_insert_id())
}

pub async fn insert_frame_error(
    pool: &MySqlPool,
    frame_uuid: uuid::Uuid,
    camera_id: &str,
    captured_at: DateTime<Utc>,
    err: &anyhow::Error,
) -> Result<()> {
    let msg = format!("{err:?}");
    sqlx::query!(
        r#"
        INSERT INTO frames (frame_uuid, camera_id, captured_at, collector_status, error_code, error_message,
                           analyzed, detected, primary_event, unknown_flag, severity)
        VALUES (?, ?, ?, 'error', 'capture_failed', ?, 0, 0, 'none', 0, 0)
        "#,
        frame_uuid.to_string(),
        camera_id,
        captured_at.naive_utc(),
        msg
    ).execute(pool).await?;
    Ok(())
}

// --- job enqueue（collector） ---
pub async fn enqueue_inference_job(pool: &MySqlPool, frame_id: u64, priority: u8) -> Result<()> {
    // uq_inference_jobs_frame があるので重複しない
    sqlx::query!(
        r#"INSERT INTO inference_jobs(frame_id, status, priority, available_at)
           VALUES (?, 'queued', ?, NOW(3))"#,
        frame_id,
        priority as i32
    ).execute(pool).await?;
    Ok(())
}

// --- claim（dispatcher） ---
pub async fn claim_one_job(pool: &MySqlPool, locked_by: &str, token: &str) -> Result<bool> {
    let r = sqlx::query!(
        r#"
        UPDATE inference_jobs
        SET status='running',
            locked_by=?,
            locked_token=?,
            locked_at=NOW(3),
            attempt=attempt+1
        WHERE status='queued'
          AND available_at <= NOW(3)
        ORDER BY priority DESC, job_id ASC
        LIMIT 1
        "#,
        locked_by, token
    ).execute(pool).await?;
    Ok(r.rows_affected() == 1)
}

pub struct ClaimedJob {
    pub job_id: u64,
    pub frame_id: u64,
    pub frame_uuid: uuid::Uuid,
    pub camera_id: String,
    pub captured_at: DateTime<Utc>,
}

pub async fn get_job_by_token(pool: &MySqlPool, token: &str) -> Result<Option<ClaimedJob>> {
    let row = sqlx::query!(
        r#"
        SELECT j.job_id, j.frame_id, f.frame_uuid, f.camera_id, f.captured_at
        FROM inference_jobs j
        JOIN frames f ON f.frame_id=j.frame_id
        WHERE j.locked_token=?
        LIMIT 1
        "#,
        token
    ).fetch_optional(pool).await?;

    Ok(row.map(|r| ClaimedJob {
        job_id: r.job_id,
        frame_id: r.frame_id,
        frame_uuid: uuid::Uuid::parse_str(&r.frame_uuid).unwrap(),
        camera_id: r.camera_id,
        captured_at: DateTime::<Utc>::from_naive_utc_and_offset(r.captured_at, Utc),
    }))
}
```

（この後に `update_frame_with_analysis`, `replace_frame_tags`, `eventize`, `close_stale_open_events`, `mark_job_done`, `requeue_or_dead_job` を同様に足していくのが、最短実装ルートです）

---

# 9. systemd分割（運用イメージ）

* `cam-collector.service`
* `cam-dispatcher.service`
* `cam-web.service`
* `cam-streamer.service`

それぞれに

* `DATABASE_URL`
* `SPOOL_DIR`
* `IS21_BASE_URL`
* `SCHEMA_VERSION`
* `MEDIA_HMAC_SECRET`
* `DRIVE_*`
  などの環境変数を渡す形がシンプルです。

---

# 10. 次に進めるなら（こちらから提案）

ここまでで **“ほぼ実装”の骨**は揃いました。次の1手で実装スピードが上がります。

1. あなたの is22 の実環境（Apache/443/内部ポート）に合わせて

   * `web` を **8080**、`streamer` を **8082** などに割当
   * Apacheで `/api` を web にリバプロ、`/hls` を streamer にリバプロ
2. `schema.json` を `schema_versions` に投入して `is_active=1` にする初期化SQL
3. is21側の `schema_tag_groups` を is22が配布する処理（dispatcher起動時にPUT）

---
