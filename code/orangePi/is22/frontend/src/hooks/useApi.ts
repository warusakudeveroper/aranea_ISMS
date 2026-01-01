import { useState, useEffect, useCallback } from "react"
import type { ApiResponse } from "@/types/api"
import { API_BASE_URL } from "@/lib/config"

interface UseApiState<T> {
  data: T | null
  loading: boolean
  error: string | null
  refetch: () => void
}

export function useApi<T>(url: string, interval?: number): UseApiState<T> {
  const [data, setData] = useState<T | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchData = useCallback(async () => {
    try {
      const fullUrl = `${API_BASE_URL}${url}`
      const response = await fetch(fullUrl)
      const json: ApiResponse<T> = await response.json()

      if (json.ok) {
        setData(json.data)
        setError(null)
      } else {
        setError(json.error || "Unknown error")
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Network error")
    } finally {
      setLoading(false)
    }
  }, [url])

  useEffect(() => {
    fetchData()

    if (interval) {
      const timer = setInterval(fetchData, interval)
      return () => clearInterval(timer)
    }
  }, [fetchData, interval])

  return { data, loading, error, refetch: fetchData }
}
