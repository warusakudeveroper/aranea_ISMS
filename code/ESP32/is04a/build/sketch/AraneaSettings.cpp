#line 1 "/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/aranea_ISMS/code/ESP32/is04a/AraneaSettings.cpp"
/**
 * AraneaSettings.cpp (is04a用)
 */

#include "AraneaSettings.h"
#include "SettingManager.h"
#include "Is04aKeys.h"

bool AraneaSettings::_initialized = false;

void AraneaSettings::init() {
    if (_initialized) return;
    _initialized = true;
    Serial.println("[AraneaSettings] Initialized (is04a)");
}

void AraneaSettings::initDefaults(SettingManager& settings) {
    // WiFi設定がなければデフォルトを設定
    if (settings.getString("wifi_ssid1", "").length() == 0) {
        settings.setString("wifi_ssid1", ARANEA_DEFAULT_WIFI_SSID_1);
        settings.setString("wifi_ssid2", ARANEA_DEFAULT_WIFI_SSID_2);
        settings.setString("wifi_ssid3", ARANEA_DEFAULT_WIFI_SSID_3);
        settings.setString("wifi_pass", ARANEA_DEFAULT_WIFI_PASS);
        Serial.println("[AraneaSettings] WiFi defaults applied");
    }

    // テナント設定がなければデフォルトを設定
    if (settings.getString("tid", "").length() == 0) {
        settings.setString("tid", ARANEA_DEFAULT_TID);
        settings.setString("fid", ARANEA_DEFAULT_FID);
        settings.setString("tenant_lacis", ARANEA_DEFAULT_TENANT_LACISID);
        settings.setString("tenant_email", ARANEA_DEFAULT_TENANT_EMAIL);
        settings.setString("tenant_cic", ARANEA_DEFAULT_TENANT_CIC);
        Serial.println("[AraneaSettings] Tenant defaults applied");
    }

    // エンドポイント設定
    if (settings.getString("gate_url", "").length() == 0) {
        settings.setString("gate_url", ARANEA_DEFAULT_GATE_URL);
        settings.setString("cloud_url", ARANEA_DEFAULT_CLOUD_URL);
        settings.setString("relay_pri", ARANEA_DEFAULT_RELAY_PRIMARY);
        settings.setString("relay_sec", ARANEA_DEFAULT_RELAY_SECONDARY);
        Serial.println("[AraneaSettings] Endpoint defaults applied");
    }

    // is04a固有設定
    if (settings.getInt(Is04aKeys::kPulseMs, 0) == 0) {
        settings.setInt(Is04aKeys::kPulseMs, IS04A_DEFAULT_PULSE_MS);
        settings.setInt(Is04aKeys::kInterlockMs, IS04A_DEFAULT_INTERLOCK_MS);
        settings.setInt(Is04aKeys::kDebounceMs, IS04A_DEFAULT_DEBOUNCE_MS);
        settings.setInt(Is04aKeys::kIn1Target, IS04A_DEFAULT_IN1_TARGET);
        settings.setInt(Is04aKeys::kIn2Target, IS04A_DEFAULT_IN2_TARGET);
        settings.setString(Is04aKeys::kOut1Name, IS04A_DEFAULT_OUT1_NAME);
        settings.setString(Is04aKeys::kOut2Name, IS04A_DEFAULT_OUT2_NAME);
        settings.setInt(Is04aKeys::kHeartbeat, IS04A_DEFAULT_HEARTBEAT_SEC);
        settings.setInt(Is04aKeys::kBootGrace, IS04A_DEFAULT_BOOT_GRACE_MS);
        Serial.println("[AraneaSettings] is04a defaults applied");
    }
}

// テナント設定
String AraneaSettings::getTid() { return ARANEA_DEFAULT_TID; }
String AraneaSettings::getFid() { return ARANEA_DEFAULT_FID; }
String AraneaSettings::getTenantLacisId() { return ARANEA_DEFAULT_TENANT_LACISID; }
String AraneaSettings::getTenantEmail() { return ARANEA_DEFAULT_TENANT_EMAIL; }
String AraneaSettings::getTenantCic() { return ARANEA_DEFAULT_TENANT_CIC; }

// エンドポイント
String AraneaSettings::getGateUrl() { return ARANEA_DEFAULT_GATE_URL; }
String AraneaSettings::getCloudUrl() { return ARANEA_DEFAULT_CLOUD_URL; }
String AraneaSettings::getRelayPrimary() { return ARANEA_DEFAULT_RELAY_PRIMARY; }
String AraneaSettings::getRelaySecondary() { return ARANEA_DEFAULT_RELAY_SECONDARY; }

// GPIO設定
int AraneaSettings::getDefaultIn1Pin() { return IS04A_DEFAULT_IN1_PIN; }
int AraneaSettings::getDefaultIn2Pin() { return IS04A_DEFAULT_IN2_PIN; }
int AraneaSettings::getDefaultOut1Pin() { return IS04A_DEFAULT_OUT1_PIN; }
int AraneaSettings::getDefaultOut2Pin() { return IS04A_DEFAULT_OUT2_PIN; }

// パルス設定
int AraneaSettings::getDefaultPulseMs() { return IS04A_DEFAULT_PULSE_MS; }
int AraneaSettings::getDefaultInterlockMs() { return IS04A_DEFAULT_INTERLOCK_MS; }
int AraneaSettings::getDefaultDebounceMs() { return IS04A_DEFAULT_DEBOUNCE_MS; }

// トリガーアサイン
int AraneaSettings::getDefaultIn1Target() { return IS04A_DEFAULT_IN1_TARGET; }
int AraneaSettings::getDefaultIn2Target() { return IS04A_DEFAULT_IN2_TARGET; }

// 出力名称
String AraneaSettings::getDefaultOut1Name() { return IS04A_DEFAULT_OUT1_NAME; }
String AraneaSettings::getDefaultOut2Name() { return IS04A_DEFAULT_OUT2_NAME; }

// 動作設定
int AraneaSettings::getDefaultHeartbeatSec() { return IS04A_DEFAULT_HEARTBEAT_SEC; }
int AraneaSettings::getDefaultBootGraceMs() { return IS04A_DEFAULT_BOOT_GRACE_MS; }
