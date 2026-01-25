/**
 * Is06PinManager.cpp
 *
 * IS06S PIN制御マネージャー実装
 */

#include "Is06PinManager.h"
#include <time.h>
#include <driver/gpio.h>  // ESP-IDF GPIO API
#include <rom/gpio.h>     // gpio_matrix_out, gpio_pad_select_gpio
#include <soc/io_mux_reg.h>  // IO_MUX registers

// ============================================================
// 初期化
// ============================================================
void Is06PinManager::begin(SettingManager* settings, NtpManager* ntp) {
  settings_ = settings;
  ntp_ = ntp;

  Serial.println("Is06PinManager: Initializing...");

  // GPIO初期化
  initGpio();

  // LEDC初期化（PWM用）
  initLedc();

  // NVSから設定読み込み
  loadFromNvs();

  // 【重要】NVSから読み込んだ設定をハードウェアに適用
  // これがないと、NVSにPWM_OUTPUTが保存されていてもLEDCがアタッチされない
  applyLoadedPinTypes();

  Serial.println("Is06PinManager: Initialization complete.");
  printStatus();
}

void Is06PinManager::initGpio() {
  using namespace AraneaSettingsDefaults;

  // CH1-4 (D/P Type): LEDCで管理するためpinModeは呼ばない
  // （pinModeとledcAttachの競合を避けるため）

  // CH5-6 (I/O Type): 初期状態はINPUT_PULLDOWN
  for (int i = IS06_DP_CHANNELS; i < IS06_CHANNEL_COUNT; i++) {
    pinMode(IS06_PIN_MAP[i], INPUT_PULLDOWN);
  }

  Serial.println("Is06PinManager: GPIO initialized (CH5-6 only, CH1-4 use LEDC).");
}

void Is06PinManager::initLedc() {
  using namespace AraneaSettingsDefaults;

  Serial.println("Is06PinManager: Initializing GPIO with explicit LEDC detach...");

  for (int i = 0; i < IS06_DP_CHANNELS; i++) {
    int pin = IS06_PIN_MAP[i];

    // 1. LEDCから明示的にデタッチ（以前のアタッチが残っている場合に対応）
    ledcDetach(pin);

    // 2. LEDCチャンネル管理をクリア
    ledcChannel_[i] = -1;

    // 3. IO_MUXでGPIO functionを選択
    gpio_pad_select_gpio(pin);

    // 4. GPIO Matrixで出力をシンプルGPIOに設定
    gpio_matrix_out(pin, 256, false, false);

    // 5. gpio_config()で完全な設定
    gpio_config_t io_conf = {};
    io_conf.intr_type = GPIO_INTR_DISABLE;
    io_conf.mode = GPIO_MODE_INPUT_OUTPUT;  // 入出力両方を有効化
    io_conf.pin_bit_mask = (1ULL << pin);
    io_conf.pull_down_en = GPIO_PULLDOWN_DISABLE;
    io_conf.pull_up_en = GPIO_PULLUP_DISABLE;
    gpio_config(&io_conf);

    // 6. ドライブ強度を最大に
    gpio_set_drive_capability((gpio_num_t)pin, GPIO_DRIVE_CAP_3);
    gpio_set_level((gpio_num_t)pin, 0);

    // 7. 確認読み取り
    int readBack = gpio_get_level((gpio_num_t)pin);
    Serial.printf("  CH%d (GPIO%d): full config, read=%d\n", i + 1, pin, readBack);
  }

  Serial.println("Is06PinManager: GPIO initialized (full config, all LOW, drive=MAX).");
}

// ============================================================
// NVSから読み込んだ設定をハードウェアに適用
// ============================================================
void Is06PinManager::applyLoadedPinTypes() {
  using namespace AraneaSettingsDefaults;

  Serial.println("Is06PinManager: Applying loaded pin types to hardware...");

  for (int ch = 1; ch <= IS06_CHANNEL_COUNT; ch++) {
    int idx = ch - 1;
    ::PinType type = pinSettings_[idx].type;
    int pin = IS06_PIN_MAP[idx];

    if (type == ::PinType::PWM_OUTPUT && idx < IS06_DP_CHANNELS) {
      // PWMモード: LEDCアタッチ
      ledcDetach(pin);
      int ledcCh = ledcAttach(pin, PWM_FREQUENCY, PWM_RESOLUTION);
      if (ledcCh >= 0) {
        ledcChannel_[idx] = ledcCh;
        ledcWriteChannel(ledcCh, 0);
        Serial.printf("  CH%d GPIO%d: LEDC attached (ch=%d)\n", ch, pin, ledcCh);
      } else {
        ledcChannel_[idx] = -1;
        // フォールバック: GPIO出力 (full config)
        gpio_pad_select_gpio(pin);
        gpio_matrix_out(pin, 256, false, false);
        gpio_config_t io_conf = {};
        io_conf.intr_type = GPIO_INTR_DISABLE;
        io_conf.mode = GPIO_MODE_INPUT_OUTPUT;
        io_conf.pin_bit_mask = (1ULL << pin);
        io_conf.pull_down_en = GPIO_PULLDOWN_DISABLE;
        io_conf.pull_up_en = GPIO_PULLUP_DISABLE;
        gpio_config(&io_conf);
        gpio_set_drive_capability((gpio_num_t)pin, GPIO_DRIVE_CAP_3);
        gpio_set_level((gpio_num_t)pin, 0);
        Serial.printf("  CH%d GPIO%d: LEDC attach failed, using GPIO (full config)\n", ch, pin);
      }
    } else if (type == ::PinType::DIGITAL_OUTPUT) {
      // Digital出力モード - IO_MUXとGPIO Matrixを完全に設定
      ledcDetach(pin);
      if (idx < IS06_DP_CHANNELS) ledcChannel_[idx] = -1;

      gpio_pad_select_gpio(pin);
      gpio_matrix_out(pin, 256, false, false);

      gpio_config_t io_conf = {};
      io_conf.intr_type = GPIO_INTR_DISABLE;
      io_conf.mode = GPIO_MODE_INPUT_OUTPUT;
      io_conf.pin_bit_mask = (1ULL << pin);
      io_conf.pull_down_en = GPIO_PULLDOWN_DISABLE;
      io_conf.pull_up_en = GPIO_PULLUP_DISABLE;
      gpio_config(&io_conf);

      gpio_set_drive_capability((gpio_num_t)pin, GPIO_DRIVE_CAP_3);
      gpio_set_level((gpio_num_t)pin, 0);
      Serial.printf("  CH%d GPIO%d: Digital OUTPUT (full config)\n", ch, pin);
    } else if (type == ::PinType::DIGITAL_INPUT) {
      // Digital入力モード - IO_MUXを正しく設定
      ledcDetach(pin);
      if (idx < IS06_DP_CHANNELS) ledcChannel_[idx] = -1;

      gpio_pad_select_gpio(pin);

      gpio_config_t io_conf = {};
      io_conf.intr_type = GPIO_INTR_DISABLE;
      io_conf.mode = GPIO_MODE_INPUT;
      io_conf.pin_bit_mask = (1ULL << pin);
      io_conf.pull_down_en = GPIO_PULLDOWN_ENABLE;
      io_conf.pull_up_en = GPIO_PULLUP_DISABLE;
      gpio_config(&io_conf);
      Serial.printf("  CH%d GPIO%d: Digital INPUT\n", ch, pin);
    } else if (type == ::PinType::PIN_DISABLED) {
      // 無効モード
      ledcDetach(pin);
      if (idx < IS06_DP_CHANNELS) ledcChannel_[idx] = -1;
      gpio_reset_pin((gpio_num_t)pin);
      gpio_set_direction((gpio_num_t)pin, GPIO_MODE_INPUT);
      Serial.printf("  CH%d GPIO%d: DISABLED (Hi-Z)\n", ch, pin);
    }
  }

  Serial.println("Is06PinManager: Pin types applied to hardware.");
}

