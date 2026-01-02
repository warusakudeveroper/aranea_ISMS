# is21 v1.6.1 全テスト生データ

作成日: 2025-12-30

---

## サマリー

| # | 画像 | primary_event | count | total_ms | yolo_ms | par_ms |
|---|------|---------------|-------|----------|---------|--------|
| 1 | Room | none | 0 | **178ms** | 77ms | 0ms |
| 2 | Intruder | human | 1 | **107ms** | 39ms | 21ms |
| 3 | Outdoor | human | 1 | **103ms** | 40ms | 20ms |
| 4 | Night | human | 1 | **113ms** | 38ms | 20ms |
| 5 | Corridor | none | 0 | **95ms** | 41ms | 0ms |
| 6 | Parking | vehicle | 0 | **64ms** | 25ms | 0ms |

**平均処理時間: 110ms** (目標200ms以内)

---

## Test 1: Room (25581835_m.jpg)

### 処理時間
```
total: 178ms | yolo: 77ms | par: 0ms
```

### レスポンス
```json
{
  "schema_version": "2025-12-29.1",
  "camera_id": "test01",
  "analyzed": true,
  "detected": false,
  "primary_event": "none",
  "tags": [],
  "confidence": 0.0,
  "severity": 0,
  "count_hint": 0,
  "bboxes": [
    {"label": "chair", "conf": 0.52, "x1": 0.59, "y1": 0.37, "x2": 0.88, "y2": 0.61},
    {"label": "chair", "conf": 0.52, "x1": 0.36, "y1": 0.14, "x2": 1.00, "y2": 0.83},
    {"label": "bed", "conf": 0.52, "x1": 0.30, "y1": 0.41, "x2": 0.67, "y2": 0.66}
  ],
  "person_details": []
}
```

### 判定
- ✅ 人物なし正しく判定
- 家具(chair, bed)は正しく除外

---

## Test 2: Intruder (camera-night-installation2.jpg)

### 処理時間
```
total: 107ms | yolo: 39ms | par: 21ms
```

### レスポンス
```json
{
  "primary_event": "human",
  "detected": true,
  "count_hint": 1,
  "confidence": 0.509,
  "severity": 1,
  "tags": [
    "count.single",
    "top_color.gray",
    "appearance.hat_like",
    "appearance.glasses",
    "appearance.uniform_like",
    "carry.backpack",
    "carry.bag",
    "carry.holding",
    "outfit.coat_like",
    "outfit.pants",
    "outfit.shorts",
    "outfit.dress",
    "outfit.boots",
    "body.size.large",
    "body.build.heavy",
    "gender.female",
    "age.adult"
  ],
  "bboxes": [
    {
      "label": "person",
      "conf": 0.509,
      "x1": 0.00, "y1": 0.03, "x2": 0.72, "y2": 0.75,
      "par": {
        "tags": ["appearance.hat_like", "appearance.glasses", "carry.backpack", "carry.bag", "carry.holding", "outfit.coat_like"],
        "meta": {
          "gender": "female",
          "age_group": "adult",
          "age_confidence": 0.73,
          "upper_sleeve": "long",
          "lower_type": "trousers"
        }
      },
      "details": {
        "top_color": {"color": "gray", "confidence": 0.92},
        "uniform_like": true,
        "body_size": "large",
        "body_build": "heavy"
      }
    },
    {"label": "couch", "conf": 0.50}
  ],
  "person_details": [
    {
      "index": 0,
      "top_color": {"color": "gray", "confidence": 0.92},
      "uniform_like": true,
      "body_size": "large",
      "body_build": "heavy",
      "meta": {
        "gender": "female",
        "age_group": "adult",
        "upper_sleeve": "long",
        "lower_type": "trousers"
      }
    }
  ]
}
```

### 不審者判定に使える属性
| 属性 | 値 | 不審度寄与 |
|------|-----|-----------|
| 服装色 | gray (92%) | 中 (暗い色) |
| 帽子 | hat_like | 高 |
| uniform_like | true | 中 (本物の作業員かも) |
| 体格 | large, heavy | - |
| 荷物 | backpack, bag, holding | 中 |

---

## Test 3: Outdoor (IMG_1F097BD1F07C-1.jpg)

### 処理時間
```
total: 103ms | yolo: 40ms | par: 20ms
```

### レスポンス
```json
{
  "primary_event": "human",
  "detected": true,
  "count_hint": 1,
  "confidence": 0.518,
  "tags": [
    "count.single",
    "top_color.gray",
    "appearance.hat_like",
    "appearance.glasses",
    "appearance.uniform_like",
    "carry.backpack",
    "carry.bag",
    "carry.holding",
    "outfit.coat_like",
    "outfit.pants",
    "outfit.shorts",
    "outfit.dress",
    "outfit.boots",
    "body.size.medium",
    "body.build.heavy",
    "gender.female",
    "age.adult"
  ],
  "bboxes": [
    {
      "label": "person",
      "conf": 0.518,
      "x1": 0.41, "y1": 0.37, "x2": 0.67, "y2": 0.60,
      "details": {
        "top_color": {"color": "gray", "confidence": 0.85},
        "body_size": "medium"
      }
    }
  ],
  "person_details": [
    {
      "top_color": {"color": "gray", "confidence": 0.85},
      "uniform_like": true,
      "body_size": "medium",
      "body_build": "heavy"
    }
  ]
}
```

