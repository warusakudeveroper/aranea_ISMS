import sys; sys.path.insert(0, "/opt/is21/src")
"""
is21 推論サーバ + araneaDevice共通機能 + PAR統合 + 全属性対応
FastAPI + RKNNLite + AraneaCommon + PAR (Person Attribute Recognition)

v1.6.0: 設計図書準拠 全属性対応版
  - top_color.* 服装色分析（HSVヒストグラム）
  - appearance.mask_like 顔遮蔽検出
  - appearance.uniform_like 制服検出
  - outfit.* 服装タイプ拡充
  - body.build.* 体型推定
  - behavior.* 基礎行動検出（bbox位置ベース）
  - 年齢・性別のタグ化
"""
import logging
from fastapi import FastAPI, File, Form, UploadFile, HTTPException
from fastapi.responses import JSONResponse, FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel
from typing import Optional, List, Dict, Any, Tuple
import numpy as np
import cv2
import time
import os
import json
from datetime import datetime

# RKNN初期化を遅延させるためここでは条件付きインポート
try:
    from rknnlite.api import RKNNLite
    RKNN_AVAILABLE = True
except ImportError:
    RKNN_AVAILABLE = False
    RKNNLite = None

# PAR推論モジュール
from par_inference import PARInference

# araneaDevice共通モジュール
from aranea_common import (
    ConfigManager,
    LacisIdGenerator,
    AraneaRegister,
    TenantPrimaryAuth,
    HardwareInfo
)

# ログ設定
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s"
)
logger = logging.getLogger("is21")

# ===== 設定 =====
YOLO_MODEL_PATH = "/opt/is21/models/yolov5s-640-640.rknn"
PAR_MODEL_PATH = "/opt/is21/models/par_resnet50_pa100k.rknn"
SCHEMA_PATH = "/opt/is21/config/schema.json"
CONFIG_DIR = "/opt/is21/config"
INPUT_SIZE = 640
MAX_IMAGE_BYTES = 5_000_000
CONF_THRESHOLD = 0.33  # v1.7.4: バランス調整
NMS_THRESHOLD = 0.40   # 重複除去

# PAR設定
PAR_ENABLED = True
PAR_MAX_PERSONS = 10  # v1.7.1: 3→10 多人数対応
PAR_THRESHOLD = 0.5

# YOLOv5 anchors (COCO pretrained)
ANCHORS = [
    [[10, 13], [16, 30], [33, 23]],      # P3/8 (80x80)
    [[30, 61], [62, 45], [59, 119]],     # P4/16 (40x40)
    [[116, 90], [156, 198], [373, 326]]  # P5/32 (20x20)
]
STRIDES = [8, 16, 32]

# is21 デバイス設定
PRODUCT_TYPE = "021"
PRODUCT_CODE = "0001"
DEVICE_TYPE = "aranea_ar-is21"
DEVICE_NAME = "camimageEdge AI"
FIRMWARE_VERSION = "1.8.0"  # camera_context対応 (excluded_objects, location_type, busy/quiet_hours)

# システムテナント設定（ParaclateEdge用デフォルト）
# 実際の登録時はAPI経由で指定する
DEFAULT_TID = "T9999999999999999999"

# COCO クラス名（YOLOv5）
COCO_CLASSES = [
    "person", "bicycle", "car", "motorcycle", "airplane", "bus", "train", "truck",
    "boat", "traffic light", "fire hydrant", "stop sign", "parking meter", "bench",
    "bird", "cat", "dog", "horse", "sheep", "cow", "elephant", "bear", "zebra",
    "giraffe", "backpack", "umbrella", "handbag", "tie", "suitcase", "frisbee",
    "skis", "snowboard", "sports ball", "kite", "baseball bat", "baseball glove",
    "skateboard", "surfboard", "tennis racket", "bottle", "wine glass", "cup",
    "fork", "knife", "spoon", "bowl", "banana", "apple", "sandwich", "orange",
    "broccoli", "carrot", "hot dog", "pizza", "donut", "cake", "chair", "couch",
    "potted plant", "bed", "dining table", "toilet", "tv", "laptop", "mouse",
    "remote", "keyboard", "cell phone", "microwave", "oven", "toaster", "sink",
    "refrigerator", "book", "clock", "vase", "scissors", "teddy bear", "hair drier",
    "toothbrush"
]

# primary_event マッピング
EVENT_MAP = {
    "person": "human",
    "bicycle": "vehicle", "car": "vehicle", "motorcycle": "vehicle",
    "bus": "vehicle", "train": "vehicle", "truck": "vehicle", "boat": "vehicle",
    "bird": "animal", "cat": "animal", "dog": "animal", "horse": "animal",
    "sheep": "animal", "cow": "animal", "elephant": "animal", "bear": "animal",
    "zebra": "animal", "giraffe": "animal",
}

# 無視するオブジェクト（家具・日用品など）
IGNORE_OBJECTS = {
    "chair", "couch", "bed", "dining table", "toilet", "tv", "laptop",
    "remote", "keyboard", "cell phone", "microwave", "oven", "toaster",
    "sink", "refrigerator", "book", "clock", "vase", "scissors",
    "potted plant", "bottle", "wine glass", "cup", "fork", "knife",
    "spoon", "bowl", "bench", "traffic light", "fire hydrant",
    "stop sign", "parking meter"
}

# ===== 色分類定義 (HSV範囲) =====
# top_color.*: red, blue, black, white, gray, other
COLOR_RANGES = {
    "red": [
        ((0, 70, 50), (10, 255, 255)),     # 赤の低域
        ((170, 70, 50), (180, 255, 255))   # 赤の高域
    ],
    "blue": [
        ((100, 70, 50), (130, 255, 255))
    ],
    "black": [
        ((0, 0, 0), (180, 255, 50))
    ],
    "white": [
        ((0, 0, 200), (180, 30, 255))
    ],
    "gray": [
        ((0, 0, 50), (180, 30, 200))
    ],
    "green": [
        ((35, 70, 50), (85, 255, 255))
    ],
    "yellow": [
        ((20, 70, 50), (35, 255, 255))
    ]
}

# ===== グローバル変数 =====
app = FastAPI(title="is21 Inference Server", version="1.6.1")

# === AraneaWebUI 静的ファイル配信 ===
FRONTEND_DIR = "/opt/is21/frontend/dist"
if os.path.exists(FRONTEND_DIR):
    app.mount("/assets", StaticFiles(directory=os.path.join(FRONTEND_DIR, "assets")), name="static")

@app.get("/")
async def serve_root():
    """AraneaWebUI SPA root"""
    index_path = os.path.join(FRONTEND_DIR, "index.html")
    if os.path.exists(index_path):
        return FileResponse(index_path)
    return JSONResponse({"error": "Frontend not deployed", "hint": "Deploy frontend to /opt/is21/frontend/dist"})

rknn_yolo: Optional[RKNNLite] = None
par_inference: Optional[PARInference] = None
start_time = time.time()
schema_version: Optional[str] = None
schema_data: Optional[dict] = None
schema_received_at: Optional[str] = None

# araneaDevice共通
config_manager = ConfigManager(CONFIG_DIR)
lacis_generator = LacisIdGenerator(PRODUCT_TYPE, PRODUCT_CODE)
aranea_register = AraneaRegister(CONFIG_DIR)
hardware_info = HardwareInfo()

# 推論統計
inference_stats = {
    "yolo": {"total_count": 0, "success_count": 0, "error_count": 0, "total_ms": 0.0},
    "par": {"total_count": 0, "success_count": 0, "error_count": 0, "total_ms": 0.0},
    "color": {"total_count": 0, "total_ms": 0.0}
}


# ===== 色分析関数 =====

def analyze_top_color(image: np.ndarray, bbox: Dict[str, float]) -> Tuple[Optional[str], float]:
    """
    人物の上半身色を分析

    Args:
        image: BGR画像
        bbox: 正規化座標 {x1, y1, x2, y2}

    Returns:
        (color_name, confidence) - "red", "blue", "black", "white", "gray", "other"
    """
    h, w = image.shape[:2]

    # bbox座標をピクセルに変換
    x1 = int(bbox['x1'] * w)
    y1 = int(bbox['y1'] * h)
    x2 = int(bbox['x2'] * w)
    y2 = int(bbox['y2'] * h)

    # 上半身領域を抽出（bbox上部40%）
    body_height = y2 - y1
    upper_y2 = y1 + int(body_height * 0.4)

    # 境界チェック
    x1 = max(0, x1)
    y1 = max(0, y1)
    x2 = min(w, x2)
    upper_y2 = min(h, upper_y2)

    if x2 <= x1 or upper_y2 <= y1:
        return None, 0.0

    upper_body = image[y1:upper_y2, x1:x2]

    if upper_body.size == 0:
        return None, 0.0

    # HSV変換
    hsv = cv2.cvtColor(upper_body, cv2.COLOR_BGR2HSV)
    total_pixels = hsv.shape[0] * hsv.shape[1]

    if total_pixels == 0:
        return None, 0.0

    # 各色のピクセル数をカウント
    color_counts = {}

    for color_name, ranges in COLOR_RANGES.items():
        mask = None
        for (lower, upper) in ranges:
            lower = np.array(lower, dtype=np.uint8)
            upper = np.array(upper, dtype=np.uint8)
            range_mask = cv2.inRange(hsv, lower, upper)
            if mask is None:
                mask = range_mask
            else:
                mask = cv2.bitwise_or(mask, range_mask)

        if mask is not None:
            color_counts[color_name] = np.sum(mask > 0)

    # 最も多い色を選択
    if not color_counts:
        return "other", 0.3

    best_color = max(color_counts, key=color_counts.get)
    best_count = color_counts[best_color]
    confidence = best_count / total_pixels

    # 閾値チェック（20%以上で有効）
    if confidence < 0.2:
        return "other", confidence

    # schema.json対応（green, yellowはotherに）
    if best_color in ["green", "yellow"]:
        return "other", confidence

    return best_color, confidence


