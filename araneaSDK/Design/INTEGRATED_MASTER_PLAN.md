# AraneaSDK 統合マスタープラン

**Version**: 1.0.0
**Last Updated**: 2025-12-25
**Status**: Final Integrated Design

---

## 1. 概要

本ドキュメントは、mobes2.0側とaranea_ISMS側の両設計計画を統合した最終マスタープランです。

### 1.1 ドキュメント配置

| リポジトリ | パス | 用途 |
|-----------|------|------|
| mobes2.0 | `doc/APPS/araneaSDK/Design/` | バックエンド・監視・AI統合 |
| aranea_ISMS | `araneaSDK/Design/` | ファーム実装・スキーマ・API |

### 1.2 統合されたドキュメント構成

```
AraneaSDK Design Documents
├── [共通] INDEX.md                    # 統合インデックス
├── [共通] ARCHITECTURE.md             # システムアーキテクチャ
├── [共通] AUTH_SPEC.md                # 認証仕様
├── [共通] SCHEMA_SPEC.md              # スキーマ仕様
├── [共通] API_REFERENCE.md            # API仕様
├── [共通] TYPE_REGISTRY.md            # Type登録仕様
│
├── [mobes] 01_SDK_Overview.md          # SDK概要
├── [mobes] 02_Device_Registration.md   # 登録プロトコル
├── [mobes] 03_State_Reporting.md       # 状態報告
├── [mobes] 07_Testing_Validation.md    # テストツール
├── [mobes] 08_Implementation_Roadmap.md # 実装ロードマップ
├── [mobes] 09_AdminSettings_Tab.md     # 管理画面設計
├── [mobes] 10_Document_Management.md   # ドキュメント管理
│
├── [aranea] DEVICE_IMPLEMENTATION.md   # ファーム実装ガイド
├── [aranea] VALIDATION_TOOLS.md        # 検証ツール
└── [aranea] DEVELOPMENT_WORKFLOW.md    # 開発ワークフロー
```

---

## 2. 二軸構造（最重要）

AraneaSDKは開発効率を最優先とし、以下の二軸で構成されます：

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    AraneaSDK Two-Axis Architecture                       │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│   ┌────────────────────────────────┐    ┌────────────────────────────┐  │
│   │    araneaSDK Tools             │    │    mobes Monitoring        │  │
│   │    (aranea_ISMS側)             │    │    (mobes2.0側)            │  │
│   ├────────────────────────────────┤    ├────────────────────────────┤  │
│   │                                │    │                            │  │
│   │ ★ 認証テスト                  │    │ ★ Firestore監視           │  │
│   │   - CIC/TID/LacisID検証       │    │   - 書き込み成功率         │  │
│   │   - Firestore照合             │    │   - データ整合性           │  │
│   │   - 権限レベル確認            │    │   - リアルタイム更新       │  │
│   │                                │    │                            │  │
│   │ ★ エンドポイントテスト        │    │ ★ BigQuery監視            │  │
│   │   - HTTP接続確認              │    │   - イベント到達率         │  │
│   │   - 登録/状態報告検証         │    │   - DLQ状態               │  │
│   │   - エラーハンドリング        │    │   - データ遅延             │  │
│   │                                │    │                            │  │
│   │ ★ MQTTテスト                  │    │ ★ Pub/Sub監視             │  │
│   │   - Broker接続                │    │   - メッセージ配信率       │  │
│   │   - Subscribe/Publish         │    │   - エラー追跡             │  │
│   │                                │    │                            │  │
│   │ ★ ESP32実装支援               │    │ ★ AI整合性チェック        │  │
│   │   - コード生成                │    │   - Metatron統合           │  │
│   │   - スキーマ検証              │    │   - 異常検知               │  │
│   │                                │    │                            │  │
│   └────────────────────────────────┘    └────────────────────────────┘  │
│                     │                              │                      │
│                     │    ┌──────────────────┐     │                      │
│                     └───►│  統合ダッシュボード│◄────┘                      │
│                          │  (AdminSettings)  │                            │
│                          └──────────────────┘                             │
│                                                                           │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. 統合コンポーネントマップ

