# AraneaSDK Architecture

## 1. System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              AraneaSDK                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    Validation & Testing Layer                        │    │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────────┐  │    │
│  │  │ Schema       │ │ Auth         │ │ Payload      │ │ E2E        │  │    │
│  │  │ Validator    │ │ Validator    │ │ Validator    │ │ Test Suite │  │    │
│  │  └──────────────┘ └──────────────┘ └──────────────┘ └────────────┘  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    Reference Data Layer                              │    │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────────┐  │    │
│  │  │ Type         │ │ Schema       │ │ ProductType  │ │ Endpoint   │  │    │
│  │  │ Registry     │ │ Repository   │ │ Registry     │ │ Config     │  │    │
│  │  └──────────────┘ └──────────────┘ └──────────────┘ └────────────┘  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
            ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
            │   mobes2.0   │ │ araneaDevice │ │   Local      │
            │   Backend    │ │  Firmware    │ │   Relay      │
            │  (Firebase)  │ │   (ESP32)    │ │ (Orange Pi)  │
            └──────────────┘ └──────────────┘ └──────────────┘
```

---

## 2. Component Architecture

### 2.1 mobes2.0 Side (Backend)

```
mobes2.0/
├── functions/                    # Cloud Functions (TypeScript)
│   └── src/
│       ├── araneaDevice/         # araneaDevice 専用ロジック
│       │   ├── auth.ts           # デバイス認証
│       │   ├── deviceStateReport.ts
│       │   ├── deviceStateReportBatch.ts
│       │   ├── araneaDeviceCommand.ts
│       │   ├── araneaDeviceConfig.ts
│       │   ├── araneaDeviceGetState.ts
│       │   └── bigQueryGateway.ts
│       ├── lacisOath/            # 認証・権限管理
│       │   ├── permissionChecker.ts
│       │   ├── cicValidator.ts
│       │   └── tokenManager.ts
│       └── http/
│           └── araneaDeviceGate.ts  # デバイス登録API
├── src/                          # Frontend (React/TypeScript)
│   └── core/
│       ├── entities/admin/       # Type定義
│       └── domain/device/        # デバイススキーマ
└── doc/LacisOath/
    └── araneaDevice関連/         # 詳細設計書
```

### 2.2 aranea_ISMS Side (Firmware)

```
aranea_ISMS/
├── code/ESP32/
│   ├── global/                   # 共通モジュール
│   │   └── src/
│   │       ├── AraneaRegister.h/cpp      # DeviceGate登録
│   │       ├── LacisIDGenerator.h/cpp    # LacisID生成
│   │       ├── StateReporter.h/cpp       # 状態レポート送信
│   │       ├── MqttManager.h/cpp         # MQTT接続
│   │       ├── AraneaWebUI.h/cpp         # Web UI基底クラス
│   │       ├── SettingManager.h          # NVS設定管理
│   │       ├── IOController.h            # GPIO制御
│   │       └── ...
│   ├── is01a～is10/              # デバイス固有実装
│   └── ...
├── code/orangePi/
│   └── is20s/                    # ネットワーク監視
└── araneaSDK/                    # SDK (このドキュメント)
    └── Design/
```

### 2.3 AraneaSDK Components

```
araneaSDK/
├── Design/                       # 設計ドキュメント
│   ├── INDEX.md
│   ├── ARCHITECTURE.md           # (this file)
│   ├── AUTH_SPEC.md
│   ├── SCHEMA_SPEC.md
│   ├── API_REFERENCE.md
│   ├── DEVICE_IMPLEMENTATION.md
│   ├── VALIDATION_TOOLS.md
│   ├── DEVELOPMENT_WORKFLOW.md
│   └── TYPE_REGISTRY.md
├── tools/                        # CLI/検証ツール
│   ├── validator/                # スキーマ検証
│   ├── auth-tester/              # 認証テスト
│   ├── payload-checker/          # ペイロード検証
│   └── e2e-test/                 # E2Eテスト
├── schemas/                      # 正本スキーマ定義
│   ├── userObject.schema.json
│   ├── userObjectDetail.schema.json
│   ├── deviceStateReport.schema.json
│   ├── araneaDeviceStates.schema.json
│   └── types/                    # Type別スキーマ
│       ├── ISMS_ar-is01.json
│       ├── ISMS_ar-is04a.json
│       └── ...
├── samples/                      # サンプルコード
│   ├── curl/                     # curlスクリプト
│   ├── esp32/                    # ESP32実装例
│   └── python/                   # Pythonクライアント
└── tests/                        # テストスイート
    ├── unit/
    ├── integration/
    └── e2e/
