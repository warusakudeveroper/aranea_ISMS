# IS06S GPIO18/GPIO5 出力不可問題 - 調査依頼書

**作成日**: 2026-01-26
**報告者**: Claude Code
**ステータス**: **調査依頼中**
**緊急度**: 高（プロダクション影響）

---

## 1. 問題の概要

### 症状
- **CH1 (GPIO18)** および **CH2 (GPIO5)** から3.3V出力が得られない
- 電圧計で測定すると「底電圧」（期待値3.3Vに対し、著しく低い電圧）
- **CH3 (GPIO15), CH4 (GPIO27), CH5 (GPIO14), CH6 (GPIO12)** は正常動作

### 重要な事実

#### ✅ 過去のコードでは動作していた
- **同一ハードウェア、同一配線で、以前のファームウェアではCH1(GPIO18)から3.3V出力を確認済み**
- PWM出力、Digital出力ともに動作確認済み
- **→ 配線・接続の問題ではない**

#### ✅ ESP32の3.3V出力ピンからの直結では動作
- ESP32の3.3V出力ピンをMOSFET/リレーのトリガーに直結すると正常動作
- **→ 外部コンポーネント（MOSFET、リレー等）の問題ではない**

#### ✅ ESP32モジュールを交換しても同一症状
- 別個体のESP32-DevKitCに交換して検証
- 同一症状（GPIO18, GPIO5のみ出力不可）
- **→ 特定ESP32チップの故障ではない**

---

## 2. ハードウェア構成

### ESP32チップ情報
```
Chip: ESP32-D0WD-V3 (revision v3.1)
Features: WiFi, BT, Dual Core, 240MHz, VRef calibration in efuse
Crystal: 40MHz
```

### GPIO割り当て
| Channel | GPIO | 用途 | 状態 |
|---------|------|------|------|
| CH1 | GPIO18 | D/P Output | **❌ 出力不可** |
| CH2 | GPIO5 | D/P Output | **❌ 出力不可** |
| CH3 | GPIO15 | D/P Output | ✅ 正常 |
| CH4 | GPIO27 | D/P Output | ✅ 正常 |
| CH5 | GPIO14 | I/O | ✅ 正常 |
| CH6 | GPIO12 | I/O | ✅ 正常 |

### GPIO特性（ESP32データシート）
| GPIO | 特性 | 備考 |
|------|------|------|
| GPIO5 | ストラッピングピン | ブート時SDIO Slave timing制御、内部プルアップ |
| GPIO18 | VSPICLK | デフォルトVSPI SPIクロックピン |

---

## 3. 実施した検証

### 3-1. 最小構成テスト

**テストコード** (`gpiotest.ino`):
```cpp
// GPIO サイクルテスト - 出力のたびに毎回設定
#include <driver/gpio.h>

int pins[] = {18, 5, 15, 27, 14, 12};
int count = 6;

void setup() {
  Serial.begin(115200);
}

void loop() {
  for (int i = 0; i < count; i++) {
    gpio_num_t pin = (gpio_num_t)pins[i];
    gpio_reset_pin(pin);
    gpio_set_direction(pin, GPIO_MODE_OUTPUT);
    gpio_set_pull_mode(pin, GPIO_FLOATING);
    gpio_set_level(pin, 1);
    delay(3000);
    gpio_set_level(pin, 0);
  }
}
```

**結果**:
- GPIO 15, 27, 14, 12: 3.3V出力確認 ✅
- GPIO 18, 5: 底電圧 ❌

### 3-2. Arduino API テスト

```cpp
pinMode(18, OUTPUT);
digitalWrite(18, HIGH);
```

**結果**: 同様に出力不可

### 3-3. ESP-IDF API テスト

```cpp
gpio_pad_select_gpio(GPIO_NUM_18);
gpio_set_direction(GPIO_NUM_18, GPIO_MODE_OUTPUT);
gpio_set_level(GPIO_NUM_18, 1);
```

**結果**: 同様に出力不可

### 3-4. レジスタ直接読み取り診断

`/api/debug/gpio` エンドポイントによる診断結果:

```json
{
  "gpio": 18,
  "channel": 1,
  "sigOutId": 256,        // Simple GPIO (ペリフェラル干渉なし)
  "isSimpleGpio": true,
  "gpioOutBit": 1,        // 出力ラッチ = HIGH
  "gpioEnableBit": 1,     // 出力有効
  "gpioInBit": 1,         // 読み返し = HIGH
  "isOpenDrain": false,   // プッシュプル
  "iomuxFunSel": 2,       // GPIO function
  "diagnosis": "STABLE_HIGH"
}
```

**矛盾点**: ソフトウェア的にはすべて正常（OUT=1, ENABLE=1, IN=1）だが、**物理的な電圧出力がない**

### 3-5. 高頻度サンプリング

```cpp
for (int i = 0; i < 1000; i++) {
  if (gpio_get_level(pin)) highCount++;
  else lowCount++;
}
```