### 3.1 SDK Core Components

| Component | mobes側 | aranea側 | 統合方針 |
|-----------|---------|----------|----------|
| Schema Registry | userObjectDetailSchemas | schemas/ | mobes Firestoreを正本、aranea側にミラー |
| Validator Engine | Cloud Functions | ESP32 compile-time | 両方で検証、仕様は共通 |
| Auth Manager | lacisOath | AraneaRegister | mobes側が認証、aranea側がクライアント |
| Protocol Generator | curlサンプル生成 | HTTPClient実装 | 仕様を共有、実装は各側 |
| Test Runner | CLI (Node.js) | Serial Monitor | 両方から実行可能 |
| Config Sync | Firestore Trigger | NVS Manager | 双方向同期 |

### 3.2 Monitoring Components (mobes側)

| Component | Description | Status |
|-----------|-------------|--------|
| FirestoreMonitor | 書き込み成功率・整合性 | Phase 2 |
| BigQueryMonitor | イベント到達率・DLQ | Phase 2 |
| PubSubMonitor | メッセージ配信統計 | Phase 2 |
| MetatronAI | 異常検知・自動修復 | Phase 5 |

### 3.3 Implementation Support (aranea側)

| Component | Description | Status |
|-----------|-------------|--------|
| LacisIDGenerator | MAC埋め込みID生成 | 実装済み |
| AraneaRegister | DeviceGate登録 | 実装済み |
| StateReporter | 状態レポート送信 | 実装済み |
| MqttManager | MQTT接続管理 | 実装済み |
| IOController | GPIO統一制御 | 実装済み |

---

## 4. 統合API仕様

### 4.1 認証方式（統合）

| 認証方式 | 用途 | 認証要素 | 実装箇所 |
|---------|------|---------|----------|
| Device Auth | デバイス→クラウド | tid + lacisId + cic | mobes: auth.ts / aranea: StateReporter |
| LacisOath | デバイス登録 | lacisId + userId + cic | mobes: araneaDeviceGate / aranea: AraneaRegister |
| Firebase Auth | UI/管理API | Firebase ID Token | mobes: middleware |
| MQTT Auth | 双方向通信 | lacisId:cic | mobes: MQTT Bridge / aranea: MqttManager |

### 4.2 エンドポイント（統合）

| Endpoint | mobes実装 | aranea実装 | 備考 |
|----------|-----------|------------|------|
| `/araneaDeviceGate` | araneaDeviceGate.ts | AraneaRegister.cpp | CIC発行 |
| `/deviceStateReport` | deviceStateReport.ts | StateReporter.cpp | 状態報告 |
| `/deviceStateReport/batch` | deviceStateReportBatch.ts | (Gateway用) | バッチ |
| `/araneaDeviceGetState/{id}` | araneaDeviceGetState.ts | (UI用) | 状態取得 |
| `/araneaDeviceCommand/{id}` | araneaDeviceCommand.ts | MqttManager.cpp | コマンド |
| `/araneaDeviceConfig/{id}` | araneaDeviceConfig.ts | SettingManager.cpp | 設定 |

---

## 5. 統合スキーマ管理

### 5.1 正本管理

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Schema Source of Truth                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│   mobes2.0 Firestore (Primary)                                           │
│   └── userObjectDetailSchemas/araneaDevice__{type}                       │
│       ├── schema.device._stateSchema    ← 状態スキーマ                   │
│       ├── schema.device.config          ← 設定スキーマ                   │
│       └── stats (fieldCount, depth)                                      │
│                                                                           │
│                          │                                               │
│                          │ SDK Sync                                      │
│                          ▼                                               │
│                                                                           │
│   AraneaSDK (Mirror)                                                     │
│   ├── aranea_ISMS/araneaSDK/schemas/                                     │
│   │   ├── userObject.schema.json                                         │
│   │   ├── deviceStateReport.schema.json                                  │
│   │   └── types/ISMS_ar-is04a.json                                       │
│   │                                                                       │
│   └── mobes2.0/doc/APPS/araneaSDK/schemas/                               │
│       └── (同一ファイルをミラー)                                         │
│                                                                           │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Type定義同期

