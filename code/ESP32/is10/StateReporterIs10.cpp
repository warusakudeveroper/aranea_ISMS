/**
 * StateReporterIs10.cpp
 *
 * IS10 deviceStateReport ペイロード構築モジュール実装
 */

#include "StateReporterIs10.h"
#include "SettingManager.h"
#include "AraneaRegister.h"
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
// ペイロード構築（is10固有）
// ========================================
String StateReporterIs10::buildPayload() {
  if (!araneaReg_) return "";

  String stateEndpoint = araneaReg_->getSavedStateEndpoint();
  if (stateEndpoint.length() == 0) {
    Serial.println("[STATE-IS10] No stateEndpoint available");
    return "";
  }

  String observedAt = (ntp_ && ntp_->isSynced()) ? ntp_->getIso8601() : "1970-01-01T00:00:00Z";
  String deviceName = getDeviceName();
  bool reportFullConfig = settings_ ? settings_->getBool(Is10Keys::kReportFull, false) : false;

  // P1: StaticJsonDocumentで動的アロケーション回避
  StaticJsonDocument<4096> doc;

  // auth
  JsonObject auth = doc.createNestedObject("auth");
  auth["tid"] = tid_;
  auth["lacisId"] = lacisId_;
  auth["cic"] = cic_;

  // report
  JsonObject report = doc.createNestedObject("report");
  report["lacisId"] = lacisId_;
  report["type"] = deviceType_;
  report["observedAt"] = observedAt;

  // state
  JsonObject state = report.createNestedObject("state");

  // 基本情報
  state["deviceName"] = deviceName;
  state["IPAddress"] = WiFi.localIP().toString();
  state["MacAddress"] = mac_;
  state["RSSI"] = WiFi.RSSI();

  // ルーター統計（sshPollerから取得）
  int routerCount = routerCount_ ? *routerCount_ : 0;
  state["routerCount"] = routerCount;
  state["successCount"] = sshPoller_ ? sshPoller_->getSuccessCount() : 0;
  state["failCount"] = sshPoller_ ? sshPoller_->getFailCount() : 0;
  state["lastPollResult"] = "---";  // TODO: 状態保持
  state["mqttConnected"] = mqtt_ ? mqtt_->isConnected() : false;
  state["heap"] = ESP.getFreeHeap();

  // Applied情報（SSOT必須）
  state["appliedConfigSchemaVersion"] = schemaVersion_;
  state["appliedConfigHash"] = configHash_;
  if (lastAppliedAt_.length() > 0) {
    state["lastConfigAppliedAt"] = lastAppliedAt_;
  }
  if (lastApplyError_.length() > 0) {
    state["lastConfigApplyError"] = lastApplyError_;
  }

  // ReportedConfig
  state["reportedConfigSchemaVersion"] = schemaVersion_;
  state["reportedConfigHash"] = configHash_;

  // reportedConfig.is10
  JsonObject reportedConfig = state.createNestedObject("reportedConfig");
  JsonObject is10Cfg = reportedConfig.createNestedObject("is10");

  is10Cfg["scanInterval"] = settings_ ? settings_->getInt(Is10Keys::kScanIntv, 60) : 60;
  is10Cfg["reportClientList"] = settings_ ? settings_->getBool(Is10Keys::kReportClnt, true) : true;

  // routers配列
  if (routers_ && routerCount_) {
    JsonArray routersArr = is10Cfg.createNestedArray("routers");
    for (int i = 0; i < *routerCount_; i++) {
      JsonObject r = routersArr.createNestedObject();
      r["rid"] = routers_[i].rid;
      r["ipAddr"] = routers_[i].ipAddr;
      r["sshPort"] = routers_[i].sshPort;
      r["enabled"] = routers_[i].enabled;
      r["platform"] = (routers_[i].osType == RouterOsType::ASUSWRT) ? "asuswrt" : "openwrt";

      if (reportFullConfig) {
        r["sshUser"] = routers_[i].username;
        r["password"] = routers_[i].password;
        if (routers_[i].publicKey.length() > 0) {
          r["privateKey"] = routers_[i].publicKey;
        }
      }
    }
  }

  // P1: String::reserve()でフラグメンテーション軽減
  String json;
  json.reserve(2048);
  serializeJson(doc, json);

  // ログ出力
  String hashShort = configHash_.length() > 8
    ? configHash_.substring(0, 8) + "..."
    : configHash_;
  Serial.printf("[STATE-IS10] deviceName=\"%s\", schema=%d, hash=%s\n",
                deviceName.c_str(), schemaVersion_, hashShort.c_str());

  // HttpManagerに反映（UI用）
  if (httpMgr_) {
    httpMgr_->setLastStateReport(observedAt, 0);  // httpCodeは送信後に更新
  }

  return json;
}
