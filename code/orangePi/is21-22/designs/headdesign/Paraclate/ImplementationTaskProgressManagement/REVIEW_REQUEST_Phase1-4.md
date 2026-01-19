# Paraclate Phase 1-4 レビュー依頼

---

## 基本情報

| 項目 | 内容 |
|------|------|
| 依頼日 | 2026-01-11 |
| 実装担当 | Claude (AI) |
| レビュー対象 | Phase 1-4 (AraneaRegister, CameraRegistry, Summary, ParaclateClient) |
| 関連Issue | #113 (親), #114, #115, #116, #117 |
| 重大Issue | **#119** (P0-critical: テナント-FID所属検証未実装) |

---

## レビュー観点

### 1. セキュリティ (最優先)

- [ ] **Issue #119**: テナント-FID所属検証の欠如について確認
- [ ] LacisOath認証ヘッダの正確性
- [ ] Pub/Sub通知受信時のTID/FID検証

### 2. 設計準拠

- [ ] DD01-DD05との整合性
- [ ] SSoT原則（テナント=mobes/カメラ=is22/長期=BQ）の遵守
- [ ] camelCase/snake_case変換の一貫性

### 3. コード品質

- [ ] エラーハンドリング
- [ ] トレーシング/ログ出力
- [ ] 型安全性

### 4. 機能動作

- [ ] APIエンドポイントの動作確認
- [ ] DB永続化の正確性
- [ ] リトライ/オフライン対応

---

## レビュー対象ファイル

### Phase 1: AraneaRegister

| ファイル | 行数 | 優先度 |
|---------|-----|-------|
| [src/aranea_register/mod.rs](../../../../../is22/src/aranea_register/mod.rs) | 55 | 低 |
| [src/aranea_register/types.rs](../../../../../is22/src/aranea_register/types.rs) | ~200 | 中 |
| [src/aranea_register/service.rs](../../../../../is22/src/aranea_register/service.rs) | ~350 | **高** |
| [src/aranea_register/repository.rs](../../../../../is22/src/aranea_register/repository.rs) | ~150 | 中 |
| [src/aranea_register/lacis_id.rs](../../../../../is22/src/aranea_register/lacis_id.rs) | ~100 | 中 |
| [src/web_api/register_routes.rs](../../../../../is22/src/web_api/register_routes.rs) | ~100 | 低 |

### Phase 2: CameraRegistry

| ファイル | 行数 | 優先度 |
|---------|-----|-------|
| [src/camera_registry/mod.rs](../../../../../is22/src/camera_registry/mod.rs) | ~55 | 低 |
| [src/camera_registry/types.rs](../../../../../is22/src/camera_registry/types.rs) | ~150 | 中 |
| [src/camera_registry/service.rs](../../../../../is22/src/camera_registry/service.rs) | ~300 | 中 |
| [src/camera_registry/repository.rs](../../../../../is22/src/camera_registry/repository.rs) | ~200 | 中 |
| [src/camera_registry/context.rs](../../../../../is22/src/camera_registry/context.rs) | ~200 | 中 |
| [src/camera_registry/lacis_id.rs](../../../../../is22/src/camera_registry/lacis_id.rs) | ~80 | 低 |
| [src/camera_registry/ipcam_connector.rs](../../../../../is22/src/camera_registry/ipcam_connector.rs) | ~150 | 低 |

### Phase 3: Summary/GrandSummary

| ファイル | 行数 | 優先度 |
|---------|-----|-------|
| [src/summary_service/mod.rs](../../../../../is22/src/summary_service/mod.rs) | ~50 | 低 |
| [src/summary_service/types.rs](../../../../../is22/src/summary_service/types.rs) | ~300 | 中 |
| [src/summary_service/generator.rs](../../../../../is22/src/summary_service/generator.rs) | ~400 | **高** |
| [src/summary_service/grand_summary.rs](../../../../../is22/src/summary_service/grand_summary.rs) | ~250 | 中 |
| [src/summary_service/repository.rs](../../../../../is22/src/summary_service/repository.rs) | ~350 | 中 |
| [src/summary_service/scheduler.rs](../../../../../is22/src/summary_service/scheduler.rs) | ~200 | 中 |
| [src/summary_service/payload_builder.rs](../../../../../is22/src/summary_service/payload_builder.rs) | ~150 | 中 |
| [src/web_api/summary_routes.rs](../../../../../is22/src/web_api/summary_routes.rs) | ~200 | 低 |

### Phase 4: ParaclateClient

| ファイル | 行数 | 優先度 |
|---------|-----|-------|
| [src/paraclate_client/mod.rs](../../../../../is22/src/paraclate_client/mod.rs) | ~60 | 低 |
| [src/paraclate_client/types.rs](../../../../../is22/src/paraclate_client/types.rs) | ~400 | **高** |
| [src/paraclate_client/client.rs](../../../../../is22/src/paraclate_client/client.rs) | ~500 | **高** |
| [src/paraclate_client/config_sync.rs](../../../../../is22/src/paraclate_client/config_sync.rs) | ~300 | 中 |
| [src/paraclate_client/repository.rs](../../../../../is22/src/paraclate_client/repository.rs) | ~400 | 中 |
| [src/paraclate_client/pubsub_subscriber.rs](../../../../../is22/src/paraclate_client/pubsub_subscriber.rs) | ~250 | **高** |
| [src/web_api/paraclate_routes.rs](../../../../../is22/src/web_api/paraclate_routes.rs) | ~300 | 中 |

