import { useState, useEffect, useMemo } from 'react'
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
} from 'recharts'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { API_BASE_URL } from '@/lib/config'
import {
  Activity,
  Clock,
  AlertTriangle,
  TrendingUp,
  TrendingDown,
  RefreshCw,
  Gauge,
} from 'lucide-react'

// Types matching backend API
interface TimelineDataPoint {
  timestamp: string
  subnet: string
  avg_lap_time_ms: number
  min_lap_time_ms: number
  max_lap_time_ms: number
  sample_count: number
}

interface ErrorRateDataPoint {
  timestamp: string
  subnet: string
  total_count: number
  success_count: number
  image_error_count: number
  inference_error_count: number
  error_rate_pct: number
}

interface ProcessingDistribution {
  camera_id: string
  camera_name: string | null
  subnet: string
  total_processing_ms: number
  avg_processing_ms: number
  sample_count: number
  percentage: number
}

interface CameraRanking {
  rank: number
  camera_id: string
  camera_name: string | null
  camera_ip: string | null
  subnet: string
  avg_time_ms: number
  min_time_ms: number
  max_time_ms: number
  sample_count: number
}

interface SubnetStats {
  subnet: string
  camera_count: number
  total_samples: number
  avg_snapshot_ms: number | null
  avg_inference_ms: number | null
  avg_total_ms: number
  success_rate_pct: number
  performance_grade: string
}

interface DashboardSummary {
  total_samples: number
  total_subnets: number
  total_cameras: number
  overall_avg_processing_ms: number
  overall_success_rate_pct: number
  overall_grade: string
}

interface PeriodInfo {
  start: string
  end: string
  duration_hours: number
  granularity_minutes: number
}

interface PerformanceDashboardData {
  period: PeriodInfo
  timeline: TimelineDataPoint[]
  error_rates: ErrorRateDataPoint[]
  distribution: ProcessingDistribution[]
  fastest_ranking: CameraRanking[]
  slowest_ranking: CameraRanking[]
  subnet_stats: SubnetStats[]
  summary: DashboardSummary
}

// Color palette for subnets
const SUBNET_COLORS: Record<string, { line: string; fill: string }> = {
  '192.168.125': { line: '#ef4444', fill: '#fca5a5' }, // red
  '192.168.126': { line: '#3b82f6', fill: '#93c5fd' }, // blue
  '192.168.127': { line: '#22c55e', fill: '#86efac' }, // green
  '192.168.128': { line: '#f59e0b', fill: '#fcd34d' }, // amber
}

const PIE_COLORS = [
  '#3b82f6', '#ef4444', '#22c55e', '#f59e0b', '#8b5cf6',
  '#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1',
]

// Grade badge colors
const GRADE_COLORS: Record<string, string> = {
  A: 'bg-green-500 text-white',
  B: 'bg-blue-500 text-white',
  C: 'bg-yellow-500 text-black',
  D: 'bg-orange-500 text-white',
  F: 'bg-red-500 text-white',
}

