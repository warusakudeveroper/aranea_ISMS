# mobes AIcam control Tower レイアウト修正設計書

## 1. 概要

### 1.1 目的
3ペイン構成のレイアウトを仕様に合わせて修正する。

### 1.2 修正対象
| ファイル | 修正内容 |
|----------|----------|
| `App.tsx` | ペイン比率修正、max-w制限撤廃 |
| `CameraGrid.tsx` | スクロール禁止、タイル縮小ロジック |
| `EventLogPane.tsx` | チャットUI追加（スタブ） |

---

## 2. レイアウト修正

### 2.1 ペイン比率

**現在** → **修正後**
```
左:30%(max450px) | 中央:45%(flex) | 右:25%(max360px)
         ↓
左:30%           | 中央:55%       | 右:15%
```

### 2.2 App.tsx 修正箇所

#### Line 239 コメント修正
```tsx
// Before
{/* Main Content - 3 Pane Layout (IS22_UI_DETAILED_SPEC Section 1.1: 30%/45%/25%) */}

// After
{/* Main Content - 3 Pane Layout: 30%/55%/15% */}
```

#### Line 242 左ペイン
```tsx
// Before
<aside className="w-[30%] min-w-[320px] max-w-[450px] border-r bg-card overflow-hidden">

// After
<aside className="w-[30%] border-r bg-card overflow-hidden flex flex-col">
```

#### Line 251 中央ペイン
```tsx
// Before
<main className="w-[45%] flex-1 p-4 overflow-auto">

// After
<main className="w-[55%] p-4 overflow-hidden flex flex-col">
```

#### Line 275 右ペイン
```tsx
// Before
<aside className="w-[25%] min-w-[280px] max-w-[360px] border-l bg-card overflow-hidden">

// After
<aside className="w-[15%] border-l bg-card overflow-hidden flex flex-col">
```

---

## 3. カメラグリッド スクロール禁止対応

### 3.1 要件
- 4列固定
- 最大8行（32タイル = 31カメラ + 1追加カード）
- 画面下端に到達したらタイルを縮小
- スクロールは発生させない
- 文字サイズは維持

### 3.2 実装方針

#### 方針A: CSS Grid + max-height計算（採用）
1. CameraGridに`containerHeight`プロパティを追加
2. 利用可能高さから行数でタイル最大高さを計算
3. タイルにmax-heightを適用、画像部分のみ縮小

#### 計算式
```
availableHeight = containerHeight - padding(32px)
rows = ceil(tileCount / 4)
maxTileHeight = availableHeight / rows - gap(8px)
```

### 3.3 CameraGrid.tsx 修正

#### Props追加
```tsx
interface CameraGridProps {
  // ... existing props
  containerHeight?: number  // 親コンテナの高さ (px)
}
```

#### タイル高さ計算
```tsx
// Calculate max tile height to fit within container without scrolling
const calculateMaxTileHeight = (
  containerHeight: number | undefined,
  tileCount: number
): number | undefined => {
  if (!containerHeight) return undefined

  const rows = Math.ceil(tileCount / COLUMNS)
  const padding = 32 // p-4 = 16px * 2
  const gap = 8 // gap-2 = 8px
  const totalGap = (rows - 1) * gap
  const availableHeight = containerHeight - padding - totalGap

  return Math.floor(availableHeight / rows)
}
```

#### グリッドCSS修正
```tsx
<div
  className="grid grid-cols-4 gap-2"
  style={{
    maxHeight: containerHeight ? `${containerHeight - 32}px` : undefined
  }}
>
```

### 3.4 App.tsx CameraGrid呼び出し修正

コンテナ高さを取得してCameraGridに渡す:
```tsx
// Add ref for measuring container height
const gridContainerRef = useRef<HTMLDivElement>(null)
const [gridContainerHeight, setGridContainerHeight] = useState<number>(0)

// Measure container height
useEffect(() => {
  const updateHeight = () => {
    if (gridContainerRef.current) {
      setGridContainerHeight(gridContainerRef.current.clientHeight)
    }
  }
  updateHeight()
  window.addEventListener('resize', updateHeight)
  return () => window.removeEventListener('resize', updateHeight)
}, [])

// In JSX:
<main
  ref={gridContainerRef}
  className="w-[55%] p-4 overflow-hidden flex flex-col"
>
  <CameraGrid
    // ... existing props
    containerHeight={gridContainerHeight}
  />
</main>
```

### 3.5 CameraTile.tsx 修正

maxHeightを受け取ってタイルサイズを制限:
```tsx
interface CameraTileProps {
  // ... existing props
  maxHeight?: number  // Maximum tile height in pixels
}

// In component:
<Card
  className={cn(...)}
  style={{ maxHeight: maxHeight ? `${maxHeight}px` : undefined }}
>
```

