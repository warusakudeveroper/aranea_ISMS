# is02a - Temp&HumSensor (Master) 設計書

**正式名称**: Aranea Temperature & Humidity Sensor - Master
**製品コード**: AR-IS02A
**作成日**: 2025/12/22
**ベース**: archive_ISMS/ESP32/is02 (ISMS専用版)
**目的**: is01a(Cluster Node)のBLE受信マスターおよび中継デバイス

---

## 1. デバイス概要

### 1.1 機能概要

- **BLE受信マスター**: is01aからのISMSv1アドバタイズ受信
- **温湿度センサー**: HT-30 (I2C) - 自己計測機能
- **MQTT送信**: クラウド直接送信（CelestialGlobe/AWS IoT等）
- **ローカルリレー**: is03aへのHTTP送信
- **MQTTステータス管理**: リモートからの状態確認・設定変更
- **常時電源**: 電源接続専用（電池非対応）

### 1.2 動作概要

is01a(Cluster Node)と同じセンサー機能を持ちつつ、BLEの受信マスターとして動作。複数のis01aからのデータを受信し、MQTTでクラウドへ直接送信、同時にis03a(LocalMQTT-Server)へもリレーする。

### 1.3 ユースケース

| 用途 | 説明 |
|------|------|
| BLEゲートウェイ | 複数のis01データを集約・中継 |
| クラウド連携 | MQTT経由でCelestialGlobeへ直接送信 |
| ローカル冗長化 | is03aへのHTTPリレーでオフライン対応 |
| 温湿度計測 | 自身のセンサーでマスター設置場所の計測 |

### 1.4 is02（ISMS版）との違い

| 項目 | is02 (ISMS版) | is02a (汎用版) |
|------|--------------|----------------|
| LacisID/CIC | **必須** | **必須** |
| AraneaRegister | **必須** | **必須** |
| TID設定 | ハードコード | NVS設定可能 |
| MQTT設定 | ハードコード | NVS設定可能 |
| リレー先 | is03固定 | 設定可能 |
| ステータス管理 | 限定的 | MQTT経由で完全対応 |
| 設定変更 | Web UIのみ | MQTT + Web UI |
| 初回セットアップ | 手動設定 | APモード→Web UI |

---

## 2. 起動フロー

### 2.1 is10同様の起動シーケンス

```
┌─────────────────────────────────────────────────────────────┐
│                    電源投入                                  │
└─────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│              WiFi設定確認                                    │
│  wifi_ssid が空？                                           │
└─────────────────────────────────────────────────────────────┘
        │                                    │
        ▼ Yes                                ▼ No
┌─────────────────────────────────────────────────────────────┐
│              APモード起動                                    │
│  SSID: "is02a-XXXX" (MACアドレス末尾)                        │
│  IP: 192.168.4.1                                            │
│  Web UI: 設定画面表示                                        │
└─────────────────────────────────────────────────────────────┘
        │ 設定保存
        ▼
┌─────────────────────────────────────────────────────────────┐
│              WiFi接続                                        │
└─────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│              AraneaDeviceGate登録                            │
│  - LacisID生成                                              │
│  - CIC取得・NVS保存                                          │
└─────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│              MQTT接続                                        │
│  - クラウドMQTTブローカー接続                                 │
│  - ステータストピック購読                                     │
└─────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│              通常動作モード                                  │
│  - BLEスキャン常時実行                                       │
│  - 自己計測（60秒間隔）                                      │
│  - バッチ送信（30秒間隔）                                    │
│  - MQTTコマンド待受                                          │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. ハードウェア仕様

### 3.1 ESP32-DevKitC GPIO割り当て

| GPIO | 機能 | 説明 |
|------|------|------|
| 21 | I2C_SDA | HT-30/OLED SDA |
| 22 | I2C_SCL | HT-30/OLED SCL |
| 25 | BTN_WIFI | WiFi再接続（3秒長押し） |
| 26 | BTN_RESET | ファクトリーリセット（5秒長押し） |

### 3.2 I2Cデバイス

| デバイス | アドレス | 用途 |
|---------|---------|------|
| HT-30 | 0x44 | 温湿度センサー（自己計測用） |
| SSD1306 | 0x3C | OLED表示（128x64） |

### 3.3 電源仕様

- **電源**: 常時電源（5V USB/ACアダプタ）
- **消費電力**: ~120mA（WiFi+BLE+MQTT同時動作時）

---

## 4. ソフトウェア設計

### 4.1 設計原則（全デバイス共通）

```
重要: ESP32では以下を遵守
- セマフォとWDTの過剰制御を避ける
- 監査系関数を入れすぎない（制御と監査で不安定化）
- 本体プログラムのタイミングとメモリ管理を重視
- コードのシンプル化
- 可能な限りシングルタスクで実装
- パーティション: min_SPIFFS使用（OTA領域確保）
```

### 4.2 メインループ

```cpp
void loop() {
    // 1. BLEスキャン結果処理（常時）
    bleScanner.processResults();

    // 2. 自己計測（60秒間隔）
    if (shouldMeasureSelf()) {
        selfTemp = sensor.readTemperature() + tempOffset;
        selfHum = sensor.readHumidity() + humOffset;
    }

    // 3. バッチ送信（30秒間隔）
    if (shouldSendBatch()) {
        sendToMqtt();       // クラウドMQTT
        sendToLocalRelay(); // is03a
    }

    // 4. MQTT処理
    mqtt.loop();

    // 5. Web UI処理
    webServer.handleClient();
}
```

---

## 5. 通信仕様

### 5.1 MQTT送信（クラウド）

**トピック**: `aranea/{tid}/device/{lacisId}/telemetry`

**ペイロード**:
```json
{
    "gateway": {
        "lacisId": "3002AABBCCDDEEFF0001",
        "cic": "123456",
        "self": {
            "temperature": 26.0,
            "humidity": 60.0
        },
        "rssi": -55,
        "ip": "192.168.1.100"
    },
    "nodes": [
        {
            "lacisId": "3001AABBCCDDEEFF0001",
            "cic": "654321",
            "temperature": 25.5,
            "humidity": 65.0,
            "battery": 85,
            "receivedAt": "2025-12-22T08:00:00Z",
            "rssi": -70
        }
    ],
    "timestamp": "2025-12-22T08:00:30Z"
}
```

### 5.2 MQTTステータス管理

**購読トピック**: `aranea/{tid}/device/{lacisId}/command`

**コマンド例**:
```json
{
    "cmd_id": "uuid-here",
    "type": "get_status"
}
```
```json
{
    "cmd_id": "uuid-here",
    "type": "set_config",
    "config": {
        "batch_interval_sec": 60,
        "self_measure_sec": 120
    }
}
```

**応答トピック**: `aranea/{tid}/device/{lacisId}/response`

```json
{
    "cmd_id": "uuid-here",
    "status": "ok",
    "data": {
        "version": "1.0.0",
        "uptime": 3600,
        "connected_nodes": 5,
        "config": {
            "batch_interval_sec": 30,
            "self_measure_sec": 60
        }
    }
}
```

### 5.3 ローカルリレー（is03a）

**エンドポイント**: `http://{is03a_ip}:8080/api/relay`

