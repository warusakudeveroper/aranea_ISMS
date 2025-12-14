/**
 * is01 - Battery-powered BLE Sensor Device
 *
 * 電池駆動の温湿度センサー。BLEアドバタイズで送信のみ。
 * 初回起動時のみWiFi接続してCICを取得（アクティベーション）。
 *
 * 動作シーケンス:
 * 1. DeepSleep復帰 / 初回起動
 * 2. GPIO5 HIGH（I2C電源ON）
 * 3. I2Cディスプレイ初期化（同期的に）
 * 4. HT-30センサーから温湿度取得
 * 5. ディスプレイ表示（20秒間維持）
 * 6. BLEアドバタイズ送信
 * 7. GPIO5 LOW（I2C電源OFF）
 * 8. DeepSleep（4分間）
 *
 * アクティベーション（初回のみ）:
 * - WiFi接続 → AraneaGateway登録 → CIC取得 → NVS保存
 */

#include <Arduino.h>
#include <Wire.h>
#include <SHT31.h>
#include <NimBLEDevice.h>
#include <esp_sleep.h>
#include <Preferences.h>
#include <WiFi.h>
#include <esp_mac.h>

// AraneaGlobal共通モジュール
#include "SettingManager.h"
#include "DisplayManager.h"
#include "WiFiManager.h"
#include "LacisIDGenerator.h"
#include "AraneaRegister.h"

// ========================================
// 定数
// ========================================
static const char* PRODUCT_TYPE = "001";
static const char* PRODUCT_CODE = "0096";
static const char* DEVICE_TYPE = "ISMS_ar-is01";
static const char* FIRMWARE_VERSION = "1.0.0";

// GPIO5: I2C電源制御（MOSFET経由）
static const int GPIO_I2C_POWER = 5;

// DeepSleep時間（4分=240秒）
static const uint64_t DEEP_SLEEP_US = 240ULL * 1000000ULL;

// ディスプレイ表示維持時間（20秒）
static const unsigned long DISPLAY_DURATION_MS = 20000;

// BLEアドバタイズ時間（5秒）
static const unsigned long BLE_ADVERTISE_MS = 5000;

// ISMSv1ペイロード定数
static const uint8_t MAGIC_I = 0x49;  // 'I'
static const uint8_t MAGIC_S = 0x53;  // 'S'
static const uint8_t ISMS_VERSION = 0x01;

// ========================================
// グローバル変数
// ========================================
// RTC_DATA_ATTR: DeepSleep後も保持される変数
RTC_DATA_ATTR uint32_t bootCount = 0;

// モジュールインスタンス
SettingManager settings;
DisplayManager display;
WiFiManager wifi;
LacisIDGenerator lacisGen;
AraneaRegister araneaReg;

// HT-30センサー（SHT31互換、I2Cアドレス0x44）
SHT31 sht(0x44);

// 自機情報
String myLacisId;
String myMac;
String myCic;
bool isActivated = false;

// センサー値
float tempC = NAN;
float humPct = NAN;
uint8_t battPct = 100;  // TODO: 実際のバッテリー電圧計測

// ========================================
// I2C電源制御
// ========================================
void enableI2CPower() {
  pinMode(GPIO_I2C_POWER, OUTPUT);
  digitalWrite(GPIO_I2C_POWER, HIGH);
  delay(100);  // 電源安定待ち
  Serial.println("[POWER] I2C power ON (GPIO5 HIGH)");
}

void disableI2CPower() {
  digitalWrite(GPIO_I2C_POWER, LOW);
  Serial.println("[POWER] I2C power OFF (GPIO5 LOW)");
}

// ========================================
// I2C初期化（同期的に実行）
// ========================================
bool initI2CDevices() {
  // Wire初期化
  Wire.begin();
  delay(50);

  // ディスプレイ初期化
  if (!display.begin()) {
    Serial.println("[ERROR] Display init failed");
    return false;
  }
  delay(50);

  // センサー初期化
  sht.begin();
  delay(50);

  Serial.println("[I2C] All devices initialized");
  return true;
}

// ========================================
// センサー読み取り
// ========================================
bool readSensor() {
  if (!sht.read()) {
    Serial.println("[SENSOR] HT-30 read failed");
    tempC = NAN;
    humPct = NAN;
    return false;
  }

  tempC = sht.getTemperature();
  humPct = sht.getHumidity();
  Serial.printf("[SENSOR] HT-30: %.2fC, %.2f%%\n", tempC, humPct);
  return true;
}

