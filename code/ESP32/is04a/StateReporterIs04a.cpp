/**
 * StateReporterIs04a.cpp
 */

#include "StateReporterIs04a.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "TriggerManager.h"
#include <WiFi.h>
#include <HTTPClient.h>
#include <ArduinoJson.h>

static const unsigned long MIN_SEND_INTERVAL_MS = 1000;
static const unsigned long HTTP_TIMEOUT_MS = 3000;      // P0: 10s→3sに短縮
static const int MAX_CONSECUTIVE_FAILURES = 3;          // P0: バックオフ閾値
static const unsigned long BACKOFF_DURATION_MS = 30000; // P0: 30秒バックオフ

StateReporterIs04a::StateReporterIs04a()
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

void StateReporterIs04a::begin(SettingManager* settings, NtpManager* ntp, TriggerManager* trigger) {
    settings_ = settings;
    ntp_ = ntp;
    trigger_ = trigger;
    Serial.println("[StateReporterIs04a] Initialized");
}

void StateReporterIs04a::setAuth(const String& tid, const String& lacisId, const String& cic) {
    tid_ = tid;
    lacisId_ = lacisId;
    cic_ = cic;
}

void StateReporterIs04a::setMac(const String& mac) {
    mac_ = mac;
}

void StateReporterIs04a::setRelayUrls(const String& primary, const String& secondary) {
    relayPrimaryUrl_ = primary;
    relaySecondaryUrl_ = secondary;
}

void StateReporterIs04a::setCloudUrl(const String& url) {
    cloudUrl_ = url;
}

bool StateReporterIs04a::sendStateReport(bool force) {
    unsigned long now = millis();

    // P0: WiFi接続チェック（未接続時は即座にスキップ）
    if (!WiFi.isConnected()) {
        Serial.println("[StateReporter] Skipped (WiFi not connected)");
        return false;
    }

    // P0: バックオフチェック（連続失敗後は30秒待機）
    if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
        if ((now - lastFailTime_) < BACKOFF_DURATION_MS) {
            Serial.println("[StateReporter] Skipped (backoff)");
            return false;
        }
        // バックオフ期間終了、リトライ許可
        Serial.println("[StateReporter] Backoff period ended, retrying...");
        consecutiveFailures_ = 0;
    }

    // 最小送信間隔チェック（force=trueならスキップ）
    if (!force && lastSendTime_ > 0 && (now - lastSendTime_) < MIN_SEND_INTERVAL_MS) {
        Serial.println("[StateReporter] Skipped (interval limit)");
        return false;
    }
    lastSendTime_ = now;

    Serial.println("[StateReporter] Sending state report...");

    int successCount = 0;

    // ローカル送信
    String localPayload = buildLocalPayload();

    if (relayPrimaryUrl_.length() > 0) {
        if (postToUrl(relayPrimaryUrl_, localPayload)) successCount++;
        yield();  // P0: WDT対策
    }

    if (relaySecondaryUrl_.length() > 0) {
        if (postToUrl(relaySecondaryUrl_, localPayload)) successCount++;
        yield();  // P0: WDT対策
    }

    // クラウド送信
    if (cloudUrl_.length() > 0 && tid_.length() > 0 && cic_.length() > 0) {
        String cloudPayload = buildCloudPayload();
        if (postToUrl(cloudUrl_, cloudPayload)) successCount++;
        yield();  // P0: WDT対策
    }

    // 結果更新とバックオフ管理
    if (successCount > 0) {
        sentCount_++;
        lastResult_ = "OK(" + String(successCount) + ")";
        consecutiveFailures_ = 0;  // 成功時はリセット
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

bool StateReporterIs04a::sendHeartbeat() {
    return sendStateReport();
}

void StateReporterIs04a::resetStats() {
    sentCount_ = 0;
    failCount_ = 0;
    lastResult_ = "---";
}

String StateReporterIs04a::buildLocalPayload() {
    String observedAt = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";
    String ip = WiFi.localIP().toString();
    int rssi = WiFi.RSSI();
    String ssid = WiFi.SSID();

    // P1: StaticJsonDocumentで動的アロケーション回避
    StaticJsonDocument<768> doc;
    doc["observedAt"] = observedAt;

    // sensor
    JsonObject sensor = doc.createNestedObject("sensor");
    sensor["lacisId"] = lacisId_;
    sensor["mac"] = mac_;
    sensor["productType"] = "004";
    sensor["productCode"] = "0001";

    // state
    JsonObject state = doc.createNestedObject("state");

    // 出力状態
    auto out1 = trigger_->getOutputState(1);
    auto out2 = trigger_->getOutputState(2);
    state["Trigger1"] = out1.active;
    state["Trigger2"] = out2.active;
    state["Trigger1_Name"] = out1.name;
    state["Trigger2_Name"] = out2.name;

    // 入力状態
    auto in1 = trigger_->getInputState(1);
    auto in2 = trigger_->getInputState(2);
    state["Input1_Physical"] = in1.active;
    state["Input2_Physical"] = in2.active;
    state["Input1_lastUpdatedAt"] = in1.lastUpdatedAt;
    state["Input2_lastUpdatedAt"] = in2.lastUpdatedAt;

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

    // P1: String::reserve()でフラグメンテーション軽減
    String json;
    json.reserve(512);
    serializeJson(doc, json);
    return json;
}

String StateReporterIs04a::buildCloudPayload() {
    String observedAt = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";

    // P1: StaticJsonDocumentで動的アロケーション回避
    StaticJsonDocument<512> doc;

    // auth
    JsonObject auth = doc.createNestedObject("auth");
    auth["tid"] = tid_;
    auth["lacisId"] = lacisId_;
    auth["cic"] = cic_;

    // report
    JsonObject report = doc.createNestedObject("report");
    report["lacisId"] = lacisId_;
    report["type"] = "aranea_ar-is04a";
    report["observedAt"] = observedAt;

    // state
    JsonObject state = report.createNestedObject("state");

    auto out1 = trigger_->getOutputState(1);
    auto out2 = trigger_->getOutputState(2);
    state["Trigger1"] = out1.active;
    state["Trigger2"] = out2.active;

    auto in1 = trigger_->getInputState(1);
    auto in2 = trigger_->getInputState(2);
    state["Input1_Physical"] = in1.active;
    state["Input2_Physical"] = in2.active;

    // P1: String::reserve()でフラグメンテーション軽減
    String json;
    json.reserve(256);
    serializeJson(doc, json);
    return json;
}

bool StateReporterIs04a::postToUrl(const String& url, const String& payload) {
    if (url.length() == 0) return false;

    HTTPClient http;
    http.begin(url);
    http.addHeader("Content-Type", "application/json");
    http.setTimeout(HTTP_TIMEOUT_MS);  // P0: 10s→3sに短縮

    int httpCode = http.POST(payload);
    yield();  // P0: WDT対策

    bool success = (httpCode >= 200 && httpCode < 300);

    if (success) {
        Serial.printf("[StateReporter] OK %d -> %s\n", httpCode, url.substring(0, 50).c_str());
    } else {
        Serial.printf("[StateReporter] NG %d -> %s\n", httpCode, url.substring(0, 50).c_str());
    }

    http.end();
    return success;
}
