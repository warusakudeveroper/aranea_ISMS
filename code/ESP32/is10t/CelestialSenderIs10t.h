/**
 * CelestialSenderIs10t.h
 *
 * CelestialGlobe SSOT送信モジュール（is10t固有）
 *
 * 責務:
 * - Tapoカメラ状態JSONペイロード構築
 * - cameras[]配列構築（MAC照合用データ含む）
 * - HTTP POST送信
 */

#ifndef CELESTIAL_SENDER_IS10T_H
#define CELESTIAL_SENDER_IS10T_H

#include <Arduino.h>
#include "TapoPollerIs10t.h"

// 前方宣言
class SettingManager;
class NtpManager;

class CelestialSenderIs10t {
public:
    CelestialSenderIs10t();

    /**
     * 初期化
     */
    void begin(SettingManager* settings, NtpManager* ntp, TapoPollerIs10t* poller);

    /**
     * 認証情報設定
     */
    void setAuth(const String& tid, const String& lacisId,
                 const String& cic, const String& fid);

    /**
     * source設定（CelestialGlobe payload.source用）
     */
    void setSource(const String& source) { source_ = source; }

    /**
     * CelestialGlobeへ送信
     * @return 成功フラグ
     */
    bool send();

    /**
     * 設定済みチェック（endpoint/secretが設定されているか）
     */
    bool isConfigured();

private:
    SettingManager* settings_ = nullptr;
    NtpManager* ntp_ = nullptr;
    TapoPollerIs10t* poller_ = nullptr;

    // 認証情報
    String tid_;
    String lacisId_;
    String cic_;
    String fid_;
    String source_;

    // バックオフ制御
    int consecutiveFailures_ = 0;
    unsigned long lastFailTime_ = 0;
};

#endif // CELESTIAL_SENDER_IS10T_H
