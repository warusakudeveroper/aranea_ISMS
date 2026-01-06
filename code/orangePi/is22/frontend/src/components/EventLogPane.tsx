/**
 * EventLogPane - AI Event Log with Patrol Feedback
 *
 * Features:
 * - Fixed patrol status area (always visible)
 * - Format: {カメラ名}/{IP} 推論処理---->score{score}検出なし
 * - Execution status: 待機中/クールダウン中/アクセス中
 * - Slide-in animation (500ms) for new logs
 * - Pulse highlight (3000ms total, 600ms intervals) for detections
 * - High-contrast icons
 */

import { useEffect, useRef, useMemo, useState } from "react"
import type { DetectionLog, Camera } from "@/types/api"
import { useEventLogStore } from "@/stores/eventLogStore"
import { Badge } from "@/components/ui/badge"
import {
  Activity,
  Camera as CameraIcon,
  CameraOff,
  RefreshCw,
  MessageCircle,
  User,
  Users,
  Car,
  Dog,
  AlertTriangle,
  Eye,
  HelpCircle,
  Flame,
  Package,
  Clock,
  Pause,
  Play,
  Loader2
} from "lucide-react"
import { cn } from "@/lib/utils"
import { ChatInput } from "./ChatInput"
import { DetectionDetailModal } from "./DetectionDetailModal"

// Execution state for patrol ticker
type ExecutionState = "idle" | "accessing" | "cooldown" | "waiting"

// Props
interface EventLogPaneProps {
  cameras: Camera[]
  onLogClick?: (log: DetectionLog) => void
  /** Cooldown seconds remaining (from App.tsx WebSocket) */
  cooldownSeconds?: number
  /** Execution state for patrol ticker (from App.tsx WebSocket) */
  executionState?: ExecutionState
}

// Event-type based color mapping
function getEventTypeColor(primaryEvent: string, severity: number): string {
  const event = primaryEvent.toLowerCase()

  // Camera status events (subtle)
  if (event === "camera_lost") {
    return "bg-gray-700 text-white border-l-2 border-l-gray-500"
  }
  if (event === "camera_recovered") {
    return "bg-white dark:bg-gray-100 text-gray-600 border-l-2 border-l-gray-300"
  }

  // Human detection - yellow (prominent)
  if (event === "human" || event === "person" || event === "humans" || event === "people") {
    return "bg-yellow-100 dark:bg-yellow-900/60 text-yellow-900 dark:text-yellow-100 border-l-2 border-l-yellow-500"
  }

  // Vehicle detection - blue (prominent)
  if (event === "vehicle" || event === "car" || event === "truck") {
    return "bg-blue-100 dark:bg-blue-900/60 text-blue-900 dark:text-blue-100 border-l-2 border-l-blue-500"
  }

  // Danger/alert - red (warning, strong)
  if (event === "suspicious" || event === "alert" || event === "intrusion" ||
      event === "fire" || event === "smoke" || severity >= 3) {
    return "bg-red-500 text-white border-l-2 border-l-red-700"
  }

  // Default by severity
  if (severity === 1) return "bg-blue-50 dark:bg-blue-900/40 border-l-2 border-l-blue-500"
  if (severity === 2) return "bg-amber-50 dark:bg-amber-900/40 border-l-2 border-l-amber-500"
  return "bg-red-50 dark:bg-red-900/50 border-l-2 border-l-red-500"
}

function getSeverityBadgeVariant(severity: number): "severity1" | "severity2" | "severity3" {
  if (severity === 1) return "severity1"
  if (severity === 2) return "severity2"
  return "severity3"
}

