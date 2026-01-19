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
import { Video, Globe, AlertTriangle, Pause } from "lucide-react"
import { Go2rtcPlayer } from "./Go2rtcPlayer"
import { cn } from "@/lib/utils"
import { motion, AnimatePresence } from "framer-motion"
import { API_BASE_URL } from "@/lib/config"
import { useWebSocket, type StreamPreemptedMessage } from "@/hooks/useWebSocket"

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
  rotation: number       // Camera rotation for display
  sessionId?: string     // AccessAbsorber session ID for stream release
  // Animation state
  isEntering: boolean
  isExiting: boolean
  isFloating: boolean  // Floating up animation (when existing camera gets new event)
  // Preemption state
  isPreempted: boolean  // True when this camera's stream was preempted
  preemptCountdown?: number  // Countdown seconds before exit
}

// AccessAbsorber error response type
interface AbsorberUserMessage {
  title: string
  message: string
  severity: "info" | "warning" | "error"
  show_duration_sec?: number
  action_hint?: string
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function generateUserMessage(error: any): AbsorberUserMessage {
  switch (error.error_type) {
    case "concurrent_limit_reached":
      return {
        title: "ストリーミング制限",
        message: `このカメラは現在${error.current_purposes?.[0] || "他のプロセス"}で再生中`,
        severity: "warning",
        show_duration_sec: 3,
        action_hint: "しばらく待ってから再度お試しください",
      }
    case "reconnect_interval_not_met":
      return {
        title: "接続間隔制限",
        message: `${Math.ceil((error.remaining_ms || 0) / 1000)}秒後に再試行してください`,
        severity: "info",
        show_duration_sec: Math.ceil((error.remaining_ms || 3000) / 1000),
      }
    case "exclusive_lock_failed":
      return {
        title: "排他制御",
        message: `カメラは${error.held_purpose || "別の処理"}で使用中`,
        severity: "warning",
        show_duration_sec: 3,
      }
    default:
      return {
        title: "ストリーム取得失敗",
        message: "カメラに接続できませんでした",
        severity: "error",
        show_duration_sec: 3,
      }
  }
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
  // AccessAbsorber conflict feedback message
  const [deniedMessage, setDeniedMessage] = useState<AbsorberUserMessage | null>(null)
  const deniedTimeoutRef = useRef<number | null>(null)
  // Preemption message state
  const [preemptMessage, setPreemptMessage] = useState<{cameraId: string, message: AbsorberUserMessage} | null>(null)
  const preemptTimeoutRef = useRef<number | null>(null)
  // Countdown interval ref for preemption
  const countdownIntervalRef = useRef<number | null>(null)

  // Find camera by lacis_id
  const findCamera = useCallback((lacisId: string): Camera | undefined => {
    return cameras.find(c => c.lacis_id === lacisId)
  }, [cameras])

  // Request stream through AccessAbsorber
  const requestStream = useCallback(async (cameraId: string): Promise<string | null> => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/access-absorber/streams/acquire`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          camera_id: cameraId,
          purpose: "suggest_play",
          client_id: `SuggestPane-${Date.now()}`,
          stream_type: "sub",
          allow_preempt: false,
        }),
      })
      const result = await response.json()

      if (!result.success || !result.token) {
        // Show conflict feedback
        if (result.error?.error_type) {
          const userMessage = generateUserMessage(result.error)
          setDeniedMessage(userMessage)

          // Auto-clear after duration
          if (deniedTimeoutRef.current) {
            clearTimeout(deniedTimeoutRef.current)
          }
          const duration = userMessage.show_duration_sec || 3
          deniedTimeoutRef.current = window.setTimeout(() => {
            setDeniedMessage(null)
          }, duration * 1000)
        }
        console.log("[SuggestPane] Stream request denied:", result.error)
        return null
      }

      console.log("[SuggestPane] Stream acquired:", result.token.session_id)
      return result.token.session_id
    } catch (e) {
      console.warn("[SuggestPane] AccessAbsorber request failed:", e)
      // Fall back to direct stream (AccessAbsorber may be disabled)
      return "fallback"
    }
  }, [])

  // Release stream through AccessAbsorber
  const releaseStream = useCallback(async (sessionId: string | undefined) => {
    if (!sessionId || sessionId === "fallback") return

    try {
      await fetch(`${API_BASE_URL}/api/access-absorber/streams/release`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ session_id: sessionId }),
      })
      console.log("[SuggestPane] Stream released:", sessionId)
    } catch (e) {
      console.warn("[SuggestPane] Failed to release stream:", e)
    }
  }, [])

  // Handle stream preemption notification from WebSocket
  const handleStreamPreempted = useCallback((msg: StreamPreemptedMessage) => {
    console.log("[SuggestPane] Stream preempted:", msg)

    // Only handle suggest_play preemptions
    if (msg.preempted_purpose !== "suggest_play") return

    // Find the camera in on-air list
    const targetCamera = onAirCameras.find(c => c.cameraId === msg.camera_id)
    if (!targetCamera) {
      console.log("[SuggestPane] Preempted camera not found in on-air list:", msg.camera_id)
      return
    }

    // Show preemption message
    setPreemptMessage({
      cameraId: msg.camera_id,
      message: {
        title: msg.user_message.title,
        message: msg.user_message.message,
        severity: msg.user_message.severity,
        show_duration_sec: msg.exit_delay_sec,
      },
    })

    // Mark camera as preempted and start countdown
    setOnAirCameras(prev =>
      prev.map(c =>
        c.cameraId === msg.camera_id
          ? { ...c, isPreempted: true, preemptCountdown: msg.exit_delay_sec }
          : c
      )
    )

    // Clear any existing countdown interval
    if (countdownIntervalRef.current) {
      clearInterval(countdownIntervalRef.current)
    }

    // Start countdown interval
    let countdown = msg.exit_delay_sec
    countdownIntervalRef.current = window.setInterval(() => {
      countdown--
      if (countdown <= 0) {
        // Countdown finished - remove camera
        if (countdownIntervalRef.current) {
          clearInterval(countdownIntervalRef.current)
          countdownIntervalRef.current = null
        }
        setPreemptMessage(null)

        // Exit the camera
        setOnAirCameras(prev => {
          const target = prev.find(c => c.cameraId === msg.camera_id)
          if (!target) return prev

          // Mark for exit animation
          const updated = prev.map(c =>
            c.cameraId === msg.camera_id ? { ...c, isExiting: true, isPreempted: false } : c
          )

          // Remove after animation completes
          setTimeout(() => {
            setOnAirCameras(current => current.filter(c => c.cameraId !== msg.camera_id))
          }, ANIMATION_DURATION)

          return updated
        })
      } else {
        // Update countdown
        setOnAirCameras(prev =>
          prev.map(c =>
            c.cameraId === msg.camera_id ? { ...c, preemptCountdown: countdown } : c
          )
        )
      }
    }, 1000)

    // Clear preemption message after delay
    if (preemptTimeoutRef.current) {
      clearTimeout(preemptTimeoutRef.current)
    }
    preemptTimeoutRef.current = window.setTimeout(() => {
      setPreemptMessage(null)
    }, msg.exit_delay_sec * 1000)
  }, [onAirCameras])

  // WebSocket for stream preemption notifications
  useWebSocket({
    onStreamPreempted: handleStreamPreempted,
  })

  // Add or extend on-air camera
  const addOrExtendCamera = useCallback(async (event: DetectionLog) => {
    const camera = findCamera(event.lacis_id || "")
    if (!camera?.rtsp_main) {
      console.log("[SuggestPane] No RTSP URL for camera:", event.lacis_id)
      return
    }

    const eventLacisId = event.lacis_id || ""

    // Check if camera already exists
    const existsInList = onAirCameras.some(c =>
      c.lacisId === eventLacisId || c.cameraId === camera.camera_id
    )

    // For new cameras, request stream through AccessAbsorber first
    let sessionId: string | null = null
    if (!existsInList) {
      sessionId = await requestStream(camera.camera_id)
      if (sessionId === null) {
        // Stream denied - don't add camera
        console.log("[SuggestPane] Camera not added due to AccessAbsorber denial:", camera.camera_id)
        return
      }
    }

    setOnAirCameras(prev => {
      // Check if already on-air - include exiting cameras for resurrection
      // Use both lacisId and cameraId for robust comparison
      const existingIndex = prev.findIndex(c =>
        c.lacisId === eventLacisId || c.cameraId === camera.camera_id
      )

      if (existingIndex >= 0) {
        const existing = prev[existingIndex]

        // If exiting, resurrect with NEW id (to invalidate the removal timeout)
        if (existing.isExiting) {
          console.log("[SuggestPane] Resurrecting exiting camera:", event.lacis_id)
          const resurrected: OnAirCamera = {
            ...existing,
            id: `${eventLacisId}-${Date.now()}`, // New ID to cancel old timeout
            lastEventAt: Date.now(),
            primaryEvent: event.primary_event,
            severity: event.severity,
            isExiting: false,
            isEntering: false,
            isFloating: existingIndex > 0, // Float to top if not already there
            isPreempted: false, // Clear preemption state on resurrection
            preemptCountdown: undefined,
          }

          // Remove old entry and add resurrected to top
          const others = prev.filter((_, i) => i !== existingIndex)
          const newList = [resurrected, ...others]

          // Remove floating state after animation
          if (resurrected.isFloating) {
            setTimeout(() => {
              setOnAirCameras(current =>
                current.map(c =>
                  c.id === resurrected.id ? { ...c, isFloating: false } : c
                )
              )
            }, ANIMATION_DURATION)
          }

          return newList
        }

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
        rotation: camera.rotation ?? 0,
        sessionId: sessionId || undefined,  // AccessAbsorber session ID
        isEntering: true,
        isExiting: false,
        isFloating: false,
        isPreempted: false,
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

      // If more than 3 active (non-exiting) cameras, remove oldest active ones
      // This ensures exiting cameras don't count towards the limit
      const activeList = newList.filter(c => !c.isExiting)
      if (activeList.length > 3) {
        const toRemove = activeList.slice(3) // Get cameras beyond the 3-camera limit
        newList = newList.filter(c => c.isExiting || !toRemove.includes(c))
      }

      return newList
    })
  }, [findCamera, onAirCameras, requestStream])

  // Remove camera on stream error with slide-out animation (no onairtime wait)
  const removeCamera = useCallback((cameraId: string) => {
    console.log("[SuggestPane] Removing camera due to stream error:", cameraId)
    setOnAirCameras(prev => {
      const target = prev.find(c => c.cameraId === cameraId)
      if (!target || target.isExiting) return prev

      // Release stream through AccessAbsorber
      releaseStream(target.sessionId)

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
  }, [releaseStream])

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

        // Release streams for expired cameras
        expired.forEach(c => {
          releaseStream(c.sessionId)
        })

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
  }, [onAirTimeSeconds, releaseStream])

  // Notify parent of on-air camera changes
  useEffect(() => {
    onOnAirChange?.(onAirCameras.map(c => c.cameraId))
  }, [onAirCameras, onOnAirChange])

  // Visible cameras: max 3 total (active first, then exiting within limit)
  // This ensures no more than 3 cameras are ever displayed
  const visibleCameras = useMemo(() => {
    const MAX_DISPLAY = 3
    const active = onAirCameras.filter(c => !c.isExiting)
    const exiting = onAirCameras.filter(c => c.isExiting)

    // If active cameras fill the slots, no room for exiting
    if (active.length >= MAX_DISPLAY) {
      return active.slice(0, MAX_DISPLAY)
    }

    // Show active cameras + remaining slots for exiting cameras
    const remainingSlots = MAX_DISPLAY - active.length
    return [...active, ...exiting.slice(0, remainingSlots)]
  }, [onAirCameras])

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      // Release all streams on unmount
      onAirCameras.forEach(cam => {
        releaseStream(cam.sessionId)
      })
      // Clear denied message timeout
      if (deniedTimeoutRef.current) {
        clearTimeout(deniedTimeoutRef.current)
      }
      // Clear preemption timeouts
      if (preemptTimeoutRef.current) {
        clearTimeout(preemptTimeoutRef.current)
      }
      if (countdownIntervalRef.current) {
        clearInterval(countdownIntervalRef.current)
      }
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

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
    <div className={cn(
      "h-full flex",
      // Issue #108: モバイル=横並び（左から右へ）、PC=縦並び（上から下へ）
      isMobile ? "flex-row" : "flex-col"
    )}>
      <AnimatePresence mode="popLayout">
        {visibleCameras.map((cam, index) => (
          <motion.div
            key={cam.lacisId}
            layout
            layoutId={cam.lacisId}
            // Issue #108: PC=top-in/left-out, モバイル=left-in/right-out
            initial={isMobile
              ? { opacity: 0, x: -100 }   // モバイル: 左からスライドイン
              : { opacity: 0, y: -50 }    // PC: 上からスライドイン
            }
            animate={{ opacity: 1, x: 0, y: 0 }}
            exit={isMobile
              ? { opacity: 0, x: 100 }    // モバイル: 右にスライドアウト
              : { opacity: 0, x: -100 }   // PC: 左にスライドアウト
            }
            transition={{
              layout: { type: "spring", stiffness: 300, damping: 30 },
              opacity: { duration: 0.2 }
            }}
            className={cn(
              // Issue #108: モバイル=横幅1/3（縦長）、PC=高さ1/3
              isMobile ? "w-1/3 h-full" : "h-1/3 w-full",
              "relative overflow-hidden bg-black",
              // Minimal gap (1px border)
              isMobile
                ? index > 0 && "border-l border-white/10"
                : index > 0 && "border-t border-white/10"
            )}
          >
          {/* Video Player with rotation applied */}
          <div
            className="h-full w-full"
            style={{ transform: `rotate(${cam.rotation || 0}deg)` }}
          >
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
          </div>

          {/* Detection Badge - Top Left */}
          <Badge
            variant={getSeverityVariant(cam.severity)}
            className={cn(
              "absolute text-xs",
              // Issue #108: モバイル時は小さめ
              isMobile ? "top-1 left-1 text-[10px] px-1 py-0" : "top-2 left-2"
            )}
          >
            {cam.primaryEvent}
          </Badge>

          {/* LIVE Indicator - Top Right */}
          <div className={cn(
            "absolute flex items-center gap-1 bg-red-600 text-white",
            // Issue #108: モバイル時は小さめ
            isMobile ? "top-1 right-1 px-1 py-0 text-[10px]" : "top-2 right-2 px-2 py-0.5 text-xs"
          )}>
            <span className={cn(
              "bg-white rounded-full animate-pulse",
              isMobile ? "w-1 h-1" : "w-1.5 h-1.5"
            )} />
            LIVE
          </div>

          {/* Camera Info Overlay - Bottom */}
          <div className={cn(
            "absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 via-black/50 to-transparent",
            // Issue #108: モバイル時はコンパクト
            isMobile ? "p-1 pt-4" : "p-2 pt-6"
          )}>
            <p className={cn(
              "text-white font-medium truncate",
              isMobile ? "text-[10px]" : "text-sm"
            )}>
              {cam.cameraName}
            </p>
            {!isMobile && (
              <div className="flex items-center gap-1 text-white/70 text-xs mt-0.5">
                <Globe className="h-3 w-3" />
                <span>{cam.ipAddress}</span>
              </div>
            )}
          </div>

          {/* Preemption Overlay - shows when camera stream is preempted */}
          {cam.isPreempted && (
            <div className="absolute inset-0 bg-black/70 flex flex-col items-center justify-center z-10">
              <Pause className={cn("text-amber-400 mb-2", isMobile ? "h-6 w-6" : "h-10 w-10")} />
              <p className={cn("text-amber-400 font-medium text-center", isMobile ? "text-xs" : "text-sm")}>
                優先制御
              </p>
              {cam.preemptCountdown !== undefined && (
                <p className={cn("text-white/80 text-center mt-1", isMobile ? "text-[10px]" : "text-xs")}>
                  {cam.preemptCountdown}秒後に終了
                </p>
              )}
            </div>
          )}
        </motion.div>
      ))}
      </AnimatePresence>

      {/* AccessAbsorber Denied Message */}
      <AnimatePresence>
        {deniedMessage && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 20 }}
            className={cn(
              "absolute z-50 flex items-center gap-2 px-4 py-2 rounded-lg shadow-lg",
              deniedMessage.severity === "error" && "bg-red-500 text-white",
              deniedMessage.severity === "warning" && "bg-amber-500 text-white",
              deniedMessage.severity === "info" && "bg-blue-500 text-white",
              isMobile
                ? "bottom-2 left-1/2 -translate-x-1/2 text-xs"
                : "bottom-4 left-1/2 -translate-x-1/2 text-sm"
            )}
          >
            <AlertTriangle className={cn(isMobile ? "h-3 w-3" : "h-4 w-4")} />
            <span>{deniedMessage.message}</span>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Stream Preemption Message */}
      <AnimatePresence>
        {preemptMessage && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 20 }}
            className={cn(
              "absolute z-50 flex items-center gap-2 px-4 py-2 rounded-lg shadow-lg bg-amber-500 text-white",
              isMobile
                ? "bottom-2 left-1/2 -translate-x-1/2 text-xs"
                : "bottom-4 left-1/2 -translate-x-1/2 text-sm"
            )}
          >
            <Pause className={cn(isMobile ? "h-3 w-3" : "h-4 w-4")} />
            <span>{preemptMessage.message.message}</span>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}
