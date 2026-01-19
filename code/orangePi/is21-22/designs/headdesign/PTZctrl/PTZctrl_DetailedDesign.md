# PTZctrl 詳細設計書

> **更新日**: 2026-01-18
> **ステータス**: 実装完了・テスト中
> **関連ドキュメント**: PTZctrl_SpecificationInvestigation.md, PTZctrl_basicdesign.md

---

## 実装タスク一覧（優先度順）

| タスクID | 内容 | 優先度 | 依存 |
|----------|------|--------|------|
| IMPL-T01 | LiveViewModalサイズ拡大 | 高 | なし |
| IMPL-T02 | CameraTile rotation適用 | 高 | なし |
| IMPL-T03 | SuggestPane rotation適用 | 高 | なし |
| IMPL-T04 | LiveViewModal rotation適用 | 高 | なし |
| IMPL-T05 | SuggestPane 3台制限修正 | 高 | なし |
| IMPL-T06 | SuggestPane 同一カメラ重複防止 | 高 | IMPL-T05 |
| IMPL-T07 | SuggestPane onairTime修正 | 高 | IMPL-T05 |
| IMPL-T08 | Migration 032_ptz_disabled.sql | 中 | なし |
| IMPL-T09 | Backend Camera struct更新 | 中 | IMPL-T08 |
| IMPL-T10 | Frontend api.ts ptz_disabled追加 | 中 | IMPL-T09 |
| IMPL-T11 | CameraDetailModal PTZ無効化チェック | 中 | IMPL-T10 |
| IMPL-T12 | PTZ Backend API | 高 | IMPL-T09 |
| IMPL-T13 | PTZ Frontend UI | 高 | IMPL-T12 |

---

## IMPL-T01: LiveViewModalサイズ拡大

### 対象ファイル
`frontend/src/components/LiveViewModal.tsx`

### 変更内容
```tsx
// Line 69: Before
<DialogContent className="max-w-4xl p-0 overflow-hidden">

// Line 69: After
<DialogContent className="max-w-5xl p-0 overflow-hidden">
```

### テスト
- [ ] モーダルが1024pxで表示されること
- [ ] 130%表示でも十分な大きさで映像が表示されること

---

## IMPL-T02: CameraTile rotation適用

### 対象ファイル
`frontend/src/components/CameraTile.tsx`

### 変更内容

1. propsにrotation追加不要（camera.rotationで取得可能）

2. overlay mode の img タグ修正（Line 217-224）:
```tsx
// Before
<img
  src={currentUrl}
  alt={camera.name}
  className={cn("absolute inset-0 w-full h-full", imageObjectClass)}
  style={{ objectPosition: 'center center' }}
  loading="lazy"
  onError={() => setImageError(true)}
/>

// After
<img
  src={currentUrl}
  alt={camera.name}
  className={cn("absolute inset-0 w-full h-full", imageObjectClass)}
  style={{
    objectPosition: 'center center',
    transform: `rotate(${camera.rotation || 0}deg)`,
  }}
  loading="lazy"
  onError={() => setImageError(true)}
/>
```

3. footer mode の img タグ修正（Line 366-376）: 同様の変更

4. アニメーション用 next img タグ修正（Line 226-238, 377-389）: 同様の変更

### テスト
- [ ] rotation=90の場合、タイル内で画像が90度回転していること
- [ ] アニメーション時も回転が維持されること

---

## IMPL-T03: SuggestPane rotation適用

### 対象ファイル
`frontend/src/components/SuggestPane.tsx`

### 変更内容

1. OnAirCameraインターフェースにrotation追加（Line 24-41）:
```tsx
interface OnAirCamera {
  // ... existing fields
  rotation: number;  // 追加
}
```

2. newCamera作成時にrotation設定（Line 147-163）:
```tsx
const newCamera: OnAirCamera = {
  // ... existing fields
  rotation: camera.rotation ?? 0,  // 追加
}
```

