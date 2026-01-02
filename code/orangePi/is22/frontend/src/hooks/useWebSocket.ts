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

export interface HubMessage {
  type: "event_log" | "suggest_update" | "system_status" | "camera_status" | "snapshot_updated"
  data: EventLogMessage | SystemStatusMessage | CameraStatusMessage | SnapshotUpdatedMessage | unknown
}

interface UseWebSocketOptions {
  onEventLog?: (msg: EventLogMessage) => void
  onSystemStatus?: (msg: SystemStatusMessage) => void
  onCameraStatus?: (msg: CameraStatusMessage) => void
  onSnapshotUpdated?: (msg: SnapshotUpdatedMessage) => void
  onMessage?: (msg: HubMessage) => void
  reconnectInterval?: number
}

export function useWebSocket(options: UseWebSocketOptions = {}) {
  const { onEventLog, onSystemStatus, onCameraStatus, onSnapshotUpdated, onMessage, reconnectInterval = 3000 } = options
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [connected, setConnected] = useState(false)
  const [error, setError] = useState<string | null>(null)

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

          // Route message to appropriate handler
          if (msg.type === "event_log" && onEventLog) {
            onEventLog(msg.data as EventLogMessage)
          } else if (msg.type === "system_status" && onSystemStatus) {
            onSystemStatus(msg.data as SystemStatusMessage)
          } else if (msg.type === "camera_status" && onCameraStatus) {
            onCameraStatus(msg.data as CameraStatusMessage)
          } else if (msg.type === "snapshot_updated" && onSnapshotUpdated) {
            onSnapshotUpdated(msg.data as SnapshotUpdatedMessage)
          }

          // Generic message handler
          if (onMessage) {
            onMessage(msg)
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
  }, [onEventLog, onSystemStatus, onCameraStatus, onSnapshotUpdated, onMessage, reconnectInterval])

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
