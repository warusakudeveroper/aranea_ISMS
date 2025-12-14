#include "HttpManager.h"
#include "SettingManager.h"
#include <WiFi.h>

bool HttpManager::begin(SettingManager* settings, int port) {
  if (initialized_) return true;
  if (!settings) return false;

  settings_ = settings;

  // ログインパスワード取得（設定されていれば）
  loginPass_ = settings_->getString("http_pass", "0000");

  server_ = new WebServer(port);

  // Cookieヘッダーを収集するよう設定（認証用）
  const char* headerKeys[] = {"Cookie"};
  server_->collectHeaders(headerKeys, 1);

  // ルート設定
  server_->on("/", HTTP_GET, [this]() { handleRoot(); });
  server_->on("/login", HTTP_POST, [this]() { handleLogin(); });
  server_->on("/status", HTTP_GET, [this]() { handleStatus(); });
  server_->on("/settings", HTTP_GET, [this]() { handleSettings(); });
  server_->on("/settings", HTTP_POST, [this]() { handleSettingsPost(); });
  server_->on("/wifi", HTTP_GET, [this]() { handleWifi(); });
  server_->on("/wifi", HTTP_POST, [this]() { handleWifiPost(); });
  server_->on("/tenant", HTTP_GET, [this]() { handleTenant(); });
  server_->on("/tenant", HTTP_POST, [this]() { handleTenantPost(); });
  server_->on("/is05", HTTP_GET, [this]() { handleIs05(); });
  server_->on("/is05", HTTP_POST, [this]() { handleIs05Post(); });
  server_->on("/reboot", HTTP_POST, [this]() { handleReboot(); });
  server_->on("/factory-reset", HTTP_POST, [this]() { handleFactoryReset(); });
  server_->onNotFound([this]() { handleNotFound(); });

  server_->begin();
  initialized_ = true;
  Serial.printf("[HTTP] Server started on port %d\n", port);
  return true;
}

void HttpManager::handle() {
  if (!initialized_) return;
  server_->handleClient();
}

void HttpManager::setDeviceInfo(const String& deviceType, const String& lacisId,
                                 const String& cic, const String& version) {
  deviceType_ = deviceType;
  lacisId_ = lacisId;
  cic_ = cic;
  version_ = version;
}

void HttpManager::setSensorValues(float temp, float hum) {
  tempC_ = temp;
  humPct_ = hum;
}

bool HttpManager::checkAuth() {
  if (!server_->hasHeader("Cookie")) return false;
  String cookie = server_->header("Cookie");
  return cookie.indexOf("auth=1") >= 0;
}

void HttpManager::handleRoot() {
  if (!checkAuth()) {
    server_->send(200, "text/html", generateLoginPage());
  } else {
    server_->send(200, "text/html", generateMainPage());
  }
}

void HttpManager::handleLogin() {
  String pass = server_->arg("pass");
  if (pass == loginPass_) {
    server_->sendHeader("Set-Cookie", "auth=1; Path=/");
    server_->sendHeader("Location", "/");
    server_->send(302, "text/plain", "OK");
  } else {
    server_->send(200, "text/html",
      generateLoginPage() + "<p style='color:red'>Invalid password</p>");
  }
}

void HttpManager::handleStatus() {
  server_->send(200, "application/json", generateStatusJson());
}

void HttpManager::handleSettings() {
  if (!checkAuth()) {
    server_->sendHeader("Location", "/");
    server_->send(302, "text/plain", "Unauthorized");
    return;
  }
  server_->send(200, "text/html", generateSettingsPage());
}

void HttpManager::handleSettingsPost() {
  if (!checkAuth()) {
    server_->send(401, "text/plain", "Unauthorized");
    return;
  }

  // 設定を保存
  if (server_->hasArg("relay_pri")) {
    settings_->setString("relay_pri", server_->arg("relay_pri"));
  }
  if (server_->hasArg("relay_sec")) {
    settings_->setString("relay_sec", server_->arg("relay_sec"));
  }
  if (server_->hasArg("cloud_url")) {
    settings_->setString("cloud_url", server_->arg("cloud_url"));
  }
  if (server_->hasArg("reboot_hour")) {
    settings_->setInt("reboot_hour", server_->arg("reboot_hour").toInt());
  }
  if (server_->hasArg("wifi_retry_limit")) {
    settings_->setInt("wifi_retry_limit", server_->arg("wifi_retry_limit").toInt());
  }
  if (server_->hasArg("http_pass") && server_->arg("http_pass").length() > 0) {
    String newPass = server_->arg("http_pass");
    settings_->setString("http_pass", newPass);
    loginPass_ = newPass;
  }

  server_->sendHeader("Location", "/settings?saved=1");
  server_->send(302, "text/plain", "OK");
}

