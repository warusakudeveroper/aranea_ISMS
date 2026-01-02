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
    // インターバル状態初期化
    for (int i = 0; i < NUM_CHANNELS; i++) {
        intervalState_[i] = {false, 0, 0, false};
    }
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
        configs_[i].outputMode = OutputMode::MOMENTARY;
        configs_[i].outputDurationMs = IS05A_DEFAULT_OUTPUT_DURATION_MS;
        configs_[i].intervalCount = IS05A_DEFAULT_INTERVAL_COUNT;

        // ch7/ch8のモード設定
        if (ch == 7) {
            String mode = settings_->getString(Is05aKeys::kCh7Mode, "input");
            configs_[i].isOutput = (mode == "output");
            int modeVal = settings_->getInt(Is05aKeys::kCh7OutMode, IS05A_OUTPUT_MODE_MOMENTARY);
            configs_[i].outputMode = (modeVal >= 0 && modeVal <= 2)
                ? static_cast<OutputMode>(modeVal)
                : OutputMode::MOMENTARY;
            configs_[i].outputDurationMs = settings_->getInt(
                Is05aKeys::kCh7OutDur, IS05A_DEFAULT_OUTPUT_DURATION_MS);
            configs_[i].intervalCount = settings_->getInt(
                Is05aKeys::kCh7IntCnt, IS05A_DEFAULT_INTERVAL_COUNT);
        } else if (ch == 8) {
            String mode = settings_->getString(Is05aKeys::kCh8Mode, "input");
            configs_[i].isOutput = (mode == "output");
            int modeVal = settings_->getInt(Is05aKeys::kCh8OutMode, IS05A_OUTPUT_MODE_MOMENTARY);
            configs_[i].outputMode = (modeVal >= 0 && modeVal <= 2)
                ? static_cast<OutputMode>(modeVal)
                : OutputMode::MOMENTARY;
            configs_[i].outputDurationMs = settings_->getInt(
                Is05aKeys::kCh8OutDur, IS05A_DEFAULT_OUTPUT_DURATION_MS);
            configs_[i].intervalCount = settings_->getInt(
                Is05aKeys::kCh8IntCnt, IS05A_DEFAULT_INTERVAL_COUNT);
        }

        lastUpdatedAt_[i] = "1970-01-01T00:00:00Z";

        Serial.printf("[ChannelManager] ch%d: pin=%d, name=%s, debounce=%d, output=%s, mode=%d\n",
            ch, configs_[i].pin, configs_[i].name.c_str(),
            configs_[i].debounceMs, configs_[i].isOutput ? "true" : "false",
            static_cast<int>(configs_[i].outputMode));
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
    unsigned long now = millis();

    for (int i = 0; i < NUM_CHANNELS; i++) {
        // 通常のパルス終了チェック
        if (configs_[i].isOutput) {
            channels_[i].update();
        }

        // インターバルモード処理
        if (intervalState_[i].active && configs_[i].outputMode == OutputMode::INTERVAL) {
            int duration = configs_[i].outputDurationMs;
            if (now - intervalState_[i].lastToggleMs >= (unsigned long)duration) {
                // トグル
                intervalState_[i].currentState = !intervalState_[i].currentState;
                channels_[i].setOutput(intervalState_[i].currentState);
                intervalState_[i].lastToggleMs = now;

                // OFFになったら回数カウントダウン
                if (!intervalState_[i].currentState) {
                    intervalState_[i].remainingCount--;
                    Serial.printf("[ChannelManager] ch%d interval: %d remaining\n",
                        i + 1, intervalState_[i].remainingCount);

                    if (intervalState_[i].remainingCount <= 0) {
                        intervalState_[i].active = false;
                        Serial.printf("[ChannelManager] ch%d interval complete\n", i + 1);
                    }
                }
            }
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

    // 出力チャンネル（ch7/ch8）は on/off
    if (cfg.isOutput) {
        return active ? "on" : "off";
    }
    // 入力チャンネルは detect/idle
    return active ? "detect" : "idle";
}

String ChannelManager::getLastUpdatedAt(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) return "";
    return lastUpdatedAt_[ch - 1];
}

ChannelManager::ChannelConfig ChannelManager::getConfig(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) {
        return ChannelConfig{0, "", "", 0, false, false, OutputMode::MOMENTARY, 3000, 3};
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
        saveOutputConfig(ch);
    } else if (ch == 8) {
        settings_->setString(Is05aKeys::kCh8Mode, config.isOutput ? "output" : "input");
        saveOutputConfig(ch);
    }

    // IOController再設定
    channels_[ch - 1].setDebounceMs(config.debounceMs);
    channels_[ch - 1].setInverted(config.inverted);
}

void ChannelManager::saveOutputConfig(int ch) {
    if (ch < 7 || ch > 8) return;
    int idx = ch - 1;

    if (ch == 7) {
        settings_->setInt(Is05aKeys::kCh7OutMode, static_cast<int>(configs_[idx].outputMode));
        settings_->setInt(Is05aKeys::kCh7OutDur, configs_[idx].outputDurationMs);
        settings_->setInt(Is05aKeys::kCh7IntCnt, configs_[idx].intervalCount);
    } else {
        settings_->setInt(Is05aKeys::kCh8OutMode, static_cast<int>(configs_[idx].outputMode));
        settings_->setInt(Is05aKeys::kCh8OutDur, configs_[idx].outputDurationMs);
        settings_->setInt(Is05aKeys::kCh8IntCnt, configs_[idx].intervalCount);
    }
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

    // インターバル動作中なら停止
    intervalState_[ch - 1].active = false;

    channels_[ch - 1].setOutput(high);
}

