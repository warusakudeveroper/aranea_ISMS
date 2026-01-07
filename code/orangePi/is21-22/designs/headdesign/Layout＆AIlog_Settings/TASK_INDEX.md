# Layout＆AIlog_Settings タスクインデックス

作成日: 2026-01-07
ステータス: 承認待ち

---

## 1. タスク一覧

### Phase 1: カメラタイル表示修正（LAS-01）

| タスクID | 内容 | 依存 | 見積 | 担当 |
|----------|------|------|------|------|
| IMPL-T01 | getModelDisplayName ヘルパー関数追加 | - | 15分 | - |
| IMPL-T02 | getLocationDisplay ヘルパー関数追加 | - | 10分 | - |
| IMPL-T03 | getCameraStatus / getStatusBadge 関数追加 | - | 20分 | - |
| IMPL-T04 | overlay モードUI修正 | T01,T02,T03 | 20分 | - |
| IMPL-T05 | footer モードUI修正 | T01,T02,T03 | 15分 | - |
| IMPL-T06 | 相対時間表示形式修正 | - | 5分 | - |
| TEST-T01 | 単体テスト作成・実行 | T01-T06 | 30分 | - |
| TEST-T02 | Chrome UIテスト実行 | T01-T06 | 20分 | - |

**Phase 1 合計**: 約2時間15分

### Phase 2: AIイベントログ設定タブ（LAS-02）

| タスクID | 内容 | 依存 | 見積 | 担当 |
|----------|------|------|------|------|
| IMPL-A01 | AiEventLogSettingsTab.tsx 新規作成 | - | 2時間 | - |
| IMPL-A02 | SettingsModal.tsx タブ追加 | A01 | 15分 | - |
| IMPL-A03 | routes.rs API追加 | - | 1時間 | - |
| IMPL-A04 | config_store 型・リポジトリ追加 | - | 30分 | - |
| TEST-A01 | バックエンドテスト作成・実行 | A03,A04 | 30分 | - |
| TEST-A02 | フロントエンドテスト作成・実行 | A01,A02 | 30分 | - |
| TEST-A03 | Chrome UIテスト実行 | A01-A04 | 20分 | - |

**Phase 2 合計**: 約5時間5分

---

## 2. 依存関係図

```
Phase 1 (並列実行可能)
├── IMPL-T01 (getModelDisplayName)
├── IMPL-T02 (getLocationDisplay)
├── IMPL-T03 (getCameraStatus)
└── IMPL-T06 (相対時間修正)
        ↓
├── IMPL-T04 (overlay UI) ← 依存: T01,T02,T03
└── IMPL-T05 (footer UI) ← 依存: T01,T02,T03
        ↓
├── TEST-T01 (単体テスト)
└── TEST-T02 (UIテスト)

Phase 2 (並列実行可能)
├── IMPL-A01 (FE: AiEventLogSettingsTab)
├── IMPL-A03 (BE: routes.rs)
└── IMPL-A04 (BE: config_store)
        ↓
├── IMPL-A02 (タブ追加) ← 依存: A01
├── TEST-A01 (BEテスト) ← 依存: A03,A04
└── TEST-A02 (FEテスト) ← 依存: A01,A02
        ↓
└── TEST-A03 (UIテスト) ← 全タスク完了後
```

---

## 3. Issue対応表

| Issue | タイトル | タスク | ステータス |
|-------|---------|--------|--------|
| #97 | カメラタイル表示仕様準拠（LAS-01） | Phase 1全タスク | OPEN |
| #98 | AIイベントログ設定タブ追加（LAS-02） | Phase 2全タスク | OPEN |

---

## 4. 受け入れ条件

### Phase 1: カメラタイル表示修正

- [ ] モデル名がフォールバック表示される（model→manufacturer→family→Unknown）
- [ ] 施設名+fidが「HALE京都丹波口 (0150)」形式で表示される
- [ ] 最終更新が「3分前 更新」形式で表示される
- [ ] ステータスバッジ（🟢/🔴/⚪）が右上に表示される
- [ ] 単体テスト11項目がPASS
- [ ] Chrome UIテスト7項目がPASS

### Phase 2: AIイベントログ設定タブ

