# AI Event Log チューニングガイド

## 改訂履歴
| 日付 | バージョン | 変更内容 |
|------|----------|----------|
| 2026-01-02 | 1.0.0 | 初版作成 |

---

## 1. 概要

### 1.1 目的
本ガイドはAI Event Log機能のプリセットパラメータおよびis21解析精度のチューニング手順を定義する。

### 1.2 チューニング対象

| 対象 | 説明 | 調整頻度 |
|------|------|---------|
| プリセットパラメータ | conf_threshold, suspicious_config等 | 月次 |
| camera_context | location_type, expected_objects等 | カメラ追加時 |
| is21モデル | YOLOv5s, PARモデル | 四半期 |
| 表示テンプレート | UI表示フォーマット | 随時 |

---

## 2. PDCAサイクル

### 2.1 Plan（計画）

#### 2.1.1 課題特定
detection_logsから以下を分析：

```sql
-- 誤検知（False Positive）の多いプリセット/カメラ
SELECT
    preset_id,
    camera_id,
    COUNT(*) as total,
    SUM(CASE WHEN manually_verified = 'false_positive' THEN 1 ELSE 0 END) as fp_count,
    SUM(CASE WHEN manually_verified = 'false_positive' THEN 1 ELSE 0 END) / COUNT(*) as fp_rate
FROM detection_logs
WHERE created_at > NOW() - INTERVAL 7 DAY
GROUP BY preset_id, camera_id
HAVING fp_rate > 0.1
ORDER BY fp_rate DESC;

-- 見逃し傾向の分析（低confidence検知の分布）
SELECT
    preset_id,
    primary_event,
    AVG(confidence) as avg_conf,
    COUNT(*) as count
FROM detection_logs
WHERE created_at > NOW() - INTERVAL 7 DAY
GROUP BY preset_id, primary_event
ORDER BY avg_conf;

-- suspicious.scoreの分布
SELECT
    preset_id,
    JSON_EXTRACT(suspicious, '$.level') as level,
    COUNT(*) as count,
    AVG(JSON_EXTRACT(suspicious, '$.score')) as avg_score
FROM detection_logs
WHERE created_at > NOW() - INTERVAL 7 DAY
GROUP BY preset_id, level;
```

#### 2.1.2 目標設定

| メトリクス | 現状 | 目標 | 期限 |
|-----------|------|------|------|
| 人物検出Precision | 0.85 | 0.95 | 2週間後 |
| 人物検出Recall | 0.80 | 0.90 | 2週間後 |
| 誤検知率（restrictedプリセット） | 15% | 5% | 1週間後 |

### 2.2 Do（実行）

#### 2.2.1 パラメータ調整

**conf_threshold調整**
```json
// 現状（誤検知多い場合）
"conf_threshold": 0.3

// 調整後（閾値上げ）
"conf_threshold": 0.45
```

**suspicious_config調整**
```json
// 現状
"suspicious_config": {
  "weights": {
    "time_factor": 2.0,
    "posture_factor": 2.0
  }
}

// 調整後（夜間検知の誤検知低減）
"suspicious_config": {
  "weights": {
    "time_factor": 1.2,  // 下げる
    "posture_factor": 2.0
  }
}
```

#### 2.2.2 A/Bテスト実施

```rust
// 同一カメラで新旧プリセットを交互適用
async fn run_ab_test(camera_id: &str, duration: Duration) {
    let preset_a = load_preset("balanced_v1.0.0");
    let preset_b = load_preset("balanced_v1.0.1"); // 調整版

    let mut use_a = true;
    let end_time = Instant::now() + duration;

    while Instant::now() < end_time {
        let preset = if use_a { &preset_a } else { &preset_b };
        let result = analyze_with_preset(camera_id, preset).await;

        // 結果にpresetバージョンをタグ付け
        save_with_ab_tag(result, if use_a { "A" } else { "B" }).await;

        use_a = !use_a;
        sleep(Duration::from_secs(5)).await;
    }
}
```

### 2.3 Check（確認）

#### 2.3.1 精度評価メトリクス

| メトリクス | 定義 | 計算式 | 目標値 |
|-----------|------|--------|--------|
| Precision | 検出のうち正解の割合 | TP / (TP + FP) | > 0.95 |
| Recall | 正解のうち検出した割合 | TP / (TP + FN) | > 0.90 |
| F1 Score | PrecisionとRecallの調和平均 | 2*P*R / (P+R) | > 0.92 |
| mAP | 平均適合率 | IoU=0.5での平均 | > 0.80 |

