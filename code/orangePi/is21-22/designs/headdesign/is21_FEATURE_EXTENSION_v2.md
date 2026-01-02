# is21 特徴抽出拡張設計書 v2

## 方針
**モデル出力は可能な限り最大化**し、パフォーマンス問題が発生した場合に削減する。

---

## 1. 現状の出力項目

### 1.1 PA100K属性（26種 - モデル出力）
| カテゴリ | 属性 | 現在のタグ出力 |
|---------|------|---------------|
| 頭部 | Hat | appearance.hat_like |
| 頭部 | Glasses | appearance.glasses |
| 上半身 | ShortSleeve | meta.upper_sleeve |
| 上半身 | LongSleeve | meta.upper_sleeve |
| 上半身 | UpperStride | (未使用) |
| 上半身 | UpperLogo | uniform_like判定 |
| 上半身 | UpperPlaid | uniform_like判定 |
| 上半身 | UpperSplice | (未使用) |
| 下半身 | LowerStripe | (未使用) |
| 下半身 | LowerPattern | (未使用) |
| 服装 | LongCoat | outfit.coat_like |
| 服装 | Trousers | outfit.pants |
| 服装 | Shorts | outfit.shorts |
| 服装 | Skirt&Dress | outfit.dress |
| 服装 | boots | outfit.boots |
| 持ち物 | HandBag | carry.bag |
| 持ち物 | ShoulderBag | carry.bag |
| 持ち物 | Backpack | carry.backpack |
| 持ち物 | HoldObjectsInFront | carry.holding |
| 属性 | AgeOver60 | age.elderly |
| 属性 | Age18-60 | age.adult |
| 属性 | AgeLess18 | age.child_teen |
| 属性 | Female | gender.female/male |
| 向き | Front | (未使用) |
| 向き | Side | (未使用) |
| 向き | Back | (未使用) |

### 1.2 画像処理による追加特徴（実装済み）
| 特徴 | 関数 | タグ出力 |
|------|------|----------|
| 上半身色 | analyze_top_color | top_color.{color} |
| 下半身色 | analyze_bottom_color | bottom_color.{color} |
| 肌色 | detect_skin_tone | (person_detailsのみ) |
| 髪型 | detect_hair_type | hair.{type} |
| 体格 | estimate_body_size | body.size.{size} |
| 体型 | estimate_body_build | body.build.heavy |
| 姿勢 | estimate_posture | posture.{type} |
| 身長 | estimate_height_category | height.{category} |
| マスク | detect_mask_like | appearance.mask_like |
| 制服 | detect_uniform_like | appearance.uniform_like |

### 1.3 カメラ品質（実装済み）
| 特徴 | タグ出力 |
|------|----------|
| ブラー | camera.blur |
| 暗い | camera.dark |
| グレア | camera.glare |
| 露出不足 | camera.underexposed |
| 露出過多 | camera.overexposed |

---

## 2. 人物特徴系拡張

### 2.1 PA100K未使用属性の活用

```python
# 追加タグ出力
def extract_extended_par_tags(par_result: Dict) -> List[str]:
    tags = []
    raw = par_result.get('raw', {})
    threshold = 0.5

    # 向き (Front/Side/Back)
    if raw.get('Front', 0) > threshold:
        tags.append('facing.front')
    elif raw.get('Side', 0) > threshold:
        tags.append('facing.side')
    elif raw.get('Back', 0) > threshold:
        tags.append('facing.back')

    # 服装パターン
    if raw.get('UpperStride', 0) > threshold:
        tags.append('pattern.upper_stride')
    if raw.get('UpperSplice', 0) > threshold:
        tags.append('pattern.upper_splice')
    if raw.get('LowerStripe', 0) > threshold:
        tags.append('pattern.lower_stripe')
    if raw.get('LowerPattern', 0) > threshold:
        tags.append('pattern.lower_pattern')

    return tags
```

### 2.2 新規人物特徴

#### 2.2.1 髪の長さ推定
```python
def estimate_hair_length(image: np.ndarray, bbox: Dict) -> Optional[str]:
    """
    Returns: "hair.short", "hair.medium", "hair.long", "hair.bald"

    アプローチ: 頭部〜肩の領域で暗色ピクセルの分布を分析
    """
```

