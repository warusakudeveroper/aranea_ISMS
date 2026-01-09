/**
 * SuggestPane - AI Event Suggest View with Multi-Camera Video Playback
 *
 * Features:
 * - Max 3 cameras displayed simultaneously
 * - onairtime countdown (default 180s, configurable in settings)
 * - Slide-in animation from top for new cameras
 * - Slide-out animation to left on expiry
 * - Yellow highlight on camera tiles for on-air cameras
 * - Camera name, IP, detection badge on video
 * - No rounded corners, minimal gaps
 * - Full-bleed video (object-cover)
 */

import { useState, useEffect, useCallback, useRef, useMemo } from "react"
import type { Camera, DetectionLog } from "@/types/api"
import { Badge } from "@/components/ui/badge"
import { Video, Globe } from "lucide-react"
import { Go2rtcPlayer } from "./Go2rtcPlayer"
import { cn } from "@/lib/utils"
import { motion, AnimatePresence } from "framer-motion"

// On-air camera state
interface OnAirCamera {
  id: string // Unique ID for React key
  cameraId: string
  lacisId: string
  rtspUrl: string
  rtspUrlSub?: string    // Sub stream for fallback
  snapshotUrl?: string   // Snapshot for final fallback
  cameraName: string
  ipAddress: string
  startedAt: number
  lastEventAt: number
  primaryEvent: string
  severity: number
  // Animation state
  isEntering: boolean
  isExiting: boolean
  isFloating: boolean  // Floating up animation (when existing camera gets new event)
}

interface SuggestPaneProps {
  /** Current detection event (latest) */
  currentEvent: DetectionLog | null
  /** List of all cameras (to look up RTSP URL) */
  cameras?: Camera[]
  /** onairtime in seconds (default: 180) */
  onAirTimeSeconds?: number
  /** Callback when on-air cameras change (for tile highlighting) */
  onOnAirChange?: (cameraIds: string[]) => void
  /** Issue #108: モバイル表示モード - アニメーション方向を変更 */
  isMobile?: boolean
}

function getSeverityVariant(severity: number): "severity0" | "severity1" | "severity2" | "severity3" {
  if (severity === 0) return "severity0"
  if (severity === 1) return "severity1"
  if (severity === 2) return "severity2"
  return "severity3"
}

// Animation duration in ms
const ANIMATION_DURATION = 300

