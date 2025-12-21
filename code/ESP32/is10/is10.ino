/**
 * is10 - Openwrt Router Inspector (ar-is10)
 *
 * OpenWrt/AsusWRT系ルーターにSSHアクセスして情報収集し、
 * エンドポイントへPOSTするデバイス。
 *
 * 機能:
 * - WiFi接続（STA）/ APモードフォールバック
 * - AraneaGateway登録（CIC取得）
 * - 複数ルーターへのSSH接続・情報取得
 * - ルーター情報をエンドポイントへPOST
 * - HTTP設定UI
 * - OTA更新
 *
 * 取得情報:
 * - WAN側IP / LAN側IP
 * - SSID (2.4GHz / 5.0GHz)
 * - 接続クライアント数
 * - クライアント詳細（ホスト名、MAC、帯域、接続時間）
 */

#include <Arduino.h>
#include <Wire.h>
#include <WiFi.h>

// Reset reason & RTC memory for crash debugging
#include "esp_system.h"
#include "rom/rtc.h"

// Brownout disable (現在は無効化 - 分類情報取得のため)
#include "soc/soc.h"
#include "soc/rtc_cntl_reg.h"
// DISABLED: LibSSH-ESP32 TLS競合テストのため無効化
// #include <WiFiClientSecure.h>
#include <HTTPClient.h>
#include <esp_mac.h>
#include <SPIFFS.h>
#include <ArduinoJson.h>
#include <esp_heap_caps.h>
// REMOVED: #include <esp_task_wdt.h>  // WDT介入禁止

// ========================================
// RTC memory for crash position tracking
// ========================================
RTC_DATA_ATTR uint32_t bootCount = 0;
RTC_DATA_ATTR uint32_t lastStage = 0;

// Stage definitions for crash debugging
enum DebugStage : uint32_t {
  STAGE_BOOT_START = 1,
  STAGE_SERIAL_INIT = 2,
  STAGE_GPIO_INIT = 3,
  STAGE_SPIFFS_INIT = 4,
  STAGE_SETTINGS_INIT = 5,
  STAGE_DISPLAY_INIT = 6,
  STAGE_LACIS_GEN = 7,
  STAGE_WIFI_START = 8,
  STAGE_WIFI_DONE = 9,
  STAGE_NTP_SYNC = 10,
  STAGE_ARANEA_REG = 11,
  STAGE_CONFIG_LOAD = 12,
  STAGE_HTTP_MGR_INIT = 13,
  STAGE_SSH_BEGIN = 14,
  STAGE_SSH_DONE = 15,
  STAGE_SETUP_COMPLETE = 20,
  STAGE_LOOP_RUNNING = 100
};

static const char* resetReasonStr(esp_reset_reason_t r) {
  switch (r) {
    case ESP_RST_POWERON:  return "ESP_RST_POWERON";
    case ESP_RST_SW:       return "ESP_RST_SW";
    case ESP_RST_PANIC:    return "ESP_RST_PANIC";
    case ESP_RST_INT_WDT:  return "ESP_RST_INT_WDT";
    case ESP_RST_TASK_WDT: return "ESP_RST_TASK_WDT";
    case ESP_RST_WDT:      return "ESP_RST_WDT";
    case ESP_RST_BROWNOUT: return "ESP_RST_BROWNOUT";
    case ESP_RST_SDIO:     return "ESP_RST_SDIO";
    default:               return "ESP_RST_UNKNOWN";
  }
}

static const char* stageStr(uint32_t stage) {
  switch (stage) {
    case STAGE_BOOT_START:     return "BOOT_START";
    case STAGE_SERIAL_INIT:    return "SERIAL_INIT";
    case STAGE_GPIO_INIT:      return "GPIO_INIT";
    case STAGE_SPIFFS_INIT:    return "SPIFFS_INIT";
    case STAGE_SETTINGS_INIT:  return "SETTINGS_INIT";
    case STAGE_DISPLAY_INIT:   return "DISPLAY_INIT";
    case STAGE_LACIS_GEN:      return "LACIS_GEN";
    case STAGE_WIFI_START:     return "WIFI_START";
    case STAGE_WIFI_DONE:      return "WIFI_DONE";
    case STAGE_NTP_SYNC:       return "NTP_SYNC";
    case STAGE_ARANEA_REG:     return "ARANEA_REG";
    case STAGE_CONFIG_LOAD:    return "CONFIG_LOAD";
    case STAGE_HTTP_MGR_INIT:  return "HTTP_MGR_INIT";
    case STAGE_SSH_BEGIN:      return "SSH_BEGIN";
    case STAGE_SSH_DONE:       return "SSH_DONE";
    case STAGE_SETUP_COMPLETE: return "SETUP_COMPLETE";
    case STAGE_LOOP_RUNNING:   return "LOOP_RUNNING";
    default:                   return "UNKNOWN";
  }
}

