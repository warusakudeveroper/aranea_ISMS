#pragma once
#include <Arduino.h>

class DisplayManager {
public:
  bool begin(); // I2C OLED 128x64 (scaffold only)
  void showBoot(const String& msg);
  void showError(const String& msg);
};
