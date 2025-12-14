/**
 * is02 - BLE Gateway Device
 *
 * is01のBLEアドバタイズを受信し、ローカルis03（Zero3）へHTTP中継する
 *
 * 機能:
 * - BLEスキャン（NimBLE）
 * - ISMSv1ペイロードのパース
 * - HTTP POST（primary/secondary フォールバック）
 * - OLED表示
 */

#include <Arduino.h>
#include <NimBLEDevice.h>
#include <ArduinoJson.h>
#include <map>
#include "AraneaGlobal.h"

// ========================================
// 定数
// ========================================
static const char* PRODUCT_TYPE = "002";
static const char* PRODUCT_CODE = "0096";

// 送信レート制限（同一センサーに対して最短間隔）
static const unsigned long SEND_INTERVAL_MS = 3000;

// イベントキューサイズ
static const int EVENT_QUEUE_SIZE = 10;

// 表示更新間隔
static const unsigned long DISPLAY_UPDATE_MS = 1000;

// ========================================
// センサーイベント構造体
// ========================================
struct SensorEvent {
  String sensorLacisId;
  String staMac12Hex;
  float tempC;
  float humPct;
  uint8_t battPct;
  int rssi;
  String rawMfgHex;
  unsigned long receivedAt;
  bool valid;
};

// ========================================
// グローバル変数
// ========================================
// モジュールインスタンス
SettingManager settings;
DisplayManager display;
WiFiManager wifi;
NtpManager ntp;
LacisIDGenerator lacisGen;
HttpRelayClient* httpClient = nullptr;

// 自機情報
String myLacisId;
String myIp;

// イベントキュー
SensorEvent eventQueue[EVENT_QUEUE_SIZE];
volatile int queueHead = 0;
volatile int queueTail = 0;
portMUX_TYPE queueMux = portMUX_INITIALIZER_UNLOCKED;

// 送信レート制限用（最終送信時刻）
std::map<String, unsigned long> lastSendTime;

// 最終受信情報（表示用）
String lastSensorId = "";
float lastTempC = 0;
float lastHumPct = 0;
int lastRssi = 0;
bool lastSendOk = true;

// 表示更新タイミング
unsigned long lastDisplayUpdate = 0;

// BLEスキャン
NimBLEScan* pBLEScan = nullptr;

// ========================================
// イベントキュー操作
// ========================================
bool queuePush(const SensorEvent& evt) {
  portENTER_CRITICAL(&queueMux);
  int nextTail = (queueTail + 1) % EVENT_QUEUE_SIZE;
  if (nextTail == queueHead) {
    // キュー満杯: 古いものを破棄
    queueHead = (queueHead + 1) % EVENT_QUEUE_SIZE;
  }
  eventQueue[queueTail] = evt;
  queueTail = nextTail;
  portEXIT_CRITICAL(&queueMux);
  return true;
}

bool queuePop(SensorEvent* evt) {
  portENTER_CRITICAL(&queueMux);
  if (queueHead == queueTail) {
    portEXIT_CRITICAL(&queueMux);
    return false;
  }
  *evt = eventQueue[queueHead];
  queueHead = (queueHead + 1) % EVENT_QUEUE_SIZE;
  portEXIT_CRITICAL(&queueMux);
  return true;
}

// ========================================
// 送信レート制限チェック
// ========================================
bool canSend(const String& sensorLacisId) {
  auto it = lastSendTime.find(sensorLacisId);
  if (it == lastSendTime.end()) {
    return true;
  }
  return (millis() - it->second) >= SEND_INTERVAL_MS;
}

void updateLastSendTime(const String& sensorLacisId) {
  lastSendTime[sensorLacisId] = millis();
}

// ========================================
// JSON生成
// ========================================
String buildJson(const SensorEvent& evt) {
  StaticJsonDocument<512> doc;

  // gateway
  JsonObject gateway = doc.createNestedObject("gateway");
  gateway["lacisId"] = myLacisId;
  gateway["ip"] = myIp;
  gateway["rssi"] = wifi.getRSSI();

  // sensor
  JsonObject sensor = doc.createNestedObject("sensor");
  sensor["lacisId"] = evt.sensorLacisId;
  sensor["mac"] = evt.staMac12Hex;
  sensor["productType"] = "001";  // is01固定
  sensor["productCode"] = PRODUCT_CODE;

  // observedAt
  doc["observedAt"] = ntp.getIso8601();

  // state
  JsonObject state = doc.createNestedObject("state");
  state["temperatureC"] = evt.tempC;
  state["humidityPct"] = evt.humPct;
  state["batteryPct"] = evt.battPct;

  // raw
  JsonObject raw = doc.createNestedObject("raw");
  raw["mfgHex"] = evt.rawMfgHex;

  String json;
  serializeJson(doc, json);
  return json;
}

