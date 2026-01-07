# AIEventLog_Redesign_v1: AI Event Log表示仕様準拠設計

文書番号: Layout＆AIlog_Settings/AIEventLog_Redesign_v1
作成日: 2026-01-08
ステータス: 設計確定
関連文書: AIEventlog.md, The_golden_rules.md

---

## 1. 大原則の宣言

本設計は以下の大原則に準拠する:

- [x] **SSoT**: AIEventlog.mdを唯一の情報源として参照
- [x] **SOLID-S**: EventLogPane.tsxの責務は表示のみ
- [x] **MECE**: 表示項目・色指定・イベント分類を網羅的・相互排他的に定義
- [x] **アンアンビギュアス**: 色コード・条件・動作を曖昧さなく明確に定義
- [x] **車輪の再発明禁止**: 既存コンポーネント構造を維持
- [x] **テスト計画あり**: フロントエンド・Chrome UIテストを定義

---

## 2. 概要

### 2.1 目的
`AIEventlog.md`で指摘された表示関連の問題を解決し、仕様通りのAI Event Log表示を実現する。

### 2.2 対象ファイル
- `frontend/src/components/EventLogPane.tsx`

### 2.3 スコープ
**Phase 1（本設計）: 表示関連の修正**
- 色スキームの仕様準拠
- カメラ名省略の改善
- ツールチップの追加
- アイコン視認性の向上

**Phase 2（将来）: AI判定関連**
- 推論統計タブ
- プリセット効果分析
- autoAttunement機能

---

## 3. ギャップ分析

### 3.1 仕様（AIEventlog.mdより）

#### 3.1.1 色スキーム仕様

| イベント種別 | 背景色 | 文字色 | 意図 |
|-------------|--------|--------|------|
| 人検知 (human) | #FFFF00 (黄) | #222222 | 最重要、目立つ色 |
| 車両検知 (vehicle) | #6495ED (青) | #FFFFFF | 人検知と明確に区別 |
| 異常検知 (alert/suspicious) | #FF0000 (赤) | #FFFF00 | 危険な印象 |
| 不明 (unknown) | #D3D3D3 (薄グレー) | #222222 | 曖昧な中間色 |
| カメラロスト (camera_lost) | #333333 (濃グレー) | #FFFFFF | ブラックアウト感 |
| カメラ復帰 (camera_recovered) | #FFFFFF (白) | #444444 | 目立たないが認識可能 |

#### 3.1.2 表示要件

| 項目 | 仕様 |
|------|------|
| カメラ名 | スペースがあれば省略しない |
| 丸数字(severity) | マウスホバーでヒント表示必須 |
| アイコン | 背景色と十分なコントラスト確保 |

### 3.2 現状実装の問題点

| ID | 問題 | 現状コード | 仕様違反 |
|----|------|-----------|----------|
| GAP-E01 | 色スキーム | Tailwind classes (bg-yellow-100等) | 完全不準拠 |
| GAP-E02 | カメラ名省略 | `max-w-[60px]` 固定truncate | スペースがあっても省略 |
| GAP-E03 | severity説明 | 数字表示のみ | ツールチップなし |
| GAP-E04 | unknown処理 | デフォルトseverity色 | 薄グレー仕様なし |
| GAP-E05 | アイコン視認性 | Tailwind色class | 背景との対比不十分 |

---

## 4. 詳細設計

### 4.1 GAP-E01: 色スキーム修正

#### 4.1.1 実装方針
Tailwindクラスから直接CSSスタイル指定に変更し、仕様の色コードを厳密に適用する。

#### 4.1.2 色定義（定数として追加）

```typescript
// EventLogPane.tsx - 色定義
const EVENT_COLORS = {
  human: {
    bg: '#FFFF00',
    text: '#222222',
    border: '#B8B800',
  },
  vehicle: {
    bg: '#6495ED',
    text: '#FFFFFF',
    border: '#4169E1',
  },
  alert: {
    bg: '#FF0000',
    text: '#FFFF00',
    border: '#CC0000',
  },
  unknown: {
    bg: '#D3D3D3',
    text: '#222222',
    border: '#A9A9A9',
  },
  camera_lost: {
    bg: '#333333',
    text: '#FFFFFF',
    border: '#1A1A1A',
  },
  camera_recovered: {
    bg: '#FFFFFF',
    text: '#444444',
    border: '#CCCCCC',
  },
} as const
```

#### 4.1.3 getEventTypeColor関数の置換

