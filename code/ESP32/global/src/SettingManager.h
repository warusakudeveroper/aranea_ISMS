#pragma once
#include <Arduino.h>

class SettingManager {
public:
  bool begin(); // init storage (NVS/SPIFFS later)
  String getString(const String& key, const String& defValue);
  void setString(const String& key, const String& value);
};