// LibSSH-ESP32を明示的にインクルード（依存関係解決のため）
// Core 3.0.5で再有効化
#include "libssh_esp32.h"
#include <libssh/libssh.h>  // ssh_finalize() 用

// ============================================================
// Araneaモジュール一括インポート
// 共通モジュール(global/)とIS10固有モジュールを一括でインクルード
// 詳細は AraneaGlobalImporter.h を参照
// ============================================================
#include "AraneaGlobalImporter.h"

// FreeRTOS for stack monitoring
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/semphr.h"

// ========================================
// Serialミューテックス（SSHタスクとのSerial競合防止）
// ========================================
static SemaphoreHandle_t gSerialMutex = nullptr;

// スレッドセーフなSerial出力マクロ
#define SERIAL_PRINTF(...) do { \
  if (gSerialMutex) xSemaphoreTake(gSerialMutex, portMAX_DELAY); \
  Serial.printf(__VA_ARGS__); \
  if (gSerialMutex) xSemaphoreGive(gSerialMutex); \
} while(0)

#define SERIAL_PRINTLN(msg) do { \
  if (gSerialMutex) xSemaphoreTake(gSerialMutex, portMAX_DELAY); \
  Serial.println(msg); \
  if (gSerialMutex) xSemaphoreGive(gSerialMutex); \
} while(0)

// ========================================
// デバッグ用: スタック残量ログ
// ========================================
static void logStack(const char* tag) {
  UBaseType_t words = uxTaskGetStackHighWaterMark(NULL);
  Serial.printf("[STACK] %s high_water=%u words (%u bytes)\n",
                tag, (unsigned)words, (unsigned)(words * sizeof(StackType_t)));
  // REMOVED: Serial.flush() - ブロッキング処理削除
}

// ========================================
// デバイス定数
// ========================================
static const char* PRODUCT_TYPE = "010";
static const char* PRODUCT_CODE = "0001";

// ========================================
// デバイス識別子（用途別分離 - 仕様書準拠）
// ========================================
// AraneaDeviceGate / deviceStateReport用（canonical）
// mobes側でaraneaDeviceConfigのバリデーション・MQTT対象判定に使用
static const char* ARANEA_DEVICE_TYPE = "aranea_ar-is10";

// CelestialGlobe payload.source用（別軸）
// Universal Ingest JSONのpayload.sourceに入れる
static const char* CELESTIAL_SOURCE = "ar-is10";

// AP SSID / hostname等 表示用
// araneaDeviceGateのuserObject.typeには使用禁止
static const char* DEVICE_SHORT_NAME = "ar-is10";

static const char* DEVICE_MODEL = "Openwrt_RouterInspector";
static const char* FIRMWARE_VERSION = "1.2.1";  // mobes連携品質改善

// テナント設定はAraneaSettings.h で定義
// 大量生産: AraneaSettings.hを施設用に編集してビルド
// 汎用: APモードのWeb UIから設定

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
static const unsigned long ROUTER_POLL_INTERVAL_MS = 30000;   // ルーターポーリング間隔（30秒）
static const unsigned long SSH_TIMEOUT_MS = 90000;            // SSHタイムアウト（ASUSWRT遅延対策: 90秒）
static const int SSH_RETRY_COUNT = 2;                         // リトライ回数
static const unsigned long ROUTER_INTERVAL_MS = 30000;        // 次ルーターまでのインターバル
static const unsigned long DISPLAY_UPDATE_MS = 1000;
static const unsigned long BUTTON_HOLD_MS = 3000;
static const unsigned long AP_MODE_TIMEOUT_MS = 300000;       // APモード5分タイムアウト

// ルーター型はRouterTypes.hで定義
// MAX_ROUTERS, RouterOsType, RouterConfig
// RouterInfoはSshPollerIs10.hで定義

