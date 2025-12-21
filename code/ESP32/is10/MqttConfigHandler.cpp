/**
 * MqttConfigHandler.cpp
 *
 * MQTT configメッセージ処理モジュール実装
 */

#include "MqttConfigHandler.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "HttpManagerIs10.h"
#include "StateReporterIs10.h"
#include <ArduinoJson.h>

MqttConfigHandler::MqttConfigHandler() {}

void MqttConfigHandler::begin(SettingManager* settings, NtpManager* ntp,
                               RouterConfig* routers, int* routerCount,
                               HttpManagerIs10* httpMgr, StateReporterIs10* stateReporter) {
  settings_ = settings;
  ntp_ = ntp;
  routers_ = routers;
  routerCount_ = routerCount;
  httpMgr_ = httpMgr;
  stateReporter_ = stateReporter;
  Serial.println("[MQTT-CFG] Initialized");
}

// ========================================
// MQTT config適用（routers配列フル更新）
// SSOT: secrets（sshUser/password/privateKey）も受け取り・保存・適用
// CelestialGlobe送信には絶対に載せない
// ========================================
void MqttConfigHandler::applyRouterConfig(JsonArray routersJson) {
  if (!routers_ || !routerCount_) return;

  // SSOT方針: MQTTからの設定で完全上書き（マージではなく置換）
  // 既存設定はDesiredで上書きされる

  int newCount = 0;
  for (JsonObject r : routersJson) {
    if (newCount >= MAX_ROUTERS) break;

    String rid = r["rid"] | "";
    if (rid.length() == 0) continue;

    RouterConfig& router = routers_[newCount];
    router.rid = rid;
    router.ipAddr = r["ipAddr"] | "";
    router.sshPort = r["sshPort"] | 22;
    router.enabled = r["enabled"] | true;

    // secrets: 受け取り・保存・適用OK（CelestialGlobeには載せない）
    // SSOT仕様: sshUser → username, password, privateKey → publicKey
    if (r.containsKey("sshUser")) {
      router.username = r["sshUser"].as<String>();
    } else if (r.containsKey("username")) {
      router.username = r["username"].as<String>();
    }

    if (r.containsKey("password")) {
      router.password = r["password"].as<String>();
    }

    if (r.containsKey("privateKey")) {
      router.publicKey = r["privateKey"].as<String>();  // publicKeyフィールドに格納
    } else if (r.containsKey("publicKey")) {
      router.publicKey = r["publicKey"].as<String>();
    }

    // platform → osType マッピング
    String platform = r["platform"] | "asuswrt";
    platform.toLowerCase();
    if (platform == "openwrt") {
      router.osType = RouterOsType::OPENWRT;
    } else {
      router.osType = RouterOsType::ASUSWRT;  // デフォルト
    }

    // ログ出力（secretsは出さない）
    Serial.printf("[CONFIG] Router %s: %s:%d (%s)\n",
      rid.c_str(), router.ipAddr.c_str(), router.sshPort,
      (router.osType == RouterOsType::ASUSWRT) ? "ASUSWRT" : "OpenWrt");

    newCount++;
  }

  *routerCount_ = newCount;
  Serial.printf("[CONFIG] Applied %d routers from MQTT config\n", newCount);

  if (httpMgr_) {
    httpMgr_->setRouterStatus(newCount, 0, 0);  // ステータス更新
  }
}

// ========================================
// MQTT configメッセージ処理（SSOT双方向同期）
// 仕様:
// - schemaVersion巻き戻し禁止
// - secrets（sshUser/password/privateKey）は受け取り・保存・適用OK
// - configHashはサーバ発行をそのまま保存（is10は計算しない）
// - 適用成功後は即時deviceStateReport
// ========================================
void MqttConfigHandler::handleConfigMessage(const char* data, int len) {
  Serial.printf("[MQTT] Config received: %d bytes\n", len);

  if (!settings_ || !stateReporter_) {
    Serial.println("[MQTT-CFG] Not initialized");
    return;
  }

  // JSONパース（is10 config用に大きめのバッファ）
  DynamicJsonDocument doc(4096);
  DeserializationError err = deserializeJson(doc, data, len);
  if (err) {
    Serial.printf("[MQTT] JSON parse error: %s\n", err.c_str());
    stateReporter_->setApplyError("JSON parse error: " + String(err.c_str()));
    return;
  }

  // schemaVersion巻き戻し禁止
  int schemaVersion = doc["schemaVersion"] | 0;
  int currentSchemaVersion = stateReporter_->getSchemaVersion();
  if (schemaVersion <= currentSchemaVersion) {
    Serial.printf("[MQTT] Ignoring old/same schema: %d <= %d (rollback prevention)\n",
                  schemaVersion, currentSchemaVersion);
    return;
  }

  // configHash取得（サーバ発行、is10は計算しない）
  String configHash = doc["configHash"] | "";
  String configHashShort = configHash.length() > 8 ? configHash.substring(0, 8) + "..." : configHash;
  Serial.printf("[MQTT] New config: schemaVersion=%d, hash=%s\n", schemaVersion, configHashShort.c_str());

  // config.is10を読む
  JsonObject is10Cfg = doc["config"]["is10"];
  if (is10Cfg.isNull()) {
    Serial.println("[MQTT] No config.is10 in message, ignoring");
    stateReporter_->setApplyError("No config.is10 in message");
    return;
  }

  // ========================================
  // Apply処理開始
  // ========================================
  bool applySuccess = true;
  String applyError = "";

  // scanInterval適用（秒）
  if (is10Cfg.containsKey("scanInterval")) {
    int sec = is10Cfg["scanInterval"];
    if (sec >= 60 && sec <= 86400) {
      settings_->setInt("is10_scan_interval_sec", sec);
      Serial.printf("[CONFIG] scanInterval=%d sec\n", sec);
    } else {
      Serial.printf("[CONFIG] scanInterval=%d out of range (60-86400), skipped\n", sec);
    }
  }

  // reportClientList適用
  if (is10Cfg.containsKey("reportClientList")) {
    bool reportClients = is10Cfg["reportClientList"];
    settings_->setBool("is10_report_clients", reportClients);
    Serial.printf("[CONFIG] reportClientList=%s\n", reportClients ? "true" : "false");
  }

  // routers適用（secrets含む完全置換）
  if (is10Cfg.containsKey("routers")) {
    applyRouterConfig(is10Cfg["routers"].as<JsonArray>());

    // routers.jsonに永続化
    if (httpMgr_) {
      httpMgr_->persistRouters();
      Serial.println("[CONFIG] Routers persisted to SPIFFS");
    }
  }

  // ========================================
  // 適用成功時の保存処理（StateReporterIs10経由）
  // ========================================
  if (applySuccess) {
    // ISO8601形式で現在時刻
    String appliedAt = (ntp_ && ntp_->isSynced()) ? ntp_->getIso8601() : "1970-01-01T00:00:00Z";

    // StateReporterIs10にSSoT状態を設定（NVS保存含む）
    stateReporter_->setAppliedConfig(schemaVersion, configHash, appliedAt);

    Serial.printf("[CONFIG] Applied successfully: schemaVersion=%d, hash=%s, at=%s\n",
                  schemaVersion, configHashShort.c_str(), appliedAt.c_str());

    // コールバック呼び出し（即時deviceStateReport送信用）
    if (configAppliedCallback_) {
      configAppliedCallback_();
    }
  } else {
    // 適用失敗時（旧設定は保持）
    stateReporter_->setApplyError(applyError);
    Serial.printf("[CONFIG] Apply failed: %s\n", applyError.c_str());
  }
}
