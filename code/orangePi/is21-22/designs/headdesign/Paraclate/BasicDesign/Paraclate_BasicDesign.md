# Paraclate 基本設計
作成日: 2026-01-XX  
対象: is21（ParaclateEdge）、is22（Paraclate Server）、mobes2.0 Paraclate APP

## 1. 目的・前提
- Paraclate_DesignOverview.mdおよびRequirementsDefinitionに基づき、is22/is21とmobes2.0間の連携を実装可能な粒度で具体化する。
- SSoT: テナント/権限=mobes2.0、カメラ台帳/検知ログ/サマリー=is22、長期分析=BigQuery、AI設定=mobes2.0、画像=LacisFiles。
- 認証: lacisOath（lacisId/userId/cic）を基本とし、越境時のみblessing(permission91+発行)を使用。TypeDomainはaraneaDeviceで統一。

## 2. アーキテクチャと責務分割
- **is22 Paraclate Server**
  - デバイス登録クライアント（AraneaRegister Rust版）
  - カメラ台帳SSoT（lacisID必須）、検知ログSSoT、Summary生成/保存/送信
  - Paraclate設定キャッシュ・適用、Pub/Subによる設定/ステータス同期
  - AIチャット用データ提供API、BQ同期サービス
- **is21 ParaclateEdge**
  - 推論API `/v1/analyze` 拡張（車両OCR・顔特徴）
  - is22起動時のアクティベート要求を受け付け、推論結果をis22へ返却
- **Paraclate APP (mobes2.0)**
  - AI Model SettingsでParaclate設定SSoT管理（endpoint/interval/time/retention等）
  - Ingest APIでSummary/ログを受信→Firestore Hot + BigQuery投入
  - Chat APIでis22データ取得→マルチステージLLM実行→回答返却
  - blessing発行・管理UI/監査

## 3. データ設計
- **既存流用**: `ai_summary_cache`、`ai_chat_history`（is22）。`bq_sync_queue`で同期管理。
- **テーブル追加/拡張（is22側）**
  - `aranea_registration(lacis_id, tid, cic, state_endpoint, mqtt_endpoint, registered_at, last_sync_at)`
  - `paraclate_config(tid, fid, endpoint, report_interval_minutes, grand_summary_times_json, retention_days, attunement, updated_at)`  
    ※SSoTはmobes2.0、is22はキャッシュとして保持。同期元タイムスタンプを保持。
  - `scheduled_reports(schedule_id, tid, fid, report_type(summary/grand_summary), interval_minutes, scheduled_times_json, last_run_at, next_run_at, enabled)`
  - `ai_summary_cache`に `summary_json JSON NULL` を追加し送信用ペイロードを保持。
  - `detection_logs`/`is21_logs`に `vehicle_ocr`, `face_features` カラム追加（JSON）。
- **カメラ台帳**
  - `cameras.lacis_id` をNOT NULLに昇格（既存データはIpcamScan承認時に付与）。
  - `cameras.product_code` にブランドコードを保持しlacisID生成に利用。
- **mobes2.0側**
  - `aiApplications.paraclate.configurationDefaults`: `endpoint`, `summary.reportIntervalMinutes`, `grandSummary.times[]`, `retention.imagesDays`, `retention.summaryDays` 等。
  - `systemBlessings` コレクション（blessingId, issuedBy, allowedTids/Fids, expiresAt, purpose）。

## 4. API/IF設計
- **is22 外部API**
  - `POST /api/register/device` 登録実行（lacisOath + device meta）。`GET /api/register/status`、`DELETE /api/register`.
  - Summary系: `POST /api/summary/generate`（手動）、`GET /api/summary/latest`、`GET /api/summary/:id`、`GET /api/grand-summary/:date`。
  - Paraclate送信: `POST /api/paraclate/summary`（DesignOverviewのJSON形式。lacisOath必須、blessingは越境時のみ）。
  - データ提供: `GET /api/detection-logs`、`GET /api/cameras`、`GET /api/summaries`（tid/fid/time-range/filter指定）。`POST /api/chat/history`で履歴保存。
  - 設定取得/更新: `GET/PUT /api/paraclate/config`（SSoT同期用）、`POST /api/paraclate/connect`（接続テスト）、`GET /api/paraclate/status`（連携状態）。
  - スケジュール: `GET/PUT /api/reports/schedule`。
- **is21 IF**
  - `/v1/analyze` レスポンスに `vehicle_ocr`, `face_features` を追加。
  - is22→is21 アクティベート: `POST /v1/activate`（payload: is22 lacisId/tid/cic, endpoints）。
