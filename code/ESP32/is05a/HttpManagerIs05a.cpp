/**
 * HttpManagerIs05a.cpp
 */

#include "HttpManagerIs05a.h"
#include "ChannelManager.h"
#include "WebhookManager.h"
#include "RuleManager.h"
#include "Is05aKeys.h"
#include <ArduinoJson.h>

HttpManagerIs05a::HttpManagerIs05a()
    : AraneaWebUI()
    , channels_(nullptr)
    , webhooks_(nullptr)
    , rules_(nullptr)
    , lastChangedChannel_(-1)
    , sendSuccessCount_(0)
    , sendFailCount_(0)
    , lastSendResult_("---")
{
}

void HttpManagerIs05a::begin(SettingManager* settings, ChannelManager* channels,
                              WebhookManager* webhooks, RuleManager* rules, int port) {
    channels_ = channels;
    webhooks_ = webhooks;
    rules_ = rules;

    // 基底クラス初期化
    AraneaWebUI::begin(settings, port);

    Serial.println("[HttpManagerIs05a] Initialized");
}

void HttpManagerIs05a::setChannelStatus(int lastChanged, const String& lastChangeTime) {
    lastChangedChannel_ = lastChanged;
    lastChangeTime_ = lastChangeTime;
}

void HttpManagerIs05a::setSendStatus(int successCount, int failCount, const String& lastResult) {
    sendSuccessCount_ = successCount;
    sendFailCount_ = failCount;
    lastSendResult_ = lastResult;
}

void HttpManagerIs05a::getTypeSpecificStatus(JsonObject& obj) {
    // チャンネル状態
    JsonArray channelsArr = obj.createNestedArray("channels");
    for (int ch = 1; ch <= 8; ch++) {
        JsonObject chObj = channelsArr.createNestedObject();
        chObj["ch"] = ch;
        chObj["name"] = channels_->getConfig(ch).name;
        chObj["state"] = channels_->getStateString(ch);
        chObj["isOutput"] = channels_->isOutputMode(ch);
        chObj["lastUpdatedAt"] = channels_->getLastUpdatedAt(ch);
    }

    // Webhook状態
    obj["webhookEnabled"] = webhooks_->isEnabled();
    obj["webhookSent"] = webhooks_->getSentCount();
    obj["webhookFail"] = webhooks_->getFailCount();

    // 送信状態
    obj["lastChangedChannel"] = lastChangedChannel_;
    obj["lastChangeTime"] = lastChangeTime_;
    obj["sendResult"] = lastSendResult_;
}

String HttpManagerIs05a::generateTypeSpecificTabs() {
    String html;
    html += F("<div class=\"tab\" data-tab=\"channels\" onclick=\"showTab('channels')\">Channels</div>");
    html += F("<div class=\"tab\" data-tab=\"webhook\" onclick=\"showTab('webhook')\">Webhook</div>");
    html += F("<div class=\"tab\" data-tab=\"rules\" onclick=\"showTab('rules')\">Rules</div>");
    return html;
}

