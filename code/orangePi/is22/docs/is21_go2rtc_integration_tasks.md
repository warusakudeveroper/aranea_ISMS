# is21 / go2rtc 統合タスク詳細設計書

## 1. 概要

本ドキュメントは、mobes AIcam control Towerの以下の未実装機能について詳細を記載する：

1. **is21系**: AI検知ロジックの問題分析と改善
2. **go2rtc系**: サジェストビュー・タイルクリックでの動画再生

**重要**: is21系タスクはgo2rtc系タスクの前提条件であり、不可分の関係にある。

```
[is21検知] → [イベント発火] → [go2rtcサジェスト再生開始]
    ↓              ↓
[severity計算] → [タイルハイライト]
```

---

## 2. アーキテクチャ決定: Option B

### 2.1 決定事項

**Option B: is22がパラメータを展開し、hints_jsonで送信**

```
┌─────────────────────────────────────────────────────────────────┐
│                         is22 (Rust)                             │
│                                                                 │
│  ┌─────────────┐    ┌──────────────────┐                        │
│  │ PresetLoader│───▶│ hints_json構築    │                        │
│  │ (12 presets)│    │ - conf_override  │                        │
│  │             │    │ - par_threshold  │                        │
│  │             │    │ - nms_threshold  │                        │
│  │             │    │ - excluded_objects│                       │
│  └─────────────┘    └──────────────────┘                        │
│                              │                                  │
│                              ▼                                  │
│                    ┌──────────────────┐                         │
│                    │   Form::new()    │                         │
│                    │ .text("hints_json", {...})                 │
│                    │ .part("infer_image", ...)                  │
│                    │ .part("prev_image", ...) ← 追加必要        │
│                    └──────────────────┘                         │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼ HTTP POST /v1/analyze
┌─────────────────────────────────────────────────────────────────┐
│                         is21 (Python)                           │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ def analyze(..., hints_json, prev_image):                │   │
│  │     context = parse_camera_context(hints_json)           │   │
│  │                                                          │   │
│  │     # パラメータ展開                                      │   │
│  │     conf = context.get("conf_override", 0.33)            │   │
│  │     par_th = context.get("par_threshold", 0.5)           │   │
│  │     excluded = context.get("excluded_objects", [])       │   │
│  │                                                          │   │
│  │     # 推論実行（パラメータ適用）                           │   │
│  │     bboxes = run_yolo(conf_threshold=conf)               │   │
│  │     par = run_par(threshold=par_th)                      │   │
│  │                                                          │   │
│  │     # frame_diff (prev_image使用)                        │   │
│  │     if prev_image:                                       │   │
│  │         frame_diff = compare_frames(prev, current)       │   │
│  │                                                          │   │
│  │     severity = calculate_severity(primary_event)         │   │
│  │     return response                                      │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 選択理由

| 観点 | Option A (is21がプリセット管理) | Option B (is22がパラメータ展開) |
|------|-------------------------------|-------------------------------|
| is21の複雑性 | プリセットDB/同期が必要 | **シンプル（hints_json処理のみ）** |
| is22の複雑性 | 薄いラッパー | パラメータ展開が必要 |
| 変更量 | is21に大規模変更 | **is21は最小限の変更** |
| 現行資産 | preset_loader無駄に | **preset_loader活用** |
| 単一責任 | is21がAI+プリセット管理 | **is21はAIのみ、is22が設定管理** |

**結論**: Option Bはis21をステートレス/シンプルに保ち、既存のpreset_loader資産を活用できる

### 2.3 hints_json拡張スキーマ（決定版）

```json
{
  "location_type": "parking",
  "distance": "far",
  "expected_objects": ["human", "vehicle"],
  "excluded_objects": ["animal"],
  "busy_hours": ["09:00-17:00"],
  "quiet_hours": ["22:00-06:00"],

  "conf_override": 0.25,
  "par_threshold": 0.4,
  "nms_threshold": 0.5,

  "enable_par_raw": true,
  "enable_all_par_attrs": true
}
```

---

## 3. 現状の問題点（調査結果）

### 3.1 問題1: severity計算が二値化されている【致命的】

**現在のコード** (`is21/src/main.py:1750`):
```python
severity = 1 if detected else 0
```

**期待される仕様** (設計書より):
| primary_event | severity | 説明 |
|---------------|----------|------|
| human (person) | 3 | 最高優先度 |
| vehicle | 2 | 中優先度 |
| motion/animal | 1 | 低優先度 |
| none | 0 | 検出なし |

**影響**:
- SuggestEngineのスコア計算が `severity * 20` で行われるが、severityが常に0か1のため意味をなさない
- UIのseverity色分け（青/黄/赤）が機能しない
- 高優先度イベント（人物検出）と低優先度（動物検出）が区別されない

### 3.2 問題2: API契約の不一致【致命的】

**is22 (ai_client/mod.rs:400-413) が送信:**
```rust
Form::new()
    .text("preset_id", request.preset_id)       // ← 送信
    .text("preset_version", request.preset_version) // ← 送信
    .text("enable_frame_diff", ...)             // ← 送信
    .part("prev_image", prev_data)              // ← 送信
