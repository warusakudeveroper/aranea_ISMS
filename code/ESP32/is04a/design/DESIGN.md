# is04a - 汎用2ch接点出力デバイス 設計書

**作成日**: 2025/12/22
**ベース**: archive_ISMS/ESP32/is04 (ISMS専用版)
**目的**: ISMS以外のプロジェクトでも使用可能な汎用版

---

## 1. デバイス概要

### 1.1 機能

- **2ch物理入力**: 手動トリガー（ボタン/スイッチ）
- **2ch接点出力**: リレー/SSR駆動用パルス出力
- **双方向制御**: ローカル（物理入力/HTTP）+ リモート（MQTT/HTTP）
- **状態レポート**: 状態変化時にHTTP POSTで送信

### 1.2 ユースケース

| 用途 | 説明 |
|------|------|
| 窓開閉制御 | 電動窓の開/閉パルス出力 |
| 照明制御 | ON/OFFスイッチ |
| ドアロック | 電気錠の施錠/解錠 |
| 汎用リレー制御 | 任意の2ch接点出力 |

### 1.3 is04（ISMS版）との違い

| 項目 | is04 (ISMS版) | is04a (汎用版) |
|------|--------------|----------------|
| LacisID/CIC | **必須** | **必須** |
| AraneaRegister | **必須** | **必須** |
| 送信先 | Firestore (deviceStateReport) | 設定可能なHTTP POST先 |
| MQTT | WebSocket + TLS (クラウド専用) | 汎用MQTT Broker対応 |
| 設定UI | ISMS専用フィールド | 汎用設定画面 |
| TID | 固定（市山水産） | 設定可能 |

**重要**: LacisIDとCICはIoT動作の必須要件。AraneaRegisterで取得する。

---

## 2. ハードウェア仕様

### 2.1 GPIO割り当て（ESP32-DevKitC）

| GPIO | 機能 | 説明 |
|------|------|------|
| 5 | PHYS_IN1 | 物理入力1（INPUT_PULLDOWN） |
| 18 | PHYS_IN2 | 物理入力2（INPUT_PULLDOWN） |
| 12 | TRG_OUT1 | トリガー出力1（デフォルトOPEN） |
| 14 | TRG_OUT2 | トリガー出力2（デフォルトCLOSE） |
| 25 | BTN_WIFI | WiFi再接続ボタン（3秒長押し） |
| 26 | BTN_RESET | ファクトリーリセット（3秒長押し） |
| 21 | I2C_SDA | OLED SDA |
| 22 | I2C_SCL | OLED SCL |

### 2.2 ピン制約

- **GPIO12**: ストラッピングピン。起動時LOWを維持（出力デフォルトLOW）
- **GPIO5**: INPUT_PULLDOWN使用（外部プルアップ不要）

### 2.3 出力仕様

- **出力形式**: パルス出力（設定可能なパルス幅）
- **パルス幅**: 10ms〜10,000ms（デフォルト3000ms）
- **インターロック**: 200ms（同時出力防止）
- **電気的仕様**: 3.3V CMOS出力（リレーモジュール駆動前提）

---

## 3. ソフトウェア設計

### 3.1 状態機械

```
┌─────────────────────────────────────────────────────────────┐
│                      IDLE (待機)                            │
│  - 物理入力監視（デバウンス20ms×3回）                        │
│  - MQTT/HTTPコマンド待ち                                     │
└─────────────────────────────────────────────────────────────┘
        │                                    │
        ▼ 物理入力エッジ                      ▼ リモートコマンド
┌─────────────────────────────────────────────────────────────┐
│                    PULSE_ACTIVE (パルス中)                   │
│  - 出力ピンHIGH                                              │
│  - タイマー監視（非ブロッキング）                            │
│  - 追加トリガー拒否                                          │
└─────────────────────────────────────────────────────────────┘
        │
        ▼ パルス終了
┌─────────────────────────────────────────────────────────────┐
│                    STATE_REPORT (状態送信)                   │
│  - HTTP POST送信                                             │
│  - MQTT ACK送信（コマンド起因時）                            │
└─────────────────────────────────────────────────────────────┘
        │
        ▼
      IDLE
```

### 3.2 デバウンス処理

```cpp
// 20ms間隔でサンプリング、3回連続一致で確定
static const unsigned long SAMPLE_INTERVAL_MS = 20;
static const int STABLE_COUNT = 3;

void samplePhysicalInputs() {
    int current = digitalRead(PHYS_IN1);
    if (current == rawState) {
        if (++stableCount >= STABLE_COUNT && current != stableState) {
            stableState = current;  // 状態確定
            // 立ち上がりエッジでパルス開始
        }
    } else {
        rawState = current;
        stableCount = 0;
    }
}
```

