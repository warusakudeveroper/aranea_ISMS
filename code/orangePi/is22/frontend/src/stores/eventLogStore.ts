/**
 * Event Log Store (Zustand)
 *
 * Manages:
 * - Detection logs from MySQL (via API)
 * - Patrol feedback (real-time "動いてる安心感")
 * - Filter/sort state
 */

import { create } from 'zustand'
import type { DetectionLog, DetectionLogStats, PatrolFeedback, Camera } from '@/types/api'
import type { SnapshotUpdatedMessage } from '@/hooks/useWebSocket'
import { API_BASE_URL } from '@/lib/config'

// Filter options
export interface EventLogFilter {
  severity_min: number | null
  camera_id: string | null
  detected_only: boolean
  limit: number
}

// Store state
interface EventLogState {
  // Detection logs (from API)
  logs: DetectionLog[]
  stats: DetectionLogStats | null
  isLoading: boolean
  error: string | null

  // Patrol feedback (real-time, ephemeral)
  patrolFeedback: PatrolFeedback[]

  // Filter
  filter: EventLogFilter

  // Camera name lookup (for patrol feedback display)
  cameraNames: Record<string, string>

  // Actions
  fetchLogs: (options?: Partial<EventLogFilter>) => Promise<void>
  fetchStats: () => Promise<void>
  addLog: (log: DetectionLog) => void
  addPatrolFeedback: (msg: SnapshotUpdatedMessage, cameras: Camera[]) => void
  setFilter: (filter: Partial<EventLogFilter>) => void
  setCameraNames: (cameras: Camera[]) => void
  clearLogs: () => void
  clearPatrolFeedback: () => void
}

// Default filter
const DEFAULT_FILTER: EventLogFilter = {
  severity_min: null,
  camera_id: null,
  detected_only: false,
  limit: 100,
}

// Max patrol feedback items to keep
const MAX_PATROL_FEEDBACK = 20

// Patrol feedback TTL (ms) - fade out after this time
export const PATROL_FEEDBACK_TTL_MS = 5000

export const useEventLogStore = create<EventLogState>((set, get) => ({
  logs: [],
  stats: null,
  isLoading: false,
  error: null,
  patrolFeedback: [],
  filter: DEFAULT_FILTER,
  cameraNames: {},

  fetchLogs: async (options) => {
    set({ isLoading: true, error: null })

    try {
      const filter = { ...get().filter, ...options }
      const params = new URLSearchParams()

      if (filter.limit) params.set('limit', filter.limit.toString())
      if (filter.camera_id) params.set('camera_id', filter.camera_id)
      if (filter.severity_min !== null) params.set('severity_min', filter.severity_min.toString())
      if (filter.detected_only) params.set('detected_only', 'true')

      const url = `${API_BASE_URL}/api/detection-logs?${params.toString()}`
      const response = await fetch(url)

      if (!response.ok) {
        throw new Error(`Failed to fetch logs: ${response.status}`)
      }

      // API returns: { ok: true, data: { logs: [...], filter: {...}, total: N }, error: null }
      const json = await response.json()
      const logs: DetectionLog[] = json.data?.logs || []
      set({ logs, isLoading: false, filter })
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : 'Unknown error',
        isLoading: false
      })
    }
  },

  fetchStats: async () => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/detection-logs/stats`)
      if (response.ok) {
        // API returns: { ok: true, data: { database: {...}, service_stats: {...} }, error: null }
        const json = await response.json()
        const dbStats = json.data?.database || {}
        const stats: DetectionLogStats = {
          total_logs: dbStats.total_logs || 0,
          detected_count: dbStats.today_logs || 0,
          detection_rate: 0,
          by_severity: {},
          by_event_type: {},
          pending_bq_sync: dbStats.pending_bq_sync || 0,
          last_detection_at: null,
        }
        set({ stats })
      }
    } catch (e) {
      console.error('Failed to fetch stats:', e)
    }
  },

  addLog: (log) => {
    set((state) => ({
      logs: [log, ...state.logs].slice(0, state.filter.limit),
    }))
  },

  addPatrolFeedback: (msg, cameras) => {
    const camera = cameras.find(c => c.camera_id === msg.camera_id)
    const cameraName = camera?.name || msg.camera_id.substring(0, 12)

    const feedback: PatrolFeedback = {
      camera_id: msg.camera_id,
      camera_name: cameraName,
      timestamp: msg.timestamp,
      primary_event: msg.primary_event,
      severity: msg.severity,
      is_detection: msg.primary_event !== null && msg.severity !== null && msg.severity > 0,
    }

    set((state) => ({
      patrolFeedback: [feedback, ...state.patrolFeedback].slice(0, MAX_PATROL_FEEDBACK),
    }))

    // Auto-cleanup old feedback after TTL
    setTimeout(() => {
      set((state) => ({
        patrolFeedback: state.patrolFeedback.filter(
          f => f.timestamp !== feedback.timestamp || f.camera_id !== feedback.camera_id
        ),
      }))
    }, PATROL_FEEDBACK_TTL_MS)
  },

  setFilter: (filter) => {
    set((state) => ({
      filter: { ...state.filter, ...filter },
    }))
  },

  setCameraNames: (cameras) => {
    const names: Record<string, string> = {}
    for (const c of cameras) {
      names[c.camera_id] = c.name
      // Also map lacis_id to name for detection log display
      if (c.lacis_id) {
        names[c.lacis_id] = c.name
      }
    }
    set({ cameraNames: names })
  },

  clearLogs: () => {
    set({ logs: [], error: null })
  },

  clearPatrolFeedback: () => {
    set({ patrolFeedback: [] })
  },
}))
