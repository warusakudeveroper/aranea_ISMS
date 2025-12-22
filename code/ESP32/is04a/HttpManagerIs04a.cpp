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
    // 注意: /api/status と /api/config は AraneaWebUI で登録済み
    // デバイス固有の情報は getTypeSpecificStatus() / getTypeSpecificConfig() で追加
    server_->on("/api/is04a/pulse", HTTP_POST, [this]() { handlePulse(); });
    server_->on("/api/is04a/trigger", HTTP_GET, [this]() { handleTriggerConfig(); });
    server_->on("/api/is04a/trigger", HTTP_POST, [this]() { handleTriggerConfig(); });
}

void HttpManagerIs04a::getTypeSpecificStatus(JsonObject& obj) {
    // 入力状態
    JsonObject inputs = obj.createNestedObject("inputs");
    for (int i = 1; i <= 2; i++) {
        auto state = trigger_->getInputState(i);
        JsonObject inObj = inputs.createNestedObject("in" + String(i));
        inObj["active"] = state.active;
        inObj["pin"] = state.pin;
        inObj["target"] = state.targetOutput;
        inObj["lastUpdatedAt"] = state.lastUpdatedAt;
    }

    // 出力状態
    JsonObject outputs = obj.createNestedObject("outputs");
    for (int i = 1; i <= 2; i++) {
        auto state = trigger_->getOutputState(i);
        JsonObject outObj = outputs.createNestedObject("out" + String(i));
        outObj["active"] = state.active;
        outObj["pin"] = state.pin;
        outObj["name"] = state.name;
    }

    // 設定
    obj["pulseMs"] = trigger_->getPulseMs();
    obj["interlockMs"] = trigger_->getInterlockMs();
    obj["debounceMs"] = trigger_->getDebounceMs();

    // 送信統計
    JsonObject stats = obj.createNestedObject("stats");
    stats["sent"] = sentCount_;
    stats["fail"] = failCount_;
    stats["lastResult"] = lastResult_;
}

