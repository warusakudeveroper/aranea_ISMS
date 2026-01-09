# IS22 モバイルビュー詳細設計

**Issue**: #108
**基本設計**: MobileView_BasicDesign.md
**作成日**: 2025-01-09

---

## 1. useMediaQuery フック

### 1.1 ファイル
`src/hooks/useMediaQuery.ts` (新規作成)

### 1.2 仕様

```typescript
/**
 * メディアクエリ判定フック
 * SSR対応のため初期値はfalse
 */
export function useMediaQuery(query: string): boolean

/**
 * 事前定義フック
 */
export function useIsMobile(): boolean  // max-width: 768px
export function useIsTablet(): boolean  // 769px - 1024px
export function useIsDesktop(): boolean // min-width: 1025px
```

### 1.3 実装詳細

```typescript
import { useState, useEffect } from 'react'

export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(false)

  useEffect(() => {
    const mediaQuery = window.matchMedia(query)
    setMatches(mediaQuery.matches)

    const handler = (event: MediaQueryListEvent) => {
      setMatches(event.matches)
    }

    mediaQuery.addEventListener('change', handler)
    return () => mediaQuery.removeEventListener('change', handler)
  }, [query])

  return matches
}

export function useIsMobile(): boolean {
  return useMediaQuery('(max-width: 768px)')
}

export function useIsTablet(): boolean {
  return useMediaQuery('(min-width: 769px) and (max-width: 1024px)')
}

export function useIsDesktop(): boolean {
  return useMediaQuery('(min-width: 1025px)')
}
```

### 1.4 表示破綻防止
- `useState(false)` で初期化（SSR安全）
- `useEffect` 内でクライアントサイドのみ判定
- リサイズ時は `matchMedia` の `change` イベントで自動更新

---

## 2. App.tsx レイアウト分岐

### 2.1 変更箇所
`src/App.tsx` 行 311-425 付近

### 2.2 デスクトップレイアウト（既存維持）

```tsx
// 既存のまま
<div className="h-screen flex flex-col">
  <header className="h-14 ...">...</header>
  <div className="flex-1 flex overflow-hidden">
    <aside className="w-[30%] ..."><SuggestPane /></aside>
    <main className="w-[55%] ..."><CameraGrid /></main>
    <aside className="w-[15%] ..."><EventLogPane /></aside>
  </div>
</div>
```

### 2.3 モバイルレイアウト（新規）

```tsx
const isMobile = useIsMobile()

return isMobile ? (
  <div className="h-screen flex flex-col">
    {/* Header - 共通 */}
    <header className="h-14 flex-shrink-0 ...">...</header>

    {/* Main Content - 縦スクロール */}
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* SuggestView - 上30% */}
      <div className="h-[30vh] flex-shrink-0 overflow-hidden">
        <SuggestPane isMobile={true} />
      </div>

      {/* CameraGrid - 残り70%、内部スクロール */}
      <div className="flex-1 overflow-y-auto p-2">
        <CameraGrid
          isMobile={true}
          columns={3}
          allowScroll={true}
        />
      </div>
    </div>

    {/* Floating AI Button */}
    <FloatingAIButton
      onClick={() => setDrawerOpen(true)}
      hasNewEvents={hasUnreadEvents}
    />

    {/* AI Event Log Drawer */}
    <MobileDrawer
      open={drawerOpen}
      onClose={() => setDrawerOpen(false)}
    >
      <EventLogPane isMobile={true} />
    </MobileDrawer>

    {/* Modals - 共通 */}
    <ScanModal ... />
    <CameraDetailModal ... />
    <LiveViewModal ... />
    <SettingsModal ... />
  </div>
) : (
  // 既存デスクトップレイアウト
  <div className="h-screen flex flex-col">...</div>
)
```

### 2.4 状態管理追加

