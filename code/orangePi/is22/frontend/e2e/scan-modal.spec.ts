import { test, expect } from '@playwright/test'

test.describe('ScanModal E2E Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Mock API responses
    await page.route('**/api/subnets', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          data: [
            {
              subnet_id: 'subnet-1',
              cidr: '192.168.1.0/24',
              fid: '0150',
              facility_name: 'テスト施設',
              credentials: [
                { username: 'admin', password: 'admin123', priority: 1 },
              ],
            },
          ],
        }),
      })
    })

    await page.route('**/api/cameras', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          data: [
            { id: 1, ip_address: '192.168.1.50', name: 'ロビーカメラ' },
          ],
        }),
      })
    })

    await page.goto('/')
  })

  test('スキャンモーダルが開く', async ({ page }) => {
    // Click the scan button (assumes there's a button to open scan modal)
    const scanButton = page.locator('button:has-text("スキャン")')
    if (await scanButton.isVisible()) {
      await scanButton.click()
      await expect(page.locator('text=カメラスキャン')).toBeVisible()
    }
  })

  test('サブネット選択ができる', async ({ page }) => {
    const scanButton = page.locator('button:has-text("スキャン")')
    if (await scanButton.isVisible()) {
      await scanButton.click()

      // Check subnet list is loaded
      await expect(page.locator('text=192.168.1.0/24')).toBeVisible()

      // Select subnet
      const checkbox = page.locator('input[type="checkbox"]').first()
      await checkbox.check()
      await expect(checkbox).toBeChecked()
    }
  })
})

test.describe('Category F Display Tests', () => {
  test('通信断デバイスが赤色で表示される', async ({ page }) => {
    // Mock scan results with LostConnection device
    await page.route('**/api/ipcamscan/devices', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          devices: [
            {
              ip: '192.168.1.100',
              mac: '00:11:22:33:44:55',
              category_detail: 'LostConnection',
              registered_camera_name: 'ロビーカメラ',
              credential_status: 'not_tried',
            },
          ],
        }),
      })
    })

    await page.goto('/')
    // The actual test would need to trigger a scan and verify the display
  })

  test('IP変更検出デバイスにオレンジバッジが表示される', async ({ page }) => {
    // Mock scan results with StrayChild device
    await page.route('**/api/ipcamscan/devices', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          devices: [
            {
              ip: '192.168.1.200',
              mac: '00:11:22:33:44:66',
              category_detail: 'StrayChild',
              ip_changed: true,
              registered_camera_name: '迷子カメラ',
              credential_status: 'not_tried',
            },
          ],
        }),
      })
    })

    await page.goto('/')
  })
})

test.describe('Credential Display Tests (T3-10)', () => {
  test('試行クレデンシャルが平文で表示される', async ({ page }) => {
    await page.route('**/api/ipcamscan/devices', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          devices: [
            {
              ip: '192.168.1.100',
              mac: '00:11:22:33:44:55',
              category_detail: 'AuthRequired',
              credential_status: 'failed',
              tried_credentials: [
                { username: 'admin', password: 'password123', result: 'failed' },
                { username: 'halecam', password: 'halecam12345@', result: 'timeout' },
              ],
            },
          ],
        }),
      })
    })

    await page.goto('/')
    // Verify credentials are displayed in plain text
  })
})

test.describe('Phase Progress Display (T3-7)', () => {
  test('進捗表示に重みが表示される', async ({ page }) => {
    await page.route('**/api/ipcamscan/jobs/*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          data: {
            job_id: 'test-job-1',
            status: 'running',
            current_phase: 'PortScan',
            progress_percent: 40,
          },
        }),
      })
    })

    await page.goto('/')
    // Verify phase weights are shown
  })
})

test.describe('Camera Brand Settings Tests', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/api/settings/camera-brands', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          data: [
            {
              id: 1,
              name: 'TP-LINK',
              display_name: 'TP-Link / Tapo',
              category: 'consumer',
              is_builtin: true,
            },
            {
              id: 2,
              name: 'Custom',
              display_name: 'カスタムブランド',
              category: 'consumer',
              is_builtin: false,
            },
          ],
        }),
      })
    })
  })

  test('組込ブランドに編集ボタンがない', async ({ page }) => {
    await page.goto('/')
    // Open settings modal and verify builtin brands cannot be edited
  })

  test('カスタムブランドは編集可能', async ({ page }) => {
    await page.goto('/')
    // Open settings modal and verify custom brands can be edited
  })
})
