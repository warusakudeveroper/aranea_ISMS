# IS06S CH1/CH2 GPIO出力電圧問題 - 検証依頼書 v3

**作成日**: 2026-01-25
**報告者**: Claude Code
**緊急度**: 高（基本機能の動作不良）
**前回報告**: [CH1_CH2_VOLTAGE_ISSUE_v2.md](./CH1_CH2_VOLTAGE_ISSUE_v2.md)

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

### 2.1 LEDC関連修正（v1-v2で実施）

| 試行内容 | 結果 |
|---------|------|
| `ledcAttach()`を呼ばずに`pinMode(OUTPUT)`のみ使用 | 変化なし |
| `ledcDetach(pin)`で明示的にLEDCから解放 | 変化なし |
| `ledcWrite(pin, 255)`でPWM 100%出力 | 変化なし |

### 2.2 GPIO制御方法（v1-v2で実施）

| 試行内容 | 結果 |
|---------|------|
| Arduino `digitalWrite(18, HIGH)` | ~1.37V |
| Arduino `pinMode(18, OUTPUT)` + `digitalWrite(18, HIGH)` | ~1.37V |
| ESP-IDF `gpio_set_level(GPIO_NUM_18, 1)` | ~1.37V |
| ESP-IDF `gpio_reset_pin()` + `gpio_set_direction()` + `gpio_set_level()` | ~1.37V |

### 2.3 初期化タイミング（v1-v2で実施）

| 試行内容 | 結果 |
|---------|------|
| `setup()`最後でGPIO18をHIGHに強制設定 | ~1.37V |
| `ledcDetach()` → `gpio_reset_pin()` → `gpio_set_direction()` → `gpio_set_level()` | ~1.37V |

### 2.4 Arduino-ESP32 3.x LEDC API修正（v3で実施・本日）

review7.mdの指摘に基づきLEDC APIの誤用を修正:

| 試行内容 | 結果 |
|---------|------|
| `bool ok = ledcAttach(...)` → `int ch = ledcAttach(...)` に変更 | **変化なし** |
| `ledcWrite(pin, duty)` → `ledcWriteChannel(ch, duty)` に変更 | **変化なし** |
| ch=0を正常値として扱う（ch < 0のみ失敗判定） | **変化なし** |
| ledcChannel_[]配列でチャンネル番号を管理 | **変化なし** |

### 2.5 WebUI修正（v3で実施・本日）

| 試行内容 | 結果 |
|---------|------|
| `generateHTML()`の一括送信 → チャンク転送に変更 | **WebUIリロード問題は解決** |
| `sendHTMLChunked()`で分割送信 | Free Heap 148KB安定 |

**注**: WebUI問題は解決したが、GPIO電圧問題は未解決。

---

## 3. 確認済みハードウェア正常性

- ✅ ESP32の3.3V端子から直接MOSFETにトリガー入力 → **100%発光**
- ✅ MOSFETおよび負荷（LEDテープ）は正常
- ✅ CH3, CH4は正常に3.3V出力
- ✅ GPIO18/GPIO5の配線・はんだ付けは目視で正常

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
- 両方ともVSPIペリフェラル関連ピン
- 両方とも同様の低電圧症状（~40%相当）
- CH3 (GPIO15), CH4 (GPIO27) は正常

---

## 5. 現在のコード状態（v3修正後）

### Is06PinManager.cpp - initLedc()
```cpp
void Is06PinManager::initLedc() {
  using namespace AraneaSettingsDefaults;

  Serial.println("Is06PinManager: Initializing GPIO with explicit LEDC detach...");

  for (int i = 0; i < IS06_DP_CHANNELS; i++) {
    int pin = IS06_PIN_MAP[i];

    // 1. LEDCから明示的にデタッチ
    ledcDetach(pin);

    // 2. LEDCチャンネル管理をクリア
    ledcChannel_[i] = -1;

    // 3. GPIOマトリックスをリセット
    gpio_reset_pin((gpio_num_t)pin);

    // 4. GPIO出力モードに設定
    gpio_set_direction((gpio_num_t)pin, GPIO_MODE_OUTPUT);
    gpio_set_level((gpio_num_t)pin, 0);

    // 5. 確認読み取り
    int readBack = gpio_get_level((gpio_num_t)pin);
    Serial.printf("  CH%d (GPIO%d): detach+reset+OUTPUT LOW, read=%d\n", i + 1, pin, readBack);
  }

  Serial.println("Is06PinManager: GPIO initialized (LEDC detached, all LOW).");
}
```

