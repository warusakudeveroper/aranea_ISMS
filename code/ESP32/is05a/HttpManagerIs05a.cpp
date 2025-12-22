/**
 * HttpManagerIs05a.cpp
 */

#include "HttpManagerIs05a.h"
#include "ChannelManager.h"
#include "WebhookManager.h"
#include "Is05aKeys.h"
#include <ArduinoJson.h>

HttpManagerIs05a::HttpManagerIs05a()
    : AraneaWebUI()
    , channels_(nullptr)
    , webhooks_(nullptr)
    , lastChangedChannel_(-1)
    , sendSuccessCount_(0)
    , sendFailCount_(0)
    , lastSendResult_("---")
{
}

void HttpManagerIs05a::begin(SettingManager* settings, ChannelManager* channels,
                              WebhookManager* webhooks, int port) {
    channels_ = channels;
    webhooks_ = webhooks;

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
    html += F("<button class=\"tab-btn\" onclick=\"showTab('channels')\">Channels</button>");
    html += F("<button class=\"tab-btn\" onclick=\"showTab('webhook')\">Webhook</button>");
    return html;
}

String HttpManagerIs05a::generateTypeSpecificTabContents() {
    String html;
    // Channels Tab
    html += F("<div id=\"channels\" class=\"tab-content\">");
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
    html += F("<div id=\"webhook\" class=\"tab-content\">");
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
    js += F("if(tab==='channels')loadChannels();if(tab==='webhook')loadWebhookConfig();};");
    js += F("setInterval(function(){if(document.getElementById('channels').style.display!=='none')loadChannels();},2000);");
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

    JsonDocument doc;
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

    JsonDocument doc;
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

    JsonDocument doc;
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
        JsonDocument doc;
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
    JsonDocument doc;
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

    JsonDocument doc;
    DeserializationError error = deserializeJson(doc, server_->arg("plain"));

    if (error) {
        server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
        return;
    }

    String platform = doc["platform"].as<String>();

    // テスト送信（ch1を仮の状態変化として送信）
    bool ok = webhooks_->sendStateChange(1, true);

    if (ok) {
        server_->send(200, "application/json", "{\"ok\":true}");
    } else {
        server_->send(500, "application/json", "{\"ok\":false,\"error\":\"Send failed\"}");
    }
}
