# is02a - 汎用BLEゲートウェイ 設計書

**作成日**: 2025/12/22
**ベース**: archive_ISMS/ESP32/is02 (ISMS専用版)
**目的**: ISMS以外のプロジェクトでも使用可能な汎用版

---

## 1. デバイス概要

### 1.1 機能

- **BLE受信**: is01からのISMSv1アドバタイズ受信
- **HTTP中継**: 受信データをis03/クラウドへ送信
- **自己計測**: HT-30センサーによる自身の温湿度計測
- **バッチ送信**: 30秒間隔でまとめて送信（通信効率化）
- **Web UI**: 設定画面・状態表示

### 1.2 ユースケース

| 用途 | 説明 |
|------|------|
| BLEゲートウェイ | 複数のis01データを集約・中継 |
| ローカルハブ | WiFi環境とBLE環境の橋渡し |
| 冗長化 | is03と併用でデータ収集の冗長化 |

### 1.3 is02（ISMS版）との違い

| 項目 | is02 (ISMS版) | is02a (汎用版) |
|------|--------------|----------------|
| LacisID/CIC | **必須** | **必須** |
| AraneaRegister | **必須** | **必須** |
| 送信先 | is03固定（192.168.96.201/202） | 設定可能なHTTP POST先 |
| TID | 固定（市山水産） | 設定可能 |
| 設定UI | ISMS専用フィールド | 汎用設定画面 |

**重要**: LacisIDとCICはIoT動作の必須要件。AraneaRegisterで取得する。

---

## 2. ハードウェア仕様

### 2.1 ESP32-DevKitC GPIO割り当て

| GPIO | 機能 | 説明 |
|------|------|------|
| 21 | I2C_SDA | HT-30/OLED SDA |
| 22 | I2C_SCL | HT-30/OLED SCL |
| 25 | BTN_WIFI | WiFi再接続（3秒長押し） |
| 26 | BTN_RESET | ファクトリーリセット（3秒長押し） |

### 2.2 I2Cデバイス

| デバイス | アドレス | 用途 |
|---------|---------|------|
| HT-30 | 0x44 | 温湿度センサー（自己計測用） |
| SSD1306 | 0x3C | OLED表示（128x64） |

### 2.3 電源仕様

- **電源**: 常時電源（5V USB/ACアダプタ）
- **消費電力**: ~120mA（WiFi+BLE同時動作時）

---

## 3. ソフトウェア設計

### 3.1 動作モード

```
┌─────────────────────────────────────────────────────────────┐
│                      IDLE (待機)                            │
│  - BLEスキャン常時実行                                       │
│  - 受信データをバッファに蓄積                                │
│  - 自己計測（60秒間隔）                                     │
└─────────────────────────────────────────────────────────────┘
        │                                    │
        ▼ 30秒経過                           ▼ HTTPリクエスト
┌─────────────────────────────────────────────────────────────┐
│                    BATCH_SEND (バッチ送信)                  │
│  - 蓄積データをまとめてHTTP POST                            │
│  - 送信成功でバッファクリア                                  │
│  - 失敗時はリトライキューへ                                  │
└─────────────────────────────────────────────────────────────┘
        │
        ▼
      IDLE
```

### 3.2 BLEスキャン

```cpp
// ISMSv1フォーマットのみ受信
class BLEScanCallback : public BLEAdvertisedDeviceCallbacks {
    void onResult(BLEAdvertisedDevice device) override {
        String name = device.getName().c_str();

        // ISMSv1フォーマット判定（40バイト、先頭"3"）
        if (name.length() == 40 && name.startsWith("3")) {
            ISMSv1Payload payload = parseISMSv1(name);
            if (payload.isValid()) {
                bleBuffer.add(payload);
            }
        }
    }
};
```

### 3.3 ISMSv1ペイロード解析

```cpp
struct ISMSv1Payload {
    String lacisId;      // 20文字
    String cic;          // 6文字
    float temperature;   // パース済み
    float humidity;      // パース済み
    int batteryPct;      // パース済み
    unsigned long receivedAt;
};

ISMSv1Payload parseISMSv1(const String& raw) {
    ISMSv1Payload p;
    // "3" + lacisId(20) + cic(6) + temp(5) + hum(5) + batt(3)
    p.lacisId = raw.substring(1, 21);
    p.cic = raw.substring(21, 27);
    p.temperature = raw.substring(27, 32).toFloat();
    p.humidity = raw.substring(32, 37).toFloat();
    p.batteryPct = raw.substring(37, 40).toInt();
    p.receivedAt = millis();
    return p;
}
```

### 3.4 バッチ送信

```cpp
// 30秒間隔でバッチ送信
static const unsigned long BATCH_INTERVAL_MS = 30000;

void sendBatch() {
    if (bleBuffer.isEmpty()) return;

    JsonDocument doc;
    JsonArray events = doc["events"].to<JsonArray>();

    for (auto& payload : bleBuffer) {
        JsonObject event = events.add<JsonObject>();
        event["lacisId"] = payload.lacisId;
        event["cic"] = payload.cic;
        event["temperature"] = payload.temperature;
        event["humidity"] = payload.humidity;
        event["battery"] = payload.batteryPct;
        event["receivedAt"] = payload.receivedAt;
    }

    doc["gateway"]["lacisId"] = myLacisId;
    doc["gateway"]["cic"] = myCic;

    if (httpPost(relayUrl, doc)) {
        bleBuffer.clear();
    }
}
```

