import { useState, useCallback, useRef, useEffect } from "react"
import type { Camera, SystemStatus, DetectionLog } from "@/types/api"
import { CameraGrid, type CameraStatus } from "@/components/CameraGrid"
import { useWebSocket, type EventLogMessage, type SnapshotUpdatedMessage, type CycleStatsMessage, type CooldownTickMessage, type SummaryReportMessage, type ChatSyncMessage } from "@/hooks/useWebSocket"
import { SuggestPane } from "@/components/SuggestPane"
import { EventLogPane } from "@/components/EventLogPane"
import { ScanModal } from "@/components/ScanModal"
import { CameraDetailModal } from "@/components/CameraDetailModal"
import { LiveViewModal } from "@/components/LiveViewModal"
import { SettingsModal } from "@/components/SettingsModal"
import { FloatingAIButton } from "@/components/FloatingAIButton"
import { MobileDrawer } from "@/components/MobileDrawer"
import { useApi } from "@/hooks/useApi"
import { useEventLogStore } from "@/stores/eventLogStore"
import { useIsMobile } from "@/hooks/useMediaQuery"
import { API_BASE_URL } from "@/lib/config"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent } from "@/components/ui/card"
import {
  Camera as CameraIcon,
  Activity,
  Settings,
  Video,
  RefreshCw,
  Search,
  Wifi,
  WifiOff,
} from "lucide-react"

// Blank card component for empty state (IS22_UI_DETAILED_SPEC Section 2.2)
function BlankCard({ onScanClick }: { onScanClick: () => void }) {
  return (
    <div className="flex items-center justify-center h-full">
      <Card className="max-w-md w-full">
        <CardContent className="flex flex-col items-center p-8">
          <div className="relative mb-6">
            <div className="relative flex h-20 w-20 items-center justify-center rounded-full bg-muted">
              <CameraIcon className="h-10 w-10 text-muted-foreground" />
            </div>
          </div>
          <h3 className="text-xl font-semibold mb-2">カメラが登録されていません</h3>
          <p className="text-sm text-muted-foreground text-center mb-6">
            ONVIF / RTSP対応カメラを自動検出します
          </p>
          <Button size="lg" className="w-full" onClick={onScanClick}>
            <Search className="mr-2 h-5 w-5" />
            カメラをスキャン
          </Button>
        </CardContent>
      </Card>
    </div>
  )
}

// Default onairtime in seconds
const DEFAULT_ONAIRTIME_SECONDS = 180

