/**
 * is04 - Window Open/Close Controller
 *
 * 2ch物理入力 + 2chトリガー出力。クラウド（MQTT）と手動（物理入力）の両方で制御。
 * 状態変化をdeviceStateReportでクラウドへ送信。
 *
 * 動作シーケンス:
 * 1. WiFi接続
 * 2. AraneaGateway登録（CIC取得）
 * 3. MQTT WebSocket接続（クラウドコマンド受信）
 * 4. 物理入力監視（デバウンス付き）
 * 5. 非ブロッキングパルス出力
 * 6. 状態変化時 → deviceStateReport送信
 */

#include <Arduino.h>
#include <Wire.h>
#include <WiFi.h>
#include <HTTPClient.h>
#include <esp_mac.h>
#include <mqtt_client.h>
#include <ArduinoJson.h>

// AraneaGlobal共通モジュール
#include "SettingManager.h"
#include "DisplayManager.h"
#include "WiFiManager.h"
#include "NtpManager.h"
#include "LacisIDGenerator.h"
#include "AraneaRegister.h"
#include "HttpManager.h"
#include "OtaManager.h"
#include "Operator.h"

// ========================================
// 定数（変更禁止）
// ========================================
static const char* PRODUCT_TYPE = "004";
static const char* PRODUCT_CODE = "0096";
static const char* DEVICE_TYPE = "ISMS_ar-is04";
static const char* FIRMWARE_VERSION = "1.0.0";

// ========================================
// ピン定義（正本）
// ========================================
// 物理入力（手動アンラッチ）
static const int PHYS_IN1 = 8;   // GPIO8
static const int PHYS_IN2 = 18;  // GPIO18

// トリガー出力（接点出力）
static const int TRG_OUT1 = 12;  // GPIO12
static const int TRG_OUT2 = 14;  // GPIO14

// ボタン
static const int BTN_WIFI = 25;   // WiFi再接続ボタン（3秒長押し）
static const int BTN_RESET = 26;  // ファクトリーリセットボタン（3秒長押し）

// I2C (OLED)
static const int I2C_SDA = 21;
static const int I2C_SCL = 22;

// ========================================
// タイミング定数
// ========================================
static const unsigned long SAMPLE_INTERVAL_MS = 20;      // デバウンスサンプル間隔
static const int STABLE_COUNT = 3;                        // 安定カウント
static const unsigned long HEARTBEAT_INTERVAL_MS = 60000; // 心拍送信間隔（60秒）
static const unsigned long DISPLAY_UPDATE_MS = 1000;      // ディスプレイ更新間隔
static const unsigned long BUTTON_HOLD_MS = 3000;         // ボタン長押し時間
static const unsigned long MQTT_RECONNECT_INTERVAL = 5000; // MQTT再接続間隔

// パルス設定
static const int PULSE_MS_MIN = 10;
static const int PULSE_MS_MAX = 10000;
static const int PULSE_MS_DEFAULT = 3000;
static const int INTERLOCK_MS_DEFAULT = 200;

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
HttpManager httpMgr;
OtaManager ota;
Operator op;

// 自機情報
String myLacisId;
String myMac;
String myCic;
String myTid;

// 送信先URL
String cloudUrl;

// MQTT設定
String mqttWsUrl;
esp_mqtt_client_handle_t mqttClient = nullptr;
bool mqttConnected = false;
unsigned long lastMqttReconnect = 0;

// is04設定（NVS）
String is04Dids;
bool is04Invert = false;
int is04PulseMs = PULSE_MS_DEFAULT;
int is04InterlockMs = INTERLOCK_MS_DEFAULT;

// 物理入力状態
int physIn1Raw = LOW;
int physIn1Stable = LOW;
int physIn1StableCount = 0;
int physIn2Raw = LOW;
int physIn2Stable = LOW;
int physIn2StableCount = 0;

// エッジ検出用（前回の安定状態）
int physIn1LastStable = LOW;
int physIn2LastStable = LOW;

// パルス状態機械（非ブロッキング）
bool pulseActive = false;
int pulseOutPin = 0;
unsigned long pulseStartMs = 0;
unsigned long pulseDurationMs = 0;

