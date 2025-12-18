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
#include <HTTPClient.h>
#include <esp_mac.h>
#include <SPIFFS.h>
#include <ArduinoJson.h>

// AraneaGlobal共通モジュール
#include "AraneaSettings.h"
#include "SettingManager.h"
#include "DisplayManager.h"
#include "WiFiManager.h"
#include "NtpManager.h"
#include "LacisIDGenerator.h"
#include "AraneaRegister.h"
#include "HttpManagerIs10.h"
#include "OtaManager.h"
#include "HttpOtaManager.h"
#include "Operator.h"
#include "SshClient.h"

// ========================================
// デバイス定数
// ========================================
static const char* PRODUCT_TYPE = "010";
static const char* PRODUCT_CODE = "0001";
static const char* DEVICE_TYPE = "ar-is10";
static const char* DEVICE_MODEL = "Openwrt_RouterInspector";
static const char* FIRMWARE_VERSION = "1.0.0";

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
static const unsigned long SSH_TIMEOUT_MS = 30000;            // SSHタイムアウト
static const int SSH_RETRY_COUNT = 2;                         // リトライ回数
static const unsigned long ROUTER_INTERVAL_MS = 30000;        // 次ルーターまでのインターバル
static const unsigned long DISPLAY_UPDATE_MS = 1000;
static const unsigned long BUTTON_HOLD_MS = 3000;
static const unsigned long AP_MODE_TIMEOUT_MS = 300000;       // APモード5分タイムアウト

// 最大ルーター数（HttpManagerIs10.hで定義）
// #define MAX_ROUTERS 20

// ========================================
// ルーター設定構造体
// ========================================
struct RouterConfig {
  String rid;           // Resource ID
  String ipAddr;        // IPアドレス
  String publicKey;     // SSH公開鍵（認証用）
  int sshPort;          // SSHポート
  String username;      // ユーザー名
  String password;      // パスワード
  bool enabled;         // 有効フラグ
};

