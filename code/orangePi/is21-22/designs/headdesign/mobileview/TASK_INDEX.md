# IS22 モバイルビュー TASK_INDEX

**Issue**: #108
**設計ドキュメント**:
- MobileView_BasicDesign.md
- MobileView_DetailedDesign.md

---

## フェーズ構成

| Phase | 内容 | 依存 |
|-------|------|------|
| Phase 1 | 基盤整備 (useMediaQuery, CSS) | なし |
| Phase 2 | レイアウト分岐 (App.tsx) | Phase 1 |
| Phase 3 | CameraGrid 3列対応 | Phase 2 |
| Phase 4 | ドロワー・フローティングボタン | Phase 2 |
| Phase 5 | モーダル最適化 | Phase 2 |
| Phase 6 | SuggestPane アニメーション | Phase 2 |
| Phase 7 | テスト・調整 | Phase 1-6 |

---

## Phase 1: 基盤整備

### タスク一覧

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| 1-1 | useMediaQueryフック作成 | `src/hooks/useMediaQuery.ts` | [ ] |
| 1-2 | ブレークポイントCSS追加 | `src/index.css` | [ ] |
| 1-3 | モバイルアニメーションCSS追加 | `src/index.css` | [ ] |

### 受け入れ条件
- [ ] `useIsMobile()` が768px以下でtrueを返す
- [ ] ブレークポイント切替でリアルタイム更新される
- [ ] SSR安全（初期値false）

---

## Phase 2: レイアウト分岐

### タスク一覧

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| 2-1 | App.tsx にuseIsMobile導入 | `src/App.tsx` | [ ] |
| 2-2 | モバイルレイアウト条件分岐実装 | `src/App.tsx` | [ ] |
| 2-3 | drawerOpen状態管理追加 | `src/App.tsx` | [ ] |
| 2-4 | hasUnreadEvents状態管理追加 | `src/App.tsx` | [ ] |

### 受け入れ条件
- [ ] 768px以下でモバイルレイアウトに切替
- [ ] 768px超でデスクトップレイアウト維持
- [ ] ドロワー開閉状態が正しく管理される

---

## Phase 3: CameraGrid 3列対応

### タスク一覧

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| 3-1 | CameraGrid Props追加 (isMobile, columns, allowScroll) | `src/components/CameraGrid.tsx` | [ ] |
| 3-2 | 3列グリッドレイアウト実装 | `src/components/CameraGrid.tsx` | [ ] |
| 3-3 | 縦スクロール許容実装 | `src/components/CameraGrid.tsx` | [ ] |
| 3-4 | 1:1正方形タイル強制 | `src/components/CameraGrid.tsx` | [ ] |
| 3-5 | CameraTile isMobile Props追加 | `src/components/CameraTile.tsx` | [ ] |
| 3-6 | タッチ操作最適化 | `src/components/CameraTile.tsx` | [ ] |

### 受け入れ条件
- [ ] モバイルで3列表示される
- [ ] タイルが正方形で表示される
- [ ] カメラ増加時に縦スクロールできる
- [ ] タッチでタイル選択できる
- [ ] DnD並べ替えがタッチで動作する

---

## Phase 4: ドロワー・フローティングボタン

### タスク一覧

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| 4-1 | FloatingAIButton新規作成 | `src/components/FloatingAIButton.tsx` | [ ] |
| 4-2 | MobileDrawer新規作成 | `src/components/MobileDrawer.tsx` | [ ] |
| 4-3 | App.tsxにFloatingAIButton配置 | `src/App.tsx` | [ ] |
| 4-4 | App.tsxにMobileDrawer配置 | `src/App.tsx` | [ ] |
| 4-5 | EventLogPane isMobile Props追加 | `src/components/EventLogPane.tsx` | [ ] |
| 4-6 | EventLogPane モバイルレイアウト調整 | `src/components/EventLogPane.tsx` | [ ] |
| 4-7 | 新着イベント通知バッジ実装 | `src/components/FloatingAIButton.tsx` | [ ] |

### 受け入れ条件
- [ ] 右下にフローティングボタンが表示される
- [ ] ボタンタップでドロワーが開く
- [ ] ドロワーが右からスライドインする
- [ ] 背景タップ/×ボタンでドロワーが閉じる
- [ ] ドロワー内にEventLogPaneが表示される
- [ ] 新着イベント時にバッジが表示される

---

## Phase 5: モーダル最適化

