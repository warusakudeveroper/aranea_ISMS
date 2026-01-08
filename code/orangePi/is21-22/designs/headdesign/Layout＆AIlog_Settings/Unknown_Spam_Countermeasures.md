# Unknown乱発問題の調査報告と対応策設計

## Issue Reference
- 関連Issue: #104 (AIEventLog Phase 4)
- 関連設計書: AIEventlog.md

## 1. 問題概要

### 1.1 現象
- **Tam-1F-Front**カメラが24時間で1,494件のunknownイベントを生成（約1件/分）
- 画像ストレージ: 20GB使用中、うちTam-1F-Frontだけで848MB（6,821画像）
- イベントログが「不明」イベントで埋まり、重要なhuman/vehicle検知が見えにくい

### 1.2 影響
- ストレージの急速な肥大化
- イベントログの視認性低下
- 本来注目すべきイベントの埋没

## 2. 根本原因分析

### 2.1 IS21の検出結果
```json
{
  "primary_event": "unknown",
  "unknown_flag": true,
  "bboxes": [
    {
      "label": "mouse",
      "conf": 0.518,
      "x1": 234, "y1": 156, "x2": 312, "y2": 198
    }
  ],
  "confidence": 0.518
}
```

**原因**: YOLOモデルが「mouse」（マウス/ネズミ）を約50%の信頼度で誤検出している。

### 2.2 プリセット設定の問題

**Tam-1F-Frontのカメラ設定:**
```sql
SELECT preset_id, conf_override FROM cameras WHERE camera_id = 'Tam-1F-Front';
-- preset_id: 'balanced', conf_override: NULL
```

**balanced プリセットの定義** (`preset_loader/mod.rs:84-102`):
```rust
pub fn balanced() -> Self {
    Self {
        id: "balanced".to_string(),
        expected_objects: vec!["human".to_string()],  // 人物のみ期待
        excluded_objects: vec![],                      // ★問題: 除外設定なし
        conf_override: None,                           // ★問題: 閾値オーバーライドなし
        // ...
    }
}
```

### 2.3 画像保存ロジック

**`detection_log_service/mod.rs`の`should_save_image`関数:**
```rust
pub fn should_save_image(primary_event: &str, severity: i32, unknown_flag: bool) -> bool {
    // BE-05: severity > 0 は常に保存
    if severity > 0 { return true; }
    // BE-02, BE-03: 「何もない」(none)は保存しない
    if primary_event == "none" { return false; }
    // BE-04: unknown判定は保存（後で手動90%削除）
    if unknown_flag || primary_event == "unknown" {
        return true;  // ★問題: unknownは全て保存される
    }
    true
}
```

### 2.4 問題のフローチャート

```
[IS21 YOLO検出]
      ↓
label="mouse", conf=0.52
      ↓
[IS22 primary_event判定]
"mouse" ∉ expected_objects ("human") → unknown_flag=true
      ↓
[should_save_image]
unknown_flag=true → 保存=true
      ↓
[画像保存] → ストレージ肥大化
[detection_logs INSERT] → ログ乱発
```

## 3. 対応策設計

### 3.1 短期対応（即時適用可能）

#### 3.1.1 balanced プリセットへのexcluded_objects追加

**変更対象**: `src/preset_loader/mod.rs`

```rust
pub fn balanced() -> Self {
    Self {
        id: "balanced".to_string(),
        // ...
        excluded_objects: vec![
            // 監視カメラで検知不要な一般的オブジェクト
            "mouse".to_string(),
            "keyboard".to_string(),
            "cell phone".to_string(),
            "laptop".to_string(),
            "tv".to_string(),
            "book".to_string(),
            "clock".to_string(),
            "vase".to_string(),
            "potted plant".to_string(),
            "teddy bear".to_string(),
            "remote".to_string(),
        ],
        // ...
    }
}
```

#### 3.1.2 カメラ別conf_overrideの設定UI実装

Settings Modal → AI推論タブに閾値スライダーを追加:
- 最小: 0.2 (person_priorityレベル - 検出漏れを防ぐ)
- デフォルト: 0.5 (balanced)
- 最大: 0.8 (厳格 - 誤検知を極力防ぐ)

**APIエンドポイント**: `/api/cameras/:id/thresholds` (既存、Phase 4で実装済み)

### 3.2 中期対応（Phase 4追加実装）

#### 3.2.1 unknown保存の制御オプション

Settings Modal → 保存タブに追加:
- `unknown_save_mode`: "all" | "high_conf" | "none"
  - "all": 現行動作（全てのunknownを保存）
  - "high_conf": conf >= 0.6 のunknownのみ保存（推奨）
  - "none": unknownは保存しない

```rust
pub fn should_save_image(
    primary_event: &str,
    severity: i32,
    unknown_flag: bool,
    confidence: f32,
    unknown_save_mode: &str,  // 追加パラメータ
) -> bool {
    if severity > 0 { return true; }
    if primary_event == "none" { return false; }

    if unknown_flag || primary_event == "unknown" {
        match unknown_save_mode {
            "none" => return false,
            "high_conf" => return confidence >= 0.6,
            _ => return true,  // "all"
        }
    }
    true
}
```

#### 3.2.2 ストレージクォータ設定

Settings Modal → 保存タブに追加:
- `max_storage_gb`: 最大保存容量 (デフォルト: 50GB)
- `retention_days`: 保存日数 (デフォルト: 30日)
- `cleanup_policy`: "oldest_first" | "low_priority_first"

