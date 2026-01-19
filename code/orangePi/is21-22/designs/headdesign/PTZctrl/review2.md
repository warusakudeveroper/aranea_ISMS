以下は、あなたが作った「IS22総合レビュー依頼（2026-01-20）」の観点（アーキテクチャ / カメラ登録 / PTZ / AccessAbsorber / Paraclate / テスト）に沿って、**現時点のIS22実装**をレビューしたものです。
（根拠は、該当ファイルの引用で示します）

---

## 0. 結論サマリ（重要度順）

### ✅ 良い点（そのまま伸ばせる）

* **IpcamScan（スキャン）は単一実行ガード + 中止(Abort) + 詳細ログ**が揃っていて、運用で強い設計です（例：実行中ジョブを弾くConflictや中止処理）
* **AccessAbsorberは「優先度・プリエンプション・再接続インターバル・排他ロック」をDB＋API＋WS通知まで含めて実装**しており、Tapo等の同時接続制限を吸収できる形です 
* **Paraclate送信キューはDB永続化＋payload種別ごとのラップ処理＋GrandSummaryの欠損スキップ**など、現場運用の“壊れにくさ”に寄っています

### 🚨 ブロッカー（先に直したい）

1. **リポジトリにCargoビルド成果物（target/）がコミットされている**
   `code/orangePi/is22/tests/e2e_paraclate/target/...` が大量に存在します（例：depsの`.d`ファイル）
   → repo肥大/差分ノイズ/セキュリティ面（ローカルパス露出）でも致命的。即 `.gitignore` + 履歴除去推奨。

2. **テストスクリプトにDBパスワードが平文で埋め込み**
   `DB_PASS="mijeos12345@"` がそのまま入っています
   → private運用でも事故源。少なくとも `.env` / 環境変数へ移し、READMEはサンプルに。

3. **フロントエンドのParaclate型定義がバックエンド実装と食い違い**
   例：フロント `ParaclateStatus.connectionStatus` に `'connecting'` がある一方、バックエンド `ConnectionStatus` は `Connected/Disconnected/Error` のみ。
   またフロント `ParaclateConnectResponse` は `success/message/latencyMs` を想定 しますが、バックエンドは `connected/endpoint/config_id/error` 形式です。
   → UIが壊れる/“なんとなく動く”の温床。**API契約の統一（OpenAPI生成 or shared types）**が必要。

---

## 1. アーキテクチャ（モジュール分割・依存・エラーハンドリング）

### 1.1 分割の良さ

* `main.rs` で主要サービスをDI的に束ねていて、責務の塊が見える（IpcamScan / Admission / AccessAbsorber / Ptz / Paraclate / など）
* グローバルタイムアウトをDB settings から読むなど、運用パラメータが“コード固定”に寄りすぎていない 

### 1.2 気になる点（設計の一貫性）

* **“接続制御”が3レイヤに分散**して見えます

  * RtspManager（RTSP多重防止）を main で初期化
  * AdmissionController（モーダルlease）を main で初期化
  * AccessAbsorber（ブランド/目的/プリエンプション）
    → どれが最終SSoTかが崩れると「UIは許可したのに実ストリームは拒否」等が起きるので、**最終的に“どれが判定主体か”を決めて、他は補助に寄せる**のが安全です。

---

## 2. カメラ登録（Category B/C・フィールド継承・PTZ自動検出）

### 2.1 API面の整理は良い

ルーティングとして、scan→verify→approve/force-register→activate までがAPIで揃っています

### 2.2 Category分類（フロント側）

フロントは **backendの`category`（SSoT）を優先し、無い場合のみ推定**という方針で、仕様として筋が良いです。

### 2.3 approve_device / force_register の期待仕様（テストドキュメントより）

* approve_device は「manufacturer/model継承」「ONVIFプローブでptz_supported自動設定」「onvif_endpoint生成」を行う想定
* force_register は pending_auth を作る想定

### 2.4 レビュー観点での “穴”

* **テストが `rtsp_sub` を検証していない**
  テストで検証しているフィールド一覧に `rtsp_sub` が入っていません。
  一方で `rtsp_sub未設定が致命的` という設計ドキュメントが存在します。
  → ここは回帰しやすいので、**Category B登録時に `rtsp_main/rtsp_sub` 両方が埋まること**をテストに追加するのが強いです（特にTapo/Vigi）。

* **scan結果のDB取得が“全件取得→Rustでフィルタ”**（list_devices）
  コメント通り簡易実装ですが、大規模運用でipcamscan_devicesが増えると重くなります。
  → `WHERE`条件（subnet/family/status/verified）をSQL側へ寄せるだけで伸びます。

---

## 3. PTZ制御（リース管理・タイムアウト・ベンダー対応）

### 3.1 ベンダー対応は明示的

現状、familyが `tapo/vigi` 以外は “未対応”で弾く作りです。
→ 仕様としてはOK。未対応時のUI表現（ボタン無効 + 理由表示）が揃うと運用が楽。

### 3.2 “止める”挙動が実質No-op（要注意）