### タスク一覧

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| 5-1 | LiveViewModal幅/高さ調整 | `src/components/LiveViewModal.tsx` | [ ] |
| 5-2 | CameraDetailModal幅/高さ調整 | `src/components/CameraDetailModal.tsx` | [ ] |
| 5-3 | DetectionDetailModal幅/高さ調整 | `src/components/DetectionDetailModal.tsx` | [ ] |
| 5-4 | ScanModal幅/高さ調整 | `src/components/ScanModal.tsx` | [ ] |
| 5-5 | SettingsModal調整確認 | `src/components/SettingsModal.tsx` | [ ] |

### 受け入れ条件
- [ ] 全モーダルがモバイル画面に収まる
- [ ] モーダル内スクロールが正常動作
- [ ] 閉じるボタンがタップしやすい

---

## Phase 6: SuggestPane アニメーション

### タスク一覧

| ID | タスク | ファイル | 状態 |
|----|--------|----------|------|
| 6-1 | SuggestPane isMobile Props追加 | `src/components/SuggestPane.tsx` | [ ] |
| 6-2 | アニメーション方向分岐実装 | `src/components/SuggestPane.tsx` | [ ] |
| 6-3 | 左からスライドイン実装 | `src/components/SuggestPane.tsx` | [ ] |
| 6-4 | 上にスライドアウト実装 | `src/components/SuggestPane.tsx` | [ ] |

### 受け入れ条件
- [ ] モバイルでイベント発生時、左からスライドインする
- [ ] イベント終了時、上にスライドアウトする
- [ ] PC版のアニメーション方向は維持される

---

## Phase 7: テスト・調整

### タスク一覧

| ID | タスク | 状態 |
|----|--------|------|
| 7-1 | Chrome DevTools モバイルエミュレーションテスト | [ ] |
| 7-2 | 実機テスト（Android） | [ ] |
| 7-3 | 表示破綻チェック | [ ] |
| 7-4 | 操作性チェック | [ ] |
| 7-5 | パフォーマンスチェック | [ ] |
| 7-6 | 調整・バグ修正 | [ ] |

---

## テスト計画

### 7.1 ユニットテスト

| テスト | 内容 |
|--------|------|
| useMediaQuery | 768px境界でのtrue/false切替 |
| calculateTileWidth | 3列時のタイル幅計算 |

### 7.2 フロントエンドテスト（Chrome DevTools）

| テスト | デバイス | 確認項目 |
|--------|----------|----------|
| レイアウト切替 | iPhone 14 Pro (390x844) | モバイルレイアウト表示 |
| レイアウト切替 | iPad (768x1024) | 境界値動作 |
| レイアウト切替 | Desktop (1920x1080) | デスクトップ維持 |
| 3列グリッド | iPhone 14 Pro | タイル3列表示 |
| スクロール | iPhone 14 Pro | 縦スクロール動作 |
| ドロワー | iPhone 14 Pro | 開閉動作 |
| モーダル | iPhone 14 Pro | 画面内収まり |
| アニメーション | iPhone 14 Pro | 方向確認 |

### 7.3 Chrome実機テスト

| テスト | URL | 確認項目 |
|--------|-----|----------|
| 基本表示 | http://192.168.125.246:3000/ | レイアウト |
| タイルタップ | 同上 | モーダル表示 |
| AIボタン | 同上 | ドロワー開閉 |
| イベント受信 | 同上 | サジェストアニメーション |

### 7.4 表示破綻チェックリスト

| 項目 | 確認方法 | 結果 |
|------|----------|------|
| 3ペーン崩壊 | 768px以下でモバイル化 | [ ] |
| タイル最小幅 | 幅100px以上維持 | [ ] |
| テキストはみ出し | カメラ名truncate | [ ] |
| モーダルはみ出し | 95vw以内 | [ ] |
| ドロワー重なり | z-index適切 | [ ] |
| タップ領域 | 44x44px以上 | [ ] |
| スクロール競合 | 各領域独立 | [ ] |

---

## 完了条件

- [ ] Phase 1-6 全タスク完了
- [ ] Phase 7 全テスト合格
- [ ] 表示破綻なし
- [ ] デスクトップレイアウト後方互換維持
- [ ] MECE確認: 全機能モバイルで利用可能
- [ ] アンアンビギュアス確認: 仕様通りの動作

---

## 進捗サマリー

| Phase | 進捗 | 備考 |
|-------|------|------|
| Phase 1 | 0/3 | |
| Phase 2 | 0/4 | |
| Phase 3 | 0/6 | |
| Phase 4 | 0/7 | |
| Phase 5 | 0/5 | |
| Phase 6 | 0/4 | |
| Phase 7 | 0/6 | |
| **合計** | **0/35** | |
