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
} from "lucide-react"
import { API_BASE_URL } from "@/lib/config"
import { PerformanceDashboard } from "@/components/PerformanceDashboard"

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
  timestamp: string
  subnet: string
  camera_count: number
  cycle_duration_ms: number
  success_count: number
  error_count: number
  slow_count: number
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

  // Load data when modal opens or tab changes
  useEffect(() => {
    if (!open) return

    const fetchData = async () => {
      setLoading(true)
      try {
        switch (activeTab) {
          case "is21":
            await fetchIS21Status()
            break
          case "system":
            await fetchSystemInfo()
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
  }, [open, activeTab, fetchIS21Status, fetchSystemInfo, fetchPerformanceLogs, fetchPollingLogs])

  // Auto-refresh for active tabs
  useEffect(() => {
    if (!open) return

    const interval = setInterval(() => {
      switch (activeTab) {
        case "is21":
          fetchIS21Status()
          break
        case "system":
          fetchSystemInfo()
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
  }, [open, activeTab, fetchIS21Status, fetchSystemInfo, fetchPerformanceLogs, fetchPollingLogs])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Settings2 className="h-5 w-5" />
            システム設定
          </DialogTitle>
        </DialogHeader>

        <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col overflow-hidden">
          <TabsList className="grid w-full grid-cols-6">
            <TabsTrigger value="display" className="flex items-center gap-1 text-xs">
              <Video className="h-4 w-4" />
              表示
            </TabsTrigger>
            <TabsTrigger value="is21" className="flex items-center gap-1 text-xs">
              <Server className="h-4 w-4" />
              IS21接続
            </TabsTrigger>
            <TabsTrigger value="system" className="flex items-center gap-1 text-xs">
              <Cpu className="h-4 w-4" />
              システム
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
            </div>
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
                        key={i}
                        className="grid grid-cols-6 gap-2 text-sm py-2 border-b border-muted/30 hover:bg-muted/30"
                      >
                        <div className="text-muted-foreground">
                          {new Date(log.timestamp).toLocaleTimeString("ja-JP")}
                        </div>
                        <div className="font-mono text-xs">{log.subnet}</div>
                        <div className="text-right">{log.camera_count}</div>
                        <div className="text-right font-mono">
                          <span className={log.cycle_duration_ms > 60000 ? "text-yellow-500" : ""}>
                            {(log.cycle_duration_ms / 1000).toFixed(1)}s
                          </span>
                        </div>
                        <div className="text-right">
                          <span className="text-green-600">{log.success_count}</span>
                        </div>
                        <div className="text-right">
                          {log.error_count > 0 ? (
                            <span className="text-red-500 flex items-center justify-end gap-1">
                              <AlertTriangle className="h-3 w-3" />
                              {log.error_count}
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

          {/* Dashboard Tab - Performance Charts and Rankings */}
          <TabsContent value="dashboard" className="flex-1 overflow-hidden mt-4">
            <div className="h-full overflow-auto">
              <PerformanceDashboard />
            </div>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}
