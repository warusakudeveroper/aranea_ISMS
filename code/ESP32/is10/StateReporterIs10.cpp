/**
 * StateReporterIs10.cpp
 *
 * IS10 deviceStateReport ペイロード構築モジュール実装
 */

#include "StateReporterIs10.h"
#include "SettingManager.h"
#include "AraneaRegister.h"
#include "AraneaSettings.h"
#include "NtpManager.h"
#include "MqttManager.h"
#include "SshPollerIs10.h"
#include "HttpManagerIs10.h"
#include "Is10Keys.h"
#include <ArduinoJson.h>
#include <WiFi.h>
#include <esp_heap_caps.h>

StateReporterIs10::StateReporterIs10() {}

void StateReporterIs10::begin(StateReporter* reporter, SettingManager* settings,
                               AraneaRegister* araneaReg, NtpManager* ntp,
                               MqttManager* mqtt, SshPollerIs10* sshPoller,
                               RouterConfig* routers, int* routerCount) {
  reporter_ = reporter;
  settings_ = settings;
  araneaReg_ = araneaReg;
  ntp_ = ntp;
  mqtt_ = mqtt;
  sshPoller_ = sshPoller;
  routers_ = routers;
  routerCount_ = routerCount;

  // NVSからSSoT状態を復元
  schemaVersion_ = settings_->getInt(Is10Keys::kSchema, 0);
  configHash_ = settings_->getString(Is10Keys::kHash, "");
  lastAppliedAt_ = settings_->getString(Is10Keys::kAppliedAt, "");
  lastApplyError_ = settings_->getString(Is10Keys::kApplyErr, "");

  // StateReporterにペイロード構築コールバックを登録
  if (reporter_) {
    reporter_->onBuildPayload([this]() { return this->buildPayload(); });
  }

  Serial.println("[STATE-IS10] Initialized");
}

void StateReporterIs10::setAppliedConfig(int schemaVersion, const String& hash,
                                          const String& appliedAt) {
  schemaVersion_ = schemaVersion;
  configHash_ = hash;
  lastAppliedAt_ = appliedAt;
  lastApplyError_ = "";  // エラークリア

  // NVS保存
  if (settings_) {
    settings_->setInt(Is10Keys::kSchema, schemaVersion);
    settings_->setString(Is10Keys::kHash, hash);
    settings_->setString(Is10Keys::kAppliedAt, appliedAt);
    settings_->setString(Is10Keys::kApplyErr, "");
  }
}

void StateReporterIs10::setApplyError(const String& error) {
  lastApplyError_ = error;
  if (settings_) {
    settings_->setString(Is10Keys::kApplyErr, error);
  }
}

// ========================================
// deviceName取得（デフォルト付き）
// ========================================
String StateReporterIs10::getDeviceName() {
  String deviceName = settings_ ? settings_->getString(CommonKeys::kDeviceName, "") : "";
  deviceName = sanitizeDeviceName(deviceName);

  // 空の場合はホスト名をデフォルトに
  if (deviceName.length() == 0) {
    deviceName = hostname_;
  }

  return deviceName;
}

// ========================================
// deviceName sanitize（SSOT準拠）
// ========================================
String StateReporterIs10::sanitizeDeviceName(const String& raw) {
  String result;
  result.reserve(64);

  bool lastWasSpace = true;
  for (int i = 0; i < (int)raw.length() && result.length() < 64; i++) {
    char c = raw.charAt(i);

    if (c < 0x20 || c == 0x7F) {
      if (!lastWasSpace && result.length() > 0) {
        result += ' ';
        lastWasSpace = true;
      }
      continue;
    }

    if (c == ' ') {
      if (!lastWasSpace) {
        result += c;
        lastWasSpace = true;
      }
      continue;
    }

    result += c;
    lastWasSpace = false;
  }

  result.trim();
  return result;
}

// ========================================
// ペイロード構築（araneaDeviceGate deviceStateReport形式）
// ========================================
String StateReporterIs10::buildPayload() {
  String observedAt = (ntp_ && ntp_->isSynced()) ? ntp_->getIso8601() : "1970-01-01T00:00:00Z";

  // ルーター情報取得（SshPollerから）
  RouterInfo* routerInfos = sshPoller_ ? sshPoller_->getRouterInfos() : nullptr;
  int routerInfoCount = sshPoller_ ? sshPoller_->getRouterInfoCount() : 0;

  // deviceStateReportペイロード構築
  StaticJsonDocument<4096> doc;

  // auth
  JsonObject auth = doc.createNestedObject("auth");
  auth["tid"] = tid_;
  auth["lacisId"] = lacisId_;
  auth["cic"] = cic_;

  // report
  JsonObject report = doc.createNestedObject("report");
  report["lacisId"] = lacisId_;
  report["type"] = deviceType_;  // "aranea_ar-is10"
  report["observedAt"] = observedAt;

  // state
  JsonObject state = report.createNestedObject("state");
  state["deviceName"] = getDeviceName();
  state["IPAddress"] = WiFi.localIP().toString();
  state["hostname"] = hostname_;
  state["mac"] = mac_;
  state["rssi"] = WiFi.RSSI();
  state["routerCount"] = routerInfoCount;

  // SSOT: 適用済み設定情報
  if (schemaVersion_ > 0) {
    state["schemaVersion"] = schemaVersion_;
    state["configHash"] = configHash_;
    state["lastAppliedAt"] = lastAppliedAt_;
  }
  if (lastApplyError_.length() > 0) {
    state["lastApplyError"] = lastApplyError_;
  }

  // reportedConfig: is10固有設定
  JsonObject reportedConfig = state.createNestedObject("reportedConfig");
  JsonObject is10Config = reportedConfig.createNestedObject("is10");

  // scanInterval取得
  int scanInterval = settings_ ? settings_->getInt(Is10Keys::kScanIntv, 60) : 60;
  is10Config["scanInterval"] = scanInterval;

  // ルーター設定配列
  JsonArray routersArray = is10Config.createNestedArray("routers");
  if (routers_ && routerCount_) {
    for (int i = 0; i < *routerCount_; i++) {
      JsonObject routerObj = routersArray.createNestedObject();
      routerObj["rid"] = routers_[i].rid;
      routerObj["ipAddr"] = routers_[i].ipAddr;
      routerObj["osType"] = (int)routers_[i].osType;
      // online状態を追加（ポーリング結果から）
      if (routerInfos && i < routerInfoCount) {
        routerObj["online"] = routerInfos[i].online;
        routerObj["clientCount"] = routerInfos[i].clientCount;
      }
    }
  }

  // P1: String::reserve()でフラグメンテーション軽減
  String json;
  json.reserve(2048);
  serializeJson(doc, json);

  // ログ出力
  Serial.printf("[STATE-IS10] deviceStateReport: routers=%d\n", routerInfoCount);

  // HttpManagerに反映（UI用）
  if (httpMgr_) {
    httpMgr_->setLastStateReport(observedAt, 0);
  }

  return json;
}
