# Paraclate実装タスク進捗管理 マスターインデックス

作成日: 2026-01-10
最終更新: 2026-01-10
バージョン: 1.0.0

---

## 概要

本ディレクトリはParaclate（旧MobesAIcamControlTower）の実装タスクを管理するためのドキュメント群を格納する。
設計ドキュメント（DD01-DD05）に基づき、MECEに整理されたタスクをフェーズごとに管理する。

### 設計ドキュメントとの対応

| Phase | 詳細設計 | モジュール | 依存関係 | GitHub Issue |
|-------|---------|-----------|---------|-------------|
| Phase 1 | DD01 | AraneaRegister | なし（基盤） | #114 |
| Phase 2 | DD05 | CameraRegistry | Phase 1 | #115 |
| Phase 3 | DD02 | Summary/GrandSummary | Phase 2 | #116 |
| Phase 4 | DD03 | ParaclateClient | Phase 1,2,3 | #117 |
| Phase 5 | DD04 | BqSyncService | Phase 3,4 | #118 |

**親Issue**: #113

---

## ドキュメント一覧

| ファイル | 内容 | タスク数 |
|---------|------|---------|
| [Phase1_AraneaRegister.md](./Phase1_AraneaRegister.md) | デバイス登録・認証基盤 | 7 |
| [Phase2_CameraRegistry.md](./Phase2_CameraRegistry.md) | カメラ台帳管理 | 7 |
| [Phase3_SummaryGrandSummary.md](./Phase3_SummaryGrandSummary.md) | AI要約生成・統合 | 8 |
| [Phase4_ParaclateClient.md](./Phase4_ParaclateClient.md) | mobes2.0連携クライアント | 7 |
| [Phase5_BqSyncService.md](./Phase5_BqSyncService.md) | BigQuery同期 | 7 |
| [COMPLETENESS_CHECK.md](./COMPLETENESS_CHECK.md) | 完全性チェック結果 | - |

---

## 全体進捗サマリー

### ステータス定義

| ステータス | 記号 | 意味 |
|-----------|------|------|
| NOT_STARTED | ⬜ | 未着手 |
| IN_PROGRESS | 🔄 | 作業中 |
| REVIEW | 🔍 | レビュー待ち |
| COMPLETED | ✅ | 完了 |
| BLOCKED | 🚫 | ブロック中 |

### Phase別進捗

| Phase | 進捗 | 完了タスク | 全タスク | 進捗率 |
|-------|------|-----------|---------|-------|
| Phase 1 | ⬜ | 0 | 7 | 0% |
| Phase 2 | ⬜ | 0 | 7 | 0% |
| Phase 3 | ⬜ | 0 | 8 | 0% |
| Phase 4 | ⬜ | 0 | 7 | 0% |
| Phase 5 | ⬜ | 0 | 7 | 0% |
| **合計** | | **0** | **36** | **0%** |

---

## 要件定義との対応（トレーサビリティ）

### 機能要件（FR）

| 要件ID | 要件名 | 対応Phase | 対応タスク |
|--------|-------|----------|-----------|
| FR-01 | デバイス登録（Paraclate対応） | Phase 1 | T1-1〜T1-7 |
| FR-02 | lacisOath認証 | Phase 1 | T1-4, T1-5 |
| FR-03 | カメラ台帳管理 | Phase 2 | T2-1〜T2-7 |
| FR-04 | 検出ログ記録 | Phase 2 | T2-4, T2-5 |
| FR-05 | Summary生成 | Phase 3 | T3-1〜T3-5 |
| FR-06 | GrandSummary統合 | Phase 3 | T3-6〜T3-8 |
| FR-07 | Paraclate Ingest API | Phase 4 | T4-1〜T4-4 |
| FR-08 | Config同期 | Phase 4 | T4-5〜T4-7 |
| FR-09 | BigQuery同期 | Phase 5 | T5-1〜T5-7 |
| FR-10 | スナップショット連携 | Phase 4 | T4-3 |
| FR-11 | 設定変更通知（Pub/Sub） | Phase 4 | T4-7 |
| FR-12 | 保持期間管理 | Phase 5 | T5-6 |
| FR-13 | 越境アクセス（blessing） | Phase 1 | T1-5 |

### 非機能要件（NFR）

| 要件ID | 要件名 | 対応Phase | 対応タスク |
|--------|-------|----------|-----------|
| NFR-01 | SSoT分担（テナント=mobes/カメラ=is22/長期=BQ） | 全Phase | 各Phase参照 |
| NFR-02 | 冗長化対応（is21/is22 2台構成） | Phase 1 | T1-7 |
| NFR-03 | パフォーマンス要件（同期遅延<5秒） | Phase 4,5 | T4-6, T5-3 |
| NFR-04 | セキュリティ（lacisOath必須） | Phase 1,4 | T1-4, T4-2 |
| NFR-05 | 可用性（offline耐性） | Phase 4 | T4-6 |
| NFR-06 | 監査ログ | Phase 1,5 | T1-6, T5-7 |

