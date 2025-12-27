# is06a - Relay Controller 設計書

**正式名称**: Aranea 6-Channel Relay Controller
**製品コード**: AR-IS06A
**作成日**: 2025/12/22
**目的**: 6chトリガー出力（PWM対応）+ 2ch入出力切替デバイス

---

## 1. デバイス概要

### 1.1 機能概要

- **6chトリガー出力**: GPIO 07, 12, 14, 27, 05, 18
- **PWM対応**: GPIO 07, 12, 14, 27 はPWM出力可能
- **入出力切替**: GPIO 05, 18 はトリガー/インプット切替可能
- **名称設定**: 各トリガーの名称変更
- **モード設定**: Digital I/O と PWM の切替
- **デバウンス**: 05, 18 のインプット時のみ

### 1.2 is04aとの違い

| 項目 | is04a | is06a |
|------|-------|-------|
| 出力チャンネル | 2ch | 6ch |
| PWM対応 | なし | 4ch (07,12,14,27) |
| 入出力切替 | なし | 2ch (05,18) |
| 物理入力スイッチ | あり | なし（05,18で代替） |
| トリガーアサイン | 逆転のみ | 任意名称設定 |

### 1.3 ユースケース

| 用途 | 説明 |
|------|------|
| LED調光 | PWM制御でLED明るさ調整 |
| モーター制御 | PWMでファン速度制御 |
| 複数リレー制御 | 6系統のリレー駆動 |
| 警報システム | 複数出力デバイスの統合制御 |

---

## 2. ハードウェア仕様

### 2.1 GPIO割り当て

| GPIO | トリガー | PWM | I/O切替 | 説明 |
|------|---------|-----|---------|------|
| 07 | TRG1 | ○ | - | トリガー1（PWM対応） |
| 12 | TRG2 | ○ | - | トリガー2（PWM対応） |
| 14 | TRG3 | ○ | - | トリガー3（PWM対応） |
| 27 | TRG4 | ○ | - | トリガー4（PWM対応） |
| 05 | TRG5 | - | ○ | トリガー5 or インプット |
| 18 | TRG6 | - | ○ | トリガー6 or インプット |
| 21 | - | - | - | I2C_SDA (OLED) |
| 22 | - | - | - | I2C_SCL (OLED) |
| 25 | - | - | - | BTN_WIFI |
| 26 | - | - | - | BTN_RESET |

### 2.2 PWM仕様

- **周波数**: 1kHz〜40kHz（設定可能）
- **解像度**: 8bit（0-255）or 10bit（0-1023）
- **チャンネル**: LEDC ch0-ch3 使用

### 2.3 I/O切替仕様（GPIO 05, 18）

| モード | 説明 |
|--------|------|
| OUTPUT | トリガー出力（パルス/レベル） |
| INPUT | 接点入力（デバウンス対応） |

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

### 3.2 PWMコントローラー（共通モジュール）

```cpp
// PWMController.h
class PWMController {
public:
    void begin(int pin, int channel);
    void setFrequency(int hz);     // 1000-40000
    void setResolution(int bits);  // 8 or 10
    void setDuty(int duty);        // 0-255 (8bit) or 0-1023 (10bit)
    void setDutyPercent(float pct); // 0.0-100.0%

    // フェード機能
    void fadeTo(int targetDuty, int durationMs);
    void update();  // loop()で呼び出し

private:
    int pin_;
    int channel_;
    int resolution_;
    int currentDuty_;
    // フェード用
    int fadeTargetDuty_;
    unsigned long fadeStartMs_;
    unsigned long fadeDurationMs_;
};
```

### 3.3 I/Oコントローラー（共通モジュール）

```cpp
// IOController.h
class IOController {
public:
    enum Mode { INPUT, OUTPUT, PWM };

    void begin(int pin);
    void setMode(Mode mode);
    void setDebounceMs(int ms);  // 入力時のみ

    // デジタル出力
    void setOutput(bool high);
    void pulse(int durationMs);

    // デジタル入力
    void sample();
    bool hasChanged() const;
    bool isActive() const;

    void update();  // loop()

private:
    int pin_;
    Mode mode_;
    // ...
};
```

