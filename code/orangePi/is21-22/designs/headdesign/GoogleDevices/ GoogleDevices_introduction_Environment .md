以下は **「SDM（Google Smart Device Management / Device Access）関連で、コードを書かずに実施する作業」**だけをまとめた手順書です。
（※Google側UIは時々文言/配置が変わるので、項目名は“入力フィールド名”中心に書きます）

---

# is22 Camserver（mAcT）向け：Google Nest Doorbell 追加のための SDM 準備手順（コード外）

## 0. 完了状態（ゴール）

この手順が終わると、**is22側で Nest Doorbell を扱うために必要な材料**が揃います。

* SDM連携に必要な **Project情報** と **OAuthクレデンシャル** を取得済み
* **refresh_token**（長期運用の鍵）を取得済み
* SDM APIで **device_id**（DoorbellのID）を取得済み
* is22（NUCbox G5）側に、秘密情報を安全に置く場所が用意できている
* （任意）go2rtcへ Nest入力を追加する準備が整っている
  ※is22は go2rtc を前提依存にしています 

---

## 1. 事前条件（チェック）

### 1.1 Google Home 側

* Nest Doorbell（電源タイプ/バッテリー問わず）が **Google Homeアプリでセットアップ済み**
* Doorbellが属する「家（Home / Structure）」が **1つに整理されている**（複数Homeだと後で見失いがち）
* 使うGoogleアカウントは **一般のGoogleアカウント（@gmail等）**推奨
  ※Workspace系だと詰まることがあります

### 1.2 is22（NUC）側

* is22 Camserver が動いている（もしくは動かす準備がある）
* go2rtc を同居させる前提（既にTapo/VIGIで使っているならそのまま）
  is22の環境変数に `GO2RTC_URL=http://localhost:1984` が想定されています 

---

## 2. まず「控えるべき値」一覧（SSoTメモ）

この手順中に出てくる値は最後にまとめて保管します（**漏洩すると危険**）。

* **GCP_PROJECT_ID**（例：my-sdm-project）
* **GCP_PROJECT_NUMBER**（数値の方）
* **SDM_ENTERPRISE_PROJECT_ID**（Device Access側の project id / enterpriseで使うID）
* **OAUTH_CLIENT_ID**
* **OAUTH_CLIENT_SECRET**
* **SDM_REFRESH_TOKEN**
* **SDM_DEVICE_ID**（Doorbellのdevice_id）

---

## 3. Google側セットアップ（Device Access + GCP）

### Step 3-1. Device Access（開発者登録）

1. Device Access Console に入り、開発者登録を完了

   * ここで少額課金（$5相当）が発生することが多いです（仕様変更あり得る）

> 目的：SDM（Smart Device Management）を使う権限を得る

---

### Step 3-2. Google Cloud Project 作成

1. Google Cloud Console → 新規プロジェクト作成
2. 以下を控える

   * **Project ID（文字列）**
   * **Project Number（数字）**

---

### Step 3-3. SDM API 有効化

1. GCP → APIs & Services → Library
2. **Smart Device Management API** を検索して「Enable」

---

### Step 3-4. OAuth 同意画面（Consent Screen）設定

1. APIs & Services → OAuth consent screen
2. 種別は通常「External」
3. 必須項目（アプリ名、メール等）を入力して保存
4. スコープは後で使う（SDM service）

> ポイント：運用中にトークンが失効する系のトラブルは、同意画面の状態（Testing/Production）で差が出ることがあります。
> “まず動かす”なら最低限でOK、長期運用にするなら後で整備。

---

### Step 3-5. OAuth クライアントID作成（Web application）

1. APIs & Services → Credentials → Create credentials → OAuth client ID

2. Application type：**Web application**

3. Authorized redirect URIs：まずは簡単に

   * `https://www.google.com`
     を登録（手動で code を拾う運用にするため）

4. 作成後、以下を控える

   * **OAUTH_CLIENT_ID**
   * **OAUTH_CLIENT_SECRET**

---

## 4. Device Access Project 作成（SDM側の“enterprise project id”を得る）

### Step 4-1. Device Access Console でプロジェクト作成

1. Device Access Console → プロジェクト作成
2. 作成したプロジェクトに、先ほどの **OAUTH_CLIENT_ID** を紐づけ
3. ここで表示される **Device Access Project ID**（enterpriseで使うID）を控える

   * これが **SDM_ENTERPRISE_PROJECT_ID**（enterprises/{id} の {id}）

---

## 5. アカウント連携（PCM）で “認可コード” を取る

### Step 5-1. 認可URLを作ってアクセス

ブラウザで以下の形式のURLへアクセスします（値を置換）。

* `<SDM_ENTERPRISE_PROJECT_ID>`
* `<OAUTH_CLIENT_ID>`

（概念形）

```
https://nestservices.google.com/partnerconnections/<SDM_ENTERPRISE_PROJECT_ID>/auth
  ?redirect_uri=https://www.google.com
  &access_type=offline
  &prompt=consent
  &client_id=<OAUTH_CLIENT_ID>
  &response_type=code
  &scope=https://www.googleapis.com/auth/sdm.service
```

### Step 5-2. 権限付与

画面で「家（Structure）」「デバイス」へのアクセスを許可して進める

### Step 5-3. code を回収

最後に `https://www.google.com/?code=XXXX...` のようにリダイレクトされるので、
URLの `code=` の値を控える（**SDM_AUTH_CODE**）