// ============================================================
// 毎ループ更新
// ============================================================
void Is06PinManager::update() {
  handleMomentaryPulse();
  handlePwmTransition();
  handleDigitalInput();
}

void Is06PinManager::handleMomentaryPulse() {
  unsigned long now = millis();

  for (int ch = 0; ch < IS06_CHANNEL_COUNT; ch++) {
    // モーメンタリパルス終了チェック
    if (pinStates_[ch].pulseEndTime > 0 && now >= pinStates_[ch].pulseEndTime) {
      // パルス終了 → OFF
      pinStates_[ch].pulseEndTime = 0;
      pinStates_[ch].digitalState = 0;
      applyDigitalOutput(ch + 1, 0);

      Serial.printf("Is06PinManager: CH%d momentary pulse ended.\n", ch + 1);

      if (stateCallback_) {
        stateCallback_(ch + 1, 0, 0);
      }
    }
  }
}

void Is06PinManager::handlePwmTransition() {
  unsigned long now = millis();

  for (int ch = 0; ch < IS06_DP_CHANNELS; ch++) {
    if (pinSettings_[ch].type != PinType::PWM_OUTPUT) continue;
    if (pinStates_[ch].pwmValue == pwmTargetValue_[ch]) continue;
    if (pwmTransitionStart_[ch] == 0) continue;

    // 遷移計算
    int rateOfChange = getEffectiveRateOfChange(ch + 1);
    unsigned long elapsed = now - pwmTransitionStart_[ch];

    if (pinSettings_[ch].actionMode == ActionMode::RAPID || rateOfChange == 0) {
      // Rapid: 即座に適用
      pinStates_[ch].pwmValue = pwmTargetValue_[ch];
      pwmTransitionStart_[ch] = 0;
    } else {
      // Slow: なだらか遷移
      float progress = (float)elapsed / (float)rateOfChange;
      if (progress >= 1.0f) {
        pinStates_[ch].pwmValue = pwmTargetValue_[ch];
        pwmTransitionStart_[ch] = 0;
      } else {
        int delta = pwmTargetValue_[ch] - pwmStartValue_[ch];
        pinStates_[ch].pwmValue = pwmStartValue_[ch] + (int)(delta * progress);
      }
    }

    applyPwmOutput(ch + 1, pinStates_[ch].pwmValue);
  }
}

void Is06PinManager::handleDigitalInput() {
  unsigned long now = millis();
  const unsigned long INPUT_STABLE_MS = 200;  // 入力安定判定時間

  for (int ch = IS06_DP_CHANNELS; ch < IS06_CHANNEL_COUNT; ch++) {
    if (pinSettings_[ch].type != PinType::DIGITAL_INPUT) continue;
    if (!pinStates_[ch].enabled) continue;

    // デバウンス中はスキップ
    if (now < pinStates_[ch].debounceEnd) continue;

    bool currentInput = digitalRead(IS06_PIN_MAP[ch]) == HIGH;

    if (currentInput) {
      // HIGH入力中
      if (pinStates_[ch].inputHighStart == 0) {
        // HIGH開始時刻を記録
        pinStates_[ch].inputHighStart = now;
      } else if (now - pinStates_[ch].inputHighStart >= INPUT_STABLE_MS) {
        // 安定時間経過 → 有効な入力として処理
        if (!pinStates_[ch].lastInputState) {
          // LOW→HIGH遷移確定
          pinStates_[ch].lastInputState = true;
          pinStates_[ch].inputState = true;
          pinStates_[ch].debounceEnd = now + (unsigned long)getEffectiveDebounce(ch + 1);

          Serial.printf("Is06PinManager: CH%d input HIGH (stable %lums)\n", ch + 1, INPUT_STABLE_MS);

          if (inputCallback_) {
            inputCallback_(ch + 1, true);
          }

          triggerAllocations(ch);
        }
      }
    } else {
      // LOW入力
      pinStates_[ch].inputHighStart = 0;  // HIGHカウントリセット

      if (pinStates_[ch].lastInputState) {
        // HIGH→LOW遷移
        pinStates_[ch].lastInputState = false;
        pinStates_[ch].inputState = false;
        pinStates_[ch].debounceEnd = now + (unsigned long)getEffectiveDebounce(ch + 1);

        Serial.printf("Is06PinManager: CH%d input LOW\n", ch + 1);

        if (inputCallback_) {
          inputCallback_(ch + 1, false);
        }
      }
    }
  }
}

void Is06PinManager::triggerAllocations(int inputChannel) {
  const PinSetting& inputSetting = pinSettings_[inputChannel];

  for (int i = 0; i < inputSetting.allocationCount; i++) {
    String target = inputSetting.allocation[i];
    if (target.isEmpty()) continue;

    // "CH1" → 1
    if (target.startsWith("CH") || target.startsWith("ch")) {
      int targetCh = target.substring(2).toInt();
      if (!isValidChannel(targetCh)) continue;

      int idx = targetCh - 1;
      const PinSetting& targetSetting = pinSettings_[idx];

      if (targetSetting.type == PinType::DIGITAL_OUTPUT) {
        // Digital Output: トグルまたはモーメンタリ
        if (targetSetting.actionMode == ActionMode::ALTERNATE) {
          int newState = pinStates_[idx].digitalState == 0 ? 1 : 0;
          setPinState(targetCh, newState);
        } else {
          // モーメンタリ
          setPinState(targetCh, 1);
        }
      } else if (targetSetting.type == PinType::PWM_OUTPUT) {
        // PWM Output: ローテート
        int nextIndex = (pinStates_[idx].currentPresetIndex + 1) % targetSetting.presetCount;
        pinStates_[idx].currentPresetIndex = nextIndex;
        int nextValue = targetSetting.pwmPresets[nextIndex];
        setPwmValue(targetCh, nextValue);
      }

      Serial.printf("Is06PinManager: Trigger CH%d → CH%d\n", inputChannel + 1, targetCh);
    }
  }
}

