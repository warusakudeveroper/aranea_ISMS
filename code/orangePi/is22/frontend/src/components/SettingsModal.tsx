/**
 * SettingsModal - System Settings and Monitoring Dashboard
 *
 * Tab structure (following is20s pattern):
 * - IS21接続: AI inference server connection settings
 * - システム: Device info, hardware status
 * - パフォーマンス: Performance metrics and logs
 * - 巡回ログ: Polling cycle logs
 * - ダッシュボード: Performance charts and rankings
 */

import { useState, useEffect, useCallback } from "react"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Slider } from "@/components/ui/slider"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Server,
  Activity,
  Clock,
  RefreshCw,
  CheckCircle2,
  XCircle,
  Cpu,
  HardDrive,
  Thermometer,
  Database,
  Wifi,
  Camera,
  AlertTriangle,
  Settings2,
  TrendingUp,
  ListFilter,
  Gauge,
  ChevronDown,
  ChevronRight,
  Code,
  Video,
  Tags,
  Bell,
  Search,
  Bot,
  Link2Off,
  FileText,
  Sliders,
} from "lucide-react"
import { API_BASE_URL } from "@/lib/config"
import { PerformanceDashboard } from "@/components/PerformanceDashboard"
import { CameraBrandsSettings } from "@/components/CameraBrandsSettings"
import { SdmSettingsTab } from "@/components/SdmSettingsTab"
import { InferenceStatsTab } from "@/components/InferenceStatsTab"
import { PresetSelector } from "@/components/PresetSelector"
import type { Camera as CameraType } from "@/types/api"

interface SettingsModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** On-air time in seconds for SuggestPane */
  onAirTimeSeconds?: number
  /** Callback when on-air time changes */
  onOnAirTimeSecondsChange?: (seconds: number) => void
}

// Types for API responses
interface IS21Status {
  connected: boolean
  url: string
  uptime_sec?: number
  par_enabled?: boolean
  last_check?: string
}

interface SystemInfo {
  hostname: string
  device_type: string
  lacis_id: string
  version: string
  uptime: string
  cpu_percent: number
  memory_percent: number
  memory_used_mb: number
  memory_total_mb: number
  disk_percent: number
  disk_used_gb: number
  disk_total_gb: number
  temperature?: number
  camera_count: number
  is21_url: string
}

interface PerformanceLog {
  timestamp: string
  camera_id: string
  camera_name?: string
  processing_ms: number
  is21_inference_ms?: number
  yolo_ms?: number
  par_ms?: number
  primary_event: string
  severity: number
  // Raw IS21 log for debugging (contains full IS21 response + IS22 timings)
  is21_log_raw?: string
  // Snapshot source: "go2rtc" (from active stream), "ffmpeg" (direct RTSP), "http" (snapshot URL)
  snapshot_source?: string
}

interface PollingLog {
  polling_id: string
  subnet: string
  subnet_octet3: number
  started_at: string
  completed_at: string | null
  cycle_number: number
  camera_count: number
  success_count: number
  failed_count: number
  timeout_count: number
  duration_ms: number | null
  avg_processing_ms: number | null
  status: string
}

// Storage settings (AIEventlog.md T1-6)
interface StorageSettings {
  config: {
    max_images_per_camera: number
    max_bytes_per_camera: number
    max_bytes_per_camera_mb: number
    max_total_bytes: number
    max_total_bytes_gb: number
  }
  usage: {
    total_bytes: number
    total_mb: number
    total_logs: number
    unknown_logs: number
    camera_count: number
    usage_percent: number
  }
}

// AI Assistant settings (Paraclate module placeholder)
interface AIAssistantSettings {
  suggestionFrequency: number  // 0=OFF, 1=低, 2=中, 3=高
  paraclate: {
    reportIntervalMinutes: number
    grandSummaryTimes: string[]
    reportDetail: "concise" | "standard" | "detailed"
    instantAlertOnAnomaly: boolean
    autoTuningEnabled: boolean
    tuningFrequency: "daily" | "weekly" | "monthly"
    tuningAggressiveness: number  // 0-100
    contextNote: string
  }
}

const AI_ASSISTANT_SETTINGS_KEY = "ai_assistant_settings"

const defaultAIAssistantSettings: AIAssistantSettings = {
  suggestionFrequency: 2,  // 中
  paraclate: {
    reportIntervalMinutes: 60,
    grandSummaryTimes: ["09:00", "17:00", "21:00"],
    reportDetail: "standard",
    instantAlertOnAnomaly: true,
    autoTuningEnabled: true,
    tuningFrequency: "weekly",
    tuningAggressiveness: 50,
    contextNote: "",
  },
}

export function loadAIAssistantSettings(): AIAssistantSettings {
  try {
    const stored = localStorage.getItem(AI_ASSISTANT_SETTINGS_KEY)
    if (stored) {
      return { ...defaultAIAssistantSettings, ...JSON.parse(stored) }
    }
  } catch (e) {
    console.error("Failed to load AI assistant settings:", e)
  }
  return defaultAIAssistantSettings
}

function saveAIAssistantSettings(settings: AIAssistantSettings): void {
  try {
    localStorage.setItem(AI_ASSISTANT_SETTINGS_KEY, JSON.stringify(settings))
  } catch (e) {
    console.error("Failed to save AI assistant settings:", e)
  }
}

