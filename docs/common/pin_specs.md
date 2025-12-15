# GPIO ピン仕様書

**必須参照ドキュメント** - ESP32 DevKitC ピンアサインと各デバイスの配線仕様

---

## 1. ESP32 DevKitC ピン制約（重要）

### 1.1 入力専用ピン（出力不可）

| GPIO | 用途制限 |
|------|----------|
| GPIO34 | 入力専用 (ADC1_CH6) |
| GPIO35 | 入力専用 (ADC1_CH7) |
| GPIO36 (VP) | 入力専用 (ADC1_CH0) |
| GPIO39 (VN) | 入力専用 (ADC1_CH3) |

**注意**: これらのピンに `pinMode(..., OUTPUT)` や `digitalWrite()` を使用するとエラー。

### 1.2 ブートストラップピン（起動時に注意）

| GPIO | 起動時の影響 | 安全な使用方法 |
|------|-------------|---------------|
| GPIO0 | LOW=ダウンロードモード | プルアップで使用、起動時LOWにしない |
| GPIO2 | LOW=ダウンロードモード | プルダウンで使用可能 |
| GPIO4 | - | 自由に使用可 |
| GPIO5 | HIGH=SPI Flash起動 | プルアップ推奨 |
| GPIO12 | LOW=3.3V Flash / HIGH=1.8V | プルダウン必須（通常3.3V Flash） |
| GPIO15 | LOW=サイレントブート | プルアップで使用 |

### 1.3 使用禁止ピン（内部SPI Flash接続）

| GPIO | 理由 |
|------|------|
| GPIO6 | SPI Flash CLK |
| GPIO7 | SPI Flash D0 |
| GPIO8 | SPI Flash D1 |
| GPIO9 | SPI Flash D2 |
| GPIO10 | SPI Flash D3 |
| GPIO11 | SPI Flash CMD |

**注意**: これらのピンは絶対に使用しないこと。

### 1.4 I2C デフォルトピン

| 機能 | GPIO |
|------|------|
| SDA | GPIO21 |
| SCL | GPIO22 |

### 1.5 その他の特殊ピン

| GPIO | 機能 |
|------|------|
| GPIO1 | UART0 TX (シリアルモニタ) |
| GPIO3 | UART0 RX (シリアルモニタ) |
| GPIO25 | DAC1 |
| GPIO26 | DAC2 |

---

## 2. is01 ピンアサイン（温湿度センサー）

### 2.1 ピン配置

| GPIO | 機能 | 方向 | 備考 |
|------|------|------|------|
| GPIO5 | MOSFET_EN | OUTPUT | 省電力ゲート制御 |
| GPIO21 | I2C SDA | I/O | SHT30 + OLED |
| GPIO22 | I2C SCL | OUTPUT | SHT30 + OLED |

### 2.2 I2Cアドレス

| デバイス | アドレス |
|---------|---------|
| SHT30 | 0x44 or 0x45 |
| OLED (SSD1306) | 0x3C or 0x3D |

### 2.3 DeepSleep復帰時の注意

```cpp
// 正しい初期化順序
void setup() {
  pinMode(MOSFET_EN, OUTPUT);
  digitalWrite(MOSFET_EN, HIGH);  // 電源ON
  delay(100);                      // 安定待ち

  Wire.begin(SDA_PIN, SCL_PIN);   // I2C初期化（同期的に）
  // 以降、SHT30/OLED初期化
}
```

---

## 3. is02 ピンアサイン（BLEゲートウェイ）

### 3.1 ピン配置

| GPIO | 機能 | 方向 | 備考 |
|------|------|------|------|
| GPIO5 | MOSFET_EN | OUTPUT | 省電力ゲート（オプション） |
| GPIO16 | INPUT1 | INPUT_PULLDOWN | 予備入力 |
| GPIO17 | INPUT2 | INPUT_PULLDOWN | 予備入力 |
| GPIO19 | INPUT3 | INPUT_PULLDOWN | 予備入力 |
| GPIO21 | I2C SDA | I/O | OLED |
| GPIO22 | I2C SCL | OUTPUT | OLED |
| GPIO25 | SW1 | INPUT_PULLUP | WiFi再接続ボタン |
| GPIO26 | SW2 | INPUT_PULLUP | リブートボタン |

---

## 4. is04 ピンアサイン（開閉コントローラ）

### 4.1 ピン配置

| GPIO | 機能 | 方向 | 備考 |
|------|------|------|------|
| GPIO5 | PHYS_IN1 | INPUT_PULLDOWN | 物理入力1 (Trigger1_Physical) |
| GPIO18 | PHYS_IN2 | INPUT_PULLDOWN | 物理入力2 (Trigger2_Physical) |
| GPIO12 | TRG_OUT1 | OUTPUT | トリガー出力1 (※ブートストラップ注意) |
| GPIO14 | TRG_OUT2 | OUTPUT | トリガー出力2 |
| GPIO21 | I2C SDA | I/O | OLED |
| GPIO22 | I2C SCL | OUTPUT | OLED |
| GPIO25 | SW1 | INPUT_PULLUP | WiFi再接続ボタン |
| GPIO26 | SW2 | INPUT_PULLUP | リブートボタン |

