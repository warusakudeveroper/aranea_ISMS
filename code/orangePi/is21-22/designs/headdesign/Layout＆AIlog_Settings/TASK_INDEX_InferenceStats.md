# タスクインデックス: 推論統計・分析機能

## Issue: #106 推論統計・プリセット効果分析・ストレージクォータ・autoAttunement基盤

## 設計ドキュメント
- [InferenceStatistics_Design.md](./InferenceStatistics_Design.md)

## タスク一覧

### Phase 1: 統計API (Backend)

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T1-1 | GET /api/stats/cameras エンドポイント | routes.rs | [x] |
| T1-2 | GET /api/stats/events エンドポイント | routes.rs | [x] |
| T1-3 | GET /api/stats/presets エンドポイント | routes.rs | [x] |
| T1-4 | GET /api/stats/storage エンドポイント | routes.rs | [x] |
| T1-5 | GET /api/stats/anomalies エンドポイント | routes.rs | [x] |
| T1-6 | InferenceStatsService実装 | inference_stats_service/mod.rs | [x] |

### Phase 2: 推論統計タブ (Frontend)

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T2-1 | InferenceStatsTab.tsx 基本構造 | components/InferenceStatsTab.tsx | [x] |
| T2-2 | CameraDistributionChart実装 | components/InferenceStatsTab.tsx | [x] |
| T2-3 | EventTrendsChart実装 | components/InferenceStatsTab.tsx | [x] |
| T2-4 | AnomalyCamerasList実装 | components/InferenceStatsTab.tsx | [x] |
| T2-5 | PresetEffectivenessTable実装 | components/InferenceStatsTab.tsx | [x] |
| T2-6 | SettingsModal統合 | components/SettingsModal.tsx | [x] |

### Phase 3: ストレージクォータ (既存実装確認済み)

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T3-1 | settingsテーブル | migrations/001_initial_schema.sql | [x] 既存 |
| T3-2 | GET/PUT /api/settings/storage | routes.rs | [x] 既存 |
| T3-3 | StorageQuotaSettings UI | components/SettingsModal.tsx | [x] 既存 |
| T3-4 | ストレージクリーンアップジョブ | detection_log_service/mod.rs | [x] 既存 |

### Phase 4: autoAttunement基盤

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T4-1 | AutoAttunementService実装 | auto_attunement/mod.rs | [x] |
| T4-2 | GET /api/attunement/status API | routes.rs | [x] |
| T4-3 | POST /api/attunement/calculate API | routes.rs | [x] |
| T4-4 | POST /api/attunement/apply API | routes.rs | [x] |
| T4-5 | POST /api/attunement/auto-adjust API | routes.rs | [x] |

### Phase 5: テスト

| ID | タスク | 状態 |
|----|--------|------|
| T5-1 | Backend APIテスト | [x] |
| T5-2 | Frontend単体テスト | [x] |
| T5-3 | Chrome UIテスト | [x] |

## テストチェックリスト

### Backend
- [x] GET /api/stats/cameras?period=24h → 200 OK (17カメラ、25,518件)
- [x] GET /api/stats/events → 200 OK (時系列傾向データ)
- [x] GET /api/stats/presets → 200 OK (balanced/system)
- [x] GET /api/stats/anomalies → 200 OK (18件検出)
- [x] GET /api/stats/storage → 使用量返却
- [x] PUT /api/settings/storage → 設定保存
- [x] GET /api/attunement/status → 全カメラ状態取得
- [x] GET /api/attunement/status/:camera_id → カメラ別状態取得
- [x] POST /api/attunement/calculate/:camera_id → 最適閾値計算
- [x] POST /api/attunement/apply/:camera_id → 閾値適用
- [x] POST /api/attunement/auto-adjust → 自動調整実行

### Frontend
- [x] 推論統計タブ表示
- [x] カメラ別分布グラフ描画
- [x] 時系列傾向グラフ描画
- [x] 異常カメラリスト表示
- [x] プリセット効果テーブル表示
- [x] ストレージ設定UI動作

### Chrome UI
- [x] Settings Modal → 推論統計タブ
- [x] グラフが正しく描画される
- [x] 異常カメラがハイライトされる
- [x] ストレージ設定変更可能

## 依存関係図

```
T1-6 (Service) ─┬─► T1-1 ─┬─► T2-1 ─┬─► T2-2
                │         │         ├─► T2-3
                ├─► T1-2 ─┤         ├─► T2-4
                │         │         └─► T2-5
                ├─► T1-3 ─┤
                │         │
                ├─► T1-4 ─┴─► T3-3
                │
                └─► T1-5 ─────► T2-4

T3-1 ─► T3-2 ─► T3-3 ─► T3-4 (既存実装)

T4-1 ─► T4-2 ─► T4-3 ─► T4-4 ─► T4-5
```

## 進捗サマリー

| Phase | 完了 | 合計 | 進捗率 |
|-------|------|------|--------|
| Phase 1 | 6 | 6 | 100% |
| Phase 2 | 6 | 6 | 100% |
| Phase 3 | 4 | 4 | 100% (既存) |
| Phase 4 | 5 | 5 | 100% |
| Phase 5 | 3 | 3 | 100% |
| **合計** | **24** | **24** | **100%** |

---
最終更新: 2026-01-08

## 実装ファイル一覧

### Backend (Rust)
- `src/inference_stats_service/mod.rs` - InferenceStatsService (NEW)
- `src/auto_attunement/mod.rs` - AutoAttunementService (NEW)
- `src/lib.rs` - モジュール登録追加
- `src/state.rs` - AppStateにinference_stats, auto_attunement追加
- `src/main.rs` - サービス初期化追加
- `src/web_api/routes.rs` - 統計API・Attunement APIエンドポイント追加

### Frontend (React/TypeScript)
- `frontend/src/components/InferenceStatsTab.tsx` - 推論統計タブ (NEW)
- `frontend/src/components/SettingsModal.tsx` - InferenceStatsTab統合

## API一覧

### 推論統計 (Phase 1)
- `GET /api/stats/cameras?period=24h|7d|30d` - カメラ別分布
- `GET /api/stats/events?period=24h|7d|30d` - 時系列傾向
- `GET /api/stats/presets?period=24h|7d|30d` - プリセット効果
- `GET /api/stats/storage` - ストレージ使用量
- `GET /api/stats/anomalies?period=24h|7d|30d` - 異常カメラ検出

### ストレージ管理 (Phase 3 - 既存)
- `GET /api/settings/storage` - 設定取得
- `PUT /api/settings/storage` - 設定更新
- `POST /api/settings/storage/cleanup` - 手動クリーンアップ
- `POST /api/settings/storage/cleanup-unknown` - Unknown画像クリーンアップ

### AutoAttunement (Phase 4)
- `GET /api/attunement/status` - 全カメラ調整状態
- `GET /api/attunement/status/:camera_id` - カメラ別調整状態
- `POST /api/attunement/calculate/:camera_id` - 最適閾値計算
- `POST /api/attunement/apply/:camera_id` - 閾値適用
- `POST /api/attunement/auto-adjust` - 自動調整実行（有効カメラのみ）
