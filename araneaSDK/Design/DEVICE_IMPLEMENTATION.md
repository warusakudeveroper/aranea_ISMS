# AraneaSDK Device Implementation Guide

## 1. Overview

ESP32デバイスでaraneaDevice機能を実装するためのガイドです。

---

## 2. Required Modules

### 2.1 Core Modules (必須)

| Module | File | Purpose |
|--------|------|---------|
| LacisIDGenerator | global/src/LacisIDGenerator.h | LacisID生成 |
| SettingManager | global/src/SettingManager.h | NVS設定管理 |
| AraneaRegister | global/src/AraneaRegister.h | DeviceGate登録 |
| StateReporter | global/src/StateReporter.h | 状態レポート送信 |

### 2.2 Optional Modules

| Module | File | Purpose |
|--------|------|---------|
| WiFiManager | global/src/WiFiManager.h | WiFi接続管理 |
| NtpManager | global/src/NtpManager.h | NTP時刻同期 |
| MqttManager | global/src/MqttManager.h | MQTT接続 |
| AraneaWebUI | global/src/AraneaWebUI.h | Web UI |
| IOController | global/src/IOController.h | GPIO制御 |
| DisplayManager | global/src/DisplayManager.h | OLED表示 |
| OtaManager | global/src/OtaManager.h | OTA更新 |

---

## 3. Implementation Steps

### Step 1: Project Setup

```cpp
// AraneaGlobalImporter.h
#ifndef ARANEA_GLOBAL_IMPORTER_H
#define ARANEA_GLOBAL_IMPORTER_H

// 共通モジュールのインクルードパス設定
#include "../global/src/LacisIDGenerator.h"
#include "../global/src/SettingManager.h"
#include "../global/src/AraneaRegister.h"
#include "../global/src/StateReporter.h"
#include "../global/src/WiFiManager.h"
#include "../global/src/NtpManager.h"
#include "../global/src/AraneaWebUI.h"

#endif
```

### Step 2: Device Settings

```cpp
// AraneaSettings.h
#ifndef ARANEA_SETTINGS_H
#define ARANEA_SETTINGS_H

namespace aranea {
  // Device Identity
  constexpr const char* DEVICE_TYPE = "ar-is04a";
  constexpr const char* MODEL_NAME = "Window Controller";
  constexpr const char* PRODUCT_TYPE = "004";
  constexpr const char* PRODUCT_CODE = "0001";

  // Firmware Version
  constexpr const char* FIRMWARE_VERSION = "1.0.0";
  constexpr const char* BUILD_DATE = __DATE__ " " __TIME__;

  // Default Tenant (市山水産)
  constexpr const char* DEFAULT_TID = "T2025120608261484221";
  constexpr const char* DEFAULT_TENANT_LACISID = "12767487939173857894";
  constexpr const char* DEFAULT_TENANT_EMAIL = "info+ichiyama@neki.tech";
  constexpr const char* DEFAULT_TENANT_CIC = "263238";

  // Endpoints
  constexpr const char* DEFAULT_GATE_URL =
    "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate";
  constexpr const char* DEFAULT_STATE_URL =
    "https://us-central1-mobesorder.cloudfunctions.net/deviceStateReport";

  // Local Relay
  constexpr const char* DEFAULT_RELAY_PRIMARY = "192.168.96.201";
  constexpr const char* DEFAULT_RELAY_SECONDARY = "192.168.96.202";
  constexpr int RELAY_PORT = 8080;

  // WiFi
  constexpr const char* DEFAULT_SSID_PREFIX = "cluster";
  constexpr const char* DEFAULT_WIFI_PASS = "isms12345@";
}

#endif
```

### Step 3: LacisID Generation

```cpp
// main.ino
#include "AraneaGlobalImporter.h"
#include "AraneaSettings.h"

LacisIDGenerator lacisGen;
String lacisId;

void setup() {
  Serial.begin(115200);

  // LacisID生成
  lacisId = lacisGen.generate(
    aranea::PRODUCT_TYPE,
    aranea::PRODUCT_CODE
  );

  Serial.printf("LacisID: %s\n", lacisId.c_str());
  // 例: 30040123456789AB0001
}
```

### Step 4: Settings Management

```cpp
SettingManager settings;

void setup() {
  // NVS初期化
  settings.begin("isms");

  // 設定読み込み
  String tid = settings.getString("tid", aranea::DEFAULT_TID);
  String cic = settings.getString("cic", "");

  // 設定保存
  settings.setString("tid", tid);
  settings.setBool("registered", false);
}
```