// ========================================
// BLEアドバタイズ（ISMSv1ペイロード）
// ========================================
void sendBleAdvertise() {
  Serial.println("[BLE] Starting advertise...");

  // BLE初期化
  NimBLEDevice::init("is01");
  NimBLEServer* pServer = NimBLEDevice::createServer();
  NimBLEAdvertising* pAdvertising = NimBLEDevice::getAdvertising();

  // ISMSv1ペイロード構築（20バイト）
  uint8_t payload[20];
  memset(payload, 0, sizeof(payload));

  // ヘッダ
  payload[0] = MAGIC_I;
  payload[1] = MAGIC_S;
  payload[2] = ISMS_VERSION;
  payload[3] = atoi(PRODUCT_TYPE);  // productType
  payload[4] = 0x0F;  // flags: HAS_TEMP | HAS_HUM | HAS_BATT | HAS_BOOTCOUNT

  // STA MAC（6バイト）- efuseから直接読み取り（WiFi初期化不要）
  uint8_t mac[6];
  esp_read_mac(mac, ESP_MAC_WIFI_STA);
  memcpy(&payload[5], mac, 6);
  Serial.printf("[BLE] STA MAC: %02X%02X%02X%02X%02X%02X\n",
    mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);

  // bootCount（4バイト、little endian）
  payload[11] = bootCount & 0xFF;
  payload[12] = (bootCount >> 8) & 0xFF;
  payload[13] = (bootCount >> 16) & 0xFF;
  payload[14] = (bootCount >> 24) & 0xFF;

  // 温度（int16 x100、little endian）
  int16_t temp_x100 = isnan(tempC) ? 0 : (int16_t)(tempC * 100);
  payload[15] = temp_x100 & 0xFF;
  payload[16] = (temp_x100 >> 8) & 0xFF;

  // 湿度（uint16 x100、little endian）
  uint16_t hum_x100 = isnan(humPct) ? 0 : (uint16_t)(humPct * 100);
  payload[17] = hum_x100 & 0xFF;
  payload[18] = (hum_x100 >> 8) & 0xFF;

  // バッテリー
  payload[19] = battPct;

  // Manufacturer Dataとして設定
  std::string mfgData(reinterpret_cast<char*>(payload), 20);

  // アドバタイズ設定
  pAdvertising->setMinInterval(160);   // 100ms
  pAdvertising->setMaxInterval(320);   // 200ms

  // Manufacturer Data設定 (NimBLE 2.x API)
  NimBLEAdvertisementData advData;
  advData.setManufacturerData(mfgData);
  advData.setName("is01");
  pAdvertising->setAdvertisementData(advData);

  // アドバタイズ開始
  pAdvertising->start();
  Serial.println("[BLE] Advertising started");

  // 指定時間アドバタイズ
  delay(BLE_ADVERTISE_MS);

  // 停止
  pAdvertising->stop();
  NimBLEDevice::deinit(true);
  Serial.println("[BLE] Advertising stopped");
}

// ========================================
// アクティベーション（初回のみ）
// ========================================
bool doActivation() {
  Serial.println("[ACTIVATE] Starting activation...");

  // ディスプレイに接続中表示
  display.showConnecting("WiFi...", 0);

  // WiFi接続
  if (!wifi.connectDefault()) {
    Serial.println("[ACTIVATE] WiFi connection failed");
    display.showError("WiFi Failed");
    delay(3000);
    return false;
  }

  Serial.printf("[ACTIVATE] WiFi connected: %s\n", wifi.getIP().c_str());
  display.showConnecting("Connected", 100);
  delay(500);

  // AraneaGateway登録
  display.showRegistering("Registering...");

  String gateUrl = settings.getString("gate_url", "");
  araneaReg.begin(gateUrl);

  // テナント認証情報
  TenantPrimaryAuth tenantAuth;
  tenantAuth.lacisId = settings.getString("tenant_lacisid", "");
  tenantAuth.userId = settings.getString("tenant_email", "");
  tenantAuth.pass = settings.getString("tenant_pass", "");
  tenantAuth.cic = settings.getString("tenant_cic", "");
  araneaReg.setTenantPrimary(tenantAuth);

  // 登録実行
  String tid = settings.getString("tid", "");
  AraneaRegisterResult regResult = araneaReg.registerDevice(
    tid, DEVICE_TYPE, myLacisId, myMac, PRODUCT_TYPE, PRODUCT_CODE
  );

  if (regResult.ok) {
    myCic = regResult.cic_code;
    Serial.printf("[ACTIVATE] CIC received: %s\n", myCic.c_str());

    // CICとアクティベーション状態を保存
    settings.setString("cic", myCic);
    settings.setInt("activated", 1);

    display.showRegistering("CIC: " + myCic);
    delay(2000);

    // WiFi切断（バッテリー節約）
    wifi.disconnect();
    WiFi.mode(WIFI_OFF);

    return true;
  } else {
    Serial.printf("[ACTIVATE] Registration failed: %s\n", regResult.error.c_str());
    display.showError("Reg Failed");
    delay(3000);

    // WiFi切断
    wifi.disconnect();
    WiFi.mode(WIFI_OFF);

    return false;
  }
}

