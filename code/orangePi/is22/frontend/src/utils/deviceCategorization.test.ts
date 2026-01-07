import { describe, it, expect } from 'vitest'
import { categorizeDevice, categorizeAndSortDevices } from './deviceCategorization'
import type { ScannedDevice } from '@/types/api'

// Helper to create a minimal ScannedDevice for testing
function createMockDevice(overrides: Partial<ScannedDevice> = {}): ScannedDevice {
  return {
    device_id: 'test-device-1',
    ip: '192.168.1.100',
    mac: '00:11:22:33:44:55',
    family: 'unknown',
    manufacturer: null,
    model: null,
    firmware: null,
    oui_vendor: null,
    hostnames: [],
    open_ports: [],
    rtsp_uri: null,
    verified: false,
    status: 'discovered',
    first_seen: '2024-01-01T00:00:00Z',
    last_seen: '2024-01-01T00:00:00Z',
    subnet: '192.168.1.0/24',
    score: 0,
    confidence: 0,
    detection: null,
    credential_status: 'not_tried',
    credential_username: null,
    credential_password: null,
    ...overrides,
  }
}

describe('categorizeDevice', () => {
  describe('カテゴリF（LostConnection/StrayChild）判定', () => {
    it('category_detail が LostConnection の場合、カテゴリ f を返す', () => {
      const device = createMockDevice({
        category_detail: 'LostConnection',
        registered_camera_id: 1,
        registered_camera_name: 'ロビーカメラ',
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('f')
    })

    it('category_detail が StrayChild の場合、カテゴリ f を返す', () => {
      const device = createMockDevice({
        category_detail: 'StrayChild',
        ip_changed: true,
        registered_camera_id: 1,
        registered_camera_name: '迷子カメラ',
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('f')
    })
  })

  describe('カテゴリA（登録済み）判定', () => {
    it('IP が登録済みリストに含まれる場合、カテゴリ a を返す', () => {
      const device = createMockDevice({ ip: '192.168.1.50' })
      const registeredIPs = new Set(['192.168.1.50'])

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('a')
    })

    it('category_detail が RegisteredAuthenticated の場合、カテゴリ a を返す', () => {
      const device = createMockDevice({
        category_detail: 'RegisteredAuthenticated',
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('a')
    })

    it('category_detail が RegisteredAuthIssue の場合、カテゴリ a を返す', () => {
      const device = createMockDevice({
        category_detail: 'RegisteredAuthIssue',
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('a')
    })
  })

  describe('カテゴリB（登録可能）判定', () => {
    it('category_detail が Registrable の場合、カテゴリ b を返す', () => {
      const device = createMockDevice({
        category_detail: 'Registrable',
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('b')
    })

    it('camera_confirmed で credential_status が success の場合、カテゴリ b を返す', () => {
      const device = createMockDevice({
        detection: {
          oui_match: 'TP-LINK',
          camera_ports: [554],
          onvif_status: 'success',
          rtsp_status: 'success',
          device_type: 'camera_confirmed',
          user_message: 'カメラ確認済み',
          suggested_action: 'none',
        },
        credential_status: 'success',
        model: 'Tapo C100',
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('b')
    })
  })

  describe('カテゴリC（認証必要）判定', () => {
    it('category_detail が AuthRequired の場合、カテゴリ c を返す', () => {
      const device = createMockDevice({
        category_detail: 'AuthRequired',
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('c')
    })

    it('camera_confirmed で credential_status が failed の場合、カテゴリ c を返す', () => {
      const device = createMockDevice({
        detection: {
          oui_match: 'TP-LINK',
          camera_ports: [554],
          onvif_status: 'auth_required',
          rtsp_status: 'auth_failed',
          device_type: 'camera_confirmed',
          user_message: '認証失敗',
          suggested_action: 'set_credentials',
        },
        credential_status: 'failed',
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('c')
    })
  })

  describe('カテゴリD（その他カメラ）判定', () => {
    it('category_detail が PossibleCamera の場合、カテゴリ d を返す', () => {
      const device = createMockDevice({
        category_detail: 'PossibleCamera',
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('d')
    })

    it('device_type が camera_likely の場合、カテゴリ d を返す', () => {
      const device = createMockDevice({
        detection: {
          oui_match: 'HIKVISION',
          camera_ports: [80],
          onvif_status: 'not_tested',
          rtsp_status: 'not_tested',
          device_type: 'camera_likely',
          user_message: 'カメラ可能性あり',
          suggested_action: 'manual_check',
        },
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('d')
    })
  })

  describe('カテゴリE（非カメラ）判定', () => {
    it('category_detail が NonCamera の場合、カテゴリ e を返す', () => {
      const device = createMockDevice({
        category_detail: 'NonCamera',
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('e')
    })

    it('detection が null の場合、カテゴリ e を返す', () => {
      const device = createMockDevice({
        detection: null,
      })
      const registeredIPs = new Set<string>()

      const result = categorizeDevice(device, registeredIPs)
      expect(result).toBe('e')
    })
  })
})

describe('categorizeAndSortDevices', () => {
  it('デバイスをカテゴリ順にソートする（a → f → b → c → d → e）', () => {
    const devices: ScannedDevice[] = [
      createMockDevice({ ip: '192.168.1.5', category_detail: 'NonCamera' }),         // e
      createMockDevice({ ip: '192.168.1.2', category_detail: 'Registrable' }),       // b
      createMockDevice({ ip: '192.168.1.4', category_detail: 'PossibleCamera' }),    // d
      createMockDevice({ ip: '192.168.1.3', category_detail: 'AuthRequired' }),      // c
      createMockDevice({ ip: '192.168.1.1', category_detail: 'RegisteredAuthenticated' }), // a
      createMockDevice({ ip: '192.168.1.6', category_detail: 'LostConnection' }),    // f
    ]
    const registeredIPs = new Set<string>()

    const result = categorizeAndSortDevices(devices, registeredIPs)

    expect(result.map(d => d.category)).toEqual(['a', 'f', 'b', 'c', 'd', 'e'])
    expect(result.map(d => d.ip)).toEqual([
      '192.168.1.1', // a
      '192.168.1.6', // f
      '192.168.1.2', // b
      '192.168.1.3', // c
      '192.168.1.4', // d
      '192.168.1.5', // e
    ])
  })

  it('同カテゴリ内ではIP順にソートする', () => {
    const devices: ScannedDevice[] = [
      createMockDevice({ ip: '192.168.1.30', category_detail: 'Registrable' }),
      createMockDevice({ ip: '192.168.1.10', category_detail: 'Registrable' }),
      createMockDevice({ ip: '192.168.1.20', category_detail: 'Registrable' }),
    ]
    const registeredIPs = new Set<string>()

    const result = categorizeAndSortDevices(devices, registeredIPs)

    expect(result.map(d => d.ip)).toEqual([
      '192.168.1.10',
      '192.168.1.20',
      '192.168.1.30',
    ])
  })

  it('CategorizedDevice に registeredCameraName と ipChanged を含める', () => {
    const devices: ScannedDevice[] = [
      createMockDevice({
        ip: '192.168.1.100',
        category_detail: 'LostConnection',
        registered_camera_name: 'ロビーカメラ',
        ip_changed: false,
      }),
      createMockDevice({
        ip: '192.168.1.200',
        category_detail: 'StrayChild',
        registered_camera_name: '迷子カメラ',
        ip_changed: true,
      }),
    ]
    const registeredIPs = new Set<string>()

    const result = categorizeAndSortDevices(devices, registeredIPs)

    expect(result[0].registeredCameraName).toBe('ロビーカメラ')
    expect(result[0].ipChanged).toBe(false)
    expect(result[1].registeredCameraName).toBe('迷子カメラ')
    expect(result[1].ipChanged).toBe(true)
  })
})