### 備考
- ❌ 2人中1人のみ検出 (遠距離人物は検出漏れ)

---

## Test 4: Night (security-camera-image03.webp)

### 処理時間
```
total: 113ms | yolo: 38ms | par: 20ms
```

### レスポンス
```json
{
  "primary_event": "human",
  "detected": true,
  "count_hint": 1,
  "confidence": 0.520,
  "tags": [
    "count.single",
    "top_color.gray",
    "appearance.hat_like",
    "appearance.glasses",
    "appearance.uniform_like",
    "carry.backpack",
    "carry.bag",
    "carry.holding",
    "outfit.coat_like",
    "outfit.pants",
    "outfit.shorts",
    "outfit.dress",
    "outfit.boots",
    "body.size.large",
    "body.build.heavy",
    "gender.female",
    "age.adult"
  ],
  "bboxes": [
    {
      "label": "person",
      "conf": 0.520,
      "x1": 0.17, "y1": 0.01, "x2": 0.50, "y2": 0.47,
      "details": {
        "top_color": {"color": "gray", "confidence": 0.96},
        "body_size": "large",
        "body_build": "heavy"
      }
    }
  ]
}
```

---

## Test 5: Corridor (security-night-02.jpg)

### 処理時間
```
total: 95ms | yolo: 41ms | par: 0ms
```

### レスポンス
```json
{
  "primary_event": "none",
  "detected": false,
  "count_hint": 0,
  "tags": [],
  "bboxes": [],
  "person_details": []
}
```

### 判定
- ✅ 人物なし正しく判定

---

## Test 6: Parking (service-camera-02.jpg)

### 処理時間
```
total: 64ms | yolo: 25ms | par: 0ms
```

### レスポンス
```json
{
  "primary_event": "vehicle",
  "detected": true,
  "count_hint": 0,
  "confidence": 0.514,
  "tags": ["vehicle_color.gray"],
  "bboxes": [
    {
      "label": "car",
      "conf": 0.514,
      "x1": 0.53, "y1": 0.15, "x2": 0.94, "y2": 0.61
    }
  ],
  "person_details": []
}
```

### 判定
- ✅ 車両検出成功
- ✅ 人物誤検出なし

---

## 不審者判定 (Suspicious Person Detection)

### 現在検出可能な不審者関連属性

| 属性 | タグ | 検出状況 |
|------|------|----------|
| **顔隠し** | appearance.mask_like | ✅ 実装済 |
| **帽子** | appearance.hat_like | ✅ 検出中 |
| **暗い服装** | top_color.black/gray | ✅ 検出中 |
| **大きな荷物** | carry.backpack | ✅ 検出中 |
| **制服風** | appearance.uniform_like | ✅ 検出中 |

### 不審者スコア計算案

```python
def calculate_suspicious_score(tags, time_of_day, location_type):
    score = 0

    # 顔を隠している (最重要)
    if "appearance.mask_like" in tags:
        score += 40

    # 夜間 + 暗い服装
    if time_of_day == "night":
        if "top_color.black" in tags or "top_color.gray" in tags:
            score += 20
        if "appearance.hat_like" in tags:
            score += 10

    # 異常な時間帯 (2:00-5:00)
    if 2 <= hour < 5:
        score += 25

    # うろつき行動 (未実装)
    # if "behavior.loitering" in tags:
    #     score += 30

    # 立入禁止区域
    if location_type == "restricted":
        score += 20

    return min(score, 100)
```

### 不審度レベル

| スコア | レベル | アクション |
|--------|--------|-----------|
| 0-20 | 正常 | ログのみ |
| 21-40 | 注意 | 通知なし、監視継続 |
| 41-60 | 警戒 | 管理者通知 |
| 61-80 | 高警戒 | 即時通知 + 録画開始 |
| 81-100 | 危険 | 緊急通報推奨 |

### 未実装で必要な機能

| 機能 | 必要技術 | 優先度 |
|------|----------|--------|
| **徘徊検出** | 複数フレーム時系列分析 | 高 |
| **走行検出** | 姿勢推定 (MoveNet) | 中 |
| **時刻連携** | captured_at解析 | 高 |
| **エリア判定** | カメラ位置マスタ | 中 |

---

## PAR Raw属性値 (参考)

Test 2 (Intruder) の PAR 生出力:

| 属性 | 確率 |
|------|------|
| Hat | 0.73 |
| Glasses | 0.52 |
| LongSleeve | 0.68 |
| LongCoat | 0.54 |
| Backpack | 0.61 |
| HandBag | 0.55 |
| ShoulderBag | 0.48 |
| HoldObjectsInFront | 0.59 |
| Age18-60 | 0.73 |
| Female | 0.52 |

---

作成: Claude Code