// タイミング
unsigned long lastSampleTime = 0;
unsigned long lastHeartbeatTime = 0;
unsigned long lastDisplayUpdate = 0;
unsigned long btnWifiPressTime = 0;
unsigned long btnResetPressTime = 0;

// 状態
int heartbeatCount = 0;
bool needSendState = false;
String lastSendResult = "---";
String lastCmdId = "";

// ========================================
// 前方宣言
// ========================================
void mqttEventHandler(void* handlerArgs, esp_event_base_t base, int32_t eventId, void* eventData);
void handleMqttMessage(const char* topic, const char* data, int dataLen);
void sendStateReport();
void startPulse(int outPin, int durationMs);
void endPulse();
String buildCloudJson();
void publishAck(const String& cmdId, const String& status, int mappedOut, int pulseMs);

// ========================================
// パルスms値のクランプ
// ========================================
int clampPulseMs(int ms) {
  if (ms < PULSE_MS_MIN) return PULSE_MS_MIN;
  if (ms > PULSE_MS_MAX) return PULSE_MS_MAX;
  return ms;
}

// ========================================
// 出力ピン決定（invert対応）
// ========================================
int getOpenPin() {
  return is04Invert ? TRG_OUT2 : TRG_OUT1;
}

int getClosePin() {
  return is04Invert ? TRG_OUT1 : TRG_OUT2;
}

int getPhysIn1TargetPin() {
  return is04Invert ? TRG_OUT2 : TRG_OUT1;
}

int getPhysIn2TargetPin() {
  return is04Invert ? TRG_OUT1 : TRG_OUT2;
}

// ========================================
// is04設定読み込み
// ========================================
void loadIs04Config() {
  is04Dids = settings.getString("is04_dids", "00001234,00005678");
  is04Invert = settings.getInt("is04_invert", 0) == 1;
  is04PulseMs = clampPulseMs(settings.getInt("is04_pulse_ms", PULSE_MS_DEFAULT));
  is04InterlockMs = settings.getInt("is04_interlock_ms", INTERLOCK_MS_DEFAULT);

  Serial.printf("[CONFIG] is04_dids: %s\n", is04Dids.c_str());
  Serial.printf("[CONFIG] is04_invert: %d\n", is04Invert ? 1 : 0);
  Serial.printf("[CONFIG] is04_pulse_ms: %d\n", is04PulseMs);
  Serial.printf("[CONFIG] is04_interlock_ms: %d\n", is04InterlockMs);
}

// ========================================
// GPIO初期化
// ========================================
void initGPIO() {
  // 最重要: 起動直後に出力をLOWに設定（暴発防止）
  Serial.printf("[GPIO] Setting TRG_OUT1 (GPIO%d) to OUTPUT LOW...\n", TRG_OUT1);
  pinMode(TRG_OUT1, OUTPUT);
  digitalWrite(TRG_OUT1, LOW);
  Serial.printf("[GPIO] Setting TRG_OUT2 (GPIO%d) to OUTPUT LOW...\n", TRG_OUT2);
  pinMode(TRG_OUT2, OUTPUT);
  digitalWrite(TRG_OUT2, LOW);
  Serial.println("[GPIO] Output pins initialized to LOW");

  // 物理入力（INPUT_PULLDOWN: HIGHでアクティブ）
  Serial.printf("[GPIO] Setting PHYS_IN1 (GPIO%d) to INPUT_PULLDOWN...\n", PHYS_IN1);
  Serial.flush();
  pinMode(PHYS_IN1, INPUT_PULLDOWN);
  Serial.println("[GPIO] PHYS_IN1 pinMode done");
  Serial.flush();

  Serial.printf("[GPIO] Setting PHYS_IN2 (GPIO%d) to INPUT_PULLDOWN...\n", PHYS_IN2);
  Serial.flush();
  pinMode(PHYS_IN2, INPUT_PULLDOWN);
  Serial.println("[GPIO] PHYS_IN2 pinMode done");
  Serial.flush();

  delay(10);
  physIn1Raw = digitalRead(PHYS_IN1);
  physIn1Stable = physIn1Raw;
  physIn1LastStable = physIn1Raw;
  physIn1StableCount = STABLE_COUNT;

  physIn2Raw = digitalRead(PHYS_IN2);
  physIn2Stable = physIn2Raw;
  physIn2LastStable = physIn2Raw;
  physIn2StableCount = STABLE_COUNT;

  Serial.printf("[GPIO] PHYS_IN1(GPIO%d): %s\n", PHYS_IN1, physIn1Stable == HIGH ? "HIGH(active)" : "LOW(inactive)");
  Serial.printf("[GPIO] PHYS_IN2(GPIO%d): %s\n", PHYS_IN2, physIn2Stable == HIGH ? "HIGH(active)" : "LOW(inactive)");

  // ボタン
  Serial.println("[GPIO] Setting button pins...");
  pinMode(BTN_WIFI, INPUT_PULLUP);
  pinMode(BTN_RESET, INPUT_PULLUP);
  Serial.println("[GPIO] All GPIO initialized");
}

