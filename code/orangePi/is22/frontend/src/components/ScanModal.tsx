import { useState, useEffect, useCallback, useRef, useMemo } from "react"
import type {
  ScannedDevice,
  ScanJob,
  DeviceType,
  ScanLogEntry,
  Camera,
  Subnet,
  TrialCredential,
  DeviceCategory,
  CategorizedDevice,
} from "@/types/api"
import { DEVICE_CATEGORIES } from "@/types/api"
import {
  clearScanResultCache,
  loadScanResultFromCache,
  saveScanResultToCache,
  type CachedScanResult,
} from "@/utils/scanCache"
import { categorizeAndSortDevices } from "@/utils/deviceCategorization"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { ConfirmDialog } from "@/components/ConfirmDialog"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Card } from "@/components/ui/card"
import {
  CheckCircle2,
  XCircle,
  AlertCircle,
  Camera as CameraIcon,
  Wifi,
  Key,
  Search,
  RefreshCw,
  Plus,
  ChevronDown,
  ChevronRight,
  Trash2,
  Settings2,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { API_BASE_URL } from "@/lib/config"

interface ScanModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onDevicesRegistered?: (count: number) => void
}

// Radar spinner component
function RadarSpinner({ className }: { className?: string }) {
  return (
    <div className={cn("relative", className)}>
      {/* Base circle */}
      <div className="absolute inset-0 rounded-full border-2 border-primary/20" />
      {/* Middle circle */}
      <div className="absolute inset-[15%] rounded-full border border-primary/30" />
      {/* Inner circle */}
      <div className="absolute inset-[30%] rounded-full border border-primary/40" />
      {/* Center dot */}
      <div className="absolute inset-[45%] rounded-full bg-primary" />
      {/* Sweep */}
      <div className="absolute inset-0 animate-spin duration-2000">
        <div
          className="h-1/2 w-1/2 origin-bottom-right"
          style={{
            background:
              "conic-gradient(from 0deg, transparent, hsl(var(--primary)) 90deg, transparent 90deg)",
            borderRadius: "100% 0 0 0",
          }}
        />
      </div>
      {/* Blips */}
      <div className="absolute inset-0 animate-pulse">
        <div className="absolute top-[20%] left-[60%] h-2 w-2 rounded-full bg-green-500 shadow-[0_0_8px_2px_rgba(34,197,94,0.6)]" />
        <div className="absolute top-[50%] left-[25%] h-2 w-2 rounded-full bg-green-500 shadow-[0_0_8px_2px_rgba(34,197,94,0.6)]" />
        <div className="absolute top-[70%] left-[70%] h-1.5 w-1.5 rounded-full bg-amber-500 shadow-[0_0_8px_2px_rgba(245,158,11,0.6)]" />
      </div>
    </div>
  )
}

// Subnet credential editor component (is22_ScanModal_Credential_Trial_Spec.md Section 6)
function SubnetCredentialEditor({
  subnet,
  allSubnets,
  onSave,
  onClose,
}: {
  subnet: Subnet
  allSubnets: Subnet[]
  onSave: (updatedSubnet: Partial<Subnet>) => Promise<void>
  onClose: () => void
}) {
  const [fid, setFid] = useState(subnet.fid || "")
  const [tid, setTid] = useState(subnet.tid || "")
  const [facilityName, setFacilityName] = useState(subnet.facility_name || "")
  const [credentials, setCredentials] = useState<TrialCredential[]>(
    subnet.credentials || []
  )
  const [newUsername, setNewUsername] = useState("")
  const [newPassword, setNewPassword] = useState("")
  const [saving, setSaving] = useState(false)

  // fid validation: 空または4桁の整数のみ許可
  const fidError = useMemo(() => {
    if (!fid) return null  // 空はOK
    if (!/^\d{4}$/.test(fid)) return "fidは4桁の数字である必要があります"
    return null
  }, [fid])

  // fid入力時に同一fidの施設名をサジェスト
  const suggestedFacilityName = useMemo(() => {
    if (!fid || fid === subnet.fid) return null
    const existing = allSubnets.find(
      (s) => s.subnet_id !== subnet.subnet_id && s.fid === fid && s.facility_name
    )
    return existing?.facility_name || null
  }, [fid, subnet.subnet_id, subnet.fid, allSubnets])

  // 同一fidのtidをサジェスト
  const suggestedTid = useMemo(() => {
    if (!fid || fid === subnet.fid) return null
    const existing = allSubnets.find(
      (s) => s.subnet_id !== subnet.subnet_id && s.fid === fid && s.tid
    )
    return existing?.tid || null
  }, [fid, subnet.subnet_id, subnet.fid, allSubnets])

  const addCredential = () => {
    if (!newUsername.trim() || !newPassword.trim()) return
    if (credentials.length >= 10) return

    const newCred: TrialCredential = {
      username: newUsername.trim(),
      password: newPassword.trim(),
      priority: credentials.length + 1,
    }
    setCredentials([...credentials, newCred])
    setNewUsername("")
    setNewPassword("")
  }

  const removeCredential = (index: number) => {
    const updated = credentials.filter((_, i) => i !== index)
    // Re-assign priorities
    setCredentials(updated.map((c, i) => ({ ...c, priority: i + 1 })))
  }

  const handleSave = async () => {
    // fid validation check
    if (fidError) {
      return
    }
    setSaving(true)
    try {
      await onSave({
        fid: fid || undefined,
        tid: tid || undefined,
        facility_name: facilityName || undefined,
        credentials: credentials.length > 0 ? credentials : undefined,
      })
      onClose()
    } catch (err) {
      console.error("Failed to save subnet settings:", err)
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-[500px] max-h-[80vh] overflow-auto rounded-lg bg-white p-6 shadow-lg">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold flex items-center gap-2">
            <Settings2 className="h-5 w-5" />
            サブネット設定: {subnet.cidr}
          </h3>
          <Button variant="ghost" size="sm" onClick={onClose}>
            <XCircle className="h-4 w-4" />
          </Button>
        </div>

        {/* fid + tid + 施設名 */}
        <div className="space-y-3 mb-4">
          <div className="flex gap-3">
            <div className="w-24">
              <label className="text-xs text-muted-foreground">施設ID (fid)</label>
              <input
                type="text"
                className={cn(
                  "w-full px-2 py-1.5 text-sm border rounded",
                  fidError && "border-red-500"
                )}
                placeholder="0000"
                maxLength={4}
                value={fid}
                onChange={(e) => setFid(e.target.value)}
              />
              {fidError && (
                <p className="text-xs text-red-500 mt-0.5">{fidError}</p>
              )}
            </div>
            <div className="flex-1">
              <label className="text-xs text-muted-foreground">テナントID (tid)</label>
              <input
                type="text"
                className="w-full px-2 py-1.5 text-sm font-mono border rounded"
                placeholder="T0000000000000000000"
                value={tid}
                onChange={(e) => setTid(e.target.value)}
              />
              {suggestedTid && (
                <button
                  type="button"
                  className="text-xs text-blue-600 mt-0.5 hover:underline"
                  onClick={() => setTid(suggestedTid)}
                >
                  サジェスト: {suggestedTid}
                </button>
              )}
            </div>
          </div>
          <div>
            <label className="text-xs text-muted-foreground">施設名</label>
            <input
              type="text"
              className="w-full px-2 py-1.5 text-sm border rounded"
              placeholder="施設名"
              value={facilityName}
              onChange={(e) => setFacilityName(e.target.value)}
            />
            {suggestedFacilityName && (
              <button
                type="button"
                className="text-xs text-blue-600 mt-1 hover:underline"
                onClick={() => setFacilityName(suggestedFacilityName)}
              >
                サジェスト: {suggestedFacilityName}
              </button>
            )}
          </div>
        </div>

        {/* クレデンシャル一覧 */}
        <div className="mb-4">
          <div className="flex items-center justify-between mb-2">
            <label className="text-sm font-medium flex items-center gap-1">
              <Key className="h-4 w-4" />
              試行クレデンシャル ({credentials.length}/10)
            </label>
          </div>

          {credentials.length === 0 ? (
            <p className="text-xs text-muted-foreground py-2">
              クレデンシャルが設定されていません。スキャン時に認証が試行されません。
            </p>
          ) : (
            <div className="space-y-1 mb-2">
              {credentials.map((cred, index) => (
                <div
                  key={index}
                  className="flex items-center gap-2 p-2 bg-muted/50 rounded text-sm"
                >
                  <span className="w-6 text-xs text-muted-foreground">
                    {cred.priority}.
                  </span>
                  <span className="flex-1 font-mono">{cred.username}</span>
                  <span className="w-32 font-mono text-muted-foreground">
                    {cred.password}
                  </span>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => removeCredential(index)}
                  >
                    <Trash2 className="h-3 w-3 text-red-500" />
                  </Button>
                </div>
              ))}
            </div>
          )}

          {/* 新規クレデンシャル追加 */}
          {credentials.length < 10 && (
            <div className="flex gap-2 mt-2">
              <input
                type="text"
                className="flex-1 px-2 py-1.5 text-sm font-mono border rounded"
                placeholder="ユーザー名"
                value={newUsername}
                onChange={(e) => setNewUsername(e.target.value)}
              />
              <input
                type="text"
                className="flex-1 px-2 py-1.5 text-sm font-mono border rounded"
                placeholder="パスワード"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") addCredential()
                }}
              />
              <Button
                size="sm"
                variant="outline"
                onClick={addCredential}
                disabled={!newUsername.trim() || !newPassword.trim()}
              >
                <Plus className="h-4 w-4" />
              </Button>
            </div>
          )}
        </div>

        {/* 保存ボタン */}
        <div className="flex justify-end gap-2 pt-4 border-t">
          <Button variant="outline" onClick={onClose}>
            キャンセル
          </Button>
          <Button onClick={handleSave} disabled={saving || !!fidError}>
            {saving ? (
              <>
                <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                保存中...
              </>
            ) : (
              "保存"
            )}
          </Button>
        </div>
      </div>
    </div>
  )
}

