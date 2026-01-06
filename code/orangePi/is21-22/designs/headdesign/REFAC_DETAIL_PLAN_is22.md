# is22 リファクタリング詳細設計・タスクリスト

**関連ドキュメント**: `REFACTORING_PLAN_is22.md`（分割計画・目的）
**対象**: `code/orangePi/is22/`
**原則**: SSoT/SOLID(SRP)/MECE/アンアンビギュアスを遵守。400行以下分割を基本とし、静的データのみ例外。

---

## 1. スコープと前提

- 対象ファイル: `web_api/routes.rs`, `ipcam_scan/scanner.rs`, `ipcam_scan/mod.rs`, `frontend/src/components/ScanModal.tsx`
- 仕様変更なし。関数名・エンドポイント・APIレスポンス・型シグネチャは既存踏襲を原則とする。
- 静的データ(`OUI_CAMERA_VENDORS`)のみ400行超を許容。その他コードは400行以下で分割。
- 既存テスト（`scanner.rs`末尾のtests等）は維持し、新配置でも動作するよう移設。
- ビルド・テストはローカル環境 (Rust 1.92.0, Node 18.19.1) 前提。

## 2. 分割設計（再掲＋補足）

### backend/web_api routes
- `routes/mod.rs`: ルーター生成 `create_router` + 各モジュール再輸出
- `routes/health.rs`: health_check, device_status
- `routes/cameras.rs`: camera CRUD/sort
- `routes/modals.rs`: modal_lease_*, modal_budget, active_modals
- `routes/suggest.rs`: get_suggest, suggest_context_*, get_presets
- `routes/detection_logs.rs`: detection_logs_*, performance_logs
- `routes/ipcam_scan.rs`: scan_*, device_*, approve_device
- `routes/subnets.rs`: subnet_*, credential_*
- `routes/streams.rs`: go2rtc_*, stream_gateway_*

### backend/ipcam_scan scanner
- `scanner/mod.rs`: NetworkScanner構造体定義・公開・共通型/const re-export
- `scanner/port_weights.rs`: `PORT_WEIGHTS`
- `scanner/oui_data.rs`: `OUI_CAMERA_VENDORS`（静的データのみ）
- `scanner/probes.rs`: probe_rtsp/onvif/http関連ロジック
- `scanner/network.rs`: scanロジックとスコア計算、外部公開関数

### backend/ipcam_scan stages
- `ipcam_scan/mod.rs`: IpcamScan struct, stage共通定義、re-export
- `ipcam_scan/job.rs`: ScanJob管理
- `ipcam_scan/stage0_discovery.rs`: ARP/mDNS discovery
- `ipcam_scan/stage1_portscan.rs`: Port scan
- `ipcam_scan/stage2_rtsp.rs`: RTSP probe
- `ipcam_scan/stage3_onvif.rs`: ONVIF probe
- `ipcam_scan/stage4_http.rs`: HTTP probe
- `ipcam_scan/stage5_aggregate.rs`: 結果集約
- `ipcam_scan/stage6_finalize.rs`: 最終化
- `ipcam_scan/db.rs`: DB操作

### frontend ScanModal
- `frontend/src/utils/scanCache.ts`
- `frontend/src/utils/deviceCategorization.ts`
- `frontend/src/components/ui/RadarSpinner.tsx`
- `frontend/src/components/scan/DeviceStatusBadge.tsx`
- `frontend/src/components/scan/PhaseIndicator.tsx`
- `frontend/src/components/scan/ScanLogViewer.tsx`
- `frontend/src/components/scan/DeviceCard.tsx`
- `frontend/src/components/scan/CategorySection.tsx`
- `frontend/src/components/scan/SubnetCredentialEditor.tsx`
- `frontend/src/components/ScanModal.tsx` (メイン本体)

## 3. タスクリスト（依存順・MECE）

- [ ] Phase1 依存なし抽出
  - [ ] `scanner/port_weights.rs` へ定数抽出
  - [ ] `scanner/oui_data.rs` へOUIデータ抽出
  - [ ] `ipcam_scan/job.rs` 作成・共通型移設
  - [ ] `frontend/src/utils/scanCache.ts` 抽出
  - [ ] `frontend/src/utils/deviceCategorization.ts` 抽出
- [ ] Phase2 基盤分割
  - [ ] `scanner/mod.rs` 作成（再export, struct宣言）
  - [ ] `scanner/probes.rs` 抽出
  - [ ] `scanner/network.rs` 抽出
  - [ ] `ipcam_scan/db.rs` 抽出
- [ ] Phase3 Stage分割
  - [ ] stage0_discovery → stage6_finalize 各ファイル作成
- [ ] Phase4 Routes分割
  - [ ] health/cameras/modals/suggest/detection_logs/ipcam_scan/subnets/streams/mod の各ファイル作成
- [ ] Phase5 Frontend分割
  - [ ] UI/scan各コンポーネント分割 + メイン `ScanModal.tsx` 差し替え
- [ ] 仕上げ
  - [ ] 既存テスト/型エラー解消、`cargo clippy` の警告確認
  - [ ] コード整形（rustfmt/prettier）※既存フォーマットルールに従う

## 4. テスト計画（アンアンビギュアス）

- ビルド
  - Phase1完了後: `cargo check`
  - Phase2完了後: `cargo build --release`
  - Phase3完了後: `cargo build --release`
  - Phase4完了後: `cargo build --release`
  - Phase5完了後: `npm install` (必要時) / `npm run build`
- 機能/API
  - ルーター分割後: 主要エンドポイント `curl/httpie` によるステータス確認（health, cameras CRUD, scan開始, suggest, logs, subnets, streams）
  - スキャン: ScanModal上でスキャン開始～完了までのUI確認
  - WebSocket: フロントエンド接続確認（必要に応じスタブ/ローカル実行）
- E2E（手動/準E2E）
  - カメラ一覧表示（ブラウザ）
  - スキャン実行完走（ブラウザ）
  - ヘッダー巡回状態表示（ブラウザ）

## 5. 受け入れ基準

- 分割後、コードファイルは400行以下（静的データファイルを除く）
- ビルド/テストが全て成功
- ルーティング/API挙動が分割前と同等（レスポンススキーマ/ステータス/副作用一致）
- UIは分割前と同等の表示・操作性（ScanModal含む）
- MECE: 全分割対象を網羅し重複なし、アンアンビギュアス: 具体ファイル名・手順・テストを明記

## 6. 今後の進め方

1. Phase1の抽出を実施し `cargo check` で退行確認
2. Phase2以降、順次ビルド確認を挟みつつ進行
3. 分割中は関数名・公開APIを変更しない（必要な内部可視性のみ調整）
4. 各フェーズ完了時に大原則を再確認し、テスト結果を記録

