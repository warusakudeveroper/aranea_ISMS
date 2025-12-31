# IS22 Camserver

mobes AIcam control Tower (mAcT)

## 概要

| 項目 | 値 |
|-----|-----|
| デバイス型番 | ar-is22 |
| ハードウェア | Ubuntu Server (x86_64/ARM64) |
| ファームウェア | v0.1.0 |
| 言語 | Rust |

## 機能

- **カメラ管理**: 最大30台のIPカメラ（Tapo/VIGI/Nest等）
- **AI連携**: IS21 camimageEdge AIとの統合
- **リアルタイム配信**: WebSocket/SSEによるイベント通知
- **サジェストエンジン**: 検知イベントに基づく「今見るべきカメラ」の決定
- **負荷制御**: AdmissionControllerによるストリーム数管理
- **自動発見**: IpcamScanによるカメラ自動検出

## アーキテクチャ

```
[RTSPカメラ×30] ←────────── [IpcamScan] ← カメラ自動発見
       ↓
[SnapshotService] ← 画像取得
       ↓
[PollingOrchestrator] ← 1台ずつ順次
       ↓
[AIClient(Adapter)] ─────────→ [IS21] ← 推論リクエスト
       ↓
[EventLogService] ← イベント記録（リングバッファ）
       ↓
┌──────┴──────┐
↓              ↓
[SuggestEngine]  [MariaDB] ← ConfigStore(SSoT)
↓              ↓
[RealtimeHub] ← WebSocket/SSE配信
↓
[WebAPI] ← REST API
       ↓
[StreamGatewayAdapter] ─→ [go2rtc] ← ストリーム配信
       ↓
[AdmissionController] ← 負荷制御・Lease管理
       ↓
   [ブラウザ] ← 3ペインUI (React+shadcn)
```

## ディレクトリ構成

```
is22/
├── src/
│   ├── main.rs                    # エントリーポイント
│   ├── lib.rs                     # ライブラリルート
│   ├── config_store/              # ConfigStore (SSoT)
│   ├── admission_controller/      # 負荷制御
│   ├── ai_client/                 # IS21連携
│   ├── event_log_service/         # イベントログ
│   ├── suggest_engine/            # サジェスト
│   ├── stream_gateway/            # go2rtc連携
│   ├── realtime_hub/              # WebSocket/SSE
│   ├── web_api/                   # REST API
│   ├── ipcam_scan/               # カメラ発見
│   ├── snapshot_service/          # スナップショット
│   ├── polling_orchestrator/      # ポーリング
│   ├── models/                    # 共通モデル
│   ├── error.rs                   # エラー定義
│   └── state.rs                   # アプリ状態
├── migrations/
│   └── 001_initial_schema.sql     # DBスキーマ
├── Cargo.toml
└── README.md
```

## 前提条件

- Ubuntu 22.04+ (x86_64 or ARM64)
- Rust 1.75+
- MariaDB 10.6+
- go2rtc (ストリーム配信)
- IS21 camimageEdge AI サーバー

## インストール

### 1. データベース作成

```bash
mysql -u root -p -e "CREATE DATABASE camserver CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"
mysql -u root -p camserver < migrations/001_initial_schema.sql
```

### 2. 環境変数設定

```bash
export DATABASE_URL="mysql://root:password@localhost/camserver"
export IS21_URL="http://192.168.3.240:9000"
export GO2RTC_URL="http://localhost:1984"
export PORT=8080
export HOST=0.0.0.0
```

### 3. ビルド・実行

```bash
cargo build --release
./target/release/camserver
```

## API

### ヘルスチェック

```bash
curl http://localhost:8080/healthz
```

### カメラ一覧

```bash
curl http://localhost:8080/api/cameras
```

### サジェスト状態

```bash
curl http://localhost:8080/api/suggest
```

### システム状態

```bash
curl http://localhost:8080/api/system/status
```

## 設計原則

### SSoT (Single Source of Truth)

- ConfigStoreがカメラ設定の唯一の情報源
- IS21は設定を「受け取る」のみ

### SOLID

- **S (単一責任)**: 各モジュールは1つの責務のみ
- **O (開放閉鎖)**: スキーマで拡張、コード変更不要
- **D (依存逆転)**: IS22→IS21への依存のみ

## 関連ドキュメント

- `designs/headdesign/00_INDEX.md` - 設計ドキュメントインデックス
- `designs/headdesign/is22_Camserver仕様案.md` - 基本設計書
- `designs/headdesign/is22_Camserver実装指示.md` - 実装指示

## バージョン履歴

| バージョン | 日付 | 主要変更 |
|-----------|------|----------|
| 0.1.0 | 2025-12-31 | 初版（11コンポーネントアーキテクチャ） |

---

作成: Claude Code
