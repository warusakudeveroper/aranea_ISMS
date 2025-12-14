#include "DisplayManager.h"

bool DisplayManager::begin() {
  if (initialized_) return true;

  // GPIO5をHIGHに設定（I2Cレベルシフタ/イネーブル制御用）
  pinMode(5, OUTPUT);
  digitalWrite(5, HIGH);
  delay(10);

  // I2C初期化（デフォルトピン: SDA=21, SCL=22）
  Wire.begin();
  Wire.setClock(100000);  // 100kHz（標準モード）

  // I2Cデバイス存在確認（複数アドレス試行 + 読み取りテスト）
  bool found = false;
  uint8_t addrs[] = {0x3C, 0x3D};
  uint8_t foundAddr = 0;

  for (uint8_t addr : addrs) {
    Wire.beginTransmission(addr);
    uint8_t error = Wire.endTransmission();
    if (error == 0) {
      // さらに1バイト読み取りテスト（偽ACK防止）
      uint8_t bytesRead = Wire.requestFrom(addr, (uint8_t)1);
      if (bytesRead == 1) {
        found = true;
        foundAddr = addr;
        Serial.printf("[DISPLAY] Found OLED at 0x%02X\n", addr);
        break;
      }
    }
  }

  if (!found) {
    Serial.println("[DISPLAY] No OLED found, display disabled");
    Wire.end();  // I2Cを終了してリソース解放
    return false;
  }

  // OLED初期化
  oled_ = new Adafruit_SSD1306(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire, -1);

  if (!oled_->begin(SSD1306_SWITCHCAPVCC, foundAddr)) {
    Serial.println("[DISPLAY] SSD1306 init failed");
    delete oled_;
    oled_ = nullptr;
    return false;
  }

  oled_->clearDisplay();
  oled_->setTextSize(1);
  oled_->setTextColor(SSD1306_WHITE);
  oled_->display();

  initialized_ = true;
  Serial.println("[DISPLAY] OLED ready");
  return true;
}

void DisplayManager::showBoot(const String& msg) {
  if (!initialized_) return;

  oled_->clearDisplay();
  oled_->setTextSize(1);
  oled_->setCursor(0, 24);
  oled_->println("ISMS is02");
  oled_->setCursor(0, 40);
  oled_->println(msg);
  oled_->display();
}

void DisplayManager::showError(const String& msg) {
  if (!initialized_) return;

  oled_->clearDisplay();
  oled_->setTextSize(1);
  oled_->setCursor(0, 0);
  oled_->println("ERROR:");
  oled_->setCursor(0, 16);
  oled_->println(msg);
  oled_->display();
}

void DisplayManager::clear() {
  if (!initialized_) return;
  oled_->clearDisplay();
  oled_->display();
}

void DisplayManager::show4Lines(const String& line1, const String& line2,
                                 const String& line3, const String& line4) {
  if (!initialized_) return;

  oled_->clearDisplay();
  oled_->setTextSize(1);

  oled_->setCursor(0, 0);
  oled_->println(line1);

  oled_->setCursor(0, 16);
  oled_->println(line2);

  oled_->setCursor(0, 32);
  oled_->println(line3);

  oled_->setCursor(0, 48);
  oled_->println(line4);

  oled_->display();
}

void DisplayManager::updateLine(int lineNum, const String& text) {
  if (!initialized_ || lineNum < 0 || lineNum > 3) return;

  int y = lineNum * LINE_HEIGHT;

  // 該当行をクリア（黒で塗りつぶし）
  oled_->fillRect(0, y, SCREEN_WIDTH, LINE_HEIGHT, SSD1306_BLACK);

  // テキスト描画
  oled_->setCursor(0, y);
  oled_->println(text);
}

void DisplayManager::display() {
  if (!initialized_) return;
  oled_->display();
}

void DisplayManager::showIs02Main(const String& line1, const String& cic,
                                   const String& sensorLine, bool showLink) {
  if (!initialized_) return;

  oled_->clearDisplay();

  // Line 1: IP + RSSI (小さいフォント, Y=0)
  oled_->setTextSize(1);
  oled_->setCursor(0, 0);
  oled_->print(line1);

  // リンクマーク（送信中）右上に表示
  if (showLink) {
    oled_->setCursor(116, 0);
    oled_->print(">>");
  }

  // Line 2-3: CIC (大きいフォント size=3, 中央寄せ, Y=12)
  oled_->setTextSize(3);
  // CICを中央に配置（6文字 * 18px = 108px, 余白 = (128-108)/2 = 10）
  int cicWidth = cic.length() * 18;
  int cicX = (SCREEN_WIDTH - cicWidth) / 2;
  if (cicX < 0) cicX = 0;
  oled_->setCursor(cicX, 14);
  oled_->print(cic);

  // Line 4: Sensor info (大きめフォント size=2, Y=44)
  oled_->setTextSize(2);
  oled_->setCursor(0, 48);
  oled_->print(sensorLine);

  oled_->display();
}

void DisplayManager::showConnecting(const String& ssid, int frame) {
  if (!initialized_) return;

  oled_->clearDisplay();

  // アニメーションドット
  String dots = "";
  for (int i = 0; i <= (frame % 4); i++) dots += ".";

  oled_->setTextSize(1);
  oled_->setCursor(0, 0);
  oled_->print("connecting");
  oled_->print(dots);

  oled_->setCursor(0, 12);
  oled_->print(ssid);

  oled_->display();
}

void DisplayManager::showRegistering(const String& status) {
  if (!initialized_) return;

  oled_->clearDisplay();

  oled_->setTextSize(1);
  oled_->setCursor(0, 0);
  oled_->println("Registering...");

  oled_->setCursor(0, 16);
  oled_->println(status);

  oled_->display();
}
