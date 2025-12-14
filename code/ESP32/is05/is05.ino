/**
 * is05 - 8ch Reed Switch Sensor Device
 *
 * 8チャンネルリードスイッチ入力。状態変化を検出してis03へHTTP送信。
 * Web UIで設定変更、OTA更新対応。
 *
 * 動作シーケンス:
 * 1. WiFi接続
 * 2. AraneaGateway登録（CIC取得）
 * 3. GPIO入力監視（デバウンス付き）
 * 4. 状態変化時 → is03へHTTP POST
 * 5. 60秒毎の心拍送信
 */

#include <Arduino.h>
#include <Wire.h>
#include <WiFi.h>
#include <HTTPClient.h>
#include <esp_mac.h>

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
// 定数
// ========================================
static const char* PRODUCT_TYPE = "005";
static const char* PRODUCT_CODE = "0096";
static const char* DEVICE_TYPE = "ISMS_ar-is05";
static const char* FIRMWARE_VERSION = "1.0.0";

// 注意: GPIO5はis05ではリードスイッチ入力(ch3)に使用
// I2C電源制御はis01/is02のみで使用

// ボタン
static const int GPIO_BTN_WIFI = 25;     // WiFi再接続ボタン
static const int GPIO_BTN_RESET = 26;    // ファクトリーリセットボタン

// タイミング
static const unsigned long SAMPLE_INTERVAL_MS = 20;     // デバウンスサンプル間隔
static const int STABLE_COUNT = 5;                       // 安定カウント（5回一致で確定）
static const unsigned long BOOT_GRACE_MS = 5000;         // 起動後の安定化期間（5秒）
static const unsigned long HEARTBEAT_INTERVAL_MS = 60000; // 心拍送信間隔（60秒）
static const unsigned long DISPLAY_UPDATE_MS = 1000;     // ディスプレイ更新間隔
static const unsigned long BUTTON_HOLD_MS = 3000;        // ボタン長押し時間

// チャンネル数
static const int NUM_CHANNELS = 8;

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

// 送信先URL
String relayPriUrl;
String relaySecUrl;
String cloudUrl;

// 自機情報
String myLacisId;
String myMac;
String myCic;

// チャンネル設定
int chPins[NUM_CHANNELS];
String chNames[NUM_CHANNELS];
String chMeanings[NUM_CHANNELS];  // "open" or "close"
String chDids[NUM_CHANNELS];

// 入力状態
int rawState[NUM_CHANNELS];       // 生の読み取り値
int stableState[NUM_CHANNELS];    // 確定状態
int stableCount_[NUM_CHANNELS];   // 安定カウンタ
String lastUpdatedAt[NUM_CHANNELS]; // 最終更新時刻

// タイミング
unsigned long lastSampleTime = 0;
unsigned long lastHeartbeatTime = 0;
unsigned long lastDisplayUpdate = 0;
unsigned long btnWifiPressTime = 0;
unsigned long btnResetPressTime = 0;
unsigned long lastSendTime = 0;
unsigned long bootTime = 0;  // 起動時刻
static const unsigned long MIN_SEND_INTERVAL_MS = 1000;  // 最小送信間隔1秒

// 状態
int lastChangedChannel = -1;
String lastSendResult = "---";
int heartbeatCount = 0;
bool needSend = false;

// ティッカー表示用（状態変化のあったチャンネルのみ表示）
static const unsigned long TICKER_DISPLAY_MS = 5000;   // 状態変化後5秒間表示
static const unsigned long TICKER_CYCLE_MS = 1000;     // 複数変化時の切り替え間隔
unsigned long chChangedAt[NUM_CHANNELS];               // 各チャンネルの変化時刻
int tickerCycleIndex = 0;
unsigned long lastTickerCycle = 0;

// ========================================
// チャンネル設定読み込み
// ========================================
void loadChannelConfig() {
  for (int i = 0; i < NUM_CHANNELS; i++) {
    int chNum = i + 1;
    chPins[i] = settings.getInt("is05_ch" + String(chNum) + "_pin", 0);
    chNames[i] = settings.getString("is05_ch" + String(chNum) + "_name", "ch" + String(chNum));
    chMeanings[i] = settings.getString("is05_ch" + String(chNum) + "_meaning", "open");
    chDids[i] = settings.getString("is05_ch" + String(chNum) + "_did", "00000000");

    Serial.printf("[CONFIG] ch%d: pin=%d, name=%s, meaning=%s, did=%s\n",
      chNum, chPins[i], chNames[i].c_str(), chMeanings[i].c_str(), chDids[i].c_str());
  }
}

