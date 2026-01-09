import { useState, useEffect, useMemo, useCallback, useRef } from "react"
import type { Camera } from "@/types/api"
import { CameraTile } from "./CameraTile"
import { API_BASE_URL } from "@/lib/config"
import { Card } from "@/components/ui/card"
import { Search, PlusCircle, GripVertical } from "lucide-react"

// Drag and drop imports
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
  type DragCancelEvent,
  DragOverlay,
} from "@dnd-kit/core"
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  rectSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable"
import { CSS } from "@dnd-kit/utilities"

// Animation style type
type AnimationStyle = "slide-down" | "fade" | "none"

// Image display mode: fit (contain, letterbox) or trim (cover, crop)
type ImageMode = "fit" | "trim"

// Display mode: footer (separate info section) or overlay (info on image)
type DisplayMode = "footer" | "overlay"

// Constants
const DEFAULT_COLUMNS = 4
const MAX_CAMERAS = 31 // 31 cameras + 1 add card = 32 tiles = 8 rows

// Camera status from WebSocket (processing time, errors)
export interface CameraStatus {
  /** Last processing time in milliseconds */
  processingMs?: number
  /** Error message (e.g., "timeout") */
  error?: string
}

interface CameraGridProps {
  cameras: Camera[]
  onCameraClick?: (camera: Camera) => void
  onSettingsClick?: (camera: Camera) => void
  onAddClick?: () => void
  /** Callback when camera order changes (for parent to update state) */
  onReorder?: (reorderedCameras: Camera[]) => void
  // Individual camera timestamps from WebSocket notifications
  // Key: camera_id, Value: Unix timestamp (ms)
  snapshotTimestamps?: Record<string, number>
  // Individual camera status from WebSocket (processing time, errors)
  // Key: camera_id, Value: CameraStatus
  cameraStatuses?: Record<string, CameraStatus>
  // Fallback polling settings (used if WebSocket is disconnected)
  fallbackPollingEnabled?: boolean
  fallbackPollingIntervalMs?: number
  animationEnabled?: boolean
  animationStyle?: AnimationStyle
  // Display settings
  imageMode?: ImageMode
  displayMode?: DisplayMode
  // Container dimensions for no-scroll layout (tiles compress to fit)
  containerHeight?: number
  containerWidth?: number
  // On-air camera IDs for yellow highlight
  onAirCameraIds?: string[]
  // Issue #108: モバイル対応
  /** モバイル表示モード */
  isMobile?: boolean
  /** グリッド列数 (デフォルト4、モバイル時3) */
  columns?: number
  /** 縦スクロール許容 (デフォルトfalse、モバイル時true) */
  allowScroll?: boolean
}

// タイルは常に1:1（正方形）デフォルト
// 画面下端に到達した場合のみmaxHeightで縦幅を調整

// Threshold for slow camera warning (10 seconds)
const SLOW_CAMERA_THRESHOLD_MS = 10000

// Calculate tile height to fit within container
// 左右・上・下のマージンを同一にする（p-4 = 16px）
//
// 仕様 (UI_Review1.md セクション1.3.2):
// | 条件 | 縦横比 | 動作 |
// | デフォルト | 1:1（正方形） | 上から順に配置 |
// | 画面下端に未到達 | 1:1（正方形） | 正方形タイルで配置 |
// | 画面下端に到達 | 縦幅のみ調整 | 全タイルの縦幅を動的に縮小し画面内に収める |
//
// 重要: 「縦幅のみ調整」= 縮小のみ。縦幅の拡大は仕様外。
// タイル高さは最大でもタイル幅と同じ（正方形が上限）
//
// Issue #108: columns パラメータ追加でモバイル3列対応
function calculateTileHeight(
  containerHeight: number | undefined,
  tileCount: number,
  containerWidth: number | undefined,
  columns: number = DEFAULT_COLUMNS,
  allowScroll: boolean = false
): { tileHeight: number | undefined; isCompressed: boolean } {
  // モバイルでスクロール許容の場合、縦幅計算をスキップして正方形固定
  if (allowScroll) {
    if (!containerWidth || containerWidth <= 0) {
      return { tileHeight: undefined, isCompressed: false }
    }
    const padding = 8 // モバイル時はp-2 = 8px
    const gap = 8 // gap-2 = 8px
    const tileWidth = Math.floor((containerWidth - (padding * 2) - (gap * (columns - 1))) / columns)
    return { tileHeight: tileWidth, isCompressed: false }
  }

  if (!containerHeight || containerHeight <= 0 || !containerWidth || containerWidth <= 0) {
    return { tileHeight: undefined, isCompressed: false }
  }

  const rows = Math.ceil(tileCount / columns)
  if (rows <= 0) return { tileHeight: undefined, isCompressed: false }

  const padding = 16 // p-4 = 16px (top and bottom)
  const gap = 8 // gap-2 = 8px
  const totalRowGap = (rows - 1) * gap
  const availableHeight = containerHeight - (padding * 2) - totalRowGap

  // タイル幅を計算（動的列数）
  const tileWidth = Math.floor((containerWidth - (padding * 2) - (gap * (columns - 1))) / columns)

  // 正方形で配置した場合の必要高さ
  const squareTotalHeight = tileWidth * rows

  if (squareTotalHeight > availableHeight) {
    // 画面下端に到達: 縦幅のみ縮小
    const compressedHeight = Math.floor(availableHeight / rows)
    return {
      tileHeight: compressedHeight,
      isCompressed: true
    }
  }

  // 画面下端に未到達: 正方形で配置（タイル高さ = タイル幅）
  return {
    tileHeight: tileWidth,
    isCompressed: false
  }
}

