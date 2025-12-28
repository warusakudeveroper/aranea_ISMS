# 完全性チェックテスト計画

文書番号: TEST_01
作成日: 2025-12-29
ステータス: Draft

---

## 1. テスト方針

### 1.1 テストレベル
| レベル | 対象 | ツール |
|--------|------|--------|
| 単体テスト | 関数・モジュール | pytest / cargo test |
| 統合テスト | サービス間連携 | pytest + httpx / cargo test |
| E2Eテスト | 全体フロー | シェルスクリプト |
| 性能テスト | 負荷・レイテンシ | wrk / hey |

### 1.2 テスト環境
| 環境 | 用途 | 構成 |
|------|------|------|
| 開発 | 単体・統合 | ローカルDB + モック |
| ステージング | E2E | 実機構成 |
| 本番 | スモーク | 限定カメラ |

---

## 2. Phase 0: 基盤テスト

### 2.1 IS21-001: RKNNランタイム

#### UT-IS21-001-01: ランタイム初期化
```python
# test_inference.py
import pytest
from inference import InferenceEngine

def test_init_runtime():
    """RKNNランタイムが正常に初期化される"""
    engine = InferenceEngine("/opt/is21/models/yolov8n_640.rknn")
    assert engine.rknn is not None

def test_init_runtime_invalid_model():
    """存在しないモデルでエラー"""
    with pytest.raises(RuntimeError):
        InferenceEngine("/nonexistent.rknn")
```

#### UT-IS21-001-02: NPU使用確認
```bash
# test_npu.sh
#!/bin/bash
# NPUデバイスの存在確認
test -c /dev/dri/renderD128 || exit 1

# NPU使用率取得
cat /sys/kernel/debug/rknpu/load | grep -q "Core0" || exit 1
```

**達成条件チェックリスト**
- [ ] `rknn.init_runtime()`が成功
- [ ] サンプルモデル推論が動作（test_sample_inference.py）
- [ ] NPU使用率がモニタリング可能

---

### 2.2 IS21-002: 推論モデル

#### UT-IS21-002-01: モデル変換検証
```python
def test_model_conversion():
    """RKNNモデルがロード可能"""
    from rknnlite.api import RKNNLite
    rknn = RKNNLite()
    ret = rknn.load_rknn("/opt/is21/models/yolov8n_640.rknn")
    assert ret == 0

def test_model_input_shape():
    """入力形状が640x640x3"""
    # モデルのinput_attrs確認
    assert engine.input_shape == (1, 640, 640, 3)
```

#### UT-IS21-002-02: 検出精度
```python
@pytest.mark.parametrize("image,expected", [
    ("test_person.jpg", ["person"]),
    ("test_dog.jpg", ["dog"]),
    ("test_empty.jpg", []),
])
def test_detection_accuracy(image, expected):
    """基本検出精度"""
    img = cv2.imread(f"test_data/{image}")
    result = engine.infer(img)
    detected_classes = [d["class"] for d in result]
    for exp in expected:
        assert exp in detected_classes
```

**達成条件チェックリスト**
- [ ] 人物検出モデルがRKNN形式で動作
- [ ] 640px入力で推論時間 < 300ms
- [ ] 検出精度mAP > 0.5

---

### 2.3 IS22-001: Rustワークスペース

#### UT-IS22-001-01: ビルドテスト
```bash
# test_build.sh
cd /opt/camserver
cargo build --release 2>&1 | tee build.log
test $? -eq 0 || exit 1

# 各バイナリの存在確認
for bin in cam_collector cam_dispatcher cam_web cam_streamer; do
    test -f target/release/$bin || exit 1
done
```

#### UT-IS22-001-02: 共通クレート
```rust
// cam_common/tests/test_config.rs
#[test]
fn test_config_load() {
    std::env::set_var("DATABASE_URL", "mysql://test:test@localhost/test");
    let config = cam_common::Config::from_env().unwrap();
    assert_eq!(config.database_url, "mysql://test:test@localhost/test");
}
```

**達成条件チェックリスト**
- [ ] `cargo build --release`成功
- [ ] 各binがビルド可能
- [ ] 共通クレートが参照可能