```typescript
// App.tsx に追加
const [drawerOpen, setDrawerOpen] = useState(false)
const [hasUnreadEvents, setHasUnreadEvents] = useState(false)

// WebSocket イベント受信時
const onEventLog = (event: EventLogMessage) => {
  // 既存処理...

  // モバイル時はドロワー閉じていれば未読フラグ
  if (isMobile && !drawerOpen) {
    setHasUnreadEvents(true)
  }
}

// ドロワー開いたら未読クリア
useEffect(() => {
  if (drawerOpen) {
    setHasUnreadEvents(false)
  }
}, [drawerOpen])
```

---

## 3. SuggestPane アニメーション分岐

### 3.1 変更箇所
`src/components/SuggestPane.tsx`

### 3.2 Props追加

```typescript
interface SuggestPaneProps {
  // 既存props...
  isMobile?: boolean  // 追加
}
```

### 3.3 アニメーションCSS

```css
/* index.css に追加 */

/* PC版アニメーション（既存） */
@keyframes suggest-slide-in-pc {
  from { transform: translateY(-100%); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
@keyframes suggest-slide-out-pc {
  from { transform: translateX(0); opacity: 1; }
  to { transform: translateX(-100%); opacity: 0; }
}

/* モバイル版アニメーション（新規） */
@keyframes suggest-slide-in-mobile {
  from { transform: translateX(-100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}
@keyframes suggest-slide-out-mobile {
  from { transform: translateY(0); opacity: 1; }
  to { transform: translateY(-100%); opacity: 0; }
}

.suggest-animate-in-pc {
  animation: suggest-slide-in-pc 500ms ease-out forwards;
}
.suggest-animate-out-pc {
  animation: suggest-slide-out-pc 500ms ease-in forwards;
}
.suggest-animate-in-mobile {
  animation: suggest-slide-in-mobile 500ms ease-out forwards;
}
.suggest-animate-out-mobile {
  animation: suggest-slide-out-mobile 500ms ease-in forwards;
}
```

### 3.4 コンポーネント内分岐

```tsx
function SuggestPane({ isMobile = false, ...props }: SuggestPaneProps) {
  const [isVisible, setIsVisible] = useState(true)
  const [animationState, setAnimationState] = useState<'in' | 'out' | 'idle'>('idle')

  const animationClass = useMemo(() => {
    if (animationState === 'idle') return ''
    const direction = isMobile ? 'mobile' : 'pc'
    return `suggest-animate-${animationState}-${direction}`
  }, [animationState, isMobile])

  return (
    <div className={cn("h-full", animationClass)}>
      {/* 既存コンテンツ */}
    </div>
  )
}
```

---

## 4. CameraGrid 3列対応

### 4.1 変更箇所
`src/components/CameraGrid.tsx`

### 4.2 Props追加

```typescript
interface CameraGridProps {
  // 既存props...
  isMobile?: boolean
  columns?: number      // デフォルト4、モバイル時3
  allowScroll?: boolean // デフォルトfalse、モバイル時true
}
```

### 4.3 グリッド計算変更

```typescript
function CameraGrid({
  isMobile = false,
  columns = 4,
  allowScroll = false,
  ...props
}: CameraGridProps) {
  const effectiveCols = isMobile ? 3 : columns
  const gap = isMobile ? 8 : 16

  // タイル高さ計算
  const calculateTileHeight = useCallback(() => {
    if (!containerRef.current) return undefined

    const containerWidth = containerRef.current.offsetWidth
    const tileWidth = (containerWidth - gap * (effectiveCols - 1)) / effectiveCols

    if (isMobile) {
      // モバイル: 常に1:1正方形
      return tileWidth
    } else {
      // PC: 既存の圧縮計算ロジック
      // ...existing logic...
    }
  }, [isMobile, effectiveCols, gap])

  return (
    <div
      ref={containerRef}
      className={cn(
        "h-full",
        allowScroll ? "overflow-y-auto" : "overflow-hidden"
      )}
    >
      <div
        className="grid"
        style={{
          gridTemplateColumns: `repeat(${effectiveCols}, 1fr)`,
          gap: `${gap}px`
        }}
      >
        {sortedCameras.map(camera => (
          <SortableCameraTile
            key={camera.camera_id}
            camera={camera}
            tileHeight={tileHeight}
            isMobile={isMobile}
          />
        ))}
        <AddCameraCard onClick={onScanClick} />
      </div>
    </div>
  )
}
```

