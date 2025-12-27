/**
 * TapoDiscovery.cpp
 *
 * Tapoデバイス検出（UDP）実装
 *
 * TP-Link Tapo Discovery Protocol:
 * - UDP port 9999
 * - XOR暗号化 (key = 171, each byte XOR'd with previous result)
 * - JSON payload with device_id, device_name, etc.
 */

#include "TapoDiscovery.h"
#include <WiFiUdp.h>
#include <ArduinoJson.h>

// TP-Link discovery request (already XOR encrypted)
// Original: {"system":{"get_sysinfo":{}}}
static const uint8_t DISCOVERY_REQUEST[] = {
    0x00, 0x00, 0x00, 0x23,  // Length (35 bytes)
    0xd0, 0xf2, 0x81, 0xf8, 0x8b, 0xff, 0x9a, 0xf7,
    0xd5, 0xef, 0x94, 0xb6, 0xd1, 0xb4, 0xc0, 0x9f,
    0xec, 0x95, 0xe6, 0x8f, 0xe1, 0x87, 0xe8, 0xca,
    0xf0, 0x8b, 0xf6, 0x8b, 0xf6, 0xcf, 0x88, 0xe7,
    0x93, 0xef, 0xd4
};

TapoDiscovery::TapoDiscovery() {}

void TapoDiscovery::xorEncrypt(uint8_t* data, size_t len) {
    uint8_t key = 171;
    for (size_t i = 0; i < len; i++) {
        uint8_t plain = data[i] ^ key;
        key = data[i];
        data[i] = plain;
    }
}

bool TapoDiscovery::discover(const char* ip, TapoStatus* status) {
    if (!ip || !status) {
        lastError_ = "Invalid parameters";
        return false;
    }

    WiFiUDP udp;
    IPAddress targetIP;
    if (!targetIP.fromString(ip)) {
        lastError_ = "Invalid IP address";
        return false;
    }

    // UDPソケット開始
    if (!udp.begin(0)) {  // 任意のローカルポート
        lastError_ = "UDP begin failed";
        return false;
    }

    // Discovery request送信
    udp.beginPacket(targetIP, TAPO_DISCOVERY_PORT);
    udp.write(DISCOVERY_REQUEST, sizeof(DISCOVERY_REQUEST));
    if (!udp.endPacket()) {
        lastError_ = "UDP send failed";
        udp.stop();
        return false;
    }

    Serial.printf("[Discovery] Sent to %s:%d\n", ip, TAPO_DISCOVERY_PORT);

    // 応答待機
    unsigned long startMs = millis();
    uint8_t buffer[1024];
    int packetSize = 0;

    while (millis() - startMs < TAPO_DISCOVERY_TIMEOUT_MS) {
        packetSize = udp.parsePacket();
        if (packetSize > 0) {
            break;
        }
        delay(10);
    }

    if (packetSize == 0) {
        lastError_ = "No response (timeout)";
        udp.stop();
        return false;
    }

    // 応答読み取り
    int len = udp.read(buffer, sizeof(buffer) - 1);
    udp.stop();

    if (len <= 4) {
        lastError_ = "Response too short";
        return false;
    }

    Serial.printf("[Discovery] Received %d bytes\n", len);

    // 最初の4バイトはlength prefix、スキップ
    uint8_t* payload = buffer + 4;
    int payloadLen = len - 4;

    // XOR復号化
    xorEncrypt(payload, payloadLen);
    payload[payloadLen] = '\0';

    Serial.printf("[Discovery] Decrypted: %s\n", (char*)payload);

    // JSON解析
    DynamicJsonDocument doc(1024);
    DeserializationError err = deserializeJson(doc, (char*)payload);
    if (err) {
        lastError_ = "JSON parse error: " + String(err.c_str());
        return false;
    }

    // デバイス情報抽出
    JsonObject sysinfo = doc["system"]["get_sysinfo"];
    if (sysinfo.isNull()) {
        lastError_ = "No sysinfo in response";
        return false;
    }

    // device_id
    const char* deviceId = sysinfo["deviceId"];
    if (deviceId) {
        strncpy(status->deviceId, deviceId, sizeof(status->deviceId) - 1);
    }

    // alias (device_name)
    const char* alias = sysinfo["alias"];
    if (alias) {
        strncpy(status->deviceName, alias, sizeof(status->deviceName) - 1);
    }

    // model (discovery版、ONVIF版と統合)
    const char* model = sysinfo["model"];
    if (model && strlen(status->model) == 0) {
        strncpy(status->model, model, sizeof(status->model) - 1);
    }

    status->discoverySuccess = true;
    Serial.printf("[Discovery] OK: id=%s, name=%s\n",
                  status->deviceId, status->deviceName);

    return true;
}
