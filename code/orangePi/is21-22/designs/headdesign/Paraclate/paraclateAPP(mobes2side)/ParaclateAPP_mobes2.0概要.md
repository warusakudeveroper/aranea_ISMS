# Paraclate APP（mobes2.0）概要 / 仕様サマリー

作成日: 2026-01-10  
対象: mobes2.0（Tenant Admin層 / ProclateAPP 側の Paraclate APP）  
想定読者: mobes2.0開発者 / is22(is21-22)開発者 / 運用管理者  
ステータス: Draft（実装開始用の“仕様固定版”）

---

## 0. この文書の目的

Paraclate は「そこにあって穏やかに見守り、対話的に報告する」カメラ観測・要約・対話システムである。  
この文書は **mobes2.0 側に実装される Paraclate APP が“何で、何を責務として持つか”**を、SSoT・権限境界・I/F・運用（コスト）まで含めて短く固定する。

---

## 1. 1行でいうと（TL;DR）

**Paraclate APP（mobes2.0）は「Paraclate設定のSSoT」「is22からのSummary/Event受信」「スナップショットをLacisFilesへ保存・削除」「Bless（越境権能）の発行/検証」「LLMによる対話応答」を担う。**  
is22 は「カメラ台帳・検知ログ・サマリー生成・BQ同期」を担い、LLM推論は原則 Paraclate APP（mobes2.0）側で実行する。

---

## 2. 構成要素（登場人物）

- **mobes2.0 / Paraclate APP（本ドキュメントの対象）**
  - Tenant Admin 層のアプリ（AppShell サイドバー配下）
  - Config SSOT / Ingest API / Chat API / Blessing / LacisFiles / Subscription連携 を提供

- **is22（Paraclate Server）**
  - カメラ台帳、検知ログ、Summary/GrandSummary生成、送信キュー、BQ同期を担当
  - Paraclate APPへ `Summary` と `Event(+snapshot)` を送信

- **is21（ParaclateEdge）**
  - 推論（車両OCR/顔特徴など）を担当
  - system tenant / fid=0000 / permission71 の特殊デバイスとして応答（クロステナント処理は“is21側が直接越境アクセスする”のではなく、is22経由でデータ提供）

- **LacisOath**
  - tid/fid/permission境界、userObjectの正本
  - Bless（越境権能）機能はここに導入する

- **LacisFiles**
  - スナップショット画像の保存・参照（署名URLは都度生成）
  - retention（削除）によるコスト制御は必須

- **BigQuery**
  - 長期分析の最終到達点
  - datasetは `paraclate` に分離（既存 `ai_logs` と衝突しない）

---

## 3. SSoT（Single Source of Truth）宣言（最重要）

| 領域 | SSoT | 備考 |
|---|---|---|
| tid/fid/permission/userObject | mobes2.0（LacisOath） | 越境禁止が原則 |
| Paraclate 設定（endpoint/schedule/retention/attunement） | mobes2.0（AI Model Settings） | is22 はキャッシュ |
| カメラ台帳 / 検知ログ / サマリー | is22（DB） | 事実データ生成と保持 |
| 長期分析 | BigQuery（dataset=paraclate） | 最終到達点 |
| スナップショット画像 | LacisFiles | is22直PUTは禁止 |
| Bless（越境権能） | mobes2.0（LacisOath拡張） | permission>=91のみ発行 |

SSoTを破る二重保存・“後で直す”前提の暫定実装は禁止。

---

## 4. Paraclate APP（mobes2.0）の機能スコープ（MECE）

### 4.1 Config SSOT（AI Model Settings）
- `/system/ai-model` の Applications タブに `paraclate` を追加する。
- `aiApplications/paraclate.configurationDefaults` と、tenant/facility override を正とする。
  - `/tenants/{tid}/aiModeOverrides/paraclate`
  - `/facilities/{fid}/aiModeOverrides/paraclate`

**paraclate config（最低限）**
- `endpoint`: Paraclate API ルート（末尾スラッシュ無し）
- `summary.reportIntervalMinutes`: Summary間隔（分）
- `grandSummary.times[]`: “HH:mm” 形式（複数）
- `retention.imagesDays`: スナップショット保持日数（デフォルト60）
- `retention.summaryDays`: サマリー保持（Hot cache用途）
- `attunement`: 追加の動作パラメータ（オブジェクト）

> endpoint定義は「/api/paraclate まで含むルートURL」とする。is22はそこに `/ingest/*` `/config/*` `/connect` を付与して呼ぶ。

---

### 4.2 Ingest API（is22 → mobes2.0）
is22 から Paraclate APP へ送られるデータの受け口。

#### エンドポイント（固定）
- `POST /api/paraclate/ingest/summary`
- `POST /api/paraclate/ingest/event`
- `GET  /api/paraclate/config/{tid}`（is22 がpullで同期）
- `POST /api/paraclate/connect`

#### 認証（固定）
- Ingest系: request body に `lacisOath: { lacisID, tid, cic, blessing? }` を含める。
- config取得: `X-Lacis-ID / X-Lacis-TID / X-Lacis-CIC` ヘッダで認証する（GETでbodyを持たないため）。
- いずれも mobes2.0 側で **DeviceAuth（tid+lacisID+cic）検証**を実施。

