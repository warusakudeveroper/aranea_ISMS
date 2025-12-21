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

// MQTT用（共通モジュール）
#include "MqttManager.h"

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

// AraneaGlobal共通モジュール
#include "AraneaSettings.h"
#include "SettingManager.h"
#include "DisplayManager.h"
#include "WiFiManager.h"
#include "NtpManager.h"
#include "LacisIDGenerator.h"
#include "AraneaRegister.h"
#include "RouterTypes.h"
#include "HttpManagerIs10.h"
#include "OtaManager.h"
#include "HttpOtaManager.h"
#include "Operator.h"
#include "SshClient.h"

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

// ========================================
// ルーター情報構造体（取得データ）
// ========================================
struct RouterInfo {
  String rid;
  String lacisId;       // ルーターのLacisID（MACから生成）- 送信しない
  String routerMac;     // ルーターのMAC（大文字12HEX）- clients[].apMac用
  String wanIp;
  String lanIp;
  String ssid24;        // 2.4GHz SSID
  String ssid50;        // 5.0GHz SSID
  String ssidMix;       // 混合SSID
  int clientCount;
  String vendorInfo;
  String modelName;
  unsigned long lastUpdate;
  bool online;
};

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
int currentRouterIndex = 0;
bool apModeActive = false;
unsigned long apModeStartTime = 0;
unsigned long lastRouterPoll = 0;
unsigned long lastDisplayUpdate = 0;
unsigned long btnWifiPressTime = 0;
unsigned long btnResetPressTime = 0;
String lastPollResult = "---";
int successCount = 0;
int failCount = 0;

// HTTPS用のグローバルSecureClient（TLSバッファ再利用でメモリ節約）
// DISABLED: LibSSH-ESP32 TLS競合テストのため無効化
// WiFiClientSecure secureClient;
// bool secureClientInitialized = false;

// SSH用のグローバルクライアント（メモリフラグメンテーション防止）
SshClient* globalSshClient = nullptr;

// SSH完了後のクリーンアップをメインループで行うためのフラグ
// SSHタスク内から ssh_finalize() を呼ぶとクラッシュするため、
// メインループのコンテキストで呼び出す
volatile bool sshCleanupNeeded = false;
TaskHandle_t sshTaskHandle = nullptr;

// フラグベースSSH制御（ミニマル版アプローチ）
volatile bool sshInProgress = false;
volatile bool sshDone = false;
volatile bool sshSuccess = false;
unsigned long sshTaskStartTime = 0;  // タスク開始時刻（ウォッチドッグ用）

// SSHタスクウォッチドッグ: libssh timeout * 2 + バッファ
// libssh timeout = 30秒、ウォッチドッグ = 90秒
static const unsigned long SSH_TASK_WATCHDOG_MS = 90000;

// ========================================
// MQTT関連（共通モジュール使用）
// ========================================
MqttManager mqtt;
String mqttWsUrl;

// ========================================
// MQTT config双方向同期（SSOT）
// ========================================
int savedSchemaVersion = 0;        // NVS保存済みschemaVersion
String savedConfigHash = "";       // サーバ発行のconfigHash（is10は計算不要）
String lastConfigAppliedAt = "";   // ISO8601形式
String lastConfigApplyError = "";  // 最後のエラーメッセージ（空=成功）

// ========================================
// deviceStateReport / heartbeat関連
// ========================================
static const unsigned long HEARTBEAT_INTERVAL_MS = 60000;  // 60秒間隔
unsigned long lastHeartbeat = 0;
bool initialStateReportSent = false;

// ========================================
// CelestialGlobe送信用ルーター情報配列
// SSHタスクで取得した情報を格納
// ========================================
static RouterInfo collectedRouterInfo[MAX_ROUTERS];
static int collectedRouterCount = 0;

// ========================================
// SSH専用タスク用構造体（32KB スタック）
// LibSSH-ESP32公式exampleに従い、SSH操作を専用タスクで実行
// ========================================
static const uint32_t SSH_TASK_STACK_SIZE = 32768;  // 32KB (heapLargest制約に対応)

struct SshTaskParams {
  const RouterConfig* router;
  RouterInfo* info;
  bool success;
  SemaphoreHandle_t doneSem;
};