function App() {
  // Issue #108: モバイル判定
  const isMobile = useIsMobile()

  const [selectedCamera, setSelectedCamera] = useState<Camera | null>(null)
  const [cameraDetailOpen, setCameraDetailOpen] = useState(false)
  const [liveViewOpen, setLiveViewOpen] = useState(false)
  const [scanModalOpen, setScanModalOpen] = useState(false)
  const [settingsModalOpen, setSettingsModalOpen] = useState(false)
  // Issue #108: モバイルドロワー状態
  const [mobileDrawerOpen, setMobileDrawerOpen] = useState(false)
  const [hasUnreadEvents, setHasUnreadEvents] = useState(false)
  // Per-camera snapshot timestamps (from WebSocket notifications)
  // Key: camera_id, Value: Unix timestamp (ms)
  const [snapshotTimestamps, setSnapshotTimestamps] = useState<Record<string, number>>({})
  // Per-camera status (processing time, errors) for slow/timeout display
  const [cameraStatuses, setCameraStatuses] = useState<Record<string, CameraStatus>>({})
  // Per-subnet cycle statistics (from WebSocket notifications)
  // Key: subnet (e.g., "192.168.125.0/24"), Value: CycleStatsMessage
  const [cycleStats, setCycleStats] = useState<Record<string, CycleStatsMessage>>({})
  // On-air camera IDs (for tile highlighting)
  const [onAirCameraIds, setOnAirCameraIds] = useState<string[]>([])
  // On-air time setting (persisted to localStorage)
  const [onAirTimeSeconds, setOnAirTimeSeconds] = useState<number>(() => {
    const saved = localStorage.getItem("onairtime_seconds")
    return saved ? parseInt(saved, 10) : DEFAULT_ONAIRTIME_SECONDS
  })

  // Patrol execution state for EventLogPane (single responsibility: App manages WebSocket state)
  const [cooldownSeconds, setCooldownSeconds] = useState<number>(0)
  const [executionState, setExecutionState] = useState<"idle" | "accessing" | "cooldown" | "waiting">("waiting")

  // FIX-002: Real-time event from WebSocket only (NOT from fetched historical logs)
  // This ensures page reload doesn't trigger video playback in SuggestPane
  const [realtimeEvent, setRealtimeEvent] = useState<DetectionLog | null>(null)

  // Summary/GrandSummary report from scheduler for AI Chat display
  const [latestSummaryReport, setLatestSummaryReport] = useState<SummaryReportMessage | null>(null)

  // Chat sync message for cross-device real-time updates
  const [latestChatSync, setLatestChatSync] = useState<ChatSyncMessage | null>(null)

  // Persist onairtime to localStorage
  useEffect(() => {
    localStorage.setItem("onairtime_seconds", String(onAirTimeSeconds))
  }, [onAirTimeSeconds])

  // Issue #108: ドロワーが開いたら未読をクリア
  useEffect(() => {
    if (mobileDrawerOpen) {
      setHasUnreadEvents(false)
    }
  }, [mobileDrawerOpen])

  // Grid container ref for measuring dimensions (no-scroll layout)
  const gridContainerRef = useRef<HTMLDivElement>(null)
  const [gridContainerHeight, setGridContainerHeight] = useState<number>(0)
  const [gridContainerWidth, setGridContainerWidth] = useState<number>(0)

  // Measure grid container dimensions for tile sizing
  useEffect(() => {
    const updateDimensions = () => {
      if (gridContainerRef.current) {
        setGridContainerHeight(gridContainerRef.current.clientHeight)
        setGridContainerWidth(gridContainerRef.current.clientWidth)
      }
    }
    updateDimensions()
    window.addEventListener('resize', updateDimensions)
    return () => window.removeEventListener('resize', updateDimensions)
  }, [])

  // Event log store for patrol feedback and log fetching
  // Note: EventLogPane accesses logs directly from the store
  const { addPatrolFeedback, fetchLogs } = useEventLogStore()

  // Fetch detection logs on mount (for SuggestPane currentEvent)
  useEffect(() => {
    fetchLogs({ detected_only: true, severity_min: 1, limit: 100 })
  }, [fetchLogs])

  // Initialize snapshot timestamps from backend on page load
  // This fetches the CURRENT state from detection_logs (most recent captures)
  useEffect(() => {
    const initializeFromBackend = async () => {
      try {
        const response = await fetch(`${API_BASE_URL}/api/detection-logs?limit=500`)
        if (!response.ok) {
          console.error('[App] Failed to fetch detection logs:', response.status)
          return
        }

        const result = await response.json()
        // API returns { ok: true, data: { filter: {...}, logs: [...] } }
        const logs = (result.data?.logs || result.data || result) as DetectionLog[]

        // Extract latest captured_at for each camera
        const latestTimestamps: Record<string, number> = {}
        logs.forEach(log => {
          const timestamp = new Date(log.captured_at).getTime()
          const existing = latestTimestamps[log.camera_id]
          if (!existing || timestamp > existing) {
            latestTimestamps[log.camera_id] = timestamp
          }
        })

        setSnapshotTimestamps(latestTimestamps)
      } catch (error) {
        console.error('[App] Failed to initialize from backend:', error)
      }
    }

    initializeFromBackend()
  }, []) // Run once on mount

  // Camera list (needed for patrol feedback camera names)
  const { data: cameras, loading: camerasLoading, refetch: refetchCameras } = useApi<Camera[]>(
    "/api/cameras",
    30000
  )

  // Handle real-time event log updates from WebSocket
  // FIX-002: Convert to DetectionLog and set as realtime event (NOT from fetched logs)
  // BUG-001 fix: Use lacis_id directly from WebSocket message (no camera lookup needed)
  const handleEventLog = useCallback((msg: EventLogMessage) => {
    // Only process events with severity > 0 (AI detections)
    if (msg.severity > 0) {
      // BUG-001 fix: lacis_id is now included in EventLogMessage from backend
      // No need to lookup from cameras cache (which could be stale or miss new cameras)

      // Create DetectionLog for SuggestPane
      // SuggestPane uses: log_id, lacis_id, severity, primary_event
      const realtimeLog: DetectionLog = {
        log_id: msg.event_id,
        tid: "",
        fid: "",
        camera_id: msg.camera_id,
        lacis_id: msg.lacis_id || null,  // BUG-001: Use lacis_id from message
        captured_at: msg.timestamp,
        analyzed_at: msg.timestamp,
        primary_event: msg.primary_event,
        severity: msg.severity,
        confidence: 0,
        count_hint: 0,
        tags: [],
        unknown_flag: false,
        bboxes: null,
        person_details: null,
        suspicious: null,
        frame_diff: null,
        loitering_detected: false,
        preset_id: null,
        preset_version: null,
        output_schema: null,
        context_applied: false,
        camera_context: null,
        is21_log: {},
        image_path_local: "",
        image_path_cloud: null,
        processing_ms: null,
        polling_cycle_id: null,
        schema_version: "1.0",
        timings: null,
        synced_to_bq: false,
        synced_at: null,
        created_at: msg.timestamp,
      }
      setRealtimeEvent(realtimeLog)

      // Issue #108: モバイルでドロワーが閉じている場合は未読フラグをセット
      if (isMobile && !mobileDrawerOpen) {
        setHasUnreadEvents(true)
      }
    }

    // Also refetch logs for EventLogPane display (historical list)
    fetchLogs({ detected_only: true, severity_min: 1, limit: 100 })
  }, [fetchLogs, isMobile, mobileDrawerOpen])  // BUG-001: Removed cameras dependency (lacis_id now from message)

  // Handle snapshot update notifications from WebSocket
  // Each camera is notified individually when its snapshot is updated
  const handleSnapshotUpdated = useCallback((msg: SnapshotUpdatedMessage) => {
    // Update camera status (processing time, errors)
    setCameraStatuses((prev) => ({
      ...prev,
      [msg.camera_id]: {
        processingMs: msg.processing_ms,
        error: msg.error,
      },
    }))

    // Update execution state for EventLogPane patrol ticker
    setExecutionState("accessing")
    // Auto-reset to waiting after a delay
    setTimeout(() => {
      setExecutionState(prev => prev === "accessing" ? "waiting" : prev)
    }, 2000)

    // Update snapshot timestamp from backend (RFC3339 string -> Unix timestamp ms)
    // CRITICAL: Use backend timestamp, NOT Date.now() (frontend time)
    const backendTimestamp = new Date(msg.timestamp).getTime()
    setSnapshotTimestamps((prev) => ({
      ...prev,
      [msg.camera_id]: backendTimestamp,
    }))

    // Add to patrol feedback for "動いてる安心感" (only for successful captures)
    if (!msg.error && cameras) {
      addPatrolFeedback(msg, cameras)
    }
  }, [cameras, addPatrolFeedback])

  // Handle cooldown tick from WebSocket (for EventLogPane patrol ticker)
  const handleCooldownTick = useCallback((msg: CooldownTickMessage) => {
    setCooldownSeconds(msg.seconds_remaining)
    if (msg.seconds_remaining > 0) {
      setExecutionState("cooldown")
    } else {
      setExecutionState("waiting")
    }
  }, [])

  // Handle cycle stats from polling orchestrator (parallel subnets)
  // Handle cycle statistics updates from WebSocket
  // Each subnet broadcasts independently with its own cycle timing
  const handleCycleStats = useCallback((msg: CycleStatsMessage) => {
    setCycleStats((prev) => ({
      ...prev,
      [msg.subnet]: msg,
    }))
  }, [])

  // Handle summary report from scheduler for AI Chat display
  const handleSummaryReport = useCallback((msg: SummaryReportMessage) => {
    console.log('[WS] Summary report received:', msg.report_type, msg.summary_id)
    setLatestSummaryReport(msg)
  }, [])

  // Handle chat sync for cross-device real-time updates
  const handleChatSync = useCallback((msg: ChatSyncMessage) => {
    console.log('[WS] Chat sync received:', msg.action, msg.message_id || msg.message?.message_id)
    setLatestChatSync(msg)
  }, [])

  // WebSocket connection for real-time notifications (single connection point)
  const { connected: wsConnected } = useWebSocket({
    onEventLog: handleEventLog,
    onSnapshotUpdated: handleSnapshotUpdated,
    onCycleStats: handleCycleStats,
    onCooldownTick: handleCooldownTick,
    onSummaryReport: handleSummaryReport,
    onChatSync: handleChatSync,
  })

  const { data: systemStatus } = useApi<SystemStatus>(
    "/api/system/status",
    5000
  )

  // FIX-002: currentEvent for SuggestPane is from realtime WebSocket events only
  // NOT from fetched logs (which include historical data)
  // This ensures page reload doesn't trigger video playback in SuggestPane
  const currentEvent = realtimeEvent

  // Handle on-air camera changes from SuggestPane
  const handleOnAirChange = useCallback((cameraIds: string[]) => {
    setOnAirCameraIds(cameraIds)
  }, [])

  // Camera card click - open live view modal for video streaming
  const handleCameraClick = useCallback((camera: Camera) => {
    setSelectedCamera(camera)
    setLiveViewOpen(true)
  }, [])

  // Settings icon click - open camera detail modal
  const handleSettingsClick = useCallback((camera: Camera) => {
    setSelectedCamera(camera)
    setCameraDetailOpen(true)
  }, [])

  const handleCameraSave = useCallback((updatedCamera: Camera) => {
    // Refresh camera list after save
    refetchCameras()
    setSelectedCamera(updatedCamera)
  }, [refetchCameras])

  const handleCameraDelete = useCallback((_cameraId: string, _hard: boolean) => {
    // Refresh camera list after delete
    refetchCameras()
    setSelectedCamera(null)
    setCameraDetailOpen(false)
  }, [refetchCameras])

  // Issue #108: ヘッダーコンポーネント（モバイル/デスクトップ共通）
  const renderHeader = () => (
    <header className="h-14 border-b bg-card px-4 flex items-center justify-between flex-shrink-0">
      <div className="flex items-center gap-2">
        <Video className="h-6 w-6 text-primary" />
        {/* モバイルでは短縮タイトル */}
        {isMobile ? (
          <h1 className="text-lg font-semibold">mAcT</h1>
        ) : (
          <>
            <h1 className="text-lg font-semibold">Paraclate</h1>
            <Badge variant="outline" className="text-xs">mobes AI control Tower</Badge>
          </>
        )}
      </div>
      <div className="flex items-center gap-4">
        {/* WebSocket connection status + Polling cycle stats */}
        <div className="flex items-center gap-2 text-sm">
          {wsConnected ? (
            <>
              <Wifi className="h-4 w-4 text-green-500" />
              <span className="text-green-500 text-xs">LIVE</span>
              {/* Subnet-specific cycle times and camera counts - デスクトップのみ表示 */}
              {!isMobile && Object.entries(cycleStats).length > 0 && (
                <div className="flex items-center gap-1">
                  <span className="text-muted-foreground text-xs">巡回</span>
                  {Object.entries(cycleStats)
                    .sort(([a], [b]) => a.localeCompare(b))
                    .map(([subnet, stats], idx) => {
                      // Format "192.168.125" -> "125.0/24"
                      const shortSubnet = subnet.split('.')[2] + '.0/24'
                      return (
                        <span key={subnet} className="text-xs">
                          {idx > 0 && <span className="text-muted-foreground mx-1">|</span>}
                          {shortSubnet}: {stats.cycle_duration_formatted} ({stats.cameras_polled}台)
                        </span>
                      )
                    })}
                </div>
              )}
            </>
          ) : (
            <>
              <WifiOff className="h-4 w-4 text-muted-foreground" />
              <span className="text-muted-foreground text-xs">Offline</span>
            </>
          )}
        </div>
        {/* システムステータス - デスクトップのみ表示 */}
        {!isMobile && systemStatus && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Activity className="h-4 w-4" />
            <span>CPU: {systemStatus.cpu_percent.toFixed(1)}%</span>
            <span>MEM: {systemStatus.memory_percent.toFixed(1)}%</span>
            <span>Modals: {systemStatus.active_modals}/{systemStatus.modal_budget_remaining + systemStatus.active_modals}</span>
            {systemStatus.healthy ? (
              <Badge variant="secondary">Healthy</Badge>
            ) : (
              <Badge variant="destructive">Unhealthy</Badge>
            )}
          </div>
        )}
        <Button variant="ghost" size="icon" onClick={() => setSettingsModalOpen(true)}>
          <Settings className="h-5 w-5" />
        </Button>
      </div>
    </header>
  )

  // Issue #108: モーダル群（共通）
  const renderModals = () => (
    <>
      {/* Scan Modal */}
      <ScanModal
        open={scanModalOpen}
        onOpenChange={setScanModalOpen}
        onDevicesRegistered={(count) => {
          console.log(`Registered ${count} devices`)
          refetchCameras()
        }}
      />

      {/* Camera Detail Modal */}
      <CameraDetailModal
        open={cameraDetailOpen}
        onOpenChange={setCameraDetailOpen}
        camera={selectedCamera}
        onSave={handleCameraSave}
        onDelete={handleCameraDelete}
      />

      {/* Live View Modal */}
      <LiveViewModal
        open={liveViewOpen}
        onOpenChange={setLiveViewOpen}
        camera={selectedCamera}
        onSettingsClick={() => {
          setLiveViewOpen(false)
          setCameraDetailOpen(true)
        }}
      />

      {/* Settings Modal */}
      <SettingsModal
        open={settingsModalOpen}
        onOpenChange={setSettingsModalOpen}
        onAirTimeSeconds={onAirTimeSeconds}
        onOnAirTimeSecondsChange={setOnAirTimeSeconds}
      />
    </>
  )

  // Issue #108: モバイルレイアウト
  if (isMobile) {
    return (
      <div className="h-screen flex flex-col">
        {renderHeader()}

        {/* Main Content - 縦積みレイアウト、全体スクロール */}
        <div
          ref={gridContainerRef}
          className="flex-1 overflow-y-auto mobile-scrollbar-hide"
        >
          {/* SuggestView - 30vh、スクロール対象 */}
          <div className="h-[30vh] min-h-[30vh] overflow-hidden border-b">
            <SuggestPane
              currentEvent={currentEvent}
              cameras={cameras || []}
              onAirTimeSeconds={onAirTimeSeconds}
              onOnAirChange={handleOnAirChange}
              isMobile={true}
            />
          </div>

          {/* CameraGrid - スクロール対象 */}
          <div className="p-2">
            {camerasLoading ? (
              <div className="flex items-center justify-center h-[50vh]">
                <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
              </div>
            ) : cameras && cameras.length > 0 ? (
              <CameraGrid
                cameras={cameras}
                onCameraClick={handleCameraClick}
                onSettingsClick={handleSettingsClick}
                onAddClick={() => setScanModalOpen(true)}
                snapshotTimestamps={snapshotTimestamps}
                cameraStatuses={cameraStatuses}
                fallbackPollingEnabled={!wsConnected}
                fallbackPollingIntervalMs={30000}
                animationEnabled={true}
                animationStyle="slide-down"
                containerHeight={undefined}  // モバイルでは無制限
                containerWidth={gridContainerWidth}
                onAirCameraIds={onAirCameraIds}
                isMobile={true}
                columns={3}
                allowScroll={true}
              />
            ) : (
              <BlankCard onScanClick={() => setScanModalOpen(true)} />
            )}
          </div>
        </div>

        {/* Floating AI Button */}
        <FloatingAIButton
          onClick={() => setMobileDrawerOpen(!mobileDrawerOpen)}
          hasNewEvents={hasUnreadEvents}
          isDrawerOpen={mobileDrawerOpen}
        />

        {/* Mobile Drawer - AI Event Log */}
        <MobileDrawer
          open={mobileDrawerOpen}
          onClose={() => setMobileDrawerOpen(false)}
          title="AI Event Log"
        >
          <EventLogPane
            cameras={cameras || []}
            cooldownSeconds={cooldownSeconds}
            executionState={executionState}
            isMobile={true}
            summaryReport={latestSummaryReport}
            onSummaryReportConsumed={() => setLatestSummaryReport(null)}
            chatSyncMessage={latestChatSync}
            onChatSyncConsumed={() => setLatestChatSync(null)}
          />
        </MobileDrawer>

        {renderModals()}
      </div>
    )
  }

  // デスクトップレイアウト（既存）
  return (
    <div className="h-screen flex flex-col">
      {renderHeader()}

      {/* Main Content - 3 Pane Layout: 30%/55%/15% */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Pane - Suggest View (30%) */}
        <aside className="w-[30%] border-r overflow-hidden flex flex-col">
          <SuggestPane
            currentEvent={currentEvent}
            cameras={cameras || []}
            onAirTimeSeconds={onAirTimeSeconds}
            onOnAirChange={handleOnAirChange}
          />
        </aside>

        {/* Center Pane - Camera Grid (55%) - No scroll, tiles compress to fit */}
        <main
          ref={gridContainerRef}
          className="w-[55%] p-4 overflow-hidden flex flex-col"
        >
          {camerasLoading ? (
            <div className="flex items-center justify-center h-full">
              <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : cameras && cameras.length > 0 ? (
            <CameraGrid
              cameras={cameras}
              onCameraClick={handleCameraClick}
              onSettingsClick={handleSettingsClick}
              onAddClick={() => setScanModalOpen(true)}
              snapshotTimestamps={snapshotTimestamps}
              cameraStatuses={cameraStatuses}
              fallbackPollingEnabled={!wsConnected}
              fallbackPollingIntervalMs={30000}
              animationEnabled={true}
              animationStyle="slide-down"
              containerHeight={gridContainerHeight}
              containerWidth={gridContainerWidth}
              onAirCameraIds={onAirCameraIds}
            />
          ) : (
            <BlankCard onScanClick={() => setScanModalOpen(true)} />
          )}
        </main>

        {/* Right Pane - Event Log (15%) */}
        <aside
          className="w-[15%] border-l bg-card overflow-hidden flex flex-col relative z-10 isolate"
          onClick={(e) => e.stopPropagation()}
        >
          <EventLogPane
            cameras={cameras || []}
            cooldownSeconds={cooldownSeconds}
            executionState={executionState}
            summaryReport={latestSummaryReport}
            onSummaryReportConsumed={() => setLatestSummaryReport(null)}
            chatSyncMessage={latestChatSync}
            onChatSyncConsumed={() => setLatestChatSync(null)}
          />
        </aside>
      </div>

      {renderModals()}
    </div>
  )
}

export default App
