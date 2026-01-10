# review1


レビュー完了です（Issue #112 / commit **fdf364e**：DD01〜DD05 + INDEX）。
※この環境からGitHub Issueへ直接コメント投稿しようとすると、ツールが「Resource not found」になって失敗することがあるため、**貼り付け用コメント**も併記します。

---

## ✅ 総評（設計としては良い／実装へ移行できる骨格）

* **詳細設計の粒度が揃っている**（目的→依存→DB→API→モジュール→フロー→エラー→テスト→MECE/SOLID→SSoT）。DD01〜DD05を役割分割し、INDEXで依存順（DD01→DD05/DD02→DD03→DD04）に落ちているのは妥当。
* lacisID生成ルール（is22: `3022{MAC}0000` / カメラ: `3801{MAC}{ProductCode}`）は明確で、登録テーブル/カメラ台帳拡張の設計も整っています。
* Summary JSONフォーマットが新形式（`lastDetectAt` + `cameraContext=object` + `cameraDetection=object[]`）で統一され、SSoTとして設計書に含まれているのは良いです。

---

## ❗P0（実装開始前に必ず潰すべき“仕様割れ/事故ポイント”）

ここが残ると、**tid/fid境界が壊れる** or **運用時刻がズレる** or **IDが割れて後で回収不能**になります。

### P0-1) DD03 client.rs の fid が「推測/ハードコード」になっている

`send_payload` で fid を payload から取れないため、`summaryID`の有無で `"0150"` を返すような **仮実装(TODO)** が残っています。
➡️ **必須修正方針**

* fid は **scheduler の schedule（scheduled_reports / paraclate_config）**から確定値として渡す
* 送信キュー `paraclate_send_queue` は tid/fid を **enqueue時点で確定**して保持（推測禁止）

### P0-2) Paraclate endpoint の“意味”が文書内で揺れている

DD03の外部依存は `POST /api/paraclate/ingest/summary` 等を定義している一方、client 実装例は `endpoint + "/ingest/summary"` を前提にしており、**base URL がどこまで含むか**が曖昧です。
➡️ **必須確定**

* `paraclate.endpoint` を「`.../api/paraclate` まで含む base」など **1つに固定**し、全ドキュメント/実装例で統一
* その上で client の path 連結を統一（例: `"{base}/ingest/summary"`）

### P0-3) SummaryID の“外部参照ID”が割れている

* Summary JSON の `summaryID` は uuid を生成している例
* BQ 送信 payload は `sum_<numeric>` のようにDB数値IDベース
  ➡️ **必須確定**（おすすめ）
* “外部参照ID” を 1つに固定する

  * 例A: `summary_uid`（UUID）列を ai_summary_cache に追加し、Paraclate送信/BQ送信は uid を使う
  * 例B: `SUM-{summary_id}` のように **数値IDを文字列化して固定**し、payload/BQで同一値を使う
    （いまのままだと「Paraclateで見えるsummaryID」と「BQ上のsummary_id」が一致しません）

### P0-4) GrandSummary の指定時刻が UTC 前提に見える

`schedule_times` が `"09:00"` 等のローカル時刻を想定しているのに、計算は `Utc` で組み立てています。
➡️ **必須確定**

* scheduled_times を **JST固定**とするなら、JSTで next_run を計算する（施設が日本なら事故が減る）
* 施設が複数TZになり得るなら、`paraclate_config` に `timezone` を追加して計算

### P0-5) DD04 の synced_to_bq 更新が “WHERE id” 固定

processor が `UPDATE {table} SET synced_to_bq = TRUE WHERE id = ?` を前提にしています。
ai_summary_cache のPKは `summary_id` なので、そのままだと不整合になります（tableによって主キーが違う）。
➡️ **必須修正**

* `table_name → primary_key_column` のマップを持つ（例: detection_logs.id / ai_summary_cache.summary_id / is21_logs.id 等）

### P0-6) DD01: JSONキー（tenantPrimaryAuth等）と Rust struct の命名がズレる可能性

DD01の Request 例は `tenantPrimaryAuth`（camelCase）だが、struct側は `tenant_primary_auth` で serde指定が見えません（例にそのまま合わせないとデシリアライズで事故ります）。
➡️ **必須修正**