---

## 6. 認可コード → refresh_token を取得（サーバ作業：コマンド実行）

### Step 6-1. is22 か手元PCで必要ツール確認

* `curl`（だいたい入ってる）
* `jq`（無ければ `sudo apt install jq`）

### Step 6-2. token 交換

以下を実行して `refresh_token` が返ることを確認します。

```bash
curl -s -X POST https://oauth2.googleapis.com/token \
  -d code="SDM_AUTH_CODE" \
  -d client_id="OAUTH_CLIENT_ID" \
  -d client_secret="OAUTH_CLIENT_SECRET" \
  -d redirect_uri="https://www.google.com" \
  -d grant_type="authorization_code" | jq .
```

* 返却JSONの `refresh_token` を **SDM_REFRESH_TOKEN** として控える
* `access_token` も返るが短命なので、refresh_tokenが主役

---

## 7. device_id（Doorbell）を取得（SDM API 呼び出し）

### Step 7-1. refresh_token で access_token を発行

```bash
ACCESS_TOKEN=$(
  curl -s -X POST https://oauth2.googleapis.com/token \
    -d client_id="OAUTH_CLIENT_ID" \
    -d client_secret="OAUTH_CLIENT_SECRET" \
    -d refresh_token="SDM_REFRESH_TOKEN" \
    -d grant_type="refresh_token" | jq -r .access_token
)
echo "$ACCESS_TOKEN"
```

### Step 7-2. devices.list

```bash
curl -s -H "Authorization: Bearer $ACCESS_TOKEN" \
  "https://smartdevicemanagement.googleapis.com/v1/enterprises/SDM_ENTERPRISE_PROJECT_ID/devices" | jq .
```

* `type` や `traits` を見て Doorbell を特定
* `name` が `enterprises/.../devices/DEVICE_ID` 形式になっているので、末尾の `DEVICE_ID` を控える
  → **SDM_DEVICE_ID**

---

## 8. 秘密情報の保管（is22上・コード外で超重要）

### Step 8-1. 秘密ファイルを作る（例）

is22（NUC）に root だけ読める場所を作ります。

```bash
sudo mkdir -p /etc/is22/secrets
sudo chmod 700 /etc/is22/secrets
sudo nano /etc/is22/secrets/sdm.env
```

中身（例）：

```bash
SDM_ENTERPRISE_PROJECT_ID="..."
OAUTH_CLIENT_ID="..."
OAUTH_CLIENT_SECRET="..."
SDM_REFRESH_TOKEN="..."
SDM_DEVICE_ID="..."
```

権限：

```bash
sudo chmod 600 /etc/is22/secrets/sdm.env
```

> これは「のちに mAcT の設定モーダル（SDMタブ）へ貼る」ためのSSoTメモです。
> GitHubやログへ絶対に出さない。

---

## 9. （任意）施設（fid/tid）に紐づけるメモ

あなたの環境だと、例えば以下の fid がサブネットに対応しています（運用メモとして有用） 

* **fid:0150** HALE京都丹波口ホテル = `192.168.125.0/24` 
* **fid:0151** HALE京都東寺ホテル = `192.168.126.0/24` 

※SDM登録はIPスキャンに乗らないので、「どの施設のカメラか」は最終的に **手で選択**できる導線を作るのが正しい（あなたの方針通り）。

---

## 10. （任意）go2rtc 側の“準備確認”（運用チェック）

is22は go2rtc を前提にしていて、`GO2RTC_URL=http://localhost:1984` を環境変数として持ちます 
（Tapo/VIGI共存が既にできているならここはスキップ可）

* go2rtc API に到達できることを確認：

```bash
curl -s http://localhost:1984/api/streams | head
```

---

# 11. よくあるハマり（切り分け）

### A) devices.list に Doorbell が出ない

* Google Homeで使っているアカウントと、SDM連携したアカウントが違う
* PCMで権限付与（家/デバイス）が不足
* Doorbellが別のHomeに入っている（Homeを間違える）

### B) refresh_token が取れない

* 認可URLに `access_type=offline` / `prompt=consent` が無い
* redirect_uri が OAuthクライアントの許可URIと一致していない

### C) 取れたのに後日死ぬ

* refresh_tokenの運用（更新・再認可）が必要になるケースがある
  → だから **秘密情報をSSoTとして保管**し、再認可できるようにしておく（この手順書の目的）

---

# 12. 最終チェックリスト（コピペ用）

* [ ] GCP: SDM API Enable 済
* [ ] GCP: OAuth consent screen 作成済
* [ ] GCP: OAuth Client ID/Secret 取得済
* [ ] Device Access: Project作成＆OAuth client紐付け済（SDM_ENTERPRISE_PROJECT_ID控えた）
* [ ] PCM: 認可完了、code取得済
* [ ] code→token交換で refresh_token 取得済
* [ ] devices.list で Doorbell の device_id 取得済
* [ ] `/etc/is22/secrets/sdm.env` に全値保存し、権限600になっている
* [ ] （任意）fid/tid と紐づけメモが取れている 

---

この手順が終わったら、次の段階（コード側）では「SDM設定モーダルに貼る」「devices一覧から登録する」「go2rtcへソース追加し、snapshot巡回に乗せる」になります。
もし希望があれば、あなたの運用に合わせて **“コピペで動く運用手順（`sdm.env` → systemd/Docker環境変数へ注入 → テストコマンド）”** まで、コード外として続編も作れます。
