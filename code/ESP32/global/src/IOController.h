/**
 * IOController.h
 *
 * 共通I/O制御モジュール
 *
 * GPIO入出力の統一的な制御を提供。
 * - INPUT/OUTPUTモードの動的切替（ch7/ch8用）
 * - 可変デバウンス処理（5-10000ms）
 * - 非ブロッキングパルス出力
 * - 論理反転（アクティブLOW/HIGH）
 *
 * 使用デバイス: is04a, is05a, is06a
 *
 * 設計原則:
 * - シングルタスク設計（loop()内でsample()/update()呼び出し）
 * - セマフォ/WDT不使用
 * - 動的メモリ確保なし
 */

#ifndef IO_CONTROLLER_H
#define IO_CONTROLLER_H

#include <Arduino.h>
#include <functional>

class IOController {
public:
    enum class Mode {
        IO_IN,    // 入力モード（デバウンス有効）
        IO_OUT    // 出力モード（パルス有効）
    };

    IOController();

    // ========================================
    // 初期化
    // ========================================
    void begin(int pin);
    void begin(int pin, Mode mode);

    // ========================================
    // モード設定
    // ========================================
    bool setMode(Mode mode);  // false if unsafe or during pulse
    Mode getMode() const { return mode_; }
    int getPin() const { return pin_; }

    // ========================================
    // 入力設定
    // ========================================
    void setDebounceMs(int ms);  // 5-10000ms
    int getDebounceMs() const { return debounceMs_; }
    void setInverted(bool inverted);
    bool isInverted() const { return inverted_; }
    void setPullup(bool pullup);  // true=INPUT_PULLUP, false=INPUT_PULLDOWN
    bool isPullup() const { return pullup_; }

    // ========================================
    // 入力処理（loop()で呼び出し）
    // ========================================
    void sample();
    bool hasChanged() const { return changed_; }
    void clearChanged() { changed_ = false; }
    bool isActive() const;
    int getRawValue() const { return rawState_; }
    int getStableValue() const { return stableState_; }
    String getStateString(const String& activeStr = "open",
                          const String& inactiveStr = "close") const;

    // ========================================
    // 出力処理
    // ========================================
    void setOutput(bool high);
    bool getOutput() const { return currentOutput_; }
    void pulse(int durationMs);
    void update();  // loop()で呼び出し（パルス終了チェック）
    bool isPulseActive() const { return pulseActive_; }

    // ========================================
    // コールバック
    // ========================================
    void onStateChange(std::function<void(bool active)> callback);
    void onPulseEnd(std::function<void()> callback);

private:
    int pin_;
    Mode mode_;
    bool inverted_;
    bool pullup_;
    bool initialized_;

    // 入力用
    int debounceMs_;
    int rawState_;
    int stableState_;
    unsigned long lastSampleMs_;
    unsigned long stableStartMs_;  // rawStateが安定し始めた時刻
    bool changed_;
    std::function<void(bool)> onChangeCallback_;

    // 出力用
    bool currentOutput_;
    bool pulseActive_;
    unsigned long pulseStartMs_;
    unsigned long pulseDurationMs_;
    std::function<void()> onPulseEndCallback_;

    // 安全チェック
    bool isSafeToSwitch(int pin) const;
    void transitionToInput();
    void transitionToOutput();
};

#endif // IO_CONTROLLER_H