### 4.4 表示破綻防止

```typescript
// 最小タイル幅を保証
const MIN_TILE_WIDTH = 100 // px

const calculateTileWidth = () => {
  const containerWidth = containerRef.current?.offsetWidth ?? 0
  const rawWidth = (containerWidth - gap * (effectiveCols - 1)) / effectiveCols
  return Math.max(rawWidth, MIN_TILE_WIDTH)
}
```

---

## 5. CameraTile タッチ最適化

### 5.1 変更箇所
`src/components/CameraTile.tsx`

### 5.2 Props追加

```typescript
interface CameraTileProps {
  // 既存props...
  isMobile?: boolean
}
```

### 5.3 タッチ操作対応

```tsx
function CameraTile({ isMobile = false, ...props }: CameraTileProps) {
  // タッチデバイスでのホバー問題対策
  const [isTouched, setIsTouched] = useState(false)

  const handleTouchStart = () => {
    if (isMobile) setIsTouched(true)
  }

  const handleTouchEnd = () => {
    if (isMobile) {
      setTimeout(() => setIsTouched(false), 300)
    }
  }

  return (
    <div
      className={cn(
        "relative rounded-lg overflow-hidden cursor-pointer",
        "transition-all duration-200",
        // PC: hover, モバイル: touch状態
        !isMobile && "hover:ring-2 hover:ring-primary",
        isMobile && isTouched && "ring-2 ring-primary"
      )}
      onTouchStart={handleTouchStart}
      onTouchEnd={handleTouchEnd}
      onClick={onClick}
    >
      {/* 既存コンテンツ */}
    </div>
  )
}
```

### 5.4 フッターテキストサイズ調整

```tsx
// モバイルでは文字を少し大きく
<div className={cn(
  "text-xs",
  isMobile && "text-sm"
)}>
  {camera.name}
</div>
```

---

## 6. FloatingAIButton（新規）

### 6.1 ファイル
`src/components/FloatingAIButton.tsx` (新規作成)

### 6.2 仕様

```typescript
interface FloatingAIButtonProps {
  onClick: () => void
  hasNewEvents?: boolean
  isDrawerOpen?: boolean
}
```

### 6.3 実装

```tsx
import { Bot, X } from 'lucide-react'
import { cn } from '@/lib/utils'

export function FloatingAIButton({
  onClick,
  hasNewEvents = false,
  isDrawerOpen = false
}: FloatingAIButtonProps) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "fixed right-4 bottom-4 z-50",
        "w-14 h-14 rounded-full",
        "bg-primary text-primary-foreground",
        "shadow-lg",
        "flex items-center justify-center",
        "transition-transform duration-200",
        "active:scale-95",
        // パルスアニメーション（新着時）
        hasNewEvents && !isDrawerOpen && "animate-pulse"
      )}
      aria-label={isDrawerOpen ? "ドロワーを閉じる" : "AIイベントログを開く"}
    >
      {isDrawerOpen ? (
        <X className="h-6 w-6" />
      ) : (
        <>
          <Bot className="h-6 w-6" />
          {hasNewEvents && (
            <span className="absolute -top-1 -right-1 w-4 h-4 bg-red-500 rounded-full" />
          )}
        </>
      )}
    </button>
  )
}
```

### 6.4 アクセシビリティ
- `aria-label` で状態を明示
- `active:scale-95` でタップフィードバック
- 最小タップ領域 44x44px 以上（56x56px）

---

## 7. MobileDrawer（新規）

### 7.1 ファイル
`src/components/MobileDrawer.tsx` (新規作成)

### 7.2 仕様

```typescript
interface MobileDrawerProps {
  open: boolean
  onClose: () => void
  children: React.ReactNode
  width?: string // デフォルト "85%"
}
```

### 7.3 実装

