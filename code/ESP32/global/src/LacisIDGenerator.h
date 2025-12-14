#pragma once
#include <Arduino.h>
#include <WiFi.h>

/**
 * LacisIDGenerator
 *
 * lacisId生成器（deviceWithMAC仕様）
 * - MACは平文12桁HEXをそのまま埋め込む（数値変換禁止）
 * - lacisId = "3" + productType(3桁) + MAC12HEX + productCode(4桁)
 * - 合計20文字固定
 */
class LacisIDGenerator {
public:
  /**
   * lacisIdを生成
   * @param productType 3桁の製品タイプ（例："001", "002"）
   * @param productCode 4桁の製品コード（例："0096"）
   * @return lacisId（20文字）
   */
  String generate(const String& productType, const String& productCode);

  /**
   * 自機のSTA MAC（WiFi MAC）を12桁HEX（大文字、コロン無し）で取得
   * @return MAC12HEX（例："AABBCCDDEEFF"）
   */
  String getStaMac12Hex();

  /**
   * MACバイト配列からlacisIdを再構成（受信側で使用）
   * @param productType 製品タイプ（数値）
   * @param staMac MACアドレス（6バイト）
   * @param productCode 製品コード（デフォルト "0096"）
   * @return lacisId（20文字）
   */
  static String reconstructFromMac(uint8_t productType, const uint8_t* staMac, const char* productCode = "0096");

  /**
   * MACバイト配列を12桁HEX文字列に変換
   * @param mac MACアドレス（6バイト）
   * @return MAC12HEX（大文字）
   */
  static String macBytesToHex(const uint8_t* mac);
};
