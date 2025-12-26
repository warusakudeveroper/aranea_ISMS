/**
 * AraneaGlobalImporter.h
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
 * @see code/ESP32/______MUST_READ_ROLE_DIVISION______.md
 */

#ifndef ARANEA_GLOBAL_IMPORTER_H
#define ARANEA_GLOBAL_IMPORTER_H

// ============================================================
// 共通モジュールインクルード（is02aで使用するもの）
// ============================================================

// --- 必須モジュール ---
#include "SettingManager.h"       // NVS永続化
#include "WiFiManager.h"          // WiFi接続管理
#include "NtpManager.h"           // NTP時刻同期
#include "LacisIDGenerator.h"     // LacisID生成
#include "AraneaRegister.h"       // araneaDeviceGate登録
#include "DisplayManager.h"       // OLED表示
#include "Operator.h"             // 状態機械

// --- 通信モジュール ---
#include "MqttManager.h"          // MQTT接続
#include "StateReporter.h"        // deviceStateReport送信
#include "HttpRelayClient.h"      // is03a中継送信
#include "AraneaWebUI.h"          // Web UI基底クラス

// --- OTA更新モジュール ---
#include "HttpOtaManager.h"       // HTTP経由OTA（推奨）

// --- BLE受信モジュール ---
#include "BleIsmsParser.h"        // BLEアドバタイズパーサー

// ============================================================
// IS02A固有モジュール（is02a/ディレクトリ内）
// ============================================================
#include "Is02aKeys.h"            // NVSキー定数
#include "AraneaSettings.h"       // 施設設定
#include "BleReceiver.h"          // BLE受信マネージャー
#include "HttpManagerIs02a.h"     // Web UI実装
#include "StateReporterIs02a.h"   // deviceStateReportペイロード構築

// ============================================================
// インポート確認用マクロ
// ============================================================
#define ARANEA_GLOBAL_IMPORTER_VERSION "1.0.0"

inline void printAraneaGlobalModules() {
  Serial.println("=== Aranea Global Modules (is02a) ===");
  Serial.println("SettingManager, WiFiManager, NtpManager");
  Serial.println("LacisIDGenerator, AraneaRegister, DisplayManager");
  Serial.println("MqttManager, StateReporter, HttpRelayClient");
  Serial.println("AraneaWebUI, HttpOtaManager, BleIsmsParser");
  Serial.println("======================================");
}

#endif // ARANEA_GLOBAL_IMPORTER_H
