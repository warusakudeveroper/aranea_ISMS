# is22 dispatcher設計

文書番号: is22_03
作成日: 2025-12-29
ステータス: Draft
参照: 特記仕様2 Section B, 特記仕様3 Section 5

---

## 1. 概要

### 1.1 責務
- inference_jobsからジョブを取得（claim）
- is21へ推論リクエスト送信
- framesテーブルの推論結果更新
- frame_tagsテーブルへのタグ保存
- イベント化（events open/merge/close）
- event_tagsテーブルへのタグ保存
- Drive保存（条件付き）
- 通知送信（条件付き、クールダウン）
- LLMエスカレーション（unknown時のみ）

### 1.2 責務外
- RTSP取得（collector担当）
- Web UI提供（web担当）

---

## 2. 処理フロー

```
[1] inference_jobs から1件claim
       ↓
[2] spool/infer/{frame_uuid}.jpg を読み込み
       ↓
[3] is21 POST /v1/analyze
       ↓
[4] frames UPDATE（推論結果）
       ↓
[5] frame_tags DELETE + INSERT
       ↓
[6] retention判定（normal/quarantine/case）
       ↓
[7] イベント化
       ├─ open event あり → can_merge判定
       │     ├─ merge可 → events UPDATE
       │     └─ merge不可 → events CLOSE + 新規OPEN
       └─ open event なし → 新規OPEN
       ↓
[8] event_tags INSERT IGNORE
       ↓
[9] Drive保存（条件付き）
       ↓
[10] 通知（条件付き、クールダウン）
       ↓
[11] LLMエスカレーション（unknown時のみ）
       ↓
[12] inference_jobs done
```

---

## 3. ジョブ管理

### 3.1 claim処理
```sql
-- 1件を排他取得
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

### 3.2 完了処理
```sql
UPDATE inference_jobs
SET status='done', finished_at=NOW(3), last_error=NULL
WHERE job_id=?;
```

### 3.3 失敗時（再キュー）
```sql
UPDATE inference_jobs
SET status=IF(attempt >= max_attempt, 'dead', 'queued'),
    available_at=IF(attempt >= max_attempt, available_at, DATE_ADD(NOW(3), INTERVAL ? SECOND)),
    last_error=?,
    locked_by=NULL, locked_token=NULL, locked_at=NULL
WHERE job_id=?;
```

---

## 4. is21通信

### 4.1 リクエスト
```rust
let resp = is21_client.analyze(
    &job.camera_id,
    job.captured_at,
    &config.schema_version,
    infer_jpeg
).await;
```

### 4.2 エラーハンドリング
| ステータス | 対応 |
|-----------|------|
| 200 OK | 正常処理 |
| 400 Bad Request | dead扱い（画像不正） |
| 409 Schema Mismatch | PUT /v1/schema → 再試行 |
| 429 Overloaded | 再キュー（バックオフ） |
| 503 Unavailable | 再キュー（バックオフ） |
| 接続エラー | 再キュー（バックオフ） |

---

## 5. frames更新

### 5.1 UPDATE SQL
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

### 5.2 frame_tags更新
```sql
-- 既存削除
DELETE FROM frame_tags WHERE frame_id=?;

-- 新規挿入
INSERT INTO frame_tags (frame_id, tag_id, tag_group)
VALUES (?, ?, ?);
```

---

## 6. イベント化

### 6.1 パラメータ
| パラメータ | デフォルト値 | 説明 |
|-----------|-------------|------|
| EVENT_CLOSE_GRACE_SEC | 120 | closeまでの猶予（秒） |
| EVENT_MERGE_GAP_SEC | 90 | merge許容ギャップ（秒） |

### 6.2 open event取得
```sql
SELECT event_id, primary_event, start_at, last_seen_at,
       retention_class, best_frame_id, severity_max, confidence_max
FROM events
WHERE camera_id=? AND state='open'
ORDER BY start_at DESC
LIMIT 1;
```

### 6.3 can_merge判定
```rust
fn can_merge(open_event: &Event, result: &AnalyzeResponse, captured_at: DateTime<Utc>) -> bool {
    // 時間ギャップ
    if captured_at - open_event.last_seen_at > Duration::seconds(EVENT_MERGE_GAP_SEC) {
        return false;
    }

    // primary_eventが異なる
    if open_event.primary_event != result.primary_event {
        return false;
    }

    true
}
```

### 6.4 events UPDATE（merge）
```sql
UPDATE events
SET last_seen_at=?,
    severity_max=GREATEST(severity_max, ?),
    confidence_max=GREATEST(COALESCE(confidence_max, 0), ?),
    retention_class=?,
    updated_at=NOW()
