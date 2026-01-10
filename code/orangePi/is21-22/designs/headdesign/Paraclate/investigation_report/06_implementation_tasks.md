# 実装タスク一覧（依存順）

作成日: 2026-01-10

## 1. 概要

Paraclate実装フェーズにおけるis21/22開発ラインの実装タスクを依存関係順に整理する。
優先順位は依存関係と実装順によってのみ定義（The_golden_rules.md #7）。

---

## 2. フェーズ分類

### Phase A: 認証基盤（前提条件）
### Phase B: データ基盤（Summary/BQ）
### Phase C: Paraclate連携
### Phase D: is21管理UI

---

## 3. Phase A: 認証基盤

**依存**: なし（最優先）

### A-1: AraneaRegister Rustモジュール作成

| 項目 | 内容 |
|------|------|
| 作業内容 | ESP32 AraneaRegister.cppのRust移植 |
| 新規ファイル | src/aranea_register/mod.rs, types.rs, lacis_id.rs, gate_client.rs |
| 参照実装 | code/ESP32/global/src/AraneaRegister.cpp |
| テスト | araneaDeviceGateへの登録テスト |

### A-2: DBスキーマ追加

| 項目 | 内容 |
|------|------|
| 作業内容 | aranea_registrationテーブル作成 |
| マイグレーション | 020_aranea_registration.sql |
| カラム | lacis_id, tid, cic, state_endpoint, mqtt_endpoint |

### A-3: 登録APIエンドポイント