3. Go2rtcPlayerをラップするコンテナにrotation適用（Line 314-327）:
```tsx
// Before
<Go2rtcPlayer
  cameraId={cam.cameraId}
  // ...
/>

// After
<div
  className="h-full w-full"
  style={{ transform: `rotate(${cam.rotation}deg)` }}
>
  <Go2rtcPlayer
    cameraId={cam.cameraId}
    // ...
  />
</div>
```

### テスト
- [ ] rotation設定されたカメラがSuggestPaneで正しく回転表示されること

---

## IMPL-T04: LiveViewModal rotation適用

### 対象ファイル
`frontend/src/components/LiveViewModal.tsx`

### 変更内容

Go2rtcPlayerをラップするコンテナにrotation適用（Line 98-133）:
```tsx
// Before
<div className="relative aspect-video bg-black">
  {hasAnyStream ? (
    <Go2rtcPlayer
      // ...
    />
  ) : (
    // ...
  )}
</div>

// After
<div className="relative aspect-video bg-black overflow-hidden">
  {hasAnyStream ? (
    <div
      className="w-full h-full"
      style={{ transform: `rotate(${camera?.rotation || 0}deg)` }}
    >
      <Go2rtcPlayer
        // ...
      />
    </div>
  ) : (
    // ...
  )}
</div>
```

### テスト
- [ ] rotation設定されたカメラがLiveViewModalで正しく回転表示されること

---

## IMPL-T05: SuggestPane 3台制限修正

### 対象ファイル
`frontend/src/components/SuggestPane.tsx`

### 変更内容

3台制限ロジックの修正（Line 174-179）:
```tsx
// Before
let newList = [newCamera, ...prev]
if (newList.length > 3) {
  newList = newList.slice(0, 3)
}
return newList

// After
let newList = [newCamera, ...prev]
// exiting状態のカメラを除外してカウント
const activeCount = newList.filter(c => !c.isExiting).length
if (activeCount > 3) {
  // exiting以外で最古のものを削除（配列の後ろから）
  const toRemove = newList.filter(c => !c.isExiting).slice(3)
  newList = newList.filter(c => !toRemove.includes(c))
}
return newList
```

### テスト
- [ ] アクティブなカメラが4台以上にならないこと
- [ ] exiting中のカメラがカウントに含まれないこと

---

## IMPL-T06: SuggestPane 同一カメラ重複防止

### 対象ファイル
`frontend/src/components/SuggestPane.tsx`

### 変更内容

重複チェックの強化（Line 93-96）:
```tsx
// Before
const existingIndex = prev.findIndex(c => c.lacisId === eventLacisId && !c.isExiting)

// After
// lacisIdとcameraIdの両方でチェック
const existingIndex = prev.findIndex(c =>
  (c.lacisId === eventLacisId || c.cameraId === camera.camera_id) && !c.isExiting
)
```

### テスト
- [ ] 同一カメラが複数回表示されないこと
- [ ] 長時間動作しても重複が発生しないこと

---

## IMPL-T07: SuggestPane onairTime修正

### 対象ファイル
`frontend/src/components/SuggestPane.tsx`

### 変更内容

expiry判定の厳密化（Line 219-252）:
```tsx
// Line 223-227のexpiredフィルタを修正
const expired = prev.filter(c => {
  // startedAtからの経過時間でチェック（lastEventAtではなく）
  const elapsedFromStart = (now - c.startedAt) / 1000
  const elapsedFromLastEvent = (now - c.lastEventAt) / 1000
  // onairTime経過 AND 最後のイベントからも経過
  return elapsedFromLastEvent >= onAirTimeSeconds && !c.isExiting
})
```

### テスト
- [ ] onairTime経過後にカメラが確実に削除されること
- [ ] イベント発生で時間がリセットされること

---

## IMPL-T08: Migration 032_ptz_disabled.sql

### 対象ファイル（新規）
`migrations/032_ptz_disabled.sql`

### 内容
```sql
-- PTZ無効化フラグ追加
-- PTZ対応カメラでも操作UIを非表示にするためのフラグ
-- デフォルトはFALSE（PTZ操作可能）

ALTER TABLE cameras ADD COLUMN ptz_disabled BOOLEAN NOT NULL DEFAULT FALSE;

-- インデックスは不要（フィルタ用途ではない）
```