// ============================================================
// PIN制御
// ============================================================
bool Is06PinManager::setPinState(int channel, int state) {
  if (!isValidChannel(channel)) return false;

  int idx = channel - 1;

  // enabled チェック (P1-1a)
  if (!pinStates_[idx].enabled) {
    Serial.printf("Is06PinManager: CH%d is disabled. Command rejected.\n", channel);
    return false;
  }

  // expiryDate チェック (P3-5)
  if (isExpired(channel)) {
    Serial.printf("Is06PinManager: CH%d has expired. Command rejected.\n", channel);
    return false;
  }

  const PinSetting& setting = pinSettings_[idx];
  unsigned long now = millis();

  // デバウンスチェック
  if (now < pinStates_[idx].debounceEnd) {
    Serial.printf("Is06PinManager: CH%d debounce active. Command rejected.\n", channel);
    return false;
  }

  if (setting.type == PinType::PWM_OUTPUT) {
    // PWM: state = 0-100%
    return setPwmValue(channel, state);
  }

  if (setting.type == PinType::DIGITAL_OUTPUT) {
    // Digital Output
    int newState = (state != 0) ? 1 : 0;

    if (setting.actionMode == ActionMode::MOMENTARY) {
      // モーメンタリ: パルス開始
      if (newState == 1) {
        pinStates_[idx].digitalState = 1;
        pinStates_[idx].pulseEndTime = now + (unsigned long)getEffectiveValidity(channel);
        pinStates_[idx].debounceEnd = now + (unsigned long)getEffectiveDebounce(channel);
        applyDigitalOutput(channel, 1);
        Serial.printf("Is06PinManager: CH%d momentary ON for %dms\n", channel, getEffectiveValidity(channel));
      }
    } else {
      // オルタネート: トグル
      pinStates_[idx].digitalState = newState;
      // Must Fix #2: Altモードでもdebounceを適用（チャタリング/連打抑止）
      pinStates_[idx].debounceEnd = now + (unsigned long)getEffectiveDebounce(channel);
      applyDigitalOutput(channel, newState);
      Serial.printf("Is06PinManager: CH%d alternate set to %d\n", channel, newState);
    }

    pinStates_[idx].lastAction = now;

    if (stateCallback_) {
      stateCallback_(channel, pinStates_[idx].digitalState, 0);
    }

    return true;
  }

  return false;
}

int Is06PinManager::getPinState(int channel) {
  if (!isValidChannel(channel)) return -1;
  return pinStates_[channel - 1].digitalState;
}

bool Is06PinManager::setPwmValue(int channel, int value) {
  if (!isValidDpChannel(channel)) return false;

  int idx = channel - 1;

  if (!pinStates_[idx].enabled) {
    Serial.printf("Is06PinManager: CH%d is disabled. PWM command rejected.\n", channel);
    return false;
  }

  if (pinSettings_[idx].type != PinType::PWM_OUTPUT) {
    Serial.printf("Is06PinManager: CH%d is not PWM type.\n", channel);
    return false;
  }

  // expiryDate チェック (P3-5)
  if (isExpired(channel)) {
    Serial.printf("Is06PinManager: CH%d has expired. PWM command rejected.\n", channel);
    return false;
  }

  // 値を0-100にクランプ
  value = constrain(value, 0, 100);

  // 遷移開始
  pwmStartValue_[idx] = pinStates_[idx].pwmValue;
  pwmTargetValue_[idx] = value;
  pwmTransitionStart_[idx] = millis();

  Serial.printf("Is06PinManager: CH%d PWM %d → %d\n", channel, pwmStartValue_[idx], value);

  if (stateCallback_) {
    stateCallback_(channel, 0, value);
  }

  return true;
}

int Is06PinManager::getPwmValue(int channel) {
  if (!isValidDpChannel(channel)) return -1;
  return pinStates_[channel - 1].pwmValue;
}

// ============================================================
// PIN有効/無効制御 (P1-1a)
// ============================================================
bool Is06PinManager::isPinEnabled(int channel) {
  if (!isValidChannel(channel)) return false;
  return pinStates_[channel - 1].enabled;
}

void Is06PinManager::setPinEnabled(int channel, bool enabled) {
  if (!isValidChannel(channel)) return;

  int idx = channel - 1;
  pinStates_[idx].enabled = enabled;

  // NVSに保存
  String key = getNvsKey(channel, "_en");
  settings_->setString(key.c_str(), enabled ? "true" : "false");

  Serial.printf("Is06PinManager: CH%d enabled=%s\n", channel, enabled ? "true" : "false");

  // 無効化時はOFF
  if (!enabled) {
    pinStates_[idx].digitalState = 0;
    pinStates_[idx].pwmValue = 0;
    applyDigitalOutput(channel, 0);
  }
}

// ============================================================
// 設定管理
// ============================================================
const PinSetting& Is06PinManager::getPinSetting(int channel) {
  static PinSetting dummy;
  if (!isValidChannel(channel)) return dummy;
  return pinSettings_[channel - 1];
}

void Is06PinManager::setPinSetting(int channel, const PinSetting& setting) {
  if (!isValidChannel(channel)) return;
  pinSettings_[channel - 1] = setting;
  saveToNvs();
}

