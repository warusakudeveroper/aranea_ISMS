import { useState, useEffect } from "react"
import type { Camera } from "@/types/api"
import { Card, CardContent } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Camera as CameraIcon, XCircle, Settings, Globe, Clock } from "lucide-react"
import { cn } from "@/lib/utils"

// Format timestamp as MM/DD HH:mm:ss (compact, no year)
function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp)
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  const hours = String(date.getHours()).padStart(2, '0')
  const minutes = String(date.getMinutes()).padStart(2, '0')
  const seconds = String(date.getSeconds()).padStart(2, '0')
  return `${month}/${day} ${hours}:${minutes}:${seconds}`
}

// Animation styles
type AnimationStyle = "slide-down" | "fade" | "none"

// Image display mode: fit (contain, letterbox) or trim (cover, crop)
type ImageMode = "fit" | "trim"

// Display mode: footer (separate info section) or overlay (info on image)
type DisplayMode = "footer" | "overlay"

// Aspect ratio: square (1:1), 4/3, or video (16:9)
export type AspectRatio = "square" | "4/3" | "video"

interface CameraTileProps {
  camera: Camera
  snapshotUrl: string
  // Unix timestamp (ms) of last snapshot update
  lastSnapshotAt?: number
  isOffline?: boolean
  severity?: number
  primaryEvent?: string
  onClick?: () => void
  onSettingsClick?: () => void
  animationEnabled?: boolean
  animationStyle?: AnimationStyle
  // Display settings
  imageMode?: ImageMode
  displayMode?: DisplayMode
  aspectRatio?: AspectRatio
}

// Get Tailwind aspect ratio class
function getAspectClass(ratio: AspectRatio): string {
  switch (ratio) {
    case "square": return "aspect-square"
    case "4/3": return "aspect-[4/3]"
    case "video": return "aspect-video"
    default: return "aspect-square"
  }
}

