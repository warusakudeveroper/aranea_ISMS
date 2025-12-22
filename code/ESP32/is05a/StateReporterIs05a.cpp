/**
 * StateReporterIs05a.cpp
 */

#include "StateReporterIs05a.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "ChannelManager.h"
#include <WiFi.h>
#include <HTTPClient.h>
#include <ArduinoJson.h>

// 最小送信間隔（連続送信防止）
static const unsigned long MIN_SEND_INTERVAL_MS = 1000;

StateReporterIs05a::StateReporterIs05a()
    : settings_(nullptr)
    , ntp_(nullptr)
    , channels_(nullptr)
    , sentCount_(0)
    , failCount_(0)
    , lastResult_("---")
    , lastSendTime_(0)
{
}

void StateReporterIs05a::begin(SettingManager* settings, NtpManager* ntp, ChannelManager* channels) {
    settings_ = settings;
    ntp_ = ntp;
    channels_ = channels;

    Serial.println("[StateReporterIs05a] Initialized");
}

void StateReporterIs05a::setAuth(const String& tid, const String& lacisId, const String& cic) {
    tid_ = tid;
    lacisId_ = lacisId;
    cic_ = cic;
}

void StateReporterIs05a::setMac(const String& mac) {
    mac_ = mac;
}

void StateReporterIs05a::setRelayUrls(const String& primary, const String& secondary) {
    relayPrimaryUrl_ = primary;
    relaySecondaryUrl_ = secondary;
}

void StateReporterIs05a::setCloudUrl(const String& url) {
    cloudUrl_ = url;
}

bool StateReporterIs05a::sendStateReport() {
    // 最小送信間隔チェック
    unsigned long now = millis();
    if (lastSendTime_ > 0 && (now - lastSendTime_) < MIN_SEND_INTERVAL_MS) {
        return false;
    }
    lastSendTime_ = now;

    Serial.println("[StateReporter] Sending state report...");

    int successCount = 0;

    // ローカル送信
    String localPayload = buildLocalPayload();

    if (relayPrimaryUrl_.length() > 0) {
        if (postToUrl(relayPrimaryUrl_, localPayload)) successCount++;
    }

    if (relaySecondaryUrl_.length() > 0) {
        if (postToUrl(relaySecondaryUrl_, localPayload)) successCount++;
    }

    // クラウド送信
    if (cloudUrl_.length() > 0 && tid_.length() > 0 && cic_.length() > 0) {
        String cloudPayload = buildCloudPayload();
        if (postToUrl(cloudUrl_, cloudPayload)) successCount++;
    }

    // 結果更新
    if (successCount > 0) {
        sentCount_++;
        lastResult_ = "OK(" + String(successCount) + ")";
    } else {
        failCount_++;
        lastResult_ = "NG";
    }

    Serial.printf("[StateReporter] Done: %d success\n", successCount);
    return successCount > 0;
}

bool StateReporterIs05a::sendHeartbeat() {
    return sendStateReport();
}

void StateReporterIs05a::resetStats() {
    sentCount_ = 0;
    failCount_ = 0;
    lastResult_ = "---";
}

String StateReporterIs05a::buildLocalPayload() {
    String observedAt = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";
    String ip = WiFi.localIP().toString();
    int rssi = WiFi.RSSI();
    String ssid = WiFi.SSID();

    JsonDocument doc;
    doc["observedAt"] = observedAt;

    // sensor
    JsonObject sensor = doc.createNestedObject("sensor");
    sensor["lacisId"] = lacisId_;
    sensor["mac"] = mac_;
    sensor["productType"] = "005";
    sensor["productCode"] = "0001";

    // state
    JsonObject state = doc.createNestedObject("state");
    for (int ch = 1; ch <= 8; ch++) {
        String chKey = "ch" + String(ch);
        state[chKey] = channels_->getStateString(ch);
        state[chKey + "_lastUpdatedAt"] = channels_->getLastUpdatedAt(ch);
    }
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
    serializeJson(doc, json);
    return json;
}

String StateReporterIs05a::buildCloudPayload() {
    String observedAt = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";

    JsonDocument doc;

    // auth
    JsonObject auth = doc.createNestedObject("auth");
    auth["tid"] = tid_;
    auth["lacisId"] = lacisId_;
    auth["cic"] = cic_;

    // report
    JsonObject report = doc.createNestedObject("report");
    report["lacisId"] = lacisId_;
    report["type"] = "aranea_ar-is05a";
    report["observedAt"] = observedAt;

    // state
    JsonObject state = report.createNestedObject("state");
    for (int ch = 1; ch <= 8; ch++) {
        String chKey = "ch" + String(ch);
        state[chKey] = channels_->getStateString(ch);
    }

    String json;
    serializeJson(doc, json);
    return json;
}

bool StateReporterIs05a::postToUrl(const String& url, const String& payload) {
    if (url.length() == 0) return false;

    HTTPClient http;
    http.begin(url);
    http.addHeader("Content-Type", "application/json");
    http.setTimeout(10000);

    int httpCode = http.POST(payload);
    bool success = (httpCode >= 200 && httpCode < 300);

    if (success) {
        Serial.printf("[StateReporter] OK %d -> %s\n", httpCode, url.substring(0, 50).c_str());
    } else {
        Serial.printf("[StateReporter] NG %d -> %s\n", httpCode, url.substring(0, 50).c_str());
    }

    http.end();
    return success;
}
