# 実装タスク算定

文書番号: IMPL_01
作成日: 2025-12-29
ステータス: Draft

---

## 1. タスク見積もり方針

### 1.1 見積もり単位
- **S**: 小規模（設定・単純実装）
- **M**: 中規模（モジュール実装）
- **L**: 大規模（複合機能・調査込み）

### 1.2 依存関係
- Phase間の依存を厳守
- Phase内は並列実行可能（依存なしの場合）

---

## 2. Phase 0: 基盤構築タスク

### IMPL-001: is21 環境構築 [L]
**Issue**: #3 (IS21-001)
**内容**:
- Orange Pi 5 Plus OS設定
- Python 3.10 + venv構築
- RKNNドライバインストール
- NPU権限設定
- ディレクトリ構成作成

**成果物**:
```
/opt/is21/
├── .venv/
├── models/
├── logs/
├── config/
└── src/
```

**確認項目**:
- [ ] rknn.init_runtime() 成功
- [ ] サンプル推論動作

---

### IMPL-002: 推論モデル変換 [M]
**Issue**: #4 (IS21-002)
**依存**: IMPL-001
**内容**:
- YOLOv8n ONNX取得
- RKNN Toolkit2でINT8量子化変換
- キャリブレーションデータ準備
- 推論速度計測

**成果物**:
- `/opt/is21/models/yolov8n_640.rknn`
- 変換ログ・ベンチマーク結果

**確認項目**:
- [ ] 推論時間 < 300ms
- [ ] 人物検出精度確認

---

### IMPL-003: Rustワークスペース構築 [M]
**Issue**: #5 (IS22-001)
**内容**:
- Cargo.toml（ワークスペースルート）作成
- cam_common クレート実装
- 共通依存関係設定
- ビルド確認

**成果物**:
```
/opt/camserver/
├── Cargo.toml
├── cam_common/
│   └── src/
│       ├── lib.rs
│       ├── db.rs
│       ├── models.rs
│       ├── config.rs
│       └── error.rs
└── target/
```

**確認項目**:
- [ ] cargo build --release 成功
- [ ] cam_common参照可能

---

### IMPL-004: MariaDB初期構築 [M]
**Issue**: #6 (IS22-002)
**内容**:
- MariaDBインストール
- camserverデータベース作成
- 特記仕様1 DDL適用
- ユーザー権限設定
- 初期データ投入

**成果物**:
- `/opt/camserver/migrations/001_initial.sql`
- 初期化スクリプト

**確認項目**:
- [ ] 全15テーブル作成
- [ ] FK制約動作
- [ ] CRUD動作確認

---

### IMPL-005: collector RTSP取得実装 [M]
**Issue**: #7 (IS22-003)
**依存**: IMPL-003
**内容**:
- cam_collector クレート作成
- ffmpegラッパー実装
- タイムアウト処理
- エラーハンドリング

**成果物**:
- `cam_collector/src/rtsp.rs`
- 単体テスト

**確認項目**:
- [ ] ローカルカメラ取得 < 3秒
- [ ] エラー時例外捕捉

---

## 3. Phase 1: コア機能タスク

### IMPL-006: FastAPI推論サーバ [M]
**Issue**: #8 (IS21-003)
**依存**: IMPL-001, IMPL-002
**内容**:
- FastAPI + uvicorn構成
- /healthz エンドポイント
- /v1/analyze エンドポイント
- 推論エンジン統合

**成果物**:
- `/opt/is21/src/main.py`
- `/opt/is21/src/inference.py`
- requirements.txt

**確認項目**:
- [ ] /healthz < 10ms
- [ ] /v1/analyze < 500ms

---

### IMPL-007: スキーマ管理 [S]
**Issue**: #9 (IS21-006)
**依存**: IMPL-006
**内容**:
- PUT /v1/schema実装
- ファイル永続化
- メモリキャッシュ
- バージョン検証

**成果物**:
- `/opt/is21/src/schema.py`

**確認項目**:
- [ ] スキーマ更新・永続化
- [ ] 409応答

---

