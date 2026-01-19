import { useState, useEffect, useCallback } from "react"
import { PresetSelector } from "@/components/PresetSelector"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ConfirmDialog } from "@/components/ConfirmDialog"
import type {
  Camera,
  UpdateCameraRequest,
  AuthTestResult,
  RescanResult,
  SoftDeleteResult,
  ApiResponse,
} from "@/types/api"
import { cn } from "@/lib/utils"
import { API_BASE_URL } from "@/lib/config"
import {
  RotateCw,
  Eye,
  EyeOff,
  Trash2,
  RefreshCw,
  ChevronDown,
  ChevronRight,
  Check,
  X,
  Loader2,
  Camera as CameraIcon,
  Wifi,
  Video,
  Mic,
  Move3d,
  Info,
  Activity,
  Database,
  Network,
  Settings2,
} from "lucide-react"

interface CameraDetailModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  camera: Camera | null
  onSave: (camera: Camera) => void
  onDelete: (cameraId: string, hard: boolean) => void
}

type DeleteType = "soft" | "hard" | null

const ROTATION_OPTIONS = [0, 90, 180, 270] as const

export function CameraDetailModal({
  open,
  onOpenChange,
  camera,
  onSave,
  onDelete,
}: CameraDetailModalProps) {
  // Form state
  const [editedFields, setEditedFields] = useState<UpdateCameraRequest>({})
  const [showPassword, setShowPassword] = useState(false)

  // Loading states
  const [isSaving, setIsSaving] = useState(false)
  const [isAuthTesting, setIsAuthTesting] = useState(false)
  const [isRescanning, setIsRescanning] = useState(false)

  // Results
  const [authTestResult, setAuthTestResult] = useState<AuthTestResult | null>(null)
  const [rescanResult, setRescanResult] = useState<RescanResult | null>(null)

  // Delete confirmation
  const [deleteType, setDeleteType] = useState<DeleteType>(null)
  const [hardDeleteInput, setHardDeleteInput] = useState("")
  const [showDeleteDropdown, setShowDeleteDropdown] = useState(false)

  // Image loading state
  const [imageLoaded, setImageLoaded] = useState(false)

  // Initialize form when camera changes
  useEffect(() => {
    if (camera) {
      setEditedFields({})
      setAuthTestResult(null)
      setRescanResult(null)
      setShowPassword(false)
      setHardDeleteInput("")
      setImageLoaded(false)
    }
  }, [camera])

  // Get current value (edited or original)
  const getValue = useCallback(
    <K extends keyof Camera>(key: K): Camera[K] => {
      if (editedFields[key as keyof UpdateCameraRequest] !== undefined) {
        return editedFields[key as keyof UpdateCameraRequest] as Camera[K]
      }
      return camera?.[key] as Camera[K]
    },
    [camera, editedFields]
  )

  // Update field
  const updateField = <K extends keyof UpdateCameraRequest>(
    key: K,
    value: UpdateCameraRequest[K]
  ) => {
    setEditedFields((prev) => ({ ...prev, [key]: value }))
  }

  // Handle save
  const handleSave = async () => {
    if (!camera) return

    setIsSaving(true)
    try {
      const response = await fetch(`${API_BASE_URL}/api/cameras/${camera.camera_id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(editedFields),
      })
      const result: ApiResponse<Camera> = await response.json()
      if (result.ok) {
        onSave(result.data)
        onOpenChange(false)
      } else {
        alert(`保存に失敗しました: ${result.error}`)
      }
    } catch (error) {
      console.error("Save failed:", error)
      alert("保存に失敗しました")
    } finally {
      setIsSaving(false)
    }
  }

  // Handle auth test
  const handleAuthTest = async () => {
    if (!camera) return

    setIsAuthTesting(true)
    setAuthTestResult(null)
    try {
      const response = await fetch(`${API_BASE_URL}/api/cameras/${camera.camera_id}/auth-test`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          username: getValue("rtsp_username") ?? "",
          password: getValue("rtsp_password") ?? "",
        }),
      })
      const result: ApiResponse<AuthTestResult> = await response.json()
      if (result.ok) {
        setAuthTestResult(result.data)
      } else {
        setAuthTestResult({
          rtsp_success: false,
          onvif_success: false,
          model: null,
          message: result.error ?? "認証テストに失敗しました",
        })
      }
    } catch (error) {
      console.error("Auth test failed:", error)
      setAuthTestResult({
        rtsp_success: false,
        onvif_success: false,
        model: null,
        message: "認証テストに失敗しました",
      })
    } finally {
      setIsAuthTesting(false)
    }
  }

  // Handle rescan
  const handleRescan = async () => {
    if (!camera) return

    setIsRescanning(true)
    setRescanResult(null)
    try {
      const response = await fetch(`${API_BASE_URL}/api/cameras/${camera.camera_id}/rescan`, {
        method: "POST",
      })
      const result: ApiResponse<RescanResult> = await response.json()
      if (result.ok) {
        setRescanResult(result.data)
      } else {
        alert(`再スキャンに失敗しました: ${result.error}`)
      }
    } catch (error) {
      console.error("Rescan failed:", error)
      alert("再スキャンに失敗しました")
    } finally {
      setIsRescanning(false)
    }
  }

  // Handle soft delete
  const handleSoftDelete = async () => {
    if (!camera) return

    try {
      const response = await fetch(`${API_BASE_URL}/api/cameras/${camera.camera_id}/soft-delete`, {
        method: "POST",
      })
      const result: ApiResponse<SoftDeleteResult> = await response.json()
      if (result.ok) {
        onDelete(camera.camera_id, false)
        onOpenChange(false)
      } else {
        alert(`削除に失敗しました: ${result.error}`)
      }
    } catch (error) {
      console.error("Soft delete failed:", error)
      alert("削除に失敗しました")
    }
    setDeleteType(null)
  }

  // Handle hard delete
  const handleHardDelete = async () => {
    if (!camera) return

    try {
      const response = await fetch(`${API_BASE_URL}/api/cameras/${camera.camera_id}`, {
        method: "DELETE",
      })
      const result: ApiResponse<null> = await response.json()
      if (result.ok) {
        onDelete(camera.camera_id, true)
        onOpenChange(false)
      } else {
        alert(`削除に失敗しました: ${result.error}`)
      }
    } catch (error) {
      console.error("Hard delete failed:", error)
      alert("削除に失敗しました")
    }
    setDeleteType(null)
    setHardDeleteInput("")
  }

  if (!camera) return null

  const currentRotation = getValue("rotation") ?? 0
  const currentFitMode = getValue("fit_mode") ?? "trim"

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <CameraIcon className="h-5 w-5" />
              カメラ設定
            </DialogTitle>
          </DialogHeader>

          {/* Snapshot Preview with Rotation */}
          <div className="relative">
            <div className="aspect-video bg-muted rounded-lg overflow-hidden flex items-center justify-center">
              <img
                src={`${API_BASE_URL}/api/snapshots/${camera.camera_id}/latest.jpg?t=${Date.now()}`}
                alt={camera.name}
                className={cn(
                  "max-h-full max-w-full transition-transform duration-300",
                  currentFitMode === 'trim' ? "h-full w-full object-cover" : "object-contain"
                )}
                style={{
                  transform: `rotate(${currentRotation}deg)`,
                }}
                onLoad={() => setImageLoaded(true)}
                onError={(e) => {
                  e.currentTarget.style.display = 'none'
                  setImageLoaded(false)
                }}
              />
              {/* Fallback icon - only shown when image fails to load */}
              {!imageLoaded && (
                <CameraIcon className="absolute h-16 w-16 text-muted-foreground pointer-events-none" />
              )}
            </div>
            <div className="mt-2 flex items-center justify-between gap-4">
              <div className="flex items-center gap-2">
                <span className="text-sm text-muted-foreground">回転:</span>
                {ROTATION_OPTIONS.map((deg) => (
                  <Button
                    key={deg}
                    variant={currentRotation === deg ? "default" : "outline"}
                    size="sm"
                    onClick={() => updateField("rotation", deg)}
                  >
                    {deg}°
                  </Button>
                ))}
              </div>
              <div className="flex items-center gap-2">
                <span className="text-sm text-muted-foreground">表示:</span>
                <Button
                  variant={currentFitMode === 'trim' ? "default" : "outline"}
                  size="sm"
                  onClick={() => updateField("fit_mode", "trim")}
                  title="カードを画像で埋める（見切れあり）"
                >
                  Trim
                </Button>
                <Button
                  variant={currentFitMode === 'fit' ? "default" : "outline"}
                  size="sm"
                  onClick={() => updateField("fit_mode", "fit")}
                  title="画像全体を収める"
                >
                  Fit
                </Button>
              </div>
            </div>
          </div>

          {/* Basic Info Section */}
          <Section title="基本情報" icon={<Info className="h-4 w-4" />}>
            <FormField label="表示名" editable>
              <input
                type="text"
                className="input-field"
                value={getValue("name") ?? ""}
                onChange={(e) => updateField("name", e.target.value)}
              />
            </FormField>
            <FormField label="部屋ID" editable>
              <input
                type="text"
                className="input-field"
                value={getValue("rid") ?? ""}
                onChange={(e) => updateField("rid", e.target.value)}
                placeholder="例: LOBBY-01"
              />
            </FormField>
            <FormField label="IP">
              <span className="text-sm">{camera.ip_address ?? "-"}</span>
            </FormField>
            <FormField label="MAC">
              <span className="text-sm font-mono">{camera.mac_address ?? "-"}</span>
            </FormField>
          </Section>

          {/* Identifier Section */}
          <Section title="識別情報" icon={<Badge className="h-4 w-4">ID</Badge>}>
            <FormField label="lacisID">
              <span className="text-sm font-mono">{camera.lacis_id ?? "(自動取得)"}</span>
            </FormField>
            <FormField label="CIC">
              <span className="text-sm font-mono">{camera.cic ?? "(自動取得)"}</span>
            </FormField>
            <FormField label="FID">
              <span className="text-sm">{camera.fid ?? "(継承)"}</span>
            </FormField>
            <FormField label="TID">
              <span className="text-sm font-mono text-xs">{camera.tid ?? "(継承)"}</span>
            </FormField>
          </Section>

          {/* Credential Section */}
          <Section title="クレデンシャル" icon={<Wifi className="h-4 w-4" />}>
            <FormField label="ユーザー名" editable>
              <input
                type="text"
                className="input-field"
                value={getValue("rtsp_username") ?? ""}
                onChange={(e) => updateField("rtsp_username", e.target.value)}
                placeholder="admin"
              />
            </FormField>
            <FormField label="パスワード" editable>
              <div className="flex gap-2">
                <input
                  type={showPassword ? "text" : "password"}
                  className="input-field flex-1"
                  value={getValue("rtsp_password") ?? ""}
                  onChange={(e) => updateField("rtsp_password", e.target.value)}
                />
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => setShowPassword(!showPassword)}
                >
                  {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </Button>
              </div>
            </FormField>
            <div className="flex items-center gap-2 mt-2">
              <Button
                variant="outline"
                size="sm"
                onClick={handleAuthTest}
                disabled={isAuthTesting}
              >
                {isAuthTesting ? (
                  <Loader2 className="h-4 w-4 animate-spin mr-1" />
                ) : (
                  <RefreshCw className="h-4 w-4 mr-1" />
                )}
                認証テスト
              </Button>
              {authTestResult && (
                <div className="flex items-center gap-2 text-sm">
                  {authTestResult.rtsp_success || authTestResult.onvif_success ? (
                    <span className="text-green-600 flex items-center gap-1">
                      <Check className="h-4 w-4" />
                      {authTestResult.message}
                    </span>
                  ) : (
                    <span className="text-red-600 flex items-center gap-1">
                      <X className="h-4 w-4" />
                      {authTestResult.message}
                    </span>
                  )}
                </div>
              )}
            </div>
          </Section>

          {/* AI Analysis Settings Section */}
          <Section title="AI分析設定" icon={<Activity className="h-4 w-4" />}>
            <FormField label="AI分析" editable>
              <div className="flex items-center gap-2">
                <Button
                  variant={getValue("ai_enabled") ? "default" : "outline"}
                  size="sm"
                  onClick={() => updateField("ai_enabled", true)}
                >
                  有効
                </Button>
                <Button
                  variant={!getValue("ai_enabled") ? "default" : "outline"}
                  size="sm"
                  onClick={() => updateField("ai_enabled", false)}
                >
                  無効
                </Button>
              </div>
            </FormField>
            {/* プリセット選択グラフィカルUI */}
            <div className="mt-2">
              <PresetSelector
                cameraId={camera.camera_id}
                currentPresetId={getValue("preset_id") ?? "balanced"}
                onPresetChange={(presetId) => updateField("preset_id", presetId)}
                apiBaseUrl={API_BASE_URL}
              />
            </div>
            <FormField label="分析間隔" editable>
              <div className="flex items-center gap-2">
                <input
                  type="number"
                  min="5"
                  max="300"
                  className="w-20 p-2 border rounded-md text-sm"
                  value={getValue("ai_interval_sec") ?? 15}
                  onChange={(e) => updateField("ai_interval_sec", parseInt(e.target.value) || 15)}
                />
                <span className="text-sm text-muted-foreground">秒</span>
              </div>
            </FormField>

            {/* Threshold Overrides Section */}
            <div className="mt-4 pt-4 border-t">
              <h4 className="text-sm font-medium mb-3 flex items-center gap-2">
                <Settings2 className="h-4 w-4" />
                閾値カスタマイズ（上級者向け）
              </h4>
              <p className="text-xs text-muted-foreground mb-3">
                プリセットのデフォルト値を上書きします。チェックを外すとプリセットのデフォルト値が使用されます。
              </p>

              {/* Confidence Threshold Override */}
              <ThresholdSlider
                label="検知信頼度（conf）"
                description="物体検出の信頼度しきい値。低いと誤検知が増え、高いと検出漏れが増えます"
                value={getValue("conf_override") ?? null}
                onChange={(v) => updateField("conf_override", v)}
                min={0.20}
                max={0.80}
                step={0.05}
                defaultHint="0.40（バランス）"
              />

              {/* NMS Threshold Override */}
              <ThresholdSlider
                label="重複除去（NMS）"
                description="Non-Maximum Suppression。同一物体の重複検出を除去する感度"
                value={getValue("nms_threshold") ?? null}
                onChange={(v) => updateField("nms_threshold", v)}
                min={0.30}
                max={0.60}
                step={0.05}
                defaultHint="0.45（バランス）"
              />

              {/* PAR Threshold Override */}
              <ThresholdSlider
                label="人物属性（PAR）"
                description="人物属性認識（服装・性別・年齢等）の信頼度しきい値"
                value={getValue("par_threshold") ?? null}
                onChange={(v) => updateField("par_threshold", v)}
                min={0.30}
                max={0.80}
                step={0.05}
                defaultHint="0.50（バランス）"
              />
            </div>
          </Section>

          {/* Camera Context Section */}
          <Section title="カメラコンテキスト" icon={<Info className="h-4 w-4" />}>
            {(() => {
              const contextValue = getValue("camera_context") ?? ""
              const contextString = typeof contextValue === 'string'
                ? contextValue
                : (typeof contextValue === 'object' && contextValue !== null
                    ? (contextValue as { description?: string }).description ?? JSON.stringify(contextValue)
                    : String(contextValue))
              const maxLength = 148
              const currentLength = contextString.length
              const remaining = maxLength - currentLength
              return (
                <>
                  <textarea
                    className="w-full min-h-[60px] p-2 border rounded-md text-sm resize-none"
                    value={contextString}
                    onChange={(e) => {
                      const newValue = e.target.value.slice(0, maxLength)
                      updateField("camera_context", newValue)
                    }}
                    maxLength={maxLength}
                    placeholder="例: 玄関入口を監視。来客・配達員の検知を重視。夜間は照明が暗め。"
                  />
                  <div className="flex justify-between items-center mt-1">
                    <p className="text-xs text-muted-foreground">
                      AI判定で使用するカメラの説明・特徴を入力
                    </p>
                    <span className={cn(
                      "text-xs font-mono",
                      remaining <= 20 ? "text-amber-600" : "text-muted-foreground",
                      remaining <= 0 && "text-red-600"
                    )}>
                      残り {remaining} 文字
                    </span>
                  </div>
                </>
              )
            })()}
          </Section>

          {/* Device Info Section */}
          <Section title="デバイス情報" icon={<CameraIcon className="h-4 w-4" />}>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <FormField label="メーカー">
                <span>{camera.manufacturer ?? "-"}</span>
              </FormField>
              <FormField label="モデル">
                <span>{camera.model ?? "-"}</span>
              </FormField>
              <FormField label="ファミリー">
                <Badge variant="outline">{camera.family}</Badge>
              </FormField>
              <FormField label="登録日">
                <span>{new Date(camera.created_at).toLocaleString("ja-JP")}</span>
              </FormField>
            </div>
            <FormField label="RTSP Main">
              <span className="text-xs font-mono break-all">{camera.rtsp_main ?? "-"}</span>
            </FormField>
            <FormField label="RTSP Sub">
              <span className="text-xs font-mono break-all">{camera.rtsp_sub ?? "-"}</span>
            </FormField>
          </Section>

          {/* ONVIF Device Info (通常表示) */}
          {(camera.serial_number || camera.hardware_id || camera.firmware_version || camera.manufacturer || camera.model) && (
            <Section title="ONVIFデバイス情報" icon={<Info className="h-4 w-4" />}>
              <div className="grid grid-cols-2 gap-2 text-sm">
                <FormField label="メーカー">
                  <span>{camera.manufacturer ?? "-"}</span>
                </FormField>
                <FormField label="モデル">
                  <span>{camera.model ?? "-"}</span>
                </FormField>
                <FormField label="シリアル番号">
                  <span className="font-mono">{camera.serial_number ?? "-"}</span>
                </FormField>
                <FormField label="ハードウェアID">
                  <span className="font-mono">{camera.hardware_id ?? "-"}</span>
                </FormField>
                <FormField label="ファームウェア">
                  <span>{camera.firmware_version ?? "-"}</span>
                </FormField>
                <FormField label="ONVIFエンドポイント">
                  <span className="text-xs font-mono">{camera.onvif_endpoint ?? "-"}</span>
                </FormField>
              </div>
            </Section>
          )}

          {/* ONVIF Scopes (折りたたみ) */}
          {camera.onvif_scopes && (
            <CollapsibleSection title="ONVIFスコープ" icon={<Database className="h-4 w-4" />}>
              <div className="grid grid-cols-2 gap-2 text-sm">
                <FormField label="デバイス名">
                  <span>{camera.onvif_scopes.name ?? "-"}</span>
                </FormField>
                <FormField label="ハードウェア">
                  <span>{camera.onvif_scopes.hardware ?? "-"}</span>
                </FormField>
                <FormField label="プロファイル">
                  <span>{camera.onvif_scopes.profile ?? "-"}</span>
                </FormField>
                <FormField label="ロケーション">
                  <span>{camera.onvif_scopes.location ?? "-"}</span>
                </FormField>
                <FormField label="タイプ">
                  <span>{camera.onvif_scopes.type ?? "-"}</span>
                </FormField>
              </div>
              {camera.onvif_scopes.raw_scopes && camera.onvif_scopes.raw_scopes.length > 0 && (
                <div className="mt-2">
                  <span className="text-xs text-muted-foreground">Raw Scopes:</span>
                  <div className="mt-1 text-xs font-mono bg-muted/50 p-2 rounded max-h-20 overflow-y-auto">
                    {camera.onvif_scopes.raw_scopes.map((scope, i) => (
                      <div key={i} className="break-all">{scope}</div>
                    ))}
                  </div>
                </div>
              )}
            </CollapsibleSection>
          )}

          {/* ONVIF Network Interfaces (折りたたみ) */}
          {camera.onvif_network_interfaces && camera.onvif_network_interfaces.length > 0 && (
            <CollapsibleSection title="ネットワークインターフェース" icon={<Network className="h-4 w-4" />}>
              <div className="space-y-2">
                {camera.onvif_network_interfaces.map((iface, i) => (
                  <div key={i} className="bg-muted/50 p-2 rounded text-sm">
                    <div className="flex items-center gap-2">
                      <span className="font-medium">{iface.name}</span>
                      {iface.enabled !== undefined && (
                        <Badge variant={iface.enabled ? "default" : "secondary"} className="text-xs">
                          {iface.enabled ? "有効" : "無効"}
                        </Badge>
                      )}
                    </div>
                    <div className="grid grid-cols-2 gap-2 mt-1 text-xs">
                      <span>MAC: <span className="font-mono">{iface.mac_address}</span></span>
                      {iface.ipv4 && (
                        <span>IPv4: <span className="font-mono">{iface.ipv4.address}/{iface.ipv4.prefix_length}</span></span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </CollapsibleSection>
          )}

          {/* ONVIF Capabilities (折りたたみ) */}
          {camera.onvif_capabilities && (
            <CollapsibleSection title="ONVIF機能" icon={<Settings2 className="h-4 w-4" />}>
              <div className="flex flex-wrap gap-2">
                {camera.onvif_capabilities.analytics?.supported && (
                  <Badge variant="secondary">Analytics</Badge>
                )}
                {camera.onvif_capabilities.device && (
                  <Badge variant="secondary">Device</Badge>
                )}
                {camera.onvif_capabilities.events && (
                  <Badge variant="secondary">Events</Badge>
                )}
                {camera.onvif_capabilities.imaging && (
                  <Badge variant="secondary">Imaging</Badge>
                )}
                {camera.onvif_capabilities.media && (
                  <Badge variant="secondary">Media</Badge>
                )}
                {camera.onvif_capabilities.ptz && (
                  <Badge variant="secondary">PTZ</Badge>
                )}
              </div>
              {camera.onvif_capabilities.media && (
                <div className="mt-2 text-xs">
                  <span className="text-muted-foreground">メディア転送:</span>
                  <div className="flex gap-2 mt-1">
                    {camera.onvif_capabilities.media.rtp_tcp && <Badge variant="outline" className="text-xs">RTP/TCP</Badge>}
                    {camera.onvif_capabilities.media.rtp_rtsp_tcp && <Badge variant="outline" className="text-xs">RTSP/TCP</Badge>}
                    {camera.onvif_capabilities.media.rtp_multicast && <Badge variant="outline" className="text-xs">Multicast</Badge>}
                  </div>
                </div>
              )}
              {camera.onvif_capabilities.analytics && (
                <div className="mt-2 text-xs">
                  <span className="text-muted-foreground">分析機能:</span>
                  <div className="flex gap-2 mt-1">
                    {camera.onvif_capabilities.analytics.rule_support && <Badge variant="outline" className="text-xs">ルール</Badge>}
                    {camera.onvif_capabilities.analytics.analytics_module_support && <Badge variant="outline" className="text-xs">分析モジュール</Badge>}
                  </div>
                </div>
              )}
            </CollapsibleSection>
          )}

          {/* Video Capabilities */}
          {(camera.resolution_main || camera.resolution_sub) && (
            <Section title="ビデオ能力" icon={<Video className="h-4 w-4" />}>
              <div className="space-y-2">
                {camera.resolution_main && (
                  <div className="bg-muted/50 p-2 rounded">
                    <span className="text-xs font-medium">メイン</span>
                    <div className="grid grid-cols-4 gap-2 text-sm mt-1">
                      <span>{camera.resolution_main}</span>
                      <span>{camera.codec_main ?? "-"}</span>
                      <span>{camera.fps_main ? `${camera.fps_main}fps` : "-"}</span>
                      <span>{camera.bitrate_main ? `${camera.bitrate_main}kbps` : "-"}</span>
                    </div>
                  </div>
                )}
                {camera.resolution_sub && (
                  <div className="bg-muted/50 p-2 rounded">
                    <span className="text-xs font-medium">サブ</span>
                    <div className="grid grid-cols-4 gap-2 text-sm mt-1">
                      <span>{camera.resolution_sub}</span>
                      <span>{camera.codec_sub ?? "-"}</span>
                      <span>{camera.fps_sub ? `${camera.fps_sub}fps` : "-"}</span>
                      <span>{camera.bitrate_sub ? `${camera.bitrate_sub}kbps` : "-"}</span>
                    </div>
                  </div>
                )}
              </div>
            </Section>
          )}

          {/* PTZ Capabilities - PTZ非対応カメラでもグレーアウト表示 */}
          <Section
            title="PTZ能力"
            icon={<Move3d className={cn("h-4 w-4", !camera.ptz_supported && "text-muted-foreground")} />}
          >
            {!camera.ptz_supported ? (
              /* PTZ非対応カメラ: グレーアウト表示 */
              <div className="opacity-50">
                <div className="flex items-center gap-2 mb-2">
                  <input
                    type="checkbox"
                    id="ptz-enabled-disabled"
                    checked={false}
                    disabled
                    className="h-4 w-4 rounded border-gray-300 cursor-not-allowed"
                  />
                  <label htmlFor="ptz-enabled-disabled" className="text-sm text-muted-foreground">
                    PTZ操作UIを表示する
                  </label>
                </div>
                <p className="text-xs text-muted-foreground">
                  このカメラはPTZ（パン・チルト・ズーム）に対応していません
                </p>
              </div>
            ) : (
              /* PTZ対応カメラ: 通常表示 */
              <>
                {/* PTZ無効化チェックボックス */}
                <FormField label="PTZ操作" editable>
                  <div className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      id="ptz-enabled"
                      checked={!getValue("ptz_disabled")}
                      onChange={(e) => updateField("ptz_disabled", !e.target.checked)}
                      className="h-4 w-4 rounded border-gray-300"
                    />
                    <label htmlFor="ptz-enabled" className="text-sm">
                      PTZ操作UIを表示する
                    </label>
                  </div>
                </FormField>
                {/* ONVIFエンドポイント設定（PTZ操作に必須） */}
                <FormField label="ONVIF URL" editable>
                  <input
                    type="text"
                    className="input-field text-xs font-mono"
                    value={getValue("onvif_endpoint") ?? ""}
                    onChange={(e) => updateField("onvif_endpoint", e.target.value)}
                    placeholder="http://192.168.x.x:2020/onvif/device_service"
                  />
                </FormField>
                {!getValue("onvif_endpoint") && (
                  <p className="text-xs text-amber-600 mt-1">
                    PTZ操作にはONVIFエンドポイントの設定が必要です。Tapoは通常ポート2020です。
                  </p>
                )}
                <div className="flex flex-wrap gap-2 mt-2">
                  {camera.ptz_continuous && <Badge variant="secondary">連続移動</Badge>}
                  {camera.ptz_absolute && <Badge variant="secondary">絶対位置</Badge>}
                  {camera.ptz_relative && <Badge variant="secondary">相対移動</Badge>}
                  {camera.ptz_home_supported && <Badge variant="secondary">ホーム</Badge>}
                </div>
                {camera.ptz_presets && camera.ptz_presets.length > 0 && (
                  <div className="mt-2 text-sm">
                    <span className="text-muted-foreground">プリセット:</span>{" "}
                    {camera.ptz_presets.length}個
                  </div>
                )}
              </>
            )}
          </Section>

          {/* Audio Capabilities */}
          {(camera.audio_input_supported || camera.audio_output_supported) && (
            <Section title="音声能力" icon={<Mic className="h-4 w-4" />}>
              <div className="flex flex-wrap gap-2">
                {camera.audio_input_supported && <Badge variant="secondary">マイク</Badge>}
                {camera.audio_output_supported && <Badge variant="secondary">スピーカー</Badge>}
                {camera.audio_codec && (
                  <Badge variant="outline">{camera.audio_codec}</Badge>
                )}
              </div>
            </Section>
          )}

          {/* Detection Meta */}
          <Section title="検出メタ情報" icon={<Info className="h-4 w-4" />}>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <FormField label="検出方法">
                <Badge variant="outline">{camera.discovery_method ?? "-"}</Badge>
              </FormField>
              <FormField label="最終疎通確認">
                <span>
                  {camera.last_verified_at
                    ? new Date(camera.last_verified_at).toLocaleString("ja-JP")
                    : "-"}
                </span>
              </FormField>
            </div>
            {rescanResult && (
              <div className="mt-2 p-2 bg-blue-50 rounded text-sm">
                {rescanResult.found ? (
                  <span className="text-blue-700">
                    {rescanResult.updated
                      ? `IPが ${rescanResult.old_ip} から ${rescanResult.new_ip} に更新されました`
                      : "IPに変更はありませんでした"}
                  </span>
                ) : (
                  <span className="text-amber-700">カメラが見つかりませんでした</span>
                )}
              </div>
            )}
          </Section>

          <DialogFooter className="flex justify-between items-center gap-2 mt-4 flex-wrap">
            <div className="flex gap-2">
              {/* Rescan Button */}
              <Button
                variant="outline"
                onClick={handleRescan}
                disabled={isRescanning}
              >
                {isRescanning ? (
                  <Loader2 className="h-4 w-4 animate-spin mr-1" />
                ) : (
                  <RotateCw className="h-4 w-4 mr-1" />
                )}
                再スキャン
              </Button>

              {/* Delete Dropdown */}
              <div className="relative">
                <Button
                  variant="outline"
                  className="text-red-600 hover:text-red-700"
                  onClick={() => setShowDeleteDropdown(!showDeleteDropdown)}
                >
                  <Trash2 className="h-4 w-4 mr-1" />
                  削除
                  <ChevronDown className="h-4 w-4 ml-1" />
                </Button>
                {showDeleteDropdown && (
                  <div className="absolute bottom-full left-0 mb-1 w-40 bg-white border rounded-md shadow-lg z-50">
                    <button
                      className="w-full px-4 py-2 text-left text-sm hover:bg-muted"
                      onClick={() => {
                        setDeleteType("soft")
                        setShowDeleteDropdown(false)
                      }}
                    >
                      ソフト削除
                    </button>
                    <hr />
                    <button
                      className="w-full px-4 py-2 text-left text-sm text-red-600 hover:bg-red-50"
                      onClick={() => {
                        setDeleteType("hard")
                        setShowDeleteDropdown(false)
                      }}
                    >
                      完全削除
                    </button>
                  </div>
                )}
              </div>
            </div>

            <div className="flex gap-2">
              <Button variant="outline" onClick={() => onOpenChange(false)}>
                キャンセル
              </Button>
              <Button onClick={handleSave} disabled={isSaving}>
                {isSaving ? (
                  <Loader2 className="h-4 w-4 animate-spin mr-1" />
                ) : (
                  <Check className="h-4 w-4 mr-1" />
                )}
                保存
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Soft Delete Confirm Dialog */}
      <ConfirmDialog
        open={deleteType === "soft"}
        onOpenChange={(open) => !open && setDeleteType(null)}
        title="カメラを削除しますか？"
        description={`「${camera.name}」をソフト削除します。\n\nMACアドレス (${camera.mac_address ?? "-"}) を保持するため、後から復元できます。`}
        confirmLabel="削除する"
        type="warning"
        onConfirm={handleSoftDelete}
      />

      {/* Hard Delete Confirm Dialog */}
      <Dialog
        open={deleteType === "hard"}
        onOpenChange={(open) => {
          if (!open) {
            setDeleteType(null)
            setHardDeleteInput("")
          }
        }}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle className="text-red-600 flex items-center gap-2">
              <Trash2 className="h-5 w-5" />
              完全削除の確認
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <p className="text-sm">
              「{camera.name}」を完全に削除します。
            </p>
            <p className="text-sm text-red-600 font-medium">
              この操作は取り消せません。
              関連するイベント・フレームデータもすべて削除されます。
            </p>
            <div>
              <label className="text-sm text-muted-foreground">
                削除するには「完全削除」と入力:
              </label>
              <input
                type="text"
                className="input-field mt-1"
                value={hardDeleteInput}
                onChange={(e) => setHardDeleteInput(e.target.value)}
                placeholder="完全削除"
              />
            </div>
          </div>
          <DialogFooter className="mt-4">
            <Button variant="outline" onClick={() => setDeleteType(null)}>
              キャンセル
            </Button>
            <Button
              variant="destructive"
              onClick={handleHardDelete}
              disabled={hardDeleteInput !== "完全削除"}
            >
              完全削除
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}

// Section component
function Section({
  title,
  icon,
  children,
}: {
  title: string
  icon?: React.ReactNode
  children: React.ReactNode
}) {
  return (
    <div className="border-t pt-4 mt-4">
      <h3 className="flex items-center gap-2 text-sm font-medium mb-3">
        {icon}
        {title}
      </h3>
      <div className="space-y-2">{children}</div>
    </div>
  )
}

// Collapsible section component (default collapsed)
function CollapsibleSection({
  title,
  icon,
  children,
  defaultOpen = false,
}: {
  title: string
  icon?: React.ReactNode
  children: React.ReactNode
  defaultOpen?: boolean
}) {
  const [isOpen, setIsOpen] = useState(defaultOpen)

  return (
    <div className="border-t pt-4 mt-4">
      <button
        type="button"
        className="flex items-center gap-2 text-sm font-medium mb-3 w-full text-left hover:text-primary transition-colors"
        onClick={() => setIsOpen(!isOpen)}
      >
        {isOpen ? (
          <ChevronDown className="h-4 w-4" />
        ) : (
          <ChevronRight className="h-4 w-4" />
        )}
        {icon}
        {title}
      </button>
      {isOpen && <div className="space-y-2 pl-6">{children}</div>}
    </div>
  )
}

// Form field component
function FormField({
  label,
  children,
  editable,
}: {
  label: string
  children: React.ReactNode
  editable?: boolean
}) {
  return (
    <div className="flex items-center gap-2">
      <span
        className={cn(
          "text-sm w-24 shrink-0",
          editable ? "text-foreground" : "text-muted-foreground"
        )}
      >
        {label}:
      </span>
      <div className="flex-1">{children}</div>
    </div>
  )
}

// Threshold slider component for AI settings
function ThresholdSlider({
  label,
  description,
  value,
  onChange,
  min,
  max,
  step,
  defaultHint,
}: {
  label: string
  description: string
  value: number | null
  onChange: (value: number | null) => void
  min: number
  max: number
  step: number
  defaultHint: string
}) {
  const isEnabled = value !== null
  const displayValue = value ?? ((min + max) / 2)

  return (
    <div className="mb-4">
      <div className="flex items-center gap-2 mb-1">
        <input
          type="checkbox"
          id={`threshold-${label}`}
          checked={isEnabled}
          onChange={(e) => {
            if (e.target.checked) {
              onChange((min + max) / 2)
            } else {
              onChange(null)
            }
          }}
          className="h-4 w-4"
        />
        <label htmlFor={`threshold-${label}`} className="text-sm font-medium">
          {label}
        </label>
        {isEnabled && (
          <span className="text-sm font-mono bg-muted px-2 py-0.5 rounded">
            {displayValue.toFixed(2)}
          </span>
        )}
        {!isEnabled && (
          <span className="text-xs text-muted-foreground">
            デフォルト: {defaultHint}
          </span>
        )}
      </div>
      <p className="text-xs text-muted-foreground ml-6 mb-2">{description}</p>
      {isEnabled && (
        <div className="ml-6 flex items-center gap-2">
          <span className="text-xs text-muted-foreground w-8">{min.toFixed(2)}</span>
          <input
            type="range"
            min={min}
            max={max}
            step={step}
            value={displayValue}
            onChange={(e) => onChange(parseFloat(e.target.value))}
            className="flex-1 h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
          />
          <span className="text-xs text-muted-foreground w-8">{max.toFixed(2)}</span>
        </div>
      )}
    </div>
  )
}