bool ChannelManager::getOutputState(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) return false;
    if (!configs_[ch - 1].isOutput) return false;
    return channels_[ch - 1].getOutput();
}

void ChannelManager::pulse(int ch, int durationMs) {
    if (ch < 1 || ch > NUM_CHANNELS) return;
    if (!configs_[ch - 1].isOutput) return;

    // インターバル動作中なら停止
    intervalState_[ch - 1].active = false;

    channels_[ch - 1].pulse(durationMs);
}

void ChannelManager::setOutputBehavior(int ch, OutputMode mode, int durationMs, int count) {
    if (ch < 7 || ch > 8) return;

    int idx = ch - 1;
    configs_[idx].outputMode = mode;
    configs_[idx].outputDurationMs = constrain(durationMs, 500, 10000);
    configs_[idx].intervalCount = constrain(count, 1, 100);

    saveOutputConfig(ch);

    Serial.printf("[ChannelManager] ch%d output: mode=%d, dur=%d, cnt=%d\n",
        ch, static_cast<int>(mode), durationMs, count);
}

ChannelManager::OutputMode ChannelManager::getOutputBehavior(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) return OutputMode::MOMENTARY;
    return configs_[ch - 1].outputMode;
}

void ChannelManager::triggerOutput(int ch) {
    if (ch < 7 || ch > 8) return;
    if (!configs_[ch - 1].isOutput) {
        Serial.printf("[ChannelManager] ch%d is not in output mode\n", ch);
        return;
    }

    int idx = ch - 1;
    OutputMode mode = configs_[idx].outputMode;

    switch (mode) {
        case OutputMode::MOMENTARY:
            // 指定時間後にOFF
            channels_[idx].pulse(configs_[idx].outputDurationMs);
            Serial.printf("[ChannelManager] ch%d momentary: %dms\n",
                ch, configs_[idx].outputDurationMs);
            break;

        case OutputMode::ALTERNATE:
            // トグル動作
            {
                bool current = channels_[idx].getOutput();
                channels_[idx].setOutput(!current);
                Serial.printf("[ChannelManager] ch%d alternate: %s -> %s\n",
                    ch, current ? "ON" : "OFF", !current ? "ON" : "OFF");
            }
            break;

        case OutputMode::INTERVAL:
            // インターバル開始
            intervalState_[idx].active = true;
            intervalState_[idx].remainingCount = configs_[idx].intervalCount;
            intervalState_[idx].lastToggleMs = millis();
            intervalState_[idx].currentState = true;
            channels_[idx].setOutput(true);
            Serial.printf("[ChannelManager] ch%d interval: %dx %dms\n",
                ch, configs_[idx].intervalCount, configs_[idx].outputDurationMs);
            break;
    }
}

void ChannelManager::stopOutput(int ch) {
    if (ch < 7 || ch > 8) return;
    if (!configs_[ch - 1].isOutput) return;

    int idx = ch - 1;
    intervalState_[idx].active = false;
    channels_[idx].setOutput(false);

    Serial.printf("[ChannelManager] ch%d output stopped\n", ch);
}

IOController* ChannelManager::getController(int ch) {
    if (ch < 1 || ch > NUM_CHANNELS) return nullptr;
    return &channels_[ch - 1];
}

void ChannelManager::onChannelChange(std::function<void(int ch, bool active)> callback) {
    onChangeCallback_ = callback;
}

String ChannelManager::toJson() const {
    DynamicJsonDocument doc(2048);
    for (int ch = 1; ch <= NUM_CHANNELS; ch++) {
        JsonObject chObj = doc.createNestedObject("ch" + String(ch));
        const ChannelConfig& cfg = configs_[ch - 1];
        chObj["pin"] = cfg.pin;
        chObj["name"] = cfg.name;
        chObj["meaning"] = cfg.meaning;
        chObj["debounce"] = cfg.debounceMs;
        chObj["inverted"] = cfg.inverted;
        chObj["isOutput"] = cfg.isOutput;
        chObj["outputMode"] = static_cast<int>(cfg.outputMode);
        chObj["outputDuration"] = cfg.outputDurationMs;
        chObj["intervalCount"] = cfg.intervalCount;
        chObj["state"] = getStateString(ch);
        chObj["outputState"] = cfg.isOutput ? getOutputState(ch) : false;
        chObj["lastUpdatedAt"] = lastUpdatedAt_[ch - 1];
    }
    String json;
    serializeJson(doc, json);
    return json;
}

String ChannelManager::getChannelJson(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) return "{}";

    DynamicJsonDocument doc(512);
    const ChannelConfig& cfg = configs_[ch - 1];
    doc["pin"] = cfg.pin;
    doc["name"] = cfg.name;
    doc["meaning"] = cfg.meaning;
    doc["debounce"] = cfg.debounceMs;
    doc["inverted"] = cfg.inverted;
    doc["isOutput"] = cfg.isOutput;
    doc["outputMode"] = static_cast<int>(cfg.outputMode);
    doc["outputDuration"] = cfg.outputDurationMs;
    doc["intervalCount"] = cfg.intervalCount;
    doc["state"] = getStateString(ch);
    doc["outputState"] = cfg.isOutput ? getOutputState(ch) : false;
    doc["lastUpdatedAt"] = lastUpdatedAt_[ch - 1];

    String json;
    serializeJson(doc, json);
    return json;
}
