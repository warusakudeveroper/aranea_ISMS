/**
 * is10t - Tapo Camera Monitor (ar-is10t)
 *
 * TP-Link TapoカメラをONVIF経由で監視し、
 * CelestialGlobeへ状態を送信するデバイス。
 *
 * 機能:
 * - WiFi接続（STA）/ APモードフォールバック
 * - AraneaGateway登録（CIC取得）
 * - 複数Tapoカメラへの ONVIF接続・情報取得
 * - カメラ情報をCelestialGlobeへPOST
 * - HTTP設定UI
 * - OTA更新
 *
 * 取得情報:
 * - MAC / IP / ゲートウェイ / DHCP
 * - モデル / ファームウェア / シリアル番号
 * - オンライン/オフライン状態
 *
 * ※SSID/RSSIはis10m（Mercury AC）データとMAC照合で取得
 */

#include <Arduino.h>
#include <Wire.h>
#include <WiFi.h>

// Reset reason & RTC memory for crash debugging
#include "esp_system.h"
#include "rom/rtc.h"

// Brownout control
#include "soc/soc.h"
#include "soc/rtc_cntl_reg.h"

#include <HTTPClient.h>
#include <esp_mac.h>
#include <SPIFFS.h>
#include <ArduinoJson.h>
#include <esp_heap_caps.h>

// ========================================
// RTC memory for crash position tracking
// ========================================
RTC_DATA_ATTR uint32_t bootCount = 0;
RTC_DATA_ATTR uint32_t lastStage = 0;

// Stage definitions for crash debugging
enum DebugStage : uint32_t {
    STAGE_BOOT_START = 1,
    STAGE_SERIAL_INIT = 2,
    STAGE_GPIO_INIT = 3,
    STAGE_SPIFFS_INIT = 4,
    STAGE_SETTINGS_INIT = 5,
    STAGE_DISPLAY_INIT = 6,
    STAGE_LACIS_GEN = 7,
    STAGE_WIFI_START = 8,
    STAGE_WIFI_DONE = 9,
    STAGE_NTP_SYNC = 10,
    STAGE_ARANEA_REG = 11,
    STAGE_CONFIG_LOAD = 12,
    STAGE_HTTP_MGR_INIT = 13,
    STAGE_POLLER_BEGIN = 14,
    STAGE_POLLER_DONE = 15,
    STAGE_SETUP_COMPLETE = 20,
    STAGE_LOOP_RUNNING = 100
};

static const char* resetReasonStr(esp_reset_reason_t r) {
    switch (r) {
        case ESP_RST_POWERON:  return "ESP_RST_POWERON";
        case ESP_RST_SW:       return "ESP_RST_SW";
        case ESP_RST_PANIC:    return "ESP_RST_PANIC";
        case ESP_RST_INT_WDT:  return "ESP_RST_INT_WDT";
        case ESP_RST_TASK_WDT: return "ESP_RST_TASK_WDT";
        case ESP_RST_WDT:      return "ESP_RST_WDT";
        case ESP_RST_BROWNOUT: return "ESP_RST_BROWNOUT";
        case ESP_RST_SDIO:     return "ESP_RST_SDIO";
        default:               return "ESP_RST_UNKNOWN";
    }
}

static const char* stageStr(uint32_t stage) {
    switch (stage) {
        case STAGE_BOOT_START:     return "BOOT_START";
        case STAGE_SERIAL_INIT:    return "SERIAL_INIT";
        case STAGE_GPIO_INIT:      return "GPIO_INIT";
        case STAGE_SPIFFS_INIT:    return "SPIFFS_INIT";
        case STAGE_SETTINGS_INIT:  return "SETTINGS_INIT";
        case STAGE_DISPLAY_INIT:   return "DISPLAY_INIT";
        case STAGE_LACIS_GEN:      return "LACIS_GEN";
        case STAGE_WIFI_START:     return "WIFI_START";
        case STAGE_WIFI_DONE:      return "WIFI_DONE";
        case STAGE_NTP_SYNC:       return "NTP_SYNC";
        case STAGE_ARANEA_REG:     return "ARANEA_REG";
        case STAGE_CONFIG_LOAD:    return "CONFIG_LOAD";
        case STAGE_HTTP_MGR_INIT:  return "HTTP_MGR_INIT";
        case STAGE_POLLER_BEGIN:   return "POLLER_BEGIN";
        case STAGE_POLLER_DONE:    return "POLLER_DONE";
        case STAGE_SETUP_COMPLETE: return "SETUP_COMPLETE";
        case STAGE_LOOP_RUNNING:   return "LOOP_RUNNING";
        default:                   return "UNKNOWN";
    }
}