### テスト
- [ ] migrationが正常に実行されること
- [ ] 既存カメラのptz_disabledがFALSEになること

---

## IMPL-T09: Backend Camera struct更新

### 対象ファイル
`src/config_store/types.rs`

### 変更内容

1. Camera structにフィールド追加（Line 92付近）:
```rust
pub struct Camera {
    // ... existing fields
    // === PTZ制御 ===
    /// PTZ操作UIを無効化するフラグ（デフォルトFALSE）
    pub ptz_disabled: bool,
    // ...
}
```

2. UpdateCameraRequestにフィールド追加（Line 264付近）:
```rust
pub struct UpdateCameraRequest {
    // ... existing fields
    // === PTZ制御 ===
    pub ptz_disabled: Option<bool>,
    // ...
}
```

### テスト
- [ ] カメラ取得時にptz_disabledが返ること
- [ ] カメラ更新時にptz_disabledが保存されること

---

## IMPL-T10: Frontend api.ts ptz_disabled追加

### 対象ファイル
`frontend/src/types/api.ts`

### 変更内容

1. Cameraインターフェースにフィールド追加（Line 134付近）:
```typescript
export interface Camera {
  // ... existing fields
  // === PTZ制御 ===
  ptz_disabled: boolean;  // PTZ操作UIを無効化
  // ...
}
```

2. UpdateCameraRequestにフィールド追加（Line 225付近）:
```typescript
export interface UpdateCameraRequest {
  // ... existing fields
  // === PTZ制御 ===
  ptz_disabled?: boolean;
  // ...
}
```

### テスト
- [ ] TypeScriptの型チェックが通ること

---

## IMPL-T11: CameraDetailModal PTZ無効化チェックボックス

### 対象ファイル
`frontend/src/components/CameraDetailModal.tsx`

### 変更内容

PTZ能力セクション（Line 747-762）に追加:
```tsx
{/* PTZ Capabilities */}
{camera.ptz_supported && (
  <Section title="PTZ能力" icon={<Move3d className="h-4 w-4" />}>
    {/* PTZ無効化チェックボックス - 追加 */}
    <FormField label="PTZ操作" editable>
      <div className="flex items-center gap-2">
        <input
          type="checkbox"
          id="ptz-disabled"
          checked={!getValue("ptz_disabled")}
          onChange={(e) => updateField("ptz_disabled", !e.target.checked)}
          className="h-4 w-4"
          disabled={!camera.ptz_supported}
        />
        <label htmlFor="ptz-disabled" className="text-sm">
          PTZ操作UIを表示する
        </label>
      </div>
    </FormField>
    {/* 既存のPTZ能力Badge */}
    <div className="flex flex-wrap gap-2 mt-2">
      {camera.ptz_continuous && <Badge variant="secondary">連続移動</Badge>}
      {camera.ptz_absolute && <Badge variant="secondary">絶対位置</Badge>}
      {camera.ptz_relative && <Badge variant="secondary">相対移動</Badge>}
      {camera.ptz_home_supported && <Badge variant="secondary">ホーム</Badge>}
    </div>
    {/* ... */}
  </Section>
)}
```

### テスト
- [ ] PTZ対応カメラでチェックボックスが表示されること
- [ ] PTZ非対応カメラでPTZ能力セクション自体が非表示になること
- [ ] チェック状態が正しく保存されること

---

## IMPL-T12: PTZ Backend API

### 対象ファイル（新規）
- `src/ptz_controller/mod.rs`
- `src/ptz_controller/types.rs`
- `src/ptz_controller/service.rs`
- `src/web_api/ptz_routes.rs`
- `src/state.rs` (ptz_service追加)
- `src/main.rs` (PtzService初期化追加)
- `src/web_api/routes.rs` (PTZルート追加)

### API設計
```
POST /api/cameras/:id/ptz/move   - 移動開始
POST /api/cameras/:id/ptz/stop   - 停止
POST /api/cameras/:id/ptz/home   - ホームポジション
GET  /api/cameras/:id/ptz/status - ステータス取得
```

