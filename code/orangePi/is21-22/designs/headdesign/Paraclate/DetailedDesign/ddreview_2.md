commit **c862bf2** の内容、こちら（mobes2.0側設計管理）として **P0は全面OK**、整合性チェック（CONSISTENCY_CHECK）も方針として正しいです。特に **retention_days=60** への統一は、ストレージ課金の観点でも “必須の前倒し” になっています 。

以下、P1の「mobes2.0側と協議待ち」4点について、**決定事項（アンアンビギュアスに確定）**として指示を出します。あわせて、Bless（LacisOath拡張）とRetention/Subscription連動も **設計として固定**します。

---

# 1) P0修正の評価（受領・OK）

* **retention_days 30→60**：DD03のDB初期値が60になっていることを確認 
* **config_sync に lacisOath認証追加**：GET config に `X-Lacis-*` で認証を付与していることを確認 
* **Event payload に snapshot追加**：Event request/response に snapshot と snapshotRef が定義されたことを確認 
* 送信URLの組み立てが `endpoint + "/ingest/..."` であることも明確 

※細かい指摘：DD03の **configレスポンス例**が `retentionDays: 30` のまま残っているので、例は60へ更新してください（実装はSSoTに追従でOK） 。

---

# 2) P1協議事項の決定（ここから先はこの仕様で固定）

## P1-1: endpoint 定義の明確化（決定）

### 結論

`paraclate.endpoint` は **「Paraclate APP API の“ルート”URL（末尾スラッシュ無し）」**とする。
is22 はそこに **`/ingest/summary` / `/ingest/event` / `/config/{tid}` / `/connect`** を付与して呼ぶ   。

### mobes2.0側 実装指示

* AI Model Settings の SSoTは既存の `resolveAiApplicationConfig()` で deep-merge する（再発明禁止）
* `aiApplications/paraclate.configurationDefaults.endpoint` を追加し、tenant/facility override も同様に機能させる（既存仕組み準拠）
* **入力値正規化**：保存時に `endpoint` の末尾 `/` を除去（`trimEnd('/')`）

### is22側 実装指示

* 送信時は `endpoint.trim_end_matches('/')` を必ず通す（ダブルスラッシュ事故防止）

---

## P1-2: snapshot_url形式の確定（storagePath採用で固定）

### 結論

`snapshot_url`（is22 DB / BQ）には **署名URLを入れない**。
**LacisFiles の storagePath（恒久参照子）を入れる**。

理由：mobes2.0 の LacisFiles は **短寿命の署名URL発行（getDownloadUrl）**が基本で、恒久URLは前提にしていない 。
storagePath はレスポンス/メタに常に含まれる 。

### storagePath の正規形

`lacisFiles/{tid}/{fileId}.{ext}`（LacisFilesの設計で定義済み）

### DD03の修正指示（is22側設計）

DD03の Event response は現在 `snapshotRef: "lacisFiles://..."`
になっていますが 、**以下に置換**してください（P1確定仕様）：

**Response 200（推奨形）**

```json
{
  "ok": true,
  "eventId": "det_12345",
  "receivedAt": "2026-01-10T12:34:57Z",
  "snapshot": {
    "tid": "T2025...",
    "fileId": "snapshot_det_12345",
    "storagePath": "lacisFiles/T2025.../snapshot_det_12345.jpg"
  }
}
```

> 既存の `snapshotRef` を残すなら、その値は **storagePath と同値**にしてください（`lacisFiles://` スキームは禁止。曖昧さが増えるため）。

### mobes2.0側 実装指示（重要）

* `/api/paraclate/ingest/event` で snapshot を受け取ったら、**mobes2.0サーバ側が Storage へ保存し、Firestore `lacisFiles/{tid}/{fileId}` を作る**（「is22が直接Storage PUTしない」要求に一致）
* 保存後に `storagePath` を返し、is22 はそれを `detection_logs.snapshot_url` に保存（DD03の記述通り）
* UIで画像表示する時は **getDownloadUrl（署名URL）**で復元する（既存fileServiceがその前提）

---

## P1-3: BigQuery dataset 名の確定（paraclateで固定）

### 結論

Paraclate用 dataset は **`paraclate`** に分離して固定。
理由：既に mobes2.0 には SemanticTags用の dataset `ai_logs` があり、同名 `semantic_events` を運用している 。衝突（スキーマ不整合）が起きるため分離必須。

### 指示