### IMPL-008: 差分判定 [M]
**Issue**: #10 (IS22-004)
**依存**: IMPL-005
**内容**:
- diff_ratio計算実装
- luma_delta計算実装
- 閾値設定
- 強制推論フラグ

**成果物**:
- `cam_collector/src/diff.rs`
- 単体テスト

**確認項目**:
- [ ] 静止画no_event
- [ ] 動き検知

---

### IMPL-009: frames INSERT [S]
**Issue**: #11 (IS22-005)
**依存**: IMPL-003, IMPL-004
**内容**:
- sqlx INSERT実装
- UUID生成
- トランザクション処理

**成果物**:
- `cam_common/src/db.rs` (frames関連)

**確認項目**:
- [ ] INSERT成功
- [ ] UUID一意性

---

### IMPL-010: ジョブキュー [M]
**Issue**: #12 (IS22-006)
**依存**: IMPL-004
**内容**:
- inference_jobs操作
- claim排他制御
- requeue/dead処理
- ワーカーID管理

**成果物**:
- `cam_dispatcher/src/job.rs`
- 単体テスト

**確認項目**:
- [ ] 排他取得
- [ ] 再キュー動作

---

### IMPL-011: is21通信クライアント [M]
**Issue**: #13 (IS22-007)
**依存**: IMPL-006
**内容**:
- reqwest multipart実装
- タイムアウト設定
- 409/429ハンドリング
- リトライ実装

**成果物**:
- `cam_dispatcher/src/inference.rs`

**確認項目**:
- [ ] 正常推論
- [ ] エラーハンドリング

---

### IMPL-012: イベント化ロジック [L]
**Issue**: #14 (IS22-008)
**依存**: IMPL-010
**内容**:
- オープンイベント検索
- can_merge判定
- merge/close処理
- best_frame選定
- severity_max更新

**成果物**:
- `cam_dispatcher/src/event.rs`
- 統合テスト

**確認項目**:
- [ ] マージ動作
- [ ] 分離動作
- [ ] 自動クローズ

---

### IMPL-013: Drive認証 [M]
**Issue**: #15 (IS22-010)
**内容**:
- GCPサービスアカウント作成
- Drive API有効化
- フォルダ構成作成
- 認証テスト

**成果物**:
- `/opt/camserver/secrets/drive-sa.json`
- 設定ドキュメント

**確認項目**:
- [ ] API呼び出し成功
- [ ] フォルダ書き込み権限

---

### IMPL-014: Axum APIサーバ [M]
**Issue**: #16 (IS22-013)
**依存**: IMPL-003, IMPL-004
**内容**:
- cam_web クレート作成
- ルーティング設計
- /api/cameras実装
- /api/events実装
- CORS設定

**成果物**:
- `cam_web/src/main.rs`
- `cam_web/src/routes/*.rs`

**確認項目**:
- [ ] API応答
- [ ] JSON形式正常

---

## 4. Phase 2: 付加機能タスク

### IMPL-015: 品質判定 [S]
**Issue**: #17 (IS21-004)
**内容**:
- blur判定（Laplacian）
- dark判定（平均輝度）
- glare判定

**成果物**:
- `/opt/is21/src/quality.py`

---

### IMPL-016: 色判定 [S]
**Issue**: #18 (IS21-005)
**依存**: IMPL-002
**内容**:
- bbox上半分切り出し
- HSV色分類
- top_color抽出

**成果物**:
- `/opt/is21/src/color.py`

---

### IMPL-017: systemd設定 [S]
**Issue**: #19 (IS21-007)
**依存**: IMPL-006
**内容**:
- is21-infer.service作成
- 環境変数設定
- 自動再起動設定
- ログローテーション

**成果物**:
- `/etc/systemd/system/is21-infer.service`

---

### IMPL-018: Drive保存 [M]
**Issue**: #20 (IS22-009)
**依存**: IMPL-013
**内容**:
- google-drive3クレート統合
- フォルダ作成ロジック
- アップロード実装
- file_id DB保存

**成果物**:
- `cam_dispatcher/src/drive.rs`

