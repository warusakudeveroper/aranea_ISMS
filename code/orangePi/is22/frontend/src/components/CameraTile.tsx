import type { Camera } from "@/types/api"
import { Card, CardContent } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Camera as CameraIcon, XCircle, Settings, Globe, Clock } from "lucide-react"
import { cn } from "@/lib/utils"

// Format relative time (e.g., "2分前", "3時間前")
function formatRelativeTime(dateStr: string): string {
  const date = new Date(dateStr)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffSec = Math.floor(diffMs / 1000)

  if (diffSec < 60) return `${diffSec}秒前`
  const diffMin = Math.floor(diffSec / 60)
  if (diffMin < 60) return `${diffMin}分前`
  const diffHour = Math.floor(diffMin / 60)
  if (diffHour < 24) return `${diffHour}時間前`
  const diffDay = Math.floor(diffHour / 24)
  return `${diffDay}日前`
}

interface CameraTileProps {
  camera: Camera
  snapshotUrl: string
  isOffline?: boolean
  severity?: number
  primaryEvent?: string
  onClick?: () => void
  onSettingsClick?: () => void
}

export function CameraTile({
  camera,
  snapshotUrl,
  isOffline = false,
  severity = 0,
  primaryEvent,
  onClick,
  onSettingsClick,
}: CameraTileProps) {
  const severityVariant = severity === 0 ? "severity0"
    : severity === 1 ? "severity1"
    : severity === 2 ? "severity2"
    : "severity3"

  const handleSettingsClick = (e: React.MouseEvent) => {
    e.stopPropagation()
    onSettingsClick?.()
  }

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
      <div className="relative aspect-video bg-muted">
        {isOffline ? (
          <div className="absolute inset-0 flex items-center justify-center">
            <XCircle className="h-12 w-12 text-muted-foreground" />
          </div>
        ) : (
          <img
            src={snapshotUrl}
            alt={camera.name}
            className="h-full w-full object-cover"
            loading="lazy"
          />
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
        {/* Location */}
        <p className="text-xs text-muted-foreground truncate pl-6">
          {camera.location}
        </p>
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
        {/* FID + Last updated */}
        <div className="flex items-center justify-between text-xs text-muted-foreground pl-6">
          {camera.fid && (
            <Badge variant="outline" className="text-[10px] px-1 h-4">
              FID:{camera.fid}
            </Badge>
          )}
          <div className="flex items-center gap-1 ml-auto">
            <Clock className="h-3 w-3" />
            <span>{formatRelativeTime(camera.updated_at)}</span>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
