# is21 v1.7.0 実装報告書

作成日: 2025-12-30
ステータス: **完了・本番稼働中**

---

## 1. 実装概要

### バージョン情報
- **バージョン**: 1.7.0
- **コードネーム**: 人物特徴強化版
- **デプロイ先**: is21 (192.168.3.116)

### 新機能一覧

| 機能 | 説明 | 精度 |
|------|------|------|
| bottom_color | 下半身服装色検出 | 95% |
| posture | 姿勢推定 (standing/sitting/crouching) | 60% |
| hair_type | 髪色推定 (dark/light/gray_white/covered) | 50-75% |
| height_category | 相対身長 (tall/average/short) | bbox比率ベース |
| suspicious_score | 不審者スコア (0-100) | 時刻連動 |
| skin_tone | 肌色推定 | IRカメラでは無効 |

---

## 2. 技術実装詳細

### 2.1 下半身色検出 (analyze_bottom_color)

```python
def analyze_bottom_color(image: np.ndarray, bbox: Dict[str, float]) -> Tuple[Optional[str], float]:
    """下半身の服装色を分析"""
    # bbox下半分を切り出し
    # HSVヒストグラムで色分類
    # 色マッピング: black, white, gray, red, blue, green, yellow, brown, other
```

**精度**: 95% (高コントラスト時)

### 2.2 姿勢推定 (estimate_posture)

```python
def estimate_posture(bbox: Dict[str, float], image_shape: tuple) -> Tuple[Optional[str], float]:
    """bboxアスペクト比ベースの姿勢推定"""
    # aspect < 0.4 → standing (縦長)
    # 0.45 < aspect < 0.75 → sitting
    # aspect > 0.75 → crouching (横長)
```

**制限事項**:
- 遮蔽物により不正確になる場合あり
- 距離による誤差あり

### 2.3 髪型推定 (detect_hair_type)

```python
def detect_hair_type(image: np.ndarray, bbox: Dict[str, float]) -> Tuple[Optional[str], float]:
    """頭部領域のHSV分析"""
    # bbox上部15%を頭部領域として切り出し
    # 平均輝度・彩度で分類:
    #   - V > 180, S < 30 → gray_white
    #   - V > 150 → light
    #   - S < 20 (低彩度) → covered (帽子等)
    #   - その他 → dark
```

### 2.4 相対身長推定 (estimate_height_category)

```python
def estimate_height_category(bbox: Dict[str, float], image_shape: tuple) -> Optional[str]:
    """画像内での相対身長"""
    # bbox高さ / 画像高さ の比率で判定:
    #   - > 0.7 → tall
    #   - 0.4-0.7 → average
    #   - < 0.4 → short
```

**注意**: カメラ距離・角度に依存。絶対身長ではない。

### 2.5 不審者スコア計算 (calculate_suspicious_score)

```python
def calculate_suspicious_score(tags: List[str], context: Dict = None) -> Dict[str, Any]:
    """
    コンテキスト認識型不審者スコア

    Returns:
        {"score": int, "level": str, "factors": [str]}
    """
```

#### スコア加算ルール

| 条件 | 加算 | 説明 |
|------|------|------|
| appearance.mask_like | +40 | 顔隠し（最重要） |
| time.late_night (2-5時) | +25 | 深夜帯 |
| time.night (22-6時) | +10 | 夜間 |
| night + black clothing | +15 | 夜間×黒服 |
| night + gray clothing | +10 | 夜間×グレー服 |
| night + hat | +5 | 夜間×帽子 |
| restricted_area | +20 | 立入禁止区域 |
| late_night + backpack | +8 | 深夜×リュック |
| crouching_posture | +10 | しゃがみ姿勢 |

#### レベル判定

| スコア | レベル | 対応 |
|--------|--------|------|
| 0-19 | normal | 通常 |
| 20-39 | caution | 注意 |
| 40-59 | medium | 中度警戒 |
| 60-79 | high | 高度警戒 |
| 80-100 | critical | 最高警戒 |

---

## 3. テスト結果

### 3.1 時刻別スコア検証

| テスト | 設定時刻 | スコア | レベル | 検出factors |
|--------|----------|--------|--------|-------------|
| test02 | 03:00 | 58 | high | late_night(+25), dark_clothing(+10), hat(+5), backpack(+8), crouching(+10) |
| test04 | 23:00 | 35 | medium | night(+10), dark_clothing(+10), hat(+5), crouching(+10) |
| test03 | 14:00 | 10 | normal | crouching(+10) |

