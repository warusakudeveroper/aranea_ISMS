/**
 * DetectionDetailModal - Detection Event Detail View
 *
 * Displays:
 * - Detection image (if available)
 * - IS21 response details
 * - Detection metadata (timestamps, confidence, etc.)
 */

import { useMemo } from "react"
import type { DetectionLog } from "@/types/api"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import {
  Camera as CameraIcon,
  CameraOff,
  User,
  Users,
  Clock,
  AlertTriangle,
  Info,
  Code,
  Image as ImageIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { API_BASE_URL } from "@/lib/config"

interface DetectionDetailModalProps {
  log: DetectionLog | null
  isOpen: boolean
  onClose: () => void
  cameraName: string
}

// Get icon based on primary_event type
function getEventIcon(primaryEvent: string): React.ReactNode {
  const iconClass = "h-5 w-5"

  switch (primaryEvent.toLowerCase()) {
    case "camera_lost":
      return <CameraOff className={cn(iconClass, "text-red-500")} />
    case "camera_recovered":
      return <CameraIcon className={cn(iconClass, "text-green-500")} />
    case "human":
    case "person":
      return <User className={cn(iconClass, "text-blue-500")} />
    case "humans":
    case "people":
    case "crowd":
      return <Users className={cn(iconClass, "text-blue-500")} />
    default:
      return <AlertTriangle className={cn(iconClass, "text-amber-500")} />
  }
}

// Format datetime for display
function formatDateTime(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleString("ja-JP", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  })
}

// Get severity badge variant
function getSeverityBadgeVariant(severity: number): "severity1" | "severity2" | "severity3" {
  if (severity === 1) return "severity1"
  if (severity === 2) return "severity2"
  return "severity3"
}

// Get severity label
function getSeverityLabel(severity: number): string {
  switch (severity) {
    case 1: return "Low"
    case 2: return "Medium"
    case 3: return "High"
    case 4: return "Critical"
    default: return `Level ${severity}`
  }
}

