# NOTE (outdated → 現行値反映)

## 修正履歴
- Before: デフォルトSSID/PASSの記載なし（空ファイル）。
- After: 現行ファームのデフォルトWi-Fi候補（cluster1-3, pass=isms12345@）を明記し、旧ドキュメントにあった「cluster1-6」との差分を解消。

## 現行デフォルトSSID/PASS（2025-01, is04a/is05a/is10共通）
| 優先 | SSID | PASS | 備考 |
|------|------|------|------|
| 1 | cluster1 | isms12345@ | AraneaSettings.h デフォルト |
| 2 | cluster2 | isms12345@ | 同上 |
| 3 | cluster3 | isms12345@ | 同上 |
| 4-6 | なし | なし | 旧資料の予備枠。現行コードでは未使用。 |

- APモードSSID: `ar-<device>-<MAC6>`（各デバイスのAraneaSettings準拠）。
- 変更が必要な場合は AraneaSettings.* と WebUI 設定フォームを同時更新すること。

# TODO
- 運用で cluster4-6 を使う場合は値を定義し、SettingManager/UI/ドキュメントを同期させる。
- 本番用SSID/PASSを共有する際の配布・ローテーション手順をセキュリティポリシーに沿って明文化する。
