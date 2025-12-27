/**
 * TriggerManagerIs06a.cpp
 */

#include "TriggerManagerIs06a.h"
#include "SettingManager.h"
#include "NtpManager.h"
#include "Is06aKeys.h"
#include "AraneaSettings.h"
#include <ArduinoJson.h>

// GPIOピン配列（6CH）
static const int kGpioPins[6] = {
    IS06A_GPIO_TRG1, IS06A_GPIO_TRG2, IS06A_GPIO_TRG3, IS06A_GPIO_TRG4,
    IS06A_GPIO_TRG5, IS06A_GPIO_TRG6
};

// 入力クールダウン時間（チャタリング防止）
// 注: IOControllerのデバウンス処理と併用。50msで十分なチャタリング防止が可能。
//     500msでは短いパルスを検出できない問題があったため短縮。
static const unsigned long INPUT_COOLDOWN_MS = 50;

TriggerManagerIs06a::TriggerManagerIs06a()
    : settings_(nullptr)
    , ntp_(nullptr)
    , interlockMs_(200)
    , changed_(false)
    , lastPulseEndMs_(0)
    , currentPulseTrg_(0)
    , currentSource_(PulseSource::NONE)
    , onPulseStartCallback_(nullptr)
    , onPulseEndCallback_(nullptr)
    , onInputChangeCallback_(nullptr)
    , onPwmChangeCallback_(nullptr)
{
    for (int i = 0; i < kNumTriggers; i++) {
        modes_[i] = TriggerMode::MODE_DIGITAL;
        names_[i] = "TRG" + String(i + 1);
        pulseMsArr_[i] = 3000;
        pwmFreqArr_[i] = 1000;
        pwmDutyArr_[i] = 128;
        debounceMsArr_[i] = 100;
        lastUpdatedAt_[i] = "1970-01-01T00:00:00Z";
    }
    for (int i = 0; i < kNumPwmCapable; i++) {
        pwmActive_[i] = false;
        ledcChannels_[i] = -1;  // 未割り当て
    }
    for (int i = 0; i < kNumIoSwitchable; i++) {
        lastInputStable_[i] = false;
        lastInputEdgeMs_[i] = 0;
    }
}

