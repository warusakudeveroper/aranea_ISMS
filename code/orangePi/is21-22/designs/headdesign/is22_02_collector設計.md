# is22 collector設計

文書番号: is22_02
作成日: 2025-12-29
ステータス: Draft
参照: 特記仕様3 Section 4

---

## 1. 概要

### 1.1 責務
- DBからカメラ一覧を読み込み
- 周期的にRTSPスナップショット取得
- infer画像・diff画像の生成
- 差分判定（早期リターン）
- framesテーブルへのINSERT
- 推論が必要な場合はinference_jobsへenqueue

### 1.2 責務外
- is21への推論リクエスト送信（dispatcher担当）
- イベント化（dispatcher担当）
- 通知送信（dispatcher担当）

---

## 2. 処理フロー

```
[1] DBからenabled camerasを取得
       ↓
[2] 各カメラに対してRTSPスナップ取得
       ↓
[3] フル画像をspool/full/に保存
       ↓
[4] infer画像を生成（横640px）
       ↓
[5] diff画像を生成（横320px, グレースケール）
       ↓
[6] 前回diff画像と比較
       ↓
[7] 差分判定
       ├─ 小（diff_ratio < 0.003）→ no_event
       └─ 大 or 強制推論 → enqueue
       ↓
[8] frames INSERT
       ↓
[9] (推論必要時) inference_jobs INSERT
```

---

## 3. RTSP取得

### 3.1 ffmpegコマンド
```bash
ffmpeg -nostdin -loglevel error \
  -rtsp_transport tcp \
  -i "rtsp://user:pass@camera_ip:554/stream" \
  -vframes 1 \
  -q:v 2 \
  -f image2pipe \
  pipe:1
```

### 3.2 タイムアウト設計
| カメラ種別 | タイムアウト |
|-----------|------------|
| ローカル | 3秒 |
| VPN経由 | 5秒 |
| 失敗時 | 次周期へスキップ |

### 3.3 エラーハンドリング
```rust
match rtsp::grab_jpeg(&cam.url, timeout).await {
    Ok(bytes) => { /* 正常処理 */ },
    Err(e) => {
        // collector_status=error で frames INSERT
        db::insert_frame_error(pool, frame_uuid, &cam.camera_id, captured_at, &e).await?;
        // 次のカメラへ
        return Ok(());
    }
}
```

---

## 4. 画像処理

### 4.1 ディレクトリ構成
```
/var/lib/camserver/spool/
├── full/         # フル解像度（JPEGのまま）
│   └── {frame_uuid}.jpg
├── infer/        # 推論用リサイズ
│   └── {frame_uuid}.jpg
└── prev_diff/    # 差分比較用（カメラごと最新1枚）
    └── {camera_id}.gray
```

### 4.2 画像サイズ
| 種別 | 解像度 | 形式 |
|-----|--------|------|
| full | 元解像度 | JPEG |
| infer | 横640px（アスペクト維持） | JPEG |
| diff | 横320px | グレースケール |

### 4.3 リサイズ処理
```rust
use image::{DynamicImage, ImageFormat};

pub fn resize_to_width(jpeg: &[u8], target_width: u32) -> Result<Vec<u8>> {
    let img = image::load_from_memory(jpeg)?;
    let ratio = target_width as f32 / img.width() as f32;
    let new_height = (img.height() as f32 * ratio) as u32;

    let resized = img.resize_exact(target_width, new_height, image::imageops::FilterType::Triangle);

    let mut buf = Vec::new();
    resized.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)?;
    Ok(buf)
}
```

---

## 5. 差分判定

### 5.1 アルゴリズム
```rust
pub struct DiffMetrics {
    pub diff_ratio: f64,    // 0.0 - 1.0
    pub luma_delta: i32,    // 平均輝度差
}

pub fn compute_diff(current: &[u8], prev_path: &Path) -> Result<DiffMetrics> {
    let current_gray = to_grayscale(current)?;

    if !prev_path.exists() {
        // 初回は差分大として扱う
        return Ok(DiffMetrics { diff_ratio: 1.0, luma_delta: 255 });
    }

    let prev_gray = read_grayscale(prev_path)?;

    // ピクセル差分
    let mut diff_count = 0;
    let mut luma_sum = 0i64;
    for (c, p) in current_gray.iter().zip(prev_gray.iter()) {
        let delta = (*c as i32 - *p as i32).abs();
        if delta > 20 {  // 閾値
            diff_count += 1;
        }
        luma_sum += delta as i64;
    }

    let diff_ratio = diff_count as f64 / current_gray.len() as f64;
    let luma_delta = (luma_sum / current_gray.len() as i64) as i32;

    Ok(DiffMetrics { diff_ratio, luma_delta })
}
```

