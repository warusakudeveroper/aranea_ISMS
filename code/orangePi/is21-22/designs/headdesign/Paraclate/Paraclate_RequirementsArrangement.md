**mobes2.0 / aranea_ISMS 両リポジトリ確認済み**の上で、統括アドバイザーとして **齟齬のない MECE 構成・メソッド**と、**is21/22 開発ライン ↔ ParaclateAPP（mobes2.0）開発ライン**の相互タスクを洗い出した実施結果です。
（※ “将来やる”で棚上げしない前提で、**依存順＝優先順**で確定しています）

---

## 1) 現状確認（両repoの根拠つき）

### 1-1. 命名の現状（aranea_ISMS）

* is22 の README はまだ **“mobes AIcam control Tower (mAcT)”** 表記です 
  → Paraclate への改名が未反映。
* is22 UI 仕様書にも “mobes2.0連携でサマリー生成＋チャット” が明記されています （= Paraclate の思想と一致）

### 1-2. is22 側に Paraclate 予定地が既に存在

* is22 frontend の SettingsModal に **“AI Assistant settings (Paraclate module placeholder)”** が既にあり、
  `reportIntervalMinutes` / `grandSummaryTimes` / `reportDetail` 等を持ち、今は **localStorage 保存**です 
  → “Summary=間隔 / GrandSummary=時間指定” の要件は、既存構造に自然にマッピング可能。

### 1-3. カメラ lacis_id が必須であること（is22）

* is22 は preset を is21 に同期する際、**camera.lacis_id が無い場合は “Register with araneaDeviceGate first.”** を返します 
  → 「カメラは araneaDevice として登録して lacisID を持つ」方針は is22 実装と整合。

### 1-4. araneaDeviceGate 登録ロジックは既に両側に存在（再発明不要）

* aranea_ISMS 側

  * Python 実装：`device_register.py` が araneaDeviceGate に POST して `cic/stateEndpoint/mqttEndpoint` を取得・キャッシュします 
  * ESP32 実装：`AraneaRegister.cpp` も同様に POST し NVS に保存します 
* mobes2.0 側

  * `araneaDeviceGate.ts` が **Tenant Primary（permission 61+）の lacisId/userId/cic** で認証し、LacisID生成・CIC発行・Products pending を実施します 
  * pass は廃止（3要素認証）であることが明記されています 
  * **Typeごとのデフォルト権限が未設定だと登録エラー**になります（fallback禁止） 
  * 現状、登録された araneaDevice の `lacis.fid` は空配列で作成されます （= fidスコープ未確定）

### 1-5. LacisID形式は既に統一されている

* aranea_ISMS のガイドに、**`[Prefix=3][ProductType=3桁][MAC=12桁][ProductCode=4桁]=20桁`** が明記されています 
* mobes2.0 の TypeDomain も `araneaDevice`（単数）を正として定義 
  → ユーザー記載の `araneaDevices` は齟齬。**正は `araneaDevice`**。

### 1-6. 権限境界（blessing設計に直結）

* mobes2.0 の境界は **System Layer=71+ / Prophet=91+（ユニーク）** 
* Firestore ルール上、グローバルアクセスは **FID=0000 かつ permission 71+** が必須 
  → 「is21を system tenant + fid=0000 + permission71」は、既存ルールと完全整合。

### 1-7. mobes AIタブ（/system/ai-model）の拡張点

* `AIModelSettingsPage.tsx` は `APP_CONFIG_DEFINITIONS` で **appIdごとの設定フィールド**を定義しています 
* `AiApplicationDocument` は `configurationDefaults` を持てます 
  → 「Paraclate endpoint のプレースホルダをAIタブに追加」は **ここに実装するのが最短**。

---

## 2) 統一アーキテクチャ（MECE）と SSoT 宣言

以下を **MECE** に分割し、各 SSoT と責務境界を固定します。

### A. Identity / Auth（LacisOath）

