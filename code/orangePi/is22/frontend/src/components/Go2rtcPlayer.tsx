/**
 * Go2rtcPlayer - MSE video player for go2rtc streams
 *
 * Uses go2rtc's WebSocket API with Media Source Extensions (MSE)
 * for low-latency browser video playback.
 *
 * Features tiered fallback with countdown timers:
 * - Phase 1: Main stream (10 second timeout)
 * - Phase 2: Sub stream with warning (20 second timeout)
 * - Phase 3: Snapshot fallback with error display
 */

import { useEffect, useRef, useState, useCallback } from "react"
import { Video, RefreshCw, WifiOff, AlertTriangle, Clock } from "lucide-react"
import { cn } from "@/lib/utils"

// go2rtc server configuration
const GO2RTC_HOST = window.location.hostname
const GO2RTC_PORT = 1984
const GO2RTC_BASE_URL = `http://${GO2RTC_HOST}:${GO2RTC_PORT}`
const GO2RTC_WS_URL = `ws://${GO2RTC_HOST}:${GO2RTC_PORT}`

// Timeout settings (milliseconds)
const MAIN_STREAM_TIMEOUT = 10000 // 10 seconds for main stream
const SUB_STREAM_TIMEOUT = 20000  // 20 seconds for sub stream

// Supported codecs for MSE streaming (from go2rtc official implementation)
const MSE_CODECS = [
  "avc1.640029", // H.264 high 4.1
  "avc1.64002A", // H.264 high 4.2
  "avc1.640033", // H.264 high 5.1
  "hvc1.1.6.L153.B0", // H.265 main 5.1
  "mp4a.40.2", // AAC LC
  "mp4a.40.5", // AAC HE
  "flac",
  "opus",
]

function getSupportedCodecs(): string {
  return MSE_CODECS
    .filter(codec => MediaSource.isTypeSupported(`video/mp4; codecs="${codec}"`))
    .join(",")
}

interface Go2rtcPlayerProps {
  cameraId: string
  rtspUrl: string       // Main stream URL
  rtspUrlSub?: string   // Sub stream URL (fallback)
  snapshotUrl?: string  // Snapshot URL (final fallback)
  autoPlay?: boolean
  muted?: boolean
  className?: string
  onError?: (error: string) => void
  onPlaying?: () => void
  onStopped?: () => void
  onPhaseChange?: (phase: StreamPhase) => void
}

type PlayerState = "idle" | "connecting" | "playing" | "error"
type StreamPhase = "main" | "sub" | "failed"