void TriggerManagerIs06a::begin(SettingManager* settings, NtpManager* ntp) {
    settings_ = settings;
    ntp_ = ntp;

    loadConfig();

    // TRG1-3: デジタル/PWM出力（通常初期化）
    for (int i = 0; i < kNumPwmCapable - 1; i++) {  // TRG1-3のみ（TRG4は別処理）
        io_[i].begin(kGpioPins[i], IOController::Mode::IO_OUT);

        // パルス終了コールバック
        int trgNum = i + 1;
        io_[i].onPulseEnd([this, trgNum]() {
            lastPulseEndMs_ = millis();
            if (currentPulseTrg_ == trgNum) {
                currentSource_ = PulseSource::NONE;
                currentPulseTrg_ = 0;
            }
            changed_ = true;
            if (onPulseEndCallback_) {
                onPulseEndCallback_(trgNum);
            }
        });

        // PWMモードの場合はセットアップ
        if (modes_[i] == TriggerMode::MODE_PWM) {
            setupPwm(trgNum);
        }
    }

    // TRG4(GPIO15): ストラップピン専用初期化
    // GPIO15はブート時HIGH必須。IOController::begin()→transitionToOutput()はLOW初期化するため、
    // IOControllerを使わず直接GPIO制御することで、LOWパルスを完全に回避する。
    // 注意: TRG4はIOControllerの内部状態管理を使用しない（直接GPIO制御のみ）
    {
        int idx = 3;  // TRG4のインデックス
        int trgNum = 4;

        // GPIO15を最初からHIGHで出力設定（LOWを経由しない）
        digitalWrite(IS06A_GPIO_TRG4, HIGH);
        pinMode(IS06A_GPIO_TRG4, OUTPUT);
        Serial.println("[TriggerManagerIs06a] TRG4(GPIO15) initialized HIGH (strap pin, no IOController)");

        // IOControllerはbegin()しない - 直接GPIO制御を使用
        // パルス終了コールバック（TRG4がPWM/DIGITALで動作する場合用）
        io_[idx].onPulseEnd([this, trgNum]() {
            lastPulseEndMs_ = millis();
            if (currentPulseTrg_ == trgNum) {
                currentSource_ = PulseSource::NONE;
                currentPulseTrg_ = 0;
            }
            changed_ = true;
            if (onPulseEndCallback_) {
                onPulseEndCallback_(trgNum);
            }
        });

        // PWMモードの場合はLEDC制御に切り替え
        if (modes_[idx] == TriggerMode::MODE_PWM) {
            setupPwm(trgNum);
        }
        // DIGITALモードは既にHIGH設定済み（アイドル状態）
    }

    // TRG5-6: I/O切替可能
    for (int i = 0; i < kNumIoSwitchable; i++) {
        int idx = kNumPwmCapable + i;  // TRG4=idx3, TRG5=idx4
        int trgNum = idx + 1;
        IOController::Mode ioMode = (modes_[idx] == TriggerMode::MODE_INPUT)
            ? IOController::Mode::IO_IN
            : IOController::Mode::IO_OUT;

        io_[idx].begin(kGpioPins[idx], ioMode);

        if (modes_[idx] == TriggerMode::MODE_INPUT) {
            io_[idx].setDebounceMs(debounceMsArr_[idx]);
            io_[idx].setPullup(false);    // INPUT_PULLDOWN
            io_[idx].setInverted(true);   // HIGHでアクティブ
        } else {
            io_[idx].onPulseEnd([this, trgNum]() {
                lastPulseEndMs_ = millis();
                if (currentPulseTrg_ == trgNum) {
                    currentSource_ = PulseSource::NONE;
                    currentPulseTrg_ = 0;
                }
                changed_ = true;
                if (onPulseEndCallback_) {
                    onPulseEndCallback_(trgNum);
                }
            });
        }
    }

    // 入力初期状態を取得（TRG5-6がINPUTモード時）
    delay(10);
    for (int i = 0; i < kNumIoSwitchable; i++) {
        int idx = kNumPwmCapable + i;  // TRG4=idx3, TRG5=idx4
        if (modes_[idx] == TriggerMode::MODE_INPUT) {
            lastInputStable_[i] = io_[idx].isActive();
        }
    }

    Serial.println("[TriggerManagerIs06a] Initialized (6CH)");
    for (int i = 0; i < kNumTriggers; i++) {
        const char* modeStr = (modes_[i] == TriggerMode::MODE_PWM) ? "PWM" :
                              (modes_[i] == TriggerMode::MODE_INPUT) ? "INPUT" : "DIGITAL";
        Serial.printf("  TRG%d: GPIO%d, Mode=%s\n", i + 1, kGpioPins[i], modeStr);
    }
}

