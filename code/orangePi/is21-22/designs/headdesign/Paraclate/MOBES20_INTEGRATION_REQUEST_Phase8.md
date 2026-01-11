# mobes2.0連携確認依頼書: Phase 8 カメラ双方向同期

**作成日**: 2026-01-12
**作成者**: IS22開発チーム (Claude Opus 4.5)
**対象**: mobes2.0開発チーム
**GitHub Issue**: #121
**優先度**: 高

---

## 1. 概要

IS22側でPhase 8「カメラ双方向同期」機能の実装が90%完了しました。
mobes2.0側での対応APIおよびPub/Sub通知の実装状況確認と、連携テスト実施のご協力をお願いいたします。

### 実装目的

IS22（カメラ管理サーバー）とmobes2.0（Paraclate APP）間で、カメラメタデータを双方向同期し、統合的なカメラ監視・管理を実現します。

### SSoT（Single Source of Truth）分担

| データ項目 | SSoT | 説明 |
|-----------|------|------|
| カメラLacisID | araneaDeviceGate | デバイス登録時に発行 |
| カメラ名・コンテキスト | **IS22** | ユーザーが設定、mobes2.0へ同期 |
| カメラ個別設定（感度等） | **mobes2.0** | ユーザーがAPPで設定、IS22へ同期 |

---

## 2. IS22側実装完了項目

### 2.1 実装済みファイル一覧

| ファイル | 内容 |
|---------|------|
| `src/camera_sync/mod.rs` | モジュール定義 |
| `src/camera_sync/types.rs` | 型定義（CameraMetadataPayload, CameraParaclateSettings等） |
| `src/camera_sync/repository.rs` | DB操作リポジトリ |
| `src/camera_sync/sync_service.rs` | 同期サービス（定期同期含む） |
| `src/paraclate_client/client.rs` | API呼び出し（send_camera_metadata, send_camera_deleted） |
| `src/paraclate_client/types.rs` | MobesCameraSettings型追加 |
| `src/paraclate_client/config_sync.rs` | GetConfigカメラ設定同期 |
| `src/paraclate_client/pubsub_subscriber.rs` | Pub/Sub通知ハンドラー |
| `src/web_api/routes.rs` | カメラ更新/削除時の同期トリガー |
| `migrations/026_camera_sync_extension.sql` | DBスキーマ |

### 2.2 実装済み機能

| 機能 | 説明 | 状態 |
|------|------|------|
| カメラメタデータ送信 | IS22→mobes2.0へカメラ名・コンテキスト同期 | ✅ 完了 |
| カメラ削除通知 | IS22→mobes2.0へ削除イベント送信 | ✅ 完了 |
| 定期同期スケジューラ | 1時間間隔で全カメラを自動同期 | ✅ 完了 |
| カメラ更新トリガー | 名前変更時に即座にmobes2.0へ同期 | ✅ 完了 |
| GetConfigカメラ設定取得 | mobes2.0からのカメラ個別設定を受信・保存 | ✅ 完了 |
| Pub/Sub camera_settings | mobes2.0からの設定変更通知を処理 | ✅ 完了 |
| Pub/Sub camera_remove | mobes2.0からの削除指示を処理 | ✅ 完了 |

---

## 3. mobes2.0側対応依頼事項

### 3.1 【必須】paraclateCameraMetadata API

IS22がカメラメタデータを送信するエンドポイントです。

**エンドポイント**: `POST https://paraclatecamerametadata-vm44u3kpua-an.a.run.app`

#### リクエストヘッダー

```
Authorization: LacisOath <base64-encoded-json>
Content-Type: application/json
```

LacisOath JSONペイロード:
```json
{
  "lacisId": "3022XXXXXXXXXXXX0000",
  "tid": "T2025120621041161827",
  "cic": "204965",
  "timestamp": "2026-01-12T10:00:00Z"
}
```

#### リクエストボディ（カメラメタデータ送信）

```json
{
  "cameras": [
    {
      "lacisId": "3801E051D815448B0001",
      "cameraId": "cam_uuid_001",
      "name": "正面玄関カメラ",
      "location": "1F エントランス",
      "context": {
        "purpose": "来客監視",
        "monitoringTarget": "人物",
        "priorityFocus": "不審者検知",
        "operationalNotes": "深夜帯は感度を上げる"
      },
      "mac": "E0:51:D8:15:44:8B",
      "brand": "HIKVISION",
      "model": "DS-2CD2143G2-I",
      "status": "online",
      "lastSeen": "2026-01-12T09:55:00Z"
    }
  ],
  "syncType": "full",
  "updatedAt": "2026-01-12T10:00:00Z"
}
```