String HttpManagerIs05a::generateTypeSpecificTabContents() {
    String html;
    // Channels Tab
    html += F("<div id=\"tab-channels\" class=\"tab-content\">");
    html += F("<h3>8ch Channel Status</h3>");
    html += F("<div id=\"channel-list\"></div>");
    html += F("<h4>ch7/ch8 Output Control</h4>");
    html += F("<div class=\"form-group\"><label>ch7 Mode:</label>");
    html += F("<select id=\"ch7-mode\" onchange=\"setChannelMode(7, this.value)\">");
    html += F("<option value=\"input\">Input</option><option value=\"output\">Output</option></select>");
    html += F("<button onclick=\"sendPulse(7)\">Pulse (3s)</button></div>");
    html += F("<div class=\"form-group\"><label>ch8 Mode:</label>");
    html += F("<select id=\"ch8-mode\" onchange=\"setChannelMode(8, this.value)\">");
    html += F("<option value=\"input\">Input</option><option value=\"output\">Output</option></select>");
    html += F("<button onclick=\"sendPulse(8)\">Pulse (3s)</button></div></div>");
    // Webhook Tab
    html += F("<div id=\"tab-webhook\" class=\"tab-content\">");
    html += F("<h3>Webhook Configuration</h3><form id=\"webhook-form\">");
    html += F("<div class=\"form-group\"><label><input type=\"checkbox\" id=\"webhook-enabled\"> Enable Webhook</label></div>");
    html += F("<div class=\"form-group\"><label>Discord URL:</label>");
    html += F("<input type=\"text\" id=\"discord-url\" placeholder=\"https://discord.com/api/webhooks/...\">");
    html += F("<button type=\"button\" onclick=\"testWebhook('discord')\">Test</button></div>");
    html += F("<div class=\"form-group\"><label>Slack URL:</label>");
    html += F("<input type=\"text\" id=\"slack-url\" placeholder=\"https://hooks.slack.com/services/...\">");
    html += F("<button type=\"button\" onclick=\"testWebhook('slack')\">Test</button></div>");
    html += F("<div class=\"form-group\"><label>Generic URL:</label>");
    html += F("<input type=\"text\" id=\"generic-url\" placeholder=\"https://your-server.com/webhook\">");
    html += F("<button type=\"button\" onclick=\"testWebhook('generic')\">Test</button></div>");
    html += F("<button type=\"submit\">Save Webhook Settings</button></form></div>");

    // Rules Tab
    html += F("<div id=\"tab-rules\" class=\"tab-content\">");
    html += F("<h3>Automation Rules</h3>");
    html += F("<p>条件: 入力ch → 出力(ch7/ch8)パルス + Webhook</p>");
    html += F("<div id=\"rule-list\"></div>");
    html += F("<h4>Add / Update Rule</h4>");
    html += F("<div class=\"form-row\">");
    html += F("<div class=\"form-group\"><label>Rule ID (0-7)</label><input type=\"number\" id=\"rule-id\" min=\"0\" max=\"7\" value=\"0\"></div>");
    html += F("<div class=\"form-group\"><label>Enabled</label><input type=\"checkbox\" id=\"rule-enabled\" checked></div>");
    html += F("</div>");
    html += F("<div class=\"form-group\"><label>Input Channels (comma)</label><input type=\"text\" id=\"rule-src\" placeholder=\"1,2,3\"></div>");
    html += F("<div class=\"form-group\"><label>State</label><select id=\"rule-state\"><option value=\"any\">Any</option><option value=\"active\">Active</option><option value=\"inactive\">Inactive</option></select></div>");
    html += F("<div class=\"form-group\"><label>Output Channels (7,8 comma)</label><input type=\"text\" id=\"rule-out\" placeholder=\"7\"></div>");
    html += F("<div class=\"form-row\">");
    html += F("<div class=\"form-group\"><label>Pulse (ms)</label><input type=\"number\" id=\"rule-pulse\" value=\"3000\" min=\"5\" max=\"10000\"></div>");
    html += F("<div class=\"form-group\"><label>Cooldown (ms)</label><input type=\"number\" id=\"rule-cooldown\" value=\"0\" min=\"0\" max=\"600000\"></div>");
    html += F("</div>");
    html += F("<div class=\"form-group\"><label>Webhook</label>");
    html += F("<label><input type=\"checkbox\" id=\"rule-wh-discord\"> Discord</label> ");
    html += F("<label><input type=\"checkbox\" id=\"rule-wh-slack\"> Slack</label> ");
    html += F("<label><input type=\"checkbox\" id=\"rule-wh-generic\"> Generic</label></div>");
    html += F("<div class=\"btn-group\"><button class=\"btn\" onclick=\"saveRule()\">Save Rule</button>");
    html += F("<button class=\"btn btn-danger\" onclick=\"deleteRule()\">Delete Rule</button></div>");
    html += F("</div>");
    return html;
}