export function Go2rtcPlayer({
  cameraId,
  rtspUrl,
  rtspUrlSub,
  snapshotUrl,
  autoPlay = true,
  muted = true,
  className,
  onError,
  onPlaying,
  onStopped,
  onPhaseChange,
}: Go2rtcPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const [state, setState] = useState<PlayerState>("idle")
  const [errorMessage, setErrorMessage] = useState<string>("")
  const [retryTrigger, setRetryTrigger] = useState(0)

  // Tiered fallback state
  const [streamPhase, setStreamPhase] = useState<StreamPhase>("main")
  const [countdown, setCountdown] = useState<number>(0)
  const [showWarning, setShowWarning] = useState(false)

  // Use refs for all mutable state to avoid React callback dependencies
  const wsRef = useRef<WebSocket | null>(null)
  const mediaSourceRef = useRef<MediaSource | null>(null)
  const sourceBufferRef = useRef<SourceBuffer | null>(null)
  const pendingBuffersRef = useRef<ArrayBuffer[]>([])
  const isMountedRef = useRef(true)
  const isStoppedRef = useRef(false)
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const countdownIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const hasReceivedVideoRef = useRef(false)
  const streamPhaseRef = useRef<StreamPhase>("main")

  // Sync streamPhase to ref
  useEffect(() => {
    streamPhaseRef.current = streamPhase
    onPhaseChange?.(streamPhase)
  }, [streamPhase, onPhaseChange])

  // Manual retry handler - reset to main stream
  const handleManualRetry = useCallback(() => {
    setErrorMessage("")
    setState("idle")
    setStreamPhase("main")
    setShowWarning(false)
    setCountdown(0)
    hasReceivedVideoRef.current = false
    setRetryTrigger(t => t + 1)
  }, [])

  // Clear all timers
  const clearTimers = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
      timeoutRef.current = null
    }
    if (countdownIntervalRef.current) {
      clearInterval(countdownIntervalRef.current)
      countdownIntervalRef.current = null
    }
  }, [])

  // Single useEffect to manage the entire player lifecycle
  useEffect(() => {
    if (!autoPlay || !rtspUrl || !cameraId) return

    isMountedRef.current = true
    isStoppedRef.current = false
    hasReceivedVideoRef.current = false

    const video = videoRef.current
    if (!video) return

    // Callbacks stored in refs to avoid dependency issues
    const callbacksRef = { onError, onPlaying, onStopped }

    // Cleanup function
    const cleanup = () => {
      isStoppedRef.current = true
      clearTimers()

      if (wsRef.current) {
        wsRef.current.onclose = null
        wsRef.current.close()
        wsRef.current = null
      }

      if (mediaSourceRef.current?.readyState === "open") {
        try {
          mediaSourceRef.current.endOfStream()
        } catch { /* ignore */ }
      }
      mediaSourceRef.current = null
      sourceBufferRef.current = null
      pendingBuffersRef.current = []

      if (video) {
        video.pause()
        video.src = ""
        video.onplaying = null
        video.onerror = null
        video.oncanplay = null
      }
    }

    // Append buffer helper
    const appendBuffer = (data: ArrayBuffer) => {
      // Mark that we received video data - cancel timeout
      if (!hasReceivedVideoRef.current) {
        hasReceivedVideoRef.current = true
        clearTimers()
        setCountdown(0)
      }

      const sb = sourceBufferRef.current
      if (!sb) {
        pendingBuffersRef.current.push(data)
        return
      }
      if (sb.updating) {
        pendingBuffersRef.current.push(data)
        return
      }
      try {
        sb.appendBuffer(data)
      } catch (e) {
        console.error("[Go2rtcPlayer] appendBuffer error:", e)
      }
    }

    // Process pending buffers
    const processPending = () => {
      const sb = sourceBufferRef.current
      if (!sb || sb.updating) return
      const next = pendingBuffersRef.current.shift()
      if (next) {
        try {
          sb.appendBuffer(next)
        } catch { /* ignore */ }
      }
    }

    // Start countdown timer
    const startCountdown = (timeoutMs: number) => {
      const startTime = Date.now()
      setCountdown(Math.ceil(timeoutMs / 1000))

      countdownIntervalRef.current = setInterval(() => {
        const elapsed = Date.now() - startTime
        const remaining = Math.ceil((timeoutMs - elapsed) / 1000)
        setCountdown(Math.max(0, remaining))

        if (remaining <= 0) {
          if (countdownIntervalRef.current) {
            clearInterval(countdownIntervalRef.current)
            countdownIntervalRef.current = null
          }
        }
      }, 100)
    }

    // Handle phase timeout
    const handlePhaseTimeout = () => {
      if (isStoppedRef.current || hasReceivedVideoRef.current) return

      const currentPhase = streamPhaseRef.current
      console.log(`[Go2rtcPlayer] Phase timeout: ${currentPhase}`)

      // Close current connection
      if (wsRef.current) {
        wsRef.current.onclose = null
        wsRef.current.close()
        wsRef.current = null
      }

      if (currentPhase === "main" && rtspUrlSub) {
        // Move to sub stream
        console.log("[Go2rtcPlayer] Falling back to sub stream")
        setStreamPhase("sub")
        setShowWarning(true)
        hasReceivedVideoRef.current = false
        startStream(rtspUrlSub, SUB_STREAM_TIMEOUT)
      } else {
        // Final failure
        console.log("[Go2rtcPlayer] All streams failed")
        setStreamPhase("failed")
        setState("error")
        setErrorMessage("接続タイムアウト")
        callbacksRef.onError?.("接続タイムアウト")
      }
    }

    // Start streaming with specific URL and timeout
    const startStream = async (streamUrl: string, timeoutMs: number) => {
      if (isStoppedRef.current || !isMountedRef.current) return

      setState("connecting")
      hasReceivedVideoRef.current = false
      clearTimers()

      // Start countdown
      startCountdown(timeoutMs)

      // Set phase timeout
      timeoutRef.current = setTimeout(() => {
        if (!hasReceivedVideoRef.current) {
          handlePhaseTimeout()
        }
      }, timeoutMs)

      // Ensure stream exists in go2rtc
      try {
        const resp = await fetch(`${GO2RTC_BASE_URL}/api/streams`)
        if (resp.ok) {
          const streams = await resp.json()
          // Use phase-specific stream ID
          const streamId = streamPhaseRef.current === "sub" ? `${cameraId}-sub` : cameraId
          if (!streams[streamId]) {
            await fetch(
              `${GO2RTC_BASE_URL}/api/streams?name=${encodeURIComponent(streamId)}&src=${encodeURIComponent(streamUrl)}`,
              { method: "PUT" }
            )
            await new Promise(r => setTimeout(r, 300))
          }
        }
      } catch (e) {
        console.error("[Go2rtcPlayer] ensureStream error:", e)
      }

      if (isStoppedRef.current) return

      // Clean up any existing session
      if (wsRef.current) {
        wsRef.current.onclose = null
        wsRef.current.close()
        wsRef.current = null
      }
      if (mediaSourceRef.current?.readyState === "open") {
        try { mediaSourceRef.current.endOfStream() } catch { /* ignore */ }
      }
      sourceBufferRef.current = null
      pendingBuffersRef.current = []

      // Create MediaSource
      const mediaSource = new MediaSource()
      mediaSourceRef.current = mediaSource
      video.src = URL.createObjectURL(mediaSource)

      mediaSource.addEventListener("sourceopen", () => {
        if (isStoppedRef.current) return
        console.log("[Go2rtcPlayer] MediaSource opened")

        const streamId = streamPhaseRef.current === "sub" ? `${cameraId}-sub` : cameraId
        const wsUrl = `${GO2RTC_WS_URL}/api/ws?src=${encodeURIComponent(streamId)}`
        console.log("[Go2rtcPlayer] Connecting to:", wsUrl)

        const ws = new WebSocket(wsUrl)
        wsRef.current = ws
        ws.binaryType = "arraybuffer"

        ws.onopen = () => {
          if (isStoppedRef.current) return
          console.log("[Go2rtcPlayer] WebSocket connected")
          const codecs = getSupportedCodecs()
          ws.send(JSON.stringify({ type: "mse", value: codecs }))
        }

        ws.onmessage = (event) => {
          if (isStoppedRef.current) return

          if (typeof event.data === "string") {
            try {
              const msg = JSON.parse(event.data)

              if (msg.type === "mse") {
                const mimeType = msg.value
                if (!MediaSource.isTypeSupported(mimeType)) {
                  console.error("[Go2rtcPlayer] Unsupported MIME:", mimeType)
                  return
                }
                try {
                  const sb = mediaSource.addSourceBuffer(mimeType)
                  sourceBufferRef.current = sb
                  sb.mode = "segments"
                  sb.addEventListener("updateend", processPending)
                } catch (e) {
                  console.error("[Go2rtcPlayer] addSourceBuffer error:", e)
                }
              } else if (msg.type === "error") {
                console.error("[Go2rtcPlayer] Server error:", msg.value)
              }
            } catch {
              // Non-JSON message, ignore
            }
          } else {
            // Binary video data
            appendBuffer(event.data)
          }
        }

        ws.onerror = () => {
          if (isStoppedRef.current) return
          console.error("[Go2rtcPlayer] WebSocket error")
        }

        ws.onclose = () => {
          if (isStoppedRef.current) return
          console.log("[Go2rtcPlayer] WebSocket closed")
        }
      })

      video.onplaying = () => {
        if (isStoppedRef.current) return
        setState("playing")
        clearTimers()
        setCountdown(0)
        callbacksRef.onPlaying?.()
      }

      video.oncanplay = () => {
        if (isStoppedRef.current) return
        video.play().catch(e => console.log("[Go2rtcPlayer] Autoplay blocked:", e))
      }

      video.onerror = () => {
        console.log("[Go2rtcPlayer] Video error (ignored):", video.error?.message)
      }
    }

    // Start playback with main stream
    startStream(rtspUrl, MAIN_STREAM_TIMEOUT)

    // Cleanup on unmount
    return () => {
      isMountedRef.current = false
      cleanup()
      callbacksRef.onStopped?.()
    }
  }, [cameraId, rtspUrl, rtspUrlSub, autoPlay, retryTrigger, clearTimers])

  // Build snapshot URL
  const snapshotSrc = snapshotUrl || `/api/cameras/${cameraId}/snapshot?t=${Date.now()}`

  return (
    <div className={cn("relative bg-black rounded overflow-hidden", className)}>
      {/* Video element (hidden when failed) */}
      <video
        ref={videoRef}
        muted={muted}
        playsInline
        autoPlay
        className={cn(
          "w-full h-full object-cover",
          streamPhase === "failed" && "hidden"
        )}
      />

      {/* Snapshot fallback (shown when failed) */}
      {streamPhase === "failed" && (
        <img
          src={snapshotSrc}
          alt="Camera snapshot"
          className="w-full h-full object-contain"
        />
      )}

      {/* Countdown timer overlay (during connection attempts) */}
      {state === "connecting" && countdown > 0 && (
        <div className="absolute top-2 right-2 flex items-center gap-1 bg-black/70 px-2 py-1 rounded text-white text-xs">
          <Clock className="h-3 w-3" />
          <span>{countdown}秒</span>
        </div>
      )}

      {/* Sub stream warning banner */}
      {showWarning && streamPhase !== "failed" && (
        <div className="absolute top-0 left-0 right-0 bg-yellow-500/90 text-black text-xs py-1 px-2 flex items-center justify-center gap-1">
          <AlertTriangle className="h-3 w-3" />
          <span>通信に問題がある可能性 - 低画質ストリームで再生中</span>
        </div>
      )}

      {/* Connecting overlay */}
      {state === "connecting" && (
        <div className="absolute inset-0 flex flex-col items-center justify-center bg-black/50">
          <RefreshCw className="h-8 w-8 text-white animate-spin mb-2" />
          <span className="text-white/80 text-sm">
            {streamPhase === "main" ? "接続中..." : "サブストリームに接続中..."}
          </span>
        </div>
      )}

      {/* Error overlay (final failure) */}
      {state === "error" && (
        <div className="absolute inset-0 flex flex-col items-center justify-center bg-black/80">
          <WifiOff className="h-8 w-8 text-red-400 mb-2" />
          <span className="text-red-300 text-sm font-medium">
            {errorMessage || "接続タイムアウト"}
          </span>
          <span className="text-white/60 text-xs mt-1 text-center px-4">
            カメラのストリームを再生できませんでした
          </span>
          <span className="text-yellow-400/80 text-xs mt-0.5">
            ネットワーク問題の可能性があります
          </span>
          <button
            onClick={handleManualRetry}
            className="mt-3 px-4 py-1.5 bg-white/20 rounded text-white text-sm hover:bg-white/30 flex items-center gap-1"
          >
            <RefreshCw className="h-3 w-3" />
            再試行
          </button>
        </div>
      )}

      {/* Idle state */}
      {state === "idle" && (
        <div className="absolute inset-0 flex items-center justify-center">
          <Video className="h-12 w-12 text-white/30" />
        </div>
      )}
    </div>
  )
}

export default Go2rtcPlayer