// ========================================
// GPIO初期化
// ========================================
void initGPIO() {
  // 入力ピン設定（INPUT_PULLUP: activeLow前提）
  for (int i = 0; i < NUM_CHANNELS; i++) {
    if (chPins[i] > 0) {
      pinMode(chPins[i], INPUT_PULLUP);
      delay(10);  // ピン安定化待ち
      rawState[i] = digitalRead(chPins[i]);
      stableState[i] = rawState[i];
      stableCount_[i] = STABLE_COUNT;  // 初期状態は安定済み
      lastUpdatedAt[i] = ntp.isSynced() ? ntp.getIso8601() : "1970-01-01T00:00:00Z";
      chChangedAt[i] = 0;  // 初期状態は変化なし
      Serial.printf("[GPIO] ch%d: pin=%d, initial=%s\n",
        i + 1, chPins[i], rawState[i] == LOW ? "LOW(active)" : "HIGH(inactive)");
    } else {
      rawState[i] = HIGH;
      stableState[i] = HIGH;
      stableCount_[i] = STABLE_COUNT;
      chChangedAt[i] = 0;
    }
  }

  // ボタンピン
  pinMode(GPIO_BTN_WIFI, INPUT_PULLUP);
  pinMode(GPIO_BTN_RESET, INPUT_PULLUP);

  Serial.println("[GPIO] Input pins initialized");
}

// ========================================
// 状態文字列取得（meaning適用）
// ========================================
String getStateString(int ch) {
  // activeLow: LOW=active, HIGH=inactive
  bool isActive = (stableState[ch] == LOW);

  if (isActive) {
    // アクティブ時はmeaningをそのまま返す
    return chMeanings[ch];
  } else {
    // 非アクティブ時はmeaningの逆を返す
    return (chMeanings[ch] == "open") ? "close" : "open";
  }
}

// ========================================
// 送信JSON生成
// ========================================
String buildEventJson() {
  String observedAt = ntp.isSynced() ? ntp.getIso8601() : "1970-01-01T00:00:00Z";
  String ip = wifi.getIP();
  int rssi = WiFi.RSSI();
  String ssid = WiFi.SSID();

  String json = "{";

  // observedAt（トップレベル）
  json += "\"observedAt\":\"" + observedAt + "\",";

  // sensor
  json += "\"sensor\":{";
  json += "\"lacisId\":\"" + myLacisId + "\",";
  json += "\"mac\":\"" + myMac + "\",";
  json += "\"productType\":\"" + String(PRODUCT_TYPE) + "\",";
  json += "\"productCode\":\"" + String(PRODUCT_CODE) + "\"";
  json += "},";

  // state
  json += "\"state\":{";
  for (int i = 0; i < NUM_CHANNELS; i++) {
    String chKey = "ch" + String(i + 1);
    json += "\"" + chKey + "\":\"" + getStateString(i) + "\",";
    json += "\"" + chKey + "_lastUpdatedAt\":\"" + lastUpdatedAt[i] + "\",";
    json += "\"" + chKey + "_did\":\"" + chDids[i] + "\",";
    json += "\"" + chKey + "_name\":\"" + chNames[i] + "\",";
  }
  json += "\"rssi\":\"" + String(rssi) + "\",";
  json += "\"ipaddr\":\"" + ip + "\",";
  json += "\"SSID\":\"" + ssid + "\"";
  json += "},";

  // meta
  json += "\"meta\":{";
  json += "\"observedAt\":\"" + observedAt + "\",";
  json += "\"direct\":true";
  json += "},";

  // gateway（is03互換）
  json += "\"gateway\":{";
  json += "\"lacisId\":\"" + myLacisId + "\",";
  json += "\"ip\":\"" + ip + "\",";
  json += "\"rssi\":" + String(rssi);
  json += "}";

  json += "}";
  return json;
}

// ========================================
// HTTP POST送信（単一URL）
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
  } else {
    Serial.printf("[HTTP] NG %d -> %s\n", httpCode, url.c_str());
  }

  http.end();
  return success;
}