// ========================================
// ルーター情報構造体（取得データ）
// ========================================
struct RouterInfo {
  String rid;
  String lacisId;       // ルーターのLacisID（MACから生成）
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

// ========================================
// 前方宣言
// ========================================
void loadIs10Config();
void saveIs10Config();
void startAPMode();
void stopAPMode();
bool pollRouter(int index);
bool executeSSHCommands(const RouterConfig& router, RouterInfo& info);
bool postRouterInfo(const RouterInfo& info);
String generateRouterLacisId(const String& mac);

// ========================================
// APモードSSID生成
// ========================================
String getAPModeSSID() {
  String mac6 = myMac.substring(6);  // 下6桁
  return String(DEVICE_TYPE) + "-" + mac6;
}

// ========================================
// ホスト名生成
// ========================================
String getHostname() {
  String mac6 = myMac.substring(6);
  return String(DEVICE_TYPE) + "-" + mac6;
}

// ========================================
// IS10設定読み込み
// ========================================
void loadIs10Config() {
  // グローバル設定
  globalConfig.endpoint = settings.getString("is10_endpoint", AraneaSettings::getCloudUrl());
  globalConfig.sshTimeout = settings.getInt("is10_ssh_timeout", SSH_TIMEOUT_MS);
  globalConfig.retryCount = settings.getInt("is10_retry_count", SSH_RETRY_COUNT);
  globalConfig.routerInterval = settings.getInt("is10_router_interval", ROUTER_INTERVAL_MS);
  globalConfig.lacisIdPrefix = settings.getString("is10_lacis_prefix", "4");
  globalConfig.routerProductType = settings.getString("is10_router_ptype", "");
  globalConfig.routerProductCode = settings.getString("is10_router_pcode", "");

  // ルーター設定をSPIFFSから読み込み
  if (SPIFFS.exists("/is10_routers.json")) {
    File file = SPIFFS.open("/is10_routers.json", "r");
    if (file) {
      String json = file.readString();
      file.close();

      DynamicJsonDocument doc(8192);
      DeserializationError error = deserializeJson(doc, json);
      if (!error) {
        JsonArray arr = doc.as<JsonArray>();
        routerCount = 0;
        for (JsonObject obj : arr) {
          if (routerCount >= MAX_ROUTERS) break;
          routers[routerCount].rid = obj["rid"] | "";
          routers[routerCount].ipAddr = obj["ipAddr"] | "";
          routers[routerCount].publicKey = obj["publicKey"] | "";
          routers[routerCount].sshPort = obj["sshPort"] | 22;
          routers[routerCount].username = obj["username"] | "";
          routers[routerCount].password = obj["password"] | "";
          routers[routerCount].enabled = obj["enabled"] | true;
          if (routers[routerCount].ipAddr.length() > 0) {
            routerCount++;
          }
        }
        Serial.printf("[CONFIG] Loaded %d routers\n", routerCount);
      }
    }
  }

  Serial.printf("[CONFIG] endpoint: %s\n", globalConfig.endpoint.c_str());
  Serial.printf("[CONFIG] sshTimeout: %lu\n", globalConfig.sshTimeout);
  Serial.printf("[CONFIG] routerCount: %d\n", routerCount);
}

// ========================================
// IS10設定保存
// ========================================
void saveIs10Config() {
  // グローバル設定
  settings.setString("is10_endpoint", globalConfig.endpoint);
  settings.setInt("is10_ssh_timeout", globalConfig.sshTimeout);
  settings.setInt("is10_retry_count", globalConfig.retryCount);
  settings.setInt("is10_router_interval", globalConfig.routerInterval);
  settings.setString("is10_lacis_prefix", globalConfig.lacisIdPrefix);
  settings.setString("is10_router_ptype", globalConfig.routerProductType);
  settings.setString("is10_router_pcode", globalConfig.routerProductCode);

  // ルーター設定をSPIFFSに保存
  DynamicJsonDocument doc(8192);
  JsonArray arr = doc.to<JsonArray>();
  for (int i = 0; i < routerCount; i++) {
    JsonObject obj = arr.createNestedObject();
    obj["rid"] = routers[i].rid;
    obj["ipAddr"] = routers[i].ipAddr;
    obj["publicKey"] = routers[i].publicKey;
    obj["sshPort"] = routers[i].sshPort;
    obj["username"] = routers[i].username;
    obj["password"] = routers[i].password;
    obj["enabled"] = routers[i].enabled;
  }

  File file = SPIFFS.open("/is10_routers.json", "w");
  if (file) {
    serializeJson(doc, file);
    file.close();
    Serial.println("[CONFIG] Saved router config");
  }
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
  Serial.printf("[SSH] Connecting to %s:%d as %s\n",
    router.ipAddr.c_str(), router.sshPort, router.username.c_str());

  // 初期化
  info.rid = router.rid;
  info.wanIp = "";
  info.lanIp = "";
  info.ssid24 = "";
  info.ssid50 = "";
  info.clientCount = 0;
  info.online = false;
  info.lastUpdate = millis();

  // SSH接続
  SshClient ssh;
  SshConfig config;
  config.host = router.ipAddr;
  config.port = router.sshPort;
  config.username = router.username;
  config.password = router.password;
  config.timeout = globalConfig.sshTimeout;

  SshResult result = ssh.connect(config);
  if (result != SshResult::OK) {
    Serial.printf("[SSH] Connection failed: %s\n", ssh.getLastError().c_str());
    return false;
  }

  info.online = true;

  // WAN IP取得（eth0またはwan）
  SshExecResult execResult = ssh.exec("ifconfig eth0 2>/dev/null || ifconfig wan 2>/dev/null");
  if (execResult.result == SshResult::OK && execResult.output.length() > 0) {
    info.wanIp = parseIpFromIfconfig(execResult.output);
    Serial.printf("[SSH] WAN IP: %s\n", info.wanIp.c_str());
  }

  // LAN IP取得（br-lanまたはlan）
  execResult = ssh.exec("ifconfig br-lan 2>/dev/null || ifconfig lan 2>/dev/null");
  if (execResult.result == SshResult::OK && execResult.output.length() > 0) {
    info.lanIp = parseIpFromIfconfig(execResult.output);
    Serial.printf("[SSH] LAN IP: %s\n", info.lanIp.c_str());
  }

  // SSID取得
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
    Serial.printf("[SSH] SSID 2.4G: %s, 5G: %s\n", info.ssid24.c_str(), info.ssid50.c_str());
  }

  // DHCPリース（接続クライアント数）
  execResult = ssh.exec("cat /tmp/dhcp.leases 2>/dev/null");
  if (execResult.result == SshResult::OK && execResult.output.length() > 0) {
    info.clientCount = parseClientCount(execResult.output);
    Serial.printf("[SSH] Client count: %d\n", info.clientCount);
  }

  // MACアドレス取得（LacisID生成用）
  execResult = ssh.exec("cat /sys/class/net/br-lan/address 2>/dev/null || cat /sys/class/net/eth0/address 2>/dev/null");
  if (execResult.result == SshResult::OK && execResult.output.length() > 0) {
    String mac = execResult.output;
    mac.trim();
    info.lacisId = generateRouterLacisId(mac);
    Serial.printf("[SSH] Router MAC: %s, LacisID: %s\n", mac.c_str(), info.lacisId.c_str());
  }

  ssh.disconnect();
  Serial.println("[SSH] Data collection complete");

  return true;
}

// ========================================
// ルーター情報をエンドポイントへPOST
// ========================================
bool postRouterInfo(const RouterInfo& info) {
  if (globalConfig.endpoint.length() == 0) {
    Serial.println("[POST] No endpoint configured");
    return false;
  }

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
  Serial.println(json);

  HTTPClient http;
  http.begin(globalConfig.endpoint);
  http.addHeader("Content-Type", "application/json");
  http.setTimeout(10000);

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
  return success;
}

