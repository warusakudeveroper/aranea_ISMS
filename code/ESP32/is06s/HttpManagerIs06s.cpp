/**
 * HttpManagerIs06s.cpp
 *
 * IS06S HTTP API & Web UI マネージャー実装
 */

#include "HttpManagerIs06s.h"
#include <driver/gpio.h>  // ESP-IDF GPIO for handleDebugGpio()

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
// クラウド接続状態取得
// ============================================================
AraneaCloudStatus HttpManagerIs06s::getCloudStatus() {
  // 基底クラスのステータスを取得
  AraneaCloudStatus status = AraneaWebUI::getCloudStatus();

  // MQTT接続状態をコールバックから取得
  if (mqttStatusCallback_) {
    status.mqttConnected = mqttStatusCallback_();
  }

  return status;
}

// ============================================================
// typeSpecificステータス（PIN状態）
// ============================================================
void HttpManagerIs06s::getTypeSpecificStatus(JsonObject& obj) {
  if (!pinManager_) return;

  JsonArray pins = obj.createNestedArray("pins");
  buildAllPinsJson(pins);

  // MQTTデバッグ情報（問題調査用）
  JsonObject mqttDebug = obj.createNestedObject("mqttDebug");
  mqttDebug["callbackSet"] = (mqttStatusCallback_ != nullptr);
  if (mqttEnabledCallback_) {
    mqttDebug["enabled"] = mqttEnabledCallback_();
  }
  if (mqttUrlCallback_) {
    mqttDebug["url"] = mqttUrlCallback_();
  }
  if (mqttStatusCallback_) {
    mqttDebug["isConnected"] = mqttStatusCallback_();
  }
}

// ============================================================
// 固有タブ生成
// ============================================================
String HttpManagerIs06s::generateTypeSpecificTabs() {
  String html = "";
  html += "<div class=\"tab\" data-tab=\"pincontrol\" onclick=\"showTab('pincontrol')\">PIN Control</div>";
  html += "<div class=\"tab\" data-tab=\"pinsetting\" onclick=\"showTab('pinsetting')\">PIN Settings</div>";
  html += "<div class=\"tab\" data-tab=\"devicesettings\" onclick=\"showTab('devicesettings')\">Device Settings</div>";
  return html;
}

