/**
 * is02 - BLE Gateway Device
 *
 * is01のBLEアドバタイズを受信し、ローカルis03（Zero3）へHTTP中継する
 * is02自身も温湿度センサー（HT-30）を持ち、自己計測データも送信
 *
 * 機能:
 * - BLEスキャン（NimBLE）でis01データ受信
 * - バッチ送信（30秒間隔）
 * - is02自身のセンサーデータ送信
 * - HTTP POST（primary/secondary フォールバック）
 * - OLED表示
 * - OTAアップデート
 * - Web UI（設定・ステータス）
 * - 定期再起動スケジューラ
 */

#include <Arduino.h>
#include <NimBLEDevice.h>
#include <ArduinoJson.h>
#include <vector>
#include <Wire.h>
#include <SHT31.h>
#include "AraneaGlobal.h"

// ========================================
// 定数（固定値のみ - 変更可能な設定はsettingManager経由）
// ========================================
static const char* PRODUCT_TYPE = "002";
static const char* PRODUCT_CODE = "0096";
static const char* DEVICE_TYPE = "ISMS_ar-is02";
static const char* FIRMWARE_VERSION = "1.0.0";

// バッチ送信間隔（30秒）
static const unsigned long BATCH_INTERVAL_MS = 30000;

// バッチ最大サイズ
static const int BATCH_MAX_SIZE = 50;

// 表示更新間隔
static const unsigned long DISPLAY_UPDATE_MS = 1000;

// 再起動チェック間隔（1分）
static const unsigned long REBOOT_CHECK_MS = 60000;

