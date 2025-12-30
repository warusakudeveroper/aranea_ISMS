# IS21 AI機能 実装報告書

バージョン: 1.8.0
作成日: 2025-12-30
デバイス: Orange Pi 5 Plus (RK3588 NPU)

---

## 1. システム概要

### 1.1 ハードウェア

| 項目 | スペック |
|------|---------|
| SoC | Rockchip RK3588 |
| NPU | 6 TOPS INT8 |
| RAM | 16GB LPDDR4X |
| Storage | 256GB eMMC |
| OS | Ubuntu 22.04 (Orange Pi) |

### 1.2 AIモデル構成

| モデル | 用途 | 入力サイズ | 処理時間 |
|--------|------|-----------|----------|
| YOLOv5s | 物体検出 | 640x640 | 40-70ms |
| PAR ResNet50 | 人物属性認識 | 192x256 | 20-30ms |

---

## 2. 検出機能 (YOLO)

### 2.1 検出可能オブジェクト

```python
EVENT_MAP = {
    "person": "human",
    "car": "vehicle",
    "truck": "vehicle",
    "bus": "vehicle",
    "motorcycle": "vehicle",
    "bicycle": "vehicle",
    "dog": "animal",
    "cat": "animal",
    "bird": "animal",
}
```

### 2.2 検出パラメータ

| パラメータ | 値 | 説明 |
|-----------|-----|------|
| CONF_THRESHOLD | 0.33 | 信頼度閾値 |
| NMS_THRESHOLD | 0.40 | 重複除去閾値 |
| INPUT_SIZE | 640 | 入力画像サイズ |

### 2.3 閾値調整履歴

| バージョン | CONF | 結果 |
|-----------|------|------|
| v1.7.0 | 0.50 | 検出漏れ多数 |
| v1.7.1 | 0.25 | 過検出 |
| v1.7.2 | 0.35 | やや検出漏れ |
| v1.7.3 | 0.32 | やや過検出 |
| **v1.7.4** | **0.33** | **採用** |

---

## 3. 人物属性認識 (PAR)

### 3.1 PA100Kデータセット属性

PAR ResNet50モデルはPA100Kデータセットで学習済み。以下の属性を認識：

| カテゴリ | 属性 |
|----------|------|
| 性別 | Female, Male |
| 年齢層 | AgeLess18, Age18-60, AgeOver60 |
| 体型 | BodyFat, BodyNormal, BodyThin |
| 上半身 | LongSleeve, ShortSleeve, UpperPlaid, UpperSplice, UpperStripe, UpperLogo |
| 下半身 | LowerStripe, LongCoat, Trousers, Shorts, Skirt&Dress |
| 所持品 | Backpack, ShoulderBag, HandBag, HoldObjectsInFront |
| アクセサリ | Hat, Glasses |
| 履物 | Boots, Sandals, Shoes, Sneaker |

### 3.2 PAR処理設定

```python
PAR_ENABLED = True
PAR_MAX_PERSONS = 10  # 最大処理人数
PAR_THRESHOLD = 0.5   # 属性判定閾値
```

---

## 4. 人物特徴強化 (v1.7)

### 4.1 追加機能一覧

| 機能 | 関数 | 出力 |
|------|------|------|
| 上半身色 | `analyze_top_color()` | black/white/gray/red/blue/green/yellow/brown |
| 下半身色 | `analyze_bottom_color()` | 同上 |
| 肌色推定 | `detect_skin_tone()` | light/medium/dark |
| 髪型推定 | `detect_hair_type()` | dark/light/gray_white/covered |
| 姿勢推定 | `estimate_posture()` | standing/sitting/crouching |
| 相対身長 | `estimate_height_category()` | tall/average/short |
| 体格推定 | `estimate_body_build()` | slim/average/heavy |
| 体サイズ | `estimate_body_size()` | small/medium/large |
| マスク検出 | `detect_mask_like()` | boolean |
| 制服判定 | `detect_uniform_like()` | boolean |