#### 2.3.2 手動検証プロセス

```
1. ランダムサンプリング（1日100件）
2. 検証者が画像とログを確認
3. 判定入力:
   - true_positive: 正しい検知
   - false_positive: 誤検知
   - false_negative: 見逃し（別途入力）
4. detection_logs.manually_verified カラム更新
```

#### 2.3.3 検証UI

```typescript
// VerificationPane コンポーネント
interface VerificationItem {
  log_id: number;
  image_url: string;
  primary_event: string;
  confidence: number;
  bboxes: BBox[];
}

function VerificationPane() {
  const [items, setItems] = useState<VerificationItem[]>([]);

  const verify = async (logId: number, result: 'tp' | 'fp' | 'fn') => {
    await fetch(`/api/detection-logs/${logId}/verify`, {
      method: 'POST',
      body: JSON.stringify({ result })
    });
  };

  return (
    <div>
      {items.map(item => (
        <div key={item.log_id}>
          <img src={item.image_url} />
          <BboxOverlay bboxes={item.bboxes} />
          <div>
            <button onClick={() => verify(item.log_id, 'tp')}>✅ 正解</button>
            <button onClick={() => verify(item.log_id, 'fp')}>❌ 誤検知</button>
          </div>
        </div>
      ))}
    </div>
  );
}
```

#### 2.3.4 A/Bテスト結果分析

```sql
SELECT
    ab_tag,
    COUNT(*) as total,
    AVG(confidence) as avg_conf,
    SUM(CASE WHEN manually_verified = 'true_positive' THEN 1 ELSE 0 END) / COUNT(*) as precision,
    AVG(JSON_EXTRACT(suspicious, '$.score')) as avg_suspicious
FROM detection_logs
WHERE ab_test_id = 'test-001'
GROUP BY ab_tag;
```

### 2.4 Act（改善）

#### 2.4.1 プリセットバージョン更新

```bash
# 1. 新バージョンファイル作成
cp camera_presets_v1.0.0.json camera_presets_v1.0.1.json

# 2. パラメータ修正
vim camera_presets_v1.0.1.json

# 3. is22/is21へ配布
scp camera_presets_v1.0.1.json is22:/opt/is22/config/presets/
scp camera_presets_v1.0.1.json is21:/opt/is21/config/presets/

# 4. 設定リロード
curl -X POST http://192.168.125.246:8080/api/admin/reload-presets
curl -X POST http://192.168.3.240:9000/api/admin/reload-presets
```

#### 2.4.2 ロールバック手順

```bash
# 問題発生時
# 1. 旧バージョンに切り替え
curl -X POST http://192.168.125.246:8080/api/admin/set-preset-version \
  -d '{"version": "1.0.0"}'

# 2. 該当カメラのプリセット強制更新
curl -X POST http://192.168.125.246:8080/api/cameras/bulk-update \
  -d '{"preset_version": "1.0.0"}'

# 3. 影響調査
SELECT * FROM detection_logs
WHERE preset_version = '1.0.1'
  AND created_at > NOW() - INTERVAL 1 HOUR;
```

---

## 3. プリセット別チューニング指針

### 3.1 person_priority

| 課題 | 調整項目 | 調整方向 |
|------|---------|---------|
| 人物以外を誤検知 | conf_threshold | 上げる (0.4→0.5) |
| 細部が取れない | par_full_attributes | true維持、is21側PAR閾値下げ |
| 処理遅い | 検討: 一部属性をオフ | par_focus指定 |

### 3.2 parking

| 課題 | 調整項目 | 調整方向 |
|------|---------|---------|
| ナンバー読めない | vehicle_ocr設定 | is21側OCRモデル確認 |
| 車両見逃し | conf_threshold | 下げる (0.5→0.4) |
| 誤検知（背景を車と誤認） | iou_threshold | 上げる (0.5→0.6) |

### 3.3 restricted

| 課題 | 調整項目 | 調整方向 |
|------|---------|---------|
| 誤検知多い | conf_threshold | 上げる (0.3→0.4) |
| suspicious高すぎ | base_score, weights | 下げる |
| 見逃し厳禁 | conf_threshold | 維持（多少の誤検知は許容） |

