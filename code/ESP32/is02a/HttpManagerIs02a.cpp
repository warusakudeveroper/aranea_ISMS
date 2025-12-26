/**
 * HttpManagerIs02a.cpp
 */

#include "HttpManagerIs02a.h"
#include "BleReceiver.h"
#include "Is02aKeys.h"
#include <ArduinoJson.h>

HttpManagerIs02a::HttpManagerIs02a()
    : AraneaWebUI()
    , bleReceiver_(nullptr)
    , selfTemp_(NAN)
    , selfHum_(NAN)
    , mqttSuccessCount_(0)
    , mqttFailCount_(0)
    , relaySuccessCount_(0)
    , relayFailCount_(0)
    , lastBatchTime_("---")
{
}

void HttpManagerIs02a::begin(SettingManager* settings, BleReceiver* bleReceiver, int port) {
  bleReceiver_ = bleReceiver;

  // 基底クラス初期化
  AraneaWebUI::begin(settings, port);

  Serial.println("[HttpManagerIs02a] Initialized");
}

void HttpManagerIs02a::setSelfSensorData(float temp, float hum) {
  selfTemp_ = temp;
  selfHum_ = hum;
}

void HttpManagerIs02a::setSendStatus(int mqttSuccess, int mqttFail, int relaySuccess, int relayFail) {
  mqttSuccessCount_ = mqttSuccess;
  mqttFailCount_ = mqttFail;
  relaySuccessCount_ = relaySuccess;
  relayFailCount_ = relayFail;
}

void HttpManagerIs02a::setLastBatchTime(const String& time) {
  lastBatchTime_ = time;
}

void HttpManagerIs02a::getTypeSpecificStatus(JsonObject& obj) {
  // 自己計測データ
  obj["selfTemp"] = selfTemp_;
  obj["selfHum"] = selfHum_;

  // BLEノード数
  int nodeCount = bleReceiver_ ? bleReceiver_->getValidNodeCount() : 0;
  obj["nodeCount"] = nodeCount;
  obj["bleScanning"] = bleReceiver_ ? bleReceiver_->isScanning() : false;

  // 送信統計
  obj["mqttSuccess"] = mqttSuccessCount_;
  obj["mqttFail"] = mqttFailCount_;
  obj["relaySuccess"] = relaySuccessCount_;
  obj["relayFail"] = relayFailCount_;
  obj["lastBatchTime"] = lastBatchTime_;
}

String HttpManagerIs02a::generateTypeSpecificTabs() {
  String html;
  html += F("<div class=\"tab\" data-tab=\"nodes\" onclick=\"showTab('nodes')\">Nodes</div>");
  html += F("<div class=\"tab\" data-tab=\"sensor\" onclick=\"showTab('sensor')\">Sensor</div>");
  html += F("<div class=\"tab\" data-tab=\"mqtt\" onclick=\"showTab('mqtt')\">MQTT</div>");
  return html;
}