// ========================================
// HTTP送信（ローカル二重 + クラウド）
// ========================================
void sendToRelay() {
  // 最小送信間隔チェック（連続送信防止）
  unsigned long now = millis();
  if (lastSendTime > 0 && (now - lastSendTime) < MIN_SEND_INTERVAL_MS) {
    return;  // 間隔が短すぎる場合はスキップ
  }
  lastSendTime = now;

  String json = buildEventJson();
  Serial.println("[SEND] Sending to all endpoints...");

  int successCount = 0;

  // ローカルリレー Primary（常に送信）
  if (relayPriUrl.length() > 0) {
    if (postToUrl(relayPriUrl, json)) successCount++;
  }

  // ローカルリレー Secondary（冗長性のため常に送信）
  if (relaySecUrl.length() > 0) {
    if (postToUrl(relaySecUrl, json)) successCount++;
  }

  // クラウド送信
  if (cloudUrl.length() > 0) {
    if (postToUrl(cloudUrl, json)) successCount++;
  }

  // 結果表示
  if (successCount > 0) {
    lastSendResult = "OK(" + String(successCount) + ")";
  } else {
    lastSendResult = "NG";
  }

  Serial.printf("[SEND] Done: %d success\n", successCount);
}

// ========================================
// 入力読み取り（デバウンス）
// ========================================
void sampleInputs() {
  unsigned long now = millis();
  bool inBootGrace = (bootTime > 0 && (now - bootTime) < BOOT_GRACE_MS);

  for (int i = 0; i < NUM_CHANNELS; i++) {
    if (chPins[i] <= 0) continue;

    int current = digitalRead(chPins[i]);

    if (current == rawState[i]) {
      // 同じ値が続いている
      if (stableCount_[i] < STABLE_COUNT * 2) {  // オーバーフロー防止
        stableCount_[i]++;
      }
      if (stableCount_[i] >= STABLE_COUNT && current != stableState[i]) {
        // 確定状態が変化した
        stableState[i] = current;
        lastUpdatedAt[i] = ntp.isSynced() ? ntp.getIso8601() : "1970-01-01T00:00:00Z";
        chChangedAt[i] = now;  // 変化時刻を記録（ティッカー表示用）
        lastChangedChannel = i;

        Serial.printf("[INPUT] ch%d changed to %s (pin=%d, raw=%d)\n",
          i + 1, getStateString(i).c_str(), chPins[i], current);

        // 起動猶予期間中は送信しない（初期ノイズ対策）
        if (!inBootGrace) {
          needSend = true;
        } else {
          Serial.println("[INPUT] Boot grace period - skipping send");
        }
      }
    } else {
      // 値が変わった、カウンタリセット
      rawState[i] = current;
      stableCount_[i] = 0;
    }
  }
}

