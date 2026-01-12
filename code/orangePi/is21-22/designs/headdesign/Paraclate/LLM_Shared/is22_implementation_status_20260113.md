# IS22 Paraclate連携 実装状況報告

**作成日**: 2026-01-13
**対象**: mobes2.0チーム
**文書ID**: IS22-STATUS-20260113

---

## 1. 実装完了項目

### 1.1 Summary/GrandSummary生成機能（Phase 3: Issue #116）

| 項目 | 状態 | 備考 |
|------|------|------|
| SummaryGenerator | ✅ 完了 | 直近N時間の検出ログを集計 |
| GrandSummaryGenerator | ✅ 完了 | シフト単位でHourlySummaryを統合 |
| SummaryScheduler | ✅ 完了 | 1分間隔でスケジュールチェック |
| DBスキーマ (ai_summary_cache) | ✅ 完了 | UPSERT対応済み |

### 1.2 Paraclate APP連携（Phase 4）

| 項目 | 状態 | 備考 |
|------|------|------|
| ParaclateClient | ✅ 完了 | send_summary/send_grand_summary実装済み |
| Ingest API送信 | ✅ 完了 | paraclate_send_queueにキュー登録 |
| AI Chat API | ✅ 完了 | /api/paraclate/chat 経由でmobes APP呼び出し |

### 1.3 WebSocket/RealtimeHub

| 項目 | 状態 | 備考 |
|------|------|------|
| SummaryReportMessage | ✅ 完了 | WebSocket経由でフロントエンドに通知 |
| チャット表示 | ✅ 完了 | EventLogPaneでシステムメッセージとして表示 |

---

## 2. 未実装/確認が必要な項目

### 2.1 画像保存系インターフェース

**Paraclate_DesignOverview.md 参照箇所**: Section 3.2 画像データフロー

| 項目 | 状態 | 設計書要件 |
|------|------|-----------|
| 検出画像のGCS保存 | ⚠️ 未実装 | is22 → GCS直接アップロード または mobes経由 |
| 画像URL生成 | ⚠️ 未実装 | signed URL生成方式の決定が必要 |
| 画像参照パス | ⚠️ 未確認 | summary_jsonに画像パスを含めるか |

**質問**: 検出画像のGCS保存について、以下のどちらを想定していますか？
1. is22がGCS APIを直接呼び出してアップロード
2. mobes APPのIngest API経由でBase64画像を送信

### 2.2 BigQuery同期インターフェース

**Paraclate_DesignOverview.md 参照箇所**: Section 4.1 データアーキテクチャ

| 項目 | 状態 | 設計書要件 |
|------|------|-----------|
| BqSyncService | ❌ 削除済み | Hot CacheはmobesのBQ、is22はSSoT |
| 検出ログ同期 | ⚠️ 未実装 | Ingest API経由で送信するか直接BQ書き込みか |
| サマリー同期 | ⚠️ 未実装 | 同上 |

**現状**: `DataArchitecture_Investigation_Report.md`の合意に基づき、is22側のBqSyncServiceは削除。検出ログ/サマリーはIngest API経由でmobes APPに送信し、mobes側でBQに書き込む方式を想定。

**確認事項**:
- 現在のIngest API (`/api/paraclate/chat`)は会話用。検出ログ/サマリー送信用の別エンドポイントが必要か？
- または既存の`paraclate_send_queue`キュー方式でmobes側がポーリング取得するか？

---

## 3. 発見した問題点

### 3.1 サマリー二重送信問題

**症状**: チャットウィンドウにサマリーが2件表示される

**原因**: App.tsxで`summaryReport`をモバイル用とデスクトップ用の両方のEventLogPaneに渡しており、両方がチャットに追加している

**修正予定**: 片方のみが処理するようロジック修正

### 3.2 思考中アニメーション非表示問題

**症状**: AI Chatでは思考中アニメーション（3点リーダー）が表示されるが、サマリー受信時は表示されない

**原因**: サマリーはWebSocket経由でプッシュされるため、`isChatLoading`状態が設定されない。ユーザー質問時のみAPIコール中に`isChatLoading=true`が設定される仕様

**対応**: サマリー受信はプッシュ通知のため、アニメーションは不要と判断。ただしUIの一貫性のため要検討

### 3.3 会話継続性の欠如

**症状**: AI Chatで継続会話する際、過去のサマリーや前回の回答が送信されない

**原因**: 現在の`/api/paraclate/chat` APIは`conversationHistory`パラメータを受け付けるが、サマリーメッセージ（systemロール）は除外されている

**修正予定**:
1. `conversationHistory`にサマリーメッセージも含める
2. 直近N件（例: 10件）のチャット履歴を自動的に送信

**現在のコード（EventLogPane.tsx）**:
```typescript
const historyForApi: ChatMessageInput[] = conversationHistory
  .slice(-10)  // Last 10 messages for context
  .map(m => ({
    role: m.role as 'user' | 'assistant',  // 'system'が除外される
    content: m.content,
  }))
```

---

## 4. mobes側への要確認事項

### 4.1 AI Chat レスポンス品質

サマリーテキストの内容が要領を得ない点について：

**現在のサマリーテキスト例**:
```
1時間の検出サマリー: 合計6件の検出（2台のカメラ）- cam-b7c34d...: 3件（最大severity: 4）- cam-bb66a6...: 3件（最大severity: 4）
```

**質問**:
1. サマリーテキストのフォーマットは現状で適切か？
2. mobes側のLLMで要約・整形する想定か？
3. より人間が読みやすいフォーマット案があれば提示希望

### 4.2 Ingest API仕様

**現状**: `paraclate_send_queue`テーブルにキュー登録し、is22起動時に送信処理

**質問**:
1. mobes側のIngest APIエンドポイントURL
2. 認証方式（Bearer token? API key?）
3. リクエストボディのスキーマ
4. レスポンス形式

### 4.3 会話履歴の扱い

**質問**:
1. mobes側AI Chatで会話履歴をどこまで保持するか？
2. is22側から送信する履歴件数の推奨値は？
3. サマリーメッセージ（systemロール）も履歴に含めるべきか？

---

## 5. 次のアクション

### IS22側
1. [ ] サマリー二重送信問題の修正
2. [ ] 会話履歴送信ロジックの改善（systemロール含める）
3. [ ] mobes側回答を受けて画像保存/BQ同期の実装方針決定

### mobes側への依頼
1. [ ] 上記質問事項への回答
2. [ ] Ingest API仕様の共有
3. [ ] サマリーテキストフォーマットの要件定義

---

## 6. 参照ドキュメント

- `code/orangePi/is21-22/designs/headdesign/Paraclate/Paraclate_DesignOverview.md`
- `doc/APPS/Paraclate/LLM/DataArchitecture_Investigation_Report.md`
- `doc/APPS/Paraclate/LLM/mobes_response_to_is22.md`

---

**連絡先**: is22開発チーム
