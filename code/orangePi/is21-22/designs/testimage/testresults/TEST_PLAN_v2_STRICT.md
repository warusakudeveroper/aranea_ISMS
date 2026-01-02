# is21推論テスト計画 v2.0（厳密版）

作成日: 2025-12-30
ステータス: 策定中

---

## 問題認識

### v1.0テストの欠陥

v1.0テストは以下の点で**不合格**と判定すべきだった：

| 画像 | 正解人数 | 検出人数 | v1.0判定 | 厳密判定 |
|------|---------|---------|---------|---------|
| camera-night-installation2.jpg | 1 | 3 | PASS | **FAIL** (+200%誤差) |
| IMG_1F097BD1F07C-1.jpg | 2 | 3 | PASS | **FAIL** (+50%誤差) |
| security-camera-image03.webp | 1 | 3 | PASS | **FAIL** (+200%誤差) |
| service-camera-02.jpg | 車6-8台 | 3台 | PASS | **FAIL** (-50%以上) |

**v1.0テスト基準は「存在検知」レベルであり、実用的な監視システムの要件を満たしていない。**

---

## v2.0 テスト基準

### 1. 人物検出精度

| 指標 | 合格基準 | 備考 |
|------|---------|------|
| 人数カウント誤差 | ±1名以内 | 3名→2-4名で合格 |
| 検出漏れ（FN） | 0 | 全員検出必須 |
| 誤検出（FP） | 実人数の50%以下 | 1名→最大1名の誤検出許容 |
| bbox IoU | >= 0.3 | 正解bboxとの重なり |

### 2. 車両検出精度

| 指標 | 合格基準 | 備考 |
|------|---------|------|
| 台数カウント誤差 | ±30%以内 | 10台→7-13台で合格 |
| 主要車両の検出 | 100% | 画面の10%以上を占める車両 |
| 車種分類 | 対象外 | YOLOv5sはcar/truck/busのみ |

### 3. 位置精度（bbox）

| 指標 | 合格基準 |
|------|---------|
| 中心位置誤差 | 画像幅の20%以内 |
| サイズ誤差 | 50%-200%の範囲 |

### 4. 負例（人なし画像）

| 指標 | 合格基準 |
|------|---------|
| 誤検出（FP） | 0 |

---

## 詳細Ground Truth（再定義）

### 画像1: 25581835_m.jpg（ホテル客室）

**正解**
```yaml
persons: 0
vehicles: 0
objects:
  - label: bed
    bbox: [0.25, 0.30, 0.72, 0.77]  # [x1, y1, x2, y2] 正規化座標
  - label: chair
    bbox: [0.60, 0.37, 0.88, 0.61]
```

**合格基準**
- person検出: 0（誤検出なし）
- primary_event: none または unknown

---

### 画像2: camera-night-installation2.jpg（室内・不審者）

**正解**
```yaml
persons: 1
persons_detail:
  - id: 1
    bbox: [0.10, 0.25, 0.45, 0.95]  # 画面左、しゃがんだ不審者
    attributes:
      clothing: black
      posture: crouching
vehicles: 0
objects:
  - label: couch
    bbox: [0.52, 0.23, 0.95, 0.74]
  - label: tv
    bbox: [0.30, 0.00, 0.70, 0.20]
```

**合格基準**
- person検出: 1名（±1許容 → 0-2名で合格）
- 検出bboxがperson_1のbboxとIoU >= 0.3
- primary_event: human

---

### 画像3: IMG_1F097BD1F07C-1.jpg（屋外・2名）

**正解**
```yaml
persons: 2
persons_detail:
  - id: 1
    bbox: [0.35, 0.45, 0.55, 0.95]  # 手前（ぼやけ）
    attributes:
      visibility: blurred
  - id: 2
    bbox: [0.55, 0.30, 0.68, 0.55]  # 奥（小さい）
    attributes:
      visibility: small
vehicles: 0
```

**合格基準**
- person検出: 2名（±1許容 → 1-3名で合格）
- 両者のbboxが検出されている（IoU >= 0.2）
- primary_event: human

---

### 画像4: security-camera-image03.webp（夜間屋外）

**正解**
```yaml
persons: 1
persons_detail:
  - id: 1
    bbox: [0.08, 0.15, 0.30, 0.90]  # 白シャツの人物
    attributes:
      clothing: white_shirt
vehicles: 0
```