export function SettingsModal({
  open,
  onOpenChange,
  onAirTimeSeconds = 180,
  onOnAirTimeSecondsChange,
}: SettingsModalProps) {
  const [activeTab, setActiveTab] = useState("display")
  const [is21Status, setIs21Status] = useState<IS21Status | null>(null)
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null)
  const [performanceLogs, setPerformanceLogs] = useState<PerformanceLog[]>([])
  const [pollingLogs, setPollingLogs] = useState<PollingLog[]>([])
  const [loading, setLoading] = useState(false)
  const [is21Url, setIs21Url] = useState("")
  const [saving, setSaving] = useState(false)
  // Expanded log entry index for showing raw IS21 log
  const [expandedLogIndex, setExpandedLogIndex] = useState<number | null>(null)
  // Global timeout settings
  const [timeoutMainSec, setTimeoutMainSec] = useState<number>(10)
  const [timeoutSubSec, setTimeoutSubSec] = useState<number>(20)
  const [savingTimeouts, setSavingTimeouts] = useState(false)

  // Storage settings (AIEventlog.md T1-6)
  const [storageSettings, setStorageSettings] = useState<StorageSettings | null>(null)
  const [storageMaxImages, setStorageMaxImages] = useState<number>(1000)
  const [storageMaxMb, setStorageMaxMb] = useState<number>(500)
  const [storageMaxTotalGb, setStorageMaxTotalGb] = useState<number>(10)
  const [savingStorage, setSavingStorage] = useState(false)
  const [cleaningStorage, setCleaningStorage] = useState(false)

  // Camera list for diagnostics (T3-3)
  const [cameras, setCameras] = useState<CameraType[]>([])

  // AI Assistant settings (Paraclate placeholder)
  const [aiSettings, setAiSettings] = useState<AIAssistantSettings>(() => loadAIAssistantSettings())

  // Fetch cameras list for diagnostics
  const fetchCameras = useCallback(async () => {
    try {
      const resp = await fetch(`${API_BASE_URL}/api/cameras`)
      if (resp.ok) {
        const json = await resp.json()
        const cameraList = json.data || json || []
        setCameras(cameraList)
      }
    } catch (error) {
      console.error("Failed to fetch cameras:", error)
    }
  }, [])

  // Fetch IS21 status
  const fetchIS21Status = useCallback(async () => {
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/is21/status`)
      if (resp.ok) {
        const json = await resp.json()
        // Backend returns {ok: true, data: {...}}
        const data = json.data || json
        setIs21Status(data)
        setIs21Url(data.url || "")
      }
    } catch (error) {
      console.error("Failed to fetch IS21 status:", error)
    }
  }, [])

  // Fetch system info
  const fetchSystemInfo = useCallback(async () => {
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/system`)
      if (resp.ok) {
        const json = await resp.json()
        // Backend returns {ok: true, data: {...}}
        const data = json.data || json
        setSystemInfo(data)
      }
    } catch (error) {
      console.error("Failed to fetch system info:", error)
    }
  }, [])

  // Fetch performance logs
  const fetchPerformanceLogs = useCallback(async () => {
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/performance/logs?limit=50`)
      if (resp.ok) {
        const json = await resp.json()
        // Backend returns {ok: true, data: {logs: [...]}}
        const data = json.data || json
        setPerformanceLogs(data.logs || [])
      }
    } catch (error) {
      console.error("Failed to fetch performance logs:", error)
    }
  }, [])

  // Fetch polling logs
  const fetchPollingLogs = useCallback(async () => {
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/polling/logs?limit=30`)
      if (resp.ok) {
        const json = await resp.json()
        // Backend returns {ok: true, data: {logs: [...]}}
        const data = json.data || json
        setPollingLogs(data.logs || [])
      }
    } catch (error) {
      console.error("Failed to fetch polling logs:", error)
    }
  }, [])

  // Fetch global timeout settings
  const fetchTimeoutSettings = useCallback(async () => {
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/timeouts`)
      if (resp.ok) {
        const json = await resp.json()
        const data = json.data || json
        if (data.timeout_main_sec) setTimeoutMainSec(data.timeout_main_sec)
        if (data.timeout_sub_sec) setTimeoutSubSec(data.timeout_sub_sec)
      }
    } catch (error) {
      console.error("Failed to fetch timeout settings:", error)
    }
  }, [])

  // Fetch storage settings (AIEventlog.md T1-6)
  const fetchStorageSettings = useCallback(async () => {
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/storage`)
      if (resp.ok) {
        const json = await resp.json()
        if (json.ok) {
          setStorageSettings({ config: json.config, usage: json.usage })
          setStorageMaxImages(json.config.max_images_per_camera)
          setStorageMaxMb(json.config.max_bytes_per_camera_mb)
          setStorageMaxTotalGb(json.config.max_total_bytes_gb)
        }
      }
    } catch (error) {
      console.error("Failed to fetch storage settings:", error)
    }
  }, [])

  // Save storage settings
  const handleSaveStorage = async () => {
    setSavingStorage(true)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/storage`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          max_images_per_camera: storageMaxImages,
          max_bytes_per_camera: storageMaxMb * 1024 * 1024,
          max_total_bytes: storageMaxTotalGb * 1024 * 1024 * 1024,
        }),
      })
      if (resp.ok) {
        await fetchStorageSettings()
      }
    } catch (error) {
      console.error("Failed to save storage settings:", error)
    } finally {
      setSavingStorage(false)
    }
  }

  // Trigger storage cleanup
  const handleStorageCleanup = async () => {
    setCleaningStorage(true)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/storage/cleanup`, {
        method: "POST",
      })
      if (resp.ok) {
        await fetchStorageSettings()
      }
    } catch (error) {
      console.error("Failed to cleanup storage:", error)
    } finally {
      setCleaningStorage(false)
    }
  }

  // Unknown cleanup preview data (v5: Rule 5準拠 - プレビュー→確認→実行フロー)
  const [cleanupPreview, setCleanupPreview] = useState<{
    total: number
    to_delete: number
    to_keep: number
  } | null>(null)
  const [showCleanupConfirm, setShowCleanupConfirm] = useState(false)

  // Step 1: Get cleanup preview (confirmed=false)
  const handleUnknownCleanupPreview = async () => {
    setCleaningStorage(true)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/storage/cleanup-unknown?confirmed=false`, {
        method: "POST",
      })
      if (resp.ok) {
        const json = await resp.json()
        if (json.preview) {
          setCleanupPreview(json.preview)
          setShowCleanupConfirm(true)
        }
      }
    } catch (error) {
      console.error("Failed to get cleanup preview:", error)
    } finally {
      setCleaningStorage(false)
    }
  }

  // Step 2: Execute cleanup after confirmation (confirmed=true)
  const handleUnknownCleanupExecute = async () => {
    setShowCleanupConfirm(false)
    setCleaningStorage(true)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/storage/cleanup-unknown?confirmed=true`, {
        method: "POST",
      })
      if (resp.ok) {
        await fetchStorageSettings()
        setCleanupPreview(null)
      }
    } catch (error) {
      console.error("Failed to cleanup unknown images:", error)
    } finally {
      setCleaningStorage(false)
    }
  }

  // ============================================
  // T3-3, T3-6: 診断・復旧UI (Phase 3)
  // ============================================

  // Diagnostics state
  const [diagnostics, setDiagnostics] = useState<{
    camera_id: string
    total_in_db: number
    existing_on_disk: number
    missing_on_disk: number
    missing_paths: string[]
    total_size_bytes: number
    diagnosis: string
  } | null>(null)
  const [diagLoading, setDiagLoading] = useState(false)
  const [selectedCameraForDiag, setSelectedCameraForDiag] = useState<string>("")

  // Recovery state
  const [recoveryMode, setRecoveryMode] = useState<"purge_orphans" | "refetch_current" | "fix_path_only">("purge_orphans")
  const [recoveryResult, setRecoveryResult] = useState<{
    attempted: number
    succeeded: number
    failed: number
  } | null>(null)
  const [recovering, setRecovering] = useState(false)

  // T3-3: Run diagnostics for a camera
  const handleRunDiagnostics = async (cameraId: string) => {
    if (!cameraId) return
    setDiagLoading(true)
    setDiagnostics(null)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/diagnostics/images/${encodeURIComponent(cameraId)}`)
      if (resp.ok) {
        const json = await resp.json()
        if (json.ok && json.diagnostics) {
          setDiagnostics(json.diagnostics)
        }
      }
    } catch (error) {
      console.error("Failed to run diagnostics:", error)
    } finally {
      setDiagLoading(false)
    }
  }

  // T3-6: Run recovery for a camera
  const handleRunRecovery = async () => {
    if (!diagnostics?.camera_id || diagnostics.missing_on_disk === 0) return
    setRecovering(true)
    setRecoveryResult(null)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/recovery/images/${encodeURIComponent(diagnostics.camera_id)}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ mode: recoveryMode }),
      })
      if (resp.ok) {
        const json = await resp.json()
        if (json.ok && json.recovery) {
          setRecoveryResult(json.recovery)
          // Re-run diagnostics to see updated state
          await handleRunDiagnostics(diagnostics.camera_id)
        }
      }
    } catch (error) {
      console.error("Failed to run recovery:", error)
    } finally {
      setRecovering(false)
    }
  }

  // Save IS21 URL
  const handleSaveIS21 = async () => {
    setSaving(true)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/is21`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: is21Url }),
      })
      if (resp.ok) {
        await fetchIS21Status()
      }
    } catch (error) {
      console.error("Failed to save IS21 settings:", error)
    } finally {
      setSaving(false)
    }
  }

  // Test IS21 connection
  const handleTestIS21 = async () => {
    setLoading(true)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/is21/test`, {
        method: "POST",
      })
      if (resp.ok) {
        await fetchIS21Status()
      }
    } catch (error) {
      console.error("Failed to test IS21 connection:", error)
    } finally {
      setLoading(false)
    }
  }

  // Save global timeout settings
  const handleSaveTimeouts = async () => {
    setSavingTimeouts(true)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/timeouts`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          timeout_main_sec: timeoutMainSec,
          timeout_sub_sec: timeoutSubSec,
        }),
      })
      if (resp.ok) {
        await fetchTimeoutSettings()
      }
    } catch (error) {
      console.error("Failed to save timeout settings:", error)
    } finally {
      setSavingTimeouts(false)
    }
  }

  // Load data when modal opens or tab changes
  useEffect(() => {
    if (!open) return

    const fetchData = async () => {
      setLoading(true)
      try {
        switch (activeTab) {
          case "display":
            await fetchTimeoutSettings()
            break
          case "is21":
            await fetchIS21Status()
            break
          case "system":
            await fetchSystemInfo()
            break
          case "storage":
            await Promise.all([fetchStorageSettings(), fetchCameras()])
            break
          case "performance":
            await fetchPerformanceLogs()
            break
          case "polling":
            await fetchPollingLogs()
            break
        }
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [open, activeTab, fetchIS21Status, fetchSystemInfo, fetchPerformanceLogs, fetchPollingLogs, fetchTimeoutSettings, fetchStorageSettings, fetchCameras])

  // Auto-refresh for active tabs
  useEffect(() => {
    if (!open) return

    const interval = setInterval(() => {
      switch (activeTab) {
        case "display":
          fetchTimeoutSettings()
          break
        case "is21":
          fetchIS21Status()
          break
        case "system":
          fetchSystemInfo()
          break
        case "storage":
          fetchStorageSettings()
          break
        case "performance":
          fetchPerformanceLogs()
          break
        case "polling":
          fetchPollingLogs()
          break
      }
    }, 5000)

    return () => clearInterval(interval)
  }, [open, activeTab, fetchIS21Status, fetchSystemInfo, fetchPerformanceLogs, fetchPollingLogs, fetchTimeoutSettings, fetchStorageSettings])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-6xl h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Settings2 className="h-5 w-5" />
            システム設定
          </DialogTitle>
        </DialogHeader>

        <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col overflow-hidden">
          <TabsList className="grid w-full grid-cols-11">
            <TabsTrigger value="display" className="flex items-center gap-1 text-xs">
              <Video className="h-4 w-4" />
              表示
            </TabsTrigger>
            <TabsTrigger value="preset" className="flex items-center gap-1 text-xs">
              <Settings2 className="h-4 w-4" />
              検出
            </TabsTrigger>
            <TabsTrigger value="sdm" className="flex items-center gap-1 text-xs">
              <Bell className="h-4 w-4" />
              Nest
            </TabsTrigger>
            <TabsTrigger value="is21" className="flex items-center gap-1 text-xs">
              <Server className="h-4 w-4" />
              IS21
            </TabsTrigger>
            <TabsTrigger value="system" className="flex items-center gap-1 text-xs">
              <Cpu className="h-4 w-4" />
              システム
            </TabsTrigger>
            <TabsTrigger value="storage" className="flex items-center gap-1 text-xs">
              <HardDrive className="h-4 w-4" />
              保存
            </TabsTrigger>
            <TabsTrigger value="performance" className="flex items-center gap-1 text-xs">
              <TrendingUp className="h-4 w-4" />
              ログ
            </TabsTrigger>
            <TabsTrigger value="polling" className="flex items-center gap-1 text-xs">
              <ListFilter className="h-4 w-4" />
              巡回
            </TabsTrigger>
            <TabsTrigger value="dashboard" className="flex items-center gap-1 text-xs">
              <Gauge className="h-4 w-4" />
              統計
            </TabsTrigger>
            <TabsTrigger value="ai" className="flex items-center gap-1 text-xs">
              <Bot className="h-4 w-4" />
              AI
            </TabsTrigger>
            <TabsTrigger value="brands" className="flex items-center gap-1 text-xs">
              <Tags className="h-4 w-4" />
              ブランド
            </TabsTrigger>
          </TabsList>

          {/* Display Settings Tab */}
          <TabsContent value="display" className="flex-1 overflow-auto mt-4">
            <div className="space-y-4">
              {/* On-Air Time Setting */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <Video className="h-4 w-4" />
                    サジェストビュー設定
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="onairtime">オンエア時間（秒）</Label>
                    <div className="flex items-center gap-4">
                      <Input
                        id="onairtime"
                        type="number"
                        min={30}
                        max={600}
                        step={10}
                        value={onAirTimeSeconds}
                        onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                          const value = parseInt(e.target.value, 10)
                          if (!isNaN(value) && value >= 30 && value <= 600) {
                            onOnAirTimeSecondsChange?.(value)
                          }
                        }}
                        className="w-32"
                      />
                      <span className="text-sm text-muted-foreground">
                        現在: {onAirTimeSeconds}秒 ({Math.floor(onAirTimeSeconds / 60)}分{onAirTimeSeconds % 60}秒)
                      </span>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      AI検知イベント発生後、サジェストビューで動画を再生する時間。
                      最後のイベント検知からこの時間が経過すると自動終了します。
                    </p>
                  </div>
                </CardContent>
              </Card>

              {/* Quick Presets */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">クイックプリセット</CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="flex flex-wrap gap-2">
                    {[60, 120, 180, 300, 600].map((seconds) => (
                      <Button
                        key={seconds}
                        variant={onAirTimeSeconds === seconds ? "default" : "outline"}
                        size="sm"
                        onClick={() => onOnAirTimeSecondsChange?.(seconds)}
                      >
                        {seconds < 60 ? `${seconds}秒` : `${seconds / 60}分`}
                      </Button>
                    ))}
                  </div>
                </CardContent>
              </Card>

              {/* Global Timeout Settings */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <Clock className="h-4 w-4" />
                    グローバルタイムアウト設定
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="timeout-main">メインストリームタイムアウト（秒）</Label>
                    <div className="flex items-center gap-4">
                      <Input
                        id="timeout-main"
                        type="number"
                        min={5}
                        max={120}
                        step={1}
                        value={timeoutMainSec}
                        onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                          const value = parseInt(e.target.value, 10)
                          if (!isNaN(value) && value >= 5 && value <= 120) {
                            setTimeoutMainSec(value)
                          }
                        }}
                        className="w-32"
                      />
                      <span className="text-sm text-muted-foreground">現在: {timeoutMainSec}秒</span>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      メインストリーム（高画質）のスナップショット取得タイムアウト時間。5〜120秒の範囲で設定可能。
                    </p>
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="timeout-sub">サブストリームタイムアウト（秒）</Label>
                    <div className="flex items-center gap-4">
                      <Input
                        id="timeout-sub"
                        type="number"
                        min={5}
                        max={120}
                        step={1}
                        value={timeoutSubSec}
                        onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                          const value = parseInt(e.target.value, 10)
                          if (!isNaN(value) && value >= 5 && value <= 120) {
                            setTimeoutSubSec(value)
                          }
                        }}
                        className="w-32"
                      />
                      <span className="text-sm text-muted-foreground">現在: {timeoutSubSec}秒</span>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      サブストリーム（低画質フォールバック）のスナップショット取得タイムアウト時間。5〜120秒の範囲で設定可能。
                    </p>
                  </div>

                  <Button onClick={handleSaveTimeouts} disabled={savingTimeouts}>
                    {savingTimeouts ? (
                      <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                    ) : null}
                    タイムアウト設定を保存
                  </Button>
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          {/* Preset Graphical UI Tab (Issue #107) */}
          <TabsContent value="preset" className="flex-1 overflow-auto mt-4">
            <div className="space-y-4">
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <Settings2 className="h-4 w-4" />
                    プリセット設定・過剰検出分析
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <p className="text-xs text-muted-foreground mb-4">
                    検出プリセットの選択と、過剰検出の傾向分析を行います。
                    プリセット変更時はバランスがアニメーションで表示されます。
                  </p>
                  <PresetSelector
                    cameraId="global"
                    currentPresetId="balanced"
                    onPresetChange={(presetId) => {
                      console.log('Preset changed to:', presetId)
                      // TODO: Apply preset change to cameras
                    }}
                    onOpenThresholdSettings={() => {
                      setActiveTab('storage')
                    }}
                    apiBaseUrl={API_BASE_URL}
                  />
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          {/* SDM (Google/Nest) Settings Tab */}
          <TabsContent value="sdm" className="flex-1 overflow-hidden mt-4">
            <SdmSettingsTab />
          </TabsContent>

          {/* IS21 Connection Tab */}
          <TabsContent value="is21" className="flex-1 overflow-auto mt-4">
            <div className="space-y-4">
              {/* Connection Status Card */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <Activity className="h-4 w-4" />
                    接続状態
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="flex items-center gap-4">
                    {is21Status?.connected ? (
                      <>
                        <CheckCircle2 className="h-8 w-8 text-green-500" />
                        <div>
                          <p className="font-medium text-green-600">接続中</p>
                          <p className="text-sm text-muted-foreground">
                            {is21Status.url}
                          </p>
                          {is21Status.uptime_sec && (
                            <p className="text-xs text-muted-foreground">
                              Uptime: {Math.floor(is21Status.uptime_sec / 60)}分
                              {is21Status.par_enabled && " | PAR有効"}
                            </p>
                          )}
                        </div>
                      </>
                    ) : (
                      <>
                        <XCircle className="h-8 w-8 text-red-500" />
                        <div>
                          <p className="font-medium text-red-600">未接続</p>
                          <p className="text-sm text-muted-foreground">
                            IS21 AI推論サーバーに接続できません
                          </p>
                        </div>
                      </>
                    )}
                  </div>
                </CardContent>
              </Card>

              {/* Connection Settings Card */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <Server className="h-4 w-4" />
                    IS21サーバー設定
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="is21-url">サーバーURL</Label>
                    <Input
                      id="is21-url"
                      placeholder="http://192.168.3.240:9000"
                      value={is21Url}
                      onChange={(e: React.ChangeEvent<HTMLInputElement>) => setIs21Url(e.target.value)}
                    />
                    <p className="text-xs text-muted-foreground">
                      IS21 AI推論サーバーのエンドポイントURLを指定してください
                    </p>
                  </div>
                  <div className="flex gap-2">
                    <Button onClick={handleSaveIS21} disabled={saving}>
                      {saving ? (
                        <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                      ) : null}
                      保存
                    </Button>
                    <Button variant="outline" onClick={handleTestIS21} disabled={loading}>
                      {loading ? (
                        <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                      ) : (
                        <Wifi className="h-4 w-4 mr-2" />
                      )}
                      接続テスト
                    </Button>
                  </div>
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          {/* System Info Tab */}
          <TabsContent value="system" className="flex-1 overflow-auto mt-4">
            <div className="space-y-4">
              {/* Device Info Card */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">デバイス情報</CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground uppercase">HOSTNAME</p>
                      <p className="font-medium">{systemInfo?.hostname || "-"}</p>
                    </div>
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground uppercase">TYPE</p>
                      <p className="font-medium">{systemInfo?.device_type || "is22"}</p>
                    </div>
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground uppercase">LACIS ID</p>
                      <p className="font-mono text-xs">{systemInfo?.lacis_id || "-"}</p>
                    </div>
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground uppercase">VERSION</p>
                      <p className="font-medium">{systemInfo?.version || "-"}</p>
                    </div>
                  </div>
                </CardContent>
              </Card>

              {/* Hardware Status Card */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <Activity className="h-4 w-4" />
                    ハードウェア状態
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                    <div className="bg-muted/50 p-3 rounded-lg">
                      <div className="flex items-center gap-2 mb-1">
                        <Cpu className="h-4 w-4 text-muted-foreground" />
                        <span className="text-xs text-muted-foreground uppercase">CPU</span>
                      </div>
                      <p className={`text-lg font-bold ${
                        (systemInfo?.cpu_percent || 0) > 80 ? "text-red-500" : "text-green-600"
                      }`}>
                        {systemInfo?.cpu_percent?.toFixed(1) || "0"}%
                      </p>
                    </div>
                    <div className="bg-muted/50 p-3 rounded-lg">
                      <div className="flex items-center gap-2 mb-1">
                        <Database className="h-4 w-4 text-muted-foreground" />
                        <span className="text-xs text-muted-foreground uppercase">MEMORY</span>
                      </div>
                      <p className={`text-lg font-bold ${
                        (systemInfo?.memory_percent || 0) > 85 ? "text-red-500" : "text-green-600"
                      }`}>
                        {systemInfo?.memory_used_mb || 0}MB / {systemInfo?.memory_total_mb || 0}MB
                      </p>
                      <p className="text-xs text-muted-foreground">
                        ({systemInfo?.memory_percent?.toFixed(1) || "0"}%)
                      </p>
                    </div>
                    <div className="bg-muted/50 p-3 rounded-lg">
                      <div className="flex items-center gap-2 mb-1">
                        <Thermometer className="h-4 w-4 text-muted-foreground" />
                        <span className="text-xs text-muted-foreground uppercase">TEMP</span>
                      </div>
                      <p className={`text-lg font-bold ${
                        (systemInfo?.temperature || 0) > 70 ? "text-red-500" : "text-green-600"
                      }`}>
                        {systemInfo?.temperature?.toFixed(1) || "-"}°C
                      </p>
                    </div>
                    <div className="bg-muted/50 p-3 rounded-lg">
                      <div className="flex items-center gap-2 mb-1">
                        <HardDrive className="h-4 w-4 text-muted-foreground" />
                        <span className="text-xs text-muted-foreground uppercase">DISK</span>
                      </div>
                      <p className={`text-lg font-bold ${
                        (systemInfo?.disk_percent || 0) > 90 ? "text-red-500" : "text-green-600"
                      }`}>
                        {systemInfo?.disk_used_gb?.toFixed(1) || "0"}GB / {systemInfo?.disk_total_gb?.toFixed(1) || "0"}GB
                      </p>
                      <p className="text-xs text-muted-foreground">
                        ({systemInfo?.disk_percent?.toFixed(1) || "0"}%)
                      </p>
                    </div>
                  </div>
                </CardContent>
              </Card>

              {/* Service Status Card */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <Camera className="h-4 w-4" />
                    サービス状態
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground uppercase">登録カメラ</p>
                      <p className="text-lg font-bold">{systemInfo?.camera_count || 0}台</p>
                    </div>
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground uppercase">IS21 URL</p>
                      <p className="font-mono text-xs">{systemInfo?.is21_url || "-"}</p>
                    </div>
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground uppercase">UPTIME</p>
                      <p className="font-medium">{systemInfo?.uptime || "-"}</p>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          {/* Storage Settings Tab (AIEventlog.md T1-6) */}
          <TabsContent value="storage" className="flex-1 overflow-auto mt-4">
            <div className="space-y-4">
              {/* Storage Usage Card */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <HardDrive className="h-4 w-4" />
                    ストレージ使用状況
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  {storageSettings ? (
                    <div className="space-y-4">
                      {/* Usage Progress */}
                      <div className="space-y-2">
                        <div className="flex justify-between text-sm">
                          <span>使用量</span>
                          <span className="font-mono">
                            {storageSettings.usage.total_mb} MB / {storageSettings.config.max_total_bytes_gb} GB
                            ({storageSettings.usage.usage_percent}%)
                          </span>
                        </div>
                        <div className="w-full bg-secondary rounded-full h-2">
                          <div
                            className={`h-2 rounded-full ${
                              storageSettings.usage.usage_percent > 90
                                ? "bg-red-500"
                                : storageSettings.usage.usage_percent > 70
                                ? "bg-yellow-500"
                                : "bg-green-500"
                            }`}
                            style={{ width: `${Math.min(storageSettings.usage.usage_percent, 100)}%` }}
                          />
                        </div>
                      </div>

                      {/* Stats Grid */}
                      <div className="grid grid-cols-3 gap-4 pt-2">
                        <div className="space-y-1">
                          <p className="text-xs text-muted-foreground uppercase">総ログ数</p>
                          <p className="text-lg font-bold">{storageSettings.usage.total_logs}</p>
                        </div>
                        <div className="space-y-1">
                          <p className="text-xs text-muted-foreground uppercase">unknown画像</p>
                          <p className="text-lg font-bold text-amber-600">{storageSettings.usage.unknown_logs}</p>
                        </div>
                        <div className="space-y-1">
                          <p className="text-xs text-muted-foreground uppercase">カメラ数</p>
                          <p className="text-lg font-bold">{storageSettings.usage.camera_count}</p>
                        </div>
                      </div>
                    </div>
                  ) : (
                    <p className="text-sm text-muted-foreground">読み込み中...</p>
                  )}
                </CardContent>
              </Card>

              {/* Storage Quota Settings Card */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <Database className="h-4 w-4" />
                    保存制限設定
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="grid grid-cols-3 gap-4">
                    <div className="space-y-2">
                      <Label htmlFor="maxImages">カメラ毎の最大枚数</Label>
                      <Input
                        id="maxImages"
                        type="number"
                        min={10}
                        max={100000}
                        value={storageMaxImages}
                        onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                          const v = parseInt(e.target.value, 10)
                          if (!isNaN(v)) setStorageMaxImages(v)
                        }}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="maxMb">カメラ毎の最大MB</Label>
                      <Input
                        id="maxMb"
                        type="number"
                        min={10}
                        max={100000}
                        value={storageMaxMb}
                        onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                          const v = parseInt(e.target.value, 10)
                          if (!isNaN(v)) setStorageMaxMb(v)
                        }}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="maxTotalGb">全体の最大GB</Label>
                      <Input
                        id="maxTotalGb"
                        type="number"
                        min={1}
                        max={1000}
                        value={storageMaxTotalGb}
                        onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                          const v = parseInt(e.target.value, 10)
                          if (!isNaN(v)) setStorageMaxTotalGb(v)
                        }}
                      />
                    </div>
                  </div>
                  <div className="flex justify-end">
                    <Button onClick={handleSaveStorage} disabled={savingStorage}>
                      {savingStorage ? (
                        <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                      ) : (
                        <CheckCircle2 className="h-4 w-4 mr-2" />
                      )}
                      保存
                    </Button>
                  </div>
                </CardContent>
              </Card>

              {/* Cleanup Actions Card */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <AlertTriangle className="h-4 w-4" />
                    手動クリーンアップ
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <p className="text-sm text-muted-foreground">
                    古い画像を削除してストレージを解放します。
                  </p>
                  <div className="flex gap-4">
                    <Button
                      variant="outline"
                      onClick={handleStorageCleanup}
                      disabled={cleaningStorage}
                    >
                      {cleaningStorage ? (
                        <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                      ) : (
                        <HardDrive className="h-4 w-4 mr-2" />
                      )}
                      クォータ強制
                    </Button>
                    <Button
                      variant="destructive"
                      onClick={handleUnknownCleanupPreview}
                      disabled={cleaningStorage}
                    >
                      {cleaningStorage ? (
                        <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                      ) : (
                        <XCircle className="h-4 w-4 mr-2" />
                      )}
                      unknown画像削除（要確認）
                    </Button>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    「unknown画像削除」は最新10%を保持し、古い90%を削除します。削除前に確認ダイアログが表示されます。
                  </p>

                  {/* Cleanup Confirmation Dialog (v5: Rule 5準拠) */}
                  {showCleanupConfirm && cleanupPreview && (
                    <div className="mt-4 p-4 border border-destructive rounded-lg bg-destructive/10">
                      <h4 className="font-bold text-destructive mb-2">削除確認</h4>
                      <p className="text-sm mb-2">
                        以下のunknown画像を削除しますか？この操作は取り消せません。
                      </p>
                      <ul className="text-sm space-y-1 mb-4">
                        <li>対象画像数: <strong>{cleanupPreview.total}</strong> 枚</li>
                        <li>削除予定: <strong className="text-destructive">{cleanupPreview.to_delete}</strong> 枚 (古い90%)</li>
                        <li>保持予定: <strong className="text-green-600">{cleanupPreview.to_keep}</strong> 枚 (最新10%)</li>
                      </ul>
                      <div className="flex gap-2">
                        <Button
                          variant="destructive"
                          size="sm"
                          onClick={handleUnknownCleanupExecute}
                        >
                          削除を実行
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => {
                            setShowCleanupConfirm(false)
                            setCleanupPreview(null)
                          }}
                        >
                          キャンセル
                        </Button>
                      </div>
                    </div>
                  )}
                </CardContent>
              </Card>

              {/* T3-3, T3-6: Image Diagnostics & Recovery Card */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <Database className="h-4 w-4" />
                    画像診断・復旧
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <p className="text-sm text-muted-foreground">
                    カメラごとのunknown画像の整合性を診断し、欠損ファイルを検出します。
                  </p>

                  {/* Camera Selection */}
                  <div className="flex gap-2 items-end">
                    <div className="flex-1">
                      <Label htmlFor="diag-camera" className="text-xs">対象カメラ</Label>
                      <select
                        id="diag-camera"
                        className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm"
                        value={selectedCameraForDiag}
                        onChange={(e) => setSelectedCameraForDiag(e.target.value)}
                      >
                        <option value="">カメラを選択...</option>
                        {cameras.map((cam) => (
                          <option key={cam.camera_id} value={cam.camera_id}>
                            {cam.name || cam.camera_id}
                          </option>
                        ))}
                      </select>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleRunDiagnostics(selectedCameraForDiag)}
                      disabled={diagLoading || !selectedCameraForDiag}
                    >
                      {diagLoading ? (
                        <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                      ) : (
                        <Search className="h-4 w-4 mr-2" />
                      )}
                      診断実行
                    </Button>
                  </div>

                  {/* Diagnostics Result */}
                  {diagnostics && (
                    <div className="p-3 rounded-lg bg-muted/50 space-y-2">
                      <h4 className="font-medium text-sm">診断結果: {diagnostics.camera_id}</h4>
                      <div className="grid grid-cols-2 gap-2 text-sm">
                        <div>DB登録数: <strong>{diagnostics.total_in_db}</strong></div>
                        <div>ファイル存在: <strong className="text-green-600">{diagnostics.existing_on_disk}</strong></div>
                        <div className={diagnostics.missing_on_disk > 0 ? "text-destructive" : ""}>
                          欠損数: <strong>{diagnostics.missing_on_disk}</strong>
                        </div>
                        <div>合計サイズ: <strong>{(diagnostics.total_size_bytes / 1024 / 1024).toFixed(2)} MB</strong></div>
                      </div>
                      <div className={`text-xs ${diagnostics.missing_on_disk > 0 ? "text-destructive" : "text-green-600"}`}>
                        {diagnostics.diagnosis}
                      </div>

                      {/* Recovery Options (only if missing files) */}
                      {diagnostics.missing_on_disk > 0 && (
                        <div className="mt-4 pt-4 border-t space-y-3">
                          <h5 className="font-medium text-sm">復旧オプション</h5>
                          <div className="space-y-2">
                            <div className="flex items-center gap-2">
                              <input
                                type="radio"
                                id="mode-purge"
                                name="recovery-mode"
                                value="purge_orphans"
                                checked={recoveryMode === "purge_orphans"}
                                onChange={() => setRecoveryMode("purge_orphans")}
                              />
                              <Label htmlFor="mode-purge" className="text-sm cursor-pointer">
                                欠損レコードを削除（推奨）
                              </Label>
                            </div>
                            <div className="flex items-center gap-2">
                              <input
                                type="radio"
                                id="mode-refetch"
                                name="recovery-mode"
                                value="refetch_current"
                                checked={recoveryMode === "refetch_current"}
                                onChange={() => setRecoveryMode("refetch_current")}
                                disabled
                              />
                              <Label htmlFor="mode-refetch" className="text-sm cursor-pointer text-muted-foreground">
                                現在のスナップショットで再保存（未実装）
                              </Label>
                            </div>
                          </div>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={handleRunRecovery}
                            disabled={recovering}
                          >
                            {recovering ? (
                              <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                            ) : (
                              <CheckCircle2 className="h-4 w-4 mr-2" />
                            )}
                            復旧実行
                          </Button>

                          {/* Recovery Result */}
                          {recoveryResult && (
                            <div className="p-2 rounded bg-muted text-sm">
                              <div>試行: {recoveryResult.attempted}</div>
                              <div className="text-green-600">成功: {recoveryResult.succeeded}</div>
                              <div className="text-destructive">失敗: {recoveryResult.failed}</div>
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          {/* Performance Logs Tab */}
          <TabsContent value="performance" className="flex-1 overflow-hidden mt-4">
            <Card className="h-full flex flex-col">
              <CardHeader className="pb-3 flex-shrink-0">
                <CardTitle className="text-sm font-medium flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <TrendingUp className="h-4 w-4" />
                    パフォーマンスログ
                  </div>
                  <Badge variant="outline">{performanceLogs.length}件</Badge>
                </CardTitle>
              </CardHeader>
              <CardContent className="flex-1 overflow-hidden p-0">
                <ScrollArea className="h-full">
                  <div className="p-4 pt-0">
                    {/* Header */}
                    <div className="grid grid-cols-7 gap-2 text-xs font-medium text-muted-foreground border-b pb-2 mb-2 sticky top-0 bg-card">
                      <div>時刻</div>
                      <div>カメラ</div>
                      <div>取得元</div>
                      <div className="text-right">処理時間</div>
                      <div className="text-right">IS21推論</div>
                      <div>イベント</div>
                      <div className="text-right">重要度</div>
                    </div>
                    {/* Rows */}
                    {performanceLogs.map((log, i) => (
                      <div key={i}>
                        {/* Main row - clickable to expand */}
                        <div
                          className="grid grid-cols-7 gap-2 text-sm py-2 border-b border-muted/30 hover:bg-muted/30 cursor-pointer"
                          onClick={() => setExpandedLogIndex(expandedLogIndex === i ? null : i)}
                        >
                          <div className="text-muted-foreground flex items-center gap-1">
                            {expandedLogIndex === i ? (
                              <ChevronDown className="h-3 w-3" />
                            ) : (
                              <ChevronRight className="h-3 w-3" />
                            )}
                            {new Date(log.timestamp).toLocaleTimeString("ja-JP")}
                          </div>
                          <div className="truncate" title={log.camera_id}>
                            {log.camera_name || log.camera_id.slice(0, 8)}
                          </div>
                          <div>
                            {log.snapshot_source ? (
                              <Badge
                                variant={log.snapshot_source === "go2rtc" ? "default" : "outline"}
                                className={`text-xs ${log.snapshot_source === "go2rtc" ? "bg-green-500" : ""}`}
                              >
                                {log.snapshot_source}
                              </Badge>
                            ) : (
                              <span className="text-muted-foreground">-</span>
                            )}
                          </div>
                          <div className="text-right font-mono">
                            <span className={log.processing_ms > 10000 ? "text-red-500" : log.processing_ms > 5000 ? "text-yellow-500" : ""}>
                              {log.processing_ms}ms
                            </span>
                          </div>
                          <div className="text-right font-mono text-muted-foreground">
                            {log.is21_inference_ms ? `${log.is21_inference_ms}ms` : "-"}
                          </div>
                          <div>
                            <Badge variant={log.primary_event === "human" ? "default" : "secondary"} className="text-xs">
                              {log.primary_event}
                            </Badge>
                          </div>
                          <div className="text-right">
                            {log.severity > 0 ? (
                              <Badge variant="destructive" className="text-xs">{log.severity}</Badge>
                            ) : (
                              <span className="text-muted-foreground">0</span>
                            )}
                          </div>
                        </div>
                        {/* Expanded detail view - IS21 raw log */}
                        {expandedLogIndex === i && log.is21_log_raw && (
                          <div className="bg-muted/50 p-3 border-b border-muted/30">
                            <div className="flex items-center gap-2 mb-2 text-xs font-medium text-muted-foreground">
                              <Code className="h-3 w-3" />
                              IS21 レスポンス (生データ)
                            </div>
                            <pre className="text-xs font-mono bg-background p-2 rounded overflow-x-auto max-h-64 overflow-y-auto">
                              {(() => {
                                try {
                                  return JSON.stringify(JSON.parse(log.is21_log_raw), null, 2)
                                } catch {
                                  return log.is21_log_raw
                                }
                              })()}
                            </pre>
                          </div>
                        )}
                        {expandedLogIndex === i && !log.is21_log_raw && (
                          <div className="bg-muted/50 p-3 border-b border-muted/30 text-xs text-muted-foreground">
                            IS21ログデータがありません
                          </div>
                        )}
                      </div>
                    ))}
                    {performanceLogs.length === 0 && (
                      <div className="text-center py-8 text-muted-foreground">
                        パフォーマンスログがありません
                      </div>
                    )}
                  </div>
                </ScrollArea>
              </CardContent>
            </Card>
          </TabsContent>

          {/* Polling Logs Tab */}
          <TabsContent value="polling" className="flex-1 overflow-hidden mt-4">
            <Card className="h-full flex flex-col">
              <CardHeader className="pb-3 flex-shrink-0">
                <CardTitle className="text-sm font-medium flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Clock className="h-4 w-4" />
                    巡回ログ
                  </div>
                  <Badge variant="outline">{pollingLogs.length}件</Badge>
                </CardTitle>
              </CardHeader>
              <CardContent className="flex-1 overflow-hidden p-0">
                <ScrollArea className="h-full">
                  <div className="p-4 pt-0">
                    {/* Header */}
                    <div className="grid grid-cols-6 gap-2 text-xs font-medium text-muted-foreground border-b pb-2 mb-2 sticky top-0 bg-card">
                      <div>時刻</div>
                      <div>サブネット</div>
                      <div className="text-right">カメラ数</div>
                      <div className="text-right">所要時間</div>
                      <div className="text-right">成功</div>
                      <div className="text-right">エラー</div>
                    </div>
                    {/* Rows */}
                    {pollingLogs.map((log, i) => (
                      <div
                        key={log.polling_id || i}
                        className="grid grid-cols-6 gap-2 text-sm py-2 border-b border-muted/30 hover:bg-muted/30"
                      >
                        <div className="text-muted-foreground">
                          {new Date(log.started_at).toLocaleTimeString("ja-JP")}
                        </div>
                        <div className="font-mono text-xs">{log.subnet}</div>
                        <div className="text-right">{log.camera_count}</div>
                        <div className="text-right font-mono">
                          {log.duration_ms != null ? (
                            <span className={log.duration_ms > 60000 ? "text-yellow-500" : ""}>
                              {(log.duration_ms / 1000).toFixed(1)}s
                            </span>
                          ) : (
                            <span className="text-blue-500 text-xs">running</span>
                          )}
                        </div>
                        <div className="text-right">
                          <span className="text-green-600">{log.success_count}</span>
                        </div>
                        <div className="text-right">
                          {log.failed_count > 0 ? (
                            <span className="text-red-500 flex items-center justify-end gap-1">
                              <AlertTriangle className="h-3 w-3" />
                              {log.failed_count}
                            </span>
                          ) : (
                            <span className="text-muted-foreground">0</span>
                          )}
                        </div>
                      </div>
                    ))}
                    {pollingLogs.length === 0 && (
                      <div className="text-center py-8 text-muted-foreground">
                        巡回ログがありません
                      </div>
                    )}
                  </div>
                </ScrollArea>
              </CardContent>
            </Card>
          </TabsContent>

          {/* Dashboard Tab - Inference Statistics and Performance Charts */}
          <TabsContent value="dashboard" className="flex-1 overflow-hidden mt-4">
            <div className="h-full overflow-auto space-y-6">
              {/* Issue #106: 推論統計・分析タブ */}
              <InferenceStatsTab />
              {/* Performance Dashboard (既存) */}
              <div className="border-t pt-6">
                <h3 className="font-medium mb-4 flex items-center gap-2">
                  <Gauge className="h-4 w-4" />
                  パフォーマンスランキング
                </h3>
                <PerformanceDashboard />
              </div>
            </div>
          </TabsContent>

          {/* AI Assistant Settings Tab (Paraclate placeholder) */}
          <TabsContent value="ai" className="flex-1 overflow-auto mt-4">
            <div className="space-y-4">
              {/* Suggestion Frequency Settings */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <Sliders className="h-4 w-4" />
                    検出傾向調整サジェスト
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="space-y-3">
                    <Label>サジェスト頻度</Label>
                    <div className="flex items-center gap-4">
                      <div className="flex-1">
                        <Slider
                          value={[aiSettings.suggestionFrequency]}
                          onValueChange={(value) => {
                            const newSettings = { ...aiSettings, suggestionFrequency: value[0] }
                            setAiSettings(newSettings)
                            saveAIAssistantSettings(newSettings)
                          }}
                          min={0}
                          max={3}
                          step={1}
                          className="w-full"
                        />
                      </div>
                    </div>
                    <div className="flex justify-between text-xs text-muted-foreground px-1">
                      <span>OFF</span>
                      <span>低</span>
                      <span>中</span>
                      <span>高</span>
                    </div>
                  </div>
                  <div className="p-3 rounded-lg bg-muted/50 text-sm">
                    {aiSettings.suggestionFrequency === 0 && (
                      <p className="text-muted-foreground">サジェストを無効化。検出傾向の調整提案は表示されません。</p>
                    )}
                    {aiSettings.suggestionFrequency === 1 && (
                      <p>低頻度: 明らかな問題がある場合のみサジェストを表示</p>
                    )}
                    {aiSettings.suggestionFrequency === 2 && (
                      <p>中頻度（推奨）: バランスの取れたサジェスト表示</p>
                    )}
                    {aiSettings.suggestionFrequency === 3 && (
                      <p>高頻度: 積極的にサジェストを表示して調整を促進</p>
                    )}
                  </div>
                </CardContent>
              </Card>

              {/* Paraclate Integration Settings (Placeholder) */}
              <Card className="border-dashed">
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium flex items-center gap-2">
                    <FileText className="h-4 w-4" />
                    Paraclate連携（mobes2.0）
                    <Badge variant="secondary" className="ml-2">準備中</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-6">
                  {/* Scheduled Report Settings */}
                  <div className="space-y-3">
                    <h4 className="text-sm font-medium flex items-center gap-2">
                      <Clock className="h-4 w-4 text-muted-foreground" />
                      定時報告設定
                    </h4>
                    <div className="grid grid-cols-2 gap-4">
                      <div className="space-y-2">
                        <Label htmlFor="report-interval">通常報告間隔（分）</Label>
                        <Input
                          id="report-interval"
                          type="number"
                          min={15}
                          max={240}
                          value={aiSettings.paraclate.reportIntervalMinutes}
                          onChange={(e) => {
                            const v = parseInt(e.target.value, 10)
                            if (!isNaN(v) && v >= 15 && v <= 240) {
                              const newSettings = {
                                ...aiSettings,
                                paraclate: { ...aiSettings.paraclate, reportIntervalMinutes: v }
                              }
                              setAiSettings(newSettings)
                              saveAIAssistantSettings(newSettings)
                            }
                          }}
                          disabled
                          className="opacity-60"
                        />
                      </div>
                      <div className="space-y-2">
                        <Label>GrandSummary時刻</Label>
                        <div className="flex flex-wrap gap-1">
                          {aiSettings.paraclate.grandSummaryTimes.map((time, i) => (
                            <Badge key={i} variant="outline" className="opacity-60">{time}</Badge>
                          ))}
                        </div>
                      </div>
                    </div>
                    <p className="text-xs text-amber-600 flex items-center gap-1">
                      <AlertTriangle className="h-3 w-3" />
                      mobes2.0への接続設定後に有効化されます
                    </p>
                  </div>

                  {/* Report Context Settings */}
                  <div className="space-y-3 pt-3 border-t">
                    <h4 className="text-sm font-medium flex items-center gap-2">
                      <FileText className="h-4 w-4 text-muted-foreground" />
                      報告コンテキスト
                    </h4>
                    <div className="space-y-2">
                      <Label htmlFor="context-note">重視するポイント</Label>
                      <textarea
                        id="context-note"
                        className="w-full min-h-[80px] p-2 border rounded-md text-sm resize-none opacity-60"
                        placeholder="例: 不審者の検出を重視。深夜帯の動体検知は特に注意して報告してください。"
                        value={aiSettings.paraclate.contextNote}
                        onChange={(e) => {
                          const newSettings = {
                            ...aiSettings,
                            paraclate: { ...aiSettings.paraclate, contextNote: e.target.value }
                          }
                          setAiSettings(newSettings)
                          saveAIAssistantSettings(newSettings)
                        }}
                        disabled
                      />
                    </div>
                    <div className="grid grid-cols-2 gap-4">
                      <div className="space-y-2">
                        <Label>報告の詳細度</Label>
                        <Select
                          value={aiSettings.paraclate.reportDetail}
                          onValueChange={(value: "concise" | "standard" | "detailed") => {
                            const newSettings = {
                              ...aiSettings,
                              paraclate: { ...aiSettings.paraclate, reportDetail: value }
                            }
                            setAiSettings(newSettings)
                            saveAIAssistantSettings(newSettings)
                          }}
                          disabled
                        >
                          <SelectTrigger className="opacity-60">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="concise">簡潔</SelectItem>
                            <SelectItem value="standard">標準</SelectItem>
                            <SelectItem value="detailed">詳細</SelectItem>
                          </SelectContent>
                        </Select>
                      </div>
                      <div className="flex items-center gap-2 pt-6">
                        <input
                          type="checkbox"
                          id="instant-alert"
                          checked={aiSettings.paraclate.instantAlertOnAnomaly}
                          onChange={(e) => {
                            const newSettings = {
                              ...aiSettings,
                              paraclate: { ...aiSettings.paraclate, instantAlertOnAnomaly: e.target.checked }
                            }
                            setAiSettings(newSettings)
                            saveAIAssistantSettings(newSettings)
                          }}
                          disabled
                          className="opacity-60"
                        />
                        <Label htmlFor="instant-alert" className="text-sm opacity-60">異常検出時の即時通知</Label>
                      </div>
                    </div>
                  </div>

                  {/* AI Attunement Settings */}
                  <div className="space-y-3 pt-3 border-t">
                    <h4 className="text-sm font-medium flex items-center gap-2">
                      <Sliders className="h-4 w-4 text-muted-foreground" />
                      AIアチューンメント
                    </h4>
                    <div className="space-y-3">
                      <div className="flex items-center gap-2">
                        <input
                          type="checkbox"
                          id="auto-tuning"
                          checked={aiSettings.paraclate.autoTuningEnabled}
                          onChange={(e) => {
                            const newSettings = {
                              ...aiSettings,
                              paraclate: { ...aiSettings.paraclate, autoTuningEnabled: e.target.checked }
                            }
                            setAiSettings(newSettings)
                            saveAIAssistantSettings(newSettings)
                          }}
                          disabled
                          className="opacity-60"
                        />
                        <Label htmlFor="auto-tuning" className="text-sm opacity-60">自動チューニング提案を有効化</Label>
                      </div>
                      <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-2">
                          <Label>提案頻度</Label>
                          <Select
                            value={aiSettings.paraclate.tuningFrequency}
                            onValueChange={(value: "daily" | "weekly" | "monthly") => {
                              const newSettings = {
                                ...aiSettings,
                                paraclate: { ...aiSettings.paraclate, tuningFrequency: value }
                              }
                              setAiSettings(newSettings)
                              saveAIAssistantSettings(newSettings)
                            }}
                            disabled
                          >
                            <SelectTrigger className="opacity-60">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="daily">毎日</SelectItem>
                              <SelectItem value="weekly">週1回</SelectItem>
                              <SelectItem value="monthly">月1回</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>
                        <div className="space-y-2">
                          <Label>チューニング積極性</Label>
                          <div className="pt-2 opacity-60">
                            <Slider
                              value={[aiSettings.paraclate.tuningAggressiveness]}
                              onValueChange={(value) => {
                                const newSettings = {
                                  ...aiSettings,
                                  paraclate: { ...aiSettings.paraclate, tuningAggressiveness: value[0] }
                                }
                                setAiSettings(newSettings)
                                saveAIAssistantSettings(newSettings)
                              }}
                              min={0}
                              max={100}
                              step={10}
                              disabled
                            />
                            <div className="flex justify-between text-xs text-muted-foreground mt-1">
                              <span>保守的</span>
                              <span>積極的</span>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Connection Status (Placeholder) */}
                  <div className="pt-3 border-t">
                    <div className="p-3 rounded-lg bg-muted/50 flex items-center gap-3">
                      <Link2Off className="h-5 w-5 text-muted-foreground" />
                      <div>
                        <p className="text-sm font-medium text-muted-foreground">Paraclate API: 未接続</p>
                        <p className="text-xs text-muted-foreground">最終同期: --</p>
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          {/* Brands Tab - Camera Brand/OUI/RTSP Template Management */}
          <TabsContent value="brands" className="flex-1 overflow-hidden mt-4">
            <div className="h-full overflow-auto">
              <CameraBrandsSettings />
            </div>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}
