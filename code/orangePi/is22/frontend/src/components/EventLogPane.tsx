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
import { API_BASE_URL } from "@/lib/config"
import { ChatInput } from "./ChatInput"
import { DetectionDetailModal } from "./DetectionDetailModal"

// Execution state for patrol ticker
type ExecutionState = "idle" | "accessing" | "cooldown" | "waiting"

// ============================================
// Phase 2: 表示仕様準拠 (Issue #102)
// AIEventLog_Redesign_v4.md Section 5.2 準拠
// ============================================

// T2-1: EVENT_COLORS定数（6色）- hex code完全一致
const EVENT_COLORS = {
  human: { bg: '#FFFF00', text: '#222222', border: '#B8B800' },
  vehicle: { bg: '#6495ED', text: '#FFFFFF', border: '#4169E1' },
  alert: { bg: '#FF0000', text: '#FFFF00', border: '#CC0000' },
  unknown: { bg: '#D3D3D3', text: '#222222', border: '#A9A9A9' },
  camera_lost: { bg: '#333333', text: '#FFFFFF', border: '#1A1A1A' },
  camera_recovered: { bg: '#FFFFFF', text: '#444444', border: '#CCCCCC' },
} as const

type EventColorKey = keyof typeof EVENT_COLORS

// T2-2: ICON_COLORS定数（6色）- 背景色に応じたコントラスト確保
const ICON_COLORS: Record<EventColorKey, string> = {
  human: '#222222',
  vehicle: '#FFFFFF',
  alert: '#FFFF00',
  unknown: '#555555',
  camera_lost: '#FFFFFF',
  camera_recovered: '#444444',
}

// T2-5: EVENT_LABELS定数（22ラベル）- 日本語翻訳
const EVENT_LABELS: Record<string, string> = {
  'human': '人物検知',
  'person': '人物検知',
  'humans': '複数人検知',
  'people': '複数人検知',
  'crowd': '群衆検知',
  'vehicle': '車両検知',
  'car': '車両検知',
  'truck': 'トラック検知',
  'suspicious': '不審行動',
  'alert': '警戒',
  'intrusion': '侵入検知',
  'fire': '火災検知',
  'smoke': '煙検知',
  'animal': '動物検知',
  'dog': '犬検知',
  'cat': '猫検知',
  'package': '荷物検知',
  'object': '物体検知',
  'motion': '動体検知',
  'movement': '動き検知',
  'unknown': '不明',
  'none': '検出なし',
  'camera_lost': '接続ロスト',
  'camera_recovered': '接続復帰',
}

// T2-7: SEVERITY_TOOLTIPS定数 - ホバー時の説明
const SEVERITY_TOOLTIPS: Record<number, string> = {
  0: '検出なし',
  1: '低優先度 - 通常の検知',
  2: '中優先度 - 注意が必要',
  3: '高優先度 - 即時確認推奨',
  4: '緊急 - 即時対応必要',
}

// T2-3: getEventStyle関数 - イベント種別に応じたスタイルを返す
function getEventColorKey(primaryEvent: string): EventColorKey {
  const event = primaryEvent.toLowerCase()

  if (event === 'human' || event === 'person' || event === 'humans' || event === 'people') {
    return 'human'
  }
  if (event === 'vehicle' || event === 'car' || event === 'truck') {
    return 'vehicle'
  }
  if (['suspicious', 'alert', 'intrusion', 'fire', 'smoke'].includes(event)) {
    return 'alert'
  }
  if (event === 'camera_lost') {
    return 'camera_lost'
  }
  if (event === 'camera_recovered') {
    return 'camera_recovered'
  }
  return 'unknown'
}

function getEventStyle(primaryEvent: string): React.CSSProperties {
  const colorKey = getEventColorKey(primaryEvent)
  const colors = EVENT_COLORS[colorKey]

  return {
    backgroundColor: colors.bg,
    color: colors.text,
    borderLeft: `3px solid ${colors.border}`,
  }
}

