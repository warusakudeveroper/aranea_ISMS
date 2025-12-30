"""
PAR (Pedestrian Attribute Recognition) 推論モジュール
is21 camimageEdge AI用

モデル: par_resnet50_pa100k.rknn (46MB, FP16)
入力: (1, 256, 192, 3) NHWC UINT8
出力: (1, 26) FP16 logits

作成日: 2025-12-30
"""
import logging
import numpy as np
import cv2
from typing import Optional, Dict, List, Any

try:
    from rknnlite.api import RKNNLite
    RKNN_AVAILABLE = True
except ImportError:
    RKNN_AVAILABLE = False
    RKNNLite = None

logger = logging.getLogger("is21.par")

# PA100K 26属性 (PaddlePaddle StrongBaseline順序)
PA100K_ATTRIBUTES = [
    'Hat',              # 0
    'Glasses',          # 1
    'ShortSleeve',      # 2
    'LongSleeve',       # 3
    'UpperStride',      # 4
    'UpperLogo',        # 5
    'UpperPlaid',       # 6
    'UpperSplice',      # 7
    'LowerStripe',      # 8
    'LowerPattern',     # 9
    'LongCoat',         # 10
    'Trousers',         # 11
    'Shorts',           # 12
    'Skirt&Dress',      # 13
    'boots',            # 14
    'HandBag',          # 15
    'ShoulderBag',      # 16
    'Backpack',         # 17
    'HoldObjectsInFront', # 18
    'AgeOver60',        # 19
    'Age18-60',         # 20
    'AgeLess18',        # 21
    'Female',           # 22
    'Front',            # 23
    'Side',             # 24
    'Back'              # 25
]

# 属性インデックス
IDX_HAT = 0
IDX_GLASSES = 1
IDX_SHORT_SLEEVE = 2
IDX_LONG_SLEEVE = 3
IDX_LONG_COAT = 10
IDX_TROUSERS = 11
IDX_SHORTS = 12
IDX_SKIRT = 13
IDX_HANDBAG = 15
IDX_SHOULDER_BAG = 16
IDX_BACKPACK = 17
IDX_HOLD_OBJECTS = 18
IDX_AGE_OVER60 = 19
IDX_AGE_18_60 = 20
IDX_AGE_LESS18 = 21
IDX_FEMALE = 22