// SSH専用タスク関数（ミニマル版アプローチ - フラグベース）
// グローバル変数を直接参照し、セマフォではなくフラグで完了通知
// CelestialGlobe送信用にcollectedRouterInfo[]に結果を格納
//
// 修正版 v1.2.1:
// - connectedOk/dataOk の2段階成功判定（接続完了を優先）
// - libssh_begin()/ssh_finalize()はsetup()で1回のみ呼び出し
// - タイムアウトはglobalConfig.sshTimeoutを使用
// - SERIAL_PRINTFでスレッドセーフ出力
void sshTaskFunction(void* pvParameters) {
  int idx = currentRouterIndex;
  RouterConfig& router = routers[idx];

  // 2段階成功判定フラグ
  bool connectedOk = false;  // SSH認証成功
  bool dataOk = false;       // データ取得成功

  SERIAL_PRINTF("[SSH] Router %d/%d: %s (%s)\n", idx + 1, routerCount, router.rid.c_str(), router.ipAddr.c_str());

  // collectedRouterInfo初期化
  RouterInfo& info = collectedRouterInfo[idx];
  info.rid = router.rid;
  info.routerMac = "";  // SSH で取得
  info.wanIp = "";
  info.lanIp = "";
  info.ssid24 = "";
  info.ssid50 = "";
  info.clientCount = 0;
  info.online = false;
  info.lastUpdate = millis();

  // libssh_begin() はsetup()で1回だけ呼び出し済み（削除）

  ssh_session session = ssh_new();
  if (session == NULL) {
    SERIAL_PRINTLN("[SSH] Failed to create session");
    // ssh_finalize() は呼び出さない（グローバル状態を保持）
    sshSuccess = false;
    sshDone = true;
    vTaskDelete(NULL);
    return;
  }

  ssh_options_set(session, SSH_OPTIONS_HOST, router.ipAddr.c_str());
  unsigned int port = router.sshPort;
  ssh_options_set(session, SSH_OPTIONS_PORT, &port);
  ssh_options_set(session, SSH_OPTIONS_USER, router.username.c_str());
  // タイムアウトをglobalConfigから取得（秒単位）
  long timeout = globalConfig.sshTimeout / 1000;
  ssh_options_set(session, SSH_OPTIONS_TIMEOUT, &timeout);

  int rc = ssh_connect(session);
  if (rc != SSH_OK) {
    SERIAL_PRINTF("[SSH] Connect failed: %s\n", ssh_get_error(session));
    ssh_free(session);
    // ssh_finalize() は呼び出さない
    sshSuccess = false;
    sshDone = true;
    vTaskDelete(NULL);
    return;
  }

  rc = ssh_userauth_password(session, NULL, router.password.c_str());
  if (rc != SSH_AUTH_SUCCESS) {
    SERIAL_PRINTF("[SSH] Auth failed: %s\n", ssh_get_error(session));
    ssh_disconnect(session);
    ssh_free(session);
    // ssh_finalize() は呼び出さない
    sshSuccess = false;
    sshDone = true;
    vTaskDelete(NULL);
    return;
  }

  // SSH認証成功 = connectedOk
  connectedOk = true;
  info.online = true;
  SERIAL_PRINTLN("[SSH] Auth success (connectedOk=true)");

  ssh_channel channel = ssh_channel_new(session);

  if (channel) {
    if (ssh_channel_open_session(channel) == SSH_OK) {
      // WAN IP取得
      if (ssh_channel_request_exec(channel, "nvram get wan0_ipaddr") == SSH_OK) {
        char buf[256];
        int n = ssh_channel_read(channel, buf, sizeof(buf) - 1, 0);
        if (n > 0) {
          buf[n] = '\0';
          // String生成を最小化
          char wanBuf[64];
          strncpy(wanBuf, buf, sizeof(wanBuf) - 1);
          wanBuf[sizeof(wanBuf) - 1] = '\0';
          // trim処理
          char* p = wanBuf;
          while (*p && isspace(*p)) p++;
          char* end = p + strlen(p) - 1;
          while (end > p && isspace(*end)) *end-- = '\0';
          info.wanIp = p;
          SERIAL_PRINTF("[SSH] WAN: %s\n", p);
          dataOk = true;  // データ取得成功
        }
      }
      ssh_channel_send_eof(channel);
      ssh_channel_close(channel);
    }
    ssh_channel_free(channel);  // open_session失敗時もfree
    channel = nullptr;
  }

  // 追加コマンド: LAN IP
  channel = ssh_channel_new(session);
  if (channel) {
    if (ssh_channel_open_session(channel) == SSH_OK) {
      if (ssh_channel_request_exec(channel, "nvram get lan_ipaddr") == SSH_OK) {
        char buf[256];
        int n = ssh_channel_read(channel, buf, sizeof(buf) - 1, 0);
        if (n > 0) {
          buf[n] = '\0';
          char* p = buf;
          while (*p && isspace(*p)) p++;
          char* end = p + strlen(p) - 1;
          while (end > p && isspace(*end)) *end-- = '\0';
          info.lanIp = p;
        }
      }
      ssh_channel_send_eof(channel);
      ssh_channel_close(channel);
    }
    ssh_channel_free(channel);
    channel = nullptr;
  }

  // 追加コマンド: 2.4GHz SSID
  channel = ssh_channel_new(session);
  if (channel) {
    if (ssh_channel_open_session(channel) == SSH_OK) {
      if (ssh_channel_request_exec(channel, "nvram get wl0_ssid") == SSH_OK) {
        char buf[256];
        int n = ssh_channel_read(channel, buf, sizeof(buf) - 1, 0);
        if (n > 0) {
          buf[n] = '\0';
          char* p = buf;
          while (*p && isspace(*p)) p++;
          char* end = p + strlen(p) - 1;
          while (end > p && isspace(*end)) *end-- = '\0';
          info.ssid24 = p;
        }
      }
      ssh_channel_send_eof(channel);
      ssh_channel_close(channel);
    }
    ssh_channel_free(channel);
    channel = nullptr;
  }

  // 追加コマンド: 5GHz SSID
  channel = ssh_channel_new(session);
  if (channel) {
    if (ssh_channel_open_session(channel) == SSH_OK) {
      if (ssh_channel_request_exec(channel, "nvram get wl1_ssid") == SSH_OK) {
        char buf[256];
        int n = ssh_channel_read(channel, buf, sizeof(buf) - 1, 0);
        if (n > 0) {
          buf[n] = '\0';
          char* p = buf;
          while (*p && isspace(*p)) p++;
          char* end = p + strlen(p) - 1;
          while (end > p && isspace(*end)) *end-- = '\0';
          info.ssid50 = p;
        }
      }
      ssh_channel_send_eof(channel);
      ssh_channel_close(channel);
    }
    ssh_channel_free(channel);
    channel = nullptr;
  }

  // 追加コマンド: クライアント数（DHCPリース）
  channel = ssh_channel_new(session);
  if (channel) {
    if (ssh_channel_open_session(channel) == SSH_OK) {
      if (ssh_channel_request_exec(channel, "cat /var/lib/misc/dnsmasq.leases 2>/dev/null | wc -l") == SSH_OK) {
        char buf[64];
        int n = ssh_channel_read(channel, buf, sizeof(buf) - 1, 0);
        if (n > 0) {
          buf[n] = '\0';
          info.clientCount = atoi(buf);
        }
      }
      ssh_channel_send_eof(channel);
      ssh_channel_close(channel);
    }
    ssh_channel_free(channel);
    channel = nullptr;
  }

  ssh_disconnect(session);
  ssh_free(session);
  // ssh_finalize() は呼び出さない（グローバル状態を保持）

  info.lastUpdate = millis();

  // 成功判定: connectedOk（SSH認証成功）を優先
  // dataOk が false でも connectedOk が true なら完走扱い
  sshSuccess = connectedOk;
  SERIAL_PRINTF("[SSH] Done: connectedOk=%d, dataOk=%d, sshSuccess=%d\n",
                connectedOk, dataOk, sshSuccess);

  sshDone = true;
  vTaskDelete(NULL);
}

// ========================================
// 前方宣言
// ========================================
void loadIs10Config();
void saveIs10Config();
void startAPMode();
void stopAPMode();
bool startSshTask(int index);
bool executeSSHCommands(const RouterConfig& router, RouterInfo& info);
// bool postRouterInfo(const RouterInfo& info);  // DISABLED for TLS isolation test
String generateRouterLacisId(const String& mac);

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
  // グローバル設定
  globalConfig.endpoint = settings.getString("is10_endpoint", AraneaSettings::getCloudUrl());
  globalConfig.sshTimeout = settings.getInt("is10_timeout", SSH_TIMEOUT_MS);
  globalConfig.retryCount = settings.getInt("is10_retry", SSH_RETRY_COUNT);
  globalConfig.routerInterval = settings.getInt("is10_rtr_intv", ROUTER_INTERVAL_MS);
  globalConfig.lacisIdPrefix = settings.getString("is10_lacis_prefix", "4");
  globalConfig.routerProductType = settings.getString("is10_router_ptype", "");
  globalConfig.routerProductCode = settings.getString("is10_router_pcode", "");

  Serial.printf("[CONFIG] endpoint: %s\n", globalConfig.endpoint.c_str());
  Serial.printf("[CONFIG] sshTimeout: %lu\n", globalConfig.sshTimeout);
  Serial.printf("[CONFIG] routerCount: %d\n", routerCount);
}

