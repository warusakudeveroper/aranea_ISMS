# タイムアウト設定機能 詳細設計

**親ドキュメント**: [TIMEOUT_SETTINGS_DESIGN.md](./TIMEOUT_SETTINGS_DESIGN.md)
**作成日**: 2026-01-06

## Phase 1: グローバルタイムアウト設定実装

### 1.1 データベース設計

#### 1.1.1 settings.polling 拡張（完了済み）

**現在の状態**:
```json
{
  "base_interval_sec": 60,
  "priority_interval_sec": 15,
  "max_concurrent": 5,
  "timeout_ms": 10000,
  "timeout_main_sec": 10,
  "timeout_sub_sec": 20
}
```

**検証**:
- ✅ `timeout_main_sec`と`timeout_sub_sec`が追加済み
- ✅ デフォルト値（10秒/20秒）が設定済み

**MECE検証**:
- ✅ 漏れなし: Main/Sub両方のタイムアウトが定義されている
- ✅ ダブりなし: 各ストリームに対するタイムアウトは一意

### 1.2 Backend API設計

#### 1.2.1 GET /api/settings/timeouts

**Purpose**: グローバルタイムアウト設定を取得

**Request**:
```
GET /api/settings/timeouts HTTP/1.1
```

**Response** (成功時):
```json
{
  "ok": true,
  "data": {
    "timeout_main_sec": 10,
    "timeout_sub_sec": 20
  }
}
```

**Response** (エラー時):
```json
{
  "ok": false,
  "error": "Failed to load settings"
}
```

**実装場所**: `src/web_api/routes.rs`

**実装詳細**:
```rust
async fn get_global_timeouts(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // settings.pollingからtimeout_main_sec, timeout_sub_secを取得
    let result = sqlx::query(
        "SELECT setting_json FROM settings WHERE setting_key = 'polling'"
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let setting_json: String = result.get("setting_json");
    let polling_settings: serde_json::Value = serde_json::from_str(&setting_json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let timeout_main = polling_settings["timeout_main_sec"]
        .as_u64()
        .unwrap_or(10);
    let timeout_sub = polling_settings["timeout_sub_sec"]
        .as_u64()
        .unwrap_or(20);

    Ok(Json(json!({
        "ok": true,
        "data": {
            "timeout_main_sec": timeout_main,
            "timeout_sub_sec": timeout_sub
        }
    })))
}
```

**エッジケース処理**:
- settings.pollingが存在しない → デフォルト値(10/20)を返す
- JSONパースエラー → 500エラー
- フィールドが存在しない → デフォルト値(10/20)を返す

#### 1.2.2 PUT /api/settings/timeouts

**Purpose**: グローバルタイムアウト設定を更新

**Request**:
```json
{
  "timeout_main_sec": 15,
  "timeout_sub_sec": 30
}
```

**Validation**:
- `timeout_main_sec`: 5 <= value <= 120
- `timeout_sub_sec`: 5 <= value <= 120
- 型チェック: 整数のみ許可

**Response** (成功時):
```json
{
  "ok": true,
  "message": "Timeout settings updated"
}
```

**Response** (バリデーションエラー):
```json
{
  "ok": false,
  "error": "timeout_main_sec must be between 5 and 120"
}
```

**実装詳細**:
```rust
#[derive(Deserialize)]
struct UpdateTimeoutRequest {
    timeout_main_sec: u64,
    timeout_sub_sec: u64,
}

async fn update_global_timeouts(
    State(state): State<AppState>,
    Json(payload): Json<UpdateTimeoutRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Validation
    if payload.timeout_main_sec < 5 || payload.timeout_main_sec > 120 {
        return Ok(Json(json!({
            "ok": false,
            "error": "timeout_main_sec must be between 5 and 120"
        })));
    }
    if payload.timeout_sub_sec < 5 || payload.timeout_sub_sec > 120 {
        return Ok(Json(json!({
            "ok": false,
            "error": "timeout_sub_sec must be between 5 and 120"
        })));
    }

    // Update settings
    sqlx::query(
        "UPDATE settings
         SET setting_json = JSON_SET(
             setting_json,
             '$.timeout_main_sec', ?,
             '$.timeout_sub_sec', ?
         )
         WHERE setting_key = 'polling'"
    )
    .bind(payload.timeout_main_sec)
    .bind(payload.timeout_sub_sec)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Refresh ConfigStore cache
    state.config_store.refresh_cache().await;

    Ok(Json(json!({
        "ok": true,
        "message": "Timeout settings updated"
    })))
}
```