// ========================================
// MQTT接続
// ========================================
void connectMqtt() {
  if (mqttWsUrl.length() == 0) {
    Serial.println("[MQTT] No URL configured");
    return;
  }

  if (myLacisId.length() == 0 || myCic.length() == 0) {
    Serial.println("[MQTT] Missing credentials");
    return;
  }

  Serial.printf("[MQTT] Connecting to %s\n", mqttWsUrl.c_str());
  Serial.printf("[MQTT] Username: %s, Password: %s\n", myLacisId.c_str(), myCic.c_str());

  esp_mqtt_client_config_t mqttCfg = {};
  mqttCfg.broker.address.uri = mqttWsUrl.c_str();
  mqttCfg.credentials.username = myLacisId.c_str();
  mqttCfg.credentials.authentication.password = myCic.c_str();
  mqttCfg.network.timeout_ms = 10000;
  mqttCfg.session.keepalive = 30;

  mqttClient = esp_mqtt_client_init(&mqttCfg);
  if (!mqttClient) {
    Serial.println("[MQTT] Failed to init client");
    return;
  }

  esp_mqtt_client_register_event(mqttClient, MQTT_EVENT_ANY, mqttEventHandler, nullptr);
  esp_err_t err = esp_mqtt_client_start(mqttClient);
  if (err != ESP_OK) {
    Serial.printf("[MQTT] Failed to start: %d\n", err);
  }
}

// ========================================
// MQTTイベントハンドラ
// ========================================
void mqttEventHandler(void* handlerArgs, esp_event_base_t base, int32_t eventId, void* eventData) {
  esp_mqtt_event_handle_t event = (esp_mqtt_event_handle_t)eventData;

  switch (eventId) {
    case MQTT_EVENT_CONNECTED: {
      Serial.println("[MQTT] Connected");
      mqttConnected = true;

      // Subscribe to command and config topics
      String cmdTopic = "aranea/" + myTid + "/" + myLacisId + "/command";
      String cfgTopic = "aranea/" + myTid + "/" + myLacisId + "/config";

      esp_mqtt_client_subscribe(mqttClient, cmdTopic.c_str(), 1);
      esp_mqtt_client_subscribe(mqttClient, cfgTopic.c_str(), 1);

      Serial.printf("[MQTT] Subscribed: %s\n", cmdTopic.c_str());
      Serial.printf("[MQTT] Subscribed: %s\n", cfgTopic.c_str());
      break;
    }

    case MQTT_EVENT_DISCONNECTED:
      Serial.println("[MQTT] Disconnected");
      mqttConnected = false;
      break;

    case MQTT_EVENT_DATA:
      if (event->topic && event->data) {
        char* topic = (char*)malloc(event->topic_len + 1);
        char* data = (char*)malloc(event->data_len + 1);
        if (topic && data) {
          memcpy(topic, event->topic, event->topic_len);
          topic[event->topic_len] = '\0';
          memcpy(data, event->data, event->data_len);
          data[event->data_len] = '\0';
          handleMqttMessage(topic, data, event->data_len);
          free(topic);
          free(data);
        }
      }
      break;

    case MQTT_EVENT_ERROR:
      Serial.println("[MQTT] Error");
      break;

    default:
      break;
  }
}

