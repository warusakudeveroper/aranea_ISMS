/**
 * pintest - GPIO18/GPIO5 単純出力テスト
 *
 * 目的: CH1(GPIO18), CH2(GPIO5) の電圧出力問題を検証
 *
 * 動作:
 * - 10秒間 HIGH (3.3V出力)
 * - 10秒間 LOW (0V出力)
 * - 上記を繰り返し
 *
 * 機能:
 * - WiFi接続（OTA用）
 * - HTTP OTA更新 (/update)
 * - OLED状態表示
 * - 最小限のGPIO制御のみ
 *
 * 検証ポイント:
 * - ESP-IDF gpio_* APIのみ使用（Arduinoのpinmode/digitalWrite不使用）
 * - gpio_pad_select_gpio() + gpio_set_direction() + gpio_set_level()
 *
 * ビルド:
 * - パーティション: min_spiffs
 * - FQBN: esp32:esp32:esp32:PartitionScheme=min_spiffs
 */

#include <Arduino.h>
#include <Wire.h>
#include <WiFi.h>
#include <WebServer.h>
#include <Update.h>
#include <Adafruit_SSD1306.h>
#include <driver/gpio.h>
#include <rom/gpio.h>
#include <soc/io_mux_reg.h>
#include <soc/gpio_reg.h>
#include <soc/gpio_periph.h>
#include <esp_mac.h>

// ========================================
// デバイス定数
// ========================================
static const char* FIRMWARE_VERSION = "pintest-1.0.0";

// ========================================
// テスト対象GPIO
// ========================================
static const gpio_num_t TEST_GPIO_CH1 = GPIO_NUM_18;
static const gpio_num_t TEST_GPIO_CH2 = GPIO_NUM_5;

// 比較用（正常動作確認済み）
static const gpio_num_t TEST_GPIO_CH3 = GPIO_NUM_15;
static const gpio_num_t TEST_GPIO_CH4 = GPIO_NUM_27;

// ========================================
// タイミング
// ========================================
static const unsigned long ON_DURATION_MS = 10000;   // 10秒ON
static const unsigned long OFF_DURATION_MS = 10000;  // 10秒OFF
static const unsigned long DISPLAY_UPDATE_MS = 1000; // 1秒毎に表示更新

// I2C & OLED
static const int I2C_SDA = 21;
static const int I2C_SCL = 22;
static const int OLED_WIDTH = 128;
static const int OLED_HEIGHT = 64;
static const int OLED_ADDR = 0x3C;

// WiFi設定（cluster1-6をトライ）
const char* WIFI_SSIDS[] = {"cluster1", "cluster2", "cluster3", "cluster4", "cluster5", "cluster6"};
const char* WIFI_PASS = "isms12345@";
const int WIFI_SSID_COUNT = 6;

// ========================================
// グローバル変数
// ========================================
Adafruit_SSD1306 oled(OLED_WIDTH, OLED_HEIGHT, &Wire, -1);
WebServer server(80);

String myMac;
String myIP;
int myRSSI = 0;

// テスト状態
bool gpioState = false;           // 現在の出力状態
unsigned long lastToggleTime = 0; // 最後のトグル時刻
unsigned long lastDisplayUpdate = 0;
int cycleCount = 0;               // トグル回数

// OTA状態
bool otaInProgress = false;
int otaProgress = 0;

// ========================================
// 関数プロトタイプ
// ========================================
void initTestGpios();
void toggleGpios();
void updateDisplay();
String getGpioDebugInfo();
bool connectWiFi();
void setupWebServer();
void handleRoot();
void handleGpioDebug();
void handleStatus();
void handleOtaUpdate();
void handleOtaUpload();

