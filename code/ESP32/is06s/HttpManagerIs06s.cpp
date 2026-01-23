/**
 * HttpManagerIs06s.cpp
 *
 * IS06S HTTP API & Web UI マネージャー実装
 */

#include "HttpManagerIs06s.h"

// ============================================================
// コンストラクタ/デストラクタ
// ============================================================
HttpManagerIs06s::HttpManagerIs06s() : AraneaWebUI() {
}

HttpManagerIs06s::~HttpManagerIs06s() {
}

// ============================================================
// 初期化
// ============================================================
void HttpManagerIs06s::begin(SettingManager* settings, Is06PinManager* pinManager, int port) {
  pinManager_ = pinManager;

  // 基底クラス初期化
  AraneaWebUI::begin(settings, port);

  Serial.println("HttpManagerIs06s: Initialized on port " + String(port));
}

// ============================================================
// typeSpecificステータス（PIN状態）
// ============================================================
void HttpManagerIs06s::getTypeSpecificStatus(JsonObject& obj) {
  if (!pinManager_) return;

  JsonArray pins = obj.createNestedArray("pins");
  buildAllPinsJson(pins);
}

// ============================================================
// 固有タブ生成
// ============================================================
String HttpManagerIs06s::generateTypeSpecificTabs() {
  String html = "";
  html += "<button class=\"tab-btn\" data-tab=\"pincontrol\">PIN Control</button>";
  html += "<button class=\"tab-btn\" data-tab=\"pinsetting\">PIN Settings</button>";
  return html;
}

// ============================================================
// 固有タブコンテンツ生成
// ============================================================
String HttpManagerIs06s::generateTypeSpecificTabContents() {
  String html = "";
  html += "<div id=\"pincontrol\" class=\"tab-content\">";
  html += "<h3>PIN Control</h3>";
  html += "<div class=\"pin-grid\" id=\"pin-control-grid\"></div>";
  html += "</div>";
  html += "<div id=\"pinsetting\" class=\"tab-content\">";
  html += "<h3>PIN Settings</h3>";
  html += "<div class=\"pin-settings\" id=\"pin-settings-list\"></div>";
  html += "<div class=\"form-buttons\">";
  html += "<button type=\"button\" onclick=\"savePinSettings()\" class=\"btn-primary\">Save All Settings</button>";
  html += "</div></div>";
  return html;
}

