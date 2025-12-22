/**
 * WebhookManager.cpp
 */

#include "WebhookManager.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "ChannelManager.h"
#include "Is05aKeys.h"
#include <WiFi.h>
#include <HTTPClient.h>
#include <ArduinoJson.h>

static const unsigned long HTTP_TIMEOUT_MS = 3000;      // P0: 10s→3sに短縮
static const int MAX_CONSECUTIVE_FAILURES = 3;          // P0: バックオフ閾値
static const unsigned long BACKOFF_DURATION_MS = 30000; // P0: 30秒バックオフ

WebhookManager::WebhookManager()
    : settings_(nullptr)
    , ntp_(nullptr)
    , channels_(nullptr)
    , enabled_(false)
    , sentCount_(0)
    , failCount_(0)
    , consecutiveFailures_(0)
    , lastFailTime_(0)
    , onSendCallback_(nullptr)
{
}

void WebhookManager::begin(SettingManager* settings, NtpManager* ntp, ChannelManager* channels) {
    settings_ = settings;
    ntp_ = ntp;
    channels_ = channels;

    // 設定読み込み
    enabled_ = settings_->getBool(Is05aKeys::kWebhookOn, false);
    discordUrl_ = settings_->getString(Is05aKeys::kDiscordUrl, "");
    slackUrl_ = settings_->getString(Is05aKeys::kSlackUrl, "");
    genericUrl_ = settings_->getString(Is05aKeys::kGenericUrl, "");

    Serial.printf("[WebhookManager] Initialized: enabled=%s, discord=%s, slack=%s, generic=%s\n",
        enabled_ ? "true" : "false",
        discordUrl_.length() > 0 ? "set" : "none",
        slackUrl_.length() > 0 ? "set" : "none",
        genericUrl_.length() > 0 ? "set" : "none");
}

void WebhookManager::setEnabled(bool enabled) {
    enabled_ = enabled;
    settings_->setBool(Is05aKeys::kWebhookOn, enabled);
}

void WebhookManager::setDiscordUrl(const String& url) {
    discordUrl_ = url;
    settings_->setString(Is05aKeys::kDiscordUrl, url);
}

void WebhookManager::setSlackUrl(const String& url) {
    slackUrl_ = url;
    settings_->setString(Is05aKeys::kSlackUrl, url);
}

void WebhookManager::setGenericUrl(const String& url) {
    genericUrl_ = url;
    settings_->setString(Is05aKeys::kGenericUrl, url);
}

void WebhookManager::setDeviceInfo(const String& lacisId, const String& deviceName) {
    lacisId_ = lacisId;
    deviceName_ = deviceName;
}

bool WebhookManager::sendStateChange(int ch, bool active) {
    if (!enabled_) return false;
    if (!channels_) return false;

    // P0: WiFi接続チェック
    if (!WiFi.isConnected()) {
        Serial.println("[Webhook] Skipped (WiFi not connected)");
        return false;
    }

    // P0: バックオフチェック
    if (!checkBackoff()) {
        Serial.println("[Webhook] Skipped (backoff)");
        return false;
    }

    String timestamp = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";

    int successCount = 0;

    // Discord
    if (discordUrl_.length() > 0) {
        String payload = buildDiscordPayload(ch, active, timestamp);
        bool ok = sendToUrl(discordUrl_, payload);
        if (ok) successCount++;
        if (onSendCallback_) onSendCallback_(Platform::DISCORD, ok);
        yield();  // P0: WDT対策
    }

    // Slack
    if (slackUrl_.length() > 0) {
        String payload = buildSlackPayload(ch, active, timestamp);
        bool ok = sendToUrl(slackUrl_, payload);
        if (ok) successCount++;
        if (onSendCallback_) onSendCallback_(Platform::SLACK, ok);
        yield();  // P0: WDT対策
    }

    // Generic
    if (genericUrl_.length() > 0) {
        String payload = buildGenericPayload(ch, active, timestamp);
        bool ok = sendToUrl(genericUrl_, payload);
        if (ok) successCount++;
        if (onSendCallback_) onSendCallback_(Platform::GENERIC, ok);
        yield();  // P0: WDT対策
    }

    // バックオフ管理
    updateBackoff(successCount > 0);

    return successCount > 0;
}

bool WebhookManager::sendHeartbeat() {
    // 心拍はGeneric Webhookのみに送信
    if (!enabled_) return false;
    if (genericUrl_.length() == 0) return false;

    // P0: WiFi接続チェック
    if (!WiFi.isConnected()) {
        Serial.println("[Webhook] Heartbeat skipped (WiFi not connected)");
        return false;
    }

    // P0: バックオフチェック
    if (!checkBackoff()) {
        Serial.println("[Webhook] Heartbeat skipped (backoff)");
        return false;
    }

    String timestamp = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";

    // P1: StaticJsonDocumentで動的アロケーション回避
    StaticJsonDocument<768> doc;
    doc["device_id"] = lacisId_;
    doc["device_name"] = deviceName_;
    doc["event"] = "heartbeat";
    doc["timestamp"] = timestamp;

    // channels情報を追加
    JsonObject channelsObj = doc.createNestedObject("channels");
    for (int ch = 1; ch <= 8; ch++) {
        auto cfg = channels_->getConfig(ch);
        JsonObject chObj = channelsObj.createNestedObject("ch" + String(ch));
        chObj["name"] = cfg.name;
        chObj["state"] = channels_->getStateString(ch);
    }

    // P1: String::reserve()でフラグメンテーション軽減
    String json;
    json.reserve(512);
    serializeJson(doc, json);

    bool ok = sendToUrl(genericUrl_, json);
    updateBackoff(ok);
    return ok;
}