void Is06PinManager::setPinType(int channel, ::PinType type) {
  namespace ASD = AraneaSettingsDefaults;

  if (!isValidChannel(channel)) return;

  int idx = channel - 1;
  pinSettings_[idx].type = type;

  // PinType変更時、actionModeを適切なデフォルトに補正（整合バリデーション）
  switch (type) {
    case ::PinType::DIGITAL_OUTPUT:
      // Digital Output: MOMENTARY or ALTERNATE のみ許可
      if (pinSettings_[idx].actionMode != ::ActionMode::MOMENTARY &&
          pinSettings_[idx].actionMode != ::ActionMode::ALTERNATE) {
        pinSettings_[idx].actionMode = ::ActionMode::MOMENTARY;
        Serial.printf("Is06PinManager: CH%d actionMode corrected to MOMENTARY\n", channel);
      }
      break;
    case ::PinType::PWM_OUTPUT:
      // PWM Output: SLOW or RAPID のみ許可
      if (pinSettings_[idx].actionMode != ::ActionMode::SLOW &&
          pinSettings_[idx].actionMode != ::ActionMode::RAPID) {
        pinSettings_[idx].actionMode = ::ActionMode::SLOW;
        Serial.printf("Is06PinManager: CH%d actionMode corrected to SLOW\n", channel);
      }
      break;
    case ::PinType::DIGITAL_INPUT:
      // Digital Input: MOMENTARY, ALTERNATE, ROTATE 許可
      // 特に補正不要（全て有効）
      break;
    default:
      break;
  }

  // GPIOモード再設定 (ESP-IDF API使用)
  int pin = IS06_PIN_MAP[idx];

  if (type == ::PinType::DIGITAL_INPUT) {
    ledcDetach(pin);  // LEDC解放
    if (idx < IS06_DP_CHANNELS) ledcChannel_[idx] = -1;  // チャンネル管理クリア
    gpio_reset_pin((gpio_num_t)pin);
    gpio_set_direction((gpio_num_t)pin, GPIO_MODE_INPUT);
    gpio_set_pull_mode((gpio_num_t)pin, GPIO_PULLDOWN_ONLY);
    Serial.printf("Is06PinManager: CH%d GPIO%d set to INPUT_PULLDOWN\n", channel, pin);
  } else if (type == ::PinType::DIGITAL_OUTPUT) {
    ledcDetach(pin);  // LEDC解放
    if (idx < IS06_DP_CHANNELS) ledcChannel_[idx] = -1;  // チャンネル管理クリア
    gpio_reset_pin((gpio_num_t)pin);
    gpio_set_direction((gpio_num_t)pin, GPIO_MODE_OUTPUT);
    gpio_set_level((gpio_num_t)pin, 0);
    int readBack = gpio_get_level((gpio_num_t)pin);
    Serial.printf("Is06PinManager: CH%d GPIO%d set to OUTPUT LOW (detach), read=%d\n", channel, pin, readBack);
  } else if (type == ::PinType::PWM_OUTPUT) {
    // PWMモード: LEDCアタッチ
    // 【重要】Arduino-ESP32 3.x: ledcAttach()はint(チャンネル番号)を返す
    // ch=0は正常値！boolで受けるとch=0がfalseになりCH1が初期化失敗扱いになる
    ledcDetach(pin);  // 既存のアタッチをクリア
    int ch = ledcAttach(pin, AraneaSettingsDefaults::PWM_FREQUENCY, AraneaSettingsDefaults::PWM_RESOLUTION);
    Serial.printf("Is06PinManager: CH%d GPIO%d ledcAttach returned ch=%d\n", channel, pin, ch);

    if (ch >= 0) {
      // ch >= 0 は成功（ch=0も正常値）
      ledcChannel_[idx] = ch;
      // Arduino-ESP32 3.x: ledcWriteChannel(ch, duty) を使用（ledcWrite(pin,duty)は非推奨）
      ledcWriteChannel(ch, 0);
      Serial.printf("Is06PinManager: CH%d GPIO%d LEDC attached ch=%d (freq=%d, res=%d)\n",
                    channel, pin, ch, AraneaSettingsDefaults::PWM_FREQUENCY, AraneaSettingsDefaults::PWM_RESOLUTION);
    } else {
      // ch < 0 は失敗
      ledcChannel_[idx] = -1;
      // LEDC失敗時はGPIO直接制御にフォールバック
      gpio_reset_pin((gpio_num_t)pin);
      gpio_set_direction((gpio_num_t)pin, GPIO_MODE_OUTPUT);
      gpio_set_level((gpio_num_t)pin, 0);
      Serial.printf("Is06PinManager: CH%d GPIO%d LEDC attach FAILED (ch=%d), fallback to GPIO\n", channel, pin, ch);
    }
  }

  // NVSに保存
  String key = getNvsKey(channel, "_type");
  const char* typeStr = "";
  switch (type) {
    case ::PinType::DIGITAL_OUTPUT: typeStr = ASD::PinType::DIGITAL_OUTPUT; break;
    case ::PinType::PWM_OUTPUT: typeStr = ASD::PinType::PWM_OUTPUT; break;
    case ::PinType::DIGITAL_INPUT: typeStr = ASD::PinType::DIGITAL_INPUT; break;
    case ::PinType::PIN_DISABLED: typeStr = ASD::PinType::PIN_DISABLED; break;
  }
  settings_->setString(key.c_str(), typeStr);

  Serial.printf("Is06PinManager: CH%d type set to %s\n", channel, typeStr);
}

// ============================================================
// PINglobal参照チェーン (P1-1b)
// ============================================================
int Is06PinManager::getEffectiveValidity(int channel) {
  using namespace AraneaSettingsDefaults;
  using namespace AraneaSettingsDefaults::NVSKeys;

  if (!isValidChannel(channel)) return DIGITAL_VALIDITY_MS;

  int idx = channel - 1;

  // 1. PINSettings値
  if (pinSettings_[idx].validity > 0) {
    return pinSettings_[idx].validity;
  }

  // 2. PINglobal値
  String globalValue = settings_->getString(PG_DIGITAL_VALIDITY, "");
  if (!globalValue.isEmpty()) {
    return globalValue.toInt();
  }

  // 3. デフォルト値
  return DIGITAL_VALIDITY_MS;
}

int Is06PinManager::getEffectiveDebounce(int channel) {
  using namespace AraneaSettingsDefaults;
  using namespace AraneaSettingsDefaults::NVSKeys;

  if (!isValidChannel(channel)) return DIGITAL_DEBOUNCE_MS;

  int idx = channel - 1;

  // 1. PINSettings値
  if (pinSettings_[idx].debounce > 0) {
    return pinSettings_[idx].debounce;
  }

  // 2. PINglobal値
  String globalValue = settings_->getString(PG_DIGITAL_DEBOUNCE, "");
  if (!globalValue.isEmpty()) {
    return globalValue.toInt();
  }

  // 3. デフォルト値
  return DIGITAL_DEBOUNCE_MS;
}

int Is06PinManager::getEffectiveRateOfChange(int channel) {
  using namespace AraneaSettingsDefaults;
  using namespace AraneaSettingsDefaults::NVSKeys;

  if (!isValidDpChannel(channel)) return PWM_RATE_OF_CHANGE_MS;

  int idx = channel - 1;

  // 1. PINSettings値
  if (pinSettings_[idx].rateOfChange > 0) {
    return pinSettings_[idx].rateOfChange;
  }

  // 2. PINglobal値
  String globalValue = settings_->getString(PG_PWM_RATE_OF_CHANGE, "");
  if (!globalValue.isEmpty()) {
    return globalValue.toInt();
  }

  // 3. デフォルト値
  return PWM_RATE_OF_CHANGE_MS;
}

// ============================================================
// コールバック
// ============================================================
void Is06PinManager::setStateCallback(PinStateCallback callback) {
  stateCallback_ = callback;
}

void Is06PinManager::setInputCallback(PinInputCallback callback) {
  inputCallback_ = callback;
}

// ============================================================
// ExpiryDate判定 (P3-5)
// ============================================================
bool Is06PinManager::isExpired(int channel) {
  if (!isValidChannel(channel)) return false;

  int idx = channel - 1;

  // expiryEnabled がfalseなら常に有効
  if (!pinSettings_[idx].expiryEnabled) {
    return false;
  }

  // expiryDate が空なら有効
  if (pinSettings_[idx].expiryDate.isEmpty()) {
    return false;
  }

  // NtpManager未設定または未同期なら有効扱い（安全側）
  if (!ntp_ || !ntp_->isSynced()) {
    Serial.println("Is06PinManager: NTP not synced, skipping expiry check.");
    return false;
  }

  // 現在時刻取得
  time_t now = ntp_->getEpoch();
  if (now == 0) {
    return false;  // 時刻取得失敗
  }

  // expiryDate パース
  time_t expiry = parseExpiryDate(pinSettings_[idx].expiryDate);
  if (expiry == 0) {
    return false;  // パース失敗
  }

  // 比較
  bool expired = (now >= expiry);
  if (expired) {
    Serial.printf("Is06PinManager: CH%d expired (now=%ld, expiry=%ld)\n",
      channel, (long)now, (long)expiry);
  }

  return expired;
}