// ============================================================================
// Sortable Camera Tile Wrapper
// ============================================================================

interface SortableCameraTileProps {
  camera: Camera
  snapshotUrl: string
  lastSnapshotAt?: number
  onClick?: () => void
  onSettingsClick?: () => void
  animationEnabled?: boolean
  animationStyle?: AnimationStyle
  imageMode?: ImageMode
  displayMode?: DisplayMode
  isSlowCamera?: boolean
  isTimeout?: boolean
  errorMessage?: string
  /** Tile height in pixels (calculated by grid) */
  tileHeight?: number
  isOnAir?: boolean
  isDragging?: boolean
  /** Issue #108: モバイル表示モード */
  isMobile?: boolean
}

function SortableCameraTile({
  camera,
  isDragging,
  isMobile,
  ...props
}: SortableCameraTileProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging: isSortableDragging,
  } = useSortable({ id: camera.camera_id })

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isSortableDragging ? 0.5 : 1,
    zIndex: isSortableDragging ? 50 : undefined,
    // Issue #108: モバイル時は確実に正方形を維持
    ...(isMobile ? { aspectRatio: '1 / 1' } : {}),
  }

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`relative group/sortable ${isMobile ? '' : 'h-full'}`}
    >
      {/* Drag handle - visible on hover (モバイルでは非表示) */}
      {!isMobile && (
        <div
          {...attributes}
          {...listeners}
          className="absolute top-1 left-1 z-10 p-1 rounded bg-black/50 cursor-grab active:cursor-grabbing opacity-0 group-hover/sortable:opacity-100 transition-opacity"
          title="ドラッグで並べ替え"
        >
          <GripVertical className="h-4 w-4 text-white" />
        </div>
      )}
      <CameraTile camera={camera} isMobile={isMobile} {...props} />
    </div>
  )
}

// ============================================================================
// Add Camera Card (末尾追加カード)
// ============================================================================

function AddCameraCard({
  onClick,
}: {
  onClick?: () => void
}) {
  return (
    <Card
      className="cursor-pointer border-2 border-dashed border-muted-foreground/30 hover:border-primary/50 hover:bg-accent/30 transition-all flex flex-col items-center justify-center gap-2 h-full"
      onClick={onClick}
    >
      <div className="flex h-10 w-10 items-center justify-center rounded-full bg-muted">
        <PlusCircle className="h-5 w-5 text-muted-foreground" />
      </div>
      <div className="flex items-center gap-1 text-sm text-muted-foreground">
        <Search className="h-4 w-4" />
        <span>カメラをスキャン</span>
      </div>
    </Card>
  )
}

// ============================================================================
// API: Update camera sort_order
// ============================================================================

