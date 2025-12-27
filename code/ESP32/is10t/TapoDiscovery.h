/**
 * TapoDiscovery.h
 *
 * Tapoデバイス検出（UDP broadcast）
 *
 * TP-Link Tapoデバイスは UDP port 9999 でbroadcast discoveryに応答する。
 * device_id, device_name などを取得可能。
 */

#ifndef TAPO_DISCOVERY_H
#define TAPO_DISCOVERY_H

#include <Arduino.h>
#include "TapoTypes.h"

// Discovery設定
#define TAPO_DISCOVERY_PORT 9999
#define TAPO_DISCOVERY_TIMEOUT_MS 3000

class TapoDiscovery {
public:
    TapoDiscovery();

    /**
     * 指定IPのデバイスからDiscovery情報を取得
     * @param ip ターゲットIPアドレス
     * @param status 取得結果を格納するTapoStatus
     * @return 成功時true
     */
    bool discover(const char* ip, TapoStatus* status);

    /**
     * 最後のエラーメッセージ
     */
    String getLastError() const { return lastError_; }

private:
    String lastError_;

    /**
     * XOR暗号化/復号化（Tapo用）
     */
    void xorEncrypt(uint8_t* data, size_t len);
};

#endif // TAPO_DISCOVERY_H
