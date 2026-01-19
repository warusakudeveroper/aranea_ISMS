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
import { PtzOverlay } from "./ptz/PtzOverlay"
import {
  Video,
  Settings,
  AlertTriangle,
  Move,
} from "lucide-react"
import { useState, useCallback, useEffect, useRef, useMemo } from "react"
import { API_BASE_URL } from "@/lib/config"

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
  const [leaseId, setLeaseId] = useState<string | null>(null)
  const [absorberSessionId, setAbsorberSessionId] = useState<string | null>(null)
  const heartbeatRef = useRef<number | null>(null)

  // PTZ操作UIの自動表示: PTZ対応 && 無効化されていない && Lease取得済み
  const showPtzControls = useMemo(() => {
    return camera?.ptz_supported === true && camera?.ptz_disabled !== true && leaseId !== null
  }, [camera?.ptz_supported, camera?.ptz_disabled, leaseId])

  // Request lease and AccessAbsorber stream when modal opens
  useEffect(() => {
    if (!open || !camera) return

    const requestLease = async () => {
      try {
        const response = await fetch(`${API_BASE_URL}/api/modal/lease`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            user_id: "LiveViewModal",
            camera_id: camera.camera_id,
            quality: "main",
          }),
        })
        const result = await response.json()
        if (result.ok && result.data?.lease_id) {
          setLeaseId(result.data.lease_id)
        }
      } catch (e) {
        console.warn("Failed to acquire lease:", e)
      }
    }

    // Request stream through AccessAbsorber (click_modal has highest priority)
    // This will preempt lower-priority streams like suggest_play
    const requestAbsorberStream = async () => {
      try {
        const response = await fetch(`${API_BASE_URL}/api/access-absorber/streams/acquire`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            camera_id: camera.camera_id,
            purpose: "click_modal",
            client_id: `LiveViewModal-${Date.now()}`,
            stream_type: "main",
            allow_preempt: true,  // Allow preemption of lower priority streams
          }),
        })
        const result = await response.json()
        if (result.success && result.token?.session_id) {
          setAbsorberSessionId(result.token.session_id)
          console.log("[LiveViewModal] AccessAbsorber stream acquired:", result.token.session_id)
          // Log if preemption occurred
          if (result.preemption) {
            console.log("[LiveViewModal] Preempted stream:", result.preemption)
          }
        } else {
          console.warn("[LiveViewModal] AccessAbsorber denied:", result.error)
        }
      } catch (e) {
        console.warn("[LiveViewModal] AccessAbsorber request failed:", e)
        // Continue without AccessAbsorber (fallback to direct stream)
      }
    }

    requestLease()
    requestAbsorberStream()

    // Cleanup on close
    return () => {
      if (heartbeatRef.current) {
        clearInterval(heartbeatRef.current)
        heartbeatRef.current = null
      }
    }
  }, [open, camera])

  // Heartbeat while modal is open
  useEffect(() => {
    if (!leaseId) return

    const sendHeartbeat = async () => {
      try {
        await fetch(`${API_BASE_URL}/api/modal/lease/${leaseId}/heartbeat`, {
          method: "POST",
        })
      } catch (e) {
        console.warn("Heartbeat failed:", e)
      }
    }

    // Send heartbeat every 30 seconds
    heartbeatRef.current = window.setInterval(sendHeartbeat, 30000)

    return () => {
      if (heartbeatRef.current) {
        clearInterval(heartbeatRef.current)
        heartbeatRef.current = null
      }
    }
  }, [leaseId])

  // Release lease and AccessAbsorber stream when modal closes
  const releaseLease = useCallback(async () => {
    // Release PTZ lease
    if (leaseId) {
      try {
        await fetch(`${API_BASE_URL}/api/modal/lease/${leaseId}`, {
          method: "DELETE",
        })
      } catch (e) {
        console.warn("Failed to release lease:", e)
      }
    }

    // Release AccessAbsorber stream
    if (absorberSessionId) {
      try {
        await fetch(`${API_BASE_URL}/api/access-absorber/streams/release`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ session_id: absorberSessionId }),
        })
        console.log("[LiveViewModal] AccessAbsorber stream released:", absorberSessionId)
      } catch (e) {
        console.warn("Failed to release AccessAbsorber stream:", e)
      }
    }
  }, [leaseId, absorberSessionId])

  // Reset state when modal opens/closes
  const handleOpenChange = useCallback((newOpen: boolean) => {
    if (!newOpen) {
      // Release lease/absorber and reset state on close
      releaseLease()
      setIsPlaying(false)
      setError(null)
      setStreamPhase("main")
      setLeaseId(null)
      setAbsorberSessionId(null)
    }
    onOpenChange(newOpen)
  }, [onOpenChange, releaseLease])

  if (!camera) return null

  // Build RTSP URLs for go2rtc
  const rtspMain = camera.rtsp_main || ""
  const rtspSub = camera.rtsp_sub || ""
  const hasAnyStream = rtspMain || rtspSub

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      {/* max-w-6xl = 1152px (約130% of max-w-4xl 896px) */}
      <DialogContent className="max-w-6xl p-0 overflow-hidden">
        <DialogHeader className="p-4 pb-2 flex flex-row items-center justify-between">
          <DialogTitle className="flex items-center gap-2">
            <Video className="h-5 w-5" />
            {camera.name}
            {camera.rid && (
              <Badge variant="outline" className="ml-2">
                {camera.rid}
              </Badge>
            )}
            {/* PTZ対応カメラはバッジ表示 */}
            {camera.ptz_supported && !camera.ptz_disabled && (
              <Badge variant="secondary" className="ml-1 flex items-center gap-1">
                <Move className="h-3 w-3" />
                PTZ
              </Badge>
            )}
          </DialogTitle>
          <div className="flex items-center gap-2">
            {/* PTZトグルボタン廃止: PTZ対応カメラは自動でオーバーレイ表示 */}
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
        <div className="relative aspect-video bg-black overflow-hidden">
          {hasAnyStream ? (
            <div
              className="w-full h-full"
              style={{ transform: `rotate(${camera.rotation || 0}deg)` }}
            >
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
            </div>
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

          {/* PTZ Overlay - PTZ対応カメラでは自動表示 */}
          {showPtzControls && leaseId && (
            <PtzOverlay
              cameraId={camera.camera_id}
              leaseId={leaseId}
              ptzSupported={camera.ptz_supported}
              ptzDisabled={camera.ptz_disabled}
              ptzHomeSupported={camera.ptz_home_supported}
              visible={true}
              rotation={camera.rotation || 0}
            />
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