WHERE event_id=?;
```

### 6.5 events INSERT（open）
```sql
INSERT INTO events (
  event_uuid, camera_id,
  start_at, last_seen_at,
  primary_event, severity_max, confidence_max,
  best_frame_id, retention_class, state
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'open');
```

### 6.6 stale event close（定期処理）
```sql
UPDATE events
SET end_at = last_seen_at,
    state='closed',
    updated_at=NOW()
WHERE state='open'
  AND last_seen_at < DATE_SUB(NOW(3), INTERVAL ? SECOND);
```

---

## 7. best_frame選定

### 7.1 スコア計算
```rust
fn compute_best_frame_score(result: &AnalyzeResponse, frame: &Frame) -> f64 {
    let severity_score = result.severity as f64 * 1000.0;
    let confidence_score = result.confidence.unwrap_or(0.0) * 100.0;
    let blur_penalty = frame.blur_score.unwrap_or(0.0) * 0.1;

    severity_score + confidence_score - blur_penalty
}
```

### 7.2 更新処理
```sql
UPDATE events
SET best_frame_id=?
WHERE event_id=?
  AND (best_frame_id IS NULL OR ? >
       (SELECT severity * 1000 + COALESCE(confidence, 0) * 100 FROM frames WHERE frame_id = events.best_frame_id));
```

---

## 8. retention判定

### 8.1 ルール
| 条件 | retention_class |
|-----|-----------------|
| case_id付与（手動） | case |
| hazard.* | quarantine |
| camera.offline/moved/occluded | quarantine |
| behavior.loitering | quarantine |
| object_missing.suspected | quarantine |
| animal.deer/boar | quarantine |
| unknown_flag=true | quarantine |
| severity>=2 | quarantine |
| detected=true（上記以外） | normal |
| detected=false | normal（Drive保存しない） |

### 8.2 実装
```rust
fn classify_retention(
    primary_event: &str,
    tags: &[String],
    severity: u8,
    unknown_flag: bool
) -> RetentionClass {
    // hazard
    if tags.iter().any(|t| t.starts_with("hazard.")) {
        return RetentionClass::Quarantine;
    }

    // camera異常
    if tags.iter().any(|t| matches!(t.as_str(),
        "camera.offline" | "camera.moved" | "camera.occluded")) {
        return RetentionClass::Quarantine;
    }

    // その他quarantine条件
    if unknown_flag || severity >= 2 {
        return RetentionClass::Quarantine;
    }

    RetentionClass::Normal
}
```

---

## 9. systemdサービス

```ini
# /etc/systemd/system/cam-dispatcher.service
[Unit]
Description=camServer Dispatcher
After=network.target mariadb.service

[Service]
Type=simple
User=camserver
WorkingDirectory=/opt/camserver
ExecStart=/opt/camserver/bin/cam_dispatcher
Restart=always
RestartSec=5

Environment=DATABASE_URL=mysql://camserver:pass@localhost/camserver
Environment=SPOOL_DIR=/var/lib/camserver/spool
Environment=IS21_BASE_URL=http://is21:9000
Environment=SCHEMA_VERSION=2025-12-28.1

[Install]
WantedBy=multi-user.target
```

---

## 10. テスト観点

### 10.1 正常系
- [ ] ジョブclaim成功
- [ ] is21推論結果のframes UPDATE
- [ ] イベントopen/merge/close
- [ ] best_frame選定

### 10.2 異常系
- [ ] is21接続失敗時の再キュー
- [ ] 409 schema mismatch時の再プッシュ
- [ ] 429 overloaded時のバックオフ
- [ ] max_attempt到達でdead

### 10.3 イベント化
- [ ] 同一primary_eventでmerge
- [ ] 異なるprimary_eventで別イベント
- [ ] 時間ギャップ超過で別イベント
- [ ] 120秒検知なしでclose