// ========================================
// setup()
// ========================================
void setup() {
  Serial.begin(115200);
  delay(100);

  Serial.println();
  Serial.println("============================================");
  Serial.println("  PINTEST - GPIO18/GPIO5 Simple Test");
  Serial.printf("  Firmware: %s\n", FIRMWARE_VERSION);
  Serial.println("============================================");

  // I2C & OLED初期化
  Wire.begin(I2C_SDA, I2C_SCL);
  if (oled.begin(SSD1306_SWITCHCAPVCC, OLED_ADDR)) {
    oled.clearDisplay();
    oled.setTextColor(SSD1306_WHITE);
    oled.setTextSize(1);
    oled.setCursor(0, 0);
    oled.println("PINTEST Boot...");
    oled.display();
  } else {
    Serial.println("OLED init failed!");
  }

  // MAC取得
  uint8_t mac[6];
  esp_read_mac(mac, ESP_MAC_WIFI_STA);
  char macStr[13];
  snprintf(macStr, sizeof(macStr), "%02X%02X%02X%02X%02X%02X",
           mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
  myMac = String(macStr);
  Serial.printf("MAC: %s\n", myMac.c_str());

  // WiFi接続
  oled.clearDisplay();
  oled.setCursor(0, 0);
  oled.println("WiFi connecting...");
  oled.display();

  if (connectWiFi()) {
    myIP = WiFi.localIP().toString();
    myRSSI = WiFi.RSSI();
    Serial.printf("WiFi: Connected! IP=%s, RSSI=%d\n", myIP.c_str(), myRSSI);

    oled.clearDisplay();
    oled.setCursor(0, 0);
    oled.println("WiFi OK");
    oled.println(myIP);
    oled.display();
  } else {
    Serial.println("WiFi: All SSIDs failed!");
    oled.clearDisplay();
    oled.setCursor(0, 0);
    oled.println("WiFi FAILED!");
    oled.display();
  }

  // WebServer設定
  setupWebServer();
  Serial.printf("HTTP: Server started at http://%s/\n", myIP.c_str());

  // GPIO初期化
  initTestGpios();

  Serial.println("Setup complete. Starting GPIO test loop.");

  oled.clearDisplay();
  oled.setCursor(0, 0);
  oled.println("PINTEST Ready");
  oled.println(myIP);
  oled.display();
  delay(1000);

  lastToggleTime = millis();
}

// ========================================
// loop()
// ========================================
void loop() {
  unsigned long now = millis();

  // HTTP処理
  server.handleClient();

  // OTA中はGPIOテスト停止
  if (otaInProgress) {
    delay(1);
    return;
  }

  // 10秒毎にトグル
  unsigned long elapsed = now - lastToggleTime;
  unsigned long targetDuration = gpioState ? ON_DURATION_MS : OFF_DURATION_MS;

  if (elapsed >= targetDuration) {
    toggleGpios();
    lastToggleTime = now;
  }

  // 1秒毎にディスプレイ更新
  if (now - lastDisplayUpdate >= DISPLAY_UPDATE_MS) {
    lastDisplayUpdate = now;
    updateDisplay();
  }

  delay(1);
}

// ========================================
// WiFi接続
// ========================================
bool connectWiFi() {
  WiFi.mode(WIFI_STA);
  WiFi.setHostname(("pintest-" + myMac.substring(8)).c_str());

  for (int i = 0; i < WIFI_SSID_COUNT; i++) {
    Serial.printf("WiFi: Trying %s...\n", WIFI_SSIDS[i]);

    WiFi.begin(WIFI_SSIDS[i], WIFI_PASS);

    int timeout = 20;  // 10秒
    while (WiFi.status() != WL_CONNECTED && timeout > 0) {
      delay(500);
      Serial.print(".");
      timeout--;
    }
    Serial.println();

    if (WiFi.status() == WL_CONNECTED) {
      return true;
    }

    WiFi.disconnect();
    delay(100);
  }

  return false;
}

// ========================================
// WebServer設定
// ========================================
void setupWebServer() {
  server.on("/", HTTP_GET, handleRoot);
  server.on("/api/debug/gpio", HTTP_GET, handleGpioDebug);
  server.on("/api/status", HTTP_GET, handleStatus);
  server.on("/update", HTTP_POST, handleOtaUpdate, handleOtaUpload);
  server.begin();
}

// ========================================
// ルートページ
// ========================================
void handleRoot() {
  String html = R"(<!DOCTYPE html>
<html><head>
<meta charset='UTF-8'>
<meta http-equiv='refresh' content='5'>
<title>PINTEST</title>
<style>
body { font-family: monospace; padding: 20px; }
pre { background: #f0f0f0; padding: 10px; overflow-x: auto; }
.high { color: green; font-weight: bold; }
.low { color: gray; }
</style>
</head><body>
<h1>PINTEST - GPIO Output Test</h1>
<p>Firmware: )";
  html += FIRMWARE_VERSION;
  html += "</p><p>State: <span class='";
  html += gpioState ? "high'>HIGH (3.3V)" : "low'>LOW (0V)";
  html += "</span></p>";
  html += "<p>Cycle: " + String(cycleCount) + "</p>";
  html += "<p>Free Heap: " + String(ESP.getFreeHeap()) + " bytes</p>";
  html += "<h2>GPIO Register Status</h2><pre>";
  html += getGpioDebugInfo();
  html += "</pre><h2>OTA Update</h2>";
  html += "<form method='POST' action='/update' enctype='multipart/form-data'>";
  html += "<input type='file' name='firmware' accept='.bin'>";
  html += "<input type='submit' value='Upload Firmware'>";
  html += "</form></body></html>";
  server.send(200, "text/html", html);
}

// ========================================
// GPIO診断API
// ========================================
void handleGpioDebug() {
  server.send(200, "application/json", getGpioDebugInfo());
}

// ========================================
// ステータスAPI
// ========================================
void handleStatus() {
  String json = "{";
  json += "\"firmware\":\"" + String(FIRMWARE_VERSION) + "\",";
  json += "\"gpioState\":\"" + String(gpioState ? "HIGH" : "LOW") + "\",";
  json += "\"cycleCount\":" + String(cycleCount) + ",";
  json += "\"freeHeap\":" + String(ESP.getFreeHeap()) + ",";
  json += "\"ip\":\"" + myIP + "\",";
  json += "\"rssi\":" + String(WiFi.RSSI());
  json += "}";
  server.send(200, "application/json", json);
}

// ========================================
// OTA更新完了ハンドラ
// ========================================
void handleOtaUpdate() {
  otaInProgress = false;
  bool success = !Update.hasError();
  String msg = success ? "Update successful! Rebooting..." : "Update failed!";
  server.send(200, "text/plain", msg);
  if (success) {
    delay(1000);
    ESP.restart();
  }
}

// ========================================
// OTAアップロードハンドラ
// ========================================
void handleOtaUpload() {
  HTTPUpload& upload = server.upload();

  if (upload.status == UPLOAD_FILE_START) {
    Serial.printf("OTA: Starting update with %s\n", upload.filename.c_str());
    otaInProgress = true;
    otaProgress = 0;

    // OLED表示
    oled.clearDisplay();
    oled.setCursor(0, 0);
    oled.setTextSize(2);
    oled.println("OTA...");
    oled.setTextSize(1);
    oled.display();

    if (!Update.begin(UPDATE_SIZE_UNKNOWN)) {
      Serial.printf("OTA: Begin failed - %s\n", Update.errorString());
    }
  } else if (upload.status == UPLOAD_FILE_WRITE) {
    if (Update.write(upload.buf, upload.currentSize) != upload.currentSize) {
      Serial.printf("OTA: Write failed - %s\n", Update.errorString());
    } else {
      otaProgress = upload.totalSize / 1024;
      Serial.printf("OTA: %d KB\r", otaProgress);
    }
  } else if (upload.status == UPLOAD_FILE_END) {
    if (Update.end(true)) {
      Serial.printf("\nOTA: Success! Total: %d bytes\n", upload.totalSize);
    } else {
      Serial.printf("\nOTA: End failed - %s\n", Update.errorString());
    }
  }
}

// ========================================
// GPIO初期化（ESP-IDF API使用）
// ========================================
void initTestGpios() {
  Serial.println("\n=== GPIO Initialization ===");

  gpio_num_t pins[] = {TEST_GPIO_CH1, TEST_GPIO_CH2, TEST_GPIO_CH3, TEST_GPIO_CH4};
  const char* names[] = {"CH1(GPIO18)", "CH2(GPIO5)", "CH3(GPIO15)", "CH4(GPIO27)"};

  for (int i = 0; i < 4; i++) {
    gpio_num_t pin = pins[i];
    Serial.printf("\nInitializing %s...\n", names[i]);

    // Step 1: IO_MUXでGPIO functionを選択
    gpio_pad_select_gpio(pin);
    Serial.printf("  [1] gpio_pad_select_gpio(%d) done\n", pin);

    // Step 2: GPIO Matrixで出力をシンプルGPIOに設定
    gpio_matrix_out(pin, 256, false, false);  // 256 = SIG_GPIO_OUT_IDX
    Serial.printf("  [2] gpio_matrix_out(%d, 256) done\n", pin);

    // Step 3: gpio_config()で完全な設定
    gpio_config_t io_conf = {};
    io_conf.intr_type = GPIO_INTR_DISABLE;
    io_conf.mode = GPIO_MODE_INPUT_OUTPUT;  // 出力＋読み返し
    io_conf.pin_bit_mask = (1ULL << pin);
    io_conf.pull_down_en = GPIO_PULLDOWN_DISABLE;
    io_conf.pull_up_en = GPIO_PULLUP_DISABLE;
    esp_err_t err = gpio_config(&io_conf);
    Serial.printf("  [3] gpio_config() = %d (0=OK)\n", err);

    // Step 4: ドライブ能力を最大に
    gpio_set_drive_capability(pin, GPIO_DRIVE_CAP_3);
    Serial.printf("  [4] gpio_set_drive_capability(CAP_3) done\n");

    // Step 5: 初期状態LOW
    gpio_set_level(pin, 0);
    Serial.printf("  [5] gpio_set_level(%d, 0) done\n", pin);

    // 確認読み取り
    int level = gpio_get_level(pin);
    Serial.printf("  [CHECK] gpio_get_level() = %d\n", level);
  }

  Serial.println("\n=== GPIO Initialization Complete ===\n");
}

// ========================================
// GPIO トグル
// ========================================
void toggleGpios() {
  gpioState = !gpioState;
  cycleCount++;

  int level = gpioState ? 1 : 0;

  Serial.printf("\n========== Toggle #%d: %s ==========\n",
                cycleCount, gpioState ? "HIGH (3.3V)" : "LOW (0V)");

  gpio_num_t pins[] = {TEST_GPIO_CH1, TEST_GPIO_CH2, TEST_GPIO_CH3, TEST_GPIO_CH4};
  const char* names[] = {"CH1(GPIO18)", "CH2(GPIO5)", "CH3(GPIO15)", "CH4(GPIO27)"};

  for (int i = 0; i < 4; i++) {
    gpio_num_t pin = pins[i];

    // 設定前の状態
    int before = gpio_get_level(pin);

    // 出力設定
    gpio_set_level(pin, level);

    // 設定後の状態
    int after = gpio_get_level(pin);

    Serial.printf("%s: before=%d -> set=%d -> after=%d %s\n",
                  names[i], before, level, after,
                  (after == level) ? "OK" : "MISMATCH!");
  }

  // レジスタ直接確認
  Serial.println("\n--- Register Check ---");
  uint32_t out_reg = REG_READ(GPIO_OUT_REG);
  uint32_t enable_reg = REG_READ(GPIO_ENABLE_REG);
  uint32_t in_reg = REG_READ(GPIO_IN_REG);

  Serial.printf("GPIO_OUT_REG    = 0x%08X\n", out_reg);
  Serial.printf("GPIO_ENABLE_REG = 0x%08X\n", enable_reg);
  Serial.printf("GPIO_IN_REG     = 0x%08X\n", in_reg);

  Serial.println("\n--- Per-Pin Status ---");
  for (int i = 0; i < 4; i++) {
    int pin = (int)pins[i];
    int out_bit = (out_reg >> pin) & 1;
    int en_bit = (enable_reg >> pin) & 1;
    int in_bit = (in_reg >> pin) & 1;

    const char* status = (out_bit == level && en_bit == 1 && in_bit == level) ? "OK" : "ISSUE";

    Serial.printf("%s (GPIO%d): OUT=%d EN=%d IN=%d [%s]\n",
                  names[i], pin, out_bit, en_bit, in_bit, status);
  }

  Serial.println("=====================================\n");
}

// ========================================
// ディスプレイ更新
// ========================================
void updateDisplay() {
  if (otaInProgress) {
    oled.clearDisplay();
    oled.setTextSize(2);
    oled.setCursor(0, 0);
    oled.println("OTA...");
    oled.setTextSize(1);
    oled.setCursor(0, 24);
    oled.printf("%d KB", otaProgress);
    oled.display();
    return;
  }

  oled.clearDisplay();

  // Line 1: IP + RSSI
  oled.setTextSize(1);
  oled.setCursor(0, 0);
  if (WiFi.status() == WL_CONNECTED) {
    oled.printf("%s %ddBm", myIP.c_str(), WiFi.RSSI());
  } else {
    oled.println("WiFi Disconnected");
  }

  // Line 2: 状態表示
  oled.setTextSize(2);
  oled.setCursor(0, 16);
  oled.println(gpioState ? "HIGH" : "LOW");

  // Line 3: 残り時間とサイクル
  oled.setTextSize(1);
  oled.setCursor(0, 40);
  unsigned long elapsed = millis() - lastToggleTime;
  unsigned long targetDuration = gpioState ? ON_DURATION_MS : OFF_DURATION_MS;
  int remaining = (targetDuration - elapsed) / 1000;
  if (remaining < 0) remaining = 0;
  oled.printf("Next: %ds  Cycle:%d", remaining, cycleCount);

  // Line 4: ヒープ
  oled.setCursor(0, 54);
  oled.printf("Heap: %d bytes", ESP.getFreeHeap());

  oled.display();
}

// ========================================
// GPIO診断情報取得
// ========================================
String getGpioDebugInfo() {
  String json = "{\n";

  gpio_num_t pins[] = {TEST_GPIO_CH1, TEST_GPIO_CH2, TEST_GPIO_CH3, TEST_GPIO_CH4};
  const char* names[] = {"CH1_GPIO18", "CH2_GPIO5", "CH3_GPIO15", "CH4_GPIO27"};

  uint32_t out_reg = REG_READ(GPIO_OUT_REG);
  uint32_t enable_reg = REG_READ(GPIO_ENABLE_REG);
  uint32_t in_reg = REG_READ(GPIO_IN_REG);

  json += "  \"firmware\": \"" + String(FIRMWARE_VERSION) + "\",\n";
  json += "  \"currentState\": \"" + String(gpioState ? "HIGH" : "LOW") + "\",\n";
  json += "  \"cycleCount\": " + String(cycleCount) + ",\n";
  json += "  \"freeHeap\": " + String(ESP.getFreeHeap()) + ",\n";
  json += "  \"GPIO_OUT_REG\": \"0x" + String(out_reg, HEX) + "\",\n";
  json += "  \"GPIO_ENABLE_REG\": \"0x" + String(enable_reg, HEX) + "\",\n";
  json += "  \"GPIO_IN_REG\": \"0x" + String(in_reg, HEX) + "\",\n";
  json += "  \"pins\": [\n";

  for (int i = 0; i < 4; i++) {
    int pin = (int)pins[i];
    int out_bit = (out_reg >> pin) & 1;
    int en_bit = (enable_reg >> pin) & 1;
    int in_bit = (in_reg >> pin) & 1;

    // IO_MUX function select
    uint32_t iomux_addr = GPIO_PIN_MUX_REG[pin];
    uint32_t iomux_val = REG_READ(iomux_addr);
    int funSel = (iomux_val >> 12) & 0x7;

    // GPIO Matrix signal ID
    uint32_t sig_out = REG_READ(GPIO_FUNC0_OUT_SEL_CFG_REG + (pin * 4));
    int sigOutId = sig_out & 0x1FF;

    int expectedLevel = gpioState ? 1 : 0;
    bool isOk = (out_bit == expectedLevel && en_bit == 1 && in_bit == expectedLevel);

    json += "    {\n";
    json += "      \"name\": \"" + String(names[i]) + "\",\n";
    json += "      \"gpio\": " + String(pin) + ",\n";
    json += "      \"OUT\": " + String(out_bit) + ",\n";
    json += "      \"ENABLE\": " + String(en_bit) + ",\n";
    json += "      \"IN\": " + String(in_bit) + ",\n";
    json += "      \"IOMUX_FUNC\": " + String(funSel) + ",\n";
    json += "      \"SIG_OUT_ID\": " + String(sigOutId) + ",\n";
    json += "      \"isSimpleGpio\": " + String(sigOutId == 256 ? "true" : "false") + ",\n";
    json += "      \"status\": \"" + String(isOk ? "OK" : "ISSUE") + "\"\n";
    json += "    }";
    if (i < 3) json += ",";
    json += "\n";
  }

  json += "  ]\n";
  json += "}";

  return json;
}