---

### 2.4 IS22-002: MariaDB DDL

#### UT-IS22-002-01: テーブル作成
```sql
-- test_ddl.sql
-- 全テーブルの存在確認
SELECT COUNT(*) = 15 as ok FROM information_schema.tables
WHERE table_schema = 'camserver';

-- cameras テーブル構造確認
DESCRIBE cameras;

-- 外部キー制約確認
SELECT COUNT(*) > 0 as ok FROM information_schema.key_column_usage
WHERE table_schema = 'camserver' AND referenced_table_name IS NOT NULL;
```

#### UT-IS22-002-02: CRUD操作
```rust
// tests/test_db.rs
#[sqlx::test]
async fn test_camera_crud(pool: MySqlPool) {
    // INSERT
    sqlx::query!(
        "INSERT INTO cameras (camera_id, display_name) VALUES (?, ?)",
        "test_cam", "Test Camera"
    )
    .execute(&pool)
    .await
    .unwrap();

    // SELECT
    let cam = sqlx::query!("SELECT * FROM cameras WHERE camera_id = ?", "test_cam")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(cam.display_name, "Test Camera");

    // UPDATE
    sqlx::query!("UPDATE cameras SET enabled = 0 WHERE camera_id = ?", "test_cam")
        .execute(&pool)
        .await
        .unwrap();

    // DELETE
    sqlx::query!("DELETE FROM cameras WHERE camera_id = ?", "test_cam")
        .execute(&pool)
        .await
        .unwrap();
}
```

**達成条件チェックリスト**
- [ ] 全テーブル作成成功
- [ ] 外部キー制約動作
- [ ] camserverユーザーでCRUD可能

---

### 2.5 IS22-003: RTSP取得

#### UT-IS22-003-01: ローカルカメラ
```rust
// tests/test_rtsp.rs
#[tokio::test]
async fn test_rtsp_local_camera() {
    let url = "rtsp://192.168.1.10:554/stream1";
    let output = "/tmp/test_frame.jpg";

    let start = Instant::now();
    let result = rtsp::capture_frame(url, output, 5).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert!(elapsed.as_secs() < 3); // 3秒以内
    assert!(Path::new(output).exists());
}
```

#### UT-IS22-003-02: エラーハンドリング
```rust
#[tokio::test]
async fn test_rtsp_invalid_url() {
    let result = rtsp::capture_frame("rtsp://invalid", "/tmp/test.jpg", 3).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rtsp_timeout() {
    // 応答しないアドレス
    let result = rtsp::capture_frame("rtsp://192.168.255.255/stream", "/tmp/test.jpg", 2).await;
    assert!(result.is_err());
}
```

**達成条件チェックリスト**
- [ ] ローカルカメラで3秒以内取得
- [ ] VPNカメラで5秒以内取得
- [ ] 失敗時にエラー捕捉

---

## 3. Phase 1: コア機能テスト

### 3.1 IS21-003: FastAPI推論サーバ

#### IT-IS21-003-01: ヘルスチェック
```python
import httpx
import pytest

@pytest.fixture
def client():
    return httpx.Client(base_url="http://localhost:8000")

def test_healthz(client):
    """ヘルスチェック応答 < 10ms"""
    import time
    start = time.time()
    resp = client.get("/healthz")
    elapsed = time.time() - start

    assert resp.status_code == 200
    assert resp.json()["status"] == "ok"
    assert elapsed < 0.01
```

#### IT-IS21-003-02: 推論エンドポイント
```python
def test_analyze_endpoint(client):
    """推論応答 < 500ms"""
    with open("test_data/test_person.jpg", "rb") as f:
        start = time.time()
        resp = client.post(
            "/v1/analyze",
            files={"image": f},
            params={"camera_id": "test_cam"}
        )
        elapsed = time.time() - start

    assert resp.status_code == 200
    assert elapsed < 0.5
    assert "tags" in resp.json()
    assert "primary_event" in resp.json()
```

#### IT-IS21-003-03: スキーマ不在時
```python
def test_analyze_no_schema(client):
    """スキーマ未設定で409"""
    # スキーマをクリア（テスト用エンドポイント）
    client.delete("/v1/schema")

    with open("test_data/test_person.jpg", "rb") as f:
        resp = client.post("/v1/analyze", files={"image": f})

    assert resp.status_code == 409
```

