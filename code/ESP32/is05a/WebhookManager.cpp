/**
 * WebhookManager.cpp
 *
 * Webhook通知モジュール（v2.0）
 *
 * 新アーキテクチャ:
 * - 最大5つの送信先エンドポイント
 * - 各エンドポイントにチャンネルマスクとカスタム通知文
 * - キュー管理（3件上限、超過分は切り捨て）
 * - 10秒インターバル制御
 */

#include "WebhookManager.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "ChannelManager.h"
#include "Is05aKeys.h"
#include <WiFi.h>
#include <HTTPClient.h>
#include <ArduinoJson.h>
#include <time.h>

static const unsigned long HTTP_TIMEOUT_MS = 3000;
static const int MAX_CONSECUTIVE_FAILURES = 3;
static const unsigned long BACKOFF_DURATION_MS = 30000;

// NVSキー配列（エンドポイントインデックスでアクセス）
static const char* const EP_URL_KEYS[] = {
    Is05aKeys::kEp0Url, Is05aKeys::kEp1Url, Is05aKeys::kEp2Url,
    Is05aKeys::kEp3Url, Is05aKeys::kEp4Url
};
static const char* const EP_CH_KEYS[] = {
    Is05aKeys::kEp0Ch, Is05aKeys::kEp1Ch, Is05aKeys::kEp2Ch,
    Is05aKeys::kEp3Ch, Is05aKeys::kEp4Ch
};
static const char* const EP_MSG_KEYS[] = {
    Is05aKeys::kEp0Msg, Is05aKeys::kEp1Msg, Is05aKeys::kEp2Msg,
    Is05aKeys::kEp3Msg, Is05aKeys::kEp4Msg
};
static const char* const EP_ON_KEYS[] = {
    Is05aKeys::kEp0On, Is05aKeys::kEp1On, Is05aKeys::kEp2On,
    Is05aKeys::kEp3On, Is05aKeys::kEp4On
};

WebhookManager::WebhookManager()
    : settings_(nullptr)
    , ntp_(nullptr)
    , channels_(nullptr)
    , enabled_(false)
    , queueSize_(DEFAULT_QUEUE_SIZE)
    , intervalSec_(DEFAULT_INTERVAL_SEC)
    , lastSendTime_(0)
    , quietEnabled_(false)
    , quietStartHour_(22)
    , quietEndHour_(6)
    , sentCount_(0)
    , failCount_(0)
    , quietSkipCount_(0)
    , queueDropCount_(0)
    , consecutiveFailures_(0)
    , lastFailTime_(0)
{
    // エンドポイント初期化
    for (int i = 0; i < MAX_ENDPOINTS; i++) {
        endpoints_[i] = Endpoint();
    }
}

void WebhookManager::begin(SettingManager* settings, NtpManager* ntp, ChannelManager* channels) {
    settings_ = settings;
    ntp_ = ntp;
    channels_ = channels;
    loadConfig();
}

void WebhookManager::loadConfig() {
    if (!settings_) return;

    // グローバル設定
    enabled_ = settings_->getBool(Is05aKeys::kWebhookOn, false);
    queueSize_ = settings_->getInt(Is05aKeys::kWhQueueSz, DEFAULT_QUEUE_SIZE);
    intervalSec_ = settings_->getInt(Is05aKeys::kWhInterval, DEFAULT_INTERVAL_SEC);

    // QuietMode設定
    quietEnabled_ = settings_->getBool(Is05aKeys::kQuietOn, false);
    quietStartHour_ = settings_->getInt(Is05aKeys::kQuietStart, 22);
    quietEndHour_ = settings_->getInt(Is05aKeys::kQuietEnd, 6);

    // エンドポイント読み込み
    for (int i = 0; i < MAX_ENDPOINTS; i++) {
        endpoints_[i].url = settings_->getString(EP_URL_KEYS[i], "");
        endpoints_[i].channelMask = (uint8_t)settings_->getInt(EP_CH_KEYS[i], 0xFF);
        endpoints_[i].message = settings_->getString(EP_MSG_KEYS[i], "");
        endpoints_[i].enabled = settings_->getBool(EP_ON_KEYS[i], false);
    }

    // レガシー互換: 旧設定があれば ep0/ep1/ep2 に移行
    migrateFromLegacy();

    Serial.printf("[WebhookManager] Initialized: enabled=%s, queue=%d, interval=%ds\n",
        enabled_ ? "true" : "false", queueSize_, intervalSec_);
    Serial.printf("[WebhookManager] QuietMode: %s, %02d:00-%02d:00\n",
        quietEnabled_ ? "on" : "off", quietStartHour_, quietEndHour_);

    int activeCount = 0;
    for (int i = 0; i < MAX_ENDPOINTS; i++) {
        if (endpoints_[i].enabled && endpoints_[i].url.length() > 0) {
            activeCount++;
            Serial.printf("[WebhookManager] EP%d: %s (mask=0x%02X)\n",
                i, endpoints_[i].url.substring(0, 40).c_str(), endpoints_[i].channelMask);
        }
    }
    Serial.printf("[WebhookManager] Active endpoints: %d\n", activeCount);
}

