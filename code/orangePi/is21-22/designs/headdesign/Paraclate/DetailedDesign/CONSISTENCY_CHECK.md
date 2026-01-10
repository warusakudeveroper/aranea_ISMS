# is22詳細設計 vs mobes2.0設計 整合性チェックレポート

作成日: 2026-01-10
対象: DD01-DD05 vs paraclateAPP(mobes2side)/*.md

## 概要

is22側の詳細設計ドキュメント（DD01-DD05）とmobes2.0側のParaclate APP設計ドキュメントを突合し、整合性を確認した結果をまとめる。

---

## ❌ 不整合（実装前に必ず解決）

### 1. retention_days デフォルト値

| ドキュメント | 値 | 記載箇所 |
|-------------|-----|---------|
| DD03 paraclate_config | 30日 | 3.1節 |
| mobes2.0 limitedIssue.md | **60日** | 2.3節 |

**影響**: 設定SSoT（mobes2.0）と初期値が乖離
**修正方針**: DD03の`retention_days default`を60に変更

---

### 2. summaryID 形式の不整合

| ドキュメント | 形式 | 記載箇所 |
|-------------|------|---------|
| DD02 SummaryOverview | `SUM-{id}` | P1-1対応で修正済み |
| DD04 build_summary_payload | `sum_{id}` | 5.3節 enqueuer.rs |

**影響**: Paraclate APPとBQで異なるsummaryIDになり、join不可
**修正方針**: DD04を`SUM-{id}`形式に統一

---

### 3. config endpoint の認証

| ドキュメント | 状態 | 記載箇所 |
|-------------|------|---------|
| DD03 7.エラーハンドリング | 「全リクエストにlacisOath必須」 | セキュリティ考慮 |
| DD03 config_sync.rs | **未認証GET** | 5.4節 疑似コード |
| mobes2.0 APPPreRequirement | 「config取得にもlacisOath必須」 | 7.未決事項 |

**影響**: 設定漏洩リスク
**修正方針**: DD03 config_sync に lacisOath 認証を追加（ヘッダまたはクエリ）

---

### 4. Event payload に snapshot フィールドがない

| ドキュメント | 状態 | 記載箇所 |
|-------------|------|---------|
| DD03 ingest/event | `detectionDetail` のみ | 4.1節 |
| mobes2.0 APPPreRequirement | `snapshot`（base64 or URL）を許可 | P0-2 (E) |

**影響**: is22からスナップショットを送信できない（LacisFiles連携不可）
**修正方針**: DD03のEvent payloadに`snapshot`フィールドを追加

---

### 5. DD04 synced_to_bq 更新のPK問題（レビューP0-5）

| テーブル | 主キー |
|---------|--------|
| detection_logs | `id` |
| ai_summary_cache | `summary_id` |
| is21_logs | `id` |

**現状**: `WHERE id = ?` で固定
**影響**: ai_summary_cache の更新が失敗
**修正方針**: テーブルごとのPKマップを実装

---

### 6. DD01 JSONキーとRust structの命名（レビューP0-6）

| 箇所 | JSONキー | Rust struct |
|------|----------|-------------|
| Request例 | `tenantPrimaryAuth` | `tenant_primary_auth` |

**影響**: デシリアライズ失敗
**修正方針**: `#[serde(rename_all = "camelCase")]` を追加

---

## ⚠️ 要確認・確定事項

### 7. endpoint の定義（基準固定が必要）

| ドキュメント | 定義 |
|-------------|------|
| DD03 | 曖昧（client.rsで `endpoint + "/ingest/summary"` 形式） |
| mobes2.0 APPPreRequirement | `https://.../api/paraclate` まで含むベースURL |

**確定案**: `paraclate.endpoint = "https://api.mobes.app/api/paraclate"` として統一

---

### 8. snapshot_url の形式（恒久参照子 vs 署名URL）

| オプション | メリット | デメリット |
|-----------|---------|-----------|
| 署名URL | 直接アクセス可能 | 期限切れで無効化 |
| storagePath | 恒久的に参照可能 | 取得時に再署名が必要 |

**mobes2.0推奨**: `storagePath`（恒久参照子）
**確定待ち**: BQ格納形式として何を正とするか

---

### 9. BQ dataset 名（衝突回避）

| 既存 | 新規（Paraclate用） |
|------|-------------------|
| `ai_logs.semantic_events`（SemanticTags用） | `paraclate.semantic_events` |

**mobes2.0推奨**: dataset=`paraclate` として分離
**確定待ち**: DD04の設定ファイル例を更新

---

### 10. Pub/Sub 設計の詳細度

| ドキュメント | 状態 |
|-------------|------|
| DD03 | 「Pub/Sub設定変更通知の受信」がスコープ内と記載のみ |
| mobes2.0 APPPreRequirement | モジュール5として「Pub/Sub（設定変更通知・状態通知）」 |

**状況**: is22側のPub/Sub受信実装の詳細設計が不足
**対応**: DD03にPub/Sub受信フローを追記、または別DDで設計

---

## ✅ 整合済み

| 項目 | 確認結果 |
|------|---------|
| ProductCode（4桁数字） | DD05 P0-1で修正済み。mobes2.0と整合 |
| カメラProductType=801 | DD05とmobes2.0で一致 |
| is22 ProductType=022 | DD01とmobes2.0で一致 |
| eventId形式 `det_{id}` | DD04とmobes2.0で一致 |
| lacisID形式（20桁） | 全ドキュメントで統一 |
| tid/fid境界（0000禁止） | DD03 P0-2で修正済み。mobes2.0と整合 |
| blessing概念 | DD03とmobes2.0で設計意図が一致 |
| SSoT分担 | テナント=mobes、カメラ台帳=is22、長期=BQ で合意 |

---

## 修正タスクリスト

### P0（実装ブロッカー）✅ 全件完了

- [x] **DD03**: retention_days default 30→60 に修正 ✅
- [x] **DD04**: summaryID形式 `sum_`→`SUM-` に統一 ✅
- [x] **DD03**: config_sync に lacisOath 認証を追加（X-Lacis-* ヘッダ使用）✅
- [x] **DD03**: Event payload に snapshot フィールド追加（+ snapshotRef応答）✅
- [x] **DD04**: synced_to_bq 更新のPKマップ実装（テーブル別PK対応）✅
- [x] **DD01**: serde rename_all = "camelCase" 追加 ✅

### P1（品質改善）

- [ ] **DD03**: endpoint 定義を明確化（/api/paraclate まで含む）
- [ ] **DD04**: snapshot_url 形式を storagePath に確定
- [ ] **DD04**: dataset名を `paraclate` に明示
- [ ] **DD03**: Pub/Sub受信フローの詳細設計追記

---

## 次のアクション

1. ~~上記P0修正をDD01-DD05に反映~~ ✅ 2026-01-10完了
2. コミット＆プッシュ
3. mobes2.0側チームとの確定事項（endpoint形式、snapshot_url形式、dataset名）を協議
4. Issue #112 に整合性チェック結果を追記
