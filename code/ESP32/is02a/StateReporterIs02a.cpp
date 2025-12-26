/**
 * StateReporterIs02a.cpp
 */

#include "StateReporterIs02a.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include <ArduinoJson.h>
#include <HTTPClient.h>
#include <WiFi.h>

StateReporterIs02a::StateReporterIs02a()
    : settings_(nullptr)
    , ntp_(nullptr)
    , bleReceiver_(nullptr)
    , relaySuccessCount_(0)
    , relayFailCount_(0)
    , lastBatchTime_("---")
    , consecutiveFailures_(0)
    , lastFailTime_(0)
{
  selfData_.temperature = NAN;
  selfData_.humidity = NAN;
  selfData_.valid = false;
}

void StateReporterIs02a::begin(SettingManager* settings, NtpManager* ntp, BleReceiver* bleReceiver) {
  settings_ = settings;
  ntp_ = ntp;
  bleReceiver_ = bleReceiver;
  Serial.println("[StateReporterIs02a] Initialized");
}

void StateReporterIs02a::setAuth(const String& tid, const String& lacisId, const String& cic) {
  tid_ = tid;
  lacisId_ = lacisId;
  cic_ = cic;
}

void StateReporterIs02a::setMac(const String& mac) {
  mac_ = mac;
}

void StateReporterIs02a::setIp(const String& ip) {
  ip_ = ip;
}

void StateReporterIs02a::setRelayUrls(const String& primary, const String& secondary) {
  relayPrimaryUrl_ = primary;
  relaySecondaryUrl_ = secondary;
}

void StateReporterIs02a::setSelfSensorData(float temp, float hum, bool valid) {
  selfData_.temperature = temp;
  selfData_.humidity = hum;
  selfData_.valid = valid;
}

void StateReporterIs02a::resetStats() {
  relaySuccessCount_ = 0;
  relayFailCount_ = 0;
  consecutiveFailures_ = 0;
}

String StateReporterIs02a::buildPayload() {
  DynamicJsonDocument doc(4096);

  // Gateway情報
  JsonObject gateway = doc.createNestedObject("gateway");
  gateway["lacisId"] = lacisId_;
  gateway["cic"] = cic_;

  // 自己計測
  JsonObject self = gateway.createNestedObject("self");
  if (selfData_.valid) {
    self["temperature"] = selfData_.temperature;
    self["humidity"] = selfData_.humidity;
  } else {
    self["temperature"] = nullptr;
    self["humidity"] = nullptr;
  }

  gateway["rssi"] = WiFi.RSSI();
  gateway["ip"] = ip_;

  // Nodes配列
  JsonArray nodesArr = doc.createNestedArray("nodes");

  if (bleReceiver_) {
    auto nodes = bleReceiver_->getNodes();
    String timestamp = ntp_ ? ntp_->getIso8601() : "";

    for (const auto& node : nodes) {
      if (!node.valid) continue;

      JsonObject nodeObj = nodesArr.createNestedObject();
      nodeObj["lacisId"] = node.lacisId;
      nodeObj["mac"] = node.mac;
      nodeObj["temperature"] = node.temperature;
      nodeObj["humidity"] = node.humidity;
      nodeObj["battery"] = node.battery;
      nodeObj["rssi"] = node.rssi;
      nodeObj["bootCount"] = node.bootCount;

      // receivedAt（相対時間から絶対時間に変換）
      if (ntp_) {
        unsigned long ageSec = (millis() - node.receivedAt) / 1000;
        // 簡易的に現在時刻からageを引く
        nodeObj["receivedAt"] = timestamp;  // TODO: 正確な時刻計算
      }
    }
  }

  // タイムスタンプ
  if (ntp_) {
    doc["timestamp"] = ntp_->getIso8601();
  }

  String payload;
  serializeJson(doc, payload);
  return payload;
}

bool StateReporterIs02a::sendBatchToRelay() {
  String payload = buildPayload();

  if (payload.length() < 10) {
    Serial.println("[StateReporterIs02a] Empty payload, skipping");
    return false;
  }

  // バックオフ制御
  if (consecutiveFailures_ > 0) {
    unsigned long backoffMs = min(consecutiveFailures_ * 5000UL, 60000UL);
    if (millis() - lastFailTime_ < backoffMs) {
      Serial.printf("[StateReporterIs02a] Backoff: %d failures, waiting %lu ms\n",
        consecutiveFailures_, backoffMs);
      return false;
    }
  }

  // Primary URL送信
  bool success = false;
  if (relayPrimaryUrl_.length() > 0) {
    success = postToUrl(relayPrimaryUrl_, payload);
  }

  // フォールバック
  if (!success && relaySecondaryUrl_.length() > 0) {
    success = postToUrl(relaySecondaryUrl_, payload);
  }

  // 統計更新
  if (success) {
    relaySuccessCount_++;
    consecutiveFailures_ = 0;
    if (ntp_) {
      lastBatchTime_ = ntp_->getIso8601();
    } else {
      lastBatchTime_ = String(millis() / 1000) + "s";
    }
  } else {
    relayFailCount_++;
    consecutiveFailures_++;
    lastFailTime_ = millis();
  }

  return success;
}

bool StateReporterIs02a::postToUrl(const String& url, const String& payload) {
  if (url.length() == 0) return false;

  HTTPClient http;
  http.begin(url);
  http.addHeader("Content-Type", "application/json");
  http.setTimeout(10000);

  int httpCode = http.POST(payload);
  http.end();

  if (httpCode >= 200 && httpCode < 300) {
    Serial.printf("[StateReporterIs02a] POST %s -> %d\n", url.c_str(), httpCode);
    return true;
  } else {
    Serial.printf("[StateReporterIs02a] POST %s FAILED -> %d\n", url.c_str(), httpCode);
    return false;
  }
}
