/**
 * AraneaGlobalImporter.h
 *
 * ============================================================
 *  !!!!! 必読 !!!!!  ARANEA共通モジュール インポート管理
 * ============================================================
 *
 * このファイルは generic/ESP32/global/ から共通モジュールを
 * 選択的にインポートするための管理クラスです。
 *
 * 【重要】ビルド設定
 * - パーティションスキーム: min_SPIFFS を必ず使用すること
 * - huge_app は絶対に使用禁止（OTA更新が不可能になる）
 * - 例: esp32:esp32:esp32:PartitionScheme=min_spiffs
 *
 * 【モジュール一覧】
 * global/src/ に配置されている共通モジュール:
 *
 * [必須モジュール]
 * - SettingManager.h     : NVS永続化ストレージ管理
 * - WiFiManager.h        : WiFi接続管理（cluster1-6自動試行）
 * - NtpManager.h         : NTP時刻同期
 * - LacisIDGenerator.h   : LacisID生成（MAC+ProductType+Code）
 * - AraneaRegister.h     : araneaDeviceGate登録（CIC取得）
 * - DisplayManager.h     : I2C OLED表示（128x64）
 * - Operator.h           : 状態機械・競合制御
 *
 * [通信モジュール]
 * - MqttManager.h        : ESP-IDF MQTT WebSocket接続
 * - StateReporter.h      : deviceStateReport HTTP送信
 * - HttpRelayClient.h    : Zero3中継送信
 * - AraneaWebUI.h        : Web UI基底クラス
 *
 * [OTA更新モジュール]
 * - OtaManager.h         : ArduinoOTA（mDNS使用）
 * - HttpOtaManager.h     : HTTP経由OTA（mDNS非使用）
 *
 * [その他]
 * - SshClient.h          : LibSSH-ESP32ラッパー
 * - BleIsmsParser.h      : BLEアドバタイズパーサー
 * - RebootScheduler.h    : 定時リブート
 * - SPIFFSManager.h      : SPIFFS初期化
 *
 * 【使用方法】
 * 1. このファイルをis10/にインクルード
 * 2. 必要なモジュールのみ選択的にインクルード
 * 3. ArduinoIDEの場合: libraries.propertiesでglobalを参照
 * 4. PlatformIOの場合: lib_depsでglobalを参照
 *
 * 【ライブラリパス設定（Arduino IDE）】
 * /private/tmp/AraneaGlobalGeneric をライブラリとして登録
 * または generic/ESP32/global/src をシンボリックリンク
 *
 * @see generic/ESP32/______MUST_READ_ROLE_DIVISION______.md
 */

#ifndef ARANEA_GLOBAL_IMPORTER_H
#define ARANEA_GLOBAL_IMPORTER_H

// ============================================================
// 共通モジュールインクルード設定
// 使用するモジュールのコメントを外してください
// ============================================================

// --- 必須モジュール（ほぼ全デバイスで使用）---
#include "SettingManager.h"
#include "WiFiManager.h"
#include "NtpManager.h"
#include "LacisIDGenerator.h"
#include "AraneaRegister.h"
#include "DisplayManager.h"
#include "Operator.h"

// --- 通信モジュール ---
#include "MqttManager.h"
#include "StateReporter.h"
// #include "HttpRelayClient.h"  // Zero3中継が必要な場合
#include "AraneaWebUI.h"

// --- OTA更新モジュール ---
#include "OtaManager.h"          // mDNS使用版（LibSSH競合注意、is10では無効化運用）
#include "HttpOtaManager.h"      // HTTP経由版（推奨）

// --- その他モジュール ---
#include "SshClient.h"           // SSH接続が必要な場合
// #include "BleIsmsParser.h"    // BLE受信が必要な場合
// #include "RebootScheduler.h"  // 定時リブートが必要な場合

// ============================================================
// IS10固有モジュール（is10/ディレクトリ内）
// ============================================================
#include "Is10Keys.h"            // NVSキー定数（15文字制限強制）
#include "AraneaSettings.h"      // 施設設定（大量生産用）
#include "RouterTypes.h"         // ルーター設定構造体
#include "HttpManagerIs10.h"     // Web UI実装
#include "SshPollerIs10.h"       // SSHルーター巡回
#include "StateReporterIs10.h"   // deviceStateReportペイロード構築
#include "CelestialSenderIs10.h" // CelestialGlobe SSOT送信
#include "MqttConfigHandler.h"   // MQTT config受信・適用

// ============================================================
// インポート確認用マクロ
// ============================================================
#define ARANEA_GLOBAL_IMPORTER_VERSION "1.0.0"

/**
 * 利用可能なモジュール一覧をシリアル出力
 */
inline void printAraneaGlobalModules() {
  Serial.println("=== Aranea Global Modules ===");
  Serial.println("SettingManager, WiFiManager, NtpManager");
  Serial.println("LacisIDGenerator, AraneaRegister, DisplayManager");
  Serial.println("Operator, MqttManager, StateReporter");
  Serial.println("AraneaWebUI, HttpOtaManager, SshClient");
  Serial.println("==============================");
}

#endif // ARANEA_GLOBAL_IMPORTER_H