void Is06PinManager::setExpiryDate(int channel, const String& expiryDate) {
  if (!isValidChannel(channel)) return;

  int idx = channel - 1;
  pinSettings_[idx].expiryDate = expiryDate;

  // NVSに保存
  String key = getNvsKey(channel, "_expDt");
  settings_->setString(key.c_str(), expiryDate.c_str());

  Serial.printf("Is06PinManager: CH%d expiryDate set to %s\n",
    channel, expiryDate.c_str());
}

String Is06PinManager::getExpiryDate(int channel) {
  if (!isValidChannel(channel)) return "";
  return pinSettings_[channel - 1].expiryDate;
}

void Is06PinManager::setExpiryEnabled(int channel, bool enabled) {
  if (!isValidChannel(channel)) return;

  int idx = channel - 1;
  pinSettings_[idx].expiryEnabled = enabled;

  // NVSに保存
  String key = getNvsKey(channel, "_expEn");
  settings_->setString(key.c_str(), enabled ? "true" : "false");

  Serial.printf("Is06PinManager: CH%d expiryEnabled=%s\n",
    channel, enabled ? "true" : "false");
}

time_t Is06PinManager::parseExpiryDate(const String& expiryDate) {
  // フォーマット: YYYYMMDDHHMM (12桁)
  if (expiryDate.length() != 12) {
    Serial.printf("Is06PinManager: Invalid expiryDate format: %s\n",
      expiryDate.c_str());
    return 0;
  }

  struct tm timeinfo = {0};

  // 各フィールドを抽出
  timeinfo.tm_year = expiryDate.substring(0, 4).toInt() - 1900;
  timeinfo.tm_mon = expiryDate.substring(4, 6).toInt() - 1;
  timeinfo.tm_mday = expiryDate.substring(6, 8).toInt();
  timeinfo.tm_hour = expiryDate.substring(8, 10).toInt();
  timeinfo.tm_min = expiryDate.substring(10, 12).toInt();
  timeinfo.tm_sec = 0;

  // バリデーション
  if (timeinfo.tm_year < 0 || timeinfo.tm_mon < 0 || timeinfo.tm_mon > 11 ||
      timeinfo.tm_mday < 1 || timeinfo.tm_mday > 31 ||
      timeinfo.tm_hour < 0 || timeinfo.tm_hour > 23 ||
      timeinfo.tm_min < 0 || timeinfo.tm_min > 59) {
    Serial.printf("Is06PinManager: Invalid expiryDate values: %s\n",
      expiryDate.c_str());
    return 0;
  }

  // epoch秒に変換
  time_t epoch = mktime(&timeinfo);
  return epoch;
}

// ============================================================
// ユーティリティ
// ============================================================
void Is06PinManager::printStatus() {
  Serial.println("=== Is06PinManager Status ===");
  for (int ch = 1; ch <= IS06_CHANNEL_COUNT; ch++) {
    int idx = ch - 1;
    const char* typeStr = "?";
    switch (pinSettings_[idx].type) {
      case PinType::DIGITAL_OUTPUT: typeStr = "DO"; break;
      case PinType::PWM_OUTPUT: typeStr = "PWM"; break;
      case PinType::DIGITAL_INPUT: typeStr = "DI"; break;
      case PinType::PIN_DISABLED: typeStr = "OFF"; break;
    }
    Serial.printf("CH%d: %s en=%d state=%d pwm=%d\n",
      ch, typeStr,
      pinStates_[idx].enabled ? 1 : 0,
      pinStates_[idx].digitalState,
      pinStates_[idx].pwmValue);
  }
  Serial.println("=============================");
}