**ルート登録**:
```rust
// src/web_api/routes.rs のbuild_router内
.route("/api/settings/timeouts", get(get_global_timeouts))
.route("/api/settings/timeouts", put(update_global_timeouts))
```

### 1.3 SnapshotService修正

#### 1.3.1 現在の実装

**ファイル**: `src/snapshot_service/mod.rs`

**現在のコード** (lines 62-65, 94-95):
```rust
pub struct SnapshotService {
    ffmpeg_timeout_main: u64,  // ハードコード: 10
    ffmpeg_timeout_sub: u64,   // ハードコード: 20
    // ...
}

impl SnapshotService {
    pub fn new(/* ... */) -> Self {
        Self {
            ffmpeg_timeout_main: 10,  // ← これを動的に
            ffmpeg_timeout_sub: 20,   // ← これを動的に
            // ...
        }
    }
}
```

#### 1.3.2 修正後の設計

**変更点**:
1. `new()`にタイムアウトパラメータを追加
2. PollingOrchestratorから設定値を注入

**修正後のコード**:
```rust
impl SnapshotService {
    pub fn new(
        pool: MySqlPool,
        go2rtc_base_url: String,
        rtsp_manager: Arc<RtspManager>,
        timeout_main_sec: u64,   // 追加
        timeout_sub_sec: u64,    // 追加
    ) -> Self {
        Self {
            pool,
            go2rtc_base_url,
            rtsp_manager,
            ffmpeg_timeout_main: timeout_main_sec,
            ffmpeg_timeout_sub: timeout_sub_sec,
        }
    }
}
```

### 1.4 PollingOrchestrator修正

#### 1.4.1 設定読み込みロジック追加

**ファイル**: `src/polling_orchestrator/mod.rs`

**修正箇所**: `spawn_subnet_loop`関数内（SnapshotService初期化部分）

**Before** (推定 line 200付近):
```rust
let snapshot_service = Arc::new(SnapshotService::new(
    pool.clone(),
    go2rtc_url.clone(),
    rtsp_manager.clone(),
));
```

**After**:
```rust
// Load global timeout settings from database
let timeout_settings = load_global_timeout_settings(&pool).await;

let snapshot_service = Arc::new(SnapshotService::new(
    pool.clone(),
    go2rtc_url.clone(),
    rtsp_manager.clone(),
    timeout_settings.timeout_main_sec,
    timeout_settings.timeout_sub_sec,
));
```

**補助関数の追加**:
```rust
#[derive(Debug, Clone)]
struct GlobalTimeoutSettings {
    timeout_main_sec: u64,
    timeout_sub_sec: u64,
}

async fn load_global_timeout_settings(pool: &MySqlPool) -> GlobalTimeoutSettings {
    let result = sqlx::query(
        "SELECT setting_json FROM settings WHERE setting_key = 'polling'"
    )
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(row)) => {
            let setting_json: String = row.get("setting_json");
            if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&setting_json) {
                let timeout_main = settings["timeout_main_sec"]
                    .as_u64()
                    .unwrap_or(10);
                let timeout_sub = settings["timeout_sub_sec"]
                    .as_u64()
                    .unwrap_or(20);

                tracing::info!(
                    timeout_main_sec = timeout_main,
                    timeout_sub_sec = timeout_sub,
                    "Loaded global timeout settings"
                );

                return GlobalTimeoutSettings {
                    timeout_main_sec: timeout_main,
                    timeout_sub_sec: timeout_sub,
                };
            }
        }
        _ => {}
    }

    // Fallback to defaults
    tracing::warn!("Failed to load timeout settings, using defaults (10s/20s)");
    GlobalTimeoutSettings {
        timeout_main_sec: 10,
        timeout_sub_sec: 20,
    }
}
```

