# AIEventlog: AIイベントログ設定機能設計

文書番号: Layout＆AIlog_Settings/AIEventlog
作成日: 2026-01-07
ステータス: 設計確定
関連文書: is22_AI_EVENT_LOG_DESIGN.md

---

## 1. 概要

### 1.1 目的
`is22_AI_EVENT_LOG_DESIGN.md` セクション9で定義されたAIイベントログ設定項目をSettingsModalで操作可能にし、運用時のチューニングを容易にする。

### 1.2 現状
- SettingsModalに「AIイベントログ」設定タブが存在しない
- 設定項目はハードコードまたはDB直接操作が必要
- EventLogPaneのフィルタ機能はUI側で実装されているが、永続化されていない

### 1.3 対象ファイル
- `frontend/src/components/SettingsModal.tsx` - タブ追加
- `frontend/src/components/AiEventLogSettingsTab.tsx` - 新規作成
- `src/web_api/routes.rs` - 設定API追加（必要に応じて）

### 1.4 MECEチェック
本設計は以下の観点で網羅的・相互排他的に整理：
- 設定項目: IS21接続/保存ポリシー/BQ同期/フィルタ永続化
- 各設定の読み取り/書き込み/検証を明確化
- 既存設計（is22_AI_EVENT_LOG_DESIGN.md）との整合性を確認

---

## 2. 設定項目定義

### 2.1 設定項目一覧（is22_AI_EVENT_LOG_DESIGN.md セクション9より）

```json
{
  "ai_event_log": {
    "is21_endpoint": "http://192.168.3.240:9000/v1/analyze",
    "timeout_ms": 30000,
    "save_none_events": false,
    "min_confidence": 0.3,
    "bq_sync_interval_sec": 60,
    "bq_sync_batch_size": 100
  }
}
```

### 2.2 設定項目詳細

| 項目 | 型 | 範囲 | デフォルト | 説明 |
|------|-----|------|-----------|------|
| is21_endpoint | string | URL形式 | http://192.168.3.240:9000/v1/analyze | IS21推論エンドポイント |
| timeout_ms | number | 5000-120000 | 30000 | IS21リクエストタイムアウト(ms) |
| save_none_events | boolean | - | false | primary_event="none"も保存するか |
| min_confidence | number | 0.1-1.0 | 0.3 | 最小信頼度閾値 |
| bq_sync_interval_sec | number | 30-600 | 60 | BQ同期間隔(秒) |
| bq_sync_batch_size | number | 10-500 | 100 | BQ同期バッチサイズ |

---

## 3. UI設計

### 3.1 タブ追加

SettingsModalに「AIログ」タブを追加（既存8タブ → 9タブ）

```typescript
// SettingsModal.tsx TabsList
<TabsTrigger value="ailog" className="flex items-center gap-1 text-xs">
  <Activity className="h-4 w-4" />
  AIログ
</TabsTrigger>
```

### 3.2 AiEventLogSettingsTab レイアウト

```
┌──────────────────────────────────────────────────────────────────────┐
│ AIイベントログ設定                                                    │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│ ┌─ IS21接続設定 ───────────────────────────────────────────────────┐ │
│ │ エンドポイント: [http://192.168.3.240:9000/v1/analyze           ] │ │
│ │ タイムアウト:   [30___] 秒  (5-120秒)                            │ │
│ │                                                                   │ │
│ │ 💡 IS21タブでも接続テストが可能です                               │ │
│ └───────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│ ┌─ 保存ポリシー ─────────────────────────────────────────────────────┐ │
│ │ □ 検出なしイベントも保存する (none events)                        │ │
│ │ 最小信頼度: [0.30] (0.10-1.00)                                    │ │
│ │                                                                   │ │
│ │ 💡 検出なしを保存すると容量が増加しますが、解析改善に役立ちます    │ │
│ └───────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│ ┌─ BigQuery同期設定 ─────────────────────────────────────────────────┐ │
│ │ 同期間隔:   [60__] 秒  (30-600秒)                                 │ │
│ │ バッチサイズ: [100_] 件  (10-500件)                               │ │
│ │                                                                   │ │
│ │ ステータス: 🟢 同期中 | 未同期: 12件 | 最終同期: 10:30:15         │ │
│ └───────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│                                        [設定を保存]                   │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

### 3.3 コンポーネント設計

```typescript
// frontend/src/components/AiEventLogSettingsTab.tsx

interface AiEventLogSettings {
  is21_endpoint: string
  timeout_ms: number
  save_none_events: boolean
  min_confidence: number
  bq_sync_interval_sec: number
  bq_sync_batch_size: number
}

interface BqSyncStatus {
  status: 'syncing' | 'idle' | 'error'
  pending_count: number
  last_synced_at: string | null
  error_message: string | null
}

export function AiEventLogSettingsTab() {
  const [settings, setSettings] = useState<AiEventLogSettings | null>(null)
  const [bqStatus, setBqStatus] = useState<BqSyncStatus | null>(null)
  const [saving, setSaving] = useState(false)

  // GET /api/settings/ai-event-log
  // GET /api/settings/bq-sync-status
  // PUT /api/settings/ai-event-log
}
```

---

## 4. API設計

### 4.1 設定取得

```
GET /api/settings/ai-event-log
```

レスポンス:
```json
{
  "ok": true,
  "data": {
    "is21_endpoint": "http://192.168.3.240:9000/v1/analyze",
    "timeout_ms": 30000,
    "save_none_events": false,
    "min_confidence": 0.3,
    "bq_sync_interval_sec": 60,
    "bq_sync_batch_size": 100
  }
}
```

### 4.2 設定更新

```
PUT /api/settings/ai-event-log
Content-Type: application/json

