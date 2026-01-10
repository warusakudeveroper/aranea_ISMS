以下、**両リポジトリ（mobes2.0 / aranea_ISMS）を実際に確認した上で**、齟齬のない **MECE 構造**・機能実装の洗い出し・**is21/22開発ライン**と **ParaclateAPP(mobes2.0)開発ライン**のタスク分解（特に **マルチステージAI×UX**）を「統括アドバイザー」として提示します。

---

## 1) 確認済み根拠（両リポジトリ確認）

### aranea_ISMS 側（is21/is22）

* is21 は Edge 推論（YOLO + PAR + camera_context 等）を担う設計・READMEが存在 
* is22 は “Camserver” として **SSoT=ConfigStore** を前提に 11コンポーネントで構成され、WebSocket/SSE/REST を提供 
* is22 には **detection_logs / ai_chat_history / ai_summary_cache / bq_sync_queue** が追加されており、AIチャット履歴とサマリーキャッシュのDB基盤が既にある  
* DetectionLogService は「AI Chat のためのクエリIF」を責務として明記 
* is22 の Camera 構造体に **fid/tid/lacis_id/cic** がすでに存在し、テナント境界をDBで持てる状態 

### mobes2.0 側（LacisOath / AI settings）

* araneaDeviceGate（デバイス登録）が **lacisOath（lacisId + userId + cic / pass不要）**で動作する設計・実装が存在  
* ESP32側の AraneaRegister はその方式で `lacisOath` を組み立てて register する実装が存在（is22で流用すべき参照） 
* AI Model Settings UI は **aiApplications（アプリ別）設定のフィールド定義（APP_CONFIG_DEFINITIONS）**を持ち、ここに Paraclate を追加するのが最短 
* 既存モジュールに **マルチステージ（stage1/2/3）**設定パターンがあり、ParaclateのマルチステージAIにも転用可能 
* LacisOath の userObject は tid と fid（配列）で境界を持ち、fid=0000 などの扱いが存在 

---

## 2) Paraclate 全体の MECE 構造（SSoTを先に固定する）

ここが曖昧だと「二重SSoT」「車輪の再発明」「境界の破綻」が起きます。現状の実装資産から、以下で固定するのが最も齟齬が出ません。

### SSoT割当（決定案）

| 領域                                 | SSoT                                                   | 理由                           |
| ---------------------------------- | ------------------------------------------------------ | ---------------------------- |
| テナント/施設/権限（tid/fid/permission）     | mobes2.0（LacisOath / tenants / facilities）             | 既に仕様・実装がある                   |
| is22 カメラ台帳・カメラ設定（RTSP/AI設定/コンテキスト） | is22 MariaDB（camerasがSSoT）                             | is22側の設計原則として明記              |
| 検知ログ（リアルタイム/近傍検索）                  | is22 MariaDB（detection_logs）                           | 既に永続・検索・BQキューまである            |
| 長期分析（横断・集計）                        | BigQuery（最終到達点）                                        | bq_sync_queue が既に存在し設計方針もある  |
| AI設定（モデル/プロンプト/上限/運用方針）            | mobes2.0 AI Model Settings（aiApplications + overrides） | UI/運用が既に整備                   |

※「Paraclateエンドポイント」は **mobes側（AI設定）**に保持し、**実データ（カメラ台帳等）は is22 に置く**のが最もMECEです（endpointは接続情報であり業務データではない）。

---

## 3) 現状の齟齬・不足（アンアンビギュアスに列挙）

### 3-1. 命名

* is22 UI/README は 아직 “mobes AIcam control Tower (mAcT)” 表記 
  → 指示通り **Paraclate（サブタイトル mobes AI control Tower）**へ改名タスクが必要
* is21 も README は “IS21 camimageEdge AI” 
  → **ParaclateEdge** へ改名（仕様）

### 3-2. is22 ↔ mobes 認証（lacisOath/登録）

* mobes 側は araneaDeviceGate が **Tenant Primary（permission 61+）**で認証する設計 
* ESP32の AraneaRegister は実装済み 
* 一方、is22（Rust）側に “AraneaRegister相当” が未整備（あなたの記述どおり）
  → is22 が mobes に **Summary/GrandSummary を投げる**にはここがブロッカー

### 3-3. Summary/GrandSummary のモデルとDB

* is22 には **ai_summary_cache** が既にあり、summary_type（hourly/daily/emergency）まである 
  → “Summary=間隔” は **hourly**、 “GrandSummary=時刻” は **daily** に自然にマップ可能
* ただしあなたの提示した **送信JSON（cameraContext / cameraDetection の形式）**は、現状DBに「丸ごと保持」するカラムがない（summary_text中心）
  → ai_summary_cache に **summary_json（JSON）**を追加するか、summary_text内に保持する設計が必要

### 3-4. マルチステージAI（UXの核）

* mobes側には stage1/2/3 のマルチステージ設計パターンが既に存在 
* ただし Paraclate 用の **Intent Router → 参照先決定 → 取得 → 要約/回答**の “会話UX” はまだ未定義
  → ここを統括としてMECEに定義する（後述）

### 3-5. “blessing” クロステナント権能

