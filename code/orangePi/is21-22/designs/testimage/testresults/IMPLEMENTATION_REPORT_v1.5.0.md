# is21 v1.5.0 PAR統合 実装レポート

作成日: 2025-12-30
ステータス: **完了・動作確認済み**

---

## 1. 概要

is21 camimageEdge AIにPAR (Pedestrian Attribute Recognition) モデルを統合し、
YOLOv5sで検出した人物に対して属性認識を実行する**マルチモデルパイプライン**を実装した。

### 1.1 バージョン履歴

| バージョン | 内容 |
|-----------|------|
| v1.4.0 | YOLOv5s単体検出 |
| **v1.5.0** | **+ PAR統合 (属性認識)** |

---

## 2. 実装内容

### 2.1 追加モデル

| モデル | ファイル | サイズ | 推論時間 |
|--------|----------|--------|----------|
| PAR ResNet50 | `par_resnet50_pa100k.rknn` | 46MB | **19.65ms** (単体) |

### 2.2 追加ファイル

| ファイル | 用途 |
|----------|------|
| `/opt/is21/src/par_inference.py` | PAR推論モジュール |
| `/opt/is21/src/main.py` | 統合版メインサーバ (v1.5.0) |
| `/opt/is21/models/par_resnet50_pa100k.rknn` | PARモデル |

### 2.3 モデル変換手順

```
PaddlePaddle (strongbaseline_r50_30e_pa100k)
    ↓ paddle2onnx (Mac)
ONNX (90MB)
    ↓ rknn-toolkit2 (is21)
RKNN (46MB, FP16)
```

詳細: `models/PAR_MODEL_CONVERSION.md`

---

## 3. 推論パイプライン

```
入力画像 (1280x720)
    ↓
[Stage 1: YOLOv5s] ────────────────── ~60ms
    ├─ person bbox抽出
    ├─ vehicle bbox抽出
    └─ その他オブジェクト検出
    ↓
[Stage 2: PAR] ────────────────────── ~20ms x N人 (max 3)
    ├─ 各personをcrop (256x192)
    ├─ 26属性推論
    └─ schema.jsonタグマッピング
    ↓
[Stage 3: 画質分析] ────────────────── <5ms
    ├─ blur検出 (Laplacian)
    ├─ 明るさ分析
    └─ camera.*タグ生成
    ↓
[Stage 4: 集約] ────────────────────── <5ms
    ├─ 体格推定 (bbox比率)
    ├─ 人数カウント
    └─ タグ統合
    ↓
JSON出力
```

---

## 4. 対応タグ

### 4.1 PAR由来タグ

| タグ | PAR属性 | 閾値 |
|------|---------|------|
| `appearance.hat_like` | Hat (idx 0) | 0.5 |
| `appearance.glasses` | Glasses (idx 1) | 0.5 |
| `carry.backpack` | Backpack (idx 17) | 0.5 |
| `carry.bag` | HandBag or ShoulderBag | 0.5 |
| `carry.holding` | HoldObjectsInFront | 0.5 |
| `outfit.coat_like` | LongCoat (idx 10) | 0.5 |

### 4.2 その他タグ

| カテゴリ | タグ | 生成方法 |
|----------|------|----------|
| count | `count.single`, `count.multiple` | 人数カウント |
| body | `body.size.small/medium/large` | bbox面積比率 |
| camera | `camera.blur/dark/glare` | 画質分析 |

### 4.3 メタ情報 (per-person)

| フィールド | 値 |
|-----------|-----|
| `gender` | male / female |
| `age_group` | child_teen / adult / elderly |
| `upper_sleeve` | short / long |
| `lower_type` | trousers / shorts / skirt_dress |

---

## 5. テスト結果

### 5.1 屋外画像 (2人)

```json
{
  "primary_event": "human",
  "tags": [
    "outfit.coat_like",
    "carry.holding",
    "body.size.medium",
    "carry.bag",
    "carry.backpack",
    "appearance.hat_like",
    "appearance.glasses",
    "count.multiple"
  ],
  "count_hint": 2,
  "processing_ms": {
    "total": 149,
    "yolo": 59,
    "par": 45
  }
}
```

### 5.2 性能ベンチマーク

| 構成 | 処理時間 | 目標比 |
|------|----------|--------|
| YOLO単体 | 59ms | ✓ |
| YOLO + PAR (1人) | 59 + 22 = 81ms | ✓ |
| YOLO + PAR (2人) | 59 + 45 = 104ms | ✓ |
| YOLO + PAR (3人) | 59 + 60 ≈ 120ms | ✓ |
| 全Stage (3人) | ~150ms | **目標200ms以内達成** |

---

## 6. API変更点

### 6.1 `/healthz`

```json
{
  "status": "ok",
  "uptime_sec": 123,
  "par_enabled": true   // 追加
}
```

### 6.2 `/v1/capabilities`

```json
{
  "models": {
    "yolo": {...},
    "par": {
      "name": "par_resnet50_pa100k",
      "version": "2025.12.30",
      "enabled": true,
      "attributes": 26
    }
  },
  "supported_tags": {
    "count": ["count.single", "count.multiple"],
    "carry": ["carry.backpack", "carry.bag", "carry.holding"],
    "appearance": ["appearance.hat_like", "appearance.glasses"],
    "outfit": ["outfit.coat_like"],
    "body": ["body.size.small", "body.size.medium", "body.size.large"],
    "camera": ["camera.blur", "camera.dark", "camera.glare", ...]
  }
}
```

### 6.3 `/v1/analyze` レスポンス

```json
{
  "bboxes": [
    {
      "label": "person",
      "conf": 0.52,
      "par": {                    // 追加
        "tags": ["carry.backpack", ...],
        "meta": {
          "gender": "female",
          "age_group": "adult"
        }
      }
    }
  ],
  "processing_ms": {
    "total": 149,
    "yolo": 59,
    "par": 45                     // 追加
  }
}
```

---

## 7. 制限事項

### 7.1 PAR処理人数

- 最大3人まで (信頼度順)
- 超過時は高信頼度の3人のみ処理

### 7.2 未対応タグ

以下はPhase 2以降で対応予定:

| タグ | 必要技術 |
|------|---------|
| `top_color.*` | 上半身色分類 |
| `behavior.*` | 姿勢推定 + 時系列分析 |
| `hazard.*` | VLM or 専用モデル |

---

## 8. デプロイ状況

| 環境 | ステータス |
|------|-----------|
| is21本体 (192.168.3.116) | **稼働中** |
| ローカルソース | `code/orangePi/is21-22/src/` |
| モデル (ローカル) | `models/pretrained/` |
| モデル (is21) | `/opt/is21/models/` |

---

## 9. 次のステップ

### Phase 2 (予定)

1. **top_color.*** - 上半身色分類 (ColorHistogram or CNN)
2. **behavior.*** - MoveNet統合
3. **閾値チューニング** - 実運用データでの最適化

---

作成: Claude Code
検証: is21実機テスト完了