---

## 4. チャットUI追加（スタブ）

### 4.1 要件
- 右ペインの下半分をチャットウインドウに
- 入力欄（自動伸縮テキストエリア）
- 送信ボタン
- mobes2.0連携は後続タスク（今回はスタブ）

### 4.2 EventLogPane.tsx 構造変更

```
現在:
┌─────────────────┐
│ Header          │
├─────────────────┤
│ Patrol Ticker   │
├─────────────────┤
│ Detection Logs  │ ← 全体
│ (scrollable)    │
└─────────────────┘

修正後:
┌─────────────────┐
│ Header          │
├─────────────────┤
│ Patrol Ticker   │
├─────────────────┤
│ Detection Logs  │ ← 上半分 (h-1/2)
│ (scrollable)    │
├─────────────────┤
│ Chat History    │ ← 下半分 (h-1/2)
│ (scrollable)    │
├─────────────────┤
│ [Input      ][⏎]│ ← 入力欄 (flex-shrink-0)
└─────────────────┘
```

### 4.3 ChatInput コンポーネント

```tsx
// components/ChatInput.tsx
interface ChatInputProps {
  onSend: (message: string) => void
  disabled?: boolean
  placeholder?: string
}

function ChatInput({ onSend, disabled, placeholder }: ChatInputProps) {
  const [message, setMessage] = useState('')
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  // Auto-resize textarea
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto'
      textareaRef.current.style.height =
        Math.min(textareaRef.current.scrollHeight, 120) + 'px'
    }
  }, [message])

  const handleSend = () => {
    if (message.trim() && !disabled) {
      onSend(message.trim())
      setMessage('')
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  return (
    <div className="flex items-end gap-2 p-2 border-t">
      <textarea
        ref={textareaRef}
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder || "メッセージを入力..."}
        disabled={disabled}
        rows={1}
        className="flex-1 resize-none rounded-md border px-3 py-2 text-sm
                   focus:outline-none focus:ring-2 focus:ring-primary
                   disabled:opacity-50 min-h-[36px] max-h-[120px]"
      />
      <Button
        size="icon"
        onClick={handleSend}
        disabled={disabled || !message.trim()}
      >
        <Send className="h-4 w-4" />
      </Button>
    </div>
  )
}
```

### 4.4 EventLogPane 修正後構造

```tsx
export function EventLogPane({ cameras, onLogClick }: EventLogPaneProps) {
  // ... existing state

  const handleChatSend = (message: string) => {
    // TODO: mobes2.0 API連携
    console.log('Chat message:', message)
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b flex-shrink-0">
        ...
      </div>

      {/* Patrol Ticker */}
      {latestPatrol && (
        <div className="px-3 py-1.5 border-b flex-shrink-0">
          ...
        </div>
      )}

      {/* Upper Half: Detection Logs */}
      <div className="h-1/2 overflow-auto border-b" ref={scrollRef}>
        <div className="p-2 space-y-1">
          {detectionLogs.map((log) => (
            <DetectionLogItem ... />
          ))}
        </div>
      </div>

      {/* Lower Half: Chat */}
      <div className="h-1/2 flex flex-col">
        {/* Chat History */}
        <div className="flex-1 overflow-auto p-2">
          <div className="text-center text-muted-foreground text-xs py-4">
            AIアシスタントとの会話
          </div>
          {/* TODO: Chat messages will be displayed here */}
        </div>

        {/* Chat Input */}
        <ChatInput
          onSend={handleChatSend}
          placeholder="質問を入力..."
        />
      </div>
    </div>
  )
}
```

---

## 5. 実装順序

1. **App.tsx**: ペイン比率修正 (30%/55%/15%)
2. **App.tsx**: max-w制限撤廃
3. **App.tsx**: overflow-auto → overflow-hidden
4. **App.tsx**: gridContainerRef追加、高さ計測
5. **CameraGrid.tsx**: containerHeightプロパティ追加
6. **CameraGrid.tsx**: タイル高さ計算ロジック追加
7. **CameraTile.tsx**: maxHeightプロパティ追加
8. **ChatInput.tsx**: 新規作成
9. **EventLogPane.tsx**: 上下分割、チャットUI追加

---

## 6. テスト項目

- [ ] FHD(1920x1080)でペイン比率が30%/55%/15%
- [ ] 4K(3840x2160)でペイン比率が30%/55%/15%
- [ ] カメラ17台以上でもスクロールなし
- [ ] カメラ31台でタイルが縮小されて収まる
- [ ] チャット入力欄が表示される
- [ ] 入力欄が文字数で自動伸縮

---

*Created: 2026-01-03*
