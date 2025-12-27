/**
 * HttpManagerIs06a.cpp
 */

#include "HttpManagerIs06a.h"
#include "TriggerManagerIs06a.h"
#include "SettingManager.h"
#include <ArduinoJson.h>

HttpManagerIs06a::HttpManagerIs06a()
    : trigger_(nullptr)
    , sentCount_(0)
    , failCount_(0)
    , lastResult_("---")
    , lastTriggerUpdatedAt_("")
    , lastTriggerNum_(0)
{
}

void HttpManagerIs06a::begin(SettingManager* settings, TriggerManagerIs06a* trigger, int port) {
    trigger_ = trigger;
    AraneaWebUI::begin(settings, port);
}

void HttpManagerIs06a::setSendStatus(int sent, int fail, const String& lastResult) {
    sentCount_ = sent;
    failCount_ = fail;
    lastResult_ = lastResult;
}

void HttpManagerIs06a::setTriggerStatus(int trgNum, const String& lastUpdatedAt) {
    lastTriggerNum_ = trgNum;
    lastTriggerUpdatedAt_ = lastUpdatedAt;
}

void HttpManagerIs06a::registerTypeSpecificEndpoints() {
    server_->on("/api/is06a/trigger", HTTP_POST, [this]() { handleTrigger(); });
    server_->on("/api/is06a/pwm", HTTP_POST, [this]() { handlePwm(); });
    server_->on("/api/is06a/config", HTTP_GET, [this]() { handleTriggerConfig(); });
    server_->on("/api/is06a/config", HTTP_POST, [this]() { handleTriggerConfig(); });
}

void HttpManagerIs06a::getTypeSpecificStatus(JsonObject& obj) {
    // 全トリガー状態（6CH）
    JsonArray triggers = obj.createNestedArray("triggers");
    for (int i = 1; i <= 6; i++) {
        auto state = trigger_->getState(i);
        JsonObject trg = triggers.createNestedObject();
        trg["num"] = state.trgNum;
        trg["pin"] = state.pin;
        trg["name"] = state.name;
        trg["active"] = state.active;
        trg["lastUpdatedAt"] = state.lastUpdatedAt;

        const char* modeStr = (state.mode == TriggerManagerIs06a::TriggerMode::MODE_PWM) ? "pwm" :
                              (state.mode == TriggerManagerIs06a::TriggerMode::MODE_INPUT) ? "input" : "digital";
        trg["mode"] = modeStr;

        if (state.mode == TriggerManagerIs06a::TriggerMode::MODE_PWM) {
            trg["pwmFreq"] = state.pwmFreq;
            trg["pwmDuty"] = state.pwmDuty;
        } else if (state.mode == TriggerManagerIs06a::TriggerMode::MODE_INPUT) {
            trg["debounceMs"] = state.debounceMs;
        } else {
            trg["pulseMs"] = state.pulseMs;
        }
    }

    // グローバル設定
    obj["interlockMs"] = trigger_->getInterlockMs();

    // 送信統計
    JsonObject stats = obj.createNestedObject("stats");
    stats["sent"] = sentCount_;
    stats["fail"] = failCount_;
    stats["lastResult"] = lastResult_;
}

void HttpManagerIs06a::getTypeSpecificConfig(JsonObject& obj) {
    obj["interlockMs"] = trigger_->getInterlockMs();

    JsonArray triggers = obj.createNestedArray("triggers");
    for (int i = 1; i <= 6; i++) {
        auto state = trigger_->getState(i);
        JsonObject trg = triggers.createNestedObject();
        trg["num"] = i;
        trg["name"] = state.name;
        trg["pulseMs"] = state.pulseMs;

        const char* modeStr = (state.mode == TriggerManagerIs06a::TriggerMode::MODE_PWM) ? "pwm" :
                              (state.mode == TriggerManagerIs06a::TriggerMode::MODE_INPUT) ? "input" : "digital";
        trg["mode"] = modeStr;

        if (i <= 4) {  // TRG1-4: PWM対応
            trg["pwmFreq"] = state.pwmFreq;
            trg["pwmDuty"] = state.pwmDuty;
        }
        if (i >= 5) {  // TRG5-6: I/O切替
            trg["debounceMs"] = state.debounceMs;
        }
    }
}

