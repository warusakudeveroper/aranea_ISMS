# IS06S CH1/CH2 GPIO出力電圧問題 - 検証依頼書 v2

**作成日**: 2026-01-25
**報告者**: Claude Code
**緊急度**: 高（基本機能の動作不良）
**前回報告**: [CH1_PWM_OUTPUT_ISSUE.md](./CH1_PWM_OUTPUT_ISSUE.md)

---

## 1. 問題概要

CH1 (GPIO18) およびCH2 (GPIO5) からの出力電圧が異常に低い。

| チャンネル | GPIO | 期待値 | 実測値 | 状態 |
|-----------|------|--------|--------|------|
| CH1 | GPIO18 | 3.3V | ~1.37V | NG |
| CH2 | GPIO5 | 3.3V | ~1.25V | NG |
| CH3 | GPIO15 | 3.3V | 3.3V | OK |
| CH4 | GPIO27 | 3.3V | 3.3V | OK |

**重要**: 以前は同じハードウェアでGPIO18から3.3V出力が確認されている。

---

## 2. 試行した修正（すべて効果なし）

### 2.1 LEDC関連

| 試行内容 | 結果 |
|---------|------|
| `ledcAttach()`を呼ばずに`pinMode(OUTPUT)`のみ使用 | 変化なし |
| `ledcDetach(pin)`で明示的にLEDCから解放 | 変化なし |
| `ledcWrite(pin, 255)`でPWM 100%出力 | 変化なし |

### 2.2 GPIO制御方法

| 試行内容 | 結果 |
|---------|------|
| Arduino `digitalWrite(18, HIGH)` | ~1.37V |
| Arduino `pinMode(18, OUTPUT)` + `digitalWrite(18, HIGH)` | ~1.37V |
| ESP-IDF `gpio_set_level(GPIO_NUM_18, 1)` | ~1.37V |
| ESP-IDF `gpio_reset_pin()` + `gpio_set_direction()` + `gpio_set_level()` | ~1.37V |

### 2.3 初期化タイミング

| 試行内容 | 結果 |
|---------|------|
| `setup()`最後でGPIO18をHIGHに強制設定 | ~1.37V |
| `ledcDetach()` → `gpio_reset_pin()` → `gpio_set_direction()` → `gpio_set_level()` | ~1.37V |

### 2.4 コード構造

- PINタイプがDigital OutputでもPWM Outputでも同じ電圧（~1.37V）
- is06s.ino側とIs06PinManager側の両方で初期化しても変化なし

---

## 3. 確認済みハードウェア正常性

- ✅ ESP32の3.3V端子から直接MOSFETにトリガー入力 → **100%発光**
- ✅ MOSFETおよび負荷（LEDテープ）は正常
- ✅ CH3, CH4は正常に3.3V出力

---

## 4. GPIO18/GPIO5の特性

### ESP32 GPIO18
- VSPI CLK（SPI Clock）として使用可能
- 通常のGPIO出力可能
- **ストラップピンではない**

### ESP32 GPIO5
- VSPI SS（SPI Chip Select）として使用可能
- 通常のGPIO出力可能
- ブート時LOGレベル制御（通常は問題なし）

### 共通点
- 両方ともVSPI関連ピン
- 両方とも同様の低電圧症状

---

## 5. 現在のコード状態

### Is06PinManager.cpp - initLedc()
```cpp
void Is06PinManager::initLedc() {
  for (int i = 0; i < IS06_DP_CHANNELS; i++) {
    int pin = IS06_PIN_MAP[i];

    // 1. LEDCから明示的にデタッチ
    ledcDetach(pin);

    // 2. GPIOマトリックスをリセット
    gpio_reset_pin((gpio_num_t)pin);

    // 3. GPIO出力モードに設定
    gpio_set_direction((gpio_num_t)pin, GPIO_MODE_OUTPUT);
    gpio_set_level((gpio_num_t)pin, 0);
  }
}
```

### Is06PinManager.cpp - applyDigitalOutput()
```cpp
void Is06PinManager::applyDigitalOutput(int channel, int state) {
  int pin = IS06_PIN_MAP[idx];

  // ESP-IDF GPIO APIで直接制御
  gpio_set_level((gpio_num_t)pin, state ? 1 : 0);

  // 確認読み取り
  int readBack = gpio_get_level((gpio_num_t)pin);
  Serial.printf("CH%d GPIO%d set=%d read=%d\n", channel, pin, state, readBack);
}
```

---

## 6. 仮説と検証項目

