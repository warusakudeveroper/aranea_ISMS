import { useEffect, useRef } from "react"
import type { Event } from "@/types/api"
import { Badge } from "@/components/ui/badge"
import { Activity, Camera } from "lucide-react"
import { cn } from "@/lib/utils"

interface EventLogPaneProps {
  events: Event[]
  onEventClick?: (event: Event) => void
}

function getSeverityColor(severity: number): string {
  // ダークテーマで視認性を確保するため、より明るい背景色と明確な左ボーダーを使用
  if (severity === 0) return "bg-zinc-100 dark:bg-zinc-800/60"
  if (severity === 1) return "bg-blue-50 dark:bg-blue-900/40 border-l-2 border-l-blue-500 dark:border-l-blue-400"
  if (severity === 2) return "bg-amber-50 dark:bg-amber-900/40 border-l-2 border-l-amber-500 dark:border-l-amber-400"
  return "bg-red-50 dark:bg-red-900/50 border-l-2 border-l-red-500 dark:border-l-red-400"
}

function getSeverityBadgeVariant(severity: number): "severity0" | "severity1" | "severity2" | "severity3" {
  if (severity === 0) return "severity0"
  if (severity === 1) return "severity1"
  if (severity === 2) return "severity2"
  return "severity3"
}

function formatTime(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleTimeString("ja-JP", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  })
}

function EventLogItem({
  event,
  onClick,
}: {
  event: Event
  onClick?: () => void
}) {
  const severityColor = getSeverityColor(event.severity)
  const badgeVariant = getSeverityBadgeVariant(event.severity)

  return (
    <div
      className={cn(
        "p-2 rounded cursor-pointer transition-colors hover:opacity-80",
        severityColor
      )}
      onClick={onClick}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <span>{formatTime(event.captured_at)}</span>
          </div>
          <div className="flex items-center gap-1 mt-0.5">
            <Camera className="h-3 w-3 text-muted-foreground flex-shrink-0" />
            <span className="text-sm font-medium truncate">
              {event.camera_id.substring(0, 12)}...
            </span>
          </div>
          <p className="text-xs text-muted-foreground mt-0.5 truncate">
            {event.primary_event}
          </p>
        </div>
        <Badge variant={badgeVariant} className="flex-shrink-0 text-xs">
          {event.severity}
        </Badge>
      </div>
      {event.tags.length > 0 && (
        <div className="flex flex-wrap gap-1 mt-1">
          {event.tags.slice(0, 2).map((tag, i) => (
            <Badge key={i} variant="outline" className="text-[10px] py-0 px-1">
              {tag}
            </Badge>
          ))}
        </div>
      )}
    </div>
  )
}

export function EventLogPane({ events, onEventClick }: EventLogPaneProps) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const isAtBottomRef = useRef(true)

  // Auto-scroll to bottom when new events arrive
  useEffect(() => {
    if (isAtBottomRef.current && scrollRef.current) {
      scrollRef.current.scrollTop = 0
    }
  }, [events])

  if (events.length === 0) {
    return (
      <div className="h-full flex flex-col">
        <div className="flex items-center gap-2 p-3 border-b">
          <Activity className="h-4 w-4" />
          <span className="text-sm font-medium">AI Event Log</span>
        </div>
        <div className="flex-1 flex items-center justify-center text-muted-foreground">
          <div className="text-center">
            <Activity className="h-12 w-12 mx-auto mb-2 opacity-30" />
            <p className="text-sm">No events yet</p>
            <p className="text-xs mt-1">Events will stream here</p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between p-3 border-b flex-shrink-0">
        <div className="flex items-center gap-2">
          <Activity className="h-4 w-4" />
          <span className="text-sm font-medium">AI Event Log</span>
        </div>
        <Badge variant="secondary" className="text-xs">
          {events.length}
        </Badge>
      </div>
      <div className="flex-1 overflow-auto" ref={scrollRef}>
        <div className="p-2 space-y-1">
          {events.map((event) => (
            <EventLogItem
              key={event.event_id}
              event={event}
              onClick={() => onEventClick?.(event)}
            />
          ))}
        </div>
      </div>
    </div>
  )
}