* `#[serde(rename_all="camelCase")]` 等で **API仕様と完全一致**させる

---

## P1（改善提案：後回し可だが事故りやすい）

* 送信キューの backoff が `created_at` 基準なので、途中再送の間隔が意図とズレます。`last_attempt_at` を持つのがおすすめ。
* ConfigSync の `GET /config/{tid}` は少なくとも lacisOath を要求してよい（設定SSoT保護の観点）。

---

## Issue #112 に貼り付け用レビューコメント（コピペ）

```md
統括アドバイザーとして commit fdf364e（DD01-DD05 + INDEX）をレビューしました。

✅ 良い点
- 詳細設計の粒度（目的/依存/スキーマ/API/モジュール/フロー/エラー/テスト/MECE/SOLID/SSoT）が揃っており、依存順も妥当（DD01→DD05/DD02→DD03→DD04）。
- lacisID生成ルール（is22=3022.. / camera=3801..）と Summary JSON 新フォーマットが一貫。

❗P0（実装前に必ず確定）
1) DD03 client.rs に fid の推測/ハードコード(TODO)が残る。tid/fid境界を壊すので、schedule/configから確定fidを渡す設計へ変更。
2) Paraclate endpoint の意味が揺れる（/api/paraclate/... vs /ingest/...）。endpoint base を1つに固定し、全記述統一。
3) summaryID が割れている（payloadはuuid、BQは sum_<numeric>）。外部参照IDを1つに固定（summary_uid追加 or SUM-<auto_increment>等）。
4) GrandSummary scheduled_times がUTC計算に見える。ローカル時刻（JST等）前提ならtimezone仕様を固定して計算へ反映。
5) DD04 synced_to_bq 更新が WHERE id 固定。ai_summary_cache は summary_id のため、テーブルごとのPKマップが必要。
6) DD01 JSONキーと Rust struct の命名がズレる可能性。serde rename_all 等でAPI例と完全一致させる。

P1（改善）
- send_queue backoffは created_at 基準でズレるので last_attempt_at を推奨。
- GET /config/{tid} にも lacisOath を付与（最低限の認証）を推奨。

上記P0を潰せば、設計として実装フェーズ移行OKです。
```

---

必要なら、このP0を「そのまま実装Issueに落とせる粒度（チェックリスト＋受入テスト）」に再編して、**is22実装フェーズ Phase1（DD01）開始用のIssue雛形**まで作ります。


---

# review2

Issue #112 / commit **fdf364e**（DD01–DD05 + INDEX）を突合レビューしました。
結論：**構成と依存順は非常に良い**ですが、**P0が3点（実装前に必ず直す）**あります。ここを直せば「実装フェーズ移行OK」です。

---

## ✅ 良い点（この方向でOK）

* INDEXで **依存 → マイグレーション順 → API一覧 → タスク → E2E** が揃っていて、実装に落とせる粒度です。 
* DD分割（登録 / サマリー / 送信 / BQ / カメラ台帳）もMECEで、責務境界が明確です。 

---

## 🚨 P0（ブロッカー：実装開始前に必ず修正）

### P0-1) **カメラ lacisID の ProductCode が仕様違反（英字入り）**

DD05 では `T001/H001...` のような **英字入り ProductCode** を前提にしています（例・テーブル・APIレスポンス・テストまで一貫して英字）。  

しかし mobes2.0 の LacisID バリデーションは prefix=3/4 の場合、**productCode は4桁数字のみ**で検証されています（`^[34]\d{3}[0-9A-F]{12}\d{4}$`）。 
araneaSDK 側のスキーマ仕様も **productCodeは数字4桁**です。 

**修正方針（決定推奨）**

* camera_brands.product_code を **0001/0002… の4桁数字**へ統一（Tapo=0001 等）
* DD05 の **例 / SQL / テスト / API例**をすべて数字に修正（`...T001` を廃止）

---

### P0-2) **ParaclateClient が fid をハードコード（多施設・権限境界が崩壊）**