void HttpManagerIs04a::getTypeSpecificConfig(JsonObject& obj) {
    obj["pulseMs"] = trigger_->getPulseMs();
    obj["interlockMs"] = trigger_->getInterlockMs();
    obj["debounceMs"] = trigger_->getDebounceMs();
    obj["out1Name"] = trigger_->getOutputName(1);
    obj["out2Name"] = trigger_->getOutputName(2);
    obj["in1Target"] = trigger_->getTriggerAssign(1);
    obj["in2Target"] = trigger_->getTriggerAssign(2);
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

String HttpManagerIs04a::generateTypeSpecificTabs() {
    return F("<div class=\"tab\" data-tab=\"trigger\" onclick=\"showTab('trigger')\">Trigger</div>");
}

String HttpManagerIs04a::generateTypeSpecificJS() {
    String js;
    js += F("<script>");

    // パルス実行（固有APIパス使用）
    js += F("function firePulse(n){fetch('/api/is04a/pulse',{method:'POST',");
    js += F("headers:{'Content-Type':'application/json'},body:JSON.stringify({output:n})})");
    js += F(".then(r=>r.json()).then(d=>{");
    js += F("if(d.ok){showToast('Pulse OUT'+n+' executed');}else{showToast('Failed: '+d.error,'error');}");
    js += F("}).catch(e=>showToast('Error','error'));}");

    // typeSpecificステータス更新（AraneaWebUIのrefreshStatusから呼ばれる）
    js += F("function refreshTypeSpecificStatus(s){");
    js += F("let ts=s.typeSpecific||{};");
    // 入力状態
    js += F("let inputs=ts.inputs||{};");
    js += F("for(let i=1;i<=2;i++){");
    js += F("let inp=inputs['in'+i]||{};");
    js += F("let el=document.getElementById('in'+i+'_state');");
    js += F("if(el){el.textContent=inp.active?'ACTIVE':'inactive';");
    js += F("el.className=inp.active?'status-item value good':'status-item value';}");
    js += F("}");
    // 出力状態
    js += F("let outputs=ts.outputs||{};");
    js += F("for(let i=1;i<=2;i++){");
    js += F("let out=outputs['out'+i]||{};");
    js += F("let el=document.getElementById('out'+i+'_state');");
    js += F("if(el){el.textContent=out.active?'PULSE':'idle';");
    js += F("el.className=out.active?'status-item value warn':'status-item value';}");
    js += F("}");
    // 統計
    js += F("let st=ts.stats||{};");
    js += F("let se=document.getElementById('sent_count');if(se)se.textContent=st.sent||0;");
    js += F("let fe=document.getElementById('fail_count');if(fe)fe.textContent=st.fail||0;");
    js += F("let le=document.getElementById('last_result');if(le)le.textContent=st.lastResult||'---';");
    js += F("}");

    // typeSpecific設定読込（AraneaWebUIのrenderから呼ばれる）
    js += F("function renderTypeSpecific(){");
    js += F("let ts=cfg.typeSpecific||{};");
    js += F("document.getElementById('pulseMs').value=ts.pulseMs||3000;");
    js += F("document.getElementById('interlockMs').value=ts.interlockMs||200;");
    js += F("document.getElementById('debounceMs').value=ts.debounceMs||50;");
    js += F("document.getElementById('out1Name').value=ts.out1Name||'OPEN';");
    js += F("document.getElementById('out2Name').value=ts.out2Name||'CLOSE';");
    js += F("document.getElementById('in1Target').value=ts.in1Target||1;");
    js += F("document.getElementById('in2Target').value=ts.in2Target||2;");
    js += F("}");

    // 設定保存（固有APIはないので不可 - 基本設定はAraneaWebUI経由）
    // トリガーアサイン保存
    js += F("function saveTrigger(){");
    js += F("let cfg={");
    js += F("in1Target:parseInt(document.getElementById('in1Target').value),");
    js += F("in2Target:parseInt(document.getElementById('in2Target').value)");
    js += F("};");
    js += F("fetch('/api/is04a/trigger',{method:'POST',headers:{'Content-Type':'application/json'},");
    js += F("body:JSON.stringify(cfg)}).then(r=>r.json()).then(d=>{");
    js += F("if(d.ok){showToast('Trigger config saved');}else{showToast('Failed','error');}");
    js += F("}).catch(e=>showToast('Error','error'));}");

    js += F("</script>");
    return js;
}

String HttpManagerIs04a::generateTypeSpecificTabContents() {
    String html;

    // Triggerタブコンテンツ
    html += F("<div id=\"tab-trigger\" class=\"tab-content\">");

    // 入力状態カード
    html += F("<div class=\"card\"><div class=\"card-title\">Physical Inputs</div>");
    html += F("<div class=\"status-grid\">");
    for (int i = 1; i <= 2; i++) {
        auto state = trigger_->getInputState(i);
        html += F("<div class=\"status-item\"><div class=\"label\">INPUT ");
        html += String(i) + " (GPIO" + String(state.pin) + ")";
        html += F("</div><div class=\"value\" id=\"in");
        html += String(i) + "_state\">";
        html += state.active ? "ACTIVE" : "inactive";
        html += F("</div><div class=\"label\">Target: OUT");
        html += String(state.targetOutput);
        html += F("</div></div>");
    }
    html += F("</div></div>");

    // 出力制御カード
    html += F("<div class=\"card\"><div class=\"card-title\">Trigger Outputs</div>");
    html += F("<div class=\"status-grid\">");
    for (int i = 1; i <= 2; i++) {
        auto state = trigger_->getOutputState(i);
        html += F("<div class=\"status-item\"><div class=\"label\">");
        html += state.name + " (GPIO" + String(state.pin) + ")";
        html += F("</div><div class=\"value\" id=\"out");
        html += String(i) + "_state\">";
        html += state.active ? "PULSE" : "idle";
        html += F("</div><button class=\"btn btn-primary btn-sm\" onclick=\"firePulse(");
        html += String(i) + ")\">Fire Pulse</button></div>";
    }
    html += F("</div></div>");

    // 統計カード
    html += F("<div class=\"card\"><div class=\"card-title\">Statistics</div>");
    html += F("<div class=\"status-grid\">");
    html += F("<div class=\"status-item\"><div class=\"label\">Sent</div><div class=\"value\" id=\"sent_count\">");
    html += String(sentCount_);
    html += F("</div></div>");
    html += F("<div class=\"status-item\"><div class=\"label\">Failed</div><div class=\"value\" id=\"fail_count\">");
    html += String(failCount_);
    html += F("</div></div>");
    html += F("<div class=\"status-item\"><div class=\"label\">Last Result</div><div class=\"value\" id=\"last_result\">");
    html += lastResult_;
    html += F("</div></div>");
    html += F("</div></div>");

    // 設定カード
    html += F("<div class=\"card\"><div class=\"card-title\">Pulse Settings</div>");
    html += F("<div class=\"form-row\">");
    html += F("<div class=\"form-group\"><label>Pulse Duration (ms)</label>");
    html += "<input type=\"number\" id=\"pulseMs\" value=\"" + String(trigger_->getPulseMs()) + "\" min=\"10\" max=\"10000\">";
    html += F("</div>");
    html += F("<div class=\"form-group\"><label>Interlock (ms)</label>");
    html += "<input type=\"number\" id=\"interlockMs\" value=\"" + String(trigger_->getInterlockMs()) + "\" min=\"0\" max=\"5000\">";
    html += F("</div>");
    html += F("<div class=\"form-group\"><label>Debounce (ms)</label>");
    html += "<input type=\"number\" id=\"debounceMs\" value=\"" + String(trigger_->getDebounceMs()) + "\" min=\"5\" max=\"1000\">";
    html += F("</div>");
    html += F("</div>");
    html += F("<div class=\"form-row\">");
    html += F("<div class=\"form-group\"><label>Output 1 Name</label>");
    html += "<input type=\"text\" id=\"out1Name\" value=\"" + trigger_->getOutputName(1) + "\">";
    html += F("</div>");
    html += F("<div class=\"form-group\"><label>Output 2 Name</label>");
    html += "<input type=\"text\" id=\"out2Name\" value=\"" + trigger_->getOutputName(2) + "\">";
    html += F("</div>");
    html += F("</div>");
    html += F("</div>");

    // トリガーアサインカード
    html += F("<div class=\"card\"><div class=\"card-title\">Trigger Assign</div>");
    html += F("<div class=\"form-row\">");
    html += F("<div class=\"form-group\"><label>Input 1 -> Output</label>");
    html += F("<select id=\"in1Target\">");
    html += trigger_->getTriggerAssign(1) == 1 ? "<option value=\"1\" selected>OUT1</option>" : "<option value=\"1\">OUT1</option>";
    html += trigger_->getTriggerAssign(1) == 2 ? "<option value=\"2\" selected>OUT2</option>" : "<option value=\"2\">OUT2</option>";
    html += F("</select></div>");
    html += F("<div class=\"form-group\"><label>Input 2 -> Output</label>");
    html += F("<select id=\"in2Target\">");
    html += trigger_->getTriggerAssign(2) == 1 ? "<option value=\"1\" selected>OUT1</option>" : "<option value=\"1\">OUT1</option>";
    html += trigger_->getTriggerAssign(2) == 2 ? "<option value=\"2\" selected>OUT2</option>" : "<option value=\"2\">OUT2</option>";
    html += F("</select></div>");
    html += F("</div>");
    html += F("<button class=\"btn btn-primary\" onclick=\"saveTrigger()\">Save Trigger Assign</button>");
    html += F("</div>");

    html += F("</div>"); // tab-trigger終了

    return html;
}
