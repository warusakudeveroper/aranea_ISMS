# NOTE (outdated)

## 修正履歴
- Before: mobes2.0/Arduino-MCPの概要のみで、最新のURL/運用フロー/AraneaDeviceGateの更新有無が不明。
- After: 最新実装との差分を明示し、URL・手順・動作確認のアップデートをTODO化。本文に2025-01時点の実装整合（Gate/StateReport URL、バックオフ・3sタイムアウト採用、Gate準備中）を追記。

# TODO
- GitHubリポジトリURLとブランチ/デプロイ先の有効性確認
- araneaDeviceGate/APIエンドポイントや認証手順を最新版に合わせる
- 開発・テストコマンドが現行依存関係で動くか再検証

---

## 2025-01 現行整合メモ
- 使用中クラウド関数: araneaDeviceGate（CIC発行）`https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate`、deviceStateReport `https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport`（AraneaSettingsデフォルト）。
- is04a/is05a/is10 は HTTPタイムアウト3s + 3連続失敗で30sバックオフを実装済み。Gate準備待ちでPENDING_TEST継続。
- mobes2.0 リポジトリは継続利用前提だが、最新ブランチ/デプロイ先の確認が必要。UI未使用時も Gate/StateReport は上記URLを利用。
- ツール: Arduino-MCP を現行も利用。`.mcp.json`設定は有効。

# 外部システム連携リファレンス

**必須参照ドキュメント** - ESP32/ISMS開発で使用する外部システムの概要

---

## 1. mobes2.0（IoT統合管理システム）

### 概要
- **リポジトリ**: https://github.com/warusakudeveroper/mobes2.0
- **説明**: Firebase/Firestoreベースの IoT 統合管理システム
- **技術スタック**: React + Firebase (Firestore)

### 役割
- ISMSデバイス（is01〜is05）の登録・認証管理
- araneaDeviceGate エンドポイントによる CIC 発行
- デバイス状態（deviceStateReport）の受信・保存
- 管理UI（将来）

### 開発コマンド
```bash
# 開発サーバー起動
npm start

# テスト実行（Firestoreエミュレータ必要）
firebase emulators:start --only firestore --project demo
npm test

# Firestoreルールテスト
npm run test:firestore

# 本番ビルド
npm run build
```

### ISMSとの連携ポイント

| 機能 | エンドポイント/API | 備考 |
|------|-------------------|------|
| デバイス登録 | POST `https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate` | lacisId + deviceMeta → CIC発行（デフォルトGate URL） |
| 状態送信 | POST `https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport` | is02/is05/is04a/is10 → クラウドへの状態通知（3s timeout/バックオフ実装済み） |
| 認証 | CIC (6桁) | 登録後にデバイスが保持 |

### 現地優先運用での位置づけ
- **クラウド未完成でも現地動作を優先**
- is03（Zero3）がローカルで代替機能を提供
- mobes2.0 が整い次第、送信先を切り替え

---

## 2. Arduino-MCP（ESP32開発環境）

### 概要
- **リポジトリ**: https://github.com/warusakudeveroper/Arduino-MCP
- **説明**: ESP32開発を自動化するMCP（Model Context Protocol）サーバー
- **対応OS**: macOS / Linux / Windows

### 主要機能

| 機能 | 説明 |
|------|------|
| 自動セットアップ | arduino-cli, ESP32コア, pyserial を自動インストール |
| コンパイル/アップロード | arduino-cliをラップした簡単操作 |
| シリアル監視 | 自動ボーレート判定、DTR/RTSリセット対応 |
| ピン検証 | DevKitC GPIO仕様との整合性チェック |
| 複数ESP32対応 | 並列フラッシュ機能 |

### MCP設定（本プロジェクトで使用中）

`.mcp.json` / `.cursor/mcp.json`:
```json
{
  "mcpServers": {
    "mcp-arduino-esp32": {
      "command": "mcp-arduino-esp32",
      "env": {
        "PATH": "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin"
      }
    }
  }
}
```

### 主要ツール

#### 初心者向け（推奨）
```json
{ "name": "quickstart", "arguments": {} }
```
依存関係→コア→検出→コンパイル→アップロード→モニタを一括実行

#### セットアップ
| ツール | 用途 |
|--------|------|
| `ensure_dependencies` | arduino-cli, pyserial等の自動インストール |
| `ensure_core` | ESP32コアのインストール |
| `version` | arduino-cliバージョン確認 |

#### ビルド & アップロード
| ツール | 用途 |
|--------|------|
| `compile` | スケッチのコンパイル |
| `upload` | ESP32へのアップロード |
| `pdca_cycle` | コンパイル→アップロード→モニタを一括実行 |
| `flash_connected` | 接続された全ESP32へ並列アップロード |

#### シリアル監視
| ツール | 用途 |
|--------|------|
| `monitor_start` | シリアル監視開始（自動ボーレート対応） |
| `monitor_stop` | シリアル監視停止 |
| `start_console` | SSEベースのリアルタイムコンソール |

#### ピンユーティリティ（重要）
| ツール | 用途 |
|--------|------|
| `pin_spec` | DevKitCピン仕様テーブル取得 |
| `pin_check` | スケッチのピン使用状況とHW制約の整合性検証 |

### ISMSプロジェクトでの活用

#### ビルドワークフロー
```bash
# is01のコンパイル
{ "name": "compile", "arguments": { "sketch_path": "code/ESP32/is01" } }

# is02のコンパイル
{ "name": "compile", "arguments": { "sketch_path": "code/ESP32/is02" } }
```

#### ピン検証（is01/is02実装前に確認）
```json
{ "name": "pin_check", "arguments": { "sketch_path": "code/ESP32/is01" } }
```
- 入力専用ピン (GPIO34-39) への出力命令を検出
- ブートストラップピン (GPIO0/2/4/5/12/15) の警告
- I2Cピン (GPIO21/22) の競合検出

### 環境変数
| 変数 | 説明 | デフォルト |
|------|------|-----------|
| `ESP32_FQBN` | ボードFQBN | `esp32:esp32:esp32` |
| `ARDUINO_CLI` | arduino-cliパス | `arduino-cli` |
| `MCP_PYTHON` | Python実行パス | `.venv/bin/python` |

---

## 3. ローカル開発の推奨フロー

### 初回セットアップ
1. Arduino-MCPの `quickstart` を実行
2. ESP32が検出され、Blinkサンプルが動作することを確認
3. is01/is02のスケッチをコンパイル

### 日常開発
1. コード編集
2. `pin_check` でピン整合性確認
3. `compile` でビルド
4. `upload` でフラッシュ
5. `monitor_start` でシリアル確認

### 複数デバイスへの一括書き込み
```json
{
  "name": "flash_connected",
  "arguments": {
    "sketch_path": "code/ESP32/is01",
    "max_ports": 10
  }
}
```

---

## 4. 参照リンク

- mobes2.0: https://github.com/warusakudeveroper/mobes2.0
- Arduino-MCP: https://github.com/warusakudeveroper/Arduino-MCP
- Arduino-MCP マニュアル: `docs/manual.md`
- Arduino-MCP コンソールガイド: `docs/console-guide.md`
- Arduino-MCP CLIセットアップ: `docs/cli-setup.md`
