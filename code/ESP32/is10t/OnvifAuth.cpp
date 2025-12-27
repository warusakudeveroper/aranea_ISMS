/**
 * OnvifAuth.cpp
 *
 * ONVIF WS-Security UsernameToken (Digest) 認証実装
 */

#include "OnvifAuth.h"
#include <mbedtls/sha1.h>
#include <mbedtls/base64.h>
#include <esp_random.h>
#include <time.h>

OnvifAuth::OnvifAuth() {
    memset(username_, 0, sizeof(username_));
    memset(password_, 0, sizeof(password_));
}

void OnvifAuth::setCredentials(const char* username, const char* password) {
    strncpy(username_, username, sizeof(username_) - 1);
    strncpy(password_, password, sizeof(password_) - 1);
}

String OnvifAuth::generateCreated() {
    time_t now;
    time(&now);
    struct tm timeinfo;
    gmtime_r(&now, &timeinfo);

    char buf[32];
    strftime(buf, sizeof(buf), "%Y-%m-%dT%H:%M:%SZ", &timeinfo);
    return String(buf);
}

void OnvifAuth::generateNonce(uint8_t* nonce, size_t len) {
    esp_fill_random(nonce, len);
}

String OnvifAuth::base64Encode(const uint8_t* data, size_t len) {
    size_t outLen = 0;
    // 必要なバッファサイズを計算
    mbedtls_base64_encode(NULL, 0, &outLen, data, len);

    char* buf = (char*)malloc(outLen + 1);
    if (!buf) return "";

    mbedtls_base64_encode((unsigned char*)buf, outLen + 1, &outLen, data, len);
    buf[outLen] = '\0';

    String result = String(buf);
    free(buf);
    return result;
}

void OnvifAuth::sha1Hash(const uint8_t* data, size_t len, uint8_t* hash) {
    mbedtls_sha1(data, len, hash);
}

String OnvifAuth::generateSecurityHeader() {
    // 1. nonce生成（16バイト）
    uint8_t nonce[16];
    generateNonce(nonce, sizeof(nonce));
    String nonceB64 = base64Encode(nonce, sizeof(nonce));

    // 2. created生成（ISO8601形式）
    String created = generateCreated();

    // 3. PasswordDigest = Base64(SHA1(nonce + created + password))
    size_t inputLen = sizeof(nonce) + created.length() + strlen(password_);
    uint8_t* input = (uint8_t*)malloc(inputLen);
    if (!input) return "";

    size_t offset = 0;
    memcpy(input + offset, nonce, sizeof(nonce));
    offset += sizeof(nonce);
    memcpy(input + offset, created.c_str(), created.length());
    offset += created.length();
    memcpy(input + offset, password_, strlen(password_));

    uint8_t sha1Result[20];
    sha1Hash(input, inputLen, sha1Result);
    free(input);

    String digestB64 = base64Encode(sha1Result, sizeof(sha1Result));

    // 4. WS-Security ヘッダ構築
    String header = "";
    header += "<wsse:Security s:mustUnderstand=\"true\" ";
    header += "xmlns:wsse=\"http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd\" ";
    header += "xmlns:wsu=\"http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd\">";
    header += "<wsse:UsernameToken>";
    header += "<wsse:Username>" + String(username_) + "</wsse:Username>";
    header += "<wsse:Password Type=\"http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest\">";
    header += digestB64;
    header += "</wsse:Password>";
    header += "<wsse:Nonce EncodingType=\"http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary\">";
    header += nonceB64;
    header += "</wsse:Nonce>";
    header += "<wsu:Created>" + created + "</wsu:Created>";
    header += "</wsse:UsernameToken>";
    header += "</wsse:Security>";

    return header;
}

String OnvifAuth::buildSoapEnvelope(const String& bodyContent) {
    String envelope = "";
    envelope += "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";
    envelope += "<s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\">";
    envelope += "<s:Header>";
    envelope += generateSecurityHeader();
    envelope += "</s:Header>";
    envelope += "<s:Body>";
    envelope += bodyContent;
    envelope += "</s:Body>";
    envelope += "</s:Envelope>";
    return envelope;
}