// ========================================
// MQTTメッセージ処理
// ========================================
void handleMqttMessage(const char* topic, const char* data, int dataLen) {
  unsigned long tCmdRx = millis();
  Serial.printf("[MQTT] Topic: %s\n", topic);
  Serial.printf("[MQTT] Data: %s\n", data);

  // Parse JSON
  StaticJsonDocument<512> doc;
  DeserializationError error = deserializeJson(doc, data, dataLen);
  if (error) {
    Serial.printf("[MQTT] JSON parse error: %s\n", error.c_str());
    return;
  }

  // Check if command topic
  String topicStr = String(topic);
  if (topicStr.indexOf("/command") < 0) {
    Serial.println("[MQTT] Not a command topic, ignoring");
    return;
  }

  // Extract command fields
  String cmdId = doc["cmd_id"] | "";
  String cmdType = doc["type"] | "";
  int ttlSec = doc["ttlSec"] | 30;
  String issuedAt = doc["issuedAt"] | "";
  JsonObject params = doc["parameters"];

  if (cmdId.length() == 0 || cmdType.length() == 0) {
    Serial.println("[MQTT] Missing cmd_id or type");
    return;
  }

  // Check TTL (skip if too old)
  // TODO: Implement TTL check with NTP time

  // Get pulse duration from parameters or use default
  int pulseMs = params["pulseMs"] | is04PulseMs;
  pulseMs = clampPulseMs(pulseMs);

  // Determine output pin based on command type
  int targetPin = 0;
  bool validCmd = false;

  if (cmdType == "is04_open") {
    targetPin = getOpenPin();
    validCmd = true;
  } else if (cmdType == "is04_close") {
    targetPin = getClosePin();
    validCmd = true;
  } else if (cmdType == "is04_pulse1") {
    targetPin = TRG_OUT1;
    validCmd = true;
  } else if (cmdType == "is04_pulse2") {
    targetPin = TRG_OUT2;
    validCmd = true;
  }

  if (!validCmd) {
    Serial.printf("[CMD] Unknown command type: %s\n", cmdType.c_str());
    return;
  }

  // Check if pulse already active (reject if so)
  if (pulseActive) {
    Serial.println("[CMD] Pulse already active, rejecting");
    publishAck(cmdId, "failed", targetPin, pulseMs);
    return;
  }

  // Start pulse
  unsigned long tPulseStart = millis();
  startPulse(targetPin, pulseMs);

  // Send ACK immediately after pulse start (speed priority)
  unsigned long tAckPub = millis();
  publishAck(cmdId, "executed", targetPin, pulseMs);
  lastCmdId = cmdId;

  // Trigger state report
  needSendState = true;

  // Log timing
  Serial.printf("[CMD] rx=%lu start=%lu ack=%lu deltaStart=%lums deltaAck=%lums\n",
    tCmdRx, tPulseStart, tAckPub,
    tPulseStart - tCmdRx,
    tAckPub - tCmdRx);
}

// ========================================
// ACK送信
// ========================================
void publishAck(const String& cmdId, const String& status, int mappedOut, int pulseMs) {
  if (!mqttConnected || !mqttClient) return;

  String ackTopic = "aranea/" + myTid + "/" + myLacisId + "/ack";

  String executedAt = ntp.isSynced() ? ntp.getIso8601() : "1970-01-01T00:00:00Z";

  StaticJsonDocument<256> ackDoc;
  ackDoc["cmd_id"] = cmdId;
  ackDoc["status"] = status;
  ackDoc["executedAt"] = executedAt;

  JsonObject result = ackDoc.createNestedObject("result");
  result["ok"] = (status == "executed" || status == "success" || status == "completed" || status == "done");
  result["mappedOut"] = mappedOut;
  result["pulseMs"] = pulseMs;
  result["invert"] = is04Invert;

  char ackJson[256];
  serializeJson(ackDoc, ackJson, sizeof(ackJson));

  esp_mqtt_client_publish(mqttClient, ackTopic.c_str(), ackJson, 0, 1, 0);
  Serial.printf("[ACK] %s -> %s\n", ackTopic.c_str(), ackJson);
}

// ========================================
// パルス開始（非ブロッキング）
// ========================================
void startPulse(int outPin, int durationMs) {
  if (pulseActive) {
    Serial.println("[PULSE] Already active, ignoring");
    return;
  }

  pulseActive = true;
  pulseOutPin = outPin;
  pulseStartMs = millis();
  pulseDurationMs = durationMs;

  digitalWrite(outPin, HIGH);
  Serial.printf("[PULSE] Start GPIO%d for %dms\n", outPin, durationMs);
}