| 項目 | 内容 |
|------|------|
| 作業内容 | /api/register/* エンドポイント実装 |
| エンドポイント | POST /api/register/device, GET /api/register/status, DELETE /api/register |

### A-4: テナント情報移行

| 項目 | 内容 |
|------|------|
| 作業内容 | 市山水産 → mijeo.inc TIDへ移行 |
| 対象 | is21 main.py DEFAULT_TID、is22設定 |

---

## 4. Phase B: データ基盤

**依存**: Phase A完了

### B-1: Summary生成サービス

| 項目 | 内容 |
|------|------|
| 作業内容 | detection_logsからSummary生成 |
| 新規ファイル | src/summary_service/mod.rs, generator.rs |
| 機能 | 期間内イベント集計、カメラ別統計、severity_max算出 |

### B-2: スケジューラー実装

| 項目 | 内容 |
|------|------|
| 作業内容 | 定時Summary/GrandSummary生成 |
| 新規テーブル | scheduled_reports |
| 機能 | 間隔ベース（Summary）、時刻ベース（GrandSummary） |

### B-3: Summary APIエンドポイント

| 項目 | 内容 |
|------|------|
| エンドポイント | POST /api/summary/generate |
| | GET /api/summary/latest |
| | GET /api/summary/:id |
| | GET /api/grand-summary/:date |

### B-4: BQ同期強化

| 項目 | 内容 |
|------|------|
| 作業内容 | Summary/GrandSummaryのBQ送信 |
| 対象テーブル | bq_sync_queue拡張 |

---

## 5. Phase C: Paraclate連携

**依存**: Phase A, Phase B完了

### C-1: Paraclateクライアントモジュール

| 項目 | 内容 |
|------|------|
| 作業内容 | Paraclate APP通信クライアント |
| 新規ファイル | src/paraclate_client/mod.rs, http_client.rs |
| 機能 | Summary送信、設定同期、ステータス報告 |

### C-2: 連携設定API

| 項目 | 内容 |
|------|------|
| エンドポイント | GET/PUT /api/paraclate/config |
| | POST /api/paraclate/connect |
| | GET /api/paraclate/status |

### C-3: フロントエンド有効化

| 項目 | 内容 |
|------|------|
| 作業内容 | SettingsModal.tsx のdisabled解除 |
| 対象 | エンドポイント入力、報告設定、接続ボタン |

### C-4: PUB/SUB統合（オプション）

| 項目 | 内容 |
|------|------|
| 作業内容 | リアルタイム設定同期 |
| 依存 | Paraclate APP側実装 |

---

## 6. Phase D: is21管理UI

**依存**: Phase A完了

### D-1: is21管理ページ作成

| 項目 | 内容 |
|------|------|
| 作業内容 | is20s相当の管理WebUI |
| フレームワーク | React + Vite（is22と同様） |
| 機能 | 設定表示、ハードウェア監視、ログ表示 |

### D-2: is22からのアクティベート

| 項目 | 内容 |
|------|------|
| 作業内容 | is22 → is21 アクティベートフロー |
| API | POST /api/activate (is21側) |
| 認証 | is22のtid, lacisID, cicで認証 |

---

## 7. タスク依存関係図

```
Phase A: 認証基盤
┌─────────────────────────────────────────┐
│ A-1: AraneaRegister                     │
│   ↓                                     │
│ A-2: DBスキーマ                         │
│   ↓                                     │
│ A-3: 登録API                            │
│   ↓                                     │
│ A-4: テナント移行                        │
└─────────────────────────────────────────┘
         ↓                    ↓
Phase B: データ基盤      Phase D: is21管理
┌─────────────────┐     ┌─────────────────┐
│ B-1: Summary    │     │ D-1: 管理ページ  │
│   ↓             │     │   ↓             │
│ B-2: スケジューラ│     │ D-2: アクティベート│
│   ↓             │     └─────────────────┘
│ B-3: Summary API│
│   ↓             │
│ B-4: BQ同期     │
└─────────────────┘
         ↓
Phase C: Paraclate連携
┌─────────────────────────────────────────┐
│ C-1: Paraclateクライアント              │
│   ↓                                     │
│ C-2: 連携設定API                        │
│   ↓                                     │
│ C-3: フロントエンド有効化               │
│   ↓                                     │
│ C-4: PUB/SUB統合 (オプション)           │
└─────────────────────────────────────────┘
```

---

## 8. 工数見積もり（参考）

| タスク | 複雑度 | 備考 |
|-------|-------|------|
| A-1 | 中 | ESP32からの移植、HTTPクライアント実装 |
| A-2 | 低 | SQLマイグレーションのみ |
| A-3 | 低 | 既存パターン踏襲 |
| A-4 | 低 | 設定値変更のみ |
| B-1 | 中 | 集計ロジック設計が必要 |
| B-2 | 中 | Tokioスケジューラー実装 |
| B-3 | 低 | 既存パターン踏襲 |
| B-4 | 低 | 既存BQ同期の拡張 |
| C-1 | 中 | 外部API連携 |
| C-2 | 低 | 既存パターン踏襲 |
| C-3 | 低 | UI変更のみ |
| D-1 | 高 | 新規WebUI作成 |
| D-2 | 中 | 認証フロー設計 |

---

## 9. テスト計画（概要）

### 単体テスト

| 対象 | テスト内容 |
|------|----------|
| LacisID生成 | 20文字形式、MAC変換 |
| Summary生成 | 期間集計、severity算出 |
| スケジューラー | 次回実行時刻計算 |

### 結合テスト

| 対象 | テスト内容 |
|------|----------|
| AraneaRegister | araneaDeviceGate登録成功 |
| Summary API | 生成→取得→表示 |
| Paraclate連携 | 送信→受信確認 |

### E2Eテスト

| 対象 | テスト内容 |
|------|----------|
| Chrome実機 | AIタブ操作、Summary表示 |

---

## 10. MECE確認

- **網羅性**: 全実装項目をフェーズ・タスクに分解
- **重複排除**: 各タスクは単一責務
- **依存関係**: 明示的な依存順序を定義
- **未カバー領域**: Paraclate APP側実装はmobes2.0開発ラインの範囲

---

## 11. 統括アドバイザーへの確認事項

1. **Phase A優先度**: 認証基盤を最優先で進めてよいか
2. **Phase D範囲**: is21管理UIはどの程度の機能を持たせるか
3. **BQ同期**: Phase B-4のBQ同期はParaclate連携前に完成させるか
4. **mobes2.0連携**: Paraclate APPの実装進捗と並行可能か