String HttpManagerIs02a::generateTypeSpecificTabContents() {
  String html;

  // Nodes Tab
  html += F("<div id=\"tab-nodes\" class=\"tab-content\">");
  html += F("<h3>BLE Nodes (is01a)</h3>");
  html += F("<div class=\"form-group\"><button onclick=\"clearNodes()\">Clear All Nodes</button></div>");
  html += F("<div id=\"node-list\"></div>");
  html += F("</div>");

  // Sensor Tab
  html += F("<div id=\"tab-sensor\" class=\"tab-content\">");
  html += F("<h3>Self Sensor (HT-30)</h3>");
  html += F("<div id=\"self-sensor-display\">Loading...</div>");
  html += F("<h4>Sensor Settings</h4>");
  html += F("<form id=\"sensor-form\">");
  html += F("<div class=\"form-group\"><label>Temp Offset (°C):</label>");
  html += F("<input type=\"number\" step=\"0.1\" id=\"temp-offset\"></div>");
  html += F("<div class=\"form-group\"><label>Hum Offset (%):</label>");
  html += F("<input type=\"number\" step=\"0.1\" id=\"hum-offset\"></div>");
  html += F("<div class=\"form-group\"><label>Self Measure Interval (sec):</label>");
  html += F("<input type=\"number\" id=\"self-interval\" min=\"10\" max=\"300\"></div>");
  html += F("<div class=\"form-group\"><label>Batch Send Interval (sec):</label>");
  html += F("<input type=\"number\" id=\"batch-interval\" min=\"10\" max=\"300\"></div>");
  html += F("<button type=\"submit\">Save Sensor Settings</button>");
  html += F("</form></div>");

  // MQTT Tab
  html += F("<div id=\"tab-mqtt\" class=\"tab-content\">");
  html += F("<h3>MQTT Configuration</h3>");
  html += F("<form id=\"mqtt-form\">");
  html += F("<div class=\"form-group\"><label>Broker URL:</label>");
  html += F("<input type=\"text\" id=\"mqtt-broker\" placeholder=\"mqtt.example.com\"></div>");
  html += F("<div class=\"form-group\"><label>Port:</label>");
  html += F("<input type=\"number\" id=\"mqtt-port\" value=\"8883\"></div>");
  html += F("<div class=\"form-group\"><label>Username:</label>");
  html += F("<input type=\"text\" id=\"mqtt-user\"></div>");
  html += F("<div class=\"form-group\"><label>Password:</label>");
  html += F("<input type=\"password\" id=\"mqtt-pass\"></div>");
  html += F("<div class=\"form-group\"><label><input type=\"checkbox\" id=\"mqtt-tls\"> Use TLS</label></div>");
  html += F("<button type=\"submit\">Save MQTT Settings</button>");
  html += F("</form>");
  html += F("<h4>Relay Settings</h4>");
  html += F("<form id=\"relay-form\">");
  html += F("<div class=\"form-group\"><label>Primary Relay URL:</label>");
  html += F("<input type=\"text\" id=\"relay-primary\" placeholder=\"http://192.168.96.201:8080/api/relay\"></div>");
  html += F("<div class=\"form-group\"><label>Secondary Relay URL:</label>");
  html += F("<input type=\"text\" id=\"relay-secondary\" placeholder=\"http://192.168.96.202:8080/api/relay\"></div>");
  html += F("<button type=\"submit\">Save Relay Settings</button>");
  html += F("</form></div>");

  return html;
}