void TriggerManagerIs06a::loadConfig() {
    interlockMs_ = settings_->getInt(Is06aKeys::kInterlockMs, IS06A_DEFAULT_INTERLOCK_MS);

    // TRG1設定
    names_[0] = settings_->getString(Is06aKeys::kTrg1Name, IS06A_DEFAULT_TRG1_NAME);
    String mode1 = settings_->getString(Is06aKeys::kTrg1Mode, IS06A_DEFAULT_TRG_MODE);
    modes_[0] = (mode1 == "pwm") ? TriggerMode::MODE_PWM : TriggerMode::MODE_DIGITAL;
    pulseMsArr_[0] = settings_->getInt(Is06aKeys::kTrg1PulseMs, IS06A_DEFAULT_PULSE_MS);
    pwmFreqArr_[0] = settings_->getInt(Is06aKeys::kTrg1PwmFreq, IS06A_DEFAULT_PWM_FREQ);
    pwmDutyArr_[0] = settings_->getInt(Is06aKeys::kTrg1PwmDuty, IS06A_DEFAULT_PWM_DUTY);

    // TRG2設定
    names_[1] = settings_->getString(Is06aKeys::kTrg2Name, IS06A_DEFAULT_TRG2_NAME);
    String mode2 = settings_->getString(Is06aKeys::kTrg2Mode, IS06A_DEFAULT_TRG_MODE);
    modes_[1] = (mode2 == "pwm") ? TriggerMode::MODE_PWM : TriggerMode::MODE_DIGITAL;
    pulseMsArr_[1] = settings_->getInt(Is06aKeys::kTrg2PulseMs, IS06A_DEFAULT_PULSE_MS);
    pwmFreqArr_[1] = settings_->getInt(Is06aKeys::kTrg2PwmFreq, IS06A_DEFAULT_PWM_FREQ);
    pwmDutyArr_[1] = settings_->getInt(Is06aKeys::kTrg2PwmDuty, IS06A_DEFAULT_PWM_DUTY);

    // TRG3設定
    names_[2] = settings_->getString(Is06aKeys::kTrg3Name, IS06A_DEFAULT_TRG3_NAME);
    String mode3 = settings_->getString(Is06aKeys::kTrg3Mode, IS06A_DEFAULT_TRG_MODE);
    modes_[2] = (mode3 == "pwm") ? TriggerMode::MODE_PWM : TriggerMode::MODE_DIGITAL;
    pulseMsArr_[2] = settings_->getInt(Is06aKeys::kTrg3PulseMs, IS06A_DEFAULT_PULSE_MS);
    pwmFreqArr_[2] = settings_->getInt(Is06aKeys::kTrg3PwmFreq, IS06A_DEFAULT_PWM_FREQ);
    pwmDutyArr_[2] = settings_->getInt(Is06aKeys::kTrg3PwmDuty, IS06A_DEFAULT_PWM_DUTY);

    // TRG4設定（PWM対応）
    names_[3] = settings_->getString(Is06aKeys::kTrg4Name, IS06A_DEFAULT_TRG4_NAME);
    String mode4 = settings_->getString(Is06aKeys::kTrg4Mode, IS06A_DEFAULT_TRG_MODE);
    modes_[3] = (mode4 == "pwm") ? TriggerMode::MODE_PWM : TriggerMode::MODE_DIGITAL;
    pulseMsArr_[3] = settings_->getInt(Is06aKeys::kTrg4PulseMs, IS06A_DEFAULT_PULSE_MS);
    pwmFreqArr_[3] = settings_->getInt(Is06aKeys::kTrg4PwmFreq, IS06A_DEFAULT_PWM_FREQ);
    pwmDutyArr_[3] = settings_->getInt(Is06aKeys::kTrg4PwmDuty, IS06A_DEFAULT_PWM_DUTY);

    // TRG5設定（I/O切替可能）
    names_[4] = settings_->getString(Is06aKeys::kTrg5Name, IS06A_DEFAULT_TRG5_NAME);
    String io5 = settings_->getString(Is06aKeys::kTrg5IoMode, IS06A_DEFAULT_IO_MODE);
    modes_[4] = (io5 == "input") ? TriggerMode::MODE_INPUT : TriggerMode::MODE_DIGITAL;
    pulseMsArr_[4] = settings_->getInt(Is06aKeys::kTrg5PulseMs, IS06A_DEFAULT_PULSE_MS);
    debounceMsArr_[4] = settings_->getInt(Is06aKeys::kTrg5Debounce, IS06A_DEFAULT_DEBOUNCE_MS);

    // TRG6設定（I/O切替可能）
    names_[5] = settings_->getString(Is06aKeys::kTrg6Name, IS06A_DEFAULT_TRG6_NAME);
    String io6 = settings_->getString(Is06aKeys::kTrg6IoMode, IS06A_DEFAULT_IO_MODE);
    modes_[5] = (io6 == "input") ? TriggerMode::MODE_INPUT : TriggerMode::MODE_DIGITAL;
    pulseMsArr_[5] = settings_->getInt(Is06aKeys::kTrg6PulseMs, IS06A_DEFAULT_PULSE_MS);
    debounceMsArr_[5] = settings_->getInt(Is06aKeys::kTrg6Debounce, IS06A_DEFAULT_DEBOUNCE_MS);
}