**達成条件チェックリスト**
- [ ] /healthz応答 < 10ms
- [ ] /v1/analyze応答 < 500ms（推論込み）
- [ ] 同時5リクエスト処理可能

---

### 3.2 IS22-004: 差分判定

#### UT-IS22-004-01: 静止画面判定
```rust
#[test]
fn test_no_change_detection() {
    let img = image::open("test_data/static_frame.jpg").unwrap();
    let result = diff::compute_diff(&img, &img, 0.003);

    assert!(result.diff_ratio < 0.003);
    assert!(!result.should_analyze);
}
```

#### UT-IS22-004-02: 動き検出
```rust
#[test]
fn test_motion_detection() {
    let frame1 = image::open("test_data/empty_room.jpg").unwrap();
    let frame2 = image::open("test_data/person_enter.jpg").unwrap();
    let result = diff::compute_diff(&frame2, &frame1, 0.003);

    assert!(result.diff_ratio >= 0.003);
    assert!(result.should_analyze);
}
```

**達成条件チェックリスト**
- [ ] 静止画面でno_event判定
- [ ] 人物出現で差分検知
- [ ] 強制推論が動作

---

### 3.3 IS22-006: ジョブキュー

#### UT-IS22-006-01: 排他取得
```rust
#[sqlx::test]
async fn test_job_claim_exclusive(pool: MySqlPool) {
    // ジョブ投入
    sqlx::query!("INSERT INTO inference_jobs (frame_id, status) VALUES (1, 'queued')")
        .execute(&pool)
        .await
        .unwrap();

    let manager1 = JobManager::new(pool.clone(), "worker1".into());
    let manager2 = JobManager::new(pool.clone(), "worker2".into());

    // 並列取得
    let (job1, job2) = tokio::join!(
        manager1.claim(),
        manager2.claim(),
    );

    // どちらか一方のみが取得
    let claimed = [job1.unwrap(), job2.unwrap()]
        .iter()
        .filter(|j| j.is_some())
        .count();
    assert_eq!(claimed, 1);
}
```

#### UT-IS22-006-02: 再キュー
```rust
#[sqlx::test]
async fn test_job_requeue(pool: MySqlPool) {
    // 初期状態
    sqlx::query!(
        "INSERT INTO inference_jobs (frame_id, status, attempt) VALUES (1, 'running', 1)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let manager = JobManager::new(pool.clone(), "worker1".into());
    manager.requeue(1, 60).await.unwrap();

    let job = sqlx::query!("SELECT * FROM inference_jobs WHERE job_id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(job.status, "queued");
    assert_eq!(job.attempt, 2);
}
```

**達成条件チェックリスト**
- [ ] 1ジョブが1workerのみに配布
- [ ] 失敗時に再キュー
- [ ] max_attempt超過でdead

---

### 3.4 IS22-008: イベント化

#### UT-IS22-008-01: イベントマージ
```rust
#[sqlx::test]
async fn test_event_merge(pool: MySqlPool) {
    let manager = EventManager::new(pool.clone());

    // 1フレーム目: 新規イベント作成
    let frame1 = AnalyzedFrame {
        camera_id: "cam1".into(),
        captured_at: Utc::now(),
        detected: true,
        primary_event: "human".into(),
        ..Default::default()
    };
    let action1 = manager.process_frame(&frame1).await.unwrap();
    assert!(matches!(action1, EventAction::Created(_)));

    // 2フレーム目: 30秒後 → マージ
    let frame2 = AnalyzedFrame {
        captured_at: Utc::now() + Duration::seconds(30),
        ..frame1.clone()
    };
    let action2 = manager.process_frame(&frame2).await.unwrap();
    assert!(matches!(action2, EventAction::Merged(_)));
}
```