**呼び出しタイミング**:
- 各サブネットループ起動時（1回のみ）
- ConfigStoreリフレッシュ時は既存機構を活用（再起動不要）

### 1.5 Frontend実装

#### 1.5.1 SettingsModal.tsx 修正

**ファイル**: `frontend/src/components/SettingsModal.tsx`

**修正箇所**: 「表示」タブ（TabsContent value="display"）内

**追加するUI要素**:
```tsx
{/* 既存のサジェストビュー設定の後に追加 */}

{/* Global Timeout Settings */}
<Card>
  <CardHeader className="pb-3">
    <CardTitle className="text-sm font-medium flex items-center gap-2">
      <Clock className="h-4 w-4" />
      グローバルタイムアウト設定
    </CardTitle>
  </CardHeader>
  <CardContent className="space-y-4">
    <div className="space-y-2">
      <Label htmlFor="timeout-main">メインストリームタイムアウト（秒）</Label>
      <div className="flex items-center gap-4">
        <Input
          id="timeout-main"
          type="number"
          min={5}
          max={120}
          step={1}
          value={timeoutMainSec}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
            const value = parseInt(e.target.value, 10)
            if (!isNaN(value) && value >= 5 && value <= 120) {
              setTimeoutMainSec(value)
            }
          }}
          className="w-32"
        />
        <span className="text-sm text-muted-foreground">
          現在: {timeoutMainSec}秒
        </span>
      </div>
    </div>

    <div className="space-y-2">
      <Label htmlFor="timeout-sub">サブストリームタイムアウト（秒）</Label>
      <div className="flex items-center gap-4">
        <Input
          id="timeout-sub"
          type="number"
          min={5}
          max={120}
          step={1}
          value={timeoutSubSec}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
            const value = parseInt(e.target.value, 10)
            if (!isNaN(value) && value >= 5 && value <= 120) {
              setTimeoutSubSec(value)
            }
          }}
          className="w-32"
        />
        <span className="text-sm text-muted-foreground">
          現在: {timeoutSubSec}秒
        </span>
      </div>
    </div>

    <p className="text-xs text-muted-foreground">
      カメラからの応答を待つ最大時間。タイムアウトを長くすると遅いカメラも接続できますが、巡回時間が延びます。
    </p>

    <Button onClick={handleSaveTimeouts} disabled={savingTimeouts}>
      {savingTimeouts ? (
        <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
      ) : null}
      タイムアウト設定を保存
    </Button>
  </CardContent>
</Card>
```

**状態管理の追加**:
```tsx
const [timeoutMainSec, setTimeoutMainSec] = useState(10)
const [timeoutSubSec, setTimeoutSubSec] = useState(20)
const [savingTimeouts, setSavingTimeouts] = useState(false)
```

**データ取得関数**:
```tsx
const fetchTimeoutSettings = useCallback(async () => {
  try {
    const resp = await fetch(`${API_BASE_URL}/api/settings/timeouts`)
    if (resp.ok) {
      const json = await resp.json()
      const data = json.data || json
      setTimeoutMainSec(data.timeout_main_sec || 10)
      setTimeoutSubSec(data.timeout_sub_sec || 20)
    }
  } catch (error) {
    console.error("Failed to fetch timeout settings:", error)
  }
}, [])
```

**保存関数**:
```tsx
const handleSaveTimeouts = async () => {
  setSavingTimeouts(true)
  try {
    const resp = await fetch(`${API_BASE_URL}/api/settings/timeouts`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        timeout_main_sec: timeoutMainSec,
        timeout_sub_sec: timeoutSubSec,
      }),
    })
    if (resp.ok) {
      // 成功通知（オプション）
      console.log("Timeout settings saved")
    } else {
      const json = await resp.json()
      console.error("Failed to save:", json.error)
    }
  } catch (error) {
    console.error("Failed to save timeout settings:", error)
  } finally {
    setSavingTimeouts(false)
  }
}
```