// Get icon based on primary_event type - HIGH CONTRAST colors
function getEventIcon(primaryEvent: string): React.ReactNode {
  const iconClass = "h-3 w-3 flex-shrink-0"

  switch (primaryEvent.toLowerCase()) {
    case "camera_lost":
      return <CameraOff className={cn(iconClass, "text-red-600 dark:text-red-400")} />
    case "camera_recovered":
      return <CameraIcon className={cn(iconClass, "text-green-600 dark:text-green-400")} />
    case "human":
    case "person":
      return <User className={cn(iconClass, "text-amber-600 dark:text-amber-400")} />
    case "humans":
    case "people":
    case "crowd":
      return <Users className={cn(iconClass, "text-amber-600 dark:text-amber-400")} />
    case "vehicle":
    case "car":
    case "truck":
      return <Car className={cn(iconClass, "text-blue-600 dark:text-blue-400")} />
    case "animal":
    case "dog":
    case "cat":
      return <Dog className={cn(iconClass, "text-orange-600 dark:text-orange-400")} />
    case "suspicious":
    case "alert":
    case "intrusion":
      return <AlertTriangle className={cn(iconClass, "text-red-600 dark:text-red-400")} />
    case "fire":
    case "smoke":
      return <Flame className={cn(iconClass, "text-red-700 dark:text-red-300")} />
    case "package":
    case "object":
      return <Package className={cn(iconClass, "text-slate-700 dark:text-slate-300")} />
    case "motion":
    case "movement":
      return <Eye className={cn(iconClass, "text-cyan-600 dark:text-cyan-400")} />
    default:
      return <HelpCircle className={cn(iconClass, "text-slate-600 dark:text-slate-400")} />
  }
}

