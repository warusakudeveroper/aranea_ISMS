/**
 * EventLogPane - AI Event Log with Patrol Feedback
 *
 * Features:
 * - Single-slot patrol ticker ("動いてる安心感") - subtle, no background
 * - Detection log list from MySQL (severity > 0 only)
 * - Severity-based styling for actual detections
 */

import { useEffect, useRef, useMemo } from "react"
import type { DetectionLog, Camera } from "@/types/api"
import { useEventLogStore } from "@/stores/eventLogStore"
import { Badge } from "@/components/ui/badge"
import { Activity, Camera as CameraIcon, RefreshCw } from "lucide-react"
import { cn } from "@/lib/utils"

// Props
interface EventLogPaneProps {
  cameras: Camera[]
  onLogClick?: (log: DetectionLog) => void
}

// Severity color mapping (for actual detections only)
function getSeverityColor(severity: number): string {
  if (severity === 1) return "bg-blue-50 dark:bg-blue-900/40 border-l-2 border-l-blue-500 dark:border-l-blue-400"
  if (severity === 2) return "bg-amber-50 dark:bg-amber-900/40 border-l-2 border-l-amber-500 dark:border-l-amber-400"
  return "bg-red-50 dark:bg-red-900/50 border-l-2 border-l-red-500 dark:border-l-red-400"
}

function getSeverityBadgeVariant(severity: number): "severity1" | "severity2" | "severity3" {
  if (severity === 1) return "severity1"
  if (severity === 2) return "severity2"
  return "severity3"
}

// Time formatting
function formatTime(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleTimeString("ja-JP", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  })
}

// Detection Log Item (persistent, severity > 0 only)
function DetectionLogItem({
  log,
  cameraName,
  onClick,
}: {
  log: DetectionLog
  cameraName: string
  onClick?: () => void
}) {
  const severityColor = getSeverityColor(log.severity)
  const badgeVariant = getSeverityBadgeVariant(log.severity)

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
            <span>{formatTime(log.captured_at)}</span>
          </div>
          <div className="flex items-center gap-1 mt-0.5">
            <CameraIcon className="h-3 w-3 text-muted-foreground flex-shrink-0" />
            <span className="text-sm font-medium truncate">
              {cameraName}
            </span>
          </div>
          <p className="text-xs text-muted-foreground mt-0.5 truncate">
            {log.primary_event}
            {log.count_hint > 0 && ` (${log.count_hint})`}
          </p>
        </div>
        <div className="flex flex-col items-end gap-1">
          <Badge variant={badgeVariant} className="flex-shrink-0 text-xs">
            {log.severity}
          </Badge>
          {log.confidence > 0 && (
            <span className="text-[10px] text-muted-foreground">
              {(log.confidence * 100).toFixed(0)}%
            </span>
          )}
        </div>
      </div>
      {log.tags && log.tags.length > 0 && (
        <div className="flex flex-wrap gap-1 mt-1">
          {log.tags.slice(0, 3).map((tag, i) => (
            <Badge key={i} variant="outline" className="text-[10px] py-0 px-1">
              {tag}
            </Badge>
          ))}
        </div>
      )}
    </div>
  )
}

export function EventLogPane({ cameras, onLogClick }: EventLogPaneProps) {
  const scrollRef = useRef<HTMLDivElement>(null)

  // Store state
  const {
    logs,
    stats,
    patrolFeedback,
    isLoading,
    fetchLogs,
    fetchStats,
    setCameraNames,
    cameraNames,
  } = useEventLogStore()

  // Initialize camera names
  useEffect(() => {
    if (cameras.length > 0) {
      setCameraNames(cameras)
    }
  }, [cameras, setCameraNames])

  // Initial fetch - only actual detections (severity > 0)
  useEffect(() => {
    fetchLogs({ detected_only: true, severity_min: 1, limit: 100 })
    fetchStats()
  }, [fetchLogs, fetchStats])

  // Get camera name helper
  const getCameraName = (cameraId: string): string => {
    return cameraNames[cameraId] || cameraId.substring(0, 12) + '...'
  }

  // Filter logs to only show severity > 0 (actual detections)
  const detectionLogs = useMemo(() => {
    return logs.filter(log => log.severity > 0)
  }, [logs])

  // Get latest patrol feedback (single slot, non-detection only)
  const latestPatrol = useMemo(() => {
    const nonDetection = patrolFeedback.find(f => !f.is_detection)
    return nonDetection || null
  }, [patrolFeedback])

  // Empty state - show only if no detections AND no patrol feedback
  const hasContent = detectionLogs.length > 0 || latestPatrol !== null

  if (!hasContent && !isLoading) {
    return (
      <div className="h-full flex flex-col">
        <div className="flex items-center gap-2 p-3 border-b">
          <Activity className="h-4 w-4" />
          <span className="text-sm font-medium">AI Event Log</span>
        </div>
        <div className="flex-1 flex items-center justify-center text-muted-foreground">
          <div className="text-center">
            <Activity className="h-12 w-12 mx-auto mb-2 opacity-30" />
            <p className="text-sm">検出イベントなし</p>
            <p className="text-xs mt-1">検出時にここに表示されます</p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b flex-shrink-0">
        <div className="flex items-center gap-2">
          <Activity className="h-4 w-4" />
          <span className="text-sm font-medium">AI Event Log</span>
        </div>
        <div className="flex items-center gap-2">
          {stats && (
            <Badge variant="secondary" className="text-xs">
              {detectionLogs.length}
            </Badge>
          )}
        </div>
      </div>

      {/* Patrol Ticker - single fixed slot, very subtle */}
      {latestPatrol && (
        <div className="px-3 py-1.5 border-b flex items-center gap-1.5 text-[11px] text-muted-foreground/50">
          <RefreshCw className="h-2.5 w-2.5 animate-spin flex-shrink-0" style={{ animationDuration: '4s' }} />
          <span className="truncate">{latestPatrol.camera_name}</span>
          <span>{formatTime(latestPatrol.timestamp)}</span>
          <span className="ml-auto">検出なし</span>
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-auto" ref={scrollRef}>
        <div className="p-2 space-y-1">
          {/* Loading indicator */}
          {isLoading && (
            <div className="flex items-center justify-center py-4">
              <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
            </div>
          )}

          {/* Detection Logs (severity > 0 only) */}
          {detectionLogs.map((log) => (
            <DetectionLogItem
              key={log.log_id}
              log={log}
              cameraName={getCameraName(log.lacis_id || '')}
              onClick={() => onLogClick?.(log)}
            />
          ))}

          {/* No detections message */}
          {detectionLogs.length === 0 && !isLoading && (
            <div className="text-center py-8 text-muted-foreground">
              <p className="text-sm">検出イベントなし</p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
