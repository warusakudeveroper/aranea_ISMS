# タスクインデックス: 推論統計・分析機能

## Issue: #105 推論統計・プリセット効果分析・ストレージクォータ・autoAttunement基盤

## 設計ドキュメント
- [InferenceStatistics_Design.md](./InferenceStatistics_Design.md)

## タスク一覧

### Phase 1: 統計API (Backend)

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T1-1 | GET /api/stats/cameras エンドポイント | routes.rs | [ ] |
| T1-2 | GET /api/stats/events エンドポイント | routes.rs | [ ] |
| T1-3 | GET /api/stats/presets エンドポイント | routes.rs | [ ] |
| T1-4 | GET /api/stats/storage エンドポイント | routes.rs | [ ] |
| T1-5 | GET /api/stats/anomalies エンドポイント | routes.rs | [ ] |
| T1-6 | InferenceStatsService実装 | inference_stats_service/mod.rs | [ ] |

### Phase 2: 推論統計タブ (Frontend)

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T2-1 | InferenceStatsTab.tsx 基本構造 | components/InferenceStatsTab.tsx | [ ] |
| T2-2 | CameraDistributionChart実装 | components/InferenceStatsTab.tsx | [ ] |
| T2-3 | EventTrendsChart実装 | components/InferenceStatsTab.tsx | [ ] |
| T2-4 | AnomalyCamerasList実装 | components/InferenceStatsTab.tsx | [ ] |
| T2-5 | PresetEffectivenessTable実装 | components/InferenceStatsTab.tsx | [ ] |
| T2-6 | SettingsModal統合 | components/SettingsModal.tsx | [ ] |

### Phase 3: ストレージクォータ

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T3-1 | facility_settings拡張マイグレーション | migrations/020_storage_quota.sql | [ ] |
| T3-2 | POST /api/settings/storage | routes.rs | [ ] |
| T3-3 | StorageQuotaSettings UI | components/StorageTab.tsx | [ ] |
| T3-4 | ストレージクリーンアップジョブ | cleanup_job.rs | [ ] |

### Phase 4: autoAttunement基盤

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T4-1 | StatsCollector実装 | stats_collector/mod.rs | [ ] |
| T4-2 | ThresholdAdjuster実装 | threshold_adjuster/mod.rs | [ ] |
| T4-3 | AutoAttunementService実装 | auto_attunement/mod.rs | [ ] |
| T4-4 | 閾値自動調整API | routes.rs | [ ] |

### Phase 5: テスト

| ID | タスク | 状態 |
|----|--------|------|
| T5-1 | Backend APIテスト | [ ] |
| T5-2 | Frontend単体テスト | [ ] |
| T5-3 | Chrome UIテスト | [ ] |

## テストチェックリスト

### Backend
- [ ] GET /api/stats/cameras?period=24h → 200 OK
- [ ] GET /api/stats/presets → 200 OK
- [ ] GET /api/stats/storage → 使用量返却
- [ ] POST /api/settings/storage → 設定保存
- [ ] ストレージ90%超過時クリーンアップ

### Frontend
- [ ] 推論統計タブ表示
- [ ] カメラ別分布グラフ描画
- [ ] 時系列傾向グラフ描画
- [ ] 異常カメラリスト表示
- [ ] ストレージ設定UI動作

### Chrome UI
- [ ] Settings Modal → 推論統計タブ
- [ ] グラフが正しく描画される
- [ ] 異常カメラがハイライトされる
- [ ] ストレージ設定変更可能

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

T3-1 ─► T3-2 ─► T3-3 ─► T3-4

T4-1 ─► T4-2 ─► T4-3 ─► T4-4
```

## 進捗サマリー

| Phase | 完了 | 合計 | 進捗率 |
|-------|------|------|--------|
| Phase 1 | 0 | 6 | 0% |
| Phase 2 | 0 | 6 | 0% |
| Phase 3 | 0 | 4 | 0% |
| Phase 4 | 0 | 4 | 0% |
| Phase 5 | 0 | 3 | 0% |
| **合計** | **0** | **23** | **0%** |

---
最終更新: 2026-01-08
