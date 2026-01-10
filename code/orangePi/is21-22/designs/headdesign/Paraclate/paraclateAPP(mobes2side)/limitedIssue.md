ここは **「Bless＝LacisOath層の権限拡張」**なので、Paraclate単体の話として実装すると確実に事故ります。
同時に、イベント（検知ログ）とスナップショットは **eventId を軸に一貫した参照モデル＋保持/削除を制度化**しないと、運用コスト（Firebase Storage）がサブスクと直結して爆発します。

以下、**mobes2.0（LacisOath / LacisFiles / Subscription）既存実装を尊重**しつつ、設計指示を「Bless計画」と「容量/課金計画」に分けて **アンアンビギュアスに**まとめます。

---

# 1) Bless（LacisOath拡張）詳細計画

## 1.1 目的（Blessが解決すること）

* 通常は **tid境界を越えない**（LacisOathの基本原則）。
* 例外として、**permission≥91** が特定の “主体（is22等）” に対して **限定スコープの越境権能**を付与する。
  Paraclate設計上も「越境時のみblessing」を想定しています 。

> 注意：Firestore Rules の “Global Access（fid=0000 & permission≥71）” は別物です 。
> Blessは **個別の越境許可**であり、`'0000'` で誤魔化すのは禁止。

---

## 1.2 認証モデル（既存実装を正として組む）

Paraclate連携（is22→mobes2.0）は、DD03で `lacisOath: {lacisID, tid, cic, blessing}` 形式です 。
mobes2.0 には **tid+lacisId+cic の DeviceAuth検証**が既にあり、`userObject.tid` と `lacis.cic_code/cic` を突合します 。

✅ **指示（固定）**

* Bless は **DeviceAuthが通った後**にだけ評価する（blessing単体での認証はしない）。
* “blessingが必要な条件” は DD03 の 403 を基準とする（tid/fid mismatch など）。

---

## 1.3 データモデル（SSoT / 監査 / 取消を前提）

Paraclate基本設計では `systemBlessings` を想定しています 。これを **LacisOathの管理対象**として確定します。

### systemBlessings/{blessingId}

**必須フィールド（アンアンビギュアス）**

* `blessingId: string`（docIdと一致）
* `status: 'active'|'revoked'|'expired'`
* `subject`（このBlessを“使う側”＝is22等）

  * `subjectLacisId: string`（20桁）
  * `subjectTid: string`
* `scope`（越境先）

  * `allowedTids: string[]`（越境先tidのホワイトリスト）
  * `allowedFids: string[]`（越境先fidのホワイトリスト、空=allowedTids内の全fid）
  * `allowedCameraLacisIds?: string[]`（必要なら最小権限でカメラ単位まで絞る）
* `capabilities: string[]`（例：`['paraclate.read', 'camera.read', 'snapshot.read']`）
* `expiresAt: Timestamp`（失効日時）
* `purpose: string`（人間が読める理由。必須）

**監査（必須）**

* `issuedBy: { lacisId, tid, permission }`
* `issuedAt`, `updatedAt`
* `revokedBy?`, `revokedAt?`, `revokeReason?`

### 履歴（推奨）

* `systemBlessings/{blessingId}/history/{eventId}`

  * `event: 'issued'|'revoked'|'expired'|'scope_changed'`
  * `actor`, `note`, `createdAt`

この「状態＋history」方式は、既存の Edict設計（状態管理＋履歴＋auditLogs）と同じ思想で、監査に強いです 。

---

## 1.4 API（発行/取消/検証）— 車輪の再発明を避ける

Blessは “LacisOath機能” なので、Paraclate専用に閉じず **共通API**として実装します。

### (A) 発行（System/Tenant管理UIから）

* `blessing.create`（Callable推奨）

  * Firebase Auth必須
  * 発行者 permission≥91 を enforce（BasicDesign要件）
  * 入力：subjectLacisId / scope / expiresAt / purpose / capabilities
  * 出力：blessingId

### (B) 取消

* `blessing.revoke`（Callable）

  * permission≥91
  * revokedBy/reason を記録
  * status=revoked

### (C) 一覧/詳細

* `blessing.list`（UI用）
* `blessing.get`

### (D) 検証（サーバ内部関数）

* `validateBlessing({subjectLacisId, subjectTid, targetTid, targetFid, cameraLacisId?}) -> ok/deny + reason`

  * Paraclate ingest / is22データ提供API 等で共通利用

---

## 1.5 Bless適用ポイント（どこで使うか）

**原則**：越境が発生する可能性があるリクエストだけ。

* `POST /api/paraclate/ingest/summary`：403条件（tid/fid mismatch）時に blessing を要求 
* is22→mobes で **他tidのカメラ/ログに触る系**（将来のクロステナント監視など）

---

## 1.6 テスト計画（必須）

* Unit

  * scope判定（allowedTids/Fids/Camera）
  * expiresAt / revoked の拒否
  * capabilities の拒否（必要権能がない）
* Integration（Emulator）

  * deviceAuth通過  → blessing無し越境=403 → blessingあり越境=200
* E2E（Chrome）

  * 発行UI→即時有効→is22側から越境OK→取消→即時403

---

# 2) イベントID×スナップショット×保持/削除（容量と課金）詳細計画

## 2.1 イベントIDの正本（決め打ち）

