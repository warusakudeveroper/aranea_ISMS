/**
 * is06a - 6-Channel Relay Controller (ar-is06a)
 *
 * 6chトリガー出力デバイス。
 * TRG1-4: PWM対応、TRG5-6: I/O切替可能。
 * HTTP経由でパルス/PWM出力を制御。
 * 状態変化をローカルリレー/クラウドへ通知。
 *
 * 機能:
 * - TRG1-4 (GPIO 12,14,27,15): デジタル出力 + PWM対応
 * - TRG5-6 (GPIO 05,18): I/O切替可能（出力 or 入力）
 * - インターロック（連続パルス防止）
 * - Web UI設定
 * - OTA更新
 *
 * 注意（ストラップピン）:
 * - GPIO12(MTDI): ブート時フラッシュ電圧選択。内部プルダウン設定推奨
 * - GPIO15(MTDO): ブート時ログ制御。内部プルアップ設定推奨
 * - GPIO5: ブート時LOGレベル制御。通常は問題なし
 *
 * 設計原則:
 * - シングルタスク設計（loop()ベース）
 * - セマフォ/WDT不使用
 * - min_SPIFFSパーティション使用
 */

#include <Arduino.h>
#include <Wire.h>
#include <WiFi.h>
#include <HTTPClient.h>
#include <esp_mac.h>

// ============================================================
// Araneaモジュール一括インポート
// ============================================================
#include "AraneaGlobalImporter.h"

// ========================================
// デバイス定数
// ========================================
static const char* PRODUCT_TYPE = "006";
static const char* PRODUCT_CODE = "0001";
static const char* ARANEA_DEVICE_TYPE = "aranea_ar-is06a";
static const char* DEVICE_SHORT_NAME = "ar-is06a";
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
static const unsigned long DISPLAY_UPDATE_MS = 1000;
static const unsigned long DISPLAY_ACTION_MS = 100;   // アクション中は高速更新
static const unsigned long BUTTON_HOLD_MS = 3000;
static const unsigned long AP_MODE_TIMEOUT_MS = 300000;  // 5分

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
OtaManager ota;
HttpOtaManager httpOta;
Operator op;

// is06a固有モジュール
TriggerManagerIs06a trigger;
HttpManagerIs06a httpMgr;
StateReporterIs06a stateReporter;

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
unsigned long lastHeartbeatTime = 0;
unsigned long bootTime = 0;
int heartbeatIntervalSec = 60;
int bootGraceMs = 3000;

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

    String line1 = wifi.getIP() + " " + String(WiFi.RSSI()) + "dBm";
    String cicStr = myCic.length() > 0 ? myCic : "------";

    // 出力状態表示（アクティブなトリガーを表示）
    String sensorLine;
    bool anyActive = false;
    for (int i = 1; i <= 6; i++) {
        if (trigger.isPulseActive(i)) {
            sensorLine = "TRG" + String(i) + ">>PULSE";
            anyActive = true;
            break;
        }
        auto state = trigger.getState(i);
        if (state.mode == TriggerManagerIs06a::TriggerMode::MODE_PWM && state.active) {
            sensorLine = "TRG" + String(i) + " PWM:" + String(state.pwmDuty);
            anyActive = true;
            break;
        }
    }

    if (!anyActive) {
        // 電波レベル表示
        int rssi = WiFi.RSSI();
        sensorLine = "@ ";
        if (rssi > -50) sensorLine += "[||||]";
        else if (rssi > -60) sensorLine += "[||| ]";
        else if (rssi > -70) sensorLine += "[||  ]";
        else if (rssi > -80) sensorLine += "[|   ]";
        else sensorLine += "[    ]";
        sensorLine += " " + String(rssi) + "dBm";
    }

    display.showIs02Main(line1, cicStr, sensorLine, anyActive);
}

// ========================================
// パルス開始ハンドラ
// ========================================
void onPulseStart(int trgNum, TriggerManagerIs06a::PulseSource source) {
    Serial.printf("[PULSE] Started: TRG%d, source=%d\n", trgNum, (int)source);

    // 起動猶予期間チェック
    if (bootTime > 0 && (millis() - bootTime) < (unsigned long)bootGraceMs) {
        Serial.println("[PULSE] Boot grace period - skipping send");
        return;
    }

    stateReporter.sendStateReport(true);
}

