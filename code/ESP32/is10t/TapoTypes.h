/**
 * TapoTypes.h
 *
 * Tapoカメラモニター用データ型定義
 */

#ifndef TAPO_TYPES_H
#define TAPO_TYPES_H

#include <Arduino.h>

// 最大カメラ数（ESP32メモリ制約考慮）
#define MAX_TAPO_CAMERAS 8

// ONVIFデフォルトポート
#define TAPO_ONVIF_PORT 2020

// タイムアウト設定
#define TAPO_HTTP_TIMEOUT_MS 8000
#define TAPO_CAMERA_GAP_MS 2000

// オフライン判定閾値
#define TAPO_FAIL_THRESHOLD 3

/**
 * カメラ設定構造体
 */
struct TapoConfig {
    char host[16];           // IPアドレス
    uint16_t onvifPort;      // ONVIFポート（デフォルト2020）
    char username[32];       // ONVIFユーザー名
    char password[64];       // ONVIFパスワード
    bool enabled;            // 有効フラグ

    TapoConfig() {
        memset(host, 0, sizeof(host));
        onvifPort = TAPO_ONVIF_PORT;
        memset(username, 0, sizeof(username));
        memset(password, 0, sizeof(password));
        enabled = false;
    }
};

/**
 * カメラステータス構造体（約350バイト/台）
 */
struct TapoStatus {
    // 識別情報（Discovery経由）
    char deviceId[48];       // device_id
    char deviceName[64];     // device_name（日本語対応）

    // デバイス情報（ONVIF経由）
    char model[32];          // Tapo C530WS
    char firmware[48];       // 1.3.3 Build...
    char serialNumber[20];   // 74614817
    char hardwareId[8];      // 2.0
    char hostname[32];       // C530WS

    // ネットワーク情報（ONVIF経由）
    uint8_t mac[6];          // MACアドレス（照合キー）
    uint32_t ip;             // IPアドレス
    uint32_t gateway;        // ゲートウェイ
    uint8_t prefixLength;    // サブネット
    bool dhcp;               // DHCP有効

    // 状態
    bool online;             // オンライン状態
    bool onvifSuccess;       // ONVIF取得成功
    bool discoverySuccess;   // Discovery取得成功
    unsigned long lastPollMs;// 最終ポーリング時刻
    uint8_t failCount;       // 連続失敗回数

    TapoStatus() {
        memset(deviceId, 0, sizeof(deviceId));
        memset(deviceName, 0, sizeof(deviceName));
        memset(model, 0, sizeof(model));
        memset(firmware, 0, sizeof(firmware));
        memset(serialNumber, 0, sizeof(serialNumber));
        memset(hardwareId, 0, sizeof(hardwareId));
        memset(hostname, 0, sizeof(hostname));
        memset(mac, 0, sizeof(mac));
        ip = 0;
        gateway = 0;
        prefixLength = 0;
        dhcp = false;
        online = false;
        onvifSuccess = false;
        discoverySuccess = false;
        lastPollMs = 0;
        failCount = 0;
    }

    /**
     * MACアドレスを文字列で取得（コロン区切り）
     */
    String getMacString() const {
        char buf[18];
        snprintf(buf, sizeof(buf), "%02X:%02X:%02X:%02X:%02X:%02X",
                 mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
        return String(buf);
    }

    /**
     * IPアドレスを文字列で取得
     */
    String getIpString() const {
        char buf[16];
        snprintf(buf, sizeof(buf), "%d.%d.%d.%d",
                 (ip >> 24) & 0xFF, (ip >> 16) & 0xFF,
                 (ip >> 8) & 0xFF, ip & 0xFF);
        return String(buf);
    }
};

/**
 * ポーリング結果サマリ
 */
struct TapoPollSummary {
    uint8_t total;           // 設定済みカメラ数
    uint8_t online;          // オンライン数
    uint8_t offline;         // オフライン数
    unsigned long lastPollMs;// 最終ポーリング完了時刻

    TapoPollSummary() : total(0), online(0), offline(0), lastPollMs(0) {}
};

#endif // TAPO_TYPES_H
