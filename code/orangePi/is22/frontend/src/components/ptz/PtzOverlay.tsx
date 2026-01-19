import { useCallback, useRef, useState, useEffect } from "react"
import { ChevronUp, ChevronDown, ChevronLeft, ChevronRight, Home, X } from "lucide-react"
import { usePtzControl } from "@/hooks/usePtzControl"
import type { PtzDirection } from "@/hooks/usePtzControl"

// 方向ラベル定義
const DIRECTION_LABELS: Record<PtzDirection, string> = {
  up: "↑ tilt up",
  down: "↓ tilt down",
  left: "← pan left",
  right: "→ pan right",
}

interface PtzOverlayProps {
  cameraId: string
  leaseId: string
  ptzSupported: boolean
  ptzDisabled: boolean
  ptzHomeSupported?: boolean
  visible?: boolean
  /** Camera rotation in degrees (0, 90, 180, 270) - UI follows camera orientation */
  rotation?: number
}

export function PtzOverlay({
  cameraId,
  leaseId,
  ptzSupported,
  ptzDisabled,
  ptzHomeSupported = false,
  visible = true,
  rotation = 0,
}: PtzOverlayProps) {
  const { move, stop, home, isMoving, currentDirection, error, clearError } = usePtzControl({
    cameraId,
    leaseId,
    enabled: ptzSupported && !ptzDisabled,
  })

  const pressTimeoutRef = useRef<number | null>(null)
  const isContinuousRef = useRef(false)

  // 操作フィードバック表示用
  const [feedback, setFeedback] = useState<string | null>(null)
  const feedbackTimeoutRef = useRef<number | null>(null)

  // フィードバック表示
  const showFeedback = useCallback((direction: PtzDirection) => {
    setFeedback(DIRECTION_LABELS[direction])
    if (feedbackTimeoutRef.current) {
      clearTimeout(feedbackTimeoutRef.current)
    }
    feedbackTimeoutRef.current = window.setTimeout(() => {
      setFeedback(null)
    }, 1500)
  }, [])

  // クリーンアップ
  useEffect(() => {
    return () => {
      if (feedbackTimeoutRef.current) {
        clearTimeout(feedbackTimeoutRef.current)
      }
    }
  }, [])

  // Handle press start (mousedown/touchstart)
  const handlePressStart = useCallback(
    (direction: PtzDirection) => {
      clearError()
      // フィードバック表示
      showFeedback(direction)
      // Immediate nudge
      move(direction, "nudge")

      // After 300ms, switch to continuous mode
      pressTimeoutRef.current = window.setTimeout(() => {
        isContinuousRef.current = true
        move(direction, "continuous")
      }, 300)
    },
    [move, clearError, showFeedback]
  )

  // Handle press end (mouseup/touchend)
  const handlePressEnd = useCallback(() => {
    if (pressTimeoutRef.current) {
      clearTimeout(pressTimeoutRef.current)
      pressTimeoutRef.current = null
    }

    if (isContinuousRef.current) {
      isContinuousRef.current = false
      stop()
    }
  }, [stop])

  // Handle home button
  const handleHome = useCallback(() => {
    clearError()
    home()
  }, [home, clearError])

  if (!visible || !ptzSupported || ptzDisabled) {
    return null
  }

  const buttonBaseClass =
    "absolute flex items-center justify-center w-12 h-12 rounded-full bg-black/50 hover:bg-black/70 active:bg-black/80 text-white transition-all select-none touch-none"
  const activeClass = "bg-orange-500/70 hover:bg-orange-500/80"

  return (
    <div className="absolute inset-0 pointer-events-none z-20">
      {/* PTZ Control Panel - rotates with camera orientation */}
      <div className="absolute bottom-4 left-4 pointer-events-auto">
        <div
          className="relative w-40 h-40 transition-transform"
          style={{ transform: `rotate(${rotation}deg)` }}
        >
          {/* Up */}
          <button
            className={`${buttonBaseClass} top-0 left-1/2 -translate-x-1/2 ${
              currentDirection === "up" ? activeClass : ""
            }`}
            onMouseDown={() => handlePressStart("up")}
            onMouseUp={handlePressEnd}
            onMouseLeave={handlePressEnd}
            onTouchStart={() => handlePressStart("up")}
            onTouchEnd={handlePressEnd}
            disabled={isMoving && currentDirection !== "up"}
          >
            <ChevronUp className="w-6 h-6" />
          </button>

          {/* Down */}
          <button
            className={`${buttonBaseClass} bottom-0 left-1/2 -translate-x-1/2 ${
              currentDirection === "down" ? activeClass : ""
            }`}
            onMouseDown={() => handlePressStart("down")}
            onMouseUp={handlePressEnd}
            onMouseLeave={handlePressEnd}
            onTouchStart={() => handlePressStart("down")}
            onTouchEnd={handlePressEnd}
            disabled={isMoving && currentDirection !== "down"}
          >
            <ChevronDown className="w-6 h-6" />
          </button>

          {/* Left */}
          <button
            className={`${buttonBaseClass} top-1/2 left-0 -translate-y-1/2 ${
              currentDirection === "left" ? activeClass : ""
            }`}
            onMouseDown={() => handlePressStart("left")}
            onMouseUp={handlePressEnd}
            onMouseLeave={handlePressEnd}
            onTouchStart={() => handlePressStart("left")}
            onTouchEnd={handlePressEnd}
            disabled={isMoving && currentDirection !== "left"}
          >
            <ChevronLeft className="w-6 h-6" />
          </button>

          {/* Right */}
          <button
            className={`${buttonBaseClass} top-1/2 right-0 -translate-y-1/2 ${
              currentDirection === "right" ? activeClass : ""
            }`}
            onMouseDown={() => handlePressStart("right")}
            onMouseUp={handlePressEnd}
            onMouseLeave={handlePressEnd}
            onTouchStart={() => handlePressStart("right")}
            onTouchEnd={handlePressEnd}
            disabled={isMoving && currentDirection !== "right"}
          >
            <ChevronRight className="w-6 h-6" />
          </button>

          {/* Center - Home button (if supported) */}
          {ptzHomeSupported && (
            <button
              className={`${buttonBaseClass} top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-10 h-10`}
              onClick={handleHome}
              disabled={isMoving}
              title="Home"
            >
              <Home className="w-5 h-5" />
            </button>
          )}
        </div>
      </div>

      {/* 操作フィードバック表示 */}
      {feedback && (
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 pointer-events-none">
          <div className="bg-black/70 text-white px-4 py-2 rounded-lg text-sm font-mono animate-pulse">
            {feedback}
          </div>
        </div>
      )}

      {/* Error display */}
      {error && (
        <div className="absolute top-4 left-1/2 -translate-x-1/2 pointer-events-auto">
          <div className="flex items-center gap-2 bg-red-500/90 text-white text-sm px-3 py-2 rounded-lg">
            <span>{error}</span>
            <button onClick={clearError} className="hover:text-red-200">
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