### 3.3 パルス出力（非ブロッキング）

```cpp
// loop()内で呼び出し
void updatePulse() {
    if (!pulseActive) return;
    if (millis() - pulseStartMs >= pulseDurationMs) {
        digitalWrite(pulseOutPin, LOW);
        pulseActive = false;
        needSendState = true;
    }
}
```

---

## 4. 通信仕様

### 4.1 状態レポート（HTTP POST）

**エンドポイント**: 設定可能（NVS: `state_report_url`）

**ペイロード**:
```json
{
    "deviceId": "is04a-AABBCCDDEEFF",
    "timestamp": "2025-12-22T08:00:00Z",
    "state": {
        "output1": false,
        "output2": false,
        "input1": true,
        "input2": false,
        "rssi": -55,
        "ip": "192.168.1.100"
    },
    "event": {
        "type": "pulse",
        "source": "manual",
        "pin": 12,
        "durationMs": 3000
    }
}
```

### 4.2 コマンド受信（MQTT）

**トピック**: `device/{deviceId}/command`

**ペイロード**:
```json
{
    "cmd_id": "uuid-here",
    "type": "pulse",
    "parameters": {
        "output": 1,
        "durationMs": 3000
    }
}
```

### 4.3 ACK送信（MQTT）

**トピック**: `device/{deviceId}/ack`

**ペイロード**:
```json
{
    "cmd_id": "uuid-here",
    "status": "executed",
    "executedAt": "2025-12-22T08:00:00Z",
    "result": {
        "ok": true,
        "pin": 12,
        "durationMs": 3000
    }
}
```

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

### 5.3 is04a固有設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `state_report_url` | string | - | 状態送信先URL |
| `mqtt_broker` | string | - | MQTTブローカーURL |
| `output1_name` | string | "OPEN" | 出力1の名前 |
| `output2_name` | string | "CLOSE" | 出力2の名前 |
| `pulse_ms` | int | 3000 | パルス幅（ms） |
| `interlock_ms` | int | 200 | インターロック時間（ms） |
| `input1_action` | string | "output1" | 入力1のアクション |
| `input2_action` | string | "output2" | 入力2のアクション |

---

## 6. Web UI

### 6.1 エンドポイント

| パス | メソッド | 説明 |
|------|---------|------|
| `/` | GET | メインUI（設定画面） |
| `/api/status` | GET | 現在の状態取得 |
| `/api/config` | GET/POST | 設定取得/更新 |
| `/api/pulse` | POST | 手動パルス実行 |
| `/api/reboot` | POST | 再起動 |

### 6.2 手動パルスAPI

```
POST /api/pulse
Content-Type: application/json

{
    "output": 1,
    "durationMs": 3000
}
```

---

## 7. 起動シーケンス

```
1. GPIO初期化（出力ピンをLOWに設定）
2. SettingManager初期化（NVS）
3. DisplayManager初期化（I2C OLED）
4. WiFi接続
5. NTP同期
6. LacisID生成（LacisIDGenerator）
7. AraneaGateway登録（AraneaRegister → CIC取得）
8. MQTT接続（設定されている場合）
9. HttpManager開始（Web UI）
10. OtaManager開始
11. メインループへ
```

**重要**: ステップ6-7はIoT動作に必須。CIC取得失敗時はNVS保存済みCICを使用。

---

## 8. 開発ステップ

### Phase 1: 基本動作
- [ ] GPIO入出力動作確認
- [ ] デバウンス処理実装
- [ ] パルス出力（非ブロッキング）
- [ ] OLED表示

### Phase 2: 通信
- [ ] WiFi接続
- [ ] HTTP POST状態送信
- [ ] Web UI実装

### Phase 3: リモート制御
- [ ] MQTT受信
- [ ] コマンド処理
- [ ] ACK送信

### Phase 4: 統合
- [ ] OTA更新
- [ ] 設定永続化
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
| MqttManager | ○ | MQTT接続 |
| Operator | ○ | 状態機械 |

---

## 10. 参照

- **is04 (ISMS版)**: `archive_ISMS/ESP32/is04/is04.ino`
- **is10 (参考実装)**: `code/ESP32/is10/`
- **global モジュール**: `code/ESP32/global/src/`
- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
