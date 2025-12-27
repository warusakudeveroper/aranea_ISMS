/**
 * AraneaSettings.cpp (is02a用)
 */

#include "AraneaSettings.h"
#include "SettingManager.h"
#include "Is02aKeys.h"

bool AraneaSettings::_initialized = false;

void AraneaSettings::init() {
  _initialized = true;
  Serial.println("[AraneaSettings] Initialized");
}

void AraneaSettings::initDefaults(SettingManager& settings) {
  // テナント設定（未設定時のみデフォルト投入）
  if (settings.getString("tid", "").length() == 0) {
    settings.setString("tid", ARANEA_DEFAULT_TID);
  }
  if (settings.getString("fid", "").length() == 0) {
    settings.setString("fid", ARANEA_DEFAULT_FID);
  }
  if (settings.getString("tenant_lacis", "").length() == 0) {
    settings.setString("tenant_lacis", ARANEA_DEFAULT_TENANT_LACISID);
  }
  if (settings.getString("tenant_email", "").length() == 0) {
    settings.setString("tenant_email", ARANEA_DEFAULT_TENANT_EMAIL);
  }
  if (settings.getString("tenant_cic", "").length() == 0) {
    settings.setString("tenant_cic", ARANEA_DEFAULT_TENANT_CIC);
  }

  // WiFi設定（6スロット）
  if (settings.getString("wifi_ssid1", "").length() == 0) {
    settings.setString("wifi_ssid1", ARANEA_DEFAULT_WIFI_SSID_1);
    settings.setString("wifi_pass1", ARANEA_DEFAULT_WIFI_PASS_1);
  }
  if (settings.getString("wifi_ssid2", "").length() == 0) {
    settings.setString("wifi_ssid2", ARANEA_DEFAULT_WIFI_SSID_2);
    settings.setString("wifi_pass2", ARANEA_DEFAULT_WIFI_PASS_2);
  }
  if (settings.getString("wifi_ssid3", "").length() == 0) {
    settings.setString("wifi_ssid3", ARANEA_DEFAULT_WIFI_SSID_3);
    settings.setString("wifi_pass3", ARANEA_DEFAULT_WIFI_PASS_3);
  }
  if (settings.getString("wifi_ssid4", "").length() == 0) {
    settings.setString("wifi_ssid4", ARANEA_DEFAULT_WIFI_SSID_4);
    settings.setString("wifi_pass4", ARANEA_DEFAULT_WIFI_PASS_4);
  }
  if (settings.getString("wifi_ssid5", "").length() == 0) {
    settings.setString("wifi_ssid5", ARANEA_DEFAULT_WIFI_SSID_5);
    settings.setString("wifi_pass5", ARANEA_DEFAULT_WIFI_PASS_5);
  }
  if (settings.getString("wifi_ssid6", "").length() == 0) {
    settings.setString("wifi_ssid6", ARANEA_DEFAULT_WIFI_SSID_6);
    settings.setString("wifi_pass6", ARANEA_DEFAULT_WIFI_PASS_6);
  }

  // エンドポイント
  if (settings.getString("gate_url", "").length() == 0) {
    settings.setString("gate_url", ARANEA_DEFAULT_GATE_URL);
  }
  if (settings.getString("cloud_url", "").length() == 0) {
    settings.setString("cloud_url", ARANEA_DEFAULT_CLOUD_URL);
  }

  // MQTT設定
  if (settings.getString(Is02aKeys::kMqttBroker, "").length() == 0) {
    settings.setString(Is02aKeys::kMqttBroker, ARANEA_DEFAULT_MQTT_BROKER);
  }
  if (settings.getInt(Is02aKeys::kMqttPort, 0) == 0) {
    settings.setInt(Is02aKeys::kMqttPort, ARANEA_DEFAULT_MQTT_PORT);
  }
  // kMqttTls はboolなので、キーが存在しない場合に初期化
  if (!settings.hasKey(Is02aKeys::kMqttTls)) {
    settings.setBool(Is02aKeys::kMqttTls, ARANEA_DEFAULT_MQTT_TLS);
  }

  // BLE設定
  if (settings.getInt(Is02aKeys::kBleScanSec, 0) == 0) {
    settings.setInt(Is02aKeys::kBleScanSec, IS02A_DEFAULT_BLE_SCAN_SEC);
  }

  // センサー設定
  if (settings.getInt(Is02aKeys::kSelfIntv, 0) == 0) {
    settings.setInt(Is02aKeys::kSelfIntv, IS02A_DEFAULT_SELF_INTERVAL);
  }
  // オフセットは0も有効な値なので、キー存在チェック（floatはStringで保存）
  if (!settings.hasKey(Is02aKeys::kTempOffset)) {
    settings.setString(Is02aKeys::kTempOffset, String(IS02A_DEFAULT_TEMP_OFFSET, 1));
  }
  if (!settings.hasKey(Is02aKeys::kHumOffset)) {
    settings.setString(Is02aKeys::kHumOffset, String(IS02A_DEFAULT_HUM_OFFSET, 1));
  }

  // バッチ送信設定
  if (settings.getInt(Is02aKeys::kBatchIntv, 0) == 0) {
    settings.setInt(Is02aKeys::kBatchIntv, IS02A_DEFAULT_BATCH_INTERVAL);
  }

  // ステータスレポート設定
  if (settings.getInt(Is02aKeys::kReportIntv, 0) == 0) {
    settings.setInt(Is02aKeys::kReportIntv, IS02A_DEFAULT_REPORT_INTERVAL);
  }

  // リブートスケジューラ設定（-1も有効な値なので hasKey でチェック）
  if (!settings.hasKey(Is02aKeys::kRebootHour)) {
    settings.setInt(Is02aKeys::kRebootHour, IS02A_DEFAULT_REBOOT_HOUR);
  }
  if (!settings.hasKey(Is02aKeys::kRebootMin)) {
    settings.setInt(Is02aKeys::kRebootMin, IS02A_DEFAULT_REBOOT_MIN);
  }

  // ノード蓄積設定（オンメモリ限定）
  if (settings.getInt(Is02aKeys::kMaxNodes, 0) == 0) {
    settings.setInt(Is02aKeys::kMaxNodes, IS02A_DEFAULT_MAX_NODES);
  }

  // リレー先設定
  if (settings.getString(Is02aKeys::kRelayUrl, "").length() == 0) {
    settings.setString(Is02aKeys::kRelayUrl, ARANEA_DEFAULT_RELAY_PRIMARY);
  }
  if (settings.getString(Is02aKeys::kRelayUrl2, "").length() == 0) {
    settings.setString(Is02aKeys::kRelayUrl2, ARANEA_DEFAULT_RELAY_SECONDARY);
  }

  Serial.println("[AraneaSettings] Defaults initialized");
}