// Scan detail log viewer (IS22_UI_DETAILED_SPEC Section 3.2)
// 改善版: 背景黒ベース、重要ログのハイライト、行数増加
function ScanLogViewer({ logs, currentPhase, progress }: {
  logs: ScanLogEntry[]
  currentPhase?: string
  progress?: number
}) {
  const logEndRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" })
  }, [logs])

  // ログの重要度に応じたスタイリング（白背景用）
  const getLogStyle = (log: ScanLogEntry) => {
    const msg = log.message

    // 成功・発見系 - 緑ハイライト
    if (msg.includes("成功") || msg.includes("★") || msg.includes("発見")) {
      return "bg-green-100 text-green-800 font-bold px-1 rounded border-l-4 border-green-500"
    }
    // 警告系（ネットワーク到達性問題など） - オレンジハイライト
    if (log.event_type === "warning" || msg.includes("⚠️")) {
      return "bg-amber-100 text-amber-800 font-bold px-1 rounded border-l-4 border-amber-500"
    }
    // エラー系 - 赤ハイライト
    if (log.event_type === "error" || msg.includes("失敗") || msg.includes("エラー")) {
      return "bg-red-100 text-red-800 font-bold px-1 rounded border-l-4 border-red-500"
    }
    // 完了系 - 青ハイライト
    if (msg.includes("完了")) {
      return "bg-blue-100 text-blue-800 font-medium px-1 rounded border-l-4 border-blue-500"
    }

    // イベントタイプ別の色（白背景に映える濃い色）
    switch (log.event_type) {
      case "arp_response":
        return "text-cyan-700"
      case "port_open":
        return "text-green-700"
      case "oui_match":
        return "text-purple-700"
      case "onvif_probe":
        return "text-amber-700"
      case "rtsp_probe":
        return "text-sky-700"
      case "device_classified":
        return "text-emerald-700 font-medium"
      case "credential_trial":
        return "text-orange-700"
      case "info":
        return "text-gray-700"
      default:
        // error は if文で処理済み
        return "text-gray-600"
    }
  }

  return (
    <div className="space-y-2">
      {/* 現在のステージ表示 - 白背景 */}
      {currentPhase && (
        <div className="flex items-center justify-between bg-blue-50 border border-blue-200 rounded-md px-3 py-2">
          <div className="flex items-center gap-2">
            <RefreshCw className="h-4 w-4 animate-spin text-blue-600" />
            <span className="text-blue-900 font-bold text-sm">{currentPhase}</span>
          </div>
          {progress !== undefined && (
            <div className="flex items-center gap-2">
              <div className="w-40 h-3 bg-blue-100 rounded-full overflow-hidden">
                <div
                  className="h-full bg-blue-500 transition-all duration-300"
                  style={{ width: `${progress}%` }}
                />
              </div>
              <span className="text-blue-700 text-sm font-bold">{progress}%</span>
            </div>
          )}
        </div>
      )}

      {/* ログビューア - 白背景、50-100行表示可能な高さ */}
      <div className="h-[500px] rounded-md border border-gray-300 bg-white p-2 overflow-auto font-mono text-xs shadow-inner">
        {(!logs || logs.length === 0) ? (
          <p className="text-gray-400">スキャンログ待機中...</p>
        ) : (
          <>
            {logs.map((log, i) => (
              <div key={i} className="flex gap-2 py-0.5 hover:bg-gray-50 border-b border-gray-100">
                <span className="text-gray-400 flex-shrink-0 text-[10px]">
                  {new Date(log.timestamp).toLocaleTimeString('ja-JP')}
                </span>
                <span className="text-gray-600 flex-shrink-0 w-28 font-medium">
                  {log.ip_address === "*" ? "SYSTEM" : log.ip_address}
                </span>
                <span className={cn("flex-1", getLogStyle(log))}>
                  {log.message}
                </span>
              </div>
            ))}
            <div ref={logEndRef} />
          </>
        )}
      </div>

      {/* ログ件数表示 */}
      <div className="text-xs text-gray-500 text-right">
        {logs?.length || 0} エントリ
      </div>
    </div>
  )
}

// Phase progress indicator with weighted progress display
// Stage weights defined inline in PhaseIndicator (T3-7: DynamicProgressCalculator)
function PhaseIndicator({
  currentPhase,
  progress,
}: {
  currentPhase: string
  progress?: number
}) {
  const phases = [
    { id: "arp", label: "ARP Scan", weight: 15, backendNames: ["hostdiscovery", "arp", "stage 0", "stage 1"] },
    { id: "port", label: "Port Scan", weight: 25, backendNames: ["portscan", "port", "stage 2", "stage 3"] },
    { id: "oui", label: "OUI Check", weight: 5, backendNames: ["ouilookup", "oui", "stage 4"] },
    { id: "onvif", label: "ONVIF", weight: 20, backendNames: ["onvifprobe", "onvif", "stage 5"] },
    { id: "rtsp", label: "RTSP", weight: 30, backendNames: ["rtspauth", "rtsp", "stage 6", "credential", "認証"] },
    { id: "match", label: "照合", weight: 5, backendNames: ["cameramatching", "match", "stage 7", "db"] },
  ]

  // バックエンドからの current_phase を正しくマッピング
  const normalizedPhase = currentPhase.toLowerCase().replace(/[_\s-]/g, '')
  const currentIndex = phases.findIndex((p) =>
    p.backendNames.some(name => normalizedPhase.includes(name.replace(/[_\s-]/g, '')))
  )

  // Calculate cumulative progress based on weights
  const calculateCumulativeWeight = (upToIndex: number): number => {
    return phases.slice(0, upToIndex + 1).reduce((sum, p) => sum + p.weight, 0)
  }

  return (
    <div className="space-y-2">
      {/* Phase pills */}
      <div className="flex items-center justify-center gap-1 flex-wrap">
        {phases.map((p, i) => {
          const isActive = i === currentIndex
          const isPast = i < currentIndex
          const isFuture = i > currentIndex

          return (
            <div key={p.id} className="flex items-center">
              <div
                className={cn(
                  "flex h-6 items-center justify-center rounded-full px-2 text-xs font-medium transition-all",
                  isActive &&
                    "bg-primary text-primary-foreground ring-2 ring-primary/30",
                  isPast && "bg-green-500/20 text-green-600 dark:text-green-400",
                  isFuture && "bg-muted text-muted-foreground"
                )}
              >
                {isPast ? (
                  <CheckCircle2 className="mr-1 h-3 w-3" />
                ) : isActive ? (
                  <RefreshCw className="mr-1 h-3 w-3 animate-spin" />
                ) : null}
                {p.label}
                <span className="ml-1 text-[10px] opacity-60">({p.weight}%)</span>
              </div>
              {i < phases.length - 1 && (
                <div
                  className={cn(
                    "mx-1 h-0.5 w-3",
                    isPast ? "bg-green-500" : "bg-muted"
                  )}
                />
              )}
            </div>
          )
        })}
      </div>
      {/* Weighted progress bar */}
      <div className="flex items-center gap-2 px-4">
        <div className="flex-1 h-2 bg-muted rounded-full overflow-hidden">
          {/* Show completed stages as segments */}
          <div className="h-full flex">
            {phases.map((p, i) => {
              const isPast = i < currentIndex
              const isActive = i === currentIndex
              const widthPercent = p.weight

              return (
                <div
                  key={p.id}
                  className={cn(
                    "h-full transition-all duration-300",
                    isPast && "bg-green-500",
                    isActive && "bg-primary animate-pulse",
                    !isPast && !isActive && "bg-transparent"
                  )}
                  style={{ width: `${widthPercent}%` }}
                />
              )
            })}
          </div>
        </div>
        <span className="text-xs text-muted-foreground min-w-[3rem] text-right">
          {progress !== undefined ? `${progress}%` : `${currentIndex >= 0 ? calculateCumulativeWeight(currentIndex - 1) : 0}%`}
        </span>
      </div>
    </div>
  )
}

