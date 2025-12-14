#include "SettingManager.h"
bool SettingManager::begin() { return false; }
String SettingManager::getString(const String&, const String& defValue) { return defValue; }
void SettingManager::setString(const String&, const String&) {}