void TriggerManagerIs06a::sample() {
    // TRG5-6がINPUTモードの場合のみサンプリング
    for (int i = kNumPwmCapable; i < kNumTriggers; i++) {
        if (modes_[i] == TriggerMode::MODE_INPUT) {
            io_[i].sample();
        }
    }
    handleInputEdges();
}

void TriggerManagerIs06a::update() {
    // 全トリガーのパルス状態更新
    for (int i = 0; i < kNumTriggers; i++) {
        if (modes_[i] != TriggerMode::MODE_INPUT) {
            io_[i].update();
        }
    }
}

void TriggerManagerIs06a::handleInputEdges() {
    unsigned long now = millis();

    // TRG5-6のエッジ検出（クールダウン付き）
    for (int i = 0; i < kNumIoSwitchable; i++) {
        int idx = kNumPwmCapable + i;  // TRG4=idx3, TRG5=idx4
        int trgNum = idx + 1;

        if (modes_[idx] != TriggerMode::MODE_INPUT) continue;

        bool current = io_[idx].isActive();
        if (current && !lastInputStable_[i]) {
            // 立ち上がりエッジ - クールダウンチェック
            if ((now - lastInputEdgeMs_[i]) < INPUT_COOLDOWN_MS) {
                // クールダウン中はスキップ（チャタリング連打防止）
                continue;
            }
            lastInputEdgeMs_[i] = now;

            Serial.printf("[TriggerManagerIs06a] TRG%d rising edge detected\n", trgNum);
            lastUpdatedAt_[idx] = getTimestamp();
            changed_ = true;

            if (onInputChangeCallback_) {
                onInputChangeCallback_(trgNum, true);
            }
        } else if (!current && lastInputStable_[i]) {
            // 立ち下がりエッジ - クールダウンチェック
            if ((now - lastInputEdgeMs_[i]) < INPUT_COOLDOWN_MS) {
                continue;
            }
            lastInputEdgeMs_[i] = now;

            lastUpdatedAt_[idx] = getTimestamp();
            changed_ = true;

            if (onInputChangeCallback_) {
                onInputChangeCallback_(trgNum, false);
            }
        }
        lastInputStable_[i] = current;
    }
}

bool TriggerManagerIs06a::startPulse(int trgNum, int durationMs, PulseSource source) {
    if (trgNum < 1 || trgNum > kNumTriggers) return false;
    int idx = trgNum - 1;

    // INPUTモードではパルス不可
    if (modes_[idx] == TriggerMode::MODE_INPUT) {
        Serial.printf("[TriggerManagerIs06a] TRG%d is INPUT mode, cannot pulse\n", trgNum);
        return false;
    }

    // PWMモードではパルス不可（PWM制御を使う）
    if (modes_[idx] == TriggerMode::MODE_PWM) {
        Serial.printf("[TriggerManagerIs06a] TRG%d is PWM mode, use setPwmDuty instead\n", trgNum);
        return false;
    }

    // パルス実行中チェック
    if (isAnyPulseActive()) {
        Serial.println("[TriggerManagerIs06a] Another pulse is active");
        return false;
    }

    // インターロックチェック
    unsigned long now = millis();
    if (lastPulseEndMs_ > 0 && (now - lastPulseEndMs_) < (unsigned long)interlockMs_) {
        Serial.printf("[TriggerManagerIs06a] Interlock active (%lums remaining)\n",
            interlockMs_ - (now - lastPulseEndMs_));
        return false;
    }

    io_[idx].pulse(durationMs);
    currentPulseTrg_ = trgNum;
    currentSource_ = source;
    changed_ = true;

    Serial.printf("[TriggerManagerIs06a] Pulse started: TRG%d, %dms, source=%d\n",
        trgNum, durationMs, (int)source);

    if (onPulseStartCallback_) {
        onPulseStartCallback_(trgNum, source);
    }

    return true;
}