// ============================================================
// 固有JavaScript生成
// ============================================================
String HttpManagerIs06s::generateTypeSpecificJS() {
  // JavaScriptはシンプルなフォーマットで、APIベースで動作
  String js = R"JS(
    function loadPinStates() {
      fetch('/api/pin/all')
        .then(function(r) { return r.json(); })
        .then(function(data) { renderPinControls(data.pins); })
        .catch(function(e) { console.error('Load pin states error:', e); });
    }

    function renderPinControls(pins) {
      var grid = document.getElementById('pin-control-grid');
      if (!grid) return;
      var html = '';
      for (var i = 0; i < pins.length; i++) {
        var p = pins[i];
        var ch = p.channel;
        var en = p.enabled;
        var st = p.state;
        var tp = p.type;
        var pw = p.pwm || 0;
        var ctrl = '';
        if (tp === 'pwmOutput') {
          ctrl = '<input type="range" min="0" max="100" value="' + pw + '" onchange="setPwm(' + ch + ', this.value)"' + (en ? '' : ' disabled') + '> <span id="pwm-val-' + ch + '">' + pw + '%</span>';
        } else if (tp === 'digitalOutput') {
          ctrl = '<button onclick="togglePin(' + ch + ')" class="btn-toggle ' + (st ? 'on' : 'off') + '"' + (en ? '' : ' disabled') + '>' + (st ? 'ON' : 'OFF') + '</button>';
        } else if (tp === 'digitalInput') {
          ctrl = '<span class="input-state">' + (st ? 'HIGH' : 'LOW') + '</span>';
        }
        html += '<div class="pin-card' + (en ? '' : ' disabled') + '"><div class="pin-header"><span class="pin-name">CH' + ch + '</span><span class="pin-type">' + tp + '</span></div><div class="pin-control">' + ctrl + '</div><label class="switch"><input type="checkbox"' + (en ? ' checked' : '') + ' onchange="setPinEnabled(' + ch + ', this.checked)"><span class="slider"></span></label></div>';
      }
      grid.innerHTML = html;
    }

    function togglePin(ch) {
      fetch('/api/pin/' + ch + '/toggle', {method: 'POST'})
        .then(function(r) { return r.json(); })
        .then(function(data) { if (data.ok) loadPinStates(); else alert('Error: ' + data.message); });
    }

    function setPwm(ch, value) {
      fetch('/api/pin/' + ch + '/state', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({state: parseInt(value)})
      }).then(function(r) { return r.json(); })
        .then(function(data) { document.getElementById('pwm-val-' + ch).textContent = value + '%'; });
    }

    function setPinEnabled(ch, enabled) {
      fetch('/api/pin/' + ch + '/setting', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({enabled: enabled})
      }).then(function(r) { return r.json(); })
        .then(function(data) { loadPinStates(); });
    }

    function loadPinSettings() {
      fetch('/api/pin/all')
        .then(function(r) { return r.json(); })
        .then(function(data) { renderPinSettings(data.pins); });
    }

    function renderPinSettings(pins) {
      var list = document.getElementById('pin-settings-list');
      if (!list) return;
      var html = '';
      for (var i = 0; i < pins.length; i++) {
        var p = pins[i];
        var ch = p.channel;
        html += '<div class="pin-setting-card"><h4>CH' + ch + '</h4>';
        html += '<div class="form-group"><label>Type</label><select id="type-' + ch + '">';
        html += '<option value="digitalOutput"' + (p.type === 'digitalOutput' ? ' selected' : '') + '>Digital Output</option>';
        html += '<option value="pwmOutput"' + (p.type === 'pwmOutput' ? ' selected' : '') + '>PWM Output</option>';
        html += '<option value="digitalInput"' + (p.type === 'digitalInput' ? ' selected' : '') + '>Digital Input</option>';
        html += '<option value="disabled"' + (p.type === 'disabled' ? ' selected' : '') + '>Disabled</option>';
        html += '</select></div>';
        html += '<div class="form-group"><label>Name</label><input type="text" id="name-' + ch + '" value="' + (p.name || '') + '"></div>';
        html += '<div class="form-group"><label>Mode</label><select id="mode-' + ch + '">';
        html += '<option value="Mom"' + (p.actionMode === 'Mom' ? ' selected' : '') + '>Momentary</option>';
        html += '<option value="Alt"' + (p.actionMode === 'Alt' ? ' selected' : '') + '>Alternate</option>';
        html += '<option value="Slow"' + (p.actionMode === 'Slow' ? ' selected' : '') + '>Slow</option>';
        html += '<option value="Rapid"' + (p.actionMode === 'Rapid' ? ' selected' : '') + '>Rapid</option>';
        html += '</select></div>';
        html += '<div class="form-group"><label>Validity (ms)</label><input type="number" id="validity-' + ch + '" value="' + (p.validity || 3000) + '"></div>';
        html += '</div>';
      }
      list.innerHTML = html;
    }

    function savePinSettings() {
      alert('Settings saved (not implemented yet)');
    }

    document.addEventListener('tabchange', function(e) {
      if (e.detail === 'pincontrol') loadPinStates();
      if (e.detail === 'pinsetting') loadPinSettings();
    });
  )JS";
  return js;
}

