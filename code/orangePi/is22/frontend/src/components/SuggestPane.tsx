import type { Event } from "@/types/api"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Video, AlertCircle } from "lucide-react"
import { API_BASE_URL } from "@/lib/config"

interface SuggestPaneProps {
  currentEvent: Event | null
  cameraName?: string
}

function getSeverityVariant(severity: number): "severity0" | "severity1" | "severity2" | "severity3" {
  if (severity === 0) return "severity0"
  if (severity === 1) return "severity1"
  if (severity === 2) return "severity2"
  return "severity3"
}

function getSeverityLabel(severity: number, primaryEvent: string): string {
  // Use primary_event for label if available
  if (primaryEvent && primaryEvent !== "none" && primaryEvent !== "unknown") {
    return primaryEvent.charAt(0).toUpperCase() + primaryEvent.slice(1)
  }
  if (severity === 0) return "Normal"
  if (severity === 1) return "Motion"
  if (severity === 2) return "Vehicle"
  return "Alert"
}

export function SuggestPane({ currentEvent, cameraName }: SuggestPaneProps) {
  if (!currentEvent) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-muted-foreground p-4">
        <Video className="h-16 w-16 mb-4 opacity-30" />
        <p className="text-sm text-center">No active events</p>
        <p className="text-xs text-center mt-1">Events will appear here when detected</p>
      </div>
    )
  }

  const severityVariant = getSeverityVariant(currentEvent.severity)

  return (
    <div className="h-full flex flex-col p-2">
      <Card className="flex-1 flex flex-col overflow-hidden">
        <CardHeader className="pb-2 flex-shrink-0">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm flex items-center gap-2">
              <AlertCircle className="h-4 w-4" />
              Suggest View
            </CardTitle>
            <Badge variant={severityVariant}>
              {getSeverityLabel(currentEvent.severity, currentEvent.primary_event)}
            </Badge>
          </div>
        </CardHeader>
        <CardContent className="flex-1 flex flex-col gap-2 overflow-hidden">
          {/* Camera snapshot/video area */}
          <div className="flex-1 bg-muted rounded overflow-hidden relative min-h-0">
            <img
              src={`${API_BASE_URL}/api/streams/${currentEvent.camera_id}/snapshot`}
              alt={cameraName || currentEvent.camera_id}
              className="h-full w-full object-contain"
            />
            {/* Event overlay */}
            <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/70 to-transparent p-3">
              <p className="text-white text-sm font-medium truncate">
                {cameraName || currentEvent.camera_id}
              </p>
              <p className="text-white/80 text-xs">
                {currentEvent.primary_event}
              </p>
            </div>
          </div>

          {/* Event info */}
          <div className="flex-shrink-0 text-xs space-y-1">
            <div className="flex justify-between">
              <span className="text-muted-foreground">Captured:</span>
              <span>{new Date(currentEvent.captured_at).toLocaleTimeString()}</span>
            </div>
            {currentEvent.tags.length > 0 && (
              <div className="flex flex-wrap gap-1">
                {currentEvent.tags.slice(0, 3).map((tag, i) => (
                  <Badge key={i} variant="outline" className="text-xs py-0">
                    {tag}
                  </Badge>
                ))}
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