### 3.4 kitchen

| 課題 | 調整項目 | 調整方向 |
|------|---------|---------|
| 帽子検出漏れ | par_focus | ['hat']確認、is21閾値下げ |
| 蒸気で誤検知 | conf_threshold | 上げる |
| 手袋検出難しい | 検討: camera_context | distance='close'推奨 |

---

## 4. camera_contextチューニング

### 4.1 location_type選択ガイド

| 設置場所 | 推奨値 | 備考 |
|----------|--------|------|
| 玄関・受付 | entrance | 入退室検知最適化 |
| 廊下 | corridor | 通過カウント優先 |
| 駐車場 | parking | 車両・OCR優先 |
| 屋外 | outdoor | 広範囲・天候対応 |
| 室内一般 | indoor | バランス設定 |
| 制限区域 | restricted | 高感度 |

### 4.2 distance設定

| 値 | 推奨カメラ画角 | 検出対象サイズ |
|----|---------------|---------------|
| close | 狭角（< 60°） | 画面の50%以上 |
| medium | 中角（60-90°） | 画面の20-50% |
| far | 広角（> 90°） | 画面の20%未満 |

### 4.3 expected_objects調整

```json
// 駐車場カメラ
{
  "expected_objects": ["vehicle", "human"],
  "excluded_objects": ["animal"]  // 鳥など除外
}

// 厨房カメラ
{
  "expected_objects": ["human"],
  "excluded_objects": ["vehicle", "animal"]
}
```

### 4.4 時間帯設定

```json
// オフィス玄関
{
  "busy_hours": ["08:30-09:30", "17:30-18:30"],
  "quiet_hours": ["22:00-06:00"]
}

// 24時間稼働施設
{
  "busy_hours": [],
  "quiet_hours": []  // 時間帯補正なし
}
```

---

## 5. 精度評価レポート

### 5.1 週次レポートテンプレート

```markdown
# AI Event Log 精度レポート

## 期間
2026-01-01 〜 2026-01-07

## サマリー
| メトリクス | 今週 | 先週 | 変化 |
|-----------|------|------|------|
| 総検知数 | 12,345 | 11,890 | +3.8% |
| Precision | 0.94 | 0.92 | +2.2% |
| Recall | 0.88 | 0.87 | +1.1% |
| 誤検知率 | 6% | 8% | -2.0% |

## プリセット別精度
| preset_id | 検知数 | Precision | Recall | 主な課題 |
|-----------|--------|-----------|--------|----------|
| balanced | 5,000 | 0.93 | 0.89 | - |
| parking | 3,000 | 0.96 | 0.91 | ナンバー読取85% |
| restricted | 500 | 0.88 | 0.95 | 誤検知やや多め |
| kitchen | 200 | 0.91 | 0.82 | 手袋検出改善要 |

## 改善アクション
1. restrictedのconf_threshold 0.3→0.35に調整
2. kitchenカメラにdistance=close推奨
3. ナンバーOCR精度向上のためis21モデル更新検討

## 次週予定
- A/Bテスト: balanced v1.0.1
- kitchenプリセット手袋検出閾値調整
```

### 5.2 自動レポート生成

```python
# generate_accuracy_report.py
import pandas as pd
from datetime import datetime, timedelta

def generate_weekly_report(start_date, end_date):
    # detection_logsからデータ取得
    query = f"""
    SELECT
        preset_id,
        primary_event,
        confidence,
        manually_verified,
        JSON_EXTRACT(suspicious, '$.score') as suspicious_score
    FROM detection_logs
    WHERE created_at BETWEEN '{start_date}' AND '{end_date}'
    """
    df = pd.read_sql(query, db_connection)

    # 精度計算
    tp = len(df[df['manually_verified'] == 'true_positive'])
    fp = len(df[df['manually_verified'] == 'false_positive'])
    precision = tp / (tp + fp) if (tp + fp) > 0 else 0

    # プリセット別集計
    preset_stats = df.groupby('preset_id').agg({
        'confidence': 'mean',
        'suspicious_score': 'mean'
    })

    # レポート生成
    report = {
        'period': f'{start_date} ~ {end_date}',
        'total_detections': len(df),
        'precision': precision,
        'preset_stats': preset_stats.to_dict()
    }

    return report
```

