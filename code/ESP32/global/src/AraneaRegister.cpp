#include "AraneaRegister.h"
#include <HTTPClient.h>
#include <ArduinoJson.h>
#include <Preferences.h>

const char* AraneaRegister::NVS_NAMESPACE = "aranea";
const char* AraneaRegister::KEY_CIC = "cic";
const char* AraneaRegister::KEY_STATE_ENDPOINT = "stateEp";
const char* AraneaRegister::KEY_REGISTERED = "registered";

void AraneaRegister::begin(const String& gateUrl) {
  gateUrl_ = gateUrl;
}

void AraneaRegister::setTenantPrimary(const TenantPrimaryAuth& auth) {
  tenantAuth_ = auth;
}

AraneaRegisterResult AraneaRegister::registerDevice(
  const String& tid,
  const String& deviceType,
  const String& lacisId,
  const String& macAddress,
  const String& productType,
  const String& productCode
) {
  AraneaRegisterResult result;

  // 既に登録済みの場合はNVSから取得
  if (isRegistered()) {
    result.ok = true;
    result.cic_code = getSavedCic();
    result.stateEndpoint = getSavedStateEndpoint();
    Serial.println("[ARANEA] Already registered, using saved CIC");
    return result;
  }

  // JSON構築
  StaticJsonDocument<1024> doc;

  // lacisOath
  JsonObject lacisOath = doc.createNestedObject("lacisOath");
  lacisOath["lacisId"] = tenantAuth_.lacisId;
  lacisOath["userId"] = tenantAuth_.userId;
  lacisOath["pass"] = tenantAuth_.pass;
  lacisOath["cic"] = tenantAuth_.cic;
  lacisOath["method"] = "register";

  // userObject
  JsonObject userObject = doc.createNestedObject("userObject");
  userObject["lacisID"] = lacisId;
  userObject["tid"] = tid;
  userObject["typeDomain"] = "araneaDevice";
  userObject["type"] = deviceType;

  // deviceMeta
  JsonObject deviceMeta = doc.createNestedObject("deviceMeta");
  deviceMeta["macAddress"] = macAddress;
  deviceMeta["productType"] = productType;
  deviceMeta["productCode"] = productCode;

  String jsonPayload;
  serializeJson(doc, jsonPayload);

  Serial.println("[ARANEA] Registering device...");
  Serial.printf("[ARANEA] URL: %s\n", gateUrl_.c_str());

  // HTTP POST
  HTTPClient http;
  http.begin(gateUrl_);
  http.addHeader("Content-Type", "application/json");
  http.setTimeout(15000);

  int httpCode = http.POST(jsonPayload);

  if (httpCode <= 0) {
    result.error = "HTTP error: " + String(httpCode);
    Serial.printf("[ARANEA] HTTP error: %d\n", httpCode);
    http.end();
    return result;
  }

  String response = http.getString();
  http.end();

  Serial.printf("[ARANEA] Response code: %d\n", httpCode);

  // レスポンス解析
  StaticJsonDocument<1024> resDoc;
  DeserializationError jsonErr = deserializeJson(resDoc, response);

  if (jsonErr) {
    result.error = "JSON parse error: " + String(jsonErr.c_str());
    Serial.printf("[ARANEA] JSON error: %s\n", jsonErr.c_str());
    return result;
  }

  if (httpCode == 201 || httpCode == 200) {
    bool ok = resDoc["ok"] | false;
    if (ok) {
      result.ok = true;
      result.cic_code = resDoc["userObject"]["cic_code"].as<String>();
      result.stateEndpoint = resDoc["stateEndpoint"].as<String>();

      // NVSに保存
      Preferences prefs;
      prefs.begin(NVS_NAMESPACE, false);
      prefs.putString(KEY_CIC, result.cic_code);
      prefs.putString(KEY_STATE_ENDPOINT, result.stateEndpoint);
      prefs.putBool(KEY_REGISTERED, true);
      prefs.end();

      Serial.printf("[ARANEA] Registered! CIC: %s\n", result.cic_code.c_str());
      Serial.printf("[ARANEA] State endpoint: %s\n", result.stateEndpoint.c_str());
    } else {
      result.error = resDoc["error"].as<String>();
      Serial.printf("[ARANEA] Registration failed: %s\n", result.error.c_str());
    }
  } else if (httpCode == 409) {
    // 既に登録済み（MAC重複）
    result.error = "Device already registered (409)";
    Serial.println("[ARANEA] Device already registered (conflict)");
  } else {
    result.error = resDoc["error"].as<String>();
    Serial.printf("[ARANEA] Error %d: %s\n", httpCode, result.error.c_str());
  }

  return result;
}

bool AraneaRegister::isRegistered() {
  Preferences prefs;
  prefs.begin(NVS_NAMESPACE, true);
  bool registered = prefs.getBool(KEY_REGISTERED, false);
  prefs.end();
  return registered;
}

String AraneaRegister::getSavedCic() {
  Preferences prefs;
  prefs.begin(NVS_NAMESPACE, true);
  String cic = prefs.getString(KEY_CIC, "");
  prefs.end();
  return cic;
}

String AraneaRegister::getSavedStateEndpoint() {
  Preferences prefs;
  prefs.begin(NVS_NAMESPACE, true);
  String endpoint = prefs.getString(KEY_STATE_ENDPOINT, "");
  prefs.end();
  return endpoint;
}