// ========================================
// パルス終了
// ========================================
void endPulse() {
  if (!pulseActive) return;

  digitalWrite(pulseOutPin, LOW);
  Serial.printf("[PULSE] End GPIO%d\n", pulseOutPin);

  pulseActive = false;
  pulseOutPin = 0;
  pulseStartMs = 0;
  pulseDurationMs = 0;

  // Trigger state report after pulse ends
  needSendState = true;
}

// ========================================
// パルス状態更新（loop内で呼び出し）
// ========================================
void updatePulse() {
  if (!pulseActive) return;

  unsigned long now = millis();
  if (now - pulseStartMs >= pulseDurationMs) {
    endPulse();
  }
}

// ========================================
// 物理入力サンプリング（デバウンス）
// ========================================
void samplePhysicalInputs() {
  // PHYS_IN1
  int current1 = digitalRead(PHYS_IN1);
  if (current1 == physIn1Raw) {
    if (physIn1StableCount < STABLE_COUNT * 2) {
      physIn1StableCount++;
    }
    if (physIn1StableCount >= STABLE_COUNT && current1 != physIn1Stable) {
      physIn1Stable = current1;
      Serial.printf("[PHYS] IN1(GPIO%d) -> %s\n", PHYS_IN1, physIn1Stable == HIGH ? "HIGH" : "LOW");
    }
  } else {
    physIn1Raw = current1;
    physIn1StableCount = 0;
  }

  // PHYS_IN2
  int current2 = digitalRead(PHYS_IN2);
  if (current2 == physIn2Raw) {
    if (physIn2StableCount < STABLE_COUNT * 2) {
      physIn2StableCount++;
    }
    if (physIn2StableCount >= STABLE_COUNT && current2 != physIn2Stable) {
      physIn2Stable = current2;
      Serial.printf("[PHYS] IN2(GPIO%d) -> %s\n", PHYS_IN2, physIn2Stable == HIGH ? "HIGH" : "LOW");
    }
  } else {
    physIn2Raw = current2;
    physIn2StableCount = 0;
  }
}

// ========================================
// 物理入力エッジ検出→パルス生成
// ========================================
void handlePhysicalInputEdges() {
  // PHYS_IN1: 立ち上がりエッジ検出
  if (physIn1Stable == HIGH && physIn1LastStable == LOW) {
    Serial.println("[PHYS] IN1 rising edge detected");
    if (!pulseActive) {
      int targetPin = getPhysIn1TargetPin();
      startPulse(targetPin, is04PulseMs);
      needSendState = true;
    } else {
      Serial.println("[PHYS] Pulse already active, ignoring");
    }
  }
  physIn1LastStable = physIn1Stable;

  // PHYS_IN2: 立ち上がりエッジ検出
  if (physIn2Stable == HIGH && physIn2LastStable == LOW) {
    Serial.println("[PHYS] IN2 rising edge detected");
    if (!pulseActive) {
      int targetPin = getPhysIn2TargetPin();
      startPulse(targetPin, is04PulseMs);
      needSendState = true;
    } else {
      Serial.println("[PHYS] Pulse already active, ignoring");
    }
  }
  physIn2LastStable = physIn2Stable;

  // 物理入力状態変化時も状態送信（Trigger*_Physical用）
  static int lastPhysIn1Stable = -1;
  static int lastPhysIn2Stable = -1;
  if (lastPhysIn1Stable >= 0 && physIn1Stable != lastPhysIn1Stable) {
    needSendState = true;
  }
  if (lastPhysIn2Stable >= 0 && physIn2Stable != lastPhysIn2Stable) {
    needSendState = true;
  }
  lastPhysIn1Stable = physIn1Stable;
  lastPhysIn2Stable = physIn2Stable;
}

