# GoogleDevices (SDM/Nest統合) タスクインデックス

## 基本情報

| 項目 | 内容 |
|------|------|
| プロジェクト | is22 RTSPカメラ総合管理サーバー |
| 機能名 | Google Nest Doorbell SDM統合 |
| 作成日 | 2026-01-07 |
| 依存完了 | Camscan機能 (#80-#90 完了済み、commit d655292) |
| 関連設計書 | GD-01〜GD-05 |

---

## 前提条件（実装開始前チェック）

- [x] Camscan機能実装完了
- [ ] go2rtc v1.9.9+ 確認（Nest対応必須）
- [ ] SDM環境準備手順完了（GD-02参照）
- [ ] 実機テスト環境準備

---

## 依存関係図

```
[Phase 0: 前提確認]
     │
     ▼
[Phase 1: データベース] ─────────────────────┐
     │                                       │
     ▼                                       │
[Phase 2: バックエンド基盤]                   │
     │                                       │
     ├──────────────────────────────────────┤
     ▼                                       ▼
[Phase 3: API実装]                [Phase 4: go2rtc連携]
     │                                       │
     └──────────────────┬───────────────────┘
                        ▼
                [Phase 5: フロントエンド]
                        │
                        ▼
                [Phase 6: 統合・検証]
                        │
                        ▼
                [Phase 7: Camscan導線連携]
```

---

## 実装タスク一覧（MECE）

### Phase 1: データベース・型定義

| ID | タスク | 詳細 | 依存 | 見積 |
|----|--------|------|------|------|
| T1-1 | マイグレーション作成 | `migrations/018_sdm_integration.sql`: cameras テーブルに `sdm_device_id`, `sdm_structure`, `sdm_traits` 追加 | - | S |
| T1-2 | Camera構造体更新 | `src/config_store/types.rs`: Camera構造体にSDMフィールド追加 | T1-1 | S |
| T1-3 | CAMERA_COLUMNS更新 | `src/config_store/repository.rs`: SELECT/UPDATE bind追加 | T1-2 | S |
| T1-4 | sdm_config型定義 | `src/sdm_integration/types.rs`: SdmConfig, SdmDevice, SdmStatus型定義 | - | S |

### Phase 2: バックエンド基盤モジュール

| ID | タスク | 詳細 | 依存 | 見積 |
|----|--------|------|------|------|
| T2-1 | sdm_integrationモジュール新設 | `src/sdm_integration/mod.rs`: モジュール構造作成 | T1-4 | S |
| T2-2 | config: load/save/validate | settings repository経由でsdm_config読み書き | T2-1 | M |
| T2-3 | token: auth_code→refresh_token | Google OAuth token endpoint呼び出し、指数バックオフ | T2-2 | M |
| T2-4 | token: refresh_token→access_token | access_token取得、エラーハンドリング | T2-3 | S |
| T2-5 | devices: SDM devices.list | SDM API呼び出し、正規化、quota保護(1分/1回) | T2-4 | M |
| T2-6 | 監査ログ実装 | client_secret/refresh_tokenマスク、操作記録 | T2-2 | S |

### Phase 3: APIエンドポイント実装

| ID | タスク | 詳細 | 依存 | 見積 |
|----|--------|------|------|------|
| T3-1 | GET /api/sdm/config | 設定状態取得（secrets非表示） | T2-2 | S |
| T3-2 | PUT /api/sdm/config | 設定保存、空文字は変更なし | T2-2 | S |
| T3-3 | POST /api/sdm/exchange-code | auth_code→refresh_token交換 | T2-3 | M |
| T3-4 | GET /api/sdm/devices | デバイス一覧取得、正規化返却 | T2-5 | S |
| T3-5 | POST /api/sdm/devices/:id/register | カメラ登録（cameras INSERT + go2rtc + cache refresh） | T2-5, T4-2 | L |
| T3-6 | POST /api/sdm/reconcile | 手動再同期トリガ | T4-3 | S |
| T3-7 | DELETE /api/sdm/devices/:id | カメラ無効化 + go2rtc削除 | T3-5 | M |
| T3-8 | GET /api/sdm/test/go2rtc-version | go2rtcバージョン確認 | T4-1 | S |
| T3-9 | GET /api/sdm/test/token | access_token取得テスト | T2-4 | S |
| T3-10 | GET /api/sdm/test/device/:id | 個別デバイス疎通テスト | T2-5 | S |
| T3-11 | RBAC/CSRF適用 | /api/sdm/* 管理者限定、CSRF必須 | T3-1〜T3-10 | S |

### Phase 4: go2rtc連携・Snapshot

| ID | タスク | 詳細 | 依存 | 見積 |
|----|--------|------|------|------|
| T4-1 | StreamGateway: Nestソース生成 | `nest://{device_id}?project_id=...` 形式の文字列組み立て | T2-2 | M |
| T4-2 | StreamGateway: add_source拡張 | Nestカメラ用ソース追加、エラーハンドリング | T4-1 | M |
| T4-3 | Reconcileジョブ実装 | 起動時 + 5分間隔で desired-state と go2rtc 実状態を同期 | T4-2 | M |
| T4-4 | snapshot_url設定 | 登録時に go2rtc frame.jpeg URLを設定 | T4-2 | S |
| T4-5 | PollingOrchestrator確認 | HTTP fallbackでNestカメラも巡回取得されることを確認 | T4-4 | S |

### Phase 5: フロントエンド実装

| ID | タスク | 詳細 | 依存 | 見積 |
|----|--------|------|------|------|
| T5-1 | SystemSettingsModal新設 | Dialog + Tabs構造、Google/Nestタブ | T3-1 | M |
| T5-2 | App.tsx Settings連携 | ヘッダ右上SettingsボタンにonClick追加 | T5-1 | S |
| T5-3 | ウィザード Step1: 概要・必要なもの | チェックリスト + フールプルーフ確認 | T5-1 | S |
| T5-4 | ウィザード Step2: SDM設定入力 | フォーム + PUT /api/sdm/config | T5-3, T3-2 | M |
| T5-5 | ウィザード Step3: 認可コード取得 | 認可URL生成 + POST /api/sdm/exchange-code | T5-4, T3-3 | M |
| T5-6 | ウィザード Step4: デバイス一覧取得 | GET /api/sdm/devices + テーブル表示 + 空リストチェックリスト | T5-5, T3-4 | M |
| T5-7 | ウィザード Step5: 登録ダイアログ | camera_id/name/location/fid/tid入力 + POST register | T5-6, T3-5 | L |
| T5-8 | ウィザード Step6: 接続確認 | snapshotプレビュー + 成功/失敗表示 | T5-7 | M |
| T5-9 | エラーハンドリング | 401再認可、go2rtc失敗、タイムアウト | T5-4〜T5-8 | M |
| T5-10 | 高度な設定パネル | go2rtcバージョン確認、tokenテスト、疎通テスト | T3-8〜T3-10 | M |
| T5-11 | ヘルプパネル | よくある詰まりと対処を右カラムに表示 | T5-1 | S |
| T5-12 | 状態管理(Zustand/Query) | sdm_config状態の取得・更新・キャッシュ | T5-1 | M |

### Phase 6: 統合テスト・検証

| ID | タスク | 詳細 | 依存 | 見積 |
|----|--------|------|------|------|
| T6-1 | バックエンド単体テスト | sdm_config validation、token交換分岐、RBAC/CSRFエラー | T3-11 | M |
| T6-2 | バックエンド統合テスト | API全体フロー（mock使用）、go2rtc stub | T6-1 | L |
| T6-3 | フロントエンドコンポーネントテスト | フォームバリデーション、状態表示分岐 | T5-12 | M |
| T6-4 | E2Eテスト（Chrome） | ウィザード全ステップ通し操作 | T6-2, T6-3 | L |
| T6-5 | 実機検証 | 実Nest Doorbell + SDM環境でのテスト | T6-4 | L |
| T6-6 | 長時間運用テスト | go2rtc再起動復旧、reconcile動作確認 | T6-5 | M |
| T6-7 | secretsマスク確認 | ログ・ネットワーク・DevToolsで秘密値が出ないこと | T6-4 | S |

### Phase 7: Camscan導線連携（後続）

| ID | タスク | 詳細 | 依存 | 見積 |
|----|--------|------|------|------|
| T7-1 | ScannedDevice型拡張 | `suggested_action`, `discovery_path`, `sdm_hint`追加 | T6-5 | M |
| T7-2 | スコアリング補正 | Google OUI + 443/8443のみ→SdmRegister分岐 | T7-1 | M |
| T7-3 | API出力追加 | /api/ipcamscan/devices に新フィールド追加 | T7-2 | S |
| T7-4 | ScanModal導線追加 | 「SDM登録へ」ボタン→SystemSettingsModal連携 | T7-3, T5-1 | M |
| T7-5 | 退行テスト | 既存Tapo/VIGI検出が変化しないこと | T7-4 | M |

---

## タスクサイズ定義

| サイズ | 目安 |
|--------|------|
| S | 1-2ファイル変更、単純な追加 |
| M | 3-5ファイル変更、中程度の複雑さ |
| L | 6+ファイル変更、複雑なロジック |

---

## 実装順序（推奨）

```
Week 1: Phase 1-2 (データベース・バックエンド基盤)
  T1-1 → T1-2 → T1-3 → T1-4
  T2-1 → T2-2 → T2-3 → T2-4 → T2-5 → T2-6

Week 2: Phase 3-4 (API・go2rtc連携)
  T3-1 → T3-2 → T3-3 → T3-4 → T3-5 → T3-6 → T3-7 → T3-8〜T3-11
  T4-1 → T4-2 → T4-3 → T4-4 → T4-5

Week 3: Phase 5 (フロントエンド)
  T5-1 → T5-2 → T5-3 → T5-4 → T5-5 → T5-6 → T5-7 → T5-8 → T5-9 → T5-10 → T5-11 → T5-12

Week 4: Phase 6-7 (テスト・Camscan連携)
  T6-1 → T6-2 → T6-3 → T6-4 → T6-5 → T6-6 → T6-7
  T7-1 → T7-2 → T7-3 → T7-4 → T7-5
```

---

## Issue登録状況

各PhaseをGitHub Issueとして登録済み:
- **#91**: Phase 1-2 SDMバックエンド基盤 ✅
- **#92**: Phase 3 SDM APIエンドポイント ✅
- **#93**: Phase 4 go2rtc連携・Reconcile ✅
- **#94**: Phase 5 設定モーダル・ウィザードUI ✅
- **#95**: Phase 6 統合テスト・検証 ✅
- **#96**: Phase 7 Camscan導線連携 ✅

---

## SSoT / 責務境界（再確認）

| 領域 | SSoT | 責務 |
|------|------|------|
| カメラ台帳 | `cameras` テーブル | family=nest, discovery_method=sdm, sdm_device_id |
| SDM設定 | `settings.sdm_config` | project/client/secrets/refresh_token |
| ストリーム | go2rtc / ConfigStore | StreamGateway経由で登録・再同期 |
| 静止画 | SnapshotService | camera.snapshot_url (go2rtc frame.jpeg) |
| 設定UI | SystemSettingsModal | SDM設定/認可/デバイス一覧/登録のプレゼンのみ |

---

## アンアンビギュアス確認

- [x] 全タスクに明確なID、詳細、依存関係を付与
- [x] 実装順序を依存関係に基づき定義
- [x] SSoT・責務境界を明記
- [x] テストタスク（Phase 6-7）を明示的に含む
- [x] Camscan完了を前提条件として明記

## MECE確認

- [x] データベース / バックエンド基盤 / API / go2rtc連携 / フロントエンド / テスト / Camscan連携 で領域網羅
- [x] 各Phase内のタスクは重複なく分割
- [x] 設計書（GD-01〜GD-05）の全要件をタスクにマップ済み

---

## 参照ドキュメント

- [GD-01] GoogleDevices_introduction_BasicDesign.md
- [GD-02] GoogleDevices_introduction_Environment.md
- [GD-03] GoogleDevices_introduction_DetailedDesign.md
- [GD-04] GoogleDevices_camscan_alignment.md
- [GD-05] GoogleDevices_settings_wizard_spec.md
- [GD-06] GoogleDevices_review.md
- [The_golden_rules.md](/The_golden_rules.md)