void WebhookManager::migrateFromLegacy() {
    // 旧Discord/Slack/Generic URLがあり、新ep0/ep1/ep2が空なら移行
    String discord = settings_->getString(Is05aKeys::kDiscordUrl, "");
    String slack = settings_->getString(Is05aKeys::kSlackUrl, "");
    String generic = settings_->getString(Is05aKeys::kGenericUrl, "");
    uint8_t legacyMask = (uint8_t)settings_->getInt(Is05aKeys::kWhChMask, 0xFF);

    bool migrated = false;

    if (discord.length() > 0 && endpoints_[0].url.length() == 0) {
        endpoints_[0].url = discord;
        endpoints_[0].channelMask = legacyMask;
        endpoints_[0].enabled = true;
        saveEndpoint(0);
        migrated = true;
    }
    if (slack.length() > 0 && endpoints_[1].url.length() == 0) {
        endpoints_[1].url = slack;
        endpoints_[1].channelMask = legacyMask;
        endpoints_[1].enabled = true;
        saveEndpoint(1);
        migrated = true;
    }
    if (generic.length() > 0 && endpoints_[2].url.length() == 0) {
        endpoints_[2].url = generic;
        endpoints_[2].channelMask = legacyMask;
        endpoints_[2].enabled = true;
        saveEndpoint(2);
        migrated = true;
    }

    if (migrated) {
        Serial.println("[WebhookManager] Migrated from legacy webhook settings");
    }
}

void WebhookManager::saveConfig() {
    if (!settings_) return;

    settings_->setBool(Is05aKeys::kWebhookOn, enabled_);
    settings_->setInt(Is05aKeys::kWhQueueSz, queueSize_);
    settings_->setInt(Is05aKeys::kWhInterval, intervalSec_);
    settings_->setBool(Is05aKeys::kQuietOn, quietEnabled_);
    settings_->setInt(Is05aKeys::kQuietStart, quietStartHour_);
    settings_->setInt(Is05aKeys::kQuietEnd, quietEndHour_);

    for (int i = 0; i < MAX_ENDPOINTS; i++) {
        saveEndpoint(i);
    }
}

void WebhookManager::saveEndpoint(int index) {
    if (!settings_ || index < 0 || index >= MAX_ENDPOINTS) return;

    settings_->setString(EP_URL_KEYS[index], endpoints_[index].url);
    settings_->setInt(EP_CH_KEYS[index], endpoints_[index].channelMask);
    settings_->setString(EP_MSG_KEYS[index], endpoints_[index].message);
    settings_->setBool(EP_ON_KEYS[index], endpoints_[index].enabled);
}

// ========================================
// メインループ処理（キュー処理）
// ========================================
void WebhookManager::handle() {
    if (!enabled_) return;
    if (queue_.empty()) return;

    unsigned long now = millis();
    unsigned long intervalMs = (unsigned long)intervalSec_ * 1000;

    // インターバルチェック
    if (lastSendTime_ > 0 && (now - lastSendTime_) < intervalMs) {
        return;  // まだ送信間隔が経過していない
    }

    // WiFi接続チェック
    if (!WiFi.isConnected()) {
        return;
    }

    // バックオフチェック
    if (!checkBackoff()) {
        return;
    }

    processQueue();
}

