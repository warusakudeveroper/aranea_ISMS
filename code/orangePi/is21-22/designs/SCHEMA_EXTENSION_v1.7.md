# is21 スキーマ拡張設計 v1.7

作成日: 2025-12-30
ステータス: **設計中**

---

## 1. 拡張概要

| 機能 | 現状 | v1.7目標 | 実装方式 |
|------|------|----------|----------|
| **車両詳細** | vehicle_color.grayのみ | 車種・色・ナンバー | 多段モデル |
| **人種特徴** | なし | appearance.ethnicity_hint | PAR拡張 |
| **差分検出** | なし | diff_mode対応 | 2枚画像比較 |
| **ナンバーOCR** | なし | license_plate | YOLO→OCR多段 |

---

## 2. 車両スキーマ拡張

### 2.1 現状

```json
{
  "primary_event": "vehicle",
  "tags": ["vehicle_color.gray"],
  "bboxes": [{"label": "car", "conf": 0.51}]
}
```

### 2.2 拡張後

```json
{
  "primary_event": "vehicle",
  "tags": [
    "vehicle.car",
    "vehicle.count.single",
    "vehicle_color.white",
    "vehicle_size.compact"
  ],
  "bboxes": [
    {
      "label": "car",
      "conf": 0.87,
      "details": {
        "vehicle_type": "sedan",
        "vehicle_color": {"primary": "white", "confidence": 0.92},
        "vehicle_size": "compact",
        "license_plate": {
          "detected": true,
          "text": "品川 500 あ 12-34",
          "confidence": 0.78,
          "bbox": {"x1": 0.3, "y1": 0.7, "x2": 0.5, "y2": 0.8}
        }
      }
    }
  ],
  "vehicle_details": [
    {
      "index": 0,
      "type": "sedan",
      "color": "white",
      "size": "compact",
      "plate": "品川 500 あ 12-34"
    }
  ]
}
```

### 2.3 新規タグ

| カテゴリ | タグ | 説明 |
|----------|------|------|
| **vehicle** | vehicle.car | 乗用車 |
| | vehicle.truck | トラック |
| | vehicle.motorcycle | バイク |
| | vehicle.bus | バス |
| | vehicle.bicycle | 自転車 |
| **vehicle_color** | vehicle_color.white | 白 |
| | vehicle_color.black | 黒 |
| | vehicle_color.silver | シルバー |
| | vehicle_color.red | 赤 |
| | vehicle_color.blue | 青 |
| | vehicle_color.other | その他 |
| **vehicle_size** | vehicle_size.compact | 軽・小型 |
| | vehicle_size.standard | 普通 |
| | vehicle_size.large | 大型 |
| **vehicle.count** | vehicle.count.single | 1台 |
| | vehicle.count.few | 2-5台 |
| | vehicle.count.many | 6台以上 |

---

## 3. ナンバープレートOCR

### 3.1 多段処理パイプライン

```
入力画像
    ↓
[Stage 1: YOLOv5s] ────────── 車両bbox検出 (~60ms)
    ↓
[Stage 2: YOLOv5n-LP] ─────── ナンバープレートbbox検出 (~15ms)
    ├─ 車両bboxを拡大してcrop
    └─ ナンバープレート位置特定
    ↓
[Stage 3: PaddleOCR-JP] ───── 文字認識 (~30ms)
    ├─ プレート領域をcrop・正規化
    └─ 日本語ナンバー形式パース
    ↓
出力: "品川 500 あ 12-34"
```

### 3.2 日本ナンバープレート形式

```
[地名] [分類番号] [ひらがな] [一連番号]
品川    500        あ        12-34
```

| フィールド | 例 | パターン |
|-----------|-----|----------|
| 地名 | 品川, 練馬, 大阪 | 漢字2-4文字 |
| 分類番号 | 500, 300, 830 | 3桁数字 |
| ひらがな | あ, い, さ, れ | 1文字 |
| 一連番号 | 12-34, 1234 | 4桁 (ハイフンあり/なし) |

### 3.3 必要モデル

| モデル | 用途 | サイズ | 推定時間 |
|--------|------|--------|----------|
| YOLOv5n-LP | ナンバープレート検出 | 4MB | 15ms |
| PaddleOCR-Japan | 日本語OCR | 12MB | 30ms |

### 3.4 制限事項