def analyze_vehicle_color(image: np.ndarray, bbox: Dict[str, float]) -> Tuple[Optional[str], float]:
    """
    車両の色を分析
    """
    h, w = image.shape[:2]

    x1 = int(bbox['x1'] * w)
    y1 = int(bbox['y1'] * h)
    x2 = int(bbox['x2'] * w)
    y2 = int(bbox['y2'] * h)

    x1 = max(0, x1)
    y1 = max(0, y1)
    x2 = min(w, x2)
    y2 = min(h, y2)

    if x2 <= x1 or y2 <= y1:
        return None, 0.0

    vehicle_region = image[y1:y2, x1:x2]

    if vehicle_region.size == 0:
        return None, 0.0

    hsv = cv2.cvtColor(vehicle_region, cv2.COLOR_BGR2HSV)
    total_pixels = hsv.shape[0] * hsv.shape[1]

    if total_pixels == 0:
        return None, 0.0

    color_counts = {}
    for color_name, ranges in COLOR_RANGES.items():
        mask = None
        for (lower, upper) in ranges:
            lower = np.array(lower, dtype=np.uint8)
            upper = np.array(upper, dtype=np.uint8)
            range_mask = cv2.inRange(hsv, lower, upper)
            if mask is None:
                mask = range_mask
            else:
                mask = cv2.bitwise_or(mask, range_mask)
        if mask is not None:
            color_counts[color_name] = np.sum(mask > 0)

    if not color_counts:
        return None, 0.0

    best_color = max(color_counts, key=color_counts.get)
    confidence = color_counts[best_color] / total_pixels

    if confidence < 0.15:
        return None, confidence

    return best_color, confidence


# ===== 顔・マスク検出 =====

def detect_mask_like(image: np.ndarray, bbox: Dict[str, float]) -> bool:
    """
    マスク着用の可能性を検出

    アプローチ:
    1. 顔領域（bbox上部25%）を抽出
    2. 顔領域の色分布を分析
    3. 肌色が少なく暗い色が多い場合 → mask_like
    """
    h, w = image.shape[:2]

    x1 = int(bbox['x1'] * w)
    y1 = int(bbox['y1'] * h)
    x2 = int(bbox['x2'] * w)
    y2 = int(bbox['y2'] * h)

    body_height = y2 - y1
    face_y2 = y1 + int(body_height * 0.25)

    x1 = max(0, x1)
    y1 = max(0, y1)
    x2 = min(w, x2)
    face_y2 = min(h, face_y2)

    if x2 <= x1 or face_y2 <= y1:
        return False

    face_region = image[y1:face_y2, x1:x2]

    if face_region.size == 0:
        return False

    # HSV変換
    hsv = cv2.cvtColor(face_region, cv2.COLOR_BGR2HSV)

    # 肌色範囲
    skin_lower = np.array([0, 20, 70], dtype=np.uint8)
    skin_upper = np.array([25, 180, 255], dtype=np.uint8)
    skin_mask = cv2.inRange(hsv, skin_lower, skin_upper)

    # 暗い色範囲（黒マスク、バラクラバなど）
    dark_lower = np.array([0, 0, 0], dtype=np.uint8)
    dark_upper = np.array([180, 255, 80], dtype=np.uint8)
    dark_mask = cv2.inRange(hsv, dark_lower, dark_upper)

    total_pixels = hsv.shape[0] * hsv.shape[1]
    skin_ratio = np.sum(skin_mask > 0) / total_pixels
    dark_ratio = np.sum(dark_mask > 0) / total_pixels

    # 肌色が少なく（<20%）暗い色が多い（>40%）場合 → mask_like
    if skin_ratio < 0.2 and dark_ratio > 0.4:
        return True

    return False


def detect_uniform_like(par_result: Dict) -> bool:
    """
    制服着用の可能性を検出（PAR属性ベース）

    判定基準:
    - UpperLogo または UpperPlaid が検出された場合
    """
    raw = par_result.get('raw', {})

    # PA100K属性インデックス
    upper_logo = raw.get('UpperLogo', 0)
    upper_plaid = raw.get('UpperPlaid', 0)

    if upper_logo > 0.5 or upper_plaid > 0.5:
        return True

    return False


# ===== v1.7 人物特徴強化 =====

def detect_skin_tone(image: np.ndarray, bbox: Dict[str, float]) -> Tuple[Optional[str], float]:
    """
    肌色推定 (顔領域のHSV分析)

    Returns:
        (tone, confidence) - "light", "medium", "dark" or None
    """
    h, w = image.shape[:2]

    # 顔領域（bbox上部25%）
    x1 = int(bbox['x1'] * w)
    y1 = int(bbox['y1'] * h)
    x2 = int(bbox['x2'] * w)
    body_height = int((bbox['y2'] - bbox['y1']) * h)
    face_y2 = y1 + int(body_height * 0.25)

    x1, y1 = max(0, x1), max(0, y1)
    x2 = min(w, x2)
    face_y2 = min(h, face_y2)

    if x2 <= x1 or face_y2 <= y1:
        return None, 0.0

    face_region = image[y1:face_y2, x1:x2]
    if face_region.size == 0:
        return None, 0.0

    # HSV変換
    hsv = cv2.cvtColor(face_region, cv2.COLOR_BGR2HSV)

    # 肌色範囲マスク (広めに取る)
    skin_mask = cv2.inRange(hsv, (0, 15, 50), (25, 170, 255))
    skin_pixels = face_region[skin_mask > 0]

    if len(skin_pixels) < 50:
        return None, 0.0

    # 肌色ピクセルの平均明度 (V channel)
    avg_v = np.mean(cv2.cvtColor(skin_pixels.reshape(-1, 1, 3), cv2.COLOR_BGR2HSV)[:, :, 2])

    # 信頼度 (肌色ピクセル比率)
    skin_ratio = len(skin_pixels) / face_region.size * 3
    confidence = min(skin_ratio, 0.9)

    if avg_v > 170:
        return "light", confidence
    elif avg_v > 110:
        return "medium", confidence
    else:
        return "dark", confidence


def detect_hair_type(image: np.ndarray, bbox: Dict[str, float]) -> Tuple[Optional[str], float]:
    """
    髪型推定 (頭部領域の色分析)

    Returns:
        (hair_type, confidence) - "dark", "light", "gray_white", "covered" or None
    """
    h, w = image.shape[:2]

    # 頭部領域（bbox最上部15%）
    x1 = int(bbox['x1'] * w)
    y1 = int(bbox['y1'] * h)
    x2 = int(bbox['x2'] * w)
    body_height = int((bbox['y2'] - bbox['y1']) * h)
    head_y2 = y1 + int(body_height * 0.15)

    x1, y1 = max(0, x1), max(0, y1)
    x2 = min(w, x2)
    head_y2 = min(h, head_y2)

    if x2 <= x1 or head_y2 <= y1:
        return None, 0.0

    head_region = image[y1:head_y2, x1:x2]
    if head_region.size == 0:
        return None, 0.0

    # HSV変換
    hsv = cv2.cvtColor(head_region, cv2.COLOR_BGR2HSV)
    h_channel = hsv[:, :, 0]
    s_channel = hsv[:, :, 1]
    v_channel = hsv[:, :, 2]

    avg_v = np.mean(v_channel)
    avg_s = np.mean(s_channel)

    # 帽子等で覆われている場合（彩度が高い＝カラフルな帽子）
    if avg_s > 80:
        return "covered", 0.7

    # 白髪/グレー
    if avg_v > 180 and avg_s < 30:
        return "gray_white", 0.7

    # 明るい髪（金髪等）
    if avg_v > 150 and avg_s < 60:
        return "light", 0.65

    # 暗い髪（黒髪）
    if avg_v < 100:
        return "dark", 0.75

    return "dark", 0.5  # デフォルト