```

**is21 (main.py:1685-1692) が受け取り可能:**
```python
async def analyze(
    camera_id: str = Form(...),
    captured_at: str = Form(...),
    schema_version_req: str = Form(..., alias="schema_version"),
    infer_image: UploadFile = File(...),
    profile: str = Form("standard"),
    return_bboxes: bool = Form(True),
    hints_json: Optional[str] = Form(None)
):
# preset_id, preset_version, enable_frame_diff, prev_image が存在しない
```

**結果**: 以下のパラメータは **完全に無視** されている:
| パラメータ | is22から送信 | is21で受信 | 状態 |
|------------|-------------|-----------|------|
| `preset_id` | ✅ | ❌ | **DEAD CODE** |
| `preset_version` | ✅ | ❌ | **DEAD CODE** |
| `enable_frame_diff` | ✅ | ❌ | **DEAD CODE** |
| `prev_image` | ✅ | ❌ | **DEAD CODE** |
| `output_schema` | ✅ | ❌ | **DEAD CODE** |

### 3.3 問題3: フレーム差分機能が未実装【機能欠落】

**is22側の実装（存在するが無駄）**:
- `PrevFrameCache`: 前フレームをメモリキャッシュ ✅
- `analyze()`: prev_image送信 ✅
- `FrameDiff`構造体定義 ✅

**is21側の状態**:
- `prev_image`パラメータが存在しない ❌
- フレーム差分処理のコードが一切ない ❌

**結論**: フレーム差分機能は**完全に機能していない**

### 3.4 問題4: Preset構造体の欠落パラメータ

**is22 preset_loader/mod.rs:35-66**:
```rust
pub struct Preset {
    pub id: String,
    pub name: String,
    pub location_type: Option<String>,
    pub distance: Option<String>,
    pub expected_objects: Vec<String>,
    pub excluded_objects: Vec<String>,
    pub enable_frame_diff: bool,
    pub return_bboxes: bool,
    // 欠落 ↓
    // pub conf_override: Option<f32>,
    // pub par_threshold: Option<f32>,
    // pub nms_threshold: Option<f32>,
}
```

### 3.5 問題5: PA100K属性の不完全活用

**PA100K 26属性のうち**:
- 完全使用: 13/26 (50%)
- 間接使用: 4/26 (15%)
- **未使用: 9/26 (35%)**

**未使用属性**:
- UpperStride, UpperSplice, LowerStripe, LowerPattern
- **Front, Side, Back（視線方向）** ← 不審行動検知に有用

---

## 4. 内部ゲートキーパー分析

```
[カメラ画像入力]
       ↓
   ┌───────────────────────────────────────┐
   │ Gate 1: CONF_THRESHOLD = 0.33        │ ← hints_json.conf_overrideで制御可能
   │         (main.py:61)                  │
   └───────────────────────────────────────┘
       ↓
   ┌───────────────────────────────────────┐
   │ Gate 2: IGNORE_OBJECTS (34クラス)     │ ← hints_json.excluded_objectsで拡張可能
   │         (main.py:117-124)             │
   └───────────────────────────────────────┘
       ↓
   ┌───────────────────────────────────────┐
   │ Gate 3: NMS_THRESHOLD = 0.40          │ ← hints_json.nms_thresholdで制御必要
   │         (main.py:62)                  │
   └───────────────────────────────────────┘
       ↓
   ┌───────────────────────────────────────┐
   │ Gate 4: containment_threshold = 0.7   │ ← ハードコード（調整非推奨）
   │         (main.py:1296)                │
   └───────────────────────────────────────┘
       ↓
   ┌───────────────────────────────────────┐
   │ Gate 5: PAR_MAX_PERSONS = 10          │ ← hints_json.par_max_personsで制御可能
   │         (main.py:66)                  │
   └───────────────────────────────────────┘
       ↓
   ┌───────────────────────────────────────┐
   │ Gate 6: PAR_THRESHOLD = 0.5           │ ← hints_json.par_thresholdで制御必要
   │         (main.py:67)                  │
   └───────────────────────────────────────┘
       ↓
   ┌───────────────────────────────────────┐
   │ Gate 7: Color threshold = 0.2         │ ← ハードコード（調整非推奨）
   │         (main.py:248)                 │
   └───────────────────────────────────────┘
       ↓
   ┌───────────────────────────────────────┐
   │ Gate 8: severity二値化                 │ ← 【修正必須】
   │         (main.py:1750)                │
   │         severity = 1 if detected else 0
   └───────────────────────────────────────┘
