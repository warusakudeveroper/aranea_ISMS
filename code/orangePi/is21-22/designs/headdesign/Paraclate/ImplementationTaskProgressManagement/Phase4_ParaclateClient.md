# Phase 4: ParaclateClient 実装タスク

対応DD: DD03_ParaclateClient.md
依存関係: Phase 1,2,3（AraneaRegister, CameraRegistry, Summary）

---

## 概要

is22からParaclate APP（mobes2.0側）へのSummary送信、設定同期、接続テスト機能を実装する。
lacisOath認証を用いた安全な通信を実現する。

---

## 通信方向

```
┌─────────────────┐                    ┌─────────────────┐
│     is22        │                    │  Paraclate APP  │
│ (Paraclate      │  ─── HTTPS ───>    │   (mobes2.0)    │
│    Server)      │  <── Response ──   │                 │
└─────────────────┘                    └─────────────────┘
        │                                      │
        │ Summary送信                          │ Ingest処理
        │ Event送信                            │ Chat API
        │ 設定取得                             │ 設定配信
        │ 接続テスト                           │ Pub/Sub通知
```

---

## タスク一覧

### T4-1: client.rs HTTPクライアント実装

**状態**: ✅ COMPLETED (2026-01-10)
**優先度**: P0（ブロッカー）
**見積もり規模**: L

**内容**:
- `ParaclateClient`クラス実装
- Summary/Event/GrandSummary送信

**主要メソッド**:
- `send_summary()`: Summary送信
- `send_grand_summary()`: GrandSummary送信
- `send_emergency()`: Emergency Event送信
- `test_connection()`: 接続テスト

**成果物**:
- `src/paraclate_client/client.rs`
- `src/paraclate_client/types.rs`

**検証方法**:
- モック接続テスト
- 実API接続テスト

---

### T4-2: lacisOath認証ヘッダ実装

**状態**: ✅ COMPLETED (2026-01-10)
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- lacisOath認証情報をリクエストに付与
- X-Lacis-* ヘッダ対応（CONSISTENCY_CHECK P0-3対応）

**認証ヘッダ形式**:
```
X-Lacis-ID: {lacis_id}
X-Lacis-TID: {tid}
X-Lacis-CIC: {cic}
```

**成果物**:
- 認証ヘッダ付与ロジック

**検証方法**:
- ヘッダ正確性テスト

---

### T4-3: snapshot連携（LacisFiles）

**状態**: ✅ COMPLETED (2026-01-11)
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- Event送信時のsnapshot付与
- snapshotRef（storagePath）応答処理
- detection_logs.snapshot_url保存

**snapshot形式**（ddreview_2 P1-2確定）:
- 署名URL禁止、storagePath（恒久参照子）を使用
- 形式: `lacisFiles/{tid}/{fileId}.{ext}`

**Event Response処理**:
```json
{
  "ok": true,
  "eventId": "det_12345",
  "snapshot": {
    "tid": "T2025...",
    "fileId": "snapshot_det_12345",
    "storagePath": "lacisFiles/T2025.../snapshot_det_12345.jpg"
  }
}
```

**成果物**:
- `src/paraclate_client/types.rs`: EventPayload, EventResponse, SnapshotRef, EventSendResult ✅
- `src/paraclate_client/client.rs`: send_event_with_snapshot() ✅
- `src/detection_log_service/mod.rs`: update_cloud_path() ✅

**検証方法**:
- snapshot付きEvent送信テスト
- storagePath保存確認

---

### T4-4: enqueuer.rs 送信キュー管理

**状態**: ✅ COMPLETED (2026-01-10)
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- `paraclate_send_queue`テーブル管理
- 送信失敗時のキュー保持
- リトライ管理

**成果物**:
- `src/paraclate_client/repository.rs` ✅
- `migrations/023_paraclate_client.sql` ✅

