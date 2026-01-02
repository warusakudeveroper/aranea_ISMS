# is21 設計見直し v2.0

作成日: 2025-12-30
ステータス: 要件ギャップ分析・再設計案

---

## 1. 現状の問題

### 1.1 根本的な機能欠落

現行実装 (v1.4.0) はYOLOv5s単体で物体検出のみを行っており、
**schema.jsonで定義された属性認識機能が全く実装されていない**。

#### 現行実装が返すもの
```json
{
  "primary_event": "human",
  "tags": ["count.multiple"],
  "count_hint": 2,
  "bboxes": [{"label": "person", "conf": 0.51, ...}]
}
```

#### 仕様で要求されているもの
```json
{
  "primary_event": "human",
  "tags": [
    "count.multiple",
    "top_color.black",
    "carry.backpack",
    "appearance.mask_like",
    "body.size.medium",
    "behavior.loitering"
  ],
  "count_hint": 2,
  "bboxes": [...]
}
```

### 1.2 欠落機能一覧

| カテゴリ | 要求タグ | 現状 | 必要な技術 |
|----------|---------|------|-----------|
| **服装色** | top_color.* (6種) | 未実装 | 色分類モデル |
| **持ち物** | carry.* (3種) | 未実装 | 物体検出拡張 or PAR |
| **外観** | appearance.* (4種) | 未実装 | 属性認識モデル |
| **服装タイプ** | outfit.* (7種) | 未実装 | 属性認識モデル |
| **体格** | body.size.* (3種) | 未実装 | 体格推定 |
| **体型** | body.build.* (1種) | 未実装 | 体格推定 |
| **行動** | behavior.* (2種) | 未実装 | 姿勢推定 + 時系列分析 |
| **動物分類** | animal.* (6種) | 部分的 | YOLO拡張 |
| **危険検知** | hazard.* (5種) | 未実装 | 専用モデル or VLM |
| **カメラ問題** | camera.* (6種) | 未実装 | 画質分析 |
| **在庫監視** | object_missing.* | 未実装 | 差分 + 背景モデル |
| **車両色** | (未定義だが要件) | 未実装 | 色分類モデル |
| **年齢/性別** | (未定義だが要件) | 未実装 | 顔/体属性モデル |

---

## 2. ハードウェア性能評価

### 2.1 RK3588 NPU スペック

| 項目 | 値 |
|------|-----|
| 演算性能 | **6 TOPS (INT8)** |
| コア数 | 3コア |
| メモリ | 8GB共有 |
| 現行モデル処理時間 | ~46ms (YOLOv5s) |

### 2.2 マルチモデル実行可能性

**結論: NPU性能的には複数モデルの追加実行は可能**

| 追加モデル案 | サイズ | 推定時間 | 合計時間 |
|-------------|--------|----------|----------|
| YOLOv5s (検出) | 8MB | 45ms | 45ms |
| + PAR (属性) | 5-10MB | 20-30ms | 65-75ms |
| + 色分類 | 2-5MB | 10-15ms | 75-90ms |
| + 姿勢推定 | 5-10MB | 15-25ms | 90-115ms |

**目標300ms以内は達成可能** (現在45ms + 追加モデル)

---

## 3. 再設計案

### 3.1 アーキテクチャ選択肢

#### 案A: マルチモデルパイプライン (推奨)

```
入力画像
    ↓
[YOLOv5s] → 物体検出 (person/car/animal bbox)
    ↓
[各bboxに対して]
    ├─ [PAR Model] → 服装色・持ち物・外観
    ├─ [Pose Model] → 姿勢 → 行動推定
    └─ [Color Hist] → 車両色
    ↓
タグ統合 → 出力
```

**メリット**:
- NPU内で完結、低レイテンシ
- オフライン動作可能
- コスト低（クラウドAPI不要）

**デメリット**:
- モデル準備・変換が必要
- 精度に限界あり

#### 案B: VLM (Vision Language Model) 併用

```
入力画像
    ↓
[YOLOv5s] → 物体検出 + primary_event
    ↓
[is22 経由で VLM に転送] (severity高/unknown時のみ)
    ↓
[Gemini Vision / GPT-4V] → 詳細属性
    ↓
タグ統合 → 出力
```

**メリット**:
- 高精度な属性認識
- 柔軟な判断（異常検知等）
- モデル準備不要

**デメリット**:
- API課金
- レイテンシ増加
- オフライン不可

#### 案C: ハイブリッド (推奨)

```
[is21 NPU]
    ├─ YOLOv5s → 物体検出
    ├─ PAR → 基本属性 (服装色・体格)
    └─ Pose → 姿勢・行動

[is22 or クラウド] (エスカレーション時のみ)
    └─ VLM → unknown確認・詳細分析
```

**メリット**:
- 通常はNPU完結 (低コスト・低レイテンシ)
- 困難なケースのみVLM (精度担保)
- 仕様のllm_reviewsテーブルと整合

---

### 3.2 推奨案: 案C (ハイブリッド)

#### Phase 1: NPUモデル拡張 (is21)

| モデル | 用途 | 出力タグ |
|--------|------|---------|
| YOLOv5s | 物体検出 | primary_event, bbox |
| PAR (Person Attribute Recognition) | 人物属性 | top_color.*, carry.*, appearance.*, outfit.*, body.* |
| MoveNet/PoseNet | 姿勢推定 | behavior.* |
| Color Classifier | 車両色 | (新規タグ追加) |
| Image Quality Analyzer | 画質判定 | camera.* |