**useEffect修正** (activeTab='display'時にデータ取得):
```tsx
useEffect(() => {
  if (!open) return

  const fetchData = async () => {
    setLoading(true)
    try {
      switch (activeTab) {
        case "display":
          await fetchTimeoutSettings()  // 追加
          break
        // ... 他のタブ
      }
    } finally {
      setLoading(false)
    }
  }

  fetchData()
}, [open, activeTab, fetchTimeoutSettings, /* ... */])
```

---

## Phase 2: カメラ別カスタムタイムアウト設定実装

### 2.1 データベース設計

#### 2.1.1 マイグレーションファイル作成

**ファイル名**: `015_camera_custom_timeouts.sql`
**配置場所**: `/opt/is22/migrations/015_camera_custom_timeouts.sql`

**内容**:
```sql
-- Add custom timeout settings to cameras table
ALTER TABLE cameras
ADD COLUMN custom_timeout_enabled TINYINT(1) NOT NULL DEFAULT 0 COMMENT 'カスタムタイムアウトを使用するか',
ADD COLUMN timeout_main_sec INT NULL COMMENT 'メインストリームのタイムアウト（秒）',
ADD COLUMN timeout_sub_sec INT NULL COMMENT 'サブストリームのタイムアウト（秒）';

-- Add index for custom_timeout_enabled for faster filtering
CREATE INDEX idx_cameras_custom_timeout ON cameras(custom_timeout_enabled);
```

**検証SQL**:
```sql
-- マイグレーション後の確認
SELECT
  camera_id,
  name,
  custom_timeout_enabled,
  timeout_main_sec,
  timeout_sub_sec
FROM cameras
LIMIT 5;
```

**ロールバックSQL** (緊急時用):
```sql
ALTER TABLE cameras
DROP COLUMN custom_timeout_enabled,
DROP COLUMN timeout_main_sec,
DROP COLUMN timeout_sub_sec;

DROP INDEX idx_cameras_custom_timeout ON cameras;
```

#### 2.1.2 MECE検証

**カメラのタイムアウト状態空間**:
```
全カメラ
├─ custom_timeout_enabled = 1
│  ├─ timeout_main_sec IS NOT NULL かつ timeout_sub_sec IS NOT NULL
│  │  └─ カスタム値を使用（正常系）
│  └─ timeout_main_sec IS NULL または timeout_sub_sec IS NULL
│     └─ グローバル値にフォールバック（異常系、データ不整合）
└─ custom_timeout_enabled = 0
   └─ グローバル値を使用（正常系、デフォルト）
```

**MECE確認**:
- ✅ 漏れなし: すべてのカメラは上記いずれかに分類される
- ✅ ダブりなし: 各カメラの分類は一意に定まる

### 2.2 Backend修正

#### 2.2.1 Camera構造体の拡張

**ファイル**: `src/models.rs` または `src/config_store/types.rs`

**Before**:
```rust
pub struct Camera {
    pub camera_id: String,
    pub name: String,
    // ... 既存フィールド
}
```

**After**:
```rust
pub struct Camera {
    pub camera_id: String,
    pub name: String,
    // ... 既存フィールド

    // Custom timeout settings
    pub custom_timeout_enabled: bool,
    pub timeout_main_sec: Option<u64>,
    pub timeout_sub_sec: Option<u64>,
}
```

#### 2.2.2 ConfigStore修正

**ファイル**: `src/config_store/repository.rs`

**修正箇所**: `load_cameras_from_db`関数のSELECTクエリ

**Before**:
```rust
sqlx::query_as::<_, Camera>(
    "SELECT camera_id, name, location, ... FROM cameras WHERE deleted_at IS NULL"
)
```

**After**:
```rust
sqlx::query_as::<_, Camera>(
    "SELECT
        camera_id,
        name,
        location,
        ...,
        custom_timeout_enabled,
        timeout_main_sec,
        timeout_sub_sec
     FROM cameras
     WHERE deleted_at IS NULL"
)
```

#### 2.2.3 SnapshotService修正（タイムアウト適用ロジック）

**ファイル**: `src/snapshot_service/mod.rs`

**修正箇所**: `capture`メソッド

