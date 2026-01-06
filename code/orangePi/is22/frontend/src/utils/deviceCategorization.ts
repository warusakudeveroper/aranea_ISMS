import type {
  CategorizedDevice,
  DeviceCategory,
  ScannedDevice,
} from "@/types/api"

// credential_status ベースのカテゴリ判定（is22_ScanModal_Credential_Trial_Spec.md Section 4）
export function categorizeDevice(
  device: ScannedDevice,
  registeredIPs: Set<string>
): DeviceCategory {
  // a: 登録済み - camerasテーブルに存在
  if (registeredIPs.has(device.ip)) {
    return "a"
  }

  const detection = device.detection
  if (!detection) {
    return "e" // 検出情報なし → 非カメラ
  }

  const { device_type, onvif_status, rtsp_status } = detection
  const credentialStatus = device.credential_status

  // カメラ確認済み（ONVIF/RTSP応答あり）の場合
  if (device_type === "camera_confirmed") {
    // credential_status='success' → b: 登録可能（モデル名取得済み）
    if (credentialStatus === "success") {
      return "b"
    }
    // credential_status='failed' → c: 認証情報確認必要（全クレデンシャル不一致）
    if (credentialStatus === "failed") {
      return "c"
    }
    // credential_status='not_tried' の場合、プロトコル応答で判定
    if (onvif_status === "success" || rtsp_status === "success") {
      return "b"
    }
    if (
      onvif_status === "auth_required" ||
      onvif_status === "auth_failed" ||
      rtsp_status === "auth_required" ||
      rtsp_status === "auth_failed"
    ) {
      return "c"
    }
    // クレデンシャル試行なし＋認証問題なし → 登録可能とみなす
    return "b"
  }

  // カメラ可能性あり（Tapo系以外も含む）→ d: その他カメラ/汎用RTSP
  if (
    device_type === "camera_likely" ||
    device_type === "camera_possible" ||
    device_type === "nvr_likely"
  ) {
    return "d"
  }

  // それ以外 → e: 非カメラ
  return "e"
}

// デバイスをカテゴリ化してソート
export function categorizeAndSortDevices(
  devices: ScannedDevice[],
  registeredIPs: Set<string>
): CategorizedDevice[] {
  const categorized = devices.map((device) => ({
    ...device,
    category: categorizeDevice(device, registeredIPs),
    isRegistered: registeredIPs.has(device.ip),
  }))

  // ソート: カテゴリ順 → サブネット順 → IP順
  const categoryOrder: Record<DeviceCategory, number> = { a: 0, b: 1, c: 2, d: 3, e: 4 }

  return categorized.sort((a, b) => {
    // カテゴリ順
    const catDiff = categoryOrder[a.category] - categoryOrder[b.category]
    if (catDiff !== 0) return catDiff

    // サブネット順
    const subnetDiff = (a.subnet || "").localeCompare(b.subnet || "")
    if (subnetDiff !== 0) return subnetDiff

    // IP順
    const ipToNum = (ip: string) =>
      ip.split(".").reduce((acc, oct) => acc * 256 + parseInt(oct, 10), 0)
    return ipToNum(a.ip) - ipToNum(b.ip)
  })
}
