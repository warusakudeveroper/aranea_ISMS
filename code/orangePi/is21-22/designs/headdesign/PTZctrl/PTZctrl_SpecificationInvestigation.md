# PTZctrl 仕様調査ドキュメント

## 概要
PTZ対応カメラのクリックモーダル操作機能の実装と、既知のバグ修正のための仕様調査結果。

## The_golden_rules.md 準拠確認
- [x] SSoT原則確認済み
- [x] SOLID原則を意識
- [x] MECE性確保
- [x] 車輪の再発明回避（既存実装の確認完了）

---

## 1. 現状調査結果

### 1.1 LiveViewModal（frontend/src/components/LiveViewModal.tsx）

| 項目 | 現状値 | 問題点 | 修正案 |
|------|--------|--------|--------|
| DialogContent幅 | `max-w-4xl` (896px) | 130%表示でも画像が小さく感じる | `max-w-5xl` (1024px) or `max-w-6xl` (1152px) に拡大 |
| rotation適用 | なし | CameraDetailModalのみで適用 | transform: rotate() を追加 |
| PTZ UI | なし | 新規実装必要 | PtzOverlay コンポーネント追加 |

**現在のコード構造：**
```tsx
// Line 69: max-w-4xl が問題
<DialogContent className="max-w-4xl p-0 overflow-hidden">
```

### 1.2 CameraTile（frontend/src/components/CameraTile.tsx）

| 項目 | 現状値 | 問題点 |
|------|--------|--------|
| rotation | props受け取りなし | camera.rotationを参照していない |
| img要素 | object-cover適用済み | rotationのtransform適用なし |

**修正必要箇所：**
- Line 220, 370: `<img>` タグに `style={{ transform: \`rotate(${camera.rotation}deg)\` }}` 追加必要

### 1.3 SuggestPane（frontend/src/components/SuggestPane.tsx）

| 項目 | 現状値（設計） | 問題点（実際の動作） |
|------|----------------|---------------------|
| 最大表示数 | 3台 | **4台以上表示されることがある** |
| 同一カメラ重複 | 禁止 | **長時間動作で重複表示される** |
| onairTime制限 | lastEventAtベースでTTL管理 | **制限が守られないことがある** |
| rotation適用 | なし | Go2rtcPlayerにtransform未適用 |

**バグ箇所特定：**

```tsx
// Line 174-179: 3台制限のロジック
if (newList.length > 3) {
  newList = newList.slice(0, 3)
}
```
問題：AnimatePresenceの `mode="popLayout"` と組み合わせた場合、exiting状態のカメラがカウントに含まれることで一時的に4台表示される可能性。

```tsx
// Line 95: 重複チェック
const existingIndex = prev.findIndex(c => c.lacisId === eventLacisId && !c.isExiting)
```
問題：`lacisId` ベースの比較だが、同一カメラが異なる `id` で登録される可能性。

### 1.4 CameraDetailModal（frontend/src/components/CameraDetailModal.tsx）

| 項目 | 現状値 | 状態 |
|------|--------|------|
| rotation設定UI | Line 294-303 | 実装済み（0/90/180/270度選択可能） |
| rotation適用（プレビュー） | Line 278 | `transform: rotate(${currentRotation}deg)` 適用済み |
| PTZ能力表示 | Line 747-762 | Badge表示のみ |
| PTZ無効化チェックボックス | なし | 新規追加必要 |

### 1.5 型定義（frontend/src/types/api.ts）

| フィールド | 存在 | 備考 |
|-----------|------|------|
| ptz_supported | ○ | Line 126 |
| ptz_continuous | ○ | Line 127 |
| ptz_absolute | ○ | Line 128 |
| ptz_relative | ○ | Line 129 |
| ptz_home_supported | ○ | Line 134 |
| **ptz_disabled** | × | **新規追加必要** |

### 1.6 バックエンド Camera struct（src/config_store/types.rs）

| フィールド | 存在 | 備考 |
|-----------|------|------|
| ptz_supported | ○ | Line 84 |
| ptz_continuous | ○ | Line 85 |
| ptz_absolute | ○ | Line 86 |
| ptz_relative | ○ | Line 87 |
| ptz_home_supported | ○ | Line 92 |
| **ptz_disabled** | × | **新規追加必要** |

### 1.7 AccessAbsorber（src/access_absorber/）

| 機能 | 実装状況 | PTZ連携 |
|------|----------|---------|
| StreamPurpose優先度 | ○ types.rs Line 160-211 | ClickModal=1, SuggestPlay=4 |
| プリエンプション | ○ can_preempt() | ClickModalが最優先 |
| AbsorberError | ○ types.rs Line 301-334 | ユーザーメッセージ生成可能 |
| **競合フィードバック** | △ | to_user_message()あるがUI表示なし |

**現在の優先度：**
```rust
// types.rs Line 173-180
pub fn priority(&self) -> u8 {
    match self {
        Self::ClickModal => 1,    // 最優先
        Self::Snapshot => 2,
        Self::Polling => 3,
        Self::SuggestPlay => 4,
        Self::HealthCheck => 5,
    }
}
```