* 仕様として提示された “blessing” フィールドは、既存コードからは直接は確認できていません（この点は**現時点では実装未確認＝アンアンビギュアス**）。
  → lacisOath 側での実装タスク化が必要（mobes側）

---

## 4) タスク洗い出し（is21/22 開発ライン）

以下は **依存関係順**（黄金ルールの「実装順＝依存関係」）で並べます。

### EP-IS22-0: Paraclate命名改定（即時）

1. is22 README / UI 表記を Paraclate に変更（副題は “mobes AI control Tower”）

   * 現状 “mobes AIcam control Tower (mAcT)” 
   * 受入: 画面ヘッダ・README・主要docの表記が統一
2. is21 README / doc 表記を ParaclateEdge に変更 

---

### EP-IS22-1: is22 の “AraneaRegister（Rust版）” 実装（P0ブロッカー）

**目的**: is22 が mobes2.0 に対し、デバイスとして正規に認証できること。

**実装方針（既存参照）**

* 認証要素は lacisOath: `lacisId + userId + cic`（pass不要） 
* ESP32実装の payload 形式を踏襲 

**タスク**

1. Rust で `AraneaRegisterClient` 実装（HTTP POST / JSON）
2. is22 起動時に「未登録なら登録→cic保存→以降再利用」（ESP32のNVS相当を is22 settings or file に保存）
3. `Typedomain=araneaDevice / Type / ProductType=022 / ProductCode=0000` の整合（mobes側のTypeDomainは prefix=3 が既存） 
4. 受入テスト

   * is22 が登録でき、mobes側 userObject に lacisId/cic が反映される
   * is22 再起動しても再登録しない（同一ID・同一CICの再利用）

---

### EP-IS22-2: Summary / GrandSummary のDB・生成・送信（P0/P1）

is22側には ai_summary_cache が既にある 。これを基盤にするのが車輪の再発明回避です。

**タスク**

1. ai_summary_cache の拡張（いずれか）

   * A案: `summary_json JSON` 追加（あなたの提示フォーマットをそのまま保存）
   * B案: `summary_text` に JSON stringify を保存（検索性が落ちるのでA案推奨）
2. summary_type の整理

   * Summary（間隔）→ `hourly`
   * GrandSummary（時刻）→ `daily`
3. 生成ロジック

   * 期間内の detection_logs を集計（tid/fid/camera_id）
   * camera_context は cameras テーブルの camera_context を参照（is22側SSoT） 
4. 送信

   * is22 → mobes に “summary payload” を送る（認証は is22 AraneaRegisterで得た lacisId/cic）
5. 受入テスト

   * intervalで hourly が生成される
   * 指定時刻で daily が生成される
   * 生成物が DB に保存され、mobes 側に到達する

---

### EP-IS22-3: AIチャットの “データ提供API” を確定（P1）

会話の LLM 実行主体は **ParaclateAPP（mobes側）**が担うのが自然です（AI設定・マルチステージ設定が mobes にあるため ）。

is22 は「答えを生成する」のではなく、**答えに必要な事実データを返す**（SSoT遵守）。

**タスク**

1. is22 WebAPI に以下の “事実取得” API を定義

   * `GET /api/detection-logs?tid&fid&start&end&filters...`（既に近いものあり）
   * `GET /api/cameras?tid&fid...`
   * `GET /api/summaries?tid&fid&type&range...`
2. is22 は ai_chat_history を “ログ” として保持（すでにテーブルあり） 
3. 受入テスト

   * “赤い服の人” のような検索で、タグ/属性から該当ログ一覧が返る（精度は別途）

---

### EP-IS22-4: BQ同期の実装（P1→P0へ昇格候補）

bq_sync_queue は作られ、DetectionLogService から enqueue も既にある  が、Rust側の同期サービスが未実装（検索上見つからず）でした。

**タスク**

1. is22 内に `BqSyncService` を実装（キュー監視→バッチ送信→成功で synced_to_bq 更新）
2. リトライ・DLQ（failed）ポリシー
3. 受入テスト

   * キューに溜めて送る
   * 失敗時に retry_count が増える
   * 成功時に synced_to_bq が true になる

---

### EP-IS21-1: is21 推論拡張（顔特徴 / 車両No OCR）（P1）

is21 は現状でも PAR/YOLO の基礎があり 、あなたの要求「車両No OCR + 顔特徴」をここに追加。

**タスク**

1. is21 `/v1/analyze` の output_schema 拡張（例: `vehicle_ocr`, `face_features`）
2. is22 detection_logs に追加フィールド（または is21_log 内の正規化）
3. 受入テスト

   * OCR結果が返り、detection_logs に保存される
   * “車両No検索” が抽出結果から可能になる

---

## 5) タスク洗い出し（ParaclateAPP / mobes2.0 開発ライン）

### EP-MOBES-1: AI Model Settings に Paraclate 設定を追加（P0）

AIModelSettingsPage は **アプリ別の設定フィールドを定義する仕組み**を既に持っています 。
ここに `paraclate` を追加するのが最短です。

**最低限の追加（あなたの指示）**