class PARInference:
    """PAR推論クラス"""

    def __init__(self, model_path: str):
        self.model_path = model_path
        self.rknn: Optional[RKNNLite] = None
        self.loaded = False
        self.inference_count = 0
        self.total_ms = 0.0

    def load(self) -> bool:
        """モデルロード"""
        if not RKNN_AVAILABLE:
            logger.warning("RKNN not available, PAR disabled")
            return False

        try:
            self.rknn = RKNNLite()
            ret = self.rknn.load_rknn(self.model_path)
            if ret != 0:
                logger.error(f"Failed to load PAR model: {ret}")
                return False

            # NPU 3コア使用
            ret = self.rknn.init_runtime(core_mask=RKNNLite.NPU_CORE_0_1_2)
            if ret != 0:
                logger.error(f"Failed to init PAR runtime: {ret}")
                return False

            self.loaded = True
            logger.info(f"PAR model loaded: {self.model_path}")
            return True

        except Exception as e:
            logger.error(f"PAR load error: {e}")
            return False

    def preprocess(self, image: np.ndarray, bbox: Dict[str, float]) -> np.ndarray:
        """
        bboxから人物をcropし、256x192にリサイズ

        Args:
            image: 元画像 (H, W, 3) BGR
            bbox: {"x1": 0-1, "y1": 0-1, "x2": 0-1, "y2": 0-1} 正規化座標

        Returns:
            (1, 256, 192, 3) NHWC UINT8
        """
        h, w = image.shape[:2]

        # 正規化座標→ピクセル座標
        x1 = int(bbox['x1'] * w)
        y1 = int(bbox['y1'] * h)
        x2 = int(bbox['x2'] * w)
        y2 = int(bbox['y2'] * h)

        # 安全なクリップ
        x1 = max(0, min(x1, w - 1))
        y1 = max(0, min(y1, h - 1))
        x2 = max(x1 + 1, min(x2, w))
        y2 = max(y1 + 1, min(y2, h))

        # crop
        crop = image[y1:y2, x1:x2]

        if crop.size == 0:
            # 空のcropの場合は黒画像
            crop = np.zeros((256, 192, 3), dtype=np.uint8)
        else:
            # 256x192にリサイズ (H x W)
            crop = cv2.resize(crop, (192, 256))

        # バッチ次元追加 (1, 256, 192, 3)
        return crop[np.newaxis, ...]

    def inference(self, image: np.ndarray) -> Optional[np.ndarray]:
        """
        PAR推論実行

        Args:
            image: (1, 256, 192, 3) NHWC UINT8

        Returns:
            確率値配列 (26,) または None
        """
        if not self.loaded or self.rknn is None:
            return None

        import time
        start = time.time()

        try:
            outputs = self.rknn.inference(inputs=[image])
            elapsed_ms = (time.time() - start) * 1000

            self.inference_count += 1
            self.total_ms += elapsed_ms

            # logits → sigmoid
            logits = outputs[0].flatten()
            probs = self._sigmoid(logits)

            return probs

        except Exception as e:
            logger.error(f"PAR inference error: {e}")
            return None

    def _sigmoid(self, x: np.ndarray) -> np.ndarray:
        """sigmoid (オーバーフロー防止)"""
        return 1 / (1 + np.exp(-np.clip(x, -500, 500)))

    def parse_attributes(self, probs: np.ndarray, threshold: float = 0.5) -> Dict[str, Any]:
        """
        確率値をschema.jsonタグに変換

        Args:
            probs: (26,) 確率値
            threshold: 判定閾値

        Returns:
            {
                "tags": ["carry.backpack", ...],
                "raw": {"Hat": 0.12, ...},
                "meta": {"gender": "female", "age_group": "adult", ...}
            }
        """
        tags = []
        meta = {}

        # ===== schema.json タグマッピング =====

        # appearance.hat_like
        if probs[IDX_HAT] > threshold:
            tags.append('appearance.hat_like')

        # appearance.glasses (カスタム追加)
        if probs[IDX_GLASSES] > threshold:
            tags.append('appearance.glasses')

        # carry.backpack
        if probs[IDX_BACKPACK] > threshold:
            tags.append('carry.backpack')

        # carry.bag (HandBag or ShoulderBag)
        if probs[IDX_HANDBAG] > threshold or probs[IDX_SHOULDER_BAG] > threshold:
            tags.append('carry.bag')

        # carry.holding (HoldObjectsInFront)
        if probs[IDX_HOLD_OBJECTS] > threshold:
            tags.append('carry.holding')

        # outfit.coat_like
        if probs[IDX_LONG_COAT] > threshold:
            tags.append('outfit.coat_like')

        # ===== メタ情報 (直接タグにはしない) =====

        # 性別推定
        if probs[IDX_FEMALE] > threshold:
            meta['gender'] = 'female'
        else:
            meta['gender'] = 'male'

        # 年齢グループ推定
        age_probs = [probs[IDX_AGE_LESS18], probs[IDX_AGE_18_60], probs[IDX_AGE_OVER60]]
        age_labels = ['child_teen', 'adult', 'elderly']
        age_idx = np.argmax(age_probs)
        meta['age_group'] = age_labels[age_idx]
        meta['age_confidence'] = float(age_probs[age_idx])

        # 服装タイプ (参考情報)
        if probs[IDX_LONG_SLEEVE] > probs[IDX_SHORT_SLEEVE]:
            meta['upper_sleeve'] = 'long'
        else:
            meta['upper_sleeve'] = 'short'

        if probs[IDX_TROUSERS] > threshold:
            meta['lower_type'] = 'trousers'
        elif probs[IDX_SHORTS] > threshold:
            meta['lower_type'] = 'shorts'
        elif probs[IDX_SKIRT] > threshold:
            meta['lower_type'] = 'skirt_dress'
        else:
            meta['lower_type'] = 'unknown'

        # raw確率値
        raw = {PA100K_ATTRIBUTES[i]: float(probs[i]) for i in range(len(probs))}

        return {
            'tags': tags,
            'raw': raw,
            'meta': meta
        }

    def infer_person(self, image: np.ndarray, bbox: Dict[str, float],
                     threshold: float = 0.5) -> Dict[str, Any]:
        """
        単一人物の属性推論（preprocess + inference + parse）

        Args:
            image: 元画像 (H, W, 3) BGR
            bbox: {"x1", "y1", "x2", "y2"} 正規化座標
            threshold: 判定閾値

        Returns:
            parse_attributes()の結果 + inference_ms
        """
        import time
        start = time.time()

        # 前処理
        input_img = self.preprocess(image, bbox)

        # 推論
        probs = self.inference(input_img)

        if probs is None:
            return {
                'tags': [],
                'raw': {},
                'meta': {},
                'inference_ms': 0,
                'error': 'inference_failed'
            }

        # 属性解析
        result = self.parse_attributes(probs, threshold)
        result['inference_ms'] = int((time.time() - start) * 1000)

        return result

    def get_stats(self) -> Dict[str, Any]:
        """推論統計取得"""
        return {
            'loaded': self.loaded,
            'inference_count': self.inference_count,
            'total_ms': round(self.total_ms, 2),
            'avg_ms': round(self.total_ms / max(1, self.inference_count), 2)
        }

    def release(self):
        """リソース解放"""
        if self.rknn:
            self.rknn.release()
            self.rknn = None
            self.loaded = False
