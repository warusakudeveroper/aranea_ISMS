#include "LacisIDGenerator.h"
#include <esp_mac.h>

String LacisIDGenerator::generate(const String& productType, const String& productCode) {
  String mac12hex = getStaMac12Hex();
  // lacisId = "3" + productType(3) + MAC12HEX + productCode(4)
  return "3" + productType + mac12hex + productCode;
}

String LacisIDGenerator::getStaMac12Hex() {
  uint8_t mac[6];
  esp_read_mac(mac, ESP_MAC_WIFI_STA);
  return macBytesToHex(mac);
}

String LacisIDGenerator::reconstructFromMac(uint8_t productType, const uint8_t* staMac, const char* productCode) {
  char lacisId[21];
  snprintf(lacisId, sizeof(lacisId),
    "3%03d%02X%02X%02X%02X%02X%02X%s",
    productType,
    staMac[0], staMac[1], staMac[2],
    staMac[3], staMac[4], staMac[5],
    productCode);
  return String(lacisId);
}

String LacisIDGenerator::macBytesToHex(const uint8_t* mac) {
  char hex[13];
  snprintf(hex, sizeof(hex), "%02X%02X%02X%02X%02X%02X",
    mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
  return String(hex);
}