bool TriggerManagerIs06a::setPwmDuty(int trgNum, int duty) {
    if (!isPwmCapable(trgNum)) return false;
    int idx = trgNum - 1;

    if (modes_[idx] != TriggerMode::MODE_PWM) {
        Serial.printf("[TriggerManagerIs06a] TRG%d is not in PWM mode\n", trgNum);
        return false;
    }

    if (ledcChannels_[idx] < 0) {
        Serial.printf("[TriggerManagerIs06a] TRG%d PWM channel not initialized\n", trgNum);
        return false;
    }

    if (duty < 0) duty = 0;
    if (duty > 255) duty = 255;

    pwmDutyArr_[idx] = duty;
    // ESP32 Core 3.x: ledcWriteChannel(channel, duty) - チャンネル指定版
    ledcWriteChannel(ledcChannels_[idx], duty);
    pwmActive_[idx] = (duty > 0);
    changed_ = true;

    Serial.printf("[TriggerManagerIs06a] PWM duty set: TRG%d = %d (ch=%d)\n", trgNum, duty, ledcChannels_[idx]);

    if (onPwmChangeCallback_) {
        onPwmChangeCallback_(trgNum, duty);
    }

    return true;
}

bool TriggerManagerIs06a::setPwmDutyPercent(int trgNum, float percent) {
    if (percent < 0.0f) percent = 0.0f;
    if (percent > 100.0f) percent = 100.0f;
    int duty = (int)(percent * 255.0f / 100.0f);
    return setPwmDuty(trgNum, duty);
}

bool TriggerManagerIs06a::stopPwm(int trgNum) {
    return setPwmDuty(trgNum, 0);
}

bool TriggerManagerIs06a::isPulseActive(int trgNum) const {
    if (trgNum < 1 || trgNum > kNumTriggers) return false;
    int idx = trgNum - 1;
    if (modes_[idx] == TriggerMode::MODE_INPUT) return false;
    return io_[idx].isPulseActive();
}

bool TriggerManagerIs06a::isAnyPulseActive() const {
    for (int i = 0; i < kNumTriggers; i++) {
        if (modes_[i] != TriggerMode::MODE_INPUT && io_[i].isPulseActive()) {
            return true;
        }
    }
    return false;
}

bool TriggerManagerIs06a::isInputActive(int trgNum) const {
    if (trgNum < 1 || trgNum > kNumTriggers) return false;
    int idx = trgNum - 1;
    if (modes_[idx] != TriggerMode::MODE_INPUT) return false;
    return io_[idx].isActive();
}

TriggerManagerIs06a::TriggerState TriggerManagerIs06a::getState(int trgNum) const {
    TriggerState state = {};
    if (trgNum < 1 || trgNum > kNumTriggers) return state;

    int idx = trgNum - 1;
    state.trgNum = trgNum;
    state.pin = kGpioPins[idx];
    state.mode = modes_[idx];
    state.name = names_[idx];
    state.pulseMs = pulseMsArr_[idx];
    state.pwmFreq = pwmFreqArr_[idx];
    state.pwmDuty = pwmDutyArr_[idx];
    state.debounceMs = debounceMsArr_[idx];
    state.lastUpdatedAt = lastUpdatedAt_[idx];

    if (modes_[idx] == TriggerMode::MODE_INPUT) {
        state.active = io_[idx].isActive();
    } else if (modes_[idx] == TriggerMode::MODE_PWM) {
        state.active = pwmActive_[idx];
    } else {
        state.active = io_[idx].isPulseActive();
    }

    if (state.active && currentPulseTrg_ == trgNum) {
        state.source = currentSource_;
    } else {
        state.source = PulseSource::NONE;
    }

    return state;
}