void HttpManagerIs06a::handleTrigger() {
    if (!checkAuth()) { requestAuth(); return; }

    int trgNum = 1;
    int durationMs = trigger_->getPulseMs(1);

    if (server_->hasArg("plain")) {
        StaticJsonDocument<256> doc;
        DeserializationError error = deserializeJson(doc, server_->arg("plain"));
        if (!error) {
            trgNum = doc["trigger"] | 1;
            durationMs = doc["duration"] | trigger_->getPulseMs(trgNum);
        }
    } else if (server_->hasArg("trigger")) {
        trgNum = server_->arg("trigger").toInt();
        if (server_->hasArg("duration")) {
            durationMs = server_->arg("duration").toInt();
        }
    }

    if (trgNum < 1 || trgNum > 6) {
        server_->send(400, "application/json", "{\"error\":\"Invalid trigger number (1-6)\"}");
        return;
    }
    if (durationMs < 10) durationMs = 10;
    if (durationMs > 10000) durationMs = 10000;

    bool success = trigger_->startPulse(trgNum, durationMs, TriggerManagerIs06a::PulseSource::HTTP);

    StaticJsonDocument<256> doc;
    doc["ok"] = success;
    doc["trigger"] = trgNum;
    doc["duration"] = durationMs;
    if (!success) {
        auto mode = trigger_->getMode(trgNum);
        if (mode == TriggerManagerIs06a::TriggerMode::MODE_INPUT) {
            doc["error"] = "Trigger is in INPUT mode";
        } else if (mode == TriggerManagerIs06a::TriggerMode::MODE_PWM) {
            doc["error"] = "Trigger is in PWM mode, use /api/is06a/pwm";
        } else if (trigger_->isAnyPulseActive()) {
            doc["error"] = "Another pulse is active";
        } else {
            doc["error"] = "Interlock active";
        }
    }

    String json;
    serializeJson(doc, json);
    server_->send(success ? 200 : 409, "application/json", json);
}

void HttpManagerIs06a::handlePwm() {
    if (!checkAuth()) { requestAuth(); return; }

    int trgNum = 1;
    int duty = -1;
    float dutyPercent = -1.0f;

    if (server_->hasArg("plain")) {
        StaticJsonDocument<256> doc;
        DeserializationError error = deserializeJson(doc, server_->arg("plain"));
        if (error) {
            server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
            return;
        }
        trgNum = doc["trigger"] | 1;
        if (doc.containsKey("duty")) {
            duty = doc["duty"];
        }
        if (doc.containsKey("duty_percent")) {
            dutyPercent = doc["duty_percent"];
        }
    } else {
        server_->send(400, "application/json", "{\"error\":\"No JSON body\"}");
        return;
    }

    if (trgNum < 1 || trgNum > 4) {
        server_->send(400, "application/json", "{\"error\":\"PWM only available on triggers 1-4\"}");
        return;
    }

    auto mode = trigger_->getMode(trgNum);
    if (mode != TriggerManagerIs06a::TriggerMode::MODE_PWM) {
        server_->send(400, "application/json", "{\"error\":\"Trigger is not in PWM mode\"}");
        return;
    }

    bool success = false;
    if (dutyPercent >= 0.0f) {
        success = trigger_->setPwmDutyPercent(trgNum, dutyPercent);
    } else if (duty >= 0) {
        success = trigger_->setPwmDuty(trgNum, duty);
    } else {
        server_->send(400, "application/json", "{\"error\":\"Specify 'duty' (0-255) or 'duty_percent' (0-100)\"}");
        return;
    }

    StaticJsonDocument<128> doc;
    doc["ok"] = success;
    doc["trigger"] = trgNum;
    doc["duty"] = trigger_->getState(trgNum).pwmDuty;

    String json;
    serializeJson(doc, json);
    server_->send(success ? 200 : 500, "application/json", json);
}

