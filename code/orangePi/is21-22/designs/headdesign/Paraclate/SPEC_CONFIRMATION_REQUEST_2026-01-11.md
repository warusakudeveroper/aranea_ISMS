# Paraclate仕様確認依頼書

作成日: 2026-01-11
作成者: IS22開発ライン
宛先: mobes2.0/ParaclateAPP開発ライン
目的: IS22-mobes2.0間の仕様整合確認

---

## 1. 背景

IS22側でParaclate連携の実装が進行中です。以下の機能について、mobes2.0側の仕様・実装状況との整合を確認したく、本書を作成しました。

### IS22側 実装完了項目

| 機能 | 状態 | 実装場所 |
|------|------|---------|
| AraneaRegister (デバイス登録) | ✅ 完了 | src/aranea_register/ |
| LacisOath認証 (Authorization形式) | ✅ 完了 | src/paraclate_client/types.rs |
| CameraRegistry (カメラ台帳) | ✅ 完了 | src/camera_registry/ |
| Summary/GrandSummary生成 | ✅ 完了 | src/summary_service/ |
| ParaclateClient (API送信) | ✅ 完了 | src/paraclate_client/ |
| BqSyncService | ✅ 完了 | src/bq_sync_service/ |
| FID管理 | ✅ 完了 | 2026-01-11追加 |

### E2Eテスト結果 (2026-01-11)

| API | URL | 結果 |
|-----|-----|------|
| Connect | paraclateconnect-vm44u3kpua-an.a.run.app | ✅ SUCCESS |
| IngestSummary | paraclateingestsummary-vm44u3kpua-an.a.run.app | ✅ SUCCESS |
| IngestEvent | paraclateingestevent-vm44u3kpua-an.a.run.app | ✅ SUCCESS |
| GetConfig | paraclategetconfig-vm44u3kpua-an.a.run.app | ✅ SUCCESS |

---

## 2. 仕様確認事項

### 2.1 LacisFiles連携

**Paraclate_DesignOverview.md記載**:
> 「イベントの画像の保管保存=LacisFilesで行う」

**IS22側実装状況**:
- `src/paraclate_client/client.rs` の `send_event_with_snapshot()` でmultipart/form-data送信実装済み
- snapshot添付時のレスポンスで `storage_path` (恒久参照子) を期待

**確認事項**:

| No | 項目 | 質問 |
|----|------|------|
| LF-1 | エンドポイント | IngestEvent APIでsnapshot添付時、LacisFilesへの保存は自動で行われるか？別APIが必要か？ |
| LF-2 | レスポンス形式 | snapshot保存成功時のレスポンス形式は以下で正しいか？ |
| LF-3 | 参照URL | 保存されたsnapshotの参照URL形式は？ |
| LF-4 | Retention | LacisFilesのRetention設定はParaclate設定と連動するか？ |

**IS22が期待するレスポンス形式**:
```json
{
  "ok": true,
  "eventId": "evt_xxxxx",
  "snapshot": {
    "storagePath": "lacisfiles://tid/fid/yyyy/mm/dd/evt_xxxxx.jpg",
    "uploadedAt": "2026-01-11T00:00:00Z"
  }
}
```

---

### 2.2 BQ連携・検出ログ同期

**Paraclate_DesignOverview.md記載**:
> 「最終的に検出ログの全てをBQにポストする(サマリー系を強化する方向)」

**IS22側実装状況**:
- `src/bq_sync_service/` でバッチ同期実装済み
- detection_logs, ai_summary_cache テーブルの同期対応
- synced_to_bq フラグ管理、冪等性保証実装済み

**確認事項**:

| No | 項目 | 質問 |
|----|------|------|
| BQ-1 | テーブル構造 | IS22からBQへ同期するテーブル構造の仕様書はあるか？ |
| BQ-2 | 認証方式 | BQ書き込みの認証方式は？（サービスアカウント？LacisOath経由？） |
| BQ-3 | データセット | BQのデータセット名、プロジェクトIDは？ |
| BQ-4 | 同期タイミング | リアルタイム同期 or バッチ同期？推奨間隔は？ |

**IS22側のBQ同期対象データ**:
```
detection_logs:
  - log_id, camera_id, camera_lacis_id
  - captured_at, primary_event, severity
  - count_hint, snapshot_path
  - synced_to_bq (boolean)

ai_summary_cache:
  - summary_id, summary_type
  - period_start, period_end
  - payload (JSON), sent_to_paraclate
  - synced_to_bq (boolean)
```

---

### 2.3 AIアシスタント・チャットボット連携

**Paraclate_DesignOverview.md記載**:
> 「AIアシスタントタブからの質問に関してもParaclate APPからのレスポンスでチャットボット機能を行う」
> 「検出特徴の人物のカメラ間での移動などを横断的に把握する」
> 「カメラ不調などの傾向も把握する」
> 「過去の記録を参照する、過去の記録範囲を対話的にユーザーと会話する」

