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
import type { DetectionLog, Camera, AIChatResponse, ChatMessageInput } from "@/types/api"
import type { SummaryReportMessage, ChatSyncMessage } from "@/hooks/useWebSocket"
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
  Loader2,
  Maximize2
} from "lucide-react"
import { cn } from "@/lib/utils"
import { API_BASE_URL } from "@/lib/config"
import { ChatInput } from "./ChatInput"
import { DetectionDetailModal } from "./DetectionDetailModal"
import { ChatExpandModal } from "./ChatExpandModal"
import { loadAIAssistantSettings } from "./SettingsModal"
import aichatIcon from "@/assets/aichat_icon.svg"
import ReactMarkdown from "react-markdown"

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
  /** Summary/GrandSummary report from scheduler (via WebSocket) */
  summaryReport?: SummaryReportMessage | null
  /** Callback after summary report is consumed (added to chat) */
  onSummaryReportConsumed?: () => void
  /** Chat sync message for cross-device real-time updates */
  chatSyncMessage?: ChatSyncMessage | null
  /** Callback after chat sync message is consumed */
  onChatSyncConsumed?: () => void
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
  dismissAt?: number  // Timestamp when message should be hidden (for auto-dismiss)
  isHiding?: boolean  // True when slide-out animation is active
}

// API-based chat message persistence (cross-device sync)
// Design: All data stored on backend, no browser-local storage

interface ChatMessageAPI {
  message_id: string
  role: string
  content: string
  timestamp: string
  handled: boolean
  action_type?: string
  action_camera_id?: string
  action_preset_id?: string
  dismiss_at?: number
  is_hiding: boolean
}

// Load chat messages from backend API
async function loadChatMessagesFromAPI(): Promise<ChatMessage[]> {
  try {
    const response = await fetch(`${API_BASE_URL}/api/chat/messages?limit=100`)
    if (!response.ok) return []
    const result = await response.json()
    if (!result.ok || !result.data) return []

    return result.data.map((msg: ChatMessageAPI) => ({
      id: msg.message_id,
      role: msg.role as 'user' | 'assistant' | 'system',
      content: msg.content,
      timestamp: new Date(msg.timestamp),
      handled: msg.handled,
      actionMeta: msg.action_type ? {
        type: msg.action_type as 'preset_change',
        cameraId: msg.action_camera_id || '',
        presetId: msg.action_preset_id || '',
      } : undefined,
      dismissAt: msg.dismiss_at,
      isHiding: msg.is_hiding,
    }))
  } catch (err) {
    console.error('Failed to load chat messages from API:', err)
    return []
  }
}

// Save a single chat message to backend API
async function saveChatMessageToAPI(msg: ChatMessage): Promise<boolean> {
  try {
    const apiMsg: ChatMessageAPI = {
      message_id: msg.id,
      role: msg.role,
      content: msg.content,
      timestamp: msg.timestamp.toISOString(),
      handled: msg.handled || false,
      action_type: msg.actionMeta?.type,
      action_camera_id: msg.actionMeta?.cameraId,
      action_preset_id: msg.actionMeta?.presetId,
      dismiss_at: msg.dismissAt,
      is_hiding: msg.isHiding || false,
    }

    const response = await fetch(`${API_BASE_URL}/api/chat/messages`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(apiMsg),
    })
    return response.ok
  } catch (err) {
    console.error('Failed to save chat message to API:', err)
    return false
  }
}