export function SuggestPane({
  currentEvent,
  cameras = [],
  onAirTimeSeconds = 180,
  onOnAirChange,
  isMobile = false,
}: SuggestPaneProps) {
  // On-air cameras (max 3)
  const [onAirCameras, setOnAirCameras] = useState<OnAirCamera[]>([])
  // Ref to track previous event to detect new events
  const lastEventIdRef = useRef<number | null>(null)

  // Find camera by lacis_id
  const findCamera = useCallback((lacisId: string): Camera | undefined => {
    return cameras.find(c => c.lacis_id === lacisId)
  }, [cameras])

  // Add or extend on-air camera
  const addOrExtendCamera = useCallback((event: DetectionLog) => {
    const camera = findCamera(event.lacis_id || "")
    if (!camera?.rtsp_main) {
      console.log("[SuggestPane] No RTSP URL for camera:", event.lacis_id)
      return
    }

    const eventLacisId = event.lacis_id || ""

    setOnAirCameras(prev => {
      // Check if already on-air (not exiting) - use normalized lacis_id for comparison
      const existingIndex = prev.findIndex(c => c.lacisId === eventLacisId && !c.isExiting)

      if (existingIndex >= 0) {
        // Extend: update lastEventAt and event info
        console.log("[SuggestPane] Extending on-air camera:", event.lacis_id, "at index:", existingIndex)

        if (existingIndex === 0) {
          // Already at top - just update event info (no animation needed)
          return prev.map((c, i) =>
            i === 0
              ? {
                  ...c,
                  lastEventAt: Date.now(),
                  primaryEvent: event.primary_event,
                  severity: event.severity,
                }
              : c
          )
        }

        // Not at top - move to top with floating animation
        console.log("[SuggestPane] Floating camera to top:", event.lacis_id)
        const existing = prev[existingIndex]
        const updated: OnAirCamera = {
          ...existing,
          lastEventAt: Date.now(),
          primaryEvent: event.primary_event,
          severity: event.severity,
          isFloating: true,
        }

        // Remove from current position and add to top
        const others = prev.filter((_, i) => i !== existingIndex)
        const newList = [updated, ...others]

        // Remove floating state after animation
        setTimeout(() => {
          setOnAirCameras(current =>
            current.map(c =>
              c.id === updated.id ? { ...c, isFloating: false } : c
            )
          )
        }, ANIMATION_DURATION)

        return newList
      }

      // New camera - use eventLacisId already normalized above
      // camera.rtsp_main is checked above, so it's guaranteed to be non-null
      const rtspUrl = camera.rtsp_main as string
      const rtspUrlSub = camera.rtsp_sub || undefined
      const snapshotUrl = `/api/snapshots/${camera.camera_id}/latest.jpg`
      const newCamera: OnAirCamera = {
        id: `${eventLacisId}-${Date.now()}`,
        cameraId: camera.camera_id,
        lacisId: eventLacisId,
        rtspUrl: rtspUrl,
        rtspUrlSub: rtspUrlSub,
        snapshotUrl: snapshotUrl,
        cameraName: camera.name || camera.camera_id,
        ipAddress: camera.ip_address || "",
        startedAt: Date.now(),
        lastEventAt: Date.now(),
        primaryEvent: event.primary_event,
        severity: event.severity,
        isEntering: true,
        isExiting: false,
        isFloating: false,
      }

      // Remove entering state after animation
      setTimeout(() => {
        setOnAirCameras(current =>
          current.map(c =>
            c.id === newCamera.id ? { ...c, isEntering: false } : c
          )
        )
      }, ANIMATION_DURATION)

      let newList = [newCamera, ...prev]

      // If more than 3, remove oldest (no exit animation for pushed-out camera)
      if (newList.length > 3) {
        newList = newList.slice(0, 3)
      }

      return newList
    })
  }, [findCamera])

  // Remove camera on stream error with slide-out animation (no onairtime wait)
  const removeCamera = useCallback((cameraId: string) => {
    console.log("[SuggestPane] Removing camera due to stream error:", cameraId)
    setOnAirCameras(prev => {
      const target = prev.find(c => c.cameraId === cameraId)
      if (!target || target.isExiting) return prev

      // Mark for exit animation
      const updated = prev.map(c =>
        c.cameraId === cameraId ? { ...c, isExiting: true } : c
      )

      // Remove after animation completes
      setTimeout(() => {
        setOnAirCameras(current => current.filter(c => c.cameraId !== cameraId))
      }, ANIMATION_DURATION)

      return updated
    })
  }, [])

  // Handle new detection events
  useEffect(() => {
    if (!currentEvent) return
    if (currentEvent.severity === 0) return

    // Check if this is a new event
    if (lastEventIdRef.current === currentEvent.log_id) return
    lastEventIdRef.current = currentEvent.log_id

    addOrExtendCamera(currentEvent)
  }, [currentEvent, addOrExtendCamera])

  // Check for expired cameras every second
  useEffect(() => {
    const interval = setInterval(() => {
      const now = Date.now()

      setOnAirCameras(prev => {
        const expired = prev.filter(c => {
          const elapsed = (now - c.lastEventAt) / 1000
          return elapsed >= onAirTimeSeconds && !c.isExiting
        })

        if (expired.length === 0) return prev

        // Mark expired cameras for exit animation
        const updated = prev.map(c => {
          const elapsed = (now - c.lastEventAt) / 1000
          if (elapsed >= onAirTimeSeconds && !c.isExiting) {
            return { ...c, isExiting: true }
          }
          return c
        })

        // Remove after animation
        expired.forEach(c => {
          setTimeout(() => {
            setOnAirCameras(current => current.filter(cam => cam.id !== c.id))
          }, ANIMATION_DURATION)
        })

        return updated
      })
    }, 1000)

    return () => clearInterval(interval)
  }, [onAirTimeSeconds])

  // Notify parent of on-air camera changes
  useEffect(() => {
    onOnAirChange?.(onAirCameras.map(c => c.cameraId))
  }, [onAirCameras, onOnAirChange])

  // All cameras including those exiting (for animation)
  const visibleCameras = useMemo(() => {
    return onAirCameras
  }, [onAirCameras])

  // Empty state - vertically centered
  if (visibleCameras.length === 0) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-muted-foreground p-4">
        <Video className="h-16 w-16 mb-4 opacity-30" />
        <p className="text-sm text-center">イベントなし</p>
        <p className="text-xs text-center mt-1 opacity-60">
          検知イベント発生時に自動表示
        </p>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      <AnimatePresence mode="popLayout">
        {visibleCameras.map((cam, index) => (
          <motion.div
            key={cam.lacisId}
            layout
            layoutId={cam.lacisId}
            // Issue #108: PC=top-in/left-out, モバイル=left-in/top-out
            initial={isMobile
              ? { opacity: 0, x: -100 }   // モバイル: 左からスライドイン
              : { opacity: 0, y: -50 }    // PC: 上からスライドイン
            }
            animate={{ opacity: 1, x: 0, y: 0 }}
            exit={isMobile
              ? { opacity: 0, y: -100 }   // モバイル: 上にスライドアウト
              : { opacity: 0, x: -100 }   // PC: 左にスライドアウト
            }
            transition={{
              layout: { type: "spring", stiffness: 300, damping: 30 },
              opacity: { duration: 0.2 }
            }}
            className={cn(
              // Fixed 1/3 height (not flex-1)
              "h-1/3 relative overflow-hidden bg-black",
              // Minimal gap (1px border)
              index > 0 && "border-t border-white/10"
            )}
          >
          {/* Video Player */}
          <Go2rtcPlayer
            cameraId={cam.cameraId}
            rtspUrl={cam.rtspUrl}
            rtspUrlSub={cam.rtspUrlSub}
            snapshotUrl={cam.snapshotUrl}
            autoPlay={true}
            muted={true}
            className="h-full w-full"
            onError={(err) => {
              console.error("[SuggestPane] Stream failed:", cam.cameraId, err)
              // Remove camera immediately on final failure (regardless of onairtime)
              removeCamera(cam.cameraId)
            }}
          />

          {/* Detection Badge - Top Left */}
          <Badge
            variant={getSeverityVariant(cam.severity)}
            className="absolute top-2 left-2 text-xs"
          >
            {cam.primaryEvent}
          </Badge>

          {/* LIVE Indicator - Top Right */}
          <div className="absolute top-2 right-2 flex items-center gap-1 px-2 py-0.5 bg-red-600 text-white text-xs">
            <span className="w-1.5 h-1.5 bg-white rounded-full animate-pulse" />
            LIVE
          </div>

          {/* Camera Info Overlay - Bottom */}
          <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 via-black/50 to-transparent p-2 pt-6">
            <p className="text-white text-sm font-medium truncate">
              {cam.cameraName}
            </p>
            <div className="flex items-center gap-1 text-white/70 text-xs mt-0.5">
              <Globe className="h-3 w-3" />
              <span>{cam.ipAddress}</span>
            </div>
          </div>
        </motion.div>
      ))}
      </AnimatePresence>
    </div>
  )
}
