# 設計必要項目インデックス（詳細調査報告）

作成日: 2025-12-29
ステータス: Issue登録完了（全24件）
Issue番号: #3〜#26

---

## 調査項目一覧

### is21（推論サーバ）- 7項目

| ID | 項目 | 依存 | 優先度 | Issue | 調査状況 |
|----|------|------|--------|-------|---------|
| IS21-001 | RKNNランタイム環境構築 | - | P0 | [#3](https://github.com/warusakudeveroper/aranea_ISMS/issues/3) | 未着手 |
| IS21-002 | 推論モデル選定・RKNN変換 | IS21-001 | P0 | [#4](https://github.com/warusakudeveroper/aranea_ISMS/issues/4) | 未着手 |
| IS21-003 | FastAPI推論サーバ実装 | IS21-002 | P1 | [#8](https://github.com/warusakudeveroper/aranea_ISMS/issues/8) | 未着手 |
| IS21-004 | 品質判定アルゴリズム（blur/dark） | - | P2 | [#17](https://github.com/warusakudeveroper/aranea_ISMS/issues/17) | 未着手 |
| IS21-005 | 色判定アルゴリズム（top_color） | IS21-002 | P2 | [#18](https://github.com/warusakudeveroper/aranea_ISMS/issues/18) | 未着手 |
| IS21-006 | スキーマ管理（受信・キャッシュ） | - | P1 | [#9](https://github.com/warusakudeveroper/aranea_ISMS/issues/9) | 未着手 |
| IS21-007 | systemdサービス設定 | IS21-003 | P2 | [#19](https://github.com/warusakudeveroper/aranea_ISMS/issues/19) | 未着手 |

### is22（メインサーバ）- 17項目

| ID | 項目 | 依存 | 優先度 | Issue | 調査状況 |
|----|------|------|--------|-------|---------|
| IS22-001 | Rustワークスペース構成 | - | P0 | [#5](https://github.com/warusakudeveroper/aranea_ISMS/issues/5) | 未着手 |
| IS22-002 | MariaDB DDL適用 | - | P0 | [#6](https://github.com/warusakudeveroper/aranea_ISMS/issues/6) | 未着手 |
| IS22-003 | collector: RTSP取得（ffmpeg） | IS22-001 | P0 | [#7](https://github.com/warusakudeveroper/aranea_ISMS/issues/7) | 未着手 |
| IS22-004 | collector: 差分判定アルゴリズム | IS22-003 | P1 | [#10](https://github.com/warusakudeveroper/aranea_ISMS/issues/10) | 未着手 |
| IS22-005 | collector: frames INSERT | IS22-002 | P1 | [#11](https://github.com/warusakudeveroper/aranea_ISMS/issues/11) | 未着手 |
| IS22-006 | dispatcher: ジョブキュー（claim/done） | IS22-002 | P1 | [#12](https://github.com/warusakudeveroper/aranea_ISMS/issues/12) | 未着手 |
| IS22-007 | dispatcher: is21通信クライアント | IS21-003 | P1 | [#13](https://github.com/warusakudeveroper/aranea_ISMS/issues/13) | 未着手 |
| IS22-008 | dispatcher: イベント化ロジック | IS22-006 | P1 | [#14](https://github.com/warusakudeveroper/aranea_ISMS/issues/14) | 未着手 |
| IS22-009 | dispatcher: Drive保存 | IS22-010 | P2 | [#20](https://github.com/warusakudeveroper/aranea_ISMS/issues/20) | 未着手 |
| IS22-010 | Drive: サービスアカウント認証 | - | P1 | [#15](https://github.com/warusakudeveroper/aranea_ISMS/issues/15) | 未着手 |
| IS22-011 | dispatcher: 通知Webhook | IS22-008 | P2 | [#21](https://github.com/warusakudeveroper/aranea_ISMS/issues/21) | 未着手 |
| IS22-012 | dispatcher: クールダウン管理 | - | P2 | [#22](https://github.com/warusakudeveroper/aranea_ISMS/issues/22) | 未着手 |
| IS22-013 | web: Axum APIサーバ | IS22-001 | P1 | [#16](https://github.com/warusakudeveroper/aranea_ISMS/issues/16) | 未着手 |
| IS22-014 | web: 署名付きMedia Proxy | IS22-013 | P2 | [#23](https://github.com/warusakudeveroper/aranea_ISMS/issues/23) | 未着手 |
| IS22-015 | web: Next.js/shadcn UIフロント | - | P2 | [#24](https://github.com/warusakudeveroper/aranea_ISMS/issues/24) | 未着手 |
| IS22-016 | streamer: ffmpeg HLS変換 | IS22-001 | P2 | [#25](https://github.com/warusakudeveroper/aranea_ISMS/issues/25) | 未着手 |
| IS22-017 | Apache: リバプロ設定 | IS22-013, IS22-016 | P2 | [#26](https://github.com/warusakudeveroper/aranea_ISMS/issues/26) | 未着手 |

---

## 依存関係図

```
[Phase 0: 基盤]
IS21-001 RKNNランタイム
IS22-001 Rustワークスペース
IS22-002 MariaDB DDL

[Phase 1: コア機能]
IS21-001 → IS21-002 モデル選定
IS21-002 → IS21-003 FastAPI実装
IS22-001 → IS22-003 collector RTSP
IS22-002 → IS22-005 frames INSERT
IS22-003 → IS22-004 差分判定
IS22-002 → IS22-006 ジョブキュー
IS21-003 → IS22-007 is21クライアント
IS22-006 → IS22-008 イベント化

[Phase 2: 付加機能]
IS22-010 Drive認証
IS22-010 → IS22-009 Drive保存
IS22-008 → IS22-011 通知
IS22-001 → IS22-013 web API
IS22-013 → IS22-014 Media Proxy
IS22-001 → IS22-016 streamer
IS22-013, IS22-016 → IS22-017 Apache
```

---

## 詳細調査項目

### IS21-001: RKNNランタイム環境構築

#### 調査内容
1. Orange Pi 5 Plus用RKNNドライバ確認
2. RKNN Toolkitバージョン確認
3. Python仮想環境構成
4. NPUアクセス権限設定

#### 調査方法
- 公式ドキュメント確認
- テスト用サンプル実行
- dmesg/lsmodでドライバ確認

#### 達成条件
- [ ] `rknn.init_runtime()`が成功
- [ ] サンプルモデル推論が動作
- [ ] NPU使用率がモニタリング可能

---

### IS21-002: 推論モデル選定・RKNN変換

#### 調査内容
1. YOLOv8n vs YOLOv5s性能比較
2. RKNN変換手順
3. 量子化（INT8）の精度影響
4. 入力サイズ（640 vs 960）の速度影響

#### 調査方法
- RKNN Toolkit2でモデル変換
- 複数画像でベンチマーク

#### 達成条件
- [ ] 人物検出モデルがRKNN形式で動作
- [ ] 640px入力で推論時間 < 300ms
- [ ] 検出精度mAP > 0.5

---

### IS21-003: FastAPI推論サーバ実装

#### 調査内容
1. FastAPI + uvicorn構成
2. multipart/form-data受信
3. 非同期推論処理
4. スキーマバリデーション

#### 調査方法
- サンプル実装・負荷テスト

#### 達成条件
- [ ] /healthz応答 < 10ms
- [ ] /v1/analyze応答 < 500ms（推論込み）
- [ ] 同時5リクエスト処理可能

---

### IS21-004: 品質判定アルゴリズム（blur/dark）

#### 調査内容
1. Laplacian分散の閾値検証
2. 平均輝度の閾値検証
3. glare（白飛び）判定

#### 調査方法
- 各条件の画像で閾値調整

#### 達成条件
- [ ] blur画像で camera.blur 付与率 > 90%
- [ ] 暗画像で camera.dark 付与率 > 90%
- [ ] 誤検知率 < 5%

---

### IS21-005: 色判定アルゴリズム（top_color）

#### 調査内容
1. HSV色空間での色分類
2. bbox上半分切り出し
3. 代表色抽出方法（ヒストグラム/k-means）

#### 調査方法
- 各色の衣服画像でテスト

#### 達成条件
- [ ] 赤/青/黒/白/灰の識別精度 > 80%
- [ ] 処理時間 < 10ms/人物

---

### IS21-006: スキーマ管理（受信・キャッシュ）

#### 調査内容
1. PUT /v1/schemaでのスキーマ保存
2. メモリキャッシュ戦略
3. バージョン不一致時の409応答

#### 調査方法
- APIテスト

#### 達成条件
- [ ] スキーマ更新が永続化
- [ ] 再起動後もスキーマ保持
- [ ] バージョン不一致で409

---

### IS21-007: systemdサービス設定

#### 調査内容
1. ユニットファイル構成
2. 環境変数設定
3. ログローテーション
4. 自動再起動設定

#### 調査方法
- systemctlテスト

#### 達成条件
- [ ] `systemctl start is21-infer`成功
- [ ] プロセス異常時に自動再起動
- [ ] ログが/opt/is21/logsに出力

---

### IS22-001: Rustワークスペース構成

#### 調査内容
1. Cargo Workspace設定
2. 共通クレート（cam_common）設計
3. 依存関係（sqlx, reqwest, axum等）
4. ビルド最適化

#### 調査方法
- サンプルワークスペース作成

#### 達成条件
- [ ] `cargo build --release`成功
- [ ] 各binがビルド可能
- [ ] 共通クレートが参照可能

---

### IS22-002: MariaDB DDL適用

#### 調査内容
1. 特記仕様1のDDL適用手順
2. 追加マイグレーション（inference_jobs等）
3. ユーザー/権限設定
4. 初期データ投入

#### 調査方法
- SQLクライアントでテスト

#### 達成条件
- [ ] 全テーブル作成成功
- [ ] 外部キー制約動作
- [ ] camserverユーザーでCRUD可能

---

### IS22-003: collector: RTSP取得（ffmpeg）

#### 調査内容
1. ffmpegコマンドオプション
2. TCPトランスポート設定
3. タイムアウト設定
4. VPNカメラ対応

#### 調査方法
- 実カメラでのテスト

#### 達成条件
- [ ] ローカルカメラで3秒以内取得
- [ ] VPNカメラで5秒以内取得
- [ ] 失敗時にエラー捕捉

---

### IS22-004: collector: 差分判定アルゴリズム

#### 調査内容
1. diff_ratio計算方法
2. luma_delta計算方法
3. 閾値の最適化
4. 強制推論間隔

#### 調査方法
- 動画からのフレーム抽出テスト

#### 達成条件
- [ ] 静止画面でno_event判定
- [ ] 人物出現で差分検知
- [ ] 強制推論が動作

---

### IS22-005: collector: frames INSERT

#### 調査内容
1. sqlxでのINSERT実装
2. トランザクション境界
3. エラー時のハンドリング

#### 調査方法
- 単体テスト

#### 達成条件
- [ ] frames INSERTが成功
- [ ] frame_uuidがユニーク
- [ ] エラー時もDBが壊れない

---

### IS22-006: dispatcher: ジョブキュー（claim/done）

#### 調査内容
1. claim UPDATEの排他制御
2. locked_tokenによる検証
3. 再キュー・dead処理
4. 複数dispatcher並列動作

#### 調査方法
- 並列実行テスト

#### 達成条件
- [ ] 1ジョブが1workerのみに配布
- [ ] 失敗時に再キュー
- [ ] max_attempt超過でdead

---

### IS22-007: dispatcher: is21通信クライアント

#### 調査内容
1. reqwest multipart実装
2. タイムアウト設定
3. 409/429エラーハンドリング
4. リトライ戦略

#### 調査方法
- モックサーバでテスト

#### 達成条件
- [ ] 正常推論でレスポンスパース
- [ ] 409でスキーマ再プッシュ
- [ ] 429でバックオフ

---

### IS22-008: dispatcher: イベント化ロジック

#### 調査内容
1. open event取得
2. can_merge判定
3. merge/close処理
4. best_frame選定

#### 調査方法
- シナリオテスト

#### 達成条件
- [ ] 連続検知でmerge
- [ ] 時間ギャップで別イベント
- [ ] 120秒無検知でclose

---

### IS22-009: dispatcher: Drive保存

#### 調査内容
1. google-drive3クレート使用
2. サブフォルダ作成
3. アップロード実装
4. エラーリトライ

#### 調査方法
- 実Driveでテスト

#### 達成条件
- [ ] 画像がDriveに保存
- [ ] フォルダ構成が正しい
- [ ] file_idがDBに保存

---

### IS22-010: Drive: サービスアカウント認証

#### 調査内容
1. GCPサービスアカウント作成
2. Drive API有効化
3. フォルダ共有設定
4. 認証トークン取得

#### 調査方法
- GCPコンソール・APIテスト

#### 達成条件
- [ ] サービスアカウントJSONでAPI呼び出し成功
- [ ] 指定フォルダへの書き込み権限確認

---

### IS22-011: dispatcher: 通知Webhook

#### 調査内容
1. HTTPクライアント実装
2. 通知テンプレート
3. 署名付きURL生成
4. 送信失敗時のリトライ

#### 調査方法
- Webhook受信モックでテスト

#### 達成条件
- [ ] 通知が正しいフォーマットで送信
- [ ] 画像URLが有効
- [ ] 失敗時にログ記録

---

### IS22-012: dispatcher: クールダウン管理

#### 調査内容
1. メモリ管理実装
2. キー設計（camera_id, event_type）
3. DB永続化（オプション）

#### 調査方法
- 単体テスト

#### 達成条件
- [ ] クールダウン中は通知抑制
- [ ] 時間経過で解除
- [ ] 再起動で状態リセット許容

---

### IS22-013: web: Axum APIサーバ

#### 調査内容
1. Axumルーティング設計
2. 状態管理（DB pool等）
3. エラーハンドリング
4. CORS設定

#### 調査方法
- curlでAPIテスト

#### 達成条件
- [ ] /api/cameras応答
- [ ] /api/events検索動作
- [ ] JSONレスポンス形式正しい

---

### IS22-014: web: 署名付きMedia Proxy

#### 調査内容
1. HMAC-SHA256署名実装
2. 署名検証
3. 有効期限チェック
4. Driveからのストリーム配信

#### 調査方法
- 署名付きURLテスト

#### 達成条件
- [ ] 正しい署名で画像取得
- [ ] 不正署名で403
- [ ] 期限切れで403

---

### IS22-015: web: Next.js/shadcn UIフロント

#### 調査内容
1. Next.js 14 App Router構成
2. shadcn/ui導入
3. カメラグリッドコンポーネント
4. SSE/WebSocket接続

#### 調査方法
- 開発サーバでUI確認

#### 達成条件
- [ ] カメラ一覧表示
- [ ] リアルタイム更新
- [ ] モーダル動作

---

### IS22-016: streamer: ffmpeg HLS変換

#### 調査内容
1. ffmpegコマンド設計
2. セッション管理
3. TTL自動停止
4. ディレクトリクリーンアップ

#### 調査方法
- HLS再生テスト

#### 達成条件
- [ ] HLSファイル生成
- [ ] ブラウザで再生可能
- [ ] セッション終了で削除

---

### IS22-017: Apache: リバプロ設定

#### 調査内容
1. VirtualHost設定
2. ProxyPass設定
3. WebSocket対応
4. SSL設定（オプション）

#### 調査方法
- Apache設定テスト

#### 達成条件
- [ ] /api → 8080転送
- [ ] /hls → 8082転送
- [ ] WebSocket接続維持

---

## Issue登録サマリー（登録完了: 2025-12-29）

### 優先度P0（必須・最優先）- 5件
1. IS21-001: RKNNランタイム環境構築 → [#3](https://github.com/warusakudeveroper/aranea_ISMS/issues/3)
2. IS21-002: 推論モデル選定・RKNN変換 → [#4](https://github.com/warusakudeveroper/aranea_ISMS/issues/4)
3. IS22-001: Rustワークスペース構成 → [#5](https://github.com/warusakudeveroper/aranea_ISMS/issues/5)
4. IS22-002: MariaDB DDL適用 → [#6](https://github.com/warusakudeveroper/aranea_ISMS/issues/6)
5. IS22-003: collector RTSP取得 → [#7](https://github.com/warusakudeveroper/aranea_ISMS/issues/7)

### 優先度P1（必須）- 9件
6. IS21-003: FastAPI推論サーバ → [#8](https://github.com/warusakudeveroper/aranea_ISMS/issues/8)
7. IS21-006: スキーマ管理 → [#9](https://github.com/warusakudeveroper/aranea_ISMS/issues/9)
8. IS22-004: 差分判定 → [#10](https://github.com/warusakudeveroper/aranea_ISMS/issues/10)
9. IS22-005: frames INSERT → [#11](https://github.com/warusakudeveroper/aranea_ISMS/issues/11)
10. IS22-006: ジョブキュー → [#12](https://github.com/warusakudeveroper/aranea_ISMS/issues/12)
11. IS22-007: is21通信クライアント → [#13](https://github.com/warusakudeveroper/aranea_ISMS/issues/13)
12. IS22-008: イベント化ロジック → [#14](https://github.com/warusakudeveroper/aranea_ISMS/issues/14)
13. IS22-010: Drive認証 → [#15](https://github.com/warusakudeveroper/aranea_ISMS/issues/15)
14. IS22-013: Axum APIサーバ → [#16](https://github.com/warusakudeveroper/aranea_ISMS/issues/16)

### 優先度P2（付加機能）- 10件
15. IS21-004: 品質判定 → [#17](https://github.com/warusakudeveroper/aranea_ISMS/issues/17)
16. IS21-005: 色判定 → [#18](https://github.com/warusakudeveroper/aranea_ISMS/issues/18)
17. IS21-007: systemd設定 → [#19](https://github.com/warusakudeveroper/aranea_ISMS/issues/19)
18. IS22-009: Drive保存 → [#20](https://github.com/warusakudeveroper/aranea_ISMS/issues/20)
19. IS22-011: 通知Webhook → [#21](https://github.com/warusakudeveroper/aranea_ISMS/issues/21)
20. IS22-012: クールダウン → [#22](https://github.com/warusakudeveroper/aranea_ISMS/issues/22)
21. IS22-014: Media Proxy → [#23](https://github.com/warusakudeveroper/aranea_ISMS/issues/23)
22. IS22-015: フロントエンド → [#24](https://github.com/warusakudeveroper/aranea_ISMS/issues/24)
23. IS22-016: HLSストリーミング → [#25](https://github.com/warusakudeveroper/aranea_ISMS/issues/25)
24. IS22-017: Apacheリバプロ → [#26](https://github.com/warusakudeveroper/aranea_ISMS/issues/26)