void Is06PinManager::loadFromNvs() {
  namespace ASD = AraneaSettingsDefaults;
  namespace NVS = AraneaSettingsDefaults::NVSKeys;

  Serial.println("Is06PinManager: Loading settings from NVS...");

  for (int ch = 1; ch <= IS06_CHANNEL_COUNT; ch++) {
    int idx = ch - 1;

    // enabled
    String enKey = getNvsKey(ch, NVS::CH_ENABLED_SUFFIX);
    String enValue = settings_->getString(enKey.c_str(), ASD::PIN_ENABLED_DEFAULT);
    pinStates_[idx].enabled = (enValue == "true");

    // type
    String typeKey = getNvsKey(ch, NVS::CH_TYPE_SUFFIX);
    String typeValue = settings_->getString(typeKey.c_str(), "");

    // 文字列からenumへ変換
    if (typeValue == ASD::PinType::DIGITAL_OUTPUT) {
      pinSettings_[idx].type = ::PinType::DIGITAL_OUTPUT;
    } else if (typeValue == ASD::PinType::PWM_OUTPUT) {
      pinSettings_[idx].type = ::PinType::PWM_OUTPUT;
    } else if (typeValue == ASD::PinType::DIGITAL_INPUT) {
      pinSettings_[idx].type = ::PinType::DIGITAL_INPUT;
    } else if (typeValue == ASD::PinType::PIN_DISABLED) {
      pinSettings_[idx].type = ::PinType::PIN_DISABLED;
    } else {
      // デフォルト: CH1-4=DO, CH5-6=DI
      if (ch <= 4) {
        pinSettings_[idx].type = ::PinType::DIGITAL_OUTPUT;
      } else {
        pinSettings_[idx].type = ::PinType::DIGITAL_INPUT;
      }
    }

    // name (P1-2)
    String nameKey = getNvsKey(ch, NVS::CH_NAME_SUFFIX);
    pinSettings_[idx].name = settings_->getString(nameKey.c_str(), "");

    // actionMode (P1-4): NVSから読み込み、なければタイプに応じたデフォルト
    String modeKey = getNvsKey(ch, NVS::CH_ACTION_MODE_SUFFIX);
    String modeValue = settings_->getString(modeKey.c_str(), "");

    if (modeValue == ASD::ActionMode::MOMENTARY) {
      pinSettings_[idx].actionMode = ::ActionMode::MOMENTARY;
    } else if (modeValue == ASD::ActionMode::ALTERNATE) {
      pinSettings_[idx].actionMode = ::ActionMode::ALTERNATE;
    } else if (modeValue == ASD::ActionMode::SLOW) {
      pinSettings_[idx].actionMode = ::ActionMode::SLOW;
    } else if (modeValue == ASD::ActionMode::RAPID) {
      pinSettings_[idx].actionMode = ::ActionMode::RAPID;
    } else if (modeValue == ASD::ActionMode::ROTATE) {
      pinSettings_[idx].actionMode = ::ActionMode::ROTATE;
    } else {
      // デフォルト: タイプに応じた値
      if (pinSettings_[idx].type == ::PinType::DIGITAL_OUTPUT) {
        pinSettings_[idx].actionMode = ::ActionMode::MOMENTARY;
      } else if (pinSettings_[idx].type == ::PinType::PWM_OUTPUT) {
        pinSettings_[idx].actionMode = ::ActionMode::SLOW;
      } else if (pinSettings_[idx].type == ::PinType::DIGITAL_INPUT) {
        pinSettings_[idx].actionMode = ::ActionMode::MOMENTARY;
      }
    }

    // validity (P1-5)
    String valKey = getNvsKey(ch, NVS::CH_VALIDITY_SUFFIX);
    String valValue = settings_->getString(valKey.c_str(), "");
    if (!valValue.isEmpty()) {
      pinSettings_[idx].validity = valValue.toInt();
    } else {
      pinSettings_[idx].validity = 0;  // 0 = PINglobalから参照
    }

    // debounce (P1-5)
    String debKey = getNvsKey(ch, NVS::CH_DEBOUNCE_SUFFIX);
    String debValue = settings_->getString(debKey.c_str(), "");
    if (!debValue.isEmpty()) {
      pinSettings_[idx].debounce = debValue.toInt();
    } else {
      pinSettings_[idx].debounce = 0;  // 0 = PINglobalから参照
    }

    // rateOfChange (P1-5)
    String rocKey = getNvsKey(ch, NVS::CH_RATE_OF_CHANGE_SUFFIX);
    String rocValue = settings_->getString(rocKey.c_str(), "");
    if (!rocValue.isEmpty()) {
      pinSettings_[idx].rateOfChange = rocValue.toInt();
    } else {
      pinSettings_[idx].rateOfChange = 0;  // 0 = PINglobalから参照
    }

    // allocation (P1-3)
    String allocKey = getNvsKey(ch, NVS::CH_ALLOCATION_SUFFIX);
    String allocValue = settings_->getString(allocKey.c_str(), "");
    parseAllocation(ch, allocValue);

    // stateName (P1-2)
    String stnKey = getNvsKey(ch, NVS::CH_STATE_NAME_SUFFIX);
    String stnValue = settings_->getString(stnKey.c_str(), "");
    parseStateName(ch, stnValue);

    // expiryDate settings (P3-5)
    String expEnKey = getNvsKey(ch, NVS::CH_EXPIRY_ENABLED_SUFFIX);
    String expEnValue = settings_->getString(expEnKey.c_str(), "false");
    pinSettings_[idx].expiryEnabled = (expEnValue == "true");

    String expDtKey = getNvsKey(ch, NVS::CH_EXPIRY_DATE_SUFFIX);
    pinSettings_[idx].expiryDate = settings_->getString(expDtKey.c_str(), "");
  }

  Serial.println("Is06PinManager: Settings loaded.");
}

void Is06PinManager::saveToNvs() {
  namespace ASD = AraneaSettingsDefaults;
  namespace NVS = AraneaSettingsDefaults::NVSKeys;

  Serial.println("Is06PinManager: Saving all settings to NVS...");

  for (int ch = 1; ch <= IS06_CHANNEL_COUNT; ch++) {
    int idx = ch - 1;
    const PinSetting& setting = pinSettings_[idx];

    // name
    String nameKey = getNvsKey(ch, NVS::CH_NAME_SUFFIX);
    settings_->setString(nameKey.c_str(), setting.name.c_str());

    // actionMode
    String modeKey = getNvsKey(ch, NVS::CH_ACTION_MODE_SUFFIX);
    const char* modeStr = "";
    switch (setting.actionMode) {
      case ::ActionMode::MOMENTARY: modeStr = ASD::ActionMode::MOMENTARY; break;
      case ::ActionMode::ALTERNATE: modeStr = ASD::ActionMode::ALTERNATE; break;
      case ::ActionMode::SLOW: modeStr = ASD::ActionMode::SLOW; break;
      case ::ActionMode::RAPID: modeStr = ASD::ActionMode::RAPID; break;
      case ::ActionMode::ROTATE: modeStr = ASD::ActionMode::ROTATE; break;
    }
    settings_->setString(modeKey.c_str(), modeStr);

    // validity
    if (setting.validity > 0) {
      String valKey = getNvsKey(ch, NVS::CH_VALIDITY_SUFFIX);
      settings_->setString(valKey.c_str(), String(setting.validity).c_str());
    }

    // debounce
    if (setting.debounce > 0) {
      String debKey = getNvsKey(ch, NVS::CH_DEBOUNCE_SUFFIX);
      settings_->setString(debKey.c_str(), String(setting.debounce).c_str());
    }

    // rateOfChange
    if (setting.rateOfChange > 0) {
      String rocKey = getNvsKey(ch, NVS::CH_RATE_OF_CHANGE_SUFFIX);
      settings_->setString(rocKey.c_str(), String(setting.rateOfChange).c_str());
    }

    // allocation (CSV形式)
    String allocKey = getNvsKey(ch, NVS::CH_ALLOCATION_SUFFIX);
    String allocStr = buildAllocationString(ch);
    settings_->setString(allocKey.c_str(), allocStr.c_str());

    // stateName (JSON配列形式)
    if (setting.stateNameCount > 0) {
      String stnKey = getNvsKey(ch, NVS::CH_STATE_NAME_SUFFIX);
      String jsonStr = "[";
      for (int i = 0; i < setting.stateNameCount; i++) {
        if (i > 0) jsonStr += ",";
        jsonStr += "\"" + setting.stateName[i] + "\"";
      }
      jsonStr += "]";
      settings_->setString(stnKey.c_str(), jsonStr.c_str());
    }
  }

  Serial.println("Is06PinManager: All settings saved.");
}