**Before**:
```rust
pub async fn capture(
    &self,
    camera: &Camera,
) -> Result<Vec<u8>, CaptureError> {
    // 常にself.ffmpeg_timeout_main, self.ffmpeg_timeout_subを使用
    // ...
}
```

**After**:
```rust
pub async fn capture(
    &self,
    camera: &Camera,
) -> Result<Vec<u8>, CaptureError> {
    // カメラ個別設定を優先、なければグローバル設定を使用
    let timeout_main = if camera.custom_timeout_enabled {
        camera.timeout_main_sec.unwrap_or(self.ffmpeg_timeout_main)
    } else {
        self.ffmpeg_timeout_main
    };

    let timeout_sub = if camera.custom_timeout_enabled {
        camera.timeout_sub_sec.unwrap_or(self.ffmpeg_timeout_sub)
    } else {
        self.ffmpeg_timeout_sub
    };

    tracing::debug!(
        camera_id = %camera.camera_id,
        custom_enabled = camera.custom_timeout_enabled,
        timeout_main_sec = timeout_main,
        timeout_sub_sec = timeout_sub,
        "Applying timeout settings"
    );

    // 以降のffmpegキャプチャでtimeout_main/timeout_subを使用
    // ...
}
```

**注意**: `capture_rtsp`メソッドの呼び出し時に動的なタイムアウト値を渡せるように修正が必要

**capture_rtspの修正**:
```rust
// Before
async fn capture_rtsp(&self, rtsp_url: &str, timeout_sec: u64) -> Result<Vec<u8>, CaptureError>

// After（変更なし、既に引数でタイムアウトを受け取っている）
```

**captureメソッド内の呼び出し修正**:
```rust
// Before (line 150付近)
match self.capture_rtsp(main_url, self.ffmpeg_timeout_main).await {

// After
match self.capture_rtsp(main_url, timeout_main).await {
```

```rust
// Before (line 177付近)
match self.capture_rtsp(sub_url, self.ffmpeg_timeout_sub).await {

// After
match self.capture_rtsp(sub_url, timeout_sub).await {
```

#### 2.2.4 Camera更新API拡張

**ファイル**: `src/web_api/routes.rs`

**対象API**: `PUT /api/cameras/{camera_id}` (既存のカメラ更新API)

**修正箇所**: `UpdateCameraRequest`構造体

**Before**:
```rust
#[derive(Deserialize)]
pub struct UpdateCameraRequest {
    pub name: Option<String>,
    pub location: Option<String>,
    // ... 既存フィールド
}
```

**After**:
```rust
#[derive(Deserialize)]
pub struct UpdateCameraRequest {
    pub name: Option<String>,
    pub location: Option<String>,
    // ... 既存フィールド

    // Custom timeout settings
    pub custom_timeout_enabled: Option<bool>,
    pub timeout_main_sec: Option<u64>,
    pub timeout_sub_sec: Option<u64>,
}
```

**更新クエリの修正**:
```rust
// 既存のUPDATEクエリに追加
"UPDATE cameras SET
    name = COALESCE(?, name),
    location = COALESCE(?, location),
    ...,
    custom_timeout_enabled = COALESCE(?, custom_timeout_enabled),
    timeout_main_sec = ?,
    timeout_sub_sec = ?
 WHERE camera_id = ?"
```

**バリデーション追加**:
```rust
if let Some(enabled) = &payload.custom_timeout_enabled {
    if *enabled {
        // カスタムタイムアウトが有効な場合、値の検証
        if let Some(main_sec) = payload.timeout_main_sec {
            if main_sec < 5 || main_sec > 120 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ok": false, "error": "timeout_main_sec must be between 5 and 120"}))
                ));
            }
        }
        if let Some(sub_sec) = payload.timeout_sub_sec {
            if sub_sec < 5 || sub_sec > 120 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ok": false, "error": "timeout_sub_sec must be between 5 and 120"}))
                ));
            }
        }
    }
}
```

### 2.3 Frontend実装

#### 2.3.1 types/api.ts 修正

**ファイル**: `frontend/src/types/api.ts`