bool TriggerManagerIs06a::setMode(int trgNum, TriggerMode mode) {
    if (trgNum < 1 || trgNum > kNumTriggers) return false;
    int idx = trgNum - 1;

    // TRG1-4はPWMかDIGITALのみ（INPUT不可）
    if (trgNum <= kNumPwmCapable && mode == TriggerMode::MODE_INPUT) {
        Serial.printf("[TriggerManagerIs06a] TRG%d cannot be INPUT mode\n", trgNum);
        return false;
    }

    // TRG5-6はINPUTかDIGITALのみ（PWM不可）
    if (trgNum > kNumPwmCapable && mode == TriggerMode::MODE_PWM) {
        Serial.printf("[TriggerManagerIs06a] TRG%d cannot be PWM mode\n", trgNum);
        return false;
    }

    // 現在のモードを保存（失敗時のロールバック用）
    TriggerMode oldMode = modes_[idx];

    // 現在のモードから切り替え
    if (modes_[idx] == TriggerMode::MODE_PWM && mode != TriggerMode::MODE_PWM) {
        teardownPwm(trgNum);
    }

    modes_[idx] = mode;

    if (mode == TriggerMode::MODE_PWM) {
        setupPwm(trgNum);
    } else if (mode == TriggerMode::MODE_INPUT) {
        bool success = io_[idx].setMode(IOController::Mode::IO_IN);
        if (!success) {
            // IOController::isSafeToSwitch()で禁止されている場合
            Serial.printf("[TriggerManagerIs06a] TRG%d setMode(IO_IN) failed\n", trgNum);
            modes_[idx] = oldMode;
            return false;
        }
        io_[idx].setDebounceMs(debounceMsArr_[idx]);
        io_[idx].setPullup(false);
        io_[idx].setInverted(true);
    } else {
        // DIGITALモードへ切替
        bool success = io_[idx].setMode(IOController::Mode::IO_OUT);
        if (!success) {
            // TRG4(GPIO15)はIOController::isSafeToSwitch()で禁止されているため、
            // 直接GPIO制御でフォールバック。
            // 注意: IOControllerの内部状態は更新されないが、TRG4はbegin()時点で
            // IOController::begin()をスキップしており、直接GPIO制御を前提としている。
            // パルス/PWM制御もLEDCまたは直接GPIO経由で行うため問題なし。
            if (trgNum == 4) {
                pinMode(IS06A_GPIO_TRG4, OUTPUT);
                digitalWrite(IS06A_GPIO_TRG4, HIGH);  // アイドル状態HIGH（ストラップピン安全）
                Serial.println("[TriggerManagerIs06a] TRG4(GPIO15) fallback to direct GPIO (idle=HIGH)");
            } else {
                Serial.printf("[TriggerManagerIs06a] TRG%d setMode(IO_OUT) failed\n", trgNum);
                modes_[idx] = oldMode;
                return false;
            }
        } else if (trgNum == 4) {
            // TRG4(GPIO15)がDIGITALモードに成功した場合もHIGHに設定
            digitalWrite(IS06A_GPIO_TRG4, HIGH);
        }
    }

    // NVS保存
    if (trgNum <= kNumPwmCapable) {
        // TRG1-4: PWM対応（digital/pwm）
        const char* modeKey = nullptr;
        switch (trgNum) {
            case 1: modeKey = Is06aKeys::kTrg1Mode; break;
            case 2: modeKey = Is06aKeys::kTrg2Mode; break;
            case 3: modeKey = Is06aKeys::kTrg3Mode; break;
            case 4: modeKey = Is06aKeys::kTrg4Mode; break;
        }
        if (modeKey) {
            settings_->setString(modeKey, (mode == TriggerMode::MODE_PWM) ? "pwm" : "digital");
        }
    } else {
        // TRG5-6: I/O切替（input/output）
        const char* ioKey = (trgNum == 5) ? Is06aKeys::kTrg5IoMode : Is06aKeys::kTrg6IoMode;
        settings_->setString(ioKey, (mode == TriggerMode::MODE_INPUT) ? "input" : "output");
    }

    changed_ = true;
    return true;
}

TriggerManagerIs06a::TriggerMode TriggerManagerIs06a::getMode(int trgNum) const {
    if (trgNum < 1 || trgNum > kNumTriggers) return TriggerMode::MODE_DIGITAL;
    return modes_[trgNum - 1];
}

void TriggerManagerIs06a::setName(int trgNum, const String& name) {
    if (trgNum < 1 || trgNum > kNumTriggers) return;
    int idx = trgNum - 1;
    names_[idx] = name;

    const char* key = nullptr;
    switch (trgNum) {
        case 1: key = Is06aKeys::kTrg1Name; break;
        case 2: key = Is06aKeys::kTrg2Name; break;
        case 3: key = Is06aKeys::kTrg3Name; break;
        case 4: key = Is06aKeys::kTrg4Name; break;
        case 5: key = Is06aKeys::kTrg5Name; break;
        case 6: key = Is06aKeys::kTrg6Name; break;
    }
    if (key) {
        settings_->setString(key, name);
    }
}

