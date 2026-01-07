import { describe, it, expect } from 'vitest'
import { DEVICE_CATEGORIES } from './api'
import type {
  DeviceCategory,
  DeviceCategoryDetail,
  CameraStatus,
  ScanStage,
  TriedCredentialResult,
} from './api'

describe('DEVICE_CATEGORIES', () => {
  it('6つのカテゴリが定義されている', () => {
    expect(DEVICE_CATEGORIES).toHaveLength(6)
  })

  it('カテゴリ a (登録済み) が含まれる', () => {
    const categoryA = DEVICE_CATEGORIES.find(c => c.id === 'a')
    expect(categoryA).toBeDefined()
    expect(categoryA?.label).toBe('登録済み')
    expect(categoryA?.collapsed).toBe(false)
  })

  it('カテゴリ b (登録可能) が含まれる', () => {
    const categoryB = DEVICE_CATEGORIES.find(c => c.id === 'b')
    expect(categoryB).toBeDefined()
    expect(categoryB?.label).toBe('登録可能')
    expect(categoryB?.collapsed).toBe(false)
  })

  it('カテゴリ c (認証必要) が含まれる', () => {
    const categoryC = DEVICE_CATEGORIES.find(c => c.id === 'c')
    expect(categoryC).toBeDefined()
    expect(categoryC?.label).toBe('認証必要')
    expect(categoryC?.collapsed).toBe(false)
  })

  it('カテゴリ d (その他カメラ) が含まれる - デフォルト折りたたみ', () => {
    const categoryD = DEVICE_CATEGORIES.find(c => c.id === 'd')
    expect(categoryD).toBeDefined()
    expect(categoryD?.label).toBe('その他カメラ')
    expect(categoryD?.collapsed).toBe(true)
  })

  it('カテゴリ e (非カメラ) が含まれる - デフォルト折りたたみ', () => {
    const categoryE = DEVICE_CATEGORIES.find(c => c.id === 'e')
    expect(categoryE).toBeDefined()
    expect(categoryE?.label).toBe('非カメラ')
    expect(categoryE?.collapsed).toBe(true)
  })

  it('カテゴリ f (通信断・迷子) が含まれる - T3-8 新規追加', () => {
    const categoryF = DEVICE_CATEGORIES.find(c => c.id === 'f')
    expect(categoryF).toBeDefined()
    expect(categoryF?.label).toBe('通信断・迷子')
    expect(categoryF?.description).toContain('IP変更検出')
    expect(categoryF?.bgClass).toContain('bg-red')
    expect(categoryF?.collapsed).toBe(false) // 重要なので折りたたまない
  })
})

describe('型定義の確認', () => {
  it('DeviceCategory に f が含まれる', () => {
    // 型チェックのみ - コンパイルが通れば OK
    const categories: DeviceCategory[] = ['a', 'b', 'c', 'd', 'e', 'f']
    expect(categories).toHaveLength(6)
  })

  it('DeviceCategoryDetail に LostConnection と StrayChild が含まれる', () => {
    // 型チェックのみ - コンパイルが通れば OK
    const details: DeviceCategoryDetail[] = [
      'RegisteredAuthenticated',
      'RegisteredAuthIssue',
      'Registrable',
      'AuthRequired',
      'PossibleCamera',
      'NetworkEquipment',
      'IoTDevice',
      'UnknownDevice',
      'NonCamera',
      'LostConnection',
      'StrayChild',
    ]
    expect(details).toHaveLength(11)
  })

  it('CameraStatus に pending_auth が含まれる - T3-9', () => {
    const statuses: CameraStatus[] = ['active', 'inactive', 'pending_auth', 'maintenance']
    expect(statuses).toContain('pending_auth')
  })

  it('ScanStage が6つ定義されている - T3-7', () => {
    const stages: ScanStage[] = [
      'HostDiscovery',
      'PortScan',
      'OuiLookup',
      'OnvifProbe',
      'RtspAuth',
      'CameraMatching',
    ]
    expect(stages).toHaveLength(6)
  })

  it('TriedCredentialResult が平文パスワードを含む - T3-10', () => {
    // 設計書 High #1 に準拠: マスクなし表示
    const cred: TriedCredentialResult = {
      username: 'admin',
      password: 'password123',  // 平文
      result: 'failed',
    }
    expect(cred.password).toBe('password123')
    expect(cred.result).toBe('failed')
  })
})
