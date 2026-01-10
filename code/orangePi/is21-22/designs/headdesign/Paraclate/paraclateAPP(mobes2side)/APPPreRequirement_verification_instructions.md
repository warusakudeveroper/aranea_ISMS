以下は **ProclateAPP（= mobes2.0 側 Paraclate APP）設計管理担当として**、`code/orangePi/is21-22/designs/headdesign/Paraclate/BasicDesign` および `DetailedDesign`（DD01〜DD05）に整合する形で作る **「mobes2.0 側 詳細設計・実装指示（再提示）」**です。
（※この内容をそのまま mobes2.0 側 Issue/PR に貼り付けて運用できる粒度にしています）

---

## 0. 目的と前提

### 0.1 目的

is22（Paraclate Server）から mobes2.0（Paraclate APP）へ **Summary/Event を安全に ingest** し、**設定SSoT（AI Model Settings）**・**Chat UX（マルチステージLLM）**・**blessing（越境許可）**・**LacisFiles（画像保管/削除）**を **tid/fid 権限境界を破らず**に成立させる。

### 0.2 SSoT（絶対固定）

* テナント/施設/権限（tid/fid/permission）SSoT = **mobes2.0**
* カメラ台帳/検知ログ/サマリーSSoT = **is22**
* 長期分析SSoT = **BigQuery**（テーブル定義SSoTは mobes2.0 側）
* AI設定SSoT = **mobes2.0 AI Model Settings**（aiApplications + overrides）
* 画像保管SSoT = **LacisFiles**（「is22が直接StorageにPUTしない」方針）

> SSoTの重複禁止：mobes2.0 側 Firestore に “検知ログの正本” や “サマリーの正本” を作らない（必要なら **TTL付きHot Cache** として明示する）。

---

## 1. 命名・ID・型（齟齬禁止の固定値）

### 1.1 表記

* UI表示名: **Paraclate**（サブタイトル: *mobes AI control Tower*）

### 1.2 is22 / camera の typeDomain/type（固定）

* is22: `typeDomain=araneaDevice`, `type=ar-is22CamServer`, `ProductType=022`, `ProductCode=0000`
* カメラ: `typeDomain=araneaDevice`, `type=ar-is801Camera`, `ProductType=801`, `ProductCode=ブランドコード(4桁)`

### 1.3 lacisID 形式

`[Prefix=3][ProductType(3桁)][MAC(12桁)][ProductCode(4桁)]` を前提にする。

---

## 2. mobes2.0 側：実装スコープ（MECE）

mobes2.0 側を以下 6 モジュールに分割（重複なし）：

1. **AI Model Settings 拡張（paraclate appId）**
2. **Paraclate APP API（ingest/config/connect/chat）**
3. **blessing（越境許可）管理（発行/検証/監査）**
4. **LacisFiles（スナップショット保存/削除）連携**
5. **Pub/Sub（設定変更通知・状態通知）**
6. **Tenant Admin UI（薄いUI：設定と疎通・受信状況中心）**

---

## 3. P0（結合試験ブロッカー）実装指示

### P0-1. AI Model Settings に `paraclate` を追加（設定SSoT化）

**狙い**：is22 が参照する “Paraclate endpoint / schedule / retention / attunement” を mobes2.0 側で一元管理する。

#### (A) Firestore: `aiApplications/paraclate` を作成

既存 `AiApplicationDocument.configurationDefaults` を利用する。
最低限の `configurationDefaults`（例）：

* `endpoint`（初期値: `https://www.example_endpoint.com`） 
* `summary.reportIntervalMinutes`（number）
* `grandSummary.times`（string[] “HH:mm”）
* `retention.retentionDays`（number）※DD03/Configで単一値 `retentionDays` を返す設計
* `attunement`（object）

> **endpoint の意味を固定**：is22 DD03 は `{endpoint}/ingest/summary` 等を叩くため、原則 `endpoint = https://.../api/paraclate` のように **/api/paraclate まで含む “ベースURL”**として運用する。

#### (B) UI: `/system/ai-model` の Applications タブで編集できるようにする

`AIModelSettingsPage.tsx` の `APP_CONFIG_DEFINITIONS` に `paraclate` セクションを追加する（既存パターン踏襲）。

#### (C) override（tenant/facility）を公式に使う