// ========================================
// メイン表示
// ========================================
void showMainDisplay() {
  // CIC表示（未アクティベートの場合は "------"）
  String cicDisplay = myCic.length() > 0 ? myCic : "------";

  // センサー値
  String sensorLine;
  if (isnan(tempC) || isnan(humPct)) {
    sensorLine = "Sensor Error";
  } else {
    char buf[32];
    snprintf(buf, sizeof(buf), "%.1fC  %.0f%%", tempC, humPct);
    sensorLine = String(buf);
  }

  // ブート回数
  String bootLine = "Boot: " + String(bootCount);

  // 表示
  display.showIs01Main(cicDisplay, sensorLine, bootLine);
}

// ========================================
// DeepSleep開始
// ========================================
void enterDeepSleep() {
  Serial.printf("[SLEEP] Entering DeepSleep for %llu seconds...\n", DEEP_SLEEP_US / 1000000ULL);

  // I2C電源OFF
  disableI2CPower();

  // DeepSleep設定
  esp_sleep_enable_timer_wakeup(DEEP_SLEEP_US);

  Serial.flush();
  esp_deep_sleep_start();
}

// ========================================
// Setup
// ========================================
void setup() {
  Serial.begin(115200);
  delay(100);

  // ブート回数インクリメント
  bootCount++;
  Serial.printf("\n[BOOT] is01 starting... (boot count: %d)\n", bootCount);

  // 1. I2C電源ON
  enableI2CPower();

  // 2. 設定読み込み
  settings.begin();

  // 3. lacisID生成
  myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
  myMac = lacisGen.getStaMac12Hex();
  Serial.printf("[LACIS] My lacisId: %s\n", myLacisId.c_str());

  // 4. アクティベーション状態確認
  isActivated = settings.getInt("activated", 0) == 1;
  myCic = settings.getString("cic", "");

  // 5. I2Cデバイス初期化（同期的に）
  if (!initI2CDevices()) {
    Serial.println("[ERROR] I2C init failed, going to sleep");
    enterDeepSleep();
    return;
  }

  // 6. センサー読み取り
  readSensor();

  // 7. アクティベーション（未完了の場合のみ）
  if (!isActivated) {
    Serial.println("[BOOT] Not activated, starting activation...");

    // 起動表示
    display.showBoot("Activating...");
    delay(1000);

    if (doActivation()) {
      isActivated = true;
      Serial.println("[BOOT] Activation complete!");
    } else {
      Serial.println("[BOOT] Activation failed, will retry next boot");
      // アクティベーション失敗でもアドバタイズは行う（Zero3で検知可能）
    }
  } else {
    Serial.printf("[BOOT] Already activated, CIC: %s\n", myCic.c_str());
  }

  // 8. メイン表示
  showMainDisplay();

  // 9. 表示維持（20秒）
  unsigned long displayStart = millis();
  while (millis() - displayStart < DISPLAY_DURATION_MS) {
    delay(100);
  }

  // 10. BLEアドバタイズ送信
  sendBleAdvertise();

  // 11. DeepSleep
  enterDeepSleep();
}

// ========================================
// Loop（使用しない - DeepSleepのため）
// ========================================
void loop() {
  // DeepSleepから復帰するとsetup()から開始されるため、
  // loop()には到達しない
  delay(1000);
}