---

## 依存関係図

```
Phase 1: AraneaRegister
    │
    ├── T1-1: ProductType定義
    ├── T1-2: データモデル設計
    ├── T1-3: registration.rs
    ├── T1-4: lacis_oath.rs
    ├── T1-5: blessing.rs
    ├── T1-6: 監査ログ
    └── T1-7: 冗長化対応
           │
           ▼
Phase 2: CameraRegistry ──────────────────┐
    │                                      │
    ├── T2-1: カメラ台帳スキーマ           │
    ├── T2-2: camera_registry.rs           │
    ├── T2-3: RTSP管理連携                 │
    ├── T2-4: detection_logs.rs            │
    ├── T2-5: ログ検索API                  │
    ├── T2-6: カメラステータス管理          │
    └── T2-7: 統合テスト                   │
           │                               │
           ▼                               │
Phase 3: Summary/GrandSummary             │
    │                                      │
    ├── T3-1: SummaryOverview設計          │
    ├── T3-2: summary_generator.rs         │
    ├── T3-3: ai_summary_cache             │
    ├── T3-4: 定時実行スケジューラ          │
    ├── T3-5: Summary API                  │
    ├── T3-6: GrandSummary設計             │
    ├── T3-7: grand_summary.rs             │
    └── T3-8: Summary→GrandSummary統合     │
           │                               │
           ▼                               ▼
Phase 4: ParaclateClient ◄─────────────────
    │
    ├── T4-1: client.rs（HTTP送信）
    ├── T4-2: lacisOath認証ヘッダ
    ├── T4-3: snapshot連携
    ├── T4-4: enqueuer.rs
    ├── T4-5: config_sync.rs
    ├── T4-6: リトライ・offline対応
    └── T4-7: Pub/Sub受信
           │
           ▼
Phase 5: BqSyncService
    │
    ├── T5-1: BQテーブル定義
    ├── T5-2: bq_sync.rs
    ├── T5-3: バッチ同期
    ├── T5-4: synced_to_bq管理
    ├── T5-5: 冪等性保証
    ├── T5-6: retention連携
    └── T5-7: 同期ログ・監視
```

---

## クリティカルパス

```
T1-1 → T1-2 → T1-3 → T2-1 → T2-2 → T3-1 → T3-2 → T4-1 → T4-4 → T5-1 → T5-2
 │                      │                    │
 └── T1-4 ──────────────┴── T4-2 ────────────┘
     (lacisOath)           (認証ヘッダ)
```

---

## マイルストーン

| マイルストーン | Phase | 完了条件 |
|--------------|-------|---------|
| M1: 基盤完成 | Phase 1完了 | デバイス登録・認証が動作 |
| M2: カメラ台帳 | Phase 2完了 | カメラ登録・検出ログ記録が動作 |
| M3: AI要約 | Phase 3完了 | Summary/GrandSummary生成が動作 |
| M4: mobes連携 | Phase 4完了 | Ingest API送信・Config同期が動作 |
| M5: BQ連携 | Phase 5完了 | BigQuery同期が動作 |
| M6: E2E完了 | E1-E7完了 | 全E2Eテストパス |

---

## E2Eテストチェックリスト

| テストID | テスト内容 | 対応Phase |
|---------|----------|----------|
| E1 | デバイス登録→台帳反映 | Phase 1,2 |
| E2 | カメラ検出→ログ記録→Summary生成 | Phase 2,3 |
| E3 | Summary→Paraclate送信→BQ同期 | Phase 3,4,5 |
| E4 | Config変更→Pub/Sub→is22同期 | Phase 4 |
| E5 | Offline→再接続→キュー送信 | Phase 4 |
| E6 | 越境アクセス（blessing）テスト | Phase 1,4 |
| E7 | Retention実行→BQ/ローカル削除 | Phase 5 |

---

## 関連ドキュメント

### 設計ドキュメント（../DetailedDesign/）

- [DD01_AraneaRegister.md](../DetailedDesign/DD01_AraneaRegister.md)
- [DD02_SummaryOverview.md](../DetailedDesign/DD02_SummaryOverview.md)
- [DD03_ParaclateClient.md](../DetailedDesign/DD03_ParaclateClient.md)
- [DD04_BqSyncService.md](../DetailedDesign/DD04_BqSyncService.md)
- [DD05_CameraRegistry.md](../DetailedDesign/DD05_CameraRegistry.md)
- [CONSISTENCY_CHECK.md](../DetailedDesign/CONSISTENCY_CHECK.md)

### 上位ドキュメント（../）

- [Paraclate_DesignOverview.md](../Paraclate_DesignOverview.md)
- [Paraclate_RequirementsDefinition.md](../Paraclate_RequirementsDefinition.md)
- [Paraclate_BasicDesign.md](../Paraclate_BasicDesign.md)

---

## 更新履歴

| 日付 | バージョン | 更新内容 | 更新者 |
|------|-----------|---------|-------|
| 2026-01-10 | 1.0.0 | 初版作成 | is22設計担当 |
