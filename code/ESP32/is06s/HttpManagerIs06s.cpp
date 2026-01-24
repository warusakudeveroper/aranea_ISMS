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
  html += "<div class=\"tab\" data-tab=\"pincontrol\" onclick=\"showTab('pincontrol')\">PIN Control</div>";
  html += "<div class=\"tab\" data-tab=\"pinsetting\" onclick=\"showTab('pinsetting')\">PIN Settings</div>";
  return html;
}

// ============================================================
// 固有タブコンテンツ生成
// ============================================================
String HttpManagerIs06s::generateTypeSpecificTabContents() {
  String html = "";
  html += "<div id=\"tab-pincontrol\" class=\"tab-content\">";
  html += "<div class=\"card\"><div class=\"card-title\">PIN Control</div>";
  html += "<div class=\"pin-grid\" id=\"pin-control-grid\"></div></div>";
  html += "</div>";
  html += "<div id=\"tab-pinsetting\" class=\"tab-content\">";
  html += "<div class=\"card\"><div class=\"card-title\">PIN Settings</div>";
  html += "<div class=\"pin-settings\" id=\"pin-settings-list\"></div>";
  html += "<div class=\"btn-group\"><button class=\"btn btn-primary\" onclick=\"savePinSettings()\">Save All Settings</button></div>";
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

    function getStateLabel(stateName, stateVal, defaultOn, defaultOff) {
      if (!stateName || stateName.length === 0) return stateVal ? defaultOn : defaultOff;
      var target = stateVal ? 'on:' : 'off:';
      for (var i = 0; i < stateName.length; i++) {
        if (stateName[i].toLowerCase().startsWith(target)) {
          return stateName[i].substring(target.length);
        }
      }
      return stateVal ? defaultOn : defaultOff;
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
        var nm = p.name || ('CH' + ch);
        var stn = p.stateName || [];
        var ctrl = '';
        if (tp === 'pwmOutput') {
          var pwmLabel = pw + '%';
          for (var j = 0; j < stn.length; j++) {
            var parts = stn[j].split(':');
            if (parts.length === 2 && parseInt(parts[0]) === pw) {
              pwmLabel = parts[1];
              break;
            }
          }
          ctrl = '<input type="range" min="0" max="100" value="' + pw + '" onchange="setPwm(' + ch + ', this.value)"' + (en ? '' : ' disabled') + '> <span id="pwm-val-' + ch + '">' + pwmLabel + '</span>';
        } else if (tp === 'digitalOutput') {
          var btnLabel = getStateLabel(stn, st, 'ON', 'OFF');
          ctrl = '<button onclick="togglePin(' + ch + ')" class="btn-toggle ' + (st ? 'on' : 'off') + '"' + (en ? '' : ' disabled') + '>' + btnLabel + '</button>';
        } else if (tp === 'digitalInput') {
          var inputLabel = getStateLabel(stn, st, 'HIGH', 'LOW');
          ctrl = '<span class="input-state">' + inputLabel + '</span>';
        }
        html += '<div class="pin-card' + (en ? '' : ' disabled') + '"><div class="pin-header"><span class="pin-name">' + nm + '</span><span class="pin-type">' + tp + '</span></div><div class="pin-control">' + ctrl + '</div><label class="switch"><input type="checkbox"' + (en ? ' checked' : '') + ' onchange="setPinEnabled(' + ch + ', this.checked)"><span class="slider"></span></label></div>';
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
        var stn = p.stateName || [];
        html += '<div class="pin-setting-card" data-ch="' + ch + '"><h4>CH' + ch + (p.name ? ' - ' + p.name : '') + '</h4>';
        html += '<div class="form-group"><label>Type</label><select id="type-' + ch + '">';
        html += '<option value="digitalOutput"' + (p.type === 'digitalOutput' ? ' selected' : '') + '>Digital Output</option>';
        html += '<option value="pwmOutput"' + (p.type === 'pwmOutput' ? ' selected' : '') + '>PWM Output</option>';
        html += '<option value="digitalInput"' + (p.type === 'digitalInput' ? ' selected' : '') + '>Digital Input</option>';
        html += '<option value="disabled"' + (p.type === 'disabled' ? ' selected' : '') + '>Disabled</option>';
        html += '</select></div>';
        html += '<div class="form-group"><label>Name</label><input type="text" id="name-' + ch + '" value="' + (p.name || '') + '" placeholder="CH' + ch + '"></div>';
        html += '<div class="form-group"><label>Mode</label><select id="mode-' + ch + '">';
        html += '<option value="Mom"' + (p.actionMode === 'Mom' ? ' selected' : '') + '>Momentary</option>';
        html += '<option value="Alt"' + (p.actionMode === 'Alt' ? ' selected' : '') + '>Alternate</option>';
        html += '<option value="Slow"' + (p.actionMode === 'Slow' ? ' selected' : '') + '>Slow (PWM)</option>';
        html += '<option value="Rapid"' + (p.actionMode === 'Rapid' ? ' selected' : '') + '>Rapid (PWM)</option>';
        html += '<option value="rotate"' + (p.actionMode === 'rotate' ? ' selected' : '') + '>Rotate (Input)</option>';
        html += '</select></div>';
        html += '<div class="form-group"><label>Validity (ms)</label><input type="number" id="validity-' + ch + '" value="' + (p.validity || 0) + '" min="0" step="100"></div>';
        html += '<div class="form-group"><label>Debounce (ms)</label><input type="number" id="debounce-' + ch + '" value="' + (p.debounce || 0) + '" min="0" step="100"></div>';
        html += '<div class="form-group"><label>Rate of Change (ms)</label><input type="number" id="rateOfChange-' + ch + '" value="' + (p.rateOfChange || 0) + '" min="0" step="100"></div>';
        html += '<div class="form-group"><label>State Names</label><input type="text" id="stateName-' + ch + '" value="' + stn.join(',') + '" placeholder="on:ON,off:OFF"></div>';
        html += '</div>';
      }
      list.innerHTML = html;
    }

    function savePinSettings() {
      var cards = document.querySelectorAll('.pin-setting-card');
      var promises = [];
      for (var i = 0; i < cards.length; i++) {
        var card = cards[i];
        var ch = card.getAttribute('data-ch');
        var stnStr = document.getElementById('stateName-' + ch).value;
        var stnArr = stnStr ? stnStr.split(',').map(function(s) { return s.trim(); }).filter(function(s) { return s; }) : [];
        var setting = {
          type: document.getElementById('type-' + ch).value,
          name: document.getElementById('name-' + ch).value,
          actionMode: document.getElementById('mode-' + ch).value,
          validity: parseInt(document.getElementById('validity-' + ch).value) || 0,
          debounce: parseInt(document.getElementById('debounce-' + ch).value) || 0,
          rateOfChange: parseInt(document.getElementById('rateOfChange-' + ch).value) || 0,
          stateName: stnArr
        };
        promises.push(
          fetch('/api/pin/' + ch + '/setting', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(setting)
          }).then(function(r) { return r.json(); })
        );
      }
      Promise.all(promises).then(function(results) {
        var allOk = results.every(function(r) { return r.ok; });
        if (allOk) {
          alert('All settings saved successfully');
        } else {
          alert('Some settings failed to save');
        }
        loadPinSettings();
      }).catch(function(e) {
        alert('Error saving settings: ' + e.message);
      });
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

  // GET/POST /api/settings
  server_->on("/api/settings", HTTP_GET, [this]() { handleSettingsGet(); });
  server_->on("/api/settings", HTTP_POST, [this]() { handleSettingsPost(); });

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

  // name設定 (P1-2)
  if (doc.containsKey("name")) {
    pinManager_->setName(ch, doc["name"].as<String>());
  }

  // actionMode設定 (P1-4)
  if (doc.containsKey("actionMode")) {
    namespace ASD = AraneaSettingsDefaults;
    String modeStr = doc["actionMode"].as<String>();
    if (modeStr == ASD::ActionMode::MOMENTARY) {
      pinManager_->setActionMode(ch, ::ActionMode::MOMENTARY);
    } else if (modeStr == ASD::ActionMode::ALTERNATE) {
      pinManager_->setActionMode(ch, ::ActionMode::ALTERNATE);
    } else if (modeStr == ASD::ActionMode::SLOW) {
      pinManager_->setActionMode(ch, ::ActionMode::SLOW);
    } else if (modeStr == ASD::ActionMode::RAPID) {
      pinManager_->setActionMode(ch, ::ActionMode::RAPID);
    } else if (modeStr == ASD::ActionMode::ROTATE) {
      pinManager_->setActionMode(ch, ::ActionMode::ROTATE);
    }
  }

  // validity設定 (P1-5)
  if (doc.containsKey("validity")) {
    pinManager_->setValidity(ch, doc["validity"].as<int>());
  }

  // debounce設定 (P1-5)
  if (doc.containsKey("debounce")) {
    pinManager_->setDebounce(ch, doc["debounce"].as<int>());
  }

  // rateOfChange設定 (P1-5)
  if (doc.containsKey("rateOfChange")) {
    pinManager_->setRateOfChange(ch, doc["rateOfChange"].as<int>());
  }

  // allocation設定 (P1-3)
  if (doc.containsKey("allocation")) {
    JsonArray allocArr = doc["allocation"].as<JsonArray>();
    String allocations[4];
    int count = 0;
    for (JsonVariant v : allocArr) {
      if (count < 4) {
        allocations[count++] = v.as<String>();
      }
    }
    pinManager_->setAllocation(ch, allocations, count);
  }

  // expiryDate設定 (P3-5)
  if (doc.containsKey("expiryDate")) {
    pinManager_->setExpiryDate(ch, doc["expiryDate"].as<String>());
  }

  // expiryEnabled設定 (P3-5)
  if (doc.containsKey("expiryEnabled")) {
    pinManager_->setExpiryEnabled(ch, doc["expiryEnabled"].as<bool>());
  }

  // stateName設定 (P1-2)
  if (doc.containsKey("stateName")) {
    JsonArray stnArr = doc["stateName"].as<JsonArray>();
    String stateNames[4];
    int count = 0;
    for (JsonVariant v : stnArr) {
      if (count < 4) {
        stateNames[count++] = v.as<String>();
      }
    }
    pinManager_->setStateName(ch, stateNames, count);
  }

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
  obj["name"] = setting.name;  // P1-2: 名前も含める

  // type to string
  const char* typeStr = "unknown";
  switch (setting.type) {
    case ::PinType::DIGITAL_OUTPUT: typeStr = ASD::PinType::DIGITAL_OUTPUT; break;
    case ::PinType::PWM_OUTPUT: typeStr = ASD::PinType::PWM_OUTPUT; break;
    case ::PinType::DIGITAL_INPUT: typeStr = ASD::PinType::DIGITAL_INPUT; break;
    case ::PinType::PIN_DISABLED: typeStr = ASD::PinType::PIN_DISABLED; break;
  }
  obj["type"] = typeStr;

  // actionMode (P1-4): 制御に必要な情報
  const char* modeStr = "unknown";
  switch (setting.actionMode) {
    case ::ActionMode::MOMENTARY: modeStr = ASD::ActionMode::MOMENTARY; break;
    case ::ActionMode::ALTERNATE: modeStr = ASD::ActionMode::ALTERNATE; break;
    case ::ActionMode::SLOW: modeStr = ASD::ActionMode::SLOW; break;
    case ::ActionMode::RAPID: modeStr = ASD::ActionMode::RAPID; break;
    case ::ActionMode::ROTATE: modeStr = ASD::ActionMode::ROTATE; break;
  }
  obj["actionMode"] = modeStr;

  // stateName (P1-2): PIN Controlでの表示用
  JsonArray stnArr = obj.createNestedArray("stateName");
  for (int i = 0; i < setting.stateNameCount; i++) {
    if (!setting.stateName[i].isEmpty()) {
      stnArr.add(setting.stateName[i]);
    }
  }
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

  // allocation (P1-3)
  JsonArray allocArr = obj.createNestedArray("allocation");
  for (int i = 0; i < setting.allocationCount; i++) {
    if (!setting.allocation[i].isEmpty()) {
      allocArr.add(setting.allocation[i]);
    }
  }

  // expiryDate (P3-5)
  obj["expiryDate"] = setting.expiryDate;
  obj["expiryEnabled"] = setting.expiryEnabled;

  // stateName (P1-2)
  JsonArray stnArr = obj.createNestedArray("stateName");
  for (int i = 0; i < setting.stateNameCount; i++) {
    if (!setting.stateName[i].isEmpty()) {
      stnArr.add(setting.stateName[i]);
    }
  }
}