void HttpManagerIs06a::handleTriggerConfig() {
    if (!checkAuth()) { requestAuth(); return; }

    if (server_->method() == HTTP_GET) {
        StaticJsonDocument<1536> doc;
        doc["interlockMs"] = trigger_->getInterlockMs();

        JsonArray triggers = doc.createNestedArray("triggers");
        for (int i = 1; i <= 6; i++) {
            auto state = trigger_->getState(i);
            JsonObject trg = triggers.createNestedObject();
            trg["num"] = i;
            trg["name"] = state.name;
            trg["pulseMs"] = state.pulseMs;

            const char* modeStr = (state.mode == TriggerManagerIs06a::TriggerMode::MODE_PWM) ? "pwm" :
                                  (state.mode == TriggerManagerIs06a::TriggerMode::MODE_INPUT) ? "input" : "digital";
            trg["mode"] = modeStr;

            if (i <= 4) {  // TRG1-4: PWM対応
                trg["pwmFreq"] = state.pwmFreq;
                trg["pwmDuty"] = state.pwmDuty;
            }
            if (i >= 5) {  // TRG5-6: I/O切替
                trg["debounceMs"] = state.debounceMs;
            }
        }

        String json;
        serializeJson(doc, json);
        server_->send(200, "application/json", json);
    } else {
        if (!server_->hasArg("plain")) {
            server_->send(400, "application/json", "{\"error\":\"No body\"}");
            return;
        }

        StaticJsonDocument<1024> doc;
        DeserializationError error = deserializeJson(doc, server_->arg("plain"));
        if (error) {
            server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
            return;
        }

        // グローバル設定
        if (doc.containsKey("interlockMs")) {
            trigger_->setInterlockMs(doc["interlockMs"]);
        }

        // 個別トリガー設定（6CH）
        if (doc.containsKey("triggers")) {
            JsonArray triggers = doc["triggers"].as<JsonArray>();
            for (JsonObject trg : triggers) {
                int num = trg["num"] | 0;
                if (num < 1 || num > 6) continue;

                if (trg.containsKey("name")) {
                    trigger_->setName(num, trg["name"].as<String>());
                }
                if (trg.containsKey("pulseMs")) {
                    trigger_->setPulseMs(num, trg["pulseMs"]);
                }
                if (trg.containsKey("mode")) {
                    String modeStr = trg["mode"].as<String>();
                    if (modeStr == "pwm" && num <= 4) {  // TRG1-4: PWM対応
                        trigger_->setMode(num, TriggerManagerIs06a::TriggerMode::MODE_PWM);
                    } else if (modeStr == "input" && num >= 5) {  // TRG5-6: I/O切替
                        trigger_->setMode(num, TriggerManagerIs06a::TriggerMode::MODE_INPUT);
                    } else if (modeStr == "digital") {
                        trigger_->setMode(num, TriggerManagerIs06a::TriggerMode::MODE_DIGITAL);
                    }
                }
                if (trg.containsKey("pwmFreq") && num <= 4) {  // TRG1-4: PWM対応
                    trigger_->setPwmFreq(num, trg["pwmFreq"]);
                }
                if (trg.containsKey("debounceMs") && num >= 5) {  // TRG5-6: I/O切替
                    trigger_->setDebounceMs(num, trg["debounceMs"]);
                }
            }
        }

        server_->send(200, "application/json", "{\"ok\":true}");
    }
}

String HttpManagerIs06a::generateTypeSpecificTabs() {
    return F("<div class=\"tab\" data-tab=\"triggers\" onclick=\"showTab('triggers')\">Triggers</div>");
}

