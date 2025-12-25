# 08 ドキュメント管理・メタデータ・AI連携（Metatron/Explore/Obsidian的運用）

## 目的
- araneaDevice 関連ドキュメントを SDK 経由で一元保管し、メタデータ付きで参照・投稿可能にする。
- SemanticTags / OperationTasksTest（aimodelSettings で多段参照）を流用し、AI モデルが関連文書を自動探索・タグ付け・回答の根拠提示まで行う。
- mobes 側 AdminSettings に混在している lacisOathAPI 情報から分離し、新設タブ「araneaDevices」に集約する。

## UI/タブ設計（mobes 側）
- AdminSettings に「araneaDevices」タブを追加（araneadevice アイコン）。lacisOathAPI から araneaDevice 情報を分離。
- 機能:
  - ドキュメント閲覧: MD を表示（Obsidian 的リンク・タグ・グラフビューは将来対応）。
  - SDK 投稿: SDK から push された MD/JSON をレビュー→承認→保存（権限: mobes-admin/aranea-dev）。
  - メタデータ編集: SemanticTags（key/value, topic, deviceType, productType, schemaVersion 等）を付与・編集。
  - AI パネル: OperationTasksTest/Metatron を呼び出し、関連文書検索・サマリ・タグ候補提示。

## メタデータ設計
- 最低限のキー:
  - `typeId`, `productType`, `productCode`, `deviceModel`
  - `schemaVersion`, `sdkVersion`
  - `semanticTags`: 配列（例: ["register", "state-report", "mqtt", "security"]）
  - `source`: sdk-upload | manual | import
  - `hash`: sha256(content) （改ざん検知）
  - `createdBy`, `updatedBy`, `updatedAt`
- 参照リンク:
  - `relatedDocs`: MD や schema への相互リンク
  - `lacisFilesRef`: Lacisfiles 側の格納パス/ID（ファイル実体を保持する場合）

## フロー（SDK ⇔ mobes ⇔ AI）
1. SDK 側: MD/JSON（仕様・設計・テスト結果）を作成し、メタデータを付与して `cli/docs push`。
2. mobes 側: AdminSettings > araneaDevices でレビュー→承認→Firestore/Lacisfiles に保存（hash 記録）。
3. AI（Metatron/OperationTasksTest）: SemanticTags で関連文書を多段参照し、QA/要約/タグ提案を実行。
4. SDK 側: `cli/docs fetch --tags ...` で検索・取得し、ローカル Obsidian 的に閲覧・編集。

## OperationTasksTest / SemanticTags の再利用
- OperationTasksTest: 「関連文書を多段参照」ジョブをタスク化し、AI にまとめて渡す。
- SemanticTags: 既存タグ構造を流用し、typeId/productType/schemaVersion を必須タグとして自動付与。
- Metatron（文書係）: タグ提案・要約・リンク更新を担当。SDK からは「タグ再計算」ジョブをキックできるようにする。

## Lacisfiles / Explore エージェント連携
- Lacisfiles: ドキュメント実体の保存先。hash とメタデータを Firestore に記録し、Lacisfiles ID を `lacisFilesRef` として保持。
- Explore エージェント: 既存 araneadevice ドキュメントをクロールし、メタデータ不足を検知→タグ追加提案を行うジョブを並列実行。

## CLI/SDK 要件
- `cli/docs push --file ... --meta meta.json --env dev`  
  メタデータ付きでアップロード（hash 計算、typeId/productType を自動抽出）。
- `cli/docs fetch --tags ... --typeId ...`  
  SemanticTags で検索し、MD/JSON をローカルへ取得。
- `cli/docs reindex --env dev`  
  Explore/Metatron によるタグ再計算タスクをキック。
- `cli/docs verify --file ...`  
  hash/必須タグ/typeId/productType の一貫性をチェック。

## 権限制御
- 投稿/編集: mobes-admin, aranea-dev。CI/SDK はサービスアカウントで限定権限。
- 閲覧: 認証済みユーザー。センシティブ情報（資格情報など）は `.env` に分離し、MD には平文で書かない。

## リスクと緩和
- メタデータ欠落 → `verify` で必須タグチェック。hash 未登録は拒否。
- ドキュメント肥大 → Lacisfiles へ移し、Firestore にはメタデータのみ保存。
- AI の誤タグ付け → ヒューマンレビュー必須フラグを設け、承認後に確定。
