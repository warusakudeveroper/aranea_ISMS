#pragma once

/**
 * AraneaGlobal - ISMS ESP32 共通モジュール
 *
 * このヘッダをインクルードすることで、全ての共通モジュールが使用可能になる
 */

// 基本モジュール
#include "LacisIDGenerator.h"
#include "SettingManager.h"
#include "WiFiManager.h"
#include "DisplayManager.h"
#include "NtpManager.h"

// BLE/HTTP モジュール
#include "BleIsmsParser.h"
#include "HttpRelayClient.h"

// 将来の拡張用（現在は未実装）
// #include "Operator.h"
// #include "OtaManager.h"
// #include "SPIFFSManager.h"
// #include "HttpManager.h"
// #include "AraneaRegister.h"

namespace aranea {
  // バージョン情報
  constexpr const char* VERSION = "0.1.0";

  // productCode（固定）
  constexpr const char* PRODUCT_CODE = "0096";

  // productType
  constexpr const char* PRODUCT_TYPE_IS01 = "001";
  constexpr const char* PRODUCT_TYPE_IS02 = "002";
  constexpr const char* PRODUCT_TYPE_IS03 = "003";
  constexpr const char* PRODUCT_TYPE_IS04 = "004";
  constexpr const char* PRODUCT_TYPE_IS05 = "005";

  // placeholder for linkage check
  void noop();
}