// ============================================================
// Araneaモジュール一括インポート
// ============================================================
#include "AraneaGlobalImporter.h"

// ========================================
// デバイス定数
// ========================================
static const char* PRODUCT_TYPE = "010";
static const char* PRODUCT_CODE = "0002";  // is10t = 0002

// デバイス識別子
static const char* ARANEA_DEVICE_TYPE = "aranea_ar-is10t";
static const char* CELESTIAL_SOURCE = "ar-is10t";
static const char* DEVICE_SHORT_NAME = "ar-is10t";
static const char* DEVICE_MODEL = "Tapo_CameraMonitor";
static const char* FIRMWARE_VERSION = "1.0.0";

// ========================================
// ピン定義
// ========================================
static const int I2C_SDA = 21;
static const int I2C_SCL = 22;
static const int BTN_WIFI = 25;   // WiFi再接続ボタン
static const int BTN_RESET = 26;  // ファクトリーリセットボタン

// ========================================
// タイミング定数
// ========================================
static const unsigned long CELESTIAL_SEND_INTERVAL_MS = 300000;  // 5分
static const unsigned long DISPLAY_UPDATE_MS = 1000;
static const unsigned long BUTTON_HOLD_MS = 3000;
static const unsigned long AP_MODE_TIMEOUT_MS = 300000;

// ========================================
// グローバル変数
// ========================================
// モジュールインスタンス
SettingManager settings;
DisplayManager display;
WiFiManager wifi;
NtpManager ntp;
LacisIDGenerator lacisGen;
AraneaRegister araneaReg;
HttpManagerIs10t httpMgr;
OtaManager ota;
HttpOtaManager httpOta;
Operator op;
TapoPollerIs10t tapoPoller;
CelestialSenderIs10t celestialSender;

// 自機情報
String myLacisId;
String myMac;
String myCic;
String myTid;
String myFid;
String myHostname;

// 状態
bool apModeActive = false;
unsigned long apModeStartTime = 0;
unsigned long lastDisplayUpdate = 0;
unsigned long btnWifiPressTime = 0;
unsigned long btnResetPressTime = 0;
unsigned long lastCelestialSend = 0;

// ========================================
// 前方宣言
// ========================================
void startAPMode();
void stopAPMode();

// ========================================
// APモードSSID生成
// ========================================
String getAPModeSSID() {
    String mac6 = myMac.substring(6);
    return String(DEVICE_SHORT_NAME) + "-" + mac6;
}

// ========================================
// ホスト名生成
// ========================================
String getHostname() {
    String mac6 = myMac.substring(6);
    return String(DEVICE_SHORT_NAME) + "-" + mac6;
}

// ========================================
// APモード開始
// ========================================
void startAPMode() {
    String apSSID = getAPModeSSID();
    Serial.printf("[AP] Starting AP mode: %s\n", apSSID.c_str());

    WiFi.mode(WIFI_AP);
    WiFi.softAP(apSSID.c_str());

    apModeActive = true;
    apModeStartTime = millis();

    display.show4Lines("AP Mode", apSSID, WiFi.softAPIP().toString(), "");

    Serial.printf("[AP] IP: %s\n", WiFi.softAPIP().toString().c_str());
}

