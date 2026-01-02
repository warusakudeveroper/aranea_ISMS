# AI Event Log テスト計画書

## 改訂履歴
| 日付 | バージョン | 変更内容 |
|------|----------|----------|
| 2026-01-02 | 1.0.0 | 初版作成 |

---

## 1. テスト概要

### 1.1 目的
AI Event Log機能の品質保証のため、以下のテストを体系的に実施する：
- ユニットテスト（自動）
- 統合テスト（自動＋手動）
- UI操作テスト（手動＋E2E自動）
- 長期放置テスト（自動監視）
- 負荷テスト（自動）

### 1.2 テスト環境

| 環境 | 用途 | 構成 |
|------|------|------|
| DEV | 開発・ユニットテスト | ローカルDocker |
| STAGING | 統合・E2Eテスト | is22(実機) + is21(実機) |
| PROD | 本番 | 同上 |

---

## 2. ユニットテスト

### 2.1 テスト対象モジュール

| モジュール | ファイル | テスト項目数 |
|-----------|----------|-------------|
| AiClient | ai_client/mod.rs | 12 |
| DetectionLogService | detection_log/mod.rs | 15 |
| BqSyncService | bq_sync/mod.rs | 10 |
| SnapshotService | snapshot_service/mod.rs | 8 |
| PresetLoader | preset/mod.rs | 6 |

