# メンテナンススクリプト

開発環境のメンテナンス用スクリプト集

## スクリプト一覧

### cleanup_settings.py

Claude Code の `settings.local.json` から不正なエントリを削除するスクリプト。

#### 背景

Claude Code CLI には、複雑なBashコマンド（heredoc、シェル変数展開など）を許可リストに追加する際に不正なエントリを書き込むバグがあります。

関連Issue:
- https://github.com/anthropics/claude-code/issues/14572
- https://github.com/anthropics/claude-code/issues/15742
- https://github.com/anthropics/claude-code/issues/12658

#### 検出する問題パターン

| パターン | 例 | 原因 |
|---------|-----|------|
| `%%` | `Bash(name="$device%%:*")` | シェル変数展開の断片 |
| `##` | `Bash(ip="$device##*:")` | シェル変数展開の断片 |
| `.*` (`:*`以外) | `grep -E 'foo.*bar'` | 正規表現がワイルドカードと誤認 |

#### 使い方

```bash
# プロジェクトルートから実行

# 確認のみ（変更しない）
python3 scripts/maintenance/cleanup_settings.py --dry-run

# 実行（問題エントリを削除）
python3 scripts/maintenance/cleanup_settings.py
```

#### 定期実行の推奨

設定エラーが発生した場合、または定期的に実行することを推奨：

```bash
# エラーが出た時
# "Files with errors are skipped entirely" が表示されたら実行
python3 scripts/maintenance/cleanup_settings.py
```

## ディレクトリ構造

```
scripts/
└── maintenance/
    ├── README.md              # このファイル
    └── cleanup_settings.py    # 設定ファイルクリーンアップ
```