* **SSoT**: mobes2.0 の `userObject` / `userObject_detail`（変更禁止、拡張はdetail側）
* is22 → mobes 認証は、既存実績のある **Tenant Primary lacisId/userId/cic（3要素）**を基軸（pass不要）

### B. Device Registry（is22 / is21 / cameras）

* **SSoT**: mobes2.0 `araneaDeviceGate` による araneaDevice 登録 
* カメラは is22 内部IDではなく、**必ず lacis_id（20桁）を持つ**（is22もそれ前提）
* ただし現状 gate は `lacis.fid=[]` で作る 
  → **fid境界必須**という要件に合わせ、ここを仕様として拡張する（後述タスク）。

### C. Config / Scheduling（Summary/GrandSummary + endpoint）

* **SSoT**: mobes2.0 `aiApplications/paraclate.configurationDefaults`（AIタブで編集）
* is22 の localStorage は「キャッシュ」に格下げ（SSoTはDB、キャッシュは端末）
  ※ is22 側に既に `reportIntervalMinutes` と `grandSummaryTimes` の器がある 

### D. Data / Storage（検知ログ・画像）

* **SSoT（イベント/検索）**: BigQuery `ai_logs.semantic_events`（tid/fid含む）
* **SSoT（画像）**: LacisFiles `lacisFiles/{tid}/{fileId}`（署名URL運用、直書き禁止）
  → Paraclate の “捨てないとパンク” は、LacisFiles の TTL/監査ログ運用に統合すべき（既存計画あり）

### E. Cross-tenant（Blessing）

* **基礎事実**: 71+ は system layer。91+ は unique で、ユニーク権限予約機構が存在 
* 方針：

  * **is21（ParaclateEdge）は system tenant + fid=0000 + permission71** としてクロステナント処理担当（Firestoreルール整合）
  * is22 の “blessing” は、**Firestore直アクセスではなく Paraclate側APIで検証→サーバ側で越境取得**（=安全に制御できる）として実装する。

---

## 3) 齟齬ポイント（アンアンビギュアスに確定）

1. **TypeDomain 名**

   * 正：`araneaDevice`（mobes実装の型定義）
   * 誤：`araneaDevices`（ユーザー記載）
     → **以後 `araneaDevice` に統一**。

2. **araneaDeviceGate 登録の前提条件**

   * “Type別デフォルト権限が未設定だと登録失敗” が実装で保証 
     → is21/is22/camera の **userTypeDefinitions / typeDefaultPermissions を先に整備**しないと gate が動かない。

3. **fid 境界**

   * gate は現状 `lacis.fid=[]` で作成 
   * 要件は「カメラは TID/FID に所属」
     → **gate に fids を渡せるように拡張**し、`lacis.fid` と `userObject_detail` に正規保存（SSoT遵守）。

4. **Paraclate endpoint 設定場所**

   * AIタブ（/system/ai-model）は `aiApplications` の `configurationDefaults` を編集できる 
     → そこに **paraclate.endpoint = https://www.example_endpoint.com（placeholder）** を追加するのが最短。

---

## 4) タスク洗い出し（依存順＝優先順 / MECE）

### P0: 共通基盤（両ラインの前提条件）

**P0-01: 命名改定（ドキュメント/UI）**

* aranea_ISMS: is22 README の “mobes AIcam control Tower (mAcT)” を Paraclate に変更 
* `mobes_AIcam_control_tower_spec.md` の見出し等も Paraclate 表記へ（内容は維持）
* is21 は正式名称 ParaclateEdge を明示（is21 README/設計インデックスに反映）

**P0-02: mobes2.0 に Paraclate 用 AIアプリ定義を追加（AIタブの設定フィールド）**

* `AIModelSettingsPage.tsx` の `APP_CONFIG_DEFINITIONS` に `paraclate` を追加 

  * fields（最低限）

    * `endpoint`（text, placeholder=`https://www.example_endpoint.com`）
    * `summary.reportIntervalMinutes`（number）
    * `grandSummary.times`（stringList, “HH:mm”）
