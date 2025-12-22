/**
 * ChannelManager.h
 *
 * 8ch入力管理モジュール
 *
 * IOControllerを使用して8チャンネルの入力/出力を管理。
 * - ch1-ch6: 入力専用
 * - ch7-ch8: 入力/出力切替可能
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

    ChannelManager();

    // ========================================
    // 初期化
    // ========================================
    void begin(SettingManager* settings, NtpManager* ntp);

    // ========================================
    // メインループ処理
    // ========================================
    void sample();   // 全チャンネルの入力サンプリング
    void update();   // パルス終了チェック

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
        int debounceMs;
        bool inverted;
        bool isOutput;     // ch7/ch8のみ true可
    };

    ChannelConfig getConfig(int ch) const;
    void setConfig(int ch, const ChannelConfig& config);

    // ch7/ch8 出力モード
    bool setOutputMode(int ch, bool output);
    bool isOutputMode(int ch) const;
    void setOutput(int ch, bool high);
    void pulse(int ch, int durationMs);

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

    void loadConfig();
    void initChannels();
};

#endif // CHANNEL_MANAGER_H
