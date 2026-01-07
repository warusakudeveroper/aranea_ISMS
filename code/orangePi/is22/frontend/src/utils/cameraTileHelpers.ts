/**
 * CameraTile helper functions for display logic
 *
 * Related design document: Layout＆AIlog_Settings/UI_Review1.md
 * Issue: #97 (LAS-01)
 */

import type { Camera } from '@/types/api'

/**
 * IMPL-T01: Get display name with fallback (model → manufacturer → family → Unknown)
 */
export function getModelDisplayName(camera: Camera): string {
  if (camera.model && camera.model.trim() !== '') {
    return camera.model
  }
  if (camera.manufacturer && camera.manufacturer.trim() !== '') {
    return camera.manufacturer
  }
  if (camera.family && camera.family !== 'unknown' && camera.family !== 'other') {
    // Capitalize first letter: "tapo" -> "Tapo"
    return camera.family.charAt(0).toUpperCase() + camera.family.slice(1)
  }
  return 'Unknown'
}

/**
 * IMPL-T02: Get location display with fid (e.g., "HALE京都丹波口 (0150)")
 */
export function getLocationDisplay(camera: Camera): string {
  const location = camera.location?.trim() || ''
  const fid = camera.fid?.trim() || ''

  if (location && fid) {
    return `${location} (${fid})`
  }
  if (location) {
    return location
  }
  if (fid) {
    return `FID:${fid}`
  }
  return '-'
}

/**
 * IMPL-T03: Camera status type
 */
export type CameraStatusType = 'online' | 'offline' | 'disabled'

/**
 * IMPL-T03: Get camera status based on enabled flag and last_verified_at
 *
 * Status rules:
 * - disabled: enabled === false
 * - offline: enabled && (last_verified_at is null OR last_verified_at > 5 minutes ago)
 * - online: enabled && last_verified_at within 5 minutes
 */
export function getCameraStatus(camera: Camera): CameraStatusType {
  if (!camera.enabled) {
    return 'disabled'
  }

  if (!camera.last_verified_at) {
    return 'offline'
  }

  const lastVerified = new Date(camera.last_verified_at).getTime()
  const now = Date.now()
  const fiveMinutesMs = 5 * 60 * 1000

  if (now - lastVerified <= fiveMinutesMs) {
    return 'online'
  }

  return 'offline'
}

/**
 * IMPL-T03: Get CSS class for status badge color
 */
export function getStatusBadgeClass(status: CameraStatusType): string {
  switch (status) {
    case 'online':
      return 'bg-green-500'
    case 'offline':
      return 'bg-red-500'
    case 'disabled':
      return 'bg-gray-400'
  }
}

/**
 * IMPL-T03: Get tooltip title for status badge
 */
export function getStatusBadgeTitle(status: CameraStatusType): string {
  switch (status) {
    case 'online':
      return 'オンライン'
    case 'offline':
      return 'オフライン'
    case 'disabled':
      return '無効'
  }
}