---

## 4. 通信仕様

### 4.1 中継データ送信（HTTP POST）

**エンドポイント**: 設定可能（NVS: `relay_url`）

**ペイロード**:
```json
{
    "auth": {
        "tid": "T2025...",
        "lacisId": "3002AABBCCDDEEFF0001",
        "cic": "123456"
    },
    "gateway": {
        "lacisId": "3002AABBCCDDEEFF0001",
        "cic": "123456",
        "rssi": -55,
        "ip": "192.168.1.100"
    },
    "events": [
        {
            "lacisId": "3001AABBCCDDEEFF0001",
            "cic": "654321",
            "temperature": 25.5,
            "humidity": 65.0,
            "battery": 85,
            "receivedAt": 1703203200000
        }
    ],
    "self": {
        "temperature": 26.0,
        "humidity": 60.0
    }
}
```

**重要**: auth.tid, auth.lacisId, auth.cic はIoT動作に必須。

### 4.2 自己計測データ

- ゲートウェイ自身のHT-30センサーで計測
- 60秒間隔で計測、バッチ送信時に含める
- `self`フィールドに格納

---

## 5. NVS設定項目

### 5.1 必須設定（AraneaRegister用）

| キー | 型 | 説明 |
|------|-----|------|
| `gate_url` | string | AraneaGateway URL |
| `tid` | string | テナントID |
| `tenant_lacisid` | string | テナントプライマリのlacisID |
| `tenant_email` | string | テナントプライマリのEmail |
| `tenant_pass` | string | テナントプライマリのパスワード |
| `tenant_cic` | string | テナントプライマリのCIC |
| `cic` | string | 自デバイスのCIC（取得後保存） |

### 5.2 WiFi設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `wifi_ssid` | string | - | WiFi SSID |
| `wifi_pass` | string | - | WiFi パスワード |

### 5.3 is02a固有設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `relay_url` | string | - | 中継先URL |
| `relay_url_2` | string | - | 中継先URL（冗長化用） |
| `batch_interval_sec` | int | 30 | バッチ送信間隔（秒） |
| `self_measure_sec` | int | 60 | 自己計測間隔（秒） |

---

## 6. Web UI

### 6.1 エンドポイント

| パス | メソッド | 説明 |
|------|---------|------|
| `/` | GET | メインUI（設定画面） |
| `/api/status` | GET | 現在の状態取得 |
| `/api/config` | GET/POST | 設定取得/更新 |
| `/api/buffer` | GET | BLEバッファ内容 |
| `/api/reboot` | POST | 再起動 |

### 6.2 状態API

```
GET /api/status

Response:
{
    "ble": {
        "scanning": true,
        "bufferedCount": 5,
        "lastReceived": "2025-12-22T08:00:00Z"
    },
    "self": {
        "temperature": 26.0,
        "humidity": 60.0,
        "lastMeasured": "2025-12-22T08:00:00Z"
    },
    "network": {
        "rssi": -55,
        "ip": "192.168.1.100"
    },
    "lastBatchSent": "2025-12-22T07:59:30Z"
}
```

---

## 7. 起動シーケンス

```
1. SettingManager初期化（NVS）
2. DisplayManager初期化（I2C OLED）
3. WiFi接続
4. NTP同期
5. LacisID生成（LacisIDGenerator）【必須】
6. AraneaGateway登録（AraneaRegister → CIC取得）【必須】
7. HT-30初期化（自己計測用）
8. BLEスキャン開始
9. HttpManager開始（Web UI）
10. OtaManager開始
11. メインループへ
```

**重要**: ステップ5-6はIoT動作に必須。CIC取得失敗時はNVS保存済みCICを使用。

---

## 8. 開発ステップ

### Phase 1: 基本動作
- [ ] WiFi接続
- [ ] HT-30自己計測
- [ ] OLED表示

### Phase 2: BLE受信
- [ ] BLEスキャン実装
- [ ] ISMSv1パース
- [ ] バッファ管理

### Phase 3: HTTP中継
- [ ] バッチ送信実装
- [ ] 冗長化（複数URL対応）
- [ ] リトライ処理

### Phase 4: 認証統合
- [ ] LacisID生成【必須】
- [ ] AraneaRegister連携【必須】
- [ ] CIC取得・保存

### Phase 5: 統合
- [ ] Web UI実装
- [ ] OTA更新
- [ ] エラーハンドリング

---

## 9. global/モジュール使用計画

| モジュール | 使用 | 備考 |
|-----------|------|------|
| WiFiManager | ○ | WiFi接続管理 |
| SettingManager | ○ | NVS永続化 |
| DisplayManager | ○ | OLED表示 |
| NtpManager | ○ | 時刻同期 |
| LacisIDGenerator | **○必須** | lacisID生成（IoT動作に必須） |
| AraneaRegister | **○必須** | CIC取得（IoT動作に必須） |
| AraneaWebUI | ○ | Web UI基底クラス |
| HttpOtaManager | ○ | OTA更新 |
| Operator | ○ | 状態機械 |

---

## 10. 参照

- **is01 (BLE送信側)**: `code/ESP32/is01a/`
- **is02 (ISMS版)**: `archive_ISMS/ESP32/is02/is02.ino`
- **is03 (受信サーバー)**: `code/ESP32/is03/`
- **global モジュール**: `code/ESP32/global/src/`
- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