### Step 5: WiFi Connection

```cpp
WiFiManager wifi;

void setup() {
  wifi.begin(&settings);

  // 接続待機
  wifi.onConnected([]() {
    Serial.printf("Connected! IP: %s\n", WiFi.localIP().toString().c_str());
  });

  wifi.onDisconnected([]() {
    Serial.println("WiFi disconnected");
  });
}

void loop() {
  wifi.handle();
}
```

### Step 6: Device Registration

```cpp
AraneaRegister reg;
bool registered = false;

void setup() {
  // 既存CIC確認
  String cic = settings.getString("cic", "");
  registered = (cic.length() == 6);

  if (!registered) {
    registerDevice();
  }
}

void registerDevice() {
  reg.begin(&settings);

  // Tenant Primary認証情報設定
  reg.setTenantPrimary(
    aranea::DEFAULT_TENANT_LACISID,
    aranea::DEFAULT_TENANT_EMAIL,
    aranea::DEFAULT_TENANT_CIC
  );

  // デバイス情報設定
  reg.setDeviceInfo(
    lacisId,
    aranea::DEFAULT_TID,
    "araneaDevice",
    String("ISMS_") + aranea::DEVICE_TYPE
  );

  reg.setDeviceMeta(
    lacisGen.getStaMac12Hex(),
    aranea::PRODUCT_TYPE,
    aranea::PRODUCT_CODE
  );

  // 登録実行
  AraneaRegisterResult result = reg.registerToGate(aranea::DEFAULT_GATE_URL);

  if (result.ok) {
    Serial.printf("Registered! CIC: %s\n", result.cic_code.c_str());
    settings.setString("cic", result.cic_code);
    settings.setString("stateEndpoint", result.stateEndpoint);
    registered = true;
  } else {
    Serial.printf("Registration failed: %s\n", result.error.c_str());
  }
}
```

### Step 7: State Reporting

```cpp
StateReporter reporter;

void setup() {
  reporter.begin(settings.getString("stateEndpoint", aranea::DEFAULT_STATE_URL));

  // 認証情報設定
  reporter.setAuth(
    settings.getString("tid", aranea::DEFAULT_TID),
    lacisId,
    settings.getString("cic", "")
  );

  // レポート間隔設定 (60秒)
  reporter.setHeartbeatInterval(60000);

  // ペイロード構築コールバック
  reporter.onBuildPayload([]() -> String {
    return buildStatePayload();
  });

  // 送信結果コールバック
  reporter.onReportSent([](bool success, int code) {
    Serial.printf("Report sent: %s (code: %d)\n",
      success ? "OK" : "FAIL", code);
  });
}

void loop() {
  reporter.handle();
}

String buildStatePayload() {
  StaticJsonDocument<512> doc;

  doc["type"] = String("ISMS_") + aranea::DEVICE_TYPE;

  JsonObject state = doc.createNestedObject("state");
  // デバイス固有の状態を設定
  state["output1"]["active"] = digitalRead(OUTPUT1_PIN);
  state["output2"]["active"] = digitalRead(OUTPUT2_PIN);

  JsonObject meta = doc.createNestedObject("meta");
  meta["firmwareVersion"] = aranea::FIRMWARE_VERSION;
  meta["rssi"] = WiFi.RSSI();
  meta["heap"] = ESP.getFreeHeap();

  String payload;
  serializeJson(doc, payload);
  return payload;
}
```

### Step 8: Local Relay Reporting

```cpp
void sendToLocalRelay() {
  String relayPri = String("http://") + aranea::DEFAULT_RELAY_PRIMARY +
                    ":" + String(aranea::RELAY_PORT) + "/api/state";

  HTTPClient http;
  http.begin(relayPri);
  http.addHeader("Content-Type", "application/json");
  http.setTimeout(3000);

  StaticJsonDocument<512> doc;
  doc["tid"] = settings.getString("tid", aranea::DEFAULT_TID);
  doc["lacisId"] = lacisId;
  doc["cic"] = settings.getString("cic", "");
  doc["mac"] = lacisGen.getStaMac12Hex();
  doc["state"] = buildState();
  doc["timestamp"] = getIsoTimestamp();

  String payload;
  serializeJson(doc, payload);

  int code = http.POST(payload);
  http.end();

  if (code != 200) {
    // Secondary にフォールバック
    sendToSecondaryRelay(payload);
  }
}
```