### Is06PinManager.cpp - applyDigitalOutput()
```cpp
void Is06PinManager::applyDigitalOutput(int channel, int state) {
  if (!isValidChannel(channel)) return;
  int idx = channel - 1;
  int pin = IS06_PIN_MAP[idx];

  // ESP-IDF GPIO APIで直接制御
  gpio_set_level((gpio_num_t)pin, state ? 1 : 0);

  // 確認読み取り
  int readBack = gpio_get_level((gpio_num_t)pin);
  Serial.printf("Is06PinManager: applyDigitalOutput CH%d GPIO%d set=%d read=%d\n",
                channel, pin, state, readBack);
}
```

### Is06PinManager.cpp - setPinType() PWM部分（v3修正）
```cpp
} else if (type == ::PinType::PWM_OUTPUT) {
  // 【重要】Arduino-ESP32 3.x: ledcAttach()はint(チャンネル番号)を返す
  // ch=0は正常値！boolで受けるとch=0がfalseになりCH1が初期化失敗扱いになる
  ledcDetach(pin);
  int ch = ledcAttach(pin, AraneaSettingsDefaults::PWM_FREQUENCY, AraneaSettingsDefaults::PWM_RESOLUTION);
  Serial.printf("Is06PinManager: CH%d GPIO%d ledcAttach returned ch=%d\n", channel, pin, ch);

  if (ch >= 0) {
    // ch >= 0 は成功（ch=0も正常値）
    ledcChannel_[idx] = ch;
    // Arduino-ESP32 3.x: ledcWriteChannel(ch, duty) を使用
    ledcWriteChannel(ch, 0);
    Serial.printf("Is06PinManager: CH%d GPIO%d LEDC attached ch=%d\n", channel, pin, ch);
  } else {
    // ch < 0 は失敗
    ledcChannel_[idx] = -1;
    gpio_reset_pin((gpio_num_t)pin);
    gpio_set_direction((gpio_num_t)pin, GPIO_MODE_OUTPUT);
    gpio_set_level((gpio_num_t)pin, 0);
    Serial.printf("Is06PinManager: CH%d GPIO%d LEDC attach FAILED (ch=%d), fallback to GPIO\n", channel, pin, ch);
  }
}
```

---

## 6. 除外された仮説

### 6.1 LEDC API誤用（除外）
- review7.mdの指摘通りに修正済み
- `int ch = ledcAttach()` でch番号を正しく取得
- `ledcWriteChannel(ch, duty)` で書き込み
- **結果**: 変化なし → LEDC APIは根本原因ではない

### 6.2 WebUIのヒープ断片化（除外・WebUI問題のみ）
- チャンク転送に変更済み
- WebUIリロード問題は解決
- **GPIO電圧問題とは無関係**

### 6.3 コード上のGPIO初期化問題（除外）
- ESP-IDF API (`gpio_reset_pin`, `gpio_set_direction`, `gpio_set_level`) を直接使用
- `gpio_get_level()` での読み取り値は正常（set=1, read=1）
- **結果**: コード上は正しく設定されているが、実際の電圧が低い

---

## 7. 残る仮説（優先度順）

### 仮説A: ハードウェア問題（優先度: 高）

**根拠**:
- ソフトウェア上は正しく設定されている（read=1）
- 同じコードでCH3, CH4は正常
- GPIO18, GPIO5のみ問題

**可能性**:
1. ESP32モジュール自体のGPIO18/GPIO5が損傷
2. 基板上のパターン問題（断線、ショート）
3. プルダウン抵抗やその他の外部回路の影響

**検証方法**:
1. **別のESP32-DevKitCで同じファームウェアをテスト**
2. オシロスコープでGPIO波形を確認
3. GPIO18/GPIO5端子と基板パターンの導通確認

### 仮説B: SPIペリフェラルの干渉（優先度: 中）

**根拠**:
- GPIO18=VSPI CLK, GPIO5=VSPI SS
- Adafruit SSD1306ライブラリがSPIを使用している可能性

