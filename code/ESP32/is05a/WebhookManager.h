/**
 * WebhookManager.h
 *
 * Webhook通知モジュール（v2.0）
 *
 * 最大5つの送信先エンドポイントを管理。
 * 各エンドポイントにチャンネルマスクとカスタム通知文を設定可能。
 * キュー管理とインターバル制御で安定送信を実現。
 */

#ifndef WEBHOOK_MANAGER_H
#define WEBHOOK_MANAGER_H

#include <Arduino.h>
#include <functional>
#include <vector>

class SettingManager;
class NtpManager;
class ChannelManager;

class WebhookManager {
public:
    static const int MAX_ENDPOINTS = 5;
    static const int DEFAULT_QUEUE_SIZE = 3;
    static const int DEFAULT_INTERVAL_SEC = 10;

    // エンドポイント設定
    struct Endpoint {
        String url;           // 送信先URL（Discord/Slack/Generic自動判定）
        uint8_t channelMask;  // 対象チャンネルマスク (bit0=ch1, 0xFF=全ch)
        String message;       // カスタム通知文（空欄時はデフォルト）
        bool enabled;         // 有効フラグ

        Endpoint() : channelMask(0xFF), enabled(false) {}
    };

    // キューアイテム
    struct QueueItem {
        int endpointIndex;    // 送信先エンドポイントインデックス
        int channel;          // チャンネル番号
        bool active;          // 検知状態
        String timestamp;     // タイムスタンプ
    };

    WebhookManager();

    // ========================================
    // 初期化
    // ========================================
    void begin(SettingManager* settings, NtpManager* ntp, ChannelManager* channels);

    // ========================================
    // メインループ処理（キュー処理）
    // ========================================
    void handle();

    // ========================================
    // グローバル設定
    // ========================================
    void setEnabled(bool enabled);
    bool isEnabled() const { return enabled_; }

    void setQueueSize(int size);
    int getQueueSize() const { return queueSize_; }

    void setIntervalSec(int sec);
    int getIntervalSec() const { return intervalSec_; }

    void setDeviceInfo(const String& lacisId, const String& deviceName);

    // ========================================
    // エンドポイント設定
    // ========================================
    void setEndpoint(int index, const Endpoint& ep);
    Endpoint getEndpoint(int index) const;
    int getEndpointCount() const;

    // レガシー互換（Discord/Slack/Generic直接設定）
    void setDiscordUrl(const String& url);
    void setSlackUrl(const String& url);
    void setGenericUrl(const String& url);
    String getDiscordUrl() const;
    String getSlackUrl() const;
    String getGenericUrl() const;

    // ========================================
    // 通知キューイング
    // ========================================
    bool sendStateChange(int ch, bool active);
    bool sendStateChangeToEndpoints(uint8_t endpointMask, int ch, bool active);  // ルール用
    bool sendHeartbeat();

    // テスト送信（即時実行、キュー経由しない）
    bool sendTestToEndpoint(int index);

    // ========================================
    // QuietMode（おやすみモード）設定
    // ========================================
    void setQuietModeEnabled(bool enabled);
    bool isQuietModeEnabled() const { return quietEnabled_; }
    void setQuietModeHours(int startHour, int endHour);
    int getQuietStartHour() const { return quietStartHour_; }
    int getQuietEndHour() const { return quietEndHour_; }
    bool isInQuietPeriod() const;

    // ========================================
    // 統計
    // ========================================
    int getSentCount() const { return sentCount_; }
    int getFailCount() const { return failCount_; }
    int getQuietSkipCount() const { return quietSkipCount_; }
    int getQueueDropCount() const { return queueDropCount_; }
    int getCurrentQueueLength() const { return queue_.size(); }
    void resetStats();

    // レガシー互換
    void setChannelMask(uint8_t mask);  // 全エンドポイントに適用
    uint8_t getChannelMask() const;     // エンドポイント0のマスク

private:
    SettingManager* settings_;
    NtpManager* ntp_;
    ChannelManager* channels_;

    bool enabled_;
    int queueSize_;
    int intervalSec_;
    String lacisId_;
    String deviceName_;

    // エンドポイント設定
    Endpoint endpoints_[MAX_ENDPOINTS];

    // キュー
    std::vector<QueueItem> queue_;
    unsigned long lastSendTime_;

    // QuietMode設定
    bool quietEnabled_;
    int quietStartHour_;
    int quietEndHour_;

    // 統計
    int sentCount_;
    int failCount_;
    int quietSkipCount_;
    int queueDropCount_;

    // バックオフ制御
    int consecutiveFailures_;
    unsigned long lastFailTime_;

    // 内部メソッド
    void loadConfig();
    void saveConfig();
    void saveEndpoint(int index);
    void migrateFromLegacy();
    void processQueue();

    // URL種別判定
    enum class UrlType { DISCORD, SLACK, GENERIC };
    UrlType detectUrlType(const String& url) const;

    // ペイロード構築
    String buildPayload(int endpointIndex, int ch, bool active, const String& timestamp);
    String buildDiscordPayload(int ch, bool active, const String& timestamp, const String& customMsg);
    String buildSlackPayload(int ch, bool active, const String& timestamp, const String& customMsg);
    String buildGenericPayload(int ch, bool active, const String& timestamp, const String& customMsg);

    // HTTP送信
    bool sendToUrl(const String& url, const String& payload);

    // バックオフ
    bool checkBackoff();
    void updateBackoff(bool success);
};

#endif // WEBHOOK_MANAGER_H
