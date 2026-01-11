# DD10: カメラ双方向同期 詳細設計書

作成日: 2026-01-11
最終更新: 2026-01-11
バージョン: 1.0.0
Phase: 8 (Camera Bidirectional Sync)
GitHub Issue: TBD (#121)

---

## 1. 概要

### 1.1 目的

IS22とmobes2.0（Paraclate APP）間でカメラメタデータ（名前、コンテキスト、設定）の双方向同期を実現する。

### 1.2 背景

現在の実装では以下の問題が存在する：

| 機能 | IS22→mobes | mobes→IS22 | 状態 |
|------|-----------|------------|------|
| カメラ登録 (LacisID) | ✅ 実装済 | - | 動作確認済 |
| カメラ名同期 | ❌ 未実装 | ❌ 未実装 | **GAP** |
| カメラコンテキスト同期 | ❌ 未実装 | ❌ 未実装 | **GAP** |
| カメラ削除 | ❌ 未実装 | ❌ 未実装 | **GAP** |
| カメラ個別設定 | ❌ 未実装 | ⚠️ 部分的 | **GAP** |
| グローバル設定 | - | ✅ 実装済 | 動作確認済 |

### 1.3 参照仕様（Paraclate_DesignOverview.md）

1. **userObjectDetail要件**:
   > "is22のuserObjectDetailスキーマでは傘下のカメラのlacisIDリスト、Paraclateでの設定項目を値として持つ"

2. **双方向同期要件**:
   > "サーバーの設定関連については設定の双方向同期、ステータスについてはPUB/SUBトピックで行う"

3. **is801 paraclateCamera要件**:
   > "カメラ設定でのカメラ側のパラメータについてはis801 paraclateCameraとして独立したuserObject,userObjectDetailで管理"

---

## 2. 設計原則

### 2.1 SSoT分担

| データ種別 | SSoT | 理由 |
|-----------|------|------|
| カメラLacisID | araneaDeviceGate | デバイス登録の唯一の入口 |
| カメラ名・コンテキスト | IS22 | 物理デバイスに最も近い |
| カメラ個別設定（感度等） | mobes2.0 | テナント管理者が設定 |
| グローバル設定 | mobes2.0 | テナント単位の設定 |
| 配下カメラリスト | IS22 (→mobes2.0に同期) | IS22が実際に管理 |

### 2.2 同期方向

```
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│    IS22 (Camera Source)              mobes2.0 (Settings SSoT)   │
│                                                                  │
│    ┌──────────────────┐              ┌──────────────────┐       │
│    │ Camera Registry  │─────────────▶│ UserObjectDetail │       │
│    │                  │  名前/コンテキスト │ (is22傘下カメラ)  │       │
│    │ - name          │              │ - cameraList[]  │       │
│    │ - context       │              │ - settings      │       │
│    │ - lacis_id      │              │                 │       │
│    └──────────────────┘              └──────────────────┘       │
│            ▲                                 │                   │
│            │                                 │                   │
│            │         カメラ個別設定           │                   │
│            └─────────────────────────────────┘                   │
│                      (Pub/Sub通知)                               │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 2.3 MECE分類

| カテゴリ | IS22→mobes | mobes→IS22 | トリガー |
|---------|-----------|------------|---------|
| カメラ登録 | ✅ 必須 | - | 新規カメラ追加時 |
| カメラ名変更 | ✅ 必須 | ❌ 不要 | IS22で名前変更時 |
| カメラコンテキスト | ✅ 必須 | ❌ 不要 | IS22でコンテキスト変更時 |
| カメラ削除 | ✅ 必須 | ✅ 必須 | どちらかで削除時 |
| カメラ個別設定 | ❌ 不要 | ✅ 必須 | mobes2.0で設定変更時 |

---

## 3. 機能仕様

### 3.1 IS22→mobes2.0 同期

#### 3.1.1 カメラメタデータ送信 API

**エンドポイント**: `POST /paraclate/camera-metadata`

**新規追加（mobes2.0側）**:
```
https://paraclatecamerametadata-vm44u3kpua-an.a.run.app
```

**リクエスト形式**:
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
- `full`: 全カメラリスト同期（初回・定期）
- `partial`: 変更分のみ（名前変更・削除）
- `single`: 単一カメラ更新

#### 3.1.2 カメラ削除通知 API

**リクエスト形式**:
```json
{
  "fid": "0150",
  "payload": {
    "deletedCameras": [
      {
        "lacisId": "3801E051D815448B0001",
        "cameraId": "cam_001",
        "deletedAt": "2026-01-11T12:00:00Z",
        "reason": "physical_removal"
      }
    ],
    "syncType": "partial",
    "updatedAt": "2026-01-11T12:00:00Z"
  }
}
```

**削除理由（reason）**:
- `physical_removal`: 物理的にカメラを取り外し
- `user_request`: ユーザーによる削除操作
- `malfunction`: 故障による削除
- `replacement`: 機器交換

### 3.2 mobes2.0→IS22 同期

#### 3.2.1 Pub/Sub通知拡張

**Topic**: `paraclate-config-updates`

**追加通知タイプ**:
```typescript
type NotificationType =
  | "config_update"      // 既存: グローバル設定変更
  | "config_delete"      // 既存: 設定削除
  | "disconnect"         // 既存: 接続解除
  | "force_sync"         // 既存: 強制同期
  | "camera_settings"    // 新規: カメラ個別設定変更
  | "camera_remove"      // 新規: カメラ削除指示
```

**camera_settings通知ペイロード**:
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

**camera_remove通知ペイロード**:
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

#### 3.2.2 GetConfig拡張

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

**拡張レスポンス**:
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

---

## 4. データモデル

### 4.1 DBスキーマ拡張

**マイグレーション**: `025_camera_sync_extension.sql`

```sql
-- カメラ同期状態テーブル
CREATE TABLE camera_sync_state (
    id INT AUTO_INCREMENT PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL,
    lacis_id VARCHAR(24) NOT NULL,
    last_sync_to_mobes DATETIME(3) NULL,
    last_sync_from_mobes DATETIME(3) NULL,
    mobes_sync_status ENUM('synced', 'pending', 'failed', 'deleted') DEFAULT 'pending',
    sync_error_message TEXT NULL,
    retry_count INT DEFAULT 0,
    created_at DATETIME(3) DEFAULT NOW(3),
    updated_at DATETIME(3) DEFAULT NOW(3) ON UPDATE NOW(3),

    UNIQUE KEY uk_camera_id (camera_id),
    UNIQUE KEY uk_lacis_id (lacis_id),
    INDEX idx_sync_status (mobes_sync_status)
);

-- カメラ個別設定テーブル（mobes2.0からの同期用）
CREATE TABLE camera_paraclate_settings (
    id INT AUTO_INCREMENT PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL,
    lacis_id VARCHAR(24) NOT NULL,
    sensitivity DECIMAL(3,2) DEFAULT 0.6,
    detection_zone VARCHAR(32) DEFAULT 'full',
    alert_threshold INT DEFAULT 3,
    custom_preset VARCHAR(64) NULL,
    settings_json JSON NULL,
    synced_at DATETIME(3) NOT NULL,
    created_at DATETIME(3) DEFAULT NOW(3),
    updated_at DATETIME(3) DEFAULT NOW(3) ON UPDATE NOW(3),

    UNIQUE KEY uk_camera_id (camera_id),
    UNIQUE KEY uk_lacis_id (lacis_id)
);

-- camerasテーブル拡張
ALTER TABLE cameras
    ADD COLUMN mobes_synced_at DATETIME(3) NULL AFTER updated_at,
    ADD COLUMN mobes_sync_version INT DEFAULT 0 AFTER mobes_synced_at;
```

### 4.2 Rust型定義

**ファイル**: `src/camera_sync/types.rs`

```rust
/// カメラメタデータ同期ペイロード
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraMetadataPayload {
    pub cameras: Vec<CameraMetadataEntry>,
    pub sync_type: SyncType,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraMetadataEntry {
    pub lacis_id: String,
    pub camera_id: String,
    pub name: String,
    pub location: Option<String>,
    pub context: CameraContextData,
    pub mac: String,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub status: CameraStatus,
    pub last_seen: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncType {
    Full,
    Partial,
    Single,
}

/// カメラ削除通知
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraDeletedEntry {
    pub lacis_id: String,
    pub camera_id: String,
    pub deleted_at: DateTime<Utc>,
    pub reason: DeleteReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteReason {
    PhysicalRemoval,
    UserRequest,
    Malfunction,
    Replacement,
    AdminRemoval,
}

/// カメラ個別設定（mobes2.0から同期）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraParaclateSettings {
    pub sensitivity: f32,
    pub detection_zone: String,
    pub alert_threshold: i32,
    pub custom_preset: Option<String>,
}
```

---

## 5. 実装タスク

### 5.1 タスク一覧

| タスクID | タスク名 | 依存 | 優先度 |
|---------|---------|------|-------|
| T8-1 | DBマイグレーション（025_camera_sync_extension.sql） | なし | 高 |
| T8-2 | camera_sync モジュール作成（types, repository） | T8-1 | 高 |
| T8-3 | IS22→mobes メタデータ送信機能 | T8-2 | 高 |
| T8-4 | カメラ名変更トリガー実装 | T8-3 | 高 |
| T8-5 | カメラ削除通知機能（IS22→mobes） | T8-3 | 高 |
| T8-6 | Pub/Sub camera_settings ハンドラー | T8-2 | 中 |
| T8-7 | Pub/Sub camera_remove ハンドラー | T8-2 | 中 |
| T8-8 | GetConfig カメラ個別設定取得拡張 | T8-2 | 中 |
| T8-9 | 定期同期スケジューラ | T8-3 | 低 |
| T8-10 | 統合テスト | T8-1〜T8-9 | 高 |

### 5.2 依存関係図

```
T8-1 (DBマイグレーション)
  │
  ▼
T8-2 (types, repository)
  │
  ├──────────────────────────┐
  ▼                          ▼
T8-3 (IS22→mobes送信)        T8-6 (Pub/Sub camera_settings)
  │                          │
  ├──────────┐               │
  ▼          ▼               ▼
T8-4        T8-5           T8-7 (camera_remove)
(名前変更)  (削除通知)        │
  │          │               │
  └──────────┴───────────────┤
                             ▼
                           T8-8 (GetConfig拡張)
                             │
                             ▼
                           T8-9 (定期同期)
                             │
                             ▼
                           T8-10 (統合テスト)
```

---

## 6. API仕様詳細

### 6.1 IS22内部API（カメラ同期用）

#### POST /api/camera-sync/push
カメラメタデータをmobes2.0へプッシュ

**リクエスト**:
```json
{
  "syncType": "full",
  "cameraIds": []  // 空の場合は全カメラ
}
```

**レスポンス**:
```json
{
  "success": true,
  "syncedCount": 5,
  "failedCount": 0,
  "details": [
    {
      "cameraId": "cam_001",
      "lacisId": "3801E051D815448B0001",
      "status": "synced"
    }
  ]
}
```

#### GET /api/camera-sync/status
同期状態を取得

**レスポンス**:
```json
{
  "lastFullSync": "2026-01-11T12:00:00Z",
  "pendingCount": 0,
  "syncedCount": 5,
  "failedCount": 0,
  "cameras": [
    {
      "cameraId": "cam_001",
      "lacisId": "3801E051D815448B0001",
      "mobesSyncStatus": "synced",
      "lastSyncToMobes": "2026-01-11T12:00:00Z",
      "lastSyncFromMobes": "2026-01-11T11:30:00Z"
    }
  ]
}
```

### 6.2 mobes2.0 API（新規追加必要）

#### POST /paraclate/camera-metadata
IS22からのカメラメタデータ受信

**必要な実装（mobes2.0側）**:
1. Cloud Functions `paraclateCameraMetadata`
2. Firestoreコレクション `paraclate_cameras/{tid}/cameras/{lacisId}`

---

## 7. シーケンス図

### 7.1 カメラ名変更フロー（IS22→mobes）

```
User         IS22 WebUI       IS22 Backend      mobes2.0
  │              │                │                │
  │ カメラ名変更  │                │                │
  ├─────────────▶│                │                │
  │              │ PUT /cameras/  │                │
  │              ├───────────────▶│                │
  │              │                │ DB更新         │
  │              │                ├────┐           │
  │              │                │    │           │
  │              │                │◀───┘           │
  │              │                │                │
  │              │                │ POST /paraclate/camera-metadata
  │              │                ├───────────────▶│
  │              │                │                │ UserObjectDetail更新
  │              │                │                ├────┐
  │              │                │                │    │
  │              │                │   success      │◀───┘
  │              │◀───────────────┼────────────────│
  │ 完了         │                │                │
  │◀─────────────│                │                │
```

### 7.2 カメラ設定変更フロー（mobes→IS22）

```
Admin        mobes2.0 UI       mobes2.0         Pub/Sub        IS22
  │              │                │                │              │
  │ 感度変更     │                │                │              │
  ├─────────────▶│                │                │              │
  │              │ PUT /settings  │                │              │
  │              ├───────────────▶│                │              │
  │              │                │ Firestore更新  │              │
  │              │                ├────┐           │              │
  │              │                │    │           │              │
  │              │                │◀───┘           │              │
  │              │                │                │              │
  │              │                │ Publish        │              │
  │              │                ├───────────────▶│              │
  │              │                │                │ Push通知     │
  │              │                │                ├─────────────▶│
  │              │                │                │              │ 設定同期
  │              │                │                │              ├────┐
  │              │                │                │              │    │
  │              │                │                │              │◀───┘
  │              │◀───────────────┼────────────────┼──────────────│
  │ 完了         │                │                │              │
  │◀─────────────│                │                │              │
```

---

## 8. テスト計画

### 8.1 単体テスト

| テストID | テスト対象 | テスト内容 |
|---------|----------|----------|
| UT8-1 | CameraSyncRepository | CRUD操作、同期状態管理 |
| UT8-2 | CameraMetadataPayload | シリアライズ/デシリアライズ |
| UT8-3 | PubSubSubscriber | camera_settings通知処理 |
| UT8-4 | PubSubSubscriber | camera_remove通知処理 |

### 8.2 統合テスト

| テストID | テストシナリオ | 期待結果 |
|---------|--------------|---------|
| IT8-1 | IS22でカメラ名変更→mobes2.0確認 | UserObjectDetailに反映 |
| IT8-2 | IS22でカメラ削除→mobes2.0確認 | 削除通知がmobesに到達 |
| IT8-3 | mobes2.0で感度変更→IS22確認 | camera_paraclate_settingsに反映 |
| IT8-4 | mobes2.0でカメラ削除指示→IS22確認 | カメラがdeleted状態に |

### 8.3 E2Eテスト

| テストID | シナリオ | 確認項目 |
|---------|---------|---------|
| E8-1 | 新規カメラ追加→同期→設定変更→反映 | 全フロー正常動作 |
| E8-2 | カメラ削除（IS22起点）→mobes反映 | 削除状態同期 |
| E8-3 | カメラ削除（mobes起点）→IS22反映 | Pub/Sub→ローカル削除 |

---

## 9. モジュール構成

### 9.1 ディレクトリ構造

```
src/
├── camera_sync/
│   ├── mod.rs
│   ├── types.rs           # 型定義
│   ├── repository.rs      # DB操作
│   ├── sync_service.rs    # 同期サービス
│   ├── metadata_pusher.rs # IS22→mobes送信
│   └── settings_puller.rs # mobes→IS22取得
├── paraclate_client/
│   ├── pubsub_subscriber.rs  # 拡張（camera_settings/remove）
│   └── config_sync.rs        # 拡張（カメラ設定取得）
└── camera_registry/
    └── service.rs            # 拡張（同期トリガー）
```

### 9.2 既存コード修正箇所

| ファイル | 修正内容 |
|---------|---------|
| `pubsub_subscriber.rs` | NotificationType拡張、ハンドラー追加 |
| `camera_registry/service.rs` | register_to_gate()でcamera_name送信 |
| `config_sync.rs` | カメラ個別設定取得追加 |
| `web_api/routes.rs` | camera-sync APIルート追加 |

---

## 10. リスク・制約

### 10.1 リスク

| リスク | 影響度 | 対策 |
|-------|-------|------|
| mobes2.0側API未実装 | 高 | 事前にAPI仕様合意、スタブ対応 |
| 同期衝突 | 中 | タイムスタンプベースの楽観ロック |
| ネットワーク障害 | 中 | リトライキュー、オフライン対応 |

### 10.2 制約

- mobes2.0側の `paraclateCameraMetadata` Cloud Function新規追加が必要
- Pub/Sub通知タイプ拡張にmobes2.0側の対応が必要
- GetConfig APIのレスポンス拡張にmobes2.0側の対応が必要

---

## 11. 完了条件

- [ ] T8-1〜T8-10 全タスク完了
- [ ] 単体テスト（UT8-1〜UT8-4）パス
- [ ] 統合テスト（IT8-1〜IT8-4）パス
- [ ] E2Eテスト（E8-1〜E8-3）パス
- [ ] mobes2.0 APIとの疎通確認
- [ ] The_golden_rules.md準拠確認

---

## 12. 更新履歴

| 日付 | バージョン | 更新内容 | 更新者 |
|------|-----------|---------|-------|
| 2026-01-11 | 1.0.0 | 初版作成 | Claude |