// ============================================================
// 固有APIエンドポイント登録
// ============================================================
void HttpManagerIs06s::registerTypeSpecificEndpoints() {
  // GET/POST /api/pin/{ch}/state
  server_->on("/api/pin/1/state", HTTP_GET, [this]() { handlePinStateGet(); });
  server_->on("/api/pin/2/state", HTTP_GET, [this]() { handlePinStateGet(); });
  server_->on("/api/pin/3/state", HTTP_GET, [this]() { handlePinStateGet(); });
  server_->on("/api/pin/4/state", HTTP_GET, [this]() { handlePinStateGet(); });
  server_->on("/api/pin/5/state", HTTP_GET, [this]() { handlePinStateGet(); });
  server_->on("/api/pin/6/state", HTTP_GET, [this]() { handlePinStateGet(); });

  server_->on("/api/pin/1/state", HTTP_POST, [this]() { handlePinStatePost(); });
  server_->on("/api/pin/2/state", HTTP_POST, [this]() { handlePinStatePost(); });
  server_->on("/api/pin/3/state", HTTP_POST, [this]() { handlePinStatePost(); });
  server_->on("/api/pin/4/state", HTTP_POST, [this]() { handlePinStatePost(); });
  server_->on("/api/pin/5/state", HTTP_POST, [this]() { handlePinStatePost(); });
  server_->on("/api/pin/6/state", HTTP_POST, [this]() { handlePinStatePost(); });

  // GET/POST /api/pin/{ch}/setting
  server_->on("/api/pin/1/setting", HTTP_GET, [this]() { handlePinSettingGet(); });
  server_->on("/api/pin/2/setting", HTTP_GET, [this]() { handlePinSettingGet(); });
  server_->on("/api/pin/3/setting", HTTP_GET, [this]() { handlePinSettingGet(); });
  server_->on("/api/pin/4/setting", HTTP_GET, [this]() { handlePinSettingGet(); });
  server_->on("/api/pin/5/setting", HTTP_GET, [this]() { handlePinSettingGet(); });
  server_->on("/api/pin/6/setting", HTTP_GET, [this]() { handlePinSettingGet(); });

  server_->on("/api/pin/1/setting", HTTP_POST, [this]() { handlePinSettingPost(); });
  server_->on("/api/pin/2/setting", HTTP_POST, [this]() { handlePinSettingPost(); });
  server_->on("/api/pin/3/setting", HTTP_POST, [this]() { handlePinSettingPost(); });
  server_->on("/api/pin/4/setting", HTTP_POST, [this]() { handlePinSettingPost(); });
  server_->on("/api/pin/5/setting", HTTP_POST, [this]() { handlePinSettingPost(); });
  server_->on("/api/pin/6/setting", HTTP_POST, [this]() { handlePinSettingPost(); });

  // POST /api/pin/{ch}/toggle
  server_->on("/api/pin/1/toggle", HTTP_POST, [this]() { handlePinToggle(); });
  server_->on("/api/pin/2/toggle", HTTP_POST, [this]() { handlePinToggle(); });
  server_->on("/api/pin/3/toggle", HTTP_POST, [this]() { handlePinToggle(); });
  server_->on("/api/pin/4/toggle", HTTP_POST, [this]() { handlePinToggle(); });
  server_->on("/api/pin/5/toggle", HTTP_POST, [this]() { handlePinToggle(); });
  server_->on("/api/pin/6/toggle", HTTP_POST, [this]() { handlePinToggle(); });

  // GET /api/pin/all
  server_->on("/api/pin/all", HTTP_GET, [this]() { handlePinAll(); });

  Serial.println("HttpManagerIs06s: PIN API endpoints registered.");
}

// ============================================================
// 固有設定取得
// ============================================================
void HttpManagerIs06s::getTypeSpecificConfig(JsonObject& obj) {
  if (!pinManager_) return;

  JsonArray pins = obj.createNestedArray("pinSettings");
  for (int ch = 1; ch <= 6; ch++) {
    JsonObject p = pins.createNestedObject();
    buildPinSettingJson(p, ch);
  }
}

// ============================================================
// APIハンドラ実装
// ============================================================
void HttpManagerIs06s::handlePinStateGet() {
  int ch = extractChannelFromUri();
  if (ch < 1 || ch > 6) {
    sendJsonError(400, "Invalid channel");
    return;
  }

  JsonDocument doc;
  JsonObject obj = doc.to<JsonObject>();
  buildPinStateJson(obj, ch);

  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}