// Update a chat message in backend API
async function updateChatMessageInAPI(msg: ChatMessage): Promise<boolean> {
  try {
    const apiMsg: Partial<ChatMessageAPI> = {
      handled: msg.handled || false,
      dismiss_at: msg.dismissAt,
      is_hiding: msg.isHiding || false,
    }

    const response = await fetch(`${API_BASE_URL}/api/chat/messages/${msg.id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(apiMsg),
    })
    return response.ok
  } catch (err) {
    console.error('Failed to update chat message in API:', err)
    return false
  }
}

// Delete a chat message from backend API
async function deleteChatMessageFromAPI(messageId: string): Promise<boolean> {
  try {
    const response = await fetch(`${API_BASE_URL}/api/chat/messages/${messageId}`, {
      method: 'DELETE',
    })
    return response.ok
  } catch (err) {
    console.error('Failed to delete chat message from API:', err)
    return false
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
  summaryReport,
  onSummaryReportConsumed,
  chatSyncMessage,
  onChatSyncConsumed,
}: EventLogPaneProps) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const chatScrollRef = useRef<HTMLDivElement>(null)

  // Chat state with backend API persistence (cross-device sync)
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([])
  const [chatMessagesLoaded, setChatMessagesLoaded] = useState(false)

  // Load chat messages from backend API on mount
  useEffect(() => {
    loadChatMessagesFromAPI().then(messages => {
      setChatMessages(messages)
      setChatMessagesLoaded(true)
    })
  }, [])

  // Overdetection suggestion tracking - initialize after chat messages loaded
  const [checkedOverdetection, setCheckedOverdetection] = useState(false)
  const [overdetectionCameras, setOverdetectionCameras] = useState<string[]>([])

  // Initialize overdetectionCameras from loaded chat messages
  useEffect(() => {
    if (chatMessagesLoaded && chatMessages.length > 0) {
      const cameraIds = chatMessages
        .filter(msg => msg.id.startsWith('overdetection-'))
        .map(msg => msg.id.replace('overdetection-', '').split('-')[0])
      if (cameraIds.length > 0) {
        setOverdetectionCameras(cameraIds)
      }
    }
  }, [chatMessagesLoaded, chatMessages])

  // Detection detail modal state
  const [selectedLog, setSelectedLog] = useState<DetectionLog | null>(null)
  const [isDetailModalOpen, setIsDetailModalOpen] = useState(false)

  // Chat expand modal state
  const [isChatExpandOpen, setIsChatExpandOpen] = useState(false)

  // AI Chat loading state - 思考中インジケーター表示用
  const [isChatLoading, setIsChatLoading] = useState(false)

  // Track new log IDs for animation (clear after animation duration)
  const [newLogIds, setNewLogIds] = useState<Set<number>>(new Set())
  const previousLogsRef = useRef<number[]>([])
  // Track processed summary IDs to prevent duplicate messages
  const processedSummaryIdsRef = useRef<Set<number>>(new Set())

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

  // Handle chat message send - AI Assistant with Paraclate APP integration
  // Paraclate_DesignOverview.md準拠:
  // - AIアシスタントタブからの質問に関してもParaclate APPからのレスポンスでチャットボット機能を行う
  const handleChatSend = async (message: string) => {
    const userMessage: ChatMessage = {
      id: `user-${Date.now()}`,
      role: "user",
      content: message,
      timestamp: new Date(),
    }
    setChatMessages((prev) => [...prev, userMessage])
    saveChatMessageToAPI(userMessage)  // Save to backend

    // 思考中状態を開始
    setIsChatLoading(true)

    try {
      // Call Paraclate APP AI Chat API
      const response = await processAIQuery(message, chatMessages)

      const assistantMessage: ChatMessage = {
        id: `assistant-${Date.now()}`,
        role: "assistant",
        content: response,
        timestamp: new Date(),
      }
      setChatMessages((prev) => [...prev, assistantMessage])
      saveChatMessageToAPI(assistantMessage)  // Save to backend
    } finally {
      // 思考中状態を終了
      setIsChatLoading(false)
    }
  }

  // AI Query Processor - Paraclate APP Integration
  // Paraclate_DesignOverview.md準拠:
  // - 検出特徴の人物のカメラ間での移動などを横断的に把握
  // - カメラ不調などの傾向も把握
  // - 過去の記録を参照する、過去の記録範囲を対話的にユーザーと会話
  async function processAIQuery(
    query: string,
    conversationHistory: ChatMessage[]
  ): Promise<string> {
    try {
      // Get FID from cameras or use default
      // FID is the facility identifier for lacisOath authentication
      const fid = cameras[0]?.fid || '0150'

      // Build conversation history for context
      // Include system messages (summaries) for continuity
      const historyForApi: ChatMessageInput[] = conversationHistory
        .slice(-10)  // Last 10 messages for context
        .map(m => ({
          role: m.role as 'user' | 'assistant' | 'system',
          content: m.content,
        }))

      // Call Paraclate APP AI Chat API
      const response = await fetch(`${API_BASE_URL}/api/paraclate/chat`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          fid,
          message: query,
          conversationHistory: historyForApi,
          autoContext: true,  // Auto-collect camera & detection context
        }),
      })

      if (!response.ok) {
        const errorText = await response.text()
        console.error('AI Chat API error:', response.status, errorText)
        return generateFallbackResponse(query, cameras, logs)
      }

      const result: AIChatResponse = await response.json()

      if (result.ok && result.message) {
        // Append processing time info if available
        let aiResponse = result.message
        if (result.processingTimeMs && result.processingTimeMs > 0) {
          aiResponse += `\n\n_応答時間: ${result.processingTimeMs}ms_`
        }
        return aiResponse
      } else {
        console.warn('AI Chat returned error:', result.error)
        return generateFallbackResponse(query, cameras, logs)
      }
    } catch (error) {
      console.error('AI Chat request failed:', error)
      return generateFallbackResponse(query, cameras, logs)
    }
  }

  // Fallback response when Paraclate APP is unavailable
  // ローカルでの簡易応答（API接続失敗時のフォールバック）
  function generateFallbackResponse(
    query: string,
    cameras: Camera[],
    recentLogs: DetectionLog[]
  ): string {
    const lowerQuery = query.toLowerCase()

    // Help/usage query
    if (lowerQuery.includes('ヘルプ') || lowerQuery.includes('使い方') || lowerQuery.includes('help')) {
      return `AIアシスタントの使い方:
• 「今日は何かあった？」- 本日の検出状況を確認
• 「[カメラ名]の状態」- 特定カメラの状態を確認
• 「カメラ一覧」- 登録カメラの概要
• 「赤い服の人来なかった？」- 過去の記録を検索
• 「異常カメラ」- 問題のあるカメラを確認

※ Paraclate APP接続中は高度なAI分析が利用できます。`
    }

    // Camera list query
    if (lowerQuery.includes('カメラ一覧') || lowerQuery.includes('登録カメラ')) {
      const online = cameras.filter(c => c.enabled && c.polling_enabled).length
      const total = cameras.length
      return `登録カメラ: ${total}台（オンライン: ${online}台）\n\n${cameras.slice(0, 5).map(c =>
        `• ${c.name}: ${c.enabled ? '✅有効' : '❌無効'}`
      ).join('\n')}${cameras.length > 5 ? `\n...他${cameras.length - 5}台` : ''}`
    }

    // Detection summary query
    if (lowerQuery.includes('検出') || lowerQuery.includes('今日') || lowerQuery.includes('何かあった')) {
      const last24h = recentLogs.filter(log => {
        const logTime = new Date(log.captured_at).getTime()
        const now = Date.now()
        return now - logTime < 24 * 60 * 60 * 1000
      })
      const humanCount = last24h.filter(l => l.primary_event.toLowerCase().includes('human')).length
      const vehicleCount = last24h.filter(l => l.primary_event.toLowerCase().includes('vehicle')).length

      return `過去24時間の検出:
• 総検出: ${last24h.length}件
• 人物検知: ${humanCount}件
• 車両検知: ${vehicleCount}件

※ 詳細な分析にはParaclate APP接続が必要です。`
    }

    // Anomaly camera query
    if (lowerQuery.includes('異常') || lowerQuery.includes('問題')) {
      const disabledCams = cameras.filter(c => !c.enabled || !c.polling_enabled)
      if (disabledCams.length === 0) {
        return '現在、問題のあるカメラはありません。全カメラ正常稼働中です。'
      }
      return `問題のあるカメラ: ${disabledCams.length}台\n\n${disabledCams.map(c =>
        `• ${c.name}: ${!c.enabled ? 'カメラ無効' : 'ポーリング停止'}`
      ).join('\n')}`
    }

    // Default response - encourage Paraclate connection
    return `ご質問ありがとうございます。

Paraclate APP接続時は以下の高度な機能が利用できます:
• 「今日は何かあった？」→ 検出傾向のAI分析
• 「赤い服の人来なかった？」→ 過去記録の対話的検索
• カメラ間での人物追跡分析
• カメラ不調傾向の把握

設定タブでParaclate APPに接続してください。`
  }

  // Auto-scroll chat to bottom (including when loading indicator appears)
  useEffect(() => {
    if (chatScrollRef.current) {
      chatScrollRef.current.scrollTop = chatScrollRef.current.scrollHeight
    }
  }, [chatMessages, isChatLoading])

  // Note: Chat messages are now saved to backend API when added/modified
  // No localStorage persistence needed

  // Handle Summary/GrandSummary report from scheduler
  // Display as system message in chat
  useEffect(() => {
    if (!summaryReport) return

    // Dedupe: skip if we've already processed this summary
    if (processedSummaryIdsRef.current.has(summaryReport.summary_id)) {
      onSummaryReportConsumed?.()
      return
    }

    // Mark as processed
    processedSummaryIdsRef.current.add(summaryReport.summary_id)

    // LLMからのsummary_textをそのまま使用（余計なヘッダー追加は削除）
    // LLM側で適切にフォーマットされたテキストが返ってくる前提
    const content = summaryReport.summary_text

    const summaryMessage: ChatMessage = {
      // 決定的なID（全クライアントで同一）→ DB ON DUPLICATE KEY で重複防止
      id: `summary-${summaryReport.summary_id}`,
      role: "system",
      content,
      timestamp: new Date(summaryReport.created_at),
    }

    setChatMessages((prev) => [...prev, summaryMessage])
    saveChatMessageToAPI(summaryMessage)  // Save to backend

    // Notify parent that we've consumed the report
    onSummaryReportConsumed?.()
  }, [summaryReport, onSummaryReportConsumed])

  // Handle chat sync messages for cross-device real-time updates
  useEffect(() => {
    if (!chatSyncMessage) return

    const { action, message, message_id } = chatSyncMessage

    switch (action) {
      case "created":
        if (message) {
          // Add new message if not already exists
          setChatMessages((prev) => {
            if (prev.some((m) => m.id === message.message_id)) {
              return prev  // Already exists, skip
            }
            return [...prev, {
              id: message.message_id,
              role: message.role as "user" | "assistant" | "system",
              content: message.content,
              timestamp: new Date(message.timestamp),
              handled: message.handled,
              action: message.action_type ? {
                type: message.action_type as "preset_change",
                cameraId: message.action_camera_id,
                presetId: message.action_preset_id,
              } : undefined,
              dismissAt: message.dismiss_at,
              isHiding: message.is_hiding,
            }]
          })
        }
        break

      case "updated":
        if (message) {
          setChatMessages((prev) =>
            prev.map((m) =>
              m.id === message.message_id
                ? {
                    ...m,
                    handled: message.handled,
                    dismissAt: message.dismiss_at,
                    isHiding: message.is_hiding,
                  }
                : m
            )
          )
        }
        break

      case "deleted":
        if (message_id) {
          setChatMessages((prev) => prev.filter((m) => m.id !== message_id))
        }
        break

      case "cleared":
        setChatMessages([])
        break
    }

    onChatSyncConsumed?.()
  }, [chatSyncMessage, onChatSyncConsumed])

  // Fetch overdetection analysis and create suggestion messages
  useEffect(() => {
    if (checkedOverdetection || cameras.length === 0) return

    const checkOverdetection = async () => {
      try {
        // Check suggestion frequency setting
        const aiSettings = loadAIAssistantSettings()
        const frequency = aiSettings.suggestionFrequency

        // If frequency is 0 (OFF), skip suggestions entirely
        if (frequency === 0) {
          setCheckedOverdetection(true)
          return
        }

        const response = await fetch(`${API_BASE_URL}/api/stats/overdetection?period=24h`)
        const data = await response.json()

        if (!data.ok || !data.data?.cameras) return

        // Frequency-based severity filtering:
        // 1 (低): only critical
        // 2 (中): warning and critical
        // 3 (高): all issues including info
        const severityFilter = (severity: string) => {
          if (frequency === 1) return severity === 'critical'
          if (frequency === 2) return severity === 'warning' || severity === 'critical'
          return true // frequency === 3: all severities
        }

        const camerasWithIssues = data.data.cameras.filter(
          (c: { camera_id: string; issues: { severity: string }[] }) =>
            c.issues.some(i => severityFilter(i.severity))
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
          // Save each new message to backend
          newMessages.forEach(msg => saveChatMessageToAPI(msg))
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
        // Delete the suggestion message from backend and UI
        deleteChatMessageFromAPI(messageId)
        setChatMessages(prev => prev.filter(msg => msg.id !== messageId))
        // Add confirmation message
        const confirmMsg: ChatMessage = {
          id: `confirm-${Date.now()}`,
          role: "assistant",
          content: `プリセットを「${presetName}」に変更しました。`,
          timestamp: new Date(),
        }
        setChatMessages(prev => [...prev, confirmMsg])
        saveChatMessageToAPI(confirmMsg)
      } else {
        throw new Error('Failed to update preset')
      }
    } catch (err) {
      console.error('Failed to change preset:', err)
      const errorMsg: ChatMessage = {
        id: `error-${Date.now()}`,
        role: "assistant",
        content: "プリセットの変更に失敗しました。設定画面から手動で変更してください。",
        timestamp: new Date(),
      }
      setChatMessages(prev => [...prev, errorMsg])
      saveChatMessageToAPI(errorMsg)
    }
  }

  // Handle dismiss suggestion - sets 1 minute auto-dismiss timer
  const handleDismissSuggestion = (messageId: string) => {
    // Update the original message as handled
    setChatMessages(prev => prev.map(msg => {
      if (msg.id === messageId) {
        const updated = { ...msg, handled: true }
        updateChatMessageInAPI(updated)
        return updated
      }
      return msg
    }))
    // Add dismiss confirmation with auto-dismiss after 1 minute
    const dismissTime = Date.now() + 60000  // 1 minute from now
    const dismissMsg: ChatMessage = {
      id: `dismiss-${Date.now()}`,
      role: "assistant",
      content: "了解しました。現在の設定を維持します。",
      timestamp: new Date(),
      dismissAt: dismissTime,
    }
    setChatMessages(prev => [...prev, dismissMsg])
    saveChatMessageToAPI(dismissMsg)
  }

  // Auto-dismiss effect - check every second for messages to hide
  useEffect(() => {
    const interval = setInterval(() => {
      const now = Date.now()
      setChatMessages(prev => {
        const messagesToHide: ChatMessage[] = []
        const updated = prev.map(msg => {
          // Check if message should start hiding animation
          if (msg.dismissAt && !msg.isHiding && now >= msg.dismissAt) {
            const hiddenMsg = { ...msg, isHiding: true }
            messagesToHide.push(hiddenMsg)
            return hiddenMsg
          }
          return msg
        })
        // Update backend for messages that were just hidden
        messagesToHide.forEach(msg => updateChatMessageInAPI(msg))
        // Only update state if something changed
        if (messagesToHide.length > 0) {
          return updated
        }
        return prev
      })
    }, 1000)

    return () => clearInterval(interval)
  }, [])

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
        <div className="flex items-center justify-between p-2 border-b flex-shrink-0">
          <div className="flex items-center gap-2">
            <MessageCircle className="h-3 w-3 text-primary" />
            <span className="text-xs font-medium">AIアシスタント</span>
          </div>
          <button
            onClick={() => setIsChatExpandOpen(true)}
            className="p-1 rounded hover:bg-muted transition-colors"
            title="拡大表示"
          >
            <Maximize2 className="h-3.5 w-3.5 text-muted-foreground hover:text-primary" />
          </button>
        </div>

        {/* Chat History */}
        <div className="flex-1 overflow-auto p-2" ref={chatScrollRef}>
          {chatMessages.filter(m => !m.isHiding).length === 0 ? (
            <div className="text-center text-muted-foreground text-xs py-4">
              質問を入力してください
            </div>
          ) : (
            <div className="space-y-3">
              {chatMessages.filter(m => !m.isHiding).map((msg) => (
                <div
                  key={msg.id}
                  className={cn(
                    "flex",
                    msg.role === "user" ? "justify-end" : "justify-start items-start gap-2"
                  )}
                >
                  {/* AI Avatar - only for assistant/system messages */}
                  {msg.role !== "user" && (
                    <img
                      src={aichatIcon}
                      alt="AI"
                      className="w-8 h-8 flex-shrink-0 rounded-full bg-white p-0.5"
                    />
                  )}
                  {/* Message Bubble */}
                  <div
                    className={cn(
                      "max-w-[80%] text-xs px-3 py-2 rounded-2xl shadow-sm",
                      // User message: iOS blue background, white text, right tail
                      msg.role === "user" && "bg-[#007AFF] text-white rounded-tr-sm",
                      // System/Assistant message: light gray background, dark text, left tail
                      (msg.role === "system" || msg.role === "assistant") && "bg-[#F0F0F0] text-[#1A1A1A] rounded-tl-sm"
                    )}
                  >
                    {/* Message content with Markdown support */}
                    <div className="leading-relaxed">
                      <ReactMarkdown
                        components={{
                          // Style markdown elements for chat display
                          p: ({ children }) => <p className="mb-1.5 last:mb-0">{children}</p>,
                          ul: ({ children }) => <ul className="list-disc list-inside mb-1.5 ml-1">{children}</ul>,
                          ol: ({ children }) => <ol className="list-decimal list-inside mb-1.5 ml-1">{children}</ol>,
                          li: ({ children }) => <li className="mb-0.5">{children}</li>,
                          strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
                          h1: ({ children }) => <h1 className="text-xs font-bold mb-1">{children}</h1>,
                          h2: ({ children }) => <h2 className="text-xs font-bold mb-1">{children}</h2>,
                          h3: ({ children }) => <h3 className="text-[11px] font-bold mb-0.5">{children}</h3>,
                          code: ({ children }) => <code className="bg-gray-200 px-0.5 rounded text-[9px]">{children}</code>,
                          pre: ({ children }) => <pre className="bg-gray-200 p-1 rounded text-[9px] overflow-x-auto my-1">{children}</pre>,
                          a: ({ href, children }) => <a href={href} className="text-blue-600 underline" target="_blank" rel="noopener noreferrer">{children}</a>,
                          hr: () => <hr className="my-1 border-gray-300" />,
                        }}
                      >
                        {msg.content}
                      </ReactMarkdown>
                    </div>
                    {/* Timestamp */}
                    <div className={cn(
                      "text-[9px] mt-1",
                      msg.role === "user" ? "text-right text-white/70" : "text-right text-gray-500"
                    )}>
                      {msg.timestamp.toLocaleTimeString("ja-JP", { hour: "2-digit", minute: "2-digit" })}
                    </div>
                    {/* Action buttons for system messages with actionMeta */}
                    {msg.role === "system" && msg.actionMeta && !msg.handled && (
                      <div className="flex gap-2 mt-2 pt-2 border-t border-gray-300">
                        <button
                          onClick={() => handlePresetChange(msg.actionMeta!.cameraId, msg.actionMeta!.presetId, msg.id)}
                          className="flex-1 px-3 py-1.5 rounded-lg text-[11px] font-medium transition-colors bg-[#007AFF] hover:bg-[#0066CC] text-white"
                        >
                          はい
                        </button>
                        <button
                          onClick={() => handleDismissSuggestion(msg.id)}
                          className="flex-1 px-3 py-1.5 rounded-lg text-[11px] font-medium transition-colors bg-gray-300 hover:bg-gray-400 text-gray-700"
                        >
                          いいえ
                        </button>
                      </div>
                    )}
                    {/* Handled indicator */}
                    {msg.role === "system" && msg.handled && (
                      <div className="mt-1.5 text-[10px] text-gray-500 flex items-center gap-1">
                        <span>✓</span>
                        <span>対応済み</span>
                      </div>
                    )}
                  </div>
                </div>
              ))}
              {/* AI思考中インジケーター - 3点リーダーアニメーション */}
              {isChatLoading && (
                <div className="flex justify-start items-start gap-2">
                  <img
                    src={aichatIcon}
                    alt="AI"
                    className="w-8 h-8 flex-shrink-0 rounded-full bg-white p-0.5"
                  />
                  <div className="bg-[#F0F0F0] text-[#1A1A1A] rounded-2xl rounded-tl-sm px-4 py-3 shadow-sm">
                    <div className="flex gap-1">
                      <span className="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                      <span className="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                      <span className="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
                    </div>
                  </div>
                </div>
              )}
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

      {/* Chat Expand Modal */}
      <ChatExpandModal
        isOpen={isChatExpandOpen}
        onClose={() => setIsChatExpandOpen(false)}
        messages={chatMessages}
        onSend={handleChatSend}
        onPresetChange={handlePresetChange}
        onDismiss={handleDismissSuggestion}
        isLoading={isChatLoading}
      />
    </div>
  )
}
