/**
 * RuleManager.cpp
 */

#include "RuleManager.h"
#include "SettingManager.h"
#include "ChannelManager.h"
#include "WebhookManager.h"

RuleManager::RuleManager()
    : settings_(nullptr)
    , channels_(nullptr)
    , webhooks_(nullptr) {
    for (int i = 0; i < kMaxRules; i++) {
        rules_[i] = {false, 0, 0, 3000, 0, StateCond::ANY, 0, 0};
    }
}

void RuleManager::begin(SettingManager* settings, ChannelManager* channels, WebhookManager* webhooks) {
    settings_ = settings;
    channels_ = channels;
    webhooks_ = webhooks;
    loadRules();
    Serial.println("[RuleManager] Initialized");
}

RuleManager::Rule RuleManager::getRule(int idx) const {
    if (idx < 0 || idx >= kMaxRules) {
        return Rule{false, 0, 0, 3000, 0, StateCond::ANY, 0, 0};
    }
    return rules_[idx];
}

bool RuleManager::setRule(int idx, const Rule& rule) {
    if (!settings_ || idx < 0 || idx >= kMaxRules) return false;
    rules_[idx] = rule;
    saveRuleToNvs(idx, rule);
    return true;
}

bool RuleManager::deleteRule(int idx) {
    if (!settings_ || idx < 0 || idx >= kMaxRules) return false;
    Rule empty{false, 0, 0, 3000, 0, StateCond::ANY, 0, 0};
    rules_[idx] = empty;
    saveRuleToNvs(idx, empty);
    return true;
}

void RuleManager::handleChannelEvent(int ch, bool active) {
    if (!channels_) return;
    unsigned long now = millis();
    uint8_t chBit = (ch >= 1 && ch <= 8) ? (1 << (ch - 1)) : 0;

    for (int i = 0; i < kMaxRules; i++) {
        Rule& r = rules_[i];
        if (!r.enabled) continue;
        if (!(r.srcMask & chBit)) continue;

        // 状態条件チェック
        if (r.stateCond == StateCond::ACTIVE && !active) continue;
        if (r.stateCond == StateCond::INACTIVE && active) continue;

        // クールダウン
        if (r.cooldownMs > 0 && (now - r.lastTriggerMs) < (unsigned long)r.cooldownMs) {
            continue;
        }

        // 出力（OutputModeを尊重してtriggerOutput使用）
        if (r.outMask & (1 << 6)) { // ch7
            if (channels_->isOutputMode(7)) {
                channels_->triggerOutput(7);
            } else {
                Serial.println("[RuleManager] Skip ch7 (not output mode)");
            }
        }
        if (r.outMask & (1 << 7)) { // ch8
            if (channels_->isOutputMode(8)) {
                channels_->triggerOutput(8);
            } else {
                Serial.println("[RuleManager] Skip ch8 (not output mode)");
            }
        }

        // Webhook
        if (r.webhookMask != 0 && webhooks_) {
            webhooks_->sendStateChangeToEndpoints(r.webhookMask, ch, active);
        }

        r.lastTriggerMs = now;
    }
}

void RuleManager::loadRules() {
    if (!settings_) return;
    for (int i = 0; i < kMaxRules; i++) {
        Rule r;
        r.enabled = settings_->getBool(ruleKey(i, "en").c_str(), false);
        r.srcMask = settings_->getInt(ruleKey(i, "src").c_str(), 0);
        r.outMask = settings_->getInt(ruleKey(i, "out").c_str(), 0);
        r.pulseMs = settings_->getInt(ruleKey(i, "ms").c_str(), 3000);
        r.webhookMask = settings_->getInt(ruleKey(i, "wh").c_str(), 0);
        r.stateCond = static_cast<StateCond>(settings_->getInt(ruleKey(i, "st").c_str(), 0));
        r.cooldownMs = settings_->getInt(ruleKey(i, "cd").c_str(), 0);
        r.lastTriggerMs = 0;
        rules_[i] = r;
    }
}

void RuleManager::saveRuleToNvs(int idx, const Rule& rule) {
    settings_->setBool(ruleKey(idx, "en").c_str(), rule.enabled);
    settings_->setInt(ruleKey(idx, "src").c_str(), rule.srcMask);
    settings_->setInt(ruleKey(idx, "out").c_str(), rule.outMask);
    settings_->setInt(ruleKey(idx, "ms").c_str(), rule.pulseMs);
    settings_->setInt(ruleKey(idx, "wh").c_str(), rule.webhookMask);
    settings_->setInt(ruleKey(idx, "st").c_str(), static_cast<int>(rule.stateCond));
    settings_->setInt(ruleKey(idx, "cd").c_str(), rule.cooldownMs);
}

String RuleManager::ruleKey(int idx, const char* suffix) const {
    // 例: "r0_en" (4文字) 15文字制限以内
    return String("r") + String(idx) + "_" + suffix;
}
