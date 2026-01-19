# DD08: IS21 AraneaWebUI実装

> **Version**: 1.0.0
> **Created**: 2026-01-11
> **Status**: Draft

---

## 概要

IS21（ParaclateEdge）にaraneaSDK/Design/ARANEA_WEB_UI.md準拠の管理WebUIを実装する。
ESP32版AraneaWebUIと同等の共通5タブ構成をReact + FastAPIで実現する。

### 準拠仕様

- araneaSDK/Design/ARANEA_WEB_UI.md v1.6.0
- MUI 7系カラーパレット
- CIC認証必須

---

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│              IS21 AraneaWebUI (React)                    │
│              code/orangePi/is21/frontend/                │
│  ┌───────────────────────────────────────────────────┐  │
│  │ 共通タブ (5タブ):                                  │  │
│  │   Status   - デバイス情報、Live Status、温度       │  │
│  │   Network  - WiFi設定、ホスト名、NTP              │  │
│  │   Cloud    - 登録状態、アクティベート             │  │
│  │   Tenant   - TID/FID/deviceName                   │  │
│  │   System   - Reboot、Factory Reset                │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────┐  │
│  │ IS21固有タブ:                                      │  │
│  │   Inference - モデル状態、推論統計、閾値設定       │  │
│  │   IS22      - 接続IS22一覧、アクティベート管理     │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                         │
                         │ API (FastAPI)
                         ▼
┌─────────────────────────────────────────────────────────┐
│              IS21 Backend (FastAPI)                      │
│              code/orangePi/is21/src/main.py              │
│  ┌───────────────────────────────────────────────────┐  │
│  │ 共通API (AraneaWebUI互換):                         │  │
│  │   GET  /api/status?cic=XXXXXX                     │  │
│  │   GET  /api/config?cic=XXXXXX                     │  │
│  │   POST /api/network/save                          │  │
│  │   POST /api/cloud/activate                        │  │
│  │   POST /api/tenant/save                           │  │
│  │   POST /api/system/reboot                         │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────┐  │
│  │ IS21固有API:                                       │  │
│  │   GET  /api/inference/status                      │  │
│  │   POST /api/inference/config                      │  │
│  │   GET  /api/is22/list                             │  │
│  │   POST /api/is22/activate                         │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## タブ仕様

### 共通タブ（ESP32 AraneaWebUI準拠）

#### Status タブ

| 項目 | 表示内容 |
|------|---------|
| Device Type | ar-is21 |
| LacisID | 3021XXXXXXXXXXXX0001 |
| Firmware | 1.8.0 |
| Uptime | 12d 4h 32m |
| CPU Temp | 35.2°C |
| NPU Temp | 34.2°C |
| Memory | 7.4% used |
| Registration | Registered / Not Registered |

#### Network タブ

| 項目 | 設定内容 |
|------|---------|
| Hostname | is21-xxxxxx |
| IP Address | 192.168.3.240 (readonly) |
| NTP Server | pool.ntp.org |

※IS21はLinuxのため、WiFi設定はOS側で管理。UIでは表示のみ。

#### Cloud タブ（★重要：アクティベーション）

| 項目 | 設定内容 |
|------|---------|
| Registration Status | Registered / Not Registered |
| CIC | 表示のみ（登録後） |
| State Endpoint | 表示のみ（登録後） |
| MQTT Endpoint | 表示のみ（登録後） |

**アクティベーションフォーム:**

```
┌─────────────────────────────────────────┐
│  Device Activation                       │
├─────────────────────────────────────────┤
│  TID:                                    │
│  ┌─────────────────────────────────────┐│
│  │ T9999999999999999999               ││
│  └─────────────────────────────────────┘│
│                                          │
│  Tenant Primary LacisID:                 │
│  ┌─────────────────────────────────────┐│
│  │ 14177489787976846466               ││
│  └─────────────────────────────────────┘│
│                                          │
│  Tenant Primary Email:                   │
│  ┌─────────────────────────────────────┐│
│  │ webadmin@mijeos.com                ││
│  └─────────────────────────────────────┘│
│                                          │
│  Tenant Primary CIC:                     │
│  ┌─────────────────────────────────────┐│
│  │ 123456                             ││
│  └─────────────────────────────────────┘│
│                                          │
│  ┌──────────────┐                        │
│  │  Activate    │                        │
│  └──────────────┘                        │
└─────────────────────────────────────────┘
```

#### Tenant タブ

| 項目 | 設定内容 |
|------|---------|
| TID | T9999999999999999999 (readonly after registration) |
| FID | 9966 |
| Device Name | ParaclateEdge-01 |

#### System タブ

| 項目 | 機能 |
|------|------|
| Reboot | システム再起動 |
| Clear Registration | 登録クリア（要確認ダイアログ） |
| Logs | システムログ表示 |

### IS21固有タブ

#### Inference タブ

| 項目 | 表示/設定 |
|------|----------|
| YOLO Model | yolov5s-640-640.rknn |
| PAR Model | par_resnet50_pa100k |
| Inference Count | 399,773 |
| Success Rate | 100% |
| Avg Latency | 31.39ms |
| Confidence Threshold | 0.33 (slider) |
| NMS Threshold | 0.45 (slider) |

#### IS22 タブ（IS22からのアクティベート管理）

