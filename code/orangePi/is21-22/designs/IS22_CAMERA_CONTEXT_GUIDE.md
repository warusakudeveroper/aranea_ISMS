# IS22 → IS21 カメラコンテキスト送信ガイド

バージョン: 1.8.0
作成日: 2025-12-30

---

## 1. 概要

IS22サーバーからIS21 (camimageEdge AI) に画像を送信する際、`hints_json` パラメータでカメラのコンテキスト情報を付与することで、検知精度と不審者スコア計算を最適化できます。

```
IS22 (カメラ管理サーバー)
    │
    ├─ カメラID・設置場所情報を保持
    │
    ▼ POST /v1/analyze
    ├─ infer_image: 画像データ
    ├─ camera_id: カメラ識別子
    ├─ captured_at: 撮影時刻 (ISO8601)
    ├─ hints_json: カメラコンテキスト ← ★ここで最適化
    │
    ▼
IS21 (Edge AI)
    ├─ コンテキストに基づくフィルタリング
    ├─ 動的閾値調整
    └─ suspicious_score計算
```

---

## 2. hints_json パラメータ仕様

### 2.1 基本構造

```json
{
  "camera_context": {
    "location_type": "corridor",
    "distance": "near",
    "excluded_objects": ["car", "truck"],
    "expected_objects": ["person"],
    "detection_roi": {"x1": 0.1, "y1": 0.0, "x2": 0.9, "y2": 1.0},
    "conf_override": 0.35,
    "busy_hours": ["15:00-18:00"],
    "quiet_hours": ["23:00-06:00"]
  }
}
```

※ `camera_context` キーは省略可能。直接パラメータを渡しても動作します。

### 2.2 パラメータ一覧

| パラメータ | 型 | 説明 | 例 |
|-----------|-----|------|-----|
| `location_type` | string | 設置場所タイプ | "corridor", "entrance", "parking", "restricted" |
| `distance` | string | カメラ距離 | "near", "medium", "far" |
| `excluded_objects` | array | 除外するオブジェクト | ["car", "truck", "bicycle"] |
| `expected_objects` | array | 期待するオブジェクト | ["person", "door"] |
| `detection_roi` | object | 検知対象領域 (0-1正規化) | {"x1": 0, "y1": 0, "x2": 1, "y2": 1} |
| `conf_override` | number | 閾値オーバーライド (0.2-0.8) | 0.35 |
| `busy_hours` | array | 繁忙時間帯 | ["15:00-18:00", "09:00-10:00"] |
| `quiet_hours` | array | 静寂時間帯 | ["23:00-06:00", "0:00-5:00"] |

---

## 3. パラメータ詳細

### 3.1 location_type (設置場所)

| 値 | 説明 | suspicious_score効果 |
|----|------|---------------------|
| `corridor` | 廊下 | なし (将来的に滞在時間判定追加予定) |
| `entrance` | 入口・エントランス | **-5** (通常往来を想定) |
| `parking` | 駐車場 | 夜間時 **+10** |
| `restricted` | 立入禁止区域 | **+20** |
| `room` | 部屋内 | なし |
| `outdoor` | 屋外 | なし |
| `elevator` | エレベーター | なし |

### 3.2 distance (カメラ距離)

| 値 | 説明 | 閾値調整 |
|----|------|---------|
| `near` | 近距離 (1-3m) | conf_threshold **-0.05** (緩和) |
| `medium` | 中距離 (3-10m) | 変更なし |
| `far` | 遠距離 (10m+) | conf_threshold **+0.05** (厳格化) |

### 3.3 excluded_objects (除外オブジェクト)

検知結果から指定したオブジェクトを除外します。

```json
{
  "excluded_objects": ["car", "truck", "bus", "motorcycle", "bicycle"]
}
```

**利用シーン:**
- 屋内カメラで車両誤検知を除外
- 廊下カメラで動物検知を除外

### 3.4 expected_objects (期待オブジェクト)

指定されたオブジェクト以外は、confidence < 0.5 の場合に除外されます。

```json
{
  "expected_objects": ["person", "door"]
}
```

**注意:** confidence >= 0.5 のオブジェクトは除外されません（安全策）。

### 3.5 detection_roi (検知領域)

画像内の特定領域のみを検知対象とします。

```json
{
  "detection_roi": {
    "x1": 0.2,
    "y1": 0.0,
    "x2": 0.8,
    "y2": 1.0
  }
}
```

座標は画像サイズに対する0-1の正規化値です。

### 3.6 busy_hours / quiet_hours (時間帯)

| パラメータ | 効果 | 適用条件 |
|-----------|------|---------|
| `busy_hours` | suspicious_score **-10** | 指定時間帯内 |
| `quiet_hours` | suspicious_score **+15** | 指定時間帯内 |

```json
{
  "busy_hours": ["09:00-12:00", "15:00-18:00"],
  "quiet_hours": ["0:00-6:00", "23:00-24:00"]
}
```

---

## 4. 使用例

### 4.1 廊下カメラ (201号室前)

```bash
curl -X POST http://192.168.3.116:9000/v1/analyze \
  -F "infer_image=@camera.jpg" \
  -F "camera_id=cam_2f_201" \
  -F "schema_version=2025-12-29.1" \
  -F "captured_at=2025-12-30T15:30:00Z" \
  -F 'hints_json={
    "location_type": "corridor",
    "distance": "near",
    "excluded_objects": ["car", "truck", "bus"],
    "expected_objects": ["person"],
    "busy_hours": ["15:00-18:00"],
    "quiet_hours": ["0:00-6:00"]
  }'
```

### 4.2 駐車場カメラ