**Camera型の拡張**:
```typescript
export interface Camera {
  camera_id: string
  name: string
  // ... 既存フィールド

  // Custom timeout settings
  custom_timeout_enabled: boolean
  timeout_main_sec: number | null
  timeout_sub_sec: number | null
}
```

#### 2.3.2 CameraDetailModal.tsx 修正

**ファイル**: `frontend/src/components/CameraDetailModal.tsx`

**追加するUI要素** (既存の設定セクションの後に追加):
```tsx
{/* Custom Timeout Settings */}
<div className="space-y-4">
  <div className="flex items-center space-x-2">
    <input
      type="checkbox"
      id="custom-timeout"
      checked={customTimeoutEnabled}
      onChange={(e) => setCustomTimeoutEnabled(e.target.checked)}
      className="rounded border-gray-300"
    />
    <Label htmlFor="custom-timeout">
      カスタムタイムアウトを使用
    </Label>
  </div>

  {customTimeoutEnabled && (
    <div className="space-y-4 ml-6">
      <div className="space-y-2">
        <Label htmlFor="timeout-main-custom">
          メインストリームタイムアウト（秒）
        </Label>
        <Input
          id="timeout-main-custom"
          type="number"
          min={5}
          max={120}
          value={timeoutMainSec ?? ''}
          onChange={(e) => {
            const value = parseInt(e.target.value, 10)
            if (!isNaN(value)) {
              setTimeoutMainSec(value)
            }
          }}
          className="w-32"
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="timeout-sub-custom">
          サブストリームタイムアウト（秒）
        </Label>
        <Input
          id="timeout-sub-custom"
          type="number"
          min={5}
          max={120}
          value={timeoutSubSec ?? ''}
          onChange={(e) => {
            const value = parseInt(e.target.value, 10)
            if (!isNaN(value)) {
              setTimeoutSubSec(value)
            }
          }}
          className="w-32"
        />
      </div>

      <p className="text-xs text-muted-foreground">
        このカメラ専用のタイムアウト値。未設定の場合はグローバル設定が使用されます。
      </p>
    </div>
  )}

  {!customTimeoutEnabled && (
    <p className="text-xs text-muted-foreground ml-6">
      グローバルタイムアウト設定を使用します（設定モーダルで変更可能）
    </p>
  )}
</div>
```

**状態管理の追加**:
```tsx
const [customTimeoutEnabled, setCustomTimeoutEnabled] = useState(false)
const [timeoutMainSec, setTimeoutMainSec] = useState<number | null>(null)
const [timeoutSubSec, setTimeoutSubSec] = useState<number | null>(null)
```

**データ取得時の初期化** (useEffect内):
```tsx
useEffect(() => {
  if (camera) {
    // 既存の初期化に追加
    setCustomTimeoutEnabled(camera.custom_timeout_enabled)
    setTimeoutMainSec(camera.timeout_main_sec)
    setTimeoutSubSec(camera.timeout_sub_sec)
  }
}, [camera])
```

**保存時のペイロード拡張** (handleSave関数内):
```tsx
const payload = {
  name,
  location,
  // ... 既存フィールド
  custom_timeout_enabled: customTimeoutEnabled,
  timeout_main_sec: customTimeoutEnabled ? timeoutMainSec : null,
  timeout_sub_sec: customTimeoutEnabled ? timeoutSubSec : null,
}
```

---

## Phase 3: 統合テスト計画

### 3.1 テストシナリオ一覧

#### Test Case 1: グローバル設定の動作確認

**前提条件**:
- すべてのカメラが`custom_timeout_enabled=0`

**手順**:
1. SettingsModalを開く
2. グローバルタイムアウトを15秒/30秒に変更
3. 保存ボタンをクリック
4. 次の巡回サイクルを待つ
5. ログで`timeout_main_sec=15, timeout_sub_sec=30`を確認

**期待結果**:
- ✅ 設定が保存される
- ✅ 次回の巡回で新しいタイムアウトが適用される
- ✅ すべてのカメラが新しいタイムアウトで動作

#### Test Case 2: カメラ個別設定の優先