// ========================================
// センサーイベント構造体
// ========================================
struct SensorEvent {
  String lacisId;       // センサーのlacisId（is01 or is02）
  String mac;           // MACアドレス（12桁HEX）
  String productType;   // 製品タイプ
  String productCode;   // 製品コード
  float tempC;
  float humPct;
  uint8_t battPct;
  int rssi;             // BLE RSSI（リレー時のみ）
  String observedAt;    // 観測時刻（ISO8601）
  bool isDirect;        // true=is02自身の計測, false=BLEリレー
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
AraneaRegister araneaReg;
HttpRelayClient* httpClient = nullptr;

// 新規追加モジュール
Operator op;
OtaManager ota;
HttpManager httpMgr;
RebootScheduler rebootSched;
SPIFFSManager spiffs;

// HT-30センサー（SHT31互換、I2Cアドレス0x44）
SHT31 sht(0x44);
bool sensorReady = false;

// 自機情報
String myLacisId;
String myMac;
String myIp;

// バッチバッファ
std::vector<SensorEvent> batchBuffer;
portMUX_TYPE batchMux = portMUX_INITIALIZER_UNLOCKED;

// 最終バッチ送信時刻
unsigned long lastBatchSend = 0;

// 統計情報（表示用）
int totalReceived = 0;
int totalSent = 0;
int batchCount = 0;
bool lastSendOk = true;

// 表示用状態
String myCic = "";           // 登録後のCIC
bool isSending = false;      // データ送信中フラグ
float lastTempC = NAN;       // 最新温度
float lastHumPct = NAN;      // 最新湿度

// 表示更新タイミング
unsigned long lastDisplayUpdate = 0;
unsigned long lastRebootCheck = 0;

// BLEスキャン
NimBLEScan* pBLEScan = nullptr;

// ========================================
// バッチバッファ操作
// ========================================
void addToBatch(const SensorEvent& evt) {
  portENTER_CRITICAL(&batchMux);
  if (batchBuffer.size() < BATCH_MAX_SIZE) {
    batchBuffer.push_back(evt);
  }
  portEXIT_CRITICAL(&batchMux);
}

std::vector<SensorEvent> getAndClearBatch() {
  portENTER_CRITICAL(&batchMux);
  std::vector<SensorEvent> copy = batchBuffer;
  batchBuffer.clear();
  portEXIT_CRITICAL(&batchMux);
  return copy;
}

int getBatchSize() {
  portENTER_CRITICAL(&batchMux);
  int size = batchBuffer.size();
  portEXIT_CRITICAL(&batchMux);
  return size;
}

// ========================================
// JSON生成（仕様準拠）
// ========================================
String buildEventJson(const SensorEvent& evt) {
  StaticJsonDocument<512> doc;

  // sensor（データの主体）
  JsonObject sensor = doc.createNestedObject("sensor");
  sensor["mac"] = evt.mac;
  sensor["productType"] = evt.productType;
  sensor["productCode"] = evt.productCode;
  sensor["lacisId"] = evt.lacisId;

  // state
  JsonObject state = doc.createNestedObject("state");
  state["temperature"] = evt.tempC;
  state["humidity"] = evt.humPct;
  state["battery"] = evt.battPct;

  // meta
  JsonObject meta = doc.createNestedObject("meta");
  meta["observedAt"] = evt.observedAt;

  if (evt.isDirect) {
    // is02自身の計測
    meta["direct"] = true;
  } else {
    // BLEリレー
    meta["rssi"] = evt.rssi;
    meta["gatewayId"] = myLacisId;
  }

  String json;
  serializeJson(doc, json);
  return json;
}

// ========================================
// バッチ送信
// ========================================
void sendBatch() {
  // OTA更新中は送信しない
  if (op.isOtaUpdating()) {
    Serial.println("[BATCH] Skipped: OTA in progress");
    return;
  }

  std::vector<SensorEvent> events = getAndClearBatch();

  if (events.empty()) {
    return;
  }

  Serial.printf("[BATCH] Sending %d events...\n", events.size());

  // 送信中フラグON → 表示更新
  isSending = true;
  updateDisplay();

  int successCount = 0;
  for (const auto& evt : events) {
    String json = buildEventJson(evt);
    HttpRelayResult result = httpClient->postJson(json);

    if (result.ok) {
      successCount++;
    } else {
      Serial.printf("[BATCH] Failed to send: %s\n", evt.lacisId.c_str());
    }
  }

  // 送信中フラグOFF
  isSending = false;

  totalSent += successCount;
  batchCount++;
  lastSendOk = (successCount == events.size());

  Serial.printf("[BATCH] Sent %d/%d events (batch #%d)\n",
    successCount, events.size(), batchCount);
}

// ========================================
// is02自身のセンサー読み取り（HT-30）
// ========================================
SensorEvent readSelfSensor() {
  SensorEvent evt;
  evt.lacisId = myLacisId;
  evt.mac = myMac;
  evt.productType = PRODUCT_TYPE;
  evt.productCode = PRODUCT_CODE;
  evt.observedAt = ntp.getIso8601();
  evt.isDirect = true;
  evt.rssi = 0;
  evt.battPct = 100;  // 常時電源

  // HT-30センサーから実際の値を読み取る
  if (sensorReady && sht.read()) {
    evt.tempC = sht.getTemperature();
    evt.humPct = sht.getHumidity();
    lastTempC = evt.tempC;    // 表示用に保存
    lastHumPct = evt.humPct;
    Serial.printf("[SENSOR] HT-30: %.2fC, %.2f%%\n", evt.tempC, evt.humPct);
  } else {
    // センサー読み取り失敗時はNaN
    evt.tempC = NAN;
    evt.humPct = NAN;
    lastTempC = NAN;
    lastHumPct = NAN;
    Serial.println("[SENSOR] HT-30 read failed");
  }

  return evt;
}

// ========================================
// 表示更新
// ========================================
void updateDisplay() {
  // OTA更新中は専用表示
  if (op.isOtaUpdating()) {
    display.showBoot("OTA: " + String(ota.progress()) + "%");
    return;
  }

  // Line1: IP + RSSI（接続済み）or 未接続状態
  String line1;
  if (wifi.isConnected()) {
    int rssi = WiFi.RSSI();
    char buf[32];
    snprintf(buf, sizeof(buf), "%s %ddBm", myIp.c_str(), rssi);
    line1 = String(buf);
  } else {
    line1 = "No WiFi";
  }

  // CIC（未登録時は"------"）
  String cicDisplay = myCic.length() > 0 ? myCic : "------";

  // センサー情報（温度/湿度 or エラー）- 短い形式でsize=2用
  String sensorLine;
  if (isnan(lastTempC) || isnan(lastHumPct)) {
    sensorLine = "ERR";
  } else {
    char buf[16];
    snprintf(buf, sizeof(buf), "%.1fC %.0f%%", lastTempC, lastHumPct);
    sensorLine = String(buf);
  }

  display.showIs02Main(line1, cicDisplay, sensorLine, isSending);
}

// ========================================
// BLEスキャンコールバック (NimBLE 2.x API)
// ========================================
class ScanCallbacks : public NimBLEScanCallbacks {
  void onResult(const NimBLEAdvertisedDevice* advertisedDevice) override {
    // OTA更新中はスキップ
    if (op.isOtaUpdating()) return;

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

    // イベント作成
    SensorEvent evt;
    evt.lacisId = sensorLacisId;
    evt.mac = payload.staMac12Hex;
    evt.productType = String(payload.productType);
    if (evt.productType.length() < 3) {
      evt.productType = "00" + evt.productType;
      evt.productType = evt.productType.substring(evt.productType.length() - 3);
    }
    evt.productCode = PRODUCT_CODE;
    evt.tempC = payload.tempC;
    evt.humPct = payload.humPct;
    evt.battPct = payload.battPct;
    evt.rssi = advertisedDevice->getRSSI();
    evt.observedAt = ntp.getIso8601();
    evt.isDirect = false;

    // バッチに追加
    addToBatch(evt);
    totalReceived++;
  }
};

// ========================================
// Setup
// ========================================
void setup() {
  Serial.begin(115200);
  delay(100);
  Serial.println("\n[BOOT] is02 starting...");

  // 0. GPIO5をHIGHに設定（I2C用MOSFET有効化 - is01と同じ基板）
  pinMode(5, OUTPUT);
  digitalWrite(5, HIGH);
  delay(50);  // MOSFET安定待ち

  // 0.5. Operator初期化
  op.setMode(OperatorMode::PROVISION);

  // 1. SettingManager - 設定をNVSから読み込み（デフォルト値は自動投入）
  settings.begin();
  // NOTE: 新しいデフォルト値を適用したい場合は settings.clear() を有効化

  // 1.5. SPIFFS初期化（ログ保存用）
  spiffs.begin();

  // 2. DisplayManager
  if (!display.begin()) {
    Serial.println("[ERROR] Display init failed");
  }
  display.showBoot("Booting...");

  // 2.5. HT-30センサー初期化（SHT31互換、I2Cアドレス0x44）
  // Wire.begin()はDisplayManagerで既に実行済み
  delay(100);  // I2Cバス安定待ち
  sht.begin();
  delay(50);
  if (sht.read()) {
    sensorReady = true;
    lastTempC = sht.getTemperature();
    lastHumPct = sht.getHumidity();
    Serial.printf("[SENSOR] HT-30 ready: %.2fC, %.2f%%\n", lastTempC, lastHumPct);
  } else {
    sensorReady = false;
    lastTempC = NAN;
    lastHumPct = NAN;
    Serial.println("[SENSOR] HT-30 not found or read failed");
  }

  // 3. WiFiManager（接続中ティッカー表示）- NVS設定を使用
  display.showConnecting("WiFi...", 0);
  wifi.connectWithSettings(&settings);
  myIp = wifi.getIP();

  // 4. NtpManager
  display.showBoot("NTP sync...");
  ntp.sync();  // 失敗しても続行

  // 5. LacisIDGenerator
  myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
  myMac = myLacisId.substring(4, 16);  // MACアドレス部分を抽出
  Serial.printf("[LACIS] My lacisId: %s\n", myLacisId.c_str());

  // 6. AraneaRegister（クラウド登録）- 設定はsettingManager経由
  display.showRegistering("Connecting...");
  String gateUrl = settings.getString("gate_url", "");
  araneaReg.begin(gateUrl);

  TenantPrimaryAuth tenantAuth;
  tenantAuth.lacisId = settings.getString("tenant_lacisid", "");
  tenantAuth.userId = settings.getString("tenant_email", "");
  tenantAuth.pass = settings.getString("tenant_pass", "");
  tenantAuth.cic = settings.getString("tenant_cic", "");
  araneaReg.setTenantPrimary(tenantAuth);

  display.showRegistering("Registering...");
  String tid = settings.getString("tid", "");
  AraneaRegisterResult regResult = araneaReg.registerDevice(
    tid, DEVICE_TYPE, myLacisId, myMac, PRODUCT_TYPE, PRODUCT_CODE
  );

  if (regResult.ok) {
    myCic = regResult.cic_code;  // 表示用にCICを保存
    Serial.printf("[ARANEA] CIC: %s\n", myCic.c_str());
    display.showRegistering("CIC: " + myCic);
    delay(500);
  } else {
    // 登録失敗でも保存済みCICがあれば使用
    myCic = araneaReg.getSavedCic();
    Serial.printf("[ARANEA] Registration failed: %s (continuing anyway)\n", regResult.error.c_str());
    if (myCic.length() > 0) {
      display.showRegistering("Saved CIC: " + myCic);
    } else {
      display.showRegistering("No CIC");
    }
    delay(500);
  }

  // 7. HttpRelayClient
  String primaryUrl = settings.getString("relay_pri", "http://192.168.96.201:8080/api/events");
  String secondaryUrl = settings.getString("relay_sec", "http://192.168.96.202:8080/api/events");
  httpClient = new HttpRelayClient(primaryUrl, secondaryUrl);
  Serial.printf("[HTTP] Primary: %s\n", primaryUrl.c_str());
  Serial.printf("[HTTP] Secondary: %s\n", secondaryUrl.c_str());

  // 8. OtaManager初期化
  display.showBoot("OTA init...");
  String hostname = settings.getString("hostname", "is02-" + myMac.substring(8));
  ota.begin(hostname, "");  // パスワードなし
  ota.onStart([]() {
    op.setOtaUpdating(true);
    op.setMode(OperatorMode::MAINTENANCE);
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
    op.setMode(OperatorMode::RUN);
    display.showError("OTA: " + err);
  });

  // 9. HttpManager（Web UI）初期化
  display.showBoot("HTTP init...");
  httpMgr.begin(&settings, 80);
  httpMgr.setDeviceInfo(DEVICE_TYPE, myLacisId, myCic, FIRMWARE_VERSION);
  httpMgr.setSensorValues(lastTempC, lastHumPct);

  // 10. RebootScheduler初期化
  int rebootHour = settings.getInt("reboot_hour", -1);  // デフォルト無効
  rebootSched.begin(rebootHour);

  // 11. BLEスキャン開始 (NimBLE 2.x API)
  display.showBoot("BLE init...");
  NimBLEDevice::init("is02");
  pBLEScan = NimBLEDevice::getScan();
  pBLEScan->setScanCallbacks(new ScanCallbacks(), false);
  pBLEScan->setActiveScan(true);
  pBLEScan->setInterval(100);
  pBLEScan->setWindow(99);
  pBLEScan->start(0, true);  // 継続スキャン (duration=0, isContinue=true)
  Serial.println("[BLE] Scan started");

  // 12. 通常動作モードへ移行
  op.setMode(OperatorMode::RUN);

  // 初期表示
  lastBatchSend = millis();
  lastRebootCheck = millis();
  updateDisplay();
  Serial.println("[BOOT] Ready");
  Serial.printf("[BOOT] Web UI: http://%s/\n", myIp.c_str());
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
    return;  // OTA中は他の処理をスキップ
  }

  // HTTP Server処理
  httpMgr.handle();

  // 1. バッチ送信タイミングチェック
  if (now - lastBatchSend >= BATCH_INTERVAL_MS) {
    // is02自身のセンサーデータを追加
    SensorEvent selfEvt = readSelfSensor();
    addToBatch(selfEvt);

    // HttpManager用にセンサー値更新
    httpMgr.setSensorValues(lastTempC, lastHumPct);

    // バッチ送信
    sendBatch();
    lastBatchSend = now;
  }

  // 2. 表示更新（1秒ごと）
  if (now - lastDisplayUpdate > DISPLAY_UPDATE_MS) {
    updateDisplay();
    lastDisplayUpdate = now;
  }

  // 3. 再起動スケジューラチェック（1分ごと）
  if (now - lastRebootCheck > REBOOT_CHECK_MS) {
    rebootSched.check();
    lastRebootCheck = now;
  }

  // 4. WiFi再接続チェック（NVS設定 + リトライ上限）
  if (!wifi.isConnected()) {
    Serial.println("[WIFI] Disconnected, reconnecting...");
    display.showError("WiFi lost");
    wifi.connectWithSettings(&settings);
    myIp = wifi.getIP();
  }

  delay(10);
}