DD03 の client.rs 例で、`fid` が `summaryID` から **固定 “0150”** にされ、fallback が **“0000”** になっています。 
これは **tid/fid不一致検知やblessing条件**の根幹を壊します（また multi-fid が動きません）。

**修正方針（どちらか必須）**

1. **payloadからfidを推測しない**：`send_payload(payload_type, payload, tid, fid)` のように **呼び出し側（scheduler）から fid を必須引数で渡す**
2. SummaryPayload 仕様（SSoT）を更新して、payloadのrootに `fid` を追加して**明示**（いずれにせよ “0000 fallback” は禁止）

---

### P0-3) **DD01 のサンプル実装が型的に矛盾（コンパイル不可）**

DD01 の register_device 例が `if let Some(existing) = self.get_registration_status().await?` になっています。 
一方で get_registration_status は `Result<RegistrationStatus, ...>` を返す定義です。 

**修正方針**

* `let existing = self.get_registration_status().await?; if existing.registered { ... }` に修正（Optionを使わない）

---

## 🟠 P1（品質・運用：直すと事故が減る）

### P1-1) SummaryID の正本がブレる（DBはBIGINT、payloadはUUID）

ai_summary_cache の主キーは `summary_id BIGINT AUTO_INCREMENT`。 
しかし payload_builder 側の SummaryOverview は `uuid::Uuid::new_v4()` を `summary_id` に入れています。 
さらに仕様サンプルでは summaryID は `{summary_id}`（DB idに見える）になっており、ドキュメント内でも正本が揺れます。 

**推奨**：`summaryOverview.summaryID` は **DBのsummary_id（文字列化）**を正本にし、必要なら `summaryUUID` を別途追加。

### P1-2) 実テナント情報（email/cic等）がドキュメント例に直書き

DD01 の Request 例に `soejim@mijeos.com` や実値のように見えるCICが入っています。 
**推奨**：`{TENANT_PRIMARY_EMAIL}` 等のプレースホルダに置換（設計書は再配布されがちなので）。

### P1-3) スケジューラのタイムゾーン

DD02 のGrandSummary生成は `Utc` で日付境界を切っています。 
運用が Asia/Tokyo 基準なら、日次境界がズレます。**Asia/Tokyo明示**が安全。

---

## Issue #112 に貼るレビューコメント（コピペ用）

（こちらの環境から GitHub へ直接コメント投稿しようとしましたが、コネクタが利用不可になってしまい投稿できませんでした。なので貼り付け用を出します。）

```text
再レビュー結果（commit: fdf364e）

✅ 全体：INDEXで依存順→タスク→E2Eチェックまで揃っており、実装に落とせる粒度。DD01→DD05分割もMECE。

【P0（実装着手前に必ず修正）】
P0-1) カメラ lacisID の ProductCode が仕様違反（英字入り）
- DD05で T001/H001… をproductCodeにしているが、mobes2.0のlacisID validateは prefix=3/4 の場合 productCodeは4桁数字のみ（^[34]\d{3}[0-9A-F]{12}\d{4}$）。araneaSDK側もproductCodeは数字4桁。
→ camera_brands.product_code を 0001/0002… の数字4桁に統一し、例/SQL/テスト/APIレスポンス例を全修正。

P0-2) ParaclateClient が fid をハードコード
- client.rs例で fid を summaryIDから固定"0150"、fallback"0000"。
→ payloadから推測禁止。scheduler側から fid を必須引数で渡す or payload rootに fid を追加して明示（SSoT更新）。0000 fallbackは禁止。

P0-3) DD01サンプル実装が型的に矛盾（コンパイル不可）
- register_device内で get_registration_status() を Option 扱いしているが、関数定義は Result<RegistrationStatus>。
→ let existing = ...; if existing.registered { ... } に修正。

【P1（品質・運用）】
- SummaryID正本：DBはBIGINTだがpayloadはUUID。summaryOverview.summaryIDはDB idに統一し、必要なら summaryUUID を追加。
- DD01のtenantPrimaryAuth例は実値っぽいのでプレースホルダ化推奨。
- GrandSummaryの時刻計算はAsia/Tokyoを明示（UTCだと日次境界ズレ）。

P0が直れば「実装フェーズ移行OK」で良いです。
```

---

