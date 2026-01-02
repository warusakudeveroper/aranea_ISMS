/**
 * EventLogPane - AI Event Log with Patrol Feedback
 *
 * Features:
 * - Real-time patrol feedback ("動いてる安心感")
 * - Detection log list from MySQL
 * - Severity-based styling
 * - Auto-fade for non-detection items
 */

import { useEffect, useRef, useState, useMemo } from "react"
import type { DetectionLog, PatrolFeedback, Camera } from "@/types/api"
import { useEventLogStore, PATROL_FEEDBACK_TTL_MS } from "@/stores/eventLogStore"
import { Badge } from "@/components/ui/badge"
import { Activity, Camera as CameraIcon, RefreshCw, Eye, EyeOff } from "lucide-react"
import { cn } from "@/lib/utils"

// Props
interface EventLogPaneProps {
  cameras: Camera[]
  onLogClick?: (log: DetectionLog) => void
}

// Severity color mapping
function getSeverityColor(severity: number): string {
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

// Time formatting
function formatTime(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleTimeString("ja-JP", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  })
}

// Patrol Feedback Item (transient, fades out)
function PatrolFeedbackItem({ feedback }: { feedback: PatrolFeedback }) {
  const [opacity, setOpacity] = useState(1)

  // Fade out animation
  useEffect(() => {
    const startTime = Date.now()
    const fadeStartAt = PATROL_FEEDBACK_TTL_MS * 0.6 // Start fading at 60%

    const interval = setInterval(() => {
      const elapsed = Date.now() - startTime
      if (elapsed > fadeStartAt) {
        const remaining = PATROL_FEEDBACK_TTL_MS - elapsed
        const newOpacity = Math.max(0, remaining / (PATROL_FEEDBACK_TTL_MS - fadeStartAt))
        setOpacity(newOpacity)
      }
    }, 50)

    return () => clearInterval(interval)
  }, [])

  return (
    <div
      className={cn(
        "p-1.5 rounded text-xs transition-all",
        feedback.is_detection
          ? getSeverityColor(feedback.severity || 0)
          : "bg-zinc-50 dark:bg-zinc-900/40"
      )}
      style={{ opacity }}
    >
      <div className="flex items-center gap-1.5">
        {feedback.is_detection ? (
          <Eye className="h-3 w-3 text-primary flex-shrink-0" />
        ) : (
          <RefreshCw className="h-3 w-3 text-muted-foreground flex-shrink-0 animate-spin" style={{ animationDuration: '3s' }} />
        )}
        <span className="font-medium truncate max-w-[100px]">
          {feedback.camera_name}
        </span>
        <span className="text-muted-foreground">
          {formatTime(feedback.timestamp)}
        </span>
        {feedback.is_detection ? (
          <Badge variant={getSeverityBadgeVariant(feedback.severity || 0)} className="text-[10px] py-0 px-1 ml-auto">
            {feedback.primary_event}
          </Badge>
        ) : (
          <span className="text-muted-foreground/60 ml-auto">検出なし</span>
        )}
      </div>
    </div>
  )
}

// Detection Log Item (persistent)
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
            {log.processing_ms && (
              <span className="text-muted-foreground/50">({log.processing_ms}ms)</span>
            )}
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
      {log.tags.length > 0 && (
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
  const [showNonDetections, setShowNonDetections] = useState(true)

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

  // Initial fetch
  useEffect(() => {
    fetchLogs({ detected_only: true, limit: 100 })
    fetchStats()
  }, [fetchLogs, fetchStats])

  // Get camera name helper
  const getCameraName = (cameraId: string): string => {
    return cameraNames[cameraId] || cameraId.substring(0, 12) + '...'
  }

  // Filter patrol feedback based on showNonDetections
  const visibleFeedback = useMemo(() => {
    if (showNonDetections) return patrolFeedback
    return patrolFeedback.filter(f => f.is_detection)
  }, [patrolFeedback, showNonDetections])

  // Empty state
  if (logs.length === 0 && patrolFeedback.length === 0 && !isLoading) {
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
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b flex-shrink-0">
        <div className="flex items-center gap-2">
          <Activity className="h-4 w-4" />
          <span className="text-sm font-medium">AI Event Log</span>
        </div>
        <div className="flex items-center gap-2">
          {/* Toggle non-detection visibility */}
          <button
            onClick={() => setShowNonDetections(!showNonDetections)}
            className={cn(
              "p-1 rounded hover:bg-muted transition-colors",
              !showNonDetections && "text-muted-foreground"
            )}
            title={showNonDetections ? "検出なしを非表示" : "検出なしを表示"}
          >
            {showNonDetections ? (
              <Eye className="h-3.5 w-3.5" />
            ) : (
              <EyeOff className="h-3.5 w-3.5" />
            )}
          </button>
          {stats && (
            <Badge variant="secondary" className="text-xs">
              {stats.total_logs}
            </Badge>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto" ref={scrollRef}>
        <div className="p-2 space-y-1">
          {/* Loading indicator */}
          {isLoading && (
            <div className="flex items-center justify-center py-4">
              <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
            </div>
          )}

          {/* Patrol Feedback (transient) */}
          {visibleFeedback.length > 0 && (
            <div className="space-y-0.5 mb-2 pb-2 border-b border-dashed">
              {visibleFeedback.map((feedback, i) => (
                <PatrolFeedbackItem key={`${feedback.camera_id}-${feedback.timestamp}-${i}`} feedback={feedback} />
              ))}
            </div>
          )}

          {/* Detection Logs (persistent) */}
          {logs.map((log) => (
            <DetectionLogItem
              key={log.log_id}
              log={log}
              cameraName={getCameraName(log.lacis_id || '')}
              onClick={() => onLogClick?.(log)}
            />
          ))}
        </div>
      </div>
    </div>
  )
}
