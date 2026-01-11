# Phase 8: カメラ双方向同期 実装タスク進捗管理

作成日: 2026-01-11
最終更新: 2026-01-12
対応設計書: DD10_CameraBidirectionalSync.md
GitHub Issue: #121（TBD）

---

## 概要

IS22とmobes2.0（Paraclate APP）間でのカメラメタデータ双方向同期を実装する。

### 目的

- カメラ名・コンテキストのIS22→mobes2.0同期
- カメラ個別設定のmobes2.0→IS22同期
- カメラ削除の双方向ハンドリング

### 対応要件（Paraclate_DesignOverview.md）

1. "is22のuserObjectDetailスキーマでは傘下のカメラのlacisIDリスト"
2. "サーバーの設定関連については設定の双方向同期"
3. "is801 paraclateCameraとして独立したuserObject,userObjectDetailで管理"

---

## タスク一覧

| タスクID | タスク名 | 状態 | 担当 |
|---------|---------|------|------|
| T8-1 | DBマイグレーション（026_camera_sync_extension.sql） | ✅ COMPLETED | Claude |
| T8-2 | camera_sync モジュール作成（types, repository, sync_service） | ✅ COMPLETED | Claude |
| T8-3 | IS22→mobes メタデータ送信機能 | ✅ COMPLETED | Claude |
| T8-4 | カメラ名変更トリガー実装 | 🔄 IN_PROGRESS | Claude |
| T8-5 | カメラ削除通知機能（IS22→mobes） | ✅ COMPLETED | Claude |
| T8-6 | Pub/Sub camera_settings ハンドラー | ✅ COMPLETED | Claude |
| T8-7 | Pub/Sub camera_remove ハンドラー | ✅ COMPLETED | Claude |
| T8-8 | GetConfig カメラ個別設定取得拡張 | ⬜ NOT_STARTED | - |
| T8-9 | 定期同期スケジューラ | ⬜ NOT_STARTED | - |
| T8-10 | 統合テスト | ⬜ NOT_STARTED | - |

---

## タスク詳細

### T8-1: DBマイグレーション

**ファイル**: `migrations/026_camera_sync_extension.sql`

**内容**:
- `camera_sync_state` テーブル作成
- `camera_paraclate_settings` テーブル作成
- `camera_sync_logs` テーブル作成

**完了条件**:
- [x] マイグレーションファイル作成
- [ ] ローカルDB適用確認
- [ ] 本番DB適用確認

---

### T8-2: camera_sync モジュール作成

**ファイル**:
- `src/camera_sync/mod.rs`
- `src/camera_sync/types.rs`
- `src/camera_sync/repository.rs`
- `src/camera_sync/sync_service.rs`

**内容**:
- `CameraMetadataPayload` 型定義
- `CameraDeletedEntry` 型定義
- `CameraParaclateSettings` 型定義
- `CameraSyncRepository` CRUD実装
- `CameraSyncService` 同期サービス

**完了条件**:
- [x] 型定義実装
- [x] Repository実装
- [x] SyncService実装
- [ ] 単体テスト（UT8-1, UT8-2）パス

---

### T8-3: IS22→mobes メタデータ送信機能

**ファイル**:
- `src/camera_sync/sync_service.rs`
- `src/paraclate_client/client.rs`

**内容**:
- `CameraSyncService.push_all_cameras()` 全カメラ同期
- `CameraSyncService.push_single_camera()` 単一カメラ同期
- `ParaclateClient.send_camera_metadata()` API呼び出し

**エンドポイント**: `POST https://paraclatecamerametadata-vm44u3kpua-an.a.run.app`

**完了条件**:
- [x] SyncService実装
- [x] ParaclateClient.send_camera_metadata()実装
- [x] エラーハンドリング実装

---

### T8-4: カメラ名変更トリガー実装

**ファイル**: `src/camera_registry/service.rs` (修正)

**内容**:
- `update_camera_name()` で同期トリガー追加
- `update_camera_context()` で同期トリガー追加
- 変更検出ロジック

**完了条件**:
- [ ] 名前変更時の自動同期
- [ ] コンテキスト変更時の自動同期
- [ ] 統合テスト（IT8-1）パス

---

### T8-5: カメラ削除通知機能（IS22→mobes）

**ファイル**:
- `src/camera_sync/sync_service.rs`
- `src/paraclate_client/client.rs`

**内容**:
- `CameraSyncService.notify_camera_deleted()` メソッド
- `ParaclateClient.send_camera_deleted()` API呼び出し
- 削除理由の付与