### 5.2 判定閾値
| パラメータ | デフォルト値 | 説明 |
|-----------|-------------|------|
| diff_ratio_no_event | 0.003 | 差分率閾値 |
| luma_delta_no_event | 5 | 輝度差閾値 |
| force_infer_every_n | 60 | 強制推論間隔（回） |

### 5.3 強制推論
- N回連続no_eventでも、強制的に推論を実行
- 見逃し防止のため

```rust
pub async fn should_force_infer(pool: &MySqlPool, camera_id: &str, force_every_n: u32) -> Result<bool> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as count
        FROM frames
        WHERE camera_id = ?
          AND analyzed = 0
          AND captured_at > DATE_SUB(NOW(), INTERVAL 1 HOUR)
        "#,
        camera_id
    ).fetch_one(pool).await?;

    Ok(count.unwrap_or(0) >= force_every_n as i64)
}
```

---

## 6. DB操作

### 6.1 frames INSERT
```sql
INSERT INTO frames (
  frame_uuid, camera_id, captured_at,
  collector_status, error_code, error_message,
  diff_ratio, luma_delta, blur_score,
  analyzed, detected, primary_event, unknown_flag, severity,
  full_w, full_h, infer_w, infer_h,
  letterbox, infer_hash64_hex
) VALUES (?, ?, ?, 'ok', NULL, NULL, ?, ?, ?, 0, 0, 'none', 0, 0, ?, ?, ?, ?, 0, ?);
```

### 6.2 inference_jobs enqueue
```sql
INSERT INTO inference_jobs (frame_id, status, priority, available_at)
VALUES (?, 'queued', ?, NOW(3));
```

### 6.3 エラー時frames INSERT
```sql
INSERT INTO frames (
  frame_uuid, camera_id, captured_at,
  collector_status, error_code, error_message,
  analyzed, detected, primary_event, unknown_flag, severity
) VALUES (?, ?, ?, 'error', 'capture_failed', ?, 0, 0, 'none', 0, 0);
```

---

## 7. 設定

### 7.1 環境変数
| 変数 | デフォルト | 説明 |
|-----|-----------|------|
| DATABASE_URL | - | MariaDB接続文字列 |
| SPOOL_DIR | /var/lib/camserver/spool | 画像保存先 |
| TICK_SEC | 60 | 収集周期（秒） |
| RTSP_TIMEOUT_SEC | 3 | ローカルカメラタイムアウト |
| RTSP_TIMEOUT_VPN_SEC | 5 | VPNカメラタイムアウト |
| DEFAULT_INFER_WIDTH | 640 | infer画像幅 |
| DEFAULT_DIFF_WIDTH | 320 | diff画像幅 |
| DIFF_RATIO_NO_EVENT | 0.003 | 差分率閾値 |
| FORCE_INFER_EVERY_N | 60 | 強制推論間隔 |

### 7.2 カメラ個別オーバーライド
- camera_overridesテーブルで個別設定可能
- infer_width, diff_width, force_infer_every_n

---

## 8. systemdサービス

```ini
# /etc/systemd/system/cam-collector.service
[Unit]
Description=camServer Collector
After=network.target mariadb.service

[Service]
Type=simple
User=camserver
WorkingDirectory=/opt/camserver
ExecStart=/opt/camserver/bin/cam_collector
Restart=always
RestartSec=5

Environment=DATABASE_URL=mysql://camserver:pass@localhost/camserver
Environment=SPOOL_DIR=/var/lib/camserver/spool
Environment=TICK_SEC=60

[Install]
WantedBy=multi-user.target
```

---

## 9. テスト観点

### 9.1 正常系
- [ ] 30台カメラから60秒周期で収集
- [ ] VPNカメラ（遅延あり）も収集成功
- [ ] 差分小でno_event、frames INSERT
- [ ] 差分大でinference_jobs enqueue

### 9.2 異常系
- [ ] カメラ接続失敗時にerror frames INSERT
- [ ] 個別失敗が他カメラに波及しない
- [ ] タイムアウト後、次カメラへ進む

### 9.3 境界値
- [ ] 初回（prev_diffなし）→ 差分大扱い
- [ ] force_infer_every_n到達で強制推論
- [ ] 巨大画像（4K）の処理
