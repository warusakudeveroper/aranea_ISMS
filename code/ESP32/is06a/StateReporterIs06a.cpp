/**
 * StateReporterIs06a.cpp
 */

#include "StateReporterIs06a.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "TriggerManagerIs06a.h"
#include <WiFi.h>
#include <HTTPClient.h>
#include <ArduinoJson.h>

static const unsigned long MIN_SEND_INTERVAL_MS = 1000;
// HTTPタイムアウト: 1.5秒に短縮（3エンドポイント×1.5秒=最大4.5秒）
// 元は3秒×3=9秒でWDT/メインループブロックの懸念があった
static const unsigned long HTTP_TIMEOUT_MS = 1500;
static const int MAX_CONSECUTIVE_FAILURES = 3;
static const unsigned long BACKOFF_DURATION_MS = 30000;

StateReporterIs06a::StateReporterIs06a()
    : settings_(nullptr)
    , ntp_(nullptr)
    , trigger_(nullptr)
    , sentCount_(0)
    , failCount_(0)
    , lastResult_("---")
    , lastSendTime_(0)
    , consecutiveFailures_(0)
    , lastFailTime_(0)
{
}

void StateReporterIs06a::begin(SettingManager* settings, NtpManager* ntp, TriggerManagerIs06a* trigger) {
    settings_ = settings;
    ntp_ = ntp;
    trigger_ = trigger;
    Serial.println("[StateReporterIs06a] Initialized");
}

void StateReporterIs06a::setAuth(const String& tid, const String& lacisId, const String& cic) {
    tid_ = tid;
    lacisId_ = lacisId;
    cic_ = cic;
}

void StateReporterIs06a::setMac(const String& mac) {
    mac_ = mac;
}

void StateReporterIs06a::setRelayUrls(const String& primary, const String& secondary) {
    relayPrimaryUrl_ = primary;
    relaySecondaryUrl_ = secondary;
}

void StateReporterIs06a::setCloudUrl(const String& url) {
    cloudUrl_ = url;
}

bool StateReporterIs06a::sendStateReport(bool force) {
    unsigned long now = millis();

    if (!WiFi.isConnected()) {
        Serial.println("[StateReporter] Skipped (WiFi not connected)");
        return false;
    }

    // バックオフチェック
    if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
        if ((now - lastFailTime_) < BACKOFF_DURATION_MS) {
            Serial.println("[StateReporter] Skipped (backoff)");
            return false;
        }
        Serial.println("[StateReporter] Backoff period ended, retrying...");
        consecutiveFailures_ = 0;
    }

    // 最小送信間隔チェック
    if (!force && lastSendTime_ > 0 && (now - lastSendTime_) < MIN_SEND_INTERVAL_MS) {
        Serial.println("[StateReporter] Skipped (interval limit)");
        return false;
    }
    lastSendTime_ = now;

    Serial.println("[StateReporter] Sending state report...");

    int successCount = 0;

    String localPayload = buildLocalPayload();

    if (relayPrimaryUrl_.length() > 0) {
        if (postToUrl(relayPrimaryUrl_, localPayload)) successCount++;
        yield();
    }

    if (relaySecondaryUrl_.length() > 0) {
        if (postToUrl(relaySecondaryUrl_, localPayload)) successCount++;
        yield();
    }

    if (cloudUrl_.length() > 0 && tid_.length() > 0 && cic_.length() > 0) {
        String cloudPayload = buildCloudPayload();
        if (postToUrl(cloudUrl_, cloudPayload)) successCount++;
        yield();
    }

    if (successCount > 0) {
        sentCount_++;
        lastResult_ = "OK(" + String(successCount) + ")";
        consecutiveFailures_ = 0;
    } else {
        failCount_++;
        lastResult_ = "NG";
        consecutiveFailures_++;
        lastFailTime_ = now;
        if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
            Serial.printf("[StateReporter] Entering backoff (%d consecutive failures)\n", consecutiveFailures_);
        }
    }

    Serial.printf("[StateReporter] Done: %d success\n", successCount);
    return successCount > 0;
}

