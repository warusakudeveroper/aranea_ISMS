# is05a - 汎用8chリードスイッチセンサー 設計書

**作成日**: 2025/12/22
**ベース**: archive_ISMS/ESP32/is05 (ISMS専用版)
**目的**: ISMS以外のプロジェクトでも使用可能な汎用版

---

## 1. デバイス概要

### 1.1 機能

- **8ch入力**: リードスイッチ/接点入力（GPIO）
- **デバウンス**: 20ms×5回で状態確定
- **状態送信**: 変化時にHTTP POST送信
- **心拍送信**: 60秒間隔で定期送信

### 1.2 ユースケース

| 用途 | 説明 |
|------|------|
| 窓/ドア監視 | 開閉状態検知 |
| 設備状態監視 | 接点信号入力 |
| セキュリティ | 侵入検知 |
| 環境監視 | 接点センサー連携 |

### 1.3 is05（ISMS版）との違い

| 項目 | is05 (ISMS版) | is05a (汎用版) |
|------|--------------|----------------|
| LacisID/CIC | **必須** | **必須** |
| AraneaRegister | **必須** | **必須** |
| 送信先 | is03 + Firestore | 設定可能なHTTP POST先 |
| TID | 固定（市山水産） | 設定可能 |
| 設定UI | ISMS専用フィールド | 汎用設定画面 |

**重要**: LacisIDとCICはIoT動作の必須要件。AraneaRegisterで取得する。

---

## 2. ハードウェア仕様

### 2.1 GPIO割り当て（ESP32-DevKitC）

| GPIO | 機能 | 説明 |
|------|------|------|
| 設定可能 | CH1〜CH8 | リードスイッチ入力（INPUT_PULLUP） |
| 25 | BTN_WIFI | WiFi再接続ボタン（3秒長押し） |
| 26 | BTN_RESET | ファクトリーリセット（3秒長押し） |
| 21 | I2C_SDA | OLED SDA |
| 22 | I2C_SCL | OLED SCL |

### 2.2 推奨GPIO割り当て

| チャンネル | 推奨GPIO | 備考 |
|-----------|---------|------|
| CH1 | 4 | 安全 |
| CH2 | 5 | 安全 |
| CH3 | 13 | 安全 |
| CH4 | 14 | 安全 |
| CH5 | 16 | 安全 |
| CH6 | 17 | 安全 |
| CH7 | 18 | 安全 |
| CH8 | 19 | 安全 |

### 2.3 ピン制約

- **GPIO2, 15**: ストラッピングピン。起動時にLOW出力で安定化後INPUT_PULLUPに変更
- **GPIO34-39**: 入力専用（プルアップなし）- 外部プルアップ必要

### 2.4 入力仕様

- **入力形式**: INPUT_PULLUP（アクティブLOW）
- **デバウンス**: 20ms間隔×5回連続一致で確定
- **起動猶予**: 5秒間は状態変化を送信しない（初期ノイズ対策）

---

## 3. ソフトウェア設計

### 3.1 状態機械

```
┌─────────────────────────────────────────────────────────────┐
│                      IDLE (待機)                            │
│  - 8ch入力監視（デバウンス20ms×5回）                         │
│  - 心拍タイマー監視（60秒）                                  │
└─────────────────────────────────────────────────────────────┘
        │                                    │
        ▼ 状態変化検出                        ▼ 心拍タイマー
┌─────────────────────────────────────────────────────────────┐
│                    SEND_STATE (状態送信)                     │
│  - HTTP POST送信（ローカル + クラウド）                      │
│  - 最小送信間隔1秒でスロットリング                           │
└─────────────────────────────────────────────────────────────┘
        │
        ▼
      IDLE
```

### 3.2 デバウンス処理