| Type | ProductType | mobes登録 | aranea実装 | 稼働数 |
|------|-------------|-----------|------------|--------|
| ISMS_ar-is01 | 001 | ✓ | ✓ | 100+ |
| ISMS_ar-is02 | 002 | ✓ | ✓ | 稼働中 |
| ISMS_ar-is04a | 004 | ✓ | ✓ | 稼働中 |
| ISMS_ar-is05a | 005 | ✓ | ✓ | 稼働中 |
| ISMS_ar-is06a | 006 | 要登録 | ✓ | 開発中 |
| ISMS_ar-is10 | 010 | ✓ | ✓ | 稼働中 |
| ISMS_ar-is20s | 020 | 要登録 | ✓ | 開発中 |

---

## 6. 統合実装ロードマップ

### Phase 1: Core Testing Tools（Week 1-2）

**担当**: 両チーム協業

```
┌────────────────────────────────────────────────────────────────┐
│ Phase 1: Core Testing Tools                                     │
├────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [mobes側]                          [aranea側]                  │
│  ├── CLI基盤構築                    ├── ESP32検証関数           │
│  │   └── aranea-sdk CLI             │   └── AraneaPayloadValidator │
│  │                                   │                            │
│  ├── 認証テスト機能                 ├── 登録フロー検証          │
│  │   ├── CIC/TID/LacisID検証        │   ├── AraneaRegister動作確認 │
│  │   └── Firestore照合              │   └── CIC保存確認          │
│  │                                   │                            │
│  ├── エンドポイントテスト          ├── 通信テスト              │
│  │   ├── HTTP接続確認               │   ├── HTTPClient動作確認   │
│  │   └── レスポンス検証             │   └── タイムアウト確認     │
│  │                                   │                            │
│  └── MQTTテスト                     └── MQTT接続テスト          │
│      ├── Broker接続確認                 ├── Subscribe確認        │
│      └── Publish/Subscribe              └── メッセージ受信確認   │
│                                                                  │
└────────────────────────────────────────────────────────────────┘
```

**成果物**:
- `aranea-sdk auth test` コマンド
- `aranea-sdk endpoint test` コマンド
- `aranea-sdk mqtt test` コマンド
- ESP32 `AraneaPayloadValidator` ライブラリ

### Phase 2: Mobes Monitoring Dashboard（Week 3-4）

**担当**: mobes2.0チーム

```
┌────────────────────────────────────────────────────────────────┐
│ Phase 2: Mobes Monitoring Dashboard                             │
├────────────────────────────────────────────────────────────────┤
│                                                                  │
│  AdminSettings → AraneaDevices タブ                             │
│  ├── デバイス一覧                                               │
│  │   ├── 状態表示（alive/offline）                              │
│  │   ├── 最終更新時刻                                           │
│  │   └── Type別フィルタ                                         │
│  │                                                               │
│  ├── Firestore監視                                              │
│  │   ├── araneaDeviceStates書き込み統計                         │
│  │   ├── データ整合性スコア                                     │
│  │   └── エラー率トレンド                                       │
│  │                                                               │
│  ├── BigQuery監視                                               │
│  │   ├── イベント到達率                                         │
│  │   ├── DLQキュー深度                                          │
│  │   └── 手動リトライUI                                         │
│  │                                                               │
│  └── Pub/Sub監視                                                │
│      ├── メッセージ配信統計                                     │
│      └── ACK率                                                  │
│                                                                  │
└────────────────────────────────────────────────────────────────┘
```