**IS22側実装状況**:
- AIアシスタントUIプレースホルダー実装済み（SettingsModal.tsx）
- 検出傾向調整サジェスト機能実装済み（EventLogPane.tsx）
- OverdetectionAnalyzer実装済み（過検出分析）

**確認事項**:

| No | 項目 | 質問 |
|----|------|------|
| AI-1 | APIエンドポイント | AIアシスタントへの質問送信APIは？ |
| AI-2 | リクエスト形式 | 質問送信時のペイロード形式は？ |
| AI-3 | レスポンス形式 | AI応答のストリーミング or 一括返却？形式は？ |
| AI-4 | コンテキスト送信 | IS22からどのコンテキストを送信すべきか？ |

**IS22が送信可能なコンテキスト**:
```json
{
  "facilityContext": {
    "tid": "T2025...",
    "fid": "0150",
    "cameras": [...]
  },
  "recentDetections": [...],
  "cameraStatus": {...},
  "summaryHistory": [...]
}
```

---

### 2.4 カメラ不調検知

**Paraclate_DesignOverview.md記載**:
> 「カメラ不調などの傾向も把握する」

**IS22側実装状況**:
- CameraStatusTracker実装済み（src/camera_status_tracker.rs）
- 以下のステータスを追跡:
  - online/offline/error
  - lastSeenAt, consecutiveErrors
  - streamHealth (fps, latency)

**確認事項**:

| No | 項目 | 質問 |
|----|------|------|
| CM-1 | 報告先 | カメラ不調情報の報告先APIは？IngestEvent？専用API？ |
| CM-2 | 報告形式 | カメラ不調イベントのペイロード形式は？ |
| CM-3 | 閾値設定 | 不調判定の閾値はIS22側？mobes側で設定？ |

---

### 2.5 CelestialGlobe連携

**Paraclate_DesignOverview.md記載**:
> 「ipアドレスとMACについてはCelestialGlobeからも参照される」

**確認事項**:

| No | 項目 | 質問 |
|----|------|------|
| CG-1 | 連携方式 | IS22→CelestialGlobe？CelestialGlobe→IS22？双方向？ |
| CG-2 | データ項目 | 連携するデータ項目は？（IP, MAC, hostname, ...） |
| CG-3 | API仕様 | CelestialGlobe APIの仕様書はあるか？ |

---

### 2.6 is22→is21アクティベート

**Paraclate_DesignOverview.md記載**:
> 「is22側からis21のサーバーアドレスのエンドポイントにアクセスしてis22のtid,lacisID,cicでアクティベートする」

**IS22側実装状況**:
- IS21管理UIプレースホルダー実装済み
- IS21サーバーアドレス設定可能

**確認事項**:

| No | 項目 | 質問 |
|----|------|------|
| IS21-1 | エンドポイント | is21のアクティベートAPIエンドポイントは？ |
| IS21-2 | リクエスト形式 | アクティベートリクエストの形式は？ |
| IS21-3 | レスポンス形式 | アクティベート成功時のレスポンスは？ |
| IS21-4 | 認証 | LacisOath認証で良いか？ |

**IS22が送信予定のアクティベートリクエスト**:
```json
{
  "lacisOath": {
    "lacisId": "3022...",
    "tid": "T2025...",
    "cic": "605123"
  },
  "is22Info": {
    "endpoint": "http://192.168.125.246:8080",
    "version": "0.1.0"
  }
}
```

---

## 3. IS22側の実装予定

上記確認事項の回答を受け、以下の順序で実装を進めます：

| 優先度 | 機能 | 依存 |
|--------|------|------|
| 1 | is22→is21アクティベート | IS21-1〜4回答後 |
| 2 | LacisFiles連携 | LF-1〜4回答後 |
| 3 | カメラ不調報告 | CM-1〜3回答後 |
| 4 | AIアシスタント連携 | AI-1〜4回答後 |
| 5 | CelestialGlobe連携 | CG-1〜3回答後 |

---

## 4. 連絡先

- IS22開発環境: 192.168.125.246:3000 (Paraclate - is22 CamServer)
- IS21開発環境: 192.168.3.240:9000 (ParaclateEdge)
- リポジトリ: https://github.com/anthropics/aranea_ISMS

---

## 5. 添付資料

- [MASTER_INDEX.md](./ImplementationTaskProgressManagement/MASTER_INDEX.md) - 実装進捗管理
- [GAP_ANALYSIS_2026-01-11.md](./ImplementationTaskProgressManagement/GAP_ANALYSIS_2026-01-11.md) - 抜け漏れ分析
- [DD03_ParaclateClient.md](./DetailedDesign/DD03_ParaclateClient.md) - ParaclateClient詳細設計

---

**回答期限**: 可能な限り早急にお願いします
**回答方法**: 本ドキュメントへの追記、またはGitHub Issueでの回答
