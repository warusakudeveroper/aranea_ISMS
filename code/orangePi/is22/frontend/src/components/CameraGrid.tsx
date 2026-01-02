import { useState, useEffect } from "react"
import type { Camera } from "@/types/api"
import { CameraTile, type AspectRatio } from "./CameraTile"
import { cn } from "@/lib/utils"
import { API_BASE_URL } from "@/lib/config"
import { Card } from "@/components/ui/card"
import { Search, PlusCircle } from "lucide-react"

// Animation style type
type AnimationStyle = "slide-down" | "fade" | "none"

// Image display mode: fit (contain, letterbox) or trim (cover, crop)
type ImageMode = "fit" | "trim"

// Display mode: footer (separate info section) or overlay (info on image)
type DisplayMode = "footer" | "overlay"

// Constants
const COLUMNS = 4
const MAX_CAMERAS = 31 // 31 cameras + 1 add card = 32 tiles = 8 rows

interface CameraGridProps {
  cameras: Camera[]
  onCameraClick?: (camera: Camera) => void
  onSettingsClick?: (camera: Camera) => void
  onAddClick?: () => void
  // Individual camera timestamps from WebSocket notifications
  // Key: camera_id, Value: Unix timestamp (ms)
  snapshotTimestamps?: Record<string, number>
  // Fallback polling settings (used if WebSocket is disconnected)
  fallbackPollingEnabled?: boolean
  fallbackPollingIntervalMs?: number
  animationEnabled?: boolean
  animationStyle?: AnimationStyle
  // Display settings
  imageMode?: ImageMode
  displayMode?: DisplayMode
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

// 末尾追加カード（BTN-003相当）
function AddCameraCard({ onClick, aspectRatio }: { onClick?: () => void; aspectRatio: AspectRatio }) {
  return (
    <Card
      className={cn(
        "cursor-pointer border-2 border-dashed border-muted-foreground/30 hover:border-primary/50 hover:bg-accent/30 transition-all flex flex-col items-center justify-center gap-2",
        getAspectClass(aspectRatio)
      )}
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

export function CameraGrid({
  cameras,
  onCameraClick,
  onSettingsClick,
  onAddClick,
  snapshotTimestamps = {},
  fallbackPollingEnabled = false,
  fallbackPollingIntervalMs = 30000,
  animationEnabled = true,
  animationStyle = "slide-down",
  imageMode = "trim",
  displayMode = "overlay",
}: CameraGridProps) {
  // Fallback timestamp for cameras without WebSocket updates
  const [fallbackTimestamp, setFallbackTimestamp] = useState<number>(Date.now())

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

  // Calculate total tiles (cameras + add card) and determine aspect ratio
  // Limit to MAX_CAMERAS
  const displayedCameras = cameras.slice(0, MAX_CAMERAS)
  const tileCount = displayedCameras.length + 1 // +1 for add card
  const aspectRatio = calculateAspectRatio(tileCount)

  return (
    <div className="grid grid-cols-4 gap-2">
      {displayedCameras.map((camera) => {
        const timestamp = getTimestamp(camera.camera_id)
        return (
          <CameraTile
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
          />
        )
      })}
      {/* 末尾に追加カード (IS22_UI_DETAILED_SPEC Section 2.2 追加要件) */}
      <AddCameraCard onClick={onAddClick} aspectRatio={aspectRatio} />
    </div>
  )
}