export function PerformanceDashboard() {
  const [data, setData] = useState<PerformanceDashboardData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [hours, setHours] = useState(24)

  const fetchData = async () => {
    setLoading(true)
    setError(null)
    try {
      const res = await fetch(`${API_BASE_URL}/api/debug/performance/dashboard?hours=${hours}`)
      const json = await res.json()
      if (json.ok) {
        setData(json.data)
      } else {
        setError(json.error?.message || 'Failed to fetch data')
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Unknown error')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchData()
    // Auto-refresh every 60 seconds
    const interval = setInterval(fetchData, 60000)
    return () => clearInterval(interval)
  }, [hours])

  // Transform timeline data for Recharts (group by timestamp, columns by subnet)
  const timelineChartData = useMemo(() => {
    if (!data?.timeline) return []
    const grouped: Record<string, Record<string, number>> = {}
    data.timeline.forEach((point) => {
      if (!grouped[point.timestamp]) {
        grouped[point.timestamp] = { timestamp: point.timestamp } as unknown as Record<string, number>
      }
      grouped[point.timestamp][`${point.subnet}_avg`] = point.avg_lap_time_ms
      grouped[point.timestamp][`${point.subnet}_min`] = point.min_lap_time_ms
      grouped[point.timestamp][`${point.subnet}_max`] = point.max_lap_time_ms
    })
    return Object.values(grouped)
  }, [data?.timeline])

  // Transform error rate data for Recharts
  const errorChartData = useMemo(() => {
    if (!data?.error_rates) return []
    const grouped: Record<string, Record<string, number>> = {}
    data.error_rates.forEach((point) => {
      if (!grouped[point.timestamp]) {
        grouped[point.timestamp] = { timestamp: point.timestamp } as unknown as Record<string, number>
      }
      grouped[point.timestamp][`${point.subnet}_error_rate`] = point.error_rate_pct
      grouped[point.timestamp][`${point.subnet}_image_errors`] = point.image_error_count
      grouped[point.timestamp][`${point.subnet}_inference_errors`] = point.inference_error_count
    })
    return Object.values(grouped)
  }, [data?.error_rates])

  // Pie chart data (top 10 by processing time)
  const pieChartData = useMemo(() => {
    if (!data?.distribution) return []
    return data.distribution.slice(0, 10).map((item) => ({
      name: item.camera_name || item.camera_id,
      value: item.total_processing_ms,
      percentage: item.percentage,
    }))
  }, [data?.distribution])

  // Get unique subnets from data
  const subnets = useMemo(() => {
    if (!data?.subnet_stats) return []
    return data.subnet_stats.map((s) => s.subnet)
  }, [data?.subnet_stats])

  if (loading && !data) {
    return (
      <div className="flex items-center justify-center h-full">
        <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <AlertTriangle className="h-12 w-12 text-destructive mx-auto mb-4" />
          <p className="text-lg font-semibold">Error loading dashboard</p>
          <p className="text-muted-foreground">{error}</p>
          <Button onClick={fetchData} className="mt-4">
            Retry
          </Button>
        </div>
      </div>
    )
  }

  if (!data) return null

  return (
    <div className="p-6 space-y-6 overflow-auto">
      {/* Header with Summary */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold flex items-center gap-2">
            <Gauge className="h-6 w-6" />
            Performance Dashboard
          </h2>
          <p className="text-muted-foreground text-sm">
            Period: {new Date(data.period.start).toLocaleString()} - {new Date(data.period.end).toLocaleString()}
            ({data.period.duration_hours.toFixed(1)}h)
          </p>
        </div>
        <div className="flex items-center gap-4">
          {/* Time range selector */}
          <div className="flex gap-2">
            {[1, 6, 12, 24, 48].map((h) => (
              <Button
                key={h}
                variant={hours === h ? 'default' : 'outline'}
                size="sm"
                onClick={() => setHours(h)}
              >
                {h}h
              </Button>
            ))}
          </div>
          <Button variant="outline" size="sm" onClick={fetchData} disabled={loading}>
            <RefreshCw className={`h-4 w-4 mr-1 ${loading ? 'animate-spin' : ''}`} />
            Refresh
          </Button>
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-5 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Overall Grade</CardTitle>
          </CardHeader>
          <CardContent>
            <Badge className={`text-2xl px-4 py-2 ${GRADE_COLORS[data.summary.overall_grade] || ''}`}>
              {data.summary.overall_grade}
            </Badge>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Avg Processing</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <Clock className="h-5 w-5 text-muted-foreground" />
              <span className="text-2xl font-bold">
                {(data.summary.overall_avg_processing_ms / 1000).toFixed(1)}s
              </span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Success Rate</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <Activity className="h-5 w-5 text-muted-foreground" />
              <span className="text-2xl font-bold">{data.summary.overall_success_rate_pct.toFixed(1)}%</span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Total Samples</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-2xl font-bold">{data.summary.total_samples.toLocaleString()}</span>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Cameras / Subnets</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-2xl font-bold">
              {data.summary.total_cameras} / {data.summary.total_subnets}
            </span>
          </CardContent>
        </Card>
      </div>

      {/* Charts Row 1: Timeline and Error Rate */}
      <div className="grid grid-cols-2 gap-4">
        {/* Lap Time Timeline Chart */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <TrendingUp className="h-5 w-5" />
              Polling Lap Time (per Subnet)
            </CardTitle>
          </CardHeader>
          <CardContent>
            {timelineChartData.length > 0 ? (
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={timelineChartData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis
                    dataKey="timestamp"
                    tickFormatter={(v) => new Date(v).toLocaleTimeString('ja-JP', { hour: '2-digit', minute: '2-digit' })}
                    fontSize={11}
                  />
                  <YAxis
                    tickFormatter={(v) => `${(v / 1000).toFixed(0)}s`}
                    fontSize={11}
                  />
                  <Tooltip
                    formatter={(value) => [`${(Number(value) / 1000).toFixed(2)}s`, '']}
                    labelFormatter={(label) => new Date(String(label)).toLocaleString('ja-JP')}
                  />
                  <Legend />
                  {subnets.map((subnet) => {
                    const colors = SUBNET_COLORS[subnet] || { line: '#888', fill: '#ccc' }
                    return (
                      <Line
                        key={subnet}
                        type="monotone"
                        dataKey={`${subnet}_avg`}
                        name={`${subnet} avg`}
                        stroke={colors.line}
                        strokeWidth={2}
                        dot={false}
                      />
                    )
                  })}
                </LineChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex items-center justify-center h-[300px] text-muted-foreground">
                No timeline data available
              </div>
            )}
          </CardContent>
        </Card>

        {/* Error Rate Chart */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <AlertTriangle className="h-5 w-5" />
              Error Rate (per Subnet)
            </CardTitle>
          </CardHeader>
          <CardContent>
            {errorChartData.length > 0 ? (
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={errorChartData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis
                    dataKey="timestamp"
                    tickFormatter={(v) => new Date(v).toLocaleTimeString('ja-JP', { hour: '2-digit', minute: '2-digit' })}
                    fontSize={11}
                  />
                  <YAxis
                    tickFormatter={(v) => `${v}%`}
                    fontSize={11}
                    domain={[0, 100]}
                  />
                  <Tooltip
                    formatter={(value) => [`${Number(value).toFixed(1)}%`, '']}
                    labelFormatter={(label) => new Date(String(label)).toLocaleString('ja-JP')}
                  />
                  <Legend />
                  {subnets.map((subnet) => {
                    const colors = SUBNET_COLORS[subnet] || { line: '#888', fill: '#ccc' }
                    return (
                      <Line
                        key={subnet}
                        type="monotone"
                        dataKey={`${subnet}_error_rate`}
                        name={`${subnet} error`}
                        stroke={colors.line}
                        strokeWidth={2}
                        dot={false}
                      />
                    )
                  })}
                </LineChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex items-center justify-center h-[300px] text-muted-foreground">
                No error data available
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Charts Row 2: Pie Chart and Subnet Stats */}
      <div className="grid grid-cols-2 gap-4">
        {/* Processing Distribution Pie Chart */}
        <Card>
          <CardHeader>
            <CardTitle>Processing Time Distribution (Top 10)</CardTitle>
          </CardHeader>
          <CardContent>
            {pieChartData.length > 0 ? (
              <ResponsiveContainer width="100%" height={300}>
                <PieChart>
                  <Pie
                    data={pieChartData}
                    dataKey="value"
                    nameKey="name"
                    cx="50%"
                    cy="50%"
                    outerRadius={100}
                    label={({ name, payload }) => `${name} (${(payload?.percentage ?? 0).toFixed(1)}%)`}
                    labelLine={true}
                  >
                    {pieChartData.map((_, index) => (
                      <Cell key={`cell-${index}`} fill={PIE_COLORS[index % PIE_COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip
                    formatter={(value) => [`${(Number(value) / 1000).toFixed(1)}s total`, '']}
                  />
                </PieChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex items-center justify-center h-[300px] text-muted-foreground">
                No distribution data available
              </div>
            )}
          </CardContent>
        </Card>

        {/* Subnet Statistics */}
        <Card>
          <CardHeader>
            <CardTitle>Subnet Performance</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {data.subnet_stats.length > 0 ? (
                data.subnet_stats.map((subnet) => (
                  <div
                    key={subnet.subnet}
                    className="flex items-center justify-between p-3 rounded-lg border"
                  >
                    <div className="flex items-center gap-3">
                      <Badge className={GRADE_COLORS[subnet.performance_grade] || ''}>
                        {subnet.performance_grade}
                      </Badge>
                      <div>
                        <div className="font-medium">{subnet.subnet}</div>
                        <div className="text-sm text-muted-foreground">
                          {subnet.camera_count} cameras, {subnet.total_samples.toLocaleString()} samples
                        </div>
                      </div>
                    </div>
                    <div className="text-right">
                      <div className="font-mono text-lg">
                        {(subnet.avg_total_ms / 1000).toFixed(1)}s avg
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {subnet.success_rate_pct.toFixed(1)}% success
                      </div>
                    </div>
                  </div>
                ))
              ) : (
                <div className="text-center text-muted-foreground py-8">
                  No subnet data available
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Rankings Row */}
      <div className="grid grid-cols-2 gap-4">
        {/* Fastest Cameras */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <TrendingUp className="h-5 w-5 text-green-500" />
              Fastest Cameras (Top 10)
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {data.fastest_ranking.length > 0 ? (
                data.fastest_ranking.map((cam) => (
                  <div
                    key={cam.camera_id}
                    className="flex items-center justify-between p-2 rounded border text-sm"
                  >
                    <div className="flex items-center gap-2">
                      <span className="font-bold text-green-500 w-6">#{cam.rank}</span>
                      <div>
                        <div className="font-medium">{cam.camera_name || cam.camera_id}</div>
                        <div className="text-xs text-muted-foreground">
                          {cam.camera_ip} ({cam.subnet})
                        </div>
                      </div>
                    </div>
                    <div className="text-right font-mono">
                      <div className="text-green-600">{(cam.avg_time_ms / 1000).toFixed(2)}s</div>
                      <div className="text-xs text-muted-foreground">
                        min {(cam.min_time_ms / 1000).toFixed(2)}s
                      </div>
                    </div>
                  </div>
                ))
              ) : (
                <div className="text-center text-muted-foreground py-4">No data</div>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Slowest Cameras */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <TrendingDown className="h-5 w-5 text-red-500" />
              Slowest Cameras (Top 10)
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {data.slowest_ranking.length > 0 ? (
                data.slowest_ranking.map((cam) => (
                  <div
                    key={cam.camera_id}
                    className="flex items-center justify-between p-2 rounded border text-sm"
                  >
                    <div className="flex items-center gap-2">
                      <span className="font-bold text-red-500 w-6">#{cam.rank}</span>
                      <div>
                        <div className="font-medium">{cam.camera_name || cam.camera_id}</div>
                        <div className="text-xs text-muted-foreground">
                          {cam.camera_ip} ({cam.subnet})
                        </div>
                      </div>
                    </div>
                    <div className="text-right font-mono">
                      <div className="text-red-600">{(cam.avg_time_ms / 1000).toFixed(2)}s</div>
                      <div className="text-xs text-muted-foreground">
                        max {(cam.max_time_ms / 1000).toFixed(2)}s
                      </div>
                    </div>
                  </div>
                ))
              ) : (
                <div className="text-center text-muted-foreground py-4">No data</div>
              )}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
