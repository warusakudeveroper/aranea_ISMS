/**
 * CelestialSenderIs10.cpp
 *
 * CelestialGlobe SSOT送信モジュール実装
 */

#include "CelestialSenderIs10.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "Is10Keys.h"
#include <WiFi.h>
#include <HTTPClient.h>
#include <ArduinoJson.h>

static const unsigned long HTTP_TIMEOUT_MS = 3000;      // P0: 15s→3sに短縮
static const int MAX_CONSECUTIVE_FAILURES = 3;          // P0: バックオフ閾値
static const unsigned long BACKOFF_DURATION_MS = 30000; // P0: 30秒バックオフ

CelestialSenderIs10::CelestialSenderIs10() {}

void CelestialSenderIs10::begin(SettingManager* settings, NtpManager* ntp,
                                 SshPollerIs10* sshPoller) {
  settings_ = settings;
  ntp_ = ntp;
  sshPoller_ = sshPoller;
  Serial.println("[CELESTIAL] Initialized");
}

void CelestialSenderIs10::setAuth(const String& tid, const String& lacisId,
                                   const String& cic, const String& fid) {
  tid_ = tid;
  lacisId_ = lacisId;
  cic_ = cic;
  fid_ = fid;
}

bool CelestialSenderIs10::isConfigured() {
  if (!settings_) return false;
  String endpoint = settings_->getString(Is10Keys::kEndpoint, "");
  String secret = settings_->getString(Is10Keys::kSecret, "");
  return endpoint.length() > 0 && secret.length() > 0;
}

