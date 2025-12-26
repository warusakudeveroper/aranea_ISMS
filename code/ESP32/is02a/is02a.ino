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

// Aranea Global Modules
#include "AraneaGlobalImporter.h"

// ========================================
// 定数
// ========================================
static const char* DEVICE_TYPE = "AR-IS02A";
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

// ========================================
// 設定値（NVSから読み込み）
// ========================================
int batchIntervalSec = 30;
int selfMeasureIntervalSec = 60;
float tempOffset = 0.0;
float humOffset = 0.0;

// ========================================
// 自己計測
// ========================================
void measureSelf() {
  if (!sensorReady) return;

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
void sendBatch() {
  isSending = true;
  updateDisplay();

  // 期限切れノード削除
  bleReceiver.cleanupExpired();

  int nodeCount = bleReceiver.getValidNodeCount();
  Serial.printf("[BATCH] Sending (self + %d nodes)...\n", nodeCount);

  bool ok = stateReporter.sendBatchToRelay();

  isSending = false;

  if (ok) {
    Serial.println("[BATCH] Sent successfully");
  } else {
    Serial.println("[BATCH] Send failed");
  }

  // 統計更新
  httpMgr.setSendStatus(0, 0,
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
// 設定再読み込み
// ========================================
void reloadSettings() {
  batchIntervalSec = settings.getInt(Is02aKeys::kBatchIntv, 30);
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

  // Operator初期化
  op.setMode(OperatorMode::PROVISION);

  // 1. SettingManager
  settings.begin();

  // 2. デフォルト設定投入
  AraneaSettings::init();
  AraneaSettings::initDefaults(settings);

  // 3. DisplayManager
  Wire.begin(PIN_I2C_SDA, PIN_I2C_SCL);
  if (!display.begin()) {
    Serial.println("[ERROR] Display init failed");
  }
  display.showBoot("Booting...");

  // 4. HT-30センサー初期化
  delay(100);
  sht.begin();
  delay(50);
  if (sht.read()) {
    sensorReady = true;
    selfTempC = sht.getTemperature();
    selfHumPct = sht.getHumidity();
    Serial.printf("[SENSOR] HT-30 ready: %.1f°C, %.1f%%\n", selfTempC, selfHumPct);
  } else {
    sensorReady = false;
    Serial.println("[SENSOR] HT-30 not found");
  }

  // 5. WiFi接続（NVS設定を使用）
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

  String tid = settings.getString("tid", AraneaSettings::getTid());
  AraneaRegisterResult regResult = araneaReg.registerDevice(
    tid, DEVICE_TYPE, myLacisId, myMac, PRODUCT_TYPE, PRODUCT_CODE
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
  stateReporter.setAuth(tid, myLacisId, myCic);
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

  // 14. 通常動作モードへ移行
  op.setMode(OperatorMode::RUN);

  // 初期表示
  unsigned long now = millis();
  lastBatchSend = now;
  lastSelfMeasure = now;
  lastDisplayUpdate = now;
  updateDisplay();

  Serial.println("[BOOT] Ready");
  Serial.printf("[BOOT] Web UI: http://%s/\n", myIp.c_str());
}

// ========================================
// Loop
// ========================================
void loop() {
  unsigned long now = millis();

  // Web UI処理
  httpMgr.handle();

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