**成果物**:
- AdminSettings AraneaDevicesタブ
- リアルタイムダッシュボード
- アラート通知システム

### Phase 3: Schema Registry & Validation（Week 5-6）

**担当**: 両チーム協業

```
┌────────────────────────────────────────────────────────────────┐
│ Phase 3: Schema Registry & Validation                           │
├────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [mobes側]                          [aranea側]                  │
│  ├── スキーマCRUD API               ├── スキーマミラー同期      │
│  │   ├── 一覧/詳細取得              │   └── schemas/*.json      │
│  │   ├── 新規登録                   │                            │
│  │   └── バージョン管理             ├── ESP32コード生成         │
│  │                                   │   ├── State構造体          │
│  ├── 検証エンジン                   │   ├── Config構造体         │
│  │   ├── 型チェック                 │   └── 検証関数             │
│  │   ├── 範囲チェック               │                            │
│  │   └── パターンチェック           └── コンパイル時検証        │
│  │                                       └── static_assert        │
│  └── TypeScript型定義生成                                       │
│                                                                  │
└────────────────────────────────────────────────────────────────┘
```

**成果物**:
- `aranea-sdk schema` コマンド群
- スキーマ管理Web UI
- ESP32/TypeScriptコード生成器

### Phase 4: E2E Testing & Document Management（Week 7-8）

**担当**: 両チーム協業

```
┌────────────────────────────────────────────────────────────────┐
│ Phase 4: E2E Testing & Document Management                      │
├────────────────────────────────────────────────────────────────┤
│                                                                  │
│  E2Eテストフレームワーク                                        │
│  ├── 登録→状態報告フロー                                        │
│  ├── 設定変更→適用確認フロー                                    │
│  ├── コマンド→ACKフロー                                         │
│  └── エラーリカバリフロー                                       │
│                                                                  │
│  ドキュメント管理システム (Metatron統合)                        │
│  ├── Obsidianライク参照                                         │
│  ├── SemanticTags v2統合                                        │
│  ├── 二段階参照メカニズム                                       │
│  └── 品質監査機能                                               │
│                                                                  │
└────────────────────────────────────────────────────────────────┘
```

**成果物**:
- E2Eテストスイート
- デバイスシミュレーター
- SDKドキュメント管理システム

### Phase 5: AI Integration（Week 9-10）

**担当**: mobes2.0チーム

```
┌────────────────────────────────────────────────────────────────┐
│ Phase 5: AI Integration                                         │
├────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Metatron AI統合                                                │
│  ├── 自動タグ付け                                               │
│  ├── 関連ドキュメント提案                                       │
│  └── 品質スコア算出                                             │
│                                                                  │
│  AI整合性チェック (gemini-2.5-flash)                            │
│  ├── データパターン分析                                         │
│  ├── 異常検知                                                   │
│  └── 修復提案生成                                               │
│                                                                  │
│  自動修復                                                       │
│  ├── 設定ドリフト自動修正                                       │
│  ├── スキーマ不整合修復                                         │
│  └── オフラインデバイスアラート                                 │
│                                                                  │
└────────────────────────────────────────────────────────────────┘
```

**成果物**:
- AI整合性チェック機能
- 自動修復機能
- 予測分析ダッシュボード

---

## 7. チーム責任分担

### 7.1 mobes2.0チーム