void HttpManagerIs06s::buildAllPinsJson(JsonArray& arr) {
  for (int ch = 1; ch <= 6; ch++) {
    JsonObject p = arr.createNestedObject();
    buildPinStateJson(p, ch);
  }
}

// ============================================================
// Global Settings API
// ============================================================
void HttpManagerIs06s::handleSettingsGet() {
  if (!settings_) {
    sendJsonError(500, "SettingManager not initialized");
    return;
  }

  JsonDocument doc;
  doc["ok"] = true;

  JsonObject settings = doc.createNestedObject("settings");

  // Device settings
  settings["device_name"] = settings_->getString("device_name", "");
  settings["mqtt_url"] = settings_->getString("mqtt_url", "");

  // WiFi settings (read-only for security)
  JsonArray wifiList = settings.createNestedArray("wifi");
  for (int i = 1; i <= 6; i++) {
    String ssidKey = "ssid" + String(i);
    String ssid = settings_->getString(ssidKey.c_str(), "");
    if (ssid.length() > 0) {
      JsonObject wifi = wifiList.createNestedObject();
      wifi["index"] = i;
      wifi["ssid"] = ssid;
      wifi["hasPassword"] = settings_->getString(("pass" + String(i)).c_str(), "").length() > 0;
    }
  }

  // PIN global settings
  JsonObject pinGlobal = settings.createNestedObject("pinGlobal");
  pinGlobal["validity"] = settings_->getInt("g_validity", 3000);
  pinGlobal["debounce"] = settings_->getInt("g_debounce", 3000);
  pinGlobal["rateOfChange"] = settings_->getInt("g_rateOfChg", 4000);

  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}