String HttpManagerIs02a::generateTypeSpecificJS() {
  String js;
  js += F("<script>");

  // ノード一覧取得
  js += F("function loadNodes(){fetch('/api/nodes').then(r=>r.json()).then(data=>{");
  js += F("let html='<p>Valid nodes: '+data.count+' / Max: '+data.max+'</p>';");
  js += F("html+='<table><tr><th>LacisID</th><th>Temp</th><th>Hum</th><th>Batt</th><th>RSSI</th><th>Age</th></tr>';");
  js += F("data.nodes.forEach(n=>{let age=Math.floor((Date.now()-n.receivedAt)/1000);");
  js += F("html+='<tr><td>'+n.lacisId.substring(0,8)+'...</td><td>'+n.temperature.toFixed(1)+'°C</td>';");
  js += F("html+='<td>'+n.humidity.toFixed(1)+'%</td><td>'+n.battery+'%</td>';");
  js += F("html+='<td>'+n.rssi+' dBm</td><td>'+age+'s</td></tr>';});");
  js += F("html+='</table>';document.getElementById('node-list').innerHTML=html;});}");

  // ノードクリア
  js += F("function clearNodes(){if(!confirm('Clear all nodes?'))return;");
  js += F("fetch('/api/nodes/clear',{method:'POST'}).then(r=>r.json()).then(data=>{");
  js += F("if(data.ok){showMessage('Nodes cleared');loadNodes();}else{showMessage('Error','error');}});}");

  // センサー設定読み込み
  js += F("function loadSensorConfig(){fetch('/api/sensor/config').then(r=>r.json()).then(data=>{");
  js += F("document.getElementById('temp-offset').value=data.tempOffset||0;");
  js += F("document.getElementById('hum-offset').value=data.humOffset||0;");
  js += F("document.getElementById('self-interval').value=data.selfInterval||60;");
  js += F("document.getElementById('batch-interval').value=data.batchInterval||30;});}");

  // センサー設定保存
  js += F("document.getElementById('sensor-form').addEventListener('submit',function(e){e.preventDefault();");
  js += F("fetch('/api/sensor/config',{method:'POST',headers:{'Content-Type':'application/json'},");
  js += F("body:JSON.stringify({tempOffset:parseFloat(document.getElementById('temp-offset').value)||0,");
  js += F("humOffset:parseFloat(document.getElementById('hum-offset').value)||0,");
  js += F("selfInterval:parseInt(document.getElementById('self-interval').value)||60,");
  js += F("batchInterval:parseInt(document.getElementById('batch-interval').value)||30})})");
  js += F(".then(r=>r.json()).then(data=>{if(data.ok){showMessage('Sensor settings saved');}");
  js += F("else{showMessage('Error saving settings','error');}});});");

  // MQTT設定読み込み
  js += F("function loadMqttConfig(){fetch('/api/mqtt/config').then(r=>r.json()).then(data=>{");
  js += F("document.getElementById('mqtt-broker').value=data.broker||'';");
  js += F("document.getElementById('mqtt-port').value=data.port||8883;");
  js += F("document.getElementById('mqtt-user').value=data.user||'';");
  js += F("document.getElementById('mqtt-pass').value='';");
  js += F("document.getElementById('mqtt-tls').checked=data.useTls!==false;});}");

  // MQTT設定保存
  js += F("document.getElementById('mqtt-form').addEventListener('submit',function(e){e.preventDefault();");
  js += F("fetch('/api/mqtt/config',{method:'POST',headers:{'Content-Type':'application/json'},");
  js += F("body:JSON.stringify({broker:document.getElementById('mqtt-broker').value,");
  js += F("port:parseInt(document.getElementById('mqtt-port').value)||8883,");
  js += F("user:document.getElementById('mqtt-user').value,");
  js += F("pass:document.getElementById('mqtt-pass').value,");
  js += F("useTls:document.getElementById('mqtt-tls').checked})})");
  js += F(".then(r=>r.json()).then(data=>{if(data.ok){showMessage('MQTT settings saved');}");
  js += F("else{showMessage('Error saving settings','error');}});});");

  // リレー設定読み込み
  js += F("function loadRelayConfig(){fetch('/api/relay/config').then(r=>r.json()).then(data=>{");
  js += F("document.getElementById('relay-primary').value=data.primary||'';");
  js += F("document.getElementById('relay-secondary').value=data.secondary||'';});}");

  // リレー設定保存
  js += F("document.getElementById('relay-form').addEventListener('submit',function(e){e.preventDefault();");
  js += F("fetch('/api/relay/config',{method:'POST',headers:{'Content-Type':'application/json'},");
  js += F("body:JSON.stringify({primary:document.getElementById('relay-primary').value,");
  js += F("secondary:document.getElementById('relay-secondary').value})})");
  js += F(".then(r=>r.json()).then(data=>{if(data.ok){showMessage('Relay settings saved');}");
  js += F("else{showMessage('Error saving settings','error');}});});");

  // 自己センサー表示更新
  js += F("function updateSelfSensor(){fetch('/api/status').then(r=>r.json()).then(data=>{");
  js += F("let temp=data.selfTemp;let hum=data.selfHum;");
  js += F("let html='<p><strong>Temperature:</strong> '+(isNaN(temp)?'ERR':temp.toFixed(1)+'°C')+'</p>';");
  js += F("html+='<p><strong>Humidity:</strong> '+(isNaN(hum)?'ERR':hum.toFixed(1)+'%')+'</p>';");
  js += F("html+='<p><strong>Last Batch:</strong> '+data.lastBatchTime+'</p>';");
  js += F("document.getElementById('self-sensor-display').innerHTML=html;});}");

  // タブ切り替え時の読み込み
  js += F("var origShowTab=showTab;showTab=function(tab){origShowTab(tab);");
  js += F("if(tab==='nodes')loadNodes();if(tab==='sensor'){loadSensorConfig();updateSelfSensor();}");
  js += F("if(tab==='mqtt'){loadMqttConfig();loadRelayConfig();}};");

  // 自動更新
  js += F("setInterval(function(){var el=document.getElementById('tab-nodes');if(el&&el.classList.contains('active'))loadNodes();");
  js += F("var el2=document.getElementById('tab-sensor');if(el2&&el2.classList.contains('active'))updateSelfSensor();},3000);");

  js += F("</script>");
  return js;
}