* is22 DD04 の設定例・INSERT先は `mobesorder.paraclate.*` に統一
* mobes2.0側で **BQテーブル定義（SSoT）**を持つ（DD04方針に一致）
* 最低限テーブル：

  * `mobesorder.paraclate.semantic_events`
  * `mobesorder.paraclate.summaries`

---

## P1-4: Pub/Sub 受信フロー詳細設計（決定）

### 結論

**「config本体をPub/Subで配らない」**。Pub/Subは **“設定変更の通知（トリガ）”**だけにし、is22 は **GET /config/{tid}** でSSoTからpullする。
（SSoTを守りつつ、pushで即時同期も可能）

DD03でも “Pub/Sub通知 or 定期ポーリング” と書かれており整合します 。

### Topic設計（新設・単一）

* Topic: `paraclate-config-updates`
* Payload:

```json
{
  "type": "paraclate.config.updated",
  "tid": "T...",
  "fids": ["0150", "0200"], 
  "updatedAt": "2026-01-10T12:00:00Z",
  "actor": { "lacisId": "....", "permission": 80 }
}
```

### mobes2.0側 publish 条件

* `aiApplications/paraclate` の更新
* `/tenants/{tid}/aiModeOverrides/paraclate` の更新
* `/facilities/{fid}/aiModeOverrides/paraclate` の更新
  （＝既存のAI設定SSoTの更新点と一致）

### is22側 subscriber 動作

* メッセージ受信→ `ConfigSyncService.sync_from_mobes()` 実行（既にGET時に `X-Lacis-*` 認証を付ける実装がある）
* `updatedAt` を比較して差分があれば `paraclate_config` をUPSERT（既存ロジック踏襲）

### 既存トピックとの整合

mobes2.0 には既に `aranea-device-config` 等の運用があり、topic乱立は避ける方針（ただし今回の `paraclate-config-updates` は **トリガ専用で1本**なので許容）。

---

# 3) Bless（LacisOath機能導入）— 詳細計画をこの仕様で開始

## 3.1 SSoT/コレクション

`systemBlessings/{blessingId}` を新設し、発行は **permission>=91（Prophet）**のみ 。
権限境界（91+）は mobes2.0側の permission boundary にも明記がある 。

## 3.2 必須フィールド（アンアンビギュアス）

* `issuedBy`（発行者 lacisId / permission / tid）
* `blessedLacisId`（is22のlacisId）
* `allowedTids[] / allowedFids[] / expiresAt`
* `purpose = "paraclate.crossTenant.cameraAccess"` 
* `status: active|revoked|expired`
* 監査ログ（発行・取消の履歴）

## 3.3 適用ポイント

* Paraclate Ingest/Query API が `blessingId` を受け、**サーバ側で越境取得を許可**（Firestoreルール越境でやらない）

---

# 4) ストレージ圧迫・コスト（Subscription連動）の設計固定

## 4.1 Retention（デフォルト）

* Paraclateの画像保持は **60日デフォルト**（DD03も60で統一済み）
* 保持日数は mobes2.0 AI設定（SSoT）で tenant/facility override 可能（既存configServiceで解決）

## 4.2 Hard delete を必須化（コスト直結）

LacisFiles は hard delete（Storage物理削除）をサポートしているため 、**Retention job は hardDelete=true** が必須。

## 4.3 Subscriptionとの関係（設計要件）

mobes2.0のサブスクはプランごとに **maxStorage** を持っている（free=1GB / pro=100GB / enterprise=無制限）。
したがって Paraclate snapshot は次のルールを採用：

* **イベント（検知ログ）は常に受ける**（データ欠損を作らない）
* **スナップショットはプラン/使用量に応じて制御**

  * 使用量が上限を超えた場合：`snapshotStored=false` を返し、is22は `snapshot_url=null` で保存（ログは残る）
  * 併せて retention job は継続（古いものから削除）

> 使用量（bytes）の正確な集計は別途 mobes2.0 側で “usage集計” が必要。これはサブスクとセットで実装タスク化してください（棚上げ禁止）。

---

# 5) 次のタスク（依存順＝優先順）

1. **mobes2.0**：AI Model Settings に `paraclate` 追加（endpoint/retention/schedule）
2. **mobes2.0**：`/api/paraclate/ingest/*` と `/config/{tid}` 実装（DD03 I/F） 
3. **mobes2.0**：snapshot保存（LacisFiles）＋ hard delete 付き retention job（60日）
4. **mobes2.0**：Blessing（systemBlessings）発行/取消/監査＋API検証 
5. **両側**：BQ dataset `paraclate` でスキーマ確定＆同期 
6. **両側**：Pub/Sub `paraclate-config-updates`（通知→GET同期）

---