def analyze_bottom_color(image: np.ndarray, bbox: Dict[str, float]) -> Tuple[Optional[str], float]:
    """
    下半身の服装色を分析

    Returns:
        (color_name, confidence) - "black", "blue", "gray", "white", "other"
    """
    h, w = image.shape[:2]

    x1 = int(bbox['x1'] * w)
    y1 = int(bbox['y1'] * h)
    x2 = int(bbox['x2'] * w)
    y2 = int(bbox['y2'] * h)

    # 下半身領域（bbox下部50%）
    body_height = y2 - y1
    lower_y1 = y1 + int(body_height * 0.5)

    x1, lower_y1 = max(0, x1), max(0, lower_y1)
    x2, y2 = min(w, x2), min(h, y2)

    if x2 <= x1 or y2 <= lower_y1:
        return None, 0.0

    lower_region = image[lower_y1:y2, x1:x2]
    if lower_region.size == 0:
        return None, 0.0

    hsv = cv2.cvtColor(lower_region, cv2.COLOR_BGR2HSV)

    # 各色のピクセル数をカウント
    color_counts = {}
    total_pixels = lower_region.shape[0] * lower_region.shape[1]

    for color_name, ranges in COLOR_RANGES.items():
        count = 0
        for (lower, upper) in ranges:
            mask = cv2.inRange(hsv, lower, upper)
            count += np.count_nonzero(mask)
        color_counts[color_name] = count

    # 最も支配的な色
    if not color_counts:
        return None, 0.0

    max_color = max(color_counts, key=color_counts.get)
    max_ratio = color_counts[max_color] / total_pixels

    if max_ratio < 0.15:
        return "other", 0.5

    return max_color, min(max_ratio * 2, 0.95)


def estimate_posture(bbox: Dict[str, float], image_shape: tuple) -> Tuple[Optional[str], float]:
    """
    姿勢推定 (bboxアスペクト比ベース)

    Returns:
        (posture, confidence) - "standing", "sitting", "crouching" or None
    """
    h, w = image_shape[:2]

    bbox_width = (bbox['x2'] - bbox['x1']) * w
    bbox_height = (bbox['y2'] - bbox['y1']) * h

    if bbox_height == 0:
        return None, 0.0

    aspect_ratio = bbox_width / bbox_height

    # 立位: 細長い (aspect < 0.45)
    # 座位: やや横長 (0.45 < aspect < 0.7)
    # しゃがみ: 横長 (aspect > 0.7)

    if aspect_ratio < 0.4:
        return "standing", 0.75
    elif aspect_ratio < 0.55:
        return "standing", 0.55  # 立位だが確信度低
    elif aspect_ratio < 0.75:
        return "sitting", 0.65
    else:
        return "crouching", 0.6


def estimate_height_category(bbox: Dict[str, float], image_shape: tuple) -> Optional[str]:
    """
    相対身長推定 (画像内でのbbox高さ比)

    Returns:
        "tall", "average", "short" or None
    """
    h, w = image_shape[:2]
    bbox_height = (bbox['y2'] - bbox['y1']) * h

    # 画像高さに対する割合
    height_ratio = bbox_height / h

    # 画面上部から下部まで写っているか確認
    if bbox['y1'] > 0.3:  # 上部が切れている場合は判定しない
        return None

    if height_ratio > 0.7:
        return "tall"
    elif height_ratio > 0.4:
        return "average"
    elif height_ratio > 0.15:
        return "short"
    return None


def parse_camera_context(hints_json: Optional[str]) -> Dict[str, Any]:
    """
    hints_jsonからcamera_contextをパース

    Args:
        hints_json: JSON文字列

    Returns:
        camera_context dict or empty dict
    """
    if not hints_json:
        return {}

    try:
        hints = json.loads(hints_json)
        return hints.get("camera_context", hints)  # camera_contextキーがあればそれを、なければ全体を返す
    except json.JSONDecodeError:
        logger.warning(f"Invalid hints_json: {hints_json[:100]}")
        return {}


def apply_context_filters(bboxes: List[Dict], context: Dict) -> Tuple[List[Dict], List[str]]:
    """
    camera_contextに基づいてbboxesをフィルタリング

    Args:
        bboxes: 検出結果リスト
        context: camera_context

    Returns:
        (filtered_bboxes, filtered_reasons)
    """
    if not context:
        return bboxes, []

    excluded = context.get("excluded_objects", [])
    expected = context.get("expected_objects", [])
    detection_roi = context.get("detection_roi")

    filtered = []
    reasons = []

    for bbox in bboxes:
        label = bbox.get("label", "")

        # excluded_objectsチェック
        if label in excluded:
            reasons.append(f"{label} excluded by context")
            continue

        # expected_objectsチェック (指定がある場合のみ)
        if expected and label not in expected:
            # ただし confidence が高い場合は残す
            if bbox.get("conf", 0) < 0.5:
                reasons.append(f"{label} not in expected_objects")
                continue

        # ROIチェック (指定がある場合)
        if detection_roi:
            cx = (bbox.get("x1", 0) + bbox.get("x2", 0)) / 2
            cy = (bbox.get("y1", 0) + bbox.get("y2", 0)) / 2
            if not (detection_roi.get("x1", 0) <= cx <= detection_roi.get("x2", 1) and
                    detection_roi.get("y1", 0) <= cy <= detection_roi.get("y2", 1)):
                reasons.append(f"{label} outside ROI")
                continue

        filtered.append(bbox)

    return filtered, reasons


def get_adjusted_threshold(context: Dict, base_threshold: float = 0.33) -> float:
    """
    camera_contextに基づいて閾値を調整

    Args:
        context: camera_context
        base_threshold: ベース閾値

    Returns:
        調整後の閾値
    """
    if not context:
        return base_threshold

    distance = context.get("distance", "medium")
    conf_override = context.get("conf_override")

    # 明示的な閾値指定があればそれを使用
    if conf_override is not None:
        return max(0.2, min(0.8, conf_override))

    # distanceに基づく調整
    if distance == "near":
        return base_threshold - 0.05  # 近距離は緩和
    elif distance == "far":
        return base_threshold + 0.05  # 遠距離は厳格化

    return base_threshold


def calculate_suspicious_score(tags: List[str], context: Dict = None) -> Dict[str, Any]:
    """
    不審者スコア計算

    Args:
        tags: 検出されたタグリスト
        context: {"hour": 0-23, "is_night": bool, "location_type": str, ...}

    Returns:
        {"score": 0-100, "level": str, "factors": [str]}
    """
    if context is None:
        context = {}

    score = 0
    factors = []

    # === 最重要: 顔隠し ===
    if "appearance.mask_like" in tags:
        score += 40
        factors.append("appearance.mask_like (+40)")

    # === 時間帯 ===
    hour = context.get('hour')
    is_night = context.get('is_night', False)

    if hour is not None:
        if 2 <= hour < 5:
            score += 25
            factors.append(f"time.late_night[{hour}:00] (+25)")
        elif 0 <= hour < 6 or hour >= 22:
            score += 10
            factors.append(f"time.night[{hour}:00] (+10)")

    # === 夜間 + 暗い服装 ===
    if is_night:
        if "top_color.black" in tags:
            score += 15
            factors.append("night+black_clothing (+15)")
        elif "top_color.gray" in tags:
            score += 10
            factors.append("night+dark_clothing (+10)")

        if "appearance.hat_like" in tags:
            score += 5
            factors.append("night+hat (+5)")

    # === 場所 ===
    location_type = context.get('location_type', '')
    if location_type == 'restricted':
        score += 20
        factors.append("restricted_area (+20)")
    elif location_type == 'entrance':
        # 入口は通常の往来があるので減点
        score -= 5
        factors.append("entrance_area (-5)")
    elif location_type == 'corridor':
        # 廊下: 滞在時間が長いと不審
        pass  # 将来的に滞在時間判定を追加
    elif location_type == 'parking':
        # 駐車場: 夜間は警戒
        if is_night:
            score += 10
            factors.append("parking+night (+10)")

    # === busy_hours / quiet_hours ===
    busy_hours = context.get('busy_hours', [])
    quiet_hours = context.get('quiet_hours', [])

    def is_in_time_range(h, ranges):
        for r in ranges:
            if '-' in r:
                start, end = r.split('-')
                sh, eh = int(start.split(':')[0]), int(end.split(':')[0])
                if sh <= h < eh:
                    return True
        return False

    if hour is not None:
        if busy_hours and is_in_time_range(hour, busy_hours):
            # 繁忙時間帯は警戒レベル下げ
            score -= 10
            factors.append("busy_hours (-10)")
        elif quiet_hours and is_in_time_range(hour, quiet_hours):
            # 静寂時間帯は警戒レベル上げ
            score += 15
            factors.append("quiet_hours (+15)")

    # === 荷物 (深夜) ===
    if hour is not None and (0 <= hour < 6):
        if "carry.backpack" in tags:
            score += 8
            factors.append("late_night+backpack (+8)")

    # === 姿勢 ===
    if "posture.crouching" in tags:
        score += 10
        factors.append("crouching_posture (+10)")

    # スコア上限
    score = min(score, 100)

    # レベル判定
    if score >= 70:
        level = "critical"
    elif score >= 50:
        level = "high"
    elif score >= 30:
        level = "medium"
    elif score >= 15:
        level = "low"
    else:
        level = "normal"

    return {
        "score": score,
        "level": level,
        "factors": factors
    }


# ===== 体型推定 =====

