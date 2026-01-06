# Camscan設計レビュー報告

## 1. 概要
- 対象: `Camscan_designers_review.md` で挙がった11指摘に対応する設計10件（開始ボタンUIは `ui_ux_improvement_design.md` に統合）
- 実施日: 2026-01-07
- レビュアー: Codex (The_golden_rules.md遵守)
- 目的: 設計と要求の整合・MECE・SSoT準拠を確認し、実装前に齟齬を解消する。

## 2. 判定（重要度順）
### High
1. `category_display_design.md` 試行クレデンシャルが `password_masked` で伏せられており、デザイナー要求「隠す必要なし」と不整合。
2. `category_display_design.md` と `camera_name_display_design.md` がそれぞれ `ScannedDevice` 拡張を別定義（`registered_camera_name` など）。SSoTが分岐し実装時に型不整合が発生する。

### Medium
3. 新規カテゴリF「通信断カメラ」のデータソース/算出ロジック（最終応答時刻、StrayChildとの優先順位）が未定義でMECEが崩れる恐れ。
4. `subnet_cleanup_design.md` の削除クエリが `/24` 前提 (`0xFFFFFF00`)。/23 など他CIDRを取りこぼすリスク。
5. `OUI_expansion_design.md` でPanasonic/Eufyを「追加候補」と記載しつつコード例に即時追加。確定/保留の境界がアンビギュアス。
6. `progress_display_design.md` の総ポイント計算が `/24=253` を固定値とし、マルチCIDR/ARPバイパス加算を考慮していない。進捗%乖離の恐れ。
7. `force_register_design.md` のデフォルト `rtsp://<ip>:554/stream1` と `pending_auth` が既存機種(Tapo/VIGI等)・状態遷移と整合するか未検証。

### Low
8. `ui_ux_improvement_design.md` のテスト計画がUIのみで、Golden Ruleのフロント/バック/Chrome実UI網羅が未記載。新バッジ/グラデが既存デザインシステムとの整合未確認。

## 3. 追加確認が必要な論点
- 試行クレデンシャルの保存/開示ポリシー（いつ・どこまで永続化するか、マスキング要否）。
- カテゴリFとStrayChild検出 (`stray_child_scan_design.md`) の優先順位と重複表示ルール。
- 強制登録時に巡回対象へ自動追加するか否か、既存ステータス遷移との整合。

## 4. 推奨アクション
1. `ScannedDevice` 拡張フィールドを一本化し、SSoTを明示（型定義も含めて1ファイルに寄せる）。
2. クレデンシャル表示方針をデザイナー要求に合わせ再定義（マスク無し/部分表示の是非を決定）。
3. カテゴリFの判定アルゴリズムとStrayChildとの優先順位を仕様化し、テストケースを追加。
4. サブネット削除クエリをCIDR汎用に修正する設計へ変更。
5. OUI「追加候補」と「確定追加」を分離し、根拠と適用タイミングを明記。
6. 進捗計算を対象サブネット・実測ホスト・ARPバイパス登録数で動的算出するよう再設計。
7. ベンダー別強制登録デフォルト値と状態遷移を既存実装と突き合わせる。
8. テスト計画にバックエンド/API/Chrome実UIを追加し、既存デザインとの整合チェックを明文化。

## 5. MECE確認
- 指摘事項をHigh/Medium/Lowで重複なく分類。
- 対象ドキュメント10件を漏れなく評価（開始ボタンUIは統合先で確認）。
- 未確定事項は「追加確認が必要な論点」に集約し、実装タスクと分離。

## 6. 大原則遵守状況（The_golden_rules.md）
- SSoT: フィールド定義の一本化が必要と判定し、再設計アクションを提示。
- MECE: 本報告で分類と漏れなしを確認。
- アンアンビギュアス: 不整合箇所を具体的に列挙し、是正案を提示。
- チェック/テスト: 必要な追加テスト計画を明示。

## 7. 次のステップ
1. 上記アクションを設計更新として反映。
2. 更新後に再レビューし、承認→GitHub Issue化。
3. Phase 1（OUI拡充＋バックエンドスキャン）実装着手前にSSoTとテスト計画を確定。