#### UT-IS22-008-02: イベント分離
```rust
#[sqlx::test]
async fn test_event_separate(pool: MySqlPool) {
    let manager = EventManager::new(pool.clone());

    // 1フレーム目
    let frame1 = AnalyzedFrame {
        camera_id: "cam1".into(),
        captured_at: Utc::now(),
        detected: true,
        primary_event: "human".into(),
        ..Default::default()
    };
    manager.process_frame(&frame1).await.unwrap();

    // 2フレーム目: 90秒後（ギャップ > 60秒）→ 新規イベント
    let frame2 = AnalyzedFrame {
        captured_at: Utc::now() + Duration::seconds(90),
        ..frame1.clone()
    };
    let action = manager.process_frame(&frame2).await.unwrap();
    assert!(matches!(action, EventAction::Created(_)));
}
```

**達成条件チェックリスト**
- [ ] 連続検知でmerge
- [ ] 時間ギャップで別イベント
- [ ] 120秒無検知でclose

---

### 3.5 IS22-013: Axum API

#### IT-IS22-013-01: カメラ一覧
```rust
#[tokio::test]
async fn test_list_cameras() {
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:8080/api/cameras")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let cameras: Vec<Camera> = resp.json().await.unwrap();
    assert!(!cameras.is_empty());
}
```

#### IT-IS22-013-02: イベント検索
```rust
#[tokio::test]
async fn test_search_events() {
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:8080/api/events")
        .query(&[
            ("camera_id", "cam1"),
            ("primary_event", "human"),
            ("limit", "10"),
        ])
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let events: EventSearchResponse = resp.json().await.unwrap();
    assert!(events.events.len() <= 10);
}
```

**達成条件チェックリスト**
- [ ] /api/cameras応答
- [ ] /api/events検索動作
- [ ] JSONレスポンス形式正しい

---

## 4. Phase 2: 付加機能テスト

### 4.1 IS22-014: 署名付きMedia Proxy

#### UT-IS22-014-01: 署名生成・検証
```rust
#[test]
fn test_sign_verify() {
    let secret = b"test-secret";
    let uuid = "abc123";

    let url = media::generate_signed_url(uuid, secret, 30);
    let params: HashMap<_, _> = url.split('?')
        .nth(1).unwrap()
        .split('&')
        .map(|p| {
            let mut kv = p.split('=');
            (kv.next().unwrap(), kv.next().unwrap())
        })
        .collect();

    let exp: i64 = params["exp"].parse().unwrap();
    let sig = params["sig"];

    assert!(media::verify_signature(uuid, exp, sig, secret));
}
```

#### UT-IS22-014-02: 期限切れ
```rust
#[test]
fn test_expired_signature() {
    let secret = b"test-secret";
    let uuid = "abc123";
    let exp = Utc::now().timestamp() - 100; // 100秒前

    // 有効な署名を生成
    let data = format!("{}:{}", uuid, exp);
    let mut mac = HmacSha256::new_from_slice(secret).unwrap();
    mac.update(data.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());

    // 期限切れで検証失敗
    assert!(!media::verify_signature(uuid, exp, &sig, secret));
}
```

**達成条件チェックリスト**
- [ ] 正しい署名で画像取得
- [ ] 不正署名で403
- [ ] 期限切れで403

---

## 5. E2Eテスト

### E2E-01: フレーム収集→推論→イベント化