void WebhookManager::processQueue() {
    if (queue_.empty()) return;

    // キューから取り出し
    QueueItem item = queue_.front();
    queue_.erase(queue_.begin());

    // エンドポイントチェック
    if (item.endpointIndex < 0 || item.endpointIndex >= MAX_ENDPOINTS) {
        Serial.printf("[Webhook] Invalid endpoint index: %d\n", item.endpointIndex);
        return;
    }

    const Endpoint& ep = endpoints_[item.endpointIndex];
    if (!ep.enabled || ep.url.length() == 0) {
        Serial.printf("[Webhook] EP%d disabled or no URL\n", item.endpointIndex);
        return;
    }

    // ペイロード構築・送信
    String payload = buildPayload(item.endpointIndex, item.channel, item.active, item.timestamp);
    bool ok = sendToUrl(ep.url, payload);

    updateBackoff(ok);
    lastSendTime_ = millis();

    if (ok) {
        Serial.printf("[Webhook] Sent to EP%d: ch%d=%s\n",
            item.endpointIndex, item.channel, item.active ? "active" : "inactive");
    }
}

// ========================================
// 通知キューイング
// ========================================
bool WebhookManager::sendStateChange(int ch, bool active) {
    if (!enabled_) return false;
    if (!channels_) return false;

    // QuietModeチェック
    if (isInQuietPeriod()) {
        quietSkipCount_++;
        Serial.printf("[Webhook] Skipped ch%d (quiet period)\n", ch);
        return false;
    }

    String timestamp = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";

    int queued = 0;

    // 各エンドポイントについて、チャンネルマスクをチェックしてキューに追加
    for (int i = 0; i < MAX_ENDPOINTS; i++) {
        if (!endpoints_[i].enabled) continue;
        if (endpoints_[i].url.length() == 0) continue;

        // チャンネルマスクチェック
        uint8_t mask = endpoints_[i].channelMask;
        if (ch >= 1 && ch <= 8) {
            if ((mask & (1 << (ch - 1))) == 0) {
                continue;  // このエンドポイントは対象チャンネルではない
            }
        }

        // キューに追加
        if ((int)queue_.size() >= queueSize_) {
            queueDropCount_++;
            Serial.printf("[Webhook] Queue full, dropped EP%d notification\n", i);
            continue;
        }

        QueueItem item;
        item.endpointIndex = i;
        item.channel = ch;
        item.active = active;
        item.timestamp = timestamp;
        queue_.push_back(item);
        queued++;
    }

    if (queued > 0) {
        Serial.printf("[Webhook] Queued %d notifications for ch%d\n", queued, ch);
    }

    return queued > 0;
}

bool WebhookManager::sendStateChangeToEndpoints(uint8_t endpointMask, int ch, bool active) {
    // ルールから呼び出される - 指定されたエンドポイントのみにキューイング
    if (!enabled_) return false;
    if (!channels_) return false;

    // QuietModeチェック
    if (isInQuietPeriod()) {
        quietSkipCount_++;
        return false;
    }

    String timestamp = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";

    int queued = 0;

    for (int i = 0; i < MAX_ENDPOINTS; i++) {
        // エンドポイントマスクチェック
        if ((endpointMask & (1 << i)) == 0) continue;

        if (!endpoints_[i].enabled) continue;
        if (endpoints_[i].url.length() == 0) continue;

        // キューに追加
        if ((int)queue_.size() >= queueSize_) {
            queueDropCount_++;
            continue;
        }

        QueueItem item;
        item.endpointIndex = i;
        item.channel = ch;
        item.active = active;
        item.timestamp = timestamp;
        queue_.push_back(item);
        queued++;
    }

    return queued > 0;
}