// ========================================
// BLEスキャンコールバック (NimBLE 2.x API)
// ========================================
class ScanCallbacks : public NimBLEScanCallbacks {
  void onResult(const NimBLEAdvertisedDevice* advertisedDevice) override {
    // Manufacturer Data 取得
    if (!advertisedDevice->haveManufacturerData()) return;

    std::string mfgData = advertisedDevice->getManufacturerData();

    // ISMSv1 判定
    if (!BleIsmsParser::isIsmsV1(mfgData)) return;

    // パース
    IsmsPayload payload;
    if (!BleIsmsParser::parse(mfgData, &payload)) return;

    // is01 の lacisId を再構成
    String sensorLacisId = LacisIDGenerator::reconstructFromMac(
      payload.productType, payload.staMac, PRODUCT_CODE);

    Serial.printf("[BLE] ISMSv1: mac=%s, temp=%.2fC, hum=%.2f%%, rssi=%d\n",
      payload.staMac12Hex.c_str(), payload.tempC, payload.humPct,
      advertisedDevice->getRSSI());

    // イベントをキューに追加
    SensorEvent evt;
    evt.sensorLacisId = sensorLacisId;
    evt.staMac12Hex = payload.staMac12Hex;
    evt.tempC = payload.tempC;
    evt.humPct = payload.humPct;
    evt.battPct = payload.battPct;
    evt.rssi = advertisedDevice->getRSSI();
    evt.rawMfgHex = payload.rawHex;
    evt.receivedAt = millis();
    evt.valid = true;

    queuePush(evt);
  }
};

// ========================================
// 表示更新
// ========================================
void updateDisplay() {
  // 1行目: IP
  String line1 = myIp;

  // 2行目: 自機lacisId末尾
  String line2 = "ID:..." + myLacisId.substring(myLacisId.length() - 8);

  // 3行目: 最終受信センサー
  String line3;
  if (lastSensorId.length() > 0) {
    line3 = "L:..." + lastSensorId.substring(lastSensorId.length() - 4) +
            " " + String(lastRssi) + "dBm";
  } else {
    line3 = "Waiting...";
  }

  // 4行目: 温湿度 + 送信状態
  String line4;
  if (lastSensorId.length() > 0) {
    line4 = String(lastTempC, 1) + "C " + String(lastHumPct, 0) + "% ";
    line4 += lastSendOk ? "SEND:OK" : "SEND:NG";
  } else {
    line4 = "---";
  }

  display.show4Lines(line1, line2, line3, line4);
}

// ========================================
// Setup
// ========================================
void setup() {
  Serial.begin(115200);
  delay(100);
  Serial.println("\n[BOOT] is02 starting...");

  // 1. SettingManager
  settings.begin();

  // 2. DisplayManager
  if (!display.begin()) {
    Serial.println("[ERROR] Display init failed");
  }
  display.showBoot("Booting...");

  // 3. WiFiManager
  display.showBoot("WiFi...");
  wifi.connectDefault();
  myIp = wifi.getIP();

  // 4. NtpManager
  display.showBoot("NTP sync...");
  ntp.sync();  // 失敗しても続行

  // 5. LacisIDGenerator
  myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
  Serial.printf("[LACIS] My lacisId: %s\n", myLacisId.c_str());

  // 6. HttpRelayClient
  String primaryUrl = settings.getString("relay_pri", "http://192.168.96.201:8080/api/ingest");
  String secondaryUrl = settings.getString("relay_sec", "http://192.168.96.202:8080/api/ingest");
  httpClient = new HttpRelayClient(primaryUrl, secondaryUrl);
  Serial.printf("[HTTP] Primary: %s\n", primaryUrl.c_str());
  Serial.printf("[HTTP] Secondary: %s\n", secondaryUrl.c_str());

  // 7. BLEスキャン開始 (NimBLE 2.x API)
  display.showBoot("BLE init...");
  NimBLEDevice::init("is02");
  pBLEScan = NimBLEDevice::getScan();
  pBLEScan->setScanCallbacks(new ScanCallbacks(), false);
  pBLEScan->setActiveScan(true);
  pBLEScan->setInterval(100);
  pBLEScan->setWindow(99);
  pBLEScan->start(0, true);  // 継続スキャン (duration=0, isContinue=true)
  Serial.println("[BLE] Scan started");

  // 初期表示
  updateDisplay();
  Serial.println("[BOOT] Ready");
}

// ========================================
// Loop
// ========================================
void loop() {
  // 1. イベント処理
  SensorEvent evt;
  while (queuePop(&evt)) {
    // 送信レート制限チェック
    if (!canSend(evt.sensorLacisId)) {
      Serial.printf("[SKIP] Rate limited: %s\n", evt.sensorLacisId.c_str());
      continue;
    }

    // JSON生成
    String json = buildJson(evt);

    // HTTP POST
    HttpRelayResult result = httpClient->postJson(json);

    // 送信時刻更新
    updateLastSendTime(evt.sensorLacisId);

    // 表示用情報更新
    lastSensorId = evt.sensorLacisId;
    lastTempC = evt.tempC;
    lastHumPct = evt.humPct;
    lastRssi = evt.rssi;
    lastSendOk = result.ok;
  }

  // 2. 表示更新（1秒ごと）
  if (millis() - lastDisplayUpdate > DISPLAY_UPDATE_MS) {
    updateDisplay();
    lastDisplayUpdate = millis();
  }

  // 3. WiFi再接続チェック
  if (!wifi.isConnected()) {
    Serial.println("[WIFI] Disconnected, reconnecting...");
    display.showError("WiFi lost");
    wifi.connectDefault();
    myIp = wifi.getIP();
  }

  delay(10);
}