```

---

## 3. Data Flow

### 3.1 Device Registration Flow

```
┌──────────────┐     ┌──────────────────────┐     ┌──────────────┐
│   ESP32      │     │    araneaDeviceGate  │     │   Firestore  │
│   Device     │     │    (Cloud Function)  │     │              │
└──────┬───────┘     └──────────┬───────────┘     └──────┬───────┘
       │                        │                        │
       │  1. POST /araneaDeviceGate                      │
       │  (lacisOath + deviceMeta)                       │
       │ ─────────────────────────>                      │
       │                        │                        │
       │                        │  2. Validate Tenant Primary
       │                        │ ─────────────────────────>
       │                        │                        │
       │                        │  3. Create userObject  │
       │                        │ ─────────────────────────>
       │                        │                        │
       │                        │  4. Generate CIC       │
       │                        │ <─────────────────────────
       │                        │                        │
       │  5. Response           │                        │
       │  { cic, endpoints }    │                        │
       │ <─────────────────────────                      │
       │                        │                        │
       │  6. Save to NVS        │                        │
       │                        │                        │
```

### 3.2 State Report Flow

```
┌──────────────┐     ┌──────────────────────┐     ┌──────────────┐
│   ESP32      │     │   deviceStateReport  │     │   Firestore  │
│   Device     │     │   (Cloud Function)   │     │   BigQuery   │
└──────┬───────┘     └──────────┬───────────┘     └──────┬───────┘
       │                        │                        │
       │  1. POST /deviceStateReport                     │
       │  { auth, report }      │                        │
       │ ─────────────────────────>                      │
       │                        │                        │
       │                        │  2. Validate auth      │
       │                        │ ─────────────────────────>
       │                        │                        │
       │                        │  3. Dedup check        │
       │                        │ ─────────────────────────>
       │                        │                        │
       │                        │  4. Update araneaDeviceStates
       │                        │ ─────────────────────────>
       │                        │                        │
       │                        │  5. Insert to BigQuery │
       │                        │ ─────────────────────────>
       │                        │                        │
       │  6. Response           │                        │
       │  { ok, dedupHash }     │                        │
       │ <─────────────────────────                      │
```

### 3.3 MQTT Bidirectional Flow

```
┌──────────────┐     ┌──────────────────────┐     ┌──────────────┐
│   ESP32      │     │    MQTT Bridge       │     │   mobes2.0   │
│   Device     │     │    (Cloud Run)       │     │   Backend    │
└──────┬───────┘     └──────────┬───────────┘     └──────┬───────┘
       │                        │                        │
       │  1. WSS Connect        │                        │
       │  (lacisId:cic)         │                        │
       │ ─────────────────────────>                      │
       │                        │                        │
       │  2. Subscribe          │                        │
       │  aranea/{lacisId}/cmd  │                        │
       │ ─────────────────────────>                      │
       │                        │                        │
       │                        │  3. Command from UI    │
       │                        │ <─────────────────────────
       │                        │                        │
       │  4. PUBLISH to Device  │                        │
       │ <─────────────────────────                      │
       │                        │                        │
       │  5. Execute & Report   │                        │
       │ ─────────────────────────>                      │
       │                        │                        │
       │                        │  6. Forward to Backend │
       │                        │ ─────────────────────────>
```

---

## 4. Firestore Collections

### 4.1 Collection Hierarchy

```
firestore/
├── userObject/                   # デバイス基本情報
│   └── {lacisId}/
│       ├── lacis: { type_domain, type, tid, permission, cic_code, cic_active }
│       ├── activity: { created_at, lastUpdated_at }
│       └── device: { macAddress, productType, productCode }
│
├── userObject_detail/            # デバイス詳細情報
│   └── {lacisId}/
│       ├── firmware: { version, buildDate }
│       ├── config: { ... device specific ... }
│       └── status: { online, lastSeen }
│
├── araneaDeviceStates/           # 最新状態キャッシュ (HOT)
│   └── {lacisId}/
│       ├── tid, fid[], type
│       ├── state: { ... }
│       ├── semanticTags[]
│       ├── advert: { gatewayLacisId, rssi, observedAt }
│       ├── alive, lastUpdatedAt
│       └── events/               # サブコレクション
│           └── {eventId}/
│
├── araneaDeviceCommands/         # コマンドキュー
│   └── {lacisId}/
│       └── {cmdId}/
│           ├── command, params
│           ├── status, createdAt, executedAt
│           └── result
│
├── araneaDeviceAdvertiseDedup/   # 重複排除 (TTL: 10分)
│   └── {hash}/
│
└── typeSettings/                 # Type別スキーマ
    └── araneaDevice/
        └── {type}/
            ├── schema: { ... }
            └── config: { ... }
