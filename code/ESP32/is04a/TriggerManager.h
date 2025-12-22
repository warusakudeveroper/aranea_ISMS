/**
 * TriggerManager.h
 *
 * IS04A用 入出力連動管理
 *
 * 機能:
 * - 2ch物理入力（デバウンス付き）
 * - 2ch接点出力（パルス出力）
 * - トリガーアサイン（入力→出力のマッピング）
 * - インターロック（連続パルス防止）
 *
 * 使用モジュール: IOController（共通）
 */

#ifndef TRIGGER_MANAGER_H
#define TRIGGER_MANAGER_H

#include <Arduino.h>
#include <functional>
#include "IOController.h"

class SettingManager;
class NtpManager;

class TriggerManager {
public:
    // パルスソース
    enum class PulseSource {
        NONE,
        CLOUD,    // クラウド（MQTT）からの命令
        MANUAL,   // 物理入力
        HTTP      // Web API
    };

    // 出力状態
    struct OutputState {
        bool active;
        int pin;
        String name;
        unsigned long startMs;
        unsigned long durationMs;
        PulseSource source;
    };

    // 入力状態
    struct InputState {
        bool active;
        int pin;
        int targetOutput;  // 1 or 2
        String lastUpdatedAt;
    };

    TriggerManager();

    // 初期化
    void begin(SettingManager* settings, NtpManager* ntp);

    // loop()で呼び出し
    void sample();   // 入力サンプリング
    void update();   // 出力状態更新

    // パルス実行
    bool startPulse(int outputNum, int durationMs, PulseSource source);
    bool isPulseActive() const;
    bool isPulseActive(int outputNum) const;
    PulseSource getCurrentSource() const { return currentSource_; }
    int getCurrentOutput() const { return currentOutput_; }

    // 入力状態
    bool isInputActive(int inputNum) const;
    InputState getInputState(int inputNum) const;

    // 出力状態
    OutputState getOutputState(int outputNum) const;

    // 設定
    void setTriggerAssign(int inputNum, int targetOutput);
    int getTriggerAssign(int inputNum) const;
    void setOutputName(int outputNum, const String& name);
    String getOutputName(int outputNum) const;
    void setPulseMs(int ms);
    int getPulseMs() const { return pulseMs_; }
    void setInterlockMs(int ms);
    int getInterlockMs() const { return interlockMs_; }
    void setDebounceMs(int ms);
    int getDebounceMs() const { return debounceMs_; }

    // コールバック
    void onPulseStart(std::function<void(int outputNum, PulseSource source)> callback);
    void onPulseEnd(std::function<void(int outputNum)> callback);
    void onInputChange(std::function<void(int inputNum, bool active)> callback);

    // 状態変化フラグ
    bool hasChanged() const { return changed_; }
    void clearChanged() { changed_ = false; }

    // JSON出力
    String toJson() const;

private:
    SettingManager* settings_;
    NtpManager* ntp_;

    // IOController（2入力 + 2出力）
    IOController inputs_[2];   // 入力1, 入力2
    IOController outputs_[2];  // 出力1, 出力2

    // 設定値
    int pulseMs_;
    int interlockMs_;
    int debounceMs_;
    int triggerAssign_[2];  // 入力1→出力?, 入力2→出力?
    String outputNames_[2]; // 出力名

    // 状態
    bool changed_;
    PulseSource currentSource_;
    int currentOutput_;
    unsigned long lastPulseEndMs_;
    String lastUpdatedAt_[2];  // 入力最終更新時刻

    // エッジ検出用
    bool lastInputStable_[2];

    // コールバック
    std::function<void(int, PulseSource)> onPulseStartCallback_;
    std::function<void(int)> onPulseEndCallback_;
    std::function<void(int, bool)> onInputChangeCallback_;

    // 内部処理
    void loadConfig();
    void handleInputEdges();
    String getTimestamp() const;
};

#endif // TRIGGER_MANAGER_H