#### 2.2.2 眼鏡タイプ分類
```python
def classify_glasses_type(image: np.ndarray, bbox: Dict, has_glasses: bool) -> Optional[str]:
    """
    Returns: "glasses.regular", "glasses.sunglasses"

    アプローチ: 目元領域の明度分析（サングラスは暗い）
    """
```

#### 2.2.3 帽子タイプ分類
```python
def classify_hat_type(image: np.ndarray, bbox: Dict, has_hat: bool) -> Optional[str]:
    """
    Returns: "hat.cap", "hat.beanie", "hat.hood", "hat.helmet"

    アプローチ: 頭部領域の形状・色分布分析
    """
```

#### 2.2.4 バッグタイプ詳細
```python
def classify_bag_type(par_result: Dict) -> List[str]:
    """
    Returns: ["bag.handbag"], ["bag.shoulder"], ["bag.backpack"],
             ["bag.suitcase"], ["bag.multiple"]
    """
```

#### 2.2.5 歩行補助具検出
```python
def detect_mobility_aid(bboxes: List[Dict]) -> Optional[str]:
    """
    Returns: "mobility.cane", "mobility.wheelchair", "mobility.walker"

    アプローチ: person bbox周辺の細長いオブジェクト検出
    """
```

### 2.3 拡張person_details構造

```json
{
  "person_details": [
    {
      "index": 0,
      "bbox": {"x1": 0.1, "y1": 0.2, "x2": 0.5, "y2": 0.9},

      // 基本属性
      "gender": {"value": "female", "confidence": 0.85},
      "age_group": {"value": "adult", "confidence": 0.73},

      // 身体特徴
      "height_category": "tall",
      "body_size": "medium",
      "body_build": null,
      "posture": {"type": "standing", "confidence": 0.8},
      "facing": "front",

      // 頭部特徴
      "hair": {
        "color": "dark",
        "length": "medium",
        "confidence": 0.7
      },
      "hat": {
        "wearing": true,
        "type": "cap",
        "color": "black"
      },
      "glasses": {
        "wearing": true,
        "type": "sunglasses"
      },
      "mask": {
        "wearing": true,
        "confidence": 0.8
      },

      // 肌特徴
      "skin_tone": {"tone": "medium", "confidence": 0.6},

      // 服装詳細
      "upper_body": {
        "color": {"primary": "white", "confidence": 0.7},
        "sleeve": "long",
        "pattern": "solid",
        "uniform_like": false
      },
      "lower_body": {
        "color": {"primary": "blue", "confidence": 0.65},
        "type": "trousers",
        "pattern": "solid"
      },
      "footwear": {
        "type": "boots"
      },

      // 持ち物詳細
      "carrying": {
        "items": ["backpack", "holding_object"],
        "bag_type": "backpack",
        "bag_color": "black"
      },

      // PAR raw確率値
      "par_raw": {
        "Hat": 0.82,
        "Glasses": 0.12,
        // ... 全26属性
      }
    }
  ]
}
```

---

## 3. 状況系拡張

### 3.1 グループ分析

```python
def analyze_group_dynamics(bboxes: List[Dict]) -> Dict:
    """
    複数人物の関係性分析

    Returns:
        {
            "group_count": 2,
            "groups": [
                {"indices": [0, 1], "type": "pair", "distance": "close"},
                {"indices": [2], "type": "solo", "distance": null}
            ],
            "formation": "clustered" | "dispersed" | "line"
        }
    """
```

### 3.2 空間占有分析

```python
def analyze_spatial_occupation(bboxes: List[Dict], image_shape: tuple) -> Dict:
    """
    画面内の空間占有状況

    Returns:
        {
            "total_person_area_ratio": 0.35,
            "zones": {
                "left": {"count": 1, "area": 0.12},
                "center": {"count": 2, "area": 0.18},
                "right": {"count": 0, "area": 0.0}
            },
            "density": "moderate",
            "crowded": false
        }
    """
```

### 3.3 行動推定拡張

```python
def estimate_activity(bbox: Dict, prev_bbox: Optional[Dict], posture: str) -> Dict:
    """
    行動推定

    Returns:
        {
            "activity": "walking" | "standing" | "sitting" | "running" | "crouching",
            "confidence": 0.7,
            "direction": "toward_camera" | "away" | "lateral" | "stationary"
        }
    """
```