export function CameraTile({
  camera,
  snapshotUrl,
  lastSnapshotAt,
  isOffline = false,
  severity = 0,
  primaryEvent,
  onClick,
  onSettingsClick,
  animationEnabled = true,
  animationStyle = "slide-down",
  imageMode = "trim",
  displayMode = "overlay",
  aspectRatio = "square",
}: CameraTileProps) {
  // Track current and next snapshot URLs for animation
  const [currentUrl, setCurrentUrl] = useState(snapshotUrl)
  const [nextUrl, setNextUrl] = useState<string | null>(null)
  const [animating, setAnimating] = useState(false)
  const [imageError, setImageError] = useState(false)

  // Handle snapshot URL changes with animation
  useEffect(() => {
    // Extract base URL without timestamp for comparison
    const getBaseUrl = (url: string) => url.split('?')[0]
    const currentBase = getBaseUrl(currentUrl)
    const newBase = getBaseUrl(snapshotUrl)

    // Only animate if it's actually a new image (same camera, different timestamp)
    if (currentBase === newBase && snapshotUrl !== currentUrl) {
      if (animationEnabled && animationStyle !== "none") {
        // Start animation
        setNextUrl(snapshotUrl)
        setAnimating(true)
        setImageError(false)

        // After animation completes, swap images
        const timer = setTimeout(() => {
          setCurrentUrl(snapshotUrl)
          setNextUrl(null)
          setAnimating(false)
        }, animationStyle === "slide-down" ? 500 : 300)

        return () => clearTimeout(timer)
      } else {
        // No animation, just swap
        setCurrentUrl(snapshotUrl)
        setImageError(false)
      }
    } else if (currentBase !== newBase) {
      // Different camera - just swap without animation
      setCurrentUrl(snapshotUrl)
      setImageError(false)
    }
  }, [snapshotUrl, currentUrl, animationEnabled, animationStyle])

  const severityVariant = severity === 0 ? "severity0"
    : severity === 1 ? "severity1"
    : severity === 2 ? "severity2"
    : "severity3"

  const handleSettingsClick = (e: React.MouseEvent) => {
    e.stopPropagation()
    onSettingsClick?.()
  }

  const animationClass = animationStyle === "slide-down"
    ? "animate-slide-down"
    : animationStyle === "fade"
    ? "animate-fade-replace"
    : ""

  // CSS class for image fit mode
  const imageObjectClass = imageMode === "fit" ? "object-contain" : "object-cover"

  // CSS class for aspect ratio
  const aspectClass = getAspectClass(aspectRatio)

  // Overlay mode: image fills entire card, info overlaid with shadow
  if (displayMode === "overlay") {
    return (
      <Card
        className={cn(
          "group cursor-pointer transition-all hover:shadow-lg overflow-hidden",
          isOffline && "opacity-60",
          severity >= 2 && "ring-2 ring-yellow-500",
          severity >= 3 && "ring-2 ring-red-500"
        )}
        onClick={onClick}
      >
        <div className={cn("relative bg-muted overflow-hidden", aspectClass)}>
          {isOffline || imageError ? (
            <div className="absolute inset-0 flex items-center justify-center">
              <XCircle className="h-12 w-12 text-muted-foreground" />
            </div>
          ) : (
            <>
              {/* Current image */}
              <img
                src={currentUrl}
                alt={camera.name}
                className={cn("h-full w-full", imageObjectClass)}
                loading="lazy"
                onError={() => setImageError(true)}
              />
              {/* Animated next image (slides over current) */}
              {animating && nextUrl && (
                <img
                  src={nextUrl}
                  alt={camera.name}
                  className={cn(
                    "absolute inset-0 h-full w-full",
                    imageObjectClass,
                    animationClass
                  )}
                  onError={() => setImageError(true)}
                />
              )}
            </>
          )}

          {/* Settings icon - top right (always with shadow bg) */}
          <button
            onClick={handleSettingsClick}
            className="absolute top-1 right-1 p-1.5 rounded-full bg-black/50 hover:bg-black/70 text-white opacity-0 group-hover:opacity-100 transition-opacity"
            title="カメラ設定"
          >
            <Settings className="h-4 w-4" />
          </button>

          {/* Primary event badge */}
          {primaryEvent && (
            <Badge
              variant={severityVariant}
              className="absolute top-1 left-1"
            >
              {primaryEvent}
            </Badge>
          )}

          {/* Overlay info - bottom gradient with text */}
          <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 via-black/50 to-transparent p-2 pt-6">
            {/* Camera name */}
            <div className="flex items-center gap-1.5 text-white drop-shadow-[0_1px_2px_rgba(0,0,0,0.8)]">
              <CameraIcon className="h-3.5 w-3.5 flex-shrink-0" />
              <span className="text-sm font-medium truncate">
                {camera.name}
              </span>
            </div>
            {/* IP + FID + Timestamp row */}
            <div className="flex items-center justify-between text-[10px] text-white/80 mt-0.5 drop-shadow-[0_1px_2px_rgba(0,0,0,0.8)]">
              <div className="flex items-center gap-1 truncate">
                <Globe className="h-2.5 w-2.5 flex-shrink-0" />
                <span className="truncate">{camera.ip_address || '-'}</span>
                {camera.fid && (
                  <span className="text-white/60 ml-1">FID:{camera.fid}</span>
                )}
              </div>
              <div className="flex items-center gap-0.5 flex-shrink-0 ml-2">
                <Clock className="h-2.5 w-2.5" />
                <span>{lastSnapshotAt ? formatTimestamp(lastSnapshotAt) : '--:--:--'}</span>
              </div>
            </div>
          </div>
        </div>
      </Card>
    )
  }

  // Footer mode: separate info section below image (original layout)
  return (
    <Card
      className={cn(
        "group cursor-pointer transition-all hover:shadow-lg overflow-hidden",
        isOffline && "opacity-60",
        severity >= 2 && "ring-2 ring-yellow-500",
        severity >= 3 && "ring-2 ring-red-500"
      )}
      onClick={onClick}
    >
      <div className={cn("relative bg-muted overflow-hidden", aspectClass)}>
        {isOffline || imageError ? (
          <div className="absolute inset-0 flex items-center justify-center">
            <XCircle className="h-12 w-12 text-muted-foreground" />
          </div>
        ) : (
          <>
            {/* Current image */}
            <img
              src={currentUrl}
              alt={camera.name}
              className={cn("h-full w-full", imageObjectClass)}
              loading="lazy"
              onError={() => setImageError(true)}
            />
            {/* Animated next image (slides over current) */}
            {animating && nextUrl && (
              <img
                src={nextUrl}
                alt={camera.name}
                className={cn(
                  "absolute inset-0 h-full w-full",
                  imageObjectClass,
                  animationClass
                )}
                onError={() => setImageError(true)}
              />
            )}
          </>
        )}
        {/* Settings icon - top right */}
        <button
          onClick={handleSettingsClick}
          className="absolute top-1 right-1 p-1.5 rounded-full bg-black/50 hover:bg-black/70 text-white opacity-0 group-hover:opacity-100 transition-opacity"
          title="カメラ設定"
        >
          <Settings className="h-4 w-4" />
        </button>
        {primaryEvent && (
          <Badge
            variant={severityVariant}
            className="absolute bottom-2 left-2"
          >
            {primaryEvent}
          </Badge>
        )}
      </div>
      <CardContent className="p-2 space-y-1">
        {/* Camera name */}
        <div className="flex items-center gap-2">
          <CameraIcon className="h-4 w-4 flex-shrink-0 text-muted-foreground" />
          <span className="text-sm font-medium truncate">
            {camera.name}
          </span>
        </div>
        {/* IP address + model */}
        <div className="flex items-center gap-1 text-xs text-muted-foreground pl-6">
          <Globe className="h-3 w-3 flex-shrink-0" />
          <span className="truncate">
            {camera.ip_address || '-'}
            {camera.model && (
              <span className="text-muted-foreground/70"> · {camera.model}</span>
            )}
          </span>
        </div>
        {/* FID + Last snapshot time */}
        <div className="flex items-center justify-between text-xs text-muted-foreground pl-6">
          {camera.fid && (
            <Badge variant="outline" className="text-[10px] px-1 h-4">
              FID:{camera.fid}
            </Badge>
          )}
          <div className="flex items-center gap-1 ml-auto">
            <Clock className="h-3 w-3" />
            <span>{lastSnapshotAt ? formatTimestamp(lastSnapshotAt) : '--/-- --:--:--'}</span>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
