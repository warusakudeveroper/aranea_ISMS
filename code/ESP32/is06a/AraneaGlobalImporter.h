/**
 * AraneaGlobalImporter.h (is06a用)
 *
 * ============================================================
 *  !!!!! 必読 !!!!!  ARANEA共通モジュール インポート管理
 * ============================================================
 *
 * このファイルは code/ESP32/global/ から共通モジュールを
 * 選択的にインポートするための管理ファイルです。
 *
 * 【重要】ビルド設定
 * - パーティションスキーム: min_SPIFFS を必ず使用すること
 * - huge_app は絶対に使用禁止（OTA更新が不可能になる）
 * - 例: esp32:esp32:esp32:PartitionScheme=min_spiffs
 *
 * 【ライブラリパス設定】
 * Arduino IDE: code/ESP32/global/src をライブラリとして登録
 * PlatformIO: lib_deps で global を参照
 */

#ifndef ARANEA_GLOBAL_IMPORTER_H
#define ARANEA_GLOBAL_IMPORTER_H

// ============================================================
// 共通モジュールインクルード
// ============================================================

// --- 必須モジュール ---
#include "SettingManager.h"       // NVS永続化
#include "WiFiManager.h"          // WiFi接続
#include "NtpManager.h"           // NTP時刻同期
#include "LacisIDGenerator.h"     // LacisID生成（必須）
#include "AraneaRegister.h"       // CIC取得（必須）
#include "DisplayManager.h"       // OLED表示
#include "Operator.h"             // 状態機械

// --- 通信モジュール ---
#include "AraneaWebUI.h"          // Web UI基底クラス
// #include "MqttManager.h"       // MQTT（将来対応）

// --- OTA更新モジュール ---
#include "OtaManager.h"           // mDNS OTA
#include "HttpOtaManager.h"       // HTTP OTA

// --- I/Oモジュール ---
#include "IOController.h"         // 共通I/O制御

// ============================================================
// IS06A固有モジュール（is06a/ディレクトリ内）
// ============================================================
#include "Is06aKeys.h"            // NVSキー定数
#include "AraneaSettings.h"       // 施設設定（大量生産用）
#include "TriggerManagerIs06a.h"  // 6chトリガー管理
#include "HttpManagerIs06a.h"     // Web UI実装
#include "StateReporterIs06a.h"   // 状態レポート構築

// ============================================================
// インポート確認用マクロ
// ============================================================
#define ARANEA_GLOBAL_IMPORTER_VERSION "1.0.0"

inline void printAraneaGlobalModules() {
  Serial.println("=== Aranea Global Modules (is06a) ===");
  Serial.println("SettingManager, WiFiManager, NtpManager");
  Serial.println("LacisIDGenerator, AraneaRegister, DisplayManager");
  Serial.println("Operator, AraneaWebUI, IOController");
  Serial.println("OtaManager, HttpOtaManager");
  Serial.println("TriggerManagerIs06a (6ch PWM/IO)");
  Serial.println("=====================================");
}

#endif // ARANEA_GLOBAL_IMPORTER_H