void HttpManagerIs06s::handlePinStatePost() {
  int ch = extractChannelFromUri();
  if (ch < 1 || ch > 6) {
    sendJsonError(400, "Invalid channel");
    return;
  }

  if (!pinManager_) {
    sendJsonError(500, "PinManager not initialized");
    return;
  }

  String body = server_->arg("plain");
  JsonDocument doc;
  DeserializationError err = deserializeJson(doc, body);

  if (err) {
    sendJsonError(400, "Invalid JSON");
    return;
  }

  int state = doc["state"] | 0;
  bool success = pinManager_->setPinState(ch, state);

  if (success) {
    sendJsonSuccess();
  } else {
    sendJsonError(400, "Command rejected (disabled or debounce)");
  }
}

void HttpManagerIs06s::handlePinSettingGet() {
  int ch = extractChannelFromUri();
  if (ch < 1 || ch > 6) {
    sendJsonError(400, "Invalid channel");
    return;
  }

  JsonDocument doc;
  JsonObject obj = doc.to<JsonObject>();
  buildPinSettingJson(obj, ch);

  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}

void HttpManagerIs06s::handlePinSettingPost() {
  int ch = extractChannelFromUri();
  if (ch < 1 || ch > 6) {
    sendJsonError(400, "Invalid channel");
    return;
  }

  if (!pinManager_) {
    sendJsonError(500, "PinManager not initialized");
    return;
  }

  String body = server_->arg("plain");
  JsonDocument doc;
  DeserializationError err = deserializeJson(doc, body);

  if (err) {
    sendJsonError(400, "Invalid JSON");
    return;
  }

  // enabled設定
  if (doc.containsKey("enabled")) {
    bool enabled = doc["enabled"];
    pinManager_->setPinEnabled(ch, enabled);
  }

  // type設定
  if (doc.containsKey("type")) {
    String typeStr = doc["type"].as<String>();
    namespace ASD = AraneaSettingsDefaults;
    if (typeStr == ASD::PinType::DIGITAL_OUTPUT) {
      pinManager_->setPinType(ch, ::PinType::DIGITAL_OUTPUT);
    } else if (typeStr == ASD::PinType::PWM_OUTPUT) {
      pinManager_->setPinType(ch, ::PinType::PWM_OUTPUT);
    } else if (typeStr == ASD::PinType::DIGITAL_INPUT) {
      pinManager_->setPinType(ch, ::PinType::DIGITAL_INPUT);
    } else if (typeStr == ASD::PinType::PIN_DISABLED) {
      pinManager_->setPinType(ch, ::PinType::PIN_DISABLED);
    }
  }

  // TODO: その他の設定 (name, actionMode, validity, etc.)

  sendJsonSuccess();
}

void HttpManagerIs06s::handlePinToggle() {
  int ch = extractChannelFromUri();
  if (ch < 1 || ch > 6) {
    sendJsonError(400, "Invalid channel");
    return;
  }

  if (!pinManager_) {
    sendJsonError(500, "PinManager not initialized");
    return;
  }

  int currentState = pinManager_->getPinState(ch);
  int newState = (currentState == 0) ? 1 : 0;
  bool success = pinManager_->setPinState(ch, newState);

  if (success) {
    JsonDocument doc;
    doc["ok"] = true;
    doc["channel"] = ch;
    doc["state"] = newState;

    String response;
    serializeJson(doc, response);
    server_->send(200, "application/json", response);
  } else {
    sendJsonError(400, "Toggle failed (disabled or debounce)");
  }
}

void HttpManagerIs06s::handlePinAll() {
  JsonDocument doc;
  JsonArray pins = doc.createNestedArray("pins");
  buildAllPinsJson(pins);

  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}

