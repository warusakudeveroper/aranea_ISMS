#line 1 "/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/aranea_ISMS/code/ESP32/is05a/StateReporterIs05a.h"
/**
 * StateReporterIs05a.h
 *
 * is05a専用 状態レポート構築・送信モジュール
 *
 * ローカルリレー（Zero3）とクラウドへの状態送信を管理。
 */

#ifndef STATE_REPORTER_IS05A_H
#define STATE_REPORTER_IS05A_H

#include <Arduino.h>

class SettingManager;
class NtpManager;
class ChannelManager;

class StateReporterIs05a {
public:
    StateReporterIs05a();

    // ========================================
    // 初期化
    // ========================================
    void begin(SettingManager* settings, NtpManager* ntp, ChannelManager* channels);

    // ========================================
    // 認証情報設定
    // ========================================
    void setAuth(const String& tid, const String& lacisId, const String& cic);
    void setMac(const String& mac);

    // ========================================
    // エンドポイント設定
    // ========================================
    void setRelayUrls(const String& primary, const String& secondary);
    void setCloudUrl(const String& url);

    // ========================================
    // 送信
    // ========================================
    bool sendStateReport();
    bool sendHeartbeat();

    // ========================================
    // 統計
    // ========================================
    int getSentCount() const { return sentCount_; }
    int getFailCount() const { return failCount_; }
    String getLastResult() const { return lastResult_; }
    void resetStats();

private:
    SettingManager* settings_;
    NtpManager* ntp_;
    ChannelManager* channels_;

    String tid_;
    String lacisId_;
    String cic_;
    String mac_;

    String relayPrimaryUrl_;
    String relaySecondaryUrl_;
    String cloudUrl_;

    int sentCount_;
    int failCount_;
    String lastResult_;
    unsigned long lastSendTime_;

    // バックオフ制御
    int consecutiveFailures_;
    unsigned long lastFailTime_;

    // ペイロード構築
    String buildLocalPayload();
    String buildCloudPayload();

    // HTTP送信
    bool postToUrl(const String& url, const String& payload);
};

#endif // STATE_REPORTER_IS05A_H
