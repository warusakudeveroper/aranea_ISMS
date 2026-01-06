以下は、Doorbellはポートスキャンで特定困難＝SDM必須）を前提にした **is22 Camserver（mAcT）への “Nest Doorbell 追加” 実装指示書（全体概要）**です。
既存の is22 実装（Axum + ConfigStore + go2rtc + PollingOrchestrator + IpcamScan + フロントReact/shadcn）に **最小の侵入で綺麗に増設**する方針で書きます。

---

# 1. 背景と結論（設計判断）

## 1.1 背景

* is22 の IpcamScan は「L2ならARP→OUI→ポート→ONVIF/RTSP…」の段階証拠で候補抽出します（Stage 0-6/7が実装済み） 
* ただし Doorbell（特にBattery/Cloud寄り）は **ローカルでRTSP/ONVIF/ディスカバリが露出しない**ため、IpcamScanの「カメラ関連ポートが開いている端末だけ保存する」条件で母集団から落ちます（camera_ports判定で保存対象外） 
* したがって、**Nest Doorbellは「サブネットスキャンの延長」ではなく「クラウド登録（SDM）」ルートが必要**。

## 1.2 結論（追加すべきルート）

* **IpcamScan（サブネットスキャン）**：Tapo/VIGI/一般ONVIF/RTSPカメラを登録
* **SDM登録（新設）**：Google Nest Doorbell を登録（IPベースの検出に依存しない）
* 登録後は、mAcT側の扱いを揃える：

  * **go2rtc に “camera_id = source名” として追加**（配信は既存と同じ）
  * **静止画は PollingOrchestrator が巡回取得 → is21に投げる**（既存と同じ）

---

# 2. 既存構造（SSoT/拡張ポイント）

is22 は既に以下の骨格を持っています。

* **ConfigStore（SSoT）**：カメラ台帳＋settings（JSON）をDBから読み書き・キャッシュ 
* **API Routes**：`/api/cameras`、`/api/ipcamscan/*`、`/api/streams/*`、`/api/snapshots/*`、`/api/ws` などが既にある 
* **StreamGateway（go2rtc adapter）**：`/api/streams`へ `{name, source}` をPOSTしてソース追加できる 
* **SnapshotService**：RTSP（ffmpeg）→失敗したら `camera.snapshot_url` のHTTP取得へフォールバック 
* **PollingOrchestrator**：有効カメラを順に巡回し、snapshot→保存→is21推論→ログ/通知（逐次処理） 

この上に “SDM統合” を乗せます。

---

# 3. 要件（MECE）

## 3.1 機能要件

A. **SDMアカウント連携（設定モーダル）**

* SDM（Device Access / Smart Device Management）に必要な値（project_id / client_id / client_secret / refresh_token 等）を登録・更新
* 登録済み状態/疎通状態を表示（Connected / Error / Not configured）

B. **SDMデバイス一覧取得**

* SDM側から “カメラ/ドアベル” デバイス一覧を取得し、mAcTのUIで見える

C. **SDMデバイスのカメラ登録**

* 1デバイス → is22の `cameras` に追加（SSoT）
* 登録時に「施設/サブネット整理に乗る」ため、**fid/tid/場所**は手入力 or サブネット一覧から選択
* 登録後に **go2rtcへ source を追加**し、既存と同じ `/api/streams/:camera_id` で見えるようにする 

D. **静止画取得**

* PollingOrchestratorが巡回で静止画を取得して is21 へ送る（既存のループに組み込む）
* 実装方式は2案：

  1. **go2rtc 統合（推奨）**：go2rtcから `frame.jpeg` を引く（HTTP）
  2. **SDM直接**：SDM APIで画像取得（対応トレイト確認が必要、仕様固定しない）

E. **長時間運用**

* SDM認証・go2rtcソースが落ちても復旧できる（再登録/再同期）
* 既存の「モーダル許可/拒否（Admission）」など全体性能は優先（モーダル拒否で守る）

## 3.2 非機能要件

* **SSoT厳守**：カメラ台帳は `cameras` テーブル、SDM連携情報は `settings` を正本
* **SOLID**：SDMロジックは独立モジュール化し、routesは薄く、ConfigStore/StreamGatewayとは疎結合
* **アンアンビギュアス**：API I/O、状態遷移、失敗時挙動を明文化

---

# 4. データ設計（SSoT）

## 4.1 SSoTの割当

* カメラ台帳：`cameras`（既存） 
* SDM連携設定：`settings`（既存）＋ `setting_key="sdm_config"` で保存（既存の set_setting を使用可能） 

## 4.2 DB拡張（推奨：camerasへ “SDM device_id” を追加）

理由：`camera_context` は is21 hints として使われており（AIの精度補助の領域）  、
**接続情報（device_id）を混ぜると責務が濁る**ため、専用カラムを足す。

### 追加マイグレーション案（例：010_sdm_integration.sql）

* `cameras` に以下を追加：

  * `sdm_device_id VARCHAR(128) NULL`：SDMの device識別子（固定ID）
  * `sdm_structure VARCHAR(128) NULL`：任意（施設/ホームの論理情報）
  * `sdm_traits JSON NULL`：任意（取得できたメタ情報の保存、is21 hintsとは別）
