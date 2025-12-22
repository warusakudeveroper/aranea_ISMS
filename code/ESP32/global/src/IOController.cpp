/**
 * IOController.cpp
 *
 * 共通I/O制御モジュール実装
 */

#include "IOController.h"

IOController::IOController()
    : pin_(-1)
    , mode_(Mode::IO_IN)
    , inverted_(false)
    , pullup_(true)
    , initialized_(false)
    , debounceMs_(100)
    , rawState_(HIGH)
    , stableState_(HIGH)
    , lastSampleMs_(0)
    , stableStartMs_(0)
    , changed_(false)
    , onChangeCallback_(nullptr)
    , currentOutput_(false)
    , pulseActive_(false)
    , pulseStartMs_(0)
    , pulseDurationMs_(0)
    , onPulseEndCallback_(nullptr)
{
}

// ========================================
// 初期化
// ========================================

void IOController::begin(int pin) {
    begin(pin, Mode::IO_IN);
}

void IOController::begin(int pin, Mode mode) {
    pin_ = pin;
    mode_ = mode;
    initialized_ = true;

    if (mode_ == Mode::IO_IN) {
        transitionToInput();
    } else {
        transitionToOutput();
    }

    Serial.printf("[IOController] GPIO%d initialized as %s\n",
        pin_, mode_ == Mode::IO_IN ? "INPUT" : "OUTPUT");
}

// ========================================
// モード切替
// ========================================

bool IOController::setMode(Mode newMode) {
    // 同一モードへの遷移は成功扱い
    if (mode_ == newMode) {
        return true;
    }

    // 未初期化チェック
    if (pin_ < 0 || !initialized_) {
        Serial.println("[IOController] ERROR: pin not initialized");
        return false;
    }

    // パルス実行中は切替禁止
    if (pulseActive_) {
        Serial.println("[IOController] ERROR: cannot switch mode during pulse");
        return false;
    }

    // 禁止ピンチェック
    if (!isSafeToSwitch(pin_)) {
        Serial.printf("[IOController] ERROR: GPIO%d is not safe for I/O switch\n", pin_);
        return false;
    }

    // 遷移実行
    if (newMode == Mode::IO_IN) {
        transitionToInput();
    } else {
        transitionToOutput();
    }

    mode_ = newMode;
    Serial.printf("[IOController] GPIO%d mode changed to %s\n",
        pin_, mode_ == Mode::IO_IN ? "INPUT" : "OUTPUT");

    return true;
}

bool IOController::isSafeToSwitch(int pin) const {
    // ストラッピングピン（起動に影響）
    if (pin == 0 || pin == 2 || pin == 15) {
        return false;
    }
    // 入力専用ピン
    if (pin >= 34 && pin <= 39) {
        return false;
    }
    // フラッシュ使用ピン（4MB WROOM）
    if (pin >= 6 && pin <= 11) {
        return false;
    }
    return true;
}

void IOController::transitionToInput() {
    // 1. 出力を停止
    if (mode_ == Mode::IO_OUT) {
        digitalWrite(pin_, LOW);
        pulseActive_ = false;
    }

    // 2. ピンモードを変更
    if (pullup_) {
        pinMode(pin_, INPUT_PULLUP);
    } else {
        pinMode(pin_, INPUT_PULLDOWN);
    }

    // 3. 安定化待ち
    delay(10);

    // 4. 状態変数を初期化
    rawState_ = digitalRead(pin_);
    stableState_ = rawState_;
    stableStartMs_ = millis();
    changed_ = false;
}

void IOController::transitionToOutput() {
    // 1. LOWを先に設定（グリッチ防止）
    digitalWrite(pin_, LOW);

    // 2. 出力モードに変更
    pinMode(pin_, OUTPUT);

    // 3. 状態変数をリセット
    pulseActive_ = false;
    currentOutput_ = false;
}

// ========================================
// 入力設定
// ========================================

void IOController::setDebounceMs(int ms) {
    debounceMs_ = constrain(ms, 5, 10000);
}

void IOController::setInverted(bool inverted) {
    inverted_ = inverted;
}

void IOController::setPullup(bool pullup) {
    pullup_ = pullup;
    if (initialized_ && mode_ == Mode::IO_IN) {
        if (pullup_) {
            pinMode(pin_, INPUT_PULLUP);
        } else {
            pinMode(pin_, INPUT_PULLDOWN);
        }
    }
}

// ========================================
// 入力処理
// ========================================

void IOController::sample() {
    if (mode_ != Mode::IO_IN) return;
    if (pin_ < 0) return;

    unsigned long now = millis();
    int current = digitalRead(pin_);

    if (current == rawState_) {
        // 同じ値が続いている - 経過時間をチェック
        if (current != stableState_ && (now - stableStartMs_) >= (unsigned long)debounceMs_) {
            // デバウンス時間経過でstableState更新
            stableState_ = current;
            changed_ = true;

            if (onChangeCallback_) {
                onChangeCallback_(isActive());
            }
        }
    } else {
        // 値が変わった、開始時刻をリセット
        rawState_ = current;
        stableStartMs_ = now;
    }
}

bool IOController::isActive() const {
    // activeLow: LOW=active (pullup時)
    // activeHigh: HIGH=active (pulldown時または反転時)
    bool rawActive = (stableState_ == LOW);
    return inverted_ ? !rawActive : rawActive;
}

String IOController::getStateString(const String& activeStr,
                                     const String& inactiveStr) const {
    return isActive() ? activeStr : inactiveStr;
}

// ========================================
// 出力処理
// ========================================

void IOController::setOutput(bool high) {
    if (mode_ != Mode::IO_OUT) return;
    if (pin_ < 0) return;

    currentOutput_ = high;
    digitalWrite(pin_, high ? HIGH : LOW);
}

void IOController::pulse(int durationMs) {
    if (mode_ != Mode::IO_OUT) return;
    if (pin_ < 0) return;
    if (pulseActive_) return;  // 既にパルス中

    digitalWrite(pin_, HIGH);
    currentOutput_ = true;
    pulseActive_ = true;
    pulseStartMs_ = millis();
    pulseDurationMs_ = durationMs;

    Serial.printf("[IOController] GPIO%d pulse started (%dms)\n", pin_, durationMs);
}

void IOController::update() {
    if (!pulseActive_) return;

    if (millis() - pulseStartMs_ >= pulseDurationMs_) {
        digitalWrite(pin_, LOW);
        currentOutput_ = false;
        pulseActive_ = false;

        Serial.printf("[IOController] GPIO%d pulse ended\n", pin_);

        if (onPulseEndCallback_) {
            onPulseEndCallback_();
        }
    }
}

// ========================================
// コールバック
// ========================================

void IOController::onStateChange(std::function<void(bool active)> callback) {
    onChangeCallback_ = callback;
}

void IOController::onPulseEnd(std::function<void()> callback) {
    onPulseEndCallback_ = callback;
}