---

## 4. Web UI Implementation

### 4.1 Base Class Extension

```cpp
// HttpManagerIs04a.h
#include "AraneaWebUI.h"

class HttpManagerIs04a : public AraneaWebUI {
public:
  void begin(SettingManager* settings, TriggerManager* trigger, int port = 80);

protected:
  // オーバーライド
  void getTypeSpecificStatus(JsonObject& obj) override;
  String generateTypeSpecificTabs() override;
  String generateTypeSpecificTabContents() override;
  String generateTypeSpecificJS() override;
  void registerTypeSpecificEndpoints() override;

private:
  TriggerManager* trigger_;
};
```

### 4.2 Implementation

```cpp
// HttpManagerIs04a.cpp
void HttpManagerIs04a::begin(SettingManager* settings, TriggerManager* trigger, int port) {
  trigger_ = trigger;
  AraneaWebUI::begin(settings, port);
}

void HttpManagerIs04a::getTypeSpecificStatus(JsonObject& obj) {
  obj["output1"] = trigger_->getOutput1State();
  obj["output2"] = trigger_->getOutput2State();
  obj["input1"] = trigger_->getInput1State();
  obj["input2"] = trigger_->getInput2State();
}

String HttpManagerIs04a::generateTypeSpecificTabs() {
  return R"(<li><a href="#control">Control</a></li>)";
}

String HttpManagerIs04a::generateTypeSpecificTabContents() {
  return R"(
    <div id="control" class="tab-content">
      <h3>Output Control</h3>
      <button onclick="pulse(1)">Output 1 Pulse</button>
      <button onclick="pulse(2)">Output 2 Pulse</button>
    </div>
  )";
}

void HttpManagerIs04a::registerTypeSpecificEndpoints() {
  server_.on("/api/pulse", HTTP_POST, [this]() {
    int output = server_.arg("output").toInt();
    int duration = server_.arg("duration").toInt();
    trigger_->startPulse(output, duration, TriggerManager::PulseSource::HTTP);
    server_.send(200, "application/json", "{\"ok\":true}");
  });
}
```

---

## 5. MQTT Implementation (Bidirectional)

### 5.1 Setup

```cpp
MqttManager mqtt;

void setup() {
  String mqttEndpoint = settings.getString("mqttEndpoint", "");
  if (mqttEndpoint.length() > 0) {
    mqtt.begin(mqttEndpoint, lacisId, settings.getString("cic", ""));

    mqtt.onConnected([]() {
      // コマンドトピック購読
      mqtt.subscribe("aranea/" + lacisId + "/cmd");
      mqtt.subscribe("aranea/" + lacisId + "/config");
    });

    mqtt.onMessage([](const String& topic, const String& payload) {
      handleMqttMessage(topic, payload);
    });
  }
}

void loop() {
  mqtt.handle();
}
```

### 5.2 Command Handling

```cpp
void handleMqttMessage(const String& topic, const String& payload) {
  StaticJsonDocument<512> doc;
  deserializeJson(doc, payload);

  if (topic.endsWith("/cmd")) {
    String command = doc["command"].as<String>();
    JsonObject params = doc["params"].as<JsonObject>();

    if (command == "pulse") {
      int output = params["output"].as<int>();
      int duration = params["durationMs"].as<int>();
      trigger_->startPulse(output, duration, TriggerManager::PulseSource::CLOUD);
    }

    // 実行結果を送信
    String resultTopic = "aranea/" + lacisId + "/event";
    StaticJsonDocument<256> result;
    result["commandId"] = doc["commandId"];
    result["status"] = "executed";
    result["timestamp"] = getIsoTimestamp();
    String resultPayload;
    serializeJson(result, resultPayload);
    mqtt.publish(resultTopic, resultPayload);
  }
}
```

---

## 6. GPIO Control

### 6.1 IOController Usage

