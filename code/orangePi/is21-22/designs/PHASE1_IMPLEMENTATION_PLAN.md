# is21 Phase 1 実装計画書

作成日: 2025-12-30
ステータス: 計画中

---

## 1. 調査結果サマリー

### 1.1 RK3588 NPU 性能ベンチマーク

| モデル | サイズ | 推論時間 | FPS | 備考 |
|--------|--------|----------|-----|------|
| ResNet18 | 11.4MB | **4.09ms** | 244 | INT8量子化 |
| MobileNet V2 | ~3MB | **~2.2ms** | 452 | 推定値 |
| YOLOv5s | 8MB | **45ms** | 22 | 現行実測値 |
| ResNet50 | ~25MB | **15-20ms** | 50-66 | 推定値 |

**出典**: [TinyComputers.io RK3588 NPU Benchmarks](https://tinycomputers.io/posts/rockchip-rk3588-npu-benchmarks.html)

### 1.2 利用可能なPARモデル

| フレームワーク | バックボーン | PA100K mAP | 特徴 |
|---------------|-------------|------------|------|
| [Rethinking_of_PAR](https://github.com/valencebond/Rethinking_of_PAR) | ResNet50 | 80.21 | シンプル、PyTorch |
| [Rethinking_of_PAR](https://github.com/valencebond/Rethinking_of_PAR) | Swin-S | 82.19 | 高精度、重い |
| [OpenPAR](https://github.com/Event-AHU/OpenPAR) | CLIP-based | 85+ | 最新、複雑 |
| [dangweili/PAR](https://github.com/dangweili/pedestrian-attribute-recognition-pytorch) | ResNet50 | ~78 | 古典的実装 |

### 1.3 データセット属性カバレッジ

#### PA100K (26属性)
- 性別 (Female)
- 年齢 (Age18-60, Age>60)
- 服装 (Trousers, Shorts, LongCoat, etc.)
- 持ち物 (Back=リュック, Hand=手荷物)
- 帽子 (Hat)
- その他 (Glasses, UpperStride, etc.)

#### PETA (65属性)
- 性別・年齢
- 服装色 (11色分類)
- 服装タイプ
- アクセサリ

#### UPAR (統一40属性)
PA100K + PETA + RAP2 + Market1501を統合
- **推奨**: 汎用性が高い

---

## 2. 推奨アーキテクチャ

### 2.1 選定: ResNet50ベースPAR

| 選択肢 | 精度 | 速度 | 変換容易性 | 採用 |
|--------|------|------|-----------|------|
| ResNet50-PAR | 中-高 | 高 | ◎ | **✓** |
| Swin-PAR | 高 | 低 | △ | - |
| CLIP-PAR | 最高 | 低 | × | - |
| MobileNet-PAR | 低 | 最高 | ◎ | 予備 |

**理由**:
1. RKNN変換実績あり (ResNet系)
2. 推論時間 15-20ms で目標達成可能
3. PA100K mAP 80%以上を達成

### 2.2 推論パイプライン

```
入力画像 (1280x720)
    ↓
[Stage 1: YOLOv5s] ────────────────── 45ms
    ├─ person bbox抽出
    ├─ vehicle bbox抽出
    └─ animal bbox抽出
    ↓
[Stage 2: PAR (ResNet50)] ─────────── 20ms x N (最大3人)
    ├─ 各personをcrop (256x192)
    ├─ top_color.*
    ├─ carry.*
    ├─ body.size.*
    └─ appearance.*
    ↓
[Stage 3: Color Histogram] ────────── 5ms x M
    ├─ 各vehicleをcrop
    └─ 支配色分類 (6カテゴリ)
    ↓
[Stage 4: Image Quality] ──────────── 5ms
    ├─ Laplacian blur検出
    ├─ 輝度分析
    └─ camera.* タグ生成
    ↓
タグ統合 → JSON出力
```

### 2.3 推定処理時間

| 構成 | 処理時間 | 目標比 |
|------|----------|--------|
| YOLO単体 (現行) | 45ms | - |
| YOLO + PAR (1人) | 45 + 20 = 65ms | ◎ |
| YOLO + PAR (3人) | 45 + 60 = 105ms | ◎ |
| 全Stage (3人) | ~120ms | ◎ |

**目標200ms以内を達成可能**

---

## 3. Phase 1 実装タスク

### 3.1 Week 1: 環境構築・モデル準備

| # | タスク | 成果物 |
|---|--------|--------|
| 1.1 | rknn-toolkit2 インストール (PC) | 変換環境 |
| 1.2 | Rethinking_of_PAR クローン・動作確認 | 学習環境 |
| 1.3 | PA100K/UPAR データセット取得 | 学習データ |
| 1.4 | ResNet50-PAR 事前学習モデル取得 | .pth ファイル |

### 3.2 Week 2: ONNX/RKNN変換

| # | タスク | 成果物 |
|---|--------|--------|
| 2.1 | PyTorch → ONNX エクスポート | par_resnet50.onnx |
| 2.2 | ONNX 簡素化 (onnx-simplifier) | par_resnet50_sim.onnx |
| 2.3 | ONNX → RKNN 変換 | par_resnet50.rknn |
| 2.4 | is21でのRKNN動作検証 | 推論時間測定 |

### 3.3 Week 3: パイプライン統合

| # | タスク | 成果物 |
|---|--------|--------|
| 3.1 | マルチモデルローダー実装 | model_loader.py |
| 3.2 | Stage2 PAR推論ロジック | par_inference.py |
| 3.3 | bbox crop/resize 前処理 | preprocessor.py |
| 3.4 | タグマッピング (PAR出力→schema) | tag_mapper.py |

### 3.4 Week 4: テスト・調整

| # | タスク | 成果物 |
|---|--------|--------|
| 4.1 | 属性認識精度テスト | テストレポート |
| 4.2 | エンドツーエンド性能測定 | ベンチマーク結果 |
| 4.3 | 閾値チューニング | 設定ファイル |
| 4.4 | v1.5.0 リリース | デプロイ |

---

## 4. モデル変換手順 (詳細)

### 4.1 PyTorch → ONNX

```python
import torch
from models.base_block import FeatClassifier

# モデルロード
model = FeatClassifier(backbone='resnet50', num_classes=26)
model.load_state_dict(torch.load('exp_result/PA100K/resnet50/best_model.pth'))
model.eval()

# ダミー入力 (256x192 = PA100K標準)
dummy_input = torch.randn(1, 3, 256, 192)

# ONNX エクスポート
torch.onnx.export(
    model,
    dummy_input,
    "par_resnet50.onnx",
    input_names=['input'],
    output_names=['output'],
    dynamic_axes={'input': {0: 'batch'}, 'output': {0: 'batch'}},
    opset_version=12
)
```

### 4.2 ONNX 簡素化

```bash
pip install onnx-simplifier
python -m onnxsim par_resnet50.onnx par_resnet50_sim.onnx
```

### 4.3 ONNX → RKNN

```python
from rknn.api import RKNN

rknn = RKNN()

# ONNXモデルロード
rknn.config(
    mean_values=[[123.675, 116.28, 103.53]],
    std_values=[[58.395, 57.12, 57.375]],
    target_platform='rk3588'
)
rknn.load_onnx(model='par_resnet50_sim.onnx')

# ビルド (INT8量子化)
rknn.build(do_quantization=True, dataset='./dataset.txt')

# エクスポート
rknn.export_rknn('par_resnet50.rknn')
```

---

## 5. タグマッピング設計

### 5.1 PA100K → schema.json マッピング

| PA100K属性 | schema.json タグ | 備考 |
|------------|------------------|------|
| Female | (gender推定用) | 直接タグなし |
| Age18-60 | (age推定用) | 直接タグなし |
| AgeOver60 | (age推定用) | 直接タグなし |
| Hat | appearance.hat_like | |
| Back (backpack) | carry.backpack | |
| HandBag | carry.bag | |
| LongCoat | outfit.coat_like | |
| Trousers | (参考情報) | |
| Shorts | (参考情報) | |

### 5.2 色分類 → top_color.* マッピング

PETAデータセットの11色分類を使用:

| PETA色 | schema.json タグ |
|--------|------------------|
| black | top_color.black |
| white | top_color.white |
| gray | top_color.gray |
| red | top_color.red |
| blue | top_color.blue |
| yellow, orange, green, purple, pink, brown | top_color.other |

---

## 6. 推論コード設計

### 6.1 ファイル構成

```
/opt/is21/src/
├── main.py              # エントリポイント (既存)
├── yolo_inference.py    # Stage 1 (既存)
├── par_inference.py     # Stage 2 (新規)
├── color_classifier.py  # Stage 3 (新規)
├── image_quality.py     # Stage 4 (新規)
├── tag_mapper.py        # タグ統合 (新規)
├── model_loader.py      # マルチモデル管理 (新規)
└── models/
    ├── yolov5s-640-640.rknn
    └── par_resnet50.rknn  # 新規追加
```

### 6.2 par_inference.py スケルトン

```python
from rknnlite.api import RKNNLite
import cv2
import numpy as np

class PARInference:
    def __init__(self, model_path: str):
        self.rknn = RKNNLite()
        self.rknn.load_rknn(model_path)
        self.rknn.init_runtime(core_mask=RKNNLite.NPU_CORE_0_1_2)

        # PA100K属性リスト
        self.attributes = [
            'Female', 'AgeLess18', 'Age18-60', 'AgeOver60',
            'Front', 'Side', 'Back',
            'Hat', 'Glasses', 'HandBag', 'ShoulderBag', 'Backpack',
            'HoldObjectsInFront', 'ShortSleeve', 'LongSleeve',
            'UpperStride', 'UpperLogo', 'UpperPlaid', 'UpperSplice',
            'LowerStripe', 'LowerPattern', 'LongCoat', 'Trousers',
            'Shorts', 'Skirt&Dress', 'boots'
        ]

    def preprocess(self, img: np.ndarray, bbox: dict) -> np.ndarray:
        """bboxからcrop, 256x192にリサイズ"""
        h, w = img.shape[:2]
        x1 = int(bbox['x1'] * w)
        y1 = int(bbox['y1'] * h)
        x2 = int(bbox['x2'] * w)
        y2 = int(bbox['y2'] * h)

        crop = img[y1:y2, x1:x2]
        resized = cv2.resize(crop, (192, 256))  # W x H
        return resized

    def inference(self, img: np.ndarray) -> dict:
        """PAR推論を実行"""
        # RGB変換
        img_rgb = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)

        # 推論
        outputs = self.rknn.inference(inputs=[img_rgb])
        probs = self._sigmoid(outputs[0][0])

        return self._parse_attributes(probs)

    def _sigmoid(self, x):
        return 1 / (1 + np.exp(-x))

    def _parse_attributes(self, probs: np.ndarray) -> dict:
        """確率値をタグに変換"""
        tags = []
        threshold = 0.5

        # Hat → appearance.hat_like
        if probs[7] > threshold:
            tags.append('appearance.hat_like')

        # Backpack → carry.backpack
        if probs[11] > threshold:
            tags.append('carry.backpack')

        # HandBag/ShoulderBag → carry.bag
        if probs[9] > threshold or probs[10] > threshold:
            tags.append('carry.bag')

        # 体格推定 (bbox aspect ratio + 補助情報)
        # TODO: 別途実装

        return {'tags': tags, 'raw_probs': probs.tolist()}

    def release(self):
        self.rknn.release()
```

---

## 7. リスクと対策

| リスク | 影響 | 対策 |
|--------|------|------|
| RKNN変換失敗 | 高 | onnx-simplifier適用、opset調整 |
| 精度劣化 (INT8) | 中 | FP16モードで検証、必要なら混合精度 |
| メモリ不足 | 低 | バッチサイズ1固定、モデル逐次ロード |
| 処理時間超過 | 中 | PAR対象を最大3人に制限 |

---

## 8. 成功基準

| 指標 | 目標 | 測定方法 |
|------|------|----------|
| PAR推論時間 | ≤ 30ms | rknn.inference計測 |
| 服装色正解率 | ≥ 70% | 手動ラベル付きテスト画像 |
| 持ち物検出率 | ≥ 60% | 同上 |
| 総処理時間 (3人) | ≤ 150ms | エンドツーエンド計測 |

---

## 9. 次のステップ

1. **即座に開始**: Rethinking_of_PAR リポジトリのクローン
2. **is21への転送**: rknn-toolkit-lite2 確認
3. **PC環境**: rknn-toolkit2 (変換用) セットアップ
4. **データ取得**: PA100K ダウンロード申請

---

## 参考リンク

- [RKNN Model Zoo](https://github.com/airockchip/rknn_model_zoo)
- [RKNN-Toolkit2](https://github.com/rockchip-linux/rknn-toolkit2)
- [Rethinking_of_PAR](https://github.com/valencebond/Rethinking_of_PAR)
- [OpenPAR](https://github.com/Event-AHU/OpenPAR)
- [PA100K Dataset](https://github.com/xh-liu/HydraPlus-Net)
- [UPAR Dataset](https://github.com/speckean/upar_dataset)

---

作成: Claude Code
レビュー: 要
