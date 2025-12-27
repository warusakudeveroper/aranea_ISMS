# is05a - 8ch Detector 設計書

**正式名称**: Aranea 8-Channel Detector
**製品コード**: AR-IS05A
**作成日**: 2025/12/22
**ベース**: archive_ISMS/ESP32/is05 (ISMS専用版)
**目的**: 汎用8ch検出器（リードスイッチ/接点入力）+ Webhook通知 + トリガー出力

---

## 1. デバイス概要

### 1.1 機能概要

- **8ch入力**: リードスイッチ/接点入力（GPIO）
- **デバウンス**: 5ms〜10,000ms（設定可能）
- **状態送信**: 変化時にHTTP POST送信
- **Webhook通知**: Discord/Slack/Generic対応
- **トリガー出力**: ch7/ch8をトリガー出力として使用可能
- **心拍送信**: 設定間隔で定期送信

### 1.2 is05（ISMS版）との違い

| 項目 | is05 (ISMS版) | is05a (汎用版) |
|------|--------------|----------------|
| LacisID/CIC | **必須** | **必須** |
| チャンネル名 | 固定 | 各ch名称設定可能 |
| デバウンス | 固定(20ms×5回) | 5ms〜10,000ms設定可能 |
| Webhook | なし | Discord/Slack/Generic対応 |
| ch7/ch8出力 | なし | トリガー出力切替可能 |
| 初回セットアップ | 手動設定 | APモード→Web UI |

### 1.3 ユースケース

| 用途 | 説明 |
|------|------|
| 窓/ドア監視 | リードスイッチで開閉検知 |
| 設備状態監視 | 接点信号入力 |
| セキュリティ | 侵入検知→Webhook通知 |
| 警報連動 | 検出→ch7/ch8でリレー駆動（回転灯、ブザー） |

---

## 2. ハードウェア仕様

### 2.1 GPIO割り当て

| GPIO | 機能 | 説明 |
|------|------|------|
| 4 | CH1 | 入力（INPUT_PULLUP） |
| 5 | CH2 | 入力（INPUT_PULLUP） |
| 13 | CH3 | 入力（INPUT_PULLUP） |
| 14 | CH4 | 入力（INPUT_PULLUP） |
| 16 | CH5 | 入力（INPUT_PULLUP） |
| 17 | CH6 | 入力（INPUT_PULLUP） |
| 18 | CH7 | 入力/出力 切替可能 |
| 19 | CH8 | 入力/出力 切替可能 |
| 21 | I2C_SDA | OLED SDA |
| 22 | I2C_SCL | OLED SCL |
| 25 | BTN_WIFI | WiFi再接続（3秒長押し） |
| 26 | BTN_RESET | ファクトリーリセット（5秒長押し） |

### 2.2 ch7/ch8 I/O切替

```cpp
// ch7/ch8はI/O切替可能
enum PinMode {
    MODE_INPUT,   // 検出入力（デフォルト）
    MODE_OUTPUT   // トリガー出力
};

// 使用例: 検出時に回転灯を回す
if (ch1.changed() && ch1.isActive()) {
    ch7.pulse(3000);  // 3秒間トリガー出力
}
```

---

## 3. ソフトウェア設計

### 3.1 設計原則（全デバイス共通）

```
重要: ESP32では以下を遵守
- セマフォとWDTの過剰制御を避ける
- 監査系関数を入れすぎない
- コードのシンプル化
- 可能な限りシングルタスクで実装
- パーティション: min_SPIFFS使用
```

### 3.2 デバウンス処理

```cpp
// デバウンス設定（5ms〜10,000ms）
class IOController {
public:
    void setDebounceMs(int ch, int ms);  // 5-10000ms
    void sample();  // loop()で呼び出し

private:
    // ESP32側でデバウンス処理
    int debounceMs_[8];
    unsigned long lastChangeMs_[8];
    int stableState_[8];
};
```

### 3.3 I/Oコントローラー（共通モジュール）

```cpp
// IOController.h - 共通モジュールとして作成
class IOController {
public:
    enum Mode { INPUT, OUTPUT };

    void begin(int pin);
    void setMode(Mode mode);
    void setDebounceMs(int ms);  // 5-10000ms
    void setInverted(bool inverted);

    // 入力
    void sample();
    bool hasChanged() const;
    bool isActive() const;
    String getStateString() const;

    // 出力
    void setOutput(bool high);
    void pulse(int durationMs);
    void update();  // パルス終了チェック

private:
    int pin_;
    Mode mode_;
    int debounceMs_;
    bool inverted_;
    // ...
};
```

---

## 4. Webhook通知

### 4.1 対応プラットフォーム

| プラットフォーム | 形式 |
|----------------|------|
| Discord | Discord Webhook形式 |
| Slack | Slack Incoming Webhook形式 |
| Generic | カスタムJSON POST |

### 4.2 Discord Webhook

