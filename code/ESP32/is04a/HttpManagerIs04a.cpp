/**
 * HttpManagerIs04a.cpp
 */

#include "HttpManagerIs04a.h"
#include "TriggerManager.h"
#include "SettingManager.h"
#include <ArduinoJson.h>

HttpManagerIs04a::HttpManagerIs04a()
    : trigger_(nullptr)
    , sentCount_(0)
    , failCount_(0)
    , lastResult_("---")
    , lastPulseUpdatedAt_("")
    , lastPulseOutput_(0)
{
}

void HttpManagerIs04a::begin(SettingManager* settings, TriggerManager* trigger, int port) {
    trigger_ = trigger;
    AraneaWebUI::begin(settings, port);
}

void HttpManagerIs04a::setSendStatus(int sent, int fail, const String& lastResult) {
    sentCount_ = sent;
    failCount_ = fail;
    lastResult_ = lastResult;
}

void HttpManagerIs04a::setPulseStatus(int outputNum, const String& lastUpdatedAt) {
    lastPulseOutput_ = outputNum;
    lastPulseUpdatedAt_ = lastUpdatedAt;
}

void HttpManagerIs04a::registerTypeSpecificEndpoints() {
    server_->on("/api/status", HTTP_GET, [this]() { handleStatus(); });
    server_->on("/api/pulse", HTTP_POST, [this]() { handlePulse(); });
    server_->on("/api/config", HTTP_GET, [this]() { handleConfig(); });
    server_->on("/api/config", HTTP_POST, [this]() { handleConfig(); });
    server_->on("/api/trigger", HTTP_GET, [this]() { handleTriggerConfig(); });
    server_->on("/api/trigger", HTTP_POST, [this]() { handleTriggerConfig(); });
}

void HttpManagerIs04a::handleStatus() {
    if (!checkAuth()) { requestAuth(); return; }

    JsonDocument doc;

    // 入力状態
    JsonObject inputs = doc.createNestedObject("inputs");
    for (int i = 1; i <= 2; i++) {
        auto state = trigger_->getInputState(i);
        JsonObject inObj = inputs.createNestedObject("in" + String(i));
        inObj["active"] = state.active;
        inObj["pin"] = state.pin;
        inObj["target"] = state.targetOutput;
        inObj["lastUpdatedAt"] = state.lastUpdatedAt;
    }

    // 出力状態
    JsonObject outputs = doc.createNestedObject("outputs");
    for (int i = 1; i <= 2; i++) {
        auto state = trigger_->getOutputState(i);
        JsonObject outObj = outputs.createNestedObject("out" + String(i));
        outObj["active"] = state.active;
        outObj["pin"] = state.pin;
        outObj["name"] = state.name;
    }

    // 設定
    doc["pulseMs"] = trigger_->getPulseMs();
    doc["interlockMs"] = trigger_->getInterlockMs();
    doc["debounceMs"] = trigger_->getDebounceMs();

    // 送信統計
    JsonObject stats = doc.createNestedObject("stats");
    stats["sent"] = sentCount_;
    stats["fail"] = failCount_;
    stats["lastResult"] = lastResult_;

    String json;
    serializeJson(doc, json);
    server_->send(200, "application/json", json);
}

void HttpManagerIs04a::handlePulse() {
    if (!checkAuth()) { requestAuth(); return; }

    // パラメータ取得
    int outputNum = 1;
    int durationMs = trigger_->getPulseMs();

    if (server_->hasArg("plain")) {
        JsonDocument doc;
        DeserializationError error = deserializeJson(doc, server_->arg("plain"));
        if (!error) {
            outputNum = doc["output"] | 1;
            durationMs = doc["duration"] | trigger_->getPulseMs();
        }
    } else if (server_->hasArg("output")) {
        outputNum = server_->arg("output").toInt();
        if (server_->hasArg("duration")) {
            durationMs = server_->arg("duration").toInt();
        }
    }

    // 範囲チェック
    if (outputNum < 1 || outputNum > 2) {
        server_->send(400, "application/json", "{\"error\":\"Invalid output number\"}");
        return;
    }
    if (durationMs < 10) durationMs = 10;
    if (durationMs > 10000) durationMs = 10000;

    // パルス実行
    bool success = trigger_->startPulse(outputNum, durationMs, TriggerManager::PulseSource::HTTP);

    JsonDocument doc;
    doc["ok"] = success;
    doc["output"] = outputNum;
    doc["duration"] = durationMs;
    if (!success) {
        if (trigger_->isPulseActive()) {
            doc["error"] = "Pulse already active";
        } else {
            doc["error"] = "Interlock active";
        }
    }

    String json;
    serializeJson(doc, json);
    server_->send(success ? 200 : 409, "application/json", json);
}