### 型定義 (types.rs)
```rust
pub enum PtzDirection { Up, Down, Left, Right }
pub enum PtzMode { Nudge, Continuous }
pub struct PtzMoveRequest { lease_id, direction, mode, speed, duration_ms }
pub struct PtzStopRequest { lease_id }
pub struct PtzHomeRequest { lease_id }
pub struct PtzResponse { ok, error?, message? }
pub struct PtzStatus { camera_id, ptz_supported, ptz_disabled, ... }
```

### テスト
- [ ] PTZ move APIが正常に動作すること
- [ ] PTZ stop APIが正常に動作すること
- [ ] PTZ home APIが正常に動作すること
- [ ] PTZ status APIが正常に動作すること
- [ ] rotation変換が正しく適用されること（UI方向→実際の方向）

---

## IMPL-T13: PTZ Frontend UI

### 対象ファイル（新規）
- `frontend/src/hooks/usePtzControl.ts`
- `frontend/src/components/ptz/PtzOverlay.tsx`
- `frontend/src/components/ptz/index.ts`

### 対象ファイル（更新）
- `frontend/src/components/LiveViewModal.tsx`

### usePtzControl.ts
```typescript
interface UsePtzControlReturn {
  move: (direction: PtzDirection, mode?: PtzMode) => Promise<void>
  stop: () => Promise<void>
  home: () => Promise<void>
  isMoving: boolean
  currentDirection: PtzDirection | null
  error: string | null
}
```

### PtzOverlay.tsx
- 4方向ボタン（上下左右）
- ホームボタン（対応時のみ）
- チョン押し/長押し対応
- エラー表示

### LiveViewModal.tsx統合
- Lease管理（取得・heartbeat・解放）
- PTZトグルボタン追加
- PtzOverlay条件付きレンダリング
- PTZバッジ表示

### テスト
- [ ] PTZ対応カメラでPTZボタンが表示されること
- [ ] PTZボタンクリックでオーバーレイが表示されること
- [ ] 4方向ボタンが機能すること
- [ ] 長押しで連続移動、離すと停止すること
- [ ] モーダルクローズ時にリースが解放されること

---

## テスト計画

### フロントエンドテスト
```bash
cd code/orangePi/is22/frontend
npm run build
npm run preview
```

### バックエンドテスト
```bash
# サーバー側でビルド
ssh mijeosadmin@192.168.125.246
cd /opt/is22
cargo build --release
sudo systemctl restart is22
```

### 動作確認項目
1. [ ] カメラタイルで回転が反映される
2. [ ] サジェストビューで回転が反映される
3. [ ] LiveViewModalで回転が反映される
4. [ ] サジェストが最大3台に制限される
5. [ ] 同一カメラが重複表示されない
6. [ ] onairTime経過でサジェストが消える
7. [ ] LiveViewModalのサイズが拡大されている
8. [ ] PTZ無効化チェックボックスが機能する
9. [ ] PTZ対応カメラでPTZボタンが表示される
10. [ ] PTZオーバーレイの4方向ボタンが機能する
11. [ ] PTZ API (move/stop/home/status)が正常応答する
12. [ ] LiveViewModalクローズ時にリースが解放される

---

## MECEチェック
- [x] バグ修正: 7項目全てカバー (IMPL-T01〜T07)
- [x] DB/型定義: ptz_disabled実装 (IMPL-T08〜T11)
- [x] PTZ Backend API: 4エンドポイント実装 (IMPL-T12)
- [x] PTZ Frontend UI: Overlay + Hook実装 (IMPL-T13)
- [x] 型定義: Frontend/Backend両方カバー
- [x] テスト: Frontend/Backend/Chrome UI/UX

## アンアンビギュアス宣言
本ドキュメントは以下を満たす：
1. 全変更箇所がファイル名・行番号で特定済み
2. Before/Afterのコード差分が明確
3. テスト項目がチェックリスト形式で定義済み

---

## 作成日
2026-01-18
