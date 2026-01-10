/**
 * InferenceStatsTab - 推論統計・分析タブ
 *
 * Issue #106: AIEventlog.md 41-47行目要件
 * - カメラ別・判定結果の分布と件数、傾向分析
 * - プリセット効果分析
 * - 異常カメラ検出
 */

import { useState, useEffect, useCallback } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  RefreshCw,
  Camera,
  AlertTriangle,
  TrendingUp,
  BarChart3,
  PieChart,
  Activity,
  Users,
  Car,
  HelpCircle,
  CheckCircle2,
} from "lucide-react"
import { API_BASE_URL } from "@/lib/config"

// Types for API responses
interface CameraStats {
  camera_id: string
  camera_name: string | null
  preset_id: string
  total: number
  by_event: Record<string, number>
  avg_confidence: number
  anomaly_score: number
  anomaly_reason: string | null
}

interface CameraDistributionResponse {
  period: string
  total_inferences: number
  cameras: CameraStats[]
}

interface TrendPoint {
  hour: string
  human: number
  vehicle: number
  unknown: number
  none: number
  other: number
}

interface EventTrendsResponse {
  period: string
  data: TrendPoint[]
}

interface PresetStats {
  preset_id: string
  camera_count: number
  total_inferences: number
  detection_rate: number
  false_positive_rate: number
  avg_confidence: number
}

interface PresetEffectivenessResponse {
  period: string
  presets: PresetStats[]
}

interface AnomalyCamera {
  camera_id: string
  camera_name: string | null
  anomaly_type: string
  anomaly_score: number
  details: string
  recommendation: string
}

interface AnomaliesResponse {
  period: string
  anomalies: AnomalyCamera[]
}

type StatsPeriod = "24h" | "7d" | "30d"