- **mobes2.0 Paraclate APP**
  - AI Model Settings: `APP_CONFIG_DEFINITIONS.paraclate` にフィールド追加。UIは /system/ai-model のApplicationsタブで編集。
  - Ingest API（Cloud Functions/Run）: `POST /api/paraclate/ingest/summary`、`POST /api/paraclate/ingest/event`。認証=デバイスCIC + tid整合。受信後Firestore Hot保存→BigQuery `semantic_events` へ投入。
  - Chat API: `POST /api/paraclate/chat`（tid/fid/userQuery/timeRange）。内部でis22 APIを呼び出し、LLMマルチステージ（Flash→Tool→Pro→Flash）で応答。返却に参照ID・根拠を含む。
  - Blessing API/UI: 発行・取消・監査ログ閲覧。発行時にpermission>=91検証。
- **Pub/Sub**
  - Topic `paraclate-{tid}`: `is22-{lacisId}-config`（設定変更通知）、`is22-{lacisId}-status`（状態）。payloadはtype/timestamp/payload(JSON)。

## 5. 処理フロー
- **デバイス登録**
  1. is22起動時にconfig_storeから登録済みを確認。未登録ならlacisID生成→araneaDeviceGateへPOST→CIC/state/mqtt保存。
  2. 再起動時は保存済みCICを再利用。
- **カメラ登録**
  1. IpcamScan承認時にMAC/ブランドからlacisID生成（3801 + MAC + productCode）。
  2. araneaDeviceGateへ登録し、lacisIDをcamerasに保存。未登録状態ではpreset同期/推論登録を拒否。
- **設定同期**
  1. mobes2.0 AI Model SettingsがSSoT。変更時にPub/Subでis22へ通知。
  2. is22はparaclate_configを更新し、UIに反映。手動更新はmobes2.0側へPUTしてから反映する（二重更新禁止）。
- **Summary/GrandSummary生成**
  1. `scheduled_reports`を基にschedulerが次回実行を算出。失敗時はリトライ + next_run_at再計算。
  2. detection_logsをtid/fid別に集計→summary_text/summary_json生成→ai_summary_cache保存→bq_sync_queueにenqueue。
  3. GrandSummaryはdailyでai_summary_cache(hourly)を統合。
- **Paraclate送信**
  1. 生成後、lacisOath付きでParaclate APP ingest APIへPOST。
  2. blessing指定がある場合のみpayloadに含め、サーバ側で越境検証後に処理。無指定時はtid/fid一致のみ許可。
- **AIチャット**
  1. Paraclate APP Chat APIが意図分類→is22データAPI呼び出し→施設コンテキスト/カメラコンテキスト/summaryを取得。
  2. Stage Proで統合推論、Stage Flashで最終回答。参照ID・不確実性ラベルを付与。
- **BQ同期**
  1. is22のBqSyncServiceがbq_sync_queueをポーリングし、まとめてBigQueryへInsert。成功でsynced_to_bq更新、失敗でretry_count・failed管理。
- **画像保存**
  1. is22は画像を直接StorageにPUTせず、Paraclate ingestが受信→LacisFilesにアップロード→署名URL/TTL管理。

## 6. フロントエンド/UI設計（is22側）
- SettingsModalのAIタブをParaclate向けに有効化。項目: endpoint、reportIntervalMinutes、grandSummaryTimes(複数)、reportDetail、attunement、接続状態表示。
- Summaryビュー・GrandSummaryビューを追加し、ai_summary_cacheの最新データとParaclate送信状態を表示。
- エラーバナーで登録失敗/設定同期失敗/BQ同期失敗を通知。

## 7. セキュリティ・運用設計
- lacisOath認証をすべての登録/送信/ingestで必須化。tid/fid整合をサーバ側で検証。
- blessingはpermission91+発行のみを受理し、allowedTids/FidsとexpiresAtをチェック。監査ログにissuer/requestor/timestampを記録。
- Pub/Sub/HTTP間の通信はHTTPS必須。機密情報はconfig_storeやSecret Managerに保存。
- ログ・メトリクス: 登録、Summary生成/送信、BQ同期、設定同期、blessing利用をイベント単位で記録。

## 8. テスト観点（受入のための設計項目）
- 登録E2E: 未登録→登録成功→再起動で再登録しない。
- カメラ承認E2E: 承認時にlacisIDが付与され、preset同期が成功。
- Summary/GrandSummary: 設定変更→scheduler反映→生成→DB保存→Paraclate送信→mobesで受信確認。
- チャット: 「今日は何かあった？」でSummary参照回答、「赤い服…」で検知ログフィルタ回答、不在時は推論/不明明示。
- 越境: blessing無しで拒否、blessing付きで許可し監査が残る。
- BQ同期: queueに積まれたログが成功時にsynced_to_bq=true、失敗時にretry/failedへ遷移。

## 9. MECE/SOLID確認
- 機能領域を登録/台帳、設定、生成、送信、データ提供、同期、UI/UX、セキュリティに分割し重複なく網羅（MECE）。
- モジュール責務を明確化し、AI推論実行をmobes側へ委譲、送信/生成/同期/登録を分離（SRP/DIP順守）。越境・設定SSoTを固定しSSoT違反を防止。
