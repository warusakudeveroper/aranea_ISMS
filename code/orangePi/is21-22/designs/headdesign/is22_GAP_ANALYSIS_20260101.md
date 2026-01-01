# IS22 ScanModal ギャップ分析レポート

**作成日**: 2026-01-01
**対象**: ScanModal.tsx デバイス表示機能
**調査方式**: API応答 vs Frontend型定義 vs 仕様書の3点比較

---

## 1. 調査概要

### 1.1 調査目的
スキャン結果でデバイス情報（IP/MAC等）が表示されない問題の根本原因特定

### 1.2 調査対象
- **仕様書**: `is22_Camserver実装指示.md`, `is22_Camserver仕様案(基本設計書草案改訂版).md`
- **API応答**: `GET /api/ipcamscan/devices`
- **Frontend型**: `src/types/api.ts` の `ScannedDevice`
- **UIコンポーネント**: `src/components/ScanModal.tsx`

---

## 2. API応答構造（実測値）

### 2.1 レスポンス形式
```json
{
  "devices": [...]
}
```
**注意**: `{ data: [...] }` ではなく `{ devices: [...] }` を返す

### 2.2 デバイスオブジェクト構造（実測）
```json
{
  "confidence": 90,
  "detection": {
    "camera_ports": [554, 2020, 443, 8443],
    "device_type": "camera_confirmed",
    "onvif_status": "success",
    "oui_match": "TP-LINK",
    "rtsp_status": "success",
    "suggested_action": "none",
    "user_message": "カメラ確認済み (ONVIF/RTSP応答あり)"
  },
  "device_id": "a650e553-42a3-486e-8173-88607bf27f67",
  "family": "tapo",
  "firmware": null,
  "first_seen": "2025-12-31T11:12:53.617Z",
  "hostnames": [],
  "ip": "192.168.125.45",
  "last_seen": "2025-12-31T17:02:08.786Z",
  "mac": "9C:53:22:0A:E7:6C",
  "manufacturer": null,
  "model": null,
  "open_ports": [
    {"port": 554, "status": "open"},
    {"port": 2020, "status": "open"}
  ],
  "oui_vendor": "TP-LINK",
  "rtsp_uri": null,
  "score": 145,
  "status": "discovered",
  "subnet": "192.168.125.0/24",
  "verified": false
}
```

---

## 3. フィールド名不整合（Critical）

| 項目 | API応答 | Frontend型 | UI使用 | 状態 |
|------|---------|------------|--------|------|
| IPアドレス | `ip` | `ip_address` | `device.ip_address` | **不整合** |
| MACアドレス | `mac` | `mac_address` | 未使用 | **不整合** |
| ポート情報 | `open_ports` | `rtsp_ports` | 未使用 | **不整合** |
| 発見日時 | `first_seen` | `discovered_at` | 未使用 | **不整合** |
| サブネット | `subnet` | 未定義 | 未表示 | **欠落** |
| OUIベンダー | `oui_vendor` | 未定義 | 未表示 | **欠落** |
| スコア | `score` | 未定義 | 未表示 | **欠落** |
| device_id | `device_id` | 未定義 | 未使用 | **欠落** |

---

## 4. UI表示問題

### 4.1 DeviceCard (ScanModal.tsx:209-267)

**現状のコード（抜粋）**:
```tsx
// Line 233-235
<span className="font-mono text-sm font-medium">
  {device.ip_address}  // ← API returns 'ip', NOT 'ip_address'
</span>
```

**問題**:
- `device.ip_address` は常に `undefined`（APIは `device.ip` を返す）
- MACアドレス表示なし
- サブネット情報表示なし
- 施設名(fid)表示なし

### 4.2 選択処理 (ScanModal.tsx:484-507)

**問題のあるコード**:
```tsx
// Line 502 - selectAll
setSelectedIds(new Set(cameraDevices.map((d) => d.ip_address)))
// ↑ d.ip_address は undefined

// Line 484-493 - toggleDevice
const toggleDevice = (ip: string) => {
  // ip は device.ip_address から渡されるが undefined
}

// Line 774-775 - DeviceCard key and toggle
<DeviceCard
  key={device.ip_address}  // undefined
  onToggle={() => toggleDevice(device.ip_address)}  // undefined
/>
```

