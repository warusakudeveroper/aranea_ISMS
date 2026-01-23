/**
 * AraneaSettings.cpp (is06s用)
 */

#include "AraneaSettings.h"
#include "SettingManager.h"

bool AraneaSettings::_initialized = false;

void AraneaSettings::init() {
    if (_initialized) return;
    _initialized = true;
    Serial.println("[AraneaSettings] Initialized (is06s)");
}

void AraneaSettings::initDefaults(SettingManager& settings) {
    Serial.println("[AraneaSettings] Checking defaults...");

    // WiFi設定がなければデフォルトを設定
    if (settings.getString("wifi_ssid1", "").length() == 0) {
        settings.setString("wifi_ssid1", ARANEA_DEFAULT_WIFI_SSID_1);
        settings.setString("wifi_pass1", ARANEA_DEFAULT_WIFI_PASS_1);
        settings.setString("wifi_ssid2", ARANEA_DEFAULT_WIFI_SSID_2);
        settings.setString("wifi_pass2", ARANEA_DEFAULT_WIFI_PASS_2);
        settings.setString("wifi_ssid3", ARANEA_DEFAULT_WIFI_SSID_3);
        settings.setString("wifi_pass3", ARANEA_DEFAULT_WIFI_PASS_3);
        Serial.printf("[AraneaSettings] WiFi defaults applied: %s\n", ARANEA_DEFAULT_WIFI_SSID_1);
    } else {
        Serial.printf("[AraneaSettings] WiFi already configured: %s\n",
            settings.getString("wifi_ssid1", "?").c_str());
    }

    // テナント設定がなければデフォルトを設定
    if (settings.getString("tid", "").length() == 0) {
        settings.setString("tid", ARANEA_DEFAULT_TID);
        settings.setString("fid", ARANEA_DEFAULT_FID);
        settings.setString("tenant_lacis", ARANEA_DEFAULT_TENANT_LACISID);
        settings.setString("tenant_email", ARANEA_DEFAULT_TENANT_EMAIL);
        settings.setString("tenant_cic", ARANEA_DEFAULT_TENANT_CIC);
        Serial.printf("[AraneaSettings] Tenant defaults applied: TID=%s, FID=%s\n",
            ARANEA_DEFAULT_TID, ARANEA_DEFAULT_FID);
    } else {
        Serial.printf("[AraneaSettings] Tenant already configured: TID=%s\n",
            settings.getString("tid", "?").c_str());
    }

    // エンドポイント設定
    if (settings.getString("gate_url", "").length() == 0) {
        settings.setString("gate_url", ARANEA_DEFAULT_GATE_URL);
        settings.setString("cloud_url", ARANEA_DEFAULT_CLOUD_URL);
        settings.setString("relay_pri", ARANEA_DEFAULT_RELAY_PRIMARY);
        settings.setString("relay_sec", ARANEA_DEFAULT_RELAY_SECONDARY);
        Serial.println("[AraneaSettings] Endpoint defaults applied");
    }

    // 動作設定
    if (settings.getInt("heartbeat", 0) == 0) {
        settings.setInt("heartbeat", IS06S_DEFAULT_HEARTBEAT_SEC);
        settings.setInt("boot_grace", IS06S_DEFAULT_BOOT_GRACE_MS);
        Serial.println("[AraneaSettings] Operation defaults applied");
    }

    Serial.println("[AraneaSettings] Defaults check complete");
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

// 動作設定
int AraneaSettings::getDefaultHeartbeatSec() { return IS06S_DEFAULT_HEARTBEAT_SEC; }
int AraneaSettings::getDefaultBootGraceMs() { return IS06S_DEFAULT_BOOT_GRACE_MS; }
