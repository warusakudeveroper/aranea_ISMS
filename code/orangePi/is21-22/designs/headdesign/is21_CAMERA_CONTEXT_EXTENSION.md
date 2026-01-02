# is21 camera_context スキーマ拡張提案

## 改訂履歴
| 日付 | バージョン | 変更内容 |
|------|----------|----------|
| 2026-01-02 | 1.0.0 | 初版作成 |

---

## 1. 現状分析

### 1.1 現在対応済みパラメータ (v1.8.0)

| パラメータ | 型 | 用途 | suspicious影響 |
|-----------|-----|------|----------------|
| `location_type` | string | 設置場所タイプ | ○ |
| `distance` | string | 撮影距離 | 閾値調整のみ |
| `expected_objects` | string[] | 期待オブジェクト | フィルタ |
| `excluded_objects` | string[] | 除外オブジェクト | フィルタ |
| `busy_hours` | string[] | 繁忙時間帯 | -10点 |
| `quiet_hours` | string[] | 静寂時間帯 | +15点 |
| `detection_roi` | object | 検出領域 | フィルタ |
| `conf_override` | float | 信頼度閾値 | 閾値調整 |

### 1.2 現在のlocation_type値

| 値 | 説明 | suspicious影響 |
|----|------|----------------|
| `restricted` | 立入制限区域 | +20点 |
| `entrance` | 入口 | -5点 |
| `corridor` | 廊下 | pass（未実装） |
| `parking` | 駐車場 | 夜間+10点 |

### 1.3 処理能力余裕
- YOLOv5s: 38ms/frame
- PAR: 27ms/frame
- Total: ~200-300ms/frame
- **NPU使用率**: 低〜中程度（RK3588の能力に対して余裕あり）

---

## 2. 拡張提案

### 2.1 location_type 追加値

| 値 | 説明 | 推奨スコア影響 | 優先度 |
|----|------|---------------|--------|
| `outdoor` | 屋外一般 | 夜間+5点 | 高 |
| `lobby` | ロビー・待合 | -3点 | 高 |
| `office` | 事務所 | 非勤務時間+15点 | 中 |
| `storage` | 倉庫・物置 | +10点 | 中 |
| `server_room` | サーバールーム | +30点（最高警戒） | 高 |
| `emergency_exit` | 非常口 | +15点 | 高 |
| `stairs` | 階段 | 滞在時+10点 | 低 |
| `elevator_hall` | EVホール | 滞在時+5点 | 低 |
| `loading_dock` | 搬入口 | 深夜+20点 | 中 |
| `rooftop` | 屋上 | +25点 | 高 |

### 2.2 新規パラメータ提案

#### 2.2.1 camera_angle（撮影角度）
```json
{
  "camera_angle": "high|eye_level|low|overhead"
}
```
- `high`: 俯瞰（天井近く）→ 姿勢検出精度補正
- `eye_level`: 水平 → 標準
- `low`: 見上げ → 顔検出困難、体型重視
- `overhead`: 真上 → 頭頂部のみ、人数カウント特化

**is21への影響**: PAR精度の期待値調整、bbox面積比率補正

#### 2.2.2 staff_patterns（スタッフ識別）
```json
{
  "staff_patterns": {
    "uniform_colors": ["white", "blue"],
    "id_badge_expected": true,
    "exempt_hours": ["08:00-18:00"]
  }
}
```
- uniform_like + 指定色 → スタッフ判定で suspicious 減点
- 勤務時間外のスタッフ服装は逆に警戒

**is21への影響**: suspicious計算でスタッフ除外ロジック追加

#### 2.2.3 vehicle_context（車両コンテキスト）
```json
{
  "vehicle_context": {
    "expected_types": ["car", "truck"],
    "license_plate_region": {"x1": 0.3, "y1": 0.6, "x2": 0.7, "y2": 0.9},
    "unknown_vehicle_alert": true
  }
}
```
- parking/loading_dock での車両検知強化
- 見慣れない車両で alert

**is21への影響**: vehicle検出時の追加処理

#### 2.2.4 loitering_config（滞在検知）
```json
{
  "loitering_config": {
    "enabled": true,
    "warning_seconds": 60,
    "alert_seconds": 180,
    "zones": [{"x1": 0.2, "y1": 0.3, "x2": 0.8, "y2": 0.9}]
  }
}
```
- corridor の「将来的に滞在時間判定を追加」を実現
- is22側で連続フレーム分析、is21にヒント送信