**前提条件**:
- グローバル設定: 10秒/20秒
- カメラA: `custom_timeout_enabled=1`, `timeout_main_sec=30`, `timeout_sub_sec=60`
- カメラB: `custom_timeout_enabled=0`

**手順**:
1. 巡回サイクル実行
2. カメラA, Bのログを確認

**期待結果**:
- ✅ カメラA: `timeout_main_sec=30, timeout_sub_sec=60`を使用
- ✅ カメラB: `timeout_main_sec=10, timeout_sub_sec=20`を使用

#### Test Case 3: フォールバック動作

**前提条件**:
- カメラC: `custom_timeout_enabled=1`, `timeout_main_sec=NULL`, `timeout_sub_sec=NULL`

**手順**:
1. 巡回サイクル実行
2. カメラCのログを確認

**期待結果**:
- ✅ グローバル設定にフォールバック
- ✅ ログに警告が出力される（データ不整合）

#### Test Case 4: 実カメラでの検証（.62カメラ）

**前提条件**:
- .62カメラ（192.168.125.62）: 現在タイムアウトで失敗している
- 現在の設定: 10秒/20秒

**手順**:
1. .62カメラにカスタムタイムアウトを設定: 30秒/40秒
2. 5サイクル実行
3. 成功率を確認

**期待結果**:
- ✅ タイムアウト延長前: 0-50%成功率
- ✅ タイムアウト延長後: 80-100%成功率

### 3.2 パフォーマンステスト

#### Test Case 5: 巡回時間への影響測定

**前提条件**:
- 125サブネット: 8台のカメラ
- テストケース:
  - A: すべて10秒/20秒（現状）
  - B: すべて30秒/60秒（最大値）

**測定項目**:
- 巡回完了時間
- CPU使用率
- メモリ使用量

**期待結果**:
- ✅ ケースAとBで巡回時間が適切に増加（線形）
- ✅ CPU/メモリ使用量に大きな変化なし

### 3.3 UI/UXテスト

#### Test Case 6: SettingsModal入力検証

**手順**:
1. タイムアウトに4秒を入力 → エラー表示
2. タイムアウトに121秒を入力 → エラー表示
3. タイムアウトに文字列を入力 → 入力不可
4. タイムアウトに50秒を入力 → 保存成功

**期待結果**:
- ✅ 範囲外の値は拒否される
- ✅ 適切なエラーメッセージが表示される

#### Test Case 7: CameraDetailModal連携

**手順**:
1. カメラ詳細モーダルを開く
2. 「カスタムタイムアウトを使用」をチェック → 入力欄が有効化
3. 値を入力して保存
4. モーダルを閉じて再度開く → 値が保持されている
5. チェックを外して保存 → 値がクリアされる

**期待結果**:
- ✅ チェックボックスで入力欄の有効/無効が切り替わる
- ✅ 保存した値が正しく保持される

---

## 4. 実装チェックリスト（The_golden_rules.md準拠）

### 4.1 実装前チェック

- [ ] **SSoT**: データソースは一意か？
  - settings.polling: グローバル設定
  - cameras: カメラ個別設定
  - 適用ロジック: SnapshotService::captureのみ

- [ ] **SOLID**: 責務は明確か？
  - SnapshotService: タイムアウト適用
  - ConfigStore: カメラ情報管理
  - API: CRUD操作のみ

- [ ] **MECE**: 漏れ・ダブりはないか？
  - 決定木を確認済み

- [ ] **アンアンビギュアス**: 曖昧さはないか？
  - タイミング: 保存後、次回巡回から適用
  - 優先順位: カメラ個別 > グローバル > デフォルト

### 4.2 実装後チェック

- [ ] **チェックリスト**: すべてのテストケースがパスしたか？
- [ ] **現場猫案件**: ハードコードや握り潰しはないか？
- [ ] **根拠なき自己解釈**: すべての情報を平等に扱っているか？
- [ ] **優先順位**: 実装順序は適切だったか？

---

**文書ステータス**: 詳細設計完成
**次のアクション**: タスクリスト・テスト計画作成 → GitHub Issue登録