// ========================================
// パルス終了ハンドラ
// ========================================
void onPulseEnd(int trgNum) {
    Serial.printf("[PULSE] Ended: TRG%d\n", trgNum);

    // 起動猶予期間チェック
    if (bootTime > 0 && (millis() - bootTime) < (unsigned long)bootGraceMs) {
        Serial.println("[PULSE] Boot grace period - skipping send");
        return;
    }

    stateReporter.sendStateReport(true);
}

// ========================================
// 入力変化ハンドラ（TRG5-6がINPUTモード時）
// ========================================
void onInputChange(int trgNum, bool active) {
    Serial.printf("[INPUT] TRG%d changed to %s\n", trgNum, active ? "ACTIVE" : "INACTIVE");

    // 起動猶予期間チェック
    if (bootTime > 0 && (millis() - bootTime) < (unsigned long)bootGraceMs) {
        Serial.println("[INPUT] Boot grace period - skipping send");
        return;
    }

    stateReporter.sendStateReport();
}

// ========================================
// PWM変化ハンドラ
// ========================================
void onPwmChange(int trgNum, int duty) {
    Serial.printf("[PWM] TRG%d duty changed to %d\n", trgNum, duty);

    // 起動猶予期間チェック
    if (bootTime > 0 && (millis() - bootTime) < (unsigned long)bootGraceMs) {
        Serial.println("[PWM] Boot grace period - skipping send");
        return;
    }

    stateReporter.sendStateReport();
}

// ========================================
// Setup
// ========================================
void setup() {
    Serial.begin(115200);
    delay(100);
    Serial.println("\n[BOOT] is06a starting...");
    Serial.printf("[BOOT] Version: %s\n", FIRMWARE_VERSION);

    // GPIO初期化
    initGPIO();

    // Operator初期化
    op.setMode(OperatorMode::PROVISION);

    // SettingManager初期化
    if (!settings.begin("isms")) {
        Serial.println("[FATAL] Settings init failed - rebooting in 5s");
        delay(5000);
        ESP.restart();
    }

    // AraneaSettings初期化
    AraneaSettings::init();
    AraneaSettings::initDefaults(settings);

    // DisplayManager初期化
    if (!display.begin()) {
        Serial.println("[WARNING] Display init failed");
    }
    display.showBoot("Booting...");

    // LacisID生成
    myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
    myLacisId.toUpperCase();
    myMac = lacisGen.getStaMac12Hex();
    myMac.toUpperCase();
    myHostname = getHostname();
    Serial.printf("[LACIS] ID: %s, MAC: %s\n", myLacisId.c_str(), myMac.c_str());

    // WiFi接続
    display.showConnecting("WiFi...", 0);
    wifi.setHostname(myHostname);
    if (!wifi.connectWithSettings(&settings)) {
        Serial.println("[WIFI] Connection failed, starting AP mode");
        startAPMode();
    } else {
        Serial.printf("[WIFI] Connected: %s\n", wifi.getIP().c_str());

        // NTP同期
        display.showBoot("NTP sync...");
        if (!ntp.sync()) {
            Serial.println("[WARNING] NTP sync failed");
        }

        // AraneaGateway登録
        display.showRegistering("Registering...");
        myTid = settings.getString("tid", ARANEA_DEFAULT_TID);
        myFid = settings.getString("fid", ARANEA_DEFAULT_FID);

        String gateUrl = settings.getString("gate_url", ARANEA_DEFAULT_GATE_URL);
        araneaReg.begin(gateUrl);

        TenantPrimaryAuth tenantAuth;
        tenantAuth.lacisId = settings.getString("tenant_lacis", ARANEA_DEFAULT_TENANT_LACISID);
        tenantAuth.userId = settings.getString("tenant_email", ARANEA_DEFAULT_TENANT_EMAIL);
        tenantAuth.cic = settings.getString("tenant_cic", ARANEA_DEFAULT_TENANT_CIC);
        araneaReg.setTenantPrimary(tenantAuth);

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
                Serial.println("[REG] ERROR: No CIC available");
                display.showError("No CIC!");
                araneaReg.clearRegistration();
                delay(5000);
                ESP.restart();
            }
        }
    }

    // 設定読み込み
    heartbeatIntervalSec = settings.getInt(Is06aKeys::kHeartbeat, 60);
    bootGraceMs = settings.getInt(Is06aKeys::kBootGrace, 3000);

    // TriggerManager初期化
    display.showBoot("Triggers...");
    trigger.begin(&settings, &ntp);
    trigger.onPulseStart(onPulseStart);
    trigger.onPulseEnd(onPulseEnd);
    trigger.onInputChange(onInputChange);
    trigger.onPwmChange(onPwmChange);

    // StateReporter初期化
    stateReporter.begin(&settings, &ntp, &trigger);
    stateReporter.setAuth(myTid, myLacisId, myCic);
    stateReporter.setMac(myMac);
    stateReporter.setRelayUrls(
        settings.getString("relay_pri", ARANEA_DEFAULT_RELAY_PRIMARY),
        settings.getString("relay_sec", ARANEA_DEFAULT_RELAY_SECONDARY)
    );
    stateReporter.setCloudUrl(settings.getString("cloud_url", ARANEA_DEFAULT_CLOUD_URL));

    // HttpManager初期化
    httpMgr.begin(&settings, &trigger, 80);

    AraneaDeviceInfo devInfo;
    devInfo.deviceType = DEVICE_SHORT_NAME;
    devInfo.modelName = "6-Channel Relay Controller";
    devInfo.contextDesc = "6ch Trigger (4 PWM + 2 I/O)";
    devInfo.lacisId = myLacisId;
    devInfo.cic = myCic;
    devInfo.mac = myMac;
    devInfo.hostname = myHostname;
    devInfo.firmwareVersion = FIRMWARE_VERSION;
    devInfo.buildDate = __DATE__ " " __TIME__;
    devInfo.modules = "WiFi,NTP,HTTP,PWM";
    httpMgr.setDeviceInfo(devInfo);

    // OTA初期化
    ota.begin(myHostname, "");
    ota.onStart([]() {
        op.setOtaUpdating(true);
        display.showBoot("OTA Start...");
    });
    ota.onEnd([]() {
        op.setOtaUpdating(false);
        display.showBoot("OTA Done!");
    });
    ota.onProgress([](int progress) {
        display.showBoot("OTA: " + String(progress) + "%");
    });
    ota.onError([](const String& err) {
        op.setOtaUpdating(false);
        display.showError("OTA: " + err);
    });

    // HTTP OTA初期化
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

    // RUNモードへ
    op.setMode(OperatorMode::RUN);

    // 起動時刻記録
    bootTime = millis();
    lastHeartbeatTime = bootTime;

    // 初期状態送信
    if (!apModeActive) {
        display.showBoot("State Report...");
        stateReporter.sendStateReport();
    }

    Serial.println("[BOOT] is06a ready!");
    Serial.printf("[BOOT] Boot grace: %dms, Heartbeat: %ds\n", bootGraceMs, heartbeatIntervalSec);
    printAraneaGlobalModules();
}