void HttpManagerIs04a::handleConfig() {
    if (!checkAuth()) { requestAuth(); return; }

    if (server_->method() == HTTP_GET) {
        // 設定取得
        JsonDocument doc;
        doc["pulseMs"] = trigger_->getPulseMs();
        doc["interlockMs"] = trigger_->getInterlockMs();
        doc["debounceMs"] = trigger_->getDebounceMs();
        doc["out1Name"] = trigger_->getOutputName(1);
        doc["out2Name"] = trigger_->getOutputName(2);
        doc["in1Target"] = trigger_->getTriggerAssign(1);
        doc["in2Target"] = trigger_->getTriggerAssign(2);

        String json;
        serializeJson(doc, json);
        server_->send(200, "application/json", json);
    } else {
        // 設定更新
        if (!server_->hasArg("plain")) {
            server_->send(400, "application/json", "{\"error\":\"No body\"}");
            return;
        }

        JsonDocument doc;
        DeserializationError error = deserializeJson(doc, server_->arg("plain"));
        if (error) {
            server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
            return;
        }

        if (doc.containsKey("pulseMs")) {
            trigger_->setPulseMs(doc["pulseMs"]);
        }
        if (doc.containsKey("interlockMs")) {
            trigger_->setInterlockMs(doc["interlockMs"]);
        }
        if (doc.containsKey("debounceMs")) {
            trigger_->setDebounceMs(doc["debounceMs"]);
        }
        if (doc.containsKey("out1Name")) {
            trigger_->setOutputName(1, doc["out1Name"].as<String>());
        }
        if (doc.containsKey("out2Name")) {
            trigger_->setOutputName(2, doc["out2Name"].as<String>());
        }

        server_->send(200, "application/json", "{\"ok\":true}");
    }
}

void HttpManagerIs04a::handleTriggerConfig() {
    if (!checkAuth()) { requestAuth(); return; }

    if (server_->method() == HTTP_GET) {
        JsonDocument doc;
        doc["in1Target"] = trigger_->getTriggerAssign(1);
        doc["in2Target"] = trigger_->getTriggerAssign(2);

        String json;
        serializeJson(doc, json);
        server_->send(200, "application/json", json);
    } else {
        if (!server_->hasArg("plain")) {
            server_->send(400, "application/json", "{\"error\":\"No body\"}");
            return;
        }

        JsonDocument doc;
        DeserializationError error = deserializeJson(doc, server_->arg("plain"));
        if (error) {
            server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
            return;
        }

        if (doc.containsKey("in1Target")) {
            trigger_->setTriggerAssign(1, doc["in1Target"]);
        }
        if (doc.containsKey("in2Target")) {
            trigger_->setTriggerAssign(2, doc["in2Target"]);
        }

        server_->send(200, "application/json", "{\"ok\":true}");
    }
}

String HttpManagerIs04a::generateTypeSpecificJS() {
    String js;
    js += F("<script>");

    // パルス実行
    js += F("function firePulse(n){fetch('/api/pulse',{method:'POST',");
    js += F("headers:{'Content-Type':'application/json'},body:JSON.stringify({output:n})})");
    js += F(".then(r=>r.json()).then(d=>{");
    js += F("if(d.ok){showToast('Pulse OUT'+n+' executed');}else{showToast('Failed: '+d.error,'error');}");
    js += F("refreshStatus();}).catch(e=>showToast('Error','error'));}");

    // ステータス更新
    js += F("function refreshStatus(){fetch('/api/status').then(r=>r.json()).then(d=>{");
    // 入力状態
    js += F("for(let i=1;i<=2;i++){");
    js += F("let inp=d.inputs['in'+i];");
    js += F("let el=document.getElementById('in'+i+'_state');");
    js += F("if(el){el.textContent=inp.active?'ACTIVE':'inactive';");
    js += F("el.className=inp.active?'badge active':'badge inactive';}");
    js += F("}");
    // 出力状態
    js += F("for(let i=1;i<=2;i++){");
    js += F("let out=d.outputs['out'+i];");
    js += F("let el=document.getElementById('out'+i+'_state');");
    js += F("if(el){el.textContent=out.active?'PULSE':'idle';");
    js += F("el.className=out.active?'badge pulse':'badge idle';}");
    js += F("}");
    // 統計
    js += F("let st=d.stats;");
    js += F("document.getElementById('sent_count').textContent=st.sent;");
    js += F("document.getElementById('fail_count').textContent=st.fail;");
    js += F("document.getElementById('last_result').textContent=st.lastResult;");
    js += F("}).catch(e=>console.error(e));}");

    // 設定保存
    js += F("function saveConfig(){");
    js += F("let cfg={");
    js += F("pulseMs:parseInt(document.getElementById('pulseMs').value),");
    js += F("interlockMs:parseInt(document.getElementById('interlockMs').value),");
    js += F("debounceMs:parseInt(document.getElementById('debounceMs').value),");
    js += F("out1Name:document.getElementById('out1Name').value,");
    js += F("out2Name:document.getElementById('out2Name').value");
    js += F("};");
    js += F("fetch('/api/config',{method:'POST',headers:{'Content-Type':'application/json'},");
    js += F("body:JSON.stringify(cfg)}).then(r=>r.json()).then(d=>{");
    js += F("if(d.ok){showToast('Config saved');}else{showToast('Failed','error');}");
    js += F("}).catch(e=>showToast('Error','error'));}");

    // トリガーアサイン保存
    js += F("function saveTrigger(){");
    js += F("let cfg={");
    js += F("in1Target:parseInt(document.getElementById('in1Target').value),");
    js += F("in2Target:parseInt(document.getElementById('in2Target').value)");
    js += F("};");
    js += F("fetch('/api/trigger',{method:'POST',headers:{'Content-Type':'application/json'},");
    js += F("body:JSON.stringify(cfg)}).then(r=>r.json()).then(d=>{");
    js += F("if(d.ok){showToast('Trigger config saved');}else{showToast('Failed','error');}");
    js += F("}).catch(e=>showToast('Error','error'));}");

    // 定期更新
    js += F("setInterval(refreshStatus,2000);");
    js += F("refreshStatus();");

    js += F("</script>");
    return js;
}