```bash
#!/bin/bash
# test_e2e_flow.sh

set -e

echo "=== E2E Test: Frame Collection to Event ==="

# 1. テストカメラ登録
mysql -u camserver -p camserver << 'EOF'
INSERT INTO cameras (camera_id, display_name, enabled)
VALUES ('e2e_test', 'E2E Test Camera', 1)
ON DUPLICATE KEY UPDATE enabled = 1;

INSERT INTO camera_streams (camera_id, stream_type, rtsp_url)
VALUES ('e2e_test', 'main', 'rtsp://192.168.1.10:554/stream1')
ON DUPLICATE KEY UPDATE rtsp_url = VALUES(rtsp_url);
EOF

# 2. collector起動確認
curl -s http://localhost:8080/api/status | jq -e '.collector == "running"'

# 3. 60秒待機（フレーム収集）
echo "Waiting 60 seconds for frame collection..."
sleep 60

# 4. framesレコード確認
FRAME_COUNT=$(mysql -u camserver -p camserver -N -e \
  "SELECT COUNT(*) FROM frames WHERE camera_id = 'e2e_test'")
test $FRAME_COUNT -gt 0 || { echo "No frames collected"; exit 1; }

# 5. 推論実行確認
ANALYZED=$(mysql -u camserver -p camserver -N -e \
  "SELECT COUNT(*) FROM frames WHERE camera_id = 'e2e_test' AND analyzed = 1")
test $ANALYZED -gt 0 || { echo "No frames analyzed"; exit 1; }

# 6. イベント確認（検知があれば）
DETECTED=$(mysql -u camserver -p camserver -N -e \
  "SELECT COUNT(*) FROM frames WHERE camera_id = 'e2e_test' AND detected = 1")
if [ $DETECTED -gt 0 ]; then
  EVENT_COUNT=$(mysql -u camserver -p camserver -N -e \
    "SELECT COUNT(*) FROM events WHERE camera_id = 'e2e_test'")
  test $EVENT_COUNT -gt 0 || { echo "Detection but no event"; exit 1; }
fi

echo "=== E2E Test Passed ==="
```

### E2E-02: API→UI表示

```bash
#!/bin/bash
# test_e2e_api_ui.sh

set -e

echo "=== E2E Test: API to UI ==="

# 1. APIからカメラ取得
CAMERAS=$(curl -s http://localhost:8080/api/cameras | jq length)
test $CAMERAS -gt 0 || { echo "No cameras from API"; exit 1; }

# 2. 最新フレーム取得
FRAME=$(curl -s "http://localhost:8080/api/cameras/e2e_test" | jq -r '.latest_frame.frame_uuid')
test "$FRAME" != "null" || { echo "No latest frame"; exit 1; }

# 3. 署名付きURL取得
MEDIA_URL=$(curl -s "http://localhost:8080/api/media/sign/$FRAME" | jq -r '.url')

# 4. 画像取得
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:8080$MEDIA_URL")
test "$HTTP_CODE" = "200" || { echo "Media fetch failed: $HTTP_CODE"; exit 1; }

echo "=== E2E Test Passed ==="
```

---

## 6. 性能テスト

### PERF-01: 推論スループット

```bash
#!/bin/bash
# test_perf_inference.sh

echo "=== Performance Test: Inference Throughput ==="

# 100リクエスト、5並列
hey -n 100 -c 5 \
  -m POST \
  -F "image=@test_data/test_person.jpg" \
  http://localhost:8000/v1/analyze

# 期待:
# - Requests/sec > 5
# - 99% latency < 1s
```

### PERF-02: API応答時間

```bash
#!/bin/bash
# test_perf_api.sh

echo "=== Performance Test: API Response Time ==="

# /api/cameras
echo "--- /api/cameras ---"
hey -n 1000 -c 10 http://localhost:8080/api/cameras

# /api/events (検索)
echo "--- /api/events ---"
hey -n 1000 -c 10 "http://localhost:8080/api/events?limit=20"

# 期待:
# - Requests/sec > 100
# - 99% latency < 100ms
```

---

## 7. テスト結果記録

### テスト実行記録テンプレート

| 日付 | テストID | 結果 | 備考 |
|------|---------|------|------|
| 2025-MM-DD | UT-IS21-001-01 | PASS/FAIL | |
| 2025-MM-DD | UT-IS21-001-02 | PASS/FAIL | |
| ... | ... | ... | ... |

### Issue連携
- テスト失敗時は関連Issueにコメント追加
- 修正後は再テスト実施
- 全テストPASSでIssueクローズ

---

## 8. テスト自動化

### CI/CDパイプライン（将来）

```yaml
# .github/workflows/test.yml
name: Test

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test-rust:
    runs-on: ubuntu-latest
    services:
      mariadb:
        image: mariadb:10.11
        env:
          MYSQL_DATABASE: camserver_test
          MYSQL_USER: test
          MYSQL_PASSWORD: test
          MYSQL_ROOT_PASSWORD: root
        ports:
          - 3306:3306
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace

  test-python:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.10'
      - run: pip install -r requirements-test.txt
      - run: pytest is21/tests/
```
