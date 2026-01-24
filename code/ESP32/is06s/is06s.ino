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
#include <ArduinoJson.h>
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
HttpManagerIs06s httpMgr;
StateReporterIs06s stateReporter;
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
unsigned long btnReconnectPressTime = 0;
unsigned long btnResetPressTime = 0;
unsigned long lastHeartbeatTime = 0;
unsigned long bootTime = 0;
int heartbeatIntervalSec = 60;

// MQTT (P2-4, P2-5)
bool mqttEnabled = false;
bool mqttJustConnected = false;  // 接続直後フラグ（loop()で初期状態送信用）
String mqttUrl;
String mqttCommandTopic;
String mqttStateTopic;
String mqttAckTopic;

// ========================================
// 関数プロトタイプ
// ========================================
void initGpio();
void initPinGlobalNvs();
void handleSystemButtons();
void updateDisplay();
String buildStatusLine();
void initMqtt();
void onMqttMessage(const String& topic, const char* data, int len);
void publishPinState(int channel);
void publishAllPinStates();

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

  // AraneaSettings初期化（大量展開用デフォルト値）
  AraneaSettings::initDefaults(settings);

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

  // NTP同期（P3-3）
  if (wifiConnected) {
    Serial.println("NTP: Syncing...");
    display.showBoot("NTP Sync...");
    if (ntp.sync()) {
      Serial.printf("NTP: Synced - %s\n", ntp.getIso8601().c_str());
    } else {
      Serial.println("NTP: Sync failed (will retry later)");
    }
  }

  // CIC取得（P2-3）
  if (wifiConnected) {
    Serial.println("AraneaRegister: Initializing...");
    araneaReg.begin(AraneaSettings::getGateUrl());

    // テナントPrimary認証設定
    TenantPrimaryAuth tenantAuth;
    tenantAuth.lacisId = AraneaSettings::getTenantLacisId();
    tenantAuth.userId = AraneaSettings::getTenantEmail();
    tenantAuth.cic = AraneaSettings::getTenantCic();
    araneaReg.setTenantPrimary(tenantAuth);

    // デバイス登録
    myTid = settings.getString("tid", AraneaSettings::getTid());
    myFid = settings.getString("fid", AraneaSettings::getFid());

    AraneaRegisterResult regResult = araneaReg.registerDevice(
      myTid,
      ARANEA_DEVICE_TYPE,
      myLacisId,
      myMac,
      PRODUCT_TYPE,
      PRODUCT_CODE
    );

    if (regResult.ok) {
      myCic = regResult.cic_code;
      Serial.printf("AraneaRegister: CIC acquired = %s\n", myCic.c_str());
      display.showBoot("CIC: " + myCic);

      // MQTT URLをAraneaRegisterから取得（is04同様）
      if (regResult.mqttEndpoint.length() > 0) {
        mqttUrl = regResult.mqttEndpoint;
        Serial.printf("AraneaRegister: MQTT endpoint = %s\n", mqttUrl.c_str());
      } else {
        // 既登録時（APIリクエストなし）はNVSから取得を試みる
        mqttUrl = araneaReg.getSavedMqttEndpoint();
        if (mqttUrl.length() > 0) {
          Serial.printf("AraneaRegister: Using saved MQTT endpoint = %s\n", mqttUrl.c_str());
        }
      }
    } else {
      // オフライン時はNVSから取得を試みる
      myCic = araneaReg.getSavedCic();
      if (myCic.length() > 0) {
        Serial.printf("AraneaRegister: Using saved CIC = %s\n", myCic.c_str());
      } else {
        Serial.printf("AraneaRegister: Failed - %s\n", regResult.error.c_str());
        display.showBoot("CIC Error");
      }
      // フォールバック: NVSから保存済みMQTT URLを取得
      mqttUrl = araneaReg.getSavedMqttEndpoint();
      if (mqttUrl.length() > 0) {
        Serial.printf("AraneaRegister: Using saved MQTT endpoint = %s\n", mqttUrl.c_str());
      }
    }
  }

  // PinManager初期化（P1-1, P3-5: NtpManager渡してexpiryDate判定可能に）
  pinManager.begin(&settings, &ntp);

  // デバイス情報設定
  AraneaDeviceInfo devInfo;
  devInfo.deviceType = ARANEA_DEVICE_TYPE;
  devInfo.modelName = "Relay & Switch Controller";
  devInfo.contextDesc = "4ch D/P出力 + 2ch I/O コントローラー";
  devInfo.lacisId = myLacisId;
  devInfo.cic = myCic;
  devInfo.mac = myMac;
  devInfo.hostname = myHostname;
  devInfo.firmwareVersion = FIRMWARE_VERSION;
  devInfo.buildDate = __DATE__;
  devInfo.modules = "WiFi,NTP,PIN,HTTP,OTA";

  // HttpManager初期化（P1-6）
  httpMgr.setDeviceInfo(devInfo);
  httpMgr.begin(&settings, &pinManager);
  Serial.printf("HTTP: Web UI available at http://%s/\n", wifi.getIP().c_str());

  // HTTP OTA初期化（P3-2）
  httpOta.begin(httpMgr.getServer());
  httpOta.onStart([]() {
    Serial.println("[OTA] Update started");
  });
  httpOta.onProgress([](int progress) {
    Serial.printf("[OTA] Progress: %d%%\n", progress);
  });
  httpOta.onEnd([]() {
    Serial.println("[OTA] Update complete, rebooting...");
  });
  httpOta.onError([](const String& error) {
    Serial.printf("[OTA] Error: %s\n", error.c_str());
  });
  Serial.println("HTTP OTA: Initialized (/api/ota/*)");

  // StateReporter初期化（P2-1）
  stateReporter.begin(&settings, &ntp, &pinManager);
  stateReporter.setAuth(myTid, myLacisId, myCic);
  stateReporter.setMac(myMac);
  stateReporter.setFirmwareVersion(FIRMWARE_VERSION);
  // HTTP relay URLを設定（TLSクラッシュ対策: HTTP通信を先に実行）
  stateReporter.setRelayUrls(
    settings.getString("relay_pri", ARANEA_DEFAULT_RELAY_PRIMARY),
    settings.getString("relay_sec", ARANEA_DEFAULT_RELAY_SECONDARY)
  );
  stateReporter.setCloudUrl(settings.getString("cloud_url", AraneaSettings::getCloudUrl()));
  stateReporter.setHeartbeatInterval(heartbeatIntervalSec);
  Serial.println("StateReporter: Initialized");

  // MQTT初期化（P2-4, P2-5）
  // Serial出力削減で安定化確認済み - MQTT再有効化
  if (wifiConnected) {
    initMqtt();
  }

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

  // HTTP処理
  httpMgr.handle();

  // 状態送信（P2-1）
  // 修正済み: HTTP relay URL追加 + タイムアウト延長 + JSONサイズ最適化
  // 参照: code/ESP32/is06s/design/review/
  stateReporter.update();

  // MQTT処理（P2-4）
  if (mqttEnabled) {
    mqtt.handle();

    // 接続直後の初期状態送信（コールバック外で実行）
    // 重要: onConnectedコールバック内でpublishするとクラッシュするため
    // フラグを使ってloop()で実行する（is05aと同じパターン）
    if (mqttJustConnected && mqtt.isConnected()) {
      mqttJustConnected = false;
      Serial.println("MQTT: Publishing initial states...");
      publishAllPinStates();
    }
  }

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
  // 共通仕様: showIs02Main(line1, cic, statusLine, showLink)
  // line1: IP + RSSI (小フォント)
  // cic: CICコード (大フォント、中央)
  // statusLine: 状態表示 (中フォント)

  if (apModeActive) {
    String apSSID = "AP:" + myHostname;
    String apIP = "192.168.250.1";
    display.showIs02Main(apSSID, "AP MODE", apIP, false);
    return;
  }

  // Line 1: IP + RSSI
  String line1 = wifi.isConnected()
    ? wifi.getIP() + " " + String(wifi.getRSSI()) + "dBm"
    : "Connecting...";

  // CIC表示（取得済みなら表示、未取得なら"------"）
  String cicStr = myCic.length() > 0 ? myCic : "------";

  // ステータス行: アクティブなPIN状態を表示
  String statusLine = buildStatusLine();

  display.showIs02Main(line1, cicStr, statusLine, false);
}

