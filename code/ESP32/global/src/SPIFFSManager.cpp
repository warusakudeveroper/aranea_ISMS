#include "SPIFFSManager.h"

bool SPIFFSManager::begin(bool formatOnFail) {
  if (initialized_) return true;

  if (!SPIFFS.begin(formatOnFail)) {
    Serial.println("[SPIFFS] Mount failed");
    return false;
  }

  initialized_ = true;
  Serial.printf("[SPIFFS] Mounted: %d/%d bytes used\n", usedSpace(), totalSpace());
  return true;
}

String SPIFFSManager::readFile(const String& path) {
  if (!initialized_) return "";

  File file = SPIFFS.open(path, "r");
  if (!file) {
    return "";
  }

  String content = file.readString();
  file.close();
  return content;
}

bool SPIFFSManager::writeFile(const String& path, const String& content) {
  if (!initialized_) return false;

  File file = SPIFFS.open(path, "w");
  if (!file) {
    Serial.printf("[SPIFFS] Failed to open %s for writing\n", path.c_str());
    return false;
  }

  size_t written = file.print(content);
  file.close();
  return written == content.length();
}

bool SPIFFSManager::appendFile(const String& path, const String& content) {
  if (!initialized_) return false;

  File file = SPIFFS.open(path, "a");
  if (!file) {
    Serial.printf("[SPIFFS] Failed to open %s for appending\n", path.c_str());
    return false;
  }

  size_t written = file.print(content);
  file.close();
  return written == content.length();
}

bool SPIFFSManager::deleteFile(const String& path) {
  if (!initialized_) return false;
  return SPIFFS.remove(path);
}

bool SPIFFSManager::exists(const String& path) {
  if (!initialized_) return false;
  return SPIFFS.exists(path);
}

size_t SPIFFSManager::freeSpace() {
  if (!initialized_) return 0;
  return SPIFFS.totalBytes() - SPIFFS.usedBytes();
}

size_t SPIFFSManager::totalSpace() {
  if (!initialized_) return 0;
  return SPIFFS.totalBytes();
}

size_t SPIFFSManager::usedSpace() {
  if (!initialized_) return 0;
  return SPIFFS.usedBytes();
}