```cpp
IOController input1, input2, output1, output2;

void setup() {
  // 入力設定
  input1.begin(INPUT1_PIN, IOController::Mode::IO_IN);
  input1.setDebounceMs(50);
  input1.setPullup(true);

  input2.begin(INPUT2_PIN, IOController::Mode::IO_IN);
  input2.setDebounceMs(50);
  input2.setPullup(true);

  // 出力設定
  output1.begin(OUTPUT1_PIN, IOController::Mode::IO_OUT);
  output2.begin(OUTPUT2_PIN, IOController::Mode::IO_OUT);

  // コールバック
  input1.onStateChange([](bool active) {
    Serial.printf("Input1: %s\n", active ? "ACTIVE" : "INACTIVE");
    if (active) {
      output1.pulse(3000);  // 3秒パルス
    }
  });
}

void loop() {
  input1.sample();
  input2.sample();
  output1.update();
  output2.update();
}
```

---

## 7. NVS Key Convention

### 7.1 Standard Keys

```cpp
// System
"device_name"     // デバイス名
"device_id"       // デバイスID (DID)

// WiFi
"wifi_ssid1"～"wifi_ssid6"
"wifi_pass1"～"wifi_pass6"

// Aranea
"tid"             // Tenant ID
"fid"             // Facility ID
"cic"             // CIC Code
"lacisId"         // LacisID (自動生成)
"registered"      // 登録フラグ
"stateEndpoint"   // 状態レポートURL
"mqttEndpoint"    // MQTT URL

// Tenant Auth
"tenant_lacisid"  // Tenant Primary LacisID
"tenant_email"    // Tenant Primary Email
"tenant_cic"      // Tenant Primary CIC
"gate_url"        // DeviceGate URL

// Local Relay
"relay_pri"       // Primary Relay IP
"relay_sec"       // Secondary Relay IP

// Device Specific
"is04_pulse_ms"   // パルス幅
"is04_interlock_ms" // インターロック
"is04_debounce_ms"  // デバウンス
```

---

## 8. Error Handling

### 8.1 Network Errors

```cpp
// バックオフ制御
static int consecutiveFailures = 0;
static const int MAX_FAILURES = 3;
static const unsigned long BACKOFF_MS = 30000;
static unsigned long backoffUntil = 0;

void sendReport() {
  if (millis() < backoffUntil) {
    return;  // バックオフ中
  }

  HTTPClient http;
  http.setTimeout(3000);  // 3秒タイムアウト
  // ...

  int code = http.POST(payload);

  if (code == 200) {
    consecutiveFailures = 0;
  } else {
    consecutiveFailures++;
    if (consecutiveFailures >= MAX_FAILURES) {
      backoffUntil = millis() + BACKOFF_MS;
      Serial.println("Entering backoff mode");
    }
  }
}
```

### 8.2 Registration Retry

```cpp
void checkRegistration() {
  if (!registered && WiFi.isConnected()) {
    static unsigned long lastAttempt = 0;
    static int attempts = 0;

    if (millis() - lastAttempt > 30000) {  // 30秒間隔
      lastAttempt = millis();
      attempts++;

      if (attempts <= 5) {
        registerDevice();
      } else {
        // 諦めてローカルモードで動作
        Serial.println("Registration failed, running in local mode");
      }
    }
  }
}
```

---

## 9. Best Practices

### 9.1 Memory Management

```cpp
// StaticJsonDocument使用
StaticJsonDocument<512> doc;  // スタック上に確保

// 大きなJSONは分割処理
void sendLargePayload() {
  // ストリーミング送信
  WiFiClient client;
  HTTPClient http;
  http.begin(client, url);
  http.addHeader("Content-Type", "application/json");

  // チャンク送信
  http.sendRequest("POST", &stream, size);
}
```

### 9.2 WDT Prevention

```cpp
void loop() {
  yield();  // WDTリセット

  // 長時間処理は分割
  static int phase = 0;
  switch (phase) {
    case 0: handleWifi(); break;
    case 1: handleMqtt(); break;
    case 2: handleReporter(); break;
    case 3: handleWebUI(); break;
  }
  phase = (phase + 1) % 4;
}
```

### 9.3 Partition Scheme

```
# platformio.ini
board_build.partitions = min_spiffs.csv

# !! NEVER use huge_app - OTA will not work !!
```

---

## 10. Testing Checklist

- [ ] LacisID生成確認（20文字、形式正しい）
- [ ] WiFi接続確認（cluster1-6）
- [ ] DeviceGate登録確認（CIC取得）
- [ ] 状態レポート送信確認
- [ ] ローカルリレー送信確認
- [ ] Web UI表示確認
- [ ] GPIO動作確認
- [ ] MQTT接続確認（双方向デバイス）
- [ ] OTA更新確認
- [ ] 再起動後の復帰確認