#### syncType値

| 値 | 説明 |
|----|------|
| `full` | 全カメラ同期（初回・定期） |
| `partial` | 変更分のみ（削除時） |
| `single` | 単一カメラ更新 |

#### 期待するレスポンス

```json
{
  "success": true,
  "syncedCount": 5,
  "error": null
}
```

---

### 3.2 【必須】カメラ削除通知

同一エンドポイントに削除ペイロードを送信します。

#### リクエストボディ（削除通知）

```json
{
  "deletedCameras": [
    {
      "lacisId": "3801E051D815448B0001",
      "cameraId": "cam_uuid_001",
      "deletedAt": "2026-01-12T10:30:00Z",
      "reason": "user_request"
    }
  ],
  "syncType": "partial",
  "updatedAt": "2026-01-12T10:30:00Z"
}
```

#### reason値

| 値 | 説明 |
|----|------|
| `physical_removal` | カメラ物理取り外し |
| `user_request` | ユーザー削除操作 |
| `malfunction` | 故障による削除 |
| `replacement` | 機器交換 |
| `admin_removal` | 管理者による削除 |

---

### 3.3 【必須】GetConfig レスポンス拡張

既存の`GET /api/paraclate/config`レスポンスに`cameras`フィールドの追加をお願いします。

#### 現行レスポンス

```json
{
  "success": true,
  "config": {
    "reportIntervalMinutes": 60,
    "grandSummaryTimes": ["09:00", "17:00", "21:00"],
    "retentionDays": 60,
    "attunement": {
      "autoTuningEnabled": true,
      "tuningFrequency": "weekly",
      "tuningAggressiveness": 50
    }
  },
  "updatedAt": "2026-01-12T10:00:00Z"
}
```

#### 拡張後レスポンス（cameras追加）

```json
{
  "success": true,
  "config": {
    "reportIntervalMinutes": 60,
    "grandSummaryTimes": ["09:00", "17:00", "21:00"],
    "retentionDays": 60,
    "attunement": { ... }
  },
  "cameras": [
    {
      "lacisId": "3801E051D815448B0001",
      "cameraId": "cam_uuid_001",
      "sensitivity": 0.7,
      "detectionZone": "full",
      "alertThreshold": 3,
      "customPreset": "night_mode",
      "updatedAt": "2026-01-11T15:30:00Z"
    }
  ],
  "updatedAt": "2026-01-12T10:00:00Z"
}
```

#### cameras各フィールド説明

| フィールド | 型 | 説明 | デフォルト |
|-----------|----|----|----------|
| lacisId | string | カメラLacisID（必須） | - |
| cameraId | string? | IS22内部ID（オプション） | null |
| sensitivity | number | 検出感度 (0.0-1.0) | 0.6 |
| detectionZone | string | 検出ゾーン設定 | "full" |
| alertThreshold | number | アラート閾値 | 3 |
| customPreset | string? | カスタムプリセット名 | null |
| updatedAt | string? | 最終更新日時 | null |

---

### 3.4 【必須】Pub/Sub通知タイプ拡張

既存の`paraclate-config-updates`トピックに以下の通知タイプを追加お願いします。

#### camera_settings 通知

ユーザーがmobes2.0 APPでカメラ個別設定を変更した際に送信。

```json
{
  "message": {
    "data": "<base64-encoded-json>",
    "attributes": {
      "tid": "T2025120621041161827",
      "fid": "0150",
      "notificationType": "camera_settings"
    }
  }
}
```

dataデコード後:
```json
{
  "lacisId": "3801E051D815448B0001",
  "sensitivity": 0.8,
  "detectionZone": "custom",
  "alertThreshold": 5,
  "customPreset": "high_security"
}
```

#### camera_remove 通知

管理者がmobes2.0側でカメラを削除した際に送信。

```json
{
  "message": {
    "data": "<base64-encoded-json>",
    "attributes": {
      "tid": "T2025120621041161827",
      "fid": "0150",
      "notificationType": "camera_remove"
    }
  }
}
```

dataデコード後:
```json
{
  "lacisId": "3801E051D815448B0001",
  "reason": "admin_removal"
}
```

---

## 4. カメラLacisID形式

### is801 paraclateCamera LacisID

```
3801{MAC12桁}{ProductCode4桁}
```