**is21への影響**: is22からの時間情報を受け取る拡張

#### 2.2.5 alert_config（アラート設定）
```json
{
  "alert_config": {
    "suspicious_threshold": 50,
    "severity_override": null,
    "immediate_alert_events": ["mask_like", "crouching"],
    "cooldown_minutes": 5
  }
}
```
- カメラ固有のアラート閾値
- 特定タグで即時アラート

**is21への影響**: suspicious結果にalert推奨フラグ追加

#### 2.2.6 time_context（時間コンテキスト拡張）
```json
{
  "time_context": {
    "timezone": "Asia/Tokyo",
    "work_hours": ["09:00-18:00"],
    "holiday_calendar": "JP",
    "special_dates": ["2026-01-01", "2026-01-02"]
  }
}
```
- 休日・特別日の警戒レベル調整
- 勤務時間外の検知強化

**is21への影響**: busy/quiet_hoursの高度化

---

## 3. 実装優先度

### Phase 1（本ターン推奨）
スキーマ定義のみ追加、is21処理は最小限

| 項目 | 作業量 | 効果 |
|------|--------|------|
| location_type追加値 | 小 | 高 |
| camera_angle | 小 | 中 |
| alert_config.suspicious_threshold | 小 | 高 |

### Phase 2（次ターン）
is21側ロジック追加必要

| 項目 | 作業量 | 効果 |
|------|--------|------|
| staff_patterns | 中 | 高 |
| loitering_config | 大 | 高 |
| time_context | 中 | 中 |

### Phase 3（将来）
大規模改修

| 項目 | 作業量 | 効果 |
|------|--------|------|
| vehicle_context | 大 | 中 |
| 連続フレーム分析 | 大 | 高 |

---

## 4. is21スキーマ定義更新案

### 4.1 hints_json 完全スキーマ (v1.9.0提案)

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "is21 camera_context v1.9.0",
  "type": "object",
  "properties": {
    "location_type": {
      "type": "string",
      "enum": [
        "entrance", "corridor", "parking", "restricted",
        "outdoor", "lobby", "office", "storage", "server_room",
        "emergency_exit", "stairs", "elevator_hall", "loading_dock", "rooftop"
      ],
      "default": "indoor"
    },
    "distance": {
      "type": "string",
      "enum": ["near", "medium", "far"],
      "default": "medium"
    },
    "camera_angle": {
      "type": "string",
      "enum": ["high", "eye_level", "low", "overhead"],
      "default": "eye_level"
    },
    "expected_objects": {
      "type": "array",
      "items": {"type": "string"},
      "default": []
    },
    "excluded_objects": {
      "type": "array",
      "items": {"type": "string"},
      "default": []
    },
    "busy_hours": {
      "type": "array",
      "items": {"type": "string", "pattern": "^\\d{2}:\\d{2}-\\d{2}:\\d{2}$"},
      "default": []
    },
    "quiet_hours": {
      "type": "array",
      "items": {"type": "string", "pattern": "^\\d{2}:\\d{2}-\\d{2}:\\d{2}$"},
      "default": []
    },
    "detection_roi": {
      "type": "object",
      "properties": {
        "x1": {"type": "number", "minimum": 0, "maximum": 1},
        "y1": {"type": "number", "minimum": 0, "maximum": 1},
        "x2": {"type": "number", "minimum": 0, "maximum": 1},
        "y2": {"type": "number", "minimum": 0, "maximum": 1}
      }
    },
    "conf_override": {
      "type": "number",
      "minimum": 0.2,
      "maximum": 0.8
    },
    "alert_config": {
      "type": "object",
      "properties": {
        "suspicious_threshold": {"type": "integer", "minimum": 0, "maximum": 100, "default": 50},
        "immediate_alert_tags": {"type": "array", "items": {"type": "string"}}
      }
    },
    "staff_patterns": {
      "type": "object",
      "properties": {
        "uniform_colors": {"type": "array", "items": {"type": "string"}},
        "exempt_suspicious": {"type": "boolean", "default": true}
      }
    },
    "description": {
      "type": "string",
      "maxLength": 256,
      "description": "カメラ設置場所の説明（ログ・チャット検索用）"
    }
  }
}
```

---

## 5. is21 main.py 修正案（Phase 1）

### 5.1 location_type 追加

```python
# calculate_suspicious_score内
location_type = context.get('location_type', '')