---

## 6. 履歴管理

### 6.1 プリセット変更履歴テーブル

```sql
CREATE TABLE preset_change_history (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    preset_id VARCHAR(32) NOT NULL,
    old_version VARCHAR(16) NOT NULL,
    new_version VARCHAR(16) NOT NULL,
    changed_params JSON NOT NULL,
    reason TEXT NOT NULL,
    changed_by VARCHAR(64) NOT NULL,
    effect_measured JSON NULL,
    rollback_at DATETIME NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 6.2 変更記録例

```json
{
  "preset_id": "restricted",
  "old_version": "1.0.0",
  "new_version": "1.0.1",
  "changed_params": {
    "conf_threshold": {
      "old": 0.3,
      "new": 0.35
    },
    "suspicious_config.weights.time_factor": {
      "old": 2.0,
      "new": 1.5
    }
  },
  "reason": "夜間の誤検知率が15%と高かったため、conf_threshold上げとtime_factor下げで対応",
  "changed_by": "admin@example.com",
  "effect_measured": {
    "before": {
      "fp_rate": 0.15,
      "precision": 0.85
    },
    "after": {
      "fp_rate": 0.08,
      "precision": 0.92
    },
    "measured_period": "2026-01-08 ~ 2026-01-14"
  }
}
```

### 6.3 変更承認フロー

```
1. チューニング担当者: 変更提案作成
   ↓
2. A/Bテスト実施（最低24時間）
   ↓
3. 精度比較レポート作成
   ↓
4. レビュー承認（管理者）
   ↓
5. プリセットバージョン更新
   ↓
6. 本番適用（段階的ロールアウト推奨）
   ↓
7. 1週間モニタリング
   ↓
8. 効果測定・履歴記録
```

---

## 7. is21モデルチューニング

### 7.1 対象モデル

| モデル | 用途 | 更新頻度 |
|--------|------|---------|
| YOLOv5s | 物体検出 | 四半期 |
| PAR (PA-100K) | 人物属性認識 | 四半期 |
| PaddleOCR | ナンバー認識 | 半期 |

### 7.2 再学習トリガー

- 特定カテゴリのRecallが80%を下回った
- 新しい検出対象が追加された
- 運用環境（照明・カメラ）が大きく変わった

### 7.3 再学習データセット

```
training_data/
├── yolo/
│   ├── images/
│   │   ├── train/    # 学習用画像
│   │   └── val/      # 検証用画像
│   └── labels/
│       ├── train/    # YOLO形式アノテーション
│       └── val/
├── par/
│   ├── images/
│   └── annotations/
└── ocr/
    ├── images/
    └── labels/
```

---

## 8. トラブルシューティング

### 8.1 よくある問題と対処

| 問題 | 原因 | 対処 |
|------|------|------|
| 全く検知しない | conf_threshold高すぎ | 0.3まで下げてテスト |
| 誤検知が多い | conf_threshold低すぎ | 0.05ずつ上げる |
| 処理が遅い | PAR属性多すぎ | par_full_attributes=false |
| suspicious高すぎ | weights設定 | time_factor, posture_factor下げ |
| ナンバー読めない | OCRモデル・画質 | distance=close, is21OCR確認 |

### 8.2 デバッグモード

```bash
# is21詳細ログ有効化
curl -X POST http://192.168.3.240:9000/api/debug/enable \
  -d '{"level": "trace", "duration_sec": 3600}'

# is22詳細ログ
export RUST_LOG=is22=trace
systemctl restart is22
```

---

## 9. チェックリスト

### 9.1 チューニング前チェック

- [ ] 現状のPrecision/Recall測定完了
- [ ] 問題のあるプリセット/カメラ特定
- [ ] 目標値設定
- [ ] A/Bテスト計画作成

### 9.2 チューニング後チェック

- [ ] A/Bテスト実施（最低24時間）
- [ ] 精度改善確認
- [ ] 副作用（他メトリクス悪化）なし
- [ ] 変更履歴記録
- [ ] レビュー承認取得
- [ ] 段階的ロールアウト計画

### 9.3 本番適用後チェック

- [ ] 1週間モニタリング
- [ ] 効果測定レポート作成
- [ ] 問題なければ完了、問題あればロールバック