`execute_ptz_stop_static` が **「停止API呼ばない。カメラ側タイムアウトに任せる」** になっています。
さらに nudge はバックグラウンドspawnで stop を呼びますが、その stop がNo-op なので、**nudgeの意味が崩れる**可能性が高いです。
→ 改善案：spawn内で state/config_store を渡して通常の stop を実行するか、最低でも family別に stop 実装を用意する。

### 3.3 Lease（AdmissionController）とPTZの結合が弱そう

フロントのPtzOverlayは `leaseId` を前提にhookを呼ぶ設計です。
一方バックエンド側は PTZ API が routes で普通に公開されているだけに見えます。
→ **PTZ操作が“モーダルlease保持中のみ許可”**なら、バックエンドでlease検証を必須化した方が事故が減ります（誤操作/多重操作/他画面からの操作）。

---

## 4. AccessAbsorber（優先度制御・同時接続制限）

### 4.1 実装の方向性はかなり良い

* 同時接続上限に達したら、`allow_preempt` が true のときプリエンプションを試みる
* 再接続インターバルが短い場合はsleep、それ以上はエラー返し＋イベントログ
* プリエンプション発生時は WebSocket に「ユーザー向け文言」を投げる設計になっている

### 4.2 改善ポイント（競合・整合性）

* `try_preempt` はセッションをdeleteして前提を作っています。
  同時に複数クライアントが acquire した場合、**DBトランザクション/ロックの粒度**が重要になります。
  → 可能なら “Acquire処理をストアド/トランザクションで一貫” に寄せる（DB_SCHEMA.mdの方向性はそれに近いです）。

---

## 5. Paraclate連携（キュー管理・ペイロード形式）

### 5.1 良い点

* Queue item を `Summary/GrandSummary/Event/Emergency` で分け、送信時に mobes側の要求形式へラップしている
* GrandSummaryの `shiftStart/shiftEnd` が無い古いデータは **Skipped** にしてリトライ地獄を避けている
* FID所属検証（SKIP_FID_VALIDATIONあり）で最低限の境界を作っている

### 5.2 気になる点（設計の一貫性）

* `connect()` は「引数endpointは保存用」とコメントしつつ、実送信は固定URL定数です。
  → **endpointフィールドが“実運用で意味を持たない”**状態になりがち。環境切替（dev/stg/prod）や障害時の迂回で困ります。

  * 改善案：`endpoint` をベースURLとして、パスだけ固定にして組み立てる / もしくは endpoint を廃止して固定化を明確にする。

* Pub/Sub Pushの署名検証が「将来実装」扱い 
  → 外部から叩ける可能性があるなら、最低でも **共有シークレット/HMAC** を先に入れた方が安全。

---

## 6. テストスイート（カバレッジ・エッジケース・CI/CD）

### 6.1 良い点

* “サーバーローカルで常時実行可能” を意識した構造（run_all_tests.sh が test_*.sh を拾う）
* テストはDBに直接注入→APIを叩くので、登録フローのE2Eとして筋が良い

### 6.2 重大な問題（運用・セキュリティ）

* **平文DBパスワード＋固定IPがリポジトリに直書き**
  → `MYSQL_PWD` や `.env` / GitHub Actions secret に寄せるべきです（`mysql -p`はプロセスリストにも出ます）。
* **Cargoのtarget成果物がコミットされている**
  → 即削除＆`.gitignore`化。

### 6.3 テストの改善提案（短期で効く）

* `rtsp_sub` を検証項目に追加（前述）
* “重複登録”“不正fid”“MAC変更/復元” のケースを追加（レビュー依頼ドキュメントも同方向）
* テスト結果の “Tests run: 15 / passed: 32” みたいな表示は混乱を呼ぶので、**SuiteとAssertionを分けたカウント**に揃える（現状スクリプトの設計上そうなり得る）

---

## 7. 指摘事項一覧（アクション付き）

| 重要度     | 領域             | 指摘                                     | 根拠 | 推奨アクション                             |
| ------- | -------------- | -------------------------------------- | -- | ----------------------------------- |
| Blocker | Repo衛生         | target/ がコミットされている                     |    | `.gitignore` + 履歴から除去（filter-repo等） |
| Blocker | Security       | テストにDBパスワード直書き                         |    | env化・secret化・READMEはサンプル化           |
| Blocker | FE/BE契約        | Paraclateの型が不一致（status/connect/queue等） |    | OpenAPI/型共有でSSoT統一                  |
| High    | PTZ            | nudge stop が実質No-opで危険                 |    | 実stop実装（family別）+ spawn設計見直し        |
| High    | CameraReg      | rtsp_sub をテストが見ていない                    |    | E2Eにrtsp_sub検証追加                    |
| Medium  | Paraclate      | endpoint保存するが実送信は固定URL                 |    | base endpointからURL生成 or endpoint廃止  |
| Medium  | AccessAbsorber | acquire/preemptの競合耐性を要確認               |    | トランザクション/ストアド寄せを検討                  |
| Medium  | IpcamScan      | list_devicesが全件取得→Rustフィルタ             |    | SQL側WHEREに寄せる                       |

---