def estimate_body_build(bbox: Dict[str, float], image_shape: tuple) -> Optional[str]:
    """
    体型推定 (bbox比率ベース)

    Returns:
        "body.build.heavy" or None
    """
    h, w = image_shape[:2]

    bbox_width = (bbox['x2'] - bbox['x1']) * w
    bbox_height = (bbox['y2'] - bbox['y1']) * h

    if bbox_height == 0:
        return None

    # アスペクト比（幅/高さ）
    aspect_ratio = bbox_width / bbox_height

    # 通常の人物は 0.3-0.5 程度
    # 横幅が広い場合 heavy と判定
    if aspect_ratio > 0.55:
        return "body.build.heavy"

    return None


def estimate_body_size(bbox: Dict[str, float], image_shape: tuple) -> Optional[str]:
    """
    体格推定 (bboxサイズベース)
    """
    h, w = image_shape[:2]
    bbox_width = (bbox['x2'] - bbox['x1']) * w
    bbox_height = (bbox['y2'] - bbox['y1']) * h
    bbox_area = bbox_width * bbox_height
    image_area = h * w

    ratio = bbox_area / image_area

    if ratio < 0.02:
        return "body.size.small"
    elif ratio < 0.15:
        return "body.size.medium"
    else:
        return "body.size.large"


# ===== 行動推定 =====

def estimate_behavior(bbox: Dict[str, float], image_shape: tuple) -> List[str]:
    """
    行動推定（静止画ベースの限定的な推定）

    注意: 本格的な行動認識には姿勢推定モデル + 時系列分析が必要
    ここでは bbox 位置に基づく簡易推定のみ

    Returns:
        behavior.* タグのリスト
    """
    tags = []

    h, w = image_shape[:2]

    # bbox の位置とサイズ
    bbox_x_center = (bbox['x1'] + bbox['x2']) / 2
    bbox_y_center = (bbox['y1'] + bbox['y2']) / 2
    bbox_height = bbox['y2'] - bbox['y1']
    bbox_width = bbox['x2'] - bbox['x1']

    # 画面端にいる場合（進入/退出の可能性）
    if bbox['x1'] < 0.05 or bbox['x2'] > 0.95:
        # 画面端は行動タグ付与の可能性があるが、静止画では判断困難
        pass

    # アスペクト比で姿勢推定（非常に簡易的）
    if bbox_height > 0:
        aspect = bbox_width / bbox_height

        # 座っている可能性（通常より幅広）
        if aspect > 0.6 and bbox_y_center > 0.5:
            tags.append("behavior.sitting")

    return tags


# ===== outfit タグ拡充 =====

def extract_outfit_tags(par_result: Dict) -> List[str]:
    """
    PAR結果からoutfit.*タグを抽出
    """
    tags = []
    raw = par_result.get('raw', {})
    threshold = 0.5

    # coat_like は既にpar_inference.pyで処理されている

    # shorts
    if raw.get('Shorts', 0) > threshold:
        tags.append('outfit.shorts')

    # dress (Skirt&Dress)
    if raw.get('Skirt&Dress', 0) > threshold:
        tags.append('outfit.dress')

    # trousers → outfit.pants として追加
    if raw.get('Trousers', 0) > threshold:
        tags.append('outfit.pants')

    # boots
    if raw.get('boots', 0) > threshold:
        tags.append('outfit.boots')

    return tags


# ===== 年齢・性別タグ化 =====

def extract_demographic_tags(par_result: Dict) -> List[str]:
    """
    PAR結果から年齢・性別タグを抽出
    """
    tags = []
    meta = par_result.get('meta', {})

    # 性別
    gender = meta.get('gender')
    if gender:
        tags.append(f"gender.{gender}")  # gender.male, gender.female

    # 年齢グループ
    age_group = meta.get('age_group')
    if age_group:
        tags.append(f"age.{age_group}")  # age.child_teen, age.adult, age.elderly

    return tags


# ===== 画質分析 =====

def analyze_image_quality(image: np.ndarray) -> List[str]:
    """画像品質分析"""
    tags = []
    gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)

    # blur検出
    laplacian_var = cv2.Laplacian(gray, cv2.CV_64F).var()
    if laplacian_var < 100:
        tags.append("camera.blur")

    # 明るさ
    mean_brightness = np.mean(gray)
    if mean_brightness < 40:
        tags.append("camera.dark")
    elif mean_brightness > 230:
        tags.append("camera.glare")

    # 露出
    hist = cv2.calcHist([gray], [0], None, [256], [0, 256])
    low_ratio = np.sum(hist[:20]) / np.sum(hist)
    high_ratio = np.sum(hist[-20:]) / np.sum(hist)

    if low_ratio > 0.5:
        tags.append("camera.underexposed")
    elif high_ratio > 0.5:
        tags.append("camera.overexposed")

    return tags


# ===== 初期化関数 =====

def load_schema_from_file():
    global schema_version, schema_data, schema_received_at
    if os.path.exists(SCHEMA_PATH):
        try:
            with open(SCHEMA_PATH, "r") as f:
                data = json.load(f)
                schema_version = data.get("schema_version")
                schema_data = data.get("schema")
                schema_received_at = data.get("received_at")
                logger.info(f"Schema loaded from file: {schema_version}")
        except Exception as e:
            logger.error(f"Failed to load schema: {e}")


def save_schema_to_file():
    try:
        os.makedirs(os.path.dirname(SCHEMA_PATH), exist_ok=True)
        with open(SCHEMA_PATH, "w") as f:
            json.dump({
                "schema_version": schema_version,
                "schema": schema_data,
                "received_at": schema_received_at
            }, f, indent=2)
        logger.info(f"Schema saved to file: {schema_version}")
    except Exception as e:
        logger.error(f"Failed to save schema: {e}")


def init_rknn():
    global rknn_yolo
    if not RKNN_AVAILABLE:
        logger.warning("RKNN not available, running in mock mode")
        return

    rknn_yolo = RKNNLite()
    ret = rknn_yolo.load_rknn(YOLO_MODEL_PATH)
    if ret != 0:
        raise RuntimeError(f"Failed to load YOLO model: {ret}")
    ret = rknn_yolo.init_runtime(core_mask=RKNNLite.NPU_CORE_0)
    if ret != 0:
        raise RuntimeError(f"Failed to init YOLO runtime: {ret}")
    logger.info(f"YOLO model loaded: {YOLO_MODEL_PATH}")
    hardware_info.set_model_loaded(True, "yolov5s-640-640.rknn")


def init_par():
    global par_inference
    if not PAR_ENABLED:
        logger.info("PAR disabled by config")
        return

    if not os.path.exists(PAR_MODEL_PATH):
        logger.warning(f"PAR model not found: {PAR_MODEL_PATH}")
        return

    par_inference = PARInference(PAR_MODEL_PATH)
    if par_inference.load():
        logger.info("PAR model loaded successfully")
    else:
        logger.warning("Failed to load PAR model")
        par_inference = None


def init_aranea_device():
    """
    デバイス初期化
    テナントプライマリ認証はCloud tabからのアクティベート時に動的に設定される
    """
    config_manager.begin("is21")
    lacis_id = lacis_generator.generate()
    logger.info(f"Device lacisId: {lacis_id}")
    logger.info(f"Device MAC: {lacis_generator.get_mac_formatted()}")

    # 登録状態を確認（テナント認証は/api/cloud/activateで動的に設定）
    if aranea_register.is_registered():
        logger.info(f"Device registered: CIC={aranea_register.get_saved_cic()}")
        logger.info(f"TID: {aranea_register.get_saved_tid()}")
    else:
        logger.info("Device not registered - use WebUI Cloud tab to activate")


# ===== YOLO後処理 =====

def sigmoid(x):
    return 1 / (1 + np.exp(-np.clip(x, -500, 500)))