String TriggerManagerIs06a::getName(int trgNum) const {
    if (trgNum < 1 || trgNum > kNumTriggers) return "";
    return names_[trgNum - 1];
}

void TriggerManagerIs06a::setPulseMs(int trgNum, int ms) {
    if (trgNum < 1 || trgNum > kNumTriggers) return;
    if (ms < 10) ms = 10;
    if (ms > 10000) ms = 10000;

    int idx = trgNum - 1;
    pulseMsArr_[idx] = ms;

    const char* key = nullptr;
    switch (trgNum) {
        case 1: key = Is06aKeys::kTrg1PulseMs; break;
        case 2: key = Is06aKeys::kTrg2PulseMs; break;
        case 3: key = Is06aKeys::kTrg3PulseMs; break;
        case 4: key = Is06aKeys::kTrg4PulseMs; break;
        case 5: key = Is06aKeys::kTrg5PulseMs; break;
        case 6: key = Is06aKeys::kTrg6PulseMs; break;
    }
    if (key) {
        settings_->setInt(key, ms);
    }
}

int TriggerManagerIs06a::getPulseMs(int trgNum) const {
    if (trgNum < 1 || trgNum > kNumTriggers) return 3000;
    return pulseMsArr_[trgNum - 1];
}

void TriggerManagerIs06a::setPwmFreq(int trgNum, int hz) {
    if (!isPwmCapable(trgNum)) return;
    if (hz < 1000) hz = 1000;
    if (hz > 40000) hz = 40000;

    int idx = trgNum - 1;
    pwmFreqArr_[idx] = hz;

    const char* key = nullptr;
    switch (trgNum) {
        case 1: key = Is06aKeys::kTrg1PwmFreq; break;
        case 2: key = Is06aKeys::kTrg2PwmFreq; break;
        case 3: key = Is06aKeys::kTrg3PwmFreq; break;
        case 4: key = Is06aKeys::kTrg4PwmFreq; break;
    }
    if (key) {
        settings_->setInt(key, hz);
    }

    // アクティブなPWMは周波数を更新（チャンネル指定版）
    if (modes_[idx] == TriggerMode::MODE_PWM && ledcChannels_[idx] >= 0) {
        uint32_t actualFreq = ledcChangeFrequency(ledcChannels_[idx], hz, 8);
        if (actualFreq == 0) {
            Serial.printf("[TriggerManagerIs06a] PWM freq change FAILED: TRG%d\n", trgNum);
        } else {
            Serial.printf("[TriggerManagerIs06a] PWM freq changed: TRG%d = %dHz (actual: %luHz)\n",
                trgNum, hz, actualFreq);
        }
    }
}

int TriggerManagerIs06a::getPwmFreq(int trgNum) const {
    if (!isPwmCapable(trgNum)) return 0;
    return pwmFreqArr_[trgNum - 1];
}

void TriggerManagerIs06a::setDebounceMs(int trgNum, int ms) {
    if (!isIoSwitchable(trgNum)) return;
    if (ms < 5) ms = 5;
    if (ms > 1000) ms = 1000;

    int idx = trgNum - 1;
    debounceMsArr_[idx] = ms;

    const char* key = (trgNum == 5) ? Is06aKeys::kTrg5Debounce : Is06aKeys::kTrg6Debounce;
    settings_->setInt(key, ms);

    if (modes_[idx] == TriggerMode::MODE_INPUT) {
        io_[idx].setDebounceMs(ms);
    }
}

int TriggerManagerIs06a::getDebounceMs(int trgNum) const {
    if (!isIoSwitchable(trgNum)) return 0;
    return debounceMsArr_[trgNum - 1];
}

void TriggerManagerIs06a::setInterlockMs(int ms) {
    if (ms < 0) ms = 0;
    if (ms > 5000) ms = 5000;
    interlockMs_ = ms;
    settings_->setInt(Is06aKeys::kInterlockMs, ms);
}