// ========================================
// Loop
// ========================================
void loop() {
    unsigned long now = millis();

    // OTA処理（最優先）
    ota.handle();
    if (op.isOtaUpdating()) {
        delay(10);
        return;
    }

    // HTTP処理
    httpMgr.handle();

    // APモードタイムアウト
    if (apModeActive && (now - apModeStartTime >= AP_MODE_TIMEOUT_MS)) {
        Serial.println("[AP] Timeout, attempting STA reconnect");
        stopAPMode();
        wifi.connectWithSettings(&settings);
        if (!wifi.isConnected()) {
            startAPMode();
        }
    }

    // トリガーサンプリング・更新
    trigger.sample();
    trigger.update();

    // 心拍送信
    if (!apModeActive && (now - lastHeartbeatTime >= (unsigned long)heartbeatIntervalSec * 1000)) {
        Serial.printf("[HEARTBEAT] Sending...\n");
        stateReporter.sendHeartbeat();
        lastHeartbeatTime = now;

        // ステータス更新
        httpMgr.setSendStatus(
            stateReporter.getSentCount(),
            stateReporter.getFailCount(),
            stateReporter.getLastResult()
        );
    }

    // ディスプレイ更新
    bool anyActive = trigger.isAnyPulseActive();
    unsigned long displayInterval = anyActive ? DISPLAY_ACTION_MS : DISPLAY_UPDATE_MS;
    if (now - lastDisplayUpdate >= displayInterval) {
        updateDisplay();
        lastDisplayUpdate = now;
    }

    // ボタン処理
    handleButtons();

    // WiFi再接続チェック
    if (!apModeActive && !wifi.isConnected()) {
        Serial.println("[WIFI] Disconnected, reconnecting...");
        display.showConnecting("Reconnect...", 0);
        wifi.connectWithSettings(&settings);
    }

    delay(10);
}