mobes2.0 には `resolveAiApplicationConfig(appId,{tenantId,fid})` があり、
`application template → tenant override → facility override` の順に deep merge される。
→ Paraclate の endpoint/schedule/retention はこの仕組みを正として実装する（車輪の再発明禁止）。

**受入条件**

* `aiApplications/paraclate` が存在し、UIで編集→保存→反映される。

---

### P0-2. Paraclate APP API（mobes2.0）を実装

is22 側詳細設計（DD03）が “Paraclate APP側API” を前提にしているため、以下は **mobes2.0 側で必須**。

#### (A) エンドポイント一覧（最低限）

* `POST /api/paraclate/ingest/summary`（Summary受信）
* `POST /api/paraclate/ingest/event`（Event受信 / Emergency）
* `GET  /api/paraclate/config/:tid?fid=`（設定取得）
* `POST /api/paraclate/connect`（疎通）

#### (B) 認証（必須）

* is22→mobes の payload は `lacisOath: { lacisID, tid, cic, blessing? }` を含む。
* **mobes2.0 側では deviceStateReport と同等の “DeviceAuth 検証” を行う**（CIC/所属/状態の正規チェック）。
  ※JSONキーが `lacisID`（大文字）で来るので、内部で `lacisId` に正規化してから検証する（is22側の変更を強制しない）。

#### (C) tid/fid 境界（推測禁止）

* is22 DD03 は **fid="0000" への送信を禁止**（境界違反）としている。
  → mobes2.0 側でも、受信データから fid を **確定**できないケースは 400/403 で落とす（“なんとなく”処理禁止）。

**fid の確定ルール（mobes2.0）**

1. `cameraContext[*].fid` を走査し、**全件同一**ならそれを fid とする。
2. `cameraContext` が空の場合は `cameraDetection` だけでは fid を確定できない可能性が高いので、**原則 400**（必要なら is22 payload に fid を追加する変更提案を出す）。
3. fid が複数混在なら **403**（越境/混在として拒否。必要なら blessing を伴う別設計に回す）

#### (D) ingest/summary の保存方針（SSoT遵守）

* サマリー正本は is22（ai_summary_cache）なので、mobes側は **受信監査 + UI表示用のHot Cache（TTL）**に限定。
* 受信成功時レスポンスは DD03 に合わせる。

#### (E) ingest/event とスナップショット（LacisFiles）

BasicDesign の「画像は is22 直PUT禁止、ingestが受信→LacisFilesへ」要求を満たす。

* Event payload に `snapshot`（base64 or 署名URL等）を **拡張フィールドとして許可**し、存在する場合は LacisFiles に保存。
* LacisFiles の既存仕組みは **Firebase Auth 前提**で callable（getUploadUrl等）なので、is22 からの deviceAuth ではそのまま呼べない。
  → **Paraclate ingest 用に「deviceAuth版の保存処理」**を新設する（既存LacisFilesのデータ構造/監査ログの流儀は踏襲）。
* 返却値は is22 が `detection_logs.snapshot_url` に持てる形（例：`storagePath` または `lacisFiles://{tid}/{fileId}` 形式）を返す。DD04 は `snapshot_url` を BigQuery に入れる設計になっている。

**受入条件**

* is22 → `/ingest/summary` が 200 で応答。
* 不正lacisOathは 401、tid/fid mismatch は 403。
* snapshot あり event が LacisFiles に保存され、is22 側に参照値が返る。

---

### P0-3. `GET /api/paraclate/config/:tid` の実装（is22 の設定同期を成立させる）

DD03 の config_sync は `{endpoint}/config/{tid}` を GET して差分判定する。
よって mobes2.0 側は **返却JSONをDD03形式で固定**する。

#### 実装要点

* `resolveAiApplicationConfig('paraclate', { tenantId: tid, fid? })` を使用（deep mergeの正本）。
* `configs[]` を fid 単位で返す（fid省略時は「そのtidが持つfid一覧」を引いて全件返す）。
* `updatedAt` は **aiApplications + overrides の max timestamp**を返す（is22 が差分判定に使う）。

#### セキュリティ注意（アンアンビギュアス宣言）