```tsx
import { useEffect, useRef } from 'react'
import { X } from 'lucide-react'
import { cn } from '@/lib/utils'

export function MobileDrawer({
  open,
  onClose,
  children,
  width = "85%"
}: MobileDrawerProps) {
  const drawerRef = useRef<HTMLDivElement>(null)

  // ESCキーで閉じる
  useEffect(() => {
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && open) onClose()
    }
    document.addEventListener('keydown', handleEsc)
    return () => document.removeEventListener('keydown', handleEsc)
  }, [open, onClose])

  // 背景クリックで閉じる
  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) onClose()
  }

  // スクロールロック
  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden'
    } else {
      document.body.style.overflow = ''
    }
    return () => { document.body.style.overflow = '' }
  }, [open])

  return (
    <>
      {/* Backdrop */}
      <div
        className={cn(
          "fixed inset-0 z-40 bg-black/50",
          "transition-opacity duration-300",
          open ? "opacity-100" : "opacity-0 pointer-events-none"
        )}
        onClick={handleBackdropClick}
      />

      {/* Drawer */}
      <div
        ref={drawerRef}
        className={cn(
          "fixed top-0 right-0 z-50 h-full bg-card",
          "shadow-xl",
          "transition-transform duration-300 ease-out",
          open ? "translate-x-0" : "translate-x-full"
        )}
        style={{ width }}
      >
        {/* Header */}
        <div className="h-14 flex items-center justify-between px-4 border-b">
          <span className="font-semibold">AI Event Log</span>
          <button
            onClick={onClose}
            className="p-2 rounded-full hover:bg-muted"
            aria-label="閉じる"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Content */}
        <div className="h-[calc(100%-56px)] overflow-hidden">
          {children}
        </div>
      </div>
    </>
  )
}
```

### 7.4 アニメーション仕様

| 状態 | 変換 |
|------|------|
| 閉じた状態 | `translateX(100%)` |
| 開いた状態 | `translateX(0)` |
| 遷移時間 | 300ms |
| イージング | ease-out |

### 7.5 表示破綻防止
- `width: 85%` で画面右端に余白確保（閉じる操作しやすく）
- `h-[calc(100%-56px)]` でヘッダー分を除外
- `overflow-hidden` で内部スクロールを子に委譲

---

## 8. EventLogPane モバイル対応

### 8.1 変更箇所
`src/components/EventLogPane.tsx`

### 8.2 Props追加

```typescript
interface EventLogPaneProps {
  // 既存props...
  isMobile?: boolean
}
```

### 8.3 レイアウト分岐

```tsx
function EventLogPane({ isMobile = false, ...props }: EventLogPaneProps) {
  return (
    <div className={cn(
      "h-full flex flex-col",
      isMobile && "bg-card"
    )}>
      {/* Header - モバイルでは非表示（ドロワーに含む） */}
      {!isMobile && (
        <div className="p-2 border-b flex items-center justify-between">
          <span className="text-sm font-medium">AI Event Log</span>
          <span className="text-xs text-muted-foreground">{logs.length}</span>
        </div>
      )}

      {/* PatrolStatusTicker - 共通 */}
      <PatrolStatusTicker ... />

      {/* Detection Logs - 上半分 */}
      <div className={cn(
        "flex-1 overflow-y-auto",
        isMobile ? "max-h-[40%]" : "h-1/2"
      )}>
        {/* logs */}
      </div>

      {/* Chat UI - 下半分 */}
      <div className={cn(
        "flex-1 flex flex-col border-t",
        isMobile ? "max-h-[60%]" : "h-1/2"
      )}>
        {/* chat */}
      </div>
    </div>
  )
}
```

---

## 9. モーダルモバイル対応

### 9.1 共通変更

各モーダル（Dialog）に以下を追加:

```tsx
// DialogContent に追加
className={cn(
  "max-w-2xl",  // 既存
  "max-w-[95vw]",  // モバイル対応追加
  "max-h-[90vh]",  // 高さ制限追加
  "overflow-y-auto" // 内部スクロール
)}
```

### 9.2 対象モーダル

| モーダル | 変更内容 |
|----------|----------|
| LiveViewModal | `max-w-[95vw]` 追加、動画サイズ調整 |
| CameraDetailModal | `max-w-[95vw]` 追加、フォームレイアウト調整 |
| DetectionDetailModal | `max-w-[95vw]` 追加 |
| ScanModal | `max-w-[95vw]` 追加 |
| SettingsModal | `max-w-[95vw]` 追加、グリッド調整（既存対応あり） |

