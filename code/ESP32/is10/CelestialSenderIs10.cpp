/**
 * CelestialSenderIs10.cpp
 *
 * CelestialGlobe SSOT送信モジュール実装
 */

#include "CelestialSenderIs10.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include <HTTPClient.h>
#include <ArduinoJson.h>

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
  String endpoint = settings_->getString("is10_endpoint", "");
  String secret = settings_->getString("is10_celestial_secret", "");
  return endpoint.length() > 0 && secret.length() > 0;
}

bool CelestialSenderIs10::send() {
  if (!settings_ || !sshPoller_) {
    Serial.println("[CELESTIAL] Not initialized");
    return false;
  }

  // エンドポイント取得
  String baseEndpoint = settings_->getString("is10_endpoint", "");
  String secret = settings_->getString("is10_celestial_secret", "");

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

  // observedAtはUnix ms（number型）
  unsigned long long observedAtMs = 0;
  if (ntp_ && ntp_->isSynced()) {
    observedAtMs = (unsigned long long)ntp_->getEpoch() * 1000ULL;
  } else {
    observedAtMs = millis();  // フォールバック（不正確だが送信は可能）
  }

  // JSON構築（最大4KB）
  DynamicJsonDocument doc(4096);

  // auth
  JsonObject auth = doc.createNestedObject("auth");
  auth["tid"] = tid_;
  auth["lacisId"] = lacisId_;  // is10のlacisId（observer）
  auth["cic"] = cic_;

  // source（CelestialGlobe用）
  doc["source"] = source_;

  // observedAt（Unix ms, number型）
  doc["observedAt"] = observedAtMs;

  // devices配列
  JsonArray devices = doc.createNestedArray("devices");

  RouterInfo* routerInfos = sshPoller_->getRouterInfos();
  int infoCount = sshPoller_->getRouterInfoCount();
  for (int i = 0; i < infoCount; i++) {
    RouterInfo& info = routerInfos[i];
    if (!info.online) continue;  // オフラインはスキップ

    JsonObject dev = devices.createNestedObject();

    // MACアドレス（大文字12HEX - サーバがlacisId生成に使用）
    // 仕様: ルーターのlacisIdはサーバがMACから生成する
    dev["mac"] = info.routerMac;  // 大文字12HEX正規化済み

    dev["type"] = "Router";
    dev["label"] = "Room" + info.rid;
    dev["status"] = info.online ? "online" : "offline";
    dev["lanIp"] = info.lanIp;
    dev["wanIp"] = info.wanIp;
    dev["ssid24"] = info.ssid24;
    dev["ssid50"] = info.ssid50;
    dev["clientCount"] = info.clientCount;
    // firmware, uptime は取得していないので省略
    // parentMac はis10のMACを入れない（仕様禁止）
  }

  // clients配列（reportClientList=trueの場合のみ）
  // 仕様: 各clientにapMac（接続先ルーターMAC）を含める
  // 例: { "mac": "AABBCCDDEEFF", "apMac": "112233445566", "hostname": "iPhone" }
  bool reportClients = settings_->getBool("is10_report_clients", true);
  if (reportClients) {
    JsonArray clients = doc.createNestedArray("clients");
    // TODO: クライアント詳細情報がSSHで取得できた場合に追加
    // DHCPリースからclient MAC取得 → apMac = info.routerMac をセット
    // parentMacはルーターの上流が確実に分かる場合のみ（不明なら空）
  }

  String json;
  serializeJson(doc, json);

  Serial.printf("[CELESTIAL] Sending to %s\n", url.c_str());
  Serial.printf("[CELESTIAL] Payload size: %d bytes, devices: %d\n",
                json.length(), devices.size());

  HTTPClient http;
  http.begin(url);
  http.addHeader("Content-Type", "application/json");
  http.addHeader("X-Celestial-Secret", secret);
  http.setTimeout(15000);

  int httpCode = http.POST(json);
  String response = http.getString();
  http.end();

  bool success = (httpCode >= 200 && httpCode < 300);
  if (success) {
    Serial.printf("[CELESTIAL] OK %d\n", httpCode);
  } else {
    Serial.printf("[CELESTIAL] NG %d: %s\n", httpCode, response.c_str());
  }

  return success;
}