void HttpManager::handleWifi() {
  if (!checkAuth()) {
    server_->sendHeader("Location", "/");
    server_->send(302, "text/plain", "Unauthorized");
    return;
  }
  server_->send(200, "text/html", generateWifiPage());
}

void HttpManager::handleWifiPost() {
  if (!checkAuth()) {
    server_->send(401, "text/plain", "Unauthorized");
    return;
  }

  // WiFi設定を保存（最大6つのSSID/パスワードペア）
  for (int i = 1; i <= 6; i++) {
    String ssidKey = "wifi_ssid" + String(i);
    String passKey = "wifi_pass" + String(i);
    if (server_->hasArg(ssidKey)) {
      settings_->setString(ssidKey.c_str(), server_->arg(ssidKey));
    }
    if (server_->hasArg(passKey)) {
      settings_->setString(passKey.c_str(), server_->arg(passKey));
    }
  }

  server_->sendHeader("Location", "/wifi?saved=1");
  server_->send(302, "text/plain", "OK");
}

void HttpManager::handleTenant() {
  if (!checkAuth()) {
    server_->sendHeader("Location", "/");
    server_->send(302, "text/plain", "Unauthorized");
    return;
  }
  server_->send(200, "text/html", generateTenantPage());
}

void HttpManager::handleTenantPost() {
  if (!checkAuth()) {
    server_->send(401, "text/plain", "Unauthorized");
    return;
  }

  // テナント設定を保存
  if (server_->hasArg("tid")) {
    settings_->setString("tid", server_->arg("tid"));
  }
  if (server_->hasArg("tenant_lacisid")) {
    settings_->setString("tenant_lacisid", server_->arg("tenant_lacisid"));
  }
  if (server_->hasArg("tenant_email")) {
    settings_->setString("tenant_email", server_->arg("tenant_email"));
  }
  if (server_->hasArg("tenant_cic")) {
    settings_->setString("tenant_cic", server_->arg("tenant_cic"));
  }
  if (server_->hasArg("tenant_pass")) {
    settings_->setString("tenant_pass", server_->arg("tenant_pass"));
  }
  if (server_->hasArg("gate_url")) {
    settings_->setString("gate_url", server_->arg("gate_url"));
  }

  server_->sendHeader("Location", "/tenant?saved=1");
  server_->send(302, "text/plain", "OK");
}

void HttpManager::handleIs05() {
  if (!checkAuth()) {
    server_->sendHeader("Location", "/");
    server_->send(302, "text/plain", "Unauthorized");
    return;
  }
  server_->send(200, "text/html", generateIs05Page());
}

void HttpManager::handleIs05Post() {
  if (!checkAuth()) {
    server_->send(401, "text/plain", "Unauthorized");
    return;
  }

  // is05チャンネル設定を保存（ch1〜ch8）
  for (int i = 1; i <= 8; i++) {
    String pinKey = "is05_ch" + String(i) + "_pin";
    String nameKey = "is05_ch" + String(i) + "_name";
    String meaningKey = "is05_ch" + String(i) + "_meaning";
    String didKey = "is05_ch" + String(i) + "_did";

    if (server_->hasArg(pinKey)) {
      settings_->setInt(pinKey, server_->arg(pinKey).toInt());
    }
    if (server_->hasArg(nameKey)) {
      settings_->setString(nameKey, server_->arg(nameKey));
    }
    if (server_->hasArg(meaningKey)) {
      settings_->setString(meaningKey, server_->arg(meaningKey));
    }
    if (server_->hasArg(didKey)) {
      // 8桁ゼロ埋め
      String did = server_->arg(didKey);
      while (did.length() < 8) {
        did = "0" + did;
      }
      if (did.length() > 8) {
        did = did.substring(did.length() - 8);
      }
      settings_->setString(didKey, did);
    }
  }

  server_->sendHeader("Location", "/is05?saved=1");
  server_->send(302, "text/plain", "OK");
}

