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

  // RT-09: ユーザーフレンドリーな文言に改善
  it('カテゴリ a (登録済みカメラ) が含まれる', () => {
    const categoryA = DEVICE_CATEGORIES.find(c => c.id === 'a')
    expect(categoryA).toBeDefined()
    expect(categoryA?.label).toBe('登録済みカメラ')
    expect(categoryA?.collapsed).toBe(false)
  })

  it('カテゴリ b (新規登録可能) が含まれる', () => {
    const categoryB = DEVICE_CATEGORIES.find(c => c.id === 'b')
    expect(categoryB).toBeDefined()
    expect(categoryB?.label).toBe('新規登録可能')
    expect(categoryB?.collapsed).toBe(false)
  })

  it('カテゴリ c (認証情報の設定が必要) が含まれる', () => {
    const categoryC = DEVICE_CATEGORIES.find(c => c.id === 'c')
    expect(categoryC).toBeDefined()
    expect(categoryC?.label).toBe('認証情報の設定が必要')
    expect(categoryC?.collapsed).toBe(false)
  })

  it('カテゴリ d (ネットワーク機器) が含まれる - デフォルト折りたたみ', () => {
    const categoryD = DEVICE_CATEGORIES.find(c => c.id === 'd')
    expect(categoryD).toBeDefined()
    expect(categoryD?.label).toBe('ネットワーク機器')
    expect(categoryD?.collapsed).toBe(true)
  })

  it('カテゴリ e (その他のデバイス) が含まれる - デフォルト折りたたみ', () => {
    const categoryE = DEVICE_CATEGORIES.find(c => c.id === 'e')
    expect(categoryE).toBeDefined()
    expect(categoryE?.label).toBe('その他のデバイス')
    expect(categoryE?.collapsed).toBe(true)
  })

  it('カテゴリ f (要確認: 通信できません) が含まれる - T3-8 新規追加', () => {
    const categoryF = DEVICE_CATEGORIES.find(c => c.id === 'f')
    expect(categoryF).toBeDefined()
    expect(categoryF?.label).toBe('要確認: 通信できません')
    expect(categoryF?.description).toContain('通信できません')
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

  it('DeviceCategoryDetail に lost_connection と stray_child が含まれる', () => {
    // 型チェックのみ - コンパイルが通れば OK
    // Backend uses snake_case for DeviceCategoryDetail
    const details: DeviceCategoryDetail[] = [
      'registered',
      'registrable',
      'auth_required',
      'auth_failed',
      'possible_camera',
      'network_equipment',
      'io_t_device',
      'unknown_device',
      'non_camera',
      'lost_connection',
      'stray_child',
      'unknown',
    ]
    expect(details).toHaveLength(12)
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
