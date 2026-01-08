# 推論統計・分析機能 詳細設計書

## Issue Reference
- 関連要件: AIEventlog.md 41-47行目
- Issue: #105 (新規作成予定)

## 1. 要件定義 (AIEventlog.mdより)

| 行 | 要件 | 優先度 |
|----|------|--------|
| 41 | 推論統計タブ: カメラ別・判定結果の分布と件数、傾向分析 | P0 |
| 42 | プリセット効果分析: プリセット変更の効果をエビデンスで確認可能に | P0 |
| 38 | ストレージクォータ: 最大保存容量設定 | P1 |
| 46-47 | autoAttunement: LLMコンテキスト連携による自動閾値調整 | P2 |

## 2. IS21ロジック検証結果

### 2.1 IS21コード解析 (2026-01-08実施)

**ファイル**: `/opt/is21/src/main.py`

| 項目 | 値 | 評価 |
|------|-----|------|
| CONF_THRESHOLD | 0.33 | 低め（検出漏れ防止優先） |
| NMS_THRESHOLD | 0.40 | 標準 |
| IGNORE_OBJECTS | 30オブジェクト | "mouse"は含まれていない → unknown発生原因 |
| PAR_MAX_PERSONS | 10 | 十分 |
| FIRMWARE_VERSION | 1.8.0 | camera_context対応済み |

### 2.2 primary_event決定ロジック

```python
# IS21 main.py L1100-1110
EVENT_MAP = {
    "person": "human",
    "bicycle": "vehicle", "car": "vehicle", ...
    "bird": "animal", "cat": "animal", "dog": "animal", ...
}

# EVENT_MAPに存在しないCOCOクラス → "unknown"
primary_event = EVENT_MAP.get(top_bbox["label"], "unknown")
```

**問題点**: "mouse"はCOCO_CLASSES[64]だがEVENT_MAPにないため`unknown`になる

### 2.3 camera_context処理確認

```python
# IS21 main.py L590-620
def apply_context_filters(bboxes, context):
    excluded = context.get("excluded_objects", [])
    expected = context.get("expected_objects", [])

    for bbox in bboxes:
        label = bbox.get("label", "")
        # excluded_objectsチェック → フィルタ
        if label in excluded:
            continue
        # expected_objectsチェック (conf < 0.5の場合のみ)
        if expected and label not in expected:
            if bbox.get("conf", 0) < 0.5:
                continue
```

**結論**: IS21はexcluded_objectsを正しく処理する。問題はIS22が送信していなかったこと。
→ 修正済み（balanced preset v1.1.0）

### 2.4 frame_diff処理確認

```python
# IS21 frame_diff.py
def calculate_frame_diff(current_image, prev_image):
    # モーション検知: 差分 > MOTION_AREA_THRESHOLD (0.5%)
    # カメラ状態判定: frozen_candidate, shifted, tampered, normal
```

**結論**: frame_diffは正常に動作。prev_imageをIS22が送信すれば機能する。

## 3. アーキテクチャ設計

### 3.1 データフロー (MECE)

```
┌─────────────────────────────────────────────────────────────────┐
│                        データソース                              │
├─────────────────────────────────────────────────────────────────┤
│  detection_logs (SSoT)                                          │
│  - primary_event, confidence, camera_id, preset_id              │
│  - created_at, bboxes, severity                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    統計集計レイヤー (Backend)                    │
├─────────────────────────────────────────────────────────────────┤
│  InferenceStatsService                                          │
│  - get_camera_distribution()     // カメラ別分布                 │
│  - get_event_trends()            // 時系列傾向                   │
│  - get_preset_effectiveness()    // プリセット効果               │
│  - get_storage_usage()           // ストレージ使用量             │
│  - get_anomaly_cameras()         // 異常カメラ検出               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    API レイヤー                                  │
├─────────────────────────────────────────────────────────────────┤
│  GET /api/stats/cameras          // カメラ別統計                 │
│  GET /api/stats/events           // イベント別統計               │
│  GET /api/stats/presets          // プリセット効果               │
│  GET /api/stats/storage          // ストレージ状況               │
│  GET /api/stats/anomalies        // 異常検出                     │
│  POST /api/settings/storage      // ストレージ設定保存           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Frontend (Settings Modal)                     │
├─────────────────────────────────────────────────────────────────┤
│  推論統計タブ (InferenceStatsTab.tsx)                            │
│  - CameraDistributionChart       // カメラ別分布グラフ           │
│  - EventTrendsChart              // 時系列傾向グラフ             │
│  - PresetEffectivenessTable      // プリセット効果テーブル       │
│  - AnomalyCamerasList            // 異常カメラリスト             │
│  - StorageQuotaSettings          // ストレージ設定               │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 DBスキーマ (既存テーブル活用)

```sql
-- SSoT: detection_logs (既存)
-- 統計クエリで直接集計、新規テーブル不要

