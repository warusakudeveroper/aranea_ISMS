#line 1 "/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/aranea_ISMS/code/ESP32/is05a/RuleManager.h"
/**
 * RuleManager.h
 *
 * 入力ch -> 出力/Webhook トリガールール管理
 */

#ifndef RULE_MANAGER_IS05A_H
#define RULE_MANAGER_IS05A_H

#include <Arduino.h>

class SettingManager;
class ChannelManager;
class WebhookManager;

class RuleManager {
public:
    static const int kMaxRules = 8;  // 0-7

    enum class StateCond : uint8_t {
        ANY = 0,
        ACTIVE = 1,
        INACTIVE = 2,
    };

    struct Rule {
        bool enabled;
        uint8_t srcMask;      // ch1-8 bit0-7
        uint8_t outMask;      // ch7->bit6, ch8->bit7
        uint16_t pulseMs;
        uint8_t webhookMask;  // bit0=discord, bit1=slack, bit2=generic
        StateCond stateCond;
        uint16_t cooldownMs;
        unsigned long lastTriggerMs;  // runtime only
    };

    RuleManager();

    void begin(SettingManager* settings, ChannelManager* channels, WebhookManager* webhooks);

    /**
     * 入力変化イベントを処理
     */
    void handleChannelEvent(int ch, bool active);

    /**
     * ルール一覧取得（JSON化用）
     */
    Rule getRule(int idx) const;
    bool setRule(int idx, const Rule& rule);
    bool deleteRule(int idx);

    /**
     * 総ルール数
     */
    int getRuleCount() const { return kMaxRules; }

private:
    Rule rules_[kMaxRules];
    SettingManager* settings_;
    ChannelManager* channels_;
    WebhookManager* webhooks_;

    void loadRules();
    void saveRuleToNvs(int idx, const Rule& rule);
    String ruleKey(int idx, const char* suffix) const;
};

#endif // RULE_MANAGER_IS05A_H