---

## 5. サブネット情報の不整合

### 5.1 仕様書記載（is22_Camserver実装指示.md）
| fid | 施設名 | CIDR |
|-----|--------|------|
| 9000 | 市山水産(牛深) | 192.168.96.0/**23** |
| 0150 | HALE京都丹波口ホテル | 192.168.125.0/24 |
| 0151 | HALE京都東寺ホテル | 192.168.126.0/24 |

### 5.2 現在のDB状態（GET /api/subnets）
| cidr | fid | description |
|------|-----|-------------|
| 192.168.125.0/24 | **9000** | Main facility network |
| 192.168.126.0/24 | **9000** | Branch office network |
| 192.168.96.0/23 | **0000** | Added from scan modal |

**問題**: 全fid値が仕様と不一致

### 5.3 サブネット削除機能
- **仕様**: 不明（要確認）
- **実装**: なし
- **UI**: 削除ボタンなし

---

## 6. 型定義の修正案

### 6.1 ScannedDevice 修正案（api.ts）

```typescript
export interface ScannedDevice {
  // --- API応答と一致させる ---
  device_id: string;
  ip: string;                    // 変更: ip_address → ip
  mac: string | null;            // 変更: mac_address → mac
  family: 'Tapo' | 'Vigi' | 'Nest' | 'Other' | 'Unknown';
  manufacturer: string | null;
  model: string | null;
  firmware: string | null;
  oui_vendor: string | null;     // 追加
  hostnames: string[];           // 追加
  open_ports: { port: number; status: string }[];  // 変更
  rtsp_uri: string | null;       // 追加
  verified: boolean;
  status: 'discovered' | 'verifying' | 'verified' | 'rejected' | 'approved';
  first_seen: string;            // 変更: discovered_at → first_seen
  last_seen: string;             // 追加
  subnet: string;                // 追加
  score: number;                 // 追加
  confidence: number;            // 追加
  detection: DetectionReason | null;
}
```

---

## 7. UI修正案

### 7.1 DeviceCard 表示改善

表示すべき情報:
1. **IPアドレス** - `device.ip` （必須）
2. **MACアドレス** - `device.mac` （必須、識別用）
3. **OUIベンダー** - `device.oui_vendor` （カメラメーカー識別）
4. **サブネット** - `device.subnet` （施設識別）
5. **施設名** - fid→施設名マッピング（要実装）

### 7.2 サブネット管理改善

追加すべき機能:
1. サブネット削除ボタン（各行に設置）
2. fid編集機能
3. 施設名(description)編集機能
4. 仕様準拠のデフォルト値設定

---

## 8. 修正優先度

### P0 (Blocker) - 即時対応必要
1. フィールド名不整合（ip_address → ip, mac_address → mac）
2. 選択処理の修正（ip_address → ip）

### P1 (High) - 本番運用に必要
3. MAC/サブネット情報の表示追加
4. サブネット削除機能
5. fid/施設名の正規化

### P2 (Medium) - UX改善
6. OUIベンダー表示
7. スコア表示
8. ソート機能

---

## 9. MECE確認

本レポートは以下を網羅:
- API応答構造 ✓
- Frontend型定義 ✓
- UI実装 ✓
- 仕様書との整合性 ✓
- サブネット情報 ✓

網羅していない項目:
- Backend Rustコードとの整合性（別途調査必要）
- 他コンポーネントへの影響（CameraGrid等）

---

## 10. 結論

**根本原因**: API応答フィールド名（`ip`, `mac`）とFrontend型定義（`ip_address`, `mac_address`）の不一致

**修正方針**:
1. Frontend型定義をAPI応答に合わせて修正
2. ScanModal.tsxの参照箇所を全て修正
3. サブネットfid値を仕様通りに修正

**アンアンビギュアス宣言**: 本ギャップ分析は実測データに基づき、不整合箇所を具体的なファイル名・行番号で特定済み。
