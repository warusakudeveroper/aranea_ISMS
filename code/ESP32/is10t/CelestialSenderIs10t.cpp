/**
 * CelestialSenderIs10t.cpp
 *
 * CelestialGlobe SSOT送信モジュール実装（is10t用）
 */

#include "CelestialSenderIs10t.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "Is10tKeys.h"
#include <WiFi.h>
#include <HTTPClient.h>
#include <ArduinoJson.h>

static const unsigned long HTTP_TIMEOUT_MS = 5000;        // 5秒タイムアウト
static const int MAX_CONSECUTIVE_FAILURES = 3;            // バックオフ閾値
static const unsigned long BACKOFF_DURATION_MS = 60000;   // 60秒バックオフ

CelestialSenderIs10t::CelestialSenderIs10t() {}

void CelestialSenderIs10t::begin(SettingManager* settings, NtpManager* ntp,
                                  TapoPollerIs10t* poller) {
    settings_ = settings;
    ntp_ = ntp;
    poller_ = poller;
    Serial.println("[CELESTIAL-T] Initialized");
}

void CelestialSenderIs10t::setAuth(const String& tid, const String& lacisId,
                                    const String& cic, const String& fid) {
    tid_ = tid;
    lacisId_ = lacisId;
    cic_ = cic;
    fid_ = fid;
}

bool CelestialSenderIs10t::isConfigured() {
    if (!settings_) return false;
    String endpoint = settings_->getString(Is10tKeys::kEndpoint, "");
    String secret = settings_->getString(Is10tKeys::kSecret, "");
    return endpoint.length() > 0 && secret.length() > 0;
}

bool CelestialSenderIs10t::send() {
    if (!settings_ || !poller_) {
        Serial.println("[CELESTIAL-T] Not initialized");
        return false;
    }

    // WiFi接続チェック
    if (!WiFi.isConnected()) {
        Serial.println("[CELESTIAL-T] Skipped (WiFi not connected)");
        return false;
    }

    // バックオフチェック
    unsigned long now = millis();
    if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
        if ((now - lastFailTime_) < BACKOFF_DURATION_MS) {
            Serial.println("[CELESTIAL-T] Skipped (backoff)");
            return false;
        }
        Serial.println("[CELESTIAL-T] Backoff period ended, retrying...");
        consecutiveFailures_ = 0;
    }

    // エンドポイント取得
    String baseEndpoint = settings_->getString(Is10tKeys::kEndpoint, "");
    String secret = settings_->getString(Is10tKeys::kSecret, "");

    if (baseEndpoint.length() == 0 || secret.length() == 0) {
        Serial.println("[CELESTIAL-T] SKIPPED - Missing configuration");
        return false;
    }

    // URL構築
    String url = baseEndpoint + "?fid=" + fid_ + "&source=araneaDevice";

    // observedAt（ISO8601形式）
    String observedAt = "";
    if (ntp_ && ntp_->isSynced()) {
        observedAt = ntp_->getIso8601();
    } else {
        observedAt = "1970-01-01T00:00:00Z";  // フォールバック
    }

    // JSON構築（4KBで十分：8カメラ x 約400バイト = 3.2KB）
    StaticJsonDocument<4096> doc;

    // auth
    JsonObject auth = doc.createNestedObject("auth");
    auth["tid"] = tid_;
    auth["lacisId"] = lacisId_;
    auth["cic"] = cic_;

    // payload
    JsonObject payload = doc.createNestedObject("payload");
    payload["observedAt"] = observedAt;
    payload["source"] = source_;

    // cameras配列
    JsonArray cameras = payload.createNestedArray("cameras");

    uint8_t onlineCount = 0;
    uint8_t offlineCount = 0;

    for (uint8_t i = 0; i < poller_->getCameraCount(); i++) {
        const TapoConfig& config = poller_->getConfig(i);
        const TapoStatus& status = poller_->getStatus(i);

        if (!config.enabled) continue;

        JsonObject cam = cameras.createNestedObject();

        // MAC（照合キー - コロン区切り大文字）
        cam["mac"] = status.getMacString();

        // オンライン状態
        cam["online"] = status.online;

        if (status.online) {
            onlineCount++;
        } else {
            offlineCount++;
        }

        // discovery情報
        JsonObject discovery = cam.createNestedObject("discovery");
        discovery["deviceId"] = status.deviceId;
        discovery["deviceName"] = status.deviceName;

        // onvif情報
        JsonObject onvif = cam.createNestedObject("onvif");
        onvif["model"] = status.model;
        onvif["firmware"] = status.firmware;
        onvif["serial"] = status.serialNumber;
        onvif["hostname"] = status.hostname;

        // network情報
        JsonObject network = cam.createNestedObject("network");
        network["ip"] = status.getIpString();

        // ゲートウェイをIP文字列に変換
        char gwBuf[16];
        snprintf(gwBuf, sizeof(gwBuf), "%d.%d.%d.%d",
                 (status.gateway >> 24) & 0xFF,
                 (status.gateway >> 16) & 0xFF,
                 (status.gateway >> 8) & 0xFF,
                 status.gateway & 0xFF);
        network["gateway"] = gwBuf;

        network["dhcp"] = status.dhcp;

        // lastSeen（最終ポーリング成功時刻）
        // 簡略化: observedAtを使用
        cam["lastSeen"] = observedAt;
    }

    // summary
    JsonObject summary = payload.createNestedObject("summary");
    summary["total"] = onlineCount + offlineCount;
    summary["online"] = onlineCount;
    summary["offline"] = offlineCount;

    // JSON文字列化
    String json;
    json.reserve(2048);
    serializeJson(doc, json);

    Serial.printf("[CELESTIAL-T] Sending to %s\n", url.c_str());
    Serial.printf("[CELESTIAL-T] Payload: %d bytes, cameras: %d\n",
                  json.length(), cameras.size());

    // HTTP送信
    HTTPClient http;
    http.begin(url);
    http.addHeader("Content-Type", "application/json");
    http.addHeader("X-Celestial-Secret", secret);
    http.setTimeout(HTTP_TIMEOUT_MS);

    int httpCode = http.POST(json);
    yield();

    String response = http.getString();
    http.end();
    yield();

    bool success = (httpCode >= 200 && httpCode < 300);

    // バックオフ管理
    if (success) {
        consecutiveFailures_ = 0;
        Serial.printf("[CELESTIAL-T] OK %d\n", httpCode);
    } else {
        consecutiveFailures_++;
        lastFailTime_ = millis();
        if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
            Serial.printf("[CELESTIAL-T] Entering backoff (%d failures)\n",
                          consecutiveFailures_);
        }
        Serial.printf("[CELESTIAL-T] NG %d: %s\n", httpCode, response.c_str());
    }

    return success;
}