- Cloud Functions (araneaDevice/*)
- Firestore Rules & Collections
- AdminSettings UI
- 監視ダッシュボード
- Metatron AI統合
- BigQuery連携
- Pub/Sub設定

### 7.2 aranea_ISMSチーム

- ESP32ファームウェア
- 共通モジュール (global/src/*)
- デバイス固有実装 (is01a〜is20s)
- ローカルリレー (Orange Pi)
- ハードウェア設計
- OTA更新

### 7.3 共同責任

- スキーマ定義・更新
- API仕様策定
- Type登録
- SDK CLIツール開発
- E2Eテスト
- ドキュメント同期

---

## 8. 同期プロトコル

### 8.1 スキーマ変更フロー

```
1. [aranea] 新Type/スキーマ変更を提案
      ↓
2. [共同] SDK Design Review
      ↓
3. [mobes] Firestore typeSettings更新
      ↓
4. [aranea] schemas/*.json ミラー更新
      ↓
5. [共同] 統合テスト
      ↓
6. [両方] ドキュメント更新
```

### 8.2 API変更フロー

```
1. [mobes/aranea] API変更提案
      ↓
2. [共同] API_REFERENCE.md更新
      ↓
3. [mobes] Cloud Function実装
      ↓
4. [aranea] HTTPClient実装更新
      ↓
5. [共同] E2Eテスト
      ↓
6. [両方] Release Notes更新
```

---

## 9. 成功指標

| 指標 | 目標値 | 計測方法 |
|------|--------|----------|
| 認証テスト実行時間 | < 3秒 | CLI計測 |
| エンドポイントテスト実行時間 | < 5秒 | CLI計測 |
| MQTTテスト実行時間 | < 10秒 | CLI計測 |
| 監視ダッシュボード更新間隔 | < 5秒 | Firebase SDK |
| BigQuery到達率 | > 99.9% | BigQuery統計 |
| DLQ滞留時間 | < 30分 | DLQ監視 |
| スキーマ検証カバレッジ | 100% | テストカバレッジ |
| ドキュメント同期遅延 | < 1日 | Git commit比較 |

---

## 10. 参照ドキュメント

### mobes2.0側

| ドキュメント | パス |
|------------|------|
| SDK Overview | doc/APPS/araneaSDK/Design/01_SDK_Overview.md |
| Device Registration | doc/APPS/araneaSDK/Design/02_Device_Registration_Protocol.md |
| State Reporting | doc/APPS/araneaSDK/Design/03_State_Reporting_Protocol.md |
| Schema Management | doc/APPS/araneaSDK/Design/04_Schema_Management.md |
| LacisOath Integration | doc/APPS/araneaSDK/Design/05_LacisOath_Permission_Integration.md |
| Communication | doc/APPS/araneaSDK/Design/06_Communication_Protocols.md |
| Testing Tools | doc/APPS/araneaSDK/Design/07_Testing_Validation_Tools.md |
| Roadmap | doc/APPS/araneaSDK/Design/08_SDK_Implementation_Roadmap.md |
| AdminSettings | doc/APPS/araneaSDK/Design/09_AdminSettings_AraneaDevices_Tab.md |
| Document Management | doc/APPS/araneaSDK/Design/10_SDK_Document_Management.md |

### aranea_ISMS側

| ドキュメント | パス |
|------------|------|
| INDEX | araneaSDK/Design/INDEX.md |
| Architecture | araneaSDK/Design/ARCHITECTURE.md |
| Auth Spec | araneaSDK/Design/AUTH_SPEC.md |
| Schema Spec | araneaSDK/Design/SCHEMA_SPEC.md |
| API Reference | araneaSDK/Design/API_REFERENCE.md |
| Device Implementation | araneaSDK/Design/DEVICE_IMPLEMENTATION.md |
| Validation Tools | araneaSDK/Design/VALIDATION_TOOLS.md |
| Type Registry | araneaSDK/Design/TYPE_REGISTRY.md |
| Development Workflow | araneaSDK/Design/DEVELOPMENT_WORKFLOW.md |

---

## 11. 次のアクション

### 即時対応

1. [ ] mobes側: is06a, is20s の typeSettings 登録
2. [x] aranea側: スキーマミラーディレクトリ作成 ✅ 2025-12-25完了
3. [ ] 共同: Phase 1 CLIツール開発開始

### 週次同期

- 毎週月曜: SDK進捗レビュー
- 毎週金曜: ドキュメント同期確認

---

**このドキュメントは両リポジトリで同期管理されます。**