void TriggerManagerIs06a::onPulseStart(std::function<void(int, PulseSource)> callback) {
    onPulseStartCallback_ = callback;
}

void TriggerManagerIs06a::onPulseEnd(std::function<void(int)> callback) {
    onPulseEndCallback_ = callback;
}

void TriggerManagerIs06a::onInputChange(std::function<void(int, bool)> callback) {
    onInputChangeCallback_ = callback;
}

void TriggerManagerIs06a::onPwmChange(std::function<void(int, int)> callback) {
    onPwmChangeCallback_ = callback;
}

String TriggerManagerIs06a::getTimestamp() const {
    if (ntp_ && ntp_->isSynced()) {
        return ntp_->getIso8601();
    }
    return "1970-01-01T00:00:00Z";
}

void TriggerManagerIs06a::setupPwm(int trgNum) {
    if (!isPwmCapable(trgNum)) return;
    int idx = trgNum - 1;

    // ESP32 Core 3.x: ledcAttach(pin, freq, resolution) -> チャンネルを返す
    int ch = ledcAttach(kGpioPins[idx], pwmFreqArr_[idx], 8);  // 8bit resolution
    if (ch < 0) {
        Serial.printf("[TriggerManagerIs06a] PWM setup FAILED: TRG%d, GPIO%d - falling back to DIGITAL\n",
            trgNum, kGpioPins[idx]);
        // フォールバック: デジタルモードに戻す
        modes_[idx] = TriggerMode::MODE_DIGITAL;
        io_[idx].setMode(IOController::Mode::IO_OUT);
        ledcChannels_[idx] = -1;
        return;
    }
    ledcChannels_[idx] = ch;  // チャンネルを保存

    // NVSから読み込んだDutyを適用（起動時復元）
    int savedDuty = pwmDutyArr_[idx];
    ledcWriteChannel(ch, savedDuty);
    pwmActive_[idx] = (savedDuty > 0);

    Serial.printf("[TriggerManagerIs06a] PWM setup: TRG%d, GPIO%d, ch=%d, %dHz, duty=%d\n",
        trgNum, kGpioPins[idx], ch, pwmFreqArr_[idx], savedDuty);
}

void TriggerManagerIs06a::teardownPwm(int trgNum) {
    if (!isPwmCapable(trgNum)) return;
    int idx = trgNum - 1;

    // ESP32 Core 3.x: ledcDetach(pin)
    ledcDetach(kGpioPins[idx]);
    pwmActive_[idx] = false;
    ledcChannels_[idx] = -1;  // チャンネルをリセット

    Serial.printf("[TriggerManagerIs06a] PWM teardown: TRG%d\n", trgNum);
}

int TriggerManagerIs06a::getLedcChannel(int trgNum) const {
    if (!isPwmCapable(trgNum)) return -1;
    return ledcChannels_[trgNum - 1];
}

String TriggerManagerIs06a::toJson() const {
    StaticJsonDocument<1536> doc;  // StaticJsonDocument化（ヒープ断片化防止）

    JsonArray triggers = doc.createNestedArray("triggers");
    for (int i = 1; i <= kNumTriggers; i++) {
        TriggerState state = getState(i);
        JsonObject trg = triggers.createNestedObject();
        trg["num"] = state.trgNum;
        trg["pin"] = state.pin;
        trg["name"] = state.name;
        trg["active"] = state.active;
        trg["lastUpdatedAt"] = state.lastUpdatedAt;

        const char* modeStr = (state.mode == TriggerMode::MODE_PWM) ? "pwm" :
                              (state.mode == TriggerMode::MODE_INPUT) ? "input" : "digital";
        trg["mode"] = modeStr;

        if (state.mode == TriggerMode::MODE_PWM) {
            trg["pwmFreq"] = state.pwmFreq;
            trg["pwmDuty"] = state.pwmDuty;
        } else if (state.mode == TriggerMode::MODE_INPUT) {
            trg["debounceMs"] = state.debounceMs;
        } else {
            trg["pulseMs"] = state.pulseMs;
        }
    }

    doc["interlockMs"] = interlockMs_;

    String json;
    serializeJson(doc, json);
    return json;
}
