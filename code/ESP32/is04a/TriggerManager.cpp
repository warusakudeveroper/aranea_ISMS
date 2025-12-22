/**
 * TriggerManager.cpp
 */

#include "TriggerManager.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "Is04aKeys.h"
#include "AraneaSettings.h"
#include <ArduinoJson.h>

TriggerManager::TriggerManager()
    : settings_(nullptr)
    , ntp_(nullptr)
    , pulseMs_(3000)
    , interlockMs_(200)
    , debounceMs_(50)
    , changed_(false)
    , currentSource_(PulseSource::NONE)
    , currentOutput_(0)
    , lastPulseEndMs_(0)
    , onPulseStartCallback_(nullptr)
    , onPulseEndCallback_(nullptr)
    , onInputChangeCallback_(nullptr)
{
    triggerAssign_[0] = 1;  // 入力1 → 出力1
    triggerAssign_[1] = 2;  // 入力2 → 出力2
    outputNames_[0] = "OPEN";
    outputNames_[1] = "CLOSE";
    lastInputStable_[0] = false;
    lastInputStable_[1] = false;
    lastUpdatedAt_[0] = "1970-01-01T00:00:00Z";
    lastUpdatedAt_[1] = "1970-01-01T00:00:00Z";
}

void TriggerManager::begin(SettingManager* settings, NtpManager* ntp) {
    settings_ = settings;
    ntp_ = ntp;

    loadConfig();

    // 出力初期化（最優先でLOWに設定）
    int out1Pin = settings_->getInt(Is04aKeys::kOut1Pin, AraneaSettings::getDefaultOut1Pin());
    int out2Pin = settings_->getInt(Is04aKeys::kOut2Pin, AraneaSettings::getDefaultOut2Pin());

    outputs_[0].begin(out1Pin, IOController::Mode::IO_OUT);
    outputs_[1].begin(out2Pin, IOController::Mode::IO_OUT);

    // パルス終了コールバック
    outputs_[0].onPulseEnd([this]() {
        lastPulseEndMs_ = millis();
        currentSource_ = PulseSource::NONE;
        currentOutput_ = 0;
        changed_ = true;
        if (onPulseEndCallback_) {
            onPulseEndCallback_(1);
        }
    });

    outputs_[1].onPulseEnd([this]() {
        lastPulseEndMs_ = millis();
        currentSource_ = PulseSource::NONE;
        currentOutput_ = 0;
        changed_ = true;
        if (onPulseEndCallback_) {
            onPulseEndCallback_(2);
        }
    });

    // 入力初期化
    int in1Pin = settings_->getInt(Is04aKeys::kIn1Pin, AraneaSettings::getDefaultIn1Pin());
    int in2Pin = settings_->getInt(Is04aKeys::kIn2Pin, AraneaSettings::getDefaultIn2Pin());

    inputs_[0].begin(in1Pin, IOController::Mode::IO_IN);
    inputs_[0].setDebounceMs(debounceMs_);
    inputs_[0].setPullup(false);   // INPUT_PULLDOWN
    inputs_[0].setInverted(true);  // HIGHでアクティブ（デフォルトはLOW=アクティブ）

    inputs_[1].begin(in2Pin, IOController::Mode::IO_IN);
    inputs_[1].setDebounceMs(debounceMs_);
    inputs_[1].setPullup(false);   // INPUT_PULLDOWN
    inputs_[1].setInverted(true);  // HIGHでアクティブ

    // 初期状態を取得
    delay(10);
    lastInputStable_[0] = inputs_[0].isActive();
    lastInputStable_[1] = inputs_[1].isActive();

    Serial.printf("[TriggerManager] Initialized: IN1=GPIO%d, IN2=GPIO%d, OUT1=GPIO%d, OUT2=GPIO%d\n",
        in1Pin, in2Pin, out1Pin, out2Pin);
    Serial.printf("[TriggerManager] Pulse=%dms, Interlock=%dms, Debounce=%dms\n",
        pulseMs_, interlockMs_, debounceMs_);
    Serial.printf("[TriggerManager] Assign: IN1->OUT%d, IN2->OUT%d\n",
        triggerAssign_[0], triggerAssign_[1]);
}

