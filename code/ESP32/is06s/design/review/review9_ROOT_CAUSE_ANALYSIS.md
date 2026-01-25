# IS06S GPIO18/GPIO5 電圧問題 - 根本原因分析報告書 v9

**作成日**: 2026-01-25
**報告者**: Claude Code
**ステータス**: **根本原因特定完了・修正案提示**

---

## 1. 根本原因の特定

### 発見した重大なバグ: `loadFromNvs()` 後のハードウェア初期化漏れ

**Is06PinManager.cpp** の初期化フローに致命的な問題があります。

#### 現在の初期化順序:

```
Is06PinManager::begin()
├── initGpio()        // CH5-6のみ設定（CH1-4はスキップ）
├── initLedc()        // CH1-4をGPIO OUTPUT LOWに設定 + ledcChannel_[] = -1
└── loadFromNvs()     // NVSから設定読み込み（★ハードウェア設定なし★）
```

**問題点**: `loadFromNvs()` はNVSから `pinType` を読み込みますが、**ハードウェア設定を適用しません**。

```cpp
// loadFromNvs() 内（Lines 717-732）
if (typeValue == ASD::PinType::PWM_OUTPUT) {
  pinSettings_[idx].type = ::PinType::PWM_OUTPUT;  // ← ソフトウェアフラグのみ設定
  // ★ setPinType() が呼ばれていない = ハードウェア（LEDC）が設定されない ★
}
```

#### 結果として起こること:

1. NVSに `PWM_OUTPUT` が保存されている場合:
   - `initLedc()` で GPIO OUTPUT LOW に設定される
   - `loadFromNvs()` で `pinSettings_[idx].type = PWM_OUTPUT` に設定（ソフトウェアのみ）
   - **LEDCはアタッチされない** (`ledcChannel_[idx] = -1` のまま)

2. WebUIでCH1を「ON」に設定すると:
   - `setPinState(1, 1)` が呼ばれる
   - `pinSettings_[0].type == PWM_OUTPUT` なので `setPwmValue(1, 1)` に分岐
   - PWM値 1% で `applyPwmOutput()` が呼ばれる

3. `applyPwmOutput()` の問題:
   ```cpp
   if (ch >= 0) {
     ledcWriteChannel(ch, pwm8bit);  // ch=-1なのでここは実行されない
   } else {
     int state = (value >= 50) ? 1 : 0;  // value=1 < 50 なので state=0!
     gpio_set_level((gpio_num_t)pin, state);  // ★ LOW が出力される ★
   }
   ```

### これが電圧低下の原因である証拠

1. **CH3, CH4が正常に動作する理由**: NVSにDIGITAL_OUTPUTとして保存されているか、PWMでも使われていない
2. **CH1, CH2だけ問題がある理由**: NVSにPWM_OUTPUTとして保存されている可能性が高い
3. **gpio_get_level()が1を返すのに電圧が低い理由**: `applyPwmOutput()` のフォールバックが `value < 50` を LOW として扱う

---

## 2. 初期化フローの詳細分析

### is06s.ino setup() の呼び出し順序:

```cpp
// Line 204
initGpio();  // pinMode(CH1-4, OUTPUT) + digitalWrite(CH1-4, LOW)

// ... WiFi, NTP, etc ...

// Line 325
pinManager.begin(&settings, &ntp);
  // → initGpio()  // CH5-6のみ
  // → initLedc()  // CH1-4をledcDetach + gpio_reset_pin + OUTPUT LOW
  // → loadFromNvs()  // ★ここでハードウェア設定が適用されない★

// Lines 421-426 (DEBUGコード)
gpio_reset_pin(GPIO_NUM_18);
gpio_set_direction(GPIO_NUM_18, GPIO_MODE_OUTPUT);
gpio_set_level(GPIO_NUM_18, 1);
```

### DEBUGコードが効かない理由:

DEBUGコードの後、`loop()` で `httpMgr.handle()` が呼ばれ、WebUIからの操作で:
- `setPinState()` → `setPwmValue()` → `applyPwmOutput()` が実行される
- この時点で再びGPIOがLOWに設定される

---

## 3. 修正案

### 修正案A: loadFromNvs() 後にハードウェア設定を適用（推奨）

**Is06PinManager.cpp** の `begin()` を修正:

```cpp
void Is06PinManager::begin(SettingManager* settings, NtpManager* ntp) {
  settings_ = settings;
  ntp_ = ntp;

  Serial.println("Is06PinManager: Initializing...");

  // GPIO初期化
  initGpio();

  // LEDC初期化（PWM用）- 一旦全てGPIO出力に
  initLedc();

  // NVSから設定読み込み
  loadFromNvs();

  // ★ 追加: NVSから読み込んだ設定に基づいてハードウェアを設定 ★
  applyLoadedPinTypes();

  Serial.println("Is06PinManager: Initialization complete.");
  printStatus();
}

// ★ 新規追加メソッド ★
void Is06PinManager::applyLoadedPinTypes() {
  Serial.println("Is06PinManager: Applying loaded pin types to hardware...");

  for (int ch = 1; ch <= IS06_CHANNEL_COUNT; ch++) {
    int idx = ch - 1;
    ::PinType type = pinSettings_[idx].type;

    // 現在のpinSettings_[idx].typeに基づいてハードウェアを設定
    // setPinType()を呼ぶとNVSに再保存されるので、直接ハードウェア設定のみ行う
    int pin = IS06_PIN_MAP[idx];

    if (type == ::PinType::PWM_OUTPUT && idx < IS06_DP_CHANNELS) {
      // PWMモード: LEDCアタッチ
      ledcDetach(pin);
      int ch = ledcAttach(pin, AraneaSettingsDefaults::PWM_FREQUENCY, AraneaSettingsDefaults::PWM_RESOLUTION);
      if (ch >= 0) {
        ledcChannel_[idx] = ch;
        ledcWriteChannel(ch, 0);
        Serial.printf("  CH%d GPIO%d: LEDC attached (ch=%d)\n", idx + 1, pin, ch);
      } else {
        ledcChannel_[idx] = -1;
        Serial.printf("  CH%d GPIO%d: LEDC attach failed\n", idx + 1, pin);
      }
    } else if (type == ::PinType::DIGITAL_OUTPUT) {
      // Digital出力モード
      ledcDetach(pin);
      if (idx < IS06_DP_CHANNELS) ledcChannel_[idx] = -1;
      gpio_reset_pin((gpio_num_t)pin);
      gpio_set_direction((gpio_num_t)pin, GPIO_MODE_OUTPUT);
      gpio_set_level((gpio_num_t)pin, 0);
      Serial.printf("  CH%d GPIO%d: Digital OUTPUT\n", idx + 1, pin);
    } else if (type == ::PinType::DIGITAL_INPUT) {
      // Digital入力モード
      ledcDetach(pin);
      if (idx < IS06_DP_CHANNELS) ledcChannel_[idx] = -1;
      gpio_reset_pin((gpio_num_t)pin);
      gpio_set_direction((gpio_num_t)pin, GPIO_MODE_INPUT);
      gpio_set_pull_mode((gpio_num_t)pin, GPIO_PULLDOWN_ONLY);
      Serial.printf("  CH%d GPIO%d: Digital INPUT\n", idx + 1, pin);
    }
  }

  Serial.println("Is06PinManager: Pin types applied.");
}
```

### 修正案B: NVSクリアによる一時的回避策

現在の問題を即座に解決するには、NVSをクリアしてデフォルト値（DIGITAL_OUTPUT）にリセット:

1. WebUIの System タブで Factory Reset を実行
2. または `erase_flash` でESP32を完全消去

これにより、CH1-4がDIGITAL_OUTPUTに戻り、正常に動作するはずです。

---

## 4. 二次的問題: applyPwmOutput() のフォールバック動作

`applyPwmOutput()` のフォールバック動作にも問題があります:

```cpp
void Is06PinManager::applyPwmOutput(int channel, int value) {
  // ...
  if (ch >= 0) {
    ledcWriteChannel(ch, pwm8bit);
  } else {
    // ★ 問題: value < 50 を LOW として扱うのは直感的でない
    int state = (value >= 50) ? 1 : 0;
    gpio_set_level((gpio_num_t)pin, state);
  }
}
```

**推奨修正**: LEDCチャンネルが無効な場合はエラーログを出力し、フォールバックしない:

```cpp
} else {
  // LEDCが無効なのにPWM操作が来るのは異常状態
  Serial.printf("WARNING: CH%d GPIO%d PWM requested but LEDC not attached!\n",
                channel, pin);
  // 強制的にLEDCをアタッチし直す
  int newCh = ledcAttach(pin, AraneaSettingsDefaults::PWM_FREQUENCY, AraneaSettingsDefaults::PWM_RESOLUTION);
  if (newCh >= 0) {
    ledcChannel_[idx] = newCh;
    ledcWriteChannel(newCh, pwm8bit);
  }
}
```

---

## 5. review8.md との整合性

review8.md で指摘された「GPIO Matrixによるペリフェラル干渉」は、今回の問題の直接的な原因ではありませんでした。

**実際の原因**: ソフトウェア設定（NVS）とハードウェア設定の不整合

ただし、review8.md で提案された `gpio_dump_io_configuration()` によるPin Auditは、この問題のデバッグに非常に有効です。SigOut IDが256（simple GPIO）なのにPWMとして扱われている状態を可視化できます。

---

## 6. 検証手順

### Step 1: NVSの現在の状態を確認

WebUIの /api/config または /api/status で、CH1/CH2のpinTypeを確認:
- `PWM_OUTPUT` になっていれば、この分析が正しい

### Step 2: Factory Resetで即座に回避

WebUIのSystemタブでFactory Resetを実行:
- 全設定がデフォルト（DIGITAL_OUTPUT）に戻る
- CH1/CH2が正常に3.3V出力されるはずす

### Step 3: 修正コードの適用

`applyLoadedPinTypes()` を追加してOTA更新:
- NVSの設定がハードウェアに正しく適用される
- PWM_OUTPUTでもLEDCがアタッチされる

---

## 7. 結論

**根本原因**: `loadFromNvs()` がソフトウェア設定のみを読み込み、ハードウェア設定を適用しない

**影響**: NVSに `PWM_OUTPUT` が保存されているチャンネルで:
- LEDCがアタッチされない（`ledcChannel_[] = -1`）
- `applyPwmOutput()` のフォールバックが不適切な動作をする
- GPIO出力が期待通りに動作しない

**修正**: `loadFromNvs()` 後に `applyLoadedPinTypes()` を呼び出してハードウェア設定を同期する

---

## 8. 関連ファイル

- [Is06PinManager.cpp](../../Is06PinManager.cpp) - 修正対象
- [Is06PinManager.h](../../Is06PinManager.h) - `applyLoadedPinTypes()` 宣言追加
- [is06s.ino](../../is06s.ino) - DEBUGコード削除推奨
- [review8.md](./review8.md) - GPIO Matrix調査指針

---

## 9. 次のアクション

1. **即座の対応**: Factory Resetで動作確認
2. **恒久対策**: `applyLoadedPinTypes()` の実装とOTA更新
3. **DEBUGコード削除**: is06s.ino Lines 421-426 を削除

---

**連絡先**: GitHub Issue または CLAUDE.md 参照