```cpp
// 20ms間隔でサンプリング、5回連続一致で確定
static const unsigned long SAMPLE_INTERVAL_MS = 20;
static const int STABLE_COUNT = 5;

void sampleInputs() {
    for (int i = 0; i < NUM_CHANNELS; i++) {
        int current = digitalRead(chPins[i]);
        if (current == rawState[i]) {
            if (++stableCount_[i] >= STABLE_COUNT && current != stableState[i]) {
                stableState[i] = current;
                lastUpdatedAt[i] = ntp.getIso8601();
                needSend = true;
            }
        } else {
            rawState[i] = current;
            stableCount_[i] = 0;
        }
    }
}
```

### 3.3 meaning変換

```cpp
// activeLow: LOW=active, HIGH=inactive
String getStateString(int ch) {
    bool isActive = (stableState[ch] == LOW);
    if (isActive) {
        return chMeanings[ch];  // "open" or "close"
    } else {
        return (chMeanings[ch] == "open") ? "close" : "open";
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
    "auth": {
        "tid": "T2025...",
        "lacisId": "3005AABBCCDDEEFF0001",
        "cic": "123456"
    },
    "report": {
        "lacisId": "3005AABBCCDDEEFF0001",
        "type": "ar-is05a",
        "observedAt": "2025-12-22T08:00:00Z",
        "state": {
            "ch1": "open",
            "ch2": "close",
            "ch3": "open",
            "ch4": "close",
            "ch5": "open",
            "ch6": "close",
            "ch7": "open",
            "ch8": "close"
        }
    }
}
```

**重要**: auth.tid, auth.lacisId, auth.cic はIoT動作に必須。

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

### 5.3 is05a固有設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `state_report_url` | string | - | 状態送信先URL |
| `is05a_ch1_pin` | int | 4 | CH1のGPIO番号 |
| `is05a_ch1_name` | string | "ch1" | CH1の名前 |
| `is05a_ch1_meaning` | string | "open" | CH1のアクティブ時の意味 |
| ... | ... | ... | CH2〜CH8も同様 |

---

## 6. Web UI

### 6.1 エンドポイント

| パス | メソッド | 説明 |
|------|---------|------|
| `/` | GET | メインUI（設定画面） |
| `/api/status` | GET | 現在の状態取得 |
| `/api/config` | GET/POST | 設定取得/更新 |
| `/api/reboot` | POST | 再起動 |

### 6.2 状態API

```
GET /api/status

Response:
{
    "channels": [
        {"name": "窓1", "state": "open", "pin": 4, "raw": 0},
        {"name": "窓2", "state": "close", "pin": 5, "raw": 1},
        ...
    ],
    "rssi": -55,
    "ip": "192.168.1.100",
    "uptime": 3600
}
```

---

## 7. 起動シーケンス

```
1. ストラッピングピン安定化（GPIO2, 15をLOW出力）
2. SettingManager初期化（NVS）
3. DisplayManager初期化（I2C OLED）
4. WiFi接続
5. NTP同期
6. LacisID生成（LacisIDGenerator）
7. AraneaGateway登録（AraneaRegister → CIC取得）
8. チャンネル設定読み込み
9. GPIO入力初期化
10. HttpManager開始（Web UI）
11. OtaManager開始
12. 起動猶予期間開始（5秒）
13. メインループへ
```

**重要**: ステップ6-7はIoT動作に必須。CIC取得失敗時はNVS保存済みCICを使用。

---

## 8. 開発ステップ

### Phase 1: 基本動作
- [ ] GPIO入力動作確認
- [ ] デバウンス処理実装
- [ ] 8ch状態管理
- [ ] OLED表示

### Phase 2: 通信
- [ ] WiFi接続
- [ ] HTTP POST状態送信
- [ ] Web UI実装

### Phase 3: 認証統合
- [ ] LacisID生成
- [ ] AraneaRegister連携
- [ ] CIC取得・保存

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
| Operator | ○ | 状態機械 |

---

## 10. 参照

- **is05 (ISMS版)**: `archive_ISMS/ESP32/is05/is05.ino`
- **is10 (参考実装)**: `code/ESP32/is10/`
- **global モジュール**: `code/ESP32/global/src/`
- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