```bash
curl -X POST http://192.168.3.116:9000/v1/analyze \
  -F "infer_image=@parking.jpg" \
  -F "camera_id=cam_parking_b1" \
  -F "schema_version=2025-12-29.1" \
  -F "captured_at=2025-12-30T23:00:00Z" \
  -F 'hints_json={
    "location_type": "parking",
    "distance": "far",
    "expected_objects": ["car", "truck", "person"],
    "quiet_hours": ["22:00-6:00"]
  }'
```

### 4.3 制限区域カメラ

```bash
curl -X POST http://192.168.3.116:9000/v1/analyze \
  -F "infer_image=@restricted.jpg" \
  -F "camera_id=cam_server_room" \
  -F "schema_version=2025-12-29.1" \
  -F "captured_at=2025-12-30T14:00:00Z" \
  -F 'hints_json={
    "location_type": "restricted",
    "distance": "medium",
    "expected_objects": ["person"]
  }'
```

---

## 5. レスポンス

### 5.1 コンテキスト関連フィールド

```json
{
  "context_applied": true,
  "filtered_by_context": [
    "car excluded by context",
    "truck excluded by context"
  ],
  "suspicious": {
    "score": 73,
    "level": "critical",
    "factors": [
      "time.late_night[3:00] (+25)",
      "night+dark_clothing (+10)",
      "quiet_hours (+15)",
      "crouching_posture (+10)"
    ]
  }
}
```

| フィールド | 説明 |
|-----------|------|
| `context_applied` | コンテキストが適用されたか |
| `filtered_by_context` | フィルタリングされたオブジェクト |
| `suspicious.factors` | スコア計算の内訳 |

### 5.2 suspicious_score レベル

| スコア | レベル | 推奨アクション |
|--------|--------|---------------|
| 0-14 | normal | 通常ログ |
| 15-29 | low | 注意ログ |
| 30-49 | medium | 警告通知 |
| 50-69 | high | アラート |
| 70-100 | critical | 即時対応 |

---

## 6. IS22側実装例 (Python)

```python
import requests
import json
from datetime import datetime

class CameraConfig:
    """カメラ設定"""
    def __init__(self, camera_id: str, location_type: str, distance: str = "medium"):
        self.camera_id = camera_id
        self.location_type = location_type
        self.distance = distance
        self.excluded_objects = []
        self.expected_objects = []
        self.busy_hours = []
        self.quiet_hours = []

    def to_hints_json(self) -> str:
        context = {
            "location_type": self.location_type,
            "distance": self.distance,
        }
        if self.excluded_objects:
            context["excluded_objects"] = self.excluded_objects
        if self.expected_objects:
            context["expected_objects"] = self.expected_objects
        if self.busy_hours:
            context["busy_hours"] = self.busy_hours
        if self.quiet_hours:
            context["quiet_hours"] = self.quiet_hours

        return json.dumps({"camera_context": context})


def analyze_image(is21_url: str, camera: CameraConfig, image_path: str) -> dict:
    """IS21に画像を送信して解析"""
    with open(image_path, 'rb') as f:
        files = {'infer_image': f}
        data = {
            'camera_id': camera.camera_id,
            'schema_version': '2025-12-29.1',
            'captured_at': datetime.utcnow().isoformat() + 'Z',
            'hints_json': camera.to_hints_json()
        }
        response = requests.post(f"{is21_url}/v1/analyze", files=files, data=data)
        return response.json()


# 使用例
if __name__ == "__main__":
    # カメラ設定
    cam_201 = CameraConfig("cam_2f_201", "corridor", "near")
    cam_201.excluded_objects = ["car", "truck"]
    cam_201.expected_objects = ["person"]
    cam_201.busy_hours = ["15:00-18:00"]
    cam_201.quiet_hours = ["0:00-6:00"]

    # 解析実行
    result = analyze_image("http://192.168.3.116:9000", cam_201, "/path/to/image.jpg")

    # 結果処理
    if result.get("suspicious", {}).get("level") in ["high", "critical"]:
        print(f"⚠️ 警告: {result['suspicious']}")
```

---

## 7. 推奨設定パターン

### 7.1 ホテル廊下

```json
{
  "location_type": "corridor",
  "distance": "near",
  "excluded_objects": ["car", "truck", "bus", "motorcycle"],
  "expected_objects": ["person"],
  "busy_hours": ["15:00-18:00", "10:00-11:00"],
  "quiet_hours": ["0:00-6:00"]
}
```

### 7.2 駐車場 (屋外)

```json
{
  "location_type": "parking",
  "distance": "far",
  "expected_objects": ["car", "truck", "person", "motorcycle"],
  "quiet_hours": ["22:00-6:00"]
}
```

### 7.3 サーバールーム

```json
{
  "location_type": "restricted",
  "distance": "medium",
  "expected_objects": ["person"],
  "excluded_objects": ["car", "truck"]
}
```

### 7.4 エントランス

```json
{
  "location_type": "entrance",
  "distance": "medium",
  "expected_objects": ["person"],
  "busy_hours": ["08:00-20:00"]
}
```

---

## 8. 注意事項

1. **時間帯形式**: `"HH:MM-HH:MM"` 形式で指定。深夜跨ぎは `"23:00-24:00"` と `"0:00-6:00"` のように分けて指定。

2. **ROI指定**: `detection_roi` はbboxの中心点で判定。大きなオブジェクトの一部がROI外にはみ出しても検知されます。

3. **閾値調整**: `conf_override` は 0.2-0.8 の範囲で指定。範囲外は自動クリップされます。

4. **フィルタリング順序**: YOLO検出 → コンテキストフィルタ → PAR推論 の順で処理されます。

---

作成: Claude Code
バージョン: IS21 v1.8.0 対応