void TriggerManager::loadConfig() {
    pulseMs_ = settings_->getInt(Is04aKeys::kPulseMs, AraneaSettings::getDefaultPulseMs());
    interlockMs_ = settings_->getInt(Is04aKeys::kInterlockMs, AraneaSettings::getDefaultInterlockMs());
    debounceMs_ = settings_->getInt(Is04aKeys::kDebounceMs, AraneaSettings::getDefaultDebounceMs());

    triggerAssign_[0] = settings_->getInt(Is04aKeys::kIn1Target, AraneaSettings::getDefaultIn1Target());
    triggerAssign_[1] = settings_->getInt(Is04aKeys::kIn2Target, AraneaSettings::getDefaultIn2Target());

    outputNames_[0] = settings_->getString(Is04aKeys::kOut1Name, AraneaSettings::getDefaultOut1Name());
    outputNames_[1] = settings_->getString(Is04aKeys::kOut2Name, AraneaSettings::getDefaultOut2Name());

    // パルスms範囲制限
    if (pulseMs_ < 10) pulseMs_ = 10;
    if (pulseMs_ > 10000) pulseMs_ = 10000;
}

void TriggerManager::sample() {
    inputs_[0].sample();
    inputs_[1].sample();
    handleInputEdges();
}

void TriggerManager::update() {
    outputs_[0].update();
    outputs_[1].update();
}

void TriggerManager::handleInputEdges() {
    // 入力1のエッジ検出
    bool current1 = inputs_[0].isActive();
    if (current1 && !lastInputStable_[0]) {
        // 立ち上がりエッジ
        Serial.println("[TriggerManager] IN1 rising edge detected");
        lastUpdatedAt_[0] = getTimestamp();
        changed_ = true;

        if (onInputChangeCallback_) {
            onInputChangeCallback_(1, true);
        }

        // トリガー実行
        int target = triggerAssign_[0];
        if (!isPulseActive()) {
            startPulse(target, pulseMs_, PulseSource::MANUAL);
        } else {
            Serial.println("[TriggerManager] Pulse active, manual rejected");
        }
    } else if (!current1 && lastInputStable_[0]) {
        // 立ち下がりエッジ
        lastUpdatedAt_[0] = getTimestamp();
        changed_ = true;
        if (onInputChangeCallback_) {
            onInputChangeCallback_(1, false);
        }
    }
    lastInputStable_[0] = current1;

    // 入力2のエッジ検出
    bool current2 = inputs_[1].isActive();
    if (current2 && !lastInputStable_[1]) {
        Serial.println("[TriggerManager] IN2 rising edge detected");
        lastUpdatedAt_[1] = getTimestamp();
        changed_ = true;

        if (onInputChangeCallback_) {
            onInputChangeCallback_(2, true);
        }

        int target = triggerAssign_[1];
        if (!isPulseActive()) {
            startPulse(target, pulseMs_, PulseSource::MANUAL);
        } else {
            Serial.println("[TriggerManager] Pulse active, manual rejected");
        }
    } else if (!current2 && lastInputStable_[1]) {
        lastUpdatedAt_[1] = getTimestamp();
        changed_ = true;
        if (onInputChangeCallback_) {
            onInputChangeCallback_(2, false);
        }
    }
    lastInputStable_[1] = current2;
}

bool TriggerManager::startPulse(int outputNum, int durationMs, PulseSource source) {
    if (outputNum < 1 || outputNum > 2) return false;

    // パルス実行中チェック
    if (isPulseActive()) {
        Serial.println("[TriggerManager] Pulse already active");
        return false;
    }

    // インターロックチェック
    unsigned long now = millis();
    if (lastPulseEndMs_ > 0 && (now - lastPulseEndMs_) < (unsigned long)interlockMs_) {
        Serial.printf("[TriggerManager] Interlock active (%lums remaining)\n",
            interlockMs_ - (now - lastPulseEndMs_));
        return false;
    }

    int idx = outputNum - 1;
    outputs_[idx].pulse(durationMs);
    currentSource_ = source;
    currentOutput_ = outputNum;
    changed_ = true;

    Serial.printf("[TriggerManager] Pulse started: OUT%d, %dms, source=%d\n",
        outputNum, durationMs, (int)source);

    if (onPulseStartCallback_) {
        onPulseStartCallback_(outputNum, source);
    }

    return true;
}

bool TriggerManager::isPulseActive() const {
    return outputs_[0].isPulseActive() || outputs_[1].isPulseActive();
}

bool TriggerManager::isPulseActive(int outputNum) const {
    if (outputNum < 1 || outputNum > 2) return false;
    return outputs_[outputNum - 1].isPulseActive();
}

bool TriggerManager::isInputActive(int inputNum) const {
    if (inputNum < 1 || inputNum > 2) return false;
    return inputs_[inputNum - 1].isActive();
}