| 部分 | 値 | 説明 |
|------|----|----|
| Prefix | `3` | aranea共通 |
| ProductType | `801` | is801 paraclateCamera |
| MAC | 12桁 | カメラMACアドレス（コロン除去） |
| ProductCode | 4桁 | `0001`〜 |

例: `3801E051D815448B0001`

---

## 5. 確認依頼事項

以下について、実装状況・予定のご回答をお願いいたします。

| # | 項目 | 状況確認 |
|---|------|---------|
| 1 | paraclateCameraMetadata Cloud Function | 実装済み / 実装中 / 未着手 / 予定日: |
| 2 | カメラ削除通知の受信処理 | 実装済み / 実装中 / 未着手 / 予定日: |
| 3 | GetConfig cameras フィールド追加 | 実装済み / 実装中 / 未着手 / 予定日: |
| 4 | Pub/Sub camera_settings 通知 | 実装済み / 実装中 / 未着手 / 予定日: |
| 5 | Pub/Sub camera_remove 通知 | 実装済み / 実装中 / 未着手 / 予定日: |
| 6 | 連携テスト可能時期 | 予定日: |

---

## 6. テスト環境情報

### IS22サーバー

| 項目 | 値 |
|------|-----|
| IPアドレス | 192.168.125.246 |
| APIポート | 8080 |
| フロントエンド | http://192.168.125.246:3000 |
| LacisID | `3022XXXXXXXXXXXX0000`（要確認） |

### テスト用アカウント

| 項目 | 値 |
|------|-----|
| TID | T2025120621041161827 |
| FID | 0150 (HALE京都丹波口ホテル) |
| CIC | 204965 |

---

## 7. 連携フロー図

```
┌─────────────────────────────────────────────────────────────────┐
│                    カメラ双方向同期フロー                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  IS22 (カメラ管理)              mobes2.0 (Paraclate APP)        │
│  ─────────────────              ─────────────────────────        │
│                                                                 │
│  [1] カメラ名/コンテキスト変更                                   │
│      ↓                                                          │
│  push_single_camera() ─────────→ paraclateCameraMetadata API    │
│                                       ↓                         │
│                                  カメラ情報更新                  │
│                                                                 │
│  [2] 定期同期（1時間間隔）                                       │
│      ↓                                                          │
│  push_all_cameras() ───────────→ paraclateCameraMetadata API    │
│                                       ↓                         │
│                                  全カメラ同期                    │
│                                                                 │
│  [3] カメラ削除                                                  │
│      ↓                                                          │
│  notify_camera_deleted() ──────→ paraclateCameraMetadata API    │
│                                       ↓                         │
│                                  カメラ削除処理                  │
│                                                                 │
│  [4] ユーザーがAPPで設定変更                                     │
│                                       ↓                         │
│  sync_camera_settings() ←────── Pub/Sub camera_settings         │
│      ↓                                                          │
│  DB保存 (camera_paraclate_settings)                             │
│                                                                 │
│  [5] 管理者がAPPでカメラ削除                                     │
│                                       ↓                         │
│  handle_camera_remove() ←─────── Pub/Sub camera_remove          │
│      ↓                                                          │
│  カメラ論理削除                                                  │
│                                                                 │
│  [6] 定期設定同期                                                │
│      ↓                                                          │
│  ConfigSyncService.sync() ─────→ GET /api/paraclate/config      │
│      ↓                                ↓                         │
│  sync_camera_settings() ←─────── cameras[] in response          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 8. 関連ドキュメント

| ドキュメント | パス |
|------------|------|
| 詳細設計書 | `code/orangePi/is21-22/designs/headdesign/Paraclate/DetailedDesign/DD10_CameraBidirectionalSync.md` |
| 進捗管理 | `code/orangePi/is21-22/designs/headdesign/Paraclate/ImplementationTaskProgressManagement/Phase8_CameraBidirectionalSync.md` |
| スキーマ定義 | `code/orangePi/is21-22/designs/headdesign/Paraclate/SCHEMA_DEFINITIONS.md` |
| is801スキーマ | `code/orangePi/is21-22/designs/headdesign/Paraclate/aranea_ar-is801.schema.json` |

---

## 9. 連絡先

ご不明点・ご質問がございましたら、以下までお問い合わせください。

- **GitHub Issue**: https://github.com/warusakudeveroper/aranea_ISMS/issues/121
- **担当**: IS22開発チーム

---

**以上、ご確認・ご対応のほどよろしくお願いいたします。**