// ========================================
// IS10設定保存（グローバル設定のみ）
// ルーター設定はHttpManagerIs10が管理
// ========================================
void saveIs10Config() {
  // グローバル設定
  settings.setString("is10_endpoint", globalConfig.endpoint);
  settings.setInt("is10_timeout", globalConfig.sshTimeout);
  settings.setInt("is10_retry", globalConfig.retryCount);
  settings.setInt("is10_rtr_intv", globalConfig.routerInterval);
  settings.setString("is10_lacis_prefix", globalConfig.lacisIdPrefix);
  settings.setString("is10_router_ptype", globalConfig.routerProductType);
  settings.setString("is10_router_pcode", globalConfig.routerProductCode);
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

// ========================================
// ルーターLacisID生成
// ========================================
String generateRouterLacisId(const String& mac) {
  // MACアドレスからLacisIDを生成
  // フォーマット: [prefix][productType][MAC12桁][productCode]
  String cleanMac = mac;
  cleanMac.replace(":", "");
  cleanMac.replace("-", "");
  cleanMac.toUpperCase();

  if (cleanMac.length() != 12) {
    return "";
  }

  String prefix = globalConfig.lacisIdPrefix;
  String ptype = globalConfig.routerProductType;
  String pcode = globalConfig.routerProductCode;

  // 仮のLacisID（ProductType/Codeが未設定の場合）
  if (ptype.length() == 0) ptype = "000";
  if (pcode.length() == 0) pcode = "0000";

  return prefix + ptype + cleanMac + pcode;
}

// ========================================
// 出力パース用ヘルパー関数
// ========================================
String parseIpFromIfconfig(const String& output) {
  // "inet addr:192.168.1.1" または "inet 192.168.1.1" を抽出
  int pos = output.indexOf("inet ");
  if (pos < 0) pos = output.indexOf("inet:");
  if (pos < 0) return "";

  String sub = output.substring(pos + 5);
  sub.replace("addr:", "");
  sub.trim();

  int spacePos = sub.indexOf(' ');
  if (spacePos > 0) {
    sub = sub.substring(0, spacePos);
  }
  return sub;
}

String parseSsidFromUci(const String& output, const String& band) {
  // "wireless.wlan0.ssid='MySSID'" を抽出
  String search = band + ".ssid='";
  int pos = output.indexOf(search);
  if (pos < 0) {
    search = band + ".ssid=";
    pos = output.indexOf(search);
  }
  if (pos < 0) return "";

  int start = pos + search.length();
  int end = output.indexOf("'", start);
  if (end < 0) end = output.indexOf("\n", start);
  if (end < 0) end = output.length();

  return output.substring(start, end);
}

int parseClientCount(const String& output) {
  // DHCPリースの行数をカウント
  int count = 0;
  int pos = 0;
  while ((pos = output.indexOf('\n', pos)) >= 0) {
    count++;
    pos++;
  }
  return count;
}

// ========================================
// SSH経由でルーター情報取得
// ========================================
bool executeSSHCommands(const RouterConfig& router, RouterInfo& info) {
  bool isAsuswrt = (router.osType == RouterOsType::ASUSWRT);
  const char* osName = isAsuswrt ? "ASUSWRT" : "OpenWrt";

  // ログ出力（secretsは出さない: username/password は表示しない）
  Serial.printf("[SSH] Connecting to %s:%d (%s)\n",
    router.ipAddr.c_str(), router.sshPort, osName);

  // 初期化
  info.rid = router.rid;
  info.wanIp = "";
  info.lanIp = "";
  info.ssid24 = "";
  info.ssid50 = "";
  info.clientCount = 0;
  info.online = false;
  info.lastUpdate = millis();

  // グローバルSSHクライアントを確認
  if (globalSshClient == nullptr) {
    Serial.println("[SSH] Creating globalSshClient...");
    globalSshClient = new SshClient();
  }

  // libsshが初期化されていない場合は初期化
  if (!globalSshClient->isInitialized()) {
    globalSshClient->begin();
  }

  SshClient& ssh = *globalSshClient;

  // 既存の接続があれば切断
  if (ssh.isConnected()) {
    ssh.disconnect();
  }

  SshConfig config;
  config.host = router.ipAddr;
  config.port = router.sshPort;
  config.username = router.username;
  config.password = router.password;
  config.timeout = globalConfig.sshTimeout;

  logStack("SSH-before-connect");
  SshResult result = ssh.connect(config);
  logStack("SSH-after-connect");
  if (result != SshResult::OK) {
    Serial.printf("[SSH] Connection failed: %s\n", ssh.getLastError().c_str());
    return false;
  }

  info.online = true;
  SshExecResult execResult;

  // ========================================
  // WAN IP取得
  // ========================================
  if (isAsuswrt) {
    // ASUSWRT: nvramからWAN IPを取得
    execResult = ssh.exec("nvram get wan0_ipaddr 2>/dev/null");
  } else {
    // OpenWrt: ifconfigからWAN IPを取得
    execResult = ssh.exec("ifconfig eth0 2>/dev/null || ifconfig wan 2>/dev/null");
  }
  if (execResult.result == SshResult::OK && execResult.output.length() > 0) {
    if (isAsuswrt) {
      info.wanIp = execResult.output;
      info.wanIp.trim();
    } else {
      info.wanIp = parseIpFromIfconfig(execResult.output);
    }
    Serial.printf("[SSH] WAN IP: %s\n", info.wanIp.c_str());
  }

  // ========================================
  // LAN IP取得
  // ========================================
  if (isAsuswrt) {
    // ASUSWRT: br0インターフェース
    execResult = ssh.exec("ifconfig br0 2>/dev/null");
  } else {
    // OpenWrt: br-lanインターフェース
    execResult = ssh.exec("ifconfig br-lan 2>/dev/null || ifconfig lan 2>/dev/null");
  }
  if (execResult.result == SshResult::OK && execResult.output.length() > 0) {
    info.lanIp = parseIpFromIfconfig(execResult.output);
    Serial.printf("[SSH] LAN IP: %s\n", info.lanIp.c_str());
  }

  // ========================================
  // SSID取得
  // ========================================
  if (isAsuswrt) {
    // ASUSWRT: nvramからSSIDを取得
    execResult = ssh.exec("nvram get wl0_ssid 2>/dev/null");
    if (execResult.result == SshResult::OK && execResult.output.length() > 0) {
      info.ssid24 = execResult.output;
      info.ssid24.trim();
    }
    execResult = ssh.exec("nvram get wl1_ssid 2>/dev/null");
    if (execResult.result == SshResult::OK && execResult.output.length() > 0) {
      info.ssid50 = execResult.output;
      info.ssid50.trim();
    }
  } else {
    // OpenWrt: uciからSSIDを取得
    execResult = ssh.exec("uci show wireless 2>/dev/null | grep ssid");
    if (execResult.result == SshResult::OK && execResult.output.length() > 0) {
      // 2.4GHz (wlan0 or radio0)
      info.ssid24 = parseSsidFromUci(execResult.output, "wlan0");
      if (info.ssid24.length() == 0) {
        info.ssid24 = parseSsidFromUci(execResult.output, "default_radio0");
      }
      // 5GHz (wlan1 or radio1)
      info.ssid50 = parseSsidFromUci(execResult.output, "wlan1");
      if (info.ssid50.length() == 0) {
        info.ssid50 = parseSsidFromUci(execResult.output, "default_radio1");
      }
    }
  }
  Serial.printf("[SSH] SSID 2.4G: %s, 5G: %s\n", info.ssid24.c_str(), info.ssid50.c_str());

  // ========================================
  // DHCPリース（接続クライアント数）
  // ========================================
  if (isAsuswrt) {
    // ASUSWRT: /var/lib/misc/dnsmasq.leases
    execResult = ssh.exec("cat /var/lib/misc/dnsmasq.leases 2>/dev/null");
  } else {
    // OpenWrt: /tmp/dhcp.leases
    execResult = ssh.exec("cat /tmp/dhcp.leases 2>/dev/null");
  }
  if (execResult.result == SshResult::OK && execResult.output.length() > 0) {
    info.clientCount = parseClientCount(execResult.output);
    Serial.printf("[SSH] Client count: %d\n", info.clientCount);
  }

  // ========================================
  // MACアドレス取得（clients[].apMac用 + LacisID生成）
  // ========================================
  if (isAsuswrt) {
    // ASUSWRT: br0インターフェースのMAC
    execResult = ssh.exec("cat /sys/class/net/br0/address 2>/dev/null");
  } else {
    // OpenWrt: br-lanインターフェースのMAC
    execResult = ssh.exec("cat /sys/class/net/br-lan/address 2>/dev/null || cat /sys/class/net/eth0/address 2>/dev/null");
  }
  if (execResult.result == SshResult::OK && execResult.output.length() > 0) {
    String mac = execResult.output;
    mac.trim();
    // routerMac: 大文字12HEX正規化（clients[].apMacで使用）
    String cleanMac = mac;
    cleanMac.replace(":", "");
    cleanMac.replace("-", "");
    cleanMac.toUpperCase();
    info.routerMac = cleanMac;
    // lacisId生成（送信しない、参照用）
    info.lacisId = generateRouterLacisId(mac);
    Serial.printf("[SSH] Router MAC: %s (norm: %s)\n", mac.c_str(), cleanMac.c_str());
  }

  ssh.disconnect();
  Serial.printf("[SSH] Data collection complete (%s)\n", osName);

  return true;
}

// ========================================
// deviceName sanitize（SSOT準拠）
// - trim
// - 制御文字除去（改行/タブ等）
// - 連続空白を1つに圧縮
// - 最大64文字
// ========================================
String sanitizeDeviceName(const String& raw) {
  String result;
  result.reserve(64);

  bool lastWasSpace = true;  // 先頭空白スキップ用
  for (int i = 0; i < raw.length() && result.length() < 64; i++) {
    char c = raw.charAt(i);

    // 制御文字（0x00-0x1F, 0x7F）はスキップまたはスペースに
    if (c < 0x20 || c == 0x7F) {
      if (!lastWasSpace && result.length() > 0) {
        result += ' ';
        lastWasSpace = true;
      }
      continue;
    }

    // スペースの連続圧縮
    if (c == ' ') {
      if (!lastWasSpace) {
        result += c;
        lastWasSpace = true;
      }
      continue;
    }

    result += c;
    lastWasSpace = false;
  }

  // 末尾空白除去
  result.trim();

  return result;
}

// ========================================
// deviceName取得（デフォルト付き）
// ========================================
String getDeviceName() {
  String deviceName = settings.getString("device_name", "");
  deviceName = sanitizeDeviceName(deviceName);

  // 空の場合はホスト名をデフォルトに
  if (deviceName.length() == 0) {
    deviceName = myHostname;  // "ar-is10-XXXXXX"
  }

  return deviceName;
}

// ========================================
// deviceStateReport送信（mobesへの状態報告）
// 仕様書: 起動後1回 + 60秒毎heartbeat
// SSOT双方向同期:
// - state.deviceName
// - state.appliedConfigSchemaVersion / appliedConfigHash
// - state.reportedConfig.is10（オプション）
// ========================================
bool sendDeviceStateReport() {
  String stateEndpoint = araneaReg.getSavedStateEndpoint();
  if (stateEndpoint.length() == 0) {
    Serial.println("[STATE] No stateEndpoint available");
    return false;
  }

  String observedAt = ntp.isSynced() ? ntp.getIso8601() : "1970-01-01T00:00:00Z";
  String deviceName = getDeviceName();  // sanitize済み

  // ReportedConfig用フラグ（デフォルトはsecretsなしで送信）
  bool reportFullConfig = settings.getBool("is10_report_full_config", false);

  // バッファサイズ: Applied + ReportedConfig分を考慮
  DynamicJsonDocument doc(4096);

  // auth
  JsonObject auth = doc.createNestedObject("auth");
  auth["tid"] = myTid;
  auth["lacisId"] = myLacisId;
  auth["cic"] = myCic;

  // report
  JsonObject report = doc.createNestedObject("report");
  report["lacisId"] = myLacisId;
  report["type"] = ARANEA_DEVICE_TYPE;  // canonical: "aranea_ar-is10"
  report["observedAt"] = observedAt;

  // state
  JsonObject state = report.createNestedObject("state");

  // 基本情報
  state["deviceName"] = deviceName;  // mobes userObject.name 連携用
  state["IPAddress"] = WiFi.localIP().toString();
  state["MacAddress"] = myMac;
  state["RSSI"] = WiFi.RSSI();
  state["routerCount"] = routerCount;
  state["successCount"] = successCount;
  state["failCount"] = failCount;
  state["lastPollResult"] = lastPollResult;
  state["mqttConnected"] = mqtt.isConnected();
  state["heap"] = ESP.getFreeHeap();

  // ========================================
  // Applied情報（SSOT必須）
  // mobesのconfigApplied.status判定に使用
  // ========================================
  state["appliedConfigSchemaVersion"] = savedSchemaVersion;
  state["appliedConfigHash"] = savedConfigHash;
  if (lastConfigAppliedAt.length() > 0) {
    state["lastConfigAppliedAt"] = lastConfigAppliedAt;
  }
  if (lastConfigApplyError.length() > 0) {
    state["lastConfigApplyError"] = lastConfigApplyError;
  }

  // ========================================
  // ReportedConfig（現地設定の可視化）
  // hashだけでもよいが、運用のためにis10設定を返す
  // ========================================
  state["reportedConfigSchemaVersion"] = savedSchemaVersion;
  state["reportedConfigHash"] = savedConfigHash;

  // reportedConfig.is10: 現在動作中の設定
  JsonObject reportedConfig = state.createNestedObject("reportedConfig");
  JsonObject is10Cfg = reportedConfig.createNestedObject("is10");

  is10Cfg["scanInterval"] = settings.getInt("is10_scan_interval_sec", 60);
  is10Cfg["reportClientList"] = settings.getBool("is10_report_clients", true);

  // routers配列（secretsは reportFullConfig フラグで制御）
  JsonArray routersArr = is10Cfg.createNestedArray("routers");
  for (int i = 0; i < routerCount; i++) {
    JsonObject r = routersArr.createNestedObject();
    r["rid"] = routers[i].rid;
    r["ipAddr"] = routers[i].ipAddr;
    r["sshPort"] = routers[i].sshPort;
    r["enabled"] = routers[i].enabled;
    r["platform"] = (routers[i].osType == RouterOsType::ASUSWRT) ? "asuswrt" : "openwrt";

    // secrets: reportFullConfig=true の場合のみ含める（運用要件で選択可能）
    if (reportFullConfig) {
      r["sshUser"] = routers[i].username;
      r["password"] = routers[i].password;
      if (routers[i].publicKey.length() > 0) {
        r["privateKey"] = routers[i].publicKey;
      }
    }
  }

  String json;
  serializeJson(doc, json);

  // ログ出力（secretsは出さない、hashは短縮）
  String hashShort = savedConfigHash.length() > 8 ? savedConfigHash.substring(0, 8) + "..." : savedConfigHash;
  Serial.printf("[STATE] deviceName=\"%s\", type=%s, schema=%d, hash=%s\n",
                deviceName.c_str(), ARANEA_DEVICE_TYPE, savedSchemaVersion, hashShort.c_str());
  Serial.printf("[STATE] Sending to %s (%d bytes)\n", stateEndpoint.c_str(), json.length());

  HTTPClient http;
  http.begin(stateEndpoint);
  http.addHeader("Content-Type", "application/json");
  http.setTimeout(10000);

  int httpCode = http.POST(json);
  String response = http.getString();
  http.end();

  bool success = (httpCode >= 200 && httpCode < 300);
  Serial.printf("[STATE] HTTP %d %s\n", httpCode, success ? "OK" : "NG");
  if (!success) {
    Serial.printf("[STATE] Response: %s\n", response.c_str());
  }

  // HttpManagerIs10にもステータス反映（UI用）
  httpMgr.setLastStateReport(observedAt, httpCode);

  return success;
}

// ========================================
// MQTT config適用（routers配列フル更新）
// SSOT: secrets（sshUser/password/privateKey）も受け取り・保存・適用
// CelestialGlobe送信には絶対に載せない
// ========================================
void applyRouterConfig(JsonArray routersJson) {
  // SSOT方針: MQTTからの設定で完全上書き（マージではなく置換）
  // 既存設定はDesiredで上書きされる

  int newCount = 0;
  for (JsonObject r : routersJson) {
    if (newCount >= MAX_ROUTERS) break;

    String rid = r["rid"] | "";
    if (rid.length() == 0) continue;

    RouterConfig& router = routers[newCount];
    router.rid = rid;
    router.ipAddr = r["ipAddr"] | "";
    router.sshPort = r["sshPort"] | 22;
    router.enabled = r["enabled"] | true;

    // secrets: 受け取り・保存・適用OK（CelestialGlobeには載せない）
    // SSOT仕様: sshUser → username, password, privateKey → publicKey
    if (r.containsKey("sshUser")) {
      router.username = r["sshUser"].as<String>();
    } else if (r.containsKey("username")) {
      router.username = r["username"].as<String>();
    }

    if (r.containsKey("password")) {
      router.password = r["password"].as<String>();
    }

    if (r.containsKey("privateKey")) {
      router.publicKey = r["privateKey"].as<String>();  // publicKeyフィールドに格納
    } else if (r.containsKey("publicKey")) {
      router.publicKey = r["publicKey"].as<String>();
    }

    // platform → osType マッピング
    String platform = r["platform"] | "asuswrt";
    platform.toLowerCase();
    if (platform == "openwrt") {
      router.osType = RouterOsType::OPENWRT;
    } else {
      router.osType = RouterOsType::ASUSWRT;  // デフォルト
    }

    // ログ出力（secretsは出さない）
    Serial.printf("[CONFIG] Router %s: %s:%d (%s)\n",
      rid.c_str(), router.ipAddr.c_str(), router.sshPort,
      (router.osType == RouterOsType::ASUSWRT) ? "ASUSWRT" : "OpenWrt");

    newCount++;
  }

  routerCount = newCount;
  Serial.printf("[CONFIG] Applied %d routers from MQTT config\n", routerCount);
  httpMgr.setRouterStatus(routerCount, 0, 0);  // ステータス更新
}

// ========================================
// MQTT configメッセージ処理（SSOT双方向同期）
// 仕様:
// - schemaVersion巻き戻し禁止
// - secrets（sshUser/password/privateKey）は受け取り・保存・適用OK
// - configHashはサーバ発行をそのまま保存（is10は計算しない）
// - 適用成功後は即時deviceStateReport
// ========================================
void handleConfigMessage(const char* data, int len) {
  Serial.printf("[MQTT] Config received: %d bytes\n", len);

  // JSONパース（is10 config用に大きめのバッファ）
  DynamicJsonDocument doc(4096);
  DeserializationError err = deserializeJson(doc, data, len);
  if (err) {
    Serial.printf("[MQTT] JSON parse error: %s\n", err.c_str());
    lastConfigApplyError = "JSON parse error: " + String(err.c_str());
    settings.setString("is10_config_apply_error", lastConfigApplyError);
    return;
  }

  // schemaVersion巻き戻し禁止
  int schemaVersion = doc["schemaVersion"] | 0;
  if (schemaVersion <= savedSchemaVersion) {
    Serial.printf("[MQTT] Ignoring old/same schema: %d <= %d (rollback prevention)\n",
                  schemaVersion, savedSchemaVersion);
    return;
  }

  // configHash取得（サーバ発行、is10は計算しない）
  String configHash = doc["configHash"] | "";
  String configHashShort = configHash.length() > 8 ? configHash.substring(0, 8) + "..." : configHash;
  Serial.printf("[MQTT] New config: schemaVersion=%d, hash=%s\n", schemaVersion, configHashShort.c_str());

  // config.is10を読む
  JsonObject is10Cfg = doc["config"]["is10"];
  if (is10Cfg.isNull()) {
    Serial.println("[MQTT] No config.is10 in message, ignoring");
    lastConfigApplyError = "No config.is10 in message";
    settings.setString("is10_config_apply_error", lastConfigApplyError);
    return;
  }

  // ========================================
  // Apply処理開始
  // ========================================
  bool applySuccess = true;
  String applyError = "";

  // scanInterval適用（秒）
  if (is10Cfg.containsKey("scanInterval")) {
    int sec = is10Cfg["scanInterval"];
    if (sec >= 60 && sec <= 86400) {
      settings.setInt("is10_scan_interval_sec", sec);
      Serial.printf("[CONFIG] scanInterval=%d sec\n", sec);
    } else {
      Serial.printf("[CONFIG] scanInterval=%d out of range (60-86400), skipped\n", sec);
    }
  }

  // reportClientList適用
  if (is10Cfg.containsKey("reportClientList")) {
    bool reportClients = is10Cfg["reportClientList"];
    settings.setBool("is10_report_clients", reportClients);
    Serial.printf("[CONFIG] reportClientList=%s\n", reportClients ? "true" : "false");
  }

  // routers適用（secrets含む完全置換）
  if (is10Cfg.containsKey("routers")) {
    applyRouterConfig(is10Cfg["routers"].as<JsonArray>());

    // routers.jsonに永続化
    httpMgr.persistRouters();
    Serial.println("[CONFIG] Routers persisted to SPIFFS");
  }

  // ========================================
  // 適用成功時の保存処理
  // ========================================
  if (applySuccess) {
    // NVS保存
    savedSchemaVersion = schemaVersion;
    settings.setInt("is10_config_schema_version", schemaVersion);

    savedConfigHash = configHash;
    settings.setString("is10_config_hash", configHash);

    // ISO8601形式で現在時刻
    lastConfigAppliedAt = ntp.isSynced() ? ntp.getIso8601() : "1970-01-01T00:00:00Z";
    settings.setString("is10_config_applied_at", lastConfigAppliedAt);

    // エラークリア
    lastConfigApplyError = "";
    settings.setString("is10_config_apply_error", "");

    Serial.printf("[CONFIG] Applied successfully: schemaVersion=%d, hash=%s, at=%s\n",
                  schemaVersion, configHashShort.c_str(), lastConfigAppliedAt.c_str());

    // ========================================
    // 即時deviceStateReport送信（SSOT必須）
    // mobesがappliedSchemaVersion/hashで反映確認
    // ========================================
    Serial.println("[CONFIG] Sending immediate deviceStateReport...");
    if (sendDeviceStateReport()) {
      Serial.println("[CONFIG] Immediate stateReport sent OK");
    } else {
      Serial.println("[CONFIG] Immediate stateReport failed (will retry in heartbeat)");
    }
  } else {
    // 適用失敗時（旧設定は保持）
    lastConfigApplyError = applyError;
    settings.setString("is10_config_apply_error", applyError);
    Serial.printf("[CONFIG] Apply failed: %s\n", applyError.c_str());
  }
}

// ========================================
// MQTTメッセージ処理（topic振り分け）
// MqttManagerのonMessageコールバック用
// ========================================
void handleMqttMessage(const String& topic, const char* data, int dataLen) {
  Serial.printf("[MQTT] Topic: %s\n", topic.c_str());

  // configトピック
  if (topic.indexOf("/config") >= 0) {
    handleConfigMessage(data, dataLen);
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

// ========================================
// CelestialGlobe SSOT送信（Universal Ingest）
// 仕様書: observedAtはUnix ms（number型）
// source="ar-is10", devices[], clients[]
// ========================================
bool sendToCelestialGlobe() {
  // エンドポイント取得
  String baseEndpoint = settings.getString("is10_endpoint", "");
  String secret = settings.getString("is10_celestial_secret", "");

  // 設定不備チェック（スキップ理由を明確にログ出力）
  if (baseEndpoint.length() == 0 || secret.length() == 0) {
    Serial.println("[CELESTIAL] SKIPPED - Missing configuration:");
    if (baseEndpoint.length() == 0) {
      Serial.println("[CELESTIAL]   - Endpoint: NOT SET");
    } else {
      Serial.println("[CELESTIAL]   - Endpoint: OK");
    }
    if (secret.length() == 0) {
      Serial.println("[CELESTIAL]   - X-Celestial-Secret: NOT SET");
    } else {
      Serial.println("[CELESTIAL]   - X-Celestial-Secret: OK");
    }
    Serial.println("[CELESTIAL] Configure via HTTP UI: http://<device-ip>/");
    Serial.println("[CELESTIAL] Settings tab > CelestialGlobe section");
    return false;
  }

  // URL構築: endpoint?fid=XXX&source=araneaDevice
  String url = baseEndpoint + "?fid=" + myFid + "&source=araneaDevice";

  // observedAtはUnix ms（number型）
  unsigned long long observedAtMs = 0;
  if (ntp.isSynced()) {
    observedAtMs = (unsigned long long)ntp.getEpoch() * 1000ULL;
  } else {
    observedAtMs = millis();  // フォールバック（不正確だが送信は可能）
  }

  // JSON構築（最大4KB）
  DynamicJsonDocument doc(4096);

  // auth
  JsonObject auth = doc.createNestedObject("auth");
  auth["tid"] = myTid;
  auth["lacisId"] = myLacisId;  // is10のlacisId（observer）
  auth["cic"] = myCic;

  // source（CelestialGlobe用）
  doc["source"] = CELESTIAL_SOURCE;  // "ar-is10"

  // observedAt（Unix ms, number型）
  doc["observedAt"] = observedAtMs;

  // devices配列
  JsonArray devices = doc.createNestedArray("devices");

  for (int i = 0; i < collectedRouterCount; i++) {
    RouterInfo& info = collectedRouterInfo[i];
    if (!info.online) continue;  // オフラインはスキップ

    JsonObject dev = devices.createNestedObject();

    // MACアドレス（大文字12HEX - サーバがlacisId生成に使用）
    // 仕様: ルーターのlacisIdはサーバがMACから生成する
    dev["mac"] = info.routerMac;  // 大文字12HEX正規化済み

    dev["type"] = "Router";
    dev["label"] = "Room" + info.rid;
    dev["status"] = info.online ? "online" : "offline";
    dev["lanIp"] = info.lanIp;
    dev["wanIp"] = info.wanIp;
    dev["ssid24"] = info.ssid24;
    dev["ssid50"] = info.ssid50;
    dev["clientCount"] = info.clientCount;
    // firmware, uptime は取得していないので省略
    // parentMac はis10のMACを入れない（仕様禁止）
  }

  // clients配列（reportClientList=trueの場合のみ）
  // 仕様: 各clientにapMac（接続先ルーターMAC）を含める
  // 例: { "mac": "AABBCCDDEEFF", "apMac": "112233445566", "hostname": "iPhone" }
  bool reportClients = settings.getBool("is10_report_clients", true);
  if (reportClients) {
    JsonArray clients = doc.createNestedArray("clients");
    // TODO: クライアント詳細情報がSSHで取得できた場合に追加
    // DHCPリースからclient MAC取得 → apMac = info.routerMac をセット
    // parentMacはルーターの上流が確実に分かる場合のみ（不明なら空）
  }

  String json;
  serializeJson(doc, json);

  Serial.printf("[CELESTIAL] Sending to %s\n", url.c_str());
  Serial.printf("[CELESTIAL] Payload size: %d bytes, devices: %d\n",
                json.length(), devices.size());

  HTTPClient http;
  http.begin(url);
  http.addHeader("Content-Type", "application/json");
  http.addHeader("X-Celestial-Secret", secret);
  http.setTimeout(15000);

  int httpCode = http.POST(json);
  String response = http.getString();
  http.end();

  bool success = (httpCode >= 200 && httpCode < 300);
  if (success) {
    Serial.printf("[CELESTIAL] OK %d\n", httpCode);
  } else {
    Serial.printf("[CELESTIAL] NG %d: %s\n", httpCode, response.c_str());
  }

  return success;
}

// ========================================
// ルーター情報をエンドポイントへPOST（旧形式 - 廃止予定）
// ========================================
// DISABLED: LibSSH-ESP32 TLS競合テストのため関数全体を無効化
// NOTE: CelestialGlobe SSOT形式に移行予定
#if 0
bool postRouterInfo(const RouterInfo& info) {
  Serial.println("[DEBUG] postRouterInfo() entered");

  if (globalConfig.endpoint.length() == 0) {
    Serial.println("[POST] No endpoint configured");
    return false;
  }

  // メモリ計測（POST前）
  size_t freeHeap = ESP.getFreeHeap();
  size_t largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
  Serial.printf("[MEM] PRE-POST: heap=%u, largest=%u\n", freeHeap, largestBlock);

  String observedAt = ntp.isSynced() ? ntp.getIso8601() : "1970-01-01T00:00:00Z";

  StaticJsonDocument<1024> doc;

  // auth
  JsonObject auth = doc.createNestedObject("auth");
  auth["tid"] = myTid;
  auth["lacisId"] = myLacisId;
  auth["cic"] = myCic;

  // report
  JsonObject report = doc.createNestedObject("report");
  report["observedAt"] = observedAt;
  report["sourceDevice"] = myLacisId;
  report["sourceType"] = DEVICE_TYPE;

  // router data
  JsonObject routerData = report.createNestedObject("router");
  routerData["rid"] = info.rid;
  routerData["lacisId"] = info.lacisId;
  routerData["wanIp"] = info.wanIp;
  routerData["lanIp"] = info.lanIp;
  routerData["ssid24"] = info.ssid24;
  routerData["ssid50"] = info.ssid50;
  routerData["clientCount"] = info.clientCount;
  routerData["online"] = info.online;

  String json;
  serializeJson(doc, json);

  Serial.printf("[POST] Sending to %s\n", globalConfig.endpoint.c_str());

  // SecureClientの初期化（一度だけ）- グローバル再利用でnew/delete削減
  // 注: ESP32 Arduino Core 3.xではsetBufferSizes()なし、デフォルト16KB×2
  if (!secureClientInitialized) {
    secureClient.setInsecure();  // 証明書検証スキップ（組み込みデバイス用）
    secureClientInitialized = true;
    Serial.println("[POST] SecureClient initialized (insecure mode)");
  }

  HTTPClient http;
  http.begin(secureClient, globalConfig.endpoint);  // グローバルSecureClientを再利用
  http.addHeader("Content-Type", "application/json");
  http.setTimeout(15000);

  // メモリ計測（TLS接続直前）
  freeHeap = ESP.getFreeHeap();
  largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
  Serial.printf("[MEM] PRE-TLS: heap=%u, largest=%u\n", freeHeap, largestBlock);

  int httpCode = http.POST(json);
  bool success = (httpCode >= 200 && httpCode < 300);

  if (success) {
    Serial.printf("[POST] OK %d\n", httpCode);
    lastPollResult = "OK";
    successCount++;
  } else {
    Serial.printf("[POST] NG %d\n", httpCode);
    lastPollResult = "NG";
    failCount++;
  }

  http.end();

  // メモリ計測（POST後）
  freeHeap = ESP.getFreeHeap();
  largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
  Serial.printf("[MEM] POST-END: heap=%u, largest=%u\n", freeHeap, largestBlock);

  return success;
}
#endif

// ========================================
// SSHタスク起動（非ブロッキング - ミニマル版アプローチ）
// ========================================
bool startSshTask(int index) {
  if (index < 0 || index >= routerCount) return false;
  if (sshInProgress) return false;  // 既にSSH実行中

  RouterConfig& router = routers[index];
  if (!router.enabled || router.ipAddr.length() == 0) {
    return false;
  }

  // フラグリセット
  sshDone = false;
  sshSuccess = false;
  sshInProgress = true;
  sshTaskStartTime = millis();  // ウォッチドッグ用

  Serial.printf("[POLL] Starting SSH for Router %d/%d\n", index + 1, routerCount);

  // SSH タスクを Core 1 で実行（ミニマル版と同じ）
  BaseType_t xReturned = xTaskCreatePinnedToCore(
    sshTaskFunction,
    "ssh",
    SSH_TASK_STACK_SIZE,
    NULL,  // パラメータ不要（グローバル変数を使用）
    tskIDLE_PRIORITY + 1,  // ミニマル版と同じ優先度
    NULL,
    1  // Core 1（ミニマル版と同じ）
  );

  if (xReturned != pdPASS) {
    Serial.println("[POLL] ERROR: Failed to create SSH task");
    sshInProgress = false;
    return false;
  }

  return true;
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
  String statusLine = "R:" + String(routerCount) + " OK:" + String(successCount) + " NG:" + String(failCount);

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
    sendDeviceStateReport();
  });

  // ========================================
  // deviceStateReport初回送信（必須）
  // mobesのaraneaDeviceConfigは araneaDeviceStates/{lacisId} が
  // ないと "Device not found" で設定投入できない
  // ========================================
  display.showBoot("StateReport...");
  if (sendDeviceStateReport()) {
    initialStateReportSent = true;
    Serial.println("[STATE] Initial state report sent");
  } else {
    Serial.println("[STATE] WARNING: Initial state report failed");
    // 失敗してもloop()で再試行するので続行
  }
  lastHeartbeat = millis();

  // ========================================
  // MQTT接続（双方向デバイス用）
  // ========================================
  mqttWsUrl = araneaReg.getSavedMqttEndpoint();

  // NVSからApplied状態を復元（SSOT双方向同期）
  savedSchemaVersion = settings.getInt("is10_config_schema_version", 0);
  savedConfigHash = settings.getString("is10_config_hash", "");
  lastConfigAppliedAt = settings.getString("is10_config_applied_at", "");
  lastConfigApplyError = settings.getString("is10_config_apply_error", "");

  String hashShort = savedConfigHash.length() > 8 ? savedConfigHash.substring(0, 8) + "..." : savedConfigHash;
  Serial.printf("[CONFIG] Saved Applied state: schema=%d, hash=%s\n", savedSchemaVersion, hashShort.c_str());

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

  // タイミング初期化
  lastRouterPoll = millis();

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
  // SSH初期化 - 新方式: setup()で1回だけlibssh_begin()
  // ========================================
  // libssh_begin() をsetup()で1回だけ呼び出す
  // ssh_finalize() は呼び出さない（グローバル状態破壊を防ぐ）
  lastStage = STAGE_SSH_BEGIN;
  Serial.printf("[STAGE] %s\n", stageStr(lastStage));
  libssh_begin();
  Serial.println("[SSH] libssh_begin() called once in setup()");

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
  // 4. ルーターポーリング（非ブロッキング - ミニマル版アプローチ）
  // ========================================
  // ポーリングステータスを更新
  httpMgr.setPollingStatus(currentRouterIndex, sshInProgress);

  if (!apModeActive && routerCount > 0 && httpMgr.isPollingEnabled()) {
    // SSHタスクウォッチドッグ: 長時間ハングしたら強制スキップ
    if (sshInProgress && !sshDone && (now - sshTaskStartTime >= SSH_TASK_WATCHDOG_MS)) {
      Serial.printf("[WATCHDOG] SSH task timeout after %lu ms, forcing skip\n",
                    now - sshTaskStartTime);
      Serial.printf("[WATCHDOG] Router %d/%d marked as FAILED (watchdog)\n",
                    currentRouterIndex + 1, routerCount);
      // 強制的に失敗として処理
      sshInProgress = false;
      sshDone = true;
      sshSuccess = false;
      failCount++;
      // 次のルーターへ進む
      currentRouterIndex++;
      if (currentRouterIndex >= routerCount) {
        Serial.printf("\n[COMPLETE] %d/%d success (watchdog triggered)\n", successCount, routerCount);
        httpMgr.setRouterStatus(routerCount, successCount, millis());  // ステータス更新
        collectedRouterCount = routerCount;
        sendToCelestialGlobe();
        currentRouterIndex = 0;
        successCount = 0;
        failCount = 0;
        lastRouterPoll = now;
      }
      sshDone = false;  // フラグリセット
    }

    // SSH完了チェック
    if (sshDone && sshInProgress) {
      sshInProgress = false;

      // ヒープ状況を毎回ログ出力（デバッグ用）
      uint32_t freeHeap = ESP.getFreeHeap();
      uint32_t largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);

      if (sshSuccess) {
        Serial.printf("[POLL] Router %d/%d SUCCESS (heap=%u, largest=%u)\n",
                      currentRouterIndex + 1, routerCount, freeHeap, largestBlock);
        successCount++;
      } else {
        Serial.printf("[POLL] Router %d/%d FAILED (heap=%u, largest=%u)\n",
                      currentRouterIndex + 1, routerCount, freeHeap, largestBlock);
        failCount++;
      }

      // ヒープ危険水準警告（SSH_TASK_STACK_SIZE + マージン）
      if (largestBlock < SSH_TASK_STACK_SIZE + 10000) {
        Serial.printf("[HEAP] WARNING: Low memory! largest=%u < required=%u\n",
                      largestBlock, SSH_TASK_STACK_SIZE + 10000);
      }

      sshDone = false;

      // 次のルーターへ（10秒待機 - メモリ回復＋MQTT/HTTP余裕）
      delay(10000);
      currentRouterIndex++;
      if (currentRouterIndex >= routerCount) {
        // 全ルーター完了
        uint32_t freeHeap = ESP.getFreeHeap();
        uint32_t largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
        Serial.printf("\n[COMPLETE] %d/%d success\n", successCount, routerCount);
        Serial.printf("[COMPLETE] heap=%u, largest=%u\n\n", freeHeap, largestBlock);
        httpMgr.setRouterStatus(routerCount, successCount, millis());  // ステータス更新

        // CelestialGlobe送信前にメモリ回復待機
        Serial.println("[COMPLETE] Waiting 3s for memory recovery before CelestialGlobe...");
        delay(3000);

        // 再チェック
        freeHeap = ESP.getFreeHeap();
        largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
        Serial.printf("[COMPLETE] After wait: heap=%u, largest=%u\n", freeHeap, largestBlock);

        // CelestialGlobe SSOT送信（ヒープ16KB以上必要）
        collectedRouterCount = routerCount;
        if (largestBlock < 16000) {
          Serial.printf("[COMPLETE] SKIP CelestialGlobe - low memory (largest=%u < 16000)\n", largestBlock);
        } else if (sendToCelestialGlobe()) {
          Serial.println("[COMPLETE] CelestialGlobe sent OK");
        } else {
          Serial.println("[COMPLETE] CelestialGlobe send failed");
        }

        currentRouterIndex = 0;
        successCount = 0;
        failCount = 0;
        lastRouterPoll = now;  // インターバルリセット
      }
    }
    // 新しいSSH開始
    else if (!sshInProgress && (currentRouterIndex == 0 || (now - lastRouterPoll >= ROUTER_POLL_INTERVAL_MS))) {
      // SSH開始前にヒープチェック（クラッシュ防止）
      uint32_t largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
      uint32_t requiredMemory = SSH_TASK_STACK_SIZE + 20000;  // スタック + libsshバッファ

      if (largestBlock < requiredMemory) {
        Serial.printf("[HEAP] CRITICAL: Skipping Router %d/%d - insufficient memory\n",
                      currentRouterIndex + 1, routerCount);
        Serial.printf("[HEAP] largest=%u < required=%u\n", largestBlock, requiredMemory);
        Serial.printf("[HEAP] Aborting cycle, %d/%d completed\n", successCount, routerCount);

        // サイクル中断・リセット
        currentRouterIndex = 0;
        successCount = 0;
        failCount = 0;
        lastRouterPoll = now;

        // 次サイクルまで待機させるため長めのインターバル
        Serial.println("[HEAP] Waiting 60s for memory recovery...");
      } else {
        startSshTask(currentRouterIndex);
        if (currentRouterIndex == 0) {
          lastRouterPoll = now;
        }
      }
    }
  }

  // ========================================
  // 4.5 deviceStateReport heartbeat（60秒毎）
  // ========================================
  if (!apModeActive && (now - lastHeartbeat >= HEARTBEAT_INTERVAL_MS)) {
    // 初回が失敗していた場合も再試行
    if (!initialStateReportSent) {
      if (sendDeviceStateReport()) {
        initialStateReportSent = true;
        Serial.println("[STATE] Retry state report sent");
      }
    } else {
      sendDeviceStateReport();
    }
    lastHeartbeat = now;
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