// ========================================
// APモード終了
// ========================================
void stopAPMode() {
    if (!apModeActive) return;

    Serial.println("[AP] Stopping AP mode");
    WiFi.softAPdisconnect(true);
    WiFi.mode(WIFI_STA);
    apModeActive = false;
}

// ========================================
// GPIO初期化
// ========================================
void initGPIO() {
    pinMode(BTN_WIFI, INPUT_PULLUP);
    pinMode(BTN_RESET, INPUT_PULLUP);
    Serial.println("[GPIO] Buttons initialized");
}

// ========================================
// ボタン処理
// ========================================
void handleButtons() {
    // WiFi再接続ボタン
    if (digitalRead(BTN_WIFI) == LOW) {
        if (btnWifiPressTime == 0) {
            btnWifiPressTime = millis();
        } else if (millis() - btnWifiPressTime >= BUTTON_HOLD_MS) {
            Serial.println("[BTN] WiFi reconnect requested");
            display.showBoot("WiFi Reconnect...");

            if (apModeActive) {
                stopAPMode();
            }

            wifi.disconnect();
            delay(500);
            wifi.connectWithSettings(&settings);
            btnWifiPressTime = 0;
        }
    } else {
        btnWifiPressTime = 0;
    }

    // ファクトリーリセットボタン
    if (digitalRead(BTN_RESET) == LOW) {
        if (btnResetPressTime == 0) {
            btnResetPressTime = millis();
        } else if (millis() - btnResetPressTime >= BUTTON_HOLD_MS) {
            Serial.println("[BTN] Factory reset requested");
            display.showError("Factory Reset!");
            delay(1000);
            araneaReg.clearRegistration();
            settings.clear();
            ESP.restart();
        }
    } else {
        btnResetPressTime = 0;
    }
}

// ========================================
// ディスプレイ更新
// ========================================
void updateDisplay() {
    if (apModeActive) {
        String apSSID = getAPModeSSID();
        String apIP = WiFi.softAPIP().toString();
        display.showIs02Main(apSSID, "AP MODE", apIP, false);
        return;
    }

    const TapoPollSummary& summary = tapoPoller.getSummary();

    String line1 = wifi.getIP() + " " + String(WiFi.RSSI()) + "dBm";
    String cicStr = myCic.length() > 0 ? myCic : "------";
    String statusLine = "Cam:" + String(summary.total) +
                        " ON:" + String(summary.online) +
                        " NG:" + String(summary.offline);

    display.showIs02Main(line1, cicStr, statusLine, false);
}

