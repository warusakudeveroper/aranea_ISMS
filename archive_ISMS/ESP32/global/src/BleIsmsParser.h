#pragma once
#include <Arduino.h>
#include <string>

/**
 * ISMSv1 ペイロード構造体
 */
struct IsmsPayload {
  uint8_t version;
  uint8_t productType;
  uint8_t flags;
  uint8_t staMac[6];      // STA MAC（6バイト）
  String staMac12Hex;     // STA MAC（12桁HEX文字列）
  uint32_t bootCount;
  float tempC;            // 温度（℃）
  float humPct;           // 湿度（%）
  uint8_t battPct;        // バッテリー（%）
  String rawHex;          // 生データ（HEX文字列）
};

/**
 * BleIsmsParser
 *
 * BLE Manufacturer Data からISMSv1ペイロードをパース
 */
class BleIsmsParser {
public:
  /**
   * ISMSv1かどうか判定
   * @param data Manufacturer Data
   * @return ISMSv1ならtrue
   */
  static bool isIsmsV1(const std::string& data);
  static bool isIsmsV1(const uint8_t* data, size_t len);

  /**
   * ペイロードをパース
   * @param data Manufacturer Data
   * @param payload 出力先
   * @return 成功時true
   */
  static bool parse(const std::string& data, IsmsPayload* payload);
  static bool parse(const uint8_t* data, size_t len, IsmsPayload* payload);

  /**
   * バイト配列をHEX文字列に変換
   * @param data バイト配列
   * @param len 長さ
   * @return HEX文字列
   */
  static String bytesToHex(const uint8_t* data, size_t len);

private:
  static const uint8_t MAGIC_I = 0x49;  // 'I'
  static const uint8_t MAGIC_S = 0x53;  // 'S'
  static const uint8_t VERSION_1 = 0x01;
  static const size_t MIN_PAYLOAD_LEN = 20;

  // flags ビット定義
  static const uint8_t FLAG_HAS_TEMP = 0x01;
  static const uint8_t FLAG_HAS_HUM = 0x02;
  static const uint8_t FLAG_HAS_BATT = 0x04;
  static const uint8_t FLAG_HAS_BOOTCOUNT = 0x08;
};
