/**
 * AraneaSettings.cpp (is06a用)
 */

#include "AraneaSettings.h"
#include "SettingManager.h"
#include "Is06aKeys.h"

bool AraneaSettings::_initialized = false;

void AraneaSettings::init() {
    if (_initialized) return;
    _initialized = true;
    Serial.println("[AraneaSettings] Initialized (is06a)");
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

    // is06a固有設定: インターロック
    if (settings.getInt(Is06aKeys::kInterlockMs, 0) == 0) {
        settings.setInt(Is06aKeys::kInterlockMs, IS06A_DEFAULT_INTERLOCK_MS);
        settings.setInt(Is06aKeys::kHeartbeat, IS06A_DEFAULT_HEARTBEAT_SEC);
        settings.setInt(Is06aKeys::kBootGrace, IS06A_DEFAULT_BOOT_GRACE_MS);
    }

    // TRG1設定
    if (settings.getString(Is06aKeys::kTrg1Name, "").length() == 0) {
        settings.setString(Is06aKeys::kTrg1Name, IS06A_DEFAULT_TRG1_NAME);
        settings.setString(Is06aKeys::kTrg1Mode, IS06A_DEFAULT_TRG_MODE);
        settings.setInt(Is06aKeys::kTrg1PulseMs, IS06A_DEFAULT_PULSE_MS);
        settings.setInt(Is06aKeys::kTrg1PwmFreq, IS06A_DEFAULT_PWM_FREQ);
        settings.setInt(Is06aKeys::kTrg1PwmDuty, IS06A_DEFAULT_PWM_DUTY);
    }

    // TRG2設定
    if (settings.getString(Is06aKeys::kTrg2Name, "").length() == 0) {
        settings.setString(Is06aKeys::kTrg2Name, IS06A_DEFAULT_TRG2_NAME);
        settings.setString(Is06aKeys::kTrg2Mode, IS06A_DEFAULT_TRG_MODE);
        settings.setInt(Is06aKeys::kTrg2PulseMs, IS06A_DEFAULT_PULSE_MS);
        settings.setInt(Is06aKeys::kTrg2PwmFreq, IS06A_DEFAULT_PWM_FREQ);
        settings.setInt(Is06aKeys::kTrg2PwmDuty, IS06A_DEFAULT_PWM_DUTY);
    }

    // TRG3設定
    if (settings.getString(Is06aKeys::kTrg3Name, "").length() == 0) {
        settings.setString(Is06aKeys::kTrg3Name, IS06A_DEFAULT_TRG3_NAME);
        settings.setString(Is06aKeys::kTrg3Mode, IS06A_DEFAULT_TRG_MODE);
        settings.setInt(Is06aKeys::kTrg3PulseMs, IS06A_DEFAULT_PULSE_MS);
        settings.setInt(Is06aKeys::kTrg3PwmFreq, IS06A_DEFAULT_PWM_FREQ);
        settings.setInt(Is06aKeys::kTrg3PwmDuty, IS06A_DEFAULT_PWM_DUTY);
    }

    // TRG4設定（PWM対応）
    if (settings.getString(Is06aKeys::kTrg4Name, "").length() == 0) {
        settings.setString(Is06aKeys::kTrg4Name, IS06A_DEFAULT_TRG4_NAME);
        settings.setString(Is06aKeys::kTrg4Mode, IS06A_DEFAULT_TRG_MODE);
        settings.setInt(Is06aKeys::kTrg4PulseMs, IS06A_DEFAULT_PULSE_MS);
        settings.setInt(Is06aKeys::kTrg4PwmFreq, IS06A_DEFAULT_PWM_FREQ);
        settings.setInt(Is06aKeys::kTrg4PwmDuty, IS06A_DEFAULT_PWM_DUTY);
    }

    // TRG5設定（I/O切替可能）
    if (settings.getString(Is06aKeys::kTrg5Name, "").length() == 0) {
        settings.setString(Is06aKeys::kTrg5Name, IS06A_DEFAULT_TRG5_NAME);
        settings.setString(Is06aKeys::kTrg5IoMode, IS06A_DEFAULT_IO_MODE);
        settings.setInt(Is06aKeys::kTrg5PulseMs, IS06A_DEFAULT_PULSE_MS);
        settings.setInt(Is06aKeys::kTrg5Debounce, IS06A_DEFAULT_DEBOUNCE_MS);
    }

    // TRG6設定（I/O切替可能）
    if (settings.getString(Is06aKeys::kTrg6Name, "").length() == 0) {
        settings.setString(Is06aKeys::kTrg6Name, IS06A_DEFAULT_TRG6_NAME);
        settings.setString(Is06aKeys::kTrg6IoMode, IS06A_DEFAULT_IO_MODE);
        settings.setInt(Is06aKeys::kTrg6PulseMs, IS06A_DEFAULT_PULSE_MS);
        settings.setInt(Is06aKeys::kTrg6Debounce, IS06A_DEFAULT_DEBOUNCE_MS);
    }

    Serial.println("[AraneaSettings] is06a defaults applied");
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

// GPIO設定（6CH）
int AraneaSettings::getGpioTrg(int trgNum) {
    switch (trgNum) {
        case 1: return IS06A_GPIO_TRG1;
        case 2: return IS06A_GPIO_TRG2;
        case 3: return IS06A_GPIO_TRG3;
        case 4: return IS06A_GPIO_TRG4;
        case 5: return IS06A_GPIO_TRG5;
        case 6: return IS06A_GPIO_TRG6;
        default: return -1;
    }
}

// トリガー共通設定
int AraneaSettings::getDefaultInterlockMs() { return IS06A_DEFAULT_INTERLOCK_MS; }

// TRG1-4設定（PWM対応）
String AraneaSettings::getDefaultTrgMode() { return IS06A_DEFAULT_TRG_MODE; }
int AraneaSettings::getDefaultPulseMs() { return IS06A_DEFAULT_PULSE_MS; }
int AraneaSettings::getDefaultPwmFreq() { return IS06A_DEFAULT_PWM_FREQ; }
int AraneaSettings::getDefaultPwmDuty() { return IS06A_DEFAULT_PWM_DUTY; }

// TRG5-6設定（I/O切替）
String AraneaSettings::getDefaultIoMode() { return IS06A_DEFAULT_IO_MODE; }
int AraneaSettings::getDefaultDebounceMs() { return IS06A_DEFAULT_DEBOUNCE_MS; }

// トリガー名称（6CH）
String AraneaSettings::getDefaultTrgName(int trgNum) {
    switch (trgNum) {
        case 1: return IS06A_DEFAULT_TRG1_NAME;
        case 2: return IS06A_DEFAULT_TRG2_NAME;
        case 3: return IS06A_DEFAULT_TRG3_NAME;
        case 4: return IS06A_DEFAULT_TRG4_NAME;
        case 5: return IS06A_DEFAULT_TRG5_NAME;
        case 6: return IS06A_DEFAULT_TRG6_NAME;
        default: return "TRG?";
    }
}

// 動作設定
int AraneaSettings::getDefaultHeartbeatSec() { return IS06A_DEFAULT_HEARTBEAT_SEC; }
int AraneaSettings::getDefaultBootGraceMs() { return IS06A_DEFAULT_BOOT_GRACE_MS; }