String HttpManagerIs05a::generateTypeSpecificJS() {
    String js;
    js += F("<script>");
    js += F("function loadChannels(){fetch('/api/channels').then(r=>r.json()).then(data=>{");
    js += F("let html='<table><tr><th>Ch</th><th>Name</th><th>State</th><th>Mode</th><th>Updated</th></tr>';");
    js += F("data.channels.forEach(ch=>{html+='<tr><td>ch'+ch.ch+'</td><td>'+ch.name+'</td><td>'+ch.state+'</td>';");
    js += F("html+='<td>'+(ch.isOutput?'OUTPUT':'INPUT')+'</td><td>'+ch.lastUpdatedAt+'</td></tr>';});");
    js += F("html+='</table>';document.getElementById('channel-list').innerHTML=html;");
    js += F("data.channels.forEach(ch=>{if(ch.ch===7)document.getElementById('ch7-mode').value=ch.isOutput?'output':'input';");
    js += F("if(ch.ch===8)document.getElementById('ch8-mode').value=ch.isOutput?'output':'input';});});}");
    js += F("function setChannelMode(ch,mode){fetch('/api/channel',{method:'POST',");
    js += F("headers:{'Content-Type':'application/json'},body:JSON.stringify({ch:ch,mode:mode})})");
    js += F(".then(r=>r.json()).then(data=>{if(data.ok){showMessage('ch'+ch+' mode changed to '+mode);loadChannels();}");
    js += F("else{showMessage('Error: '+data.error,'error');}});}");
    js += F("function sendPulse(ch){fetch('/api/pulse',{method:'POST',headers:{'Content-Type':'application/json'},");
    js += F("body:JSON.stringify({channel:ch,duration:3000})}).then(r=>r.json()).then(data=>{");
    js += F("if(data.ok){showMessage('Pulse sent to ch'+ch);}else{showMessage('Error: '+data.error,'error');}});}");
    js += F("function loadWebhookConfig(){fetch('/api/webhook/config').then(r=>r.json()).then(data=>{");
    js += F("document.getElementById('webhook-enabled').checked=data.enabled;");
    js += F("document.getElementById('discord-url').value=data.discordUrl||'';");
    js += F("document.getElementById('slack-url').value=data.slackUrl||'';");
    js += F("document.getElementById('generic-url').value=data.genericUrl||'';});}");
    js += F("function testWebhook(platform){fetch('/api/webhook/test',{method:'POST',");
    js += F("headers:{'Content-Type':'application/json'},body:JSON.stringify({platform:platform})})");
    js += F(".then(r=>r.json()).then(data=>{if(data.ok){showMessage(platform+' webhook test sent');}");
    js += F("else{showMessage('Error: '+data.error,'error');}});}");
    js += F("document.getElementById('webhook-form').addEventListener('submit',function(e){e.preventDefault();");
    js += F("fetch('/api/webhook/config',{method:'POST',headers:{'Content-Type':'application/json'},");
    js += F("body:JSON.stringify({enabled:document.getElementById('webhook-enabled').checked,");
    js += F("discordUrl:document.getElementById('discord-url').value,");
    js += F("slackUrl:document.getElementById('slack-url').value,");
    js += F("genericUrl:document.getElementById('generic-url').value})})");
    js += F(".then(r=>r.json()).then(data=>{if(data.ok){showMessage('Webhook settings saved');}");
    js += F("else{showMessage('Error saving settings','error');}});});");
    js += F("var origShowTab=showTab;showTab=function(tab){origShowTab(tab);");
    js += F("if(tab==='channels')loadChannels();if(tab==='webhook')loadWebhookConfig();if(tab==='rules')loadRules();};");
    js += F("function parseChList(v){return v.split(',').map(x=>parseInt(x.trim())).filter(x=>!isNaN(x));}");
    js += F("function loadRules(){fetch('/api/rules').then(r=>r.json()).then(data=>{let html='<table><tr><th>ID</th><th>Enabled</th><th>Src</th><th>State</th><th>Out</th><th>Pulse</th><th>Cooldown</th><th>Webhook</th></tr>';data.rules.forEach(r=>{");
    js += F("html+='<tr><td>'+r.id+'</td><td>'+(r.enabled?'ON':'OFF')+'</td><td>'+r.src.join(',')+'</td><td>'+r.state+'</td><td>'+r.outputs.join(',')+'</td><td>'+r.pulseMs+'</td><td>'+r.cooldownMs+'</td>';let wh=[];if(r.webhook.discord)wh.push('D');if(r.webhook.slack)wh.push('S');if(r.webhook.generic)wh.push('G');html+='<td>'+wh.join('/')+'</td></tr>';});html+='</table>';document.getElementById('rule-list').innerHTML=html;});}");
    js += F("function saveRule(){let id=parseInt(document.getElementById('rule-id').value)||0;let body={id:id,enabled:document.getElementById('rule-enabled').checked,src:parseChList(document.getElementById('rule-src').value),state:document.getElementById('rule-state').value,outputs:parseChList(document.getElementById('rule-out').value),pulseMs:parseInt(document.getElementById('rule-pulse').value)||3000,cooldownMs:parseInt(document.getElementById('rule-cooldown').value)||0,webhook:{discord:document.getElementById('rule-wh-discord').checked,slack:document.getElementById('rule-wh-slack').checked,generic:document.getElementById('rule-wh-generic').checked}};fetch('/api/rules',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}).then(r=>r.json()).then(res=>{if(res.ok){showMessage('Rule saved');loadRules();}else{showMessage(res.error||'Save failed','error');}});}");
    js += F("function deleteRule(){let id=parseInt(document.getElementById('rule-id').value)||0;if(!confirm('Delete rule '+id+'?'))return;fetch('/api/rules/'+id,{method:'DELETE'}).then(r=>r.json()).then(res=>{if(res.ok){showMessage('Rule deleted');loadRules();}else{showMessage(res.error||'Delete failed','error');}});}");
    js += F("setInterval(function(){var el=document.getElementById('tab-channels');if(el&&el.classList.contains('active'))loadChannels();},2000);");
    js += F("</script>");
    return js;
}