- 夜間・低照度では精度低下
- 斜め角度からの認識は困難
- 汚れ・隠れがあると認識不可
- **プライバシー考慮**: 設定で無効化可能とする

---

## 4. 人種特徴 (Ethnicity Hint)

### 4.1 設計方針

**目的**: 人物特定の補助情報として外見特徴を記録
**注意**: 差別的利用を防ぐため、以下を遵守

1. **hint** として表現（確定的判定ではない）
2. 精度表示必須
3. ログ・通知からは除外可能設定
4. 法的要件（GDPR等）に準拠

### 4.2 タグ設計

```json
{
  "person_details": [
    {
      "appearance_hints": {
        "skin_tone": {"value": "medium", "confidence": 0.72},
        "hair_type": {"value": "dark_straight", "confidence": 0.68}
      }
    }
  ]
}
```

| 属性 | 値 | 説明 |
|------|-----|------|
| **skin_tone** | light | 明るい肌色 |
| | medium | 中間 |
| | dark | 暗い肌色 |
| **hair_type** | dark_straight | 黒/直毛 |
| | dark_curly | 黒/カール |
| | light | 明るい髪 |
| | gray_white | 白髪 |
| | bald | 禿頭 |
| | covered | 帽子等で隠れ |

### 4.3 実装方式

- PAR拡張: PA100Kには含まれないため、追加学習が必要
- または: 顔領域の色ヒストグラム解析（簡易版）

```python
def estimate_skin_tone(face_roi):
    """顔領域のHSV解析で肌色推定"""
    hsv = cv2.cvtColor(face_roi, cv2.COLOR_BGR2HSV)
    # 肌色領域マスク
    skin_mask = cv2.inRange(hsv, (0, 20, 70), (20, 255, 255))
    skin_pixels = face_roi[skin_mask > 0]

    if len(skin_pixels) < 100:
        return None, 0.0

    avg_v = np.mean(skin_pixels[:, 2])  # Value (明度)

    if avg_v > 180:
        return "light", 0.7
    elif avg_v > 120:
        return "medium", 0.7
    else:
        return "dark", 0.7
```

---

## 5. 差分検出モード (Diff Mode)

### 5.1 設計仕様 (特記仕様書より)

```yaml
# APIパラメータ
prev_infer_image: binary  # 前回画像 (optional)
hints_json: {"diff_mode": true}

# レスポンス
diff:
  used_prev_image: true
  diff_ratio: 0.003      # 変化率
  luma_delta: 5          # 輝度差
```

### 5.2 処理フロー

```
[is22 CamServer]
    │
    ├─ 前回画像をキャッシュ
    │
    ├─ 新フレーム受信
    │     ↓
    ├─ diff_ratio計算 (is22側)
    │     ↓
    ├─ diff_ratio < 0.003 かつ luma_delta < 5
    │     → 推論スキップ (前回結果を返す)
    │
    └─ diff_ratio >= threshold
          → is21に推論リクエスト送信
```

### 5.3 is21側オプション実装

```python
def compute_diff(current_img, prev_img, width=320):
    """軽量差分計算"""
    # ダウンサンプル
    curr_small = cv2.resize(current_img, (width, width * 9 // 16))
    prev_small = cv2.resize(prev_img, (width, width * 9 // 16))

    # グレースケール変換
    curr_gray = cv2.cvtColor(curr_small, cv2.COLOR_BGR2GRAY)
    prev_gray = cv2.cvtColor(prev_small, cv2.COLOR_BGR2GRAY)

    # 差分計算
    diff = cv2.absdiff(curr_gray, prev_gray)
    diff_ratio = np.count_nonzero(diff > 25) / diff.size
    luma_delta = int(np.mean(curr_gray) - np.mean(prev_gray))

    return {
        "used_prev_image": True,
        "diff_ratio": round(diff_ratio, 6),
        "luma_delta": luma_delta
    }
```

### 5.4 ドア開閉検知への応用

```python
def detect_door_state(current_img, baseline_img, door_roi):
    """ドア開閉検知 (特定ROI内の差分)"""
    # ROI切り出し
    curr_roi = current_img[door_roi['y1']:door_roi['y2'],
                           door_roi['x1']:door_roi['x2']]
    base_roi = baseline_img[door_roi['y1']:door_roi['y2'],
                            door_roi['x1']:door_roi['x2']]

    diff = cv2.absdiff(curr_roi, base_roi)
    diff_ratio = np.count_nonzero(diff > 30) / diff.size

    if diff_ratio > 0.15:
        return "door.open"
    else:
        return "door.closed"
```