-- ストレージ設定用 (facility_settingsに追加)
-- 019_phase4_feedback_threshold.sqlに追記
ALTER TABLE facility_settings ADD COLUMN IF NOT EXISTS
    storage_quota_gb INT DEFAULT 50 COMMENT '最大保存容量(GB)',
    retention_days INT DEFAULT 30 COMMENT '保存日数',
    cleanup_policy ENUM('oldest_first', 'low_priority_first') DEFAULT 'oldest_first'
    COMMENT 'クリーンアップポリシー';
```

## 4. 詳細設計 (MECE・アンアンビギュアス)

### 4.1 統計API仕様

#### GET /api/stats/cameras

**Request**:
```
?period=24h|7d|30d (default: 24h)
&camera_id=<optional>
```

**Response**:
```json
{
  "period": "24h",
  "total_inferences": 12500,
  "cameras": [
    {
      "camera_id": "cam-xxx",
      "camera_name": "Tam-1F-Front",
      "preset_id": "balanced",
      "total": 1500,
      "by_event": {
        "none": 1400,
        "human": 50,
        "unknown": 48,
        "vehicle": 2
      },
      "avg_confidence": 0.51,
      "anomaly_score": 3.2,  // 標準偏差ベース
      "anomaly_reason": "unknown発生率が平均の3.2倍"
    }
  ]
}
```

#### GET /api/stats/presets

**Response**:
```json
{
  "period": "7d",
  "presets": [
    {
      "preset_id": "balanced",
      "camera_count": 15,
      "total_inferences": 8000,
      "detection_rate": 0.05,  // none以外の割合
      "false_positive_rate": 0.02,  // フィードバックベース
      "avg_confidence": 0.52,
      "comparison": {
        "vs_person_priority": {
          "detection_rate_diff": -0.02,
          "false_positive_diff": -0.01
        }
      }
    }
  ]
}
```

### 4.2 Frontend コンポーネント設計

#### InferenceStatsTab.tsx

```tsx
// 主要セクション (MECE)
1. サマリーカード
   - 総推論回数 / 検出率 / 平均処理時間

2. カメラ別分布 (横棒グラフ)
   - X軸: 件数
   - Y軸: カメラ名
   - 色分け: primary_event別

3. 時系列傾向 (折れ線グラフ)
   - X軸: 時間 (1時間ごと)
   - Y軸: 件数
   - 線: human, vehicle, unknown, none

4. 異常カメラアラート
   - unknownが多いカメラ
   - 検出率が異常に高い/低いカメラ

5. プリセット効果テーブル
   - プリセット別の検出率・誤検知率
```

### 4.3 ストレージクォータ設計

```typescript
// StorageQuotaSettings コンポーネント
interface StorageSettings {
  quota_gb: number;        // 最大容量 (10-500GB)
  retention_days: number;  // 保存日数 (7-365日)
  cleanup_policy: 'oldest_first' | 'low_priority_first';
  auto_cleanup: boolean;   // 自動クリーンアップ有効
}

// クリーンアップロジック (Backend)
async function runStorageCleanup(settings: StorageSettings) {
  const currentUsage = await getStorageUsage();

  if (currentUsage > settings.quota_gb * 0.9) {  // 90%超過
    if (settings.cleanup_policy === 'oldest_first') {
      // 古い画像から削除
      await deleteOldImages(settings.retention_days);
    } else {
      // unknown/低severity画像を優先削除
      await deleteLowPriorityImages();
    }
  }
}
```

### 4.4 autoAttunement基盤設計

```rust
// Rust Backend: AutoAttunementService
pub struct AutoAttunementService {
    stats_collector: Arc<StatsCollector>,
    threshold_adjuster: Arc<ThresholdAdjuster>,
}