TriggerManager::InputState TriggerManager::getInputState(int inputNum) const {
    InputState state = {false, 0, 0, ""};
    if (inputNum < 1 || inputNum > 2) return state;

    int idx = inputNum - 1;
    state.active = inputs_[idx].isActive();
    state.pin = inputs_[idx].getPin();
    state.targetOutput = triggerAssign_[idx];
    state.lastUpdatedAt = lastUpdatedAt_[idx];
    return state;
}

TriggerManager::OutputState TriggerManager::getOutputState(int outputNum) const {
    OutputState state = {false, 0, "", 0, 0, PulseSource::NONE};
    if (outputNum < 1 || outputNum > 2) return state;

    int idx = outputNum - 1;
    state.active = outputs_[idx].isPulseActive();
    state.pin = outputs_[idx].getPin();
    state.name = outputNames_[idx];
    if (state.active && currentOutput_ == outputNum) {
        state.source = currentSource_;
    }
    return state;
}

void TriggerManager::setTriggerAssign(int inputNum, int targetOutput) {
    if (inputNum < 1 || inputNum > 2) return;
    if (targetOutput < 1 || targetOutput > 2) return;

    triggerAssign_[inputNum - 1] = targetOutput;

    if (inputNum == 1) {
        settings_->setInt(Is04aKeys::kIn1Target, targetOutput);
    } else {
        settings_->setInt(Is04aKeys::kIn2Target, targetOutput);
    }
}

int TriggerManager::getTriggerAssign(int inputNum) const {
    if (inputNum < 1 || inputNum > 2) return 0;
    return triggerAssign_[inputNum - 1];
}

void TriggerManager::setOutputName(int outputNum, const String& name) {
    if (outputNum < 1 || outputNum > 2) return;

    outputNames_[outputNum - 1] = name;

    if (outputNum == 1) {
        settings_->setString(Is04aKeys::kOut1Name, name);
    } else {
        settings_->setString(Is04aKeys::kOut2Name, name);
    }
}

String TriggerManager::getOutputName(int outputNum) const {
    if (outputNum < 1 || outputNum > 2) return "";
    return outputNames_[outputNum - 1];
}

void TriggerManager::setPulseMs(int ms) {
    if (ms < 10) ms = 10;
    if (ms > 10000) ms = 10000;
    pulseMs_ = ms;
    settings_->setInt(Is04aKeys::kPulseMs, ms);
}

void TriggerManager::setInterlockMs(int ms) {
    if (ms < 0) ms = 0;
    if (ms > 5000) ms = 5000;
    interlockMs_ = ms;
    settings_->setInt(Is04aKeys::kInterlockMs, ms);
}

void TriggerManager::setDebounceMs(int ms) {
    if (ms < 5) ms = 5;
    if (ms > 1000) ms = 1000;
    debounceMs_ = ms;
    settings_->setInt(Is04aKeys::kDebounceMs, ms);

    inputs_[0].setDebounceMs(ms);
    inputs_[1].setDebounceMs(ms);
}

void TriggerManager::onPulseStart(std::function<void(int, PulseSource)> callback) {
    onPulseStartCallback_ = callback;
}

void TriggerManager::onPulseEnd(std::function<void(int)> callback) {
    onPulseEndCallback_ = callback;
}

void TriggerManager::onInputChange(std::function<void(int, bool)> callback) {
    onInputChangeCallback_ = callback;
}

String TriggerManager::getTimestamp() const {
    if (ntp_ && ntp_->isSynced()) {
        return ntp_->getIso8601();
    }
    return "1970-01-01T00:00:00Z";
}

String TriggerManager::toJson() const {
    JsonDocument doc;

    // 入力
    JsonObject inputs = doc.createNestedObject("inputs");
    for (int i = 1; i <= 2; i++) {
        InputState state = getInputState(i);
        JsonObject inObj = inputs.createNestedObject("in" + String(i));
        inObj["active"] = state.active;
        inObj["pin"] = state.pin;
        inObj["target"] = state.targetOutput;
        inObj["lastUpdatedAt"] = state.lastUpdatedAt;
    }

    // 出力
    JsonObject outputs = doc.createNestedObject("outputs");
    for (int i = 1; i <= 2; i++) {
        OutputState state = getOutputState(i);
        JsonObject outObj = outputs.createNestedObject("out" + String(i));
        outObj["active"] = state.active;
        outObj["pin"] = state.pin;
        outObj["name"] = state.name;
    }

    // 設定
    doc["pulseMs"] = pulseMs_;
    doc["interlockMs"] = interlockMs_;
    doc["debounceMs"] = debounceMs_;

    String json;
    serializeJson(doc, json);
    return json;
}