// Device status badge based on DetectionReason
function DeviceStatusBadge({ device }: { device: ScannedDevice }) {
  const detection = device.detection

  if (!detection) {
    return (
      <Badge variant="outline" className="text-xs">
        未分類
      </Badge>
    )
  }

  const badgeConfig: Record<
    DeviceType,
    { label: string; variant: "default" | "secondary" | "outline" | "destructive" }
  > = {
    camera_confirmed: { label: "カメラ確認済", variant: "default" },
    camera_likely: { label: "カメラ可能性高", variant: "secondary" },
    camera_possible: { label: "カメラ可能性", variant: "outline" },
    nvr_likely: { label: "NVR可能性", variant: "secondary" },
    network_device: { label: "ネットワーク機器", variant: "outline" },
    other_device: { label: "その他", variant: "outline" },
    unknown: { label: "不明", variant: "outline" },
  }

  const config = badgeConfig[detection.device_type] || badgeConfig.unknown

  return (
    <Badge variant={config.variant} className="text-xs">
      {config.label}
    </Badge>
  )
}

// Scanned device card - Backend API応答に合わせて修正
// カテゴリbデバイスは強調表示（モデル名・認証成功ユーザー名を表示）
// カテゴリfデバイス（通信断・迷子）は警告表示
function DeviceCard({
  device,
  selected,
  onToggle,
  categoryBgClass,
}: {
  device: ScannedDevice | CategorizedDevice
  selected: boolean
  onToggle: () => void
  categoryBgClass?: string
}) {
  const detection = device.detection
  const isRegisterable = device.credential_status === 'success' && device.model
  const categorizedDevice = device as CategorizedDevice
  const isLostConnection = categorizedDevice.category === 'f'
  const isStrayChild = device.ip_changed === true
  const isRegisteredCamera = categorizedDevice.category === 'a'  // RT-05: カテゴリA判定

  return (
    <Card
      className={cn(
        "cursor-pointer p-3 transition-all",
        categoryBgClass || "hover:bg-accent/50",
        selected && "ring-2 ring-primary",
        isRegisterable && "shadow-md", // カテゴリb強調
        isLostConnection && "border-l-4 border-l-red-500" // カテゴリf強調
      )}
      onClick={onToggle}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex-1 min-w-0">
          {/* RT-05: 登録済みカメラ名表示（カテゴリA） */}
          {isRegisteredCamera && device.registered_camera_name && (
            <p className="text-base font-bold text-green-700 dark:text-green-300 mb-1">
              {device.registered_camera_name}
            </p>
          )}
          {/* IP + Status Badge */}
          <div className="flex items-center gap-2 flex-wrap">
            <Wifi className={cn(
              "h-4 w-4 flex-shrink-0",
              isLostConnection ? "text-red-500" :
              isRegisteredCamera ? "text-green-500" : "text-muted-foreground"
            )} />
            <span className={cn(
              "font-mono text-sm font-medium",
              isLostConnection && "text-red-700",
              isRegisteredCamera && "text-green-700 dark:text-green-300"
            )}>
              {device.ip}
            </span>
            <DeviceStatusBadge device={device} />
            {/* RT-05: カテゴリAバッジ */}
            {isRegisteredCamera && (
              <Badge variant="default" className="text-xs bg-green-600">
                登録済
              </Badge>
            )}
            {/* Credential success indicator */}
            {device.credential_status === 'success' && !isRegisteredCamera && (
              <Badge variant="default" className="text-xs bg-green-600">
                認証済
              </Badge>
            )}
            {/* Category F badges */}
            {isLostConnection && !isStrayChild && (
              <Badge variant="destructive" className="text-xs">
                通信断
              </Badge>
            )}
            {isStrayChild && (
              <Badge variant="outline" className="text-xs border-orange-500 text-orange-600">
                IP変更検出
              </Badge>
            )}
          </div>
          {/* Registered camera name for category F */}
          {isLostConnection && device.registered_camera_name && (
            <p className="mt-1 text-sm font-medium text-red-700 dark:text-red-400">
              登録名: {device.registered_camera_name}
            </p>
          )}
          {/* MAC + OUI Vendor */}
          <div className="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
            {device.mac && (
              <span className="font-mono">{device.mac}</span>
            )}
            {device.oui_vendor && (
              <Badge
                variant="outline"
                className={cn(
                  "text-xs px-1 py-0",
                  device.oui_vendor === "TP-LINK" && "text-red-600 font-bold border-red-400"
                )}
              >
                {device.oui_vendor}
              </Badge>
            )}
          </div>
          {/* Subnet info */}
          {device.subnet && (
            <p className="mt-0.5 text-xs text-muted-foreground">
              Subnet: {device.subnet}
            </p>
          )}
          {/* RT-06: タイムスタンプ表示 */}
          {device.last_seen && (
            <p className="mt-0.5 text-xs text-muted-foreground">
              最終検出: {new Date(device.last_seen).toLocaleString('ja-JP', {
                month: 'numeric',
                day: 'numeric',
                hour: '2-digit',
                minute: '2-digit'
              })}
              {device.first_seen && device.first_seen !== device.last_seen && (
                <span className="ml-2 text-gray-400">
                  (初回: {new Date(device.first_seen).toLocaleDateString('ja-JP')})
                </span>
              )}
            </p>
          )}
          {/* Model info - 強調表示（カテゴリb） */}
          {device.model && (
            <p className={cn(
              "mt-1 truncate",
              isRegisterable
                ? "text-sm font-semibold text-blue-700 dark:text-blue-300"
                : "text-xs text-muted-foreground"
            )}>
              {device.manufacturer && `${device.manufacturer} `}
              {device.model}
              {device.firmware && ` (${device.firmware})`}
            </p>
          )}
          {/* Credential username on success */}
          {device.credential_status === 'success' && device.credential_username && (
            <div className="mt-1 flex items-center gap-1 text-xs text-green-600 dark:text-green-400">
              <Key className="h-3 w-3" />
              <span>認証: {device.credential_username}</span>
            </div>
          )}
          {/* Tried credentials display (T3-10: 平文表示) */}
          {device.tried_credentials && device.tried_credentials.length > 0 && (
            <div className="mt-2 text-xs border-t pt-2">
              <p className="text-muted-foreground mb-1 flex items-center gap-1">
                <Key className="h-3 w-3" />
                試行クレデンシャル:
              </p>
              <div className="space-y-0.5 font-mono text-[11px]">
                {device.tried_credentials.map((cred, idx) => (
                  <div key={idx} className="flex items-center gap-2">
                    <span className={cn(
                      cred.result === 'success' ? 'text-green-600' :
                      cred.result === 'timeout' ? 'text-amber-600' : 'text-red-500'
                    )}>
                      {cred.result === 'success' ? '✓' : cred.result === 'timeout' ? '⏱' : '×'}
                    </span>
                    <span>{cred.username} / {cred.password}</span>
                    <span className="text-muted-foreground">
                      → {cred.result === 'success' ? '成功' :
                         cred.result === 'timeout' ? 'タイムアウト' : '失敗'}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
          {/* Category F help message */}
          {isLostConnection && (
            <div className="mt-2 text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 rounded p-2">
              <p className="font-medium mb-1">考えられる原因:</p>
              <ul className="list-disc list-inside space-y-0.5">
                <li>電源が切れている</li>
                <li>ネットワークケーブルが抜けている</li>
                {isStrayChild && <li>IPアドレスが変更された（DHCPリース切れ）</li>}
              </ul>
            </div>
          )}
          {/* Detection message */}
          {detection && !isRegisterable && !isLostConnection && (
            <p className="mt-1 text-xs text-muted-foreground">
              {detection.user_message}
            </p>
          )}
          {/* Credential failed warning */}
          {device.credential_status === 'failed' && !isLostConnection && (
            <div className="mt-1 flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400">
              <Key className="h-3 w-3" />
              <span>全クレデンシャル不一致 - 設定確認が必要</span>
            </div>
          )}
          {/* Credential not tried + auth needed */}
          {device.credential_status === 'not_tried' && detection?.suggested_action === "set_credentials" && (
            <div className="mt-1 flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400">
              <Key className="h-3 w-3" />
              <span>クレデンシャル設定が必要</span>
            </div>
          )}
        </div>
        <div
          className={cn(
            "flex h-5 w-5 items-center justify-center rounded border-2 transition-colors",
            selected
              ? "border-primary bg-primary text-primary-foreground"
              : "border-muted-foreground/30"
          )}
        >
          {selected && <CheckCircle2 className="h-3 w-3" />}
        </div>
      </div>
    </Card>
  )
}

// Category section component with collapsible
// RT-09: 説明文と推奨アクションを表示
function CategorySection({
  category,
  devices,
  selectedIds,
  onToggle,
  collapsed,
  onToggleCollapse,
}: {
  category: typeof DEVICE_CATEGORIES[number]
  devices: CategorizedDevice[]
  selectedIds: Set<string>
  onToggle: (ip: string) => void
  collapsed: boolean
  onToggleCollapse: () => void
}) {
  if (devices.length === 0) return null

  const isCollapsible = category.id === 'd' || category.id === 'e'
  const isCategoryA = category.id === 'a'
  const isCategoryF = category.id === 'f'
  const isCategoryB = category.id === 'b'
  const isCategoryC = category.id === 'c'

  return (
    <div className="space-y-2">
      {/* Section header */}
      <div
        className={cn(
          "flex flex-col gap-1 px-3 py-2 rounded-lg",
          isCollapsible && "cursor-pointer hover:bg-muted/50",
          isCategoryA && "bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800",
          isCategoryB && "bg-blue-50 dark:bg-blue-900/20 border-2 border-blue-500",
          isCategoryC && "bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-300",
          isCategoryF && "bg-red-50 dark:bg-red-900/20 border-2 border-red-400"
        )}
        onClick={isCollapsible ? onToggleCollapse : undefined}
      >
        <div className="flex items-center gap-2">
          {isCollapsible && (
            collapsed ? (
              <ChevronRight className="h-4 w-4 text-muted-foreground" />
            ) : (
              <ChevronDown className="h-4 w-4 text-muted-foreground" />
            )
          )}
          {isCategoryA && <CheckCircle2 className="h-4 w-4 text-green-600" />}
          {isCategoryB && <Plus className="h-4 w-4 text-blue-600" />}
          {isCategoryC && <Key className="h-4 w-4 text-yellow-600" />}
          {isCategoryF && <AlertCircle className="h-4 w-4 text-red-500" />}
          <span className={cn(
            "text-sm font-semibold",
            isCategoryA && "text-green-700 dark:text-green-300",
            isCategoryB && "text-blue-700 dark:text-blue-300",
            isCategoryC && "text-yellow-700 dark:text-yellow-300",
            isCategoryF && "text-red-700 dark:text-red-300"
          )}>
            {category.label}
          </span>
          <Badge
            variant={isCategoryF ? "destructive" : isCategoryB ? "default" : "secondary"}
            className={cn("ml-1", isCategoryB && "bg-blue-600")}
          >
            {devices.length}台
          </Badge>
        </div>
        {/* RT-09: 説明文 */}
        <p className="text-xs text-muted-foreground ml-6">
          {category.description}
        </p>
        {/* RT-09: 推奨アクション */}
        {category.userAction && (isCategoryB || isCategoryC || isCategoryF) && (
          <p className={cn(
            "text-xs font-medium ml-6 mt-1",
            isCategoryB && "text-blue-600 dark:text-blue-400",
            isCategoryC && "text-yellow-600 dark:text-yellow-400",
            isCategoryF && "text-red-600 dark:text-red-400"
          )}>
            → {category.userAction}
          </p>
        )}
      </div>

      {/* Devices list */}
      {!collapsed && (
        <div className="grid gap-2 pl-2">
          {devices.map((device) => (
            <DeviceCard
              key={device.ip}
              device={device}
              selected={selectedIds.has(device.ip)}
              onToggle={() => onToggle(device.ip)}
              categoryBgClass={category.bgClass}
            />
          ))}
        </div>
      )}
    </div>
  )
}

export function ScanModal({
  open,
  onOpenChange,
  onDevicesRegistered,
}: ScanModalProps) {
  const [scanJob, setScanJob] = useState<ScanJob | null>(null)
  const [devices, setDevices] = useState<ScannedDevice[]>([])
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  const [confirmOpen, setConfirmOpen] = useState(false)
  const [closeConfirmOpen, setCloseConfirmOpen] = useState(false)  // IS22_UI_DETAILED_SPEC Section 3.4
  const [registering, setRegistering] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // ===== Brute Force Mode (Camscan_designers_review.md Item 4) =====
  // デフォルトOFF: カメラの可能性が低いデバイスにはクレデンシャル試行をスキップ
  const [bruteForceMode, setBruteForceMode] = useState(false)

  // ===== スキャン中止処理用 =====
  const [aborting, setAborting] = useState(false)

  // ===== スキャン完了フロー状態 (is22_14_UI改善設計_v1.1.md Section 5) =====
  // 'idle' | 'scanning' | 'complete_display' | 'summary_display' | 'registration'
  const [completionPhase, setCompletionPhase] = useState<
    'idle' | 'scanning' | 'complete_display' | 'summary_display' | 'registration'
  >('idle')

  // ===== 前回スキャン結果キャッシュ (is22_14_UI改善設計_v1.1.md Section 1) =====
  const [cachedResult, setCachedResult] = useState<CachedScanResult | null>(null)

  // ===== スキャン中の登録可能カメラスタック (is22_14_UI改善設計_v1.1.md Section 4) =====
  // TODO: バックエンドがスキャン中のリアルタイムデバイス取得をサポートしたら実装
  // 現在はスキャン完了後にのみデバイスリストが取得されるため、リアルタイム表示は未実装

  // サブネット状態（APIから取得）
  const [subnets, setSubnets] = useState<Subnet[]>([])
  const [selectedSubnetIds, setSelectedSubnetIds] = useState<Set<string>>(new Set())
  const [newSubnetCidr, setNewSubnetCidr] = useState("")
  const [showAddSubnet, setShowAddSubnet] = useState(false)
  const [loadingSubnets, setLoadingSubnets] = useState(false)
  const [editingSubnet, setEditingSubnet] = useState<Subnet | null>(null)

  // 登録済みカメラ（カテゴリ判定用）
  const [registeredCameras, setRegisteredCameras] = useState<Camera[]>([])

  // カテゴリ折りたたみ状態（d, e はデフォルト折りたたみ）
  const [collapsedCategories, setCollapsedCategories] = useState<Set<DeviceCategory>>(
    new Set(['d', 'e'])
  )

  // 登録済みカメラのIPセット
  const registeredIPs = useMemo(() => {
    return new Set(
      registeredCameras
        .map((c) => c.ip_address)
        .filter((ip): ip is string => ip !== null)
    )
  }, [registeredCameras])

  // デバイスをカテゴリ化・ソート
  const categorizedDevices = useMemo(() => {
    return categorizeAndSortDevices(devices, registeredIPs)
  }, [devices, registeredIPs])

  // カテゴリ別にグループ化
  const devicesByCategory = useMemo(() => {
    const grouped: Record<DeviceCategory, CategorizedDevice[]> = {
      a: [],
      b: [],
      c: [],
      d: [],
      e: [],
      f: [],
    }
    for (const device of categorizedDevices) {
      grouped[device.category].push(device)
    }
    return grouped
  }, [categorizedDevices])

  // モーダルを開いたときにサブネット一覧、登録済みカメラ、前回結果キャッシュを取得
  useEffect(() => {
    if (open && !scanJob && completionPhase === 'idle') {
      setLoadingSubnets(true)

      // 前回スキャン結果キャッシュを読み込み
      const cached = loadScanResultFromCache()
      setCachedResult(cached)

      // 並列でサブネットとカメラを取得
      Promise.all([
        fetch(`${API_BASE_URL}/api/subnets`)
          .then((res) => res.json())
          .then((data) => {
            const list = data.data || data
            setSubnets(Array.isArray(list) ? list : [])
          }),
        fetch(`${API_BASE_URL}/api/cameras`)
          .then((res) => res.json())
          .then((data) => {
            const list = data.data || data
            setRegisteredCameras(Array.isArray(list) ? list : [])
          }),
      ])
        .catch((err) => {
          console.error("Failed to fetch data:", err)
        })
        .finally(() => {
          setLoadingSubnets(false)
        })
    }
  }, [open, scanJob, completionPhase])

  // 選択されたサブネットのCIDRリスト
  const selectedSubnets = subnets
    .filter((s) => selectedSubnetIds.has(s.subnet_id))
    .map((s) => s.cidr)

  // Start scan
  const startScan = useCallback(async () => {
    console.log("[ScanModal] startScan called, subnets:", selectedSubnets)
    if (selectedSubnets.length === 0) {
      setError("スキャン対象のサブネットを選択してください")
      return
    }

    setError(null)
    setDevices([])
    setSelectedIds(new Set())
    setCompletionPhase('scanning')   // スキャン中状態へ
    setCachedResult(null)            // 前回結果を非表示

    try {
      console.log("[ScanModal] POST /api/ipcamscan/jobs, bruteForceMode:", bruteForceMode)
      const response = await fetch(
        `${API_BASE_URL}/api/ipcamscan/jobs`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            targets: selectedSubnets,
            brute_force: bruteForceMode,  // Camscan_designers_review.md Item 4
          }),
        }
      )

      console.log("[ScanModal] POST response status:", response.status)
      if (!response.ok) {
        throw new Error(`Scan failed: ${response.statusText}`)
      }

      const data = await response.json()
      console.log("[ScanModal] POST response data keys:", Object.keys(data))
      // Extract job from API response wrapper
      const job = data.data || data
      console.log("[ScanModal] Job created:", job.job_id, "status:", job.status)
      setScanJob(job)
    } catch (err) {
      console.error("[ScanModal] startScan error:", err)
      setError(err instanceof Error ? err.message : "スキャン開始に失敗しました")
      setCompletionPhase('idle')
    }
  }, [selectedSubnets])

  // Poll for scan progress
  useEffect(() => {
    console.log("[ScanModal] useEffect triggered, scanJob:", scanJob?.job_id, "status:", scanJob?.status)
    if (!scanJob || scanJob.status === "success" || scanJob.status === "failed") {
      console.log("[ScanModal] Skipping poll setup (no job or already complete)")
      return
    }

    console.log("[ScanModal] Setting up poll interval for job:", scanJob.job_id)
    const pollInterval = setInterval(async () => {
      const pollUrl = `${API_BASE_URL}/api/ipcamscan/jobs/${scanJob.job_id}`
      console.log("[ScanModal] Polling:", pollUrl)
      try {
        const response = await fetch(pollUrl)
        console.log("[ScanModal] Poll response status:", response.status)

        if (!response.ok) {
          throw new Error("Failed to fetch job status")
        }

        const data = await response.json()
        console.log("[ScanModal] Poll raw data keys:", Object.keys(data))
        // Extract job from API response wrapper
        const job = data.data || data
        console.log("[ScanModal] Poll job status:", job.status, "phase:", job.current_phase, "progress:", job.progress_percent)

        // ステータス変化を検出
        if (job.status !== scanJob.status) {
          console.log("[ScanModal] STATUS CHANGED:", scanJob.status, "->", job.status)
        }

        setScanJob(job)

        if (job.status === "success") {
          console.log("[ScanModal] === JOB SUCCESS - FETCHING DEVICES ===")
          // Fetch discovered devices
          const devicesUrl = `${API_BASE_URL}/api/ipcamscan/devices`
          console.log("[ScanModal] Fetching devices from:", devicesUrl)
          const devicesResponse = await fetch(devicesUrl)
          console.log("[ScanModal] Devices response status:", devicesResponse.status, devicesResponse.statusText)

          if (devicesResponse.ok) {
            const devicesData = await devicesResponse.json()
            console.log("[ScanModal] Devices raw keys:", Object.keys(devicesData))
            console.log("[ScanModal] Has 'devices' key:", 'devices' in devicesData)
            console.log("[ScanModal] Has 'data' key:", 'data' in devicesData)

            // Extract devices from API response wrapper
            // API returns { devices: [...] } not { data: [...] }
            const devicesList = devicesData.devices || devicesData.data || devicesData
            const isArray = Array.isArray(devicesList)
            console.log("[ScanModal] devicesList isArray:", isArray, "length:", isArray ? devicesList.length : "N/A")

            if (isArray && devicesList.length > 0) {
              console.log("[ScanModal] First device sample:", JSON.stringify(devicesList[0]).substring(0, 200))
            }

            console.log("[ScanModal] Calling setDevices with", isArray ? devicesList.length : 0, "items")
            const finalDevices = isArray ? devicesList : []
            setDevices(finalDevices)

            // ===== キャッシュ保存 (is22_14_UI改善設計_v1.1.md Section 1) =====
            saveScanResultToCache(finalDevices, selectedSubnets, job.summary)

            // ===== スキャン完了フロー開始 (is22_14_UI改善設計_v1.1.md Section 5) =====
            // Phase A: スキャン完了表示（2秒）
            setCompletionPhase('complete_display')
            setTimeout(() => {
              // Phase C: サマリー表示（3秒）
              setCompletionPhase('summary_display')
              setTimeout(() => {
                // Phase D: 登録画面へ遷移
                setCompletionPhase('registration')
              }, 3000)
            }, 2000)
          } else {
            console.error("[ScanModal] Devices fetch failed:", devicesResponse.status, devicesResponse.statusText)
            setCompletionPhase('registration')  // エラー時も登録画面へ
          }
        }
      } catch (err) {
        console.error("[ScanModal] Poll error:", err)
      }
    }, 2000)

    return () => {
      console.log("[ScanModal] Clearing poll interval")
      clearInterval(pollInterval)
    }
  }, [scanJob])

  // NOTE: 自動スキャン開始を削除。ユーザーが「スキャン開始」ボタンを押すまで待機。

  // Reset on close
  useEffect(() => {
    if (!open) {
      setScanJob(null)
      setDevices([])
      setSelectedIds(new Set())
      setError(null)
      setSelectedSubnetIds(new Set())
      setShowAddSubnet(false)
      setNewSubnetCidr("")
      setRegisteredCameras([])
      setCollapsedCategories(new Set(['d', 'e']))
      setCompletionPhase('idle')          // 完了フロー状態リセット
      // Note: cachedResult は次回表示用に保持（LocalStorageから再読み込み）
    }
  }, [open])

  // カテゴリ折りたたみトグル
  const toggleCategoryCollapse = useCallback((categoryId: DeviceCategory) => {
    setCollapsedCategories((prev) => {
      const next = new Set(prev)
      if (next.has(categoryId)) {
        next.delete(categoryId)
      } else {
        next.add(categoryId)
      }
      return next
    })
  }, [])

  // サブネット追加（POST /api/subnets）→ 追加後に編集モーダルを表示
  const addSubnet = async () => {
    if (!newSubnetCidr.trim()) return

    try {
      const response = await fetch(`${API_BASE_URL}/api/subnets`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          cidr: newSubnetCidr.trim(),
          fid: "",  // デフォルトは空文字（"0000"は別の意味を持つため禁止）
          description: "Added from scan modal",
        }),
      })

      if (!response.ok) {
        throw new Error("Failed to add subnet")
      }

      const data = await response.json()
      const newSubnet = data.data || data
      setSubnets((prev) => [...prev, newSubnet])
      setSelectedSubnetIds((prev) => new Set([...prev, newSubnet.subnet_id]))
      setNewSubnetCidr("")
      setShowAddSubnet(false)
      // 追加後に編集モーダルを表示してfid/tid/クレデンシャルを設定可能に
      setEditingSubnet(newSubnet)
    } catch (err) {
      setError(err instanceof Error ? err.message : "サブネット追加に失敗しました")
    }
  }

  // サブネット設定を保存 (PUT /api/subnets/:id)
  const saveSubnetSettings = async (subnetId: string, updates: Partial<Subnet>) => {
    const response = await fetch(`${API_BASE_URL}/api/subnets/${subnetId}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(updates),
    })

    if (!response.ok) {
      throw new Error("Failed to save subnet settings")
    }

    // ローカル状態を更新
    setSubnets((prev) =>
      prev.map((s) =>
        s.subnet_id === subnetId
          ? { ...s, ...updates }
          : s
      )
    )
  }

  // サブネット削除 (DELETE /api/subnets/:id)
  // RT-08: サブネット削除確認状態
  const [deletingSubnet, setDeletingSubnet] = useState<Subnet | null>(null)

  // RT-08: サブネット削除（確認後実行）
  const confirmDeleteSubnet = (subnetId: string) => {
    const subnet = subnets.find((s) => s.subnet_id === subnetId)
    if (subnet) {
      setDeletingSubnet(subnet)
    }
  }

  const executeDeleteSubnet = async () => {
    if (!deletingSubnet) return

    try {
      // RT-08: cleanup_scan_results=true パラメータでスキャン結果も削除
      const response = await fetch(
        `${API_BASE_URL}/api/subnets/${deletingSubnet.subnet_id}?cleanup_scan_results=true`,
        { method: "DELETE" }
      )

      if (!response.ok) {
        throw new Error("Failed to delete subnet")
      }

      // ローカル状態から削除
      setSubnets((prev) => prev.filter((s) => s.subnet_id !== deletingSubnet.subnet_id))
      setSelectedSubnetIds((prev) => {
        const next = new Set(prev)
        next.delete(deletingSubnet.subnet_id)
        return next
      })
      setDeletingSubnet(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "サブネット削除に失敗しました")
      setDeletingSubnet(null)
    }
  }

  // デバイス選択トグル - device.ip を使用
  const toggleDevice = (ip: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      if (next.has(ip)) {
        next.delete(ip)
      } else {
        next.add(ip)
      }
      return next
    })
  }

  // カメラを全選択 - カテゴリb（登録可能）のみを選択
  const selectAll = () => {
    const registerableDevices = devicesByCategory.b
    setSelectedIds(new Set(registerableDevices.map((d) => d.ip)))
  }

  // RT-07: カテゴリCも含めて全選択
  const selectAllIncludingPending = () => {
    const devices = [...devicesByCategory.b, ...devicesByCategory.c]
    setSelectedIds(new Set(devices.map((d) => d.ip)))
  }

  const selectNone = () => {
    setSelectedIds(new Set())
  }

  // 登録可能デバイス数（カテゴリb + c）
  const registerableCount = devicesByCategory.b.length
  const pendingAuthCount = devicesByCategory.c.length

  // RT-07: 選択中のカテゴリCデバイス数（強制登録対象）
  const selectedCategoryC = useMemo(() => {
    return devicesByCategory.c.filter((d) => selectedIds.has(d.ip))
  }, [devicesByCategory.c, selectedIds])

  const handleRegister = async () => {
    if (selectedIds.size === 0) return

    setRegistering(true)

    try {
      // RT-07: カテゴリB + カテゴリC（強制登録）の両方から選択デバイスを取得
      const selectedFromB = devicesByCategory.b.filter((d) => selectedIds.has(d.ip))
      const selectedFromC = devicesByCategory.c.filter((d) => selectedIds.has(d.ip))
      const allSelectedDevices = [...selectedFromB, ...selectedFromC]

      // バックエンドが期待する形式に変換
      const devices = allSelectedDevices.map((device) => {
        // デバイスのサブネットからfidを取得
        const subnet = subnets.find((s) => s.cidr === device.subnet)
        const fid = subnet?.fid || "0000"

        // カテゴリBならクレデンシャルあり、カテゴリCならなし
        const isFromCategoryC = devicesByCategory.c.some((d) => d.ip === device.ip)
        const credentials =
          device.credential_status === "success" && device.credential_username
            ? {
                username: device.credential_username,
                password: device.credential_password || "",
              }
            : undefined

        return {
          ip: device.ip,
          display_name: device.model || device.manufacturer || `Camera-${device.ip.split(".").pop()}`,
          location: device.subnet,
          fid,
          credentials,
          // RT-07: 強制登録フラグ（カテゴリCの場合）
          force_register: isFromCategoryC,
          // RT-07: pending_auth ステータスで登録（カテゴリCの場合）
          initial_status: isFromCategoryC ? 'pending_auth' : 'active',
        }
      })

      const response = await fetch(`${API_BASE_URL}/api/ipcamscan/devices/approve-batch`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ devices }),
      })

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}))
        throw new Error(errorData.error || "Registration failed")
      }

      const data = await response.json()
      onDevicesRegistered?.(data.results?.length || selectedIds.size)
      setConfirmOpen(false)
      onOpenChange(false)
    } catch (err) {
      setError(err instanceof Error ? err.message : "登録に失敗しました")
    } finally {
      setRegistering(false)
    }
  }

  const isScanning = scanJob?.status === "running" || scanJob?.status === "queued" || completionPhase === 'scanning'

  // ===== スキャン中止 (Camscan_designers_review.md Item 3) =====
  const abortScan = useCallback(async () => {
    if (!scanJob) return

    setAborting(true)
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/ipcamscan/jobs/${scanJob.job_id}/abort`,
        { method: "POST" }
      )

      if (!response.ok) {
        // Fallback to DELETE if POST abort not supported
        const deleteResponse = await fetch(
          `${API_BASE_URL}/api/ipcamscan/jobs/${scanJob.job_id}`,
          { method: "DELETE" }
        )
        if (!deleteResponse.ok) {
          throw new Error("Failed to abort scan")
        }
      }

      // Fetch whatever devices were found so far
      const devicesResponse = await fetch(`${API_BASE_URL}/api/ipcamscan/devices`)
      if (devicesResponse.ok) {
        const devicesData = await devicesResponse.json()
        const devicesList = devicesData.devices || devicesData.data || devicesData
        if (Array.isArray(devicesList)) {
          setDevices(devicesList)
        }
      }

      setScanJob(null)
      setCompletionPhase('registration')
    } catch (err) {
      console.error("[ScanModal] abortScan error:", err)
      setError(err instanceof Error ? err.message : "スキャン中止に失敗しました")
    } finally {
      setAborting(false)
    }
  }, [scanJob])

  // 前回結果を読み込む
  const loadCachedResult = useCallback(() => {
    if (cachedResult) {
      setDevices(cachedResult.devices)
      setCompletionPhase('registration')
      setCachedResult(null)  // 表示後はキャッシュ参照を解除
    }
  }, [cachedResult])

  // Handle modal close with confirmation (IS22_UI_DETAILED_SPEC Section 3.4)
  const handleCloseAttempt = (shouldClose: boolean) => {
    if (!shouldClose) {
      onOpenChange(false)
      return
    }

    // Show confirmation if scanning or there are unregistered registerable devices
    const hasUnregisteredDevices = registerableCount > 0 && selectedIds.size < registerableCount
    if (isScanning || hasUnregisteredDevices) {
      setCloseConfirmOpen(true)
    } else {
      onOpenChange(false)
    }
  }

  const handleConfirmClose = () => {
    setCloseConfirmOpen(false)
    onOpenChange(false)
  }

  return (
    <>
      <Dialog open={open} onOpenChange={handleCloseAttempt}>
        <DialogContent className="w-[60vw] min-w-[800px] max-w-[1200px] h-[90vh] overflow-hidden flex flex-col">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Search className="h-5 w-5" />
              カメラスキャン
              {selectedSubnets.length > 0 && (
                <Badge variant="secondary" className="ml-2">
                  {selectedSubnets.length}サブネット
                </Badge>
              )}
            </DialogTitle>
          </DialogHeader>

          <div className="flex-1 overflow-hidden flex flex-col gap-4">
            {/* Initial state - Settings before scan (IS22_UI_DETAILED_SPEC Section 3.1) */}
            {!scanJob && completionPhase === 'idle' && (
              <div className="flex flex-col gap-6 py-4">
                {/* ===== 前回スキャン結果キャッシュ表示 (is22_14_UI改善設計_v1.1.md Section 1.4) ===== */}
                {cachedResult && (
                  <div className="rounded-lg border-2 border-blue-200 bg-blue-50 p-4 dark:border-blue-800 dark:bg-blue-900/20">
                    <div className="flex items-start gap-3">
                      <div className="flex h-10 w-10 items-center justify-center rounded-full bg-blue-100 dark:bg-blue-800">
                        <RefreshCw className="h-5 w-5 text-blue-600 dark:text-blue-300" />
                      </div>
                      <div className="flex-1">
                        <h4 className="font-medium text-blue-900 dark:text-blue-100">
                          前回スキャン結果があります
                        </h4>
                        <p className="text-sm text-blue-700 dark:text-blue-300 mt-1">
                          {new Date(cachedResult.timestamp).toLocaleString('ja-JP')} | {cachedResult.devices.length}台検出
                        </p>
                        <p className="text-xs text-blue-600 dark:text-blue-400 mt-0.5">
                          {cachedResult.subnets.join(', ')}
                        </p>
                        <div className="flex gap-2 mt-3">
                          <Button
                            size="sm"
                            variant="default"
                            onClick={loadCachedResult}
                          >
                            前回結果を表示
                          </Button>
                          <Button
                            size="sm"
                            variant="outline"
                            onClick={() => {
                              clearScanResultCache()
                              setCachedResult(null)
                            }}
                          >
                            新規スキャン
                          </Button>
                        </div>
                      </div>
                    </div>
                  </div>
                )}

                {/* 前回結果がある場合はセパレータを表示 */}
                {cachedResult && (
                  <div className="flex items-center gap-4 text-muted-foreground">
                    <div className="flex-1 h-px bg-border" />
                    <span className="text-xs">または</span>
                    <div className="flex-1 h-px bg-border" />
                  </div>
                )}

                {/* サブネット設定セクション */}
                <div className="space-y-3">
                  <h3 className="text-sm font-medium">スキャン対象サブネット（複数選択可）</h3>
                  <div className="rounded-lg border p-4 space-y-2">
                    {loadingSubnets ? (
                      <div className="flex items-center justify-center py-4">
                        <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
                        <span className="ml-2 text-sm text-muted-foreground">読み込み中...</span>
                      </div>
                    ) : subnets.length === 0 ? (
                      <p className="text-sm text-muted-foreground py-2">
                        登録済みサブネットがありません。下のボタンから追加してください。
                      </p>
                    ) : (
                      subnets.map((subnet) => (
                        <div
                          key={subnet.subnet_id}
                          className={cn(
                            "flex items-center gap-3 p-2 rounded transition-colors",
                            selectedSubnetIds.has(subnet.subnet_id)
                              ? "bg-primary/10"
                              : "hover:bg-muted/50"
                          )}
                        >
                          <label className="flex items-center gap-3 flex-1 min-w-0 cursor-pointer">
                            <input
                              type="checkbox"
                              className="h-4 w-4 rounded border-gray-300"
                              checked={selectedSubnetIds.has(subnet.subnet_id)}
                              onChange={(e) => {
                                setSelectedSubnetIds((prev) => {
                                  const next = new Set(prev)
                                  if (e.target.checked) {
                                    next.add(subnet.subnet_id)
                                  } else {
                                    next.delete(subnet.subnet_id)
                                  }
                                  return next
                                })
                              }}
                            />
                            <Wifi className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                            <div className="flex-1 min-w-0">
                              <span className="font-mono text-sm">{subnet.cidr}</span>
                              {subnet.facility_name && (
                                <span className="text-xs text-muted-foreground ml-2">
                                  {subnet.facility_name}
                                </span>
                              )}
                            </div>
                          </label>
                          {/* fid badge */}
                          <Badge variant="outline" className="text-xs flex-shrink-0">
                            fid:{subnet.fid || "未設定"}
                          </Badge>
                          {/* Credential count badge */}
                          {subnet.credentials && subnet.credentials.length > 0 && (
                            <Badge variant="secondary" className="text-xs flex-shrink-0">
                              <Key className="h-3 w-3 mr-1" />
                              {subnet.credentials.length}
                            </Badge>
                          )}
                          {/* Settings button */}
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation()
                              setEditingSubnet(subnet)
                            }}
                            title="サブネット設定"
                          >
                            <Settings2 className="h-4 w-4" />
                          </Button>
                          {/* Delete button */}
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation()
                              confirmDeleteSubnet(subnet.subnet_id)
                            }}
                            title="サブネット削除"
                            className="text-red-500 hover:text-red-700 hover:bg-red-50"
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      ))
                    )}

                    {/* サブネット追加フォーム */}
                    {showAddSubnet ? (
                      <div className="flex gap-2 mt-2 p-2 rounded border border-dashed">
                        <input
                          type="text"
                          className="flex-1 px-2 py-1 text-sm font-mono border rounded"
                          placeholder="192.168.x.0/24"
                          value={newSubnetCidr}
                          onChange={(e) => setNewSubnetCidr(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") {
                              addSubnet()
                            }
                          }}
                        />
                        <Button size="sm" onClick={addSubnet}>
                          追加
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => {
                            setShowAddSubnet(false)
                            setNewSubnetCidr("")
                          }}
                        >
                          キャンセル
                        </Button>
                      </div>
                    ) : (
                      <Button
                        variant="outline"
                        size="sm"
                        className="w-full mt-2"
                        onClick={() => setShowAddSubnet(true)}
                      >
                        <Plus className="mr-2 h-4 w-4" />
                        サブネットを追加
                      </Button>
                    )}
                  </div>
                </div>

                {/* ===== Brute Force Mode トグル (Camscan_designers_review.md Item 4) ===== */}
                <div className="rounded-lg border border-amber-200 bg-amber-50 p-4 dark:border-amber-800 dark:bg-amber-900/20">
                  <label className="flex items-start gap-3 cursor-pointer">
                    <input
                      type="checkbox"
                      className="mt-1 h-4 w-4 rounded border-amber-400"
                      checked={bruteForceMode}
                      onChange={(e) => setBruteForceMode(e.target.checked)}
                    />
                    <div className="flex-1">
                      <span className="font-medium text-amber-900 dark:text-amber-100">
                        Brute Force Mode
                      </span>
                      <p className="text-sm text-amber-700 dark:text-amber-300 mt-0.5">
                        カメラの可能性が低い検出対象にも全認証を試行します。
                        非常に時間がかかります。
                      </p>
                      {!bruteForceMode && (
                        <p className="text-xs text-amber-600 dark:text-amber-400 mt-1">
                          OFF: ONVIF/RTSP応答のないデバイスはクレデンシャル試行をスキップ
                        </p>
                      )}
                    </div>
                  </label>
                </div>

                {/* エラー表示 */}
                {error && (
                  <div className="flex items-center gap-2 rounded-lg border border-red-200 bg-red-50 p-3 dark:border-red-800 dark:bg-red-900/20">
                    <XCircle className="h-5 w-5 text-red-500" />
                    <span className="text-sm text-red-700 dark:text-red-300">
                      {error}
                    </span>
                  </div>
                )}

                {/* スキャン開始ボタン (BTN-004) */}
                <Button
                  size="lg"
                  className="w-full"
                  onClick={startScan}
                  disabled={selectedSubnets.length === 0}
                >
                  <Search className="mr-2 h-5 w-5" />
                  スキャン開始 ({selectedSubnets.length}サブネット)
                </Button>
              </div>
            )}

            {/* Scanning state (IS22_UI_DETAILED_SPEC Section 3.2) */}
            {isScanning && (
              <div className="flex flex-col gap-4 py-4">
                {/* Radar and status */}
                <div className="flex items-center gap-6">
                  <RadarSpinner className="h-24 w-24 flex-shrink-0" />
                  <div className="flex-1">
                    <p className="text-lg font-medium mb-2">
                      ネットワークをスキャン中...
                    </p>
                    {scanJob && (
                      <>
                        <PhaseIndicator
                          currentPhase={scanJob.current_phase || "scanning"}
                          progress={scanJob.progress_percent}
                        />
                        <p className="mt-3 text-sm text-muted-foreground text-center">
                          {scanJob.summary
                            ? `${scanJob.summary.hosts_alive}ホスト応答 / ${scanJob.summary.cameras_found}カメラ検出`
                            : "ネットワークを探索中..."}
                        </p>
                      </>
                    )}
                  </div>
                </div>
                {/* Scan detail log */}
                <div>
                  <ScanLogViewer
                    logs={scanJob?.logs || []}
                    currentPhase={scanJob?.current_phase}
                    progress={scanJob?.progress_percent}
                  />
                </div>

                {/* ===== スキャン中止ボタン (Camscan_designers_review.md Item 3) ===== */}
                <div className="flex justify-center mt-4">
                  <Button
                    variant="destructive"
                    size="lg"
                    onClick={abortScan}
                    disabled={aborting}
                  >
                    {aborting ? (
                      <>
                        <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                        中止処理中...
                      </>
                    ) : (
                      <>
                        <XCircle className="mr-2 h-4 w-4" />
                        スキャンを中止して結果表示
                      </>
                    )}
                  </Button>
                </div>
              </div>
            )}

            {/* ===== スキャン完了表示 Phase A (is22_14_UI改善設計_v1.1.md Section 5.2) ===== */}
            {completionPhase === 'complete_display' && (
              <div className="flex flex-col items-center justify-center py-16 animate-in fade-in duration-300">
                <div className="flex h-20 w-20 items-center justify-center rounded-full bg-green-100 dark:bg-green-900/30 mb-6">
                  <CheckCircle2 className="h-12 w-12 text-green-600 dark:text-green-400" />
                </div>
                <h2 className="text-2xl font-bold mb-2">スキャン完了</h2>
                <p className="text-lg text-muted-foreground">
                  {devices.length}台のデバイスを検出しました
                </p>
              </div>
            )}

            {/* ===== サマリー表示 Phase C (is22_14_UI改善設計_v1.1.md Section 5.2) ===== */}
            {completionPhase === 'summary_display' && (
              <div className="flex flex-col items-center justify-center py-8 animate-in fade-in duration-300">
                <div className="w-full max-w-md">
                  <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
                    <Search className="h-5 w-5" />
                    スキャンサマリー
                  </h2>
                  <div className="rounded-lg border bg-card p-6 space-y-4">
                    {scanJob?.summary && (
                      <>
                        <div className="grid grid-cols-2 gap-3 text-sm">
                          <div className="flex justify-between">
                            <span className="text-muted-foreground">スキャン対象:</span>
                            <span className="font-medium">{scanJob.summary.total_ips} IP ({selectedSubnets.length}サブネット)</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-muted-foreground">応答ホスト:</span>
                            <span className="font-medium">{scanJob.summary.hosts_alive}台</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-muted-foreground">カメラ検出:</span>
                            <span className="font-medium">{scanJob.summary.cameras_found}台</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-muted-foreground">認証成功:</span>
                            <span className="font-medium text-green-600">{scanJob.summary.cameras_verified}台</span>
                          </div>
                        </div>
                        <div className="border-t pt-4 space-y-2">
                          <div className="flex items-center gap-2">
                            <div className="h-3 w-3 rounded-full bg-green-500" />
                            <span className="text-sm">登録可能: {devicesByCategory.b.length}台</span>
                          </div>
                          <div className="flex items-center gap-2">
                            <div className="h-3 w-3 rounded-full bg-yellow-500" />
                            <span className="text-sm">認証必要: {devicesByCategory.c.length}台</span>
                          </div>
                          {devicesByCategory.f.length > 0 && (
                            <div className="flex items-center gap-2">
                              <div className="h-3 w-3 rounded-full bg-red-500" />
                              <span className="text-sm text-red-600 font-medium">
                                通信断・迷子: {devicesByCategory.f.length}台
                              </span>
                            </div>
                          )}
                          <div className="flex items-center gap-2">
                            <div className="h-3 w-3 rounded-full bg-gray-400" />
                            <span className="text-sm">その他: {devicesByCategory.d.length + devicesByCategory.e.length}台</span>
                          </div>
                        </div>
                      </>
                    )}
                    <div className="border-t pt-4 text-center">
                      <p className="text-sm text-muted-foreground animate-pulse">
                        登録画面へ進みます...
                      </p>
                      <div className="mt-2 h-1 bg-muted rounded-full overflow-hidden">
                        <div className="h-full bg-primary animate-[progress_3s_linear]" style={{ width: '100%' }} />
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Scan complete - device list by category */}
            {completionPhase === 'registration' && devices.length > 0 && (
              <>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <CameraIcon className="h-5 w-5" />
                    <span className="font-medium">
                      検出デバイス ({devices.length}台)
                    </span>
                    {/* カテゴリ別件数サマリ */}
                    <div className="flex gap-1 ml-2">
                      {DEVICE_CATEGORIES.map((cat) => {
                        const count = devicesByCategory[cat.id].length
                        if (count === 0) return null
                        return (
                          <Badge
                            key={cat.id}
                            variant={cat.id === 'b' ? 'default' : 'outline'}
                            className="text-xs"
                          >
                            {cat.id.toUpperCase()}:{count}
                          </Badge>
                        )
                      })}
                    </div>
                  </div>
                  {/* RT-07: 選択ボタン群 */}
                  <div className="flex gap-2 flex-wrap">
                    <Button variant="outline" size="sm" onClick={selectAll}>
                      認証済カメラを選択 ({registerableCount})
                    </Button>
                    {pendingAuthCount > 0 && (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={selectAllIncludingPending}
                        className="border-yellow-400 text-yellow-700 hover:bg-yellow-50"
                      >
                        認証待ちも含めて選択 (+{pendingAuthCount})
                      </Button>
                    )}
                    <Button variant="ghost" size="sm" onClick={selectNone}>
                      選択解除
                    </Button>
                  </div>
                </div>
                <div className="flex-1 overflow-auto">
                  <div className="space-y-4">
                    {DEVICE_CATEGORIES.map((category) => (
                      <CategorySection
                        key={category.id}
                        category={category}
                        devices={devicesByCategory[category.id]}
                        selectedIds={selectedIds}
                        onToggle={toggleDevice}
                        collapsed={collapsedCategories.has(category.id)}
                        onToggleCollapse={() => toggleCategoryCollapse(category.id)}
                      />
                    ))}
                  </div>
                </div>
              </>
            )}

            {/* Scan complete - no devices */}
            {completionPhase === 'registration' && devices.length === 0 && (
              <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
                <AlertCircle className="h-12 w-12 mb-4" />
                <p className="text-lg font-medium">デバイスが見つかりませんでした</p>
                <p className="text-sm">
                  サブネット設定を確認するか、再スキャンしてください
                </p>
                <Button
                  variant="outline"
                  className="mt-4"
                  onClick={() => {
                    setScanJob(null)
                    startScan()
                  }}
                >
                  <RefreshCw className="mr-2 h-4 w-4" />
                  再スキャン
                </Button>
              </div>
            )}
          </div>

          {/* Footer */}
          {completionPhase === 'registration' && devices.length > 0 && (
            <div className="flex items-center justify-between border-t pt-4 mt-4">
              <div className="text-sm text-muted-foreground">
                <span>{selectedIds.size}台選択中</span>
                {/* RT-07: 強制登録デバイス数の表示 */}
                {selectedCategoryC.length > 0 && (
                  <span className="ml-2 text-yellow-600">
                    (うち{selectedCategoryC.length}台は認証待ち)
                  </span>
                )}
              </div>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  onClick={() => onOpenChange(false)}
                >
                  キャンセル
                </Button>
                <Button
                  disabled={selectedIds.size === 0}
                  onClick={() => setConfirmOpen(true)}
                >
                  選択したデバイスを登録 ({selectedIds.size})
                </Button>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title="カメラを登録しますか？"
        description={
          selectedCategoryC.length > 0
            ? `${selectedIds.size}台のデバイスをカメラとして登録します。\n\n⚠️ ${selectedCategoryC.length}台は認証情報が未設定のため「認証待ち」ステータスで登録されます。後から認証情報を設定してください。`
            : `${selectedIds.size}台のデバイスをカメラとして登録します。登録後、ストリーム設定が必要な場合があります。`
        }
        type="info"
        confirmLabel="登録する"
        onConfirm={handleRegister}
        loading={registering}
      />

      {/* Close confirmation dialog (IS22_UI_DETAILED_SPEC Section 3.4) */}
      <ConfirmDialog
        open={closeConfirmOpen}
        onOpenChange={setCloseConfirmOpen}
        title="CameraScanを終了しますか？"
        description={
          isScanning
            ? "スキャン処理中です。終了するとスキャン結果が失われます。"
            : `検出デバイス: ${devices.length}台\n登録可能なカメラ: ${registerableCount}台\n選択中: ${selectedIds.size}台\n\n登録されていない登録可能デバイスがあります。CameraScanを終了してもよろしいですか？`
        }
        type="warning"
        confirmLabel="終了する"
        cancelLabel="キャンセル"
        onConfirm={handleConfirmClose}
      />

      {/* RT-08: サブネット削除確認ダイアログ */}
      <ConfirmDialog
        open={!!deletingSubnet}
        onOpenChange={(open) => !open && setDeletingSubnet(null)}
        title="サブネットを削除しますか？"
        description={
          deletingSubnet
            ? `サブネット ${deletingSubnet.cidr} を削除します。\n\n` +
              `• このサブネットのスキャン結果は削除されます\n` +
              `• 既に登録済みのカメラは影響を受けません\n\n` +
              `この操作は取り消せません。続行しますか？`
            : ""
        }
        type="warning"
        confirmLabel="削除する"
        cancelLabel="キャンセル"
        onConfirm={executeDeleteSubnet}
      />

      {/* Subnet credential editor modal (is22_ScanModal_Credential_Trial_Spec.md Section 6) */}
      {editingSubnet && (
        <SubnetCredentialEditor
          subnet={editingSubnet}
          allSubnets={subnets}
          onSave={async (updates) => {
            await saveSubnetSettings(editingSubnet.subnet_id, updates)
          }}
          onClose={() => setEditingSubnet(null)}
        />
      )}
    </>
  )
}