def postprocess_yolov5(outputs, img_shape, conf_threshold: float = None, nms_threshold: float = None):
    """
    YOLOv5出力の後処理

    Args:
        outputs: RKNN推論出力
        img_shape: 元画像サイズ (h, w)
        conf_threshold: 信頼度閾値 (None=デフォルトCONF_THRESHOLD使用)
        nms_threshold: NMS閾値 (None=デフォルトNMS_THRESHOLD使用)
    """
    # デフォルト値の適用
    effective_conf = conf_threshold if conf_threshold is not None else CONF_THRESHOLD
    effective_nms = nms_threshold if nms_threshold is not None else NMS_THRESHOLD

    all_boxes = []
    all_scores = []
    all_classes = []

    for idx, output in enumerate(outputs):
        batch, channels, h, w = output.shape
        num_anchors = 3
        num_attrs = 85

        output = output.reshape(batch, num_anchors, num_attrs, h, w)
        output = output.transpose(0, 1, 3, 4, 2)
        output = output[0]

        stride = STRIDES[idx]
        anchors = np.array(ANCHORS[idx])

        grid_y, grid_x = np.meshgrid(np.arange(h), np.arange(w), indexing='ij')

        for anchor_idx in range(num_anchors):
            anchor_w, anchor_h = anchors[anchor_idx]
            pred = output[anchor_idx]

            tx = pred[:, :, 0]
            ty = pred[:, :, 1]
            tw = pred[:, :, 2]
            th = pred[:, :, 3]

            cx = (sigmoid(tx) * 2 - 0.5 + grid_x) * stride
            cy = (sigmoid(ty) * 2 - 0.5 + grid_y) * stride
            bw = (sigmoid(tw) * 2) ** 2 * anchor_w
            bh = (sigmoid(th) * 2) ** 2 * anchor_h

            obj_conf = sigmoid(pred[:, :, 4])
            class_scores = sigmoid(pred[:, :, 5:])
            scores = obj_conf[:, :, np.newaxis] * class_scores

            mask = scores.max(axis=2) > effective_conf

            if mask.sum() == 0:
                continue

            cx_masked = cx[mask]
            cy_masked = cy[mask]
            bw_masked = bw[mask]
            bh_masked = bh[mask]
            scores_masked = scores[mask]

            x1 = np.clip((cx_masked - bw_masked / 2) / INPUT_SIZE, 0, 1)
            y1 = np.clip((cy_masked - bh_masked / 2) / INPUT_SIZE, 0, 1)
            x2 = np.clip((cx_masked + bw_masked / 2) / INPUT_SIZE, 0, 1)
            y2 = np.clip((cy_masked + bh_masked / 2) / INPUT_SIZE, 0, 1)

            for i in range(len(x1)):
                class_id = np.argmax(scores_masked[i])
                score = scores_masked[i, class_id]
                all_boxes.append([x1[i], y1[i], x2[i], y2[i]])
                all_scores.append(score)
                all_classes.append(class_id)

    if len(all_boxes) == 0:
        return [], [], []

    all_boxes = np.array(all_boxes)
    all_scores = np.array(all_scores)
    all_classes = np.array(all_classes)

    boxes_pixel = all_boxes * INPUT_SIZE
    boxes_xywh = np.zeros_like(boxes_pixel)
    boxes_xywh[:, 0] = boxes_pixel[:, 0]
    boxes_xywh[:, 1] = boxes_pixel[:, 1]
    boxes_xywh[:, 2] = boxes_pixel[:, 2] - boxes_pixel[:, 0]
    boxes_xywh[:, 3] = boxes_pixel[:, 3] - boxes_pixel[:, 1]

    indices = cv2.dnn.NMSBoxes(
        boxes_xywh.tolist(),
        all_scores.tolist(),
        effective_conf,
        effective_nms
    )

    if len(indices) == 0:
        return [], [], []

    indices = indices.flatten()
    return all_boxes[indices], all_scores[indices], all_classes[indices]


# ===== 含有重複除去 =====

def remove_contained_boxes(bboxes: List[Dict], containment_threshold: float = 0.7) -> List[Dict]:
    """
    一方のbboxが他方に含まれている場合、信頼度の低い方を除去

    Args:
        bboxes: bbox辞書のリスト
        containment_threshold: 含有率の閾値（0.7 = 70%以上含まれていたら除去）

    Returns:
        重複除去後のbboxリスト
    """
    if len(bboxes) <= 1:
        return bboxes

    # 同じラベルごとにグループ化
    label_groups = {}
    for bbox in bboxes:
        label = bbox['label']
        if label not in label_groups:
            label_groups[label] = []
        label_groups[label].append(bbox)

    result = []

    for label, group in label_groups.items():
        if len(group) <= 1:
            result.extend(group)
            continue

        # 信頼度でソート（高い順）
        sorted_group = sorted(group, key=lambda b: b['conf'], reverse=True)
        keep = [True] * len(sorted_group)

        for i in range(len(sorted_group)):
            if not keep[i]:
                continue

            for j in range(i + 1, len(sorted_group)):
                if not keep[j]:
                    continue

                # i番目とj番目のbboxの含有判定
                box_i = sorted_group[i]
                box_j = sorted_group[j]

                # 交差領域を計算
                inter_x1 = max(box_i['x1'], box_j['x1'])
                inter_y1 = max(box_i['y1'], box_j['y1'])
                inter_x2 = min(box_i['x2'], box_j['x2'])
                inter_y2 = min(box_i['y2'], box_j['y2'])

                if inter_x2 > inter_x1 and inter_y2 > inter_y1:
                    inter_area = (inter_x2 - inter_x1) * (inter_y2 - inter_y1)

                    area_j = (box_j['x2'] - box_j['x1']) * (box_j['y2'] - box_j['y1'])

                    # j番目がi番目に含まれている割合
                    if area_j > 0:
                        containment = inter_area / area_j
                        if containment > containment_threshold:
                            # j番目は除去（信頼度が低い方）
                            keep[j] = False

        for i, bbox in enumerate(sorted_group):
            if keep[i]:
                result.append(bbox)

    return result


# ===== フレーム差分計算 (Issue #66) =====

def calculate_iou(box1: Dict, box2: Dict) -> float:
    """
    2つのbbox間のIoU (Intersection over Union) を計算

    Args:
        box1, box2: {"x1", "y1", "x2", "y2"} を含むdict

    Returns:
        IoU値 (0.0-1.0)
    """
    x1 = max(box1["x1"], box2["x1"])
    y1 = max(box1["y1"], box2["y1"])
    x2 = min(box1["x2"], box2["x2"])
    y2 = min(box1["y2"], box2["y2"])

    inter_width = max(0, x2 - x1)
    inter_height = max(0, y2 - y1)
    inter_area = inter_width * inter_height

    area1 = (box1["x2"] - box1["x1"]) * (box1["y2"] - box1["y1"])
    area2 = (box2["x2"] - box2["x1"]) * (box2["y2"] - box2["y1"])
    union_area = area1 + area2 - inter_area

    if union_area <= 0:
        return 0.0
    return inter_area / union_area


def calculate_frame_diff(
    current_persons: List[Dict],
    prev_persons: List[Dict],
    iou_threshold: float = 0.3,
    movement_threshold: float = 0.1
) -> Dict:
    """
    2フレーム間の人物変化を計算

    Args:
        current_persons: 現フレームの人物bbox一覧
        prev_persons: 前フレームの人物bbox一覧
        iou_threshold: 同一人物判定のIoU閾値
        movement_threshold: 移動判定の相対距離閾値

    Returns:
        {
            "enabled": True,
            "person_changes": {
                "appeared": int,    # 新規出現
                "disappeared": int, # 消失
                "moved": int,       # 移動
                "stationary": int   # 静止
            },
            "camera_status": "stable" | "scene_change"
        }
    """
    if not prev_persons:
        return {
            "enabled": True,
            "person_changes": {
                "appeared": len(current_persons),
                "disappeared": 0,
                "moved": 0,
                "stationary": 0
            },
            "camera_status": "stable"
        }

    # マッチング: 現在の各人物に対して前フレームで最もIoUが高い人物を探す
    matched_prev = set()
    matched_current = set()
    moved_count = 0
    stationary_count = 0

    for i, curr in enumerate(current_persons):
        best_iou = 0
        best_j = -1
        for j, prev in enumerate(prev_persons):
            if j in matched_prev:
                continue
            iou = calculate_iou(curr, prev)
            if iou > best_iou:
                best_iou = iou
                best_j = j

        if best_iou >= iou_threshold and best_j >= 0:
            matched_prev.add(best_j)
            matched_current.add(i)

            # 移動判定: bboxの中心点の相対移動量
            prev_box = prev_persons[best_j]
            curr_cx = (curr["x1"] + curr["x2"]) / 2
            curr_cy = (curr["y1"] + curr["y2"]) / 2
            prev_cx = (prev_box["x1"] + prev_box["x2"]) / 2
            prev_cy = (prev_box["y1"] + prev_box["y2"]) / 2

            # 画像サイズを基準にした相対移動量
            box_size = max(curr["x2"] - curr["x1"], curr["y2"] - curr["y1"], 1)
            movement = ((curr_cx - prev_cx) ** 2 + (curr_cy - prev_cy) ** 2) ** 0.5 / box_size

            if movement > movement_threshold:
                moved_count += 1
            else:
                stationary_count += 1

    appeared = len(current_persons) - len(matched_current)
    disappeared = len(prev_persons) - len(matched_prev)

    # シーン変化判定: 大幅な人数変動
    total_prev = len(prev_persons)
    total_curr = len(current_persons)
    if total_prev > 0 and abs(total_curr - total_prev) / total_prev > 0.5:
        camera_status = "scene_change"
    else:
        camera_status = "stable"

    return {
        "enabled": True,
        "person_changes": {
            "appeared": appeared,
            "disappeared": disappeared,
            "moved": moved_count,
            "stationary": stationary_count
        },
        "camera_status": camera_status
    }


# ===== 推論実行 =====