// ========================================
// ステータス行構築（PIN状態サマリ）
// ========================================
String buildStatusLine() {
  // アクティブなPIN情報を収集
  String digitalOns = "";   // ON中のデジタルCH
  String pwmValues = "";    // PWM値

  for (int ch = 1; ch <= 6; ch++) {
    if (!pinManager.isPinEnabled(ch)) continue;

    const PinSetting& setting = pinManager.getPinSetting(ch);

    if (setting.type == PinType::DIGITAL_OUTPUT) {
      if (pinManager.getPinState(ch) == 1) {
        if (digitalOns.length() > 0) digitalOns += ",";
        digitalOns += String(ch);
      }
    } else if (setting.type == PinType::PWM_OUTPUT) {
      int pwmVal = pinManager.getPwmValue(ch);
      if (pwmVal > 0) {
        if (pwmValues.length() > 0) pwmValues += " ";
        pwmValues += String(ch) + ":" + String(pwmVal) + "%";
      }
    }
  }

  // 状態サマリ構築
  if (digitalOns.length() == 0 && pwmValues.length() == 0) {
    return "Ready";
  }

  String status = "";

  // デジタル: "ON:1,2" 形式
  if (digitalOns.length() > 0) {
    status = "ON:" + digitalOns;
  }

  // PWM: "1:50% 2:80%" 形式
  if (pwmValues.length() > 0) {
    if (status.length() > 0) status += " ";
    status += pwmValues;
  }

  return status;
}

