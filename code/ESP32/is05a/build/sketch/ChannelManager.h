#line 1 "/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/aranea_ISMS/code/ESP32/is05a/ChannelManager.h"
/**
 * ChannelManager.h
 *
 * 8ch入力管理モジュール
 *
 * IOControllerを使用して8チャンネルの入力/出力を管理。
 * - ch1-ch6: 入力専用
 * - ch7-ch8: 入力/出力切替可能
 *
 * 出力モード:
 * - Momentary: 指定時間後にOFF
 * - Alternate: トグル動作（ON/OFF切替）
 * - Interval: 指定回数繰り返し
 *
 * 設計原則:
 * - シングルタスク設計（loop()内でsample()/update()呼び出し）
 * - セマフォ/WDT不使用
 */

#ifndef CHANNEL_MANAGER_H
#define CHANNEL_MANAGER_H

#include <Arduino.h>
#include "IOController.h"

class SettingManager;
class NtpManager;

class ChannelManager {
public:
    static const int NUM_CHANNELS = 8;

    // 出力モード定義
    enum class OutputMode : uint8_t {
        MOMENTARY = 0,   // 指定時間後にOFF
        ALTERNATE = 1,   // トグル動作
        INTERVAL = 2     // 指定回数繰り返し
    };

    ChannelManager();

    // ========================================
    // 初期化
    // ========================================
    void begin(SettingManager* settings, NtpManager* ntp);

    // ========================================
    // メインループ処理
    // ========================================
    void sample();   // 全チャンネルの入力サンプリング
    void update();   // パルス終了チェック / インターバル処理

    // ========================================
    // 状態取得
    // ========================================
    bool hasChanged() const { return changed_; }
    void clearChanged() { changed_ = false; }
    int getLastChangedChannel() const { return lastChangedCh_; }

    // 個別チャンネル状態
    bool isActive(int ch) const;
    String getStateString(int ch) const;
    String getLastUpdatedAt(int ch) const;

    // ========================================
    // チャンネル設定
    // ========================================
    struct ChannelConfig {
        int pin;
        String name;
        String meaning;    // "open" or "close"
        int debounceMs;    // 10-10000ms
        bool inverted;
        bool isOutput;     // ch7/ch8のみ true可
        OutputMode outputMode;     // 出力モード
        int outputDurationMs;      // 出力時間(ms) 500-10000
        int intervalCount;         // インターバル回数
    };

    ChannelConfig getConfig(int ch) const;
    void setConfig(int ch, const ChannelConfig& config);

    // ch7/ch8 出力モード
    bool setOutputMode(int ch, bool output);
    bool isOutputMode(int ch) const;

    // 出力制御
    void setOutput(int ch, bool high);
    bool getOutputState(int ch) const;
    bool isOutputActive(int ch) const { return getOutputState(ch); }  // UI用エイリアス
    void pulse(int ch, int durationMs);

    // 出力モード設定
    void setOutputBehavior(int ch, OutputMode mode, int durationMs, int count);
    OutputMode getOutputBehavior(int ch) const;

    // 出力トリガー（モードに応じて動作）
    void triggerOutput(int ch);
    void stopOutput(int ch);

    // IOControllerへのアクセス
    IOController* getController(int ch);

    // ========================================
    // 変化コールバック
    // ========================================
    void onChannelChange(std::function<void(int ch, bool active)> callback);

    // ========================================
    // JSON出力（API用）
    // ========================================
    String toJson() const;
    String getChannelJson(int ch) const;

private:
    SettingManager* settings_;
    NtpManager* ntp_;
    IOController channels_[NUM_CHANNELS];
    ChannelConfig configs_[NUM_CHANNELS];
    String lastUpdatedAt_[NUM_CHANNELS];
    bool changed_;
    int lastChangedCh_;
    std::function<void(int, bool)> onChangeCallback_;

    // インターバル出力用状態
    struct IntervalState {
        bool active;
        int remainingCount;
        unsigned long lastToggleMs;
        bool currentState;
    };
    IntervalState intervalState_[NUM_CHANNELS];

    void loadConfig();
    void initChannels();
    void saveOutputConfig(int ch);
};

#endif // CHANNEL_MANAGER_H