---

### IMPL-019: 通知Webhook [M]
**Issue**: #21 (IS22-011)
**依存**: IMPL-012
**内容**:
- 通知テンプレート
- HTTPクライアント
- 署名付きURL生成
- リトライ実装

**成果物**:
- `cam_dispatcher/src/notify.rs`

---

### IMPL-020: クールダウン [S]
**Issue**: #22 (IS22-012)
**内容**:
- メモリ管理
- キー設計
- 時間経過チェック

**成果物**:
- `cam_dispatcher/src/cooldown.rs`

---

### IMPL-021: Media Proxy [M]
**Issue**: #23 (IS22-014)
**依存**: IMPL-014
**内容**:
- HMAC-SHA256署名
- 署名検証
- Drive画像取得
- ストリーム配信

**成果物**:
- `cam_web/src/routes/media.rs`

---

### IMPL-022: フロントエンド [L]
**Issue**: #24 (IS22-015)
**依存**: IMPL-014
**内容**:
- Next.js 14 App Router
- shadcn/ui導入
- カメラグリッド
- イベント一覧
- SSE接続

**成果物**:
- `/opt/camserver/frontend/`

---

### IMPL-023: HLSストリーミング [M]
**Issue**: #25 (IS22-016)
**依存**: IMPL-003
**内容**:
- cam_streamer クレート
- ffmpeg HLS変換
- セッション管理
- TTL自動停止

**成果物**:
- `cam_streamer/src/*.rs`

---

### IMPL-024: Apacheリバプロ [S]
**Issue**: #26 (IS22-017)
**依存**: IMPL-014, IMPL-023
**内容**:
- VirtualHost設定
- ProxyPass設定
- WebSocket対応

**成果物**:
- `/etc/apache2/sites-available/camserver.conf`

---

## 5. 実装順序

### Phase 0 (並列可能)
```
IMPL-001 (is21環境)     IMPL-003 (Rust)     IMPL-004 (MariaDB)
     ↓                      ↓                    ↓
IMPL-002 (モデル)       IMPL-005 (RTSP)
```

### Phase 1 (依存順)
```
IMPL-006 (FastAPI) ← IMPL-001, IMPL-002
     ↓
IMPL-007 (スキーマ)
IMPL-011 (is21クライアント)

IMPL-008 (差分) ← IMPL-005
IMPL-009 (frames) ← IMPL-003, IMPL-004
IMPL-010 (ジョブ) ← IMPL-004
     ↓
IMPL-012 (イベント) ← IMPL-010

IMPL-013 (Drive認証) ← なし
IMPL-014 (Axum) ← IMPL-003, IMPL-004
```

### Phase 2 (依存順)
```
IMPL-015,016 (品質・色) ← IMPL-002
IMPL-017 (systemd) ← IMPL-006
IMPL-018 (Drive保存) ← IMPL-013
IMPL-019 (通知) ← IMPL-012
IMPL-020 (クールダウン) ← なし
IMPL-021 (MediaProxy) ← IMPL-014
IMPL-022 (フロント) ← IMPL-014
IMPL-023 (HLS) ← IMPL-003
IMPL-024 (Apache) ← IMPL-014, IMPL-023
```

---

## 6. タスクサマリー

| Phase | タスク数 | S | M | L |
|-------|---------|---|---|---|
| 0 | 5 | 0 | 4 | 1 |
| 1 | 9 | 2 | 6 | 1 |
| 2 | 10 | 4 | 5 | 1 |
| **合計** | **24** | **6** | **15** | **3** |

---

## 7. マイルストーン

### MS-1: 基盤完成
- IMPL-001〜005完了
- is21 NPU動作確認
- is22 Rust+DB動作確認

### MS-2: コアパイプライン完成
- IMPL-006〜014完了
- フレーム収集→推論→イベント化の一連フロー動作

### MS-3: 付加機能・UI完成
- IMPL-015〜024完了
- 全機能統合テストパス

### MS-4: 本番デプロイ
- ステージング環境テスト完了
- 本番環境デプロイ
- スモークテスト完了