**ペイロード**: MQTT送信と同一形式

---

## 6. NVS設定項目

### 6.1 必須設定（AraneaDeviceGate用）

| キー | 型 | 説明 |
|------|-----|------|
| `gate_url` | string | AraneaDeviceGate URL |
| `tid` | string | テナントID |
| `tenant_lacisid` | string | テナントプライマリのlacisID |
| `tenant_email` | string | テナントプライマリのEmail |
| `tenant_cic` | string | テナントプライマリのCIC |
| `cic` | string | 自デバイスのCIC（取得後保存） |

### 6.2 WiFi設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `wifi_ssid` | string | "" | WiFi SSID |
| `wifi_pass` | string | "" | WiFi パスワード |

### 6.3 MQTT設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `mqtt_broker` | string | "" | MQTTブローカーURL |
| `mqtt_port` | int | 8883 | MQTTポート |
| `mqtt_user` | string | "" | MQTT認証ユーザー |
| `mqtt_pass` | string | "" | MQTT認証パスワード |
| `mqtt_use_tls` | bool | true | TLS使用 |

### 6.4 is02a固有設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `relay_url` | string | "" | is03aリレー先URL |
| `relay_url_2` | string | "" | 冗長化リレー先URL |
| `batch_interval_sec` | int | 30 | バッチ送信間隔（秒） |
| `self_measure_sec` | int | 60 | 自己計測間隔（秒） |
| `temp_offset` | float | 0.0 | 温度補正値 |
| `hum_offset` | float | 0.0 | 湿度補正値 |
| `device_name` | string | "is02a" | デバイス表示名 |

---

## 7. Web UI

### 7.1 エンドポイント

| パス | メソッド | 説明 |
|------|---------|------|
| `/` | GET | ダッシュボード |
| `/config` | GET | 設定画面 |
| `/api/status` | GET | 現在の状態取得 |
| `/api/config` | GET/POST | 設定取得/更新 |
| `/api/nodes` | GET | 受信ノード一覧 |
| `/api/reboot` | POST | 再起動 |
| `/api/factory_reset` | POST | ファクトリーリセット |

### 7.2 ダッシュボード表示

```
=== is02a Master ===
IP: 192.168.1.100 | RSSI: -55 dBm

自己計測: 26.0°C / 60.0%
接続ノード数: 5

[最近の受信]
is01a-001: 25.5°C / 65.0% (3s前)
is01a-002: 24.0°C / 70.0% (15s前)
...

MQTT: 接続中 | is03a: 接続中
```

---

## 8. 共通コンポーネント使用

| モジュール | 使用 | 備考 |
|-----------|------|------|
| WiFiManager | ○ | APモード/STA切替対応 |
| SettingManager | ○ | NVS永続化 |
| DisplayManager | ○ | I2C OLED表示 |
| NtpManager | ○ | 時刻同期 |
| MqttManager | ○ | MQTT接続管理 |
| LacisIDGenerator | **○必須** | lacisID生成 |
| AraneaRegister | **○必須** | CIC取得 |
| AraneaWebUI | ○ | Web UI基底クラス |
| HttpOtaManager | ○ | OTA更新 |
| Operator | ○ | 状態機械 |

---

## 9. is01aとの機能比較

| 機能 | is01a (Node) | is02a (Master) |
|------|--------------|----------------|
| 温湿度計測 | ○ | ○ |
| BLE発信 | ○ | × |
| BLE受信 | × | ○ |
| WiFi常時接続 | × | ○ |
| MQTT送信 | × | ○ |
| ローカルリレー | × | ○ |
| Web UI | △(設定時のみ) | ○(常時) |
| OTA | × | ○ |
| 電源 | 電池 | 常時電源 |
| DeepSleep | ○ | × |

---

## 10. 参照

- **is01a (Node)**: `code/ESP32/is01a/`
- **is02 (ISMS版)**: `archive_ISMS/ESP32/is02/is02.ino`
- **is03a (LocalMQTT)**: `code/ESP32/is03a/`
- **global モジュール**: `code/ESP32/global/src/`
- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
