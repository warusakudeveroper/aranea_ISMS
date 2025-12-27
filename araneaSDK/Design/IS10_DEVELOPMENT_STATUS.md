# is10 (Router Inspector) 開発進捗

> **Last Updated**: 2025-12-27
> **Firmware Version**: 1.2.1
> **UI Version**: 1.6.0
> **Device**: ar-is10

---

## 概要

is10はルーター監視デバイスで、SSHを使用してOpenWRT/ASUSWRTルーターからクライアント・AP情報を収集し、CelestialGlobeへ送信します。

---

## 開発ステータス

### 完了項目 ✅

| 機能 | ステータス | 確認日 | 備考 |
|------|-----------|--------|------|
| **AraneaWebUI統合** | ✅ 完了 | 2025-12-27 | v1.6.0 |
| **デバイス登録** | ✅ 完了 | 2025-12-27 | araneaDeviceGate経由 |
| **deviceStateReport** | ✅ 完了 | 2025-12-27 | HTTP 200確認 |
| **WiFiスキャン機能** | ✅ 完了 | 2025-12-27 | v1.6.0で追加 |
| **チップ温度監視** | ✅ 完了 | 2025-12-27 | 74.4°C表示 |
| **SSH接続** | ✅ 完了 | - | LibSSH-ESP32使用 |
| **ルーターポーリング** | ✅ 完了 | 2025-12-27 | 16/16成功 |
| **OpenWRT対応** | ✅ 完了 | - | UCI/ubus解析 |
| **ASUSWRT対応** | ✅ 完了 | - | nvram/ARP解析 |
| **UIタブ (Inspector)** | ✅ 完了 | 2025-12-27 | ポーリング設定 |
| **UIタブ (Routers)** | ✅ 完了 | 2025-12-27 | CRUD操作 |
| **SpeedDial対応** | ✅ 完了 | - | INI形式設定 |
| **MQTT受信** | ✅ 完了 | - | configコマンド対応 |
| **OTA更新** | ✅ 完了 | 2025-12-27 | HTTP OTA確認 |

### 未完了項目 ❌

| 機能 | ステータス | 備考 |
|------|-----------|------|
| **CelestialGlobe送信** | ❌ 未テスト | endpoint/secret未設定 |
| **MQTT送信** | ❌ 未テスト | mqttEndpoint未取得問題 |

---

## 現在のデバイス状態

```json
{
  "typeSpecific": {
    "totalRouters": 16,
    "successfulPolls": 16,
    "pollingEnabled": true,
    "celestialConfigured": false,
    "mqttConnected": false,
    "lastStateReportCode": 200
  }
}
```

- **ルーター設定**: 16台登録済み
- **ポーリング**: 全16台成功
- **deviceStateReport**: 200 OK
- **CelestialGlobe**: 未設定（endpoint/secret空）
- **MQTT**: 未接続

---

## CelestialGlobe設定要件

### 必要な設定値

| 項目 | キー | 説明 |
|------|------|------|
| Endpoint URL | `is10_endpoint` | CelestialGlobe Universal Ingestエンドポイント |
| Secret | `is10_secret` | X-Celestial-Secretヘッダー値 |

### エンドポイント形式

```
https://us-central1-mobesorder.cloudfunctions.net/celestialGlobe_universalIngest
  ?fid=XXXX
  &source=araneaDevice
```

### 送信ペイロード（ルーター単位）

```json
{
  "source": "ar-is10",
  "timestamp": "2025-12-27T04:00:00Z",
  "report": {
    "type": "client_ap_list",
    "rid": "306",
    "hostname": "openwrt-306",
    "mac": "AA:BB:CC:DD:EE:FF",
    "total_clients": 15,
    "total_aps": 8,
    "clients": [...],
    "aps": [...]
  }
}
```

### UI設定場所

```
http://<device-ip>/
  → Inspector タブ
    → CelestialGlobe SSOT セクション
      - Endpoint URL
      - X-Celestial-Secret
```

---

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│                       is10.ino                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │ SshPoller   │  │ StateReport │  │ CelestialSender │ │
│  │ Is10        │  │ Is10        │  │ Is10            │ │
│  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘ │
│         │                │                   │          │
│         ▼                ▼                   ▼          │
│  ┌──────────────────────────────────────────────────┐  │
│  │            HttpManagerIs10                        │  │
│  │         (extends AraneaWebUI)                     │  │
│  │  ┌────────────┐ ┌────────────┐ ┌──────────────┐  │  │
│  │  │ Inspector  │ │ Routers    │ │ 共通5タブ    │  │  │
│  │  │ Tab        │ │ Tab        │ │ (AraneaWebUI)│  │  │
│  │  └────────────┘ └────────────┘ └──────────────┘  │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
         │                │                   │
         ▼                ▼                   ▼
   ┌──────────┐    ┌────────────┐    ┌────────────────┐
   │ Routers  │    │ mobes API  │    │ CelestialGlobe │
   │ (SSH)    │    │ stateReport│    │ universalIngest│
   └──────────┘    └────────────┘    └────────────────┘
```

---

## データフロー

### deviceStateReport（IoT管理）

```
is10 → deviceStateReport API → mobes Firestore
  - デバイス自身の状態（heap, uptime, rssi等）
  - ポーリング統計（totalRouters, successfulPolls）
  - 60秒間隔
```

### CelestialGlobe（ビジネスデータ）

```
is10 → CelestialGlobe universalIngest → BigQuery/SSOT
  - 収集したルーター情報（clients, APs）
  - ルーター単位で送信
  - ポーリングサイクル完了後
```

**重要**: この2つは別軸。deviceStateReportはデバイス管理、CelestialGlobeはビジネスデータ収集。

---

## 次のステップ

1. **CelestialGlobeテスト**
   - endpoint URLをUIから設定
   - X-Celestial-Secretを設定
   - ポーリングサイクル完了後の送信確認

2. **MQTT問題調査**
   - mqttEndpoint未取得の原因調査
   - araneaDeviceGate側の対応確認

---

## 関連ドキュメント

- [ARANEA_WEB_UI.md](./ARANEA_WEB_UI.md) - 共通UIフレームワーク
- [DEVICE_IMPLEMENTATION.md](./DEVICE_IMPLEMENTATION.md) - デバイス実装ガイド
- [ESP32_BEST_PRACTICES.md](../ESP32_BEST_PRACTICES.md) - ESP32開発Tips

---

## ファイル構成

```
code/ESP32/is10/
├── is10.ino                    # メインスケッチ
├── AraneaGlobalImporter.h      # 共通モジュールインポート
├── AraneaSettings.cpp/h        # is10固有設定
├── HttpManagerIs10.cpp/h       # Web UI（AraneaWebUI継承）
├── SshPollerIs10.cpp/h         # SSHポーリング
├── StateReporterIs10.cpp/h     # deviceStateReport
├── CelestialSenderIs10.cpp/h   # CelestialGlobe送信
├── MqttConfigHandler.cpp/h     # MQTT設定受信
├── Is10Keys.h                  # NVSキー定義
└── README.md                   # デバイス概要
```

---

## バージョン履歴

| Version | Date | Changes |
|---------|------|---------|
| 1.2.1 | 2025-12-27 | AraneaWebUI v1.6.0統合（WiFiスキャン、チップ温度） |
| 1.2.0 | 2025-12-26 | SSH安定化、メモリ最適化 |
| 1.1.0 | 2025-12-24 | ASUSWRT対応追加 |
| 1.0.0 | 2025-12-22 | 初版（OpenWRTのみ）|
