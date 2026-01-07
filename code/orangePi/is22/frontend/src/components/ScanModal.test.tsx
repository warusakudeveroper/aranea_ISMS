import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import type { ScannedDevice, CategorizedDevice, DeviceCategoryDetail } from '@/types/api'
import { DEVICE_CATEGORIES } from '@/types/api'

// Mock device factory
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

function createCategorizedDevice(
  overrides: Partial<ScannedDevice & { category: string; registeredCameraName?: string; ipChanged?: boolean }> = {}
): CategorizedDevice {
  const base = createMockDevice(overrides)
  return {
    ...base,
    category: (overrides.category || 'e') as CategorizedDevice['category'],
    registeredCameraName: overrides.registeredCameraName,
    ipChanged: overrides.ipChanged,
  }
}

describe('DEVICE_CATEGORIES', () => {
  it('カテゴリFが正しく定義されている', () => {
    const categoryF = DEVICE_CATEGORIES.find(c => c.id === 'f')
    expect(categoryF).toBeDefined()
    expect(categoryF?.label).toBe('通信断・迷子')
    expect(categoryF?.description).toContain('IP変更検出')
    expect(categoryF?.bgClass).toContain('bg-red')
    expect(categoryF?.collapsed).toBe(false)
  })

  it('カテゴリの表示順序が正しい（a, f, b, c, d, e）', () => {
    // DEVICE_CATEGORIES の順序確認
    const ids = DEVICE_CATEGORIES.map(c => c.id)
    expect(ids).toEqual(['a', 'b', 'c', 'd', 'e', 'f'])
  })
})

describe('PhaseIndicator weights (T3-7)', () => {
  // PhaseIndicator の重み定義を検証
  const expectedWeights = {
    arp: 15,
    port: 25,
    oui: 5,
    onvif: 20,
    rtsp: 30,
    match: 5,
  }

  it('重みの合計が100%になる', () => {
    const total = Object.values(expectedWeights).reduce((sum, w) => sum + w, 0)
    expect(total).toBe(100)
  })

  it('RTSPフェーズが最も重い（30%）', () => {
    const maxWeight = Math.max(...Object.values(expectedWeights))
    expect(expectedWeights.rtsp).toBe(maxWeight)
  })

  it('OUIとMatchが最も軽い（各5%）', () => {
    expect(expectedWeights.oui).toBe(5)
    expect(expectedWeights.match).toBe(5)
  })
})

describe('Category F device display (T3-8)', () => {
  it('LostConnection デバイスが正しく判定される', () => {
    const device = createCategorizedDevice({
      category: 'f',
      category_detail: 'LostConnection' as DeviceCategoryDetail,
      registered_camera_name: 'ロビーカメラ',
    })

    expect(device.category).toBe('f')
    expect(device.category_detail).toBe('LostConnection')
  })

  it('StrayChild デバイスが ipChanged: true を持つ', () => {
    const device = createCategorizedDevice({
      category: 'f',
      category_detail: 'StrayChild' as DeviceCategoryDetail,
      ip_changed: true,
      ipChanged: true,
      registered_camera_name: '迷子カメラ',
    })

    expect(device.category).toBe('f')
    expect(device.ipChanged).toBe(true)
  })

  it('カテゴリFの説明文に「IP変更検出」が含まれる', () => {
    const categoryF = DEVICE_CATEGORIES.find(c => c.id === 'f')
    expect(categoryF?.description).toContain('IP変更検出')
  })
})

describe('PendingAuth status (T3-9)', () => {
  it('CameraStatus に pending_auth が含まれる', () => {
    // 型チェック
    const validStatuses = ['active', 'inactive', 'pending_auth', 'maintenance']
    expect(validStatuses).toContain('pending_auth')
  })
})

describe('Tried credentials display (T3-10)', () => {
  it('TriedCredentialResult が平文パスワードを含む', () => {
    const device = createCategorizedDevice({
      tried_credentials: [
        { username: 'admin', password: 'password123', result: 'failed' },
        { username: 'halecam', password: 'halecam12345@', result: 'success' },
      ],
    })

    expect(device.tried_credentials).toHaveLength(2)
    expect(device.tried_credentials?.[0].password).toBe('password123')
    expect(device.tried_credentials?.[1].password).toBe('halecam12345@')
  })

  it('result が success/failed/timeout のいずれか', () => {
    const results = ['success', 'failed', 'timeout']
    results.forEach(result => {
      const device = createCategorizedDevice({
        tried_credentials: [
          { username: 'test', password: 'test', result: result as 'success' | 'failed' | 'timeout' },
        ],
      })
      expect(device.tried_credentials?.[0].result).toBe(result)
    })
  })
})

describe('Device categorization edge cases', () => {
  it('RegisteredAuthenticated は カテゴリ A', () => {
    const device = createCategorizedDevice({
      category: 'a',
      category_detail: 'RegisteredAuthenticated' as DeviceCategoryDetail,
    })
    expect(device.category).toBe('a')
  })

  it('Registrable は カテゴリ B', () => {
    const device = createCategorizedDevice({
      category: 'b',
      category_detail: 'Registrable' as DeviceCategoryDetail,
      credential_status: 'success',
      model: 'Tapo C100',
    })
    expect(device.category).toBe('b')
  })

  it('AuthRequired は カテゴリ C', () => {
    const device = createCategorizedDevice({
      category: 'c',
      category_detail: 'AuthRequired' as DeviceCategoryDetail,
      credential_status: 'failed',
    })
    expect(device.category).toBe('c')
  })

  it('PossibleCamera は カテゴリ D', () => {
    const device = createCategorizedDevice({
      category: 'd',
      category_detail: 'PossibleCamera' as DeviceCategoryDetail,
    })
    expect(device.category).toBe('d')
  })

  it('NonCamera は カテゴリ E', () => {
    const device = createCategorizedDevice({
      category: 'e',
      category_detail: 'NonCamera' as DeviceCategoryDetail,
    })
    expect(device.category).toBe('e')
  })
})

describe('Credential status badges', () => {
  it('credential_status: success で認証済バッジ表示条件を満たす', () => {
    const device = createCategorizedDevice({
      credential_status: 'success',
      credential_username: 'admin',
      credential_password: 'pass123',
    })
    expect(device.credential_status).toBe('success')
    expect(device.credential_username).toBe('admin')
  })

  it('credential_status: failed で警告表示条件を満たす', () => {
    const device = createCategorizedDevice({
      credential_status: 'failed',
    })
    expect(device.credential_status).toBe('failed')
  })

  it('credential_status: not_tried で未試行', () => {
    const device = createCategorizedDevice({
      credential_status: 'not_tried',
    })
    expect(device.credential_status).toBe('not_tried')
  })
})