**結果**:
- GPIO18: highCount=1000, lowCount=0, transitions=0
- **→ PWMトグルなし、ソフトウェア的にはHIGH安定**

---

## 4. 排除された仮説

| 仮説 | 検証方法 | 結果 |
|------|----------|------|
| 配線・接続の問題 | 過去コードで動作確認済み | ❌ 排除 |
| 外部コンポーネント故障 | 3.3V直結で動作確認 | ❌ 排除 |
| ESP32チップ故障 | 別個体で同一症状 | ❌ 排除 |
| LEDCペリフェラル干渉 | sigOutId=256確認 | ❌ 排除 |
| SPI干渉 | コード検索でSPI使用なし | ❌ 排除 |
| IO_MUX function未設定 | iomuxFunSel=2確認 | ❌ 排除 |
| オープンドレイン設定 | GPIO_PINx_REG確認 | ❌ 排除 |
| 出力有効化漏れ | GPIO_ENABLE_REG確認 | ❌ 排除 |
| PWMトグル | 高頻度サンプリング | ❌ 排除 |

---

## 5. 未検証項目

### ソフトウェア
- [ ] `spi_bus_free(VSPI_HOST)` によるSPIバス明示的解放
- [ ] `PIN_FUNC_SELECT(GPIO_PIN_MUX_REG[18], PIN_FUNC_GPIO)` 直接呼び出し
- [ ] Arduino-ESP32 2.x系へのダウングレード
- [ ] ESP-IDF純正環境での同一テスト

### ハードウェア
- [ ] オシロスコープによる波形確認
- [ ] ESP32チップパッド直接のプローブ測定
- [ ] 基板パターンの導通確認

---

## 6. 環境情報

### 開発環境
- **Arduino-ESP32**: 3.0.5
- **ESP-IDF**: (Arduino-ESP32 3.0.5内蔵)
- **Arduino CLI**: 最新版
- **ビルドオプション**: `esp32:esp32:esp32:PartitionScheme=min_spiffs`

### デバイス
- **IPアドレス**: 192.168.77.108
- **Firmware**: 1.0.0
- **CIC**: 188839
- **MAC**: 6CC8408C75E4

---

## 7. 関連ファイル

### ソースコード
- [is06s.ino](../is06s.ino) - メインファームウェア
- [Is06PinManager.cpp](../../Is06PinManager.cpp) - PIN制御実装
- [Is06PinManager.h](../../Is06PinManager.h) - PIN制御ヘッダ
- [HttpManagerIs06s.cpp](../../HttpManagerIs06s.cpp) - HTTP API（/api/debug/gpio含む）

### 設計ドキュメント
- [review8.md](./review8.md) - GPIO Matrix調査指針
- [review10_INVESTIGATION_SUMMARY.md](./review10_INVESTIGATION_SUMMARY.md) - 初期調査報告
- [CH1_CH2_VOLTAGE_ISSUE_v3.md](./CH1_CH2_VOLTAGE_ISSUE_v3.md) - 問題詳細

### テストコード
- [gpiotest/gpiotest.ino](../../gpiotest/gpiotest.ino) - 最小構成GPIOテスト

---

## 8. 調査依頼事項

### 依頼1: GPIO18/GPIO5特有の初期化要件
ESP32のGPIO18（VSPICLK）およびGPIO5（ストラッピングピン）を汎用GPIO出力として使用する際に、
標準的な`gpio_set_direction()`以外に必要な初期化手順があるか？

### 依頼2: Arduino-ESP32 3.x の既知問題
Arduino-ESP32 3.0.5において、GPIO18またはGPIO5に関する既知のバグや制限はあるか？

関連Issue:
- [GitHub Issue #11343 - PHY and GPIO error after 3.1](https://github.com/espressif/arduino-esp32/issues/11343)
- [GitHub Issue #9154 - GPIO pinMode error](https://github.com/espressif/arduino-esp32/issues/9154)

### 依頼3: レジスタ状態と物理出力の不一致
GPIO_OUT_REG, GPIO_ENABLE_REG, GPIO_IN_REGすべてが期待値（HIGH）を示しているにもかかわらず、
物理的な電圧出力がない状況の原因として考えられるものは何か？

### 依頼4: 過去動作コードとの差分
以前のファームウェアでは動作していたという事実から、コードの何が変わったことで
GPIO18/5が動作しなくなったか？（差分調査）

---

## 9. 連絡先

- **リポジトリ**: [aranea_ISMS](https://github.com/warusakudeveroper/aranea_ISMS)
- **パス**: `code/ESP32/is06s/`
- **プロジェクト指示**: `CLAUDE.md` 参照

---

## 10. 更新履歴

| 日付 | 内容 |
|------|------|
| 2026-01-25 | 初期調査（review10作成） |
| 2026-01-26 | ESP32交換検証、最小構成テスト実施 |
| 2026-01-26 | 本調査依頼書作成 |