bool CelestialSenderIs10::send() {
  if (!settings_ || !sshPoller_) {
    Serial.println("[CELESTIAL] Not initialized");
    return false;
  }

  // P0: WiFi接続チェック
  if (!WiFi.isConnected()) {
    Serial.println("[CELESTIAL] Skipped (WiFi not connected)");
    return false;
  }

  // P0: バックオフチェック
  unsigned long now = millis();
  if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
    if ((now - lastFailTime_) < BACKOFF_DURATION_MS) {
      Serial.println("[CELESTIAL] Skipped (backoff)");
      return false;
    }
    Serial.println("[CELESTIAL] Backoff period ended, retrying...");
    consecutiveFailures_ = 0;
  }

  // エンドポイント取得
  String baseEndpoint = settings_->getString(Is10Keys::kEndpoint, "");
  String secret = settings_->getString(Is10Keys::kSecret, "");

  // 設定不備チェック（スキップ理由を明確にログ出力）
  if (baseEndpoint.length() == 0 || secret.length() == 0) {
    Serial.println("[CELESTIAL] SKIPPED - Missing configuration:");
    if (baseEndpoint.length() == 0) {
      Serial.println("[CELESTIAL]   - Endpoint: NOT SET");
    } else {
      Serial.println("[CELESTIAL]   - Endpoint: OK");
    }
    if (secret.length() == 0) {
      Serial.println("[CELESTIAL]   - X-Celestial-Secret: NOT SET");
    } else {
      Serial.println("[CELESTIAL]   - X-Celestial-Secret: OK");
    }
    Serial.println("[CELESTIAL] Configure via HTTP UI: http://<device-ip>/");
    Serial.println("[CELESTIAL] Settings tab > CelestialGlobe section");
    return false;
  }

  // URL構築: endpoint?fid=XXX&source=araneaDevice
  String url = baseEndpoint + "?fid=" + fid_ + "&source=araneaDevice";

  // observedAt（ISO8601形式）
  String observedAt = "1970-01-01T00:00:00.000Z";
  if (ntp_ && ntp_->isSynced()) {
    observedAt = ntp_->getIso8601();
  }

  // 各ルーターごとにレポート送信（仕様: router は単一オブジェクト）
  RouterInfo* routerInfos = sshPoller_->getRouterInfos();
  int infoCount = sshPoller_->getRouterInfoCount();
  int successCount = 0;
  int sendCount = 0;

  bool reportClients = settings_->getBool(Is10Keys::kReportClnt, true);

  for (int i = 0; i < infoCount; i++) {
    RouterInfo& info = routerInfos[i];

    // P1: StaticJsonDocumentで動的アロケーション回避
    StaticJsonDocument<4096> doc;

    // auth
    JsonObject auth = doc.createNestedObject("auth");
    auth["tid"] = tid_;
    auth["lacisId"] = lacisId_;
    auth["cic"] = cic_;

    // report（CelestialGlobe仕様準拠）
    JsonObject report = doc.createNestedObject("report");
    report["observedAt"] = observedAt;
    report["sourceDevice"] = lacisId_;
    report["sourceType"] = "ar-is10";

    // router（単一オブジェクト）
    JsonObject router = report.createNestedObject("router");
    router["mac"] = formatMacWithColons(info.routerMac);
    router["wanIp"] = info.wanIp;
    router["lanIp"] = info.lanIp;
    router["ssid24"] = info.ssid24;
    router["ssid50"] = info.ssid50;
    router["online"] = info.online;
    router["clientCount"] = info.clientCount;

    // clients配列（reportClientList=trueの場合のみ）
    JsonArray clients = report.createNestedArray("clients");
    if (reportClients && info.online) {
      // TODO: クライアント詳細情報がSSHで取得できた場合に追加
      // 現状はSSH経由でクライアントリストを取得していないため空配列
    }

    // P1: String::reserve()でフラグメンテーション軽減
    String json;
    json.reserve(1024);
    serializeJson(doc, json);

    sendCount++;
    Serial.printf("[CELESTIAL] Sending router %d/%d (MAC: %s)\n",
                  sendCount, infoCount, info.routerMac.c_str());

    HTTPClient http;
    http.begin(url);
    http.addHeader("Content-Type", "application/json");

    // CelestialGlobe認証（既存）
    http.addHeader("X-Celestial-Secret", secret);

    // X-Aranea-* ヘッダー（Webhook共通仕様）
    http.addHeader("X-Aranea-SourceType", source_);  // "ar-is10"
    http.addHeader("X-Aranea-LacisId", lacisId_);
    if (deviceMac_.length() > 0) {
      http.addHeader("X-Aranea-Mac", deviceMac_);  // ESP32 MAC（12桁HEX大文字）
    }
    // ISO8601タイムスタンプ
    char timestamp[32];
    time_t now_t = time(nullptr);
    struct tm* timeinfo = gmtime(&now_t);
    strftime(timestamp, sizeof(timestamp), "%Y-%m-%dT%H:%M:%SZ", timeinfo);
    http.addHeader("X-Aranea-Timestamp", timestamp);

    http.setTimeout(HTTP_TIMEOUT_MS);

    int httpCode = http.POST(json);
    yield();  // P0: WDT対策

    String response = http.getString();
    http.end();
    yield();  // P0: WDT対策

    if (httpCode >= 200 && httpCode < 300) {
      successCount++;
      Serial.printf("[CELESTIAL] Router %s: OK %d\n", info.rid.c_str(), httpCode);
    } else {
      Serial.printf("[CELESTIAL] Router %s: NG %d - %s\n",
                    info.rid.c_str(), httpCode, response.c_str());
    }
  }

  bool success = (sendCount > 0 && successCount == sendCount);

  // P0: バックオフ管理
  if (success) {
    consecutiveFailures_ = 0;
    Serial.printf("[CELESTIAL] All %d routers sent OK\n", successCount);
  } else if (sendCount == 0) {
    Serial.println("[CELESTIAL] No routers to send");
  } else {
    consecutiveFailures_++;
    lastFailTime_ = millis();
    if (consecutiveFailures_ >= MAX_CONSECUTIVE_FAILURES) {
      Serial.printf("[CELESTIAL] Entering backoff (%d consecutive failures)\n", consecutiveFailures_);
    }
    Serial.printf("[CELESTIAL] Partial failure: %d/%d succeeded\n", successCount, sendCount);
  }

  return success;
}

// MACアドレスをコロン区切り形式に変換 (AABBCCDDEEFF → AA:BB:CC:DD:EE:FF)
String CelestialSenderIs10::formatMacWithColons(const String& mac) {
  if (mac.length() != 12) return mac;
  String formatted;
  formatted.reserve(17);
  for (int i = 0; i < 12; i += 2) {
    if (i > 0) formatted += ':';
    formatted += mac.substring(i, i + 2);
  }
  return formatted;
}
