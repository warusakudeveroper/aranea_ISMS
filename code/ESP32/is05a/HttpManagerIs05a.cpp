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

AraneaCloudStatus HttpManagerIs05a::getCloudStatus() {
    AraneaCloudStatus status;
    status.registered = deviceInfo_.cic.length() > 0;
    status.mqttConnected = mqttConnected_;
    status.lastStateReport = "";
    status.lastStateReportCode = 0;
    status.schemaVersion = 0;
    return status;
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
    obj["webhookQueue"] = webhooks_->getCurrentQueueLength();
    obj["webhookDrop"] = webhooks_->getQueueDropCount();
    obj["webhookEndpoints"] = webhooks_->getEndpointCount();

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
    // Channels Tab - 既存テーマ準拠
    html += F("<style>");
    html += F(".ch-lamp{width:24px;height:24px;border-radius:50%;display:inline-block;margin-right:12px;");
    html += F("border:2px solid #ccc;transition:all 0.3s;flex-shrink:0;}");
    html += F(".ch-lamp.active{background:#4caf50;border-color:#4caf50;box-shadow:0 0 12px #4caf50;}");
    html += F(".ch-lamp.inactive{background:#e0e0e0;border-color:#bbb;}");
    html += F(".ch-lamp.output-on{background:#ff9800;border-color:#ff9800;box-shadow:0 0 12px #ff9800;}");
    html += F(".ch-panel{background:#fff;border:1px solid #ddd;border-radius:8px;margin:8px 0;}");
    html += F(".ch-header{display:flex;align-items:center;padding:12px 16px;cursor:pointer;}");
    html += F(".ch-header:hover{background:#f5f5f5;}");
    html += F(".ch-title{flex:1;font-weight:600;color:#333;}");
    html += F(".ch-state-badge{font-size:0.8em;padding:4px 10px;border-radius:12px;margin-left:8px;}");
    html += F(".ch-state-badge.detecting{background:#4caf50;color:#fff;}");
    html += F(".ch-state-badge.idle{background:#e0e0e0;color:#666;}");
    html += F(".ch-state-badge.on{background:#ff9800;color:#fff;}");
    html += F(".ch-state-badge.off{background:#e0e0e0;color:#666;}");
    html += F(".ch-body{padding:16px;display:none;border-top:1px solid #eee;background:#fafafa;}");
    html += F(".ch-body.open{display:block;}");
    html += F(".ch-expand{color:#999;transition:transform 0.2s;}.ch-expand.open{transform:rotate(180deg);}");
    html += F(".ch-warning{background:#fff3e0;border:1px solid #ffcc80;color:#e65100;padding:10px;border-radius:6px;margin:8px 0;font-size:0.9em;}");
    html += F(".toggle-row{display:flex;align-items:center;gap:12px;margin:12px 0;}");
    html += F(".toggle-label{min-width:80px;color:#333;}.toggle-switch{position:relative;width:50px;height:26px;}");
    html += F(".toggle-switch input{opacity:0;width:0;height:0;}");
    html += F(".toggle-slider{position:absolute;cursor:pointer;top:0;left:0;right:0;bottom:0;background:#ccc;border-radius:26px;transition:0.3s;}");
    html += F(".toggle-slider:before{position:absolute;content:\"\";height:20px;width:20px;left:3px;bottom:3px;background:#fff;border-radius:50%;transition:0.3s;box-shadow:0 1px 3px rgba(0,0,0,0.3);}");
    html += F(".toggle-switch input:checked+.toggle-slider{background:#ff9800;}");
    html += F(".toggle-switch input:checked+.toggle-slider:before{transform:translateX(24px);}");
    html += F(".output-controls{margin-top:12px;padding:12px;background:#fff;border:1px solid #ddd;border-radius:6px;}");
    html += F("</style>");
    html += F("<div id=\"tab-channels\" class=\"tab-content\">");
    html += F("<h3>8ch Detector</h3>");
    html += F("<p style=\"color:#666;font-size:0.9em\">各チャンネルをクリックして設定を展開</p>");
    html += F("<div id=\"channel-list\"></div></div>");
    // Webhook Tab v2.0 - 5エンドポイント対応
    html += F("<div id=\"tab-webhook\" class=\"tab-content\">");
    html += F("<h3>Webhook Configuration</h3>");
    // Stats
    html += F("<div id=\"webhook-stats\" style=\"background:#f5f5f5;padding:12px;border-radius:6px;margin-bottom:16px;\">");
    html += F("<span>送信: <b id=\"wh-sent\">0</b></span> ");
    html += F("<span>失敗: <b id=\"wh-fail\">0</b></span> ");
    html += F("<span>キュー: <b id=\"wh-queue\">0</b></span> ");
    html += F("<span>破棄: <b id=\"wh-drop\">0</b></span></div>");
    html += F("<form id=\"webhook-form\">");
    html += F("<div class=\"form-group\"><label><input type=\"checkbox\" id=\"webhook-enabled\"> Webhook有効</label></div>");
    // Queue/Interval settings
    html += F("<div class=\"form-row\">");
    html += F("<div class=\"form-group\"><label>キューサイズ:</label><input type=\"number\" id=\"wh-queue-size\" min=\"1\" max=\"10\" value=\"3\"></div>");
    html += F("<div class=\"form-group\"><label>送信間隔(秒):</label><input type=\"number\" id=\"wh-interval\" min=\"1\" max=\"60\" value=\"10\"></div>");
    html += F("</div>");
    // 5 Endpoints
    html += F("<h4>送信先エンドポイント (最大5件)</h4>");
    for (int ep = 0; ep < 5; ep++) {
        html += F("<div class=\"ep-panel\" style=\"background:#fafafa;border:1px solid #ddd;border-radius:6px;padding:12px;margin:8px 0;\">");
        html += F("<div style=\"display:flex;align-items:center;gap:12px;margin-bottom:8px;\">");
        html += F("<label><input type=\"checkbox\" id=\"ep");
        html += String(ep);
        html += F("-on\"> EP");
        html += String(ep);
        html += F("</label>");
        html += F("<button type=\"button\" onclick=\"testEndpoint(");
        html += String(ep);
        html += F(")\">Test</button></div>");
        html += F("<div class=\"form-group\"><label>URL:</label><input type=\"text\" id=\"ep");
        html += String(ep);
        html += F("-url\" placeholder=\"https://...\" style=\"width:100%\"></div>");
        html += F("<div class=\"form-group\"><label>カスタムメッセージ:</label><input type=\"text\" id=\"ep");
        html += String(ep);
        html += F("-msg\" placeholder=\"{name}が{state}になりました (空欄時デフォルト)\" style=\"width:100%\"></div>");
        html += F("<div class=\"form-group\"><label>通知対象ch:</label><div class=\"ch-mask-row\" id=\"ep");
        html += String(ep);
        html += F("-ch\">");
        for (int ch = 1; ch <= 8; ch++) {
            html += F("<label style=\"margin-right:8px\"><input type=\"checkbox\" data-ep=\"");
            html += String(ep);
            html += F("\" data-ch=\"");
            html += String(ch);
            html += F("\" checked> ");
            html += String(ch);
            html += F("</label>");
        }
        html += F("</div></div></div>");
    }
    // QuietMode
    html += F("<h4>おやすみモード (QuietMode)</h4>");
    html += F("<div class=\"form-group\"><label><input type=\"checkbox\" id=\"quiet-enabled\"> 有効にする</label></div>");
    html += F("<div class=\"form-row\"><div class=\"form-group\"><label>開始時刻:</label>");
    html += F("<select id=\"quiet-start\">");
    for (int h = 0; h < 24; h++) {
        html += F("<option value=\"");
        html += String(h);
        html += F("\">");
        html += (h < 10 ? "0" : "") + String(h) + ":00";
        html += F("</option>");
    }
    html += F("</select></div><div class=\"form-group\"><label>終了時刻:</label>");
    html += F("<select id=\"quiet-end\">");
    for (int h = 0; h < 24; h++) {
        html += F("<option value=\"");
        html += String(h);
        html += F("\">");
        html += (h < 10 ? "0" : "") + String(h) + ":00";
        html += F("</option>");
    }
    html += F("</select></div></div>");
    html += F("<p style=\"color:#aaa;font-size:0.9em\">※この時間帯はWebhook通知を送信しません</p>");
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
    js += F("var chData={};var openPanels={};");  // 展開状態を保持
    // Build channel panel HTML
    js += F("function buildChPanel(ch){let d=chData[ch.ch];let lampClass='ch-lamp '+(ch.isOutput?(ch.outputActive?'output-on':'inactive'):(ch.active?'active':'inactive'));");
    js += F("let badgeClass='ch-state-badge '+(ch.isOutput?(ch.outputActive?'on':'off'):(ch.active?'detecting':'idle'));");
    js += F("let badgeText=ch.isOutput?(ch.outputActive?'ON':'OFF'):(ch.active?'検知':'待機');");
    js += F("let html='<div class=\"ch-panel\" id=\"panel-'+ch.ch+'\">';");
    js += F("html+='<div class=\"ch-header\" onclick=\"togglePanel('+ch.ch+')\">';");
    js += F("html+='<span class=\"'+lampClass+'\"></span>';");
    js += F("html+='<span class=\"ch-title\">ch'+ch.ch+': '+ch.name+'</span>';");
    js += F("html+='<span class=\"'+badgeClass+'\">'+badgeText+'</span>';");
    js += F("html+='<span class=\"ch-expand'+(openPanels[ch.ch]?' open':'')+'\" id=\"expand-'+ch.ch+'\">▼</span></div>';");
    // Panel body (openPanels state restore)
    js += F("let isOpen=openPanels[ch.ch]||false;");
    js += F("html+='<div class=\"ch-body'+(isOpen?' open':'')+'\" id=\"body-'+ch.ch+'\">';");
    // Name
    js += F("html+='<div class=\"form-group\"><label>名前:</label><input type=\"text\" id=\"name-'+ch.ch+'\" value=\"'+ch.name+'\" onchange=\"saveChConfig('+ch.ch+')\"></div>';");
    // Debounce
    js += F("html+='<div class=\"form-group\"><label>デバウンス(ms):</label><input type=\"number\" id=\"debounce-'+ch.ch+'\" min=\"10\" max=\"10000\" value=\"'+(ch.debounceMs||100)+'\" onchange=\"saveChConfig('+ch.ch+')\"></div>';");
    // ch7/ch8: IO toggle
    js += F("if(ch.ch>=7){");
    js += F("html+='<div class=\"toggle-row\"><span class=\"toggle-label\">出力モード:</span>';");
    js += F("html+='<label class=\"toggle-switch\"><input type=\"checkbox\" id=\"output-'+ch.ch+'\" '+(ch.isOutput?'checked':'')+' onchange=\"toggleOutputMode('+ch.ch+')\">';");
    js += F("html+='<span class=\"toggle-slider\"></span></label></div>';");
    // Warning
    js += F("html+='<div class=\"ch-warning\" id=\"warn-'+ch.ch+'\" style=\"display:'+(ch.isOutput?'block':'none')+'\">⚠️ 入力ピンを出力として使用する際はオプトカプラをバイパスして該当ピンをコネクタに接続する必要があります</div>';");
    // Output controls
    js += F("html+='<div class=\"output-controls\" id=\"outctrl-'+ch.ch+'\" style=\"display:'+(ch.isOutput?'block':'none')+'\">';");
    js += F("html+='<div class=\"form-group\"><label>動作モード:</label><select id=\"outmode-'+ch.ch+'\" onchange=\"saveOutputCfg('+ch.ch+')\">';");
    js += F("html+='<option value=\"0\"'+(ch.outputMode===0?' selected':'')+'>Momentary</option>';");
    js += F("html+='<option value=\"1\"'+(ch.outputMode===1?' selected':'')+'>Alternate</option>';");
    js += F("html+='<option value=\"2\"'+(ch.outputMode===2?' selected':'')+'>Interval</option></select></div>';");
    // Duration (hide for Alternate)
    js += F("html+='<div class=\"form-group\" id=\"dur-grp-'+ch.ch+'\" style=\"display:'+(ch.outputMode===1?'none':'block')+'\"><label>出力時間(ms):</label>';");
    js += F("html+='<input type=\"number\" id=\"dur-'+ch.ch+'\" min=\"500\" max=\"10000\" value=\"'+(ch.durationMs||3000)+'\" onchange=\"saveOutputCfg('+ch.ch+')\"></div>';");
    // Count (only for Interval)
    js += F("html+='<div class=\"form-group\" id=\"cnt-grp-'+ch.ch+'\" style=\"display:'+(ch.outputMode===2?'block':'none')+'\"><label>回数:</label>';");
    js += F("html+='<input type=\"number\" id=\"cnt-'+ch.ch+'\" min=\"1\" max=\"100\" value=\"'+(ch.intervalCnt||3)+'\" onchange=\"saveOutputCfg('+ch.ch+')\"></div>';");
    // Buttons
    js += F("html+='<div class=\"btn-group\"><button onclick=\"triggerOutput('+ch.ch+')\">ON</button><button onclick=\"stopOutput('+ch.ch+')\">OFF</button></div>';");
    js += F("html+='</div>';}");
    js += F("html+='</div></div>';return html;}");
    // Load channels
    js += F("function loadChannels(){fetch('/api/channels').then(r=>r.json()).then(data=>{");
    js += F("let html='';data.channels.forEach(ch=>{chData[ch.ch]=ch;html+=buildChPanel(ch);});");
    js += F("document.getElementById('channel-list').innerHTML=html;});}");
    // Toggle panel expand (with state tracking)
    js += F("function togglePanel(ch){let body=document.getElementById('body-'+ch);let exp=document.getElementById('expand-'+ch);");
    js += F("if(body.classList.contains('open')){body.classList.remove('open');exp.classList.remove('open');openPanels[ch]=false;}");
    js += F("else{body.classList.add('open');exp.classList.add('open');openPanels[ch]=true;}}");
    // Toggle output mode (ch7/ch8)
    js += F("function toggleOutputMode(ch){let checked=document.getElementById('output-'+ch).checked;");
    js += F("if(checked&&!confirm('⚠️ 出力モードに変更します。\\n\\nオプトカプラをバイパスして該当ピンをコネクタに接続する必要があります。\\n\\n続行しますか？')){");
    js += F("document.getElementById('output-'+ch).checked=false;return;}");
    js += F("fetch('/api/channel',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({ch:ch,mode:checked?'output':'input'})})");
    js += F(".then(r=>r.json()).then(d=>{if(d.ok){showMessage('ch'+ch+(checked?' OUTPUT':' INPUT'));loadChannels();}else{showMessage(d.error||'Error','error');document.getElementById('output-'+ch).checked=!checked;}});}");
    // Save channel config (name, debounce)
    js += F("function saveChConfig(ch){let body={ch:ch,name:document.getElementById('name-'+ch).value,debounce:parseInt(document.getElementById('debounce-'+ch).value)||100};");
    js += F("fetch('/api/channel',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}).then(r=>r.json()).then(d=>{if(d.ok)showMessage('ch'+ch+' saved');else showMessage(d.error||'Error','error');});}");
    // Save output config
    js += F("function saveOutputCfg(ch){let mode=parseInt(document.getElementById('outmode-'+ch).value);");
    js += F("document.getElementById('dur-grp-'+ch).style.display=mode===1?'none':'block';");
    js += F("document.getElementById('cnt-grp-'+ch).style.display=mode===2?'block':'none';");
    js += F("let body={ch:ch,outputMode:mode,durationMs:parseInt(document.getElementById('dur-'+ch).value)||3000,intervalCount:parseInt(document.getElementById('cnt-'+ch).value)||3};");
    js += F("fetch('/api/output/config',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}).then(r=>r.json()).then(d=>{if(!d.ok)showMessage(d.error||'Error','error');});}");
    // Trigger and stop output
    js += F("function triggerOutput(ch){fetch('/api/output/trigger',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({ch:ch})}).then(r=>r.json()).then(d=>{if(d.ok)showMessage('ch'+ch+' ON');else showMessage(d.error||'Error','error');});}");
    js += F("function stopOutput(ch){fetch('/api/output/stop',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({ch:ch})}).then(r=>r.json()).then(d=>{if(d.ok)showMessage('ch'+ch+' OFF');else showMessage(d.error||'Error','error');});}");
    // Load webhook config v2.0
    js += F("function loadWebhookConfig(){fetch('/api/webhook/config').then(r=>r.json()).then(data=>{");
    js += F("document.getElementById('webhook-enabled').checked=data.enabled;");
    js += F("document.getElementById('wh-queue-size').value=data.queueSize||3;");
    js += F("document.getElementById('wh-interval').value=data.intervalSec||10;");
    js += F("document.getElementById('wh-sent').textContent=data.sentCount||0;");
    js += F("document.getElementById('wh-fail').textContent=data.failCount||0;");
    js += F("document.getElementById('wh-queue').textContent=data.queueLength||0;");
    js += F("document.getElementById('wh-drop').textContent=data.dropCount||0;");
    // QuietMode
    js += F("document.getElementById('quiet-enabled').checked=data.quietEnabled||false;");
    js += F("document.getElementById('quiet-start').value=data.quietStart||22;");
    js += F("document.getElementById('quiet-end').value=data.quietEnd||6;");
    // Endpoints
    js += F("if(data.endpoints){data.endpoints.forEach((ep,i)=>{");
    js += F("document.getElementById('ep'+i+'-on').checked=ep.enabled;");
    js += F("document.getElementById('ep'+i+'-url').value=ep.url||'';");
    js += F("document.getElementById('ep'+i+'-msg').value=ep.message||'';");
    js += F("let mask=ep.channelMask!==undefined?ep.channelMask:0xFF;");
    js += F("document.querySelectorAll('#ep'+i+'-ch input').forEach(el=>{let ch=parseInt(el.dataset.ch);el.checked=(mask&(1<<(ch-1)))!==0;});});}});}");
    // Test endpoint
    js += F("function testEndpoint(idx){fetch('/api/webhook/test',{method:'POST',");
    js += F("headers:{'Content-Type':'application/json'},body:JSON.stringify({index:idx})})");
    js += F(".then(r=>r.json()).then(data=>{if(data.ok){showMessage('EP'+idx+' test sent');}");
    js += F("else{showMessage('Error: '+(data.error||'failed'),'error');}});}");
    // Save webhook v2.0
    js += F("document.getElementById('webhook-form').addEventListener('submit',function(e){e.preventDefault();");
    js += F("let endpoints=[];for(let i=0;i<5;i++){let mask=0;");
    js += F("document.querySelectorAll('#ep'+i+'-ch input').forEach(el=>{if(el.checked)mask|=(1<<(parseInt(el.dataset.ch)-1));});");
    js += F("endpoints.push({enabled:document.getElementById('ep'+i+'-on').checked,");
    js += F("url:document.getElementById('ep'+i+'-url').value,");
    js += F("message:document.getElementById('ep'+i+'-msg').value,channelMask:mask});}");
    js += F("fetch('/api/webhook/config',{method:'POST',headers:{'Content-Type':'application/json'},");
    js += F("body:JSON.stringify({enabled:document.getElementById('webhook-enabled').checked,");
    js += F("queueSize:parseInt(document.getElementById('wh-queue-size').value)||3,");
    js += F("intervalSec:parseInt(document.getElementById('wh-interval').value)||10,");
    js += F("quietEnabled:document.getElementById('quiet-enabled').checked,");
    js += F("quietStart:parseInt(document.getElementById('quiet-start').value),");
    js += F("quietEnd:parseInt(document.getElementById('quiet-end').value),");
    js += F("endpoints:endpoints})})");
    js += F(".then(r=>r.json()).then(data=>{if(data.ok){showMessage('Webhook settings saved');}");
    js += F("else{showMessage('Error saving settings','error');}});});");
    // Tab switching
    js += F("var origShowTab=showTab;showTab=function(tab){origShowTab(tab);");
    js += F("if(tab==='channels')loadChannels();if(tab==='webhook')loadWebhookConfig();if(tab==='rules')loadRules();};");
    // Rules functions
    js += F("function parseChList(v){return v.split(',').map(x=>parseInt(x.trim())).filter(x=>!isNaN(x));}");
    js += F("function loadRules(){fetch('/api/rules').then(r=>r.json()).then(data=>{let html='<table><tr><th>ID</th><th>Enabled</th><th>Src</th><th>State</th><th>Out</th><th>Pulse</th><th>Cooldown</th><th>Webhook</th></tr>';data.rules.forEach(r=>{");
    js += F("html+='<tr><td>'+r.id+'</td><td>'+(r.enabled?'ON':'OFF')+'</td><td>'+r.src.join(',')+'</td><td>'+r.state+'</td><td>'+r.outputs.join(',')+'</td><td>'+r.pulseMs+'</td><td>'+r.cooldownMs+'</td>';let wh=[];if(r.webhook.discord)wh.push('D');if(r.webhook.slack)wh.push('S');if(r.webhook.generic)wh.push('G');html+='<td>'+wh.join('/')+'</td></tr>';});html+='</table>';document.getElementById('rule-list').innerHTML=html;});}");
    js += F("function saveRule(){let id=parseInt(document.getElementById('rule-id').value)||0;let body={id:id,enabled:document.getElementById('rule-enabled').checked,src:parseChList(document.getElementById('rule-src').value),state:document.getElementById('rule-state').value,outputs:parseChList(document.getElementById('rule-out').value),pulseMs:parseInt(document.getElementById('rule-pulse').value)||3000,cooldownMs:parseInt(document.getElementById('rule-cooldown').value)||0,webhook:{discord:document.getElementById('rule-wh-discord').checked,slack:document.getElementById('rule-wh-slack').checked,generic:document.getElementById('rule-wh-generic').checked}};fetch('/api/rules',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}).then(r=>r.json()).then(res=>{if(res.ok){showMessage('Rule saved');loadRules();}else{showMessage(res.error||'Save failed','error');}});}");
    js += F("function deleteRule(){let id=parseInt(document.getElementById('rule-id').value)||0;if(!confirm('Delete rule '+id+'?'))return;fetch('/api/rules/'+id,{method:'DELETE'}).then(r=>r.json()).then(res=>{if(res.ok){showMessage('Rule deleted');loadRules();}else{showMessage(res.error||'Delete failed','error');}});}");
    // Auto refresh
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

    // パルス出力（レガシー）
    server_->on("/api/pulse", HTTP_POST, [this]() { handlePulse(); });

    // 出力制御（新規）
    server_->on("/api/output/trigger", HTTP_POST, [this]() { handleOutputTrigger(); });
    server_->on("/api/output/stop", HTTP_POST, [this]() { handleOutputStop(); });
    server_->on("/api/output/config", HTTP_POST, [this]() { handleOutputConfig(); });

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
    obj["webhookQueueSize"] = webhooks_->getQueueSize();
    obj["webhookIntervalSec"] = webhooks_->getIntervalSec();
    obj["webhookEndpointCount"] = webhooks_->getEndpointCount();
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
        chObj["active"] = channels_->isActive(ch);  // 検知状態 (boolean)
        chObj["isOutput"] = channels_->isOutputMode(ch);
        chObj["outputActive"] = channels_->isOutputActive(ch);  // 出力状態
        chObj["debounceMs"] = cfg.debounceMs;  // デバウンス設定
        chObj["lastUpdatedAt"] = channels_->getLastUpdatedAt(ch);

        // ch7/ch8の出力モード情報を追加
        if (ch >= 7) {
            chObj["outputMode"] = static_cast<int>(cfg.outputMode);
            chObj["durationMs"] = cfg.outputDurationMs;
            chObj["intervalCnt"] = cfg.intervalCount;
        }
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
        DynamicJsonDocument doc(2048);
        doc["enabled"] = webhooks_->isEnabled();
        doc["queueSize"] = webhooks_->getQueueSize();
        doc["intervalSec"] = webhooks_->getIntervalSec();
        // Stats
        doc["sentCount"] = webhooks_->getSentCount();
        doc["failCount"] = webhooks_->getFailCount();
        doc["queueLength"] = webhooks_->getCurrentQueueLength();
        doc["dropCount"] = webhooks_->getQueueDropCount();
        doc["quietSkipCount"] = webhooks_->getQuietSkipCount();
        // QuietMode
        doc["quietEnabled"] = webhooks_->isQuietModeEnabled();
        doc["quietStart"] = webhooks_->getQuietStartHour();
        doc["quietEnd"] = webhooks_->getQuietEndHour();
        // Endpoints
        JsonArray endpoints = doc.createNestedArray("endpoints");
        for (int i = 0; i < WebhookManager::MAX_ENDPOINTS; i++) {
            auto ep = webhooks_->getEndpoint(i);
            JsonObject epObj = endpoints.createNestedObject();
            epObj["enabled"] = ep.enabled;
            epObj["url"] = ep.url;
            epObj["message"] = ep.message;
            epObj["channelMask"] = ep.channelMask;
        }

        String json;
        serializeJson(doc, json);
        server_->send(200, "application/json", json);
        return;
    }

    // POST
    DynamicJsonDocument doc(2048);
    DeserializationError error = deserializeJson(doc, server_->arg("plain"));

    if (error) {
        server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
        return;
    }

    if (doc.containsKey("enabled")) webhooks_->setEnabled(doc["enabled"]);
    if (doc.containsKey("queueSize")) webhooks_->setQueueSize(doc["queueSize"]);
    if (doc.containsKey("intervalSec")) webhooks_->setIntervalSec(doc["intervalSec"]);

    // QuietMode設定
    if (doc.containsKey("quietEnabled")) webhooks_->setQuietModeEnabled(doc["quietEnabled"]);
    if (doc.containsKey("quietStart") && doc.containsKey("quietEnd")) {
        webhooks_->setQuietModeHours(doc["quietStart"], doc["quietEnd"]);
    }

    // Endpoints設定
    if (doc.containsKey("endpoints")) {
        JsonArray arr = doc["endpoints"].as<JsonArray>();
        int i = 0;
        for (JsonObject epObj : arr) {
            if (i >= WebhookManager::MAX_ENDPOINTS) break;
            WebhookManager::Endpoint ep;
            ep.enabled = epObj["enabled"] | false;
            ep.url = epObj["url"].as<String>();
            ep.message = epObj["message"].as<String>();
            ep.channelMask = epObj["channelMask"] | 0xFF;
            webhooks_->setEndpoint(i, ep);
            i++;
        }
    }

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

    // 新方式: index指定
    if (doc.containsKey("index")) {
        int idx = doc["index"];
        if (idx < 0 || idx >= WebhookManager::MAX_ENDPOINTS) {
            server_->send(400, "application/json", "{\"error\":\"Invalid index\"}");
            return;
        }
        bool ok = webhooks_->sendTestToEndpoint(idx);
        if (ok) {
            server_->send(200, "application/json", "{\"ok\":true}");
        } else {
            server_->send(500, "application/json", "{\"ok\":false,\"error\":\"Send failed\"}");
        }
        return;
    }

    // レガシー互換: platform指定
    String platform = doc["platform"].as<String>();
    int idx = -1;
    if (platform == "discord") idx = 0;
    else if (platform == "slack") idx = 1;
    else if (platform == "generic") idx = 2;

    if (idx < 0) {
        server_->send(400, "application/json", "{\"error\":\"Invalid platform or index\"}");
        return;
    }

    bool ok = webhooks_->sendTestToEndpoint(idx);
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
// 出力制御ハンドラ
// ========================================
void HttpManagerIs05a::handleOutputTrigger() {
    if (!checkAuth()) { requestAuth(); return; }

    DynamicJsonDocument doc(256);
    DeserializationError error = deserializeJson(doc, server_->arg("plain"));

    if (error) {
        server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
        return;
    }

    int ch = doc["ch"] | 0;
    if (ch < 7 || ch > 8) {
        server_->send(400, "application/json", "{\"error\":\"Only ch7/ch8 support output\"}");
        return;
    }

    if (!channels_->isOutputMode(ch)) {
        server_->send(400, "application/json", "{\"error\":\"Channel not in output mode\"}");
        return;
    }

    channels_->triggerOutput(ch);
    server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs05a::handleOutputStop() {
    if (!checkAuth()) { requestAuth(); return; }

    DynamicJsonDocument doc(256);
    DeserializationError error = deserializeJson(doc, server_->arg("plain"));

    if (error) {
        server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
        return;
    }

    int ch = doc["ch"] | 0;
    if (ch < 7 || ch > 8) {
        server_->send(400, "application/json", "{\"error\":\"Only ch7/ch8 support output\"}");
        return;
    }

    channels_->stopOutput(ch);
    server_->send(200, "application/json", "{\"ok\":true}");
}

void HttpManagerIs05a::handleOutputConfig() {
    if (!checkAuth()) { requestAuth(); return; }

    DynamicJsonDocument doc(512);
    DeserializationError error = deserializeJson(doc, server_->arg("plain"));

    if (error) {
        server_->send(400, "application/json", "{\"error\":\"Invalid JSON\"}");
        return;
    }

    int ch = doc["ch"] | 0;
    if (ch < 7 || ch > 8) {
        server_->send(400, "application/json", "{\"error\":\"Only ch7/ch8 support output config\"}");
        return;
    }

    int outputMode = doc["outputMode"] | 0;
    int durationMs = doc["durationMs"] | 3000;
    int intervalCount = doc["intervalCount"] | 3;

    // 値の正規化
    outputMode = constrain(outputMode, 0, 2);
    durationMs = constrain(durationMs, 500, 10000);
    intervalCount = constrain(intervalCount, 1, 100);

    channels_->setOutputBehavior(ch,
        static_cast<ChannelManager::OutputMode>(outputMode),
        durationMs, intervalCount);

    server_->send(200, "application/json", "{\"ok\":true}");
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
