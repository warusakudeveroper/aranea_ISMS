/**
 * TriggerManagerIs06a.h
 *
 * IS06A用 6chトリガー管理
 *
 * 機能（6CH構成）:
 * - TRG1-4: デジタル出力 + PWM対応 (GPIO 12,14,27,15)
 * - TRG5-6: I/O切替可能 (GPIO 05,18)
 * - パルス出力（デジタルモード時）
 * - PWM出力（PWMモード時）
 * - 入力時デバウンス（TRG5-6のみ）
 * - インターロック（連続パルス防止）
 *
 * 使用モジュール: IOController（共通）
 */

#ifndef TRIGGER_MANAGER_IS06A_H
#define TRIGGER_MANAGER_IS06A_H

#include <Arduino.h>
#include <functional>
#include "IOController.h"

class SettingManager;
class NtpManager;

class TriggerManagerIs06a {
public:
    static const int kNumTriggers = 6;
    static const int kNumPwmCapable = 4;   // TRG1-4
    static const int kNumIoSwitchable = 2; // TRG5-6

    // トリガーモード
    enum class TriggerMode {
        MODE_DIGITAL,  // デジタル出力（パルス）
        MODE_PWM,      // PWM出力
        MODE_INPUT     // 入力モード（TRG5-6のみ）
    };

    // パルスソース
    enum class PulseSource {
        NONE,
        CLOUD,    // クラウド（MQTT）からの命令
        MANUAL,   // 物理入力
        HTTP      // Web API
    };

    // トリガー状態
    struct TriggerState {
        int trgNum;           // 1-6
        int pin;              // GPIO番号
        TriggerMode mode;     // digital/pwm/input
        String name;          // 名称
        bool active;          // 出力中(digital/pwm) or 入力アクティブ
        int pulseMs;          // パルス幅(ms)
        int pwmFreq;          // PWM周波数(Hz)
        int pwmDuty;          // PWMデューティ(0-255)
        int debounceMs;       // デバウンス(ms) TRG5-6のみ
        String lastUpdatedAt; // 最終更新時刻
        PulseSource source;   // パルスソース
    };

    TriggerManagerIs06a();

    // 初期化
    void begin(SettingManager* settings, NtpManager* ntp);

    // loop()で呼び出し
    void sample();   // 入力サンプリング（TRG5-6がINPUTモード時）
    void update();   // 出力状態更新

    // トリガー実行
    bool startPulse(int trgNum, int durationMs, PulseSource source);
    bool setPwmDuty(int trgNum, int duty);
    bool setPwmDutyPercent(int trgNum, float percent);
    bool stopPwm(int trgNum);

    // 状態取得
    bool isPulseActive(int trgNum) const;
    bool isAnyPulseActive() const;
    bool isInputActive(int trgNum) const;
    TriggerState getState(int trgNum) const;

    // モード設定
    bool setMode(int trgNum, TriggerMode mode);
    TriggerMode getMode(int trgNum) const;
    bool isPwmCapable(int trgNum) const { return trgNum >= 1 && trgNum <= 4; }
    bool isIoSwitchable(int trgNum) const { return trgNum == 5 || trgNum == 6; }

    // 設定
    void setName(int trgNum, const String& name);
    String getName(int trgNum) const;
    void setPulseMs(int trgNum, int ms);
    int getPulseMs(int trgNum) const;
    void setPwmFreq(int trgNum, int hz);
    int getPwmFreq(int trgNum) const;
    void setDebounceMs(int trgNum, int ms);
    int getDebounceMs(int trgNum) const;

    // グローバル設定
    void setInterlockMs(int ms);
    int getInterlockMs() const { return interlockMs_; }

    // コールバック
    void onPulseStart(std::function<void(int trgNum, PulseSource source)> callback);
    void onPulseEnd(std::function<void(int trgNum)> callback);
    void onInputChange(std::function<void(int trgNum, bool active)> callback);
    void onPwmChange(std::function<void(int trgNum, int duty)> callback);

    // 状態変化フラグ
    bool hasChanged() const { return changed_; }
    void clearChanged() { changed_ = false; }

    // JSON出力
    String toJson() const;

private:
    SettingManager* settings_;
    NtpManager* ntp_;

    // IOController（6ch）
    IOController io_[kNumTriggers];

    // トリガー設定
    TriggerMode modes_[kNumTriggers];
    String names_[kNumTriggers];
    int pulseMsArr_[kNumTriggers];
    int pwmFreqArr_[kNumTriggers];
    int pwmDutyArr_[kNumTriggers];
    int debounceMsArr_[kNumTriggers];
    String lastUpdatedAt_[kNumTriggers];

    // PWM状態（TRG1-4）
    bool pwmActive_[kNumPwmCapable];
    int ledcChannels_[kNumPwmCapable];  // ledcAttachの戻り値を保存

    // グローバル設定
    int interlockMs_;

    // 状態
    bool changed_;
    unsigned long lastPulseEndMs_;

    // 現在パルス中のトリガー
    int currentPulseTrg_;
    PulseSource currentSource_;

    // エッジ検出用（TRG5-6入力時）
    bool lastInputStable_[kNumIoSwitchable];
    unsigned long lastInputEdgeMs_[kNumIoSwitchable];  // クールダウン用

    // コールバック
    std::function<void(int, PulseSource)> onPulseStartCallback_;
    std::function<void(int)> onPulseEndCallback_;
    std::function<void(int, bool)> onInputChangeCallback_;
    std::function<void(int, int)> onPwmChangeCallback_;

    // 内部処理
    void loadConfig();
    void handleInputEdges();
    String getTimestamp() const;

    // PWM制御（ESP32 LEDC API）
    void setupPwm(int trgNum);
    void teardownPwm(int trgNum);
    int getLedcChannel(int trgNum) const;
};

#endif // TRIGGER_MANAGER_IS06A_H
