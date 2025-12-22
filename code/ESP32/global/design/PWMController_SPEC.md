# PWMController - 共通PWM制御モジュール仕様書

**作成日**: 2025/12/22
**配置先**: `code/ESP32/global/src/PWMController.h/cpp`
**使用デバイス**: is06a

---

## 1. 概要

PWMControllerは、ESP32のLEDC PWM出力を統一的に制御する共通モジュール。周波数・解像度設定、デューティ制御、フェード機能を提供する。

---

## 2. 機能

| 機能 | 説明 |
|------|------|
| PWM出力 | LEDC経由のハードウェアPWM |
| 周波数設定 | 1kHz〜40kHz |
| 解像度設定 | 8bit (0-255) / 10bit (0-1023) |
| デューティ設定 | 値/パーセント指定 |
| フェード | 非ブロッキングの滑らかな変化 |

---

## 3. API設計

### 3.1 クラス定義

```cpp
// PWMController.h
#pragma once
#include <Arduino.h>
#include <functional>

class PWMController {
public:
    PWMController();

    // 初期化
    void begin(int pin, int channel);  // channel: 0-15

    // PWM設定
    void setFrequency(int hz);         // 1000-40000
    void setResolution(int bits);      // 8 or 10
    int getMaxDuty() const;            // 255 (8bit) or 1023 (10bit)

    // デューティ設定
    void setDuty(int duty);            // 0-255 (8bit) or 0-1023 (10bit)
    void setDutyPercent(float pct);    // 0.0-100.0%
    int getDuty() const;
    float getDutyPercent() const;

    // ON/OFF
    void on();   // 100%
    void off();  // 0%

    // フェード（非ブロッキング）
    void fadeTo(int targetDuty, int durationMs);
    void fadeToPercent(float targetPct, int durationMs);
    void update();  // loop()で呼び出し
    bool isFading() const;

    // コールバック
    void onFadeComplete(std::function<void()> callback);

private:
    int pin_;
    int channel_;
    int frequency_;
    int resolution_;
    int currentDuty_;

    // フェード用
    bool fading_;
    int fadeStartDuty_;
    int fadeTargetDuty_;
    unsigned long fadeStartMs_;
    unsigned long fadeDurationMs_;
    std::function<void()> onFadeCompleteCallback_;

    void applyDuty(int duty);
};
```

### 3.2 使用例

```cpp
#include "PWMController.h"

PWMController led1;
PWMController led2;

void setup() {
    // LED1: GPIO7, チャンネル0
    led1.begin(7, 0);
    led1.setFrequency(5000);
    led1.setResolution(8);

    // LED2: GPIO12, チャンネル1
    led2.begin(12, 1);
    led2.setFrequency(5000);
    led2.setResolution(8);
}

void loop() {
    // 即時設定
    led1.setDutyPercent(50.0);  // 50%

    // フェード
    if (!led2.isFading()) {
        static bool fadeUp = true;
        if (fadeUp) {
            led2.fadeTo(255, 1000);  // 1秒かけて100%へ
        } else {
            led2.fadeTo(0, 1000);    // 1秒かけて0%へ
        }
        fadeUp = !fadeUp;
    }

    led1.update();
    led2.update();
}
```

---

## 4. 実装詳細

### 4.1 初期化

```cpp
void PWMController::begin(int pin, int channel) {
    pin_ = pin;
    channel_ = channel;
    frequency_ = 5000;
    resolution_ = 8;
    currentDuty_ = 0;

    ledcSetup(channel_, frequency_, resolution_);
    ledcAttachPin(pin_, channel_);
    ledcWrite(channel_, 0);
}
```

### 4.2 フェード処理（非ブロッキング）

```cpp
void PWMController::fadeTo(int targetDuty, int durationMs) {
    fadeStartDuty_ = currentDuty_;
    fadeTargetDuty_ = constrain(targetDuty, 0, getMaxDuty());
    fadeStartMs_ = millis();
    fadeDurationMs_ = durationMs;
    fading_ = true;
}

void PWMController::update() {
    if (!fading_) return;

    unsigned long elapsed = millis() - fadeStartMs_;
    if (elapsed >= fadeDurationMs_) {
        // フェード完了
        applyDuty(fadeTargetDuty_);
        fading_ = false;
        if (onFadeCompleteCallback_) {
            onFadeCompleteCallback_();
        }
    } else {
        // 線形補間
        float progress = (float)elapsed / fadeDurationMs_;
        int duty = fadeStartDuty_ + (fadeTargetDuty_ - fadeStartDuty_) * progress;
        applyDuty(duty);
    }
}

void PWMController::applyDuty(int duty) {
    currentDuty_ = constrain(duty, 0, getMaxDuty());
    ledcWrite(channel_, currentDuty_);
}
```

---

## 5. ESP32 LEDCチャンネル

| チャンネル | 推奨用途 |
|-----------|---------|
| 0-3 | PWMController用 |
| 4-7 | 予備 |
| 8-15 | 高速PWM用 |

**注意**: 同じチャンネルを複数ピンで共有可能だが、周波数・解像度は共通

---

## 6. 設計原則

### 6.1 シングルタスク設計

- loop()内で`update()`を呼び出すのみ
- セマフォ不使用
- WDT制御不使用
- タスク生成不使用

### 6.2 メモリ効率

- 1インスタンスあたり約40バイト
- 動的メモリ確保なし

---

## 7. 使用デバイス

| デバイス | 使用箇所 |
|---------|---------|
| is06a | GPIO 07, 12, 14, 27 のPWM出力 |

---

## 8. 参照

- **is06a設計書**: `code/ESP32/is06a/design/DESIGN.md`
- **ESP32 LEDCドキュメント**: https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/peripherals/ledc.html
