/**
 * AraneaGlobalImporter.h
 *
 * ============================================================
 *  !!!!! 必読 !!!!!  ARANEA共通モジュール インポート管理
 * ============================================================
 *
 * このファイルは code/ESP32/global/ から共通モジュールを
 * 選択的にインポートするための管理クラスです。
 *
 * 【重要】ビルド設定
 * - パーティションスキーム: min_SPIFFS を必ず使用すること
 * - huge_app は絶対に使用禁止（OTA更新が不可能になる）
 * - 例: esp32:esp32:esp32:PartitionScheme=min_spiffs
 *
 * 【is10t固有機能】
 * - Tapoカメラモニター
 * - ONVIF経由でカメラ情報取得
 * - CelestialGlobeへカメラ状態送信
 * - MACアドレスでis10m（Mercury AC）データと照合してSSID/RSSI取得
 *
 * @see code/ESP32/is10t/design/DESIGN.md
 */

#ifndef ARANEA_GLOBAL_IMPORTER_H
#define ARANEA_GLOBAL_IMPORTER_H

// ============================================================
// 共通モジュールインクルード（global/src/）
// ============================================================

// --- 必須モジュール ---
#include "SettingManager.h"
#include "WiFiManager.h"
#include "NtpManager.h"
#include "LacisIDGenerator.h"
#include "AraneaRegister.h"
#include "DisplayManager.h"
#include "Operator.h"

// --- 通信モジュール ---
// #include "MqttManager.h"         // MQTT不要（ポーリング専用）
// #include "StateReporter.h"       // deviceStateReport不要
// #include "HttpRelayClient.h"     // Zero3中継不要
#include "AraneaWebUI.h"            // Web UI基底クラス

// --- OTA更新モジュール ---
#include "OtaManager.h"             // mDNS版OTA
#include "HttpOtaManager.h"         // HTTP版OTA（推奨）

// --- その他モジュール ---
// #include "SshClient.h"           // SSH不要
// #include "BleIsmsParser.h"       // BLE不要
// #include "RebootScheduler.h"     // 定時リブート（検討中）

// ============================================================
// IS10T固有モジュール（is10t/ディレクトリ内）
// ============================================================
#include "Is10tKeys.h"              // NVSキー定数（15文字制限強制）
#include "TapoTypes.h"              // カメラデータ型定義
#include "OnvifAuth.h"              // WS-Security認証
#include "OnvifClient.h"            // ONVIF SOAP通信
#include "TapoDiscovery.h"          // UDP Discovery
#include "TapoPollerIs10t.h"        // カメラ巡回ポーラー
#include "HttpManagerIs10t.h"       // Web UI実装
#include "CelestialSenderIs10t.h"   // CelestialGlobe送信

// ============================================================
// インポート確認用マクロ
// ============================================================
#define ARANEA_GLOBAL_IMPORTER_VERSION "1.0.0"
#define IS10T_VERSION "1.0.0"

/**
 * 利用可能なモジュール一覧をシリアル出力
 */
inline void printAraneaGlobalModules() {
    Serial.println("=== Aranea IS10T Modules ===");
    Serial.println("SettingManager, WiFiManager, NtpManager");
    Serial.println("LacisIDGenerator, AraneaRegister, DisplayManager");
    Serial.println("Operator, AraneaWebUI, HttpOtaManager");
    Serial.println("TapoTypes, OnvifAuth, OnvifClient");
    Serial.println("TapoPollerIs10t, HttpManagerIs10t");
    Serial.println("CelestialSenderIs10t");
    Serial.println("=============================");
}

#endif // ARANEA_GLOBAL_IMPORTER_H