if location_type == 'restricted':
    score += 20
    factors.append("restricted_area (+20)")
elif location_type == 'server_room':
    score += 30
    factors.append("server_room (+30)")
elif location_type == 'rooftop':
    score += 25
    factors.append("rooftop (+25)")
elif location_type == 'emergency_exit':
    score += 15
    factors.append("emergency_exit (+15)")
elif location_type == 'storage':
    score += 10
    factors.append("storage_area (+10)")
elif location_type == 'entrance':
    score -= 5
    factors.append("entrance_area (-5)")
elif location_type == 'lobby':
    score -= 3
    factors.append("lobby_area (-3)")
elif location_type == 'outdoor':
    if is_night:
        score += 5
        factors.append("outdoor+night (+5)")
elif location_type == 'office':
    # 非勤務時間チェック（work_hoursと連携）
    work_hours = context.get('work_hours', ["09:00-18:00"])
    if hour is not None and not is_in_time_range(hour, work_hours):
        score += 15
        factors.append("office+after_hours (+15)")
elif location_type == 'loading_dock':
    if hour is not None and (0 <= hour < 6):
        score += 20
        factors.append("loading_dock+late_night (+20)")
elif location_type == 'parking':
    if is_night:
        score += 10
        factors.append("parking+night (+10)")
```

### 5.2 camera_angle 対応

```python
# apply_context_filters内またはPAR処理前
camera_angle = context.get('camera_angle', 'eye_level')

# overheadの場合は姿勢検出をスキップ（精度低いため）
if camera_angle == 'overhead':
    # posture関連タグを除外
    tags = [t for t in tags if not t.startswith('posture.')]
```

---

## 6. CameraDetailModal UI更新案

### 6.1 コンテキスト設定タブ追加

```
[基本情報] [映像設定] [コンテキスト設定] [アラート設定]

── コンテキスト設定 ──────────────────────

設置場所タイプ
[入口 ▼] ← enum選択

撮影角度
( ) 俯瞰（天井）  (●) 水平  ( ) 見上げ  ( ) 真上

撮影距離
( ) 近距離  (●) 中距離  ( ) 遠距離

期待オブジェクト
[✓] 人物  [✓] 車両  [ ] 動物

除外オブジェクト
[ ] 猫  [ ] 犬  [✓] 鳥

検出領域（ROI）
[□ 全体] [■ 範囲指定] → プレビュー上でドラッグ描画

── 時間設定 ──────────────────────────

繁忙時間帯（警戒緩和）
[08:00-10:00] [17:00-19:00] [+追加]

静寂時間帯（警戒強化）
[23:00-06:00] [+追加]

── アラート設定 ──────────────────────

不審スコア閾値
[====●====] 50

即時アラートタグ
[✓] マスク着用  [✓] しゃがみ姿勢  [ ] バックパック

── 説明 ──────────────────────────────

設置場所メモ（検索用）
[1F正面玄関、来客用入口          ]
```

---

## 7. 実装作業

### 7.1 is21側（Phase 1）
- [ ] location_type追加値の処理追加
- [ ] camera_angle処理追加
- [ ] スキーマバージョン1.9.0更新
- [ ] README更新

### 7.2 is22側（Phase 1）
- [ ] cameras.camera_context JSONスキーマ更新
- [ ] CameraDetailModal コンテキスト設定タブ追加
- [ ] PollingOrchestrator camera_context送信確認

### 7.3 テスト
- [ ] 全location_typeでのスコア変動確認
- [ ] camera_angle別精度比較
- [ ] UI保存・復元確認

---

## 8. 結論

is21の処理能力には余裕があり、camera_contextスキーマ拡張は**Phase 1として本ターンで実施推奨**。

主な拡張:
1. **location_type追加**: 10種類追加（server_room, rooftop等の高警戒エリア対応）
2. **camera_angle**: 撮影角度による検出精度補正
3. **alert_config**: カメラ固有の閾値設定
4. **description**: 検索用メモフィールド

is21側の修正は`calculate_suspicious_score`関数内の条件分岐追加（約50行）で対応可能。