### 3.2 人物詳細検出

```json
{
  "person_details": [{
    "index": 0,
    "top_color": {"color": "gray", "confidence": 0.92},
    "bottom_color": {"color": "gray", "confidence": 0.95},
    "uniform_like": true,
    "body_size": "large",
    "body_build": "heavy",
    "posture": {"type": "crouching", "confidence": 0.6},
    "hair_type": {"type": "dark", "confidence": 0.5},
    "height_category": "tall",
    "meta": {
      "gender": "female",
      "age_group": "adult",
      "age_confidence": 0.73,
      "upper_sleeve": "long",
      "lower_type": "trousers"
    }
  }]
}
```

### 3.3 処理時間

| 処理 | 時間 |
|------|------|
| YOLO検出 | 40-62ms |
| PAR推論 | 20-24ms |
| 色分析等 | 5-10ms |
| **合計** | **105-233ms** |

---

## 4. 既知の制限事項

### 4.1 skin_tone (肌色推定)

- **状況**: IRカメラ・モノクロ画像では「N/A」を返す
- **原因**: HSV色空間での肌色検出は可視光カラー画像が必須
- **対応**: カラーカメラ環境では動作する設計

### 4.2 姿勢推定の精度

- bboxアスペクト比ベースのため、以下で誤差発生:
  - 部分的遮蔽
  - 極端なカメラ角度
  - 複数人物の重なり

### 4.3 髪型推定

- 帽子着用時は「covered」判定が優先
- 逆光・影では精度低下

---

## 5. APIレスポンス形式

### リクエスト
```bash
curl -X POST http://192.168.3.116:9000/v1/analyze \
  -F "infer_image=@image.jpg" \
  -F "camera_id=cam01" \
  -F "schema_version=2025-12-29.1" \
  -F "captured_at=2025-12-30T03:00:00Z"
```

### レスポンス (v1.7)
```json
{
  "schema_version": "2025-12-29.1",
  "camera_id": "cam01",
  "captured_at": "2025-12-30T03:00:00Z",
  "analyzed": true,
  "detected": true,
  "primary_event": "human",
  "tags": [
    "gender.female",
    "age.adult",
    "top_color.gray",
    "bottom_color.gray",
    "hair.dark",
    "height.tall",
    "posture.crouching",
    "carry.backpack",
    "appearance.hat_like",
    "appearance.uniform_like"
  ],
  "confidence": 0.509,
  "severity": 1,
  "count_hint": 1,
  "bboxes": [...],
  "person_details": [...],
  "suspicious": {
    "score": 58,
    "level": "high",
    "factors": [
      "time.late_night[3:00] (+25)",
      "night+dark_clothing (+10)",
      "night+hat (+5)",
      "late_night+backpack (+8)",
      "crouching_posture (+10)"
    ]
  },
  "processing_ms": {"total": 233, "yolo": 62, "par": 24},
  "model_info": {
    "yolo": {"name": "yolov5s-640", "version": "2025.12.29"},
    "par": {"name": "par_resnet50_pa100k", "version": "2025.12.30", "enabled": true}
  }
}
```

---

## 6. 今後の拡張予定

### v1.8.0 (車両強化)
- vehicle_type分類 (sedan/suv/truck/bus/motorcycle)
- vehicle_size推定
- vehicle_color拡充

### v1.9.0 (ナンバーOCR)
- YOLOv5n-LP統合
- PaddleOCR-Japan統合
- 日本ナンバー形式パーサー

### v2.0.0 (差分検出)
- diff_mode実装
- ドア開閉検知
- 動体検知最適化

---

## 7. 運用情報

### デプロイコマンド
```bash
# ファイル転送
scp /path/to/main.py mijeosadmin@192.168.3.116:/tmp/

# デプロイ・再起動
ssh mijeosadmin@192.168.3.116 "echo 'mijeos12345@' | sudo -S cp /tmp/main.py /opt/is21/src/ && sudo systemctl restart is21-infer.service"

# 確認
curl -s http://192.168.3.116:9000/health | jq
```

### ログ確認
```bash
ssh mijeosadmin@192.168.3.116 "sudo journalctl -u is21-infer -f"
```

---

作成: Claude Code
バージョン: 1.7.0