- [ ] SettingsModalに「AIログ」タブが表示される
- [ ] IS21エンドポイント・タイムアウトが設定可能
- [ ] 保存ポリシー（none events/min_confidence）が設定可能
- [ ] BQ同期設定（間隔/バッチサイズ）が設定可能
- [ ] BQ同期ステータスが表示される
- [ ] バックエンドテスト5項目がPASS
- [ ] フロントエンドテスト5項目がPASS
- [ ] Chrome UIテスト3項目がPASS

---

## 5. テストチェックリスト（統合）

### Phase 1 テスト

| ID | カテゴリ | テスト内容 | 期待結果 | 状態 |
|----|----------|-----------|----------|------|
| UT-T01 | 単体 | model設定済み → getModelDisplayName | modelの値を返す | - |
| UT-T02 | 単体 | model未設定, manufacturer設定 | manufacturerを返す | - |
| UT-T03 | 単体 | model/manufacturer未設定, family="tapo" | "Tapo"を返す | - |
| UT-T04 | 単体 | 全未設定 → getModelDisplayName | "Unknown"を返す | - |
| UT-T05 | 単体 | location+fid設定 → getLocationDisplay | "{location} ({fid})"形式 | - |
| UT-T06 | 単体 | locationのみ → getLocationDisplay | locationのみ | - |
| UT-T07 | 単体 | fidのみ → getLocationDisplay | "FID:{fid}"形式 | - |
| UT-T08 | 単体 | 両方未設定 → getLocationDisplay | "-" | - |
| UT-T09 | 単体 | enabled=true, last_verified_at=1分前 | "online" | - |
| UT-T10 | 単体 | enabled=true, last_verified_at=10分前 | "offline" | - |
| UT-T11 | 単体 | enabled=false → getCameraStatus | "disabled" | - |
| UI-T01 | Chrome | カメラタイル表示確認 | 仕様通りのレイアウト | - |
| UI-T02 | Chrome | モデル名フォールバック確認 | manufacturer表示 | - |
| UI-T03 | Chrome | 施設名+fid表示確認 | 正しい形式 | - |
| UI-T04 | Chrome | 相対時間表示確認 | "3分前 更新"形式 | - |
| UI-T05 | Chrome | ステータスバッジ(オンライン) | 緑色バッジ | - |
| UI-T06 | Chrome | ステータスバッジ(オフライン) | 赤色バッジ | - |
| UI-T07 | Chrome | ステータスバッジ(無効) | グレーバッジ | - |

### Phase 2 テスト

| ID | カテゴリ | テスト内容 | 期待結果 | 状態 |
|----|----------|-----------|----------|------|
| BE-T01 | バックエンド | GET /api/settings/ai-event-log | 設定値を返す | - |
| BE-T02 | バックエンド | PUT 正常値 | 200 OK, 設定保存 | - |
| BE-T03 | バックエンド | PUT timeout_ms範囲外 | 400 Bad Request | - |
| BE-T04 | バックエンド | PUT min_confidence範囲外 | 400 Bad Request | - |
| BE-T05 | バックエンド | GET /api/settings/bq-sync-status | ステータス返却 | - |
| FE-T01 | フロントエンド | AIログタブ表示 | タブがクリック可能 | - |
| FE-T02 | フロントエンド | 設定読み込み | フォームに値が表示 | - |
| FE-T03 | フロントエンド | 範囲外入力 | エラーメッセージ表示 | - |
| FE-T04 | フロントエンド | 保存ボタン押下 | APIコール、成功通知 | - |
| FE-T05 | フロントエンド | BQ同期ステータス表示 | pending_count等表示 | - |
| UI-T08 | Chrome | 設定モーダル→AIログタブ | タブ遷移、設定表示 | - |
| UI-T09 | Chrome | 値変更→保存 | 保存成功、再読み込みで反映 | - |
| UI-T10 | Chrome | BQ同期ステータス確認 | リアルタイム更新 | - |

---

## 6. 大原則チェック

本タスクインデックスは以下の原則を遵守:

- [x] SSoT: 設計文書を唯一の情報源として参照
- [x] SOLID-S: 各タスクは単一責務
- [x] MECE: Phase 1/2 の全タスクを網羅的に定義
- [x] アンアンビギュアス: 各タスクの見積・依存関係を明確化
- [x] テスト計画あり: 28項目のテストを定義（Phase 1: 18項目、Phase 2: 10項目）
- [x] 依存関係と実装順: 依存関係図で明示