void HttpManagerIs02a::registerTypeSpecificEndpoints() {
  // ノード一覧取得
  server_->on("/api/nodes", HTTP_GET, [this]() { handleNodes(); });

  // ノードクリア
  server_->on("/api/nodes/clear", HTTP_POST, [this]() { handleNodesClear(); });

  // センサー設定
  server_->on("/api/sensor/config", HTTP_GET, [this]() { handleSensorConfig(); });
  server_->on("/api/sensor/config", HTTP_POST, [this]() { handleSensorConfig(); });

  // MQTT設定
  server_->on("/api/mqtt/config", HTTP_GET, [this]() { handleMqttConfig(); });
  server_->on("/api/mqtt/config", HTTP_POST, [this]() { handleMqttConfig(); });

  // リレー設定
  server_->on("/api/relay/config", HTTP_GET, [this]() { handleRelayConfig(); });
  server_->on("/api/relay/config", HTTP_POST, [this]() { handleRelayConfig(); });

  Serial.println("[HttpManagerIs02a] Endpoints registered");
}

void HttpManagerIs02a::getTypeSpecificConfig(JsonObject& obj) {
  // センサー設定（floatはStringで保存）
  obj["tempOffset"] = settings_->getString(Is02aKeys::kTempOffset, "0.0").toFloat();
  obj["humOffset"] = settings_->getString(Is02aKeys::kHumOffset, "0.0").toFloat();
  obj["selfInterval"] = settings_->getInt(Is02aKeys::kSelfIntv, 60);
  obj["batchInterval"] = settings_->getInt(Is02aKeys::kBatchIntv, 30);

  // MQTT設定
  obj["mqttBroker"] = settings_->getString(Is02aKeys::kMqttBroker, "");
  obj["mqttPort"] = settings_->getInt(Is02aKeys::kMqttPort, 8883);
  obj["mqttTls"] = settings_->getBool(Is02aKeys::kMqttTls, true);

  // リレー設定
  obj["relayPrimary"] = settings_->getString(Is02aKeys::kRelayUrl, "");
  obj["relaySecondary"] = settings_->getString(Is02aKeys::kRelayUrl2, "");
}

void HttpManagerIs02a::handleNodes() {
  if (!checkAuth()) { requestAuth(); return; }

  DynamicJsonDocument doc(4096);
  doc["count"] = bleReceiver_ ? bleReceiver_->getValidNodeCount() : 0;
  doc["max"] = BLE_MAX_NODES;

  JsonArray arr = doc.createNestedArray("nodes");

  if (bleReceiver_) {
    auto nodes = bleReceiver_->getNodes();
    for (const auto& node : nodes) {
      if (!node.valid) continue;

      JsonObject obj = arr.createNestedObject();
      obj["lacisId"] = node.lacisId;
      obj["mac"] = node.mac;
      obj["temperature"] = node.temperature;
      obj["humidity"] = node.humidity;
      obj["battery"] = node.battery;
      obj["rssi"] = node.rssi;
      obj["receivedAt"] = node.receivedAt;
      obj["bootCount"] = node.bootCount;
    }
  }

  String json;
  serializeJson(doc, json);
  server_->send(200, "application/json", json);
}