// ============================================================
// 設定セッター (P1-4, P1-5, P1-2, P1-3)
// ============================================================
void Is06PinManager::setActionMode(int channel, ::ActionMode mode) {
  namespace ASD = AraneaSettingsDefaults;
  namespace NVS = AraneaSettingsDefaults::NVSKeys;

  if (!isValidChannel(channel)) return;

  int idx = channel - 1;

  // PinTypeに対して許可されたactionModeかチェック（整合バリデーション）
  ::ActionMode validatedMode = mode;
  switch (pinSettings_[idx].type) {
    case ::PinType::DIGITAL_OUTPUT:
      // MOMENTARY or ALTERNATE のみ
      if (mode != ::ActionMode::MOMENTARY && mode != ::ActionMode::ALTERNATE) {
        validatedMode = ::ActionMode::MOMENTARY;
        Serial.printf("Is06PinManager: CH%d actionMode %d invalid for DO, using MOMENTARY\n", channel, (int)mode);
      }
      break;
    case ::PinType::PWM_OUTPUT:
      // SLOW or RAPID のみ
      if (mode != ::ActionMode::SLOW && mode != ::ActionMode::RAPID) {
        validatedMode = ::ActionMode::SLOW;
        Serial.printf("Is06PinManager: CH%d actionMode %d invalid for PWM, using SLOW\n", channel, (int)mode);
      }
      break;
    case ::PinType::DIGITAL_INPUT:
      // MOMENTARY, ALTERNATE, ROTATE 許可
      if (mode != ::ActionMode::MOMENTARY && mode != ::ActionMode::ALTERNATE && mode != ::ActionMode::ROTATE) {
        validatedMode = ::ActionMode::MOMENTARY;
        Serial.printf("Is06PinManager: CH%d actionMode %d invalid for DI, using MOMENTARY\n", channel, (int)mode);
      }
      break;
    default:
      break;
  }

  pinSettings_[idx].actionMode = validatedMode;

  // NVSに保存
  String modeKey = getNvsKey(channel, NVS::CH_ACTION_MODE_SUFFIX);
  const char* modeStr = "";
  switch (validatedMode) {
    case ::ActionMode::MOMENTARY: modeStr = ASD::ActionMode::MOMENTARY; break;
    case ::ActionMode::ALTERNATE: modeStr = ASD::ActionMode::ALTERNATE; break;
    case ::ActionMode::SLOW: modeStr = ASD::ActionMode::SLOW; break;
    case ::ActionMode::RAPID: modeStr = ASD::ActionMode::RAPID; break;
    case ::ActionMode::ROTATE: modeStr = ASD::ActionMode::ROTATE; break;
  }
  settings_->setString(modeKey.c_str(), modeStr);

  Serial.printf("Is06PinManager: CH%d actionMode set to %s\n", channel, modeStr);
}

void Is06PinManager::setValidity(int channel, int validity) {
  namespace NVS = AraneaSettingsDefaults::NVSKeys;

  if (!isValidChannel(channel)) return;

  int idx = channel - 1;
  pinSettings_[idx].validity = validity;

  String valKey = getNvsKey(channel, NVS::CH_VALIDITY_SUFFIX);
  settings_->setString(valKey.c_str(), String(validity).c_str());

  Serial.printf("Is06PinManager: CH%d validity set to %d\n", channel, validity);
}

void Is06PinManager::setDebounce(int channel, int debounce) {
  namespace NVS = AraneaSettingsDefaults::NVSKeys;

  if (!isValidChannel(channel)) return;

  int idx = channel - 1;
  pinSettings_[idx].debounce = debounce;

  String debKey = getNvsKey(channel, NVS::CH_DEBOUNCE_SUFFIX);
  settings_->setString(debKey.c_str(), String(debounce).c_str());

  Serial.printf("Is06PinManager: CH%d debounce set to %d\n", channel, debounce);
}

void Is06PinManager::setRateOfChange(int channel, int rateOfChange) {
  namespace NVS = AraneaSettingsDefaults::NVSKeys;

  if (!isValidDpChannel(channel)) return;

  int idx = channel - 1;
  pinSettings_[idx].rateOfChange = rateOfChange;

  String rocKey = getNvsKey(channel, NVS::CH_RATE_OF_CHANGE_SUFFIX);
  settings_->setString(rocKey.c_str(), String(rateOfChange).c_str());

  Serial.printf("Is06PinManager: CH%d rateOfChange set to %d\n", channel, rateOfChange);
}

void Is06PinManager::setName(int channel, const String& name) {
  namespace NVS = AraneaSettingsDefaults::NVSKeys;

  if (!isValidChannel(channel)) return;

  int idx = channel - 1;
  pinSettings_[idx].name = name;

  String nameKey = getNvsKey(channel, NVS::CH_NAME_SUFFIX);
  settings_->setString(nameKey.c_str(), name.c_str());

  Serial.printf("Is06PinManager: CH%d name set to %s\n", channel, name.c_str());
}

void Is06PinManager::setAllocation(int channel, const String allocations[], int count) {
  namespace NVS = AraneaSettingsDefaults::NVSKeys;

  if (!isValidChannel(channel)) return;

  int idx = channel - 1;

  // 配列クリア
  for (int i = 0; i < 4; i++) {
    pinSettings_[idx].allocation[i] = "";
  }

  // 新しい値を設定（最大4個）
  int actualCount = min(count, 4);
  for (int i = 0; i < actualCount; i++) {
    pinSettings_[idx].allocation[i] = allocations[i];
  }
  pinSettings_[idx].allocationCount = actualCount;

  // NVSにCSV形式で保存
  String allocKey = getNvsKey(channel, NVS::CH_ALLOCATION_SUFFIX);
  String allocStr = buildAllocationString(channel);
  settings_->setString(allocKey.c_str(), allocStr.c_str());

  Serial.printf("Is06PinManager: CH%d allocation set to %s\n", channel, allocStr.c_str());
}

// ============================================================
// Allocation ヘルパー
// ============================================================
void Is06PinManager::parseAllocation(int channel, const String& allocStr) {
  if (!isValidChannel(channel)) return;

  int idx = channel - 1;

  // 配列クリア
  for (int i = 0; i < 4; i++) {
    pinSettings_[idx].allocation[i] = "";
  }
  pinSettings_[idx].allocationCount = 0;

  if (allocStr.isEmpty()) return;

  // CSV形式 "CH1,CH2,CH3" をパース
  int count = 0;
  int start = 0;
  int commaPos;

  while ((commaPos = allocStr.indexOf(',', start)) != -1 && count < 4) {
    String item = allocStr.substring(start, commaPos);
    item.trim();
    if (!item.isEmpty()) {
      pinSettings_[idx].allocation[count++] = item;
    }
    start = commaPos + 1;
  }

  // 最後の要素
  if (start < (int)allocStr.length() && count < 4) {
    String item = allocStr.substring(start);
    item.trim();
    if (!item.isEmpty()) {
      pinSettings_[idx].allocation[count++] = item;
    }
  }

  pinSettings_[idx].allocationCount = count;
}

String Is06PinManager::buildAllocationString(int channel) {
  if (!isValidChannel(channel)) return "";

  int idx = channel - 1;
  String result = "";

  for (int i = 0; i < pinSettings_[idx].allocationCount; i++) {
    if (!pinSettings_[idx].allocation[i].isEmpty()) {
      if (!result.isEmpty()) result += ",";
      result += pinSettings_[idx].allocation[i];
    }
  }

  return result;
}

