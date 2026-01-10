# タスクインデックス: プリセットグラフィカルUI

## Issue: #107 プリセットグラフィカルUI・過剰検出判定・タグ傾向分析

## 設計ドキュメント
- [PresetGraphicalUI_Design.md](./PresetGraphicalUI_Design.md)

## 大原則の宣言

本実装は以下を遵守します：
- SSoT: PresetLoader単一ソース
- SOLID: 単一責任・依存逆転
- MECE: 過剰検出カテゴリの網羅性
- 車輪の再発明禁止: 既存サービス活用

---

## タスク一覧

### Phase 1: Backend - 過剰検出分析サービス

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T1-1 | OverdetectionAnalyzer構造体定義 | overdetection_analyzer/mod.rs | [x] |
| T1-2 | タグ固定化検出ロジック | overdetection_analyzer/tag_analyzer.rs | [x] |
| T1-3 | 固定物反応検出ロジック | overdetection_analyzer/static_detector.rs | [x] |
| T1-4 | lib.rs モジュール登録 | lib.rs | [x] |
| T1-5 | state.rs サービス追加 | state.rs | [x] |
| T1-6 | main.rs 初期化追加 | main.rs | [x] |

### Phase 2: Backend - API エンドポイント

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T2-1 | GET /api/presets/balance エンドポイント | routes.rs | [x] |
| T2-2 | GET /api/stats/overdetection エンドポイント | routes.rs | [x] |
| T2-3 | GET /api/stats/tags/:camera_id エンドポイント | routes.rs | [x] |
| T2-4 | GET /api/stats/tags/summary エンドポイント | routes.rs | [x] |
| T2-5 | PUT /api/cameras/:id/tag-filter エンドポイント | routes.rs | [ ] (将来実装) |

### Phase 3: Frontend - プリセットセレクターUI

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T3-1 | PresetSelector基本構造 | components/PresetSelector/index.tsx | [x] |
| T3-2 | BalanceChart棒グラフ | components/PresetSelector/BalanceChart.tsx | [x] |
| T3-3 | usePresetAnimationフック | hooks/usePresetAnimation.ts | [x] |
| T3-4 | プリセット切替アニメーション | components/PresetSelector/index.tsx | [x] |

### Phase 4: Frontend - 過剰検出表示

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T4-1 | OverdetectionAlert警告表示 | components/PresetSelector/OverdetectionAlert.tsx | [x] |
| T4-2 | TagDistribution分布グラフ | components/PresetSelector/TagDistribution.tsx | [x] |
| T4-3 | 調整導線ボタン | components/PresetSelector/OverdetectionAlert.tsx | [x] |
| T4-4 | チャットサジェスト機能 | components/EventLogPane.tsx | [x] |

### Phase 5: Frontend - カスタムタグフィルター

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| T5-1 | TagFilterModal | components/TagFilterModal.tsx | [ ] (将来実装) |
| T5-2 | SettingsModal統合 | components/SettingsModal.tsx | [x] |

### Phase 6: テスト

| ID | タスク | 状態 |
|----|--------|------|
| T6-1 | Backend APIテスト | [ ] |
| T6-2 | Frontend単体テスト | [ ] |
| T6-3 | Chrome UIテスト | [ ] |

---

## テストチェックリスト

### Backend

- [ ] GET /api/presets/balance → 12プリセットのバランス値取得
- [ ] GET /api/stats/overdetection → 過剰検出カメラリスト
- [ ] GET /api/stats/tags/:camera_id → カメラ別タグ傾向
- [ ] GET /api/stats/tags/summary → 全体タグサマリー
- [ ] PUT /api/cameras/:id/tag-filter → タグフィルター保存

### Frontend

- [ ] プリセットセレクター表示
- [ ] 棒グラフ4軸表示（検出感度/人物詳細/対象多様性/動体検知）
- [ ] プリセット切替アニメーション（500ms）
- [ ] 過剰検出警告表示（赤/黄）
- [ ] タグ分布棒グラフ（上位10件）
- [ ] 調整導線ボタン動作

### Chrome UI

- [ ] Settings Modal → 検出タブ
- [ ] 棒グラフがアニメーションで変化
- [ ] 過剰検出警告がハイライト
- [ ] タグ除外設定画面遷移
- [ ] カスタムタグフィルター保存

### チャットサジェスト

- [ ] 過剰検出時にAIアシスタント欄に通知表示
- [ ] 「{カメラ名}の検出の調整が必要そうです」メッセージ
- [ ] 「現在{現在プリセット}→{推奨プリセット}」表示
- [ ] はい/いいえボタン表示
- [ ] 「はい」クリック時にプリセット変更API呼び出し
- [ ] 対応済み表示

---

## 依存関係図

```
T1-1 ─┬─► T1-2 ─┬─► T1-4 ─► T1-5 ─► T1-6
      │         │
      └─► T1-3 ─┘
               │
               ▼
         T2-1 ─┬─► T3-1 ─► T3-2 ─► T3-3 ─► T3-4
               │
         T2-2 ─┴─► T4-1 ─► T4-2 ─► T4-3
               │
         T2-3 ─┤
               │
         T2-4 ─┤
               │
         T2-5 ─┴─► T5-1 ─► T5-2
                        │
                        ▼
                       T6-1 ─► T6-2 ─► T6-3
```

---

## API一覧

### プリセットバランス
- `GET /api/presets/balance` - 全プリセットのバランス値取得

### 過剰検出分析
- `GET /api/stats/overdetection?period=24h|7d` - 過剰検出判定結果
- `GET /api/stats/tags/:camera_id?period=24h|7d` - カメラ別タグ傾向
- `GET /api/stats/tags/summary?period=24h|7d` - 全体タグサマリー

### カスタムフィルター
- `PUT /api/cameras/:id/tag-filter` - タグフィルター設定

---

## 進捗サマリー

| Phase | 完了 | 合計 | 進捗率 |
|-------|------|------|--------|
| Phase 1 | 6 | 6 | 100% |
| Phase 2 | 4 | 5 | 80% |
| Phase 3 | 4 | 4 | 100% |
| Phase 4 | 4 | 4 | 100% |
| Phase 5 | 1 | 2 | 50% |
| Phase 6 | 0 | 3 | 0% |
| **合計** | **19** | **24** | **79%** |

---

## 実装ファイル一覧

### Backend (Rust)
- `src/overdetection_analyzer/mod.rs` - OverdetectionAnalyzer (NEW)
- `src/overdetection_analyzer/tag_analyzer.rs` - TagAnalyzer (NEW)
- `src/overdetection_analyzer/static_detector.rs` - StaticDetector (NEW)
- `src/lib.rs` - モジュール登録追加
- `src/state.rs` - AppStateにoverdetection_analyzer追加
- `src/main.rs` - サービス初期化追加
- `src/web_api/routes.rs` - 過剰検出・タグ傾向APIエンドポイント追加

### Frontend (React/TypeScript)
- `frontend/src/components/PresetSelector/index.tsx` - PresetSelector (NEW)
- `frontend/src/components/PresetSelector/BalanceChart.tsx` - BalanceChart (NEW)
- `frontend/src/components/PresetSelector/OverdetectionAlert.tsx` - OverdetectionAlert (NEW)
- `frontend/src/components/PresetSelector/TagDistribution.tsx` - TagDistribution (NEW)
- `frontend/src/hooks/usePresetAnimation.ts` - usePresetAnimation (NEW)
- `frontend/src/components/SettingsModal.tsx` - PresetSelector統合
- `frontend/src/components/EventLogPane.tsx` - チャットサジェスト機能追加

---

最終更新: 2026-01-08