* `aiApplications` に `appId=paraclate` を追加し、`configurationDefaults` に上記初期値を投入 

**P0-03: araneaDeviceGate に is21/is22/camera Type を通すための “Type定義 + defaultPermission” を整備**

* gate は default permission が未設定だと失敗する 
* mobes2.0 側で次を追加（最低限）

  * `userTypeDefinitions/araneaDevice__<type>`
  * `LacisOathConfig/permissionConfig/typeDefaultPermissions/araneaDevice_<type>` もしくは `userTypeDefinitions.defaultCathedralOrder`
* 推奨（安全側）：device の defaultPermission は **10（Guest上限）**に寄せる（例が既に `|| 10` を使っている）

---

### P1: Registry（fid境界・カメラ araneaDevice 化）

**P1-01: araneaDeviceGate を fid対応に拡張（必須）**

* 現状 `lacis.fid: []` で作成される 
* 要件：カメラは TID/FID に所属
* 仕様：`userObject` に `fids?: string[]` を追加し、

  * tenant primary が管理する fid のみ許容
  * `lacis.fid=fids` に格納
  * `userObject_detail` にも `facilityScope` 等で同値保持（検索/運用用）

**P1-02: カメラ（ParaclateCamera）の lacisID ルール確定**

* 20桁: `3 + 801 + MAC12 + productCode(4)`

  * この形式は既存ルールと一致 
* productCode は “カメラブランド” で割り当て（あなたの指示通り）

---

### P2: is22 側実装（登録・送信・同期）

**P2-01: is22 に araneaDeviceGate 登録機能を実装（再発明禁止＝既存参照で実装）**

* 参照実装（Python/ESP32）を踏襲

  * Python: `device_register.py` の payload 形式 
  * ESP32: `AraneaRegister.cpp` の POST/レスポンス処理 
* 実施内容（is22）

  * is22 自身（ar-is22CamServer）の登録 → cic/stateEndpoint/mqttEndpoint を保持
  * IpcamScan で “管理化（approve）” したカメラに対し、

    * まだ lacis_id が無ければ gate 登録して lacis_id を発行
    * is22 DB（camerasテーブル）に lacis_id を保存
  * これにより preset sync が成立（lacis_id 必須）

**P2-02: is22 の Summary/GrandSummary を “DB整備” し、Paraclate へ送信**

* is22 側には既に schedule の器（reportIntervalMinutes / grandSummaryTimes）がある 
* ただし現状 localStorage → **is22 DB（SSoTはmobesだが、is22側は運用キャッシュとしてDB保持）**へ移行
* 送信 payload は、あなた提示の JSON を採用（summaryOverview / cameraContext / cameraDetection）

**P2-03: is22 → Paraclate の認証方式（blessing含む）**

* 基本：is22 は “device cic” を持つので、**CIC認証で Paraclate API を叩く**（deviceStateReport 方式に寄せる）
* 越境が必要な場合のみ `blessing` を付与（P3で実装）

---

### P3: ParaclateAPP（mobes2.0）側実装（受信・保存・検索・チャット）

**P3-01: Paraclate Ingest API（Cloud Functions/Run）**

* is22 から Summary/Detection を受信し、以下を実施

  1. 認証（device cic + tid一致）
  2. Firestore “Hot” 保存（直近表示用）
  3. BigQuery `semantic_events` に投入（検索/分析用）

     * `sem_source=device_event`
     * `sem_entity=camera_detection | summary | grand_summary`
     * `sem_domain` にあなたの JSON を格納

**P3-02: 画像の LacisFiles 保管（TTL前提）**

* Storage ルールは direct write 禁止で、署名URL運用が前提 
* よって “is22 が直接 Storage にPUT” ではなく、

  * Paraclate Ingest API が受け取った画像（or is22 から pull）を **サーバ側で LacisFiles に格納**する
* 監査ログ/TTL/削除は LacisFiles の既存タスク群に統合 

**P3-03: Paraclate チャット（AIアシスタント）の実装方針**

