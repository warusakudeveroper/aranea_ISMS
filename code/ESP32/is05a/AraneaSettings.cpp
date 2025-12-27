/**
 * AraneaSettings.cpp (is05a用)
 */

#include "AraneaSettings.h"
#include "SettingManager.h"
#include "Is05aKeys.h"

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

  // WiFi設定（6ペア）
  const char* ssids[] = {
    ARANEA_DEFAULT_WIFI_SSID_1, ARANEA_DEFAULT_WIFI_SSID_2,
    ARANEA_DEFAULT_WIFI_SSID_3, ARANEA_DEFAULT_WIFI_SSID_4,
    ARANEA_DEFAULT_WIFI_SSID_5, ARANEA_DEFAULT_WIFI_SSID_6
  };
  const char* passes[] = {
    ARANEA_DEFAULT_WIFI_PASS_1, ARANEA_DEFAULT_WIFI_PASS_2,
    ARANEA_DEFAULT_WIFI_PASS_3, ARANEA_DEFAULT_WIFI_PASS_4,
    ARANEA_DEFAULT_WIFI_PASS_5, ARANEA_DEFAULT_WIFI_PASS_6
  };
  for (int i = 0; i < 6; i++) {
    String ssidKey = "wifi_ssid" + String(i + 1);
    String passKey = "wifi_pass" + String(i + 1);
    if (settings.getString(ssidKey.c_str(), "").length() == 0 && strlen(ssids[i]) > 0) {
      settings.setString(ssidKey.c_str(), ssids[i]);
      settings.setString(passKey.c_str(), passes[i]);
    }
  }

  // エンドポイント
  if (settings.getString("gate_url", "").length() == 0) {
    settings.setString("gate_url", ARANEA_DEFAULT_GATE_URL);
  }
  if (settings.getString("cloud_url", "").length() == 0) {
    settings.setString("cloud_url", ARANEA_DEFAULT_CLOUD_URL);
  }
  if (settings.getString("relay_pri", "").length() == 0) {
    settings.setString("relay_pri", ARANEA_DEFAULT_RELAY_PRIMARY);
  }
  if (settings.getString("relay_sec", "").length() == 0) {
    settings.setString("relay_sec", ARANEA_DEFAULT_RELAY_SECONDARY);
  }

  // MQTT設定
  if (settings.getString(Is05aKeys::kMqttBroker, "").length() == 0) {
    settings.setString(Is05aKeys::kMqttBroker, ARANEA_DEFAULT_MQTT_BROKER);
  }
  if (settings.getInt(Is05aKeys::kMqttPort, 0) == 0) {
    settings.setInt(Is05aKeys::kMqttPort, ARANEA_DEFAULT_MQTT_PORT);
  }

  // チャンネル設定（ch1-8）
  const int defaultPins[] = {
    IS05A_DEFAULT_CH1_PIN, IS05A_DEFAULT_CH2_PIN,
    IS05A_DEFAULT_CH3_PIN, IS05A_DEFAULT_CH4_PIN,
    IS05A_DEFAULT_CH5_PIN, IS05A_DEFAULT_CH6_PIN,
    IS05A_DEFAULT_CH7_PIN, IS05A_DEFAULT_CH8_PIN
  };

  for (int i = 0; i < 8; i++) {
    int ch = i + 1;
    String keyPin = Is05aChannelKeys::getChannelKey(ch, "pin");
    String keyName = Is05aChannelKeys::getChannelKey(ch, "name");
    String keyMean = Is05aChannelKeys::getChannelKey(ch, "mean");
    String keyDeb = Is05aChannelKeys::getChannelKey(ch, "deb");

    if (settings.getInt(keyPin.c_str(), 0) == 0) {
      settings.setInt(keyPin.c_str(), defaultPins[i]);
    }
    if (settings.getString(keyName.c_str(), "").length() == 0) {
      settings.setString(keyName.c_str(), "ch" + String(ch));
    }
    if (settings.getString(keyMean.c_str(), "").length() == 0) {
      settings.setString(keyMean.c_str(), "open");
    }
    if (settings.getInt(keyDeb.c_str(), 0) == 0) {
      settings.setInt(keyDeb.c_str(), IS05A_DEFAULT_DEBOUNCE_MS);
    }
  }

  // ch7/ch8 モード設定
  if (settings.getString(Is05aKeys::kCh7Mode, "").length() == 0) {
    settings.setString(Is05aKeys::kCh7Mode, "input");
  }
  if (settings.getString(Is05aKeys::kCh8Mode, "").length() == 0) {
    settings.setString(Is05aKeys::kCh8Mode, "input");
  }

  // ch7/ch8 出力モード設定
  if (settings.getInt(Is05aKeys::kCh7OutMode, -1) < 0) {
    settings.setInt(Is05aKeys::kCh7OutMode, IS05A_OUTPUT_MODE_MOMENTARY);
  }
  if (settings.getInt(Is05aKeys::kCh8OutMode, -1) < 0) {
    settings.setInt(Is05aKeys::kCh8OutMode, IS05A_OUTPUT_MODE_MOMENTARY);
  }
  if (settings.getInt(Is05aKeys::kCh7OutDur, 0) == 0) {
    settings.setInt(Is05aKeys::kCh7OutDur, IS05A_DEFAULT_OUTPUT_DURATION_MS);
  }
  if (settings.getInt(Is05aKeys::kCh8OutDur, 0) == 0) {
    settings.setInt(Is05aKeys::kCh8OutDur, IS05A_DEFAULT_OUTPUT_DURATION_MS);
  }
  if (settings.getInt(Is05aKeys::kCh7IntCnt, 0) == 0) {
    settings.setInt(Is05aKeys::kCh7IntCnt, IS05A_DEFAULT_INTERVAL_COUNT);
  }
  if (settings.getInt(Is05aKeys::kCh8IntCnt, 0) == 0) {
    settings.setInt(Is05aKeys::kCh8IntCnt, IS05A_DEFAULT_INTERVAL_COUNT);
  }

  // QuietMode設定
  if (!settings.getBool(Is05aKeys::kQuietOn, false)) {
    settings.setBool(Is05aKeys::kQuietOn, IS05A_DEFAULT_QUIET_ENABLED);
  }
  if (settings.getInt(Is05aKeys::kQuietStart, -1) < 0) {
    settings.setInt(Is05aKeys::kQuietStart, IS05A_DEFAULT_QUIET_START);
  }
  if (settings.getInt(Is05aKeys::kQuietEnd, -1) < 0) {
    settings.setInt(Is05aKeys::kQuietEnd, IS05A_DEFAULT_QUIET_END);
  }

  // Webhookチャンネルマスク（全チャンネル有効）
  if (settings.getInt(Is05aKeys::kWhChMask, 0) == 0) {
    settings.setInt(Is05aKeys::kWhChMask, 0xFF);  // ch1-8全て有効
  }

  // 動作設定
  if (settings.getInt(Is05aKeys::kHeartbeat, 0) == 0) {
    settings.setInt(Is05aKeys::kHeartbeat, IS05A_DEFAULT_HEARTBEAT_SEC);
  }
  if (settings.getInt(Is05aKeys::kBootGrace, 0) == 0) {
    settings.setInt(Is05aKeys::kBootGrace, IS05A_DEFAULT_BOOT_GRACE_MS);
  }
  if (settings.getInt(Is05aKeys::kReportInt, 0) == 0) {
    settings.setInt(Is05aKeys::kReportInt, IS05A_DEFAULT_REPORT_INTERVAL);
  }

  // リブートスケジューラ
  if (settings.getInt(Is05aKeys::kRebootHour, -2) == -2) {
    settings.setInt(Is05aKeys::kRebootHour, IS05A_DEFAULT_REBOOT_HOUR);
  }
  if (settings.getInt(Is05aKeys::kRebootMin, -1) < 0) {
    settings.setInt(Is05aKeys::kRebootMin, IS05A_DEFAULT_REBOOT_MIN);
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

int AraneaSettings::getDefaultPin(int ch) {
  const int pins[] = {
    IS05A_DEFAULT_CH1_PIN, IS05A_DEFAULT_CH2_PIN,
    IS05A_DEFAULT_CH3_PIN, IS05A_DEFAULT_CH4_PIN,
    IS05A_DEFAULT_CH5_PIN, IS05A_DEFAULT_CH6_PIN,
    IS05A_DEFAULT_CH7_PIN, IS05A_DEFAULT_CH8_PIN
  };
  if (ch >= 1 && ch <= 8) {
    return pins[ch - 1];
  }
  return 0;
}

int AraneaSettings::getDefaultDebounceMs() {
  return IS05A_DEFAULT_DEBOUNCE_MS;
}

int AraneaSettings::getDefaultHeartbeatSec() {
  return IS05A_DEFAULT_HEARTBEAT_SEC;
}

int AraneaSettings::getDefaultBootGraceMs() {
  return IS05A_DEFAULT_BOOT_GRACE_MS;
}

int AraneaSettings::getDefaultOutputMode() {
  return IS05A_OUTPUT_MODE_MOMENTARY;
}

int AraneaSettings::getDefaultOutputDurationMs() {
  return IS05A_DEFAULT_OUTPUT_DURATION_MS;
}

int AraneaSettings::getDefaultIntervalCount() {
  return IS05A_DEFAULT_INTERVAL_COUNT;
}

int AraneaSettings::getDefaultQuietStart() {
  return IS05A_DEFAULT_QUIET_START;
}

int AraneaSettings::getDefaultQuietEnd() {
  return IS05A_DEFAULT_QUIET_END;
}

bool AraneaSettings::getDefaultQuietEnabled() {
  return IS05A_DEFAULT_QUIET_ENABLED;
}

int AraneaSettings::getReportIntervalSec() {
  return IS05A_DEFAULT_REPORT_INTERVAL;
}

int AraneaSettings::getRebootHour() {
  return IS05A_DEFAULT_REBOOT_HOUR;
}

int AraneaSettings::getRebootMin() {
  return IS05A_DEFAULT_REBOOT_MIN;
}
