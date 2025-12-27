/**
 * is02a - BLE Gateway Device (Generic Version)
 *
 * is01aのBLEアドバタイズを受信し、ローカルis03a（Zero3）へHTTP中継
 * is02a自身も温湿度センサー（HT-30）を持ち、自己計測データも送信
 *
 * 機能:
 * - BLEスキャン（NimBLE）でis01aデータ受信
 * - バッチ送信（30秒間隔）
 * - is02a自身のセンサーデータ送信
 * - HTTP POST（primary/secondary フォールバック）
 * - OLED表示
 * - HTTP OTAアップデート
 * - Web UI（設定・ステータス）
 *
 * パーティション: min_SPIFFS使用（OTA領域確保）
 */

#include <Arduino.h>
#include <Wire.h>
#include <SHT31.h>
#include <ArduinoJson.h>

// Aranea Global Modules
#include "AraneaGlobalImporter.h"

// ========================================
// 定数
// ========================================
static const char* DEVICE_TYPE = "ISMS_ar-is02";
static const char* PRODUCT_TYPE = "002";
static const char* PRODUCT_CODE = "0096";
static const char* FIRMWARE_VERSION = "1.0.0";

// GPIO定義
static const int PIN_I2C_SDA = 21;
static const int PIN_I2C_SCL = 22;

// 表示更新間隔
static const unsigned long DISPLAY_UPDATE_MS = 1000;

// ========================================
// モジュールインスタンス（共通）
// ========================================
SettingManager settings;
DisplayManager display;
WiFiManager wifi;
NtpManager ntp;
LacisIDGenerator lacisGen;
AraneaRegister araneaReg;
Operator op;
HttpOtaManager ota;

// ========================================
// is02a固有インスタンス
// ========================================
BleReceiver bleReceiver;
HttpManagerIs02a httpMgr;
StateReporterIs02a stateReporter;
MqttManager mqtt;

// HT-30センサー（SHT31互換、I2Cアドレス0x44）
SHT31 sht(0x44);
bool sensorReady = false;

// ========================================
// 状態変数
// ========================================
String myLacisId;
String myMac;
String myIp;
String myCic = "";

float selfTempC = NAN;
float selfHumPct = NAN;

unsigned long lastBatchSend = 0;
unsigned long lastSelfMeasure = 0;
unsigned long lastDisplayUpdate = 0;

bool isSending = false;
bool mqttConnected = false;
String myTid;
String cmdTopic;
String ackTopic;

// MQTTコマンド遅延処理用
String pendingMqttCmd = "";
String pendingMqttPayload = "";

// ========================================
// 設定値（NVSから読み込み）
// ========================================
int batchIntervalSec = 300;  // デフォルト5分
int selfMeasureIntervalSec = 60;
float tempOffset = 0.0;
float humOffset = 0.0;

// ========================================
// 自己計測
// ========================================
void measureSelf() {
  if (!sensorReady) {
    // センサーなし：NANを維持
    stateReporter.setSelfSensorData(NAN, NAN, false);
    httpMgr.setSelfSensorData(NAN, NAN);
    return;
  }

  if (sht.read()) {
    selfTempC = sht.getTemperature() + tempOffset;
    selfHumPct = sht.getHumidity() + humOffset;
    Serial.printf("[SENSOR] HT-30: %.1f°C, %.1f%%\n", selfTempC, selfHumPct);

    // StateReporter更新
    stateReporter.setSelfSensorData(selfTempC, selfHumPct, true);
    httpMgr.setSelfSensorData(selfTempC, selfHumPct);
  } else {
    selfTempC = NAN;
    selfHumPct = NAN;
    stateReporter.setSelfSensorData(NAN, NAN, false);
    httpMgr.setSelfSensorData(NAN, NAN);
    Serial.println("[SENSOR] HT-30 read failed");
  }
}

// ========================================
// バッチ送信
// ========================================
static int mqttSuccessCount = 0;
static int mqttFailCount = 0;

