/**
 * Unit tests for CameraTile helper functions
 *
 * Related design document: Layout＆AIlog_Settings/UI_Review1.md
 * Test plan: UT-T01 ~ UT-T11
 * Issue: #97 (LAS-01)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  getModelDisplayName,
  getLocationDisplay,
  getCameraStatus,
  getStatusBadgeClass,
  getStatusBadgeTitle,
} from './cameraTileHelpers'
import type { Camera, CameraFamily } from '@/types/api'

// Helper to create a minimal Camera object for testing
function createMockCamera(overrides: Partial<Camera> = {}): Camera {
  return {
    camera_id: 'test-camera-1',
    name: 'Test Camera',
    location: '',
    floor: null,
    rid: null,
    rtsp_main: null,
    rtsp_sub: null,
    rtsp_username: null,
    rtsp_password: null,
    snapshot_url: null,
    family: 'unknown' as CameraFamily,
    manufacturer: null,
    model: null,
    ip_address: '192.168.1.100',
    mac_address: null,
    lacis_id: null,
    cic: null,
    enabled: true,
    polling_enabled: true,
    polling_interval_sec: 30,
    suggest_policy_weight: 1.0,
    camera_context: null,
    rotation: 0,
    fit_mode: 'trim',
    fid: null,
    tid: null,
    sort_order: 0,
    preset_id: null,
    preset_version: null,
    ai_enabled: false,
    ai_interval_sec: 60,
    conf_override: null,
    nms_threshold: null,
    par_threshold: null,
    serial_number: null,
    hardware_id: null,
    firmware_version: null,
    onvif_endpoint: null,
    rtsp_port: null,
    http_port: null,
    onvif_port: null,
    resolution_main: null,
    codec_main: null,
    fps_main: null,
    bitrate_main: null,
    resolution_sub: null,
    codec_sub: null,
    fps_sub: null,
    bitrate_sub: null,
    ptz_supported: false,
    ptz_continuous: false,
    ptz_absolute: false,
    ptz_relative: false,
    ptz_pan_range: null,
    ptz_tilt_range: null,
    ptz_zoom_range: null,
    ptz_presets: null,
    ptz_home_supported: false,
    audio_input_supported: false,
    audio_output_supported: false,
    audio_codec: null,
    onvif_profiles: null,
    onvif_scopes: null,
    onvif_network_interfaces: null,
    onvif_capabilities: null,
    discovery_method: null,
    last_verified_at: null,
    last_rescan_at: null,
    deleted_at: null,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    ...overrides,
  }
}

describe('getModelDisplayName', () => {
  // UT-T01: model設定済み → modelの値を返す
  it('UT-T01: model設定済みの場合、modelの値を返す', () => {
    const camera = createMockCamera({ model: 'Tapo C100' })
    expect(getModelDisplayName(camera)).toBe('Tapo C100')
  })

  // UT-T02: model未設定, manufacturer設定 → manufacturerを返す
  it('UT-T02: model未設定, manufacturer設定の場合、manufacturerを返す', () => {
    const camera = createMockCamera({
      model: null,
      manufacturer: 'TP-Link',
    })
    expect(getModelDisplayName(camera)).toBe('TP-Link')
  })

  // UT-T03: model/manufacturer未設定, family="tapo" → "Tapo"を返す
  it('UT-T03: model/manufacturer未設定, family="tapo"の場合、"Tapo"を返す', () => {
    const camera = createMockCamera({
      model: null,
      manufacturer: null,
      family: 'tapo' as CameraFamily,
    })
    expect(getModelDisplayName(camera)).toBe('Tapo')
  })

  // UT-T04: 全未設定 → "Unknown"を返す
  it('UT-T04: 全未設定の場合、"Unknown"を返す', () => {
    const camera = createMockCamera({
      model: null,
      manufacturer: null,
      family: 'unknown' as CameraFamily,
    })
    expect(getModelDisplayName(camera)).toBe('Unknown')
  })

  it('空文字のmodelは無視してmanufacturerを使用', () => {
    const camera = createMockCamera({
      model: '  ',
      manufacturer: 'Hikvision',
    })
    expect(getModelDisplayName(camera)).toBe('Hikvision')
  })

  it('family="other"はUnknownにフォールバック', () => {
    const camera = createMockCamera({
      model: null,
      manufacturer: null,
      family: 'other' as CameraFamily,
    })
    expect(getModelDisplayName(camera)).toBe('Unknown')
  })
})

describe('getLocationDisplay', () => {
  // UT-T05: location+fid設定 → "{location} ({fid})"形式
  it('UT-T05: location+fid設定の場合、"{location} ({fid})"形式を返す', () => {
    const camera = createMockCamera({
      location: 'HALE京都丹波口',
      fid: '0150',
    })
    expect(getLocationDisplay(camera)).toBe('HALE京都丹波口 (0150)')
  })

  // UT-T06: locationのみ → locationのみ
  it('UT-T06: locationのみ設定の場合、locationのみを返す', () => {
    const camera = createMockCamera({
      location: 'HALE京都丹波口',
      fid: null,
    })
    expect(getLocationDisplay(camera)).toBe('HALE京都丹波口')
  })

  // UT-T07: fidのみ → "FID:{fid}"形式
  it('UT-T07: fidのみ設定の場合、"FID:{fid}"形式を返す', () => {
    const camera = createMockCamera({
      location: '',
      fid: '0150',
    })
    expect(getLocationDisplay(camera)).toBe('FID:0150')
  })

  // UT-T08: 両方未設定 → "-"
  it('UT-T08: 両方未設定の場合、"-"を返す', () => {
    const camera = createMockCamera({
      location: '',
      fid: null,
    })
    expect(getLocationDisplay(camera)).toBe('-')
  })

  it('空白のみのlocationは未設定扱い', () => {
    const camera = createMockCamera({
      location: '   ',
      fid: '0150',
    })
    expect(getLocationDisplay(camera)).toBe('FID:0150')
  })
})

describe('getCameraStatus', () => {
  beforeEach(() => {
    // Mock Date.now to return a fixed time
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2024-01-01T12:00:00Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  // UT-T09: enabled=true, last_verified_at=1分前 → "online"
  it('UT-T09: enabled=true, last_verified_at=1分前の場合、"online"を返す', () => {
    const camera = createMockCamera({
      enabled: true,
      last_verified_at: '2024-01-01T11:59:00Z', // 1 minute ago
    })
    expect(getCameraStatus(camera)).toBe('online')
  })

  // UT-T10: enabled=true, last_verified_at=10分前 → "offline"
  it('UT-T10: enabled=true, last_verified_at=10分前の場合、"offline"を返す', () => {
    const camera = createMockCamera({
      enabled: true,
      last_verified_at: '2024-01-01T11:50:00Z', // 10 minutes ago
    })
    expect(getCameraStatus(camera)).toBe('offline')
  })

  // UT-T11: enabled=false → "disabled"
  it('UT-T11: enabled=falseの場合、"disabled"を返す', () => {
    const camera = createMockCamera({
      enabled: false,
      last_verified_at: '2024-01-01T11:59:00Z', // recent but disabled
    })
    expect(getCameraStatus(camera)).toBe('disabled')
  })

  it('last_verified_at=nullの場合、"offline"を返す', () => {
    const camera = createMockCamera({
      enabled: true,
      last_verified_at: null,
    })
    expect(getCameraStatus(camera)).toBe('offline')
  })

  it('last_verified_at=ちょうど5分前は"online"', () => {
    const camera = createMockCamera({
      enabled: true,
      last_verified_at: '2024-01-01T11:55:00Z', // exactly 5 minutes ago
    })
    expect(getCameraStatus(camera)).toBe('online')
  })

  it('last_verified_at=5分1秒前は"offline"', () => {
    const camera = createMockCamera({
      enabled: true,
      last_verified_at: '2024-01-01T11:54:59Z', // 5 minutes and 1 second ago
    })
    expect(getCameraStatus(camera)).toBe('offline')
  })
})

describe('getStatusBadgeClass', () => {
  it('onlineの場合、緑色のクラスを返す', () => {
    expect(getStatusBadgeClass('online')).toBe('bg-green-500')
  })

  it('offlineの場合、赤色のクラスを返す', () => {
    expect(getStatusBadgeClass('offline')).toBe('bg-red-500')
  })

  it('disabledの場合、グレーのクラスを返す', () => {
    expect(getStatusBadgeClass('disabled')).toBe('bg-gray-400')
  })
})

describe('getStatusBadgeTitle', () => {
  it('onlineの場合、"オンライン"を返す', () => {
    expect(getStatusBadgeTitle('online')).toBe('オンライン')
  })

  it('offlineの場合、"オフライン"を返す', () => {
    expect(getStatusBadgeTitle('offline')).toBe('オフライン')
  })

  it('disabledの場合、"無効"を返す', () => {
    expect(getStatusBadgeTitle('disabled')).toBe('無効')
  })
})