// ============================================================
// ヘルパーメソッド
// ============================================================
int HttpManagerIs06s::extractChannelFromUri() {
  String uri = server_->uri();
  // /api/pin/X/... → extract X
  int start = uri.indexOf("/api/pin/") + 9;
  int end = uri.indexOf("/", start);
  if (end < 0) end = uri.length();

  String chStr = uri.substring(start, end);
  return chStr.toInt();
}

void HttpManagerIs06s::sendJsonResponse(int code, const String& message) {
  JsonDocument doc;
  doc["ok"] = (code == 200);
  doc["message"] = message;

  String response;
  serializeJson(doc, response);
  server_->send(code, "application/json", response);
}

void HttpManagerIs06s::sendJsonSuccess(const String& message) {
  sendJsonResponse(200, message);
}

void HttpManagerIs06s::sendJsonError(int code, const String& message) {
  sendJsonResponse(code, message);
}

void HttpManagerIs06s::buildPinStateJson(JsonObject& obj, int channel) {
  if (!pinManager_) return;

  namespace ASD = AraneaSettingsDefaults;
  const PinSetting& setting = pinManager_->getPinSetting(channel);

  obj["channel"] = channel;
  obj["enabled"] = pinManager_->isPinEnabled(channel);
  obj["state"] = pinManager_->getPinState(channel);
  obj["pwm"] = pinManager_->getPwmValue(channel);

  // type to string
  const char* typeStr = "unknown";
  switch (setting.type) {
    case ::PinType::DIGITAL_OUTPUT: typeStr = ASD::PinType::DIGITAL_OUTPUT; break;
    case ::PinType::PWM_OUTPUT: typeStr = ASD::PinType::PWM_OUTPUT; break;
    case ::PinType::DIGITAL_INPUT: typeStr = ASD::PinType::DIGITAL_INPUT; break;
    case ::PinType::PIN_DISABLED: typeStr = ASD::PinType::PIN_DISABLED; break;
  }
  obj["type"] = typeStr;
}

void HttpManagerIs06s::buildPinSettingJson(JsonObject& obj, int channel) {
  if (!pinManager_) return;

  namespace ASD = AraneaSettingsDefaults;
  const PinSetting& setting = pinManager_->getPinSetting(channel);

  obj["channel"] = channel;
  obj["enabled"] = pinManager_->isPinEnabled(channel);
  obj["name"] = setting.name;
  obj["validity"] = setting.validity;
  obj["debounce"] = setting.debounce;
  obj["rateOfChange"] = setting.rateOfChange;

  // type
  const char* typeStr = "unknown";
  switch (setting.type) {
    case ::PinType::DIGITAL_OUTPUT: typeStr = ASD::PinType::DIGITAL_OUTPUT; break;
    case ::PinType::PWM_OUTPUT: typeStr = ASD::PinType::PWM_OUTPUT; break;
    case ::PinType::DIGITAL_INPUT: typeStr = ASD::PinType::DIGITAL_INPUT; break;
    case ::PinType::PIN_DISABLED: typeStr = ASD::PinType::PIN_DISABLED; break;
  }
  obj["type"] = typeStr;

  // actionMode
  const char* modeStr = "unknown";
  switch (setting.actionMode) {
    case ::ActionMode::MOMENTARY: modeStr = ASD::ActionMode::MOMENTARY; break;
    case ::ActionMode::ALTERNATE: modeStr = ASD::ActionMode::ALTERNATE; break;
    case ::ActionMode::SLOW: modeStr = ASD::ActionMode::SLOW; break;
    case ::ActionMode::RAPID: modeStr = ASD::ActionMode::RAPID; break;
    case ::ActionMode::ROTATE: modeStr = ASD::ActionMode::ROTATE; break;
  }
  obj["actionMode"] = modeStr;
}

void HttpManagerIs06s::buildAllPinsJson(JsonArray& arr) {
  for (int ch = 1; ch <= 6; ch++) {
    JsonObject p = arr.createNestedObject();
    buildPinStateJson(p, ch);
  }
}