void sendBatch() {
  isSending = true;
  updateDisplay();

  // 期限切れノード削除
  bleReceiver.cleanupExpired();

  int nodeCount = bleReceiver.getValidNodeCount();
  Serial.printf("[BATCH] Sending (self + %d nodes)...\n", nodeCount);

  // ペイロード構築
  String payload = stateReporter.buildPayload();

  // 1. MQTT送信（優先）
  bool mqttOk = false;
  if (mqttConnected && payload.length() > 10) {
    String stateTopic = "aranea/" + myTid + "/" + myLacisId + "/state";
    int msgId = mqtt.publish(stateTopic, payload, 1, false);
    mqttOk = (msgId >= 0);
    if (mqttOk) {
      mqttSuccessCount++;
      Serial.printf("[BATCH] MQTT publish OK (msgId=%d)\n", msgId);
    } else {
      mqttFailCount++;
      Serial.println("[BATCH] MQTT publish failed");
    }
  }

  // 2. HTTPリレー送信（フォールバック or 並行）
  bool relayOk = stateReporter.sendBatchToRelay();

  isSending = false;

  bool ok = mqttOk || relayOk;
  if (ok) {
    Serial.println("[BATCH] Sent successfully");
    stateReporter.setLastBatchTimeNow();
  } else {
    Serial.println("[BATCH] Send failed (both MQTT and relay)");
  }

  // 統計更新
  httpMgr.setSendStatus(mqttSuccessCount, mqttFailCount,
    stateReporter.getRelaySuccessCount(),
    stateReporter.getRelayFailCount());
  httpMgr.setLastBatchTime(stateReporter.getLastBatchTime());
}

// ========================================
// 表示更新
// ========================================
void updateDisplay() {
  // Line1: IP + RSSI
  String line1;
  if (wifi.isConnected()) {
    int rssi = WiFi.RSSI();
    char buf[32];
    snprintf(buf, sizeof(buf), "%s %ddBm", myIp.c_str(), rssi);
    line1 = String(buf);
  } else {
    line1 = "No WiFi";
  }

  // CIC
  String cicDisplay = myCic.length() > 0 ? myCic : "------";

  // センサー情報
  String sensorLine;
  if (isnan(selfTempC) || isnan(selfHumPct)) {
    sensorLine = "ERR";
  } else {
    char buf[16];
    snprintf(buf, sizeof(buf), "%.1fC %.0f%%", selfTempC, selfHumPct);
    sensorLine = String(buf);
  }

  display.showIs02Main(line1, cicDisplay, sensorLine, isSending);
}

// ========================================
// MQTTコマンドハンドラ（遅延処理）
// ========================================
// イベントハンドラ内でpublishするとスタック問題が起きるため
// コマンドを保留して loop() で処理する

void handleMqttCommand(const String& topic, const char* data, int len) {
  Serial.printf("[MQTT] Received: %s\n", data);
  // 保留変数にコピー（loop()で処理）
  pendingMqttPayload = String(data);
}

// 実際のコマンド処理（loop()から呼び出す）
void processPendingMqttCommand() {
  if (pendingMqttPayload.length() == 0) return;

  String payload = pendingMqttPayload;
  pendingMqttPayload = "";  // クリア

  // JSON解析
  DynamicJsonDocument doc(512);
  DeserializationError err = deserializeJson(doc, payload);
  if (err) {
    Serial.printf("[MQTT] JSON parse error: %s\n", err.c_str());
    return;
  }

  // action または type フィールドをサポート（SDK互換）
  String action = doc["action"] | "";
  if (action.length() == 0) {
    action = doc["type"] | "";
  }
  Serial.printf("[MQTT] Processing action: %s\n", action.c_str());

  if (action == "setConfig") {
    // 設定変更コマンド
    String key = doc["key"] | "";
    String value = doc["value"] | "";

    if (key.length() == 0) {
      Serial.println("[MQTT] Missing key");
      return;
    }

    // NVS保存（値の型に応じて）
    bool saved = false;
    if (value == "true" || value == "false") {
      settings.setBool(key.c_str(), value == "true");
      saved = true;
    } else if (value.indexOf('.') >= 0) {
      settings.setString(key.c_str(), value);
      saved = true;
    } else {
      bool isNum = true;
      for (unsigned int i = 0; i < value.length(); i++) {
        char c = value.charAt(i);
        if (i == 0 && c == '-') continue;
        if (!isDigit(c)) { isNum = false; break; }
      }
      if (isNum && value.length() > 0) {
        settings.setInt(key.c_str(), value.toInt());
      } else {
        settings.setString(key.c_str(), value);
      }
      saved = true;
    }

    if (saved) {
      Serial.printf("[MQTT] Config saved: %s = %s\n", key.c_str(), value.c_str());
      reloadSettings();

      DynamicJsonDocument ack(256);
      ack["ok"] = true;
      ack["action"] = "setConfig";
      ack["key"] = key;
      ack["value"] = value;
      String ackPayload;
      serializeJson(ack, ackPayload);
      mqtt.publish(ackTopic, ackPayload);
    }
  } else if (action == "getConfig") {
    String key = doc["key"] | "";
    String value = settings.getString(key.c_str(), "");

    DynamicJsonDocument ack(256);
    ack["ok"] = true;
    ack["action"] = "getConfig";
    ack["key"] = key;
    ack["value"] = value;
    String ackPayload;
    serializeJson(ack, ackPayload);
    mqtt.publish(ackTopic, ackPayload);
  } else if (action == "reboot") {
    Serial.println("[MQTT] Reboot requested");
    DynamicJsonDocument ack(128);
    ack["ok"] = true;
    ack["action"] = "reboot";
    String ackPayload;
    serializeJson(ack, ackPayload);
    mqtt.publish(ackTopic, ackPayload);
    delay(500);
    ESP.restart();
  } else if (action == "status") {
    DynamicJsonDocument ack(512);
    ack["ok"] = true;
    ack["action"] = "status";
    ack["lacisId"] = myLacisId;
    ack["cic"] = myCic;
    ack["ip"] = myIp;
    ack["uptime"] = millis() / 1000;
    ack["nodes"] = bleReceiver.getValidNodeCount();
    ack["rssi"] = WiFi.RSSI();
    if (!isnan(selfTempC)) ack["temp"] = selfTempC;
    if (!isnan(selfHumPct)) ack["hum"] = selfHumPct;
    String ackPayload;
    serializeJson(ack, ackPayload);
    mqtt.publish(ackTopic, ackPayload);
    Serial.println("[MQTT] Status response sent");
  } else {
    Serial.printf("[MQTT] Unknown action: %s\n", action.c_str());
  }
}