### 2.2 AiClient テストケース

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // 正常系
    #[tokio::test]
    async fn test_analyze_success_human_detection() {
        let client = create_mock_client();
        let req = AnalyzeRequest {
            camera_id: "cam-test-001".to_string(),
            captured_at: Utc::now(),
            image_data: load_test_image("human.jpg"),
            camera_context: None,
            preset_id: "balanced".to_string(),
            preset_version: "1.0.0".to_string(),
        };
        let result = client.analyze(req).await.unwrap();
        assert_eq!(result.primary_event, "human");
        assert!(result.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_analyze_success_vehicle_detection() { /* ... */ }

    #[tokio::test]
    async fn test_analyze_success_none_detection() { /* ... */ }

    #[tokio::test]
    async fn test_analyze_with_camera_context() { /* ... */ }

    #[tokio::test]
    async fn test_analyze_with_preset_parking() { /* ... */ }

    // 異常系
    #[tokio::test]
    async fn test_analyze_timeout() {
        let client = create_slow_mock_client(Duration::from_secs(35));
        let result = client.analyze(default_request()).await;
        assert!(matches!(result, Err(AiError::Timeout)));
    }

    #[tokio::test]
    async fn test_analyze_connection_refused() { /* ... */ }

    #[tokio::test]
    async fn test_analyze_invalid_image() { /* ... */ }

    #[tokio::test]
    async fn test_analyze_is21_500_error() { /* ... */ }

    // リトライ
    #[tokio::test]
    async fn test_analyze_retry_success_on_second_attempt() { /* ... */ }

    #[tokio::test]
    async fn test_analyze_retry_exhausted() { /* ... */ }
}
```

### 2.3 DetectionLogService テストケース

```rust
#[cfg(test)]
mod tests {
    // 保存テスト
    #[tokio::test]
    async fn test_save_detection_success() { /* ... */ }

    #[tokio::test]
    async fn test_save_detection_with_bq_queue() {
        let service = create_test_service().await;
        let response = create_human_response();
        let meta = create_test_meta();

        let log_id = service.save_detection(&response, &meta).await.unwrap();

        // BQキュー登録確認
        let queue_entry = service.db.query_one(
            "SELECT * FROM bq_sync_queue WHERE record_id = ?",
            log_id
        ).await.unwrap();
        assert_eq!(queue_entry.status, "pending");
    }

    #[tokio::test]
    async fn test_save_detection_none_event_skipped() {
        let service = create_test_service().await;
        let response = AnalyzeResponse {
            primary_event: "none".to_string(),
            detected: false,
            ..default_response()
        };

        let result = service.save_detection(&response, &meta).await;
        assert!(matches!(result, Err(DetectionError::NoneEventSkipped)));
    }

    #[tokio::test]
    async fn test_save_detection_preset_info_stored() { /* ... */ }

    // 検索テスト
    #[tokio::test]
    async fn test_get_recent_logs() { /* ... */ }

    #[tokio::test]
    async fn test_search_logs_by_camera() { /* ... */ }

    #[tokio::test]
    async fn test_search_logs_by_severity() { /* ... */ }

    #[tokio::test]
    async fn test_search_logs_by_time_range() { /* ... */ }

    #[tokio::test]
    async fn test_search_logs_by_preset() { /* ... */ }

    // 異常系
    #[tokio::test]
    async fn test_save_detection_db_connection_error() { /* ... */ }

    #[tokio::test]
    async fn test_save_detection_duplicate_handling() { /* ... */ }
}
```

### 2.4 モック戦略

| 対象 | モック方法 | 備考 |
|------|-----------|------|
| is21 API | wiremock-rs | レスポンスJSONをテストファイルから読み込み |
| MySQL | testcontainers | 実DBで動作確認 |
| BigQuery | モックstruct | API呼び出しのみ検証 |
| 画像ファイル | fixtures/ | テスト用画像セット |

### 2.5 テストデータセット

```
tests/fixtures/
├── images/
│   ├── human_single.jpg       # 人物1名
│   ├── human_multiple.jpg     # 人物複数
│   ├── human_suspicious.jpg   # 不審姿勢
│   ├── vehicle_sedan.jpg      # セダン
│   ├── vehicle_plate.jpg      # ナンバー付き
│   ├── animal_cat.jpg         # 猫
│   ├── empty.jpg              # 空（検知なし）
│   └── error_corrupt.jpg      # 破損画像
├── responses/
│   ├── human_detected.json
│   ├── vehicle_detected.json
│   ├── none_detected.json
│   └── error_500.json
└── contexts/
    ├── entrance.json
    ├── parking.json
    └── restricted.json
```

### 2.6 CI/CD統合

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    services:
      mysql:
        image: mysql:8.0
        env:
          MYSQL_DATABASE: test_db
          MYSQL_ROOT_PASSWORD: test
        ports:
          - 3306:3306
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: cargo test --all-features
        env:
          DATABASE_URL: mysql://root:test@localhost:3306/test_db
```

---

## 3. 統合テスト

### 3.1 is21 → is22 E2Eシナリオ

| ID | シナリオ | 事前条件 | 手順 | 期待結果 |
|----|----------|----------|------|----------|
| E2E-001 | 人物検知フロー完走 | カメラ登録済み | 1. カメラ前に人物配置<br>2. ポーリング実行<br>3. is21解析<br>4. DB保存確認<br>5. WS通知確認 | detection_logsにレコード作成、UI更新 |
| E2E-002 | 車両検知+OCR | parkingプリセット | 1. 車両画像投入<br>2. ナンバー認識確認 | plate_number取得 |
| E2E-003 | 検知なし処理 | - | 1. 空画像送信<br>2. DB未保存確認 | detection_logs無し、WS通知無し |
| E2E-004 | プリセット切替 | balanced→restricted | 1. プリセット変更<br>2. 再解析<br>3. preset_id確認 | preset_id="restricted"で保存 |
| E2E-005 | BQ同期 | BQ接続設定済み | 1. 検知ログ保存<br>2. 60秒待機<br>3. BQ確認 | BQにレコード存在 |

### 3.2 テスト環境構成

```
STAGING環境:
┌─────────────────┐     ┌─────────────────┐
│ is22 (OrangePi) │────▶│ is21 (OrangePi) │
│ 192.168.125.246 │     │ 192.168.3.240   │
└─────────────────┘     └─────────────────┘
        │
        ▼
┌─────────────────┐     ┌─────────────────┐
│ MySQL (is22)    │     │ BigQuery (GCP)  │
│ 192.168.125.246 │────▶│ staging dataset │
└─────────────────┘     └─────────────────┘
```

### 3.3 テストデータ投入手順

```bash
# 1. テストカメラ登録
curl -X POST http://192.168.125.246:8080/api/cameras \
  -H "Content-Type: application/json" \
  -d '{
    "display_name": "E2E Test Camera",
    "ip": "192.168.125.100",
    "preset_id": "balanced",
    "tid": "T_TEST",
    "fid": "F_TEST"
  }'

# 2. テスト画像でポーリング実行
curl -X POST http://192.168.125.246:8080/api/debug/trigger_poll \
  -H "Content-Type: application/json" \
  -d '{"camera_id": "cam-xxx", "image_path": "/tmp/test_human.jpg"}'

# 3. 結果確認
curl http://192.168.125.246:8080/api/detection-logs?limit=1
```

### 3.4 結果検証方法

```rust
// E2Eテストフレームワーク
async fn verify_detection_flow(test_image: &Path, expected: Expected) {
    // 1. ポーリング実行
    let poll_result = trigger_poll(test_image).await;
    assert!(poll_result.is_ok());

    // 2. DB確認（リトライ付き）
    let log = retry(5, Duration::from_secs(1), || {
        get_latest_detection_log()
    }).await.unwrap();

    // 3. 検証
    assert_eq!(log.primary_event, expected.primary_event);
    assert!(log.confidence >= expected.min_confidence);
    assert_eq!(log.preset_id, expected.preset_id);

    // 4. WebSocket通知確認
    let ws_msg = ws_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(ws_msg.log_id, log.log_id);
}
```

---

## 4. UI操作テスト

### 4.1 CameraDetailModal テスト

| ID | テスト項目 | 手順 | 期待結果 |
|----|-----------|------|----------|
| UI-CAM-001 | プリセット選択 | 1. モーダル開く<br>2. プリセットドロップダウン開く<br>3. "parking"選択 | 説明・用途・傾向が表示される |
| UI-CAM-002 | プリセット変更保存 | 1. プリセット変更<br>2. 保存ボタン押下 | DBのpreset_id更新、トースト表示 |
| UI-CAM-003 | camera_context編集 | 1. location_type変更<br>2. expected_objects追加<br>3. 保存 | camera_context JSON更新 |
| UI-CAM-004 | バリデーションエラー | 1. 必須項目空欄<br>2. 保存試行 | エラーメッセージ表示、保存されない |
| UI-CAM-005 | キャンセル | 1. 値変更<br>2. キャンセル押下 | 変更破棄、モーダル閉じる |

### 4.2 EventLogPane テスト

| ID | テスト項目 | 手順 | 期待結果 |
|----|-----------|------|----------|
| UI-EVT-001 | ログ一覧表示 | 1. ページ読み込み | 最新100件表示 |
| UI-EVT-002 | フィルタ適用 | 1. severity=3選択<br>2. 適用 | severity=3のみ表示 |
| UI-EVT-003 | ログクリック | 1. ログ行クリック | 該当カメラハイライト、スナップショット表示 |
| UI-EVT-004 | 詳細展開 | 1. 展開アイコンクリック | bboxes, person_details表示 |
| UI-EVT-005 | スクロール追加読み込み | 1. 下端までスクロール | 次の100件読み込み |
| UI-EVT-006 | リアルタイム更新 | 1. 新規検知発生 | 自動で先頭に追加、スクロール |

### 4.3 CameraGrid連携テスト

| ID | テスト項目 | 手順 | 期待結果 |
|----|-----------|------|----------|
| UI-GRID-001 | EventLog→カメラ選択 | 1. EventLogクリック | CameraGrid該当タイルハイライト |
| UI-GRID-002 | bbox overlay | 1. ログ詳細展開<br>2. スナップショット表示 | 検出ボックスがオーバーレイ表示 |
| UI-GRID-003 | カメラ切替 | 1. 別カメラタイルクリック | EventLogフィルタ更新（オプション） |

### 4.4 E2E自動テスト（Playwright）

```typescript
// tests/e2e/event-log.spec.ts
import { test, expect } from '@playwright/test';

test.describe('EventLogPane', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://192.168.125.246:3000');
    // WebSocket接続待ち
    await expect(page.locator('[data-testid="ws-status"]')).toHaveText('🟢');
  });

  test('displays detection logs', async ({ page }) => {
    const logs = page.locator('[data-testid="event-log-item"]');
    await expect(logs).toHaveCount.greaterThan(0);
  });

  test('filter by primary_event', async ({ page }) => {
    await page.click('[data-testid="filter-human"]');
    const logs = page.locator('[data-testid="event-log-item"]');
    for (const log of await logs.all()) {
      await expect(log).toContainText('🧍');
    }
  });

  test('click log selects camera', async ({ page }) => {
    await page.click('[data-testid="event-log-item"]:first-child');
    const selectedCamera = page.locator('[data-testid="camera-tile"].selected');
    await expect(selectedCamera).toBeVisible();
  });

  test('realtime update on new detection', async ({ page }) => {
    const initialCount = await page.locator('[data-testid="event-log-item"]').count();

    // 新規検知をトリガー
    await fetch('http://192.168.125.246:8080/api/debug/trigger_poll', {
      method: 'POST',
      body: JSON.stringify({ camera_id: 'cam-test', image_path: '/tmp/human.jpg' })
    });

    // 自動更新待ち
    await expect(page.locator('[data-testid="event-log-item"]'))
      .toHaveCount(initialCount + 1, { timeout: 10000 });
  });
});
```

---

## 5. 長期放置テスト

### 5.1 テストシナリオ

| 項目 | 値 |
|------|-----|
| 期間 | 72時間連続稼働 |
| カメラ数 | 20台 |
| ポーリング間隔 | 5秒 |
| 想定解析回数 | 1,036,800回 |
| 想定検知ログ | 約100,000件（検知率10%想定） |

### 5.2 KPI指標

| KPI | 測定方法 | 目標値 | 警告閾値 | 緊急閾値 |
|-----|---------|--------|---------|---------|
| is21応答時間 P95 | processing_ms.total | < 500ms | > 800ms | > 2000ms |
| is21可用性 | 成功率 | > 99.9% | < 99.5% | < 99% |
| DB書き込み成功率 | INSERT成功/試行 | > 99.99% | < 99.9% | < 99% |
| BQ同期遅延 | captured_at - synced_at | < 5分 | > 10分 | > 30分 |
| WebSocket配信遅延 | analyzed_at - UI表示 | < 1秒 | > 3秒 | > 10秒 |
| メモリ使用量 | RSS | < 2GB | > 1.5GB | > 1.8GB |
| ディスク使用量 | /var/lib/is22 | < 80% | > 70% | > 85% |
| CPU使用率 | is22プロセス | < 70% | > 60% | > 80% |

### 5.3 監視スクリプト

```bash
#!/bin/bash
# long_term_monitor.sh

LOG_FILE="/var/log/is22_long_term_test.log"
PROMETHEUS_PUSHGATEWAY="http://localhost:9091"

while true; do
    timestamp=$(date +%Y-%m-%dT%H:%M:%S)

    # メモリ使用量
    mem_rss=$(ps -o rss= -p $(pgrep camserver) | awk '{print $1/1024}')

    # ディスク使用量
    disk_usage=$(df /var/lib/is22 | tail -1 | awk '{print $5}' | tr -d '%')

    # CPU使用率
    cpu_usage=$(top -bn1 | grep camserver | awk '{print $9}')

    # is21応答時間（直近1分の平均）
    is21_latency=$(mysql -N -e "SELECT AVG(processing_ms) FROM detection_logs WHERE created_at > NOW() - INTERVAL 1 MINUTE")

    # ログ出力
    echo "$timestamp mem=$mem_rss MB disk=$disk_usage% cpu=$cpu_usage% is21_latency=$is21_latency ms" >> $LOG_FILE

    # Prometheus Push
    cat <<EOF | curl --data-binary @- $PROMETHEUS_PUSHGATEWAY/metrics/job/is22_long_term
# TYPE is22_memory_rss_mb gauge
is22_memory_rss_mb $mem_rss
# TYPE is22_disk_usage_percent gauge
is22_disk_usage_percent $disk_usage
# TYPE is22_cpu_percent gauge
is22_cpu_percent $cpu_usage
# TYPE is22_is21_latency_ms gauge
is22_is21_latency_ms $is21_latency
EOF

    sleep 60
done
```

### 5.4 アラート設定

```yaml
# prometheus/rules/is22_alerts.yml
groups:
  - name: is22_long_term
    rules:
      - alert: Is22MemoryHigh
        expr: is22_memory_rss_mb > 1500
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "IS22メモリ使用量が高い"

      - alert: Is22MemoryCritical
        expr: is22_memory_rss_mb > 1800
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "IS22メモリ使用量が危険域"

      - alert: Is21LatencyHigh
        expr: is22_is21_latency_ms > 800
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "is21応答遅延"
```

### 5.5 メモリリーク検出

```rust
// 1時間ごとにメモリ使用量を記録、線形回帰で増加傾向を検出
fn check_memory_leak(history: &[MemorySnapshot]) -> LeakStatus {
    if history.len() < 24 { // 24時間分未満
        return LeakStatus::InsufficientData;
    }

    let slope = linear_regression_slope(history);

    // 1時間あたり10MB以上増加は要注意
    if slope > 10.0 {
        LeakStatus::Suspected(slope)
    } else {
        LeakStatus::Normal
    }
}
```

---

## 6. 負荷テスト

### 6.1 シナリオ

| シナリオ | カメラ数 | ポーリング間隔 | 期待スループット |
|----------|---------|---------------|-----------------|
| Light | 10台 | 5秒 | 2 req/s |
| Normal | 20台 | 5秒 | 4 req/s |
| Heavy | 30台 | 5秒 | 6 req/s |
| Burst | 30台 | 1秒 | 30 req/s（短時間） |

### 6.2 測定メトリクス

| メトリクス | 測定方法 | Normal目標 | Heavy目標 |
|-----------|---------|-----------|-----------|
| スループット | 解析完了数/秒 | 4 req/s | 6 req/s |
| レイテンシ P50 | processing_ms | < 300ms | < 400ms |
| レイテンシ P95 | processing_ms | < 500ms | < 800ms |
| エラー率 | 失敗数/全試行 | < 0.1% | < 1% |
| CPU使用率 | is22プロセス | < 50% | < 70% |
| is21 CPU | is21プロセス | < 80% | < 90% |

### 6.3 負荷生成スクリプト

```rust
// load_test.rs
use tokio::time::{interval, Duration};

async fn run_load_test(config: LoadTestConfig) {
    let cameras: Vec<Camera> = config.cameras;
    let interval_ms = config.interval_ms;

    let mut handles = vec![];

    for camera in cameras {
        let handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(interval_ms));
            let mut stats = Stats::new();

            loop {
                ticker.tick().await;
                let start = Instant::now();

                match poll_camera(&camera).await {
                    Ok(result) => {
                        stats.record_success(start.elapsed());
                    }
                    Err(e) => {
                        stats.record_failure(e);
                    }
                }
            }
        });
        handles.push(handle);
    }

    // 定期レポート
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            print_stats_report().await;
        }
    });
}
```

### 6.4 ボトルネック特定手順

1. **CPU bound**: is21のNPU使用率確認
2. **I/O bound**: ディスクI/O待ち時間確認
3. **Network bound**: is22→is21レイテンシ確認
4. **Memory bound**: GC頻度・メモリ使用量確認

---

## 7. 異常系テスト

### 7.1 テストケース

| ID | 異常条件 | 手順 | 期待動作 |
|----|---------|------|----------|
| ERR-001 | is21ダウン | 1. is21停止<br>2. ポーリング実行 | リトライ3回後スキップ、エラーログ記録 |
| ERR-002 | is21応答遅延 | 1. is21に遅延注入<br>2. タイムアウト確認 | 30秒でタイムアウト、次回リトライ |
| ERR-003 | DB接続失敗 | 1. MySQL停止<br>2. 検知ログ保存試行 | エラーログ記録、メモリキュー待機 |
| ERR-004 | ディスクフル | 1. ディスク100%<br>2. 画像保存試行 | エラーログ、古い画像自動削除 |
| ERR-005 | メモリ枯渇 | 1. 大量リクエスト投入 | OOMKill前に自己制御、ログ保存 |
| ERR-006 | ネットワーク断 | 1. VPN切断<br>2. 復旧後確認 | 自動再接続、キュー再処理 |

### 7.2 Chaos Engineering

```yaml
# chaos/is21_failure.yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: NetworkChaos
metadata:
  name: is21-network-delay
spec:
  action: delay
  mode: all
  selector:
    namespaces:
      - is22
  delay:
    latency: "5s"
    jitter: "1s"
  duration: "5m"
```

---

## 8. テスト実施スケジュール

| フェーズ | 期間 | テスト種別 | 担当 |
|---------|------|-----------|------|
| Phase 1 | 実装中 | ユニットテスト | 開発者 |
| Phase 2 | 実装完了後 | 統合テスト | 開発者 |
| Phase 3 | UI完成後 | UI操作テスト | QA |
| Phase 4 | リリース前 | 長期放置・負荷テスト | QA |
| Phase 5 | リリース前 | 異常系テスト | 開発者+QA |

---

## 9. テスト完了基準

### 9.1 必須条件
- [ ] ユニットテストカバレッジ 80%以上
- [ ] 統合テスト全件パス
- [ ] UI操作テスト全件パス
- [ ] 長期放置テスト（72時間）でKPI目標達成
- [ ] 負荷テスト（Normal）で目標スループット達成
- [ ] 異常系テストで想定通りのエラーハンドリング確認

### 9.2 推奨条件
- [ ] 負荷テスト（Heavy）で目標達成
- [ ] メモリリーク検出なし（72時間）
- [ ] E2E自動テスト全件パス