| 項目 | 表示 |
|------|------|
| Connected IS22 | 一覧表示 |
| IS22 LacisID | 3022XXXXXXXXXXXX0001 |
| IS22 TID | T2025120621041161827 |
| Status | Active / Inactive |
| Last Access | 2026-01-11 08:30:00 |

---

## API仕様

### POST /api/cloud/activate

IS21をaraneaDeviceGateに登録する。

**Request:**
```json
{
  "tid": "T9999999999999999999",
  "tenant_primary": {
    "lacis_id": "14177489787976846466",
    "user_id": "webadmin@mijeos.com",
    "cic": "123456"
  }
}
```

**Response (Success):**
```json
{
  "ok": true,
  "cic_code": "XXXXXX",
  "lacis_id": "3021C0742BFCCF950001",
  "state_endpoint": "https://...",
  "mqtt_endpoint": "mqtt://..."
}
```

**Response (Error):**
```json
{
  "ok": false,
  "error": "AUTH_FAILED"
}
```

### POST /api/is22/activate

IS22からIS21へのアクティベート要求を受け付ける。

**Request:**
```json
{
  "is22_lacis_id": "3022E051D815448B0001",
  "is22_tid": "T2025120621041161827",
  "is22_cic": "605123"
}
```

**Response:**
```json
{
  "ok": true,
  "message": "IS22 activated"
}
```

---

## ファイル構成

```
code/orangePi/is21/
├── frontend/                    # NEW: React frontend
│   ├── src/
│   │   ├── App.tsx
│   │   ├── components/
│   │   │   ├── tabs/
│   │   │   │   ├── StatusTab.tsx
│   │   │   │   ├── NetworkTab.tsx
│   │   │   │   ├── CloudTab.tsx      # ★アクティベーション
│   │   │   │   ├── TenantTab.tsx
│   │   │   │   ├── SystemTab.tsx
│   │   │   │   ├── InferenceTab.tsx  # IS21固有
│   │   │   │   └── IS22Tab.tsx       # IS21固有
│   │   │   └── common/
│   │   │       ├── Card.tsx
│   │   │       ├── Button.tsx
│   │   │       └── Toast.tsx
│   │   ├── hooks/
│   │   │   └── useApi.ts
│   │   ├── styles/
│   │   │   └── theme.css             # MUI 7系カラー
│   │   └── types/
│   │       └── api.ts
│   ├── package.json
│   ├── vite.config.ts
│   └── index.html
├── src/
│   ├── main.py                  # MODIFY: ハードコード削除、API追加
│   ├── api/                     # NEW: APIルート分離
│   │   ├── __init__.py
│   │   ├── status_routes.py
│   │   ├── cloud_routes.py      # ★アクティベーションAPI
│   │   ├── tenant_routes.py
│   │   ├── system_routes.py
│   │   └── is22_routes.py
│   └── aranea_common/
│       ├── aranea_register.py   # 既存
│       └── config_manager.py    # 既存
└── config/
    └── is21_settings.json       # 設定永続化
```

---

## 実装タスク

| タスクID | タスク名 | 優先度 | 見積もり |
|---------|---------|--------|---------|
| UI-1 | React project setup (Vite) | P0 | S |
| UI-2 | 共通コンポーネント (Card, Button, Toast) | P0 | S |
| UI-3 | Status タブ実装 | P0 | M |
| UI-4 | Network タブ実装 | P1 | S |
| UI-5 | Cloud タブ実装（アクティベーション） | P0 | L |
| UI-6 | Tenant タブ実装 | P1 | S |
| UI-7 | System タブ実装 | P1 | S |
| UI-8 | Inference タブ実装 | P1 | M |
| UI-9 | IS22 タブ実装 | P2 | M |
| API-1 | /api/cloud/activate 実装 | P0 | M |
| API-2 | /api/is22/activate 実装 | P1 | M |
| API-3 | main.py ハードコード削除 | P0 | S |
| API-4 | static file serving設定 | P0 | S |
| TEST-1 | aranea_SDKで登録テスト | P0 | M |

---

## テスト計画

### 単体テスト

| テストID | 内容 | 期待結果 |
|---------|------|---------|
| UT-1 | /api/status 取得 | デバイス情報JSON返却 |
| UT-2 | /api/cloud/activate 正常系 | CIC発行、登録完了 |
| UT-3 | /api/cloud/activate 認証エラー | AUTH_FAILEDエラー |
| UT-4 | /api/is22/activate 正常系 | IS22登録完了 |

### E2Eテスト

| テストID | 内容 | 期待結果 |
|---------|------|---------|
| E2E-1 | ブラウザでCloud tabアクセス | アクティベーションフォーム表示 |
| E2E-2 | テナント情報入力→Activate | CIC取得、登録状態に遷移 |
| E2E-3 | 登録後Status tab | Registered表示 |
| E2E-4 | araneaSDK Metatronで確認 | IS21がデバイス一覧に表示 |

---

## 依存関係

- Phase 1（AraneaRegister）: 完了済み
- aranea_common/aranea_register.py: 利用
- aranea_common/config_manager.py: 利用

---

## 制約事項

1. MUI 7系カラーパレット遵守（ARANEA_WEB_UI.md準拠）
2. CIC認証必須（登録後のAPI呼び出し）
3. Single Page Application（React Router不使用、タブ切り替え）

---

## 更新履歴

| 日付 | バージョン | 内容 |
|------|-----------|------|
| 2026-01-11 | 1.0.0 | 初版作成 |