void HttpManagerIs05a::registerTypeSpecificEndpoints() {
    // チャンネル状態取得
    server_->on("/api/channels", HTTP_GET, [this]() { handleChannels(); });

    // 個別チャンネル設定
    server_->on("/api/channel", HTTP_GET, [this]() { handleChannel(); });
    server_->on("/api/channel", HTTP_POST, [this]() { handleChannelConfig(); });

    // パルス出力
    server_->on("/api/pulse", HTTP_POST, [this]() { handlePulse(); });

    // Webhook設定
    server_->on("/api/webhook/config", HTTP_GET, [this]() { handleWebhookConfig(); });
    server_->on("/api/webhook/config", HTTP_POST, [this]() { handleWebhookConfig(); });
    server_->on("/api/webhook/test", HTTP_POST, [this]() { handleWebhookTest(); });

    // Rules
    server_->on("/api/rules", HTTP_GET, [this]() { handleRules(); });
    server_->on("/api/rules", HTTP_POST, [this]() { handleRuleSave(); });
    server_->on("/api/rules/0", HTTP_DELETE, [this]() { handleRuleDelete(); });
    server_->on("/api/rules/1", HTTP_DELETE, [this]() { handleRuleDelete(); });
    server_->on("/api/rules/2", HTTP_DELETE, [this]() { handleRuleDelete(); });
    server_->on("/api/rules/3", HTTP_DELETE, [this]() { handleRuleDelete(); });
    server_->on("/api/rules/4", HTTP_DELETE, [this]() { handleRuleDelete(); });
    server_->on("/api/rules/5", HTTP_DELETE, [this]() { handleRuleDelete(); });
    server_->on("/api/rules/6", HTTP_DELETE, [this]() { handleRuleDelete(); });
    server_->on("/api/rules/7", HTTP_DELETE, [this]() { handleRuleDelete(); });

    Serial.println("[HttpManagerIs05a] Endpoints registered");
}

void HttpManagerIs05a::getTypeSpecificConfig(JsonObject& obj) {
    // チャンネル設定
    JsonArray channelsArr = obj.createNestedArray("channels");
    for (int ch = 1; ch <= 8; ch++) {
        JsonObject chObj = channelsArr.createNestedObject();
        auto cfg = channels_->getConfig(ch);
        chObj["ch"] = ch;
        chObj["pin"] = cfg.pin;
        chObj["name"] = cfg.name;
        chObj["meaning"] = cfg.meaning;
        chObj["debounce"] = cfg.debounceMs;
        chObj["inverted"] = cfg.inverted;
        chObj["isOutput"] = cfg.isOutput;
    }

    // Webhook設定
    obj["webhookEnabled"] = webhooks_->isEnabled();
    obj["discordUrl"] = webhooks_->getDiscordUrl();
    obj["slackUrl"] = webhooks_->getSlackUrl();
    obj["genericUrl"] = webhooks_->getGenericUrl();
}

void HttpManagerIs05a::handleChannels() {
    if (!checkAuth()) { requestAuth(); return; }

    DynamicJsonDocument doc(2048);
    JsonArray arr = doc.createNestedArray("channels");

    for (int ch = 1; ch <= 8; ch++) {
        JsonObject chObj = arr.createNestedObject();
        auto cfg = channels_->getConfig(ch);
        chObj["ch"] = ch;
        chObj["name"] = cfg.name;
        chObj["state"] = channels_->getStateString(ch);
        chObj["isOutput"] = channels_->isOutputMode(ch);
        chObj["lastUpdatedAt"] = channels_->getLastUpdatedAt(ch);
    }

    String json;
    serializeJson(doc, json);
    server_->send(200, "application/json", json);
}

