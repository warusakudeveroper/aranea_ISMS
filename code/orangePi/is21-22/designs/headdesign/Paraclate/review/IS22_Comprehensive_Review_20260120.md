# IS22 総合レビュー依頼

**日付**: 2026-01-20
**リポジトリ**: https://github.com/warusakudeveroper/aranea_ISMS
**ブランチ**: `main`
**最新コミット**: `9fc738cb45ed93e0e532f5b4f4fb72d20c643777`

---

## 1. レビュー対象概要

IS22（RTSPカメラ総合管理サーバー）の全体アーキテクチャおよびテストスイートの完全性レビューを依頼します。

### 運用環境

| 項目 | 値 |
|------|-----|
| サーバーIP | 192.168.125.246 |
| APIポート | 3000 |
| ユーザー | mijeosadmin |
| OS | Ubuntu (aarch64) |
| データベース | MySQL (camserver) |

---

## 2. リポジトリ構造・ファイルリンク

### 2.1 バックエンド（Rust）

**ベースパス**: [`code/orangePi/is22/src/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src)

#### コアモジュール

| モジュール | パス | 説明 |
|-----------|------|------|
| main | [`src/main.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/main.rs) | エントリーポイント |
| lib | [`src/lib.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/lib.rs) | モジュール定義 |
| state | [`src/state.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/state.rs) | アプリケーション状態 |
| error | [`src/error.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/error.rs) | エラー型定義 |

#### 機能モジュール

| モジュール | パス | 説明 |
|-----------|------|------|
| **ipcam_scan** | [`src/ipcam_scan/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/ipcam_scan) | カメラスキャン・登録 |
| **camera_registry** | [`src/camera_registry/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/camera_registry) | カメラレジストリ管理 |
| **config_store** | [`src/config_store/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/config_store) | 設定ストア |
| **polling_orchestrator** | [`src/polling_orchestrator/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/polling_orchestrator) | ポーリング制御 |
| **detection_log_service** | [`src/detection_log_service/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/detection_log_service) | 検知ログ管理 |
| **summary_service** | [`src/summary_service/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/summary_service) | サマリー生成・送信 |
| **paraclate_client** | [`src/paraclate_client/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/paraclate_client) | Paraclate連携 |
| **aranea_register** | [`src/aranea_register/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/aranea_register) | Aranea登録・状態同期 |
| **ptz_controller** | [`src/ptz_controller/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/ptz_controller) | PTZ制御 |
| **access_absorber** | [`src/access_absorber/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/access_absorber) | ストリームアクセス制限 |
| **lost_cam_tracker** | [`src/lost_cam_tracker/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/lost_cam_tracker) | カメラ消失追跡 |
| **ai_client** | [`src/ai_client/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/ai_client) | AI推論クライアント |
| **suggest_engine** | [`src/suggest_engine/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/suggest_engine) | 提案エンジン |
| **realtime_hub** | [`src/realtime_hub/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/realtime_hub) | WebSocketハブ |
| **admission_controller** | [`src/admission_controller/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/src/admission_controller) | リース管理 |

#### Web API

| モジュール | パス | 説明 |
|-----------|------|------|
| routes | [`src/web_api/routes.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/web_api/routes.rs) | メインルート |
| chat_routes | [`src/web_api/chat_routes.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/web_api/chat_routes.rs) | チャットAPI |
| summary_routes | [`src/web_api/summary_routes.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/web_api/summary_routes.rs) | サマリーAPI |
| ptz_routes | [`src/web_api/ptz_routes.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/web_api/ptz_routes.rs) | PTZ API |
| access_absorber_routes | [`src/web_api/access_absorber_routes.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/web_api/access_absorber_routes.rs) | アクセス制限API |

### 2.2 フロントエンド（React/TypeScript）

**ベースパス**: [`code/orangePi/is22/frontend/src/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/frontend/src)

#### 主要コンポーネント

| コンポーネント | パス | 説明 |
|--------------|------|------|
| App | [`src/App.tsx`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/frontend/src/App.tsx) | メインアプリ |
| CameraGrid | [`src/components/CameraGrid.tsx`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/frontend/src/components/CameraGrid.tsx) | カメラグリッド |
| CameraTile | [`src/components/CameraTile.tsx`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/frontend/src/components/CameraTile.tsx) | カメラタイル |
| LiveViewModal | [`src/components/LiveViewModal.tsx`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/frontend/src/components/LiveViewModal.tsx) | ライブビュー |
| ScanModal | [`src/components/ScanModal.tsx`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/frontend/src/components/ScanModal.tsx) | スキャン・登録 |
| SuggestPane | [`src/components/SuggestPane.tsx`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/frontend/src/components/SuggestPane.tsx) | 提案パネル |
| EventLogPane | [`src/components/EventLogPane.tsx`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/frontend/src/components/EventLogPane.tsx) | イベントログ |
| PtzOverlay | [`src/components/ptz/PtzOverlay.tsx`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/frontend/src/components/ptz/PtzOverlay.tsx) | PTZ操作UI |

#### 型定義・ユーティリティ