String HttpManagerIs06a::generateTypeSpecificJS() {
    String js;
    js += F("<script>");

    // パルス実行
    js += F("function firePulse(n){fetch('/api/is06a/trigger',{method:'POST',");
    js += F("headers:{'Content-Type':'application/json'},body:JSON.stringify({trigger:n})})");
    js += F(".then(r=>r.json()).then(d=>{");
    js += F("if(d.ok){showToast('Pulse TRG'+n+' executed');}else{showToast('Failed: '+d.error,'error');}");
    js += F("}).catch(e=>showToast('Error','error'));}");

    // PWM設定
    js += F("function setPwm(n){let v=document.getElementById('pwm'+n).value;");
    js += F("fetch('/api/is06a/pwm',{method:'POST',");
    js += F("headers:{'Content-Type':'application/json'},body:JSON.stringify({trigger:n,duty:parseInt(v)})})");
    js += F(".then(r=>r.json()).then(d=>{");
    js += F("if(d.ok){showToast('PWM TRG'+n+' set to '+d.duty);}else{showToast('Failed: '+d.error,'error');}");
    js += F("}).catch(e=>showToast('Error','error'));}");

    // ステータス更新
    js += F("function refreshTypeSpecificStatus(s){");
    js += F("let ts=s.typeSpecific||{};");
    js += F("let triggers=ts.triggers||[];");
    js += F("for(let t of triggers){");
    js += F("let el=document.getElementById('trg'+t.num+'_state');");
    js += F("if(el){");
    js += F("if(t.mode==='input'){el.textContent=t.active?'ACTIVE':'inactive';el.className=t.active?'value good':'value';}");
    js += F("else if(t.mode==='pwm'){el.textContent='PWM:'+t.pwmDuty;el.className=t.active?'value warn':'value';}");
    js += F("else{el.textContent=t.active?'PULSE':'idle';el.className=t.active?'value warn':'value';}");
    js += F("}}");
    js += F("let st=ts.stats||{};");
    js += F("let se=document.getElementById('sent_count');if(se)se.textContent=st.sent||0;");
    js += F("let fe=document.getElementById('fail_count');if(fe)fe.textContent=st.fail||0;");
    js += F("let le=document.getElementById('last_result');if(le)le.textContent=st.lastResult||'---';");
    js += F("}");

    // 設定読込
    js += F("function renderTypeSpecific(){");
    js += F("let ts=cfg.typeSpecific||{};");
    js += F("document.getElementById('interlockMs').value=ts.interlockMs||200;");
    js += F("let triggers=ts.triggers||[];");
    js += F("for(let t of triggers){");
    js += F("let nm=document.getElementById('trg'+t.num+'_name');if(nm)nm.value=t.name||'TRG'+t.num;");
    js += F("let pm=document.getElementById('trg'+t.num+'_pulse');if(pm)pm.value=t.pulseMs||3000;");
    js += F("let mo=document.getElementById('trg'+t.num+'_mode');if(mo)mo.value=t.mode||'digital';");
    js += F("if(t.num<=4){let pf=document.getElementById('trg'+t.num+'_freq');if(pf)pf.value=t.pwmFreq||1000;}");
    js += F("if(t.num>=5){let db=document.getElementById('trg'+t.num+'_deb');if(db)db.value=t.debounceMs||100;}");
    js += F("}}");

    // 設定保存
    js += F("function saveConfig(){");
    js += F("let cfg={interlockMs:parseInt(document.getElementById('interlockMs').value),triggers:[]};");
    js += F("for(let i=1;i<=6;i++){");
    js += F("let t={num:i};");
    js += F("let nm=document.getElementById('trg'+i+'_name');if(nm)t.name=nm.value;");
    js += F("let pm=document.getElementById('trg'+i+'_pulse');if(pm)t.pulseMs=parseInt(pm.value);");
    js += F("let mo=document.getElementById('trg'+i+'_mode');if(mo)t.mode=mo.value;");
    js += F("if(i<=4){let pf=document.getElementById('trg'+i+'_freq');if(pf)t.pwmFreq=parseInt(pf.value);}");
    js += F("if(i>=5){let db=document.getElementById('trg'+i+'_deb');if(db)t.debounceMs=parseInt(db.value);}");
    js += F("cfg.triggers.push(t);}");
    js += F("fetch('/api/is06a/config',{method:'POST',headers:{'Content-Type':'application/json'},");
    js += F("body:JSON.stringify(cfg)}).then(r=>r.json()).then(d=>{");
    js += F("if(d.ok){showToast('Settings saved');}else{showToast('Failed','error');}");
    js += F("}).catch(e=>showToast('Error','error'));}");

    js += F("</script>");
    return js;
}