**cleanup_policy説明:**
- "oldest_first": 古い画像から削除
- "low_priority_first": unknown/低severity画像を優先削除

#### 3.2.3 IS21検出オブジェクトの除外フィルタ

IS22側でIS21レスポンスをフィルタリング:

```rust
// polling_orchestrator内でIS21レスポンス受信後
fn filter_irrelevant_detections(
    response: &mut AnalyzeResponse,
    excluded_labels: &[String],
) {
    // 除外ラベルに該当するbboxを削除
    response.bboxes.retain(|bbox| {
        !excluded_labels.contains(&bbox.label.to_lowercase())
    });

    // bboxが空になったらunknownではなくnoneに変更
    if response.bboxes.is_empty() && response.primary_event == "unknown" {
        response.primary_event = "none".to_string();
        response.unknown_flag = false;
        response.detected = false;
    }
}
```

### 3.3 長期対応（autoAttunement機能）

#### 3.3.1 推論統計の収集

```sql
-- 推論統計テーブル (Phase 5で実装予定)
CREATE TABLE inference_statistics (
    stat_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL,
    date DATE NOT NULL,
    hour TINYINT NOT NULL,  -- 0-23

    -- カウンター
    total_inferences INT NOT NULL DEFAULT 0,
    human_detections INT NOT NULL DEFAULT 0,
    vehicle_detections INT NOT NULL DEFAULT 0,
    unknown_detections INT NOT NULL DEFAULT 0,
    none_detections INT NOT NULL DEFAULT 0,

    -- 信頼度統計
    avg_confidence DECIMAL(5,4) NULL,
    min_confidence DECIMAL(5,4) NULL,
    max_confidence DECIMAL(5,4) NULL,

    -- 誤検知統計
    false_positive_count INT NOT NULL DEFAULT 0,

    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    UNIQUE KEY uk_camera_date_hour (camera_id, date, hour),
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE
);
```

#### 3.3.2 autoAttunement自動閾値調整

```rust
// TODO: Phase 5で実装
// 誤検知報告と統計から最適閾値を自動計算
async fn calculate_optimal_threshold(
    camera_id: &str,
    stats: &InferenceStatistics,
    feedback: &[MisdetectionFeedback],
) -> f32 {
    // 誤検知率を計算
    let false_positive_rate = feedback.len() as f32 / stats.total_inferences as f32;

    // 現在の閾値
    let current_threshold = get_camera_threshold(camera_id).await;

    // 誤検知率が高い場合は閾値を上げる
    if false_positive_rate > 0.1 {
        (current_threshold + 0.05).min(0.8)
    } else if false_positive_rate < 0.01 {
        // 誤検知率が非常に低い場合は閾値を下げられる可能性
        (current_threshold - 0.02).max(0.2)
    } else {
        current_threshold
    }
}
```

## 4. 実装優先順位

| 優先度 | 対応項目 | 工数 | 効果 |
|--------|----------|------|------|
| P0 | balanced プリセットにexcluded_objects追加 | 0.5h | 高 |
| P0 | Tam-1F-Frontのconf_override設定 (0.6) | 即時 | 高 |
| P1 | unknown_save_mode設定UI | 2h | 高 |
| P1 | ストレージクォータ設定 | 4h | 中 |
| P2 | 推論統計ダッシュボード | 8h | 中 |
| P3 | autoAttunement自動調整 | 16h | 高 |

## 5. 即時対応手順

### 5.1 Tam-1F-Frontへの即時対応

```sql
-- camera_thresholdsに閾値を設定
INSERT INTO camera_thresholds (camera_id, conf_threshold, auto_adjust_enabled)
VALUES ('Tam-1F-Front', 0.60, FALSE)
ON DUPLICATE KEY UPDATE
    conf_threshold = 0.60;

-- 履歴に記録
INSERT INTO threshold_change_history (camera_id, old_threshold, new_threshold, change_reason)
VALUES ('Tam-1F-Front', 0.50, 0.60, 'manual');
```

### 5.2 古いunknown画像のクリーンアップ

```bash
# Tam-1F-Frontの古い画像を削除（7日以上前）
find /var/lib/is22/events/Tam-1F-Front -type f -mtime +7 -name "*.jpg" -delete

# ディスク使用量確認
du -sh /var/lib/is22/events/*/
```

## 6. 検証方法

### 6.1 対応後のモニタリング

```sql
-- unknown発生率の推移確認（1時間ごと）
SELECT
    DATE(created_at) as date,
    HOUR(created_at) as hour,
    COUNT(*) as unknown_count
FROM detection_logs
WHERE camera_id = 'Tam-1F-Front'
  AND primary_event = 'unknown'
  AND created_at >= NOW() - INTERVAL 48 HOUR
GROUP BY DATE(created_at), HOUR(created_at)
ORDER BY date, hour;
```

### 6.2 成功基準

- unknown発生率: 1件/分 → 1件/時間以下（95%削減）
- ストレージ増加率: 848MB/日 → 100MB/日以下
- 重要イベント（human/vehicle）の視認性向上

## 7. 関連ドキュメント

- AIEventlog.md - 元の指摘事項
- is22_AI_EVENT_LOG_DESIGN.md - AI Event Log設計書
- migrations/019_phase4_feedback_threshold.sql - 閾値テーブル定義

---
作成日: 2026-01-08
作成者: Claude Code (Issue #104対応)