bool StateReporterIs06a::sendHeartbeat() {
    return sendStateReport();
}

void StateReporterIs06a::resetStats() {
    sentCount_ = 0;
    failCount_ = 0;
    lastResult_ = "---";
}

String StateReporterIs06a::buildLocalPayload() {
    String observedAt = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";
    String ip = WiFi.localIP().toString();
    int rssi = WiFi.RSSI();
    String ssid = WiFi.SSID();

    StaticJsonDocument<1536> doc;
    doc["observedAt"] = observedAt;

    // sensor
    JsonObject sensor = doc.createNestedObject("sensor");
    sensor["lacisId"] = lacisId_;
    sensor["mac"] = mac_;
    sensor["productType"] = "006";
    sensor["productCode"] = "0001";

    // state (6ch対応)
    JsonObject state = doc.createNestedObject("state");

    for (int i = 1; i <= 6; i++) {
        auto trg = trigger_->getState(i);
        String prefix = "TRG" + String(i);

        state[prefix + "_Active"] = trg.active;
        state[prefix + "_Name"] = trg.name;
        state[prefix + "_lastUpdatedAt"] = trg.lastUpdatedAt;

        const char* modeStr = (trg.mode == TriggerManagerIs06a::TriggerMode::MODE_PWM) ? "pwm" :
                              (trg.mode == TriggerManagerIs06a::TriggerMode::MODE_INPUT) ? "input" : "digital";
        state[prefix + "_Mode"] = modeStr;

        if (trg.mode == TriggerManagerIs06a::TriggerMode::MODE_PWM) {
            state[prefix + "_PWMDuty"] = trg.pwmDuty;
        }
    }

    // ネットワーク情報
    state["rssi"] = String(rssi);
    state["ipaddr"] = ip;
    state["SSID"] = ssid;

    // meta
    JsonObject meta = doc.createNestedObject("meta");
    meta["observedAt"] = observedAt;
    meta["direct"] = true;

    // gateway
    JsonObject gateway = doc.createNestedObject("gateway");
    gateway["lacisId"] = lacisId_;
    gateway["ip"] = ip;
    gateway["rssi"] = rssi;

    String json;
    json.reserve(1024);
    serializeJson(doc, json);
    return json;
}

String StateReporterIs06a::buildCloudPayload() {
    String observedAt = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";

    StaticJsonDocument<1024> doc;

    // auth
    JsonObject auth = doc.createNestedObject("auth");
    auth["tid"] = tid_;
    auth["lacisId"] = lacisId_;
    auth["cic"] = cic_;

    // report
    JsonObject report = doc.createNestedObject("report");
    report["lacisId"] = lacisId_;
    report["type"] = "aranea_ar-is06a";
    report["observedAt"] = observedAt;

    // state (6ch対応)
    JsonObject state = report.createNestedObject("state");

    for (int i = 1; i <= 6; i++) {
        auto trg = trigger_->getState(i);
        state["TRG" + String(i)] = trg.active;
        if (trg.mode == TriggerManagerIs06a::TriggerMode::MODE_PWM) {
            state["TRG" + String(i) + "_PWM"] = trg.pwmDuty;
        }
    }

    String json;
    json.reserve(512);
    serializeJson(doc, json);
    return json;
}

bool StateReporterIs06a::postToUrl(const String& url, const String& payload) {
    if (url.length() == 0) return false;

    HTTPClient http;
    http.begin(url);
    http.addHeader("Content-Type", "application/json");
    http.setTimeout(HTTP_TIMEOUT_MS);

    int httpCode = http.POST(payload);
    yield();

    bool success = (httpCode >= 200 && httpCode < 300);

    if (success) {
        Serial.printf("[StateReporter] OK %d -> %s\n", httpCode, url.substring(0, 50).c_str());
    } else {
        Serial.printf("[StateReporter] NG %d -> %s\n", httpCode, url.substring(0, 50).c_str());
    }

    http.end();
    return success;
}