#### fid境界（固定）
- fid=0000 は “全施設アクセス”の予約値。通常運用のイベント/スナップショット保存に使用しない。
- 受信payloadから fid を確定できない場合は 400（推測禁止）。
- 越境が必要な場合のみ blessing を要求し、検証に合格した場合だけ許可。

---

### 4.3 スナップショット保存（LacisFiles）
- Event ingest で snapshot を受け取った場合、Paraclate APP が **LacisFilesへ保存**する（is22直PUTは禁止）。
- is22側へは `storagePath`（恒久参照子）を返す。署名URLをDB/BQに保存しない。

**storagePath例**
- `lacisFiles/{tid}/{fileId}.jpg`

---

### 4.4 Retention（自動削除）とコスト制御（必須）
- デフォルト保持日数: **60日**
- 期限超過の snapshot は **hard delete（Storage物理削除）**する。Firestore doc のみ削除ではコストが下がらない。
- retention job は Cloud Scheduler（毎日）で実行し、tenant/facility override を反映して削除対象を決定する。

---

### 4.5 Bless（越境権能 / LacisOath拡張）
Blessは Paraclate専用機能ではなく、**LacisOathの権限拡張機能**として導入する。

- コレクション: `systemBlessings/{blessingId}`
- 発行条件: permission >= 91（Prophet以上）
- 付与スコープ: allowedTids / allowedFids / allowedCameraLacisIds（必要なら） / expiresAt / purpose / capabilities
- 取消（revoked）と監査ログ（issuedBy/revokedBy/理由）は必須
- 越境を伴う API では blessing を検証し、未検証越境は禁止

---

### 4.6 Chat API（multi-stage LLM）
ParaclateのUX核。以下を最小要件として実装する。

- `POST /api/paraclate/chat`
  - 入力: tid/fid, userQuery, timeRange（省略可）
  - 出力: 回答 / 根拠ID（eventId/summaryId）/ 追加質問候補

**multi-stage（固定）**
- Stage A（Flash）: 意図分類・不足情報抽出
- Stage B（Tool）: is22 API + facilityContext + cameraContext +（将来）rooms/reservations/horarium/celestial-globe +（必要なら）BQ
- Stage C（Pro）: 統合推論（GrandSummary/深掘り）
- Stage D（Flash）: UX文体の最適化（穏やか・対話的、ただし不確実性は必ず明示）

---

## 5. データモデル（mobes2.0側）

### 5.1 aiApplications / overrides（SSoT）
- `aiApplications/paraclate`
- `/tenants/{tid}/aiModeOverrides/paraclate`
- `/facilities/{fid}/aiModeOverrides/paraclate`

### 5.2 systemBlessings（SSoT）
- `systemBlessings/{blessingId}`

### 5.3 LacisFiles（SSoT）
- `lacisFiles/{tid}/{fileId}`

### 5.4 Subscription（コスト上限）
- `tenants/{tid}/subscription/current`
- plan: free/pro/enterprise（maxStorage: 1GB / 100GB / 無制限）
- Paraclate snapshot は plan と使用量により保存を制御する（ログは受ける・画像保存は抑制可能）。

---

## 6. BigQuery（dataset固定）
- dataset: `paraclate`
- tables（最低限）:
  - `paraclate.semantic_events`
  - `paraclate.summaries`

※既存 `ai_logs.*`（SemanticTags/Metatron）と衝突させない。

---

## 7. Pub/Sub（通知だけ / 実体はpull）
- 目的: 設定変更を is22 に通知して同期を早める（実体をpushしない）
- Topic例: `paraclate-config-updates`
- is22 は通知を受けたら `GET /api/paraclate/config/{tid}` を呼び直して同期する。

---

## 8. 非スコープ（この段階でやらない）
- リッチな可視化UI（3D/高度なタイムライン/高度な検索UI）
  - is21/is22 の AIチャット機能拡充後に再計画（現フェーズはバックエンド重視）
- is22 内部の検知アルゴリズム詳細（is21/is22側の責務）

---

## 9. 受入条件（最低限のDoD）
1. AI Model Settings に `paraclate` が存在し、tenant/facility override が保存できる  
2. is22 → mobes2.0 の ingest が成功し、tid/fid境界違反は 403 で止まる  
3. snapshot あり event が LacisFiles に保存され、storagePath が is22 に返る  
4. retention job が動き、期限超過の画像が hard delete される  
5. blessing無し越境は拒否、blessingあり越境は許可され監査が残る  
6. BigQuery dataset `paraclate` に同期できる（衝突しない）

---

## 10. 参照（設計SSoT）
- aranea_ISMS:
  - `code/orangePi/is21-22/designs/headdesign/Paraclate/BasicDesign/`
  - `code/orangePi/is21-22/designs/headdesign/Paraclate/DetailedDesign/`（DD01-DD05）
- mobes2.0:
  - `src/presentation/pages/system/AIModelSettingsPage.tsx`
  - `doc/APPS/INDEX.md`
  - `src/core/entities/Subscription.ts`

---

## 変更履歴
- 2026-01-10: 初版（mobes2.0側Paraclate APPの責務・SSoT・I/F・コスト制御を固定）