void HttpManagerIs06s::handleSettingsPost() {
  if (!settings_) {
    sendJsonError(500, "SettingManager not initialized");
    return;
  }

  String body = server_->arg("plain");
  JsonDocument doc;
  DeserializationError err = deserializeJson(doc, body);

  if (err) {
    sendJsonError(400, "Invalid JSON");
    return;
  }

  bool changed = false;

  // Device name
  if (doc.containsKey("device_name")) {
    settings_->setString("device_name", doc["device_name"].as<String>());
    changed = true;
  }

  // MQTT URL
  if (doc.containsKey("mqtt_url")) {
    settings_->setString("mqtt_url", doc["mqtt_url"].as<String>());
    changed = true;
    Serial.printf("[Settings] mqtt_url set to: %s\n", doc["mqtt_url"].as<String>().c_str());
  }

  // WiFi credentials
  if (doc.containsKey("wifi")) {
    JsonArray wifiArr = doc["wifi"].as<JsonArray>();
    for (JsonObject wifi : wifiArr) {
      if (wifi.containsKey("index") && wifi.containsKey("ssid")) {
        int idx = wifi["index"];
        if (idx >= 1 && idx <= 6) {
          String ssidKey = "ssid" + String(idx);
          String passKey = "pass" + String(idx);
          settings_->setString(ssidKey.c_str(), wifi["ssid"].as<String>());
          if (wifi.containsKey("password")) {
            settings_->setString(passKey.c_str(), wifi["password"].as<String>());
          }
          changed = true;
        }
      }
    }
  }

  // PIN global settings
  if (doc.containsKey("pinGlobal")) {
    JsonObject pg = doc["pinGlobal"];
    if (pg.containsKey("validity")) {
      settings_->setInt("g_validity", pg["validity"]);
      changed = true;
    }
    if (pg.containsKey("debounce")) {
      settings_->setInt("g_debounce", pg["debounce"]);
      changed = true;
    }
    if (pg.containsKey("rateOfChange")) {
      settings_->setInt("g_rateOfChg", pg["rateOfChange"]);
      changed = true;
    }
  }

  if (changed) {
    // NVS auto-persists on each set operation
    sendJsonSuccess("Settings saved");
  } else {
    sendJsonSuccess("No changes");
  }
}