String HttpManagerIs06a::generateTypeSpecificTabContents() {
    String html;

    html += F("<div id=\"tab-triggers\" class=\"tab-content\">");

    // TRG1-4 (PWM対応)
    html += F("<div class=\"card\"><div class=\"card-title\">Triggers 1-4 (PWM Capable)</div>");
    html += F("<div class=\"status-grid\">");
    for (int i = 1; i <= 4; i++) {
        auto state = trigger_->getState(i);
        html += F("<div class=\"status-item\"><div class=\"label\">");
        html += state.name + " (GPIO" + String(state.pin) + ")";
        html += F("</div><div class=\"value\" id=\"trg");
        html += String(i) + "_state\">";
        if (state.mode == TriggerManagerIs06a::TriggerMode::MODE_PWM) {
            html += "PWM:" + String(state.pwmDuty);
        } else {
            html += state.active ? "PULSE" : "idle";
        }
        html += F("</div>");
        if (state.mode == TriggerManagerIs06a::TriggerMode::MODE_PWM) {
            html += "<input type=\"range\" id=\"pwm" + String(i) + "\" min=\"0\" max=\"255\" value=\"" + String(state.pwmDuty) + "\" style=\"width:80px;\">";
            html += "<button class=\"btn btn-sm\" onclick=\"setPwm(" + String(i) + ")\">Set</button>";
        } else {
            html += "<button class=\"btn btn-primary btn-sm\" onclick=\"firePulse(" + String(i) + ")\">Fire</button>";
        }
        html += F("</div>");
    }
    html += F("</div></div>");

    // TRG5-6 (I/O切替可能)
    html += F("<div class=\"card\"><div class=\"card-title\">Triggers 5-6 (I/O Switchable)</div>");
    html += F("<div class=\"status-grid\">");
    for (int i = 5; i <= 6; i++) {
        auto state = trigger_->getState(i);
        html += F("<div class=\"status-item\"><div class=\"label\">");
        html += state.name + " (GPIO" + String(state.pin) + ")";
        html += F("</div><div class=\"value\" id=\"trg");
        html += String(i) + "_state\">";
        if (state.mode == TriggerManagerIs06a::TriggerMode::MODE_INPUT) {
            html += state.active ? "ACTIVE" : "inactive";
        } else {
            html += state.active ? "PULSE" : "idle";
        }
        html += F("</div>");
        if (state.mode != TriggerManagerIs06a::TriggerMode::MODE_INPUT) {
            html += "<button class=\"btn btn-primary btn-sm\" onclick=\"firePulse(" + String(i) + ")\">Fire</button>";
        }
        html += F("</div>");
    }
    html += F("</div></div>");

    // 統計
    html += F("<div class=\"card\"><div class=\"card-title\">Statistics</div>");
    html += F("<div class=\"status-grid\">");
    html += F("<div class=\"status-item\"><div class=\"label\">Sent</div><div class=\"value\" id=\"sent_count\">");
    html += String(sentCount_) + F("</div></div>");
    html += F("<div class=\"status-item\"><div class=\"label\">Failed</div><div class=\"value\" id=\"fail_count\">");
    html += String(failCount_) + F("</div></div>");
    html += F("<div class=\"status-item\"><div class=\"label\">Last Result</div><div class=\"value\" id=\"last_result\">");
    html += lastResult_ + F("</div></div>");
    html += F("</div></div>");

    // 設定
    html += F("<div class=\"card\"><div class=\"card-title\">Trigger Configuration</div>");

    // グローバル設定
    html += F("<div class=\"form-row\"><div class=\"form-group\"><label>Interlock (ms)</label>");
    html += "<input type=\"number\" id=\"interlockMs\" value=\"" + String(trigger_->getInterlockMs()) + "\" min=\"0\" max=\"5000\">";
    html += F("</div></div>");

    // TRG1-4設定
    html += F("<h4>Triggers 1-4</h4>");
    for (int i = 1; i <= 4; i++) {
        auto state = trigger_->getState(i);
        html += F("<div class=\"form-row\">");
        html += "<div class=\"form-group\"><label>TRG" + String(i) + " Name</label>";
        html += "<input type=\"text\" id=\"trg" + String(i) + "_name\" value=\"" + state.name + "\" style=\"width:80px;\">";
        html += F("</div>");
        html += "<div class=\"form-group\"><label>Mode</label><select id=\"trg" + String(i) + "_mode\">";
        html += (state.mode == TriggerManagerIs06a::TriggerMode::MODE_DIGITAL) ? "<option value=\"digital\" selected>Digital</option>" : "<option value=\"digital\">Digital</option>";
        html += (state.mode == TriggerManagerIs06a::TriggerMode::MODE_PWM) ? "<option value=\"pwm\" selected>PWM</option>" : "<option value=\"pwm\">PWM</option>";
        html += F("</select></div>");
        html += "<div class=\"form-group\"><label>Pulse(ms)</label>";
        html += "<input type=\"number\" id=\"trg" + String(i) + "_pulse\" value=\"" + String(state.pulseMs) + "\" min=\"10\" max=\"10000\" style=\"width:70px;\">";
        html += F("</div>");
        html += "<div class=\"form-group\"><label>PWM Freq</label>";
        html += "<input type=\"number\" id=\"trg" + String(i) + "_freq\" value=\"" + String(state.pwmFreq) + "\" min=\"1000\" max=\"40000\" style=\"width:70px;\">";
        html += F("</div></div>");
    }

    // TRG5-6設定
    html += F("<h4>Triggers 5-6</h4>");
    for (int i = 5; i <= 6; i++) {
        auto state = trigger_->getState(i);
        html += F("<div class=\"form-row\">");
        html += "<div class=\"form-group\"><label>TRG" + String(i) + " Name</label>";
        html += "<input type=\"text\" id=\"trg" + String(i) + "_name\" value=\"" + state.name + "\" style=\"width:80px;\">";
        html += F("</div>");
        html += "<div class=\"form-group\"><label>I/O Mode</label><select id=\"trg" + String(i) + "_mode\">";
        html += (state.mode == TriggerManagerIs06a::TriggerMode::MODE_DIGITAL) ? "<option value=\"digital\" selected>Output</option>" : "<option value=\"digital\">Output</option>";
        html += (state.mode == TriggerManagerIs06a::TriggerMode::MODE_INPUT) ? "<option value=\"input\" selected>Input</option>" : "<option value=\"input\">Input</option>";
        html += F("</select></div>");
        html += "<div class=\"form-group\"><label>Pulse(ms)</label>";
        html += "<input type=\"number\" id=\"trg" + String(i) + "_pulse\" value=\"" + String(state.pulseMs) + "\" min=\"10\" max=\"10000\" style=\"width:70px;\">";
        html += F("</div>");
        html += "<div class=\"form-group\"><label>Debounce</label>";
        html += "<input type=\"number\" id=\"trg" + String(i) + "_deb\" value=\"" + String(state.debounceMs) + "\" min=\"5\" max=\"1000\" style=\"width:70px;\">";
        html += F("</div></div>");
    }

    html += F("<button class=\"btn btn-primary\" onclick=\"saveConfig()\">Save All Settings</button>");
    html += F("</div>");

    html += F("</div>"); // tab-triggers

    return html;
}
