/**
 * LiveViewModal - Camera live streaming modal
 *
 * Opens when clicking a camera tile (not the settings icon).
 * Displays live video stream using Go2rtcPlayer with tiered fallback.
 *
 * Fallback order:
 * 1. Main stream (10 second timeout)
 * 2. Sub stream with warning (20 second timeout)
 * 3. Snapshot with error display
 */

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import type { Camera } from "@/types/api"
import { Go2rtcPlayer } from "./Go2rtcPlayer"
import {
  Video,
  Settings,
  AlertTriangle,
} from "lucide-react"
import { useState, useCallback } from "react"

interface LiveViewModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  camera: Camera | null
  onSettingsClick?: () => void
}

type StreamPhase = "main" | "sub" | "failed"

export function LiveViewModal({
  open,
  onOpenChange,
  camera,
  onSettingsClick,
}: LiveViewModalProps) {
  const [isPlaying, setIsPlaying] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [streamPhase, setStreamPhase] = useState<StreamPhase>("main")

  // Reset state when modal opens/closes
  const handleOpenChange = useCallback((newOpen: boolean) => {
    if (!newOpen) {
      // Reset state on close
      setIsPlaying(false)
      setError(null)
      setStreamPhase("main")
    }
    onOpenChange(newOpen)
  }, [onOpenChange])

  if (!camera) return null

  // Build RTSP URLs for go2rtc
  const rtspMain = camera.rtsp_main || ""
  const rtspSub = camera.rtsp_sub || ""
  const hasAnyStream = rtspMain || rtspSub

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-4xl p-0 overflow-hidden">
        <DialogHeader className="p-4 pb-2 flex flex-row items-center justify-between">
          <DialogTitle className="flex items-center gap-2">
            <Video className="h-5 w-5" />
            {camera.name}
            {camera.rid && (
              <Badge variant="outline" className="ml-2">
                {camera.rid}
              </Badge>
            )}
          </DialogTitle>
          <div className="flex items-center gap-2">
            {onSettingsClick && (
              <Button
                variant="ghost"
                size="icon"
                onClick={() => {
                  handleOpenChange(false)
                  onSettingsClick()
                }}
                title="カメラ設定"
              >
                <Settings className="h-4 w-4" />
              </Button>
            )}
          </div>
        </DialogHeader>

        {/* Video Player */}
        <div className="relative aspect-video bg-black">
          {hasAnyStream ? (
            <Go2rtcPlayer
              cameraId={camera.camera_id}
              rtspUrl={rtspMain || rtspSub}
              rtspUrlSub={rtspMain ? rtspSub : undefined}
              autoPlay={true}
              muted={true}
              className="w-full h-full"
              onPlaying={() => {
                setIsPlaying(true)
                setError(null)
              }}
              onStopped={() => setIsPlaying(false)}
              onError={(err) => setError(err)}
              onPhaseChange={(phase) => setStreamPhase(phase)}
            />
          ) : (
            <div className="absolute inset-0 flex flex-col items-center justify-center text-white/70">
              <Video className="h-16 w-16 mb-4" />
              <p>RTSPストリームが設定されていません</p>
              <Button
                variant="outline"
                size="sm"
                className="mt-4"
                onClick={() => {
                  handleOpenChange(false)
                  onSettingsClick?.()
                }}
              >
                <Settings className="h-4 w-4 mr-2" />
                カメラ設定を開く
              </Button>
            </div>
          )}
        </div>

        {/* Footer with camera info */}
        <div className="p-3 bg-muted/50 flex items-center justify-between text-sm">
          <div className="flex items-center gap-4 text-muted-foreground">
            <span>{camera.ip_address}</span>
            {camera.manufacturer && <span>{camera.manufacturer}</span>}
            {camera.model && <span>{camera.model}</span>}
          </div>
          <div className="flex items-center gap-2">
            {/* Stream phase indicators */}
            {isPlaying && streamPhase === "main" && (
              <Badge variant="default" className="bg-green-600">
                LIVE
              </Badge>
            )}
            {isPlaying && streamPhase === "sub" && (
              <Badge variant="default" className="bg-yellow-600 flex items-center gap-1">
                <AlertTriangle className="h-3 w-3" />
                低画質
              </Badge>
            )}
            {streamPhase === "failed" && (
              <Badge variant="destructive" className="flex items-center gap-1">
                <AlertTriangle className="h-3 w-3" />
                接続失敗
              </Badge>
            )}
            {error && streamPhase !== "failed" && (
              <Badge variant="destructive">
                {error}
              </Badge>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}

export default LiveViewModal