// T2-4: getIconColor関数 - イベント種別に応じたアイコン色を返す
function getIconColor(primaryEvent: string): string {
  const colorKey = getEventColorKey(primaryEvent)
  return ICON_COLORS[colorKey]
}

// Props
interface EventLogPaneProps {
  cameras: Camera[]
  onLogClick?: (log: DetectionLog) => void
  /** Cooldown seconds remaining (from App.tsx WebSocket) */
  cooldownSeconds?: number
  /** Execution state for patrol ticker (from App.tsx WebSocket) */
  executionState?: ExecutionState
  /** Issue #108: モバイル表示モード */
  isMobile?: boolean
}

// [REMOVED] getEventTypeColor - replaced by getEventStyle() in Phase 2

function getSeverityBadgeVariant(severity: number): "severity1" | "severity2" | "severity3" {
  if (severity === 1) return "severity1"
  if (severity === 2) return "severity2"
  return "severity3"
}

// Get icon based on primary_event type - T2-4: 背景色に応じたアイコン色使用
function getEventIcon(primaryEvent: string): React.ReactNode {
  const iconClass = "h-3 w-3 flex-shrink-0"
  const iconColor = getIconColor(primaryEvent)
  const iconStyle = { color: iconColor }

  switch (primaryEvent.toLowerCase()) {
    case "camera_lost":
      return <CameraOff className={iconClass} style={iconStyle} />
    case "camera_recovered":
      return <CameraIcon className={iconClass} style={iconStyle} />
    case "human":
    case "person":
      return <User className={iconClass} style={iconStyle} />
    case "humans":
    case "people":
    case "crowd":
      return <Users className={iconClass} style={iconStyle} />
    case "vehicle":
    case "car":
    case "truck":
      return <Car className={iconClass} style={iconStyle} />
    case "animal":
    case "dog":
    case "cat":
      return <Dog className={iconClass} style={iconStyle} />
    case "suspicious":
    case "alert":
    case "intrusion":
      return <AlertTriangle className={iconClass} style={iconStyle} />
    case "fire":
    case "smoke":
      return <Flame className={iconClass} style={iconStyle} />
    case "package":
    case "object":
      return <Package className={iconClass} style={iconStyle} />
    case "motion":
    case "movement":
      return <Eye className={iconClass} style={iconStyle} />
    default:
      return <HelpCircle className={iconClass} style={iconStyle} />
  }
}