* is22 UI仕様が期待する “mobes2.0連携チャット” を実装 
* 回答の基本戦略（あなたの要件に合わせて固定）

  * 会話応答：Flash系（速度）
  * サマリー生成：Pro系（思考/整形）
  * “赤い服の女性” 等の検索は semantic_events に入れた属性/自由タグ（sem_free_tags）で NL2SQL する 
  * データ欠損時も “カメラコンテキスト” から推論で回答（明示的に「推論」ラベルを付ける）

---

### P4: Blessing（クロステナント越境の確定実装）

**P4-01: Blessing レコード（権限91+のみ発行）**

* Prophet 91+ はユニーク権限であり、既に予約システムが存在 
* 仕様：`systemBlessings/{blessingId}` を新設

  * `issuedBy`（permission>=91 を必須）
  * `blessedLacisId`（is22の lacisId）
  * `allowedTids[] / allowedFids[] / expiresAt`
  * `purpose = "paraclate.crossTenant.cameraAccess"`
* Paraclate Ingest/Query API は `blessingId` を受け、サーバ側で越境取得を許可する

  * Firestoreルールの越境ではなく “サーバ側越境” にする（安全・監査が可能）

---

## 5) is21/22開発ライン / ParaclateAPP開発ライン への “指示に落ちるタスク分担”

### is21/22 開発ライン（aranea_ISMS）

* ① Paraclate 名称反映（docs/UI）
* ② is22 に araneaDeviceGate 登録導入（既存 Python/ESP32 実装を参照して再発明しない）
* ③ IpcamScan approve 時に camera.lacis_id を必ず埋める（無いと is21 sync が落ちる）
* ④ Summary/GrandSummary を is22 DBへ（localStorage脱却）し、送信ジョブ化 
* ⑤ `code/orangePi/is21-22/designs/headdesign/Paraclate/investigation_report/`（※typo修正して `investigation_report` 推奨）に、

  * 00_INDEX（依存順）
  * I/F仕様（認証・payload・エラー）
  * テスト計画（E2E含む）
    を作成（AGENTS の大原則に従う）

### ParaclateAPP 開発ライン（mobes2.0）

* ① /system/ai-model に paraclate 設定フィールド追加（endpoint placeholder含む）
* ② TypeDefinitions + defaultPermission を追加し、gate が新 type を受け入れる状態にする 
* ③ araneaDeviceGate の fid対応拡張（要件）
* ④ Paraclate Ingest API（受信→Hot保存→BigQuery）
* ⑤ LacisFiles 連携（サーバ側アップロード＋TTL）
* ⑥ Blessing 実装（permission91+のみ発行、API越境制御）

---

## 6) 受け入れ条件（最低限・アンアンビギュアス）

### E2E（必須）

1. is22 がカメラを approve → **camera.lacis_id が20桁で確実に付与**される 
2. is22 が Summary/GrandSummary を Paraclate Ingest に送信 →

   * Firestore Hot に保存される（直近表示）
   * BigQuery `semantic_events` に投入される（検索）
3. Paraclate チャットで「赤い服の人来なかった？」→

   * NL2SQL で該当イベントを引ける or 無ければ「推論/不明」を明示して回答（黙って捏造禁止）

### セキュリティ（必須）

4. Cross-tenant アクセスは **blessing無しでは不可**／blessing有りでのみ可能（監査ログ必須）
5. 画像は LacisFiles 配下に保存され、TTLで削除される（ストレージパンク回避）

---

## 7) MECEチェック / アンアンビギュアス宣言

* **MECE**：上記は「Auth / Registry / Config / Data&Storage / Query&AI / Cross-tenant / Ops」を重複なく分割し、各 SSoT と責務境界を固定しています。
* **アンアンビギュアス**：齟齬の確定点（TypeDomain名、gateのdefaultPermission依存、fid空配列問題、AIタブ拡張点、クロステナントの実装方式）を根拠付きで確定しました 

---