// ========================================
// MQTT初期化 (P2-4)
// ========================================
// デフォルトMQTT URL (araneaSDK Designより)
const char* DEFAULT_MQTT_URL = "wss://aranea-mqtt-bridge-1010551946141.asia-northeast1.run.app";

void initMqtt() {
  // MQTT URLはAraneaRegisterから取得済み（setup()で設定）
  // フォールバック1: settings.mqtt_urlを確認
  if (mqttUrl.isEmpty()) {
    mqttUrl = settings.getString("mqtt_url", "");
  }

  // フォールバック2: デフォルトMQTT URLを使用
  if (mqttUrl.isEmpty()) {
    Serial.println("MQTT: Using default MQTT URL");
    mqttUrl = DEFAULT_MQTT_URL;
  }

  // トピック設定 (aranea/{tid}/{lacisId}/... 形式)
  mqttCommandTopic = "aranea/" + myTid + "/" + myLacisId + "/command";
  mqttStateTopic = "aranea/" + myTid + "/" + myLacisId + "/state";
  mqttAckTopic = "aranea/" + myTid + "/" + myLacisId + "/ack";

  Serial.printf("MQTT: URL=%s\n", mqttUrl.c_str());
  Serial.printf("MQTT: CommandTopic=%s\n", mqttCommandTopic.c_str());

  // コールバック設定
  // 注意: onConnectedコールバック内でpublish()を呼ぶとクラッシュする
  // （ESP-IDF MQTTイベントハンドラコンテキストの制約）
  // → フラグを立ててloop()で処理する
  mqtt.onConnected([]() {
    Serial.println("MQTT: Connected!");
    // コマンドトピック購読
    mqtt.subscribe(mqttCommandTopic, 1);
    Serial.printf("MQTT: Subscribed to %s\n", mqttCommandTopic.c_str());
    // 初期状態送信はloop()で行う（コールバック内でのpublish禁止）
    mqttJustConnected = true;
  });

  mqtt.onDisconnected([]() {
    Serial.println("MQTT: Disconnected.");
  });

  mqtt.onMessage(onMqttMessage);

  mqtt.onError([](const String& error) {
    Serial.printf("MQTT: Error - %s\n", error.c_str());
  });

  // 接続開始（認証: lacisId + cic）
  Serial.println("MQTT: Calling mqtt.begin()...");
  Serial.printf("MQTT: Free heap: %d bytes\n", ESP.getFreeHeap());
  Serial.flush();  // ログを確実に出力

  if (mqtt.begin(mqttUrl, myLacisId, myCic)) {
    Serial.println("MQTT: Connection started.");
    mqttEnabled = true;
  } else {
    Serial.println("MQTT: Failed to start connection.");
    mqttEnabled = false;
  }
}

