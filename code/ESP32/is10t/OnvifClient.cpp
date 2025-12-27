/**
 * OnvifClient.cpp
 *
 * ONVIF SOAP通信クライアント実装
 */

#include "OnvifClient.h"
#include <HTTPClient.h>
#include <WiFiClient.h>

OnvifClient::OnvifClient() {
    memset(host_, 0, sizeof(host_));
    port_ = TAPO_ONVIF_PORT;
    timeoutMs_ = TAPO_HTTP_TIMEOUT_MS;
}

void OnvifClient::setTarget(const char* host, uint16_t port) {
    strncpy(host_, host, sizeof(host_) - 1);
    port_ = port;
}

void OnvifClient::setCredentials(const char* username, const char* password) {
    auth_.setCredentials(username, password);
}

String OnvifClient::sendSoapRequest(const String& soapBody) {
    WiFiClient client;
    HTTPClient http;

    String url = "http://" + String(host_) + ":" + String(port_) + "/onvif/device_service";

    http.begin(client, url);
    http.setTimeout(timeoutMs_);
    http.addHeader("Content-Type", "application/soap+xml; charset=utf-8");

    String envelope = auth_.buildSoapEnvelope(soapBody);

    int httpCode = http.POST(envelope);

    String response = "";
    if (httpCode == HTTP_CODE_OK) {
        response = http.getString();
    } else {
        lastError_ = "HTTP Error: " + String(httpCode);
        Serial.printf("[OnvifClient] HTTP Error: %d\n", httpCode);
    }

    http.end();
    return response;
}

String OnvifClient::extractElement(const String& xml, const String& tag) {
    // まず完全なタグ名で検索
    String startTag = "<" + tag + ">";
    String endTag = "</" + tag + ">";

    int start = xml.indexOf(startTag);
    if (start >= 0) {
        int end = xml.indexOf(endTag, start);
        if (end > start) {
            return xml.substring(start + startTag.length(), end);
        }
    }

    // 名前空間付きタグを検索（例: <tds:Model> -> :Model>）
    String nsTag = ":" + tag + ">";
    start = xml.indexOf(nsTag);
    if (start >= 0) {
        // 開始タグの先頭を探す
        int tagStart = xml.lastIndexOf('<', start);
        if (tagStart >= 0) {
            start = start + nsTag.length();
            // 終了タグを探す
            String nsEndTag = ":" + tag + ">";
            int end = xml.indexOf("</" , start);
            if (end >= 0) {
                int endClose = xml.indexOf(">", end);
                if (endClose >= 0 && xml.substring(end, endClose + 1).indexOf(tag) >= 0) {
                    return xml.substring(start, end);
                }
            }
            // 単純に次の < まで取得
            end = xml.indexOf('<', start);
            if (end > start) {
                return xml.substring(start, end);
            }
        }
    }

    return "";
}

bool OnvifClient::parseMacAddress(const String& macStr, uint8_t* mac) {
    // 形式: "bc-07-1d-b9-48-17" または "BC:07:1D:B9:48:17"
    String normalized = macStr;
    normalized.replace("-", ":");
    normalized.toUpperCase();

    int idx = 0;
    int pos = 0;
    for (int i = 0; i < 6 && pos < normalized.length(); i++) {
        String hex = normalized.substring(pos, pos + 2);
        mac[i] = strtol(hex.c_str(), NULL, 16);
        pos += 3; // Skip "XX:"
    }

    return true;
}

uint32_t OnvifClient::parseIpAddress(const String& ipStr) {
    uint32_t result = 0;
    int parts[4] = {0};
    int idx = 0;

    String temp = ipStr;
    for (int i = 0; i < 4 && idx < temp.length(); i++) {
        int dotPos = temp.indexOf('.', idx);
        if (dotPos < 0) dotPos = temp.length();
        parts[i] = temp.substring(idx, dotPos).toInt();
        idx = dotPos + 1;
    }

    result = (parts[0] << 24) | (parts[1] << 16) | (parts[2] << 8) | parts[3];
    return result;
}

bool OnvifClient::getDeviceInfo(TapoStatus* status) {
    String body = "<GetDeviceInformation xmlns=\"http://www.onvif.org/ver10/device/wsdl\"/>";
    String response = sendSoapRequest(body);

    if (response.length() == 0) {
        return false;
    }

    // Fault検出
    if (response.indexOf("Fault") >= 0) {
        lastError_ = "SOAP Fault in response";
        Serial.println("[OnvifClient] SOAP Fault detected");
        return false;
    }

    // デバイス情報抽出
    String model = extractElement(response, "Model");
    String firmware = extractElement(response, "FirmwareVersion");
    String serial = extractElement(response, "SerialNumber");
    String hwId = extractElement(response, "HardwareId");

    if (model.length() > 0) {
        strncpy(status->model, model.c_str(), sizeof(status->model) - 1);
    }
    if (firmware.length() > 0) {
        strncpy(status->firmware, firmware.c_str(), sizeof(status->firmware) - 1);
    }
    if (serial.length() > 0) {
        strncpy(status->serialNumber, serial.c_str(), sizeof(status->serialNumber) - 1);
    }
    if (hwId.length() > 0) {
        strncpy(status->hardwareId, hwId.c_str(), sizeof(status->hardwareId) - 1);
    }

    // Hostname取得
    body = "<GetHostname xmlns=\"http://www.onvif.org/ver10/device/wsdl\"/>";
    response = sendSoapRequest(body);
    if (response.length() > 0 && response.indexOf("Fault") < 0) {
        String hostname = extractElement(response, "Name");
        if (hostname.length() > 0) {
            strncpy(status->hostname, hostname.c_str(), sizeof(status->hostname) - 1);
        }
    }

    return model.length() > 0;
}

bool OnvifClient::getNetworkInfo(TapoStatus* status) {
    // GetNetworkInterfaces
    String body = "<GetNetworkInterfaces xmlns=\"http://www.onvif.org/ver10/device/wsdl\"/>";
    String response = sendSoapRequest(body);

    if (response.length() == 0 || response.indexOf("Fault") >= 0) {
        lastError_ = "GetNetworkInterfaces failed";
        return false;
    }

    // MAC抽出
    String hwAddress = extractElement(response, "HwAddress");
    if (hwAddress.length() > 0) {
        parseMacAddress(hwAddress, status->mac);
    }

    // IP抽出
    String ipAddress = extractElement(response, "Address");
    if (ipAddress.length() > 0) {
        status->ip = parseIpAddress(ipAddress);
    }

    // Prefix抽出
    String prefix = extractElement(response, "PrefixLength");
    if (prefix.length() > 0) {
        status->prefixLength = prefix.toInt();
    }

    // DHCP抽出
    if (response.indexOf("<tt:DHCP>true</tt:DHCP>") >= 0 ||
        response.indexOf(":DHCP>true<") >= 0) {
        status->dhcp = true;
    }

    // GetNetworkDefaultGateway
    body = "<GetNetworkDefaultGateway xmlns=\"http://www.onvif.org/ver10/device/wsdl\"/>";
    response = sendSoapRequest(body);
    if (response.length() > 0 && response.indexOf("Fault") < 0) {
        String gateway = extractElement(response, "IPv4Address");
        if (gateway.length() > 0) {
            status->gateway = parseIpAddress(gateway);
        }
    }

    return status->mac[0] != 0 || status->mac[1] != 0;
}

bool OnvifClient::getAllInfo(TapoStatus* status) {
    bool deviceOk = getDeviceInfo(status);
    bool networkOk = getNetworkInfo(status);

    status->onvifSuccess = deviceOk && networkOk;
    return status->onvifSuccess;
}