is22のBQ同期（DD04）は `event_id: det_{log.id}` で payload を作る設計です 。
**これを “全システム共通の eventId” として固定**するのが最も手戻りが少ないです（joinが一発でできる）。

✅ **指示（固定）**

* is22→mobes の `POST /api/paraclate/ingest/event` に載る `event.eventId` は **`det_{detection_logs.id}`** を原則にする（uuid乱立禁止）。
  DD03のevent payload自体は `eventId` を要求しています 。

---

## 2.2 スナップショット保存（LacisFiles再利用が前提）

LacisFilesは既に multi-tenant + fidScope + Cloud Functions only を備えています  。
よって Paraclateは **“LacisFilesの上に乗る”** が正解です（再発明禁止）。

✅ **指示（固定）**

* Paraclate snapshot は `module='paraclate'`（または 'paraclateSnapshot'）として LacisFiles に登録する。
* `tags/description` に **eventId / cameraLacisId / severity** を必ず入れる（後でクリーンアップ・監査・検索に効く）。

---

## 2.3 retention（デフォルト60日、調整可）の設計

DD03の is22 側 `paraclate_config.retention_days` は **default 30**になっています 。
ユーザー要望は **60日デフォルト＋調整**なので、mobes2.0（SSoT）の設定に合わせて **is22側も60に揃える**のがアンアンビギュアスです。

✅ **指示（設計変更）**

* mobes2.0 `aiApplications.paraclate.configurationDefaults` に

  * `retention.imagesDays = 60`
  * `retention.summaryDays = 60`（Hot Cache用。正本はis22/BQなので値は運用上で決める）
* `GET /api/paraclate/config/{tid}` の返却 `retentionDays` も 60 を初期値にする（DD03のレスポンス形式に存在）
* is22側 `paraclate_config.retention_days` default を 60 に変更（SSoT同期で上書きされるが、初期値が重要）

---

## 2.4 自動クリーンアップ（Firebase側の“実装”として必須）

LacisFilesは `deleteLacisFile` が soft/hard delete をサポートしています 。
保持期限を過ぎたら **hard delete**（ストレージを減らす）しないとコストは下がりません。

✅ **指示（P0扱い）**

* Cloud Scheduler（日次）で `paraclateSnapshotRetentionJob` を実装：

  1. tenantごとに Paraclate設定（retentionDays）を取得（aiApplications + overrides 合成）
  2. `lacisFiles` の `module=='paraclate'` を対象に、`uploadedAt <= now - retentionDays` を抽出
  3. `deleteLacisFile(hardDelete=true)` 相当の内部処理で Storage object を削除し、FireStore doc を deleted に更新

> 重要：`fidScope` に `'0000'` は入れられません（予約値）。
> 越境は blessing で表現し、fidScopeで破壊しない。

---

## 2.5 「ローカル（is22）も削除」が必要な理由とルール

is22は送信キューを持ち、ネットワーク断で local に溜まります（DD03 send_queue 等）。
よって **ローカル保持の上限**も設計に含める必要があります。

✅ **is22側への要求（mobes設計としてのインターフェース要求）**

* 送信成功（mobesが snapshotRef を返した）時点で、ローカルの該当画像は **即削除**（最低でも24h以内）
* 送信失敗時は `retentionDays` を上限に残すが、ディスク逼迫時は「古い順に削除」する（保護期間よりディスク保護を優先）

---

# 3) サブスクリプションとの接続（コストが露骨に効く部分）

Subscriptionには planごとの `maxStorage` 概念が既にあります（free=1GB / pro=100GB / enterprise=無制限）。
Paraclate snapshot はここに直結させるのが自然です。

## 3.1 方式（どれを採るかを設計で固定）

**A案（推奨：運用が壊れにくい）**

* プラン上限（maxStorageBytes）を超えたら：

  * 以後の snapshot 保存を **拒否**（ログだけ受ける）
  * 返却に `snapshotStored=false, reason='quota_exceeded'` を付与
  * UIに「プラン上限のため画像は保存されません」を表示

**B案（控えめ）**

* 上限超過したら retentionDays を自動で縮める（ただしユーザーに説明責任が出る）

どちらでも良いですが、**後から揉めないのはA案**です（“保存されると思ってた”を防ぐ）。

## 3.2 使用量の算出（SSoTを作る）

* `tenants/{tid}/usage/storageSnapshots`（集計ドキュメント）を追加

  * `activeBytes`, `activeCount`, `lastCalcAt`
  * `planMaxBytes`, `plan`, `effectiveRetentionDays`
* 更新方式は2段構え：

  * ingest時に `FieldValue.increment(bytes)` で日次カウンタ
  * 日次ジョブで正確に再集計（ズレ補正）

---

# 4) 追加で “設計ドキュメントに反映すべき変更点”

* DD03の `retention_days default 30` を **60**へ（要件）
* eventIdは uuid ではなく **det_{log.id} を第一候補**（DD04と統一）
* Paraclate event ingest payloadに snapshot を含めるか、別APIにするかを確定（現状DD03はeventのみ）

---

必要なら次の出力も同じ粒度で作れます（どれも “設計指示としてIssueに貼れる形”）：

1. **Blessing 設計書（LacisOath拡張）**：Firestoreスキーマ＋API＋監査＋テスト
2. **Snapshot Retention 設計書**：Job設計＋Quota設計＋Subscription連携＋受入試験
3. **DD03/DD04へ差分反映パッチ指示**（is22側へ投げるレビューコメント文面）