DD03 は「全リクエストに lacisOath 必須」も明記している。
一方、config_sync の疑似コードでは未認証GETになっているため、**ここは設計上の齟齬（現時点アンアンビギュアス）**。
→ mobes2.0 側は **config取得にも lacisOath を必須**（ヘッダ or クエリ）とし、is22 側に合わせ込み修正を依頼する（セキュリティ優先）。

---

### P0-4. blessing（越境権能）管理

blessing は “境界破壊” なので、**設計→テスト→実装**の順で P0 扱い。

* 発行条件：permission>=91（仕様）
* 保存先：`systemBlessings`（BasicDesign案）
* ingest 時に `blessing` が入っていたら必ず検証し、allowedTids/Fids/LacisIDs/期限を満たさない場合は 403。
* 監査ログ必須（誰が/いつ/何を許可したか）

---

### P0-5. BQ（テーブル定義SSoTは mobes2.0）と命名衝突の回避

is22 DD04 は `semantic_events` と `summaries` を BigQuery に作る前提で、**テーブル定義のSSoTは mobes2.0 側**と明記。
ただし mobes2.0 には既に “SemanticTags 用の ai_logs.semantic_events” という概念があり得るため（運用衝突リスク）、**dataset を分離**する（例：dataset=`paraclate`）。

**受入条件**

* dataset/テーブルが衝突しない命名で作成され、is22 が insert できる。

---

## 4. P1（UX価値の核）実装指示

### P1-1. `POST /api/paraclate/chat`（マルチステージLLM）

会話UX要求：

* “見守り”口調、ただし不確実性は必ず明示、時間範囲を会話で伸縮。
* 出力に「答え + 根拠ID + 次の質問候補」を必須。

データ参照は is22 API + mobes facilityContext +（将来）rooms/reservations/horarium/celestial-globe + BQ。

### P1-2. Tenant Admin UI（薄いUIで開始）

* AppShell（Tenant Admin セクション）に Paraclate を追加（最小）。
* 機能：設定編集（overrides）、接続テスト、受信状況、Chat のみ。

---

## 5. mobes2.0 側で追加すべき araneaDevice 詳細スキーマ（推奨）

mobes2.0 には `DEFAULT_DETAIL_SCHEMAS` があり、araneaDevice type ごとに state/config を差異化している。
→ `araneaDevice / ar-is22CamServer` と `araneaDevice / ar-is801Camera` を追加し、最低限:

* is22: device state（稼働/queue/lastSeen など）＋ Paraclate設定の反映先（必要なら）
* camera: rid/did/preset/context など、Paraclateで編集対象になる最小セット
  ※ is22 詳細設計ではカメラは仮想araneaDeviceとして登録する方針。

---

## 6. テスト計画（黄金ルール：テスト無し完了禁止）

### 6.1 Unit/Integration（最低限）

* config resolver（override合成）のユニットテスト（deep merge順序）
* ingest/summary の auth/validation（401/403/400）
* blessing 検証（期限/スコープ/issuer権限）

### 6.2 E2E（Chrome実機）

is22 側 INDEX の E4「Paraclate送信→mobes受信確認」が結合試験の核。
mobes側 E2E 受入条件：

1. `/config/:tid` が返る → is22 が設定同期できる
2. `/ingest/summary` が通る（正/異常）
3. `/connect` が latency 付きで返る
4. snapshot event → LacisFiles 保存 → 参照値が is22 に返る
5. retention job が動き容量が増え続けない

---

## 7. 未決事項（アンアンビギュアスに宣言）

* **config endpoint の認証**：DD03 では “全リクエスト lacisOath必須” と読める一方、config_sync 擬似コードは未認証GET。
  → mobes2.0 は “必須認証” で実装し、is22 側に合わせ込み修正依頼（セキュリティ優先）。
* **snapshot_url の形式**：DD04 は `snapshot_url STRING` を想定。
  → “短寿命署名URL”を直接入れると後から無効化されるので、`storagePath` など **恒久参照子**を入れる方が運用に強い（この形式を設計で確定する必要あり）。

---

## 8. （重要）GitHub へのコメント反映について

今回、こちらから aranea_ISMS Issue #112 にコメントを投稿しようとしましたが、手元の GitHub 連携ツール側で **`add_comment_to_issue` が利用不可（Resource not found）**となり、投稿できませんでした。
なので、このメッセージ本文を **そのまま mobes2.0 側の実装Issue**、または aranea_ISMS 側の該当Issueに転記して運用してください。

---