**検証方法**:
1. `SPI.end()` を明示的に呼び出し
2. SPIペリフェラルのレジスタを直接確認
3. OLEDディスプレイを外してテスト

### 仮説C: 電源/GND問題（優先度: 中）

**根拠**:
- 複数ピンが同様の症状
- 電圧が約40%相当（分圧？）

**検証方法**:
1. ESP32の3.3V電源電圧を確認
2. GND接続を確認
3. 電源ライン上のノイズ確認

### 仮説D: ブートローダー/NVS残留設定（優先度: 低）

**根拠**:
- 以前のファームウェアの設定が影響している可能性

**検証方法**:
1. `nvs_flash_erase()` でNVS完全消去
2. ファクトリーリセット後に再テスト
3. `erase_flash` でESP32を完全消去後に書き込み

---

## 8. 推奨する検証手順

### Step 1: 最小スケッチでのテスト（最優先）

USB経由で以下の最小限コードをフラッシュし、GPIO18の電圧を測定:

```cpp
#include <driver/gpio.h>

void setup() {
  Serial.begin(115200);
  delay(1000);

  // 他のペリフェラルを一切初期化しない状態でGPIO18をテスト
  gpio_reset_pin(GPIO_NUM_18);
  gpio_set_direction(GPIO_NUM_18, GPIO_MODE_OUTPUT);
  gpio_set_level(GPIO_NUM_18, 1);

  int read = gpio_get_level(GPIO_NUM_18);
  Serial.printf("GPIO18 set=1 read=%d\n", read);
  Serial.println("GPIO18 should be HIGH (3.3V) - measure now");
}

void loop() {
  delay(1000);
}
```

**期待結果**: 3.3V
**NG結果**: ~1.37V → ハードウェア問題の可能性が極めて高い

### Step 2: 別のESP32でテスト

同じis06sファームウェアを**別のESP32-DevKitC**にフラッシュし、GPIO18の電圧を測定。

**期待結果**: 3.3V → 元のESP32のハードウェア問題確定
**NG結果**: ~1.37V → ファームウェア問題の可能性

### Step 3: 完全消去後にテスト

```bash
esptool.py --port /dev/cu.usbserial-XXXX erase_flash
```

その後、最小スケッチを書き込んでテスト。

---

## 9. 制約事項

- **シリアル接続による長時間デバッグは不可** - シリアル接続自体が不安定化の原因となることが確認されており、現場設置済み
- OTAによるファームウェア更新のみ可能
- WebUIでの操作・状態確認は可能（リロード問題は解決済み）

---

## 10. 関連ファイル

- [Is06PinManager.cpp](../../Is06PinManager.cpp)
- [Is06PinManager.h](../../Is06PinManager.h)
- [is06s.ino](../../is06s.ino)
- [AraneaWebUI.cpp](../../../global/src/AraneaWebUI.cpp) - WebUIチャンク転送修正
- [AraneaSettingsDefaults.h](../../../global/src/AraneaSettingsDefaults.h)
- [review7.md](./review7.md) - LEDC API誤用の指摘

---

## 11. 関連コミット履歴

```
# 本日の修正
- WebUIチャンク転送対応（AraneaWebUI.cpp）
- LEDC API修正（Is06PinManager.cpp）
  - bool ok → int ch
  - ledcWrite → ledcWriteChannel
  - ledcChannel_[]配列管理追加

# 過去の修正
90360d7 docs(is06s): CH1 PWM出力問題検証依頼書作成
6db6d7d fix(is06s): レビュー指摘対応 - Must Fix項目すべて解消
025146f feat(is06s): PIN設定API/WebUI完全実装
ec26370 feat(is06s): P1コア機能実装 - Is06PinManager完成
```

---

## 12. 結論

ソフトウェア側で考えられる修正（LEDC API、GPIO API、初期化順序等）はすべて試行済み。
`gpio_get_level()` での読み取り値は正常（1）だが、実測電圧は~1.37Vのまま。

**最も可能性が高い原因**: ハードウェア問題（ESP32モジュール自体のGPIO18/GPIO5の損傷、または基板配線の問題）

**推奨アクション**:
1. 別のESP32-DevKitCで同じファームウェアをテスト
2. 最小スケッチでGPIO18のみをテスト
3. GPIO18/GPIO5端子の導通・パターン確認

---

**連絡先**: GitHub Issue または CLAUDE.md 参照