async function updateCameraSortOrder(cameraId: string, sortOrder: number): Promise<boolean> {
  try {
    const response = await fetch(`${API_BASE_URL}/api/cameras/${cameraId}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ sort_order: sortOrder }),
    })
    return response.ok
  } catch (error) {
    console.error("[CameraGrid] Failed to update sort_order:", cameraId, error)
    return false
  }
}

// Batch update sort_order for multiple cameras
async function batchUpdateSortOrder(cameras: Camera[]): Promise<void> {
  // Update all cameras with new sort_order values (1, 2, 3, ...)
  const updates = cameras.map((camera, index) =>
    updateCameraSortOrder(camera.camera_id, index + 1)
  )
  await Promise.all(updates)
}

// ============================================================================
// Camera Grid Component
// ============================================================================

export function CameraGrid({
  cameras,
  onCameraClick,
  onSettingsClick,
  onAddClick,
  onReorder,
  snapshotTimestamps = {},
  cameraStatuses = {},
  fallbackPollingEnabled = false,
  fallbackPollingIntervalMs = 30000,
  animationEnabled = true,
  animationStyle = "slide-down",
  imageMode = "trim",
  displayMode = "overlay",
  containerHeight,
  containerWidth,
  onAirCameraIds = [],
  // Issue #108: モバイル対応
  isMobile = false,
  columns = DEFAULT_COLUMNS,
  allowScroll = false,
}: CameraGridProps) {
  // Fallback timestamp for cameras without WebSocket updates
  const [fallbackTimestamp, setFallbackTimestamp] = useState<number>(Date.now())

  // Currently dragging camera ID
  const [activeId, setActiveId] = useState<string | null>(null)
  const isDraggingRef = useRef(false)

  // Local camera order (sorted by sort_order initially)
  const [orderedCameras, setOrderedCameras] = useState<Camera[]>([])

  // Track if we've made local reorder changes (to prevent prop sync overwriting them)
  const localChangeTimeRef = useRef<number>(0)

  // Sort cameras by sort_order on initial load and when cameras prop changes
  useEffect(() => {
    // Skip update if dragging to prevent collision detection issues
    if (isDraggingRef.current) return

    // Skip if we just made local changes (debounce 2 seconds to let backend sync)
    const timeSinceChange = Date.now() - localChangeTimeRef.current
    if (timeSinceChange < 2000) return

    const sorted = [...cameras].sort((a, b) => a.sort_order - b.sort_order)
    setOrderedCameras(sorted)
  }, [cameras])

  // Fallback polling - only used if WebSocket is disconnected (longer interval)
  useEffect(() => {
    if (!fallbackPollingEnabled || cameras.length === 0) return

    const interval = setInterval(() => {
      setFallbackTimestamp(Date.now())
    }, fallbackPollingIntervalMs)

    return () => clearInterval(interval)
  }, [fallbackPollingEnabled, fallbackPollingIntervalMs, cameras.length])

  // Get timestamp for a specific camera (prefer WebSocket timestamp, fallback to global)
  const getTimestamp = (cameraId: string): number => {
    return snapshotTimestamps[cameraId] ?? fallbackTimestamp
  }

  // DnD sensors
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8, // Start drag after 8px movement
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  )

  // Handle drag start
  const handleDragStart = useCallback((event: DragStartEvent) => {
    isDraggingRef.current = true
    setActiveId(event.active.id as string)
  }, [])

  // Handle drag cancel - ensure state is reset
  const handleDragCancel = useCallback((_event: DragCancelEvent) => {
    console.log("[CameraGrid] Drag cancelled")
    isDraggingRef.current = false
    setActiveId(null)
  }, [])

  // Handle drag end - use functional setState to prevent stale state issues
  const handleDragEnd = useCallback((event: DragEndEvent) => {
    const { active, over } = event
    isDraggingRef.current = false
    setActiveId(null)

    // Early exit if no valid drop target or dropped on same position
    if (!over || active.id === over.id) {
      console.log("[CameraGrid] Drag cancelled - no valid drop target or same position")
      return
    }

    // Use functional update to ensure we have the latest state
    setOrderedCameras(prev => {
      const oldIndex = prev.findIndex(c => c.camera_id === active.id)
      const newIndex = prev.findIndex(c => c.camera_id === over.id)

      // Validate indices
      if (oldIndex === -1 || newIndex === -1) {
        console.warn("[CameraGrid] Invalid drag indices:", { oldIndex, newIndex, activeId: active.id, overId: over.id })
        return prev // Return unchanged state
      }

      // Same position, no change needed
      if (oldIndex === newIndex) {
        return prev
      }

      // Reorder
      const newOrder = arrayMove(prev, oldIndex, newIndex)
      console.log("[CameraGrid] Reordered:", { from: oldIndex, to: newIndex })

      // Mark the time of local change to prevent useEffect from overwriting
      localChangeTimeRef.current = Date.now()

      // Notify parent (async, don't block state update)
      setTimeout(() => onReorder?.(newOrder), 0)

      // Update sort_order in backend (async, don't block UI)
      batchUpdateSortOrder(newOrder).catch(err => {
        console.error("[CameraGrid] Failed to save sort order:", err)
      })

      return newOrder
    })
  }, [onReorder])

  // Calculate total tiles (cameras + add card)
  // Limit to MAX_CAMERAS
  const displayedCameras = useMemo(() =>
    orderedCameras.slice(0, MAX_CAMERAS),
    [orderedCameras]
  )
  const tileCount = displayedCameras.length + 1 // +1 for add card

  // Calculate tile height for grid layout
  // 仕様: デフォルト正方形、画面下端到達時のみ縦幅縮小
  // Issue #108: モバイル時はallowScroll=trueで常に正方形、スクロール許容
  const { tileHeight } = calculateTileHeight(containerHeight, tileCount, containerWidth, columns, allowScroll)

  // Camera IDs for SortableContext
  const cameraIds = useMemo(() =>
    displayedCameras.map(c => c.camera_id),
    [displayedCameras]
  )

  // Active camera for DragOverlay
  const activeCamera = useMemo(() =>
    activeId ? displayedCameras.find(c => c.camera_id === activeId) : null,
    [activeId, displayedCameras]
  )

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      onDragCancel={handleDragCancel}
    >
      <SortableContext items={cameraIds} strategy={rectSortingStrategy}>
        {/* tileHeightが指定されている場合、gridAutoRowsで行高を固定 */}
        {/* これにより正方形モードでも圧縮モードでも確実にタイル高さを制御 */}
        {/* Issue #108: 動的列数対応 (PC:4列, モバイル:3列) */}
        {/* Issue #108: モバイル時はaspectRatioでタイルサイズを制御するためgridAutoRowsは使わない */}
        <div
          className="grid gap-2"
          style={{
            gridTemplateColumns: `repeat(${columns}, 1fr)`,
            ...(!isMobile && tileHeight ? { gridAutoRows: `${tileHeight}px` } : {})
          }}
        >
          {displayedCameras.map((camera) => {
            const timestamp = getTimestamp(camera.camera_id)
            const status = cameraStatuses[camera.camera_id]
            const isSlowCamera = status?.processingMs !== undefined && status.processingMs >= SLOW_CAMERA_THRESHOLD_MS && !status.error
            // Show timeout placeholder for any error (timeout, no route, connection refused, etc.)
            const isTimeout = !!status?.error
            const isOnAir = onAirCameraIds.includes(camera.camera_id)
            return (
              <SortableCameraTile
                key={camera.camera_id}
                camera={camera}
                // Use cached snapshot endpoint with per-camera timestamp for cache busting
                snapshotUrl={`${API_BASE_URL}/api/snapshots/${camera.camera_id}/latest.jpg?t=${timestamp}`}
                lastSnapshotAt={snapshotTimestamps[camera.camera_id]}
                onClick={() => onCameraClick?.(camera)}
                onSettingsClick={() => onSettingsClick?.(camera)}
                animationEnabled={animationEnabled}
                animationStyle={animationStyle}
                imageMode={imageMode}
                displayMode={displayMode}
                isSlowCamera={isSlowCamera}
                isTimeout={isTimeout}
                errorMessage={status?.error}
                tileHeight={tileHeight}
                isOnAir={isOnAir}
                isMobile={isMobile}
              />
            )
          })}
          {/* 末尾に追加カード (IS22_UI_DETAILED_SPEC Section 2.2 追加要件) */}
          {/* AddCameraCard is not draggable, always at the end */}
          <AddCameraCard onClick={onAddClick} />
        </div>
      </SortableContext>

      {/* Drag overlay - shows the camera tile being dragged */}
      <DragOverlay>
        {activeCamera && (
          <div className="opacity-90 shadow-2xl ring-2 ring-primary rounded-lg">
            <CameraTile
              camera={activeCamera}
              snapshotUrl={`${API_BASE_URL}/api/snapshots/${activeCamera.camera_id}/latest.jpg?t=${getTimestamp(activeCamera.camera_id)}`}
              lastSnapshotAt={snapshotTimestamps[activeCamera.camera_id]}
              imageMode={imageMode}
              displayMode={displayMode}
              tileHeight={tileHeight}
            />
          </div>
        )}
      </DragOverlay>
    </DndContext>
  )
}
