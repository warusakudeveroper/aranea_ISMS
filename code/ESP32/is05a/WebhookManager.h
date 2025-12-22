/**
 * WebhookManager.h
 *
 * Webhook通知モジュール
 *
 * Discord/Slack/Generic Webhook への通知送信を管理。
 * 状態変化時に各プラットフォームへ非同期で通知を送信する。
 */

#ifndef WEBHOOK_MANAGER_H
#define WEBHOOK_MANAGER_H

#include <Arduino.h>
#include <functional>

class SettingManager;
class NtpManager;
class ChannelManager;

class WebhookManager {
public:
    enum class Platform {
        DISCORD,
        SLACK,
        GENERIC
    };

    WebhookManager();

    // ========================================
    // 初期化
    // ========================================
    void begin(SettingManager* settings, NtpManager* ntp, ChannelManager* channels);

    // ========================================
    // 設定
    // ========================================
    void setEnabled(bool enabled);
    bool isEnabled() const { return enabled_; }

    void setDiscordUrl(const String& url);
    void setSlackUrl(const String& url);
    void setGenericUrl(const String& url);

    String getDiscordUrl() const { return discordUrl_; }
    String getSlackUrl() const { return slackUrl_; }
    String getGenericUrl() const { return genericUrl_; }

    void setDeviceInfo(const String& lacisId, const String& deviceName);

    // ========================================
    // 通知送信
    // ========================================
    bool sendStateChange(int ch, bool active);
    bool sendHeartbeat();

    // ========================================
    // 送信結果コールバック
    // ========================================
    void onSendComplete(std::function<void(Platform, bool success)> callback);

    // ========================================
    // 統計
    // ========================================
    int getSentCount() const { return sentCount_; }
    int getFailCount() const { return failCount_; }
    void resetStats() { sentCount_ = 0; failCount_ = 0; }

private:
    SettingManager* settings_;
    NtpManager* ntp_;
    ChannelManager* channels_;

    bool enabled_;
    String discordUrl_;
    String slackUrl_;
    String genericUrl_;
    String lacisId_;
    String deviceName_;

    int sentCount_;
    int failCount_;

    std::function<void(Platform, bool)> onSendCallback_;

    // プラットフォーム固有フォーマット
    String buildDiscordPayload(int ch, bool active, const String& timestamp);
    String buildSlackPayload(int ch, bool active, const String& timestamp);
    String buildGenericPayload(int ch, bool active, const String& timestamp);

    // HTTP送信
    bool sendToUrl(const String& url, const String& payload);
};

#endif // WEBHOOK_MANAGER_H