```json
POST {discord_webhook_url}
{
    "content": "🚨 **窓1** が **開** になりました",
    "embeds": [{
        "title": "is05a 状態変化",
        "fields": [
            {"name": "チャンネル", "value": "窓1 (ch1)", "inline": true},
            {"name": "状態", "value": "開", "inline": true},
            {"name": "時刻", "value": "2025-12-22 08:00:00", "inline": false}
        ],
        "color": 15158332
    }]
}
```

### 4.3 Slack Webhook

```json
POST {slack_webhook_url}
{
    "text": "🚨 *窓1* が *開* になりました",
    "attachments": [{
        "color": "danger",
        "fields": [
            {"title": "チャンネル", "value": "窓1 (ch1)", "short": true},
            {"title": "状態", "value": "開", "short": true}
        ]
    }]
}
```

### 4.4 Generic Webhook

```json
POST {generic_webhook_url}
{
    "device_id": "is05a-AABBCCDDEEFF",
    "lacis_id": "3005AABBCCDDEEFF0001",
    "event": "state_change",
    "channel": 1,
    "channel_name": "窓1",
    "state": "open",
    "timestamp": "2025-12-22T08:00:00Z"
}
```

---

## 5. NVS設定項目

### 5.1 必須設定（AraneaDeviceGate用）

| キー | 型 | 説明 |
|------|-----|------|
| `gate_url` | string | AraneaDeviceGate URL |
| `tid` | string | テナントID |
| `tenant_lacisid` | string | テナントプライマリのlacisID |
| `tenant_email` | string | テナントプライマリのEmail |
| `tenant_cic` | string | テナントプライマリのCIC |
| `cic` | string | 自デバイスのCIC |

### 5.2 チャンネル設定（ch1〜ch8）

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `ch{N}_name` | string | "ch{N}" | チャンネル名称 |
| `ch{N}_meaning` | string | "open" | アクティブ時の意味 |
| `ch{N}_debounce` | int | 100 | デバウンス（5-10000ms） |
| `ch{N}_inverted` | bool | false | 論理反転 |

### 5.3 ch7/ch8 I/O設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `ch7_mode` | string | "input" | "input" or "output" |
| `ch8_mode` | string | "input" | "input" or "output" |
| `ch7_pulse_ms` | int | 3000 | 出力時パルス幅 |
| `ch8_pulse_ms` | int | 3000 | 出力時パルス幅 |

### 5.4 Webhook設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `webhook_discord` | string | "" | Discord Webhook URL |
| `webhook_slack` | string | "" | Slack Webhook URL |
| `webhook_generic` | string | "" | Generic Webhook URL |
| `webhook_enabled` | bool | false | Webhook有効化 |

---

## 6. Web UI

### 6.1 エンドポイント

| パス | メソッド | 説明 |
|------|---------|------|
| `/` | GET | ダッシュボード |
| `/config` | GET | 設定画面 |
| `/api/status` | GET | 現在の状態 |
| `/api/config` | GET/POST | 設定取得/更新 |
| `/api/channels` | GET | 全チャンネル状態 |
| `/api/pulse` | POST | 手動トリガー出力 |
| `/api/reboot` | POST | 再起動 |

### 6.2 ダッシュボード

```
=== is05a 8ch Detector ===
IP: 192.168.1.100 | RSSI: -55 dBm

[チャンネル状態]
ch1 窓1:     ● 開   (5s前)
ch2 窓2:     ○ 閉
ch3 ドア1:   ○ 閉
ch4 ドア2:   ○ 閉
ch5 センサー1: ○ OFF
ch6 センサー2: ○ OFF
ch7 警報灯:   → 出力モード
ch8 ブザー:   → 出力モード

[Webhook] Discord: 有効 | Slack: 無効
```

---

## 7. 共通コンポーネント使用

| モジュール | 使用 | 備考 |
|-----------|------|------|
| WiFiManager | ○ | APモード/STA切替対応 |
| SettingManager | ○ | NVS永続化 |
| DisplayManager | ○ | I2C OLED表示 |
| NtpManager | ○ | 時刻同期 |
| LacisIDGenerator | **○必須** | lacisID生成 |
| AraneaRegister | **○必須** | CIC取得 |
| AraneaWebUI | ○ | Web UI基底クラス |
| HttpOtaManager | ○ | OTA更新 |
| IOController | ○ | **新規共通モジュール** |
| Operator | ○ | 状態機械 |

---

## 8. 開発ステップ

### Phase 1: 基本動作
- [ ] 8ch入力動作確認
- [ ] デバウンス処理（5-10000ms）
- [ ] OLED表示

### Phase 2: I/O切替
- [ ] IOControllerクラス実装
- [ ] ch7/ch8のI/O切替
- [ ] パルス出力

### Phase 3: 通信
- [ ] HTTP状態送信
- [ ] Webhook（Discord/Slack/Generic）
- [ ] Web UI

### Phase 4: 統合
- [ ] LacisID/CIC取得
- [ ] OTA更新
- [ ] APモード設定

---

## 9. 参照

- **is05 (ISMS版)**: `archive_ISMS/ESP32/is05/is05.ino`
- **IOController**: `code/ESP32/global/src/IOController.h`
- **global モジュール**: `code/ESP32/global/src/`
- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
