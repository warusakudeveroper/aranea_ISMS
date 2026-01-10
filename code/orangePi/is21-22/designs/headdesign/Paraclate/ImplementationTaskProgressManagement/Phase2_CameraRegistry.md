# Phase 2: CameraRegistry 実装タスク

対応DD: DD05_CameraRegistry.md
依存関係: Phase 1（AraneaRegister）

---

## 概要

is22配下のカメラを仮想araneaDevice（is801 paraclateCamera）として管理し、lacisIDを付与してParaclate連携を実現する。

---

## タスク一覧

### T2-1: カメラ台帳スキーマ設計・マイグレーション

**状態**: ✅ COMPLETED
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- `cameras`テーブル拡張（lacis_id, tid, fid, rid, cic等）
- `camera_brands`にproduct_code追加
- ProductCode割当（Tapo=0001, Hikvision=0002等）

**成果物**:
- `migrations/022_camera_registry.sql`

**マイグレーションSQL**:
```sql
-- camera_brands に product_code 追加
ALTER TABLE camera_brands
ADD COLUMN product_code VARCHAR(4) UNIQUE;

-- cameras テーブル拡張
ALTER TABLE cameras
ADD COLUMN lacis_id VARCHAR(20) UNIQUE,
ADD COLUMN tid VARCHAR(32),
ADD COLUMN fid VARCHAR(32),
ADD COLUMN rid VARCHAR(32),
ADD COLUMN cic VARCHAR(16),
ADD COLUMN camera_context TEXT,
ADD COLUMN registration_state ENUM('pending', 'registered', 'failed') DEFAULT 'pending',
ADD COLUMN registered_at DATETIME(3),
ADD INDEX idx_lacis_id (lacis_id),
ADD INDEX idx_tid_fid (tid, fid);
```

**検証方法**:
- マイグレーション実行成功
- 既存データ影響なし確認

---

### T2-2: camera_registry.rs サービス実装

**状態**: ✅ COMPLETED
**優先度**: P0（ブロッカー）
**見積もり規模**: L

**内容**:
- `CameraRegistryService`クラス実装
- カメラlacisID生成（3801{MAC}{ProductCode}）
- araneaDeviceGateへの登録

**主要メソッド**:
- `register_camera()`
- `get_registration()`
- `clear_registration()`
- `get_camera_by_lacis_id()`
- `get_registered_cameras()`

**成果物**:
- `src/camera_registry/mod.rs`
- `src/camera_registry/service.rs`
- `src/camera_registry/lacis_id.rs`
- `src/camera_registry/repository.rs`

**検証方法**:
- カメラ登録成功→lacisID付与
- ブランド別ProductCode正確

---

### T2-3: RTSP管理連携

**状態**: ✅ COMPLETED
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- 既存ipcam_scanモジュールとの連携
- 承認済みカメラのみ登録可能
- RTSPステータスとの同期

**成果物**:
- ipcam_scan→camera_registry連携コード

**検証方法**:
- 未承認カメラ登録拒否
- 承認→登録フロー動作

---

### T2-4: detection_logs.rs 検出ログ拡張

**状態**: ✅ COMPLETED
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- 検出ログにcamera_lacis_id紐付け
- tid/fid境界でのログ分離

**成果物**:
- detection_logs保存時のlacisID付与
- 検索APIのlacisID対応

**検証方法**:
- 検出ログにcamera_lacis_id正確
- tid/fid検索動作

---

### T2-5: ログ検索API拡張

**状態**: ✅ COMPLETED
**優先度**: P1（品質改善）
**見積もり規模**: M

**内容**:
- lacisIDでの検出ログ検索
- tid/fid絞り込み

**成果物**:
- API: `GET /api/detection-logs?camera_lacis_id=xxx`

**検証方法**:
- 検索結果の正確性

---

### T2-6: カメラステータス管理

**状態**: ✅ COMPLETED
**優先度**: P1（品質改善）
**見積もり規模**: S

**内容**:
- online/offline/unknown管理
- 登録状態との連携

**成果物**:
- ステータス更新ロジック

**検証方法**:
- ステータス遷移正確

---

### T2-7: カメラコンテキスト管理

**状態**: ✅ COMPLETED
**優先度**: P1（品質改善）
**見積もり規模**: M

**内容**:
- `CameraContextService`実装
- Summary用コンテキストマップ構築

**主要メソッド**:
- `get_context()`
- `update_context()`
- `build_context_map()`

**成果物**:
- `src/camera_registry/context.rs`

**検証方法**:
- コンテキスト保存・取得
- Summary用マップ構築

---

## API実装

### カメラ登録API

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/cameras/:id/register` | POST | カメラ登録 |
| `/api/cameras/:id/registration` | GET | 登録状態取得 |
| `/api/cameras/:id/registration` | DELETE | 登録解除 |

### カメラコンテキストAPI

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/cameras/:id/context` | GET | コンテキスト取得 |
| `/api/cameras/:id/context` | PUT | コンテキスト更新 |

### カメラ一覧API（拡張）

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/cameras` | GET | カメラ一覧（lacisID情報付き） |

**成果物**:
- `src/web_api/camera_registry_routes.rs`

---

## ProductCode割当表

| ブランド | ProductCode |
|---------|-------------|
| Tapo | 0001 |
| Hikvision | 0002 |
| Dahua | 0003 |
| Reolink | 0004 |
| Axis | 0005 |
| Generic | 0000 |

---

## テストチェックリスト

- [ ] T2-1: マイグレーション実行確認
- [ ] T2-2: カメラlacisID生成テスト（各ブランド）
- [ ] T2-2: 登録E2Eテスト
- [ ] T2-3: 未承認カメラ登録拒否テスト
- [ ] T2-4: 検出ログlacisID紐付けテスト
- [ ] T2-5: ログ検索APIテスト
- [ ] T2-7: コンテキスト保存・取得テスト

---

## E2E統合テスト（Phase完了時）

| テストID | 内容 | 確認項目 |
|---------|------|---------|
| E1 | デバイス登録→台帳反映 | Phase 1,2 |
| E2 | カメラ検出→ログ記録→Summary生成 | Phase 2,3 |

---

## 完了条件

1. Phase 1完了が前提
2. 全タスク（T2-1〜T2-7）が✅ COMPLETED
3. テストチェックリスト全項目パス
4. E1, E2テスト実行可能

---

## Issue連携

**Phase Issue**: #115
**親Issue**: #113

全タスクは#115で一括管理。個別タスクのサブIssue化は必要に応じて実施。

---

## 更新履歴

| 日付 | 更新内容 |
|------|---------|
| 2026-01-10 | 初版作成 |
| 2026-01-10 | T2-1〜T2-7全タスク完了 |