// ========================================
// MQTTメッセージ受信処理 (P2-5)
// ========================================
void onMqttMessage(const String& topic, const char* data, int len) {
  Serial.printf("MQTT: Received [%s] len=%d\n", topic.c_str(), len);

  if (topic != mqttCommandTopic) {
    return;
  }

  // JSONパース
  StaticJsonDocument<512> doc;
  DeserializationError err = deserializeJson(doc, data, len);
  if (err) {
    Serial.printf("MQTT: JSON parse error - %s\n", err.c_str());
    return;
  }

  // コマンド解析
  const char* cmd = doc["cmd"] | "";
  int ch = doc["ch"] | 0;
  int state = doc["state"] | -1;
  const char* expiryDate = doc["expiryDate"] | "";
  const char* requestId = doc["requestId"] | "";

  String ackStatus = "ok";
  String ackError = "";

  Serial.printf("MQTT: cmd=%s, ch=%d, state=%d\n", cmd, ch, state);

  if (strcmp(cmd, "set") == 0) {
    // PIN状態設定
    if (ch >= 1 && ch <= 6 && state >= 0) {
      // expiryDate設定（オプション）
      if (strlen(expiryDate) == 12) {
        pinManager.setExpiryDate(ch, expiryDate);
        pinManager.setExpiryEnabled(ch, true);
      }

      bool ok = pinManager.setPinState(ch, state);
      if (ok) {
        Serial.printf("MQTT: CH%d set to %d\n", ch, state);
        publishPinState(ch);
      } else {
        ackStatus = "error";
        if (!pinManager.isPinEnabled(ch)) {
          ackError = "PIN disabled";
        } else if (pinManager.isExpired(ch)) {
          ackError = "PIN expired";
        } else {
          ackError = "Command rejected";
        }
      }
    } else {
      ackStatus = "error";
      ackError = "Invalid channel or state";
    }
  } else if (strcmp(cmd, "pulse") == 0) {
    // パルス出力
    if (ch >= 1 && ch <= 6) {
      bool ok = pinManager.setPinState(ch, 1);
      if (ok) {
        Serial.printf("MQTT: CH%d pulse\n", ch);
        publishPinState(ch);
      } else {
        ackStatus = "error";
        ackError = "Pulse rejected";
      }
    } else {
      ackStatus = "error";
      ackError = "Invalid channel";
    }
  } else if (strcmp(cmd, "allOff") == 0) {
    // 全チャンネルOFF
    for (int i = 1; i <= 6; i++) {
      if (pinManager.isPinEnabled(i)) {
        pinManager.setPinState(i, 0);
      }
    }
    Serial.println("MQTT: All channels OFF");
    publishAllPinStates();
  } else if (strcmp(cmd, "getState") == 0) {
    // 状態取得
    if (ch >= 1 && ch <= 6) {
      publishPinState(ch);
    } else {
      publishAllPinStates();
    }
  } else if (strcmp(cmd, "setEnabled") == 0) {
    // PIN有効/無効設定
    if (ch >= 1 && ch <= 6) {
      bool enabled = doc["enabled"] | true;
      pinManager.setPinEnabled(ch, enabled);
      Serial.printf("MQTT: CH%d enabled=%s\n", ch, enabled ? "true" : "false");
    } else {
      ackStatus = "error";
      ackError = "Invalid channel";
    }
  } else {
    ackStatus = "error";
    ackError = "Unknown command";
  }

  // ACK送信
  StaticJsonDocument<256> ackDoc;
  ackDoc["requestId"] = requestId;
  ackDoc["status"] = ackStatus;
  if (ackError.length() > 0) {
    ackDoc["error"] = ackError;
  }
  ackDoc["timestamp"] = ntp.isSynced() ? ntp.getIso8601() : "unknown";

  String ackJson;
  serializeJson(ackDoc, ackJson);
  mqtt.publish(mqttAckTopic, ackJson, 1);
  Serial.printf("MQTT: ACK sent [%s]\n", ackStatus.c_str());
}

// ========================================
// PIN状態パブリッシュ (P2-5)
// ========================================
void publishPinState(int channel) {
  if (!mqttEnabled || !mqtt.isConnected()) return;
  if (channel < 1 || channel > 6) return;

  const PinSetting& setting = pinManager.getPinSetting(channel);

  StaticJsonDocument<256> doc;
  doc["ch"] = channel;
  doc["enabled"] = pinManager.isPinEnabled(channel);

  if (setting.type == PinType::PWM_OUTPUT) {
    doc["state"] = String(pinManager.getPwmValue(channel));
  } else {
    doc["state"] = pinManager.getPinState(channel) ? "on" : "off";
  }

  doc["type"] = (setting.type == PinType::PWM_OUTPUT) ? "pwm" : "digital";
  doc["timestamp"] = ntp.isSynced() ? ntp.getIso8601() : "unknown";

  String json;
  serializeJson(doc, json);
  mqtt.publish(mqttStateTopic, json, 1);
}

void publishAllPinStates() {
  if (!mqttEnabled || !mqtt.isConnected()) return;

  StaticJsonDocument<1024> doc;
  JsonObject pinState = doc.createNestedObject("PINState");

  for (int ch = 1; ch <= 6; ch++) {
    const PinSetting& setting = pinManager.getPinSetting(ch);
    JsonObject chObj = pinState.createNestedObject("CH" + String(ch));

    chObj["enabled"] = pinManager.isPinEnabled(ch);

    if (setting.type == PinType::PWM_OUTPUT) {
      chObj["state"] = String(pinManager.getPwmValue(ch));
    } else {
      chObj["state"] = pinManager.getPinState(ch) ? "on" : "off";
    }

    const char* typeStr = "digitalOutput";
    switch (setting.type) {
      case PinType::DIGITAL_OUTPUT: typeStr = "digitalOutput"; break;
      case PinType::PWM_OUTPUT: typeStr = "pwmOutput"; break;
      case PinType::DIGITAL_INPUT: typeStr = "digitalInput"; break;
      case PinType::PIN_DISABLED: typeStr = "disabled"; break;
    }
    chObj["type"] = typeStr;
  }

  doc["timestamp"] = ntp.isSynced() ? ntp.getIso8601() : "unknown";
  doc["lacisId"] = myLacisId;

  String json;
  serializeJson(doc, json);
  mqtt.publish(mqttStateTopic, json, 1);
  Serial.println("MQTT: All states published.");
}
