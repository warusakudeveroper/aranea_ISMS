import type { ScannedDevice } from "@/types/api"

// LocalStorage cache for scan results (is22_14_UI改善設計_v1.1.md Section 1)
const SCAN_CACHE_KEY = "is22_lastScanResult"
const CACHE_EXPIRY_HOURS = 24

export interface CachedScanResult {
  timestamp: string
  expiry: string
  subnets: string[]
  devices: ScannedDevice[]
  summary: {
    total_ips: number
    hosts_alive: number
    cameras_found: number
    cameras_verified: number
  } | null
}

export function saveScanResultToCache(
  devices: ScannedDevice[],
  subnets: string[],
  summary: CachedScanResult["summary"]
): void {
  const now = new Date()
  const expiry = new Date(now.getTime() + CACHE_EXPIRY_HOURS * 60 * 60 * 1000)
  const cache: CachedScanResult = {
    timestamp: now.toISOString(),
    expiry: expiry.toISOString(),
    subnets,
    devices,
    summary,
  }
  localStorage.setItem(SCAN_CACHE_KEY, JSON.stringify(cache))
}

export function loadScanResultFromCache(): CachedScanResult | null {
  try {
    const cached = localStorage.getItem(SCAN_CACHE_KEY)
    if (!cached) return null

    const data: CachedScanResult = JSON.parse(cached)
    const now = new Date()
    const expiry = new Date(data.expiry)

    // 期限切れチェック
    if (now > expiry) {
      localStorage.removeItem(SCAN_CACHE_KEY)
      return null
    }

    return data
  } catch {
    localStorage.removeItem(SCAN_CACHE_KEY)
    return null
  }
}

export function clearScanResultCache(): void {
  localStorage.removeItem(SCAN_CACHE_KEY)
}
