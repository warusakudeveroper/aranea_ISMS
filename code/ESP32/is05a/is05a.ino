/**
 * is05a - 8ch Detector (ar-is05a)
 *
 * 8チャンネルリードスイッチ/接点入力デバイス。
 * 状態変化を検出してローカルリレー/クラウド/Webhookへ通知。
 *
 * 機能:
 * - 8ch入力（ch1-6: 入力専用、ch7-8: I/O切替可能）
 * - 可変デバウンス（5-10000ms）
 * - Webhook通知（Discord/Slack/Generic）
 * - Web UI設定
 * - OTA更新
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
#include <ArduinoJson.h>
#include <vector>

// ============================================================
// Araneaモジュール一括インポート
// ============================================================
#include "AraneaGlobalImporter.h"

// ========================================
// デバイス定数
// ========================================
static const char* PRODUCT_TYPE = "005";
static const char* PRODUCT_CODE = "0001";
static const char* ARANEA_DEVICE_TYPE = "aranea_ar-is05a";
static const char* DEVICE_SHORT_NAME = "ar-is05a";
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

// is05a固有モジュール
ChannelManager channels;
WebhookManager webhooks;
HttpManagerIs05a httpMgr;
StateReporterIs05a stateReporter;
RuleManager ruleMgr;
MqttManager mqtt;

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
int bootGraceMs = 5000;

// OLED表示用（複数検知時の切り替え表示）
unsigned long lastDisplayCycleTime = 0;
int displayCycleIndex = 0;
static const unsigned long DISPLAY_CYCLE_MS = 500;  // 500ms切り替え

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

    // アクティブな検知と出力を収集
    std::vector<String> displayItems;

    // 検知中のチャンネル
    for (int ch = 1; ch <= 8; ch++) {
        if (!channels.isOutputMode(ch) && channels.isActive(ch)) {
            displayItems.push_back("Detect:" + String(ch));
        }
    }

    // 出力中のチャンネル（ch7/ch8）
    for (int ch = 7; ch <= 8; ch++) {
        if (channels.isOutputMode(ch) && channels.isOutputActive(ch)) {
            auto cfg = channels.getConfig(ch);
            String modeChar;
            switch (cfg.outputMode) {
                case ChannelManager::OutputMode::MOMENTARY: modeChar = "M"; break;
                case ChannelManager::OutputMode::ALTERNATE: modeChar = "A"; break;
                case ChannelManager::OutputMode::INTERVAL:  modeChar = "I"; break;
            }
            displayItems.push_back("Output:" + modeChar + String(ch));
        }
    }

    // 表示内容決定
    String sensorLine;
    if (displayItems.empty()) {
        sensorLine = "Ready";
    } else if (displayItems.size() == 1) {
        sensorLine = displayItems[0];
    } else {
        // 複数アイテム: 500msサイクル
        unsigned long now = millis();
        if (now - lastDisplayCycleTime >= DISPLAY_CYCLE_MS) {
            displayCycleIndex = (displayCycleIndex + 1) % displayItems.size();
            lastDisplayCycleTime = now;
        }
        if (displayCycleIndex >= (int)displayItems.size()) {
            displayCycleIndex = 0;
        }
        sensorLine = displayItems[displayCycleIndex];
    }

    display.showIs02Main(line1, cicStr, sensorLine, false);
}

// ========================================
// MQTTメッセージハンドラ
// ========================================
void handleMqttMessage(const String& topic, const char* data, int len) {
    String payload(data, len);
    Serial.printf("[MQTT] Received: %s -> %s\n", topic.c_str(), payload.c_str());

    // トピック: aranea/{tid}/{lacisId}/command
    // コマンド形式: {"action":"...", "params":{...}}
    StaticJsonDocument<512> doc;
    DeserializationError error = deserializeJson(doc, payload);
    if (error) {
        Serial.println("[MQTT] JSON parse error");
        return;
    }

    // SDKフォーマット: type/parameters, デバイスフォーマット: action/params
    String action = doc["type"].as<String>();
    if (action.isEmpty()) action = doc["action"].as<String>();
    JsonObject params = doc["parameters"];
    if (params.isNull()) params = doc["params"];

    // 出力ON/OFF
    if (action == "output_on") {
        int ch = params["ch"] | 0;
        if (ch >= 7 && ch <= 8 && channels.isOutputMode(ch)) {
            channels.triggerOutput(ch);
            Serial.printf("[MQTT] Output ON: ch%d\n", ch);
        }
    }
    else if (action == "output_off") {
        int ch = params["ch"] | 0;
        if (ch >= 7 && ch <= 8) {
            channels.stopOutput(ch);
            Serial.printf("[MQTT] Output OFF: ch%d\n", ch);
        }
    }
    // 設定変更
    else if (action == "set_config") {
        // paramsは上で定義済み
        // Webhook設定
        if (params.containsKey("webhookEnabled")) {
            webhooks.setEnabled(params["webhookEnabled"]);
        }
        if (params.containsKey("quietEnabled")) {
            webhooks.setQuietModeEnabled(params["quietEnabled"]);
        }
        if (params.containsKey("quietStart") && params.containsKey("quietEnd")) {
            webhooks.setQuietModeHours(params["quietStart"], params["quietEnd"]);
        }
        if (params.containsKey("channelMask")) {
            webhooks.setChannelMask(params["channelMask"].as<uint8_t>());
        }

        // 出力モード設定
        if (params.containsKey("ch7_mode")) {
            channels.setOutputMode(7, params["ch7_mode"].as<String>() == "output");
        }
        if (params.containsKey("ch8_mode")) {
            channels.setOutputMode(8, params["ch8_mode"].as<String>() == "output");
        }
        if (params.containsKey("ch7_outMode")) {
            channels.setOutputBehavior(7,
                static_cast<ChannelManager::OutputMode>(params["ch7_outMode"].as<int>()),
                params["ch7_duration"] | 3000,
                params["ch7_count"] | 3);
        }
        if (params.containsKey("ch8_outMode")) {
            channels.setOutputBehavior(8,
                static_cast<ChannelManager::OutputMode>(params["ch8_outMode"].as<int>()),
                params["ch8_duration"] | 3000,
                params["ch8_count"] | 3);
        }

        Serial.println("[MQTT] Config updated");
    }
    // ステータス要求
    else if (action == "get_status") {
        // ステータスを返す
        StaticJsonDocument<1024> resp;
        resp["lacisId"] = myLacisId;
        resp["cic"] = myCic;
        JsonArray chArr = resp.createNestedArray("channels");
        for (int ch = 1; ch <= 8; ch++) {
            JsonObject chObj = chArr.createNestedObject();
            chObj["ch"] = ch;
            chObj["state"] = channels.getStateString(ch);
            chObj["isOutput"] = channels.isOutputMode(ch);
        }
        resp["webhookEnabled"] = webhooks.isEnabled();
        resp["quietEnabled"] = webhooks.isQuietModeEnabled();
        resp["quietInPeriod"] = webhooks.isInQuietPeriod();

        String respStr;
        serializeJson(resp, respStr);

        String respTopic = "aranea/" + myTid + "/" + myLacisId + "/status";
        mqtt.publish(respTopic, respStr);
    }
}

// ========================================
// 状態変化ハンドラ
// ========================================
void onChannelChanged(int ch, bool active) {
    // 起動猶予期間チェック
    if (bootTime > 0 && (millis() - bootTime) < (unsigned long)bootGraceMs) {
        Serial.println("[CHANNEL] Boot grace period - skipping send");
        return;
    }

    Serial.printf("[CHANNEL] ch%d changed to %s\n", ch, active ? "ACTIVE" : "INACTIVE");

    // 状態レポート送信
    stateReporter.sendStateReport();

    // Webhook通知
    webhooks.sendStateChange(ch, active);

    // MQTT通知
    if (mqtt.isConnected()) {
        StaticJsonDocument<256> doc;
        doc["event"] = "state_change";
        doc["ch"] = ch;
        doc["active"] = active;
        doc["state"] = channels.getStateString(ch);
        doc["timestamp"] = ntp.isSynced() ? ntp.getIso8601() : "";

        String payload;
        serializeJson(doc, payload);
        String topic = "aranea/" + myTid + "/" + myLacisId + "/event";
        mqtt.publish(topic, payload);
    }

    // ディスプレイ更新
    httpMgr.setChannelStatus(ch, channels.getLastUpdatedAt(ch));

    // 送信ステータス更新
    httpMgr.setSendStatus(
        stateReporter.getSentCount(),
        stateReporter.getFailCount(),
        stateReporter.getLastResult()
    );

    // ルール処理（入力→出力/Webhook）
    ruleMgr.handleChannelEvent(ch, active);
}

// ========================================
// Setup
// ========================================
void setup() {
    Serial.begin(115200);
    delay(100);
    Serial.println("\n[BOOT] is05a starting...");
    Serial.printf("[BOOT] Version: %s\n", FIRMWARE_VERSION);

    // GPIO初期化
    initGPIO();

    // Operator初期化
    op.setMode(OperatorMode::PROVISION);

    // SettingManager初期化
    if (!settings.begin("isms")) {
        Serial.println("[ERROR] Settings init failed");
        return;
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
    heartbeatIntervalSec = settings.getInt(Is05aKeys::kHeartbeat, 60);
    bootGraceMs = settings.getInt(Is05aKeys::kBootGrace, 5000);

    // ChannelManager初期化
    display.showBoot("Channels...");
    channels.begin(&settings, &ntp);
    channels.onChannelChange(onChannelChanged);

    // WebhookManager初期化
    webhooks.begin(&settings, &ntp, &channels);
    webhooks.setDeviceInfo(myLacisId, myHostname);

    // RuleManager初期化
    ruleMgr.begin(&settings, &channels, &webhooks);

    // StateReporter初期化
    stateReporter.begin(&settings, &ntp, &channels);
    stateReporter.setAuth(myTid, myLacisId, myCic);
    stateReporter.setMac(myMac);
    stateReporter.setRelayUrls(
        settings.getString("relay_pri", ARANEA_DEFAULT_RELAY_PRIMARY),
        settings.getString("relay_sec", ARANEA_DEFAULT_RELAY_SECONDARY)
    );
    stateReporter.setCloudUrl(settings.getString("cloud_url", ARANEA_DEFAULT_CLOUD_URL));

    // HttpManager初期化
    httpMgr.begin(&settings, &channels, &webhooks, &ruleMgr, 80);

    AraneaDeviceInfo devInfo;
    devInfo.deviceType = DEVICE_SHORT_NAME;
    devInfo.modelName = "8ch Detector";
    devInfo.contextDesc = "8チャンネル入力検出とWebhook通知";
    devInfo.lacisId = myLacisId;
    devInfo.cic = myCic;
    devInfo.mac = myMac;
    devInfo.hostname = myHostname;
    devInfo.firmwareVersion = FIRMWARE_VERSION;
    devInfo.buildDate = __DATE__ " " __TIME__;
    devInfo.modules = "WiFi,NTP,HTTP,MQTT,Webhook";
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

    // MQTT初期化（ブローカーが設定されている場合のみ）
    String mqttBroker = AraneaSettings::getMqttBroker();
    if (mqttBroker.length() > 0 && !apModeActive) {
        display.showBoot("MQTT...");
        Serial.printf("[MQTT] Connecting to %s\n", mqttBroker.c_str());

        // コールバック設定
        mqtt.onConnected([]() {
            Serial.println("[MQTT] Connected!");
            httpMgr.setMqttConnected(true);
            // コマンドトピック購読
            String cmdTopic = "aranea/" + myTid + "/" + myLacisId + "/command";
            mqtt.subscribe(cmdTopic);
            Serial.printf("[MQTT] Subscribed: %s\n", cmdTopic.c_str());
        });
        mqtt.onDisconnected([]() {
            Serial.println("[MQTT] Disconnected");
            httpMgr.setMqttConnected(false);
        });
        mqtt.onMessage(handleMqttMessage);
        mqtt.onError([](const String& err) {
            Serial.printf("[MQTT] Error: %s\n", err.c_str());
        });

        // 接続開始
        bool mqttOk = mqtt.begin(mqttBroker, myLacisId, myCic);
        if (!mqttOk) {
            Serial.println("[MQTT] Connection failed (non-fatal)");
        }
    } else {
        Serial.println("[MQTT] No broker configured, skipping");
    }

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

    Serial.println("[BOOT] is05a ready!");
    Serial.printf("[BOOT] Boot grace: %dms, Heartbeat: %ds\n", bootGraceMs, heartbeatIntervalSec);
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

    // MQTT処理
    mqtt.handle();

    // APモードタイムアウト
    if (apModeActive && (now - apModeStartTime >= AP_MODE_TIMEOUT_MS)) {
        Serial.println("[AP] Timeout, attempting STA reconnect");
        stopAPMode();
        wifi.connectWithSettings(&settings);
        if (!wifi.isConnected()) {
            startAPMode();
        }
    }

    // チャンネルサンプリング
    channels.sample();
    channels.update();

    // Webhookキュー処理
    webhooks.handle();

    // 心拍送信
    if (!apModeActive && (now - lastHeartbeatTime >= (unsigned long)heartbeatIntervalSec * 1000)) {
        Serial.printf("[HEARTBEAT] Sending...\n");
        stateReporter.sendHeartbeat();
        webhooks.sendHeartbeat();
        lastHeartbeatTime = now;

        // ステータス更新
        httpMgr.setSendStatus(
            stateReporter.getSentCount(),
            stateReporter.getFailCount(),
            stateReporter.getLastResult()
        );
    }

    // ディスプレイ更新
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
        wifi.connectWithSettings(&settings);
    }

    delay(10);
}