String HttpManagerIs04a::generateTypeSpecificTabContents() {
    String html;

    // 入力状態セクション
    html += F("<div class='section'>");
    html += F("<h2>Physical Inputs</h2>");
    html += F("<div class='grid'>");
    for (int i = 1; i <= 2; i++) {
        auto state = trigger_->getInputState(i);
        html += F("<div class='card'>");
        html += "<h3>INPUT " + String(i) + " (GPIO" + String(state.pin) + ")</h3>";
        html += "<span id='in" + String(i) + "_state' class='badge ";
        html += state.active ? "active'>ACTIVE" : "inactive'>inactive";
        html += F("</span>");
        html += "<p>Target: OUT" + String(state.targetOutput) + "</p>";
        html += F("</div>");
    }
    html += F("</div>");
    html += F("</div>");

    // 出力制御セクション
    html += F("<div class='section'>");
    html += F("<h2>Trigger Outputs</h2>");
    html += F("<div class='grid'>");
    for (int i = 1; i <= 2; i++) {
        auto state = trigger_->getOutputState(i);
        html += F("<div class='card'>");
        html += "<h3>" + state.name + " (GPIO" + String(state.pin) + ")</h3>";
        html += "<span id='out" + String(i) + "_state' class='badge ";
        html += state.active ? "pulse'>PULSE" : "idle'>idle";
        html += F("</span>");
        html += "<button onclick='firePulse(" + String(i) + ")'>Fire Pulse</button>";
        html += F("</div>");
    }
    html += F("</div>");
    html += F("</div>");

    // 統計セクション
    html += F("<div class='section'>");
    html += F("<h2>Statistics</h2>");
    html += F("<p>Sent: <span id='sent_count'>");
    html += String(sentCount_);
    html += F("</span> / Fail: <span id='fail_count'>");
    html += String(failCount_);
    html += F("</span> / Last: <span id='last_result'>");
    html += lastResult_;
    html += F("</span></p>");
    html += F("</div>");

    // 設定セクション
    html += F("<div class='section'>");
    html += F("<h2>Pulse Settings</h2>");

    html += F("<label>Pulse Duration (ms):</label>");
    html += "<input type='number' id='pulseMs' value='" + String(trigger_->getPulseMs()) + "' min='10' max='10000'>";

    html += F("<label>Interlock (ms):</label>");
    html += "<input type='number' id='interlockMs' value='" + String(trigger_->getInterlockMs()) + "' min='0' max='5000'>";

    html += F("<label>Debounce (ms):</label>");
    html += "<input type='number' id='debounceMs' value='" + String(trigger_->getDebounceMs()) + "' min='5' max='1000'>";

    html += F("<label>Output 1 Name:</label>");
    html += "<input type='text' id='out1Name' value='" + trigger_->getOutputName(1) + "'>";

    html += F("<label>Output 2 Name:</label>");
    html += "<input type='text' id='out2Name' value='" + trigger_->getOutputName(2) + "'>";

    html += F("<button onclick='saveConfig()'>Save Settings</button>");
    html += F("</div>");

    // トリガーアサイン設定
    html += F("<div class='section'>");
    html += F("<h2>Trigger Assign</h2>");

    html += F("<label>Input 1 -> Output:</label>");
    html += F("<select id='in1Target'>");
    html += "<option value='1'" + String(trigger_->getTriggerAssign(1) == 1 ? " selected" : "") + ">OUT1</option>";
    html += "<option value='2'" + String(trigger_->getTriggerAssign(1) == 2 ? " selected" : "") + ">OUT2</option>";
    html += F("</select>");

    html += F("<label>Input 2 -> Output:</label>");
    html += F("<select id='in2Target'>");
    html += "<option value='1'" + String(trigger_->getTriggerAssign(2) == 1 ? " selected" : "") + ">OUT1</option>";
    html += "<option value='2'" + String(trigger_->getTriggerAssign(2) == 2 ? " selected" : "") + ">OUT2</option>";
    html += F("</select>");

    html += F("<button onclick='saveTrigger()'>Save Trigger Assign</button>");
    html += F("</div>");

    return html;
}