---

## 4. NVS設定項目

### 4.1 必須設定（AraneaDeviceGate用）

| キー | 型 | 説明 |
|------|-----|------|
| `gate_url` | string | AraneaDeviceGate URL |
| `tid` | string | テナントID |
| `tenant_lacisid` | string | テナントプライマリのlacisID |
| `tenant_email` | string | テナントプライマリのEmail |
| `tenant_cic` | string | テナントプライマリのCIC |
| `cic` | string | 自デバイスのCIC |

### 4.2 トリガー設定（trg1〜trg6）

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `trg{N}_name` | string | "trigger{N}" | トリガー名称 |
| `trg{N}_mode` | string | "digital" | "digital" or "pwm" |
| `trg{N}_pulse_ms` | int | 3000 | パルス幅（digital時） |
| `trg{N}_pwm_freq` | int | 1000 | PWM周波数 |
| `trg{N}_pwm_duty` | int | 128 | PWMデューティ（0-255） |

### 4.3 I/O切替設定（trg5, trg6）

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `trg5_io_mode` | string | "output" | "input" or "output" |
| `trg6_io_mode` | string | "output" | "input" or "output" |
| `trg5_debounce` | int | 100 | 入力時デバウンス（ms） |
| `trg6_debounce` | int | 100 | 入力時デバウンス（ms） |

---

## 5. Web UI

### 5.1 エンドポイント

| パス | メソッド | 説明 |
|------|---------|------|
| `/` | GET | ダッシュボード |
| `/config` | GET | 設定画面 |
| `/api/status` | GET | 現在の状態 |
| `/api/config` | GET/POST | 設定取得/更新 |
| `/api/trigger` | POST | トリガー実行 |
| `/api/pwm` | POST | PWM値設定 |
| `/api/reboot` | POST | 再起動 |

### 5.2 トリガーAPI

```json
POST /api/trigger
{
    "trigger": 1,
    "action": "pulse",
    "duration_ms": 3000
}

POST /api/pwm
{
    "trigger": 1,
    "duty": 128
}
// or
{
    "trigger": 1,
    "duty_percent": 50.0
}
```

---

## 6. 共通コンポーネント使用

| モジュール | 使用 | 備考 |
|-----------|------|------|
| WiFiManager | ○ | APモード/STA切替対応 |
| SettingManager | ○ | NVS永続化 |
| DisplayManager | ○ | I2C OLED表示 |
| NtpManager | ○ | 時刻同期 |
| MqttManager | ○ | MQTT接続 |
| LacisIDGenerator | **○必須** | lacisID生成 |
| AraneaRegister | **○必須** | CIC取得 |
| AraneaWebUI | ○ | Web UI基底クラス |
| HttpOtaManager | ○ | OTA更新 |
| IOController | ○ | **共通モジュール** |
| PWMController | ○ | **新規共通モジュール** |
| Operator | ○ | 状態機械 |

---

## 7. 開発ステップ

### Phase 1: 基本動作
- [ ] 6ch デジタル出力
- [ ] パルス出力
- [ ] OLED表示

### Phase 2: PWM
- [ ] PWMController実装
- [ ] 4ch PWM出力
- [ ] フェード機能

### Phase 3: I/O切替
- [ ] GPIO 05, 18 の I/O切替
- [ ] デバウンス（入力時）

### Phase 4: 統合
- [ ] Web UI
- [ ] MQTT制御
- [ ] OTA更新

---

## 8. 参照

- **is04a (2ch版)**: `code/ESP32/is04a/`
- **IOController**: `code/ESP32/global/src/IOController.h`
- **PWMController**: `code/ESP32/global/src/PWMController.h`
- **global モジュール**: `code/ESP32/global/src/`