export function InferenceStatsTab() {
  const [loading, setLoading] = useState(false)
  const [period, setPeriod] = useState<StatsPeriod>("24h")

  // Data states
  const [cameraDistribution, setCameraDistribution] = useState<CameraDistributionResponse | null>(null)
  const [eventTrends, setEventTrends] = useState<EventTrendsResponse | null>(null)
  const [presetEffectiveness, setPresetEffectiveness] = useState<PresetEffectivenessResponse | null>(null)
  const [anomalies, setAnomalies] = useState<AnomaliesResponse | null>(null)

  // Fetch all stats
  const fetchStats = useCallback(async () => {
    setLoading(true)
    try {
      const [camerasResp, eventsResp, presetsResp, anomaliesResp] = await Promise.all([
        fetch(`${API_BASE_URL}/api/stats/cameras?period=${period}`),
        fetch(`${API_BASE_URL}/api/stats/events?period=${period}`),
        fetch(`${API_BASE_URL}/api/stats/presets?period=${period}`),
        fetch(`${API_BASE_URL}/api/stats/anomalies?period=${period}`),
      ])

      if (camerasResp.ok) {
        const json = await camerasResp.json()
        if (json.ok) setCameraDistribution(json.data)
      }
      if (eventsResp.ok) {
        const json = await eventsResp.json()
        if (json.ok) setEventTrends(json.data)
      }
      if (presetsResp.ok) {
        const json = await presetsResp.json()
        if (json.ok) setPresetEffectiveness(json.data)
      }
      if (anomaliesResp.ok) {
        const json = await anomaliesResp.json()
        if (json.ok) setAnomalies(json.data)
      }
    } catch (error) {
      console.error("Failed to fetch inference stats:", error)
    } finally {
      setLoading(false)
    }
  }, [period])

  // Load data on mount and period change
  useEffect(() => {
    fetchStats()
  }, [fetchStats])

  // Get event icon
  const getEventIcon = (event: string) => {
    switch (event) {
      case "human":
        return <Users className="h-3 w-3" />
      case "vehicle":
        return <Car className="h-3 w-3" />
      case "unknown":
        return <HelpCircle className="h-3 w-3" />
      default:
        return <Activity className="h-3 w-3" />
    }
  }

  // Get event color
  const getEventColor = (event: string) => {
    switch (event) {
      case "human":
        return "bg-blue-500"
      case "vehicle":
        return "bg-green-500"
      case "unknown":
        return "bg-amber-500"
      case "none":
        return "bg-gray-300"
      case "animal":
        return "bg-purple-500"
      default:
        return "bg-gray-400"
    }
  }

  return (
    <div className="space-y-4">
      {/* Header with period selector and refresh */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <BarChart3 className="h-5 w-5" />
          <h3 className="font-medium">推論統計</h3>
          {cameraDistribution && (
            <Badge variant="outline" className="ml-2">
              {cameraDistribution.total_inferences.toLocaleString()} 件
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-2">
          {/* Period Selector */}
          <div className="flex gap-1">
            {(["24h", "7d", "30d"] as StatsPeriod[]).map((p) => (
              <Button
                key={p}
                variant={period === p ? "default" : "outline"}
                size="sm"
                onClick={() => setPeriod(p)}
              >
                {p}
              </Button>
            ))}
          </div>
          <Button variant="outline" size="sm" onClick={fetchStats} disabled={loading}>
            {loading ? (
              <RefreshCw className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
          </Button>
        </div>
      </div>

      {/* Anomaly Alerts (if any) */}
      {anomalies && anomalies.anomalies.length > 0 && (
        <Card className="border-amber-500 bg-amber-50 dark:bg-amber-950/20">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium flex items-center gap-2 text-amber-700 dark:text-amber-400">
              <AlertTriangle className="h-4 w-4" />
              異常検出 ({anomalies.anomalies.length}件)
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ScrollArea className="max-h-40">
              <div className="space-y-2">
                {anomalies.anomalies.map((anomaly, i) => (
                  <div
                    key={i}
                    className="flex items-start justify-between p-2 rounded bg-white dark:bg-gray-800 border"
                  >
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <Camera className="h-3 w-3 text-muted-foreground" />
                        <span className="font-medium text-sm">
                          {anomaly.camera_name || anomaly.camera_id}
                        </span>
                        <Badge
                          variant="outline"
                          className={
                            anomaly.anomaly_type === "high_unknown_rate"
                              ? "border-amber-500 text-amber-700"
                              : anomaly.anomaly_type === "high_detection_rate"
                              ? "border-red-500 text-red-700"
                              : "border-blue-500 text-blue-700"
                          }
                        >
                          {anomaly.anomaly_type}
                        </Badge>
                      </div>
                      <p className="text-xs text-muted-foreground mt-1">{anomaly.details}</p>
                      <p className="text-xs text-blue-600 dark:text-blue-400 mt-1">
                        {anomaly.recommendation}
                      </p>
                    </div>
                    <Badge variant="secondary" className="ml-2">
                      {anomaly.anomaly_score.toFixed(1)}
                    </Badge>
                  </div>
                ))}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>
      )}

      {/* Main stats grid */}
      <div className="grid grid-cols-2 gap-4">
        {/* Camera Distribution Card */}
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <PieChart className="h-4 w-4" />
              カメラ別分布
            </CardTitle>
          </CardHeader>
          <CardContent>
            {cameraDistribution ? (
              <ScrollArea className="h-64">
                <div className="space-y-2">
                  {cameraDistribution.cameras.map((cam) => (
                    <div
                      key={cam.camera_id}
                      className={`p-2 rounded border ${
                        cam.anomaly_score > 2.0
                          ? "border-amber-500 bg-amber-50 dark:bg-amber-950/20"
                          : "border-muted"
                      }`}
                    >
                      <div className="flex items-center justify-between mb-1">
                        <span className="text-sm font-medium truncate flex-1">
                          {cam.camera_name || cam.camera_id}
                        </span>
                        <span className="text-xs text-muted-foreground ml-2">
                          {cam.total.toLocaleString()}件
                        </span>
                      </div>
                      {/* Event distribution bar */}
                      <div className="flex h-2 rounded overflow-hidden">
                        {Object.entries(cam.by_event)
                          .filter(([_, count]) => count > 0)
                          .sort((a, b) => b[1] - a[1])
                          .map(([event, count]) => (
                            <div
                              key={event}
                              className={`${getEventColor(event)}`}
                              style={{ width: `${(count / cam.total) * 100}%` }}
                              title={`${event}: ${count}件 (${((count / cam.total) * 100).toFixed(1)}%)`}
                            />
                          ))}
                      </div>
                      <div className="flex items-center justify-between mt-1">
                        <span className="text-xs text-muted-foreground">
                          conf: {cam.avg_confidence.toFixed(2)}
                        </span>
                        <Badge variant="outline" className="text-xs">
                          {cam.preset_id}
                        </Badge>
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            ) : (
              <div className="h-64 flex items-center justify-center text-muted-foreground">
                {loading ? "読み込み中..." : "データなし"}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Event Trends Card */}
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <TrendingUp className="h-4 w-4" />
              時系列傾向
            </CardTitle>
          </CardHeader>
          <CardContent>
            {eventTrends && eventTrends.data.length > 0 ? (
              <ScrollArea className="h-64">
                <div className="space-y-1">
                  {/* Legend */}
                  <div className="flex gap-3 mb-2 text-xs">
                    <span className="flex items-center gap-1">
                      <div className="w-2 h-2 rounded bg-blue-500" /> human
                    </span>
                    <span className="flex items-center gap-1">
                      <div className="w-2 h-2 rounded bg-green-500" /> vehicle
                    </span>
                    <span className="flex items-center gap-1">
                      <div className="w-2 h-2 rounded bg-amber-500" /> unknown
                    </span>
                    <span className="flex items-center gap-1">
                      <div className="w-2 h-2 rounded bg-gray-300" /> none
                    </span>
                  </div>
                  {/* Simple bar chart representation */}
                  {eventTrends.data.slice(-24).map((point, i) => {
                    const total = point.human + point.vehicle + point.unknown + point.none + point.other
                    const maxTotal = Math.max(
                      ...eventTrends.data.slice(-24).map(
                        (p) => p.human + p.vehicle + p.unknown + p.none + p.other
                      )
                    )
                    return (
                      <div key={i} className="flex items-center gap-2">
                        <span className="text-xs text-muted-foreground w-12 text-right">
                          {point.hour.split(" ")[1]?.substring(0, 5) || point.hour}
                        </span>
                        <div className="flex-1 flex h-3 rounded overflow-hidden bg-muted">
                          {total > 0 && (
                            <>
                              <div
                                className="bg-blue-500"
                                style={{ width: `${(point.human / maxTotal) * 100}%` }}
                              />
                              <div
                                className="bg-green-500"
                                style={{ width: `${(point.vehicle / maxTotal) * 100}%` }}
                              />
                              <div
                                className="bg-amber-500"
                                style={{ width: `${(point.unknown / maxTotal) * 100}%` }}
                              />
                              <div
                                className="bg-gray-300"
                                style={{ width: `${(point.none / maxTotal) * 100}%` }}
                              />
                            </>
                          )}
                        </div>
                        <span className="text-xs text-muted-foreground w-8 text-right">
                          {total}
                        </span>
                      </div>
                    )
                  })}
                </div>
              </ScrollArea>
            ) : (
              <div className="h-64 flex items-center justify-center text-muted-foreground">
                {loading ? "読み込み中..." : "データなし"}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Preset Effectiveness Card */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <CheckCircle2 className="h-4 w-4" />
            プリセット効果分析
          </CardTitle>
        </CardHeader>
        <CardContent>
          {presetEffectiveness && presetEffectiveness.presets.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b">
                    <th className="text-left py-2 font-medium">プリセット</th>
                    <th className="text-right py-2 font-medium">カメラ数</th>
                    <th className="text-right py-2 font-medium">推論回数</th>
                    <th className="text-right py-2 font-medium">検出率</th>
                    <th className="text-right py-2 font-medium">誤検知率</th>
                    <th className="text-right py-2 font-medium">平均信頼度</th>
                  </tr>
                </thead>
                <tbody>
                  {presetEffectiveness.presets.map((preset) => (
                    <tr key={preset.preset_id} className="border-b border-muted/30">
                      <td className="py-2">
                        <Badge variant="outline">{preset.preset_id}</Badge>
                      </td>
                      <td className="text-right py-2">{preset.camera_count}</td>
                      <td className="text-right py-2 font-mono">
                        {preset.total_inferences.toLocaleString()}
                      </td>
                      <td className="text-right py-2">
                        <span
                          className={
                            preset.detection_rate > 0.1
                              ? "text-amber-600"
                              : preset.detection_rate > 0.05
                              ? "text-blue-600"
                              : "text-green-600"
                          }
                        >
                          {(preset.detection_rate * 100).toFixed(1)}%
                        </span>
                      </td>
                      <td className="text-right py-2">
                        <span
                          className={
                            preset.false_positive_rate > 0.1
                              ? "text-red-600"
                              : preset.false_positive_rate > 0.05
                              ? "text-amber-600"
                              : "text-green-600"
                          }
                        >
                          {(preset.false_positive_rate * 100).toFixed(2)}%
                        </span>
                      </td>
                      <td className="text-right py-2 font-mono">
                        {preset.avg_confidence.toFixed(3)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="py-8 text-center text-muted-foreground">
              {loading ? "読み込み中..." : "プリセットデータなし"}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Event Distribution Legend */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">イベント凡例</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-4 text-sm">
            <div className="flex items-center gap-2">
              {getEventIcon("human")}
              <div className={`w-3 h-3 rounded ${getEventColor("human")}`} />
              <span>human (人物検知)</span>
            </div>
            <div className="flex items-center gap-2">
              {getEventIcon("vehicle")}
              <div className={`w-3 h-3 rounded ${getEventColor("vehicle")}`} />
              <span>vehicle (車両検知)</span>
            </div>
            <div className="flex items-center gap-2">
              {getEventIcon("unknown")}
              <div className={`w-3 h-3 rounded ${getEventColor("unknown")}`} />
              <span>unknown (不明物体)</span>
            </div>
            <div className="flex items-center gap-2">
              <Activity className="h-3 w-3" />
              <div className={`w-3 h-3 rounded ${getEventColor("animal")}`} />
              <span>animal (動物)</span>
            </div>
            <div className="flex items-center gap-2">
              <Activity className="h-3 w-3" />
              <div className={`w-3 h-3 rounded ${getEventColor("none")}`} />
              <span>none (検知なし)</span>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