### 4.2 GPIO12 の注意

GPIO12 はブートストラップピンのため、起動時に HIGH になっているとフラッシュ電圧が変更される。
- 回路設計時にプルダウン抵抗を追加
- または他のピンに変更を検討

---

## 5. is05 ピンアサイン（8ch開閉センサー）

### 5.1 リードスイッチ入力ピン（固定）

| チャンネル | GPIO | 機能 | 方向 | 備考 |
|-----------|------|------|------|------|
| ch1 | GPIO19 | REED_CH1 | INPUT_PULLUP | ReedSW-1 |
| ch2 | GPIO18 | REED_CH2 | INPUT_PULLUP | ReedSW-2 |
| ch3 | GPIO5 | REED_CH3 | INPUT_PULLUP | ReedSW-3 (※ブートストラップ注意) |
| ch4 | GPIO17 | REED_CH4 | INPUT_PULLUP | ReedSW-4 |
| ch5 | GPIO16 | REED_CH5 | INPUT_PULLUP | ReedSW-5 |
| ch6 | GPIO4 | REED_CH6 | INPUT_PULLUP | ReedSW-6 (※ブートストラップ注意) |
| ch7 | GPIO2 | REED_CH7 | INPUT_PULLUP | ReedSW-7 (※ブートストラップ注意) |
| ch8 | GPIO15 | REED_CH8 | INPUT_PULLUP | ReedSW-8 (※ブートストラップ注意) |

### 5.2 その他のピン

| GPIO | 機能 | 方向 | 備考 |
|------|------|------|------|
| GPIO25 | SW_WIFI | INPUT_PULLUP | WiFi再接続ボタン（3秒長押し） |
| GPIO26 | SW_RESET | INPUT_PULLUP | ファクトリーリセットボタン（3秒長押し） |
| GPIO21 | I2C SDA | I/O | OLED |
| GPIO22 | I2C SCL | OUTPUT | OLED |
| GPIO5 | MOSFET_EN | OUTPUT | I2C電源制御（ch3と共用注意） |

### 5.3 ブートストラップピンの注意

GPIO2/4/5/15 はブートストラップに関わるため、以下に注意:
- **GPIO2**: 起動時にLOWだとダウンロードモード。プルダウンで使用可能。
- **GPIO4**: 比較的自由に使用可能。
- **GPIO5**: 起動時にHIGHでSPI Flash起動。MOSFET制御と共用。
- **GPIO15**: 起動時にLOWだとサイレントブート。プルアップ推奨。

入力としてはオープンコレクタのオプトカプラ出力を想定（INPUT_PULLUP）。
起動後にピンモードを設定するため、基本的に問題なし。

### 5.4 入力設定（activeLow）

```cpp
// オプトカプラのオープンコレクタ出力を想定
// LOW = アクティブ（ON）、HIGH = 非アクティブ（OFF）

static const int chPins[8] = {19, 18, 5, 17, 16, 4, 2, 15};

void setup() {
  for (int i = 0; i < 8; i++) {
    pinMode(chPins[i], INPUT_PULLUP);
  }
}
```

### 5.5 チャタリング対策

```cpp
// サンプル周期: 20ms
// 安定判定: 3連続一致で確定

#define STABLE_COUNT 3
#define SAMPLE_INTERVAL_MS 20

int rawState[8];       // 生の読み取り値
int stableState[8];    // 確定状態
int stableCount_[8];   // 安定カウンタ
```

---

## 6. 共通 I2C デバイス

### 6.1 OLED (SSD1306 128x64)

| 項目 | 値 |
|------|-----|
| アドレス | 0x3C (または 0x3D) |
| 解像度 | 128 x 64 |
| 通信速度 | 400kHz (Fast Mode) |

### 6.2 温湿度センサー (SHT30)

| 項目 | 値 |
|------|-----|
| アドレス | 0x44 (ADDR=LOW) / 0x45 (ADDR=HIGH) |
| 測定範囲 | -40〜125℃, 0〜100%RH |
| 精度 | ±0.2℃, ±2%RH |

---

## 7. Arduino-MCP pin_check の活用

### 7.1 実行方法

```json
{ "name": "pin_check", "arguments": { "sketch_path": "code/ESP32/is01" } }
```

### 7.2 検出される問題

| Severity | 内容 |
|----------|------|
| Error | GPIO34-39 への出力命令 |
| Error | GPIO6-11 (SPI Flash) の使用 |
| Warning | ブートストラップピンの出力使用 |
| Warning | ADC非対応ピンでの analogRead() |
| Info | I2Cピンの使用状況 |

### 7.3 出力例

```json
{
  "ok": false,
  "warnings": [
    {
      "severity": "error",
      "gpio": 34,
      "message": "GPIO34 is input-only, cannot use as OUTPUT"
    },
    {
      "severity": "warning",
      "gpio": 12,
      "message": "GPIO12 is a strapping pin, may affect boot mode"
    }
  ],
  "usage": [
    { "gpio": 21, "mode": "I2C_SDA" },
    { "gpio": 22, "mode": "I2C_SCL" }
  ]
}
```
