# Camscan機能改善 タスクインデックス

## 概要

本ドキュメントは`review_fixes_and_oui_rtsp_ssot_design.md`に基づく実装タスクを定義する。

## 設計ドキュメント参照

| ドキュメント | 内容 | ステータス |
|------------|------|-----------|
| [review_fixes_and_oui_rtsp_ssot_design.md](./review_fixes_and_oui_rtsp_ssot_design.md) | 統合設計（レビュー修正8件+OUI/RTSP SSoT） | 設計完了 |
| [README.md](./README.md) | 設計ドキュメント一覧 | 更新済み |

---

## タスク一覧

### Phase 1: データベースマイグレーション

| ID | タスク | 依存 | 対応設計セクション |
|----|-------|------|------------------|
| T1-1 | camera_brandsテーブル作成 | - | 2.2 |
| T1-2 | oui_entriesテーブル作成（status列含む） | T1-1 | 2.2 |
| T1-3 | rtsp_templatesテーブル作成 | T1-1 | 2.2 |
| T1-4 | generic_rtsp_pathsテーブル作成 | - | 2.2 |
| T1-5 | camerasテーブル拡張（rtsp_template_id, PendingAuth） | T1-3 | 2.4, 7.1 |
| T1-6 | 初期データ投入（ブランド、OUI、テンプレート） | T1-1〜T1-4 | 2.3 |
| T1-7 | 既存カメラRTSPバックフィル | T1-5, T1-6 | 2.4 |

### Phase 2: バックエンド実装

| ID | タスク | 依存 | 対応設計セクション |
|----|-------|------|------------------|
| T2-1 | CameraBrandService実装（CRUD+キャッシュ） | T1-6 | 3.1 |
| T2-2 | カメラブランドAPI実装 | T2-1 | 3.2 |
| T2-3 | OUIエントリAPI実装 | T2-1 | 3.2 |
| T2-4 | RTSPテンプレートAPI実装 | T2-1 | 3.2 |
| T2-5 | 汎用パスAPI実装 | T2-1 | 3.2 |
| T2-6 | API認可・エラーハンドリング実装 | T2-2〜T2-5 | 3.3 |
| T2-7 | DynamicProgressCalculator実装 | - | Medium #6 |
| T2-8 | カテゴリF (LostConnection) 流し込み実装 | - | Medium #3: 3.2 |
| T2-9 | サブネット削除CIDR汎用化（IPv4専用） | - | Medium #4 |
| T2-10 | 強制登録API実装（PendingAuthステータス） | T1-5 | Medium #7 |
| T2-11 | クレデンシャル監査ポリシー実装（24h自動削除） | - | High #1: 1.2 |
| T2-12 | ScannedDevice SSoT型統一 | - | High #2 |

### Phase 3: フロントエンド実装

| ID | タスク | 依存 | 対応設計セクション |
|----|-------|------|------------------|
| T3-1 | カメラブランド設定タブ追加 | T2-2〜T2-5 | 4.1 |
| T3-2 | ブランド一覧・追加モーダル | T3-1 | 4.2 |
| T3-3 | OUI管理UI | T3-1 | 4.2 |
| T3-4 | RTSPテンプレート管理UI | T3-1 | 4.2 |
| T3-5 | 汎用パス管理UI | T3-1 | 4.2 |
| T3-6 | is_builtin編集制限UI表示 | T3-1〜T3-5 | 3.3 |
| T3-7 | スキャン進捗表示改善 | T2-7 | Medium #6 |
| T3-8 | カテゴリF表示対応 | T2-8 | Medium #3 |
| T3-9 | PendingAuthステータスバッジ | T2-10 | Medium #7 |
| T3-10 | クレデンシャル平文表示 | T2-11 | High #1 |

### Phase 4: 既存コード削除・テスト

| ID | タスク | 依存 | 対応設計セクション |
|----|-------|------|------------------|
| T4-1 | oui_data.rs ハードコード削除 | T2-1 | - |
| T4-2 | RtspTemplate ハードコード削除 | T2-1 | - |
| T4-3 | バックエンド単体テスト | T2-* | Low #8 |
| T4-4 | フロントエンド単体テスト | T3-* | Low #8 |
| T4-5 | Chrome実UIテスト | T4-3, T4-4 | Low #8 |
| T4-6 | マイグレーションロールバック検証 | T1-* | 2.4, 7.1 |