// ========================================
// クラウド送信用JSON生成（deviceStateReport用）
// ========================================
String buildCloudJson() {
  String observedAt = ntp.isSynced() ? ntp.getIso8601() : "1970-01-01T00:00:00Z";
  String ip = wifi.getIP();
  int rssi = WiFi.RSSI();

  StaticJsonDocument<512> doc;

  // auth
  JsonObject auth = doc.createNestedObject("auth");
  auth["tid"] = myTid;
  auth["lacisId"] = myLacisId;
  auth["cic"] = myCic;

  // report
  JsonObject report = doc.createNestedObject("report");
  report["lacisId"] = myLacisId;
  report["type"] = DEVICE_TYPE;
  report["observedAt"] = observedAt;

  // state (stateSnapshot)
  JsonObject state = report.createNestedObject("state");
  state["Trigger1"] = (pulseActive && pulseOutPin == TRG_OUT1);
  state["Trigger2"] = (pulseActive && pulseOutPin == TRG_OUT2);
  state["Trigger3"] = false;  // 未使用
  state["Trigger1_Physical"] = (physIn1Stable == HIGH);
  state["Trigger2_Physical"] = (physIn2Stable == HIGH);
  state["Trigger3_Physical"] = false;  // 未使用
  state["IPAddress"] = ip;
  state["MacAddress"] = myMac;
  state["RSSI"] = rssi;

  String json;
  serializeJson(doc, json);
  return json;
}

// ========================================
// HTTP POST送信
// ========================================
bool postToUrl(const String& url, const String& json) {
  if (url.length() == 0) return false;

  HTTPClient http;
  http.begin(url);
  http.addHeader("Content-Type", "application/json");
  http.setTimeout(10000);

  int httpCode = http.POST(json);
  bool success = (httpCode >= 200 && httpCode < 300);

  if (success) {
    Serial.printf("[HTTP] OK %d -> %s\n", httpCode, url.c_str());
    lastSendResult = "OK";
  } else {
    Serial.printf("[HTTP] NG %d -> %s\n", httpCode, url.c_str());
    lastSendResult = "NG";
  }

  http.end();
  return success;
}

// ========================================
// 状態送信（deviceStateReport）
// ========================================
void sendStateReport() {
  if (cloudUrl.length() == 0) {
    Serial.println("[SEND] No cloud URL configured");
    return;
  }

  if (myTid.length() == 0 || myCic.length() == 0) {
    Serial.println("[SEND] Missing TID or CIC");
    return;
  }

  String json = buildCloudJson();
  Serial.printf("[SEND] deviceStateReport: %s\n", json.c_str());
  postToUrl(cloudUrl, json);
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
      settings.clear();
      ESP.restart();
    }
  } else {
    btnResetPressTime = 0;
  }
}

// ========================================
// ディスプレイ更新（is02パターン）
// ========================================
void updateDisplay() {
  // Line1: IP + RSSI
  String line1 = wifi.getIP() + " " + String(WiFi.RSSI()) + "dBm";

  // CIC
  String cicStr = myCic.length() > 0 ? myCic : "------";

  // sensorLine: 出力状態表示
  String sensorLine = "";
  if (pulseActive) {
    sensorLine = "OUT" + String(pulseOutPin == TRG_OUT1 ? "1" : "2") + " PULSE";
  } else {
    sensorLine = "READY";
  }

  // MQTT接続状態
  if (mqttConnected) {
    sensorLine += " [M]";
  }

  display.showIs02Main(line1, cicStr, sensorLine, pulseActive);
}