// ============================================================
// 固有タブコンテンツ生成
// ============================================================
String HttpManagerIs06s::generateTypeSpecificTabContents() {
  String html = "";

  // DR1-08: PIN Control改善用CSS
  html += "<style>";
  html += ".pin-row{display:flex;align-items:center;padding:10px;border-bottom:1px solid #e0e0e0;gap:12px;}";
  html += ".pin-row:last-child{border-bottom:none;}";
  html += ".pin-row.disabled{opacity:0.5;}";
  html += ".pin-info{min-width:120px;display:flex;flex-direction:column;}";
  html += ".pin-ch{font-weight:bold;color:#1976d2;font-size:14px;}";
  html += ".pin-name{font-size:12px;color:#666;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:100px;}";
  html += ".pin-meta{min-width:100px;display:flex;flex-direction:column;font-size:11px;color:#888;}";
  html += ".pin-type{font-weight:500;}";
  html += ".pin-mode{color:#666;}";
  html += ".pin-control{flex:1;min-width:120px;display:flex;align-items:center;gap:8px;}";
  html += ".pin-control input[type=range]{flex:1;max-width:150px;}";
  html += ".pin-control .pwm-label{min-width:50px;text-align:right;font-size:12px;}";
  html += ".pin-enable{min-width:50px;}";
  html += ".btn-toggle{padding:8px 16px;border:none;border-radius:4px;cursor:pointer;font-size:13px;min-width:60px;}";
  html += ".btn-toggle.on{background:#4CAF50;color:#fff;}";
  html += ".btn-toggle.off{background:#9E9E9E;color:#fff;}";
  html += ".btn-toggle:disabled{cursor:not-allowed;opacity:0.6;}";
  html += ".input-state{padding:8px 12px;border-radius:4px;font-size:13px;background:#f5f5f5;}";
  html += ".input-state.high{background:#e3f2fd;color:#1976d2;}";
  html += ".input-state.low{background:#fafafa;color:#666;}";
  html += ".input-state.disabled-state{background:#eee;color:#999;}";
  html += ".pin-setting-card{border:1px solid #e0e0e0;border-radius:8px;padding:16px;margin-bottom:16px;}";
  html += ".pin-setting-card h4{margin:0 0 12px 0;color:#333;}";
  html += ".form-row{display:flex;gap:12px;flex-wrap:wrap;}";
  html += ".form-row .form-group{flex:1;min-width:120px;}";
  // T6: Allocation チェックボックス用CSS
  html += ".allocation-checkboxes{display:flex;gap:12px;flex-wrap:wrap;}";
  html += ".alloc-cb{display:flex;align-items:center;gap:4px;font-size:13px;}";
  html += ".alloc-cb input{margin:0;}";
  // PWM遷移表示用スタイル
  html += ".pwm-transition{color:#ff9800;font-size:11px;margin-left:4px;}";
  html += ".pin-control input[type=range].transitioning{opacity:0.6;pointer-events:none;}";
  // Momカウントダウン表示
  html += ".countdown{color:#ff5722;font-size:11px;font-weight:bold;}";
  html += "@media(max-width:600px){.pin-row{flex-wrap:wrap;}.pin-info,.pin-meta{min-width:80px;}.pin-control{width:100%;order:3;margin-top:8px;}}";
  html += "</style>";

  html += "<div id=\"tab-pincontrol\" class=\"tab-content\">";
  html += "<div class=\"card\"><div class=\"card-title\">PIN Control</div>";
  html += "<div id=\"pin-control-grid\"></div></div>";
  html += "</div>";
  html += "<div id=\"tab-pinsetting\" class=\"tab-content\">";
  html += "<div class=\"card\"><div class=\"card-title\">PIN Settings</div>";
  html += "<div class=\"pin-settings\" id=\"pin-settings-list\"></div>";
  html += "<div class=\"btn-group\"><button class=\"btn btn-primary\" onclick=\"savePinSettings()\">Save All Settings</button></div>";
  html += "</div></div>";

  // Device Settings タブ（rid含む）
  html += "<div id=\"tab-devicesettings\" class=\"tab-content\">";
  html += "<div class=\"card\"><div class=\"card-title\">Device Identity</div>";
  html += "<div class=\"form-group\"><label>Device Name</label><input type=\"text\" id=\"ds-device-name\" placeholder=\"例: 1F照明コントローラー\"></div>";
  html += "<div class=\"form-group\"><label>Room ID (rid)</label><input type=\"text\" id=\"ds-rid\" placeholder=\"例: villa1, 101, 松の間\"></div>";
  html += "<p style=\"font-size:12px;color:#666;margin:4px 0 12px 0\">ridはグループ/部屋識別に使用されます（mobes2.0連携）</p>";
  html += "</div>";
  html += "<div class=\"card\"><div class=\"card-title\">PIN Global Defaults</div>";
  html += "<div class=\"form-row\">";
  html += "<div class=\"form-group\"><label>Default Validity (ms)</label><input type=\"number\" id=\"ds-g-validity\" min=\"0\" step=\"100\"></div>";
  html += "<div class=\"form-group\"><label>Default Debounce (ms)</label><input type=\"number\" id=\"ds-g-debounce\" min=\"0\" step=\"100\"></div>";
  html += "<div class=\"form-group\"><label>Default RateOfChange (ms)</label><input type=\"number\" id=\"ds-g-rateofchange\" min=\"0\" step=\"100\"></div>";
  html += "</div></div>";
  html += "<div class=\"btn-group\"><button class=\"btn btn-primary\" onclick=\"saveDeviceSettings()\">Save Device Settings</button></div>";
  html += "</div>";

  return html;
}