* Paraclate endpoint を `https://www.example_endpoint.com` のプレースホルダーで保持できるフィールドを追加

**タスク**

1. `aiApplications/paraclate` を新設（displayName=Paraclate、説明、capabilities 等）
2. `APP_CONFIG_DEFINITIONS.paraclate` を追加し、少なくとも:

   * `endpoint`（text）
   * `summary.intervalSec`（number）
   * `grandSummary.time`（text “HH:mm”）
   * `retention.*`（画像保持やサマリー保持など）
3. 受入テスト

   * `/system/ai-model` の Applications タブで Paraclate が編集でき、保存される

---

### EP-MOBES-2: ParaclateAPP（テナント管理層サイドバー）を追加（P0）

あなたの指示「APPSHELLサイドバーのテナント管理層にParaclate APP」が必要。

**タスク**

1. サイドバーに Paraclate を追加（権限/表示条件は lacisOath に準拠）
2. ページ骨格（最初はバックエンド重点のため “薄いUI”）

   * 設定（endpoint / summary schedule）
   * サマリー閲覧（Summary / GrandSummary）
   * AIチャット（後述の multi-stage へ接続）
3. 受入テスト

   * tid に紐づいた施設一覧・設定が表示される（テナント越境しない）

---

### EP-MOBES-3: Paraclate AIアシスタント（マルチステージ×UX）実装（P1）

OperationReportToTasks で “プロンプト4層” の既存パターンがあります 。
Paraclate でも同じ思想で、**会話UXを崩さずに参照先を段階的に増やす**のが最短です。

**マルチステージ（決定案）**

* Stage A（Flash）: 意図分類・不足情報の抽出（超短）
* Stage B（Tool/検索）:

  * is22 から detection_logs / camera_context / summaries
  * mobes の facilityContext（settings）
  * （将来）rooms/reservations/horarium/celestial-globe
* Stage C（Pro）: 統合推論・長文サマリー生成（GrandSummary/深掘り回答）
* Stage D（Flash）: 最終応答（ユーザーUX最優先）

※既に別領域で stage1/2/3 設定があるため、この構造は既存思想と整合します 。

**UXの必須ルール（Paraclateのセマンティクス）**

* “監査”ではなく“見守り”口調（穏やか・対話的）
* ただし **不確実性は必ず明示**（曖昧な推測で断定しない）
* 「もっと遡って確認しますか？」のように **時間範囲を会話で伸縮**する

**実装タスク（最小）**

1. `paraclateChatApi`（Cloud Function / API）

   * 入力: tid/fid + userQuery +（必要なら）timeRange
   * 内部で is22 を参照して facts を集め、LLMに渡す
2. 出力は **(a)答え (b)参照した根拠ID（log_id等）(c)次の質問候補** を必ず返す
3. 受入テスト

   * 「今日は何かあった？」→ 件数/傾向/重要イベント/カメラ不調の有無を返す
   * 「赤い服きた女の人来なかった？」→ “候補ログ” を時刻+カメラで返し、追加探索を促す

---

## 6) “相互の齟齬が出やすいポイント”の先回り対策（統括としての指示）

### 6-1. TypeDomain/type の確定（最優先の合意事項）

* mobes 側の正規 TypeDomain には **araneaDevice（prefix=3）**が存在 
* よってあなたの指定（Prefix3 / ProductType=022 / ProductCode=0000）は実装可能。
* **結論**: is22 / is801 camera は **typeDomain=araneaDevice** に寄せる（既存 araneaDeviceGate をそのまま使えるため ）

### 6-2. Summary/GrandSummary の “DB二重化” を禁止

* ai_summary_cache は is22 に既にある 
* mobes 側に別DBを作るとSSoT違反になりやすい
* **結論**: サマリー実体は is22 に保存。mobes は「生成/閲覧UI/スケジュール投入」の役に徹する。

### 6-3. blessing は “境界破壊” なので必ず設計で固定

* blessing は仕様上強力で、実装を誤るとtid境界が崩壊する
* 現時点で既存実装としては未確認なので、**mobes2.0側で必ず設計→テスト→実装**の順で進める（P0扱い）

---

## 7) is21/22開発ラインへの指示（あなたの「今から行うこと」への具体化）

`code/orangePi/is21-22/designs/headdesign/Paraclate/investigation_report/` に、最低限この **インデックス＋4ドキュメント**を作成してください（統括指示として固定します）。

1. `INDEX.md`（調査項目・SSoT・決定事項・未決事項）
2. `AUTH_AND_TENANT_BOUNDARY.md`（tid/fid/blessing/permission・is22登録）
3. `SUMMARY_SPEC.md`（Summary/GrandSummary：DB/生成/送信JSON/テスト）
4. `AI_ASSISTANT_UX.md`（会話シナリオ、時間範囲伸縮、根拠提示、失敗時UX）
5. `TASKS_AND_TESTS.md`（依存順タスクリスト＋受入テスト）

※ is22 側は設計インデックス（00_INDEX）で “設計ドキュメント体系化” が既に進んでいるので 、この Paraclate フォルダは **「mobes連携と会話UXに絞った追加レイヤ」**としてMECEに分離してください。

---


