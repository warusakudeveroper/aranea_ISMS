/**
 * StateReporter.cpp
 *
 * Aranea デバイス共通 deviceStateReport送信 実装
 */

#include "StateReporter.h"
#include <WiFi.h>
#include <HTTPClient.h>

static const unsigned long HTTP_TIMEOUT_MS = 3000;      // P0: 10s→3sに短縮
static const int MAX_CONSECUTIVE_FAILURES = 3;          // P0: バックオフ閾値
static const unsigned long BACKOFF_DURATION_MS = 30000; // P0: 30秒バックオフ

StateReporter::StateReporter() {}

void StateReporter::begin(const String& endpoint) {
  endpoint_ = endpoint;
  lastHeartbeat_ = millis();
  Serial.println("[STATE-REPORTER] Initialized");
}

void StateReporter::handle() {
  if (!enabled_ || endpoint_.length() == 0) return;

  unsigned long now = millis();

  // Heartbeat送信（インターバル経過後）
  if (now - lastHeartbeat_ >= heartbeatIntervalMs_) {
    if (buildPayloadCallback_) {
      String payload = buildPayloadCallback_();
      if (payload.length() > 0) {
        bool success = sendReport(payload);
        if (!initialReportSent_ && success) {
          initialReportSent_ = true;
          Serial.println("[STATE-REPORTER] Initial report sent");
        }
      }
    }
    lastHeartbeat_ = now;
  }
}

bool StateReporter::sendReport(const String& jsonPayload) {
  if (endpoint_.length() == 0) {
    Serial.println("[STATE-REPORTER] No endpoint configured");
    return false;
  }

  if (jsonPayload.length() == 0) {
    Serial.println("[STATE-REPORTER] Empty payload, skipping");
    return false;
  }

  // P0: WiFi接続チェック（未接続時は即座にスキップ）
  if (!WiFi.isConnected()) {
    Serial.println("[STATE-REPORTER] Skipped (WiFi not connected)");
    return false;
  }

  // P0: バックオフチェック（連続失敗後は30秒待機）
  unsigned long now = millis();
  if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
    if ((now - lastFailTime_) < BACKOFF_DURATION_MS) {
      Serial.println("[STATE-REPORTER] Skipped (backoff)");
      return false;
    }
    // バックオフ期間終了、リトライ許可
    Serial.println("[STATE-REPORTER] Backoff period ended, retrying...");
    consecutiveFailures_ = 0;
  }

  Serial.printf("[STATE-REPORTER] Sending to %s (%d bytes)\n",
                endpoint_.c_str(), jsonPayload.length());

  HTTPClient http;
  http.begin(endpoint_);
  http.addHeader("Content-Type", "application/json");
  http.setTimeout(HTTP_TIMEOUT_MS);  // P0: 10s→3sに短縮

  int httpCode = http.POST(jsonPayload);
  yield();  // P0: WDT対策

  String response = http.getString();
  http.end();
  yield();  // P0: WDT対策

  bool success = (httpCode >= 200 && httpCode < 300);
  Serial.printf("[STATE-REPORTER] HTTP %d %s\n", httpCode, success ? "OK" : "NG");

  // P0: バックオフ管理
  if (success) {
    consecutiveFailures_ = 0;
  } else {
    consecutiveFailures_++;
    lastFailTime_ = now;
    if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
      Serial.printf("[STATE-REPORTER] Entering backoff (%d consecutive failures)\n", consecutiveFailures_);
    }
    Serial.printf("[STATE-REPORTER] Response: %s\n", response.c_str());
  }

  // コールバック呼び出し
  if (reportSentCallback_) {
    reportSentCallback_(success, httpCode);
  }

  return success;
}