// ========================================
// Setup
// ========================================
void setup() {
  Serial.begin(115200);
  delay(100);
  Serial.println("\n[BOOT] is04 starting...");

  // 最重要: 起動直後に出力をLOWに設定（暴発防止）
  initGPIO();

  // 0. Operator初期化
  op.setMode(OperatorMode::PROVISION);

  // 1. SettingManager初期化
  if (!settings.begin()) {
    Serial.println("[ERROR] Settings init failed");
    return;
  }

  // 2. DisplayManager初期化
  if (!display.begin()) {
    Serial.println("[WARNING] Display init failed");
  }
  display.showBoot("Booting...");

  // 3. WiFi接続
  display.showConnecting("WiFi...", 0);
  if (!wifi.connectWithSettings(&settings)) {
    Serial.println("[ERROR] WiFi connection failed");
    display.showError("WiFi Failed");
    delay(5000);
    ESP.restart();
  }
  Serial.printf("[WIFI] Connected: %s\n", wifi.getIP().c_str());

  // 4. NTP同期
  display.showBoot("NTP sync...");
  if (!ntp.sync()) {
    Serial.println("[WARNING] NTP sync failed");
  }

  // 5. LacisID生成
  myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
  myMac = lacisGen.getStaMac12Hex();
  Serial.printf("[LACIS] ID: %s, MAC: %s\n", myLacisId.c_str(), myMac.c_str());

  // 6. AraneaGateway登録
  display.showRegistering("Registering...");
  String gateUrl = settings.getString("gate_url", "");
  araneaReg.begin(gateUrl);

  TenantPrimaryAuth tenantAuth;
  tenantAuth.lacisId = settings.getString("tenant_lacisid", "");
  tenantAuth.userId = settings.getString("tenant_email", "");
  tenantAuth.pass = settings.getString("tenant_pass", "");
  tenantAuth.cic = settings.getString("tenant_cic", "");
  araneaReg.setTenantPrimary(tenantAuth);

  myTid = settings.getString("tid", "");
  AraneaRegisterResult regResult = araneaReg.registerDevice(
    myTid, DEVICE_TYPE, myLacisId, myMac, PRODUCT_TYPE, PRODUCT_CODE
  );

  if (regResult.ok) {
    myCic = regResult.cic_code;
    settings.setString("cic", myCic);
    Serial.printf("[REG] CIC: %s\n", myCic.c_str());
  } else {
    myCic = settings.getString("cic", "");
    Serial.printf("[REG] Failed, using saved CIC: %s\n", myCic.c_str());
  }

  // 7. 送信先URL読み込み
  cloudUrl = settings.getString("cloud_url", "https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport");
  mqttWsUrl = settings.getString("mqtt_ws_url", "");
  Serial.printf("[CONFIG] Cloud: %s\n", cloudUrl.c_str());
  Serial.printf("[CONFIG] MQTT: %s\n", mqttWsUrl.length() > 0 ? mqttWsUrl.c_str() : "(none)");

  // 8. is04設定読み込み
  loadIs04Config();

  // 9. OtaManager初期化
  String hostname = "is04-" + myMac.substring(8);
  ota.begin(hostname, "");
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

  // 10. HttpManager開始
  httpMgr.begin(&settings, 80);
  httpMgr.setDeviceInfo(DEVICE_TYPE, myLacisId, myCic, FIRMWARE_VERSION);

  // 11. MQTT接続
  if (mqttWsUrl.length() > 0) {
    display.showBoot("MQTT...");
    connectMqtt();
  }

  // 12. RUNモードへ
  op.setMode(OperatorMode::RUN);

  // 13. タイミング初期化
  lastHeartbeatTime = millis();

  Serial.println("[BOOT] is04 ready!");
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

  // パルス状態更新（非ブロッキング）
  updatePulse();

  // 物理入力サンプリング（20ms間隔）
  if (now - lastSampleTime >= SAMPLE_INTERVAL_MS) {
    samplePhysicalInputs();
    handlePhysicalInputEdges();
    lastSampleTime = now;
  }

  // 状態送信
  if (needSendState) {
    sendStateReport();
    needSendState = false;
  }

  // 心拍送信（60秒間隔）
  if (now - lastHeartbeatTime >= HEARTBEAT_INTERVAL_MS) {
    heartbeatCount++;
    Serial.printf("[HEARTBEAT] #%d\n", heartbeatCount);
    sendStateReport();
    lastHeartbeatTime = now;
  }

  // ディスプレイ更新（1秒間隔）
  if (now - lastDisplayUpdate >= DISPLAY_UPDATE_MS) {
    updateDisplay();
    lastDisplayUpdate = now;
  }

  // ボタン処理
  handleButtons();

  // MQTT再接続チェック
  if (mqttWsUrl.length() > 0 && !mqttConnected) {
    if (now - lastMqttReconnect >= MQTT_RECONNECT_INTERVAL) {
      Serial.println("[MQTT] Attempting reconnect...");
      if (mqttClient) {
        esp_mqtt_client_reconnect(mqttClient);
      } else {
        connectMqtt();
      }
      lastMqttReconnect = now;
    }
  }

  // WiFi再接続チェック
  if (!wifi.isConnected()) {
    Serial.println("[WIFI] Disconnected, reconnecting...");
    display.showConnecting("Reconnect...", 0);
    wifi.connectWithSettings(&settings);
  }

  delay(10);
}