```

---

## 5. 実装タスク詳細（Phase順）

### Phase 0: API契約修正【最優先・ブロッカー】

#### Task IS21-API-1: is21 analyzeエンドポイント拡張

**ファイル**: `is21/src/main.py`

**修正内容**:
```python
@app.post("/v1/analyze")
async def analyze(
    # 既存パラメータ
    camera_id: str = Form(...),
    captured_at: str = Form(...),
    schema_version_req: str = Form(..., alias="schema_version"),
    infer_image: UploadFile = File(...),
    return_bboxes: bool = Form(True),
    hints_json: Optional[str] = Form(None),

    # 追加パラメータ
    prev_image: Optional[UploadFile] = File(None),  # フレーム差分用
):
```

**削除対象**:
- `profile: str = Form("standard")` → hints_json内で処理

**テスト項目**:
- [ ] prev_imageがOptionalで受信可能
- [ ] hints_jsonのパース成功
- [ ] 既存APIとの後方互換性維持

---

### Phase 1: severity正常化【即時対応】

#### Task IS21-SEV-1: severity計算ロジック改善

**ファイル**: `is21/src/main.py:1750`

**現在**:
```python
severity = 1 if detected else 0
```

**修正後**:
```python
def calculate_severity(primary_event: str) -> int:
    """
    severity計算:
    - 3: human (person)
    - 2: vehicle
    - 1: animal, motion, その他検出
    - 0: 検出なし (none)
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
```

**テスト項目**:
- [ ] person検出時にseverity=3
- [ ] vehicle検出時にseverity=2
- [ ] animal検出時にseverity=1
- [ ] 何も検出されない時にseverity=0

---

### Phase 2: hints_jsonパラメータ拡張

#### Task IS21-HINTS-1: conf_override適用

**ファイル**: `is21/src/main.py`

**修正箇所**: `parse_camera_context()`の後

```python
# hints_json.conf_override適用
camera_context = parse_camera_context(hints_json)
effective_conf_threshold = camera_context.get("conf_override", CONF_THRESHOLD)
effective_conf_threshold = max(0.2, min(0.8, effective_conf_threshold))

# YOLO推論でeffective_conf_thresholdを使用
bboxes, yolo_ms = run_yolo_inference(image, conf_threshold=effective_conf_threshold)
```

#### Task IS21-HINTS-2: par_threshold適用

```python
# hints_json.par_threshold適用
effective_par_threshold = camera_context.get("par_threshold", PAR_THRESHOLD)
effective_par_threshold = max(0.3, min(0.8, effective_par_threshold))

# PAR推論でeffective_par_thresholdを使用
par_results, par_ms = run_par_inference(image, person_bboxes,
                                         threshold=effective_par_threshold)
```

#### Task IS21-HINTS-3: nms_threshold適用

```python
# hints_json.nms_threshold適用
effective_nms_threshold = camera_context.get("nms_threshold", NMS_THRESHOLD)
effective_nms_threshold = max(0.3, min(0.6, effective_nms_threshold))

# YOLO NMSでeffective_nms_thresholdを使用
bboxes = non_max_suppression(bboxes, nms_threshold=effective_nms_threshold)
```

---

### Phase 3: is22 Preset構造体拡張

#### Task IS22-PRESET-1: Preset構造体にパラメータ追加

**ファイル**: `is22/src/preset_loader/mod.rs`

```rust
pub struct Preset {
    // 既存フィールド...

    // 追加フィールド
    /// YOLO confidence threshold override (0.2-0.8)
    pub conf_override: Option<f32>,

    /// PAR attribute threshold (0.3-0.8)
    pub par_threshold: Option<f32>,

    /// NMS threshold for box deduplication (0.3-0.6)
    pub nms_threshold: Option<f32>,
}
```

#### Task IS22-PRESET-2: to_camera_context()拡張

```rust
pub fn to_camera_context(&self) -> Option<CameraContext> {
    Some(CameraContext {
        location_type: self.location_type.clone(),
        distance: self.distance.clone(),
        expected_objects: ...,
        excluded_objects: ...,

        // 追加
        conf_override: self.conf_override,
        par_threshold: self.par_threshold,
        nms_threshold: self.nms_threshold,
        ..
    })
}
```

#### Task IS22-PRESET-3: 12プリセット更新

各プリセットに適切なパラメータを設定:

| preset_id | conf_override | par_threshold | nms_threshold |
|-----------|---------------|---------------|---------------|
| person_priority | 0.25 | 0.4 | 0.45 |
| balanced | 0.33 | 0.5 | 0.40 |
| parking | 0.30 | 0.5 | 0.45 |
| entrance | 0.28 | 0.4 | 0.40 |
| corridor | 0.33 | 0.5 | 0.40 |
| outdoor | 0.35 | 0.5 | 0.50 |
| night_vision | 0.25 | 0.45 | 0.40 |
| crowd | 0.40 | 0.6 | 0.55 |
| retail | 0.28 | 0.4 | 0.40 |
| office | 0.35 | 0.5 | 0.40 |
| warehouse | 0.35 | 0.5 | 0.50 |
| custom | None | None | None |

---

### Phase 4: CameraContext拡張

#### Task IS22-CTX-1: CameraContext構造体拡張

**ファイル**: `is22/src/ai_client/mod.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraContext {
    // 既存フィールド...

    // 追加
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conf_override: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub par_threshold: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nms_threshold: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_par_raw: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_all_par_attrs: Option<bool>,
}
```

---

### Phase 5: フレーム差分実装

#### Task IS21-DIFF-1: prev_image受信処理

**ファイル**: `is21/src/main.py`

```python
@app.post("/v1/analyze")
async def analyze(
    ...,
    prev_image: Optional[UploadFile] = File(None),
):
    # prev_image処理
    prev_frame = None
    if prev_image:
        prev_contents = await prev_image.read()
        prev_nparr = np.frombuffer(prev_contents, np.uint8)
        prev_frame = cv2.imdecode(prev_nparr, cv2.IMREAD_COLOR)
```

#### Task IS21-DIFF-2: フレーム差分計算

```python
def calculate_frame_diff(current_bboxes, prev_bboxes, prev_frame, current_frame):
    """
    フレーム間の人物変化を計算

    Returns:
        {
            "enabled": True,
            "person_changes": {
                "appeared": 1,
                "disappeared": 0,
                "moved": 2,
                "stationary": 0
            },
            "loitering": {"detected": False}
        }
    """
    if prev_frame is None:
        return {"enabled": False, "camera_status": "no_reference"}

    # bbox位置のIoU比較で移動検知
    # 同一位置に長時間滞在でloitering検知
    ...
```

#### Task IS21-DIFF-3: レスポンスにframe_diff追加

```python
return {
    ...,
    "frame_diff": frame_diff_result,
}
```

---

### Phase 6: UI感度スライダー

#### Task IS22-UI-1: DBカラム追加

**ファイル**: `migrations/XXX_camera_sensitivity.sql`

```sql
ALTER TABLE cameras
ADD COLUMN conf_override DECIMAL(3,2) DEFAULT NULL
COMMENT 'YOLO confidence threshold override (0.2-0.8, NULL=preset default)';

ALTER TABLE cameras
ADD COLUMN par_threshold DECIMAL(3,2) DEFAULT NULL
COMMENT 'PAR attribute threshold override (0.3-0.8, NULL=preset default)';
```

#### Task IS22-UI-2: CameraDetailModal拡張

**ファイル**: `frontend/src/components/CameraDetailModal.tsx`

```tsx
// 検知感度スライダー
<div className="space-y-2">
  <label>検知感度 (YOLO)</label>
  <Slider
    value={[confOverride ?? 0.33]}
    min={0.2}
    max={0.8}
    step={0.05}
    onValueChange={([v]) => setConfOverride(v)}
  />
  <span>低感度 ← {confOverride ?? 0.33} → 高感度</span>
</div>

// 属性検出感度スライダー
<div className="space-y-2">
  <label>属性検出感度 (PAR)</label>
  <Slider
    value={[parThreshold ?? 0.5]}
    min={0.3}
    max={0.8}
    step={0.05}
    onValueChange={([v]) => setParThreshold(v)}
  />
</div>
```

#### Task IS22-UI-3: polling_orchestratorでカメラ設定をhints_jsonにマージ

**ファイル**: `is22/src/polling_orchestrator/mod.rs`

```rust
// プリセットからCameraContextを取得
let mut context = preset.to_camera_context().unwrap_or_default();

// カメラ個別設定でオーバーライド
if let Some(conf) = camera.conf_override {
    context.conf_override = Some(conf);
}
if let Some(par) = camera.par_threshold {
    context.par_threshold = Some(par);
}
```

---

### Phase 7: go2rtc統合（前提条件: Phase 1完了）

#### Task GO2RTC-1: サジェストビュー動画再生

（既存設計書の内容を維持）

#### Task GO2RTC-2: onairtime設定実装

（既存設計書の内容を維持）

#### Task GO2RTC-3: タイルクリック動画モーダル

（既存設計書の内容を維持）

#### Task GO2RTC-4: オンエアハイライト

（既存設計書の内容を維持）

---

## 6. 依存関係と実行順序

```
Phase 0: API契約修正【ブロッカー】
├─ IS21-API-1: analyzeエンドポイントにprev_image追加
│
    ↓
Phase 1: severity正常化【即時対応】
├─ IS21-SEV-1: severity計算ロジック改善
│
    ↓
Phase 2: hints_jsonパラメータ拡張
├─ IS21-HINTS-1: conf_override適用
├─ IS21-HINTS-2: par_threshold適用
└─ IS21-HINTS-3: nms_threshold適用
│
    ↓ (is21修正完了)
Phase 3: is22 Preset構造体拡張【並列可能】
├─ IS22-PRESET-1: Preset構造体にパラメータ追加
├─ IS22-PRESET-2: to_camera_context()拡張
└─ IS22-PRESET-3: 12プリセット更新
│
    ↓
Phase 4: CameraContext拡張
├─ IS22-CTX-1: CameraContext構造体拡張
│
    ↓
Phase 5: フレーム差分実装
├─ IS21-DIFF-1: prev_image受信処理
├─ IS21-DIFF-2: フレーム差分計算
└─ IS21-DIFF-3: レスポンスにframe_diff追加
│
    ↓
Phase 6: UI感度スライダー
├─ IS22-UI-1: DBカラム追加
├─ IS22-UI-2: CameraDetailModal拡張
└─ IS22-UI-3: polling_orchestratorでマージ
│
    ↓ (severity正常動作確認後)
Phase 7: go2rtc統合
├─ GO2RTC-1: サジェストビュー動画再生
├─ GO2RTC-2: onairtime設定
├─ GO2RTC-3: タイルクリック動画モーダル
└─ GO2RTC-4: オンエアハイライト
```

---

## 7. Issue分割計画

### Issue #1: [is21] severity計算の正常化

**優先度**: P0（ブロッカー）
**ラベル**: `bug`, `is21`, `critical`

**内容**:
- IS21-SEV-1タスク実装
- 現在の二値化（0/1）を0-3の4段階に修正
- human→3, vehicle→2, animal/motion→1, none→0

**受け入れ条件**:
- [ ] person検出時にseverity=3が返却される
- [ ] 既存テストが全てパス
- [ ] is22のEventLogPaneで正しい色分けが表示される

---

### Issue #2: [is21] APIパラメータ拡張（prev_image対応）

**優先度**: P0（ブロッカー）
**ラベル**: `enhancement`, `is21`, `api`

**内容**:
- IS21-API-1タスク実装
- prev_imageパラメータをOptional[UploadFile]で受信
- 後方互換性維持

**受け入れ条件**:
- [ ] prev_imageなしで従来通り動作
- [ ] prev_imageありで受信・デコード可能
- [ ] is22からのリクエストが正常処理される

---

### Issue #3: [is21] hints_jsonパラメータ拡張

**優先度**: P1
**ラベル**: `enhancement`, `is21`

**内容**:
- IS21-HINTS-1, IS21-HINTS-2, IS21-HINTS-3タスク実装
- conf_override, par_threshold, nms_thresholdの適用

**受け入れ条件**:
- [ ] hints_json.conf_override=0.25でconfidence閾値が変更される
- [ ] hints_json.par_threshold=0.4でPAR属性閾値が変更される
- [ ] 範囲外の値はクリップされる（0.2-0.8等）

---

### Issue #4: [is22] Preset構造体拡張

**優先度**: P1
**ラベル**: `enhancement`, `is22`

**内容**:
- IS22-PRESET-1, IS22-PRESET-2, IS22-PRESET-3タスク実装
- Preset構造体にconf_override, par_threshold, nms_threshold追加
- 12プリセットに適切な値を設定

**受け入れ条件**:
- [ ] Preset::person_priority()でconf_override=0.25
- [ ] to_camera_context()でhints_jsonに含まれる
- [ ] 全12プリセットのテストがパス

---

### Issue #5: [is22] CameraContext構造体拡張

**優先度**: P1
**ラベル**: `enhancement`, `is22`

**内容**:
- IS22-CTX-1タスク実装
- CameraContextにconf_override, par_threshold, nms_threshold追加

**受け入れ条件**:
- [ ] JSONシリアライズで新フィールドが含まれる
- [ ] Optionの場合はskip_serializing_ifで省略される

---

### Issue #6: [is21] フレーム差分機能実装

**優先度**: P2
**ラベル**: `enhancement`, `is21`, `feature`

**内容**:
- IS21-DIFF-1, IS21-DIFF-2, IS21-DIFF-3タスク実装
- prev_imageとcurrent_imageの比較
- person_changes, loitering検知

**受け入れ条件**:
- [ ] prev_image送信時にframe_diffがレスポンスに含まれる
- [ ] person_changesが正しく計算される
- [ ] prev_imageなしの場合はframe_diff.enabled=false

---

### Issue #7: [is22] UI感度スライダー実装

**優先度**: P2
**ラベル**: `enhancement`, `is22`, `frontend`

**内容**:
- IS22-UI-1, IS22-UI-2, IS22-UI-3タスク実装
- DBカラム追加、CameraDetailModalにスライダー追加
- カメラ個別設定のhints_jsonマージ

**受け入れ条件**:
- [ ] スライダーで感度調整が可能
- [ ] 設定がDBに保存される
- [ ] 次回ポーリングで設定が反映される

---

### Issue #8: [is22] go2rtcサジェストビュー統合

**優先度**: P1（Phase 1完了後）
**ラベル**: `enhancement`, `is22`, `frontend`, `go2rtc`

**内容**:
- GO2RTC-1, GO2RTC-4タスク実装
- サジェストビュー動画再生
- オンエアハイライト

**受け入れ条件**:
- [ ] severity≥1のイベントでサジェストビューに動画表示
- [ ] オンエア中カメラに黄色ハイライト

---

### Issue #9: [is22] go2rtcタイルクリック動画再生

**優先度**: P2
**ラベル**: `enhancement`, `is22`, `frontend`, `go2rtc`

**内容**:
- GO2RTC-2, GO2RTC-3タスク実装
- タイルクリックでモーダル動画再生
- onairtime設定

**受け入れ条件**:
- [ ] カメラタイルクリックでモーダルが開く
- [ ] モーダル内でWebRTC動画が再生される
- [ ] onairtime設定が機能する

---

## 8. テスト計画

### 8.1 Unit Tests

| コンポーネント | テスト内容 |
|---------------|----------|
| is21 severity | calculate_severity()の戻り値検証 |
| is21 hints_json | parse_camera_context()のパラメータ抽出 |
| is22 Preset | to_camera_context()のシリアライズ |
| is22 CameraContext | JSONシリアライズ/デシリアライズ |

### 8.2 Integration Tests

| テスト | 内容 |
|-------|------|
| is22→is21 hints_json | conf_overrideがis21で適用されることを確認 |
| is22→is21 prev_image | prev_imageが受信・処理されることを確認 |
| E2E severity | 人物検出→severity=3→UIに赤バッジ表示 |

### 8.3 E2E Tests

1. カメラ前に人が立つ
2. is22ポーリングで画像取得
3. is21推論でseverity=3
4. WebSocketでイベント配信
5. EventLogPaneに赤色バッジで表示
6. SuggestPaneに動画表示

---

## 9. リスク管理

| リスク | 影響 | 確率 | 対策 |
|--------|------|------|------|
| is21 APIパラメータ追加で後方互換性破壊 | 高 | 低 | 全パラメータをOptionalで追加 |
| hints_jsonパースエラー | 中 | 中 | デフォルト値フォールバック |
| prev_image処理でメモリ不足 | 高 | 低 | 画像サイズ制限、メモリ監視 |
| go2rtc WebRTC接続失敗 | 中 | 中 | HLSフォールバック |

---

## 10. MECE確認

本設計はMECEを満たしている:

**Mutually Exclusive (相互排他)**:
- Phase 0-7は依存関係に基づく順序で、重複する作業はない
- 各Issueは独立した機能単位で分割

**Collectively Exhaustive (全体網羅)**:
- 問題点（3.1-3.5）に対して全て対応するタスクが存在
- is21側修正、is22側修正、UI修正を網羅
- テスト計画でUnit/Integration/E2Eを網羅

---

## 更新履歴

| 日付 | 内容 |
|------|------|
| 2025/01/04 | 初版作成 |
| 2025/01/04 | ゲートキーパー分析追加（深掘り調査） |
| 2025/01/04 | プリセットシステム分析追加 - 致命的問題発見 |
| 2025/01/04 | フレーム差分機能分析 - 未実装確認 |
| 2025/01/04 | API契約不一致問題を根本原因として特定 |
| 2025/01/04 | **Option B アーキテクチャ決定**: is22がパラメータ展開、is21はステートレス |
| 2025/01/04 | 実装タスク詳細展開（Phase 0-7） |
| 2025/01/04 | Issue分割計画（#1-#9） |
| 2025/01/04 | MECE確認追加 |