def run_yolo_inference(
    image: np.ndarray,
    conf_threshold: float = None,
    nms_threshold: float = None
) -> tuple:
    """
    YOLO推論実行

    Args:
        image: 入力画像
        conf_threshold: 信頼度閾値 (None=デフォルト使用)
        nms_threshold: NMS閾値 (None=デフォルト使用)

    Returns:
        (bboxes, elapsed_ms)
    """
    global rknn_yolo, inference_stats

    if rknn_yolo is None:
        return [], 0

    h, w = image.shape[:2]
    scale = INPUT_SIZE / max(h, w)
    new_h, new_w = int(h * scale), int(w * scale)
    resized = cv2.resize(image, (new_w, new_h))

    padded = np.zeros((INPUT_SIZE, INPUT_SIZE, 3), dtype=np.uint8)
    padded[:new_h, :new_w] = resized

    input_data = np.expand_dims(padded, axis=0)

    start = time.time()
    outputs = rknn_yolo.inference(inputs=[input_data])
    elapsed_ms = (time.time() - start) * 1000

    inference_stats["yolo"]["total_count"] += 1
    inference_stats["yolo"]["success_count"] += 1
    inference_stats["yolo"]["total_ms"] += elapsed_ms
    hardware_info.update_inference_stats(elapsed_ms, "yolov5s-640-640.rknn")

    boxes, scores, classes = postprocess_yolov5(outputs, (h, w), conf_threshold, nms_threshold)

    bboxes = []
    for box, score, cls_id in zip(boxes, scores, classes):
        x1, y1, x2, y2 = box[:4]
        bboxes.append({
            "x1": float(x1),
            "y1": float(y1),
            "x2": float(x2),
            "y2": float(y2),
            "label": COCO_CLASSES[int(cls_id)],
            "conf": float(score)
        })

    # v1.6.1: 含有重複除去（小さいbboxが大きいbboxに含まれる場合を除去）
    bboxes = remove_contained_boxes(bboxes, containment_threshold=0.7)

    return bboxes, int(elapsed_ms)


def run_par_inference(
    image: np.ndarray,
    person_bboxes: List[Dict],
    par_threshold: float = None
) -> tuple:
    """
    PAR推論実行

    Args:
        image: 入力画像
        person_bboxes: 人物bboxリスト
        par_threshold: 属性閾値 (None=デフォルトPAR_THRESHOLD使用)

    Returns:
        (par_results, total_ms)
    """
    global par_inference, inference_stats

    if par_inference is None or not person_bboxes:
        return [], 0

    # デフォルト値の適用
    effective_par_threshold = par_threshold if par_threshold is not None else PAR_THRESHOLD

    sorted_persons = sorted(person_bboxes, key=lambda x: x['conf'], reverse=True)
    target_persons = sorted_persons[:PAR_MAX_PERSONS]

    par_results = []
    total_ms = 0

    for bbox in target_persons:
        result = par_inference.infer_person(image, bbox, effective_par_threshold)
        par_results.append(result)
        total_ms += result.get('inference_ms', 0)

        if 'error' not in result:
            inference_stats["par"]["success_count"] += 1
        else:
            inference_stats["par"]["error_count"] += 1

    inference_stats["par"]["total_count"] += len(target_persons)
    inference_stats["par"]["total_ms"] += total_ms

    return par_results, total_ms


# ===== タグ集約 (v1.6.0 全属性対応) =====

def aggregate_tags(
    bboxes: List[Dict],
    par_results: List[Dict],
    image: np.ndarray
) -> Tuple[List[str], List[Dict], List[Dict]]:
    """
    全検出結果からschema.jsonタグを集約 (v1.7.0 人物特徴強化版, DD19車両詳細追加)

    Returns:
        (tags, person_details, vehicle_details) - タグリストと人物詳細情報と車両詳細情報
    """
    tags = set()
    person_details = []

    color_start = time.time()

    # 人数カウント
    person_bboxes = [b for b in bboxes if b["label"] == "person"]
    person_count = len(person_bboxes)

    if person_count == 1:
        tags.add("count.single")
    elif person_count > 1:
        tags.add("count.multiple")

    # 人物ごとの属性分析
    for i, (bbox, par_result) in enumerate(zip(person_bboxes[:len(par_results)], par_results)):
        person_info = {"index": i}

        # PAR由来タグ
        for tag in par_result.get('tags', []):
            tags.add(tag)

        # 服装色分析（上半身）
        top_color, color_conf = analyze_top_color(image, bbox)
        if top_color and top_color != "other":
            tags.add(f"top_color.{top_color}")
            person_info["top_color"] = {"color": top_color, "confidence": color_conf}

        # === v1.7 新機能: 下半身色 ===
        bottom_color, bottom_conf = analyze_bottom_color(image, bbox)
        if bottom_color and bottom_color != "other":
            tags.add(f"bottom_color.{bottom_color}")
            person_info["bottom_color"] = {"color": bottom_color, "confidence": bottom_conf}

        # マスク検出
        if detect_mask_like(image, bbox):
            tags.add("appearance.mask_like")
            person_info["mask_like"] = True

        # 制服検出
        if detect_uniform_like(par_result):
            tags.add("appearance.uniform_like")
            person_info["uniform_like"] = True

        # outfit拡充
        outfit_tags = extract_outfit_tags(par_result)
        for tag in outfit_tags:
            tags.add(tag)

        # 体格
        body_size_tag = estimate_body_size(bbox, image.shape)
        if body_size_tag:
            tags.add(body_size_tag)
            person_info["body_size"] = body_size_tag.split(".")[-1]

        # 体型
        body_build_tag = estimate_body_build(bbox, image.shape)
        if body_build_tag:
            tags.add(body_build_tag)
            person_info["body_build"] = "heavy"

        # === v1.7 新機能: 姿勢推定 ===
        posture, posture_conf = estimate_posture(bbox, image.shape)
        if posture:
            tags.add(f"posture.{posture}")
            person_info["posture"] = {"type": posture, "confidence": posture_conf}

        # === v1.7 新機能: 肌色推定 ===
        skin_tone, skin_conf = detect_skin_tone(image, bbox)
        if skin_tone:
            person_info["skin_tone"] = {"tone": skin_tone, "confidence": skin_conf}

        # === v1.7 新機能: 髪型推定 ===
        hair_type, hair_conf = detect_hair_type(image, bbox)
        if hair_type:
            tags.add(f"hair.{hair_type}")
            person_info["hair_type"] = {"type": hair_type, "confidence": hair_conf}

        # === v1.7 新機能: 相対身長 ===
        height_cat = estimate_height_category(bbox, image.shape)
        if height_cat:
            tags.add(f"height.{height_cat}")
            person_info["height_category"] = height_cat

        # 行動推定
        behavior_tags = estimate_behavior(bbox, image.shape)
        for tag in behavior_tags:
            tags.add(tag)

        # 年齢・性別タグ
        demographic_tags = extract_demographic_tags(par_result)
        for tag in demographic_tags:
            tags.add(tag)

        # メタ情報
        person_info["meta"] = par_result.get('meta', {})
        person_details.append(person_info)

    # 車両の詳細分析 (DD19対応: vehicle_details追加)
    vehicle_bboxes = [b for b in bboxes if b["label"] in ["car", "truck", "bus", "motorcycle"]]
    vehicle_details = []
    for i, vbbox in enumerate(vehicle_bboxes[:5]):  # 最大5台
        vehicle_info = {"index": i}

        # 車両タイプ
        vehicle_type = vbbox["label"]
        vehicle_info["vehicle_type"] = vehicle_type
        tags.add(f"vehicle_type.{vehicle_type}")

        # 車両の色分析
        v_color, v_conf = analyze_vehicle_color(image, vbbox)
        if v_color:
            tags.add(f"vehicle_color.{v_color}")
            vehicle_info["color"] = {"color": v_color, "confidence": v_conf}

        # 車両サイズ推定（bbox面積から）
        h, w = image.shape[:2]
        bbox_w = (vbbox["x2"] - vbbox["x1"]) * w
        bbox_h = (vbbox["y2"] - vbbox["y1"]) * h
        bbox_area = bbox_w * bbox_h
        image_area = h * w
        size_ratio = bbox_area / image_area if image_area > 0 else 0

        if size_ratio > 0.3:
            vehicle_info["size_category"] = "large"
            tags.add("vehicle_size.large")
        elif size_ratio > 0.1:
            vehicle_info["size_category"] = "medium"
            tags.add("vehicle_size.medium")
        else:
            vehicle_info["size_category"] = "small"
            tags.add("vehicle_size.small")

        # confidence
        vehicle_info["confidence"] = vbbox.get("conf", 0.0)

        # 位置情報（正規化座標）
        vehicle_info["position"] = {
            "x1": vbbox["x1"],
            "y1": vbbox["y1"],
            "x2": vbbox["x2"],
            "y2": vbbox["y2"]
        }

        vehicle_details.append(vehicle_info)

    # 画像品質タグ
    quality_tags = analyze_image_quality(image)
    for tag in quality_tags:
        tags.add(tag)

    # 色分析の統計
    color_ms = (time.time() - color_start) * 1000
    inference_stats["color"]["total_count"] += 1
    inference_stats["color"]["total_ms"] += color_ms

    return list(tags), person_details, vehicle_details


# ===== エンドポイント =====

@app.on_event("startup")
async def startup_event():
    load_schema_from_file()
    init_aranea_device()
    init_rknn()
    init_par()


