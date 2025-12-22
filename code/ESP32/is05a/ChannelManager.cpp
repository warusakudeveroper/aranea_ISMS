/**
 * ChannelManager.cpp
 */

#include "ChannelManager.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "Is05aKeys.h"
#include "AraneaSettings.h"
#include <ArduinoJson.h>

ChannelManager::ChannelManager()
    : settings_(nullptr)
    , ntp_(nullptr)
    , changed_(false)
    , lastChangedCh_(-1)
    , onChangeCallback_(nullptr)
{
}

void ChannelManager::begin(SettingManager* settings, NtpManager* ntp) {
    settings_ = settings;
    ntp_ = ntp;

    loadConfig();
    initChannels();

    Serial.println("[ChannelManager] Initialized 8 channels");
}

void ChannelManager::loadConfig() {
    for (int i = 0; i < NUM_CHANNELS; i++) {
        int ch = i + 1;
        String keyPin = Is05aChannelKeys::getChannelKey(ch, Is05aChannelKeys::kPin);
        String keyName = Is05aChannelKeys::getChannelKey(ch, Is05aChannelKeys::kName);
        String keyMean = Is05aChannelKeys::getChannelKey(ch, Is05aChannelKeys::kMeaning);
        String keyDeb = Is05aChannelKeys::getChannelKey(ch, Is05aChannelKeys::kDebounce);
        String keyInv = Is05aChannelKeys::getChannelKey(ch, Is05aChannelKeys::kInverted);

        configs_[i].pin = settings_->getInt(keyPin.c_str(), AraneaSettings::getDefaultPin(ch));
        configs_[i].name = settings_->getString(keyName.c_str(), "ch" + String(ch));
        configs_[i].meaning = settings_->getString(keyMean.c_str(), "open");
        configs_[i].debounceMs = settings_->getInt(keyDeb.c_str(), AraneaSettings::getDefaultDebounceMs());
        configs_[i].inverted = settings_->getBool(keyInv.c_str(), false);
        configs_[i].isOutput = false;

        // ch7/ch8のモード設定
        if (ch == 7) {
            String mode = settings_->getString(Is05aKeys::kCh7Mode, "input");
            configs_[i].isOutput = (mode == "output");
        } else if (ch == 8) {
            String mode = settings_->getString(Is05aKeys::kCh8Mode, "input");
            configs_[i].isOutput = (mode == "output");
        }

        lastUpdatedAt_[i] = "1970-01-01T00:00:00Z";

        Serial.printf("[ChannelManager] ch%d: pin=%d, name=%s, meaning=%s, debounce=%d, output=%s\n",
            ch, configs_[i].pin, configs_[i].name.c_str(), configs_[i].meaning.c_str(),
            configs_[i].debounceMs, configs_[i].isOutput ? "true" : "false");
    }
}

void ChannelManager::initChannels() {
    for (int i = 0; i < NUM_CHANNELS; i++) {
        int ch = i + 1;
        if (configs_[i].pin <= 0) continue;

        IOController::Mode mode = configs_[i].isOutput
            ? IOController::Mode::IO_OUT
            : IOController::Mode::IO_IN;

        channels_[i].begin(configs_[i].pin, mode);
        channels_[i].setDebounceMs(configs_[i].debounceMs);
        channels_[i].setInverted(configs_[i].inverted);

        // 状態変化コールバック
        channels_[i].onStateChange([this, ch](bool active) {
            int idx = ch - 1;
            lastUpdatedAt_[idx] = (ntp_ && ntp_->isSynced())
                ? ntp_->getIso8601()
                : "1970-01-01T00:00:00Z";
            changed_ = true;
            lastChangedCh_ = ch;

            Serial.printf("[ChannelManager] ch%d changed: %s\n",
                ch, active ? "ACTIVE" : "INACTIVE");

            if (onChangeCallback_) {
                onChangeCallback_(ch, active);
            }
        });
    }
}

void ChannelManager::sample() {
    for (int i = 0; i < NUM_CHANNELS; i++) {
        if (configs_[i].pin > 0 && !configs_[i].isOutput) {
            channels_[i].sample();
        }
    }
}

void ChannelManager::update() {
    for (int i = 0; i < NUM_CHANNELS; i++) {
        if (configs_[i].isOutput) {
            channels_[i].update();
        }
    }
}

bool ChannelManager::isActive(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) return false;
    return channels_[ch - 1].isActive();
}

String ChannelManager::getStateString(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) return "";
    const ChannelConfig& cfg = configs_[ch - 1];
    bool active = channels_[ch - 1].isActive();

    // アクティブ時はmeaningをそのまま、非アクティブ時は逆
    if (active) {
        return cfg.meaning;
    } else {
        return (cfg.meaning == "open") ? "close" : "open";
    }
}

String ChannelManager::getLastUpdatedAt(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) return "";
    return lastUpdatedAt_[ch - 1];
}