// ========================================
// ボタン処理
// ========================================
void handleButtons() {
  // WiFi再接続ボタン
  if (digitalRead(GPIO_BTN_WIFI) == LOW) {
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
  if (digitalRead(GPIO_BTN_RESET) == LOW) {
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
// 電波レベルインジケータ生成
// ========================================
String getSignalIndicator() {
  int rssi = WiFi.RSSI();
  String indicator = "@ ";  // アンテナマーク

  // RSSIに応じたバー表示
  if (rssi > -50) {
    indicator += "[||||]";      // 強い
  } else if (rssi > -60) {
    indicator += "[||| ]";      // 良好
  } else if (rssi > -70) {
    indicator += "[||  ]";      // 中程度
  } else if (rssi > -80) {
    indicator += "[|   ]";      // 弱い
  } else {
    indicator += "[    ]";      // 非常に弱い
  }

  indicator += " " + String(rssi) + "dBm";
  return indicator;
}

// ========================================
// ティッカー文字列生成（状態変化のあったchのみ5秒間表示）
// ========================================
String getTickerString() {
  unsigned long now = millis();

  // 最近5秒以内に変化したチャンネルを収集
  int recentChannels[NUM_CHANNELS];
  int recentCount = 0;

  for (int i = 0; i < NUM_CHANNELS; i++) {
    if (chChangedAt[i] > 0 && (now - chChangedAt[i]) < TICKER_DISPLAY_MS) {
      recentChannels[recentCount++] = i;
    }
  }

  // 変化なし → 電波レベルインジケータを表示
  if (recentCount == 0) {
    return getSignalIndicator();
  }

  // 複数変化時は1秒ごとに切り替え
  if (now - lastTickerCycle >= TICKER_CYCLE_MS) {
    tickerCycleIndex = (tickerCycleIndex + 1) % recentCount;
    lastTickerCycle = now;
  }

  // インデックスが範囲外にならないよう調整
  if (tickerCycleIndex >= recentCount) {
    tickerCycleIndex = 0;
  }

  int ch = recentChannels[tickerCycleIndex];
  return chNames[ch] + "=" + getStateString(ch);
}

// ========================================
// ディスプレイ更新（is02パターン）
// ========================================
void updateDisplay() {
  // Line1: IP + RSSI
  String line1 = wifi.getIP() + " " + String(WiFi.RSSI()) + "dBm";

  // CIC（大きいフォントで表示）
  String cicStr = myCic.length() > 0 ? myCic : "------";

  // sensorLine: ティッカー表示（チャンネル名=状態）
  String sensorLine = getTickerString();

  // showIs02Mainを使用（is01/02と同じレイアウト）
  display.showIs02Main(line1, cicStr, sensorLine, false);
}

// ========================================
// Setup
// ========================================
void setup() {
  Serial.begin(115200);
  delay(100);
  Serial.println("\n[BOOT] is05 starting...");

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
    Serial.println("[WARNING] NTP sync failed, using default time");
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

  String tid = settings.getString("tid", "");
  AraneaRegisterResult regResult = araneaReg.registerDevice(
    tid, DEVICE_TYPE, myLacisId, myMac, PRODUCT_TYPE, PRODUCT_CODE
  );

  if (regResult.ok) {
    myCic = regResult.cic_code;
    settings.setString("cic", myCic);
    Serial.printf("[REG] CIC: %s\n", myCic.c_str());
  } else {
    myCic = settings.getString("cic", "");
    Serial.printf("[REG] Failed, using saved CIC: %s\n", myCic.c_str());
  }

  // 7. 送信先URL読み込み（ローカル二重 + クラウド）
  relayPriUrl = settings.getString("relay_pri", "http://192.168.96.201:8080/api/events");
  relaySecUrl = settings.getString("relay_sec", "http://192.168.96.202:8080/api/events");
  cloudUrl = settings.getString("cloud_url", "https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport");
  Serial.printf("[SEND] Local1: %s\n", relayPriUrl.c_str());
  Serial.printf("[SEND] Local2: %s\n", relaySecUrl.c_str());
  Serial.printf("[SEND] Cloud:  %s\n", cloudUrl.length() > 0 ? cloudUrl.c_str() : "(none)");

  // 8. OtaManager初期化
  String hostname = "is05-" + myMac.substring(8);
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

  // 9. HttpManager開始
  httpMgr.begin(&settings, 80);
  httpMgr.setDeviceInfo(DEVICE_TYPE, myLacisId, myCic, FIRMWARE_VERSION);

  // 10. チャンネル設定読み込み
  loadChannelConfig();

  // 11. GPIO入力初期化
  initGPIO();

  // 12. RUNモードへ
  op.setMode(OperatorMode::RUN);

  // 13. 起動時刻を記録（起動猶予期間の基準）
  bootTime = millis();
  lastHeartbeatTime = bootTime;
  lastSendTime = bootTime;

  Serial.println("[BOOT] is05 ready!");
  Serial.printf("[BOOT] Boot grace period: %lu ms\n", BOOT_GRACE_MS);

  // 初期状態送信（猶予期間後に最初の心拍で送信される）
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

  // 入力サンプリング（20ms間隔）
  if (now - lastSampleTime >= SAMPLE_INTERVAL_MS) {
    sampleInputs();
    lastSampleTime = now;
  }

  // 状態変化時の送信
  if (needSend) {
    sendToRelay();
    needSend = false;
  }

  // 心拍送信（60秒間隔）
  if (now - lastHeartbeatTime >= HEARTBEAT_INTERVAL_MS) {
    heartbeatCount++;
    Serial.printf("[HEARTBEAT] #%d\n", heartbeatCount);
    sendToRelay();
    lastHeartbeatTime = now;
  }

  // ディスプレイ更新（1秒間隔）
  if (now - lastDisplayUpdate >= DISPLAY_UPDATE_MS) {
    updateDisplay();
    lastDisplayUpdate = now;
  }

  // ボタン処理
  handleButtons();

  // WiFi再接続チェック
  if (!wifi.isConnected()) {
    Serial.println("[WIFI] Disconnected, reconnecting...");
    display.showConnecting("Reconnect...", 0);
    wifi.connectWithSettings(&settings);
  }

  delay(10);
}