{
  "is21_endpoint": "http://192.168.3.240:9000/v1/analyze",
  "timeout_ms": 30000,
  "save_none_events": false,
  "min_confidence": 0.3,
  "bq_sync_interval_sec": 60,
  "bq_sync_batch_size": 100
}
```

レスポンス:
```json
{
  "ok": true,
  "message": "Settings saved successfully"
}
```

### 4.3 BQ同期ステータス

```
GET /api/settings/bq-sync-status
```

レスポンス:
```json
{
  "ok": true,
  "data": {
    "status": "syncing",
    "pending_count": 12,
    "last_synced_at": "2026-01-07T10:30:15Z",
    "error_message": null
  }
}
```

---

## 5. バリデーション

### 5.1 フロントエンドバリデーション

| 項目 | ルール | エラーメッセージ |
|------|--------|----------------|
| is21_endpoint | URL形式、http/https | "有効なURLを入力してください" |
| timeout_ms | 5000-120000 | "5〜120秒の範囲で入力してください" |
| min_confidence | 0.1-1.0 | "0.1〜1.0の範囲で入力してください" |
| bq_sync_interval_sec | 30-600 | "30〜600秒の範囲で入力してください" |
| bq_sync_batch_size | 10-500 | "10〜500件の範囲で入力してください" |

### 5.2 バックエンドバリデーション

```rust
// src/web_api/routes.rs
fn validate_ai_event_log_settings(req: &AiEventLogSettingsRequest) -> Result<(), ValidationError> {
    if req.timeout_ms < 5000 || req.timeout_ms > 120000 {
        return Err(ValidationError::new("timeout_ms must be between 5000 and 120000"));
    }
    if req.min_confidence < 0.1 || req.min_confidence > 1.0 {
        return Err(ValidationError::new("min_confidence must be between 0.1 and 1.0"));
    }
    // ...
    Ok(())
}
```

---

## 6. 変更ファイル一覧

| ファイル | 変更内容 | 行数見積 |
|----------|---------|---------|
| `frontend/src/components/SettingsModal.tsx` | タブ追加、import追加 | +10行 |
| `frontend/src/components/AiEventLogSettingsTab.tsx` | 新規作成 | +250行 |
| `src/web_api/routes.rs` | API追加 | +100行 |
| `src/config_store/types.rs` | AiEventLogSettings型追加 | +20行 |
| `src/config_store/repository.rs` | 設定読み書きメソッド追加 | +50行 |

---

## 7. テスト計画

### 7.1 バックエンドテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| BE-T01 | GET /api/settings/ai-event-log | 設定値を返す |
| BE-T02 | PUT 正常値 | 200 OK, 設定保存 |
| BE-T03 | PUT timeout_ms範囲外 | 400 Bad Request |
| BE-T04 | PUT min_confidence範囲外 | 400 Bad Request |
| BE-T05 | GET /api/settings/bq-sync-status | ステータス返却 |

### 7.2 フロントエンドテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| FE-T01 | AIログタブ表示 | タブがクリック可能 |
| FE-T02 | 設定読み込み | フォームに値が表示 |
| FE-T03 | 範囲外入力 | エラーメッセージ表示 |
| FE-T04 | 保存ボタン押下 | APIコール、成功通知 |
| FE-T05 | BQ同期ステータス表示 | pending_count等表示 |

### 7.3 Chrome UIテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| UI-T01 | 設定モーダル→AIログタブ | タブ遷移、設定表示 |
| UI-T02 | 値変更→保存 | 保存成功、再読み込みで反映 |
| UI-T03 | BQ同期ステータス確認 | リアルタイム更新 |

---

## 8. 依存関係

### 8.1 既存依存
- `is22_AI_EVENT_LOG_DESIGN.md` セクション9の設定項目定義
- SettingsModalの既存タブ構造（8タブ → 9タブに拡張）

### 8.2 新規依存
- settings テーブルに `ai_event_log` JSON カラムが存在すること
- bq_sync_queue テーブル（BQ同期ステータス取得用）

---

## 9. 実装タスク

| タスクID | 内容 | 見積 |
|----------|------|------|
| IMPL-A01 | AiEventLogSettingsTab.tsx 新規作成 | 2時間 |
| IMPL-A02 | SettingsModal.tsx タブ追加 | 15分 |
| IMPL-A03 | routes.rs API追加 | 1時間 |
| IMPL-A04 | config_store 型・リポジトリ追加 | 30分 |
| TEST-A01 | バックエンドテスト作成・実行 | 30分 |
| TEST-A02 | フロントエンドテスト作成・実行 | 30分 |
| TEST-A03 | Chrome UIテスト実行 | 20分 |

---

## 10. 大原則チェックリスト

- [x] SSoT: is22_AI_EVENT_LOG_DESIGN.md の設定定義を唯一の情報源として参照
- [x] SOLID-S: AiEventLogSettingsTab は設定UI表示・保存の単一責務
- [x] MECE: 全設定項目と操作（読取/書込/検証）を網羅
- [x] アンアンビギュアス: バリデーションルール・範囲を明確に定義
- [x] 車輪の再発明禁止: 既存SettingsModalパターンを踏襲
- [x] テスト計画あり: BE5項目、FE5項目、UI3項目を定義