String WebhookManager::buildDiscordPayload(int ch, bool active, const String& timestamp) {
    ChannelManager::ChannelConfig cfg = channels_->getConfig(ch);
    String state = channels_->getStateString(ch);

    // 絵文字
    String emoji = active ? "🚨" : "✅";
    int color = active ? 15158332 : 3066993;  // 赤 or 緑

    // P1: StaticJsonDocumentで動的アロケーション回避
    StaticJsonDocument<512> doc;
    doc["content"] = emoji + " **" + cfg.name + "** が **" + state + "** になりました";

    JsonArray embeds = doc.createNestedArray("embeds");
    JsonObject embed = embeds.createNestedObject();
    embed["title"] = "is05a 状態変化";
    embed["color"] = color;

    JsonArray fields = embed.createNestedArray("fields");

    JsonObject f1 = fields.createNestedObject();
    f1["name"] = "チャンネル";
    f1["value"] = cfg.name + " (ch" + String(ch) + ")";
    f1["inline"] = true;

    JsonObject f2 = fields.createNestedObject();
    f2["name"] = "状態";
    f2["value"] = state;
    f2["inline"] = true;

    JsonObject f3 = fields.createNestedObject();
    f3["name"] = "時刻";
    f3["value"] = timestamp;
    f3["inline"] = false;

    // P1: String::reserve()でフラグメンテーション軽減
    String json;
    json.reserve(384);
    serializeJson(doc, json);
    return json;
}

String WebhookManager::buildSlackPayload(int ch, bool active, const String& timestamp) {
    ChannelManager::ChannelConfig cfg = channels_->getConfig(ch);
    String state = channels_->getStateString(ch);

    String emoji = active ? "🚨" : "✅";
    String color = active ? "danger" : "good";

    // P1: StaticJsonDocumentで動的アロケーション回避
    StaticJsonDocument<512> doc;
    doc["text"] = emoji + " *" + cfg.name + "* が *" + state + "* になりました";

    JsonArray attachments = doc.createNestedArray("attachments");
    JsonObject attachment = attachments.createNestedObject();
    attachment["color"] = color;

    JsonArray fields = attachment.createNestedArray("fields");

    JsonObject f1 = fields.createNestedObject();
    f1["title"] = "チャンネル";
    f1["value"] = cfg.name + " (ch" + String(ch) + ")";
    f1["short"] = true;

    JsonObject f2 = fields.createNestedObject();
    f2["title"] = "状態";
    f2["value"] = state;
    f2["short"] = true;

    JsonObject f3 = fields.createNestedObject();
    f3["title"] = "時刻";
    f3["value"] = timestamp;
    f3["short"] = false;

    // P1: String::reserve()でフラグメンテーション軽減
    String json;
    json.reserve(384);
    serializeJson(doc, json);
    return json;
}

String WebhookManager::buildGenericPayload(int ch, bool active, const String& timestamp) {
    ChannelManager::ChannelConfig cfg = channels_->getConfig(ch);
    String state = channels_->getStateString(ch);

    // P1: StaticJsonDocumentで動的アロケーション回避
    StaticJsonDocument<256> doc;
    doc["device_id"] = lacisId_;
    doc["device_name"] = deviceName_;
    doc["event"] = "state_change";
    doc["channel"] = ch;
    doc["channel_name"] = cfg.name;
    doc["state"] = state;
    doc["active"] = active;
    doc["timestamp"] = timestamp;

    // P1: String::reserve()でフラグメンテーション軽減
    String json;
    json.reserve(192);
    serializeJson(doc, json);
    return json;
}

bool WebhookManager::sendToUrl(const String& url, const String& payload) {
    if (url.length() == 0) return false;

    HTTPClient http;
    http.begin(url);
    http.addHeader("Content-Type", "application/json");
    http.setTimeout(HTTP_TIMEOUT_MS);  // P0: 10s→3sに短縮

    int httpCode = http.POST(payload);
    yield();  // P0: WDT対策

    bool success = (httpCode >= 200 && httpCode < 300);

    if (success) {
        sentCount_++;
        Serial.printf("[Webhook] OK %d -> %s\n", httpCode, url.substring(0, 50).c_str());
    } else {
        failCount_++;
        Serial.printf("[Webhook] NG %d -> %s\n", httpCode, url.substring(0, 50).c_str());
    }

    http.end();
    return success;
}

void WebhookManager::onSendComplete(std::function<void(Platform, bool success)> callback) {
    onSendCallback_ = callback;
}

// P0: バックオフチェック
bool WebhookManager::checkBackoff() {
    if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
        unsigned long now = millis();
        if ((now - lastFailTime_) < BACKOFF_DURATION_MS) {
            return false;  // バックオフ中
        }
        // バックオフ期間終了、リトライ許可
        Serial.println("[Webhook] Backoff period ended, retrying...");
        consecutiveFailures_ = 0;
    }
    return true;
}

// P0: バックオフ状態更新
void WebhookManager::updateBackoff(bool success) {
    if (success) {
        consecutiveFailures_ = 0;
    } else {
        consecutiveFailures_++;
        lastFailTime_ = millis();
        if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
            Serial.printf("[Webhook] Entering backoff (%d consecutive failures)\n", consecutiveFailures_);
        }
    }
}