---

## 4. 前回画像差分系

### 4.1 camera_context拡張（is22→is21送信）

```json
{
  "previous_frame": {
    "captured_at": "2026-01-02T00:30:00Z",
    "person_count": 2,
    "person_positions": [
      {"center_x": 0.3, "center_y": 0.5, "area": 0.08},
      {"center_x": 0.7, "center_y": 0.6, "area": 0.05}
    ],
    "background_hash": "a1b2c3d4",
    "primary_event": "human"
  }
}
```

### 4.2 差分分析レスポンス

```python
def analyze_frame_diff(current_bboxes: List[Dict], prev_context: Dict) -> Dict:
    """
    前回フレームとの差分分析

    Returns:
        {
            "person_changes": {
                "appeared": 1,
                "disappeared": 0,
                "moved": 2,
                "stationary": 0
            },
            "movement_vectors": [
                {"index": 0, "dx": 0.05, "dy": -0.02, "magnitude": 0.054},
                {"index": 1, "dx": -0.1, "dy": 0.0, "magnitude": 0.1}
            ],
            "scene_change": {
                "level": "minor" | "moderate" | "major",
                "background_changed": false
            },
            "alerts": [
                {"type": "new_person", "location": "left"},
                {"type": "rapid_movement", "index": 1}
            ]
        }
    """
```

### 4.3 滞在検知

```python
def detect_loitering(current_bboxes: List[Dict], prev_context: Dict,
                     loitering_config: Dict) -> Dict:
    """
    滞在検知（連続フレームで同位置に人物が存在）

    Returns:
        {
            "loitering_detected": true,
            "loiterers": [
                {
                    "index": 0,
                    "position": {"x": 0.3, "y": 0.5},
                    "duration_estimate_sec": 45,
                    "in_zone": true
                }
            ]
        }
    """
```

---

## 5. カメラ障害系拡張

### 5.1 追加検出項目

```python
def analyze_camera_issues(image: np.ndarray, prev_image: Optional[np.ndarray] = None) -> Dict:
    """
    カメラ障害包括分析

    Returns:
        {
            "issues": ["camera.blur", "camera.partial_obstruction"],
            "details": {
                "blur": {"level": 0.7, "type": "motion"},
                "brightness": {"value": 85, "status": "normal"},
                "obstruction": {
                    "detected": true,
                    "coverage": 0.15,
                    "region": "bottom_right"
                },
                "tampering": {
                    "detected": false,
                    "confidence": 0.0
                },
                "focus": {
                    "quality": 0.6,
                    "degraded": true
                },
                "noise": {
                    "level": 0.2,
                    "status": "normal"
                }
            },
            "overall_quality": 0.65,
            "usable": true
        }
    """
```

### 5.2 新規検出タグ

| タグ | 説明 | 検出方法 |
|------|------|----------|
| `camera.obstructed` | 部分遮蔽 | エッジ検出 + 動き無し領域 |
| `camera.obstructed_severe` | 重度遮蔽 (>50%) | 上記 + 面積閾値 |
| `camera.tampered` | 改ざん疑い | 急激な全体変化 |
| `camera.focus_lost` | フォーカス喪失 | Laplacian分散の急激低下 |
| `camera.noise_high` | 高ノイズ | ノイズ推定 |
| `camera.frozen` | フリーズ疑い | 前回と完全一致 |
| `camera.shifted` | 画角ズレ | 背景特徴点の移動 |
| `camera.infrared` | 赤外線モード | 色相分布分析 |

### 5.3 実装

```python
def detect_obstruction(image: np.ndarray) -> Tuple[bool, float, Optional[str]]:
    """
    遮蔽検出

    Returns: (detected, coverage_ratio, region)

    アプローチ:
    1. エッジ検出で動きのない大きな領域を特定
    2. 単色ブロックの検出
    3. 画面分割して各領域のエントロピー分析
    """

def detect_tampering(image: np.ndarray, prev_image: np.ndarray) -> Tuple[bool, float]:
    """
    改ざん検出

    Returns: (detected, confidence)

    アプローチ:
    1. 前回画像との構造類似度 (SSIM)
    2. 急激な全体変化（レンズ塗りつぶし等）
    3. エッジ分布の異常
    """

def detect_camera_frozen(image: np.ndarray, prev_image: np.ndarray) -> bool:
    """
    フリーズ検出

    アプローチ: ピクセル差分が閾値以下
    """

def detect_camera_shift(image: np.ndarray, prev_image: np.ndarray) -> Tuple[bool, float]:
    """
    画角ズレ検出

    アプローチ: ORB特徴点マッチングで変換行列推定
    """
```