// Get display label for event type
function getEventLabel(primaryEvent: string, countHint: number): string {
  const event = primaryEvent.toLowerCase()

  // Camera status events
  if (event === "camera_lost") return "接続ロスト"
  if (event === "camera_recovered") return "接続復帰"

  // Detection events with count
  if (countHint > 0) {
    return `${primaryEvent} (${countHint})`
  }

  return primaryEvent
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

// Check if event is a detection (not camera status)
function isDetectionEvent(primaryEvent: string): boolean {
  const event = primaryEvent.toLowerCase()
  return event !== "camera_lost" && event !== "camera_recovered"
}

// Compact Detection Log Item with animation support
function DetectionLogItem({
  log,
  cameraName,
  onClick,
  isNew,
}: {
  log: DetectionLog
  cameraName: string
  onClick?: () => void
  isNew?: boolean
}) {
  const eventColor = getEventTypeColor(log.primary_event, log.severity)
  const badgeVariant = getSeverityBadgeVariant(log.severity)
  const shouldPulse = isNew && isDetectionEvent(log.primary_event)

  return (
    <div
      className={cn(
        "px-1.5 py-1 rounded cursor-pointer transition-colors hover:opacity-80",
        eventColor,
        isNew && "animate-event-slide-in",
        shouldPulse && "animate-detection-pulse"
      )}
      onClick={(e) => {
        e.stopPropagation()
        onClick?.()
      }}
    >
      <div className="flex items-center justify-between gap-1">
        {/* Left: Icon + Camera + Event */}
        <div className="flex items-center gap-1 min-w-0 flex-1">
          {getEventIcon(log.primary_event)}
          <span className="text-[10px] font-medium truncate max-w-[60px]">
            {cameraName}
          </span>
          <span className="text-[9px] opacity-80 truncate">
            {getEventLabel(log.primary_event, log.count_hint)}
          </span>
        </div>

        {/* Right: Time + Severity + Confidence */}
        <div className="flex items-center gap-1 flex-shrink-0">
          <span className="text-[9px] opacity-70">
            {formatTime(log.captured_at)}
          </span>
          <Badge variant={badgeVariant} className="text-[9px] px-1 py-0 h-4 min-w-[18px] justify-center">
            {log.severity}
          </Badge>
          {log.confidence > 0 && (
            <span className="text-[8px] opacity-70 w-[28px] text-right">
              {(log.confidence * 100).toFixed(0)}%
            </span>
          )}
        </div>
      </div>

      {/* Tags row (only if has tags, very compact) */}
      {log.tags && log.tags.length > 0 && (
        <div className="flex flex-wrap gap-0.5 mt-0.5 pl-4">
          {log.tags.slice(0, 4).map((tag, i) => (
            <Badge key={i} variant="outline" className="text-[8px] py-0 px-0.5 h-3">
              {tag}
            </Badge>
          ))}
          {log.tags.length > 4 && (
            <span className="text-[8px] opacity-60">+{log.tags.length - 4}</span>
          )}
        </div>
      )}
    </div>
  )
}

// Chat message type (stub for now)
interface ChatMessage {
  id: string
  role: "user" | "assistant"
  content: string
  timestamp: Date
}

// Patrol Status Ticker - Fixed frame, always visible
function PatrolStatusTicker({
  cameras,
  latestPatrol,
  executionState,
  cooldownSeconds,
}: {
  cameras: Camera[]
  latestPatrol: { camera_id: string; camera_name: string; timestamp: string; confidence?: number } | null
  executionState: ExecutionState
  cooldownSeconds: number
}) {
  // Find camera IP address
  const cameraIp = useMemo(() => {
    if (!latestPatrol) return null
    const camera = cameras.find(c => c.camera_id === latestPatrol.camera_id || c.name === latestPatrol.camera_name)
    return camera?.ip_address || null
  }, [cameras, latestPatrol])

  // Inference progress animation state
  const [inferenceProgress, setInferenceProgress] = useState(0)

  // Animate inference progress when accessing
  useEffect(() => {
    if (executionState === "accessing") {
      const timer = setInterval(() => {
        setInferenceProgress(p => (p + 1) % 4)
      }, 150)
      return () => clearInterval(timer)
    }
    setInferenceProgress(0)
  }, [executionState])

  const inferenceArrow = useMemo(() => {
    const dots = ["-", "--", "---", "----"]
    return dots[inferenceProgress]
  }, [inferenceProgress])

  // Get status icon and text
  const getStatusDisplay = () => {
    switch (executionState) {
      case "accessing":
        return {
          icon: <Loader2 className="h-2.5 w-2.5 animate-spin text-blue-500" />,
          text: `推論処理${inferenceArrow}>`,
          color: "text-blue-600 dark:text-blue-400"
        }
      case "cooldown":
        return {
          icon: <Pause className="h-2.5 w-2.5 text-amber-500" />,
          text: `クールダウン ${cooldownSeconds}s`,
          color: "text-amber-600 dark:text-amber-400"
        }
      case "waiting":
        return {
          icon: <Clock className="h-2.5 w-2.5 text-gray-500" />,
          text: "次巡回待機中",
          color: "text-gray-500"
        }
      default:
        return {
          icon: <Play className="h-2.5 w-2.5 text-green-500" />,
          text: "待機中",
          color: "text-gray-500"
        }
    }
  }

  const status = getStatusDisplay()

  return (
    <div className="px-2 py-1.5 border-b bg-muted/30 flex-shrink-0">
      {/* Main info line */}
      <div className="flex items-center gap-1.5 text-[10px]">
        <CameraIcon className="h-3 w-3 text-primary flex-shrink-0" />
        {latestPatrol ? (
          <>
            <span className="font-medium truncate max-w-[70px]">
              {latestPatrol.camera_name}
            </span>
            {cameraIp && (
              <span className="text-muted-foreground truncate">
                /{cameraIp}
              </span>
            )}
          </>
        ) : (
          <span className="text-muted-foreground">--</span>
        )}
        <span className="ml-auto text-muted-foreground">
          {latestPatrol ? formatTime(latestPatrol.timestamp) : "--:--:--"}
        </span>
      </div>

      {/* Status line */}
      <div className="flex items-center gap-1.5 text-[9px] mt-0.5">
        {status.icon}
        <span className={cn("truncate flex-1", status.color)}>
          {status.text}
        </span>
        {latestPatrol && executionState !== "accessing" && (
          <span className="text-green-600 dark:text-green-400 font-medium flex-shrink-0">
            検出なし
          </span>
        )}
      </div>
    </div>
  )
}

export function EventLogPane({
  cameras,
  cooldownSeconds = 0,
  executionState = "waiting",
}: EventLogPaneProps) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const chatScrollRef = useRef<HTMLDivElement>(null)

  // Chat state (stub - will connect to mobes2.0 later)
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([])

  // Detection detail modal state
  const [selectedLog, setSelectedLog] = useState<DetectionLog | null>(null)
  const [isDetailModalOpen, setIsDetailModalOpen] = useState(false)

  // Track new log IDs for animation (clear after animation duration)
  const [newLogIds, setNewLogIds] = useState<Set<number>>(new Set())
  const previousLogsRef = useRef<number[]>([])

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

  // Track new logs for animation
  useEffect(() => {
    const currentLogIds = logs.map(l => l.log_id)
    const previousLogIds = previousLogsRef.current

    // Find new logs
    const newIds = currentLogIds.filter(id => !previousLogIds.includes(id))

    if (newIds.length > 0) {
      setNewLogIds(new Set(newIds))

      // Clear new status after animation completes (500ms slide + 3000ms pulse)
      setTimeout(() => {
        setNewLogIds(new Set())
      }, 3500)
    }

    previousLogsRef.current = currentLogIds
  }, [logs])

  // Get camera name helper
  const getCameraName = (cameraId: string): string => {
    return cameraNames[cameraId] || cameraId.substring(0, 8) + '...'
  }

  // Filter logs to only show severity > 0 (actual detections)
  const detectionLogs = useMemo(() => {
    return logs.filter(log => log.severity > 0)
  }, [logs])

  // Get latest patrol feedback (single slot, non-detection only)
  const latestPatrol = useMemo(() => {
    const nonDetection = patrolFeedback.find(f => !f.is_detection)
    if (!nonDetection) return null
    return {
      camera_id: nonDetection.camera_id,
      camera_name: nonDetection.camera_name,
      timestamp: nonDetection.timestamp,
    }
  }, [patrolFeedback])

  // Handle log click - open detection detail modal
  const handleLogClick = (log: DetectionLog) => {
    setSelectedLog(log)
    setIsDetailModalOpen(true)
  }

  // Handle chat message send (stub)
  const handleChatSend = (message: string) => {
    const userMessage: ChatMessage = {
      id: `user-${Date.now()}`,
      role: "user",
      content: message,
      timestamp: new Date(),
    }
    setChatMessages((prev) => [...prev, userMessage])

    // TODO: Connect to mobes2.0 API
    setTimeout(() => {
      const assistantMessage: ChatMessage = {
        id: `assistant-${Date.now()}`,
        role: "assistant",
        content: "mobes2.0との連携は後続タスクで実装予定です。",
        timestamp: new Date(),
      }
      setChatMessages((prev) => [...prev, assistantMessage])
    }, 500)
  }

  // Auto-scroll chat to bottom
  useEffect(() => {
    if (chatScrollRef.current) {
      chatScrollRef.current.scrollTop = chatScrollRef.current.scrollHeight
    }
  }, [chatMessages])

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-2 border-b flex-shrink-0">
        <div className="flex items-center gap-2">
          <Activity className="h-4 w-4 text-primary" />
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

      {/* Patrol Status Ticker - FIXED FRAME, always visible */}
      <PatrolStatusTicker
        cameras={cameras}
        latestPatrol={latestPatrol}
        executionState={executionState}
        cooldownSeconds={cooldownSeconds}
      />

      {/* Upper Half: Detection Logs */}
      <div className="h-1/2 overflow-auto border-b" ref={scrollRef}>
        <div className="p-1 space-y-0.5">
          {/* Loading indicator */}
          {isLoading && (
            <div className="flex items-center justify-center py-4">
              <RefreshCw className="h-4 w-4 animate-spin text-muted-foreground" />
            </div>
          )}

          {/* Detection Logs (severity > 0 only) */}
          {detectionLogs.map((log) => (
            <DetectionLogItem
              key={log.log_id}
              log={log}
              cameraName={getCameraName(log.lacis_id || '')}
              onClick={() => handleLogClick(log)}
              isNew={newLogIds.has(log.log_id)}
            />
          ))}

          {/* No detections message */}
          {detectionLogs.length === 0 && !isLoading && (
            <div className="text-center py-4 text-muted-foreground">
              <p className="text-xs">検出イベントなし</p>
            </div>
          )}
        </div>
      </div>

      {/* Lower Half: Chat */}
      <div className="h-1/2 flex flex-col">
        {/* Chat Header */}
        <div className="flex items-center gap-2 p-2 border-b flex-shrink-0">
          <MessageCircle className="h-3 w-3 text-primary" />
          <span className="text-xs font-medium">AIアシスタント</span>
        </div>

        {/* Chat History */}
        <div className="flex-1 overflow-auto p-2" ref={chatScrollRef}>
          {chatMessages.length === 0 ? (
            <div className="text-center text-muted-foreground text-xs py-4">
              質問を入力してください
            </div>
          ) : (
            <div className="space-y-2">
              {chatMessages.map((msg) => (
                <div
                  key={msg.id}
                  className={cn(
                    "text-xs p-2 rounded",
                    msg.role === "user"
                      ? "bg-primary/10 ml-4"
                      : "bg-muted mr-4"
                  )}
                >
                  {msg.content}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Chat Input */}
        <ChatInput onSend={handleChatSend} placeholder="質問を入力..." />
      </div>

      {/* Detection Detail Modal */}
      <DetectionDetailModal
        log={selectedLog}
        isOpen={isDetailModalOpen}
        onClose={() => setIsDetailModalOpen(false)}
        cameraName={selectedLog ? getCameraName(selectedLog.lacis_id || '') : ''}
      />
    </div>
  )
}
