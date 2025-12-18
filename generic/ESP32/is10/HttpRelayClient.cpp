#include "HttpRelayClient.h"

HttpRelayClient::HttpRelayClient(const String& primaryUrl, const String& secondaryUrl)
  : primaryUrl_(primaryUrl), secondaryUrl_(secondaryUrl) {
}

HttpRelayResult HttpRelayClient::postJson(const String& json) {
  // まずprimaryに送信
  HttpRelayResult result = postJsonTo(primaryUrl_, json);
  if (result.ok) {
    return result;
  }

  Serial.printf("[HTTP] Primary failed (%d), trying secondary...\n", result.httpCode);

  // primaryが失敗したらsecondaryにフォールバック
  result = postJsonTo(secondaryUrl_, json);
  return result;
}

HttpRelayResult HttpRelayClient::postJsonTo(const String& url, const String& json) {
  HttpRelayResult result;
  result.url = url;
  result.ok = false;
  result.httpCode = -1;

  HTTPClient http;
  http.setTimeout(TIMEOUT_MS);

  if (!http.begin(url)) {
    result.error = "Failed to begin HTTP";
    Serial.printf("[HTTP] %s\n", result.error.c_str());
    return result;
  }

  http.addHeader("Content-Type", "application/json");

  int httpCode = http.POST(json);
  result.httpCode = httpCode;

  if (httpCode == 200 || httpCode == 201 || httpCode == 204) {
    result.ok = true;
    Serial.printf("[HTTP] POST OK to %s (%d)\n", url.c_str(), httpCode);
  } else if (httpCode > 0) {
    result.error = "HTTP error: " + String(httpCode);
    Serial.printf("[HTTP] POST failed to %s: %d\n", url.c_str(), httpCode);
  } else {
    result.error = http.errorToString(httpCode);
    Serial.printf("[HTTP] POST failed to %s: %s\n", url.c_str(), result.error.c_str());
  }

  http.end();
  return result;
}

void HttpRelayClient::setUrls(const String& primaryUrl, const String& secondaryUrl) {
  primaryUrl_ = primaryUrl;
  secondaryUrl_ = secondaryUrl;
}