impl AutoAttunementService {
    /// 誤検知統計に基づいて閾値を自動調整
    pub async fn calculate_optimal_threshold(
        &self,
        camera_id: &str,
    ) -> Result<f32> {
        // 1. 過去7日の検出統計を取得
        let stats = self.stats_collector
            .get_camera_stats(camera_id, Duration::days(7))
            .await?;

        // 2. 誤検知報告を取得
        let feedbacks = self.stats_collector
            .get_feedbacks(camera_id)
            .await?;

        // 3. 誤検知率を計算
        let false_positive_rate = feedbacks.len() as f32
            / stats.total_detections as f32;

        // 4. 閾値調整
        let current = stats.current_threshold;
        let adjustment = if false_positive_rate > 0.1 {
            0.05  // 誤検知多い → 閾値上げ
        } else if false_positive_rate < 0.01 {
            -0.02  // 誤検知少ない → 閾値下げ可能
        } else {
            0.0
        };

        Ok((current + adjustment).clamp(0.2, 0.8))
    }
}
```

## 5. タスクリスト

| # | タスク | 工数 | 依存 |
|---|--------|------|------|
| T1 | 統計APIエンドポイント実装 (BE) | 4h | - |
| T2 | InferenceStatsTab.tsx実装 (FE) | 4h | T1 |
| T3 | CameraDistributionChart実装 | 2h | T2 |
| T4 | EventTrendsChart実装 | 2h | T2 |
| T5 | AnomalyCamerasList実装 | 1h | T2 |
| T6 | StorageQuotaSettings実装 | 3h | - |
| T7 | ストレージクリーンアップジョブ実装 | 2h | T6 |
| T8 | autoAttunement基盤実装 | 4h | T1 |
| T9 | Settings Modal統合 | 2h | T2,T6 |
| T10 | Chrome UIテスト | 2h | T9 |

**合計工数**: 26h

## 6. テスト計画

### 6.1 Backend テスト

| テストケース | 期待結果 |
|--------------|----------|
| GET /api/stats/cameras?period=24h | カメラ別統計JSON返却 |
| GET /api/stats/presets | プリセット効果JSON返却 |
| POST /api/settings/storage | 設定保存成功 |
| ストレージ90%超過時 | 自動クリーンアップ実行 |

### 6.2 Frontend テスト

| テストケース | 期待結果 |
|--------------|----------|
| 推論統計タブ表示 | グラフ・テーブル正常描画 |
| 期間切替 (24h/7d/30d) | データ再取得・グラフ更新 |
| 異常カメラクリック | 詳細モーダル表示 |
| ストレージ設定変更 | 保存成功・反映確認 |

### 6.3 Chrome UIテスト

| # | 手順 | 期待結果 |
|---|------|----------|
| 1 | Settings Modal開く | モーダル表示 |
| 2 | 「推論統計」タブクリック | 統計画面表示 |
| 3 | カメラ別分布グラフ確認 | Tam-1F-Frontがunknown多い |
| 4 | 異常カメラリスト確認 | Tam-1F-Front表示 |
| 5 | ストレージ設定変更 | 保存成功 |

## 7. 受け入れ基準 (MECE)

1. **推論統計タブ**
   - カメラ別分布グラフが表示される
   - 時系列傾向グラフが表示される
   - 異常カメラが特定・表示される

2. **プリセット効果分析**
   - プリセット別の検出率が表示される
   - 誤検知率が表示される（フィードバックベース）

3. **ストレージクォータ**
   - 現在の使用量が表示される
   - 上限設定が保存できる
   - 自動クリーンアップが動作する

4. **autoAttunement基盤**
   - 閾値自動調整APIが実装される
   - 統計データが収集される

## 8. MECE検証

| カテゴリ | 項目 | 網羅性 |
|----------|------|--------|
| 表示機能 | カメラ別分布、時系列、プリセット効果 | ✅ |
| 設定機能 | ストレージクォータ、クリーンアップポリシー | ✅ |
| 分析機能 | 異常検出、傾向分析 | ✅ |
| 自動化 | autoAttunement基盤 | ✅ |

## 9. 依存関係確認

- detection_logs (SSoT) ✅ 既存
- cameras テーブル ✅ 既存
- facility_settings ✅ 既存（拡張必要）
- misdetection_feedbacks ✅ 019で定義済み

---
作成日: 2026-01-08
作成者: Claude Code
大原則確認: SSoT遵守、SOLID準拠、MECE達成、アンアンビギュアス達成