// ============================================================
// StateName 設定 (P1-2)
// ============================================================
void Is06PinManager::setStateName(int channel, const String stateNames[], int count) {
  namespace NVS = AraneaSettingsDefaults::NVSKeys;

  if (!isValidChannel(channel)) return;

  int idx = channel - 1;

  // 配列クリア
  for (int i = 0; i < 4; i++) {
    pinSettings_[idx].stateName[i] = "";
  }

  // 新しい値を設定（最大4個）
  int actualCount = min(count, 4);
  for (int i = 0; i < actualCount; i++) {
    pinSettings_[idx].stateName[i] = stateNames[i];
  }
  pinSettings_[idx].stateNameCount = actualCount;

  // NVSにJSON配列形式で保存
  String key = getNvsKey(channel, NVS::CH_STATE_NAME_SUFFIX);
  String jsonStr = "[";
  for (int i = 0; i < actualCount; i++) {
    if (i > 0) jsonStr += ",";
    jsonStr += "\"" + stateNames[i] + "\"";
  }
  jsonStr += "]";
  settings_->setString(key.c_str(), jsonStr.c_str());

  Serial.printf("Is06PinManager: CH%d stateName set to %s\n", channel, jsonStr.c_str());
}

String Is06PinManager::getStateName(int channel, int index) {
  if (!isValidChannel(channel)) return "";
  if (index < 0 || index >= 4) return "";

  int idx = channel - 1;
  return pinSettings_[idx].stateName[index];
}

void Is06PinManager::parseStateName(int channel, const String& jsonStr) {
  if (!isValidChannel(channel)) return;

  int idx = channel - 1;

  // 配列クリア
  for (int i = 0; i < 4; i++) {
    pinSettings_[idx].stateName[i] = "";
  }
  pinSettings_[idx].stateNameCount = 0;

  if (jsonStr.isEmpty()) return;

  // 簡易JSONパース: ["value1","value2",...]
  int count = 0;
  int start = jsonStr.indexOf('[');
  int end = jsonStr.lastIndexOf(']');
  if (start < 0 || end < 0 || end <= start) return;

  String content = jsonStr.substring(start + 1, end);

  // カンマ区切りで分解（引用符内のカンマは無視）
  int pos = 0;
  while (pos < (int)content.length() && count < 4) {
    // 次の引用符を探す
    int quoteStart = content.indexOf('"', pos);
    if (quoteStart < 0) break;
    int quoteEnd = content.indexOf('"', quoteStart + 1);
    if (quoteEnd < 0) break;

    pinSettings_[idx].stateName[count++] = content.substring(quoteStart + 1, quoteEnd);
    pos = quoteEnd + 1;
  }

  pinSettings_[idx].stateNameCount = count;
}

// ============================================================
// 内部メソッド
// ============================================================
void Is06PinManager::applyDigitalOutput(int channel, int state) {
  if (!isValidChannel(channel)) return;
  int idx = channel - 1;
  int pin = IS06_PIN_MAP[idx];

  // 【重要】LEDCがアタッチされている可能性があるため、まずデタッチ
  // これがないと、gpio_set_level()が効かない場合がある
  if (idx < IS06_DP_CHANNELS && ledcChannel_[idx] >= 0) {
    ledcDetach(pin);
    ledcChannel_[idx] = -1;
    Serial.printf("Is06PinManager: CH%d GPIO%d LEDC detached for digital output\n", channel, pin);
  }

  // 【根本原因修正】IO_MUXとGPIO Matrixを明示的に設定
  // Step 1: IO_MUXでGPIO functionを選択
  gpio_pad_select_gpio(pin);

  // Step 2: GPIO Matrixで出力をシンプルGPIOに設定 (SIG_GPIO_OUT_IDX = 256)
  gpio_matrix_out(pin, 256, false, false);

  // Step 3: gpio_config()で完全な設定
  gpio_config_t io_conf = {};
  io_conf.intr_type = GPIO_INTR_DISABLE;
  io_conf.mode = GPIO_MODE_INPUT_OUTPUT;  // 入出力両方を有効化（読み取り可能に）
  io_conf.pin_bit_mask = (1ULL << pin);
  io_conf.pull_down_en = GPIO_PULLDOWN_DISABLE;
  io_conf.pull_up_en = GPIO_PULLUP_DISABLE;
  esp_err_t err = gpio_config(&io_conf);

  // Step 4: ドライブ強度を最大に設定
  gpio_set_drive_capability((gpio_num_t)pin, GPIO_DRIVE_CAP_3);

  // Step 5: 出力レベル設定
  gpio_set_level((gpio_num_t)pin, state ? 1 : 0);

  // 確認読み取り
  int readBack = gpio_get_level((gpio_num_t)pin);
  Serial.printf("Is06PinManager: applyDigitalOutput CH%d GPIO%d set=%d read=%d err=%d (full config)\n",
                channel, pin, state, readBack, err);
}

void Is06PinManager::applyPwmOutput(int channel, int value) {
  using namespace AraneaSettingsDefaults;

  if (!isValidDpChannel(channel)) return;
  int idx = channel - 1;
  int pin = IS06_PIN_MAP[idx];
  int ch = ledcChannel_[idx];

  // 0-100% → 0-255
  int pwm8bit = map(value, 0, 100, 0, 255);

  if (ch >= 0) {
    // LEDCチャンネルが有効な場合はledcWriteChannel()を使用
    // Arduino-ESP32 3.x: ledcWrite(pin,duty)は非推奨、ledcWriteChannel(ch,duty)を使う
    ledcWriteChannel(ch, pwm8bit);
    Serial.printf("Is06PinManager: applyPwmOutput CH%d GPIO%d ch=%d value=%d duty=%d\n",
                  channel, pin, ch, value, pwm8bit);
  } else {
    // LEDCチャンネルが無効 - 自動的にLEDCをアタッチして再試行
    Serial.printf("Is06PinManager: WARNING CH%d GPIO%d LEDC not attached, attempting attach...\n",
                  channel, pin);

    ledcDetach(pin);
    int newCh = ledcAttach(pin, PWM_FREQUENCY, PWM_RESOLUTION);
    if (newCh >= 0) {
      ledcChannel_[idx] = newCh;
      ledcWriteChannel(newCh, pwm8bit);
      Serial.printf("Is06PinManager: applyPwmOutput CH%d GPIO%d LEDC attached ch=%d value=%d duty=%d\n",
                    channel, pin, newCh, value, pwm8bit);
    } else {
      // LEDC失敗時はGPIO直接制御（ON/OFFのみ）
      // value > 0 ならHIGH（フルオン相当）、0ならLOW
      int state = (value > 0) ? 1 : 0;
      gpio_set_level((gpio_num_t)pin, state);
      Serial.printf("Is06PinManager: applyPwmOutput CH%d GPIO%d LEDC failed, GPIO=%d\n",
                    channel, pin, state);
    }
  }
}

String Is06PinManager::getNvsKey(int channel, const char* suffix) {
  return String("ch") + String(channel) + String(suffix);
}

bool Is06PinManager::isValidChannel(int channel) {
  return (channel >= 1 && channel <= IS06_CHANNEL_COUNT);
}

bool Is06PinManager::isValidDpChannel(int channel) {
  return (channel >= 1 && channel <= IS06_DP_CHANNELS);
}

bool Is06PinManager::isValidIoChannel(int channel) {
  return (channel > IS06_DP_CHANNELS && channel <= IS06_CHANNEL_COUNT);
}
