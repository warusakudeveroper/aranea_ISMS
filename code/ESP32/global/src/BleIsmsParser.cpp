#include "BleIsmsParser.h"

bool BleIsmsParser::isIsmsV1(const std::string& data) {
  return isIsmsV1(reinterpret_cast<const uint8_t*>(data.data()), data.size());
}

bool BleIsmsParser::isIsmsV1(const uint8_t* data, size_t len) {
  if (len < MIN_PAYLOAD_LEN) return false;
  if (data[0] != MAGIC_I || data[1] != MAGIC_S) return false;
  if (data[2] != VERSION_1) return false;
  return true;
}

bool BleIsmsParser::parse(const std::string& data, IsmsPayload* payload) {
  return parse(reinterpret_cast<const uint8_t*>(data.data()), data.size(), payload);
}

bool BleIsmsParser::parse(const uint8_t* data, size_t len, IsmsPayload* payload) {
  if (!isIsmsV1(data, len)) return false;
  if (!payload) return false;

  // ヘッダ
  payload->version = data[2];
  payload->productType = data[3];
  payload->flags = data[4];

  // STA MAC（offset 5-10）
  memcpy(payload->staMac, &data[5], 6);
  payload->staMac12Hex = bytesToHex(payload->staMac, 6);

  // bootCount（offset 11-14, little endian）
  payload->bootCount = data[11] |
                       (data[12] << 8) |
                       (data[13] << 16) |
                       (data[14] << 24);

  // 温度（offset 15-16, int16 little endian, x100）
  int16_t temp_x100 = data[15] | (data[16] << 8);
  payload->tempC = temp_x100 / 100.0f;

  // 湿度（offset 17-18, uint16 little endian, x100）
  uint16_t hum_x100 = data[17] | (data[18] << 8);
  payload->humPct = hum_x100 / 100.0f;

  // バッテリー（offset 19）
  payload->battPct = data[19];

  // 生データHEX
  payload->rawHex = bytesToHex(data, len);

  return true;
}

String BleIsmsParser::bytesToHex(const uint8_t* data, size_t len) {
  String hex;
  hex.reserve(len * 2);
  char buf[3];
  for (size_t i = 0; i < len; i++) {
    snprintf(buf, sizeof(buf), "%02X", data[i]);
    hex += buf;
  }
  return hex;
}