```typescript
// イベント種別から色スタイルを取得
function getEventStyle(primaryEvent: string): React.CSSProperties {
  const event = primaryEvent.toLowerCase()

  // 人検知（最優先）
  if (event === 'human' || event === 'person' || event === 'humans' || event === 'people') {
    return {
      backgroundColor: EVENT_COLORS.human.bg,
      color: EVENT_COLORS.human.text,
      borderLeft: `3px solid ${EVENT_COLORS.human.border}`,
    }
  }

  // 車両検知
  if (event === 'vehicle' || event === 'car' || event === 'truck') {
    return {
      backgroundColor: EVENT_COLORS.vehicle.bg,
      color: EVENT_COLORS.vehicle.text,
      borderLeft: `3px solid ${EVENT_COLORS.vehicle.border}`,
    }
  }

  // 異常検知
  if (event === 'suspicious' || event === 'alert' || event === 'intrusion' ||
      event === 'fire' || event === 'smoke') {
    return {
      backgroundColor: EVENT_COLORS.alert.bg,
      color: EVENT_COLORS.alert.text,
      borderLeft: `3px solid ${EVENT_COLORS.alert.border}`,
    }
  }

  // カメラロスト
  if (event === 'camera_lost') {
    return {
      backgroundColor: EVENT_COLORS.camera_lost.bg,
      color: EVENT_COLORS.camera_lost.text,
      borderLeft: `3px solid ${EVENT_COLORS.camera_lost.border}`,
    }
  }

  // カメラ復帰
  if (event === 'camera_recovered') {
    return {
      backgroundColor: EVENT_COLORS.camera_recovered.bg,
      color: EVENT_COLORS.camera_recovered.text,
      borderLeft: `3px solid ${EVENT_COLORS.camera_recovered.border}`,
    }
  }

  // デフォルト（unknown含む）
  return {
    backgroundColor: EVENT_COLORS.unknown.bg,
    color: EVENT_COLORS.unknown.text,
    borderLeft: `3px solid ${EVENT_COLORS.unknown.border}`,
  }
}
```

### 4.2 GAP-E02: カメラ名省略改善

#### 4.2.1 現状の問題
```tsx
// 現状: 固定幅で強制truncate
<span className="text-[10px] font-medium truncate max-w-[60px]">
  {cameraName}
</span>
```

#### 4.2.2 改善設計
```tsx
// 改善: flex-shrinkで可能な限り表示、必要時のみtruncate
<span
  className="text-[10px] font-medium truncate min-w-[40px] max-w-[120px] flex-shrink"
  title={cameraName}  // 常にフルネームをtitleで表示
>
  {cameraName}
</span>
```

- `min-w-[40px]`: 最低40pxは確保
- `max-w-[120px]`: 最大120pxまで拡張可能
- `flex-shrink`: 他要素との兼ね合いで縮小
- `title={cameraName}`: ホバーでフルネーム表示

### 4.3 GAP-E03: Severityツールチップ追加

#### 4.3.1 severity定義

| 値 | 意味 | 説明テキスト |
|----|------|-------------|
| 1 | Low | 低優先度 - 通常の検知 |
| 2 | Medium | 中優先度 - 注意が必要 |
| 3 | High | 高優先度 - 即時確認推奨 |
| 4 | Critical | 緊急 - 即時対応必要 |

#### 4.3.2 実装

```tsx
// Severityツールチップ定義
const SEVERITY_TOOLTIPS: Record<number, string> = {
  1: '低優先度 - 通常の検知',
  2: '中優先度 - 注意が必要',
  3: '高優先度 - 即時確認推奨',
  4: '緊急 - 即時対応必要',
}

// Badge修正
<Badge
  variant={badgeVariant}
  className="text-[9px] px-1 py-0 h-4 min-w-[18px] justify-center cursor-help"
  title={SEVERITY_TOOLTIPS[log.severity] || `重要度: ${log.severity}`}
>
  {log.severity}
</Badge>
```

### 4.4 GAP-E04/E05: アイコン視認性改善

#### 4.4.1 設計方針
背景色に応じてアイコン色を動的に決定し、常にコントラスト比4.5:1以上を確保する。

#### 4.4.2 アイコン色定義

```typescript
// 背景色に対応するアイコン色
const ICON_COLORS = {
  human: '#222222',      // 黄色背景 → 濃い文字
  vehicle: '#FFFFFF',    // 青背景 → 白文字
  alert: '#FFFF00',      // 赤背景 → 黄文字
  unknown: '#555555',    // グレー背景 → 濃いグレー
  camera_lost: '#FFFFFF', // 濃グレー背景 → 白
  camera_recovered: '#444444', // 白背景 → グレー
}

function getIconColor(primaryEvent: string): string {
  const event = primaryEvent.toLowerCase()

  if (event === 'human' || event === 'person' || event === 'humans' || event === 'people') {
    return ICON_COLORS.human
  }
  if (event === 'vehicle' || event === 'car' || event === 'truck') {
    return ICON_COLORS.vehicle
  }
  if (event === 'suspicious' || event === 'alert' || event === 'intrusion' ||
      event === 'fire' || event === 'smoke') {
    return ICON_COLORS.alert
  }
  if (event === 'camera_lost') {
    return ICON_COLORS.camera_lost
  }
  if (event === 'camera_recovered') {
    return ICON_COLORS.camera_recovered
  }
  return ICON_COLORS.unknown
}
```