// ========================================
// Setup
// ========================================
void setup() {
    bootCount++;
    lastStage = STAGE_BOOT_START;

    Serial.begin(115200);
    delay(50);
    lastStage = STAGE_SERIAL_INIT;

    // リセット理由出力
    esp_reset_reason_t resetReason = esp_reset_reason();
    Serial.println("\n========================================");
    Serial.println("[CRASH DEBUG] Reset Reason Analysis");
    Serial.println("========================================");
    Serial.printf("[BOOT] bootCount=%u\n", bootCount);
    Serial.printf("[BOOT] esp_reset_reason=%d (%s)\n", (int)resetReason, resetReasonStr(resetReason));
    Serial.printf("[BOOT] lastStage(previous)=%u (%s)\n", lastStage, stageStr(lastStage));
    Serial.printf("[BOOT] heap=%u, largest=%u\n",
        ESP.getFreeHeap(), heap_caps_get_largest_free_block(MALLOC_CAP_8BIT));
    Serial.println("========================================");

    Serial.println("\n[BOOT] is10t (ar-is10t) starting...");
    Serial.printf("[BOOT] Model: %s, Version: %s\n", DEVICE_MODEL, FIRMWARE_VERSION);

    // GPIO初期化
    lastStage = STAGE_GPIO_INIT;
    initGPIO();

    // Operator初期化
    op.setMode(OperatorMode::PROVISION);

    // SPIFFS初期化
    lastStage = STAGE_SPIFFS_INIT;
    Serial.printf("[STAGE] %s\n", stageStr(lastStage));
    if (!SPIFFS.begin(true)) {
        Serial.println("[ERROR] SPIFFS init failed");
    }

    // SettingManager初期化（NVS）
    lastStage = STAGE_SETTINGS_INIT;
    Serial.printf("[STAGE] %s\n", stageStr(lastStage));
    if (!settings.begin("isms10t")) {
        Serial.println("[ERROR] Settings init failed");
        return;
    }

    // DisplayManager初期化
    lastStage = STAGE_DISPLAY_INIT;
    Serial.printf("[STAGE] %s\n", stageStr(lastStage));
    if (!display.begin()) {
        Serial.println("[WARNING] Display init failed");
    }
    display.showBoot("Booting...");

    // LacisID生成
    lastStage = STAGE_LACIS_GEN;
    Serial.printf("[STAGE] %s\n", stageStr(lastStage));
    myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
    myLacisId.toUpperCase();
    myMac = lacisGen.getStaMac12Hex();
    myMac.toUpperCase();
    myHostname = getHostname();
    Serial.printf("[LACIS] ID: %s, MAC: %s\n", myLacisId.c_str(), myMac.c_str());

    // WiFi接続試行
    lastStage = STAGE_WIFI_START;
    Serial.printf("[STAGE] %s\n", stageStr(lastStage));
    display.showConnecting("WiFi...", 0);
    wifi.setHostname(myHostname);
    if (!wifi.connectWithSettings(&settings)) {
        Serial.println("[WIFI] Connection failed, starting AP mode");
        startAPMode();
    } else {
        lastStage = STAGE_WIFI_DONE;
        Serial.printf("[STAGE] %s\n", stageStr(lastStage));
        Serial.printf("[WIFI] Connected: %s\n", wifi.getIP().c_str());

        // NTP同期（ONVIF認証に必須）
        lastStage = STAGE_NTP_SYNC;
        Serial.printf("[STAGE] %s\n", stageStr(lastStage));
        display.showBoot("NTP sync...");
        if (!ntp.sync()) {
            Serial.println("[WARNING] NTP sync failed - ONVIF auth may fail");
        }

        // AraneaGateway登録
        lastStage = STAGE_ARANEA_REG;
        Serial.printf("[STAGE] %s\n", stageStr(lastStage));
        display.showRegistering("Registering...");

        myTid = settings.getString("tid", "");
        myFid = settings.getString("fid", "");

        String gateUrl = settings.getString(CommonKeys::kGateUrl, "");
        if (gateUrl.length() > 0) {
            araneaReg.begin(gateUrl);

            AraneaRegisterResult regResult = araneaReg.registerDevice(
                myTid, ARANEA_DEVICE_TYPE, myLacisId, myMac, PRODUCT_TYPE, PRODUCT_CODE
            );

            if (regResult.ok) {
                myCic = regResult.cic_code;
                settings.setString("cic", myCic);
                Serial.printf("[REG] CIC: %s\n", myCic.c_str());
            } else {
                myCic = settings.getString("cic", "");
                if (myCic.length() > 0) {
                    Serial.printf("[REG] Failed, using saved CIC: %s\n", myCic.c_str());
                } else {
                    Serial.println("[REG] WARNING: No CIC available");
                }
            }
        } else {
            myCic = settings.getString("cic", "");
            Serial.println("[REG] No gate_url configured");
        }
    }

    // TapoPoller初期化
    lastStage = STAGE_POLLER_BEGIN;
    Serial.printf("[STAGE] %s\n", stageStr(lastStage));
    tapoPoller.begin(&settings);

    // HttpManager開始
    lastStage = STAGE_HTTP_MGR_INIT;
    Serial.printf("[STAGE] %s\n", stageStr(lastStage));
    httpMgr.begin(&settings, &tapoPoller, 80);

    // デバイス情報設定
    AraneaDeviceInfo devInfo;
    devInfo.deviceType = DEVICE_SHORT_NAME;
    devInfo.modelName = "Tapo Camera Monitor";
    devInfo.contextDesc = "Tapoカメラの監視とCelestialGlobeへの状態送信";
    devInfo.lacisId = myLacisId;
    devInfo.cic = myCic;
    devInfo.mac = myMac;
    devInfo.hostname = myHostname;
    devInfo.firmwareVersion = FIRMWARE_VERSION;
    devInfo.buildDate = __DATE__ " " __TIME__;
    devInfo.modules = "WiFi,NTP,ONVIF,HTTP";
    httpMgr.setDeviceInfo(devInfo);

    // CelestialSender初期化
    celestialSender.begin(&settings, &ntp, &tapoPoller);
    celestialSender.setAuth(myTid, myLacisId, myCic, myFid);
    celestialSender.setSource(CELESTIAL_SOURCE);

    // HTTP OTA開始
    httpOta.begin(httpMgr.getServer());
    httpOta.onStart([]() {
        op.setOtaUpdating(true);
        display.showBoot("HTTP OTA Start...");
    });
    httpOta.onProgress([](int progress) {
        display.showBoot("HTTP OTA: " + String(progress) + "%");
    });
    httpOta.onError([](const String& err) {
        op.setOtaUpdating(false);
        display.showError("HTTP OTA: " + err);
    });
    Serial.println("[HTTP-OTA] Enabled");

    // RUNモードへ
    op.setMode(OperatorMode::RUN);

    lastStage = STAGE_SETUP_COMPLETE;
    Serial.printf("[STAGE] %s\n", stageStr(lastStage));
    Serial.println("[BOOT] is10t ready!");
}