**マイグレーションSQL**:
```sql
CREATE TABLE IF NOT EXISTS paraclate_send_queue (
    queue_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32) NOT NULL,
    payload_type ENUM('summary', 'grand_summary', 'event', 'emergency') NOT NULL,
    payload JSON NOT NULL,
    status ENUM('pending', 'sending', 'sent', 'failed') DEFAULT 'pending',
    retry_count INT DEFAULT 0,
    max_retries INT DEFAULT 3,
    last_error TEXT,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    sent_at DATETIME(3),
    INDEX idx_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

**検証方法**:
- キュー追加・取得テスト
- リトライテスト

---

### T4-5: config_sync.rs 設定同期

**状態**: ✅ COMPLETED (2026-01-10)
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- `ConfigSyncService`実装
- mobes2.0から設定取得（GET /config/{tid}）
- ローカルキャッシュ（paraclate_config）管理

**主要メソッド**:
- `sync_from_mobes()`: SSoTから設定取得
- `push_to_mobes()`: ローカル変更を送信（慎重に）

**成果物**:
- `src/paraclate_client/config_sync.rs`

**検証方法**:
- 設定同期テスト
- タイムスタンプ比較ロジック

---

### T4-6: リトライ・offline対応

**状態**: ✅ COMPLETED (2026-01-10)
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- `SendQueueProcessor`バックグラウンド処理
- 指数バックオフリトライ
- ネットワーク断時の保持

**リトライ戦略**:
- 10秒間隔でキュー処理
- 指数バックオフ（5秒→10秒→20秒...最大5分）
- 3回失敗でfailed状態

**成果物**:
- send_queue.rs内のprocessor実装

**検証方法**:
- ネットワーク断→復旧→再送信テスト

---

### T4-7: Pub/Sub受信フロー（設定変更通知）

**状態**: ✅ COMPLETED (2026-01-11)
**優先度**: P1（品質改善）
**見積もり規模**: M

**内容**:
- `paraclate-config-updates` Topic受信
- 通知受信→GET /config/{tid}でSSoTからpull

**Pub/Sub設計**（ddreview_2 P1-4確定）:
- Topic: `paraclate-config-updates`
- Payload: `{type, tid, fids[], updatedAt, actor}`
- config本体は配らない（通知のみ）

**実装内容**:
- `PubSubSubscriber` クラス実装
- Push Subscriptionエンドポイント (`POST /api/paraclate/pubsub/push`)
- 直接通知エンドポイント (`POST /api/paraclate/notify`)
- 通知タイプ: `config_update`, `config_delete`, `disconnect`, `force_sync`
- TID/FID検証によるセキュリティフィルタリング
- ConfigSyncService連携による設定同期

**成果物**:
- `src/paraclate_client/pubsub_subscriber.rs` ✅
- `src/web_api/paraclate_routes.rs` (handle_pubsub_push, handle_direct_notification追加) ✅

**検証方法**:
- 設定変更通知→同期テスト ✅
- Pub/Sub Push形式受信テスト ✅
- 直接通知受信テスト ✅

---

## API実装

### is22側内部API

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/paraclate/status` | GET | 接続状態取得 |
| `/api/paraclate/connect` | POST | 接続テスト実行 |
| `/api/paraclate/config` | GET | 現在の設定取得 |
| `/api/paraclate/config` | PUT | 設定更新 |
| `/api/paraclate/queue` | GET | 送信キュー一覧 |
| `/api/paraclate/queue/process` | POST | キュー手動処理 |
| `/api/paraclate/pubsub/push` | POST | Pub/Sub Push通知受信 (T4-7) |
| `/api/paraclate/notify` | POST | 直接通知受信 (T4-7) |

**成果物**:
- `src/web_api/paraclate_routes.rs`
- `src/paraclate_client/pubsub_subscriber.rs` (T4-7)

---

## Paraclate APP側API（呼び出し先）

| エンドポイント | メソッド | 用途 |
|---------------|---------|------|
| `/api/paraclate/ingest/summary` | POST | Summary受信 |
| `/api/paraclate/ingest/event` | POST | Event受信 |
| `/api/paraclate/config/{tid}` | GET | 設定取得 |
| `/api/paraclate/connect` | POST | 接続テスト |

**endpoint定義**（ddreview_2 P1-1確定）:
- `paraclate.endpoint`は末尾スラッシュなしのルートURL
- is22は `/ingest/summary` `/ingest/event` `/config/{tid}` `/connect` を付与

---

## テストチェックリスト

- [ ] T4-1: HTTPクライアント基本動作テスト
- [ ] T4-2: lacisOath認証ヘッダテスト
- [ ] T4-3: snapshot送信・応答処理テスト
- [ ] T4-4: 送信キューCRUDテスト
- [ ] T4-5: 設定同期テスト
- [ ] T4-6: リトライ・バックオフテスト
- [ ] T4-6: offline→復旧再送信テスト
- [ ] T4-7: Pub/Sub受信テスト

---

## E2E統合テスト（Phase完了時）

| テストID | 内容 | 確認項目 |
|---------|------|---------|
| E3 | Summary→Paraclate送信→BQ同期 | Phase 3,4,5 |
| E4 | Config変更→Pub/Sub→is22同期 | Phase 4 |
| E5 | Offline→再接続→キュー送信 | Phase 4 |

---

## 完了条件

1. Phase 1,2,3完了が前提
2. 全タスク（T4-1〜T4-7）が✅ COMPLETED
3. テストチェックリスト全項目パス
4. E3, E4, E5テスト実行可能

---

## Issue連携

**Phase Issue**: #117
**親Issue**: #113

全タスクは#117で一括管理。個別タスクのサブIssue化は必要に応じて実施。

---

## 更新履歴

| 日付 | 更新内容 |
|------|---------|
| 2026-01-10 | 初版作成 |
| 2026-01-10 | T4-1,T4-2,T4-4,T4-5,T4-6 COMPLETED |
| 2026-01-11 | T4-3 COMPLETED（snapshot連携実装）、P0タスク全完了 |
| 2026-01-11 | T4-7 COMPLETED（Pub/Sub受信フロー実装）、Phase 4全タスク完了 |
| 2026-01-11 | **Issue #119対応**: FidValidator実装 - テナント-FID所属検証をparaclate_routes, PubSubSubscriberに追加 |