@app.get("/api/status")
async def api_status():
    lacis_id = lacis_generator.generate()
    reg_info = aranea_register.get_registration_info()
    uptime = hardware_info.get_uptime()
    summary = hardware_info.get_summary()

    par_stats = None
    if par_inference:
        par_stats = par_inference.get_stats()

    return {
        "device": {
            "type": DEVICE_TYPE,
            "name": DEVICE_NAME,
            "productType": PRODUCT_TYPE,
            "productCode": PRODUCT_CODE,
            "firmwareVersion": FIRMWARE_VERSION
        },
        "lacisId": lacis_id,
        "mac": lacis_generator.get_mac_formatted(),
        "registration": reg_info,
        "uptime": uptime["uptime_formatted"],
        "uptimeSeconds": uptime["uptime_seconds"],
        "bootTime": uptime["boot_time"],
        "health": {
            "memoryPercent": summary["memory_percent"],
            "cpuLoad": summary["cpu_load"],
            "cpuTemp": summary["cpu_temp"],
            "npuTemp": summary["npu_temp"]
        },
        "inference": {
            "yolo": {
                "totalCount": inference_stats["yolo"]["total_count"],
                "successCount": inference_stats["yolo"]["success_count"],
                "errorCount": inference_stats["yolo"]["error_count"],
                "avgMs": round(inference_stats["yolo"]["total_ms"] / max(1, inference_stats["yolo"]["total_count"]), 2)
            },
            "par": {
                "enabled": par_inference is not None,
                "totalCount": inference_stats["par"]["total_count"],
                "successCount": inference_stats["par"]["success_count"],
                "errorCount": inference_stats["par"]["error_count"],
                "avgMs": round(inference_stats["par"]["total_ms"] / max(1, inference_stats["par"]["total_count"]), 2)
            } if par_inference else {"enabled": False},
            "color": {
                "totalCount": inference_stats["color"]["total_count"],
                "avgMs": round(inference_stats["color"]["total_ms"] / max(1, inference_stats["color"]["total_count"]), 2)
            }
        },
        "schemaVersion": schema_version,
        "timestamp": datetime.now().isoformat()
    }


@app.get("/api/hardware")
async def api_hardware():
    return hardware_info.get_all()


@app.get("/api/hardware/summary")
async def api_hardware_summary():
    return hardware_info.get_summary()


class TenantPrimaryRequest(BaseModel):
    lacis_id: str
    user_id: str
    cic: str


class CloudActivateRequest(BaseModel):
    tid: str
    tenant_primary: TenantPrimaryRequest


class RegisterRequest(BaseModel):
    tid: Optional[str] = None


@app.post("/api/cloud/activate")
async def api_cloud_activate(req: CloudActivateRequest):
    """
    AraneaWebUI互換: Cloud tabからのアクティベーション
    テナントプライマリ認証を動的に受け付ける
    """
    # テナントプライマリ認証を設定
    tenant_auth = TenantPrimaryAuth(
        lacis_id=req.tenant_primary.lacis_id,
        user_id=req.tenant_primary.user_id,
        cic=req.tenant_primary.cic
    )
    aranea_register.set_tenant_primary(tenant_auth)

    lacis_id = lacis_generator.generate()
    mac = lacis_generator.get_primary_mac_hex()

    result = aranea_register.register_device(
        tid=req.tid,
        device_type=DEVICE_TYPE,
        lacis_id=lacis_id,
        mac_address=mac,
        product_type=PRODUCT_TYPE,
        product_code=PRODUCT_CODE
    )

    if result.ok:
        # TIDを設定に保存
        config_manager.set_string("tid", req.tid)
        return {
            "ok": True,
            "cic_code": result.cic_code,
            "lacis_id": lacis_id,
            "state_endpoint": result.state_endpoint,
            "mqtt_endpoint": result.mqtt_endpoint
        }
    else:
        return {"ok": False, "error": result.error}


@app.post("/api/register")
async def api_register(req: RegisterRequest = None):
    """レガシー互換: 既存のテナント認証で登録"""
    tid = DEFAULT_TID
    if req and req.tid:
        tid = req.tid

    lacis_id = lacis_generator.generate()
    mac = lacis_generator.get_primary_mac_hex()

    result = aranea_register.register_device(
        tid=tid,
        device_type=DEVICE_TYPE,
        lacis_id=lacis_id,
        mac_address=mac,
        product_type=PRODUCT_TYPE,
        product_code=PRODUCT_CODE
    )

    if result.ok:
        return {
            "ok": True,
            "cic_code": result.cic_code,
            "lacis_id": lacis_id,
            "state_endpoint": result.state_endpoint,
            "mqtt_endpoint": result.mqtt_endpoint
        }
    else:
        raise HTTPException(status_code=400, detail=result.error)


@app.delete("/api/register")
async def api_clear_registration():
    aranea_register.clear_registration()
    return {"ok": True, "message": "Registration cleared"}


@app.get("/api/config")
async def api_get_config():
    return config_manager.get_all()


class ConfigUpdate(BaseModel):
    key: str
    value: Any


@app.post("/api/config")
async def api_set_config(data: ConfigUpdate):
    if isinstance(data.value, str):
        config_manager.set_string(data.key, data.value)
    elif isinstance(data.value, bool):
        config_manager.set_bool(data.key, data.value)
    elif isinstance(data.value, int):
        config_manager.set_int(data.key, data.value)
    elif isinstance(data.value, dict):
        config_manager.set_dict(data.key, data.value)
    else:
        config_manager.set_string(data.key, str(data.value))

    return {"ok": True, "key": data.key}


# ===== AraneaWebUI互換API =====

class NetworkSaveRequest(BaseModel):
    hostname: Optional[str] = None
    ntp_server: Optional[str] = None


@app.post("/api/network/save")
async def api_network_save(data: NetworkSaveRequest):
    """AraneaWebUI互換: Network設定保存"""
    if data.hostname:
        config_manager.set_string("hostname", data.hostname)
    if data.ntp_server:
        config_manager.set_string("ntp_server", data.ntp_server)
    return {"ok": True}


class TenantSaveRequest(BaseModel):
    fid: Optional[str] = None
    device_name: Optional[str] = None


@app.post("/api/tenant/save")
async def api_tenant_save(data: TenantSaveRequest):
    """AraneaWebUI互換: Tenant設定保存"""
    if data.fid:
        config_manager.set_string("fid", data.fid)
    if data.device_name:
        config_manager.set_string("device_name", data.device_name)
    return {"ok": True}


@app.post("/api/system/reboot")
async def api_system_reboot():
    """AraneaWebUI互換: システム再起動"""
    import subprocess
    import asyncio

    async def do_reboot():
        await asyncio.sleep(2)
        subprocess.run(["sudo", "reboot"], check=False)

    asyncio.create_task(do_reboot())
    return {"ok": True, "message": "Rebooting in 2 seconds"}


@app.post("/api/system/restart-service")
async def api_system_restart_service():
    """IS21サービス再起動"""
    import subprocess
    import asyncio

    async def do_restart():
        await asyncio.sleep(1)
        subprocess.run(["sudo", "systemctl", "restart", "is21-infer"], check=False)

    asyncio.create_task(do_restart())
    return {"ok": True, "message": "Service restarting"}


class InferenceConfigRequest(BaseModel):
    conf_threshold: Optional[float] = None
    nms_threshold: Optional[float] = None
    par_enabled: Optional[bool] = None


@app.post("/api/inference/config")
async def api_inference_config(data: InferenceConfigRequest):
    """推論設定変更"""
    global CONF_THRESHOLD, NMS_THRESHOLD

    if data.conf_threshold is not None:
        CONF_THRESHOLD = data.conf_threshold
        config_manager.set_string("conf_threshold", str(data.conf_threshold))

    if data.nms_threshold is not None:
        NMS_THRESHOLD = data.nms_threshold
        config_manager.set_string("nms_threshold", str(data.nms_threshold))

    if data.par_enabled is not None:
        config_manager.set_bool("par_enabled", data.par_enabled)

    return {"ok": True}


@app.get("/healthz")
async def healthz():
    uptime = int(time.time() - start_time)
    return {"status": "ok", "uptime_sec": uptime, "par_enabled": par_inference is not None}


@app.get("/v1/capabilities")
async def capabilities():
    return {
        "api_version": "1.0.0",
        "device": {
            "type": DEVICE_TYPE,
            "lacis_id": lacis_generator.generate(),
            "firmware_version": FIRMWARE_VERSION
        },
        "runtime": {
            "npu": "rknpu2",
            "npu_available": RKNN_AVAILABLE,
            "os": "Armbian Ubuntu 24.04"
        },
        "models": {
            "yolo": {"name": "yolov5s-640", "version": "2025.12.29"},
            "par": {
                "name": "par_resnet50_pa100k",
                "version": "2025.12.30",
                "enabled": par_inference is not None,
                "attributes": 26
            }
        },
        "supported_primary_events": [
            "none", "human", "animal", "vehicle",
            "hazard", "camera_issue", "object_missing", "unknown"
        ],
        "supported_tags": {
            "count": ["count.single", "count.multiple"],
            "top_color": ["top_color.red", "top_color.blue", "top_color.black",
                         "top_color.white", "top_color.gray", "top_color.other"],
            "vehicle_color": ["vehicle_color.white", "vehicle_color.black",
                             "vehicle_color.gray", "vehicle_color.red", "vehicle_color.blue"],
            "carry": ["carry.backpack", "carry.bag", "carry.holding"],
            "appearance": ["appearance.hat_like", "appearance.glasses",
                          "appearance.mask_like", "appearance.uniform_like"],
            "outfit": ["outfit.coat_like", "outfit.shorts", "outfit.dress",
                      "outfit.pants", "outfit.boots"],
            "body": ["body.size.small", "body.size.medium", "body.size.large",
                    "body.build.heavy"],
            "behavior": ["behavior.sitting"],
            "gender": ["gender.male", "gender.female"],
            "age": ["age.child_teen", "age.adult", "age.elderly"],
            "camera": ["camera.blur", "camera.dark", "camera.glare",
                      "camera.underexposed", "camera.overexposed"]
        },
        "max_image_bytes": MAX_IMAGE_BYTES,
        "recommended_infer_widths": [640, 960]
    }