bool WebhookManager::sendHeartbeat() {
    // ハートビートはGenericタイプのエンドポイントのみに送信
    if (!enabled_) return false;

    String timestamp = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";

    int sent = 0;
    for (int i = 0; i < MAX_ENDPOINTS; i++) {
        if (!endpoints_[i].enabled) continue;
        if (endpoints_[i].url.length() == 0) continue;
        if (detectUrlType(endpoints_[i].url) != UrlType::GENERIC) continue;

        // Heartbeatペイロード構築
        StaticJsonDocument<768> doc;
        doc["device_id"] = lacisId_;
        doc["device_name"] = deviceName_;
        doc["event"] = "heartbeat";
        doc["timestamp"] = timestamp;

        JsonObject channelsObj = doc.createNestedObject("channels");
        for (int ch = 1; ch <= 8; ch++) {
            auto cfg = channels_->getConfig(ch);
            JsonObject chObj = channelsObj.createNestedObject("ch" + String(ch));
            chObj["name"] = cfg.name;
            chObj["state"] = channels_->getStateString(ch);
        }

        String json;
        json.reserve(512);
        serializeJson(doc, json);

        if (sendToUrl(endpoints_[i].url, json)) {
            sent++;
        }
        yield();
    }

    return sent > 0;
}

bool WebhookManager::sendTestToEndpoint(int index) {
    if (index < 0 || index >= MAX_ENDPOINTS) return false;

    const Endpoint& ep = endpoints_[index];
    if (ep.url.length() == 0) {
        Serial.printf("[Webhook] EP%d has no URL\n", index);
        return false;
    }

    if (!WiFi.isConnected()) {
        Serial.println("[Webhook] Test skipped (WiFi not connected)");
        return false;
    }

    String timestamp = (ntp_ && ntp_->isSynced())
        ? ntp_->getIso8601()
        : "1970-01-01T00:00:00Z";

    // テスト送信（ch=0, active=false）
    String payload = buildPayload(index, 0, false, timestamp);
    bool ok = sendToUrl(ep.url, payload);

    Serial.printf("[Webhook] Test to EP%d: %s\n", index, ok ? "OK" : "FAILED");
    return ok;
}

// ========================================
// URL種別判定
// ========================================
WebhookManager::UrlType WebhookManager::detectUrlType(const String& url) const {
    if (url.indexOf("discord.com/api/webhooks") >= 0 ||
        url.indexOf("discordapp.com/api/webhooks") >= 0) {
        return UrlType::DISCORD;
    }
    if (url.indexOf("hooks.slack.com") >= 0) {
        return UrlType::SLACK;
    }
    return UrlType::GENERIC;
}

// ========================================
// ペイロード構築
// ========================================
String WebhookManager::buildPayload(int endpointIndex, int ch, bool active, const String& timestamp) {
    const Endpoint& ep = endpoints_[endpointIndex];
    UrlType type = detectUrlType(ep.url);

    switch (type) {
        case UrlType::DISCORD:
            return buildDiscordPayload(ch, active, timestamp, ep.message);
        case UrlType::SLACK:
            return buildSlackPayload(ch, active, timestamp, ep.message);
        default:
            return buildGenericPayload(ch, active, timestamp, ep.message);
    }
}

String WebhookManager::buildDiscordPayload(int ch, bool active, const String& timestamp, const String& customMsg) {
    String chName = "ch" + String(ch);
    String state = active ? "検知" : "非検知";

    if (ch >= 1 && ch <= 8 && channels_) {
        auto cfg = channels_->getConfig(ch);
        if (cfg.name.length() > 0) {
            chName = cfg.name;
        }
        state = channels_->getStateString(ch);
    }

    // カスタムメッセージがあれば使用、なければデフォルト
    String emoji = active ? "🚨" : "✅";
    String content;
    if (customMsg.length() > 0) {
        content = customMsg;
        content.replace("{ch}", String(ch));
        content.replace("{name}", chName);
        content.replace("{state}", state);
    } else {
        content = emoji + " **" + chName + "** が **" + state + "** になりました";
    }

    int color = active ? 15158332 : 3066993;

    StaticJsonDocument<512> doc;
    doc["content"] = content;

    JsonArray embeds = doc.createNestedArray("embeds");
    JsonObject embed = embeds.createNestedObject();
    embed["title"] = "is05a 状態変化";
    embed["color"] = color;

    JsonArray fields = embed.createNestedArray("fields");

    JsonObject f1 = fields.createNestedObject();
    f1["name"] = "チャンネル";
    f1["value"] = chName + " (ch" + String(ch) + ")";
    f1["inline"] = true;

    JsonObject f2 = fields.createNestedObject();
    f2["name"] = "状態";
    f2["value"] = state;
    f2["inline"] = true;

    JsonObject f3 = fields.createNestedObject();
    f3["name"] = "時刻";
    f3["value"] = timestamp;
    f3["inline"] = false;

    String json;
    json.reserve(384);
    serializeJson(doc, json);
    return json;
}