| ファイル | パス | 説明 |
|---------|------|------|
| api.ts | [`src/types/api.ts`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/frontend/src/types/api.ts) | API型定義 |
| paraclate.ts | [`src/types/paraclate.ts`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/frontend/src/types/paraclate.ts) | Paraclate型定義 |
| deviceCategorization | [`src/utils/deviceCategorization.ts`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/frontend/src/utils/deviceCategorization.ts) | デバイス分類 |

### 2.3 マイグレーション

**ベースパス**: [`code/orangePi/is22/migrations/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/migrations)

| マイグレーション | パス | 説明 |
|----------------|------|------|
| 001 | [`001_initial_schema.sql`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/migrations/001_initial_schema.sql) | 初期スキーマ |
| 020 | [`020_aranea_registration.sql`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/migrations/020_aranea_registration.sql) | Aranea登録 |
| 021 | [`021_camera_registry.sql`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/migrations/021_camera_registry.sql) | カメラレジストリ |
| 022 | [`022_summary_service.sql`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/migrations/022_summary_service.sql) | サマリーサービス |
| 023 | [`023_paraclate_client.sql`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/migrations/023_paraclate_client.sql) | Paraclateクライアント |
| 029 | [`029_lost_cam_tracker.sql`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/migrations/029_lost_cam_tracker.sql) | カメラ消失追跡 |
| 031 | [`031_access_absorber.sql`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/migrations/031_access_absorber.sql) | アクセス制限 |
| 032 | [`032_ptz_disabled.sql`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/migrations/032_ptz_disabled.sql) | PTZ無効化 |

### 2.4 テストスイート

**ベースパス**: [`code/orangePi/is22/tests/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is22/tests)

| ファイル | パス | 説明 |
|---------|------|------|
| README | [`tests/README.md`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/tests/README.md) | テスト実行ガイド |
| run_all_tests | [`tests/run_all_tests.sh`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/tests/run_all_tests.sh) | テストランナー |
| test_camera_registration | [`tests/test_camera_registration.sh`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/tests/test_camera_registration.sh) | カメラ登録テスト |

---

## 3. 機能別アーキテクチャ

### 3.1 カメラ登録フロー

```
[ipcam_scan] スキャン
      ↓
[ipcamscan_devices] ステージングDB
      ↓
[approve_device / force_register_camera]
      ↓
[cameras] 本番DB
```

**関連ファイル**:
- [`src/ipcam_scan/mod.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/ipcam_scan/mod.rs) - スキャン・登録ロジック
- [`src/ipcam_scan/types.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/ipcam_scan/types.rs) - 型定義

**登録時に自動設定されるフィールド**:
| フィールド | 設定元 | 自動検出 |
|-----------|--------|---------|
| family | ipcamscan_devices | ✓ |
| access_family | manufacturer/model | ✓ |
| mac_address | ipcamscan_devices | ✓ |
| manufacturer | ipcamscan_devices | ✓ |
| model | ipcamscan_devices | ✓ |
| lacis_id | MACアドレス | ✓ (生成) |
| onvif_endpoint | family | ✓ (生成) |
| ptz_supported | ONVIFプローブ | ✓ |

### 3.2 PTZ制御

```
[Frontend] PtzOverlay
      ↓ WebSocket
[web_api] ptz_routes
      ↓
[ptz_controller] service
      ↓
[tapo_ptz] ONVIF/ベンダー固有API
```

**関連ファイル**:
- [`src/ptz_controller/service.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/ptz_controller/service.rs)
- [`src/ptz_controller/tapo_ptz.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/ptz_controller/tapo_ptz.rs)

### 3.3 AccessAbsorber（ストリームアクセス制限）

```
[ストリーム要求]
      ↓
[access_absorber] 優先度チェック
      ↓
許可 → ストリーム取得
拒否 → プリエンプション or 待機
```

**優先度**:
1. `click_modal` - LiveViewModal（最高）
2. `suggest_play` - 提案再生
3. `polling` - 定期ポーリング（最低）

**関連ファイル**:
- [`src/access_absorber/service.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/access_absorber/service.rs)

### 3.4 Paraclate連携

```
[summary_service] サマリー生成
      ↓
[paraclate_client] キュー投入
      ↓
[send_queue] 永続化
      ↓
[Paraclate API] 送信
```

**関連ファイル**:
- [`src/paraclate_client/client.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/paraclate_client/client.rs)
- [`src/summary_service/generator.rs`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is22/src/summary_service/generator.rs)

---

## 4. テストスイート詳細

### 4.1 テスト実行結果（2026-01-20）

```
=============================================
TEST SUMMARY
=============================================
Tests run:    15
Tests passed: 32
Tests failed: 0
=============================================
ALL TESTS PASSED
```

### 4.2 テストカバレッジ

#### test_camera_registration.sh

| Phase | 内容 | テスト数 |
|-------|------|---------|
| 1 | 接続検証（API/MySQL） | 2 |
| 2 | 事前クリーンアップ | 1 |
| 3 | テストデバイス注入 | 3 |
| 4 | Category B登録 + フィールド検証 | 11 |
| 5 | Category C登録 + フィールド検証 | 12 |
| 6 | API経由検証 | 1 |
| 7 | 削除テスト | 2 |
| 8 | 完全クリーンアップ | 2 |