export function DetectionDetailModal({
  log,
  isOpen,
  onClose,
  cameraName,
}: DetectionDetailModalProps) {
  // Check if this is a camera status event (no image)
  const isCameraStatusEvent = log?.primary_event === "camera_lost" || log?.primary_event === "camera_recovered"

  // Build image URL - the image_path_local is stored as local filesystem path
  // We need to construct API URL to serve it
  const imageUrl = useMemo(() => {
    if (!log?.image_path_local || isCameraStatusEvent || log.image_path_local === '') return null
    // Image path format: /var/lib/is22/events/{camera_id}/{timestamp}.jpg
    // We need to serve this via API
    // For now, construct a relative URL that the backend can serve
    const pathParts = log.image_path_local.split('/')
    const filename = pathParts[pathParts.length - 1]
    const cameraDir = pathParts[pathParts.length - 2]
    return `${API_BASE_URL}/api/events/images/${cameraDir}/${filename}`
  }, [log?.image_path_local, isCameraStatusEvent])

  if (!log) return null

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-w-2xl max-h-[85vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {getEventIcon(log.primary_event)}
            <span>{cameraName}</span>
            <Badge variant={getSeverityBadgeVariant(log.severity)} className="ml-2">
              {getSeverityLabel(log.severity)}
            </Badge>
          </DialogTitle>
          <DialogDescription className="flex items-center gap-4 text-xs">
            <span className="flex items-center gap-1">
              <Clock className="h-3 w-3" />
              {formatDateTime(log.captured_at)}
            </span>
            <span className="text-muted-foreground">
              {log.primary_event}
              {log.count_hint > 0 && ` (${log.count_hint})`}
            </span>
          </DialogDescription>
        </DialogHeader>

        <Tabs defaultValue={isCameraStatusEvent ? "details" : "image"} className="flex-1 flex flex-col min-h-0">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="image" disabled={isCameraStatusEvent}>
              <ImageIcon className="h-3 w-3 mr-1" />
              画像
            </TabsTrigger>
            <TabsTrigger value="details">
              <Info className="h-3 w-3 mr-1" />
              詳細
            </TabsTrigger>
            <TabsTrigger value="raw">
              <Code className="h-3 w-3 mr-1" />
              Raw
            </TabsTrigger>
          </TabsList>

          {/* Image Tab */}
          <TabsContent value="image" className="flex-1 min-h-0 mt-2">
            {isCameraStatusEvent ? (
              <div className="flex items-center justify-center h-full text-muted-foreground">
                <div className="text-center">
                  <CameraOff className="h-12 w-12 mx-auto mb-2 opacity-30" />
                  <p className="text-sm">カメラステータスイベントには画像がありません</p>
                </div>
              </div>
            ) : imageUrl ? (
              <div className="relative w-full h-full min-h-[300px] bg-black rounded overflow-hidden">
                <img
                  src={imageUrl}
                  alt="Detection"
                  className="w-full h-full object-contain"
                  onError={(e) => {
                    const target = e.target as HTMLImageElement
                    target.style.display = 'none'
                    target.parentElement!.innerHTML = `
                      <div class="flex items-center justify-center h-full text-gray-400">
                        <div class="text-center">
                          <svg class="h-12 w-12 mx-auto mb-2 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"></path>
                          </svg>
                          <p class="text-sm">画像を読み込めません</p>
                        </div>
                      </div>
                    `
                  }}
                />
                {/* Bounding boxes overlay */}
                {log.bboxes && log.bboxes.length > 0 && (
                  <div className="absolute inset-0 pointer-events-none">
                    {log.bboxes.map((bbox, i) => (
                      <div
                        key={i}
                        className="absolute border-2 border-green-500"
                        style={{
                          left: `${bbox.x * 100}%`,
                          top: `${bbox.y * 100}%`,
                          width: `${bbox.w * 100}%`,
                          height: `${bbox.h * 100}%`,
                        }}
                      >
                        <span className="absolute -top-5 left-0 bg-green-500 text-white text-[10px] px-1 rounded">
                          {bbox.label} {(bbox.confidence * 100).toFixed(0)}%
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <div className="flex items-center justify-center h-full text-muted-foreground min-h-[300px]">
                <div className="text-center">
                  <ImageIcon className="h-12 w-12 mx-auto mb-2 opacity-30" />
                  <p className="text-sm">画像がありません</p>
                </div>
              </div>
            )}
          </TabsContent>

          {/* Details Tab */}
          <TabsContent value="details" className="flex-1 min-h-0 mt-2">
            <ScrollArea className="h-[350px]">
              <div className="space-y-4 pr-4">
                {/* Event Info */}
                <section>
                  <h4 className="text-sm font-medium mb-2">イベント情報</h4>
                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div className="bg-muted p-2 rounded">
                      <span className="text-muted-foreground">イベント:</span>
                      <span className="ml-2 font-medium">{log.primary_event}</span>
                    </div>
                    <div className="bg-muted p-2 rounded">
                      <span className="text-muted-foreground">重要度:</span>
                      <span className="ml-2 font-medium">{getSeverityLabel(log.severity)}</span>
                    </div>
                    <div className="bg-muted p-2 rounded">
                      <span className="text-muted-foreground">信頼度:</span>
                      <span className="ml-2 font-medium">{(log.confidence * 100).toFixed(1)}%</span>
                    </div>
                    <div className="bg-muted p-2 rounded">
                      <span className="text-muted-foreground">検出数:</span>
                      <span className="ml-2 font-medium">{log.count_hint}</span>
                    </div>
                  </div>
                </section>

                {/* Tags */}
                {log.tags && log.tags.length > 0 && (
                  <section>
                    <h4 className="text-sm font-medium mb-2">タグ</h4>
                    <div className="flex flex-wrap gap-1">
                      {log.tags.map((tag, i) => (
                        <Badge key={i} variant="outline" className="text-xs">
                          {tag}
                        </Badge>
                      ))}
                    </div>
                  </section>
                )}

                {/* Person Details */}
                {log.person_details && log.person_details.length > 0 && (
                  <section>
                    <h4 className="text-sm font-medium mb-2">人物属性</h4>
                    <div className="space-y-2">
                      {log.person_details.map((person, i) => (
                        <div key={i} className="bg-muted p-2 rounded text-xs">
                          <div className="flex gap-4">
                            {person.gender && (
                              <span><span className="text-muted-foreground">性別:</span> {person.gender}</span>
                            )}
                            {person.age_group && (
                              <span><span className="text-muted-foreground">年齢:</span> {person.age_group}</span>
                            )}
                            {person.clothing_color && (
                              <span><span className="text-muted-foreground">服装:</span> {person.clothing_color}</span>
                            )}
                            {person.posture && (
                              <span><span className="text-muted-foreground">姿勢:</span> {person.posture}</span>
                            )}
                          </div>
                        </div>
                      ))}
                    </div>
                  </section>
                )}

                {/* Suspicious Info */}
                {log.suspicious && (
                  <section>
                    <h4 className="text-sm font-medium mb-2 text-red-600">不審行動</h4>
                    <div className="bg-red-50 p-2 rounded text-xs">
                      <div className="font-medium">スコア: {log.suspicious.score}</div>
                      {log.suspicious.reasons && log.suspicious.reasons.length > 0 && (
                        <ul className="mt-1 list-disc list-inside text-muted-foreground">
                          {log.suspicious.reasons.map((reason, i) => (
                            <li key={i}>{reason}</li>
                          ))}
                        </ul>
                      )}
                    </div>
                  </section>
                )}

                {/* Frame Diff */}
                {log.frame_diff && (
                  <section>
                    <h4 className="text-sm font-medium mb-2">フレーム差分</h4>
                    <div className="grid grid-cols-3 gap-2 text-xs">
                      <div className="bg-muted p-2 rounded">
                        <span className="text-muted-foreground">変化:</span>
                        <span className="ml-2 font-medium">{log.frame_diff.has_change ? 'あり' : 'なし'}</span>
                      </div>
                      <div className="bg-muted p-2 rounded">
                        <span className="text-muted-foreground">変化率:</span>
                        <span className="ml-2 font-medium">{log.frame_diff.change_percent.toFixed(1)}%</span>
                      </div>
                      <div className="bg-muted p-2 rounded">
                        <span className="text-muted-foreground">動体領域:</span>
                        <span className="ml-2 font-medium">{log.frame_diff.motion_regions}</span>
                      </div>
                    </div>
                  </section>
                )}

                {/* Processing Info */}
                <section>
                  <h4 className="text-sm font-medium mb-2">処理情報</h4>
                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div className="bg-muted p-2 rounded">
                      <span className="text-muted-foreground">処理時間:</span>
                      <span className="ml-2 font-medium">{log.processing_ms ?? '-'}ms</span>
                    </div>
                    <div className="bg-muted p-2 rounded">
                      <span className="text-muted-foreground">プリセット:</span>
                      <span className="ml-2 font-medium">{log.preset_id ?? '-'}</span>
                    </div>
                    <div className="bg-muted p-2 rounded">
                      <span className="text-muted-foreground">Log ID:</span>
                      <span className="ml-2 font-medium">{log.log_id}</span>
                    </div>
                    <div className="bg-muted p-2 rounded">
                      <span className="text-muted-foreground">BQ同期:</span>
                      <span className="ml-2 font-medium">{log.synced_to_bq ? '済' : '未'}</span>
                    </div>
                  </div>
                </section>
              </div>
            </ScrollArea>
          </TabsContent>

          {/* Raw Tab */}
          <TabsContent value="raw" className="flex-1 min-h-0 mt-2">
            <ScrollArea className="h-[350px]">
              <pre className="text-[10px] bg-muted p-2 rounded overflow-x-auto whitespace-pre-wrap">
                {JSON.stringify(log.is21_log, null, 2)}
              </pre>
            </ScrollArea>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}