String AraneaSettings::getTid() {
  return ARANEA_DEFAULT_TID;
}

String AraneaSettings::getFid() {
  return ARANEA_DEFAULT_FID;
}

String AraneaSettings::getTenantLacisId() {
  return ARANEA_DEFAULT_TENANT_LACISID;
}

String AraneaSettings::getTenantEmail() {
  return ARANEA_DEFAULT_TENANT_EMAIL;
}

String AraneaSettings::getTenantCic() {
  return ARANEA_DEFAULT_TENANT_CIC;
}

String AraneaSettings::getGateUrl() {
  return ARANEA_DEFAULT_GATE_URL;
}

String AraneaSettings::getCloudUrl() {
  return ARANEA_DEFAULT_CLOUD_URL;
}

String AraneaSettings::getRelayPrimary() {
  return ARANEA_DEFAULT_RELAY_PRIMARY;
}

String AraneaSettings::getRelaySecondary() {
  return ARANEA_DEFAULT_RELAY_SECONDARY;
}

String AraneaSettings::getMqttBroker() {
  return ARANEA_DEFAULT_MQTT_BROKER;
}

int AraneaSettings::getMqttPort() {
  return ARANEA_DEFAULT_MQTT_PORT;
}

String AraneaSettings::getMqttUser() {
  return ARANEA_DEFAULT_MQTT_USER;
}

String AraneaSettings::getMqttPass() {
  return ARANEA_DEFAULT_MQTT_PASS;
}

bool AraneaSettings::getMqttTls() {
  return ARANEA_DEFAULT_MQTT_TLS;
}

int AraneaSettings::getBleScanSec() {
  return IS02A_DEFAULT_BLE_SCAN_SEC;
}

String AraneaSettings::getBleFilter() {
  return IS02A_DEFAULT_BLE_FILTER;
}

float AraneaSettings::getTempOffset() {
  return IS02A_DEFAULT_TEMP_OFFSET;
}

float AraneaSettings::getHumOffset() {
  return IS02A_DEFAULT_HUM_OFFSET;
}

int AraneaSettings::getSelfIntervalSec() {
  return IS02A_DEFAULT_SELF_INTERVAL;
}

int AraneaSettings::getBatchIntervalSec() {
  return IS02A_DEFAULT_BATCH_INTERVAL;
}