#### 検証フィールド

| フィールド | Category B | Category C |
|-----------|-----------|-----------|
| camera_id | ✓ | ✓ |
| fid | ✓ 9999 | ✓ 9999 |
| family | ✓ tapo | ✓ unknown |
| access_family | ✓ unknown | ✓ unknown |
| mac_address | ✓ AA:BB:CC:DD:EE:01 | ✓ AA:BB:CC:DD:EE:02 |
| manufacturer | ✓ TestManufacturer | ✓ TestManufacturer |
| model | ✓ TestModel-B | ✓ TestModel-C |
| lacis_id | ✓ 3022AABBCCDDEE01* | ✓ 3022AABBCCDDEE02* |
| onvif_endpoint | ✓ 自動生成 | - 未設定 |
| ptz_supported | ✓ 0 | ✓ 0 |
| status | active | ✓ pending_auth |
| polling_enabled | 1 | ✓ 0 |

### 4.3 テスト実行方法

```bash
# サーバー上で実行
ssh mijeosadmin@192.168.125.246 "cd /opt/is22/tests && ./run_all_tests.sh"

# 特定テストのみ
./run_all_tests.sh camera_registration
```

---

## 5. レビュー依頼項目

### 5.1 アーキテクチャレビュー

- [ ] **モジュール分割**: 責務分離は適切か
- [ ] **依存関係**: 循環依存や過度な結合はないか
- [ ] **エラーハンドリング**: エラー伝播は一貫しているか
- [ ] **非同期処理**: tokioの使用は適切か

### 5.2 カメラ登録フローレビュー

- [ ] **Category B/C分岐**: 条件分岐は網羅的か
- [ ] **フィールド継承**: approve_device / force_register_camera で一貫しているか
- [ ] **PTZ自動検出**: ONVIFプローブのタイミングは適切か
- [ ] **重複登録防止**: IP/MAC重複チェックは十分か

### 5.3 PTZ制御レビュー

- [ ] **リース管理**: 排他制御は正しく動作するか
- [ ] **タイムアウト**: ハートビート・リース期限切れ処理
- [ ] **ベンダー対応**: Tapo以外の拡張性

### 5.4 AccessAbsorberレビュー

- [ ] **優先度制御**: プリエンプションは期待通りか
- [ ] **同時接続制限**: access_familyベースの制限は機能するか
- [ ] **セッション管理**: リーク防止策

### 5.5 Paraclate連携レビュー

- [ ] **キュー管理**: 送信失敗時のリトライ
- [ ] **ペイロード形式**: mobes2.0との整合性
- [ ] **状態同期**: aranea_registerとの連携

### 5.6 テストスイートレビュー

- [ ] **カバレッジ**: 未テストの重要パスはないか
- [ ] **エッジケース**: 異常系テストは十分か
- [ ] **テストデータ安全性**: 192.168.99.x/fid=9999は予約されているか
- [ ] **CI/CD統合**: 自動テスト実行の準備

---

## 6. 既知の課題・改善提案

### 6.1 現状の課題

1. **discovery_onvif未活用**: ipcamscan_devicesに格納されるが、登録時に参照されていない
2. **access_family自動設定**: 現状は常にunknown（manufacturer/modelからの自動判定未実装）
3. **PTZホームポジション**: ptz_home_supportedの検出は未実装

### 6.2 提案する追加テスト

1. **重複登録テスト**: 同一IP/MACでの再登録エラー確認
2. **不正データテスト**: 不正なfid/credentials での登録拒否
3. **並列登録テスト**: 同時複数登録の競合確認
4. **PTZ操作テスト**: リース取得→操作→解放のE2E
5. **AccessAbsorberテスト**: 優先度プリエンプションの検証

---

## 7. 関連ドキュメント

| ドキュメント | パス |
|-------------|------|
| 設計概要 | [`Paraclate_DesignOverview.md`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is21-22/designs/headdesign/Paraclate/Paraclate_DesignOverview.md) |
| スキーマ定義 | [`SCHEMA_DEFINITIONS.md`](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is21-22/designs/headdesign/Paraclate/SCHEMA_DEFINITIONS.md) |
| PTZ設計 | [`PTZctrl/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is21-22/designs/headdesign/PTZctrl) |
| AccessAbsorber設計 | [`camBrand/accessAbsorber/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is21-22/designs/headdesign/camBrand/accessAbsorber) |
| LostCamTracker設計 | [`lostcamTracker/`](https://github.com/warusakudeveroper/aranea_ISMS/tree/main/code/orangePi/is21-22/designs/headdesign/lostcamTracker) |

---

## 8. 回答期限・連絡先

**回答期限**: 2026-01-27（1週間）

レビュー完了後は本ドキュメントに回答セクションを追記してください。

---

## 9. 変更履歴

| 日付 | 内容 |
|------|------|
| 2026-01-20 | 初版作成 |

---

**レビュー担当者署名欄**:

- [ ] アーキテクチャレビュー完了: _______________
- [ ] テストスイートレビュー完了: _______________
- [ ] 総合承認: _______________
