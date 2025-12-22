# IOController - 共通I/O制御モジュール仕様書

**作成日**: 2025/12/22
**配置先**: `code/ESP32/global/src/IOController.h/cpp`
**使用デバイス**: is04a, is05a, is06a

---

## 1. 概要

IOControllerは、GPIO入出力の統一的な制御を提供する共通モジュール。入力/出力モードの切替、デバウンス処理、パルス出力を1つのクラスで実現する。

---

## 2. 機能

| 機能 | 説明 |
|------|------|
| モード切替 | INPUT / OUTPUT の動的切替 |
| デバウンス | 5ms〜10,000ms の可変デバウンス |
| パルス出力 | 非ブロッキングパルス出力 |
| 論理反転 | アクティブLOW/HIGHの切替 |
| 状態変化検出 | 入力時の状態変化イベント |

---

## 3. API設計

### 3.1 クラス定義

```cpp
// IOController.h
#pragma once
#include <Arduino.h>
#include <functional>

class IOController {
public:
    enum class Mode {
        INPUT,    // 入力モード（デバウンス有効）
        OUTPUT    // 出力モード（パルス有効）
    };

    IOController();

    // 初期化
    void begin(int pin);
    void begin(int pin, Mode mode);

    // モード設定
    void setMode(Mode mode);
    Mode getMode() const;

    // 入力設定
    void setDebounceMs(int ms);  // 5-10000ms
    void setInverted(bool inverted);
    void setPullup(bool pullup);  // INPUT_PULLUP or INPUT_PULLDOWN

    // 入力処理（loop()で呼び出し）
    void sample();
    bool hasChanged() const;
    void clearChanged();
    bool isActive() const;
    int getRawValue() const;
    String getStateString(const String& activeStr = "open",
                          const String& inactiveStr = "close") const;

    // 出力処理
    void setOutput(bool high);
    void pulse(int durationMs);
    void update();  // loop()で呼び出し（パルス終了チェック）
    bool isPulseActive() const;

    // コールバック
    void onStateChange(std::function<void(bool active)> callback);
    void onPulseEnd(std::function<void()> callback);

private:
    int pin_;
    Mode mode_;
    bool inverted_;
    bool pullup_;

    // 入力用
    int debounceMs_;
    int rawState_;
    int stableState_;
    unsigned long lastChangeMs_;
    int stableCount_;
    bool changed_;
    std::function<void(bool)> onChangeCallback_;

    // 出力用
    bool pulseActive_;
    unsigned long pulseStartMs_;
    unsigned long pulseDurationMs_;
    std::function<void()> onPulseEndCallback_;
};
```

### 3.2 使用例

```cpp
#include "IOController.h"

IOController ch1;
IOController ch7;

void setup() {
    // ch1: 入力モード（リードスイッチ）
    ch1.begin(4, IOController::Mode::INPUT);
    ch1.setDebounceMs(100);
    ch1.setInverted(false);
    ch1.onStateChange([](bool active) {
        Serial.printf("ch1 changed: %s\n", active ? "OPEN" : "CLOSE");
    });

    // ch7: 出力モード（警報灯）
    ch7.begin(18, IOController::Mode::OUTPUT);
}

void loop() {
    // 入力サンプリング
    ch1.sample();
    if (ch1.hasChanged()) {
        ch1.clearChanged();
        if (ch1.isActive()) {
            ch7.pulse(3000);  // 警報灯3秒点灯
        }
    }

    // パルス更新
    ch7.update();
}
```

---

## 4. 実装詳細

### 4.1 デバウンス処理

```cpp
void IOController::sample() {
    if (mode_ != Mode::INPUT) return;

    unsigned long now = millis();
    if (now - lastSampleMs_ < 1) return;  // 1ms最小間隔
    lastSampleMs_ = now;

    int current = digitalRead(pin_);
    if (inverted_) current = !current;

    if (current == rawState_) {
        stableCount_++;
        if (stableCount_ >= (debounceMs_ / 1)) {  // 1msサンプリング
            if (current != stableState_) {
                stableState_ = current;
                changed_ = true;
                if (onChangeCallback_) {
                    onChangeCallback_(stableState_ == HIGH);
                }
            }
        }
    } else {
        rawState_ = current;
        stableCount_ = 0;
    }
}
```

### 4.2 パルス出力（非ブロッキング）

```cpp
void IOController::pulse(int durationMs) {
    if (mode_ != Mode::OUTPUT) return;
    if (pulseActive_) return;  // 既にパルス中

    digitalWrite(pin_, HIGH);
    pulseActive_ = true;
    pulseStartMs_ = millis();
    pulseDurationMs_ = durationMs;
}

void IOController::update() {
    if (!pulseActive_) return;

    if (millis() - pulseStartMs_ >= pulseDurationMs_) {
        digitalWrite(pin_, LOW);
        pulseActive_ = false;
        if (onPulseEndCallback_) {
            onPulseEndCallback_();
        }
    }
}
```

---

## 5. 設計原則

### 5.1 シングルタスク設計

- loop()内で`sample()`と`update()`を呼び出すのみ
- セマフォ不使用
- WDT制御不使用
- タスク生成不使用

### 5.2 メモリ効率

- 1インスタンスあたり約50バイト
- コールバックはstd::function（オプショナル）
- 動的メモリ確保なし

---

## 6. 使用デバイス

| デバイス | 使用箇所 |
|---------|---------|
| is04a | 2ch入力 + 2ch出力 |
| is05a | 6ch入力 + 2ch入出力切替 |
| is06a | 4ch出力 + 2ch入出力切替 |

---

## 7. 参照

- **is05a設計書**: `code/ESP32/is05a/design/DESIGN.md`
- **is06a設計書**: `code/ESP32/is06a/design/DESIGN.md`