String WebhookManager::buildSlackPayload(int ch, bool active, const String& timestamp, const String& customMsg) {
    String chName = "ch" + String(ch);
    String state = active ? "検知" : "非検知";

    if (ch >= 1 && ch <= 8 && channels_) {
        auto cfg = channels_->getConfig(ch);
        if (cfg.name.length() > 0) {
            chName = cfg.name;
        }
        state = channels_->getStateString(ch);
    }

    String emoji = active ? "🚨" : "✅";
    String text;
    if (customMsg.length() > 0) {
        text = customMsg;
        text.replace("{ch}", String(ch));
        text.replace("{name}", chName);
        text.replace("{state}", state);
    } else {
        text = emoji + " *" + chName + "* が *" + state + "* になりました";
    }

    String color = active ? "danger" : "good";

    StaticJsonDocument<512> doc;
    doc["text"] = text;

    JsonArray attachments = doc.createNestedArray("attachments");
    JsonObject attachment = attachments.createNestedObject();
    attachment["color"] = color;

    JsonArray fields = attachment.createNestedArray("fields");

    JsonObject f1 = fields.createNestedObject();
    f1["title"] = "チャンネル";
    f1["value"] = chName + " (ch" + String(ch) + ")";
    f1["short"] = true;

    JsonObject f2 = fields.createNestedObject();
    f2["title"] = "状態";
    f2["value"] = state;
    f2["short"] = true;

    JsonObject f3 = fields.createNestedObject();
    f3["title"] = "時刻";
    f3["value"] = timestamp;
    f3["short"] = false;

    String json;
    json.reserve(384);
    serializeJson(doc, json);
    return json;
}

String WebhookManager::buildGenericPayload(int ch, bool active, const String& timestamp, const String& customMsg) {
    String chName = "ch" + String(ch);
    String state = active ? "active" : "inactive";

    if (ch >= 1 && ch <= 8 && channels_) {
        auto cfg = channels_->getConfig(ch);
        if (cfg.name.length() > 0) {
            chName = cfg.name;
        }
        state = channels_->getStateString(ch);
    }

    StaticJsonDocument<384> doc;
    doc["device_id"] = lacisId_;
    doc["device_name"] = deviceName_;
    doc["event"] = "state_change";
    doc["channel"] = ch;
    doc["channel_name"] = chName;
    doc["state"] = state;
    doc["active"] = active;
    doc["timestamp"] = timestamp;

    if (customMsg.length() > 0) {
        doc["custom_message"] = customMsg;
    }

    String json;
    json.reserve(256);
    serializeJson(doc, json);
    return json;
}