### 共通ファイル

| ファイル | 変更内容 | 優先度 |
|---------|---------|-------|
| [src/state.rs](../../../../../is22/src/state.rs) | AppState拡張 | **高** |
| [src/main.rs](../../../../../is22/src/main.rs) | 初期化順序 | 中 |
| [src/lib.rs](../../../../../is22/src/lib.rs) | モジュール公開 | 低 |

### マイグレーション

| ファイル | 内容 |
|---------|------|
| [migrations/020_aranea_registration.sql](../../../../../is22/migrations/020_aranea_registration.sql) | デバイス登録テーブル |
| [migrations/021_camera_registry.sql](../../../../../is22/migrations/021_camera_registry.sql) | カメラ拡張カラム |
| [migrations/022_summary_service.sql](../../../../../is22/migrations/022_summary_service.sql) | Summary/スケジュールテーブル |
| [migrations/023_paraclate_client.sql](../../../../../is22/migrations/023_paraclate_client.sql) | Paraclate設定/キューテーブル |

---

## 重点レビュー箇所

### 1. Issue #119 関連 (CRITICAL)

以下のファイルでテナント-FID所属検証が必要:

```rust
// src/paraclate_client/client.rs
// 各メソッド（send_summary, send_event等）でFID検証を追加すべき箇所

// src/paraclate_client/pubsub_subscriber.rs
// handle_push, handle_direct でFID検証を追加すべき箇所
impl PubSubSubscriber {
    pub async fn handle_push(&self, msg: PubSubPushMessage) -> Result<(), ParaclateError> {
        // TODO: FID所属検証
    }
}
```

### 2. LacisOath認証ヘッダ

```rust
// src/paraclate_client/client.rs
fn add_lacis_headers(&self, request: RequestBuilder, tid: &str, fid: &str) -> RequestBuilder {
    request
        .header("X-Lacis-ID", &self.lacis_id)
        .header("X-Lacis-TID", tid)
        .header("X-Lacis-CIC", &self.cic)
        // blessing は現在 Option<String> で未実装
}
```

### 3. serde camelCase変換

```rust
// src/paraclate_client/types.rs
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryPayload {
    pub lacis_oath: LacisOath,       // -> "lacisOath"
    pub summary_overview: SummaryOverview, // -> "summaryOverview"
    // ...
}
```

---

## 動作確認済みAPI

### Phase 1

```bash
# デバイス登録
POST http://192.168.125.246:8080/api/register/device
# 結果: CIC=605123取得済み

# 登録状態
GET http://192.168.125.246:8080/api/register/status
```

### Phase 3

```bash
# Summary生成
POST http://192.168.125.246:8080/api/summary/generate?tid=T2025120621041161827&fid=0150
# 結果: 100検出イベント、8カメラからSummary生成

# スケジュール取得
GET http://192.168.125.246:8080/api/reports/schedule/summary?tid=T2025120621041161827&fid=0150
```

### Phase 4

```bash
# 接続状態
GET http://192.168.125.246:8080/api/paraclate/status?tid=T2025120621041161827&fid=0150

# 直接通知テスト
POST http://192.168.125.246:8080/api/paraclate/notify
Content-Type: application/json
{"type":"config_update","tid":"T2025120621041161827","fids":["0150"],"updatedAt":"2026-01-11T10:00:00Z","actor":"test"}
```

---

## 関連ドキュメント

| ドキュメント | リンク |
|-------------|-------|
| 実装報告書 | [IMPLEMENTATION_REPORT_Phase1-4.md](./IMPLEMENTATION_REPORT_Phase1-4.md) |
| マスターインデックス | [MASTER_INDEX.md](./MASTER_INDEX.md) |
| Phase1タスク管理 | [Phase1_AraneaRegister.md](./Phase1_AraneaRegister.md) |
| Phase2タスク管理 | [Phase2_CameraRegistry.md](./Phase2_CameraRegistry.md) |
| Phase3タスク管理 | [Phase3_SummaryGrandSummary.md](./Phase3_SummaryGrandSummary.md) |
| Phase4タスク管理 | [Phase4_ParaclateClient.md](./Phase4_ParaclateClient.md) |

### 設計ドキュメント

| ドキュメント | リンク |
|-------------|-------|
| DD01_AraneaRegister | [../DetailedDesign/DD01_AraneaRegister.md](../DetailedDesign/DD01_AraneaRegister.md) |
| DD02_SummaryOverview | [../DetailedDesign/DD02_SummaryOverview.md](../DetailedDesign/DD02_SummaryOverview.md) |
| DD03_ParaclateClient | [../DetailedDesign/DD03_ParaclateClient.md](../DetailedDesign/DD03_ParaclateClient.md) |
| DD05_CameraRegistry | [../DetailedDesign/DD05_CameraRegistry.md](../DetailedDesign/DD05_CameraRegistry.md) |

---

## レビュー完了条件

1. [ ] 全ファイルのコードレビュー完了
2. [ ] Issue #119 の対応方針決定
3. [ ] セキュリティ観点の確認完了
4. [ ] 設計準拠の確認完了
5. [ ] レビューコメントへの対応完了

---

## 連絡事項

- Phase 5 (BqSyncService) はGCP BigQuery環境のセットアップ待ち
- Issue #119はPhase 4完了とは独立してP0-criticalとして対応必要
- 統合テスト（T3-8, T6-9）は別途実施予定

---

*本文書はParaclate Phase 1-4のレビュー依頼として作成されました。*