// ========================================
// 設定再読み込み
// ========================================
void reloadSettings() {
  batchIntervalSec = settings.getInt(Is02aKeys::kBatchIntv, 300);
  selfMeasureIntervalSec = settings.getInt(Is02aKeys::kSelfIntv, 60);
  tempOffset = settings.getString(Is02aKeys::kTempOffset, "0.0").toFloat();
  humOffset = settings.getString(Is02aKeys::kHumOffset, "0.0").toFloat();

  // BLE設定
  int bleScanSec = settings.getInt(Is02aKeys::kBleScanSec, 5);
  String bleFilter = settings.getString(Is02aKeys::kBleFilter, "");
  bleReceiver.setScanDuration(bleScanSec);
  bleReceiver.setFilterPrefix(bleFilter);

  // リレーURL
  String relayPri = settings.getString(Is02aKeys::kRelayUrl, "");
  String relaySec = settings.getString(Is02aKeys::kRelayUrl2, "");
  stateReporter.setRelayUrls(relayPri, relaySec);

  Serial.printf("[CONFIG] Batch=%ds, Self=%ds, TempOff=%.1f, HumOff=%.1f\n",
    batchIntervalSec, selfMeasureIntervalSec, tempOffset, humOffset);
}

// ========================================
// Setup
// ========================================
void setup() {
  Serial.begin(115200);
  delay(100);
  Serial.println("\n[BOOT] is02a starting...");

  // 0. GPIO5をHIGHに設定（I2C用MOSFET有効化）
  pinMode(5, OUTPUT);
  digitalWrite(5, HIGH);
  delay(50);  // MOSFET安定待ち

  // Operator初期化
  op.setMode(OperatorMode::PROVISION);

  // 1. SettingManager
  settings.begin();

  // 2. デフォルト設定投入
  AraneaSettings::init();
  AraneaSettings::initDefaults(settings);

  // 3. I2C初期化（DisplayとSensorで共有）
  Wire.begin(PIN_I2C_SDA, PIN_I2C_SCL);
  Wire.setClock(100000);
  delay(100);  // I2C安定待ち

  // I2Cスキャン（デバッグ用）
  Serial.println("[I2C] Scanning...");
  for (uint8_t addr = 1; addr < 127; addr++) {
    Wire.beginTransmission(addr);
    if (Wire.endTransmission() == 0) {
      Serial.printf("[I2C] Found device at 0x%02X\n", addr);
    }
    if (addr % 16 == 0) yield();  // WDT対策
  }
  Serial.println("[I2C] Scan complete");

  // 4. DisplayManager（I2C共有、失敗してもI2Cバス維持）
  if (!display.begin()) {
    Serial.println("[WARN] Display init failed, continuing without OLED");
  } else {
    display.showBoot("Booting...");
  }

  // 5. HT-30センサー初期化（SHT31互換、I2Cアドレス0x44）
  // I2Cスキャンで0x44が見つかった場合のみ初期化試行
  bool sensorFound = false;
  Wire.beginTransmission(0x44);
  if (Wire.endTransmission() == 0) {
    sensorFound = true;
  }

  if (sensorFound) {
    delay(100);  // I2Cバス安定待ち
    sht.begin();
    delay(50);
    if (sht.read()) {
      sensorReady = true;
      selfTempC = sht.getTemperature();
      selfHumPct = sht.getHumidity();
      Serial.printf("[SENSOR] HT-30 ready: %.1f°C, %.1f%%\n", selfTempC, selfHumPct);
    } else {
      sensorReady = false;
      selfTempC = NAN;
      selfHumPct = NAN;
      Serial.println("[SENSOR] HT-30 read failed");
    }
  } else {
    sensorReady = false;
    selfTempC = NAN;
    selfHumPct = NAN;
    Serial.println("[SENSOR] No sensor (headless mode)");
  }

  // 6. WiFi接続（NVS設定を使用）
  display.showConnecting("WiFi...", 0);
  String hostname = "is02a-" + WiFi.macAddress().substring(12);
  hostname.replace(":", "");
  wifi.setHostname(hostname);

  if (!wifi.connectWithSettings(&settings)) {
    // 接続失敗時はデフォルトSSIDを試行
    Serial.println("[WIFI] Settings failed, trying default...");
    if (!wifi.connectDefault()) {
      Serial.println("[WIFI] All connection attempts failed");
      display.showError("WiFi Failed");
      delay(3000);
      ESP.restart();
    }
  }
  myIp = wifi.getIP();

  // 6. NTP同期
  display.showBoot("NTP sync...");
  ntp.sync();

  // 7. LacisID生成
  myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
  myMac = myLacisId.substring(4, 16);
  Serial.printf("[LACIS] ID: %s\n", myLacisId.c_str());

  // 8. AraneaRegister（クラウド登録）
  display.showRegistering("Registering...");
  String gateUrl = settings.getString("gate_url", AraneaSettings::getGateUrl());
  araneaReg.begin(gateUrl);

  TenantPrimaryAuth tenantAuth;
  tenantAuth.lacisId = settings.getString("tenant_lacis", AraneaSettings::getTenantLacisId());
  tenantAuth.userId = settings.getString("tenant_email", AraneaSettings::getTenantEmail());
  tenantAuth.cic = settings.getString("tenant_cic", AraneaSettings::getTenantCic());
  araneaReg.setTenantPrimary(tenantAuth);

  myTid = settings.getString("tid", AraneaSettings::getTid());
  AraneaRegisterResult regResult = araneaReg.registerDevice(
    myTid, DEVICE_TYPE, myLacisId, myMac, PRODUCT_TYPE, PRODUCT_CODE
  );

  if (regResult.ok) {
    myCic = regResult.cic_code;
    settings.setString("cic", myCic);
    Serial.printf("[ARANEA] CIC: %s\n", myCic.c_str());
    display.showRegistering("CIC: " + myCic);
  } else {
    myCic = settings.getString("cic", "");
    Serial.printf("[ARANEA] Registration failed: %s\n", regResult.error.c_str());
    if (myCic.length() > 0) {
      display.showRegistering("Saved: " + myCic);
    } else {
      display.showRegistering("No CIC");
    }
  }
  delay(500);

  // 9. 設定読み込み
  reloadSettings();

  // 10. StateReporter初期化
  stateReporter.begin(&settings, &ntp, &bleReceiver);
  stateReporter.setAuth(myTid, myLacisId, myCic);
  stateReporter.setMac(myMac);
  stateReporter.setIp(myIp);

  // 11. BLE初期化・スキャン開始
  display.showBoot("BLE init...");
  bleReceiver.begin(
    settings.getInt(Is02aKeys::kBleScanSec, 5),
    settings.getString(Is02aKeys::kBleFilter, "")
  );
  bleReceiver.startContinuousScan();
  Serial.println("[BLE] Continuous scan started");

  // 12. Web UI初期化
  display.showBoot("HTTP init...");
  httpMgr.begin(&settings, &bleReceiver, 80);

  // デバイス情報設定
  AraneaDeviceInfo devInfo;
  devInfo.deviceType = DEVICE_TYPE;
  devInfo.modelName = "Temp/Hum Master";
  devInfo.contextDesc = "BLE Gateway for is01a nodes";
  devInfo.lacisId = myLacisId;
  devInfo.cic = myCic;
  devInfo.mac = myMac;
  devInfo.hostname = hostname;
  devInfo.firmwareVersion = FIRMWARE_VERSION;
  devInfo.buildDate = __DATE__;
  devInfo.modules = "WiFi,NTP,BLE,HTTP,OTA";
  httpMgr.setDeviceInfo(devInfo);

  httpMgr.setSelfSensorData(selfTempC, selfHumPct);

  // 13. HTTP OTA初期化
  display.showBoot("OTA init...");
  ota.begin(httpMgr.getServer());
  ota.onProgress([](int progress) {
    display.showBoot("OTA: " + String(progress) + "%");
  });

  // 14. MQTT初期化（オンライン設定変更用）
  display.showBoot("MQTT init...");
  String mqttBroker = settings.getString(Is02aKeys::kMqttBroker, AraneaSettings::getMqttBroker());
  if (mqttBroker.length() > 0 && myCic.length() > 0) {
    // トピック設定（is04互換形式: aranea/{tid}/{lacisId}/xxx）
    cmdTopic = "aranea/" + myTid + "/" + myLacisId + "/command";
    ackTopic = "aranea/" + myTid + "/" + myLacisId + "/ack";

    // コールバック設定
    mqtt.onConnected([&]() {
      Serial.println("[MQTT] Connected, subscribing...");
      mqttConnected = true;
      httpMgr.setMqttConnected(true);
      mqtt.subscribe(cmdTopic, 1);
    });

    mqtt.onDisconnected([&]() {
      Serial.println("[MQTT] Disconnected");
      mqttConnected = false;
      httpMgr.setMqttConnected(false);
    });

    mqtt.onMessage(handleMqttCommand);

    mqtt.onError([](const String& error) {
      Serial.printf("[MQTT] Error: %s\n", error.c_str());
    });

    // 接続開始（lacisId + cicで認証）
    if (mqtt.begin(mqttBroker, myLacisId, myCic)) {
      Serial.printf("[MQTT] Connecting to %s\n", mqttBroker.c_str());
      Serial.printf("[MQTT] Topic: %s\n", cmdTopic.c_str());
    } else {
      Serial.println("[MQTT] Init failed");
    }
  } else {
    Serial.println("[MQTT] Skipped (no broker or CIC)");
  }

  // 15. 通常動作モードへ移行
  op.setMode(OperatorMode::RUN);

  // 初期表示
  unsigned long now = millis();
  lastBatchSend = now;
  lastSelfMeasure = now;
  lastDisplayUpdate = now;

  Serial.println("[BOOT] Ready");
  Serial.print("[BOOT] Web UI: http://");
  Serial.print(myIp);
  Serial.println("/");
  Serial.flush();

  // 初期表示（Ready後に実行）
  updateDisplay();
}

// ========================================
// Loop
// ========================================
void loop() {
  unsigned long now = millis();

  // Web UI処理
  httpMgr.handle();

  // MQTT処理（再接続チェック含む）
  mqtt.handle();

  // 保留中MQTTコマンド処理
  processPendingMqttCommand();

  // 自己計測（60秒間隔）
  if (now - lastSelfMeasure >= (unsigned long)selfMeasureIntervalSec * 1000) {
    measureSelf();
    lastSelfMeasure = now;
  }

  // バッチ送信（30秒間隔）
  if (now - lastBatchSend >= (unsigned long)batchIntervalSec * 1000) {
    sendBatch();
    lastBatchSend = now;
  }

  // 表示更新（1秒ごと）
  if (now - lastDisplayUpdate > DISPLAY_UPDATE_MS) {
    updateDisplay();
    lastDisplayUpdate = now;
  }

  // WiFi再接続チェック
  if (!wifi.isConnected()) {
    Serial.println("[WIFI] Disconnected, reconnecting...");
    display.showError("WiFi lost");
    if (wifi.connectWithSettings(&settings)) {
      myIp = wifi.getIP();
      stateReporter.setIp(myIp);
    }
  }

  delay(10);
}