ChannelManager::ChannelConfig ChannelManager::getConfig(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) {
        return ChannelConfig{0, "", "", 0, false, false};
    }
    return configs_[ch - 1];
}

void ChannelManager::setConfig(int ch, const ChannelConfig& config) {
    if (ch < 1 || ch > NUM_CHANNELS) return;
    configs_[ch - 1] = config;

    // NVSに保存
    String keyPin = Is05aChannelKeys::getChannelKey(ch, Is05aChannelKeys::kPin);
    String keyName = Is05aChannelKeys::getChannelKey(ch, Is05aChannelKeys::kName);
    String keyMean = Is05aChannelKeys::getChannelKey(ch, Is05aChannelKeys::kMeaning);
    String keyDeb = Is05aChannelKeys::getChannelKey(ch, Is05aChannelKeys::kDebounce);
    String keyInv = Is05aChannelKeys::getChannelKey(ch, Is05aChannelKeys::kInverted);

    settings_->setInt(keyPin.c_str(), config.pin);
    settings_->setString(keyName.c_str(), config.name);
    settings_->setString(keyMean.c_str(), config.meaning);
    settings_->setInt(keyDeb.c_str(), config.debounceMs);
    settings_->setBool(keyInv.c_str(), config.inverted);

    // ch7/ch8のモード保存
    if (ch == 7) {
        settings_->setString(Is05aKeys::kCh7Mode, config.isOutput ? "output" : "input");
    } else if (ch == 8) {
        settings_->setString(Is05aKeys::kCh8Mode, config.isOutput ? "output" : "input");
    }

    // IOController再設定
    channels_[ch - 1].setDebounceMs(config.debounceMs);
    channels_[ch - 1].setInverted(config.inverted);
}

bool ChannelManager::setOutputMode(int ch, bool output) {
    if (ch < 7 || ch > 8) {
        Serial.printf("[ChannelManager] ERROR: Only ch7/ch8 support I/O switch\n");
        return false;
    }

    int idx = ch - 1;
    IOController::Mode mode = output
        ? IOController::Mode::IO_OUT
        : IOController::Mode::IO_IN;

    if (!channels_[idx].setMode(mode)) {
        return false;
    }

    configs_[idx].isOutput = output;

    // NVS保存
    if (ch == 7) {
        settings_->setString(Is05aKeys::kCh7Mode, output ? "output" : "input");
    } else {
        settings_->setString(Is05aKeys::kCh8Mode, output ? "output" : "input");
    }

    return true;
}

bool ChannelManager::isOutputMode(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) return false;
    return configs_[ch - 1].isOutput;
}

void ChannelManager::setOutput(int ch, bool high) {
    if (ch < 1 || ch > NUM_CHANNELS) return;
    if (!configs_[ch - 1].isOutput) return;
    channels_[ch - 1].setOutput(high);
}

void ChannelManager::pulse(int ch, int durationMs) {
    if (ch < 1 || ch > NUM_CHANNELS) return;
    if (!configs_[ch - 1].isOutput) return;
    channels_[ch - 1].pulse(durationMs);
}

IOController* ChannelManager::getController(int ch) {
    if (ch < 1 || ch > NUM_CHANNELS) return nullptr;
    return &channels_[ch - 1];
}

void ChannelManager::onChannelChange(std::function<void(int ch, bool active)> callback) {
    onChangeCallback_ = callback;
}

String ChannelManager::toJson() const {
    JsonDocument doc;
    for (int ch = 1; ch <= NUM_CHANNELS; ch++) {
        JsonObject chObj = doc.createNestedObject("ch" + String(ch));
        const ChannelConfig& cfg = configs_[ch - 1];
        chObj["pin"] = cfg.pin;
        chObj["name"] = cfg.name;
        chObj["meaning"] = cfg.meaning;
        chObj["debounce"] = cfg.debounceMs;
        chObj["inverted"] = cfg.inverted;
        chObj["isOutput"] = cfg.isOutput;
        chObj["state"] = getStateString(ch);
        chObj["lastUpdatedAt"] = lastUpdatedAt_[ch - 1];
    }
    String json;
    serializeJson(doc, json);
    return json;
}

String ChannelManager::getChannelJson(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) return "{}";

    JsonDocument doc;
    const ChannelConfig& cfg = configs_[ch - 1];
    doc["pin"] = cfg.pin;
    doc["name"] = cfg.name;
    doc["meaning"] = cfg.meaning;
    doc["debounce"] = cfg.debounceMs;
    doc["inverted"] = cfg.inverted;
    doc["isOutput"] = cfg.isOutput;
    doc["state"] = getStateString(ch);
    doc["lastUpdatedAt"] = lastUpdatedAt_[ch - 1];

    String json;
    serializeJson(doc, json);
    return json;
}
