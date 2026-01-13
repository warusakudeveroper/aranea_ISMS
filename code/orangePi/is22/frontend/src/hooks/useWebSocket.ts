import { useEffect, useRef, useCallback, useState } from "react"
import { WS_BASE_URL } from "@/lib/config"

// Hub message types matching backend
// WebSocket is used for:
// - Event notifications (AI Event Log)
// - Snapshot update notifications (camera_id + timestamp only, NOT image data)
// Actual images are fetched via HTTP GET /api/snapshots/{camera_id}/latest.jpg

export interface EventLogMessage {
  event_id: number
  camera_id: string
  /** LacisID for camera lookup (BUG-001 fix) */
  lacis_id: string
  primary_event: string
  severity: number
  timestamp: string
}

// Snapshot update notification - triggers CameraGrid to fetch new image for this camera
export interface SnapshotUpdatedMessage {
  camera_id: string
  timestamp: string
  primary_event: string | null
  severity: number | null
  /** Processing time in milliseconds (capture + AI inference) */
  processing_ms?: number
  /** Error message if capture failed (timeout, network error, etc.) */
  error?: string
  /** Source of snapshot capture: "go2rtc" (from active stream), "ffmpeg" (direct RTSP), "http" (snapshot URL) */
  snapshot_source?: string
}

export interface SystemStatusMessage {
  healthy: boolean
  cpu_percent: number
  memory_percent: number
  active_streams: number
}

export interface CameraStatusMessage {
  camera_id: string
  online: boolean
  last_frame_at: string | null
}

// Polling cycle statistics - broadcast at end of each cycle
export interface CycleStatsMessage {
  subnet: string
  cycle_duration_sec: number
  cycle_duration_formatted: string  // "mm:ss"
  cameras_polled: number
  successful: number
  failed: number
  cycle_number: number
  completed_at: string
}

// Cooldown countdown during inter-cycle pause OR pre-cycle countdown
export interface CooldownTickMessage {
  subnet: string
  seconds_remaining: number
  total_cooldown_sec: number
  phase: string  // "pre_cycle" (3-2-1 countdown before cycle start) or "inter_cycle" (cooldown between cycles)
}

// Summary/GrandSummary report for AI Chat display
export interface SummaryReportMessage {
  report_type: string  // "summary" or "grand_summary"
  summary_id: number
  period_start: string  // ISO8601
  period_end: string    // ISO8601
  detection_count: number
  severity_max: number
  camera_count: number
  summary_text: string
  created_at: string
}

// Chat message data for WebSocket sync
export interface ChatMessageData {
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

// Chat message sync for cross-device real-time updates
export interface ChatSyncMessage {
  action: "created" | "updated" | "deleted" | "cleared"
  message?: ChatMessageData
  message_id?: string
}

export interface HubMessage {
  type: "event_log" | "suggest_update" | "system_status" | "camera_status" | "snapshot_updated" | "cycle_stats" | "cooldown_tick" | "summary_report" | "chat_sync"
  data: EventLogMessage | SystemStatusMessage | CameraStatusMessage | SnapshotUpdatedMessage | CycleStatsMessage | CooldownTickMessage | SummaryReportMessage | ChatSyncMessage | unknown
}

interface UseWebSocketOptions {
  onEventLog?: (msg: EventLogMessage) => void
  onSystemStatus?: (msg: SystemStatusMessage) => void
  onCameraStatus?: (msg: CameraStatusMessage) => void
  onSnapshotUpdated?: (msg: SnapshotUpdatedMessage) => void
  onCycleStats?: (msg: CycleStatsMessage) => void
  onCooldownTick?: (msg: CooldownTickMessage) => void
  onSummaryReport?: (msg: SummaryReportMessage) => void
  onChatSync?: (msg: ChatSyncMessage) => void
  onMessage?: (msg: HubMessage) => void
  reconnectInterval?: number
}

export function useWebSocket(options: UseWebSocketOptions = {}) {
  const { reconnectInterval = 3000 } = options
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [connected, setConnected] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Store callbacks in refs to avoid reconnection on callback changes
  // This prevents the infinite reconnect loop when inline callbacks are passed
  const callbacksRef = useRef(options)
  callbacksRef.current = options

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return
    }

    try {
      const ws = new WebSocket(`${WS_BASE_URL}/api/ws`)

      ws.onopen = () => {
        console.log("[WS] Connected")
        setConnected(true)
        setError(null)
      }

      ws.onmessage = (event) => {
        try {
          const msg: HubMessage = JSON.parse(event.data)
          const callbacks = callbacksRef.current

          // Route message to appropriate handler
          if (msg.type === "event_log" && callbacks.onEventLog) {
            callbacks.onEventLog(msg.data as EventLogMessage)
          } else if (msg.type === "system_status" && callbacks.onSystemStatus) {
            callbacks.onSystemStatus(msg.data as SystemStatusMessage)
          } else if (msg.type === "camera_status" && callbacks.onCameraStatus) {
            callbacks.onCameraStatus(msg.data as CameraStatusMessage)
          } else if (msg.type === "snapshot_updated" && callbacks.onSnapshotUpdated) {
            callbacks.onSnapshotUpdated(msg.data as SnapshotUpdatedMessage)
          } else if (msg.type === "cycle_stats" && callbacks.onCycleStats) {
            callbacks.onCycleStats(msg.data as CycleStatsMessage)
          } else if (msg.type === "cooldown_tick" && callbacks.onCooldownTick) {
            callbacks.onCooldownTick(msg.data as CooldownTickMessage)
          } else if (msg.type === "summary_report" && callbacks.onSummaryReport) {
            callbacks.onSummaryReport(msg.data as SummaryReportMessage)
          } else if (msg.type === "chat_sync" && callbacks.onChatSync) {
            callbacks.onChatSync(msg.data as ChatSyncMessage)
          }

          // Generic message handler
          if (callbacks.onMessage) {
            callbacks.onMessage(msg)
          }
        } catch (e) {
          console.error("[WS] Failed to parse message:", e)
        }
      }

      ws.onclose = (event) => {
        console.log("[WS] Disconnected:", event.code, event.reason)
        setConnected(false)
        wsRef.current = null

        // Reconnect after delay
        if (reconnectTimeoutRef.current) {
          clearTimeout(reconnectTimeoutRef.current)
        }
        reconnectTimeoutRef.current = setTimeout(connect, reconnectInterval)
      }

      ws.onerror = (event) => {
        console.error("[WS] Error:", event)
        setError("WebSocket connection error")
      }

      wsRef.current = ws
    } catch (e) {
      console.error("[WS] Failed to connect:", e)
      setError(e instanceof Error ? e.message : "Connection failed")

      // Retry after delay
      reconnectTimeoutRef.current = setTimeout(connect, reconnectInterval)
    }
  }, [reconnectInterval]) // Only reconnectInterval in deps - callbacks are in ref

  useEffect(() => {
    connect()

    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
      }
      if (wsRef.current) {
        wsRef.current.close()
      }
    }
  }, [connect])

  return { connected, error }
}