// ========================================
// HTTP送信
// ========================================
bool WebhookManager::sendToUrl(const String& url, const String& payload) {
    if (url.length() == 0) return false;

    HTTPClient http;
    http.begin(url);
    http.addHeader("Content-Type", "application/json");
    http.setTimeout(HTTP_TIMEOUT_MS);

    int httpCode = http.POST(payload);
    yield();

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

// ========================================
// グローバル設定
// ========================================
void WebhookManager::setEnabled(bool enabled) {
    enabled_ = enabled;
    if (settings_) settings_->setBool(Is05aKeys::kWebhookOn, enabled);
}

void WebhookManager::setQueueSize(int size) {
    queueSize_ = constrain(size, 1, 10);
    if (settings_) settings_->setInt(Is05aKeys::kWhQueueSz, queueSize_);
}

void WebhookManager::setIntervalSec(int sec) {
    intervalSec_ = constrain(sec, 1, 60);
    if (settings_) settings_->setInt(Is05aKeys::kWhInterval, intervalSec_);
}

void WebhookManager::setDeviceInfo(const String& lacisId, const String& deviceName) {
    lacisId_ = lacisId;
    deviceName_ = deviceName;
}

// ========================================
// エンドポイント設定
// ========================================
void WebhookManager::setEndpoint(int index, const Endpoint& ep) {
    if (index < 0 || index >= MAX_ENDPOINTS) return;
    endpoints_[index] = ep;
    saveEndpoint(index);
}

WebhookManager::Endpoint WebhookManager::getEndpoint(int index) const {
    if (index < 0 || index >= MAX_ENDPOINTS) return Endpoint();
    return endpoints_[index];
}

int WebhookManager::getEndpointCount() const {
    int count = 0;
    for (int i = 0; i < MAX_ENDPOINTS; i++) {
        if (endpoints_[i].enabled && endpoints_[i].url.length() > 0) {
            count++;
        }
    }
    return count;
}

// レガシー互換
void WebhookManager::setDiscordUrl(const String& url) {
    endpoints_[0].url = url;
    endpoints_[0].enabled = url.length() > 0;
    saveEndpoint(0);
}

void WebhookManager::setSlackUrl(const String& url) {
    endpoints_[1].url = url;
    endpoints_[1].enabled = url.length() > 0;
    saveEndpoint(1);
}

void WebhookManager::setGenericUrl(const String& url) {
    endpoints_[2].url = url;
    endpoints_[2].enabled = url.length() > 0;
    saveEndpoint(2);
}

String WebhookManager::getDiscordUrl() const {
    return endpoints_[0].url;
}

String WebhookManager::getSlackUrl() const {
    return endpoints_[1].url;
}

String WebhookManager::getGenericUrl() const {
    return endpoints_[2].url;
}

void WebhookManager::setChannelMask(uint8_t mask) {
    // 全エンドポイントに適用（レガシー互換）
    for (int i = 0; i < MAX_ENDPOINTS; i++) {
        endpoints_[i].channelMask = mask;
    }
    saveConfig();
}

uint8_t WebhookManager::getChannelMask() const {
    return endpoints_[0].channelMask;
}

// ========================================
// QuietMode設定
// ========================================
void WebhookManager::setQuietModeEnabled(bool enabled) {
    quietEnabled_ = enabled;
    if (settings_) settings_->setBool(Is05aKeys::kQuietOn, enabled);
}

void WebhookManager::setQuietModeHours(int startHour, int endHour) {
    quietStartHour_ = constrain(startHour, 0, 23);
    quietEndHour_ = constrain(endHour, 0, 23);
    if (settings_) {
        settings_->setInt(Is05aKeys::kQuietStart, quietStartHour_);
        settings_->setInt(Is05aKeys::kQuietEnd, quietEndHour_);
    }
}

bool WebhookManager::isInQuietPeriod() const {
    if (!quietEnabled_) return false;
    if (quietStartHour_ == quietEndHour_) return false;
    if (!ntp_ || !ntp_->isSynced()) return false;

    time_t epoch = ntp_->getEpoch();
    if (epoch == 0) return false;

    struct tm timeinfo;
    localtime_r(&epoch, &timeinfo);
    int currentHour = timeinfo.tm_hour;

    if (quietStartHour_ < quietEndHour_) {
        return (currentHour >= quietStartHour_ && currentHour < quietEndHour_);
    } else {
        return (currentHour >= quietStartHour_ || currentHour < quietEndHour_);
    }
}

// ========================================
// 統計
// ========================================
void WebhookManager::resetStats() {
    sentCount_ = 0;
    failCount_ = 0;
    quietSkipCount_ = 0;
    queueDropCount_ = 0;
}

// ========================================
// バックオフ
// ========================================
bool WebhookManager::checkBackoff() {
    if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
        unsigned long now = millis();
        if ((now - lastFailTime_) < BACKOFF_DURATION_MS) {
            return false;
        }
        Serial.println("[Webhook] Backoff period ended, retrying...");
        consecutiveFailures_ = 0;
    }
    return true;
}

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