* **注**：最小なら `sdm_device_id` だけでOK。

実装上は、ConfigRepository が SELECT列を固定で持つため（CAMERA_COLUMNS） 
カラム追加したら以下も必ず更新：

* `src/config_store/types.rs` の `Camera` 構造体 
* `src/config_store/repository.rs` の `CAMERA_COLUMNS` と update bind（動的updateにも追加） 

---

# 5. バックエンド実装（全体概要）

## 5.1 新規モジュール：`sdm_integration`（SOLID）

### 責務（単一責務）

* SDM設定の保存/取得（settings）
* SDM API呼び出し（devices.list 等）
* go2rtcへの “Nestソース追加” の組み立て（グローバル設定＋device_idから生成）
* カメラ登録（ConfigStoreへの登録＋go2rtc登録＋スナップショット経路設定）

### 依存

* ConfigStore（settings read/write）
* StreamGateway（go2rtc add_source）
* reqwest（HTTP）

---

## 5.2 APIエンドポイント追加（routes.rsへ追加）

既存ルータは `/api/*` にまとまっている  ので、同じ流儀で追加。

### (A) SDM設定

* `GET  /api/sdm/config`

  * 返す：`{ configured: bool, project_id?: string, client_id?: string, has_refresh_token: bool }`
  * **client_secret/refresh_token は返さない**（UIに再表示しない方針）
* `PUT  /api/sdm/config`

  * 受ける：`{ project_id, client_id, client_secret, refresh_token? }`
  * 保存先：`settings.sdm_config`（set_settingで保存できる） 
  * 保存後：`config_store.refresh_cache()` を必ず呼ぶ（SSoT反映）

### (B) 認可コード→refresh_token 交換（UIで完結させる場合）

* `POST /api/sdm/exchange-code`

  * 受ける：`{ auth_code }`
  * やる：Google OAuth token endpoint へ交換（refresh_token取得）
  * 保存：`settings.sdm_config.refresh_token`
  * 結果：`{ ok: true }` or `{ ok:false, error }`

※ここは「実装を固定しすぎない」方針でも良いが、**UIで完結**させたいなら必要。

### (C) SDMデバイス一覧

* `GET /api/sdm/devices`

  * SDMから devices を取り、UIが表示できる形に正規化して返す
  * 返す例：

    ```json
    {
      "devices": [
        {"sdm_device_id":"...", "name":"Front Door", "type":"doorbell", "traits_summary": {...}}
      ]
    }
    ```

### (D) SDMデバイスをカメラ登録

* `POST /api/sdm/devices/:sdm_device_id/register`

  * 受ける（最低限）：

    * `camera_id`（go2rtcのsrc名＝is22 camera_idにするため必須）
    * `name`, `location`
    * `fid`, `tid`（“サブネット整理に乗せる”ため必須扱い推奨）
    * `camera_context`（任意：is21精度向上ヒント）
  * 処理（順序が重要）：

    1. `cameras` にINSERT（family=nest、sdm_device_id=保存、discovery_method="sdm" 等）
    2. go2rtcへ `add_source(camera_id, nest_source_string)`（失敗してもカメラ登録は残す/要検討）
    3. `snapshot_url` を go2rtc frame.jpeg に設定（静止画巡回をHTTPに寄せる）
    4. `config_store.refresh_cache()`（PollingOrchestratorが拾える）
  * 返す：登録したCameraオブジェクト

> 既存でも approve_device で「go2rtc登録→config refresh」をしているので同じパターンを踏襲するのが自然です 

---

## 5.3 “静止画取得” の実装方針（重要）

SnapshotService は **RTSP→失敗したらHTTP snapshot_url** へ落ちる構造です 
→ NestはRTSPが無い前提なので、登録時に `snapshot_url` を **go2rtc の frame.jpeg** にしておけば、PollingOrchestrator側は変更最小で動きます。

PollingOrchestratorは `snapshot_service.capture(camera)` を必ず通るので 
ここがHTTP fallbackに切り替わるだけ、という形にする。

---

## 5.4 go2rtc ソースの永続性（長時間運用の落とし穴）

現状は「登録時に go2rtcへ add_source」ですが、**go2rtc再起動**でソースが消える可能性があります（実装依存）。
この対策は **is22側で“再同期（reconcile）”するタスクを持つのが堅い**です。

### 推奨：起動時＋定期で “go2rtc sources reconcile”

* 起動時（main.rs のコンポーネント初期化後）に：

  * ConfigStoreのカメラ一覧を読み、必要なソースを go2rtc に再登録
* 定期（例：5分間隔）で：

  * go2rtcの `/api/streams` を見て不足分だけ add_source

StreamGatewayには `list_streams()` が既にあり  、routesでも `/api/streams` がある 
→ これを再同期ロジックに使う。

Nestソース文字列の生成は **sdm_config + sdm_device_id** から作る（秘密情報をcamera行に埋め込まない）。

---