#### Phase 2: VLMエスカレーション (is22)

- `unknown_flag=true` または `severity>=2` の場合のみ
- cooldown適用 (10分に1回)
- Gemini Vision API 使用

---

## 4. 必要な追加モデル

### 4.1 PAR (Person Attribute Recognition)

**候補モデル**:
| モデル | 精度 | サイズ | 備考 |
|--------|------|--------|------|
| OSNet-AIN | 高 | 8MB | ReID向けだが属性も |
| ResNet50-PAR | 中-高 | 25MB | PETA/PA100Kで学習 |
| MobileNetV3-PAR | 中 | 5MB | 軽量版 |

**出力属性** (学習データ依存):
- 上衣色 (9色)
- 下衣色 (9色)
- 性別 (male/female)
- 年齢帯 (child/young/adult/old)
- 持ち物 (backpack/bag/handbag)
- 帽子有無
- 眼鏡有無

### 4.2 姿勢推定

**候補モデル**:
| モデル | 用途 | サイズ |
|--------|------|--------|
| MoveNet Lightning | リアルタイム | 3MB |
| MoveNet Thunder | 高精度 | 7MB |
| PoseNet | 軽量 | 4MB |

**行動判定ロジック**:
```python
def detect_behavior(keypoints, prev_keypoints):
    # loitering: 位置変化小 + 時間経過
    if position_delta < threshold and duration > 60:
        return "behavior.loitering"

    # running: 脚の角速度高
    if leg_angular_velocity > run_threshold:
        return "behavior.running"

    return None
```

### 4.3 色分類

**手法**:
1. bbox内の色ヒストグラム計算
2. 上半身/下半身を分離
3. 支配的色を6カテゴリに分類 (red/blue/black/white/gray/other)

**または**:
- MobileNetV3 + FC (512→6) の軽量分類器

### 4.4 画質分析

**手法** (ルールベース):
```python
def analyze_image_quality(img):
    tags = []

    # blur: Laplacian variance
    if cv2.Laplacian(img, cv2.CV_64F).var() < blur_threshold:
        tags.append("camera.blur")

    # dark: 平均輝度
    if np.mean(img) < dark_threshold:
        tags.append("camera.dark")

    # glare: 過露出ピクセル率
    if np.sum(img > 250) / img.size > glare_threshold:
        tags.append("camera.glare")

    return tags
```

---

## 5. 実装計画

### Phase 1: 基盤整備 (1週目)

1. NPU対応PARモデルの調査・選定
2. RKNNへの変換テスト
3. マルチモデル推論パイプライン設計

### Phase 2: コアモデル実装 (2-3週目)

1. PAR (属性認識) 追加
   - 服装色 (top_color.*)
   - 体格 (body.size.*)
   - 持ち物 (carry.*)
   - 外観 (appearance.*)

2. 色分類追加
   - 車両色対応

3. 画質分析追加
   - camera.* タグ

### Phase 3: 行動認識 (4週目)

1. MoveNet統合
2. 行動判定ロジック
   - behavior.loitering
   - behavior.running

### Phase 4: VLMエスカレーション (5週目)

1. is22側でのVLM統合
2. クールダウン実装
3. llm_reviews記録

### Phase 5: 動物・危険検知 (6週目)

1. animal.*分類改善
2. hazard.*検知 (VLM依存)

---

## 6. 新しいテスト基準

### 6.1 属性認識テスト

| テスト項目 | 合格基準 |
|-----------|---------|
| 服装色正解率 | >= 70% |
| 持ち物検出率 | >= 60% |
| 体格推定正解率 | >= 65% |
| 行動検出率 | >= 50% |

### 6.2 統合テスト

| テスト項目 | 合格基準 |
|-----------|---------|
| primary_event正解率 | >= 95% |
| 必須タグ(notify_safe)出力率 | >= 80% |
| 誤検出率 | <= 10% |
| 平均レスポンス時間 | <= 200ms |

---

## 7. 結論

### 現状
- **YOLOv5s単体では仕様要件の約10%しか満たせていない**
- 「通過させるためのテスト」で合格判定を出していた
- 根本的な設計見直しが必要

### 推奨アクション
1. **ハイブリッドアーキテクチャ採用** (NPU + VLM)
2. **Phase 1-2を優先実装** (PAR + 色分類)
3. **テスト基準の厳格化** (属性レベルでの評価)

### ハードウェア制約
- **RK3588 NPUは十分な性能** (6 TOPS)
- マルチモデル実行しても目標200ms以内は達成可能
- 性能がボトルネックではなく、**モデル実装が未着手**なことが問題

---

## 付録: 優先実装タグ

### Tier 1 (必須・Phase 2)

| タグ | 理由 |
|------|------|
| top_color.* | 人物特定に重要 |
| carry.backpack/bag | 不審者判定に重要 |
| appearance.mask_like | セキュリティ上重要 |
| body.size.* | 人物区別に有用 |

### Tier 2 (重要・Phase 3)

| タグ | 理由 |
|------|------|
| behavior.loitering | 防犯上重要 |
| camera.blur/dark | 運用監視に必要 |

### Tier 3 (VLM依存・Phase 4+)

| タグ | 理由 |
|------|------|
| hazard.* | 高精度が必要 |
| outfit.* 詳細 | 判定が難しい |
| reason.unknown_activity | VLM確認必須 |

---

作成: Claude Code
レビュー: 要
