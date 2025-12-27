/**
 * OnvifAuth.h
 *
 * ONVIF WS-Security UsernameToken (Digest) 認証
 *
 * Password_Digest = Base64(SHA1(nonce_raw + created + password))
 */

#ifndef ONVIF_AUTH_H
#define ONVIF_AUTH_H

#include <Arduino.h>

class OnvifAuth {
public:
    OnvifAuth();

    /**
     * 認証情報設定
     */
    void setCredentials(const char* username, const char* password);

    /**
     * WS-Security ヘッダ生成
     * @return SOAP Header内のSecurity要素（XML文字列）
     */
    String generateSecurityHeader();

    /**
     * 完全なSOAPエンベロープ生成
     * @param bodyContent SOAP Body内のコンテンツ
     * @return 完全なSOAP XMLエンベロープ
     */
    String buildSoapEnvelope(const String& bodyContent);

private:
    char username_[32];
    char password_[64];

    /**
     * ISO8601形式のタイムスタンプ生成
     */
    String generateCreated();

    /**
     * 16バイトのランダムnonce生成
     */
    void generateNonce(uint8_t* nonce, size_t len);

    /**
     * Base64エンコード
     */
    String base64Encode(const uint8_t* data, size_t len);

    /**
     * SHA1ハッシュ計算
     */
    void sha1Hash(const uint8_t* data, size_t len, uint8_t* hash);
};

#endif // ONVIF_AUTH_H
