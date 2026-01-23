/**
 * StateReporterIs06s.cpp
 */

#include "StateReporterIs06s.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "Is06PinManager.h"
#include <WiFi.h>
#include <HTTPClient.h>
#include <ArduinoJson.h>

static const unsigned long MIN_SEND_INTERVAL_MS = 1000;
static const unsigned long HTTP_TIMEOUT_MS = 1500;
static const int MAX_CONSECUTIVE_FAILURES = 3;
static const unsigned long BACKOFF_DURATION_MS = 30000;
static const int DEFAULT_HEARTBEAT_INTERVAL_SEC = 60;

StateReporterIs06s::StateReporterIs06s()
    : settings_(nullptr)
    , ntp_(nullptr)
    , pinMgr_(nullptr)
    , sentCount_(0)
    , failCount_(0)
    , lastResult_("---")
    , lastSendTime_(0)
    , lastHeartbeatTime_(0)
    , heartbeatIntervalSec_(DEFAULT_HEARTBEAT_INTERVAL_SEC)
    , consecutiveFailures_(0)
    , lastFailTime_(0)
{
}

void StateReporterIs06s::begin(SettingManager* settings, NtpManager* ntp, Is06PinManager* pinMgr) {
    settings_ = settings;
    ntp_ = ntp;
    pinMgr_ = pinMgr;
    Serial.println("[StateReporterIs06s] Initialized");
}

void StateReporterIs06s::setAuth(const String& tid, const String& lacisId, const String& cic) {
    tid_ = tid;
    lacisId_ = lacisId;
    cic_ = cic;
}

void StateReporterIs06s::setMac(const String& mac) {
    mac_ = mac;
}

void StateReporterIs06s::setFirmwareVersion(const String& version) {
    firmwareVersion_ = version;
}

void StateReporterIs06s::setRelayUrls(const String& primary, const String& secondary) {
    relayPrimaryUrl_ = primary;
    relaySecondaryUrl_ = secondary;
}

void StateReporterIs06s::setCloudUrl(const String& url) {
    cloudUrl_ = url;
}

void StateReporterIs06s::setHeartbeatInterval(int seconds) {
    heartbeatIntervalSec_ = seconds;
}

void StateReporterIs06s::update() {
    unsigned long now = millis();

    // ハートビート送信チェック
    if (heartbeatIntervalSec_ > 0) {
        unsigned long interval = (unsigned long)heartbeatIntervalSec_ * 1000UL;
        if (lastHeartbeatTime_ == 0 || (now - lastHeartbeatTime_) >= interval) {
            lastHeartbeatTime_ = now;
            sendHeartbeat();
        }
    }
}

bool StateReporterIs06s::sendStateReport(bool force) {
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

bool StateReporterIs06s::sendHeartbeat() {
    return sendStateReport();
}

void StateReporterIs06s::resetStats() {
    sentCount_ = 0;
    failCount_ = 0;
    lastResult_ = "---";
}

String StateReporterIs06s::buildLocalPayload() {
    String observedAt = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";
    String ip = WiFi.localIP().toString();
    int rssi = WiFi.RSSI();
    String ssid = WiFi.SSID();

    StaticJsonDocument<2048> doc;
    doc["observedAt"] = observedAt;

    // sensor
    JsonObject sensor = doc.createNestedObject("sensor");
    sensor["lacisId"] = lacisId_;
    sensor["mac"] = mac_;
    sensor["productType"] = "006";
    sensor["productCode"] = "0200";
    sensor["deviceType"] = "aranea_ar-is06s";

    // state (6ch対応)
    JsonObject state = doc.createNestedObject("state");

    // PINState
    JsonObject pinState = state.createNestedObject("PINState");
    for (int ch = 1; ch <= 6; ch++) {
        if (pinMgr_) {
            JsonObject chObj = pinState.createNestedObject("CH" + String(ch));
            const PinSetting& setting = pinMgr_->getPinSetting(ch);

            // state value
            if (setting.type == PinType::PWM_OUTPUT) {
                chObj["state"] = String(pinMgr_->getPwmValue(ch));
            } else {
                chObj["state"] = pinMgr_->getPinState(ch) ? "on" : "off";
            }

            chObj["enabled"] = pinMgr_->isPinEnabled(ch);

            // type
            const char* typeStr = "digitalOutput";
            switch (setting.type) {
                case PinType::DIGITAL_OUTPUT: typeStr = "digitalOutput"; break;
                case PinType::PWM_OUTPUT: typeStr = "pwmOutput"; break;
                case PinType::DIGITAL_INPUT: typeStr = "digitalInput"; break;
                case PinType::PIN_DISABLED: typeStr = "disabled"; break;
            }
            chObj["type"] = typeStr;
        }
    }

    // globalState
    JsonObject globalState = state.createNestedObject("globalState");
    globalState["ipAddress"] = ip;
    globalState["rssi"] = rssi;
    globalState["ssid"] = ssid;
    globalState["uptime"] = millis() / 1000;
    globalState["heapFree"] = ESP.getFreeHeap();
    globalState["firmwareVersion"] = firmwareVersion_;

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
    json.reserve(1536);
    serializeJson(doc, json);
    return json;
}

String StateReporterIs06s::buildCloudPayload() {
    String observedAt = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";

    StaticJsonDocument<1536> doc;

    // auth
    JsonObject auth = doc.createNestedObject("auth");
    auth["tid"] = tid_;
    auth["lacisId"] = lacisId_;
    auth["cic"] = cic_;

    // report
    JsonObject report = doc.createNestedObject("report");
    report["lacisId"] = lacisId_;
    report["type"] = "aranea_ar-is06s";
    report["observedAt"] = observedAt;

    // state
    JsonObject state = report.createNestedObject("state");

    // PINState
    JsonObject pinState = state.createNestedObject("PINState");
    for (int ch = 1; ch <= 6; ch++) {
        if (pinMgr_) {
            JsonObject chObj = pinState.createNestedObject("CH" + String(ch));
            const PinSetting& setting = pinMgr_->getPinSetting(ch);

            if (setting.type == PinType::PWM_OUTPUT) {
                chObj["state"] = String(pinMgr_->getPwmValue(ch));
            } else {
                chObj["state"] = pinMgr_->getPinState(ch) ? "on" : "off";
            }
        }
    }

    // globalState
    JsonObject globalState = state.createNestedObject("globalState");
    globalState["ipAddress"] = WiFi.localIP().toString();
    globalState["uptime"] = millis() / 1000;
    globalState["rssi"] = WiFi.RSSI();
    globalState["heapFree"] = ESP.getFreeHeap();

    String json;
    json.reserve(1024);
    serializeJson(doc, json);
    return json;
}

bool StateReporterIs06s::postToUrl(const String& url, const String& payload) {
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