# 6. フロントエンド実装（設定モーダルにSDMを足す）

## 6.1 入口（ヘッダ右上の Settings ボタン）

App.tsx には Settingsアイコンボタンがあり、現状は onClick未実装です 
→ ここに `SystemSettingsModal` をぶら下げるのが自然。

## 6.2 新規コンポーネント案：`SystemSettingsModal.tsx`

* shadcn/ui の Dialog + Tabs で実装
* タブ例（将来拡張を見据えてSOLID/MECE）：

  1. **Google/Nest (SDM)** ←今回の主題
  2. Polling/Admission（将来）
  3. その他（将来）

### SDMタブのUIフロー（アンアンビギュアス）

**Step 0: 状態表示**

* `GET /api/sdm/config` を呼び

  * Not configured / Configured but not authorized / Authorized の3状態を表示

**Step 1: 設定入力**

* project_id / client_id / client_secret を入力して保存（PUT /api/sdm/config）
* client_secret は “表示/非表示” 可能なpassword入力

**Step 2: 認可（2方式を用意して詰まりを減らす）**

* A案：UIが “認可URL” を表示 → ユーザーが開いて code を貼り付け → `POST /api/sdm/exchange-code`
* B案：refresh_token を既に持っている人向けに “直接貼り付け” でもOK（PUT /api/sdm/config）

**Step 3: デバイス一覧**

* `GET /api/sdm/devices` で一覧表示
* 行ごとに “登録” ボタン

**Step 4: 登録ダイアログ**

* camera_id（自動提案＋編集可）
* name / location
* 施設（fid/tid）：

  * 既存の `/api/subnets` から候補を出し、選択で fid/tid を埋める（あなたの “サブネット整理に乗せる” 要件）
* 送信：`POST /api/sdm/devices/:id/register`
* 成功したら `refetchCameras()`（既存と同じ流れ）

---

# 7. IpcamScan側の扱い（仕様の修正ポイント）

あなたの調査結論通り、Doorbellは「ポートスキャンで特定不可」なので、**IpcamScanに“Doorbell検出”を期待しない**を仕様に明記します。

ただし設計上のUXとしては以下を入れると親切です：

* スキャン結果に `oui:google` だけ出るケース（Nest Hub/Chromecast等）を「Nest候補」として見せるのはOK（既にOUIテーブルにGoogleは入っている） 
* その上で `SuggestedAction` に `manual_check` を割当て、UIで
  「Nest Doorbell 等はローカル検出できない場合があります → SDM登録へ」
  という導線を出す（ScanModalからSystemSettingsModalへのリンク）

---

# 8. 静止画・配信・AIの統合（全体フロー）

最終的な “一枚絵” はこうなります：

1. SDMでNest Doorbellを登録（設定モーダル）
2. is22は camera_id をSSoTに追加（cameras）
3. is22が go2rtc にソースを追加（StreamGateway.add_source） 
4. PollingOrchestrator が巡回し、SnapshotServiceが HTTP snapshot_url 経由で画像取得 
5. is21へ送ってイベント/ログ（既存通り）
6. UIは CameraGrid にサムネ表示、必要時は go2rtc WebRTCで再生（既存の /api/modal/lease などの路線上） 

---

# 9. 検証項目（最低限の受入）

## 9.1 SDM登録

* sdm_config 保存→再読み込みで維持される（SSoT）
* devices.list が取得できる
* 登録ボタンで cameras に追加される（/api/camerasに出る）

## 9.2 go2rtc統合

* 登録直後に `/api/streams/:camera_id/snapshot` が200になる（フレームが取れる） 
* 失敗時のエラーがUIに見える（「再認可が必要」「go2rtcが死んでる」等）

## 9.3 Polling/AI

* `/api/snapshots/:camera_id/latest.jpg` が更新され続ける（PollingOrchestrator経由） 
* detection_logs が貯まる（既存通り）

## 9.4 長時間運用

* go2rtcを再起動しても is22 の reconcile で復旧する（推奨タスク）
* モーダル多発時も Admissionが効く（既存）

---

# 10. 実装タスク分解（次の「詳細実装指示書」につながる形）

1. **DB migration**：`sdm_device_id` 追加（最小）
2. **backend**：

   * `sdm_integration` モジュール作成
   * routes追加（/api/sdm/*）
   * config_store settings 保存利用（set_setting） 
   * go2rtc登録（StreamGateway.add_source） 
   * reconcileタスク（任意だが強く推奨）
3. **frontend**：

   * `SystemSettingsModal` 新設
   * App.tsx のヘッダSettingsボタンに onClick を付ける 
   * SDMタブ wizard 実装（config→authorize→devices→register）
4. **動作確認**（9章）

---

必要なら、次のターンで **「詳細実装指示書」**として、

* 追加するRustファイル/関数シグネチャ
* JSONスキーマ（sdm_config / devices正規化）
* routes.rsへの具体的追加コード位置（既存のルーティング並びに合わせる） 
* フロントのUIワイヤー（Tabs/フォーム/登録ダイアログ）
  を、コピペで実装に入れる粒度まで落として書けます。
