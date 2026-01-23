/**
 * is06s - Relay & Switch Controller (ar-is06s)
 *
 * 4ch D/P出力 + 2ch I/O コントローラー（PWM対応）
 * CH1-4: Digital/PWM切替出力
 * CH5-6: Digital Input/Output切替
 *
 * 機能:
 * - CH1-4 (GPIO 18,05,15,27): Digital出力 or PWM出力
 * - CH5-6 (GPIO 14,12): Digital出力 or Digital入力（連動トリガー）
 * - PIN Enable/Disable制御
 * - PINglobal参照チェーン（ハードコード排除）
 * - Web UI設定
 * - MQTT連携
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
 * - The_golden_rules.md準拠
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
static const char* PRODUCT_CODE = "0200";
static const char* ARANEA_DEVICE_TYPE = "aranea_ar-is06s";
static const char* DEVICE_SHORT_NAME = "ar-is06s";
static const char* FIRMWARE_VERSION = "1.0.0";

// ========================================
// ピン定義
// ========================================
// D/P Type (Digital/PWM Output)
static const int PIN_CH1 = 18;
static const int PIN_CH2 = 5;
static const int PIN_CH3 = 15;
static const int PIN_CH4 = 27;

// I/O Type (Digital Input/Output)
static const int PIN_CH5 = 14;
static const int PIN_CH6 = 12;

// System PIN
static const int PIN_RECONNECT = 25;  // WiFi再接続（5秒長押し）
static const int PIN_RESET = 26;      // ファクトリーリセット（15秒長押し）

// I2C
static const int I2C_SDA = 21;
static const int I2C_SCL = 22;

// ========================================
// タイミング定数（AraneaSettingsDefaultsを参照）
// ========================================
static const unsigned long DISPLAY_UPDATE_MS = 1000;
static const unsigned long DISPLAY_ACTION_MS = 100;
// System PIN timing: AraneaSettingsDefaults::RECONNECT_HOLD_MS, RESET_HOLD_MS を使用

// ========================================
// グローバル変数
// ========================================
// モジュールインスタンス（共通）
SettingManager settings;
DisplayManager display;
WiFiManager wifi;
NtpManager ntp;
LacisIDGenerator lacisGen;
AraneaRegister araneaReg;
OtaManager ota;
HttpOtaManager httpOta;
Operator op;

// is06s固有モジュール
Is06PinManager pinManager;
// HttpManagerIs06s httpMgr;      // P1-6で実装
// StateReporterIs06s stateReporter;  // P2-1で実装

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
unsigned long btnReconnectPressTime = 0;
unsigned long btnResetPressTime = 0;
unsigned long lastHeartbeatTime = 0;
unsigned long bootTime = 0;
int heartbeatIntervalSec = 60;

// ========================================
// 関数プロトタイプ
// ========================================
void initGpio();
void initPinGlobalNvs();
void handleSystemButtons();
void updateDisplay();

// ========================================
// setup()
// ========================================
void setup() {
  Serial.begin(115200);
  delay(100);

  Serial.println();
  Serial.println("============================================");
  Serial.println("  IS06S - Relay & Switch Controller");
  Serial.println("  Type: ar-is06s");
  Serial.println("  ProductCode: 0200");
  Serial.printf("  Firmware: %s\n", FIRMWARE_VERSION);
  Serial.println("============================================");

  bootTime = millis();

  // GPIO初期化
  initGpio();

  // I2C初期化（OLED用）
  Wire.begin(I2C_SDA, I2C_SCL);

  // SettingManager初期化
  settings.begin();

  // PINglobal NVS初期化（P0-8）
  initPinGlobalNvs();

  // DisplayManager初期化
  display.begin();
  display.showBoot("IS06S Booting...");

  // MAC取得
  myMac = lacisGen.getStaMac12Hex();

  // LacisID生成
  myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
  Serial.printf("LacisID: %s\n", myLacisId.c_str());

  // ホスト名設定
  myHostname = "is06s-" + myMac.substring(8);

  // WiFi接続（P0-4）
  Serial.println("WiFi: Connecting...");
  wifi.setHostname(myHostname);

  // 設定からWiFi接続試行 → 失敗時はデフォルト（cluster1-6）試行
  bool wifiConnected = wifi.connectWithSettings(&settings);
  if (!wifiConnected) {
    Serial.println("WiFi: Settings connection failed, trying defaults...");
    wifiConnected = wifi.connectDefault();
  }

  if (wifiConnected) {
    Serial.printf("WiFi: Connected! IP=%s, RSSI=%d\n",
      wifi.getIP().c_str(), wifi.getRSSI());
    display.showBoot("WiFi OK");
  } else {
    Serial.println("WiFi: All connections failed. Starting AP mode...");
    apModeActive = true;
    apModeStartTime = millis();
    display.showBoot("AP Mode");
    // TODO: APモード実装（P3で拡張）
  }

  // NTP同期（P3-3で実装）
  // ntp.begin();

  // CIC取得（P2-3で実装）
  // araneaReg.begin();

  // PinManager初期化（P1-1）
  pinManager.begin(&settings);

  Serial.println("Setup complete.");
  display.showBoot("IS06S Ready");
}

// ========================================
// loop()
// ========================================
void loop() {
  unsigned long now = millis();

  // システムボタン処理
  handleSystemButtons();

  // ディスプレイ更新
  if (now - lastDisplayUpdate >= DISPLAY_UPDATE_MS) {
    lastDisplayUpdate = now;
    updateDisplay();
  }

  // PIN更新
  pinManager.update();

  // HTTP処理（P1-6で実装）
  // httpMgr.handleClient();

  // 状態送信（P2-1で実装）
  // stateReporter.update();

  delay(1);  // 省電力
}

// ========================================
// GPIO初期化
// ========================================
void initGpio() {
  Serial.println("GPIO: Initializing...");

  // D/P Type (CH1-4): 初期状態はOUTPUT LOW
  pinMode(PIN_CH1, OUTPUT);
  pinMode(PIN_CH2, OUTPUT);
  pinMode(PIN_CH3, OUTPUT);
  pinMode(PIN_CH4, OUTPUT);
  digitalWrite(PIN_CH1, LOW);
  digitalWrite(PIN_CH2, LOW);
  digitalWrite(PIN_CH3, LOW);
  digitalWrite(PIN_CH4, LOW);

  // I/O Type (CH5-6): 初期状態はINPUT_PULLDOWN（設定で切替）
  pinMode(PIN_CH5, INPUT_PULLDOWN);
  pinMode(PIN_CH6, INPUT_PULLDOWN);

  // System PIN: INPUT_PULLDOWN
  pinMode(PIN_RECONNECT, INPUT_PULLDOWN);
  pinMode(PIN_RESET, INPUT_PULLDOWN);

  Serial.println("GPIO: CH1-4=OUTPUT, CH5-6=INPUT, System=INPUT");
}

// ========================================
// PINglobal NVS初期化 (P0-8)
// ========================================
void initPinGlobalNvs() {
  using namespace AraneaSettingsDefaults;
  using namespace AraneaSettingsDefaults::NVSKeys;

  Serial.println("PINglobal: Checking NVS initialization...");

  // PINglobal namespace初期化チェック
  // 初回起動判定: dgValidityキーの存在確認
  String existingValue = settings.getString(PG_DIGITAL_VALIDITY, "");

  if (existingValue.isEmpty()) {
    Serial.println("PINglobal: First boot detected. Writing defaults...");

    // Digital Output defaults
    settings.setString(PG_DIGITAL_VALIDITY, DIGITAL_VALIDITY);
    settings.setString(PG_DIGITAL_DEBOUNCE, DIGITAL_DEBOUNCE);
    settings.setString(PG_DIGITAL_ACTION_MODE, DIGITAL_ACTION_MODE);
    settings.setString(PG_DIGITAL_EXPIRY, DEFAULT_EXPIRY);
    settings.setString(PG_DIGITAL_EXPIRY_EN, DEFAULT_EXPIRY_ENABLED);

    // PWM Output defaults
    settings.setString(PG_PWM_RATE_OF_CHANGE, PWM_RATE_OF_CHANGE);
    settings.setString(PG_PWM_ACTION_MODE, PWM_ACTION_MODE);
    settings.setString(PG_PWM_EXPIRY, DEFAULT_EXPIRY);
    settings.setString(PG_PWM_EXPIRY_EN, DEFAULT_EXPIRY_ENABLED);

    // PIN enabled defaults (CH1-6)
    for (int i = 1; i <= 6; i++) {
      String key = String(CH_ENABLED_PREFIX) + String(i) + String(CH_ENABLED_SUFFIX);
      settings.setString(key.c_str(), PIN_ENABLED_DEFAULT);
    }

    // PIN type defaults
    // CH1-4: Digital Output, CH5-6: Digital Input
    for (int i = 1; i <= 4; i++) {
      String key = String(CH_ENABLED_PREFIX) + String(i) + String(CH_TYPE_SUFFIX);
      settings.setString(key.c_str(), PinType::DIGITAL_OUTPUT);
    }
    for (int i = 5; i <= 6; i++) {
      String key = String(CH_ENABLED_PREFIX) + String(i) + String(CH_TYPE_SUFFIX);
      settings.setString(key.c_str(), PinType::DIGITAL_INPUT);
    }

    Serial.println("PINglobal: Defaults written to NVS.");
  } else {
    Serial.println("PINglobal: Existing NVS values found. Skipping initialization.");
  }

  // 確認出力
  Serial.printf("PINglobal: Digital validity=%s, debounce=%s, mode=%s\n",
    settings.getString(PG_DIGITAL_VALIDITY, "?").c_str(),
    settings.getString(PG_DIGITAL_DEBOUNCE, "?").c_str(),
    settings.getString(PG_DIGITAL_ACTION_MODE, "?").c_str());
  Serial.printf("PINglobal: PWM rateOfChange=%s, mode=%s\n",
    settings.getString(PG_PWM_RATE_OF_CHANGE, "?").c_str(),
    settings.getString(PG_PWM_ACTION_MODE, "?").c_str());
}

// ========================================
// システムボタン処理
// ========================================
void handleSystemButtons() {
  using namespace AraneaSettingsDefaults;
  unsigned long now = millis();

  // Reconnectボタン (GPIO25)
  if (digitalRead(PIN_RECONNECT) == HIGH) {
    if (btnReconnectPressTime == 0) {
      btnReconnectPressTime = now;
      Serial.println("Reconnect button pressed");
    }
    unsigned long held = now - btnReconnectPressTime;
    if (held >= RECONNECT_HOLD_MS) {
      Serial.println("Reconnect: WiFi reconnecting + NTP sync...");
      display.showBoot("Reconnecting...");

      // WiFi再接続
      wifi.disconnect();
      delay(100);
      bool reconnected = wifi.connectWithSettings(&settings);
      if (!reconnected) {
        reconnected = wifi.connectDefault();
      }

      if (reconnected) {
        Serial.printf("Reconnect: Success! IP=%s\n", wifi.getIP().c_str());
        apModeActive = false;
        // NTP同期（P3-3で実装）
        // ntp.sync();
      } else {
        Serial.println("Reconnect: Failed. AP mode.");
        apModeActive = true;
        apModeStartTime = millis();
      }

      btnReconnectPressTime = 0;
      delay(500);
    } else {
      // カウントダウン表示
      int remaining = (RECONNECT_HOLD_MS - held) / 1000 + 1;
      char buf[32];
      snprintf(buf, sizeof(buf), "Reconnect: %d sec", remaining);
      display.showBoot(buf);
    }
  } else {
    btnReconnectPressTime = 0;
  }

  // Resetボタン (GPIO26)
  if (digitalRead(PIN_RESET) == HIGH) {
    if (btnResetPressTime == 0) {
      btnResetPressTime = now;
      Serial.println("Reset button pressed");
    }
    unsigned long held = now - btnResetPressTime;
    if (held >= RESET_HOLD_MS) {
      Serial.println("Reset: Factory reset...");
      display.showBoot("Resetting...");
      settings.clear();
      delay(1000);
      ESP.restart();
    } else {
      // カウントダウン表示
      int remaining = (RESET_HOLD_MS - held) / 1000 + 1;
      char buf[32];
      snprintf(buf, sizeof(buf), "Reset: %d sec", remaining);
      display.showBoot(buf);
    }
  } else {
    btnResetPressTime = 0;
  }
}

// ========================================
// ディスプレイ更新
// ========================================
void updateDisplay() {
  // 簡易表示（P3-1で拡張）
  String line1 = "IS06S " + String(FIRMWARE_VERSION);
  String line2 = "MAC:" + myMac.substring(6);
  String line3;
  String line4;

  if (apModeActive) {
    line3 = "AP Mode";
    unsigned long elapsed = (millis() - apModeStartTime) / 1000;
    line4 = "T:" + String(elapsed) + "s";
  } else if (wifi.isConnected()) {
    line3 = wifi.getIP();
    line4 = "RSSI:" + String(wifi.getRSSI());
  } else {
    line3 = "Connecting...";
    line4 = "";
  }

  display.show4Lines(line1, line2, line3, line4);
}
