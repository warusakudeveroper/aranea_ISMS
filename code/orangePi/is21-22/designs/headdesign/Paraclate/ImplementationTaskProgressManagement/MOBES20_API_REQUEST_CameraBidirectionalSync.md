# mobes2.0チームへのAPI実装依頼: カメラ双方向同期

作成日: 2026-01-11
作成者: IS22開発チーム
優先度: **高**
関連Issue: #121

---

## 概要

IS22（カメラ管理サーバー）とmobes2.0（Paraclate APP）間でカメラメタデータの双方向同期を実現するため、mobes2.0側に以下のAPI実装をお願いいたします。

**IS22側の実装は完了しており、mobes2.0側APIが実装され次第、即座に接続可能です。**

---

## 背景・目的

### Paraclate_DesignOverview.mdの要件

> - "is22のuserObjectDetailスキーマでは傘下のカメラのlacisIDリスト、Paraclateでの設定項目を値として持つ"
> - "サーバーの設定関連については設定の双方向同期、ステータスについてはPUB/SUBトピックで行う"
> - "カメラ設定でのカメラ側のパラメータについてはis801 paraclateCameraとして独立したuserObject,userObjectDetailで管理"

### 現状の課題

| 機能 | IS22→mobes | mobes→IS22 | 状態 |
|------|-----------|------------|------|
| カメラLacisID登録 | ✅ 実装済 | - | 動作中 |
| カメラ名・コンテキスト同期 | ✅ IS22実装済 | - | **mobes API待ち** |
| カメラ個別設定同期 | - | ⚠️ GetConfig拡張必要 | **mobes対応待ち** |
| カメラ削除通知 | ✅ IS22実装済 | ⚠️ Pub/Sub拡張必要 | **mobes対応待ち** |

---

## 実装依頼内容

### 1. `paraclateCameraMetadata` Cloud Function（新規）

IS22からカメラメタデータを受信し、Firestoreに保存するAPIの新規追加をお願いします。

**エンドポイント案**:
```
POST https://paraclatecamerametadata-vm44u3kpua-an.a.run.app
```

**認証**: LacisOath（既存の形式）
```
Authorization: LacisOath <base64-encoded-json>

JSON構造:
{
  "lacisId": "3022...",
  "tid": "T2025120621041161827",
  "cic": "605123",
  "timestamp": "2026-01-11T12:00:00Z"
}
```

**リクエストBody**:
```json
{
  "fid": "0150",
  "payload": {
    "cameras": [
      {
        "lacisId": "3801E051D815448B0001",
        "cameraId": "cam_001",
        "name": "正面玄関カメラ",
        "location": "1F エントランス",
        "context": {
          "purpose": "来客監視",
          "monitoringTarget": "人物",
          "priorityFocus": "不審者検出",
          "operationalNotes": "深夜は感度を上げる"
        },
        "mac": "E0:51:D8:15:44:8B",
        "brand": "HIKVISION",
        "model": "DS-2CD2143G2-I",
        "status": "online",
        "lastSeen": "2026-01-11T12:00:00Z"
      }
    ],
    "syncType": "full",
    "updatedAt": "2026-01-11T12:00:00Z"
  }
}
```

**syncType値**:
| 値 | 説明 | トリガー |
|----|------|---------|
| `full` | 全カメラ同期 | 初回接続時、定期同期（1時間ごと） |
| `partial` | 変更分のみ | カメラ削除時 |
| `single` | 単一カメラ | カメラ名変更時 |

**Firestoreコレクション案**:
```
paraclate_cameras/{tid}/cameras/{lacisId}
```

**期待するレスポンス**:
```json
{
  "success": true,
  "syncedCount": 5,
  "updatedAt": "2026-01-11T12:00:00Z"
}
```

---

### 2. Pub/Sub通知タイプ拡張

`paraclate-config-updates` Topicに以下の通知タイプを追加してください。

#### 2.1 `camera_settings` 通知

mobes2.0側でカメラ個別設定（感度等）が変更された際にIS22へ通知。

```json
{
  "type": "camera_settings",
  "tid": "T2025120621041161827",
  "fids": ["0150"],
  "updatedAt": "2026-01-11T12:00:00Z",
  "actor": "user_123",
  "changedCameras": [
    {
      "lacisId": "3801E051D815448B0001",
      "changedFields": ["sensitivity", "detectionZone"]
    }
  ]
}
```