void HttpManager::handleReboot() {
  if (!checkAuth()) {
    server_->send(401, "text/plain", "Unauthorized");
    return;
  }
  server_->send(200, "text/html",
    "<!DOCTYPE html><html><head><meta charset='UTF-8'>"
    "<meta http-equiv='refresh' content='10;url=/'></head>"
    "<body style='background:#1a1a2e;color:#fff;font-family:sans-serif;text-align:center;padding:50px'>"
    "<h1>Rebooting...</h1><p>Please wait 10 seconds.</p></body></html>");
  delay(500);
  ESP.restart();
}

void HttpManager::handleFactoryReset() {
  if (!checkAuth()) {
    server_->send(401, "text/plain", "Unauthorized");
    return;
  }

  String resetPass = server_->arg("reset_pass");
  if (resetPass != FACTORY_RESET_PASS) {
    server_->send(200, "text/html",
      "<!DOCTYPE html><html><head><meta charset='UTF-8'>"
      "<meta http-equiv='refresh' content='3;url=/settings'></head>"
      "<body style='background:#1a1a2e;color:#fff;font-family:sans-serif;text-align:center;padding:50px'>"
      "<h1 style='color:#e94560'>Invalid Password</h1>"
      "<p>Factory reset password is incorrect.</p></body></html>");
    return;
  }

  // 設定をクリア
  settings_->clear();

  server_->send(200, "text/html",
    "<!DOCTYPE html><html><head><meta charset='UTF-8'>"
    "<meta http-equiv='refresh' content='10;url=/'></head>"
    "<body style='background:#1a1a2e;color:#fff;font-family:sans-serif;text-align:center;padding:50px'>"
    "<h1 style='color:#e94560'>Factory Reset Complete</h1>"
    "<p>Device will reboot in 10 seconds...</p></body></html>");
  delay(500);
  ESP.restart();
}

void HttpManager::handleNotFound() {
  server_->send(404, "text/plain", "Not Found");
}