### 4.2 色分析アルゴリズム

HSV色空間でのヒストグラム分析：

```python
COLOR_RANGES = {
    "black":  {"h": (0, 180), "s": (0, 100), "v": (0, 50)},
    "white":  {"h": (0, 180), "s": (0, 30), "v": (200, 255)},
    "gray":   {"h": (0, 180), "s": (0, 50), "v": (50, 200)},
    "red":    {"h": [(0, 10), (160, 180)], "s": (100, 255), "v": (50, 255)},
    "blue":   {"h": (100, 130), "s": (100, 255), "v": (50, 255)},
    "green":  {"h": (35, 85), "s": (100, 255), "v": (50, 255)},
    "yellow": {"h": (20, 35), "s": (100, 255), "v": (100, 255)},
    "brown":  {"h": (10, 25), "s": (100, 255), "v": (50, 150)},
}
```

### 4.3 姿勢推定ロジック

bboxアスペクト比ベース：

```python
def estimate_posture(bbox, image_shape):
    width = bbox["x2"] - bbox["x1"]
    height = bbox["y2"] - bbox["y1"]
    aspect = width / height if height > 0 else 0

    if aspect < 0.4:
        return "standing", 0.7
    elif 0.45 < aspect < 0.75:
        return "sitting", 0.6
    elif aspect > 0.75:
        return "crouching", 0.6
    return None, 0
```

---

## 5. 不審者スコア計算

### 5.1 スコア加算ルール

| 条件 | 加算 | 説明 |
|------|------|------|
| appearance.mask_like | **+40** | 顔隠し（最重要） |
| time.late_night (2-5時) | +25 | 深夜帯 |
| time.night (22-6時) | +10 | 夜間 |
| night + black clothing | +15 | 夜間×黒服 |
| night + gray clothing | +10 | 夜間×グレー服 |
| night + hat | +5 | 夜間×帽子 |
| restricted_area | +20 | 立入禁止区域 |
| parking + night | +10 | 駐車場×夜間 |
| entrance_area | **-5** | 入口（減点） |
| busy_hours | **-10** | 繁忙時間帯（減点） |
| quiet_hours | +15 | 静寂時間帯 |
| late_night + backpack | +8 | 深夜×リュック |
| crouching_posture | +10 | しゃがみ姿勢 |

### 5.2 レベル判定

| スコア | レベル | 推奨アクション |
|--------|--------|---------------|
| 0-14 | normal | 通常ログ |
| 15-29 | low | 注意ログ |
| 30-49 | medium | 警告通知 |
| 50-69 | high | アラート |
| 70-100 | critical | 即時対応 |

### 5.3 計算例

**深夜3時、廊下、quiet_hours設定:**
```
time.late_night[3:00] (+25)
night+dark_clothing (+10)
night+hat (+5)
quiet_hours (+15)
late_night+backpack (+8)
crouching_posture (+10)
─────────────────────
合計: 73 → critical
```

**チェックイン時間帯(15:30)、entrance:**
```
entrance_area (-5)
busy_hours (-10)
crouching_posture (+10)
─────────────────────
合計: -5 → normal
```

---

## 6. camera_context (v1.8)

### 6.1 対応パラメータ

| パラメータ | 効果 |
|-----------|------|
| `location_type` | suspicious_score調整 |
| `distance` | conf_threshold調整 |
| `excluded_objects` | 検出結果フィルタ |
| `expected_objects` | 低信頼度フィルタ |
| `detection_roi` | 領域フィルタ |
| `conf_override` | 閾値オーバーライド |
| `busy_hours` | スコア減点 |
| `quiet_hours` | スコア加点 |

### 6.2 フィルタリング処理フロー

```
YOLO検出
    ↓
camera_contextパース
    ↓
excluded_objectsフィルタ
    ↓
expected_objectsフィルタ (conf < 0.5)
    ↓
detection_roi フィルタ
    ↓
PAR推論
    ↓
suspicious_score計算 (context反映)
    ↓
レスポンス
```