// T2-6: getEventLabel関数修正 - EVENT_LABELSを使用
function getEventLabel(primaryEvent: string, countHint: number): string {
  const event = primaryEvent.toLowerCase()
  const label = EVENT_LABELS[event] || primaryEvent

  if (countHint > 1) {
    return `${label} (${countHint})`
  }
  return label
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

// T2-10: Compact Detection Log Item with animation support - 統合修正
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
  // T2-3: getEventStyle使用（hex code直接指定）
  const eventStyle = getEventStyle(log.primary_event)
  const badgeVariant = getSeverityBadgeVariant(log.severity)
  const shouldPulse = isNew && isDetectionEvent(log.primary_event)

  return (
    <div
      className={cn(
        "px-1.5 py-1 rounded cursor-pointer transition-colors hover:opacity-80",
        isNew && "animate-event-slide-in",
        shouldPulse && "animate-detection-pulse"
      )}
      style={eventStyle}
      onClick={(e) => {
        e.stopPropagation()
        onClick?.()
      }}
    >
      <div className="flex items-center justify-between gap-1">
        {/* Left: Icon + Camera + Event */}
        <div className="flex items-center gap-1 min-w-0 flex-1">
          {getEventIcon(log.primary_event)}
          {/* T2-9: カメラ名表示（min-w/max-w/title） */}
          <span
            className="text-[10px] font-medium truncate min-w-[40px] max-w-[120px] flex-shrink"
            title={cameraName}
          >
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
          {/* T2-8: Severityツールチップ適用 */}
          <Badge
            variant={badgeVariant}
            className="text-[9px] px-1 py-0 h-4 min-w-[18px] justify-center cursor-help"
            title={SEVERITY_TOOLTIPS[log.severity] ?? `重要度: ${log.severity}`}
          >
            {log.severity}
          </Badge>
          {log.confidence != null && log.confidence > 0 && (
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

// Chat message type with action support
interface ChatMessageAction {
  label: string
  variant: "primary" | "secondary"
  onClick: () => void
}

// Action metadata for persistence (functions can't be serialized)
interface ActionMeta {
  type: "preset_change"
  cameraId: string
  presetId: string
}

interface ChatMessage {
  id: string
  role: "user" | "assistant" | "system"
  content: string
  timestamp: Date
  actions?: ChatMessageAction[]
  actionMeta?: ActionMeta  // Serializable action metadata
  handled?: boolean  // For tracking if action was taken
}

// LocalStorage key for chat messages persistence
const CHAT_STORAGE_KEY = 'is22_chat_messages'

// Load chat messages from localStorage
function loadChatMessages(): ChatMessage[] {
  try {
    const stored = localStorage.getItem(CHAT_STORAGE_KEY)
    if (!stored) return []
    const parsed = JSON.parse(stored)
    // Convert timestamp strings back to Date objects
    return parsed.map((msg: ChatMessage & { timestamp: string }) => ({
      ...msg,
      timestamp: new Date(msg.timestamp),
      // Restore actions for system messages (not persisted, will be re-created on next check)
      actions: undefined,
    }))
  } catch {
    return []
  }
}

// Save chat messages to localStorage
function saveChatMessages(messages: ChatMessage[]) {
  try {
    // Don't save action functions (they can't be serialized)
    const toSave = messages.map(msg => ({
      ...msg,
      actions: undefined,
    }))
    localStorage.setItem(CHAT_STORAGE_KEY, JSON.stringify(toSave))
  } catch (err) {
    console.error('Failed to save chat messages:', err)
  }
}

// Preset display names (v2.0)
const PRESET_NAMES: Record<string, string> = {
  person_priority: "人物優先",
  balanced: "バランス",
  parking: "駐車場",
  entrance: "エントランス",
  corridor: "廊下",
  hotel_corridor: "ホテル廊下",  // v2.0 新規
  outdoor: "屋外",
  crowd: "群衆",
  office: "オフィス",
  restaurant: "飲食店ホール",   // v2.0 新規
  security_zone: "警戒区域",    // v2.0 新規
  object_focus: "対物検知",     // v2.0 新規
  custom: "カスタム",
  // v2.0 廃止（後方互換）
  night_vision: "夜間（廃止）",
  retail: "小売店（廃止）",
  warehouse: "倉庫（廃止）",
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
  isMobile: _isMobile = false,  // Reserved for future mobile-specific behavior
}: EventLogPaneProps) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const chatScrollRef = useRef<HTMLDivElement>(null)

  // Chat state with localStorage persistence
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>(() => loadChatMessages())

  // Overdetection suggestion tracking - initialize from persisted messages
  const [checkedOverdetection, setCheckedOverdetection] = useState(false)
  const [overdetectionCameras, setOverdetectionCameras] = useState<string[]>(() => {
    // Extract camera IDs from existing overdetection messages
    const stored = loadChatMessages()
    return stored
      .filter(msg => msg.id.startsWith('overdetection-'))
      .map(msg => msg.id.replace('overdetection-', '').split('-')[0])
  })

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

  // Save chat messages to localStorage
  useEffect(() => {
    saveChatMessages(chatMessages)
  }, [chatMessages])

  // Fetch overdetection analysis and create suggestion messages
  useEffect(() => {
    if (checkedOverdetection || cameras.length === 0) return

    const checkOverdetection = async () => {
      try {
        const response = await fetch(`${API_BASE_URL}/api/stats/overdetection?period=24h`)
        const data = await response.json()

        if (!data.ok || !data.data?.cameras) return

        const camerasWithIssues = data.data.cameras.filter(
          (c: { camera_id: string; issues: { severity: string }[] }) =>
            c.issues.some(i => i.severity === 'warning' || i.severity === 'critical')
        )

        if (camerasWithIssues.length === 0) {
          setCheckedOverdetection(true)
          return
        }

        // Create suggestion messages for each camera with issues
        const newMessages: ChatMessage[] = []

        for (const cam of camerasWithIssues) {
          // Skip if already shown for this camera
          if (overdetectionCameras.includes(cam.camera_id)) continue

          // Find camera name
          const camera = cameras.find(c => c.camera_id === cam.camera_id)
          const cameraName = camera?.name || cam.camera_name || cam.camera_id.substring(0, 8)
          const currentPreset = camera?.preset_id || 'balanced'

          // Determine recommended preset based on issues
          // Issue #107: Fix suggest logic - consider issue type and cause
          let recommendedPreset = 'balanced'
          let suggestionReason = ''

          const hasUnknownFlood = cam.issues.some((i: { type: string }) => i.type === 'unknown_flood')
          const hasHighFrequency = cam.issues.some((i: { type: string }) => i.type === 'high_frequency')
          const hasStaticObject = cam.issues.some((i: { type: string }) => i.type === 'static_object')
          const tagFixation = cam.issues.find((i: { type: string; tag?: string }) => i.type === 'tag_fixation')

          // Office item labels that should be excluded
          const officeItemLabels = ['mouse', 'keyboard', 'laptop', 'cell phone', 'tv', 'remote', 'book']

          if (hasUnknownFlood) {
            // Unknown flood → balanced has the most excluded_objects (14 items)
            recommendedPreset = 'balanced'
            suggestionReason = 'Unknown検出が多発しています。除外リストが充実した'
          } else if (hasHighFrequency && hasStaticObject) {
            // High frequency + static object → balanced to filter static office items
            recommendedPreset = 'balanced'
            suggestionReason = '静止物の繰り返し検出があります。除外リストが充実した'
          } else if (tagFixation) {
            // Tag fixation → check if it's an office item
            if (officeItemLabels.includes(tagFixation.tag || '')) {
              recommendedPreset = 'balanced'
              suggestionReason = `${tagFixation.tag}の固定検出があります。除外リストが充実した`
            } else {
              recommendedPreset = 'person_priority'
              suggestionReason = '特定タグの固定検出があります。人物検知に特化した'
            }
          } else if (hasHighFrequency) {
            // High frequency without static object → might be busy area, try crowd mode
            recommendedPreset = 'crowd'
            suggestionReason = '検出頻度が高いです。群衆モードの'
          }

          // Skip if already using recommended preset
          if (currentPreset === recommendedPreset) continue

          const currentPresetName = PRESET_NAMES[currentPreset] || currentPreset
          const recommendedPresetName = PRESET_NAMES[recommendedPreset] || recommendedPreset

          const messageId = `overdetection-${cam.camera_id}-${Date.now()}`

          newMessages.push({
            id: messageId,
            role: "system",
            content: `${cameraName}の検出の調整が必要そうです。${suggestionReason}「${recommendedPresetName}」への変更をお勧めします。（現在: ${currentPresetName}）変更しますか？`,
            timestamp: new Date(),
            handled: false,
            actionMeta: {
              type: "preset_change",
              cameraId: cam.camera_id,
              presetId: recommendedPreset,
            },
          })
        }

        if (newMessages.length > 0) {
          setChatMessages(prev => [...prev, ...newMessages])
          setOverdetectionCameras(prev => [
            ...prev,
            ...camerasWithIssues.map((c: { camera_id: string }) => c.camera_id)
          ])
        }

        setCheckedOverdetection(true)
      } catch (err) {
        console.error('Failed to check overdetection:', err)
        setCheckedOverdetection(true)
      }
    }

    // Delay check to allow cameras to load
    const timer = setTimeout(checkOverdetection, 2000)
    return () => clearTimeout(timer)
  }, [cameras, checkedOverdetection, overdetectionCameras])

  // Handle preset change from suggestion
  const handlePresetChange = async (cameraId: string, presetId: string, messageId: string) => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/cameras/${cameraId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ preset_id: presetId }),
      })

      if (response.ok) {
        const presetName = PRESET_NAMES[presetId] || presetId
        // Mark message as handled and add confirmation
        setChatMessages(prev => prev.map(msg =>
          msg.id === messageId ? { ...msg, handled: true } : msg
        ))
        setChatMessages(prev => [...prev, {
          id: `confirm-${Date.now()}`,
          role: "assistant",
          content: `プリセットを「${presetName}」に変更しました。`,
          timestamp: new Date(),
        }])
      } else {
        throw new Error('Failed to update preset')
      }
    } catch (err) {
      console.error('Failed to change preset:', err)
      setChatMessages(prev => [...prev, {
        id: `error-${Date.now()}`,
        role: "assistant",
        content: "プリセットの変更に失敗しました。設定画面から手動で変更してください。",
        timestamp: new Date(),
      }])
    }
  }

  // Handle dismiss suggestion
  const handleDismissSuggestion = (messageId: string) => {
    setChatMessages(prev => prev.map(msg =>
      msg.id === messageId ? { ...msg, handled: true } : msg
    ))
    setChatMessages(prev => [...prev, {
      id: `dismiss-${Date.now()}`,
      role: "assistant",
      content: "了解しました。現在の設定を維持します。",
      timestamp: new Date(),
    }])
  }

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
                    "flex",
                    msg.role === "user" ? "justify-end" : "justify-start"
                  )}
                >
                  <div
                    className={cn(
                      "max-w-[85%] text-xs px-3 py-2 rounded-2xl",
                      // User message: blue background, white text, right tail
                      msg.role === "user" && "bg-blue-500 text-white rounded-br-sm",
                      // System/Assistant message: light gray background, dark text, left tail
                      (msg.role === "system" || msg.role === "assistant") && "bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-100 rounded-bl-sm"
                    )}
                  >
                    {/* Message content */}
                    <div className="leading-relaxed">{msg.content}</div>
                    {/* Timestamp */}
                    <div className={cn(
                      "text-[9px] mt-1 opacity-60",
                      msg.role === "user" ? "text-right text-blue-100" : "text-right text-gray-500 dark:text-gray-400"
                    )}>
                      {msg.timestamp.toLocaleTimeString("ja-JP", { hour: "2-digit", minute: "2-digit" })}
                    </div>
                    {/* Action buttons for system messages with actionMeta */}
                    {msg.role === "system" && msg.actionMeta && !msg.handled && (
                      <div className="flex gap-2 mt-2 pt-2 border-t border-gray-300 dark:border-gray-600">
                        <button
                          onClick={() => handlePresetChange(msg.actionMeta!.cameraId, msg.actionMeta!.presetId, msg.id)}
                          className="flex-1 px-3 py-1.5 rounded-lg text-[11px] font-medium transition-colors bg-blue-500 hover:bg-blue-600 text-white"
                        >
                          はい
                        </button>
                        <button
                          onClick={() => handleDismissSuggestion(msg.id)}
                          className="flex-1 px-3 py-1.5 rounded-lg text-[11px] font-medium transition-colors bg-gray-300 dark:bg-gray-600 hover:bg-gray-400 dark:hover:bg-gray-500 text-gray-700 dark:text-gray-200"
                        >
                          いいえ
                        </button>
                      </div>
                    )}
                    {/* Handled indicator */}
                    {msg.role === "system" && msg.handled && (
                      <div className="mt-1.5 text-[10px] text-gray-500 dark:text-gray-400 flex items-center gap-1">
                        <span>✓</span>
                        <span>対応済み</span>
                      </div>
                    )}
                  </div>
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