---

## 10. index.css 追加定義

### 10.1 ブレークポイント

```css
/* モバイル判定 */
@media (max-width: 768px) {
  /* モバイル専用スタイル */

  /* スクロールバー非表示（モバイル） */
  .mobile-scrollbar-hide::-webkit-scrollbar {
    display: none;
  }
  .mobile-scrollbar-hide {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
}
```

### 10.2 アニメーション定義

```css
/* SuggestPane アニメーション */
@keyframes suggest-slide-in-mobile {
  from { transform: translateX(-100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}
@keyframes suggest-slide-out-mobile {
  from { transform: translateY(0); opacity: 1; }
  to { transform: translateY(-100%); opacity: 0; }
}

.suggest-animate-in-mobile {
  animation: suggest-slide-in-mobile 500ms ease-out forwards;
}
.suggest-animate-out-mobile {
  animation: suggest-slide-out-mobile 500ms ease-in forwards;
}

/* パルスアニメーション強化 */
@keyframes pulse-strong {
  0%, 100% { transform: scale(1); box-shadow: 0 0 0 0 rgba(var(--primary), 0.7); }
  50% { transform: scale(1.05); box-shadow: 0 0 0 10px rgba(var(--primary), 0); }
}
.animate-pulse-strong {
  animation: pulse-strong 2s infinite;
}
```

---

## 11. 表示破綻防止チェックリスト

### 11.1 レイアウト破綻

| チェック項目 | 対策 |
|-------------|------|
| 3ペーン崩壊 | isMobile時は縦積みレイアウトに切替 |
| タイル最小幅 | MIN_TILE_WIDTH = 100px 保証 |
| テキストはみ出し | truncate + text-xs/text-sm 調整 |
| モーダルはみ出し | max-w-[95vw] + max-h-[90vh] |
| ドロワー重なり | z-index: backdrop=40, drawer=50 |

### 11.2 操作破綻

| チェック項目 | 対策 |
|-------------|------|
| タップ領域小さい | 最小44x44px保証 |
| ホバー効果残り | タッチ用state分離 |
| スクロール競合 | overflow制御分離 |
| ドロワー閉じにくい | 85%幅 + 背景タップ対応 |

### 11.3 アニメーション破綻

| チェック項目 | 対策 |
|-------------|------|
| ちらつき | will-change + transform使用 |
| 途切れ | forwards指定 |
| 重複実行 | animationState管理 |

---

## 12. MECE確認

本詳細設計は以下を網羅:

| カテゴリ | 項目 | 対応 |
|---------|------|------|
| フック | useMediaQuery | ○ |
| レイアウト | App.tsx分岐 | ○ |
| サジェスト | アニメーション分岐 | ○ |
| グリッド | 3列対応 | ○ |
| タイル | タッチ最適化 | ○ |
| ボタン | FloatingAIButton | ○ |
| ドロワー | MobileDrawer | ○ |
| イベントログ | モバイル対応 | ○ |
| モーダル | 幅/高さ調整 | ○ |
| CSS | ブレークポイント | ○ |

漏れなし。

---

## 13. アンアンビギュアス確認

| 項目 | 定義値 |
|------|--------|
| モバイル判定 | max-width: 768px |
| グリッド列数 | PC: 4列, モバイル: 3列 |
| グリッドgap | PC: 16px, モバイル: 8px |
| タイル最小幅 | 100px |
| SuggestView高さ | 30vh (画面高さの30%) |
| ドロワー幅 | 85% |
| アニメーション時間 | 300ms (ドロワー), 500ms (サジェスト) |
| FloatingButton位置 | right-4 bottom-4 (16px) |
| FloatingButtonサイズ | 56x56px |
| モーダル最大幅 | 95vw |
| モーダル最大高さ | 90vh |
| z-index backdrop | 40 |
| z-index drawer | 50 |
| z-index floating | 50 |

曖昧な点なし。
