import { useState, useEffect, useMemo, useCallback, useRef } from "react"
import type { Camera } from "@/types/api"
import { CameraTile, type AspectRatio } from "./CameraTile"
import { cn } from "@/lib/utils"
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
const COLUMNS = 4
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
  // Container height for no-scroll layout (tiles compress to fit)
  containerHeight?: number
  // On-air camera IDs for yellow highlight
  onAirCameraIds?: string[]
}

// Calculate aspect ratio based on tile count
// - ≤16 tiles (4 rows): square (1:1)
// - 17-24 tiles (5-6 rows): 4:3
// - 25-32 tiles (7-8 rows): video (16:9)
function calculateAspectRatio(tileCount: number): AspectRatio {
  const rows = Math.ceil(tileCount / COLUMNS)
  if (rows <= 4) return "square"
  if (rows <= 6) return "4/3"
  return "video"
}

// Get Tailwind aspect ratio class for AddCameraCard
function getAspectClass(ratio: AspectRatio): string {
  switch (ratio) {
    case "square": return "aspect-square"
    case "4/3": return "aspect-[4/3]"
    case "video": return "aspect-video"
    default: return "aspect-square"
  }
}

// Threshold for slow camera warning (10 seconds)
const SLOW_CAMERA_THRESHOLD_MS = 10000

// Calculate max tile height to fit within container without scrolling
function calculateMaxTileHeight(
  containerHeight: number | undefined,
  tileCount: number
): number | undefined {
  if (!containerHeight || containerHeight <= 0) return undefined

  const rows = Math.ceil(tileCount / COLUMNS)
  if (rows <= 0) return undefined

  const padding = 0 // padding is on parent container
  const gap = 8 // gap-2 = 8px
  const totalGap = (rows - 1) * gap
  const availableHeight = containerHeight - padding - totalGap

  return Math.floor(availableHeight / rows)
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
  aspectRatio?: AspectRatio
  isSlowCamera?: boolean
  isTimeout?: boolean
  errorMessage?: string
  maxHeight?: number
  isOnAir?: boolean
  isDragging?: boolean
}

function SortableCameraTile({
  camera,
  isDragging,
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

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isSortableDragging ? 0.5 : 1,
    zIndex: isSortableDragging ? 50 : undefined,
  }

  return (
    <div ref={setNodeRef} style={style} className="relative group/sortable h-full">
      {/* Drag handle - visible on hover */}
      <div
        {...attributes}
        {...listeners}
        className="absolute top-1 left-1 z-10 p-1 rounded bg-black/50 cursor-grab active:cursor-grabbing opacity-0 group-hover/sortable:opacity-100 transition-opacity"
        title="ドラッグで並べ替え"
      >
        <GripVertical className="h-4 w-4 text-white" />
      </div>
      <CameraTile camera={camera} {...props} />
    </div>
  )
}

// ============================================================================
// Add Camera Card (末尾追加カード)
// ============================================================================

function AddCameraCard({
  onClick,
  aspectRatio,
  maxHeight,
}: {
  onClick?: () => void
  aspectRatio: AspectRatio
  maxHeight?: number
}) {
  return (
    <Card
      className={cn(
        "cursor-pointer border-2 border-dashed border-muted-foreground/30 hover:border-primary/50 hover:bg-accent/30 transition-all flex flex-col items-center justify-center gap-2",
        getAspectClass(aspectRatio)
      )}
      style={{ maxHeight: maxHeight ? `${maxHeight}px` : undefined }}
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
  onAirCameraIds = [],
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

  // Calculate total tiles (cameras + add card) and determine aspect ratio
  // Limit to MAX_CAMERAS
  const displayedCameras = useMemo(() =>
    orderedCameras.slice(0, MAX_CAMERAS),
    [orderedCameras]
  )
  const tileCount = displayedCameras.length + 1 // +1 for add card
  const aspectRatio = calculateAspectRatio(tileCount)

  // Calculate max tile height to fit within container without scrolling
  const maxTileHeight = calculateMaxTileHeight(containerHeight, tileCount)

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
        <div className="grid grid-cols-4 gap-2 h-full">
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
                aspectRatio={aspectRatio}
                isSlowCamera={isSlowCamera}
                isTimeout={isTimeout}
                errorMessage={status?.error}
                maxHeight={maxTileHeight}
                isOnAir={isOnAir}
              />
            )
          })}
          {/* 末尾に追加カード (IS22_UI_DETAILED_SPEC Section 2.2 追加要件) */}
          {/* AddCameraCard is not draggable, always at the end */}
          <AddCameraCard onClick={onAddClick} aspectRatio={aspectRatio} maxHeight={maxTileHeight} />
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
              aspectRatio={aspectRatio}
              maxHeight={maxTileHeight}
            />
          </div>
        )}
      </DragOverlay>
    </DndContext>
  )
}