// ========================================
// グローバル設定構造体
// ========================================
struct Is10GlobalConfig {
  String endpoint;                    // POSTエンドポイント
  unsigned long sshTimeout;           // SSHタイムアウト
  int retryCount;                     // リトライ回数
  unsigned long routerInterval;       // ルーター間インターバル
  String lacisIdPrefix;               // LacisID prefix (デフォルト: 4)
  String routerProductType;           // ルーターのProductType
  String routerProductCode;           // ルーターのProductCode
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
HttpManagerIs10 httpMgr;
OtaManager ota;
HttpOtaManager httpOta;
Operator op;
SshPollerIs10 sshPoller;
StateReporter stateReporter;
StateReporterIs10 stateReporterIs10;
CelestialSenderIs10 celestialSender;
MqttConfigHandler mqttConfigHandler;

// 自機情報
String myLacisId;
String myMac;
String myCic;
String myTid;
String myFid;
String myHostname;

// 設定
Is10GlobalConfig globalConfig;
RouterConfig routers[MAX_ROUTERS];
int routerCount = 0;

// 状態
bool apModeActive = false;
unsigned long apModeStartTime = 0;
unsigned long lastDisplayUpdate = 0;
unsigned long btnWifiPressTime = 0;
unsigned long btnResetPressTime = 0;
String lastPollResult = "---";
unsigned long celestialDueAt = 0;  // CelestialGlobe送信予約時刻（非ブロッキング用）

// ========================================
// MQTT関連（共通モジュール使用）
// ========================================
MqttManager mqtt;
String mqttWsUrl;

// ========================================
// deviceStateReport / heartbeat関連
// ========================================
static const unsigned long HEARTBEAT_INTERVAL_MS = 60000;  // 60秒間隔
// SSOT変数はStateReporterIs10が管理

// ========================================
// 前方宣言
// ========================================
void loadIs10Config();
void saveIs10Config();
void startAPMode();
void stopAPMode();

// ========================================
// APモードSSID生成
// ========================================
String getAPModeSSID() {
  String mac6 = myMac.substring(6);  // 下6桁
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
// IS10設定読み込み（グローバル設定のみ）
// ルーター設定はHttpManagerIs10が共有配列に読み込む
// ========================================
void loadIs10Config() {
  // グローバル設定（Is10Keys.h定数を使用）
  globalConfig.endpoint = settings.getString(Is10Keys::kEndpoint, AraneaSettings::getCloudUrl());
  globalConfig.sshTimeout = settings.getInt(Is10Keys::kTimeout, SSH_TIMEOUT_MS);
  globalConfig.retryCount = settings.getInt(Is10Keys::kRetry, SSH_RETRY_COUNT);
  globalConfig.routerInterval = settings.getInt(Is10Keys::kRtrIntv, ROUTER_INTERVAL_MS);
  globalConfig.lacisIdPrefix = settings.getString(Is10Keys::kLacisPrefix, "4");
  globalConfig.routerProductType = settings.getString(Is10Keys::kRouterPtype, "");
  globalConfig.routerProductCode = settings.getString(Is10Keys::kRouterPcode, "");

  Serial.printf("[CONFIG] endpoint: %s\n", globalConfig.endpoint.c_str());
  Serial.printf("[CONFIG] sshTimeout: %lu\n", globalConfig.sshTimeout);
  Serial.printf("[CONFIG] routerCount: %d\n", routerCount);
}

// ========================================
// IS10設定保存（グローバル設定のみ）
// ルーター設定はHttpManagerIs10が管理
// ========================================
void saveIs10Config() {
  // グローバル設定（Is10Keys.h定数を使用）
  settings.setString(Is10Keys::kEndpoint, globalConfig.endpoint);
  settings.setInt(Is10Keys::kTimeout, globalConfig.sshTimeout);
  settings.setInt(Is10Keys::kRetry, globalConfig.retryCount);
  settings.setInt(Is10Keys::kRtrIntv, globalConfig.routerInterval);
  settings.setString(Is10Keys::kLacisPrefix, globalConfig.lacisIdPrefix);
  settings.setString(Is10Keys::kRouterPtype, globalConfig.routerProductType);
  settings.setString(Is10Keys::kRouterPcode, globalConfig.routerProductCode);
  Serial.println("[CONFIG] Saved global config");
}

// ========================================
// APモード開始
// ========================================
void startAPMode() {
  String apSSID = getAPModeSSID();
  Serial.printf("[AP] Starting AP mode: %s\n", apSSID.c_str());

  WiFi.mode(WIFI_AP);
  WiFi.softAP(apSSID.c_str());  // パスワードなし

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

// SSH/StateReport関連関数はモジュールに移動済み

// ========================================
// stateReport送信ヘルパー（StateReporter経由）
// ========================================
bool sendStateReport() {
  String payload = stateReporterIs10.buildPayload();
  if (payload.length() == 0) return false;
  return stateReporter.sendReport(payload);
}

// MQTT config処理はMqttConfigHandlerに移動済み

// ========================================
// MQTTメッセージ処理（topic振り分け）
// MqttManagerのonMessageコールバック用
// ========================================
void handleMqttMessage(const String& topic, const char* data, int dataLen) {
  Serial.printf("[MQTT] Topic: %s\n", topic.c_str());

  // configトピック
  if (topic.indexOf("/config") >= 0) {
    mqttConfigHandler.handleConfigMessage(data, dataLen);
    return;
  }

  // commandトピック（将来用）
  if (topic.indexOf("/command") >= 0) {
    Serial.println("[MQTT] Command topic received (not implemented for is10)");
    return;
  }

  Serial.println("[MQTT] Unknown topic, ignoring");
}

// ========================================
// MQTT接続コールバック
// MqttManagerのonConnectedで呼ばれる
// ========================================
void onMqttConnected() {
  // Subscribe to config and command topics
  String cfgTopic = "aranea/" + myTid + "/" + myLacisId + "/config";
  String cmdTopic = "aranea/" + myTid + "/" + myLacisId + "/command";

  mqtt.subscribe(cfgTopic, 1);
  mqtt.subscribe(cmdTopic, 1);

  // HttpManagerのMQTTステータス更新
  httpMgr.setMqttStatus(true);
}

// ========================================
// MQTT切断コールバック
// MqttManagerのonDisconnectedで呼ばれる
// ========================================
void onMqttDisconnected() {
  httpMgr.setMqttStatus(false);
}

// CelestialGlobe送信はCelestialSenderIs10に移動済み

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
      SPIFFS.remove("/routers.json");
      SPIFFS.remove("/aranea_settings.json");
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
  String statusLine = "R:" + String(routerCount) + " OK:" + String(sshPoller.getSuccessCount()) + " NG:" + String(sshPoller.getFailCount());

  display.showIs02Main(line1, cicStr, statusLine, false);
}

// ========================================
// Setup
// ========================================
void setup() {
  // ========================================
  // STEP 0: Brownout Detection - 分類情報取得のため有効のまま
  // ========================================
  // 注: 以下をコメントアウトして rst:0x1 の本当の意味を確認する
  // WRITE_PERI_REG(RTC_CNTL_BROWN_OUT_REG, 0);

  // ========================================
  // STEP 1: RTC Boot Count & Stage Log
  // ========================================
  bootCount++;
  lastStage = STAGE_BOOT_START;

  Serial.begin(115200);
  delay(50);
  lastStage = STAGE_SERIAL_INIT;

  // リセット理由と前回クラッシュ位置を出力
  esp_reset_reason_t resetReason = esp_reset_reason();
  Serial.println("\n========================================");
  Serial.println("[CRASH DEBUG] Reset Reason Analysis");
  Serial.println("========================================");
  Serial.printf("[BOOT] bootCount=%u\n", bootCount);
  Serial.printf("[BOOT] esp_reset_reason=%d (%s)\n", (int)resetReason, resetReasonStr(resetReason));
  Serial.printf("[BOOT] rtc_get_reset_reason(0)=%d\n", rtc_get_reset_reason(0));
  Serial.printf("[BOOT] rtc_get_reset_reason(1)=%d\n", rtc_get_reset_reason(1));
  Serial.printf("[BOOT] lastStage(previous)=%u (%s)\n", lastStage, stageStr(lastStage));
  Serial.printf("[BOOT] heap=%u, largest=%u\n",
    ESP.getFreeHeap(), heap_caps_get_largest_free_block(MALLOC_CAP_8BIT));
  Serial.println("========================================");

  Serial.println("\n[BOOT] is10 (ar-is10) starting...");
  Serial.printf("[BOOT] Model: %s, Version: %s\n", DEVICE_MODEL, FIRMWARE_VERSION);

  // GPIO初期化
  lastStage = STAGE_GPIO_INIT;
  initGPIO();

  // Operator初期化
  op.setMode(OperatorMode::PROVISION);

  // SPIFFS初期化
  lastStage = STAGE_SPIFFS_INIT;
  Serial.printf("[STAGE] %s\n", stageStr(lastStage));
  if (!SPIFFS.begin(true)) {
    Serial.println("[ERROR] SPIFFS init failed");
  }

  // AraneaSettings初期化（SPIFFS設定ロード）
  AraneaSettings::init();

  // SettingManager初期化（NVS）
  lastStage = STAGE_SETTINGS_INIT;
  Serial.printf("[STAGE] %s\n", stageStr(lastStage));
  if (!settings.begin("isms")) {
    Serial.println("[ERROR] Settings init failed");
    return;
  }

  // デバイス固有のデフォルト設定を投入
  AraneaSettings::initDefaults(settings);

  // DisplayManager初期化
  lastStage = STAGE_DISPLAY_INIT;
  Serial.printf("[STAGE] %s\n", stageStr(lastStage));
  if (!display.begin()) {
    Serial.println("[WARNING] Display init failed");
  }
  display.showBoot("Booting...");

  // LacisID生成（WiFi接続前にMACを取得）
  lastStage = STAGE_LACIS_GEN;
  Serial.printf("[STAGE] %s\n", stageStr(lastStage));
  myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
  myLacisId.toUpperCase();  // 大文字正規化（mobes連携で必須）
  myMac = lacisGen.getStaMac12Hex();
  myMac.toUpperCase();  // 大文字正規化（コロンなし12HEX）
  myHostname = getHostname();
  Serial.printf("[LACIS] ID: %s, MAC: %s\n", myLacisId.c_str(), myMac.c_str());
  Serial.printf("[HOST] Hostname: %s\n", myHostname.c_str());

  // WiFi接続試行
  lastStage = STAGE_WIFI_START;
  Serial.printf("[STAGE] %s\n", stageStr(lastStage));
  display.showConnecting("WiFi...", 0);
  wifi.setHostname(myHostname);  // ホスト名を設定（WiFi接続前に必要）
  if (!wifi.connectWithSettings(&settings)) {
    Serial.println("[WIFI] Connection failed, starting AP mode");
    startAPMode();
  } else {
    lastStage = STAGE_WIFI_DONE;
    Serial.printf("[STAGE] %s\n", stageStr(lastStage));
    Serial.printf("[WIFI] Connected: %s\n", wifi.getIP().c_str());

    // NTP同期
    lastStage = STAGE_NTP_SYNC;
    Serial.printf("[STAGE] %s\n", stageStr(lastStage));
    display.showBoot("NTP sync...");
    if (!ntp.sync()) {
      Serial.println("[WARNING] NTP sync failed");
    }

    // AraneaGateway登録
    lastStage = STAGE_ARANEA_REG;
    Serial.printf("[STAGE] %s\n", stageStr(lastStage));
    display.showRegistering("Registering...");

    // テナント設定をSettingManager→AraneaSettingsの順でフォールバック
    myTid = settings.getString("tid", ARANEA_DEFAULT_TID);
    myFid = settings.getString("fid", ARANEA_DEFAULT_FID);

    String gateUrl = AraneaSettings::getGateUrl();
    araneaReg.begin(gateUrl);

    // 認証3要素: lacisId + userId + cic（passは廃止）
    TenantPrimaryAuth tenantAuth;
    tenantAuth.lacisId = settings.getString("tenant_lacisid", ARANEA_DEFAULT_TENANT_LACISID);
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
      // 登録失敗時は保存済みCICを使用
      myCic = settings.getString("cic", "");
      if (myCic.length() > 0) {
        Serial.printf("[REG] Failed, using saved CIC: %s\n", myCic.c_str());
      } else {
        // CICがない場合はエラー状態
        Serial.println("[REG] ERROR: No CIC available, device cannot operate");
        Serial.println("[REG] Check network connection and retry registration");
        display.showError("No CIC!");
        // 登録フラグをクリアして次回起動時に再試行
        araneaReg.clearRegistration();
        // 5秒後にリブートして再試行
        delay(5000);
        ESP.restart();
      }
    }
  }

  // IS10設定読み込み
  lastStage = STAGE_CONFIG_LOAD;
  Serial.printf("[STAGE] %s\n", stageStr(lastStage));
  loadIs10Config();

  // OtaManager初期化 - DISABLED due to mDNS conflict with LibSSH
  // Use HTTP OTA (httpOta) instead for firmware updates
  /*
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
  */

  // HttpManager開始（ルーター配列を共有）- Core 3.0.5で再有効化
  httpMgr.begin(&settings, routers, &routerCount, 80);

  // デバイス情報設定（AraneaWebUI共通形式）
  AraneaDeviceInfo devInfo;
  devInfo.deviceType = DEVICE_SHORT_NAME;
  devInfo.modelName = "Router Inspector";
  devInfo.contextDesc = "ルーターの状態監視とクライアント情報収集を行います";
  devInfo.lacisId = myLacisId;
  devInfo.cic = myCic;
  devInfo.mac = myMac;
  devInfo.hostname = myHostname;
  devInfo.firmwareVersion = FIRMWARE_VERSION;
  devInfo.buildDate = __DATE__ " " __TIME__;
  devInfo.modules = "WiFi,NTP,SSH,MQTT,HTTP";
  httpMgr.setDeviceInfo(devInfo);

  // ルーターステータス初期化（totalRouters=0問題の修正）
  httpMgr.setRouterStatus(routerCount, 0, 0);
  Serial.printf("[HTTP] Initial router status: %d routers configured\n", routerCount);

  // deviceName変更時の即時deviceStateReport送信（SSOT）
  httpMgr.onDeviceNameChanged([]() {
    Serial.println("[WebUI] deviceName changed - sending immediate deviceStateReport");
    sendStateReport();
  });

  // ========================================
  // StateReporter初期化（送信機能:global + 構築:is10固有）
  // ========================================
  String stateEndpoint = araneaReg.getSavedStateEndpoint();
  stateReporter.begin(stateEndpoint);
  stateReporter.setHeartbeatInterval(HEARTBEAT_INTERVAL_MS);

  stateReporterIs10.begin(&stateReporter, &settings, &araneaReg, &ntp, &mqtt,
                           &sshPoller, routers, &routerCount);
  stateReporterIs10.setDeviceType(ARANEA_DEVICE_TYPE);
  stateReporterIs10.setLacisId(myLacisId);
  stateReporterIs10.setMac(myMac);
  stateReporterIs10.setHostname(myHostname);
  stateReporterIs10.setTid(myTid);
  stateReporterIs10.setCic(myCic);
  stateReporterIs10.setHttpManager(&httpMgr);

  // HttpManagerにステータス反映コールバック
  stateReporter.onReportSent([](bool success, int httpCode) {
    String now = ntp.isSynced() ? ntp.getIso8601() : "";
    httpMgr.setLastStateReport(now, httpCode);
  });

  // ========================================
  // CelestialSender初期化
  // ========================================
  celestialSender.begin(&settings, &ntp, &sshPoller);
  celestialSender.setAuth(myTid, myLacisId, myCic, myFid);
  celestialSender.setSource(CELESTIAL_SOURCE);  // "ar-is10"

  // ========================================
  // MqttConfigHandler初期化
  // ========================================
  mqttConfigHandler.begin(&settings, &ntp, routers, &routerCount, &httpMgr, &stateReporterIs10);
  mqttConfigHandler.onConfigApplied([]() {
    Serial.println("[CONFIG] Sending immediate deviceStateReport...");
    if (sendStateReport()) {
      Serial.println("[CONFIG] Immediate stateReport sent OK");
    } else {
      Serial.println("[CONFIG] Immediate stateReport failed (will retry in heartbeat)");
    }
  });

  // ========================================
  // deviceStateReport初回送信（必須）
  // ========================================
  display.showBoot("StateReport...");
  if (sendStateReport()) {
    stateReporter.setInitialReportSent(true);
    Serial.println("[STATE] Initial state report sent");
  } else {
    Serial.println("[STATE] WARNING: Initial state report failed");
  }

  // ========================================
  // MQTT接続（双方向デバイス用）
  // ========================================
  mqttWsUrl = araneaReg.getSavedMqttEndpoint();

  String hashShort = stateReporterIs10.getConfigHash().length() > 8
    ? stateReporterIs10.getConfigHash().substring(0, 8) + "..."
    : stateReporterIs10.getConfigHash();
  Serial.printf("[CONFIG] Saved Applied state: schema=%d, hash=%s\n",
    stateReporterIs10.getSchemaVersion(), hashShort.c_str());

  if (mqttWsUrl.length() > 0) {
    display.showBoot("MQTT...");
    Serial.printf("[MQTT] Endpoint: %s\n", mqttWsUrl.c_str());

    // MqttManagerコールバック設定
    mqtt.onConnected(onMqttConnected);
    mqtt.onDisconnected(onMqttDisconnected);
    mqtt.onMessage(handleMqttMessage);

    // MQTT接続開始
    mqtt.begin(mqttWsUrl, myLacisId, myCic);
  } else {
    // MQTT endpointが空 = legacy登録 or サーバ側でtype未対応
    Serial.println("[MQTT] WARNING: No mqttEndpoint available");
    Serial.println("[MQTT] Possible causes:");
    Serial.printf("[MQTT]   1. Device type '%s' not registered on mobes\n", ARANEA_DEVICE_TYPE);
    Serial.println("[MQTT]   2. Legacy registration without mqttEndpoint");
    Serial.println("[MQTT] Solution: Clear registration (hold RESET 3s) and re-register");
    Serial.println("[MQTT] Or configure type on mobes araneaDeviceConfig");
  }

  // HTTP OTA開始（httpMgrのWebServerを共有）
  httpOta.begin(httpMgr.getServer());
  httpOta.onStart([]() {
    op.setOtaUpdating(true);
    display.showBoot("HTTP OTA Start...");
    Serial.println("[HTTP-OTA] Update started");
  });
  httpOta.onProgress([](int progress) {
    display.showBoot("HTTP OTA: " + String(progress) + "%");
  });
  httpOta.onError([](const String& err) {
    op.setOtaUpdating(false);
    display.showError("HTTP OTA: " + err);
  });
  Serial.println("[HTTP-OTA] Enabled");

  // RUNモードへ
  op.setMode(OperatorMode::RUN);

  // 3秒間の安定化待機（yield()を入れてバックグラウンドタスクを処理）
  Serial.println("[BOOT] Waiting for all modules to stabilize (3s)...");
  for (int i = 0; i < 30; i++) {
    delay(100);
    yield();
  }
  Serial.println("[BOOT] Stabilization complete");

  // ========================================
  // Serialミューテックス初期化（SSHタスクとの競合防止）
  // ========================================
  gSerialMutex = xSemaphoreCreateMutex();
  if (gSerialMutex == nullptr) {
    Serial.println("[ERROR] Failed to create Serial mutex");
  } else {
    Serial.println("[MUTEX] Serial mutex created");
  }

  // ========================================
  // SSH Poller初期化
  // ========================================
  lastStage = STAGE_SSH_BEGIN;
  Serial.printf("[STAGE] %s\n", stageStr(lastStage));

  sshPoller.begin(routers, &routerCount, globalConfig.sshTimeout);
  sshPoller.setRouterInterval(globalConfig.routerInterval);
  sshPoller.setPollInterval(ROUTER_POLL_INTERVAL_MS);
  sshPoller.setSerialMutex(gSerialMutex);  // Serial出力ミューテックス統一

  // サイクル完了時にCelestialGlobe送信予約（非ブロッキング）
  sshPoller.onCycleComplete([](int success, int total) {
    Serial.printf("[SSH-POLLER] Cycle complete: %d/%d\n", success, total);
    httpMgr.setRouterStatus(total, success, millis());

    // CelestialGlobe送信を3秒後に予約（メモリ回復待機）
    // delay()はメインループをブロックするため非使用
    celestialDueAt = millis() + 3000;
  });

  Serial.println("[SSH-POLLER] Initialized");

  lastStage = STAGE_SSH_DONE;
  Serial.printf("[STAGE] %s\n", stageStr(lastStage));
  Serial.printf("[SSH] POST-INIT heap=%u, largest=%u\n",
    ESP.getFreeHeap(), heap_caps_get_largest_free_block(MALLOC_CAP_8BIT));

  // ========================================
  // Setup Complete
  // ========================================
  lastStage = STAGE_SETUP_COMPLETE;
  Serial.printf("[STAGE] %s\n", stageStr(lastStage));
  Serial.println("[BOOT] is10 ready!");
}

// ========================================
// Loop
// ========================================
void loop() {
  static unsigned long loopCount = 0;
  static unsigned long lastLoopLog = 0;
  unsigned long now = millis();

  loopCount++;

  // loop開始時にステージ更新
  if (loopCount == 1) {
    lastStage = STAGE_LOOP_RUNNING;
  }

  // ========================================
  // ステータス出力（30秒毎のみ - ミニマル版準拠）
  // ========================================
  if (now - lastLoopLog >= 30000) {
    Serial.printf("[STATUS] heap=%d largest=%u\n",
      ESP.getFreeHeap(),
      heap_caps_get_largest_free_block(MALLOC_CAP_8BIT));
    lastLoopLog = now;
  }

  // ========================================
  // 1. OTA処理チェック
  // ========================================
  // OTA処理（最優先）- ArduinoOTA disabled due to mDNS conflict with LibSSH
  // ota.handle();  // Use HTTP OTA instead

  if (op.isOtaUpdating()) {
    delay(10);
    return;
  }

  // ========================================
  // 2. HTTP処理
  // ========================================
  httpMgr.handle();

  // ========================================
  // 2.5 MQTT処理（自動再接続含む）
  // ========================================
  if (!apModeActive && mqttWsUrl.length() > 0) {
    mqtt.handle();
  }

  // ========================================
  // 3. APモードタイムアウトチェック
  // ========================================
  if (apModeActive && (now - apModeStartTime >= AP_MODE_TIMEOUT_MS)) {
    Serial.println("[AP] Timeout, attempting STA reconnect");
    stopAPMode();
    wifi.connectWithSettings(&settings);
    if (!wifi.isConnected()) {
      startAPMode();
    }
  }

  // ========================================
  // 4. ルーターポーリング（SshPollerIs10経由）
  // ========================================
  // ポーリングステータスを更新
  httpMgr.setPollingStatus(sshPoller.getCurrentIndex(), sshPoller.isPolling());

  // SshPollerIs10の有効/無効をhttpMgrと同期
  sshPoller.setEnabled(!apModeActive && routerCount > 0 && httpMgr.isPollingEnabled());

  // SshPollerIs10のメインループ処理
  sshPoller.handle();

  // SshPollerからrestart要求があれば再起動
  // watchdogタイムアウト等で立つ。ランダム開始インデックスで故障ルーター回避
  if (sshPoller.needsRestart()) {
    Serial.printf("[SSH-POLLER] Restart requested: %s\n", sshPoller.getRestartReason().c_str());
    delay(1000);  // ログ出力待ち
    ESP.restart();
  }

  // CelestialGlobe送信（予約時刻到達時）
  if (celestialDueAt > 0 && now >= celestialDueAt) {
    celestialDueAt = 0;  // 予約クリア
    uint32_t largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
    if (largestBlock >= 16000) {
      celestialSender.send();
    } else {
      Serial.printf("[LOOP] Skip CelestialGlobe - low memory (%u < 16000)\n", largestBlock);
    }
  }

  // ========================================
  // 4.5 deviceStateReport heartbeat（StateReporter経由）
  // ========================================
  if (!apModeActive) {
    stateReporter.handle();
  }

  // ========================================
  // 5. ディスプレイ更新（毎秒）
  // ========================================
  if (now - lastDisplayUpdate >= DISPLAY_UPDATE_MS) {
    updateDisplay();
    lastDisplayUpdate = now;
  }

  // ========================================
  // 6. ボタン処理
  // ========================================
  handleButtons();

  // ========================================
  // 7. WiFi再接続チェック
  // ========================================
  if (!apModeActive && !wifi.isConnected()) {
    Serial.println("[WIFI] Disconnected, reconnecting...");
    display.showConnecting("Reconnect...", 0);

    if (!wifi.connectWithSettings(&settings)) {
      startAPMode();
    } else {
      // WiFi再接続成功 → MQTT再接続をトリガー
      Serial.println("[WIFI] Reconnected, triggering MQTT reconnect...");
      if (mqttWsUrl.length() > 0) {
        mqtt.reconnect();
      }
    }
  }

  yield();
  delay(10);
}