---

## 7. 出力タグ一覧

### 7.1 人物タグ

```
gender.male / gender.female
age.child / age.teen / age.adult / age.elderly
top_color.{color}
bottom_color.{color}
hair.dark / hair.light / hair.gray_white / hair.covered
height.tall / height.average / height.short
posture.standing / posture.sitting / posture.crouching
body.size.small / body.size.medium / body.size.large
body.build.slim / body.build.average / body.build.heavy
carry.backpack / carry.bag / carry.holding
appearance.hat_like / appearance.glasses / appearance.mask_like / appearance.uniform_like
outfit.coat_like / outfit.dress / outfit.boots / outfit.shorts / outfit.pants
count.single / count.few / count.many
```

### 7.2 車両タグ

```
vehicle_color.{color}
```

---

## 8. API仕様

### 8.1 解析エンドポイント

```
POST /v1/analyze
Content-Type: multipart/form-data

Parameters:
- infer_image: file (required)
- camera_id: string (required)
- captured_at: string ISO8601 (required)
- schema_version: string (required)
- hints_json: string JSON (optional)
- profile: string (optional, default: "standard")
- return_bboxes: boolean (optional, default: true)
```

### 8.2 レスポンス構造

```json
{
  "schema_version": "2025-12-29.1",
  "camera_id": "cam_2f_201",
  "captured_at": "2025-12-30T15:30:00Z",
  "analyzed": true,
  "detected": true,
  "primary_event": "human",
  "tags": ["gender.female", "age.adult", ...],
  "confidence": 0.52,
  "severity": 1,
  "count_hint": 3,
  "bboxes": [...],
  "person_details": [...],
  "suspicious": {
    "score": 0,
    "level": "normal",
    "factors": []
  },
  "context_applied": true,
  "filtered_by_context": null,
  "processing_ms": {
    "total": 150,
    "yolo": 60,
    "par": 25
  },
  "model_info": {
    "yolo": {"name": "yolov5s-640", "version": "2025.12.29"},
    "par": {"name": "par_resnet50_pa100k", "version": "2025.12.30", "enabled": true}
  }
}
```

---

## 9. 性能ベンチマーク

### 9.1 処理時間

| 処理 | 時間 |
|------|------|
| YOLO推論 | 40-70ms |
| PAR推論 (1人) | 20-30ms |
| PAR推論 (3人) | 60-90ms |
| 色分析 | 5-10ms |
| **合計 (1人)** | **100-150ms** |
| **合計 (3人)** | **150-250ms** |

### 9.2 検出精度

| 画像タイプ | 検出率 |
|-----------|--------|
| 近距離 (1-3m) | 95%+ |
| 中距離 (3-10m) | 85-95% |
| 遠距離 (10m+) | 70-85% |
| 夜間/IR | 80-90% |

---

## 10. 既知の制限事項

### 10.1 skin_tone推定

- IRカメラ/モノクロ画像では動作しない
- 可視光カラーカメラでのみ有効

### 10.2 姿勢推定

- bboxアスペクト比ベースのため精度に限界
- 部分的遮蔽時に誤判定の可能性

### 10.3 髪型推定

- 帽子着用時は "covered" 判定が優先
- 逆光・影では精度低下

### 10.4 quiet_hours時間形式

- 深夜跨ぎは分割指定が必要
- 例: `["23:00-24:00", "0:00-6:00"]`

---

## 11. バージョン履歴

| バージョン | 日付 | 主要変更 |
|-----------|------|----------|
| 1.6.0 | 2025-12-29 | 基本PAR実装 |
| 1.6.1 | 2025-12-29 | PAR修正・タグ拡充 |
| 1.7.0 | 2025-12-30 | 人物特徴強化 (skin_tone, hair, posture, suspicious_score) |
| 1.7.4 | 2025-12-30 | 検出閾値調整 (CONF=0.33) |
| **1.8.0** | 2025-12-30 | **camera_context対応** |

---

作成: Claude Code