### 1.8 AdmissionController / Modal Lease（src/admission_controller/）

| 機能 | 実装状況 | PTZ利用可否 |
|------|----------|------------|
| LeaseManager | ○ lease.rs | PTZ認証に利用可能 |
| TTL | 300秒 | AdmissionPolicy.modal_ttl_sec |
| Heartbeat | 30秒間隔 | AdmissionPolicy.heartbeat_interval_sec |
| Grace期間 | 45秒 | AdmissionPolicy.heartbeat_grace_sec |

---

## 2. 既知のバグ一覧

### Bug-01: LiveViewModalサイズが小さい
- **現象**: 130%表示でも画像が小さく感じる
- **原因**: max-w-4xl (896px)
- **修正**: max-w-5xl (1024px) に変更
- **ファイル**: frontend/src/components/LiveViewModal.tsx:69

### Bug-02: rotation未適用（CameraTile）
- **現象**: CameraDetailModalで回転設定しても、タイル表示に反映されない
- **原因**: CameraTileのimg要素にtransform未適用
- **修正**: transform: rotate(rotation deg) 追加
- **ファイル**: frontend/src/components/CameraTile.tsx:220,370

### Bug-03: rotation未適用（SuggestPane）
- **現象**: SuggestPaneの映像に回転が反映されない
- **原因**: Go2rtcPlayerにrotation props未渡し
- **修正**: Go2rtcPlayerまたは親コンテナにtransform適用
- **ファイル**: frontend/src/components/SuggestPane.tsx:314

### Bug-04: SuggestPane 3台制限違反
- **現象**: 4台以上表示されることがある
- **原因**: exiting状態のカメラがカウントに含まれる
- **修正**: exiting除外後のカウントでslice
- **ファイル**: frontend/src/components/SuggestPane.tsx:174-179

### Bug-05: SuggestPane同一カメラ重複
- **現象**: 同じカメラが複数回表示される
- **原因**: lacisIdチェックのタイミング問題
- **修正**: cameraIdベースの重複排除強化
- **ファイル**: frontend/src/components/SuggestPane.tsx:95

### Bug-06: onairTime制限違反
- **現象**: 設定されたonairTimeが守られないことがある
- **原因**: lastEventAtの更新ロジック、expiry判定
- **修正**: expiry判定の厳密化
- **ファイル**: frontend/src/components/SuggestPane.tsx:219-252

### Bug-07: AccessAbsorber競合フィードバックなし
- **現象**: クリックモーダルとSuggestが競合時、理由が表示されない
- **原因**: AbsorberErrorのto_user_message()がUIで未使用
- **修正**: フロントエンドでエラーメッセージ表示追加
- **ファイル**: Go2rtcPlayer.tsx / LiveViewModal.tsx

---

## 3. PTZ実装要件

### 3.1 新規DBフィールド
```sql
-- Migration 032_ptz_disabled.sql
ALTER TABLE cameras ADD COLUMN ptz_disabled BOOLEAN NOT NULL DEFAULT FALSE;
```

### 3.2 新規APIエンドポイント
```
POST /api/cameras/:id/ptz/move   - 移動開始
POST /api/cameras/:id/ptz/stop   - 停止
POST /api/cameras/:id/ptz/home   - ホームポジション
```

### 3.3 フロントエンド新規コンポーネント
```
frontend/src/components/ptz/
├── PtzOverlay.tsx      # PTZ操作UIオーバーレイ
├── PtzStick4Way.tsx    # 4方向ジョイスティック
└── PtzDpad.tsx         # D-padボタン（PC用）
frontend/src/hooks/
└── usePtzControl.ts    # PTZ操作フック
```

---

## 4. 依存関係図

```
[PTZ UI] ─depends→ [Modal Lease] ─uses→ [LeaseManager]
                         │
                         ↓
              [AccessAbsorber] ─manages→ [StreamToken]
                         │
                         ↓
              [PTZ Controller] ─calls→ [ONVIF PTZ API]
```

---

## 5. MECEチェック

### 修正対象の網羅性
- [x] LiveViewModal: サイズ拡大、PTZ UI追加
- [x] CameraTile: rotation適用
- [x] SuggestPane: rotation適用、3台制限、重複防止、onairTime
- [x] CameraDetailModal: PTZ無効化チェックボックス
- [x] Backend: ptz_disabled、PTZ API

### 機能の重複排除
- rotation適用: 共通のCSS transform使用（実装パターン統一）
- PTZ認証: 既存のModal Leaseを再利用（SSoT維持）
- エラー表示: AbsorberError.to_user_message()を活用（車輪の再発明回避）

---

## 6. アンアンビギュアス宣言

本ドキュメントは以下の点で明確性を担保：
1. 各バグの原因と修正箇所がファイル名・行番号レベルで特定済み
2. 新規実装の構造が具体的なファイルパスで定義済み
3. 依存関係が図示され、既存実装との整合性が確認済み

---

## 作成日
2026-01-18

## 関連ドキュメント
- PTZctrl_BasicInstructions.md
- PTZctrl_basicdesign.md
- The_golden_rules.md