---

## 6. レスポンス構造拡張案

```json
{
  "schema_version": "2026-01-02.1",
  "camera_id": "cam-xxx",
  "captured_at": "2026-01-02T00:34:11Z",

  // 基本結果
  "analyzed": true,
  "detected": true,
  "primary_event": "human",
  "severity": 2,
  "confidence": 0.85,
  "count_hint": 2,

  // タグ（フラット配列 - 検索用）
  "tags": [
    "count.multiple",
    "gender.female", "gender.male",
    "age.adult",
    "top_color.white", "top_color.black",
    "bottom_color.blue",
    "outfit.pants", "outfit.coat_like",
    "carry.backpack", "carry.bag",
    "appearance.hat_like", "appearance.glasses",
    "posture.standing",
    "facing.front", "facing.side",
    "height.tall", "height.average",
    "pattern.upper_stride",
    "group.pair",
    "activity.walking"
  ],

  // 不審スコア
  "suspicious": {
    "score": 45,
    "level": "medium",
    "factors": [...]
  },

  // 検出ボックス（詳細付き）
  "bboxes": [...],

  // 人物詳細（2.3の構造）
  "person_details": [...],

  // グループ・空間分析
  "scene_analysis": {
    "group_dynamics": {...},
    "spatial_occupation": {...},
    "activity_summary": ["walking", "standing"]
  },

  // 差分分析（prev_frame送信時のみ）
  "frame_diff": {
    "person_changes": {...},
    "movement_vectors": [...],
    "loitering": {...},
    "scene_change": {...}
  },

  // カメラ状態
  "camera_status": {
    "issues": [],
    "quality": 0.85,
    "details": {...}
  },

  // コンテキスト適用結果
  "context_applied": true,
  "filtered_by_context": [...],

  // 処理時間
  "processing_ms": {
    "total": 320,
    "yolo": 38,
    "par": 27,
    "color": 15,
    "features": 40,
    "diff": 25,
    "camera_check": 10
  }
}
```

---

## 7. パフォーマンス削減オプション

問題発生時に無効化する順序（影響小→大）:

### Level 1: 軽微な削減
- `frame_diff` 分析スキップ（前回画像不要に）
- `scene_analysis.group_dynamics` スキップ
- `camera_status.details` 簡略化

### Level 2: 中程度の削減
- 髪の長さ/帽子タイプ/眼鏡タイプ詳細スキップ
- `par_raw` 出力省略
- パターン検出（stride/splice/stripe）スキップ

### Level 3: 大幅削減
- `person_details` を1人目のみに制限
- 色分析を上半身のみに制限
- 姿勢推定スキップ

### Level 4: 最小構成
- PA100K推論のみ（画像処理系全スキップ）
- タグは基本属性のみ

---

## 8. 実装優先度

| 項目 | 優先度 | 作業量 | 備考 |
|------|--------|--------|------|
| PA100K未使用属性活用 | 高 | 小 | facing, patternタグ追加 |
| カメラ障害拡張 | 高 | 中 | obstruction, tampering |
| frame_diff基盤 | 高 | 中 | is22からprev_frame送信 |
| 髪の長さ推定 | 中 | 小 | 既存detect_hair_type拡張 |
| 眼鏡/帽子タイプ | 中 | 小 | 分類追加 |
| グループ分析 | 中 | 中 | 複数人時のみ |
| 滞在検知 | 高 | 中 | corridor対応 |
| 歩行補助具検出 | 低 | 中 | 特殊ケース |

---

## 9. 次アクション

1. **is21修正**
   - PA100K未使用属性タグ化（facing, pattern）
   - カメラ障害拡張（obstruction, frozen）
   - prev_frame受け取り + frame_diff出力

2. **is22修正**
   - prev_frame context送信
   - 拡張レスポンス保存（detection_logs）

3. **テスト**
   - 全タグ出力確認
   - パフォーマンス計測
   - 削減レベル別動作確認
