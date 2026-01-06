# Camscan機能改善 設計ドキュメント一覧

## 概要

`Camscan_designers_review.md`で指摘された11項目の問題に対する設計ドキュメント群です。

## ★ 重要: レビュー修正 + SSoT統合設計

| ファイル名 | 内容 | ステータス |
|-----------|------|-----------|
| [review_fixes_and_oui_rtsp_ssot_design.md](./review_fixes_and_oui_rtsp_ssot_design.md) | **レビュー指摘8項目の修正 + OUI/RTSPパスSSoT統合** | 設計完了 |
| [TASK_INDEX.md](./TASK_INDEX.md) | **タスク一覧・テスト計画・GitHub Issue** | 登録完了 |
| [Camscan_design_review_report.md](./Camscan_design_review_report.md) | 設計レビュー報告（指摘事項） | レビュー完了 |

### レビュー修正内容
- High #1: クレデンシャル表示方針（マスクなし）
- High #2: ScannedDevice拡張フィールドSSoT統一
- Medium #3: カテゴリFとStrayChild優先順位
- Medium #4: サブネット削除CIDR汎用化
- Medium #5: OUI追加候補/確定の明確化
- Medium #6: 進捗計算動的算出化
- Medium #7: 強制登録デフォルト値検証
- Low #8: テスト計画網羅化

### OUI/RTSPパスSSoT統合
- OUI情報のDB管理（camera_brands, oui_entries）
- ブランド別RTSPテンプレート管理（rtsp_templates）
- 汎用RTSPパス（generic_rtsp_paths）
- 設定モーダルからの閲覧・編集UI
- ユーザーによる新規ブランド登録

---

## 初回設計ドキュメント一覧

| # | ファイル名 | 内容 | ステータス |
|---|-----------|------|-----------|
| 1 | [OUI_expansion_design.md](./OUI_expansion_design.md) | OUI判定リスト拡充（48件追加、計71件） | 修正版で統合 |
| 2 | [category_display_design.md](./category_display_design.md) | スキャン結果カテゴリ表示改善 | 修正版で統合 |
| 3 | [backend_scan_design.md](./backend_scan_design.md) | バックエンドスキャン実行化 | 設計完了 |
| 4 | [brute_force_mode_design.md](./brute_force_mode_design.md) | Brute Force Modeトグル追加 | 設計完了 |
| 5 | [stray_child_scan_design.md](./stray_child_scan_design.md) | StrayChildScan（迷子カメラ検出） | 修正版で統合 |
| 6 | [camera_name_display_design.md](./camera_name_display_design.md) | 登録済みカメラ名表示機能 | 修正版で統合 |
| 7 | [progress_display_design.md](./progress_display_design.md) | 進捗表示改善 | 修正版で統合 |
| 8 | [ui_ux_improvement_design.md](./ui_ux_improvement_design.md) | UI/UX改善（文言・ボタン） | 設計完了 |
| 9 | (8に統合) | スキャン開始ボタンUI改善 | 8に統合 |
| 10 | [force_register_design.md](./force_register_design.md) | 強制登録機能 | 修正版で統合 |
| 11 | [subnet_cleanup_design.md](./subnet_cleanup_design.md) | サブネット削除時クリーンアップ | 修正版で統合 |

## 関連ドキュメント

- [Camscan_designers_review.md](./Camscan_designers_review.md) - 設計者レビュー（元ドキュメント）
- [The_golden_rules.md](/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/aranea_ISMS/The_golden_rules.md) - 開発ルール

## 実装優先度（推奨）

### Phase 1: 基盤改善（必須）
1. OUI判定リスト拡充 - 即座に効果が出る
2. バックエンドスキャン実行化 - アーキテクチャ変更

### Phase 2: UX改善
3. カテゴリ表示改善
4. 登録済みカメラ名表示
5. 進捗表示改善
6. UI/UX改善（文言・ボタン）

### Phase 3: 機能追加
7. Brute Force Mode
8. 強制登録機能
9. StrayChildScan
10. サブネット削除時クリーンアップ

## 次のステップ

1. [x] 設計レビュー実施
2. [x] GitHub Issueへの登録 (#80〜#86)
3. [ ] 実装開始前のコミット
4. [ ] Phase 1から順次実装

## GitHub Issues

- [#80 DBマイグレーション](https://github.com/warusakudeveroper/aranea_ISMS/issues/80)
- [#81 CameraBrandService + API](https://github.com/warusakudeveroper/aranea_ISMS/issues/81)
- [#82 スキャン機能改善](https://github.com/warusakudeveroper/aranea_ISMS/issues/82)
- [#83 強制登録・クレデンシャル監査](https://github.com/warusakudeveroper/aranea_ISMS/issues/83)
- [#84 フロントエンド: ブランド設定UI](https://github.com/warusakudeveroper/aranea_ISMS/issues/84)
- [#85 フロントエンド: スキャン結果表示改善](https://github.com/warusakudeveroper/aranea_ISMS/issues/85)
- [#86 ハードコード削除・テスト完了](https://github.com/warusakudeveroper/aranea_ISMS/issues/86)

---

**作成日**: 2026-01-07
**作成者**: Claude Code