void HttpManagerIs05a::handleChannel() {
    if (!checkAuth()) { requestAuth(); return; }

    String chStr = server_->arg("ch");
    int ch = chStr.toInt();

    if (ch < 1 || ch > 8) {
        server_->send(400, "application/json", "{\"error\":\"Invalid channel\"}");
        return;
    }

    server_->send(200, "application/json", channels_->getChannelJson(ch));
}

void HttpManagerIs05a::handleChannelConfig() {
    if (!checkAuth()) { requestAuth(); return; }

    DynamicJsonDocument doc(512);
    DeserializationError error = deserializeJson(doc, server_->arg("plain"));

    if (error) {
        server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
        return;
    }

    int ch = doc["ch"] | 0;
    if (ch < 1 || ch > 8) {
        server_->send(400, "application/json", "{\"error\":\"Invalid channel\"}");
        return;
    }

    // モード変更
    if (doc.containsKey("mode")) {
        String mode = doc["mode"].as<String>();
        bool output = (mode == "output");
        if (!channels_->setOutputMode(ch, output)) {
            server_->send(400, "application/json", "{\"error\":\"Cannot change mode\"}");
            return;
        }
    }

    // その他の設定
    auto cfg = channels_->getConfig(ch);
    if (doc.containsKey("name")) cfg.name = doc["name"].as<String>();
    if (doc.containsKey("meaning")) cfg.meaning = doc["meaning"].as<String>();
    if (doc.containsKey("debounce")) cfg.debounceMs = doc["debounce"];
    if (doc.containsKey("inverted")) cfg.inverted = doc["inverted"];
    channels_->setConfig(ch, cfg);

    server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs05a::handlePulse() {
    if (!checkAuth()) { requestAuth(); return; }

    DynamicJsonDocument doc(256);
    DeserializationError error = deserializeJson(doc, server_->arg("plain"));

    if (error) {
        server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
        return;
    }

    int ch = doc["channel"] | 0;
    int duration = doc["duration"] | 3000;

    if (ch < 7 || ch > 8) {
        server_->send(400, "application/json", "{\"error\":\"Only ch7/ch8 support pulse\"}");
        return;
    }

    if (!channels_->isOutputMode(ch)) {
        server_->send(400, "application/json", "{\"error\":\"Channel not in output mode\"}");
        return;
    }

    channels_->pulse(ch, duration);
    server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs05a::handleWebhookConfig() {
    if (!checkAuth()) { requestAuth(); return; }

    if (server_->method() == HTTP_GET) {
        DynamicJsonDocument doc(1024);
        doc["enabled"] = webhooks_->isEnabled();
        doc["discordUrl"] = webhooks_->getDiscordUrl();
        doc["slackUrl"] = webhooks_->getSlackUrl();
        doc["genericUrl"] = webhooks_->getGenericUrl();

        String json;
        serializeJson(doc, json);
        server_->send(200, "application/json", json);
        return;
    }

    // POST
    DynamicJsonDocument doc(1024);
    DeserializationError error = deserializeJson(doc, server_->arg("plain"));

    if (error) {
        server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
        return;
    }

    if (doc.containsKey("enabled")) webhooks_->setEnabled(doc["enabled"]);
    if (doc.containsKey("discordUrl")) webhooks_->setDiscordUrl(doc["discordUrl"].as<String>());
    if (doc.containsKey("slackUrl")) webhooks_->setSlackUrl(doc["slackUrl"].as<String>());
    if (doc.containsKey("genericUrl")) webhooks_->setGenericUrl(doc["genericUrl"].as<String>());

    server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs05a::handleWebhookTest() {
    if (!checkAuth()) { requestAuth(); return; }

    DynamicJsonDocument doc(256);
    DeserializationError error = deserializeJson(doc, server_->arg("plain"));

    if (error) {
        server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
        return;
    }

    String platform = doc["platform"].as<String>();

    // プラットフォーム別にテスト送信
    WebhookManager::Platform targetPlatform;
    if (platform == "discord") {
        targetPlatform = WebhookManager::Platform::DISCORD;
    } else if (platform == "slack") {
        targetPlatform = WebhookManager::Platform::SLACK;
    } else if (platform == "generic") {
        targetPlatform = WebhookManager::Platform::GENERIC;
    } else {
        server_->send(400, "application/json", "{\"error\":\"Invalid platform\"}");
        return;
    }

    bool ok = webhooks_->sendTestTo(targetPlatform);

    if (ok) {
        server_->send(200, "application/json", "{\"ok\":true}");
    } else {
        server_->send(500, "application/json", "{\"ok\":false,\"error\":\"Send failed\"}");
    }
}

static void maskToArray(uint8_t mask, int startCh, int endCh, JsonArray arr) {
    for (int ch = startCh; ch <= endCh; ch++) {
        if (mask & (1 << (ch - 1))) {
            arr.add(ch);
        }
    }
}

void HttpManagerIs05a::handleRules() {
    if (!checkAuth()) { requestAuth(); return; }
    if (!rules_) {
        server_->send(500, "application/json", "{\"error\":\"Rule manager not initialized\"}");
        return;
    }

    DynamicJsonDocument doc(2048);
    JsonArray arr = doc.createNestedArray("rules");

    for (int i = 0; i < rules_->getRuleCount(); i++) {
        RuleManager::Rule r = rules_->getRule(i);
        JsonObject obj = arr.createNestedObject();
        obj["id"] = i;
        obj["enabled"] = r.enabled;
        obj["pulseMs"] = r.pulseMs;
        obj["cooldownMs"] = r.cooldownMs;
        String stateStr = "any";
        if (r.stateCond == RuleManager::StateCond::ACTIVE) stateStr = "active";
        else if (r.stateCond == RuleManager::StateCond::INACTIVE) stateStr = "inactive";
        obj["state"] = stateStr;

        JsonArray src = obj.createNestedArray("src");
        maskToArray(r.srcMask, 1, 8, src);

        JsonArray outputs = obj.createNestedArray("outputs");
        maskToArray(r.outMask, 7, 8, outputs);

        JsonObject wh = obj.createNestedObject("webhook");
        wh["discord"] = (r.webhookMask & 0x01) != 0;
        wh["slack"] = (r.webhookMask & 0x02) != 0;
        wh["generic"] = (r.webhookMask & 0x04) != 0;
    }

    String json;
    serializeJson(doc, json);
    server_->send(200, "application/json", json);
}

void HttpManagerIs05a::handleRuleSave() {
    if (!checkAuth()) { requestAuth(); return; }
    if (!rules_) {
        server_->send(500, "application/json", "{\"error\":\"Rule manager not initialized\"}");
        return;
    }

    DynamicJsonDocument doc(1024);
    DeserializationError error = deserializeJson(doc, server_->arg("plain"));
    if (error) {
        server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
        return;
    }

    int id = doc["id"] | -1;
    if (id < 0 || id >= rules_->getRuleCount()) {
        server_->send(400, "application/json", "{\"error\":\"Invalid rule id\"}");
        return;
    }

    RuleManager::Rule r = rules_->getRule(id);
    r.enabled = doc["enabled"] | true;

    // src channels
    r.srcMask = 0;
    if (doc["src"].is<JsonArray>()) {
        for (JsonVariant v : doc["src"].as<JsonArray>()) {
            int ch = v.as<int>();
            if (ch >= 1 && ch <= 8) {
                r.srcMask |= (1 << (ch - 1));
            }
        }
    }
    if (r.srcMask == 0) {
        server_->send(400, "application/json", "{\"error\":\"No input channels\"}");
        return;
    }

    // outputs
    r.outMask = 0;
    if (doc["outputs"].is<JsonArray>()) {
        for (JsonVariant v : doc["outputs"].as<JsonArray>()) {
            int ch = v.as<int>();
            if (ch == 7 || ch == 8) {
                r.outMask |= (1 << (ch - 1));
            }
        }
    }
    if (r.outMask == 0) {
        server_->send(400, "application/json", "{\"error\":\"No output channels\"}");
        return;
    }

    String state = doc["state"] | "any";
    if (state == "active") r.stateCond = RuleManager::StateCond::ACTIVE;
    else if (state == "inactive") r.stateCond = RuleManager::StateCond::INACTIVE;
    else r.stateCond = RuleManager::StateCond::ANY;

    r.pulseMs = doc["pulseMs"] | 3000;
    r.pulseMs = constrain(r.pulseMs, 5, 10000);
    r.cooldownMs = doc["cooldownMs"] | 0;

    r.webhookMask = 0;
    JsonObject wh = doc["webhook"];
    if (!wh.isNull()) {
        if (wh["discord"]) r.webhookMask |= 0x01;
        if (wh["slack"]) r.webhookMask |= 0x02;
        if (wh["generic"]) r.webhookMask |= 0x04;
    }

    if (rules_->setRule(id, r)) {
        server_->send(200, "application/json", "{\"ok\":true}");
    } else {
        server_->send(500, "application/json", "{\"error\":\"Failed to save rule\"}");
    }
}

void HttpManagerIs05a::handleRuleDelete() {
    if (!checkAuth()) { requestAuth(); return; }
    if (!rules_) {
        server_->send(500, "application/json", "{\"error\":\"Rule manager not initialized\"}");
        return;
    }

    String uri = server_->uri();
    int idx = uri.substring(uri.lastIndexOf('/') + 1).toInt();
    if (idx < 0 || idx >= rules_->getRuleCount()) {
        server_->send(400, "application/json", "{\"error\":\"Invalid rule id\"}");
        return;
    }

    if (rules_->deleteRule(idx)) {
        server_->send(200, "application/json", "{\"ok\":true}");
    } else {
        server_->send(500, "application/json", "{\"error\":\"Failed to delete\"}");
    }
}

// ========================================
// SpeedDial - is05a固有セクション生成
// ========================================
String HttpManagerIs05a::generateTypeSpecificSpeedDial() {
    String text = "";

    // [Channels] セクション
    text += "[Channels]\n";
    for (int ch = 1; ch <= 8; ch++) {
        auto cfg = channels_->getConfig(ch);
        text += "ch" + String(ch) + "_name=" + cfg.name + "\n";
        text += "ch" + String(ch) + "_meaning=" + cfg.meaning + "\n";
        if (ch >= 7) {
            text += "ch" + String(ch) + "_mode=" + (cfg.isOutput ? "output" : "input") + "\n";
        }
    }
    text += "\n";

    // [Webhook] セクション
    text += "[Webhook]\n";
    text += "enabled=" + String(webhooks_->isEnabled() ? "true" : "false") + "\n";
    text += "discordUrl=" + webhooks_->getDiscordUrl() + "\n";
    text += "slackUrl=" + webhooks_->getSlackUrl() + "\n";
    text += "genericUrl=" + webhooks_->getGenericUrl() + "\n";
    text += "\n";

    // [Rules] セクション（読み取り専用情報として）
    text += "; [Rules] - 8 rules available (0-7)\n";
    text += "; Configure via Web UI or API\n";

    return text;
}

// ========================================
// SpeedDial - is05a固有セクション適用
// ========================================
bool HttpManagerIs05a::applyTypeSpecificSpeedDial(const String& section, const std::vector<String>& lines) {
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

    if (section == "Channels") {
        for (const auto& line : lines) {
            String key, value;
            if (!parseLine(line, key, value)) continue;

            // ch1_name, ch1_meaning, ch7_mode, ch8_mode
            for (int ch = 1; ch <= 8; ch++) {
                String chPrefix = "ch" + String(ch) + "_";
                if (key.startsWith(chPrefix)) {
                    String field = key.substring(chPrefix.length());
                    auto cfg = channels_->getConfig(ch);

                    if (field == "name") {
                        cfg.name = value;
                        channels_->setConfig(ch, cfg);
                    } else if (field == "meaning") {
                        cfg.meaning = value;
                        channels_->setConfig(ch, cfg);
                    } else if (field == "mode" && ch >= 7) {
                        bool isOutput = (value == "output");
                        channels_->setOutputMode(ch, isOutput);
                    }
                    break;
                }
            }
        }
        return true;
    }

    if (section == "Webhook") {
        for (const auto& line : lines) {
            String key, value;
            if (!parseLine(line, key, value)) continue;

            if (key == "enabled") {
                webhooks_->setEnabled(value == "true" || value == "1");
            } else if (key == "discordUrl") {
                webhooks_->setDiscordUrl(value);
            } else if (key == "slackUrl") {
                webhooks_->setSlackUrl(value);
            } else if (key == "genericUrl") {
                webhooks_->setGenericUrl(value);
            }
        }
        return true;
    }

    // 未知のセクションは無視（エラーにしない）
    return true;
}
