import { useState, useMemo, useCallback } from "react"
import type { Camera, SystemStatus, Event } from "@/types/api"
import { CameraGrid } from "@/components/CameraGrid"
import { SuggestPane } from "@/components/SuggestPane"
import { EventLogPane } from "@/components/EventLogPane"
import { ScanModal } from "@/components/ScanModal"
import { CameraDetailModal } from "@/components/CameraDetailModal"
import { useApi } from "@/hooks/useApi"
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

function App() {
  const [selectedCamera, setSelectedCamera] = useState<Camera | null>(null)
  const [cameraDetailOpen, setCameraDetailOpen] = useState(false)
  const [scanModalOpen, setScanModalOpen] = useState(false)

  const { data: cameras, loading: camerasLoading, refetch: refetchCameras } = useApi<Camera[]>(
    "/api/cameras",
    30000
  )

  const { data: systemStatus } = useApi<SystemStatus>(
    "/api/system/status",
    5000
  )

  // Mock events for now - will be replaced with WebSocket
  const { data: events } = useApi<Event[]>(
    "/api/events?limit=100",
    10000
  )

  // Get the most recent event for suggest pane
  const currentEvent = useMemo(() => {
    if (!events || events.length === 0) return null
    // Find the most recent event with severity > 0
    const significantEvent = events.find(e => e.severity > 0)
    return significantEvent || events[0]
  }, [events])

  // Get camera name for suggest pane
  const suggestCameraName = useMemo(() => {
    if (!currentEvent || !cameras) return undefined
    const camera = cameras.find(c => c.camera_id === currentEvent.camera_id)
    return camera?.name
  }, [currentEvent, cameras])

  // Camera card click - for future video modal playback
  const handleCameraClick = useCallback((camera: Camera) => {
    // TODO: Open video playback modal
    console.log('Camera clicked for video:', camera.camera_id)
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

  const handleEventClick = useCallback((event: Event) => {
    if (!cameras) return
    const camera = cameras.find(c => c.camera_id === event.camera_id)
    if (camera) {
      setSelectedCamera(camera)
      setCameraDetailOpen(true)
    }
  }, [cameras])

  return (
    <div className="h-screen flex flex-col">
      {/* Header */}
      <header className="h-14 border-b bg-card px-4 flex items-center justify-between flex-shrink-0">
        <div className="flex items-center gap-2">
          <Video className="h-6 w-6 text-primary" />
          <h1 className="text-lg font-semibold">mobes AI<span className="text-primary">cam</span> control Tower</h1>
          <Badge variant="outline" className="text-xs">mAcT</Badge>
        </div>
        <div className="flex items-center gap-4">
          {systemStatus && (
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
          <Button variant="ghost" size="icon">
            <Settings className="h-5 w-5" />
          </Button>
        </div>
      </header>

      {/* Main Content - 3 Pane Layout (IS22_UI_DETAILED_SPEC Section 1.1: 30%/45%/25%) */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Pane - Suggest View (30%) */}
        <aside className="w-[30%] min-w-[320px] max-w-[450px] border-r bg-card overflow-hidden">
          <SuggestPane
            currentEvent={currentEvent}
            cameraName={suggestCameraName}
          />
        </aside>

        {/* Center Pane - Camera Grid (45%) */}
        <main className="w-[45%] flex-1 p-4 overflow-auto">
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
            />
          ) : (
            <BlankCard onScanClick={() => setScanModalOpen(true)} />
          )}
        </main>

        {/* Right Pane - Event Log (25%) */}
        <aside className="w-[25%] min-w-[280px] max-w-[360px] border-l bg-card overflow-hidden">
          <EventLogPane
            events={events || []}
            onEventClick={handleEventClick}
          />
        </aside>
      </div>

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
    </div>
  )
}

export default App
