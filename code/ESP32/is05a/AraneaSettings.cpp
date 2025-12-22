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

  // 動作設定
  if (settings.getInt(Is05aKeys::kHeartbeat, 0) == 0) {
    settings.setInt(Is05aKeys::kHeartbeat, IS05A_DEFAULT_HEARTBEAT_SEC);
  }
  if (settings.getInt(Is05aKeys::kBootGrace, 0) == 0) {
    settings.setInt(Is05aKeys::kBootGrace, IS05A_DEFAULT_BOOT_GRACE_MS);
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