---

## 5. 変更ファイル一覧

| ファイル | 変更内容 | 行数見積 |
|----------|---------|----------|
| `frontend/src/components/EventLogPane.tsx` | 色定義追加、getEventStyle関数置換、ツールチップ追加、アイコン色対応 | +80行, -30行 |

---

## 6. タスクリスト

| タスクID | 内容 | 依存 | 見積 |
|----------|------|------|------|
| IMPL-E01 | EVENT_COLORS定数追加 | - | 10分 |
| IMPL-E02 | getEventStyle関数実装 | E01 | 15分 |
| IMPL-E03 | DetectionLogItemコンポーネント修正 | E02 | 20分 |
| IMPL-E04 | カメラ名表示幅改善 | - | 10分 |
| IMPL-E05 | SEVERITY_TOOLTIPS追加・Badge修正 | - | 10分 |
| IMPL-E06 | getIconColor関数追加・適用 | E02 | 15分 |
| TEST-E01 | フロントエンドビルド確認 | E01-E06 | 5分 |
| TEST-E02 | Chrome UIテスト実行 | TEST-E01 | 30分 |

---

## 7. テスト計画

### 7.1 フロントエンドテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| FE-E01 | TypeScriptコンパイル | エラーなし |
| FE-E02 | npm run build | 成功 |

### 7.2 Chrome UIテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| UI-E01 | 人検知イベント表示 | 背景#FFFF00、文字#222222 |
| UI-E02 | 車両検知イベント表示 | 背景#6495ED、文字#FFFFFF |
| UI-E03 | 異常検知イベント表示 | 背景#FF0000、文字#FFFF00 |
| UI-E04 | unknownイベント表示 | 背景#D3D3D3、文字#222222 |
| UI-E05 | camera_lost表示 | 背景#333333、文字#FFFFFF |
| UI-E06 | camera_recovered表示 | 背景#FFFFFF、文字#444444 |
| UI-E07 | カメラ名表示（短い名前） | 省略なし |
| UI-E08 | カメラ名表示（長い名前） | 120px超でtruncate、hoverでフル表示 |
| UI-E09 | severityバッジhover | ツールチップ表示 |
| UI-E10 | アイコン視認性（全種別） | 背景との対比が明確 |

### 7.3 テスト実行環境
- ブラウザ: Chrome
- URL: http://192.168.125.246:3000/
- 確認方法: スクリーンショット + 開発者ツールでCSS確認

---

## 8. 受け入れ条件

1. [ ] 全色スキームがAIEventlog.md仕様通り
2. [ ] カメラ名がスペースがある場合は省略されない
3. [ ] severityバッジにツールチップが表示される
4. [ ] アイコンが全背景色で視認可能
5. [ ] Chrome UIテスト10項目全てPass

---

## 9. MECEチェック

本設計は以下の観点で網羅的・相互排他的:

| 観点 | 分類 | 網羅性確認 |
|------|------|-----------|
| イベント種別 | human/vehicle/alert/unknown/camera_lost/camera_recovered | 全種別カバー |
| 色指定 | 背景/文字/ボーダー | 各種別で定義 |
| 表示要素 | アイコン/カメラ名/イベントラベル/時刻/severity/confidence/tags | 全要素対応 |
| ユーザー操作 | クリック/ホバー | 両方対応 |

---

## 10. 将来拡張（Phase 2）

AIEventlog.mdで指摘されている以下の項目は、本設計完了後に別設計として対応:

1. 推論統計タブ（設定モーダル内）
2. プリセット効果分析機能
3. ストレージクォータ設定
4. autoAttunement基本設計
5. LLMコンテキスト連携

これらは本設計のPhase 1完了を前提条件とする。

---

## 付録A: 現状実装との差分サマリ

```diff
// getEventTypeColor → getEventStyle に置換
- function getEventTypeColor(primaryEvent: string, severity: number): string {
-   // Tailwind classes
-   return "bg-yellow-100 dark:bg-yellow-900/60 text-yellow-900..."
- }
+ function getEventStyle(primaryEvent: string): React.CSSProperties {
+   // 直接CSS Style
+   return { backgroundColor: '#FFFF00', color: '#222222', ... }
+ }

// カメラ名表示
- <span className="... truncate max-w-[60px]">
+ <span className="... truncate min-w-[40px] max-w-[120px] flex-shrink" title={cameraName}>

// Severityバッジ
- <Badge variant={badgeVariant} className="...">
+ <Badge variant={badgeVariant} className="... cursor-help" title={SEVERITY_TOOLTIPS[log.severity]}>
```
