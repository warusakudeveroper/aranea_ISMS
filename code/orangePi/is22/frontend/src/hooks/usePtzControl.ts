import { useState, useCallback, useRef, useEffect } from "react"
import { API_BASE_URL } from "@/lib/config"

export type PtzDirection = "up" | "down" | "left" | "right"
export type PtzMode = "nudge" | "continuous"

interface PtzMoveRequest {
  lease_id: string
  direction: PtzDirection
  mode: PtzMode
  speed?: number
  duration_ms?: number
}

interface PtzResponse {
  ok: boolean
  error?: string
  message?: string
}

interface UsePtzControlOptions {
  cameraId: string
  leaseId: string
  enabled?: boolean
}

interface UsePtzControlReturn {
  move: (direction: PtzDirection, mode?: PtzMode) => Promise<void>
  stop: () => Promise<void>
  home: () => Promise<void>
  isMoving: boolean
  currentDirection: PtzDirection | null
  error: string | null
  clearError: () => void
}

export function usePtzControl({
  cameraId,
  leaseId,
  enabled = true,
}: UsePtzControlOptions): UsePtzControlReturn {
  const [isMoving, setIsMoving] = useState(false)
  const [currentDirection, setCurrentDirection] = useState<PtzDirection | null>(null)
  const [error, setError] = useState<string | null>(null)
  const continuousRef = useRef(false)

  // Stop movement on unmount (safety)
  useEffect(() => {
    return () => {
      if (continuousRef.current) {
        fetch(`${API_BASE_URL}/api/cameras/${cameraId}/ptz/stop`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ lease_id: leaseId }),
        }).catch(() => {
          // Ignore errors during cleanup
        })
      }
    }
  }, [cameraId, leaseId])

  const move = useCallback(
    async (direction: PtzDirection, mode: PtzMode = "nudge") => {
      if (!enabled || !leaseId) return

      setError(null)
      setIsMoving(true)
      setCurrentDirection(direction)

      if (mode === "continuous") {
        continuousRef.current = true
      }

      const request: PtzMoveRequest = {
        lease_id: leaseId,
        direction,
        mode,
        speed: 0.5,
        duration_ms: 120,
      }

      try {
        const response = await fetch(
          `${API_BASE_URL}/api/cameras/${cameraId}/ptz/move`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(request),
          }
        )

        const result: PtzResponse = await response.json()

        if (!result.ok) {
          setError(result.error || "PTZ operation failed")
          setIsMoving(false)
          setCurrentDirection(null)
          continuousRef.current = false
        } else if (mode === "nudge") {
          // Nudge mode auto-stops after duration_ms
          setTimeout(() => {
            setIsMoving(false)
            setCurrentDirection(null)
          }, 150)
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Network error")
        setIsMoving(false)
        setCurrentDirection(null)
        continuousRef.current = false
      }
    },
    [cameraId, leaseId, enabled]
  )

  const stop = useCallback(async () => {
    if (!enabled || !leaseId) return

    continuousRef.current = false

    try {
      const response = await fetch(
        `${API_BASE_URL}/api/cameras/${cameraId}/ptz/stop`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ lease_id: leaseId }),
        }
      )

      const result: PtzResponse = await response.json()

      if (!result.ok) {
        setError(result.error || "PTZ stop failed")
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Network error")
    } finally {
      setIsMoving(false)
      setCurrentDirection(null)
    }
  }, [cameraId, leaseId, enabled])

  const home = useCallback(async () => {
    if (!enabled || !leaseId) return

    setError(null)
    setIsMoving(true)

    try {
      const response = await fetch(
        `${API_BASE_URL}/api/cameras/${cameraId}/ptz/home`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ lease_id: leaseId }),
        }
      )

      const result: PtzResponse = await response.json()

      if (!result.ok) {
        setError(result.error || "PTZ home failed")
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Network error")
    } finally {
      // Home position movement takes a few seconds
      setTimeout(() => {
        setIsMoving(false)
        setCurrentDirection(null)
      }, 3000)
    }
  }, [cameraId, leaseId, enabled])

  const clearError = useCallback(() => {
    setError(null)
  }, [])

  return {
    move,
    stop,
    home,
    isMoving,
    currentDirection,
    error,
    clearError,
  }
}