---

## テスト計画

### バックエンドテスト

| テストケース | 対象 | 確認項目 |
|------------|------|---------|
| CameraBrandService CRUD | T2-1 | ブランド作成・更新・削除・一覧取得 |
| is_builtin制約 | T2-6 | 組込ブランド編集時403エラー |
| OUI検索性能 | T2-1 | 1000件OUI時のレスポンス<100ms |
| キャッシュリフレッシュ | T2-1 | DB更新後キャッシュ反映 |
| 進捗計算精度 | T2-7 | 各ステージ重み通りの進捗% |
| LostConnection注入 | T2-8 | 未応答カメラがカテゴリFで返却 |
| CIDR範囲削除 | T2-9 | /23, /24, /25 で正確に削除 |
| PendingAuth状態遷移 | T2-10 | 強制登録→認証成功→Active |
| クレデンシャル24h削除 | T2-11 | バッチジョブによる自動クリア |

### フロントエンドテスト

| テストケース | 対象 | 確認項目 |
|------------|------|---------|
| ブランド一覧表示 | T3-2 | 組込/カスタムの区別表示 |
| ブランド追加バリデーション | T3-2 | 必須項目・OUI形式チェック |
| 編集制限表示 | T3-6 | 組込ブランドに🔒アイコン |
| 進捗バー精度 | T3-7 | ステージ別重みに基づく表示 |
| カテゴリFアラート | T3-8 | 通信断カメラの警告表示 |
| PendingAuthバッジ | T3-9 | 認証待ちステータス表示 |
| クレデンシャル平文表示 | T3-10 | マスクなしでパスワード表示 |

### Chrome実UIテスト

| テストケース | 確認項目 |
|------------|---------|
| 設定モーダル→カメラブランドタブ | タブ切り替え動作 |
| ブランド追加→スキャン→認識 | 新規OUIでブランド判定 |
| 汎用パス順序変更→スキャン | 優先度通りの試行順序 |
| 強制登録→認証待ち表示 | PendingAuthバッジ表示 |
| スキャン進捗表示 | ステージ別進捗の可視化 |

---

## GitHub Issue構成

| Issue # | タイトル | 含むタスク | URL |
|---------|---------|-----------|-----|
| #80 | [Camscan] DBマイグレーション: OUI/RTSPブランドSSoT | T1-1〜T1-7 | https://github.com/warusakudeveroper/aranea_ISMS/issues/80 |
| #81 | [Camscan] CameraBrandService + API実装 | T2-1〜T2-6 | https://github.com/warusakudeveroper/aranea_ISMS/issues/81 |
| #82 | [Camscan] スキャン機能改善（進捗・カテゴリF・CIDR・SSoT型） | T2-7〜T2-9, T2-12 | https://github.com/warusakudeveroper/aranea_ISMS/issues/82 |
| #83 | [Camscan] 強制登録・クレデンシャル監査 | T2-10, T2-11 | https://github.com/warusakudeveroper/aranea_ISMS/issues/83 |
| #84 | [Camscan] フロントエンド: ブランド設定UI | T3-1〜T3-6 | https://github.com/warusakudeveroper/aranea_ISMS/issues/84 |
| #85 | [Camscan] フロントエンド: スキャン結果表示改善 | T3-7〜T3-10 | https://github.com/warusakudeveroper/aranea_ISMS/issues/85 |
| #86 | [Camscan] ハードコード削除・テスト完了 | T4-1〜T4-6 | https://github.com/warusakudeveroper/aranea_ISMS/issues/86 |

---

## MECE確認

- [x] 全タスクが設計セクションに紐付け
- [x] 依存関係が明確
- [x] Phase順序が依存関係に従う
- [x] テスト計画がバックエンド/フロントエンド/Chrome実UIを網羅
- [x] GitHub Issue構成が論理的にグループ化

---

**作成日**: 2026-01-07
**作成者**: Claude Code
