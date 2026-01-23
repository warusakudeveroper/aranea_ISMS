/**
 * StateReporterIs06s.h
 *
 * IS06S用 状態レポート送信
 *
 * 機能:
 * - ローカルRelay (Orange Pi Zero3) への状態送信
 * - クラウド (deviceStateReport) への状態送信
 * - バックオフ制御
 * - 最小送信間隔制御
 */

#ifndef STATE_REPORTER_IS06S_H
#define STATE_REPORTER_IS06S_H

#include <Arduino.h>

class SettingManager;
class NtpManager;
class Is06PinManager;

class StateReporterIs06s {
public:
    StateReporterIs06s();

    /**
     * 初期化
     * @param settings SettingManager参照
     * @param ntp NtpManager参照
     * @param pinMgr Is06PinManager参照
     */
    void begin(SettingManager* settings, NtpManager* ntp, Is06PinManager* pinMgr);

    // 認証情報設定
    void setAuth(const String& tid, const String& lacisId, const String& cic);
    void setMac(const String& mac);
    void setFirmwareVersion(const String& version);

    // エンドポイント設定
    void setRelayUrls(const String& primary, const String& secondary);
    void setCloudUrl(const String& url);

    /**
     * 状態送信
     * @param force true: インターバル制限を無視（トリガーイベント用）
     * @return 成功時true
     */
    bool sendStateReport(bool force = false);

    /**
     * ハートビート送信（状態送信と同じ）
     */
    bool sendHeartbeat();

    // 定期送信処理（loop()から呼び出し）
    void update();

    // 送信間隔設定（秒）
    void setHeartbeatInterval(int seconds);

    // 統計
    int getSentCount() const { return sentCount_; }
    int getFailCount() const { return failCount_; }
    String getLastResult() const { return lastResult_; }
    void resetStats();

private:
    SettingManager* settings_;
    NtpManager* ntp_;
    Is06PinManager* pinMgr_;

    String tid_;
    String lacisId_;
    String cic_;
    String mac_;
    String firmwareVersion_;

    String relayPrimaryUrl_;
    String relaySecondaryUrl_;
    String cloudUrl_;

    int sentCount_;
    int failCount_;
    String lastResult_;
    unsigned long lastSendTime_;
    unsigned long lastHeartbeatTime_;
    int heartbeatIntervalSec_;

    // バックオフ制御
    int consecutiveFailures_;
    unsigned long lastFailTime_;

    String buildLocalPayload();
    String buildCloudPayload();
    bool postToUrl(const String& url, const String& payload);
};

#endif // STATE_REPORTER_IS06S_H
