# Aranea Web UI 共通フレームワーク設計

## 概要

全Araneaデバイス（is02, is04, is05, is10, 将来デバイス）で共通利用可能なWeb UI管理インターフェース。
デバイスタイプ別の設定分離と、将来の巡回ポーリング対応を考慮した設計。

---

## 1. アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│                    AraneaWebUI (Base)                    │
│  ┌─────────────────────────────────────────────────────┐│
│  │  Common CSS/JS Framework                            ││
│  │  - Chakra UI inspired styling                       ││
│  │  - Tab navigation system                            ││
│  │  - Toast notifications                              ││
│  │  - Modal dialogs                                    ││
│  └─────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────┐│
│  │  Common API Endpoints                               ││
│  │  - GET /                     → HTML UI              ││
│  │  - GET /?cic=XXXXXX          → JSON Status          ││
│  │  - GET /api/status           → Full device status   ││
│  │  - GET /api/config           → All config           ││
│  │  - POST /api/common/*        → Common settings      ││
│  │  - POST /api/system/*        → System operations    ││
│  └─────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────┐│
│  │  Common Settings Tabs                               ││
│  │  - Status (デバイス状態)                             ││
│  │  - Network (WiFi/AP設定)                            ││
│  │  - Cloud (Aranea連携)                               ││
│  │  - System (再起動/リセット)                          ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Device-Specific Extension                   │
│  ┌─────────────────────────────────────────────────────┐│
│  │  is10: Router Inspector                             ││
│  │  - Routers tab (ルーター設定)                        ││
│  │  - CelestialGlobe tab (SSOT送信設定)                ││
│  │  - SSH Settings (タイムアウト等)                     ││
│  └─────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────┐│
│  │  is02: BLE Gateway                                  ││
│  │  - BLE tab (受信設定)                               ││
│  │  - Relay tab (中継先設定)                           ││
│  └─────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────┐│
│  │  is04: GPIO Controller                              ││
│  │  - GPIO tab (出力設定)                              ││
│  │  - Schedule tab (スケジュール)                       ││
│  └─────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────┐│
│  │  is05: Reed Input                                   ││
│  │  - Inputs tab (入力設定)                            ││
│  │  - Alerts tab (アラート設定)                         ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

---

## 2. タブ構成

### 2.1 共通タブ（全デバイス）

| タブ名 | 内容 | API |
|--------|------|-----|
| **Status** | デバイス状態（リアルタイム更新） | GET /api/status |
| **Network** | WiFi/AP設定 | POST /api/common/wifi, /api/common/ap |
| **Cloud** | Aranea連携設定 | POST /api/common/cloud |
| **Tenant** | テナント認証情報 | POST /api/common/tenant |
| **System** | 再起動/リセット/ログ | POST /api/system/* |

### 2.2 デバイス固有タブ（Type-Specific）

| デバイス | タブ名 | 内容 |
|----------|--------|------|
| is10 | **Routers** | SSH接続先ルーター一覧 |
| is10 | **Inspector** | スキャン間隔、CelestialGlobe設定 |
| is02 | **BLE** | BLEスキャン設定 |
| is02 | **Relay** | 中継先設定 |
| is04 | **GPIO** | 出力ピン設定 |
| is04 | **Schedule** | スケジュール制御 |
| is05 | **Inputs** | リード入力設定 |
| is05 | **Alerts** | アラート閾値設定 |

---

## 3. 設定カテゴリ詳細

### 3.1 Status タブ（読み取り専用情報）

```
┌─────────────────────────────────────────────────┐
│ Device Information                              │
├─────────────────────────────────────────────────┤
│ Type        │ ar-is10                           │
│ LacisID     │ 30101234567890AB0001              │
│ CIC         │ 397815                            │
│ MAC         │ F8:B3:B7:49:6D:EC                 │
│ Hostname    │ ar-is10-496DEC                    │
├─────────────────────────────────────────────────┤
│ Network Status                                  │
├─────────────────────────────────────────────────┤
│ IP Address  │ 192.168.3.91                      │
│ SSID        │ cluster1                          │
│ RSSI        │ -45 dBm                           │
│ Gateway     │ 192.168.3.1                       │
├─────────────────────────────────────────────────┤
│ System Status                                   │
├─────────────────────────────────────────────────┤
│ NTP Time    │ 2025-12-20 20:51:56 JST           │
│ Uptime      │ 2h 34m 12s                        │
│ Free Heap   │ 208,456 bytes                     │
│ Largest Blk │ 110,580 bytes                     │
│ CPU Freq    │ 240 MHz                           │
│ Flash Size  │ 4 MB                              │
├─────────────────────────────────────────────────┤
│ Firmware                                        │
├─────────────────────────────────────────────────┤
│ Version     │ 1.2.1                             │
│ UI Version  │ 1.0.0                             │
│ Build Date  │ 2025-12-20                        │
│ Modules     │ WiFi, NTP, SSH, MQTT, SPIFFS      │
└─────────────────────────────────────────────────┘
```

### 3.2 Network タブ

```
┌─────────────────────────────────────────────────┐
│ WiFi Settings (STA Mode)                        │
├─────────────────────────────────────────────────┤
│ SSID 1-6    │ [cluster1] [********]             │
│             │ Priority順に接続試行              │
├─────────────────────────────────────────────────┤
│ AP Mode Settings                                │
├─────────────────────────────────────────────────┤
│ AP SSID     │ [ar-is10-496DEC] (自動生成)       │
│ AP Password │ [********] (未設定=オープン)      │
│ AP Timeout  │ [300] 秒 (0=無制限)               │
├─────────────────────────────────────────────────┤
│ Hostname                                        │
├─────────────────────────────────────────────────┤
│ Hostname    │ [ar-is10-496DEC] (mDNS対応)       │
│ NTP Server  │ [pool.ntp.org]                    │
│ Timezone    │ [Asia/Tokyo] (+9:00)              │
└─────────────────────────────────────────────────┘
```

### 3.3 Cloud タブ

```
┌─────────────────────────────────────────────────┐
│ AraneaDeviceGate (デバイス登録)                  │
├─────────────────────────────────────────────────┤
│ Gate URL    │ [https://...araneaDeviceGate]     │
│             │ ※初期値はファームにハードコード    │
│ 登録状態    │ ✓ 登録済み (CIC: 397815)          │
├─────────────────────────────────────────────────┤
│ DeviceStateReport (状態報告)                    │
├─────────────────────────────────────────────────┤
│ Report URL  │ [https://...deviceStateReport]    │
│ Interval    │ [60] 秒                           │
│ Last Report │ 2025-12-20 20:51:59 (200 OK)      │
├─────────────────────────────────────────────────┤
│ MQTT (双方向通信)                               │
├─────────────────────────────────────────────────┤
│ Status      │ ○ Connected / × Disconnected     │
│ Endpoint    │ wss://...                         │
│ Schema Ver  │ 3                                 │
├─────────────────────────────────────────────────┤
│ Relay Endpoints (Zero3)                         │
├─────────────────────────────────────────────────┤
│ Primary     │ [http://192.168.96.201:8080/...]  │
│ Secondary   │ [http://192.168.96.202:8080/...]  │
└─────────────────────────────────────────────────┘
```

### 3.4 Tenant タブ

```
┌─────────────────────────────────────────────────┐
│ Tenant Authentication (3要素認証)               │
├─────────────────────────────────────────────────┤
│ Tenant ID   │ [T2025120608261484221]            │
│ Facility ID │ [9000]                            │
├─────────────────────────────────────────────────┤
│ Primary Account (認証用)                        │
├─────────────────────────────────────────────────┤
│ LacisID     │ [12767487939173857894]            │
│ Email       │ [info+ichiyama@neki.tech]         │
│ CIC         │ [263238]                          │
└─────────────────────────────────────────────────┘
```

### 3.5 System タブ

```
┌─────────────────────────────────────────────────┐
│ Security                                        │
├─────────────────────────────────────────────────┤
│ UI Password │ [********] (未設定=認証なし)      │
│             │ Basic認証でUI全体を保護           │
├─────────────────────────────────────────────────┤
│ Scheduled Reboot                                │
├─────────────────────────────────────────────────┤
│ Enabled     │ [✓] 定期再起動                    │
│ Time        │ [03:00] 毎日                      │
├─────────────────────────────────────────────────┤
│ Actions                                         │
├─────────────────────────────────────────────────┤
│ [Reboot Now]  [Factory Reset]  [Export Config]  │
├─────────────────────────────────────────────────┤
│ Logs (最新50件)                                 │
├─────────────────────────────────────────────────┤
│ 20:51:59 [STATE] OK 200                         │
│ 20:51:56 [NTP] Synced                           │
│ 20:51:54 [WIFI] Connected to cluster1           │
└─────────────────────────────────────────────────┘
```

---

## 4. CICパラメータ付きステータスエンドポイント

### 4.1 エンドポイント仕様

```
GET /?cic=397815
GET /api/status?cic=397815
```

### 4.2 レスポンス形式

```json
{
  "ok": true,
  "auth": {
    "method": "cic",
    "valid": true
  },
  "device": {
    "type": "ar-is10",
    "lacisId": "30101234567890AB0001",
    "mac": "F8B3B7496DEC",
    "cic": "397815",
    "hostname": "ar-is10-496DEC"
  },
  "network": {
    "ip": "192.168.3.91",
    "ssid": "cluster1",
    "rssi": -45,
    "gateway": "192.168.3.1",
    "apMode": false
  },
  "system": {
    "ntpTime": "2025-12-20T11:51:56Z",
    "ntpSynced": true,
    "uptime": 9252,
    "uptimeHuman": "2h 34m 12s",
    "heap": 208456,
    "heapLargest": 110580,
    "cpuFreq": 240,
    "flashSize": 4194304
  },
  "firmware": {
    "version": "1.2.1",
    "uiVersion": "1.0.0",
    "buildDate": "2025-12-20",
    "modules": ["WiFi", "NTP", "SSH", "MQTT", "SPIFFS", "OTA"]
  },
  "cloud": {
    "registered": true,
    "mqttConnected": false,
    "lastStateReport": "2025-12-20T11:51:59Z",
    "lastStateReportCode": 200
  },
  "typeSpecific": {
    "routerCount": 16,
    "successCount": 12,
    "failCount": 4,
    "lastScanTime": "2025-12-20T11:50:00Z"
  }
}
```

### 4.3 認証フロー

```
1. GET /?cic=XXXXXX でアクセス
2. CICが一致 → JSON status返却
3. CICが不一致/未指定 → HTML UI返却（パスワード保護時は401）
```

### 4.4 巡回ポーリング用途

```
┌─────────────┐     GET /?cic=XXX     ┌─────────────┐
│  Inspector  │ ──────────────────► │  Target     │
│  (is10等)   │ ◄────────────────── │  Device     │
│             │     JSON Status      │             │
└─────────────┘                      └─────────────┘
```

---

## 5. ファイル構成

```
generic/ESP32/global/
├── src/
│   ├── AraneaWebUI.h           # 基底クラス宣言
│   ├── AraneaWebUI.cpp         # 基底クラス実装
│   ├── AraneaWebUIAssets.h     # CSS/JS/共通HTML (PROGMEM)
│   ├── AraneaStatusAPI.h       # ステータスAPI
│   └── AraneaStatusAPI.cpp
│
generic/ESP32/is10/
├── HttpManagerIs10.h           # is10拡張（継承）
├── HttpManagerIs10.cpp
├── Is10SpecificUI.h            # is10固有タブHTML
│
generic/ESP32/is02/
├── HttpManagerIs02.h           # is02拡張（継承）
├── HttpManagerIs02.cpp
│
...
```

---

## 6. 実装計画

### Phase 1: 基盤構築（AraneaWebUI）

1. **AraneaWebUI基底クラス作成**
   - 共通CSS/JS (PROGMEM格納)
   - タブナビゲーションシステム
   - 基本レイアウト生成

2. **共通APIエンドポイント実装**
   - GET /api/status
   - GET /?cic=XXX
   - POST /api/common/wifi
   - POST /api/common/ap
   - POST /api/common/cloud
   - POST /api/common/tenant
   - POST /api/system/reboot
   - POST /api/system/factory-reset

3. **共通タブUI実装**
   - Status タブ
   - Network タブ
   - Cloud タブ
   - Tenant タブ
   - System タブ

### Phase 2: is10移行

1. **HttpManagerIs10をAraneaWebUI継承に変更**
2. **is10固有タブ実装**
   - Routers タブ
   - Inspector タブ（CelestialGlobe + SSH設定）
3. **既存機能の動作確認**

### Phase 3: 他デバイス対応

1. **is02対応**: BLE + Relay タブ
2. **is04対応**: GPIO + Schedule タブ
3. **is05対応**: Inputs + Alerts タブ

### Phase 4: 高度な機能

1. **UIパスワード保護 (Basic認証)**
2. **定期再起動スケジュール**
3. **設定エクスポート/インポート**
4. **ログビューア**

---

## 7. バージョニング

| 項目 | 形式 | 例 |
|------|------|-----|
| Firmware Version | SemVer | 1.2.1 |
| UI Version | SemVer | 1.0.0 |
| Build Date | ISO8601 | 2025-12-20 |

UIバージョンはAraneaWebUIの共通部分のバージョン。
Firmwareバージョンはデバイスファームウェア全体のバージョン。

---

## 8. 将来拡張

- **OTAアップデートUI**: ファームウェアアップロード画面
- **診断モード**: 詳細デバッグ情報表示
- **多言語対応**: 日本語/英語切り替え
- **ダークモード**: UI テーマ切り替え
- **WebSocket**: リアルタイムステータス更新

---

## 9. 移行戦略

### 既存HttpManagerIs10との互換性

1. **既存APIパスを維持**
   - /api/config, /api/global 等は引き続き動作
2. **段階的移行**
   - 新規APIを追加しつつ、旧APIも維持
3. **設定キー互換**
   - NVS/SPIFFSのキー名は変更しない