```

---

## 5. SDK Tool Architecture

### 5.1 Schema Validator

```
┌─────────────────────────────────────────────────────────────┐
│                    Schema Validator                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │ JSON Schema │  │ Type        │  │ Cross-      │          │
│  │ Parser      │  │ Resolver    │  │ Reference   │          │
│  │             │  │             │  │ Checker     │          │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘          │
│         │                │                │                  │
│         └────────────────┼────────────────┘                  │
│                          ▼                                   │
│                   ┌─────────────┐                            │
│                   │ Validation  │                            │
│                   │ Report      │                            │
│                   └─────────────┘                            │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 Auth Tester

```
┌─────────────────────────────────────────────────────────────┐
│                      Auth Tester                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │ LacisID     │  │ CIC         │  │ Permission  │          │
│  │ Generator   │  │ Validator   │  │ Checker     │          │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘          │
│         │                │                │                  │
│         └────────────────┼────────────────┘                  │
│                          ▼                                   │
│                   ┌─────────────┐                            │
│                   │ Test Report │                            │
│                   └─────────────┘                            │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. Security Architecture

### 6.1 Authentication Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    Authentication Stack                      │
├─────────────────────────────────────────────────────────────┤
│ Layer 1: Device Authentication (deviceStateReport)          │
│   - tid + lacisId + cic (6桁)                               │
│   - Firestore userObject検証                                │
├─────────────────────────────────────────────────────────────┤
│ Layer 2: Tenant Primary Authentication (araneaDeviceGate)   │
│   - lacisId + userId + cic (3要素)                          │
│   - permission >= 61                                         │
├─────────────────────────────────────────────────────────────┤
│ Layer 3: Firebase Auth (UI/API)                             │
│   - Firebase ID Token                                        │
│   - permission-based access control                          │
├─────────────────────────────────────────────────────────────┤
│ Layer 4: MQTT Authentication                                │
│   - username: lacisId                                       │
│   - password: cic                                           │
│   - TLS/SSL required                                         │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 Permission Hierarchy

```
Permission Level:
  100: Lacis (System SU)
   71+: System Authority Layer
   61+: Tenant Primary / Patriarch
   41+: Tenant Management Authority
   10: Staff / Device Basic

araneaDevice固有:
  - 登録: Tenant Primary (61+) 必須
  - 状態報告: Device Auth (tid+lacisId+cic)
  - 状態取得: Staff (10+)
  - コマンド発行: Order (61+)
  - 設定変更: Order (61+)
```

---

## 7. Integration Points

### 7.1 mobes2.0 → araneaDevice

| 連携ポイント | 方式 | 用途 |
|-------------|------|------|
| araneaDeviceGate | HTTP POST | デバイス登録・CIC発行 |
| deviceStateReport | HTTP POST | 状態レポート受信 |
| MQTT Bridge | WebSocket | 双方向通信 |
| Pub/Sub | Event | 非同期イベント配信 |
| BigQuery | Streaming | データ分析・履歴 |

### 7.2 araneaDevice → mobes2.0

| 連携ポイント | 方式 | 用途 |
|-------------|------|------|
| 登録リクエスト | HTTP | 初回登録 |
| 状態レポート | HTTP/MQTT | 定期/イベント報告 |
| コマンド応答 | MQTT | コマンド実行結果 |
| Heartbeat | HTTP/MQTT | 死活監視 |

---

## 8. Scalability Considerations

### 8.1 Current Scale
- 稼働デバイス: 100台以上 (archive_ISMS系)
- 想定最大: 10,000台/テナント

### 8.2 Design Principles
1. **Stateless Backend**: Cloud Functions はステートレス
2. **HOT/COLD 分離**: araneaDeviceStates (HOT) / BigQuery (COLD)
3. **Deduplication**: 10分TTLで重複排除
4. **Batch Processing**: deviceStateReportBatch で効率化
5. **Rate Limiting**: テナント単位でレート制限

---

## 9. Version Compatibility

### 9.1 API Versioning
- 現在: v1 (implicit)
- 将来: パス/ヘッダーでバージョニング

### 9.2 Schema Versioning
- userObject.schemaVersion
- typeSettings/{type}/version

### 9.3 Firmware Versioning
- `firmwareVersion` in userObject_detail
- OTA更新対応 (is02a, is04a, is05a, is06a, is10)
