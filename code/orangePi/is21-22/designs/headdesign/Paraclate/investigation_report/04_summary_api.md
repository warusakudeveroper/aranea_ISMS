# Summary/GrandSummary API要件

作成日: 2026-01-10

## 1. 概要

Paraclate機能の中核となるSummary/GrandSummary APIの要件を整理する。
DBスキーマは定義済み（migration 008）だが、APIエンドポイント・生成ロジックは未実装。

---

## 2. Summary種別

### 2.1 Summary（通常報告）

| 項目 | 値 |
|------|-----|
| 報告間隔 | 設定可能（デフォルト60分） |
| トリガー | 時間経過 |
| 内容 | 期間内の検出イベント要約 |

### 2.2 GrandSummary（統合報告）

| 項目 | 値 |
|------|-----|
| 報告時刻 | 設定可能（デフォルト09:00, 17:00, 21:00） |
| トリガー | 指定時刻到達 |
| 内容 | 複数Summaryの統合・1日の総括 |

### 2.3 Emergency（即時報告）

| 項目 | 値 |
|------|-----|
| トリガー | 異常検出時 |
| 条件 | severity >= 閾値、suspicious.level == critical等 |

---

## 3. データベーススキーマ（既存）

### 3.1 ai_summary_cache

```sql
CREATE TABLE ai_summary_cache (
    summary_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32) NOT NULL,
    summary_type ENUM('hourly', 'daily', 'emergency') NOT NULL,
    period_start DATETIME(3) NOT NULL,
    period_end DATETIME(3) NOT NULL,
    summary_text TEXT NOT NULL,
    detection_count INT NOT NULL DEFAULT 0,
    severity_max INT NOT NULL DEFAULT 0,
    camera_ids JSON NOT NULL,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    expires_at DATETIME(3) NOT NULL
);
```

### 3.2 追加が必要なスキーマ

```sql
-- scheduled_reports: 定時報告スケジュール管理
CREATE TABLE scheduled_reports (
    schedule_id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32) NOT NULL,
    report_type ENUM('summary', 'grand_summary') NOT NULL,
    interval_minutes INT,           -- summary用（60, 120等）
    scheduled_times JSON,           -- grand_summary用（["09:00", "17:00"]）
    last_run_at DATETIME(3),
    next_run_at DATETIME(3),
    enabled BOOLEAN DEFAULT TRUE,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3)
);
```

---

## 4. Summary送信形式（Paraclate設計書より）

### 4.1 summaryOverview

```json
{
  "summaryOverview": {
    "summaryID": "{summaryID}",
    "firstDetectAt": "{timestamp}",
    "fendDetectAt": "{timestamp}",
    "detectedEvents": "n"
  }
}
```

### 4.2 cameraContext

```json
{
  "cameraContext": {
    "{camera_lacisID1}": ["{cameraName}", "{cameraContext}", "{fid}", "{rid}", "preset"],
    "{camera_lacisID2}": ["{cameraName}", "{cameraContext}", "{fid}", "{rid}", "preset"]
  }
}
```

### 4.3 cameraDetection

```json
{
  "cameraDetection": [
    "{DetectionTimestamp},{camera_lacisID},{detectionDetail}",
    "{DetectionTimestamp},{camera_lacisID},{detectionDetail}"
  ]
}
```

### 4.4 完全なペイロード

```json
{
  "lacisOath": {
    "lacisID": "is22Device_lacisID",
    "tid": "is22Device_tid",
    "cic": "is22Device_cic",
    "blessing": "..."
  },
  "summaryOverview": { ... },
  "cameraContext": { ... },
  "cameraDetection": [ ... ]
}
```

---

## 5. 必要なAPIエンドポイント

### 5.1 Summary生成・取得

| エンドポイント | メソッド | 機能 |
|---------------|---------|------|
| POST /api/summary/generate | POST | Summary手動生成 |
| GET /api/summary/latest | GET | 最新Summary取得 |
| GET /api/summary/:id | GET | 特定Summary取得 |
| GET /api/summary/range | GET | 期間指定Summary一覧 |

### 5.2 GrandSummary

| エンドポイント | メソッド | 機能 |
|---------------|---------|------|
| GET /api/grand-summary/:date | GET | 日付指定GrandSummary |
| GET /api/grand-summary/latest | GET | 最新GrandSummary |

### 5.3 スケジュール管理

| エンドポイント | メソッド | 機能 |
|---------------|---------|------|
| GET /api/reports/schedule | GET | スケジュール取得 |
| PUT /api/reports/schedule | PUT | スケジュール更新 |

---

## 6. 生成ロジック

### 6.1 Summary生成フロー

```
1. 前回生成時刻から現在までのdetection_logsを取得
2. カメラごとにイベントを集計
3. severity_maxを算出
4. summary_textを生成（将来的にはLLM）
5. ai_summary_cacheに保存
6. (オプション) Paraclate APPへ送信
```

### 6.2 GrandSummary生成フロー

```
1. 指定期間のai_summary_cacheを取得
2. 複数Summaryを統合
3. 1日の特徴・傾向を抽出
4. grand_summary_textを生成
5. ai_summary_cache（type=daily）に保存
6. Paraclate APPへ送信
```

### 6.3 スケジューラー実装

```rust
// バックグラウンドタスク
async fn summary_scheduler(pool: Pool<MySql>) {
    loop {
        // 次回実行時刻までスリープ
        let next_run = get_next_scheduled_run(&pool).await;
        tokio::time::sleep_until(next_run).await;

        // Summary生成
        generate_summary(&pool).await;

        // 次回実行時刻更新
        update_next_run(&pool).await;
    }
}
```

---

## 7. LLM統合（将来）

### 7.1 現段階

- 単純なテンプレートベースの要約
- 検出数・severity・カメラ名の列挙

### 7.2 Paraclate APP連携後

- Paraclate APPにコンテキスト送信
- mobes側でFacilityContext + LLMで診断
- マルチステージLLM（Flash系/Pro系）

---

## 8. BigQuery同期

### 8.1 既存テーブル

```sql
CREATE TABLE bq_sync_queue (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    table_name VARCHAR(64) NOT NULL,
    record_id BIGINT UNSIGNED NOT NULL,
    operation ENUM('INSERT', 'UPDATE', 'DELETE') NOT NULL,
    payload JSON NOT NULL,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    synced_at DATETIME(3),
    sync_status ENUM('pending', 'syncing', 'completed', 'failed') DEFAULT 'pending'
);
```

### 8.2 Summary/BQ連携

Paraclate_DesignOverview.mdより:
> 最終的に検出ログの全てをBQにポストする(サマリー系を強化する方向)、
> この場合にイベントID,サマリーIDベースとなりParaclateにイベントを
> 送信する必要はなくなる

---

## 9. MECE確認

- **網羅性**: Summary/GrandSummary/Emergency、DB/API/ロジックを全調査
- **重複排除**: DB定義済み部分と未実装部分を明確に分離
- **未カバー領域**: LLM統合の詳細仕様はParaclate APP側の範囲