---

## 6. 不審者判定スコア (v1.7組み込み)

### 6.1 スコア計算

```python
def calculate_suspicious_score(result, context):
    """
    result: is21解析結果
    context: {hour, location_type, is_night, camera_id}
    """
    score = 0
    tags = result.get('tags', [])
    hour = context.get('hour', 12)

    # === 最重要 ===
    if 'appearance.mask_like' in tags:
        score += 40  # 顔隠し

    # === 時間帯 ===
    if 2 <= hour < 5:  # 深夜
        score += 25
    elif context.get('is_night') and ('top_color.black' in tags or 'top_color.gray' in tags):
        score += 15  # 夜間 + 暗い服

    # === 行動 (未実装) ===
    # if 'behavior.loitering' in tags:
    #     score += 30

    # === 場所 ===
    if context.get('location_type') == 'restricted':
        score += 20

    # === 持ち物 ===
    if 'carry.backpack' in tags and hour < 6:
        score += 10  # 深夜のバックパック

    return min(score, 100)
```

### 6.2 レスポンスへの追加

```json
{
  "primary_event": "human",
  "suspicious_score": 65,
  "suspicious_level": "high",
  "suspicious_factors": [
    "appearance.mask_like (+40)",
    "time.late_night (+25)"
  ]
}
```

---

## 7. 実装ロードマップ

### v1.7.0 - 車両詳細 + 差分モード

| 機能 | 優先度 | 工数 |
|------|--------|------|
| vehicle_type分類 | 高 | 2日 |
| vehicle_color拡充 (silver追加) | 高 | 0.5日 |
| vehicle_size推定 | 中 | 1日 |
| diff_mode基本実装 | 高 | 2日 |
| suspicious_score組み込み | 中 | 1日 |

### v1.8.0 - ナンバーOCR

| 機能 | 優先度 | 工数 |
|------|--------|------|
| YOLOv5n-LP統合 | 高 | 3日 |
| PaddleOCR-JP統合 | 高 | 3日 |
| 日本ナンバーパーサー | 中 | 1日 |
| プライバシー設定 | 高 | 1日 |

### v1.9.0 - 人物特徴拡張

| 機能 | 優先度 | 工数 |
|------|--------|------|
| skin_tone推定 | 中 | 2日 |
| hair_type推定 | 低 | 2日 |
| 設定による無効化 | 高 | 0.5日 |

---

## 8. APIスキーマ変更

### 8.1 /v1/analyze リクエスト拡張

```yaml
# 追加パラメータ
prev_infer_image:
  type: file
  description: 差分計算用の前回画像 (optional)

hints_json:
  type: string
  example: '{"diff_mode": true, "enable_ocr": true}'
```

### 8.2 /v1/analyze レスポンス拡張

```yaml
# 追加フィールド
vehicle_details:
  type: array
  items:
    type: object
    properties:
      index: integer
      type: string  # sedan, suv, truck, etc.
      color: string
      size: string  # compact, standard, large
      plate:
        type: object
        properties:
          text: string
          confidence: number
          region: string  # 品川, 練馬, etc.

diff:
  type: object
  properties:
    used_prev_image: boolean
    diff_ratio: number
    luma_delta: integer
    door_state: string  # open, closed, unknown

suspicious_score: integer  # 0-100
suspicious_level: string   # normal, caution, warning, alert, critical
suspicious_factors: array
```

---

## 9. 技術的課題

### 9.1 ナンバーOCR

| 課題 | 対策 |
|------|------|
| 夜間認識精度 | IR照明付きカメラ推奨 |
| 斜め角度 | 射影変換で補正 |
| 処理時間増加 | 車両検出時のみOCR実行 |

### 9.2 差分検出

| 課題 | 対策 |
|------|------|
| 照明変化による誤検知 | luma_deltaで補正 |
| カメラ揺れ | 位置補正 or 閾値緩和 |
| メモリ消費 | 縮小画像でdiff計算 |

### 9.3 人種特徴

| 課題 | 対策 |
|------|------|
| 倫理的懸念 | hint表現、設定で無効化 |
| 精度限界 | confidence必須表示 |
| 法的リスク | GDPR/個人情報保護法準拠 |

---

作成: Claude Code
