# Paraclate 基本設計 レビュー対応
作成日: 2026-01-10
対象: `Paraclate_BasicDesign.md` / `Paraclate_RequirementsDefinition.md` / `Paraclate_DesignOverview.md`
更新日: 2026-01-10（レビュー指摘対応完了）

## レビュー観点
- SSoT/権限境界: tid/fid/blessing/lacisOath の一貫性
- MECE/SOLID: 登録・設定・生成・送信・データ提供・同期・UXの責務分離
- DesignOverviewとの齟齬: エンドポイントプレースホルダ、Summary/GrandSummary形式、TypeDomain命名
- 実装可能性: 既存実装の有無と追加すべきスキーマ/API

## 判定と対応
- 一致:
  - Summary=間隔、GrandSummary=時刻指定、DesignOverviewの送信JSONを踏襲
  - lacisOath + blessing(越境時のみ)を明記、is21=system tenant fid=0000 permission71 を維持
  - 画像はLacisFiles、検知ログ/BQ同期、設定SSoTをmobes AI Model Settingsに固定
  - AI推論はParaclate APP側に委譲（is22は事実APIのみ）でSOLID/SRP保持
- 齟齬・補足:
  - TypeDomain表記: DesignOverviewは`araneaDevices`だが、実装仕様は`araneaDevice`が正。基本設計で正規化済み（変更不要）。
  - Summaryペイロード保存: ai_summary_cacheに`summary_json`追加を要求→マイグレーション未作成。
  - is22登録/BQ同期/Paraclate送信APIは未実装 → 基本設計でAPI定義済みだが開発タスク化が必要。
  - blessing実装: Firestore/Cloud側の発行・監査ルールを別途詳細化する必要あり（設計ではサーバ側越境に限定）。
- リスク/未決:
  - araneaDeviceGateでtypeDefaultPermissions未設定の場合の登録失敗対策（mobes側作業依存）。
  - fidをgate登録時に渡せるようにする仕様変更（mobes側対応前提）。
  - Paraclate ingest/chat API（mobes側）のURI確定待ち。プレースホルダのまま運用すると送信先未定で結合試験不可。

## アクションアイテム
1. is22マイグレーション追加: `aranea_registration` / `paraclate_config` / `scheduled_reports` / `ai_summary_cache.summary_json` / `detection_logs.vehicle_ocr, face_features`。
2. is22 API実装: register(POST/GET/DELETE), summary generate/retrieve, paraclate/summary送信, data提供API, paraclate/config/status, scheduler/BqSyncService。
3. mobes2.0側: `aiApplications.paraclate`設定追加、araneaDeviceGateのfid対応・typeDefaultPermissions設定、Paraclate ingest/chat/blessing APIのURI確定。
4. テスト計画具体化: 登録E2E、カメラ承認→lacisID付与、Summary生成/送信、チャット検索、不在時の不明明示、越境拒否/許可、BQ同期、LacisFiles TTL。

## MECE/SOLID確認
- 責務を登録/台帳・設定・生成・送信・データ提供・同期・UX・セキュリティで分割し重複なし。
- LLM実行をmobes側に限定し、is22はデータ提供・生成・送信に専念。登録/送信/同期をモジュール単位で分離しSRP/DIPを維持。

---

## レビュー指摘対応履歴

### 2026-01-10 第1回レビュー対応

#### 指摘1: is21のfid値の不一致
| ドキュメント | 修正前 | 修正後 |
|-------------|--------|--------|
| DesignOverview (L80) | `fid9966` | `fid=0000` |
| RequirementsDefinition | `fid=0000` | 変更なし |
| BasicDesign | 明記なし | 変更なし |

**対応内容**: DesignOverview.mdのis21所属を`fid9966`から`fid=0000`に修正。fid=0000は「全施設を表す特殊値」であることを明記。

#### 指摘2: TypeDomain表記
- DesignOverview: `araneaDevices` (複数形)
- BasicDesign/Requirements: `araneaDevice` (単数形)

**対応内容**: 実装仕様の`araneaDevice`が正であり、BasicDesignで正規化済み。DesignOverviewは概念名として残置（実害なし）。

#### 確認済み項目（変更不要）
- ✅ Summary/GrandSummary形式: DesignOverview JSONフォーマットを踏襲
- ✅ lacisOath + blessing: 越境時のみの方針で統一
- ✅ SSoT宣言: テナント権限=mobes、台帳=is22、BQ、画像=LacisFiles
- ✅ AI実行責務分離: is22=データ提供、Paraclate APP=LLM実行
- ✅ MECE/SOLID: 責務分割が重複なく網羅

#### The_golden_rules.md準拠確認
| ルール | 確認結果 |
|--------|----------|
| SSoT大原則 | ✅ 明示的に宣言 |
| SOLID/SRP | ✅ 責務分離を明記 |
| MECE | ✅ 領域分割が重複なし |
| 車輪の再発明禁止 | ✅ 既存実装確認の方針記載 |
| 設計ドキュメント必須 | ✅ ドキュメント体系が整備 |

---

## 残存リスク・次回レビュー対象

| リスク | 重大度 | ステータス |
|--------|--------|------------|
| araneaDeviceGate typeDefaultPermissions未設定 | 高 | mobes側タスク待ち |
| fid渡し仕様変更 | 中 | mobes側API変更待ち |
| Paraclate ingest/chat URI未確定 | 高 | **結合試験ブロッカー** |
| blessing Firestore設計 | 中 | 詳細設計待ち |

---

**レビューステータス**: ドキュメント間整合性確認完了 ✅