@app.get("/v1/schema")
async def get_schema():
    if schema_version is None:
        raise HTTPException(status_code=404, detail="Schema not set")
    return {
        "schema_version": schema_version,
        "schema": schema_data,
        "received_at": schema_received_at
    }


class SchemaUpdate(BaseModel):
    schema_version: str
    schema: dict


@app.put("/v1/schema")
async def put_schema(data: SchemaUpdate):
    global schema_version, schema_data, schema_received_at
    schema_version = data.schema_version
    schema_data = data.schema
    schema_received_at = datetime.now().isoformat()
    save_schema_to_file()
    return {"schema_version": schema_version, "status": "accepted"}


@app.post("/v1/analyze")
async def analyze(
    camera_id: str = Form(...),
    captured_at: str = Form(...),
    schema_version_req: str = Form(..., alias="schema_version"),
    infer_image: UploadFile = File(...),
    profile: str = Form("standard"),
    return_bboxes: bool = Form(True),
    hints_json: Optional[str] = Form(None),
    # === v1.9 prev_image追加 (Issue #62) ===
    prev_image: Optional[UploadFile] = File(None),
):
    if schema_version and schema_version_req != schema_version:
        raise HTTPException(
            status_code=409,
            detail={"error": "schema_mismatch", "current": schema_version}
        )

    contents = await infer_image.read()
    if len(contents) > MAX_IMAGE_BYTES:
        raise HTTPException(status_code=400, detail="Image too large")

    nparr = np.frombuffer(contents, np.uint8)
    image = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if image is None:
        raise HTTPException(status_code=400, detail="Invalid image format")

    # === v1.9 prev_image処理 (Issue #62) ===
    prev_frame = None
    if prev_image:
        try:
            prev_contents = await prev_image.read()
            if len(prev_contents) > 0:
                prev_nparr = np.frombuffer(prev_contents, np.uint8)
                prev_frame = cv2.imdecode(prev_nparr, cv2.IMREAD_COLOR)
                if prev_frame is not None:
                    logger.debug(f"prev_image loaded: {prev_frame.shape}")
        except Exception as e:
            logger.warning(f"Failed to load prev_image: {e}")
            prev_frame = None

    total_start = time.time()

    # === v1.8 camera_context処理 ===
    camera_context = parse_camera_context(hints_json)
    context_applied = bool(camera_context)
    filtered_reasons = []

    # === v1.9 hints_jsonパラメータ抽出 (Issue #63) ===
    effective_conf = None
    effective_nms = None
    effective_par = None
    if camera_context:
        # conf_override: 0.2-0.8の範囲にクリップ
        if "conf_override" in camera_context:
            effective_conf = max(0.2, min(0.8, float(camera_context["conf_override"])))
        # nms_threshold: 0.3-0.6の範囲にクリップ
        if "nms_threshold" in camera_context:
            effective_nms = max(0.3, min(0.6, float(camera_context["nms_threshold"])))
        # par_threshold: 0.3-0.8の範囲にクリップ
        if "par_threshold" in camera_context:
            effective_par = max(0.3, min(0.8, float(camera_context["par_threshold"])))

    # Stage 1: YOLO推論 (conf_override, nms_threshold適用)
    try:
        bboxes, yolo_ms = run_yolo_inference(image, effective_conf, effective_nms)
    except Exception as e:
        inference_stats["yolo"]["error_count"] += 1
        logger.error(f"YOLO inference error: {e}")
        raise HTTPException(status_code=500, detail=str(e))

    # === v1.8 コンテキストフィルタリング ===
    if camera_context:
        bboxes, filtered_reasons = apply_context_filters(bboxes, camera_context)

    # Stage 2: PAR推論（人物検出時のみ） - v1.9 par_threshold適用
    person_bboxes = [b for b in bboxes if b["label"] == "person"]
    par_results, par_ms = run_par_inference(image, person_bboxes, effective_par)

    # Stage 3: タグ集約 (v1.7.0 人物特徴強化版, DD19車両詳細追加)
    tags, person_details, vehicle_details = aggregate_tags(bboxes, par_results, image)

    total_ms = int((time.time() - total_start) * 1000)

    # primary_event決定
    relevant_bboxes = [b for b in bboxes if b["label"] not in IGNORE_OBJECTS]

    detected = len(relevant_bboxes) > 0
    if detected:
        top_bbox = max(relevant_bboxes, key=lambda b: b["conf"])
        primary_event = EVENT_MAP.get(top_bbox["label"], "unknown")
        confidence = top_bbox["conf"]
    else:
        primary_event = "none"
        confidence = 0.0

    person_count = len(person_bboxes)

    # === v1.9 severity計算の正常化 (Issue #61) ===
    def calculate_severity(primary_event: str) -> int:
        """
        severity計算:
        - 3: human (person) - 最高優先度
        - 2: vehicle - 中優先度
        - 1: animal, motion, unknown - 低優先度
        - 0: none - 検出なし
        """
        severity_map = {
            "human": 3,
            "vehicle": 2,
            "animal": 1,
            "motion": 1,
            "unknown": 1,
        }
        return severity_map.get(primary_event, 0)

    severity = calculate_severity(primary_event) if detected else 0

    # PAR詳細をbboxに付与
    if return_bboxes and par_results:
        for i, bbox in enumerate(person_bboxes[:len(par_results)]):
            par_result = par_results[i]
            bbox["par"] = {
                "tags": par_result.get("tags", []),
                "meta": par_result.get("meta", {})
            }
            # 人物詳細も付与
            if i < len(person_details):
                bbox["details"] = person_details[i]

    # === v1.7/v1.8 不審者スコア計算 (camera_context統合) ===
    suspicious_result = {"score": 0, "level": "normal", "factors": []}
    if primary_event == "human" and person_count > 0:
        # captured_atから時刻を抽出
        try:
            dt = datetime.fromisoformat(captured_at.replace('Z', '+00:00'))
            hour = dt.hour
            is_night = hour < 6 or hour >= 20
        except:
            hour = None
            is_night = False

        # camera_contextからの情報をマージ
        context = {
            "hour": hour,
            "is_night": is_night,
            "location_type": camera_context.get("location_type", ""),
            "busy_hours": camera_context.get("busy_hours", []),
            "quiet_hours": camera_context.get("quiet_hours", []),
        }
        suspicious_result = calculate_suspicious_score(tags, context)

    # === v1.9 frame_diff結果 (Issue #62, #66) ===
    if prev_frame is not None:
        try:
            # prev_frameでYOLO推論
            prev_bboxes, _ = run_yolo_inference(prev_frame)
            prev_person_bboxes = [b for b in prev_bboxes if b["label"] == "person"]
            # フレーム差分計算
            frame_diff_result = calculate_frame_diff(person_bboxes, prev_person_bboxes)
        except Exception as e:
            logger.warning(f"Frame diff calculation failed: {e}")
            frame_diff_result = {
                "enabled": False,
                "camera_status": "error",
                "error": str(e)
            }
    else:
        frame_diff_result = {
            "enabled": False,
            "camera_status": "no_reference"
        }

    return {
        "schema_version": schema_version or "unset",
        "camera_id": camera_id,
        "captured_at": captured_at,
        "analyzed": True,
        "detected": detected,
        "primary_event": primary_event,
        "tags": tags,
        "confidence": confidence,
        "severity": severity,
        "unknown_flag": primary_event == "unknown",
        "count_hint": person_count,
        "bboxes": bboxes if return_bboxes else [],
        "person_details": person_details,
        "vehicle_details": vehicle_details,  # DD19対応: 車両詳細追加
        "suspicious": suspicious_result,
        # === v1.9 frame_diff結果 (Issue #62) ===
        "frame_diff": frame_diff_result,
        # === v1.8 camera_context結果 ===
        "context_applied": context_applied,
        "filtered_by_context": filtered_reasons if filtered_reasons else None,
        # v1.9 フィールド名をis22の期待に合わせる
        "performance": {
            "inference_ms": total_ms,
            "yolo_ms": yolo_ms,
            "par_ms": par_ms
        },
        "model_info": {
            "yolo": {"name": "yolov5s-640", "version": "2025.12.29"},
            "par": {
                "name": "par_resnet50_pa100k",
                "version": "2025.12.30",
                "enabled": par_inference is not None
            }
        }
    }


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=9000)