**完了条件**:
- [x] 削除通知送信実装
- [x] 削除理由の適切な設定
- [ ] 統合テスト（IT8-2）パス

---

### T8-6: Pub/Sub camera_settings ハンドラー

**ファイル**: `src/paraclate_client/pubsub_subscriber.rs` (修正)

**内容**:
- `NotificationType::CameraSettings` 追加
- `handle_camera_settings()` ハンドラー実装
- CameraSyncService統合

**完了条件**:
- [x] 通知タイプ拡張
- [x] ハンドラー実装
- [x] CameraSyncService統合
- [ ] 単体テスト（UT8-3）パス

---

### T8-7: Pub/Sub camera_remove ハンドラー

**ファイル**: `src/paraclate_client/pubsub_subscriber.rs` (修正)

**内容**:
- `NotificationType::CameraRemove` 追加
- `handle_camera_remove()` ハンドラー実装
- CameraSyncService統合

**完了条件**:
- [x] 通知タイプ拡張
- [x] ハンドラー実装
- [x] CameraSyncService統合
- [ ] 単体テスト（UT8-4）パス
- [ ] 統合テスト（IT8-4）パス

---

### T8-8: GetConfig カメラ個別設定取得拡張

**ファイル**: `src/paraclate_client/config_sync.rs` (修正)

**内容**:
- GetConfigレスポンスの `cameras` フィールドパース
- カメラ個別設定の抽出・保存
- `camera_paraclate_settings` テーブル更新

**完了条件**:
- [ ] レスポンスパーサー拡張
- [ ] カメラ設定保存実装
- [ ] 統合テスト（IT8-3）パス

---

### T8-9: 定期同期スケジューラ

**ファイル**: `src/camera_sync/sync_service.rs`

**内容**:
- 定期的な全カメラ同期
- 同期間隔設定（デフォルト: 1時間）
- 同期状態ログ出力

**完了条件**:
- [ ] スケジューラ実装
- [ ] 設定可能な同期間隔
- [ ] ヘルスチェック連携

---

### T8-10: 統合テスト

**内容**:
- E2Eテストツール拡張
- 全シナリオテスト実行
- mobes2.0 APIとの疎通確認

**完了条件**:
- [ ] E2Eテスト（E8-1〜E8-3）パス
- [ ] 実機確認（Chrome 192.168.125.246:3000）

---

## 進捗サマリー

| 項目 | 値 |
|------|-----|
| 全タスク数 | 10 |
| 完了タスク | 7 |
| 進行中タスク | 1 |
| 未着手タスク | 2 |
| 進捗率 | 70% |

### 実装済みファイル一覧（2026-01-12）

| ファイル | 内容 |
|---------|------|
| `migrations/026_camera_sync_extension.sql` | DBスキーマ（camera_sync_state, camera_paraclate_settings, camera_sync_logs） |
| `src/camera_sync/mod.rs` | モジュール定義 |
| `src/camera_sync/types.rs` | 型定義（15型: CameraMetadataPayload, CameraParaclateSettings等） |
| `src/camera_sync/repository.rs` | DB操作リポジトリ（479行） |
| `src/camera_sync/sync_service.rs` | 同期サービス（push_all_cameras, notify_camera_deleted, API呼び出し統合） |
| `src/paraclate_client/client.rs` | 拡張（send_camera_metadata, send_camera_deleted メソッド追加） |
| `src/paraclate_client/pubsub_subscriber.rs` | 拡張（CameraSettings, CameraRemove通知タイプ + CameraSyncService統合） |

---

## 依存関係

### 前提条件

- Phase 1〜7 完了済み
- IS22サーバーデプロイ済み（192.168.125.246:8080）
- mobes2.0 Connect API疎通確認済み

### mobes2.0側対応必要事項

| 項目 | 状態 | 備考 |
|------|------|------|
| paraclateCameraMetadata Cloud Function | ⚠️ 確認中 | IS22側実装完了、mobes2.0側API待ち |
| Pub/Sub通知タイプ拡張 | ⚠️ 確認中 | camera_settings, camera_remove |
| GetConfigレスポンス拡張 | ⬜ 未実装 | cameras フィールド追加 |

---

## 更新履歴

| 日付 | 更新内容 |
|------|---------|
| 2026-01-11 | 初版作成 |
| 2026-01-12 | ParaclateClient API呼び出し実装、PubSubSubscriber CameraSyncService統合 |