String HttpManager::generateLoginPage() {
  String html = R"(<!DOCTYPE html>
<html><head>
<meta charset='UTF-8'>
<meta name='viewport' content='width=device-width,initial-scale=1'>
<title>ISMS Device Login</title>
<style>
body{font-family:sans-serif;background:#1a1a2e;color:#fff;display:flex;justify-content:center;align-items:center;height:100vh;margin:0}
.login{background:#16213e;padding:40px;border-radius:10px;text-align:center}
input{padding:10px;margin:10px;border:none;border-radius:5px;font-size:16px}
button{padding:10px 30px;background:#e94560;color:#fff;border:none;border-radius:5px;cursor:pointer;font-size:16px}
button:hover{background:#ff6b6b}
h1{color:#e94560}
.info{font-size:12px;color:#888;margin:15px 0}
.cic{font-size:24px;color:#e94560;font-weight:bold;margin:10px 0}
</style>
</head><body>
<div class='login'>
<h1>ISMS Device</h1>
<div class='cic'>CIC: )";
  html += cic_.length() > 0 ? cic_ : "------";
  html += R"(</div>
<div class='info'>)";
  html += lacisId_;
  html += R"(</div>
<div class='info'>IP: )";
  html += WiFi.localIP().toString();
  html += R"(</div>
<form method='POST' action='/login'>
<input type='password' name='pass' placeholder='Password' required><br>
<button type='submit'>Login</button>
</form>
</div>
</body></html>)";
  return html;
}

String HttpManager::generateMainPage() {
  String tempStr = isnan(tempC_) ? "N/A" : String(tempC_, 1) + " C";
  String humStr = isnan(humPct_) ? "N/A" : String(humPct_, 0) + " %";

  String html = R"(<!DOCTYPE html>
<html><head>
<meta charset='UTF-8'>
<meta name='viewport' content='width=device-width,initial-scale=1'>
<title>ISMS Device</title>
<style>
body{font-family:sans-serif;background:#1a1a2e;color:#fff;margin:0;padding:20px}
.container{max-width:600px;margin:0 auto}
.card{background:#16213e;padding:20px;border-radius:10px;margin:10px 0}
h1{color:#e94560;text-align:center}
.info{display:flex;justify-content:space-between;padding:10px 0;border-bottom:1px solid #333}
.label{color:#888}
.big{font-size:32px;text-align:center;color:#e94560}
.nav{text-align:center;margin:20px 0}
.nav a{color:#e94560;margin:0 10px;text-decoration:none}
.nav a:hover{text-decoration:underline}
button{padding:10px 20px;background:#e94560;color:#fff;border:none;border-radius:5px;cursor:pointer}
button:hover{background:#ff6b6b}
</style>
</head><body>
<div class='container'>
<h1>)";
  html += deviceType_;
  html += R"(</h1>
<div class='card'>
<div class='big'>CIC: )";
  html += cic_;
  html += R"(</div>
</div>
<div class='card'>
<div class='info'><span class='label'>Temperature</span><span>)";
  html += tempStr;
  html += R"(</span></div>
<div class='info'><span class='label'>Humidity</span><span>)";
  html += humStr;
  html += R"(</span></div>
<div class='info'><span class='label'>LacisID</span><span style='font-size:12px'>)";
  html += lacisId_;
  html += R"(</span></div>
<div class='info'><span class='label'>Version</span><span>)";
  html += version_;
  html += R"(</span></div>
<div class='info'><span class='label'>Uptime</span><span>)";
  html += String(millis() / 1000 / 60);
  html += R"( min</span></div>
<div class='info'><span class='label'>Free Heap</span><span>)";
  html += String(ESP.getFreeHeap() / 1024);
  html += R"( KB</span></div>
</div>
<div class='nav'>
<a href='/settings'>Settings</a>
<a href='/wifi'>WiFi</a>
<a href='/tenant'>Tenant</a>
<a href='/is05'>is05</a>
<a href='/status'>API</a>
</div>
<div class='card' style='text-align:center'>
<form method='POST' action='/reboot' onsubmit='return confirm("Reboot device?")'>
<button type='submit'>Reboot Device</button>
</form>
</div>
</div>
</body></html>)";
  return html;
}

String HttpManager::generateSettingsPage() {
  String relayPri = settings_->getString("relay_pri", "");
  String relaySec = settings_->getString("relay_sec", "");
  String cloudUrl = settings_->getString("cloud_url", "");
  int rebootHour = settings_->getInt("reboot_hour", -1);
  int wifiRetryLimit = settings_->getInt("wifi_retry_limit", 30);
  String saved = server_->arg("saved") == "1" ? "<p style='color:#4caf50'>Settings saved!</p>" : "";

  String html = R"(<!DOCTYPE html>
<html><head>
<meta charset='UTF-8'>
<meta name='viewport' content='width=device-width,initial-scale=1'>
<title>Settings</title>
<style>
body{font-family:sans-serif;background:#1a1a2e;color:#fff;margin:0;padding:20px}
.container{max-width:600px;margin:0 auto}
.card{background:#16213e;padding:20px;border-radius:10px;margin:10px 0}
h1,h3{color:#e94560}
h1{text-align:center}
label{display:block;margin:15px 0 5px;color:#888}
input,select{width:100%;padding:10px;border:none;border-radius:5px;box-sizing:border-box;font-size:14px}
button{padding:10px 30px;background:#e94560;color:#fff;border:none;border-radius:5px;cursor:pointer;margin-top:20px}
button:hover{background:#ff6b6b}
.btn-danger{background:#c62828}
.btn-danger:hover{background:#e53935}
a{color:#e94560}
.hint{font-size:12px;color:#666;margin-top:5px}
.nav{text-align:center;margin:20px 0}
.nav a{margin:0 10px}
</style>
</head><body>
<div class='container'>
<h1>Settings</h1>
)";
  html += saved;
  html += R"(
<form method='POST' action='/settings'>
<div class='card'>
<h3>Relay Endpoints</h3>
<label>Primary URL</label>
<input type='text' name='relay_pri' value=')";
  html += relayPri;
  html += R"('>
<label>Secondary URL</label>
<input type='text' name='relay_sec' value=')";
  html += relaySec;
  html += R"('>
<label>Cloud URL (optional)</label>
<input type='text' name='cloud_url' value=')";
  html += cloudUrl;
  html += R"('>
<p class='hint'>Send to cloud endpoint in addition to local relays</p>
</div>
<div class='card'>
<h3>Scheduled Reboot</h3>
<label>Reboot Hour (0-23, -1 to disable)</label>
<input type='number' name='reboot_hour' min='-1' max='23' value=')";
  html += String(rebootHour);
  html += R"('>
<p class='hint'>Device will reboot daily at the specified hour</p>
</div>
<div class='card'>
<h3>WiFi Recovery</h3>
<label>WiFi Retry Limit (reboot after N failures)</label>
<input type='number' name='wifi_retry_limit' min='5' max='100' value=')";
  html += String(wifiRetryLimit);
  html += R"('>
<p class='hint'>Device will reboot after this many WiFi connection failures</p>
</div>
<div class='card'>
<h3>Security</h3>
<label>New Login Password (leave empty to keep current)</label>
<input type='password' name='http_pass' placeholder='New password'>
</div>
<div style='text-align:center'>
<button type='submit'>Save Settings</button>
</div>
</form>

<div class='card'>
<h3 style='color:#c62828'>Factory Reset</h3>
<p class='hint'>This will clear all settings and reboot the device.</p>
<form method='POST' action='/factory-reset' onsubmit='return confirm("Are you sure? All settings will be lost!")'>
<label>Factory Reset Password</label>
<input type='password' name='reset_pass' placeholder='Enter factory reset password' required>
<div style='text-align:center'>
<button type='submit' class='btn-danger'>Factory Reset</button>
</div>
</form>
</div>

<div class='nav'>
<a href='/'>Home</a>
<a href='/wifi'>WiFi</a>
<a href='/tenant'>Tenant</a>
</div>
</div>
</body></html>)";
  return html;
}

String HttpManager::generateWifiPage() {
  String saved = server_->arg("saved") == "1" ? "<p style='color:#4caf50'>WiFi settings saved! Reboot to apply.</p>" : "";

  String html = R"(<!DOCTYPE html>
<html><head>
<meta charset='UTF-8'>
<meta name='viewport' content='width=device-width,initial-scale=1'>
<title>WiFi Settings</title>
<style>
body{font-family:sans-serif;background:#1a1a2e;color:#fff;margin:0;padding:20px}
.container{max-width:600px;margin:0 auto}
.card{background:#16213e;padding:20px;border-radius:10px;margin:10px 0}
h1,h3{color:#e94560}
h1{text-align:center}
label{display:block;margin:15px 0 5px;color:#888}
input{width:100%;padding:10px;border:none;border-radius:5px;box-sizing:border-box;font-size:14px}
button{padding:10px 30px;background:#e94560;color:#fff;border:none;border-radius:5px;cursor:pointer;margin-top:20px}
button:hover{background:#ff6b6b}
a{color:#e94560}
.hint{font-size:12px;color:#666;margin-top:5px}
.nav{text-align:center;margin:20px 0}
.nav a{margin:0 10px}
.wifi-pair{display:grid;grid-template-columns:1fr 1fr;gap:10px;margin:10px 0}
.current{background:#0f3460;padding:10px;border-radius:5px;margin-bottom:15px}
</style>
</head><body>
<div class='container'>
<h1>WiFi Settings</h1>
)";
  html += saved;
  html += R"(
<div class='card'>
<h3>Current Connection</h3>
<div class='current'>
<div>SSID: <strong>)";
  html += WiFi.SSID();
  html += R"(</strong></div>
<div>IP: <strong>)";
  html += WiFi.localIP().toString();
  html += R"(</strong></div>
<div>RSSI: <strong>)";
  html += String(WiFi.RSSI());
  html += R"( dBm</strong></div>
</div>
</div>

<form method='POST' action='/wifi'>
<div class='card'>
<h3>WiFi Networks (Priority Order)</h3>
<p class='hint'>Device will try these networks in order until connected.</p>
)";

  // WiFi設定フォーム（6つのSSID/パスワードペア）
  for (int i = 1; i <= 6; i++) {
    String ssidKey = "wifi_ssid" + String(i);
    String passKey = "wifi_pass" + String(i);
    String ssid = settings_->getString(ssidKey.c_str(), i <= 6 ? "cluster" + String(i) : "");
    String pass = settings_->getString(passKey.c_str(), "isms12345@");

    html += "<label>Network " + String(i) + "</label>";
    html += "<div class='wifi-pair'>";
    html += "<input type='text' name='" + ssidKey + "' placeholder='SSID' value='" + ssid + "'>";
    html += "<input type='password' name='" + passKey + "' placeholder='Password' value='" + pass + "'>";
    html += "</div>";
  }

  html += R"(
</div>
<div style='text-align:center'>
<button type='submit'>Save WiFi Settings</button>
</div>
</form>

<div class='nav'>
<a href='/'>Home</a>
<a href='/settings'>Settings</a>
<a href='/tenant'>Tenant</a>
</div>
</div>
</body></html>)";
  return html;
}

String HttpManager::generateTenantPage() {
  String tid = settings_->getString("tid", "");
  String tenantLacisId = settings_->getString("tenant_lacisid", "");
  String tenantEmail = settings_->getString("tenant_email", "");
  String tenantCic = settings_->getString("tenant_cic", "");
  String tenantPass = settings_->getString("tenant_pass", "");
  String gateUrl = settings_->getString("gate_url", "");
  String saved = server_->arg("saved") == "1" ? "<p style='color:#4caf50'>Tenant settings saved! Reboot to apply.</p>" : "";

  String html = R"(<!DOCTYPE html>
<html><head>
<meta charset='UTF-8'>
<meta name='viewport' content='width=device-width,initial-scale=1'>
<title>Tenant Settings</title>
<style>
body{font-family:sans-serif;background:#1a1a2e;color:#fff;margin:0;padding:20px}
.container{max-width:600px;margin:0 auto}
.card{background:#16213e;padding:20px;border-radius:10px;margin:10px 0}
h1,h3{color:#e94560}
h1{text-align:center}
label{display:block;margin:15px 0 5px;color:#888}
input{width:100%;padding:10px;border:none;border-radius:5px;box-sizing:border-box;font-size:14px}
button{padding:10px 30px;background:#e94560;color:#fff;border:none;border-radius:5px;cursor:pointer;margin-top:20px}
button:hover{background:#ff6b6b}
a{color:#e94560}
.hint{font-size:12px;color:#666;margin-top:5px}
.nav{text-align:center;margin:20px 0}
.nav a{margin:0 10px}
</style>
</head><body>
<div class='container'>
<h1>Tenant Settings</h1>
)";
  html += saved;
  html += R"(
<form method='POST' action='/tenant'>
<div class='card'>
<h3>Tenant Information</h3>
<label>Tenant ID (TID)</label>
<input type='text' name='tid' value=')";
  html += tid;
  html += R"('>
<label>Tenant LacisID</label>
<input type='text' name='tenant_lacisid' value=')";
  html += tenantLacisId;
  html += R"('>
<label>Tenant Email</label>
<input type='email' name='tenant_email' value=')";
  html += tenantEmail;
  html += R"('>
<label>Tenant CIC</label>
<input type='text' name='tenant_cic' value=')";
  html += tenantCic;
  html += R"('>
<label>Tenant Password</label>
<input type='password' name='tenant_pass' value=')";
  html += tenantPass;
  html += R"('>
</div>
<div class='card'>
<h3>Cloud Endpoint</h3>
<label>Device Gate URL</label>
<input type='url' name='gate_url' value=')";
  html += gateUrl;
  html += R"('>
<p class='hint'>araneaDeviceGate endpoint URL</p>
</div>
<div style='text-align:center'>
<button type='submit'>Save Tenant Settings</button>
</div>
</form>

<div class='nav'>
<a href='/'>Home</a>
<a href='/settings'>Settings</a>
<a href='/wifi'>WiFi</a>
</div>
</div>
</body></html>)";
  return html;
}

String HttpManager::generateIs05Page() {
  String saved = server_->arg("saved") == "1" ? "<p style='color:#4caf50'>is05 settings saved!</p>" : "";

  String html = R"(<!DOCTYPE html>
<html><head>
<meta charset='UTF-8'>
<meta name='viewport' content='width=device-width,initial-scale=1'>
<title>is05 Channel Settings</title>
<style>
body{font-family:sans-serif;background:#1a1a2e;color:#fff;margin:0;padding:20px}
.container{max-width:800px;margin:0 auto}
.card{background:#16213e;padding:20px;border-radius:10px;margin:10px 0}
h1,h3{color:#e94560}
h1{text-align:center}
table{width:100%;border-collapse:collapse;margin:15px 0}
th,td{padding:10px;text-align:left;border-bottom:1px solid #333}
th{color:#888;font-weight:normal}
input,select{padding:8px;border:none;border-radius:5px;font-size:14px;background:#fff;color:#000}
input[type='number']{width:60px}
input[type='text']{width:100px}
select{width:80px}
button{padding:10px 30px;background:#e94560;color:#fff;border:none;border-radius:5px;cursor:pointer;margin-top:20px}
button:hover{background:#ff6b6b}
a{color:#e94560}
.nav{text-align:center;margin:20px 0}
.nav a{margin:0 10px}
.hint{font-size:12px;color:#666;margin-top:5px}
</style>
</head><body>
<div class='container'>
<h1>is05 Channel Settings</h1>
)";
  html += saved;
  html += R"(
<form method='POST' action='/is05'>
<div class='card'>
<h3>8ch Reed Switch Configuration</h3>
<p class='hint'>Configure GPIO pins, names, meaning (open/close when active), and device IDs for each channel.</p>
<table>
<tr><th>CH</th><th>GPIO</th><th>Name</th><th>Meaning</th><th>DID (8 digits)</th></tr>
)";

  // チャンネル設定フォーム（ch1〜ch8）
  for (int i = 1; i <= 8; i++) {
    String pinKey = "is05_ch" + String(i) + "_pin";
    String nameKey = "is05_ch" + String(i) + "_name";
    String meaningKey = "is05_ch" + String(i) + "_meaning";
    String didKey = "is05_ch" + String(i) + "_did";

    int pin = settings_->getInt(pinKey, 0);
    String name = settings_->getString(nameKey, "ch" + String(i));
    String meaning = settings_->getString(meaningKey, "open");
    String did = settings_->getString(didKey, "00000000");

    html += "<tr>";
    html += "<td><strong>" + String(i) + "</strong></td>";
    html += "<td><input type='number' name='" + pinKey + "' value='" + String(pin) + "' min='0' max='39'></td>";
    html += "<td><input type='text' name='" + nameKey + "' value='" + name + "' maxlength='16'></td>";
    html += "<td><select name='" + meaningKey + "'>";
    html += "<option value='open'" + String(meaning == "open" ? " selected" : "") + ">open</option>";
    html += "<option value='close'" + String(meaning == "close" ? " selected" : "") + ">close</option>";
    html += "</select></td>";
    html += "<td><input type='text' name='" + didKey + "' value='" + did + "' maxlength='8' pattern='[0-9]{1,8}'></td>";
    html += "</tr>";
  }

  html += R"(
</table>
</div>
<div style='text-align:center'>
<button type='submit'>Save Channel Settings</button>
</div>
</form>

<div class='nav'>
<a href='/'>Home</a>
<a href='/settings'>Settings</a>
<a href='/wifi'>WiFi</a>
<a href='/tenant'>Tenant</a>
</div>
</div>
</body></html>)";
  return html;
}

String HttpManager::generateStatusJson() {
  String json = "{";
  json += "\"deviceType\":\"" + deviceType_ + "\",";
  json += "\"lacisId\":\"" + lacisId_ + "\",";
  json += "\"cic\":\"" + cic_ + "\",";
  json += "\"version\":\"" + version_ + "\",";
  json += "\"uptime\":" + String(millis() / 1000) + ",";
  json += "\"freeHeap\":" + String(ESP.getFreeHeap()) + ",";
  json += "\"wifiRssi\":" + String(WiFi.RSSI()) + ",";
  if (!isnan(tempC_)) {
    json += "\"temperature\":" + String(tempC_, 2) + ",";
  }
  if (!isnan(humPct_)) {
    json += "\"humidity\":" + String(humPct_, 2) + ",";
  }
  if (statusCallback_) {
    json += "\"custom\":" + statusCallback_() + ",";
  }
  // 末尾のカンマを削除
  if (json.endsWith(",")) {
    json = json.substring(0, json.length() - 1);
  }
  json += "}";
  return json;
}
