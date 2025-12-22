# is04a - Window & Door Controller 設計書

**正式名称**: Aranea Window & Door Controller
**製品コード**: AR-IS04A
**作成日**: 2025/12/22
**ベース**: archive_ISMS/ESP32/is04 (ISMS専用版)
**目的**: is06aの2ch限定・物理接点スイッチ付きモデル

---

## 1. デバイス概要

### 1.1 機能概要

- **2ch物理入力**: 手動トリガー（ボタン/スイッチ）
- **2ch接点出力**: リレー/SSR駆動用パルス出力
- **双方向制御**: ローカル（物理入力/HTTP）+ リモート（MQTT/HTTP）
- **状態レポート**: 状態変化時にHTTP POSTで送信
- **物理スイッチ連動**: 入力→出力の直接連動

### 1.2 is06aとの関係

is04aはis06aの2ch限定版で、物理接点スイッチ付きモデル。
- is04a: 2ch入力 + 2ch出力 + 物理スイッチ
- is06a: 6ch出力（PWM対応） + 2ch入出力切替

### 1.3 is04（ISMS版）との違い

| 項目 | is04 (ISMS版) | is04a (汎用版) |
|------|--------------|----------------|
| LacisID/CIC | **必須** | **必須** |
| TID設定 | ハードコード | NVS設定可能 |
| 出力名称 | 固定(OPEN/CLOSE) | 設定可能 |
| トリガーアサイン | 逆転のみ | 任意アサイン |
| 初回セットアップ | 手動設定 | APモード→Web UI |

### 1.4 ユースケース

| 用途 | 説明 |
|------|------|
| 窓開閉制御 | 電動窓の開/閉パルス出力 |
| 照明制御 | ON/OFFスイッチ |
| ドアロック | 電気錠の施錠/解錠 |
| 汎用リレー制御 | 任意の2ch接点出力 |

---

## 2. ハードウェア仕様

### 2.1 GPIO割り当て

| GPIO | 機能 | 説明 |
|------|------|------|
| 5 | PHYS_IN1 | 物理入力1（INPUT_PULLDOWN） |
| 18 | PHYS_IN2 | 物理入力2（INPUT_PULLDOWN） |
| 12 | TRG_OUT1 | トリガー出力1 |
| 14 | TRG_OUT2 | トリガー出力2 |
| 21 | I2C_SDA | OLED SDA |
| 22 | I2C_SCL | OLED SCL |
| 25 | BTN_WIFI | WiFi再接続（3秒長押し） |
| 26 | BTN_RESET | ファクトリーリセット（5秒長押し） |

### 2.2 出力仕様

- **出力形式**: パルス出力（設定可能なパルス幅）
- **パルス幅**: 10ms〜10,000ms（デフォルト3000ms）
- **インターロック**: 200ms（同時出力防止）
- **電気的仕様**: 3.3V CMOS出力（リレーモジュール駆動前提）

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

### 3.2 トリガーアサイン

```cpp
// トリガーアサイン設定
// 入力1を押したら出力1 or 出力2を動作
struct TriggerAssign {
    int inputPin;
    int outputPin;
    int pulseDurationMs;
};

// 設定例
TriggerAssign assigns[] = {
    {PHYS_IN1, TRG_OUT1, 3000},  // 入力1→出力1
    {PHYS_IN2, TRG_OUT2, 3000},  // 入力2→出力2
};

// アサイン変更も可能
// {PHYS_IN1, TRG_OUT2, 3000}  // 入力1→出力2
```

### 3.3 パルス出力（非ブロッキング）

```cpp
class PulseController {
public:
    void begin(int pin);
    bool startPulse(int durationMs);
    void update();  // loop()で呼び出し
    bool isActive() const;

private:
    int pin_;
    bool active_;
    unsigned long startMs_;
    unsigned long durationMs_;
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
| `tenant_pass` | string | テナントプライマリのパスワード |
| `tenant_cic` | string | テナントプライマリのCIC |
| `cic` | string | 自デバイスのCIC |

### 4.2 出力設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `output1_name` | string | "OPEN" | 出力1の名前 |
| `output2_name` | string | "CLOSE" | 出力2の名前 |
| `pulse_ms` | int | 3000 | パルス幅（ms） |
| `interlock_ms` | int | 200 | インターロック時間（ms） |

### 4.3 トリガーアサイン

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `input1_target` | int | 1 | 入力1のターゲット出力(1 or 2) |
| `input2_target` | int | 2 | 入力2のターゲット出力(1 or 2) |

---

## 5. 共通コンポーネント使用

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
| Operator | ○ | 状態機械 |

---

## 6. 参照

- **is04 (ISMS版)**: `archive_ISMS/ESP32/is04/is04.ino`
- **is06a (6ch版)**: `code/ESP32/is06a/`
- **global モジュール**: `code/ESP32/global/src/`