**IS22側ハンドラー**: `pubsub_subscriber.rs` に実装済み

#### 2.2 `camera_remove` 通知

mobes2.0側でカメラを削除（登録解除）した際にIS22へ通知。

```json
{
  "type": "camera_remove",
  "tid": "T2025120621041161827",
  "fids": ["0150"],
  "updatedAt": "2026-01-11T12:00:00Z",
  "actor": "admin_456",
  "removedCameras": [
    {
      "lacisId": "3801E051D815448B0001",
      "reason": "admin_removal"
    }
  ]
}
```

**削除理由（reason）**:
- `admin_removal`: 管理者による削除
- `tenant_offboarding`: テナント解約
- `security_violation`: セキュリティ違反

---

### 3. GetConfig APIレスポンス拡張

`GET /config/{tid}?fid={fid}` のレスポンスに `cameras` フィールドを追加してください。

**現行レスポンス**:
```json
{
  "success": true,
  "config": {
    "reportIntervalMinutes": 60,
    "grandSummaryTimes": ["09:00", "17:00", "21:00"],
    "reportDetail": "standard"
  }
}
```

**拡張後レスポンス**:
```json
{
  "success": true,
  "config": {
    "global": {
      "reportIntervalMinutes": 60,
      "grandSummaryTimes": ["09:00", "17:00", "21:00"],
      "reportDetail": "standard"
    },
    "cameras": {
      "3801E051D815448B0001": {
        "sensitivity": 0.7,
        "detectionZone": "full",
        "alertThreshold": 3,
        "customPreset": "entrance"
      },
      "3801AABBCCDDEEFF0002": {
        "sensitivity": 0.5,
        "detectionZone": "lower_half",
        "alertThreshold": 5,
        "customPreset": "parking"
      }
    }
  }
}
```

**カメラ設定フィールド**:
| フィールド | 型 | デフォルト | 説明 |
|-----------|-----|----------|------|
| sensitivity | float | 0.6 | 検出感度 (0.0-1.0) |
| detectionZone | string | "full" | 検出ゾーン |
| alertThreshold | int | 3 | アラート閾値 |
| customPreset | string | null | カスタムプリセット名 |

---

## IS22側実装状況

### 完了済み（即座に接続可能）

| ファイル | 内容 |
|---------|------|
| `migrations/026_camera_sync_extension.sql` | DBスキーマ |
| `src/camera_sync/types.rs` | 型定義（15型） |
| `src/camera_sync/repository.rs` | DB操作 |
| `src/camera_sync/sync_service.rs` | 同期サービス |
| `src/paraclate_client/pubsub_subscriber.rs` | Pub/Subハンドラー拡張 |

### GitHub リンク

- **詳細設計書**: [`DD10_CameraBidirectionalSync.md`](../DetailedDesign/DD10_CameraBidirectionalSync.md)
- **タスク進捗**: [`Phase8_CameraBidirectionalSync.md`](./Phase8_CameraBidirectionalSync.md)
- **マスターインデックス**: [`MASTER_INDEX.md`](./MASTER_INDEX.md)

---

## 実装優先順位提案

| 順位 | 項目 | 理由 |
|------|------|------|
| 1 | `paraclateCameraMetadata` API | IS22からのメタデータ同期の基盤 |
| 2 | GetConfig `cameras`フィールド | カメラ個別設定の同期に必須 |
| 3 | Pub/Sub `camera_settings` | 設定変更のリアルタイム反映 |
| 4 | Pub/Sub `camera_remove` | 削除の双方向連携 |

---

## 質問・確認事項

1. **既存実装の有無**: 上記機能の一部または全部がすでに実装されている場合、エンドポイントURLと仕様を教えてください。

2. **Firestoreスキーマ**: `paraclate_cameras`コレクションの構造に既存の設計がある場合、それに合わせます。

3. **Pub/Subトピック**: `paraclate-config-updates`以外のトピックを使用する場合、トピック名を教えてください。

4. **スケジュール**: 上記APIの実装予定時期があれば教えてください。

---

## 連絡先

- **IS22開発担当**: aranea_ISMS/code/orangePi/is22
- **関連リポジトリ**: https://github.com/warusakudeveroper/aranea_ISMS

---

## 更新履歴

| 日付 | 内容 |
|------|------|
| 2026-01-11 | 初版作成 |