**合格基準**
- person検出: 1名（±1許容 → 0-2名で合格）
- 検出bboxがperson_1のbboxとIoU >= 0.3
- primary_event: human

---

### 画像5: security-night-02.jpg（ホテル廊下）

**正解**
```yaml
persons: 0
vehicles: 0
```

**合格基準**
- person検出: 0（誤検出なし）
- vehicle検出: 0
- primary_event: none

---

### 画像6: service-camera-02.jpg（駐車場）

**正解**
```yaml
persons: 0
vehicles: 8
vehicles_detail:
  - id: 1
    bbox: [0.00, 0.35, 0.25, 0.75]
    type: van  # 白いバン（左手前）
  - id: 2
    bbox: [0.53, 0.15, 0.94, 0.61]
    type: suv  # グレーSUV（右）
  - id: 3
    bbox: [0.15, 0.08, 0.35, 0.30]
    type: sedan
  - id: 4
    bbox: [0.35, 0.05, 0.50, 0.25]
    type: sedan
  - id: 5
    bbox: [0.05, 0.05, 0.20, 0.25]
    type: sedan
  # ... 他3台は遠方で小さい
main_vehicles: 5  # 画面の5%以上を占める車両
```

**合格基準**
- vehicle検出: 5台以上（主要車両のみ）
- person検出: 0
- primary_event: vehicle

---

## v2.0テスト判定ロジック

```python
def evaluate_strict(ground_truth, result):
    score = {
        "person_count": 0,  # -2 ~ +2 (正確なら+2)
        "person_bbox": 0,   # IoU評価
        "vehicle_count": 0,
        "false_positive": 0,  # 誤検出ペナルティ
        "primary_event": 0
    }

    # 人数カウント
    gt_persons = ground_truth["persons"]
    detected_persons = result["count_hint"]
    diff = abs(gt_persons - detected_persons)

    if diff == 0:
        score["person_count"] = 2
    elif diff == 1:
        score["person_count"] = 1
    else:
        score["person_count"] = -1  # 2名以上の誤差は減点

    # 誤検出チェック（人なし画像）
    if gt_persons == 0 and detected_persons > 0:
        score["false_positive"] = -2

    # 車両カウント（該当画像のみ）
    if "vehicles" in ground_truth:
        gt_vehicles = ground_truth["main_vehicles"]
        detected_vehicles = len([b for b in result["bboxes"] if b["label"] in ["car", "truck", "bus"]])
        if detected_vehicles >= gt_vehicles * 0.5:
            score["vehicle_count"] = 1
        else:
            score["vehicle_count"] = -1

    # 総合判定
    total = sum(score.values())
    return "PASS" if total >= 2 else "FAIL", score
```

---

## 現状の実測値による再評価

| 画像 | 正解人数 | 検出人数 | 誤差 | v2.0判定 |
|------|---------|---------|------|---------|
| 25581835_m.jpg | 0 | 0 | 0 | **PASS** |
| camera-night-installation2.jpg | 1 | 3 | +2 | **FAIL** |
| IMG_1F097BD1F07C-1.jpg | 2 | 3 | +1 | **PASS** |
| security-camera-image03.webp | 1 | 3 | +2 | **FAIL** |
| security-night-02.jpg | 0 | 0 | 0 | **PASS** |
| service-camera-02.jpg | 車8台 | 3台 | -5 | **FAIL** |

**v2.0基準での合格率: 3/6 (50%)**

---

## 次のアクション

### 1. モデル/後処理の改善

| 対策 | 期待効果 | 優先度 |
|------|---------|--------|
| NMS閾値を0.35に下げる | 重複検出削減 | 高 |
| CONF閾値を0.52に上げる | 誤検出削減 | 中 |
| より大きなモデル(YOLOv5m)検討 | 精度向上 | 低 |

### 2. bbox IoU評価の実装

Ground Truthのbbox座標と検出bboxのIoU計算を実装し、位置精度を定量評価する。

### 3. 車両カウント改善

駐車場画像で8台中3台しか検出できていない問題を分析：
- 信頼度閾値が高すぎる？
- 小さい車両が除外されている？
- モデルの限界？

---

## 結論

**現行is21 v1.1.0は、厳密な評価基準では50%の合格率であり、本番運用には追加のチューニングが必要。**

主な課題：
1. 人物の過検出（1名→3名）
2. 車両の検出漏れ（8台→3台）

改善後、再テストを実施し、80%以上の合格率を目指す。
