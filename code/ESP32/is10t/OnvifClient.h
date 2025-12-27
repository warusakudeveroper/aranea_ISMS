/**
 * OnvifClient.h
 *
 * ONVIF SOAP通信クライアント
 *
 * 対応API:
 * - GetDeviceInformation
 * - GetNetworkInterfaces
 * - GetHostname
 * - GetNetworkDefaultGateway
 */

#ifndef ONVIF_CLIENT_H
#define ONVIF_CLIENT_H

#include <Arduino.h>
#include "TapoTypes.h"
#include "OnvifAuth.h"

class OnvifClient {
public:
    OnvifClient();

    /**
     * 接続先設定
     */
    void setTarget(const char* host, uint16_t port = TAPO_ONVIF_PORT);

    /**
     * 認証情報設定
     */
    void setCredentials(const char* username, const char* password);

    /**
     * タイムアウト設定
     */
    void setTimeout(uint32_t timeoutMs) { timeoutMs_ = timeoutMs; }

    /**
     * デバイス情報取得
     * @param status 結果格納先
     * @return 成功/失敗
     */
    bool getDeviceInfo(TapoStatus* status);

    /**
     * ネットワーク情報取得
     * @param status 結果格納先
     * @return 成功/失敗
     */
    bool getNetworkInfo(TapoStatus* status);

    /**
     * 全情報取得（デバイス情報 + ネットワーク情報）
     * @param status 結果格納先
     * @return 成功/失敗
     */
    bool getAllInfo(TapoStatus* status);

    /**
     * 最後のエラーメッセージ取得
     */
    String getLastError() const { return lastError_; }

private:
    char host_[16];
    uint16_t port_;
    uint32_t timeoutMs_;
    OnvifAuth auth_;
    String lastError_;

    /**
     * SOAP POST送信
     */
    String sendSoapRequest(const String& soapBody);

    /**
     * XML要素抽出（軽量パーサー）
     */
    String extractElement(const String& xml, const String& tag);

    /**
     * MACアドレス文字列をバイト配列に変換
     */
    bool parseMacAddress(const String& macStr, uint8_t* mac);

    /**
     * IPアドレス文字列をuint32_tに変換
     */
    uint32_t parseIpAddress(const String& ipStr);
};

#endif // ONVIF_CLIENT_H