// ========================================
// 単一ルーターのポーリング
// ========================================
bool pollRouter(int index) {
  if (index < 0 || index >= routerCount) return false;

  RouterConfig& router = routers[index];
  if (!router.enabled || router.ipAddr.length() == 0) {
    return false;
  }

  Serial.printf("[POLL] Router %d: %s (%s)\n", index, router.rid.c_str(), router.ipAddr.c_str());

  RouterInfo info;

  // SSH経由で情報取得
  bool sshOk = executeSSHCommands(router, info);

  if (sshOk) {
    // LacisID生成（MACが取得できた場合）
    // info.lacisId = generateRouterLacisId(mac);

    // エンドポイントへPOST
    postRouterInfo(info);
  }

  return sshOk;
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
      SPIFFS.remove("/is10_routers.json");
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
  Serial.begin(115200);
  delay(100);
  Serial.println("\n[BOOT] is10 (ar-is10) starting...");
  Serial.printf("[BOOT] Model: %s, Version: %s\n", DEVICE_MODEL, FIRMWARE_VERSION);

  // GPIO初期化
  initGPIO();

  // Operator初期化
  op.setMode(OperatorMode::PROVISION);

  // SPIFFS初期化
  if (!SPIFFS.begin(true)) {
    Serial.println("[ERROR] SPIFFS init failed");
  }

  // AraneaSettings初期化（SPIFFS設定ロード）
  AraneaSettings::init();

  // SettingManager初期化（NVS）
  if (!settings.begin("isms")) {
    Serial.println("[ERROR] Settings init failed");
    return;
  }

  // デバイス固有のデフォルト設定を投入
  AraneaSettings::initDefaults(settings);

  // DisplayManager初期化
  if (!display.begin()) {
    Serial.println("[WARNING] Display init failed");
  }
  display.showBoot("Booting...");

  // LacisID生成（WiFi接続前にMACを取得）
  myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
  myMac = lacisGen.getStaMac12Hex();
  myHostname = getHostname();
  Serial.printf("[LACIS] ID: %s, MAC: %s\n", myLacisId.c_str(), myMac.c_str());
  Serial.printf("[HOST] Hostname: %s\n", myHostname.c_str());

  // WiFi接続試行
  display.showConnecting("WiFi...", 0);
  if (!wifi.connectWithSettings(&settings)) {
    Serial.println("[WIFI] Connection failed, starting AP mode");
    startAPMode();
  } else {
    Serial.printf("[WIFI] Connected: %s\n", wifi.getIP().c_str());

    // NTP同期
    display.showBoot("NTP sync...");
    if (!ntp.sync()) {
      Serial.println("[WARNING] NTP sync failed");
    }

    // AraneaGateway登録
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
      myTid, DEVICE_TYPE, myLacisId, myMac, PRODUCT_TYPE, PRODUCT_CODE
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
  loadIs10Config();

  // OtaManager初期化
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

  // HttpManager開始
  httpMgr.begin(&settings, 80);
  httpMgr.setDeviceInfo(DEVICE_TYPE, myLacisId, myCic, FIRMWARE_VERSION);

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

  // RUNモードへ
  op.setMode(OperatorMode::RUN);

  // タイミング初期化
  lastRouterPoll = millis();

  Serial.println("[BOOT] is10 ready!");
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

  // APモードタイムアウトチェック
  if (apModeActive && (now - apModeStartTime >= AP_MODE_TIMEOUT_MS)) {
    Serial.println("[AP] Timeout, attempting STA reconnect");
    stopAPMode();
    wifi.connectWithSettings(&settings);
    if (!wifi.isConnected()) {
      startAPMode();  // 再度APモードへ
    }
  }

  // ルーターポーリング（STAモード時のみ）
  if (!apModeActive && routerCount > 0 && (now - lastRouterPoll >= ROUTER_POLL_INTERVAL_MS)) {
    pollRouter(currentRouterIndex);

    currentRouterIndex++;
    if (currentRouterIndex >= routerCount) {
      currentRouterIndex = 0;
    }

    lastRouterPoll = now;
  }

  // ディスプレイ更新
  if (now - lastDisplayUpdate >= DISPLAY_UPDATE_MS) {
    updateDisplay();
    lastDisplayUpdate = now;
  }

  // ボタン処理
  handleButtons();

  // WiFi再接続チェック（STAモード時）
  if (!apModeActive && !wifi.isConnected()) {
    Serial.println("[WIFI] Disconnected, reconnecting...");
    display.showConnecting("Reconnect...", 0);
    if (!wifi.connectWithSettings(&settings)) {
      startAPMode();
    }
  }

  delay(10);
}
