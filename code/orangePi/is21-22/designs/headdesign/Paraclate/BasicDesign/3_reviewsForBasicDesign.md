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
  - TypeDomain表記: 当初DesignOverviewは`araneaDevices`だったが、第2回・第3回レビューで全ドキュメントを`araneaDevice`に統一済み。
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

### 2026-01-10 第2回レビュー対応

#### P0-1: 02_is21_status.md のFID不一致
| ドキュメント | 修正前 | 修正後 |
|-------------|--------|--------|
| investigation_report/02_is21_status.md (セクション8.2) | `FID: 9966` | `FID: 0000` |

**対応内容**:
- 権限設定セクションに「設計（To-be）」ラベルを追加
- FID値を0000（全施設を表す特殊値）に修正
- 注釈を追加してfid=0000とPermission71の関係を明記

#### P0-2: DesignOverview JSON形式の修正
| 修正箇所 | 修正前 | 修正後 |
|---------|--------|--------|
| Paraclate_DesignOverview.md (L17-39) | 不正なJSON（`"key","value"`形式） | 有効なJSON（`"key":"value"`形式） |

**対応内容**:
- summaryOverviewのキー・バリュー区切りを`,`から`:`に修正
- cameraContextを配列形式からオブジェクト形式に変更
- cameraDetectionをオブジェクト配列に変更
- `fendDetectAt`を`lastDetectAt`に修正（タイポ）

#### P0-3: DesignOverview TypeDomain修正
| 修正前 | 修正後 |
|--------|--------|
| `Typedomain=araneaDevices` | `TypeDomain=araneaDevice` |
| `araneaDecvices共通` | `araneaDevice共通` |
| `ar-is22Camsetver` | `ar-is22CamServer` |

**対応内容**: 実装仕様の`araneaDevice`（単数形）に統一。タイポも併せて修正。

#### P0-4: 05_paraclate_integration.md 認証方式統一
| 修正前 | 修正後 |
|--------|--------|
| JWTベアラートークン認証 | lacisOath認証 |
| HTTPヘッダー認証 | リクエストボディ内lacisOathオブジェクト |

**対応内容**:
- セクション3.2を「認証方式（lacisOath）」に改題
- lacisOathフィールド（lacisID/tid/cic/blessing）の説明表を追加
- JWT/Bearer認証ではなくlacisOath方式であることを明記

#### P1-2: FunctionArrangement 日本語化・SSoT表記修正
| 修正箇所 | 修正前 | 修正後 |
|---------|--------|--------|
| L47 | `아직`（韓国語） | `現在`（日本語） |
| SSoT表 | 列幅不揃い・不明瞭 | 整形済み・「SSoTソース」列明示 |

**対応内容**:
- 韓国語混入を日本語に置換
- SSoT割当表を整形し、列名を明確化
- 画像保存（LacisFiles）行を追加

#### 確認済み項目（第2回）
- ✅ P0指摘事項: 全4件対応完了
- ✅ P1指摘事項: 1件対応完了
- ✅ ドキュメント間整合性: 統一済み
- ✅ The_golden_rules.md準拠: 継続

---

### 2026-01-10 第3回レビュー対応

第2回レビュー対応後の整合性レビューで追加指摘された3点を修正。

#### (A) P0相当: 03_authentication.md TypeDomain/Type旧表記
| 修正箇所 | 修正前 | 修正後 |
|---------|--------|--------|
| 表2.2 TypeDomain | `araneaDevices` | `araneaDevice` |
| 表2.2 Type | `ar-is22Camserver` | `ar-is22CamServer` |
| 表2.2 Prefix根拠 | `araneaDevices共通` | `araneaDevice共通` |
| Rust例 device_type | `ar-is22Camserver` | `ar-is22CamServer` |

**対応内容**: DesignOverviewと同一値に統一。AraneaRegister実装時の参照元として整合性を確保。

#### (B) P1: 05_paraclate_integration.md Summary例旧フォーマット
| 修正箇所 | 修正前 | 修正後 |
|---------|--------|--------|
| summaryOverview | `fendDetectAt` | `lastDetectAt` |
| cameraContext | 配列形式 | オブジェクト形式 |
| cameraDetection | CSV文字列配列 | オブジェクト配列 |

**対応内容**: DesignOverviewの新フォーマットと完全一致させ、フォーマット仕様の説明を追加。

#### (C) P1: DesignOverview lacisOath JSON構文破損
| 修正前 | 修正後 |
|--------|--------|
| JSONとして無効（`lacisOath:`キー未括弧、`cic/fid`混線） | 有効なJSON + フィールド説明表追加 |

**対応内容**:
- lacisOathを正規JSONオブジェクト形式に修正
- フィールド説明表（lacisID/tid/cic/blessing）を追加
- blessingの説明を引用ブロックに分離

#### 確認済み項目（第3回）
- ✅ P0相当指摘: 1件対応完了（03_authentication TypeDomain/Type統一）
- ✅ P1指摘: 2件対応完了（Summary例更新、lacisOath JSON正規化）
- ✅ ドキュメント間整合性: 全ファイルで統一確認

---

**レビューステータス**: 第3回レビュー対応完了 ✅ - ドキュメント間整合性OK