// ============================================================
// 固有JavaScript生成
// DR1-03～DR1-09対応: WebUI改善
// ============================================================
String HttpManagerIs06s::generateTypeSpecificJS() {
  String js = R"JS(
    // DR1-04: ポーリング用変数
    var pinPollInterval = null;
    // 最後にキャッシュしたPINデータ
    var cachedPins = null;

    function loadPinStates() {
      fetch('/api/pin/all')
        .then(function(r) {
          if (!r.ok) throw new Error('HTTP ' + r.status);
          return r.json();
        })
        .then(function(data) {
          cachedPins = data.pins;
          renderPinControls(data.pins);
        })
        .catch(function(e) {
          console.error('Load pin states error:', e);
          // エラー時は画面を再構築しない（最後の状態を維持）
        });
    }

    // ポーリング制御（3秒固定）
    function startPinPolling() {
      if (!pinPollInterval) {
        pinPollInterval = setInterval(loadPinStates, 3000);
      }
    }
    function stopPinPolling() {
      if (pinPollInterval) {
        clearInterval(pinPollInterval);
        pinPollInterval = null;
      }
    }

    function escapeHtml(str) {
      if (!str) return '';
      return String(str).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;').replace(/'/g,'&#39;');
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

    // PIN Control レイアウト（整理版）
    // - disabled: 非表示
    // - digitalOutput/Input: 状態表示のみ（物理ボタン操作）
    // - pwmOutput: スライダー操作可
    // - Mom: カウントダウン表示
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
        var nm = p.name || '';
        var stn = p.stateName || [];
        var mode = p.actionMode || 'Alt';
        var validity = p.validity || 0;
        var pulseRemaining = p.pulseRemaining || 0;

        // Disabled PIN は非表示
        if (tp === 'disabled') continue;

        // タイプ表示文字列
        var typeLabel = tp === 'pwmOutput' ? 'PWM' : tp === 'digitalInput' ? 'INPUT' : 'DIGITAL';
        // モード表示文字列
        var modeLabel = mode;
        if (mode === 'Mom' && validity > 0) modeLabel = 'Mom';
        if (mode === 'Slow' || mode === 'Rapid') modeLabel = mode;

        // 制御部分の構築
        var ctrl = '';
        var roc = p.rateOfChange || 0;  // Slow遷移時間(ms)
        if (tp === 'pwmOutput') {
          // PWM: スライダー操作 + CSS遷移アニメーション
          var sliderVal = pw;
          // 遷移中のスライダーは現在値を維持
          var existingRow = document.querySelector('[data-ch=\"' + ch + '\"]');
          var isAnimating = existingRow && existingRow.dataset.animating === 'true';
          if (isAnimating) {
            var existingSlider = existingRow.querySelector('input[type=range]');
            if (existingSlider) sliderVal = parseInt(existingSlider.value);
          }
          ctrl = '<input type=\"range\" min=\"0\" max=\"100\" value=\"' + sliderVal + '\" ';
          ctrl += 'data-roc=\"' + roc + '\" ';
          ctrl += 'onchange=\"onPwmSliderChange(' + ch + ', this)\"';
          ctrl += (en && !isAnimating ? '' : ' disabled') + '>';
          ctrl += '<span class=\"pwm-label\">' + sliderVal + '%</span>';
        } else if (tp === 'digitalOutput') {
          // Digital Output: ボタン操作可能
          var btnLabel = getStateLabel(stn, st, 'ON', 'OFF');
          ctrl = '<button data-ch=\"' + ch + '\" data-mode=\"' + mode + '\" data-validity=\"' + validity + '\" onclick=\"doToggle(this)\" class=\"btn-toggle ' + (st ? 'on' : 'off') + '\"' + (en ? '' : ' disabled') + '>' + btnLabel;
          // Momモード＆ON状態時はカウントダウン表示
          if (mode === 'Mom' && st && pulseRemaining > 0) {
            var secRemain = (pulseRemaining / 1000).toFixed(1);
            ctrl += ' <span class=\"countdown\">(' + secRemain + 's)</span>';
          }
          ctrl += '</button>';
        } else if (tp === 'digitalInput') {
          // 入力状態表示のみ
          var inputLabel = getStateLabel(stn, st, 'HIGH', 'LOW');
          ctrl = '<span class=\"input-state ' + (st ? 'high' : 'low') + '\">' + inputLabel + '</span>';
        }

        html += '<div class=\"pin-row' + (en ? '' : ' disabled') + '\" data-ch=\"' + ch + '\">';
        html += '<div class=\"pin-info\"><span class=\"pin-ch\">CH' + ch + '</span>';
        if (nm) html += '<span class=\"pin-name\">' + escapeHtml(nm) + '</span>';
        html += '</div>';
        html += '<div class=\"pin-meta\"><span class=\"pin-type\">' + typeLabel + '</span>';
        if (tp === 'pwmOutput' || tp === 'digitalOutput') html += '<span class=\"pin-mode\">' + modeLabel + '</span>';
        html += '</div>';
        html += '<div class=\"pin-control\">' + ctrl + '</div>';
        html += '<div class=\"pin-enable\"><label class=\"switch\"><input type=\"checkbox\"' + (en ? ' checked' : '') + ' onchange=\"setPinEnabled(' + ch + ', this.checked)\"><span class=\"slider\"></span></label></div>';
        html += '</div>';
      }
      grid.innerHTML = html;
    }

    // ボタンクリックハンドラ（data属性から値を取得）
    function doToggle(btn) {
      var ch = parseInt(btn.dataset.ch);
      var mode = btn.dataset.mode;
      var validity = parseInt(btn.dataset.validity) || 0;
      togglePin(ch, mode, validity);
    }

    // DR1-03: モーメンタリ自動復帰
    function togglePin(ch, mode, validity) {
      fetch('/api/pin/' + ch + '/toggle', {method: 'POST'})
        .then(function(r) { return r.json(); })
        .then(function(data) {
          if (data.ok) {
            loadPinStates();
            // DR1-03: Momモードでvalidityミリ秒後に再取得
            if (mode === 'Mom' && data.state === 1 && validity > 0) {
              setTimeout(loadPinStates, validity + 100);
            }
          } else {
            alert('Error: ' + data.message);
          }
        });
    }

    function setPwm(ch, value) {
      fetch('/api/pin/' + ch + '/state', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({state: parseInt(value)})
      });
    }

    // PWMスライダー変更時：CSS遷移アニメーション + 操作禁止
    function onPwmSliderChange(ch, slider) {
      var targetVal = parseInt(slider.value);
      var roc = parseInt(slider.dataset.roc) || 0;  // rateOfChange(ms)
      var row = slider.closest('.pin-row');
      var label = row.querySelector('.pwm-label');

      // サーバーに送信
      setPwm(ch, targetVal);
      label.textContent = targetVal + '%';

      // Slowモード(roc>0)の場合はアニメーション
      if (roc > 0) {
        row.dataset.animating = 'true';
        slider.disabled = true;
        // roc後に操作可能に戻す
        setTimeout(function() {
          row.dataset.animating = 'false';
          slider.disabled = false;
        }, roc);
      }
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

    // T1: Type別Modeオプション生成関数
    function getModeOptionsForType(type, currentMode) {
      var opts = '';
      if (type === 'digitalOutput') {
        opts += '<option value="Mom"' + (currentMode === 'Mom' ? ' selected' : '') + '>Momentary</option>';
        opts += '<option value="Alt"' + (currentMode === 'Alt' ? ' selected' : '') + '>Alternate</option>';
      } else if (type === 'pwmOutput') {
        opts += '<option value="Slow"' + (currentMode === 'Slow' ? ' selected' : '') + '>Slow</option>';
        opts += '<option value="Rapid"' + (currentMode === 'Rapid' ? ' selected' : '') + '>Rapid</option>';
      } else if (type === 'digitalInput') {
        opts += '<option value="Mom"' + (currentMode === 'Mom' ? ' selected' : '') + '>Momentary</option>';
        opts += '<option value="Alt"' + (currentMode === 'Alt' ? ' selected' : '') + '>Alternate</option>';
        opts += '<option value="rotate"' + (currentMode === 'rotate' ? ' selected' : '') + '>Rotate</option>';
      }
      // disabled: no mode options
      return opts;
    }

    // T4: Allocation チェックボックス生成関数
    function getAllocationCheckboxes(ch, alloc) {
      var html = '<div class="allocation-checkboxes" id="allocation-' + ch + '">';
      for (var j = 1; j <= 4; j++) {
        var chk = (alloc.indexOf('CH' + j) >= 0) ? ' checked' : '';
        html += '<label class="alloc-cb"><input type="checkbox" value="CH' + j + '"' + chk + '> CH' + j + '</label>';
      }
      html += '</div>';
      return html;
    }

    // DR1-09: 条件付き入力制御つきPIN Settings
    function renderPinSettings(pins) {
      var list = document.getElementById('pin-settings-list');
      if (!list) return;
      var html = '';
      for (var i = 0; i < pins.length; i++) {
        var p = pins[i];
        var ch = p.channel;
        var stn = p.stateName || [];
        var alloc = p.allocation || [];
        html += '<div class="pin-setting-card" data-ch="' + ch + '"><h4>CH' + ch + (p.name ? ' - ' + escapeHtml(p.name) : '') + '</h4>';
        html += '<div class="form-row">';
        html += '<div class="form-group"><label>Type</label><select id="type-' + ch + '" onchange="updateFieldVisibility(' + ch + ')">';
        html += '<option value="digitalOutput"' + (p.type === 'digitalOutput' ? ' selected' : '') + '>Digital Output</option>';
        html += '<option value="pwmOutput"' + (p.type === 'pwmOutput' ? ' selected' : '') + '>PWM Output</option>';
        html += '<option value="digitalInput"' + (p.type === 'digitalInput' ? ' selected' : '') + '>Digital Input</option>';
        html += '<option value="disabled"' + (p.type === 'disabled' ? ' selected' : '') + '>Disabled</option>';
        html += '</select></div>';
        // T2: Mode ドロップダウン動的生成
        html += '<div class="form-group" id="fg-mode-' + ch + '"><label>Mode</label><select id="mode-' + ch + '" onchange="updateFieldVisibility(' + ch + ')">';
        html += getModeOptionsForType(p.type, p.actionMode);
        html += '</select></div>';
        html += '</div>';
        html += '<div class="form-group"><label>Name</label><input type="text" id="name-' + ch + '" value="' + escapeHtml(p.name || '') + '" placeholder="CH' + ch + '"></div>';
        html += '<div class="form-row">';
        html += '<div class="form-group" id="fg-validity-' + ch + '"><label>Validity (ms)</label><input type="number" id="validity-' + ch + '" value="' + (p.validity || 0) + '" min="0" step="100"></div>';
        html += '<div class="form-group" id="fg-debounce-' + ch + '"><label>Debounce (ms)</label><input type="number" id="debounce-' + ch + '" value="' + (p.debounce || 0) + '" min="0" step="100"></div>';
        html += '<div class="form-group" id="fg-rateOfChange-' + ch + '"><label>Rate of Change (ms)</label><input type="number" id="rateOfChange-' + ch + '" value="' + (p.rateOfChange || 0) + '" min="0" step="100"></div>';
        html += '</div>';
        html += '<div class="form-group" id="fg-stateName-' + ch + '"><label>State Names</label><input type="text" id="stateName-' + ch + '" value="' + escapeHtml(stn.join(',')) + '" placeholder="on:ON,off:OFF"></div>';
        // T4: Allocation チェックボックス化
        html += '<div class="form-group" id="fg-allocation-' + ch + '"><label>Allocation (Input→Output)</label>' + getAllocationCheckboxes(ch, alloc) + '</div>';
        html += '<div class="form-row">';
        html += '<div class="form-group" id="fg-expiryDate-' + ch + '"><label>Expiry Date</label><input type="text" id="expiryDate-' + ch + '" value="' + (p.expiryDate || '') + '" placeholder="YYYYMMDDHHMM"></div>';
        html += '<div class="form-group" id="fg-expiryEnabled-' + ch + '"><label>Expiry Enabled</label><input type="checkbox" id="expiryEnabled-' + ch + '"' + (p.expiryEnabled ? ' checked' : '') + '></div>';
        html += '</div>';
        html += '</div>';
      }
      list.innerHTML = html;
      // DR1-09: 初期表示時に各チャンネルのフィールド可視性を設定
      for (var i = 0; i < pins.length; i++) {
        updateFieldVisibility(pins[i].channel);
      }
    }

    // DR1-09: タイプ/モードに応じたフィールド可視性制御
    // T3: Type変更時にModeオプションを再生成
    function updateFieldVisibility(ch) {
      var type = document.getElementById('type-' + ch).value;
      var modeSelect = document.getElementById('mode-' + ch);
      var currentMode = modeSelect.value;

      // T3: Modeドロップダウンを再生成
      var newOpts = getModeOptionsForType(type, currentMode);
      modeSelect.innerHTML = newOpts;
      // 現在のモードが新しいオプションにない場合、最初のオプションを選択
      if (modeSelect.value !== currentMode) {
        modeSelect.selectedIndex = 0;
      }
      var mode = modeSelect.value;

      // Mode フィールド: disabled では無効
      var modeEnabled = (type !== 'disabled');
      setFieldEnabled('mode-' + ch, 'fg-mode-' + ch, modeEnabled);

      // Validity: digitalOutput/Input の Mom モードのみ
      var validityEnabled = (type === 'digitalOutput' || type === 'digitalInput') && mode === 'Mom';
      setFieldEnabled('validity-' + ch, 'fg-validity-' + ch, validityEnabled);

      // Debounce: digitalOutput/Input のみ (pwm/disabledでは無効)
      var debounceEnabled = (type === 'digitalOutput' || type === 'digitalInput');
      setFieldEnabled('debounce-' + ch, 'fg-debounce-' + ch, debounceEnabled);

      // RateOfChange: pwmOutput のみ
      var rocEnabled = (type === 'pwmOutput');
      setFieldEnabled('rateOfChange-' + ch, 'fg-rateOfChange-' + ch, rocEnabled);

      // Allocation: digitalInput のみ (チェックボックス無効化)
      var allocEnabled = (type === 'digitalInput');
      var allocDiv = document.getElementById('allocation-' + ch);
      if (allocDiv) {
        var cbs = allocDiv.querySelectorAll('input[type=checkbox]');
        cbs.forEach(function(cb) { cb.disabled = !allocEnabled; });
      }
      var allocGroup = document.getElementById('fg-allocation-' + ch);
      if (allocGroup) allocGroup.style.opacity = allocEnabled ? '1' : '0.4';

      // stateName: digitalOutput/pwmOutput のみ
      var stnEnabled = (type === 'digitalOutput' || type === 'pwmOutput');
      setFieldEnabled('stateName-' + ch, 'fg-stateName-' + ch, stnEnabled);

      // expiryDate/Enabled: digitalOutput/pwmOutput のみ
      var expiryEnabled = (type === 'digitalOutput' || type === 'pwmOutput');
      setFieldEnabled('expiryDate-' + ch, 'fg-expiryDate-' + ch, expiryEnabled);
      setFieldEnabled('expiryEnabled-' + ch, 'fg-expiryEnabled-' + ch, expiryEnabled);
    }

    function setFieldEnabled(inputId, groupId, enabled) {
      var input = document.getElementById(inputId);
      var group = document.getElementById(groupId);
      if (input) input.disabled = !enabled;
      if (group) group.style.opacity = enabled ? '1' : '0.4';
    }

    function savePinSettings() {
      var cards = document.querySelectorAll('.pin-setting-card');
      var promises = [];
      for (var i = 0; i < cards.length; i++) {
        var card = cards[i];
        var ch = card.getAttribute('data-ch');
        var stnStr = document.getElementById('stateName-' + ch).value;
        var stnArr = stnStr ? stnStr.split(',').map(function(s) { return s.trim(); }).filter(function(s) { return s; }) : [];
        // T5: Allocation をチェックボックスから収集
        var allocArr = [];
        var allocCbs = document.querySelectorAll('#allocation-' + ch + ' input[type=checkbox]:checked');
        allocCbs.forEach(function(cb) { allocArr.push(cb.value); });
        var setting = {
          type: document.getElementById('type-' + ch).value,
          name: document.getElementById('name-' + ch).value,
          actionMode: document.getElementById('mode-' + ch).value,
          validity: parseInt(document.getElementById('validity-' + ch).value) || 0,
          debounce: parseInt(document.getElementById('debounce-' + ch).value) || 0,
          rateOfChange: parseInt(document.getElementById('rateOfChange-' + ch).value) || 0,
          stateName: stnArr,
          allocation: allocArr,
          expiryDate: document.getElementById('expiryDate-' + ch).value || '',
          expiryEnabled: document.getElementById('expiryEnabled-' + ch).checked
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

    // Device Settings 読み込み
    function loadDeviceSettings() {
      fetch('/api/settings')
        .then(function(r) { return r.json(); })
        .then(function(data) {
          if (data.ok && data.settings) {
            var s = data.settings;
            document.getElementById('ds-device-name').value = s.device_name || '';
            document.getElementById('ds-rid').value = s.rid || '';
            if (s.pinGlobal) {
              document.getElementById('ds-g-validity').value = s.pinGlobal.validity || 0;
              document.getElementById('ds-g-debounce').value = s.pinGlobal.debounce || 0;
              document.getElementById('ds-g-rateofchange').value = s.pinGlobal.rateOfChange || 0;
            }
          }
        })
        .catch(function(e) { console.error('Load device settings error:', e); });
    }

    // Device Settings 保存
    function saveDeviceSettings() {
      var settings = {
        device_name: document.getElementById('ds-device-name').value,
        rid: document.getElementById('ds-rid').value,
        pinGlobal: {
          validity: parseInt(document.getElementById('ds-g-validity').value) || 0,
          debounce: parseInt(document.getElementById('ds-g-debounce').value) || 0,
          rateOfChange: parseInt(document.getElementById('ds-g-rateofchange').value) || 0
        }
      };
      fetch('/api/settings', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify(settings)
      })
        .then(function(r) { return r.json(); })
        .then(function(data) {
          if (data.ok) {
            alert('Device settings saved');
          } else {
            alert('Error: ' + data.message);
          }
        })
        .catch(function(e) { alert('Error: ' + e.message); });
    }

    // DR1-04: タブ切り替え時のポーリング制御
    document.addEventListener('tabchange', function(e) {
      // 他タブに移動したらポーリング停止
      stopPinPolling();

      if (e.detail === 'pincontrol') {
        loadPinStates();
        startPinPolling();  // DR1-04: PIN Controlタブでポーリング開始
      }
      if (e.detail === 'pinsetting') loadPinSettings();
      if (e.detail === 'devicesettings') loadDeviceSettings();
    });

    // ページ離脱時にポーリング停止
    window.addEventListener('beforeunload', stopPinPolling);
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

  // GPIO診断API (review8.md)
  server_->on("/api/debug/gpio", HTTP_GET, [this]() { handleDebugGpio(); });

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
    // OLED通知コールバック（API制御）
    if (pinStateChangeCallback_) {
      pinStateChangeCallback_(ch, state);
    }
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

    // OLED通知コールバック（API制御）
    if (pinStateChangeCallback_) {
      pinStateChangeCallback_(ch, newState);
    }
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
    // Must Fix #1: 状態と設定の両方を含める（PIN Settingsタブ対応）
    buildPinStateJson(p, ch);
    // 設定値も追加（buildPinStateJsonに含まれないもの）
    if (pinManager_) {
      const PinSetting& setting = pinManager_->getPinSetting(ch);
      p["validity"] = setting.validity;
      p["debounce"] = setting.debounce;
      p["rateOfChange"] = setting.rateOfChange;
      p["expiryDate"] = setting.expiryDate;
      p["expiryEnabled"] = setting.expiryEnabled;
      // allocation
      JsonArray allocArr = p.createNestedArray("allocation");
      for (int i = 0; i < setting.allocationCount; i++) {
        if (!setting.allocation[i].isEmpty()) {
          allocArr.add(setting.allocation[i]);
        }
      }
      // PWM遷移状態（Slowモード対応）CH1-4のみ
      if (ch <= 4) {
        p["pwmTransitioning"] = pinManager_->isPwmTransitioning(ch);
        p["pwmTarget"] = pinManager_->getPwmTargetValue(ch);
      }
      // Momentaryカウントダウン用：残り時間（ms）
      // state=1 かつ Mom モードの場合のみ pulseRemaining を返す
      if (setting.actionMode == ActionMode::MOMENTARY && pinManager_->getPinState(ch) == 1) {
        unsigned long now = millis();
        unsigned long pulseEnd = pinManager_->getPulseEndTime(ch);
        if (pulseEnd > now) {
          p["pulseRemaining"] = (int)(pulseEnd - now);
        }
      }
    }
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
  settings["rid"] = settings_->getString("rid", "");  // roomID - グループ/部屋識別
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

  // Room ID (rid) - グループ/部屋識別
  if (doc.containsKey("rid")) {
    settings_->setString("rid", doc["rid"].as<String>());
    changed = true;
    Serial.printf("[Settings] rid set to: %s\n", doc["rid"].as<String>().c_str());
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

// ============================================================
// GPIO診断API (review8.md対応)
// GPIO Matrix状態と高頻度サンプリングで診断
// ============================================================
void HttpManagerIs06s::handleDebugGpio() {
  JsonDocument doc;
  doc["ok"] = true;
  doc["description"] = "GPIO diagnostic (review8.md Method A+B)";

  // 診断対象GPIO: CH1-4のみ（問題のあるGPIO18, GPIO5を含む）
  const int TARGET_GPIOS[] = {18, 5, 15, 27};  // CH1, CH2, CH3, CH4
  const int TARGET_COUNT = 4;

  JsonArray gpios = doc.createNestedArray("gpios");

  for (int i = 0; i < TARGET_COUNT; i++) {
    int pin = TARGET_GPIOS[i];
    JsonObject g = gpios.createNestedObject();
    g["gpio"] = pin;
    g["channel"] = i + 1;

    // === Method B: GPIO Matrix Register Read ===
    // GPIO_FUNCx_OUT_SEL_CFG_REG: bits 8:0 = signal output index
    // Signal ID 256 = simple GPIO output
    // Other values = peripheral (LEDC, SPI, etc.)
    volatile uint32_t* funcOutSelReg = (volatile uint32_t*)(0x3FF44530 + (pin * 4));
    uint32_t regValue = *funcOutSelReg;
    int sigOutId = regValue & 0x1FF;  // bits 8:0
    g["sigOutId"] = sigOutId;
    g["sigOutIdHex"] = String("0x") + String(sigOutId, HEX);
    g["isSimpleGpio"] = (sigOutId == 256);

    // GPIOの現在レベル（単発）
    int currentLevel = gpio_get_level((gpio_num_t)pin);
    g["currentLevel"] = currentLevel;

    // === 追加診断: GPIO_OUT_REG, GPIO_ENABLE_REG, GPIO_IN_REG 直接読み取り ===
    // GPIO_OUT_REG (0x3FF44004): 出力ラッチの状態
    // GPIO_ENABLE_REG (0x3FF44020): 出力有効化状態
    // GPIO_IN_REG (0x3FF4403C): 入力レベル
    volatile uint32_t* gpioOutReg = (volatile uint32_t*)0x3FF44004;
    volatile uint32_t* gpioEnableReg = (volatile uint32_t*)0x3FF44020;
    volatile uint32_t* gpioInReg = (volatile uint32_t*)0x3FF4403C;
    bool outBit = (*gpioOutReg >> pin) & 1;
    bool enableBit = (*gpioEnableReg >> pin) & 1;
    bool inBit = (*gpioInReg >> pin) & 1;
    g["gpioOutBit"] = outBit ? 1 : 0;
    g["gpioEnableBit"] = enableBit ? 1 : 0;
    g["gpioInBit"] = inBit ? 1 : 0;

    // === GPIO_PINx_REG: オープンドレイン設定確認 ===
    // GPIO_PINx_REG: bit 2 = PAD_DRIVER (0=push-pull, 1=open-drain)
    volatile uint32_t* gpioPinReg = (volatile uint32_t*)(0x3FF44088 + pin * 4);
    uint32_t pinRegValue = *gpioPinReg;
    bool isOpenDrain = (pinRegValue >> 2) & 1;
    g["isOpenDrain"] = isOpenDrain;
    g["gpioPinReg"] = String("0x") + String(pinRegValue, HEX);

    // === IO_MUX Register: GPIO function選択確認 ===
    // IO_MUXレジスタはGPIO番号によってオフセットが異なる
    // bits 14:12 = MCU_SEL (function select, 2=GPIO)
    // bits 7 = FUN_OE (output enable)
    // bits 0 = SLP_SEL (sleep mode)
    static const uint16_t IOMUX_OFFSETS[] = {
      0x44, 0x88, 0x40, 0x84, 0x48, 0x6C, // GPIO 0-5
      0,0,0,0,0,0, // GPIO 6-11 (flash, not available)
      0x34, 0x38, 0x30, 0x4C, 0x50, 0x54, // GPIO 12-17
      0x70, 0x74, 0, 0x7C, 0x80, 0x8C, // GPIO 18-23 (20 not available)
      0, 0x24, 0x28, 0x2C // GPIO 24-27
    };
    uint16_t iomuxOffset = 0;
    if (pin < 28) iomuxOffset = IOMUX_OFFSETS[pin];
    if (iomuxOffset != 0) {
      volatile uint32_t* iomuxReg = (volatile uint32_t*)(0x3FF49000 + iomuxOffset);
      uint32_t iomuxValue = *iomuxReg;
      int funSel = (iomuxValue >> 12) & 0x7;  // MCU_SEL bits 14:12
      bool funOe = (iomuxValue >> 7) & 1;     // FUN_OE bit 7
      g["iomuxReg"] = String("0x") + String(iomuxValue, HEX);
      g["iomuxFunSel"] = funSel;  // 2 = GPIO function
      g["iomuxFunOe"] = funOe;
    }

    // Is06PinManagerからのソフトウェア状態
    if (pinManager_) {
      int swState = pinManager_->getPinState(i + 1);
      g["softwareState"] = swState;
      // LEDCチャンネル番号（PWMデバッグ用）
      g["ledcChannel"] = pinManager_->getLedcChannel(i + 1);
      g["pwmValue"] = pinManager_->getPwmValue(i + 1);
    }

    // === Method A: High-frequency sampling with delay ===
    // 100ms遅延後に1000サンプルを高速取得してPWM/トグルを検出
    delay(100);  // 他のコードが介入する時間を与える

    const int SAMPLE_COUNT = 1000;
    int highCount = 0;
    int lowCount = 0;
    int transitions = 0;
    int lastSample = gpio_get_level((gpio_num_t)pin);

    for (int s = 0; s < SAMPLE_COUNT; s++) {
      int sample = gpio_get_level((gpio_num_t)pin);
      if (sample == 1) {
        highCount++;
      } else {
        lowCount++;
      }
      if (sample != lastSample) {
        transitions++;
        lastSample = sample;
      }
      if (s % 100 == 0) delayMicroseconds(10);  // 時間分散
    }

    // 遅延後の状態
    g["delayedLevel"] = gpio_get_level((gpio_num_t)pin);

    g["sampleCount"] = SAMPLE_COUNT;
    g["highCount"] = highCount;
    g["lowCount"] = lowCount;
    g["transitions"] = transitions;
    g["highPercent"] = (highCount * 100) / SAMPLE_COUNT;

    // 診断結果の解釈
    String diagnosis = "";
    if (transitions > 10) {
      diagnosis = "PWM_DETECTED: Pin is toggling (peripheral driving)";
    } else if (sigOutId != 256) {
      diagnosis = "PERIPHERAL_ASSIGNED: SigOut=" + String(sigOutId) + " (not GPIO)";
    } else if (highCount == SAMPLE_COUNT) {
      diagnosis = "STABLE_HIGH: Simple GPIO, DC high";
    } else if (lowCount == SAMPLE_COUNT) {
      diagnosis = "STABLE_LOW: Simple GPIO, DC low";
    } else {
      diagnosis = "UNSTABLE: Mixed readings without transitions";
    }
    g["diagnosis"] = diagnosis;

    // LEDCチャンネル状態（Is06PinManagerから取得可能であれば）
    if (pinManager_ && i < 4) {
      // 注: ledcChannel_はprivateなので外部からはアクセス不可
      // setPinType()を経由して取得するか、公開メソッドが必要
      // 今回は「type」から推測
      const PinSetting& setting = pinManager_->getPinSetting(i + 1);
      g["configuredType"] = (setting.type == ::PinType::PWM_OUTPUT) ? "pwmOutput" :
                           (setting.type == ::PinType::DIGITAL_OUTPUT) ? "digitalOutput" :
                           (setting.type == ::PinType::DIGITAL_INPUT) ? "digitalInput" : "disabled";
    }
  }

  // GPIO Matrix peripheral reference (参考情報)
  JsonObject reference = doc.createNestedObject("sigOutIdReference");
  reference["256"] = "Simple GPIO output";
  reference["79-86"] = "LEDC channels (PWM)";
  reference["63"] = "SPICLK (VSPI/HSPI)";
  reference["68"] = "SPICS0 (VSPI/HSPI)";

  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}