### 仮説A: SPIペリフェラルが自動初期化されている（優先度: 高）

**根拠**: GPIO18とGPIO5は両方ともVSPIピン。何らかのライブラリがSPIを初期化している可能性。

**検証方法**:
1. Adafruit SSD1306ライブラリがSPIを初期化していないか確認
2. `SPI.end()`を呼んでSPIを明示的に停止
3. SPIペリフェラルのレジスタを直接確認

### 仮説B: WiFi/Bluetoothがピンを使用（優先度: 中）

**根拠**: ESP32のRadioがGPIOに影響を与える場合がある。

**検証方法**:
1. WiFi/BT無効の最小スケッチでGPIO18をテスト
2. `WiFi.mode(WIFI_OFF)`後にGPIOテスト

### 仮説C: ハードウェア問題（優先度: 低）

**根拠**: 以前は動作していたため、突然の故障は考えにくい。

**検証方法**:
1. 別のESP32モジュールで同じコードをテスト
2. オシロスコープでGPIO波形を確認

### 仮説D: NVS/フラッシュの設定残留（優先度: 中）

**根拠**: 以前のファームウェアの設定が影響している可能性。

**検証方法**:
1. `nvs_flash_erase()`でNVS完全消去後に再テスト
2. ファクトリーリセット後に再テスト

---

## 7. 推奨する検証手順

### Step 1: 最小スケッチでのテスト

以下の最小限コードをUSB経由でフラッシュし、GPIO18の電圧を測定:

```cpp
#include <driver/gpio.h>

void setup() {
  Serial.begin(115200);

  // GPIO18を出力HIGHに設定
  gpio_reset_pin(GPIO_NUM_18);
  gpio_set_direction(GPIO_NUM_18, GPIO_MODE_OUTPUT);
  gpio_set_level(GPIO_NUM_18, 1);

  Serial.println("GPIO18 should be HIGH (3.3V)");
}

void loop() {
  // 何もしない
  delay(1000);
}
```

**期待結果**: 3.3V
**NG結果**: ~1.37V → ハードウェア問題の可能性

### Step 2: SPI明示停止テスト

```cpp
#include <SPI.h>
#include <driver/gpio.h>

void setup() {
  Serial.begin(115200);

  // SPIを明示的に停止
  SPI.end();

  // GPIO18設定
  gpio_reset_pin(GPIO_NUM_18);
  gpio_set_direction(GPIO_NUM_18, GPIO_MODE_OUTPUT);
  gpio_set_level(GPIO_NUM_18, 1);

  Serial.println("SPI stopped, GPIO18 should be HIGH");
}

void loop() {
  delay(1000);
}
```

### Step 3: 別のESP32でテスト

同じis06sファームウェアを別のESP32-DevKitCにフラッシュし、GPIO18の電圧を測定。

---

## 8. 関連ファイル

- [Is06PinManager.cpp](/code/ESP32/is06s/Is06PinManager.cpp)
- [Is06PinManager.h](/code/ESP32/is06s/Is06PinManager.h)
- [is06s.ino](/code/ESP32/is06s/is06s.ino)
- [AraneaSettingsDefaults.h](/code/ESP32/global/src/AraneaSettingsDefaults.h)

---

## 9. 関連コミット履歴

```
90360d7 docs(is06s): CH1 PWM出力問題検証依頼書作成
6db6d7d fix(is06s): レビュー指摼対応 - Must Fix項目すべて解消
025146f feat(is06s): PIN設定API/WebUI完全実装
ec26370 feat(is06s): P1コア機能実装 - Is06PinManager完成
```

---

## 10. 補足情報

### 電圧の意味
- 1.37V / 3.3V ≈ 41.5%
- 1.25V / 3.3V ≈ 37.9%
- なぜこれらの値になるのかは不明

### WebUI再読み込み問題（関連）
- WebUIが再読み込み後に動作しなくなる問題が別途発生
- ESP32再起動で復旧
- 原因: `generateHTML()`の大きなString生成によるヒープ断片化の可能性
- GPIO問題の検証を妨げている

---

## 11. 次のアクション

1. **USB接続でシリアルログを取得** - `gpio_get_level()`の読み取り値を確認
2. **最小スケッチでGPIO18テスト** - コード依存の問題を排除
3. **別のESP32でテスト** - ハードウェア問題を切り分け
4. **SPI.end()追加テスト** - VSPIペリフェラルの影響を確認

---

**連絡先**: GitHub Issue または CLAUDE.md 参照
