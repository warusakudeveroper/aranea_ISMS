/**
 * StateReporterIs04a.h
 *
 * IS04A用 状態レポート送信
 */

#ifndef STATE_REPORTER_IS04A_H
#define STATE_REPORTER_IS04A_H

#include <Arduino.h>

class SettingManager;
class NtpManager;
class TriggerManager;

class StateReporterIs04a {
public:
    StateReporterIs04a();

    void begin(SettingManager* settings, NtpManager* ntp, TriggerManager* trigger);

    // 認証情報設定
    void setAuth(const String& tid, const String& lacisId, const String& cic);
    void setMac(const String& mac);

    // エンドポイント設定
    void setRelayUrls(const String& primary, const String& secondary);
    void setCloudUrl(const String& url);

    // 状態送信
    // force=true: インターバル制限を無視（パルス開始/終了イベント用）
    bool sendStateReport(bool force = false);
    bool sendHeartbeat();

    // 統計
    int getSentCount() const { return sentCount_; }
    int getFailCount() const { return failCount_; }
    String getLastResult() const { return lastResult_; }
    void resetStats();

private:
    SettingManager* settings_;
    NtpManager* ntp_;
    TriggerManager* trigger_;

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

    String buildLocalPayload();
    String buildCloudPayload();
    bool postToUrl(const String& url, const String& payload);
};

#endif // STATE_REPORTER_IS04A_H