// ========================================
// Loop
// ========================================
void loop() {
    static unsigned long loopCount = 0;
    static unsigned long lastLoopLog = 0;
    unsigned long now = millis();

    loopCount++;

    if (loopCount == 1) {
        lastStage = STAGE_LOOP_RUNNING;
    }

    // ステータス出力（30秒毎）
    if (now - lastLoopLog >= 30000) {
        Serial.printf("[STATUS] heap=%d largest=%u\n",
            ESP.getFreeHeap(),
            heap_caps_get_largest_free_block(MALLOC_CAP_8BIT));
        lastLoopLog = now;
    }

    // OTA処理チェック
    if (op.isOtaUpdating()) {
        delay(10);
        return;
    }

    // HTTP処理
    httpMgr.handle();

    // APモードタイムアウトチェック
    if (apModeActive && (now - apModeStartTime >= AP_MODE_TIMEOUT_MS)) {
        Serial.println("[AP] Timeout, attempting STA reconnect");
        stopAPMode();
        wifi.connectWithSettings(&settings);
        if (!wifi.isConnected()) {
            startAPMode();
        }
    }

    // Tapoポーリング処理
    if (!apModeActive) {
        // ポーリングステータスをHttpManagerに通知
        httpMgr.setPollingStatus(tapoPoller.isPolling(), tapoPoller.getCurrentIndex());

        // ポーリング実行
        bool cycleComplete = tapoPoller.handle();

        // サイクル完了時にCelestialGlobe送信
        if (cycleComplete && celestialSender.isConfigured()) {
            // メモリ回復待機後に送信
            if (heap_caps_get_largest_free_block(MALLOC_CAP_8BIT) >= 8000) {
                celestialSender.send();
                lastCelestialSend = now;
            } else {
                Serial.println("[LOOP] Skip CelestialGlobe - low memory");
            }
        }
    }

    // ディスプレイ更新（毎秒）
    if (now - lastDisplayUpdate >= DISPLAY_UPDATE_MS) {
        updateDisplay();
        lastDisplayUpdate = now;
    }

    // ボタン処理
    handleButtons();

    // WiFi再接続チェック
    if (!apModeActive && !wifi.isConnected()) {
        Serial.println("[WIFI] Disconnected, reconnecting...");
        display.showConnecting("Reconnect...", 0);

        if (!wifi.connectWithSettings(&settings)) {
            startAPMode();
        }
    }

    yield();
    delay(10);
}