void HttpManagerIs02a::handleNodesClear() {
  if (!checkAuth()) { requestAuth(); return; }

  if (bleReceiver_) {
    bleReceiver_->clearNodes();
  }

  server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs02a::handleSensorConfig() {
  if (!checkAuth()) { requestAuth(); return; }

  if (server_->method() == HTTP_GET) {
    DynamicJsonDocument doc(256);
    doc["tempOffset"] = settings_->getString(Is02aKeys::kTempOffset, "0.0").toFloat();
    doc["humOffset"] = settings_->getString(Is02aKeys::kHumOffset, "0.0").toFloat();
    doc["selfInterval"] = settings_->getInt(Is02aKeys::kSelfIntv, 60);
    doc["batchInterval"] = settings_->getInt(Is02aKeys::kBatchIntv, 30);

    String json;
    serializeJson(doc, json);
    server_->send(200, "application/json", json);
    return;
  }

  // POST
  DynamicJsonDocument doc(256);
  DeserializationError error = deserializeJson(doc, server_->arg("plain"));

  if (error) {
    server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
    return;
  }

  if (doc.containsKey("tempOffset")) {
    settings_->setString(Is02aKeys::kTempOffset, String((float)doc["tempOffset"], 1));
  }
  if (doc.containsKey("humOffset")) {
    settings_->setString(Is02aKeys::kHumOffset, String((float)doc["humOffset"], 1));
  }
  if (doc.containsKey("selfInterval")) {
    settings_->setInt(Is02aKeys::kSelfIntv, doc["selfInterval"]);
  }
  if (doc.containsKey("batchInterval")) {
    settings_->setInt(Is02aKeys::kBatchIntv, doc["batchInterval"]);
  }

  server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs02a::handleMqttConfig() {
  if (!checkAuth()) { requestAuth(); return; }

  if (server_->method() == HTTP_GET) {
    DynamicJsonDocument doc(512);
    doc["broker"] = settings_->getString(Is02aKeys::kMqttBroker, "");
    doc["port"] = settings_->getInt(Is02aKeys::kMqttPort, 8883);
    doc["user"] = settings_->getString(Is02aKeys::kMqttUser, "");
    // パスワードは返さない（セキュリティ）
    doc["useTls"] = settings_->getBool(Is02aKeys::kMqttTls, true);

    String json;
    serializeJson(doc, json);
    server_->send(200, "application/json", json);
    return;
  }

  // POST
  DynamicJsonDocument doc(512);
  DeserializationError error = deserializeJson(doc, server_->arg("plain"));

  if (error) {
    server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
    return;
  }

  if (doc.containsKey("broker")) {
    settings_->setString(Is02aKeys::kMqttBroker, doc["broker"].as<String>());
  }
  if (doc.containsKey("port")) {
    settings_->setInt(Is02aKeys::kMqttPort, doc["port"]);
  }
  if (doc.containsKey("user")) {
    settings_->setString(Is02aKeys::kMqttUser, doc["user"].as<String>());
  }
  if (doc.containsKey("pass")) {
    String pass = doc["pass"].as<String>();
    if (pass.length() > 0) {
      settings_->setString(Is02aKeys::kMqttPass, pass);
    }
  }
  if (doc.containsKey("useTls")) {
    settings_->setBool(Is02aKeys::kMqttTls, doc["useTls"]);
  }

  server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs02a::handleRelayConfig() {
  if (!checkAuth()) { requestAuth(); return; }

  if (server_->method() == HTTP_GET) {
    DynamicJsonDocument doc(512);
    doc["primary"] = settings_->getString(Is02aKeys::kRelayUrl, "");
    doc["secondary"] = settings_->getString(Is02aKeys::kRelayUrl2, "");

    String json;
    serializeJson(doc, json);
    server_->send(200, "application/json", json);
    return;
  }

  // POST
  DynamicJsonDocument doc(512);
  DeserializationError error = deserializeJson(doc, server_->arg("plain"));

  if (error) {
    server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
    return;
  }

  if (doc.containsKey("primary")) {
    settings_->setString(Is02aKeys::kRelayUrl, doc["primary"].as<String>());
  }
  if (doc.containsKey("secondary")) {
    settings_->setString(Is02aKeys::kRelayUrl2, doc["secondary"].as<String>());
  }

  server_->send(200, "application/json", "{\"ok\":true}");
}

// ========================================
// SpeedDial - is02a固有セクション生成
// ========================================
String HttpManagerIs02a::generateTypeSpecificSpeedDial() {
  String text = "";

  // [Sensor] セクション
  text += "[Sensor]\n";
  text += "tempOffset=" + settings_->getString(Is02aKeys::kTempOffset, "0.0") + "\n";
  text += "humOffset=" + settings_->getString(Is02aKeys::kHumOffset, "0.0") + "\n";
  text += "selfInterval=" + String(settings_->getInt(Is02aKeys::kSelfIntv, 60)) + "\n";
  text += "batchInterval=" + String(settings_->getInt(Is02aKeys::kBatchIntv, 30)) + "\n";
  text += "\n";

  // [MQTT] セクション
  text += "[MQTT]\n";
  text += "broker=" + settings_->getString(Is02aKeys::kMqttBroker, "") + "\n";
  text += "port=" + String(settings_->getInt(Is02aKeys::kMqttPort, 8883)) + "\n";
  text += "user=" + settings_->getString(Is02aKeys::kMqttUser, "") + "\n";
  text += "; pass= (not exported for security)\n";
  text += "tls=" + String(settings_->getBool(Is02aKeys::kMqttTls, true) ? "true" : "false") + "\n";
  text += "\n";

  // [Relay] セクション
  text += "[Relay]\n";
  text += "primary=" + settings_->getString(Is02aKeys::kRelayUrl, "") + "\n";
  text += "secondary=" + settings_->getString(Is02aKeys::kRelayUrl2, "") + "\n";

  return text;
}

// ========================================
// SpeedDial - is02a固有セクション適用
// ========================================
bool HttpManagerIs02a::applyTypeSpecificSpeedDial(const String& section, const std::vector<String>& lines) {
  // key=value をパースするラムダ
  auto parseLine = [](const String& line, String& key, String& value) -> bool {
    int eq = line.indexOf('=');
    if (eq <= 0) return false;
    key = line.substring(0, eq);
    value = line.substring(eq + 1);
    key.trim();
    value.trim();
    return true;
  };

  if (section == "Sensor") {
    for (const auto& line : lines) {
      String key, value;
      if (!parseLine(line, key, value)) continue;

      if (key == "tempOffset") {
        settings_->setString(Is02aKeys::kTempOffset, String(value.toFloat(), 1));
      } else if (key == "humOffset") {
        settings_->setString(Is02aKeys::kHumOffset, String(value.toFloat(), 1));
      } else if (key == "selfInterval") {
        settings_->setInt(Is02aKeys::kSelfIntv, value.toInt());
      } else if (key == "batchInterval") {
        settings_->setInt(Is02aKeys::kBatchIntv, value.toInt());
      }
    }
    return true;
  }

  if (section == "MQTT") {
    for (const auto& line : lines) {
      String key, value;
      if (!parseLine(line, key, value)) continue;

      if (key == "broker") {
        settings_->setString(Is02aKeys::kMqttBroker, value);
      } else if (key == "port") {
        settings_->setInt(Is02aKeys::kMqttPort, value.toInt());
      } else if (key == "user") {
        settings_->setString(Is02aKeys::kMqttUser, value);
      } else if (key == "pass" && value.length() > 0) {
        settings_->setString(Is02aKeys::kMqttPass, value);
      } else if (key == "tls") {
        settings_->setBool(Is02aKeys::kMqttTls, value == "true" || value == "1");
      }
    }
    return true;
  }

  if (section == "Relay") {
    for (const auto& line : lines) {
      String key, value;
      if (!parseLine(line, key, value)) continue;

      if (key == "primary") {
        settings_->setString(Is02aKeys::kRelayUrl, value);
      } else if (key == "secondary") {
        settings_->setString(Is02aKeys::kRelayUrl2, value);
      }
    }
    return true;
  }

  // 未知のセクションは無視（エラーにしない）
  return true;
}
