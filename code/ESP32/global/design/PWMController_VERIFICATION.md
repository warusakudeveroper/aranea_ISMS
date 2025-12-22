# PWMController Digital↔PWM切替 検証設計書

**作成日**: 2025/12/22
**対象**: DigitalOutput ↔ PWM モード切替
**リスクレベル**: 高（LEDCチャンネル競合・出力異常の可能性）

---

## 1. 危険性分析

### 1.1 想定される事故シナリオ

| # | シナリオ | 影響 | 深刻度 |
|---|---------|------|--------|
| 1 | LEDCチャンネル二重割当 | PWM出力異常・フリーズ | **致命的** |
| 2 | Digital→PWM切替時のグリッチ | モーター急加速等 | 高 |
| 3 | PWM→Digital切替でLEDCリソースリーク | チャンネル枯渇 | 高 |
| 4 | 周波数変更中の出力不定 | 接続機器誤動作 | 中 |
| 5 | デューティ設定前のPWM開始 | 意図しない出力 | 中 |
| 6 | フェード中のモード切替 | 中間状態で固定 | 中 |

### 1.2 ESP32 LEDC制約

```
ESP32 LEDC仕様:
- 16チャンネル (0-15)
- 高速チャンネル (0-7): 80MHz基準
- 低速チャンネル (8-15): 8MHz基準
- 同一チャンネルは周波数・解像度共有
- チャンネル割当は明示的に管理必要
```

---

## 2. LEDCチャンネル管理

### 2.1 チャンネル割当ポリシー

```cpp
// グローバルチャンネル管理（シングルトン）
class LEDCChannelManager {
public:
    static LEDCChannelManager& getInstance() {
        static LEDCChannelManager instance;
        return instance;
    }

    // チャンネル予約（-1: 失敗）
    int allocateChannel(int pin) {
        for (int ch = 0; ch < MAX_CHANNELS; ch++) {
            if (!allocated_[ch]) {
                allocated_[ch] = true;
                pinToChannel_[pin] = ch;
                return ch;
            }
        }
        Serial.println("[LEDC] ERROR: No available channel");
        return -1;
    }

    // チャンネル解放
    void releaseChannel(int pin) {
        auto it = pinToChannel_.find(pin);
        if (it != pinToChannel_.end()) {
            allocated_[it->second] = false;
            pinToChannel_.erase(it);
        }
    }

    // ピンのチャンネル取得（-1: 未割当）
    int getChannel(int pin) {
        auto it = pinToChannel_.find(pin);
        return (it != pinToChannel_.end()) ? it->second : -1;
    }

    // デバッグ: 使用状況表示
    void printStatus() {
        Serial.println("=== LEDC Channel Status ===");
        for (auto& pair : pinToChannel_) {
            Serial.printf("  GPIO%d -> CH%d\n", pair.first, pair.second);
        }
    }

private:
    static const int MAX_CHANNELS = 8;  // is06aでは4chまで使用
    bool allocated_[MAX_CHANNELS] = {false};
    std::map<int, int> pinToChannel_;

    LEDCChannelManager() {}
};
```

### 2.2 予約済みチャンネル

| チャンネル | 用途 | デバイス |
|-----------|------|---------|
| 0 | GPIO07 PWM | is06a TRG1 |
| 1 | GPIO12 PWM | is06a TRG2 |
| 2 | GPIO14 PWM | is06a TRG3 |
| 3 | GPIO27 PWM | is06a TRG4 |
| 4-7 | 予備 | - |
| 8-15 | 使用禁止 | 低速のため |

---

## 3. モード切替設計

### 3.1 3つのモード

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            UNINITIALIZED                                │
│                      ピン未設定、操作無効                                │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ begin(pin)
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            DIGITAL_OUTPUT                               │
│  - digitalWrite()使用                                                   │
│  - LEDCチャンネル未使用                                                  │
│  - pulse()対応                                                          │
└─────────────────────────────────────────────────────────────────────────┘
            │                                       ▲
            │ enablePWM()                           │ disablePWM()
            ▼                                       │
┌─────────────────────────────────────────────────────────────────────────┐
│                              PWM_OUTPUT                                 │
│  - ledcWrite()使用                                                      │
│  - LEDCチャンネル占有                                                    │
│  - setDuty()/fadeTo()対応                                               │
│  - pulse()無効                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Digital → PWM 遷移

```cpp
bool TriggerController::enablePWM() {
    if (mode_ == Mode::PWM_OUTPUT) {
        return true;  // 既にPWM
    }

    // 1. パルス実行中チェック
    if (pulseActive_) {
        Serial.println("[TRG] ERROR: cannot enable PWM during pulse");
        return false;
    }

    // 2. フェード中チェック
    if (fading_) {
        Serial.println("[TRG] ERROR: cannot enable PWM during fade");
        return false;
    }

    // 3. LEDCチャンネル確保
    channel_ = LEDCChannelManager::getInstance().allocateChannel(pin_);
    if (channel_ < 0) {
        Serial.println("[TRG] ERROR: no LEDC channel available");
        return false;
    }

    // 4. 現在の出力状態を保存
    bool wasHigh = (digitalRead(pin_) == HIGH);

    // 5. LEDC初期化（デューティ0で開始 = 安全）
    ledcSetup(channel_, frequency_, resolution_);
    ledcAttachPin(pin_, channel_);
    ledcWrite(channel_, 0);  // 必ず0から開始

    // 6. 必要なら元の状態を復元
    if (wasHigh) {
        ledcWrite(channel_, getMaxDuty());
    }

    // 7. モード更新
    mode_ = Mode::PWM_OUTPUT;
    currentDuty_ = wasHigh ? getMaxDuty() : 0;

    Serial.printf("[TRG] GPIO%d: PWM enabled (CH%d)\n", pin_, channel_);
    return true;
}
```

### 3.3 PWM → Digital 遷移

```cpp
bool TriggerController::disablePWM() {
    if (mode_ != Mode::PWM_OUTPUT) {
        return true;  // 既にDigital
    }

    // 1. フェード中チェック
    if (fading_) {
        Serial.println("[TRG] ERROR: cannot disable PWM during fade");
        return false;
    }

    // 2. 現在のデューティを判定（50%超ならHIGH扱い）
    bool shouldBeHigh = (currentDuty_ > getMaxDuty() / 2);

    // 3. LEDC解除（重要: 先にデタッチ）
    ledcDetachPin(pin_);
    LEDCChannelManager::getInstance().releaseChannel(pin_);
    channel_ = -1;

    // 4. デジタル出力に切替
    pinMode(pin_, OUTPUT);
    digitalWrite(pin_, shouldBeHigh ? HIGH : LOW);

    // 5. モード更新
    mode_ = Mode::DIGITAL_OUTPUT;
    currentOutput_ = shouldBeHigh;

    Serial.printf("[TRG] GPIO%d: PWM disabled, output=%s\n",
                  pin_, shouldBeHigh ? "HIGH" : "LOW");
    return true;
}
```

---

## 4. 安全機構

### 4.1 PWM開始前チェック

```cpp
bool TriggerController::setDuty(int duty) {
    // 1. PWMモードチェック
    if (mode_ != Mode::PWM_OUTPUT) {
        Serial.println("[TRG] ERROR: setDuty requires PWM mode");
        return false;
    }

    // 2. チャンネル有効チェック
    if (channel_ < 0) {
        Serial.println("[TRG] ERROR: LEDC channel not allocated");
        return false;
    }

    // 3. 範囲チェック
    duty = constrain(duty, 0, getMaxDuty());

    // 4. 適用
    ledcWrite(channel_, duty);
    currentDuty_ = duty;

    return true;
}
```

### 4.2 フェード安全機構

```cpp
bool TriggerController::fadeTo(int targetDuty, int durationMs) {
    // 1. PWMモードチェック
    if (mode_ != Mode::PWM_OUTPUT) {
        Serial.println("[TRG] ERROR: fadeTo requires PWM mode");
        return false;
    }

    // 2. 既にフェード中なら上書き
    if (fading_) {
        Serial.println("[TRG] WARNING: overwriting existing fade");
    }

    // 3. パラメータ検証
    targetDuty = constrain(targetDuty, 0, getMaxDuty());
    if (durationMs <= 0) {
        // 即時設定
        return setDuty(targetDuty);
    }

    // 4. フェード開始
    fadeStartDuty_ = currentDuty_;
    fadeTargetDuty_ = targetDuty;
    fadeStartMs_ = millis();
    fadeDurationMs_ = durationMs;
    fading_ = true;

    return true;
}

void TriggerController::update() {
    if (!fading_) return;

    unsigned long elapsed = millis() - fadeStartMs_;

    if (elapsed >= fadeDurationMs_) {
        // フェード完了
        ledcWrite(channel_, fadeTargetDuty_);
        currentDuty_ = fadeTargetDuty_;
        fading_ = false;

        if (onFadeCompleteCallback_) {
            onFadeCompleteCallback_();
        }
    } else {
        // 線形補間（オーバーフロー防止）
        int32_t delta = (int32_t)fadeTargetDuty_ - fadeStartDuty_;
        int32_t progress = (delta * elapsed) / fadeDurationMs_;
        int newDuty = fadeStartDuty_ + progress;

        ledcWrite(channel_, newDuty);
        currentDuty_ = newDuty;
    }
}
```

---

## 5. 検証テストケース

### 5.1 Digital↔PWM切替テスト

| # | テスト名 | 手順 | 期待結果 | 判定 |
|---|---------|------|---------|------|
| T1 | Digital→PWM基本 | begin→enablePWM→setDuty(128) | LED 50%点灯 | |
| T2 | PWM→Digital基本 | enablePWM→setDuty(255)→disablePWM | LED点灯維持 | |
| T3 | PWM→Digital(LOW) | enablePWM→setDuty(0)→disablePWM | LED消灯維持 | |
| T4 | パルス中PWM切替拒否 | pulse(3000)→enablePWM | false返却 | |
| T5 | フェード中PWM解除拒否 | fadeTo(255,1000)→disablePWM | false返却 | |

### 5.2 LEDCチャンネル管理テスト

| # | テスト名 | 手順 | 期待結果 | 判定 |
|---|---------|------|---------|------|
| C1 | チャンネル自動割当 | 4ピンでenablePWM | CH0-3が割当 | |
| C2 | チャンネル解放 | enablePWM→disablePWM→enablePWM | 同じCHが再割当 | |
| C3 | チャンネル枯渇 | 9ピンでenablePWM | 9番目でfalse | |
| C4 | 二重割当防止 | 同じピンで2回enablePWM | 2回目はtrue(変化なし) | |

### 5.3 フェードテスト

| # | テスト名 | 手順 | 期待結果 | 判定 |
|---|---------|------|---------|------|
| F1 | 0→100%フェード | fadeTo(255, 1000) | 1秒かけて点灯 | |
| F2 | 100→0%フェード | fadeTo(0, 1000) | 1秒かけて消灯 | |
| F3 | フェード上書き | fadeTo(255,2000)→1秒後→fadeTo(0,500) | 途中で反転 | |
| F4 | 即時完了(0ms) | fadeTo(255, 0) | 即座に100% | |

### 5.4 グリッチ検証（オシロスコープ）

| # | テスト名 | 手順 | 期待結果 | 判定 |
|---|---------|------|---------|------|
| G1 | Digital→PWM波形 | モード切替を観測 | グリッチ<100μs | |
| G2 | PWM→Digital波形 | モード切替を観測 | グリッチ<100μs | |
| G3 | PWM周波数変更波形 | 1kHz→10kHz変更を観測 | 連続的に変化 | |

### 5.5 異常系テスト

| # | テスト名 | 手順 | 期待結果 | 判定 |
|---|---------|------|---------|------|
| E1 | 未初期化setDuty | setDuty(128) (begin未実行) | false返却 | |
| E2 | Digitalモードでフェード | fadeTo(255,1000) (PWM無効) | false返却 | |
| E3 | 負のデューティ | setDuty(-10) | 0に補正 | |
| E4 | 最大超過デューティ | setDuty(1000) (8bit時) | 255に補正 | |

---

## 6. 周波数・解像度変更

### 6.1 安全な周波数変更

```cpp
bool TriggerController::setFrequency(int hz) {
    // 1. 範囲チェック
    if (hz < 1000 || hz > 40000) {
        Serial.printf("[TRG] ERROR: frequency %d out of range\n", hz);
        return false;
    }

    // 2. フェード中チェック
    if (fading_) {
        Serial.println("[TRG] ERROR: cannot change frequency during fade");
        return false;
    }

    // 3. PWMモードなら再設定必要
    if (mode_ == Mode::PWM_OUTPUT) {
        int savedDuty = currentDuty_;

        ledcSetup(channel_, hz, resolution_);
        ledcWrite(channel_, savedDuty);  // デューティ復元
    }

    frequency_ = hz;
    return true;
}
```

### 6.2 解像度変更は再初期化

```cpp
bool TriggerController::setResolution(int bits) {
    // 1. 範囲チェック
    if (bits != 8 && bits != 10) {
        Serial.println("[TRG] ERROR: resolution must be 8 or 10");
        return false;
    }

    // 2. PWMモードでは再初期化
    if (mode_ == Mode::PWM_OUTPUT) {
        // デューティをパーセントで保存
        float pct = getDutyPercent();

        resolution_ = bits;
        ledcSetup(channel_, frequency_, resolution_);

        // パーセントで復元（新しい解像度に合わせる）
        setDutyPercent(pct);
    } else {
        resolution_ = bits;
    }

    return true;
}
```

---

## 7. 実装チェックリスト

### 7.1 コードレビュー項目

- [ ] LEDCChannelManager がシングルトンで実装されている
- [ ] enablePWM()でチャンネル確保失敗を正しくハンドリング
- [ ] disablePWM()でledcDetachPin()を呼んでいる
- [ ] disablePWM()でチャンネルを解放している
- [ ] フェード中のモード切替を禁止している
- [ ] デューティ設定前にPWMモードをチェック
- [ ] 周波数・解像度変更時にデューティを保存/復元
- [ ] 範囲外の値をconstrainしている

### 7.2 動作確認項目

- [ ] 4ch同時PWM動作確認
- [ ] Digital→PWM→Digitalサイクル確認
- [ ] フェード完了コールバック確認
- [ ] 電源OFF/ONで状態復元確認

---

## 8. 参照

- **ESP32 LEDC**: https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/peripherals/ledc.html
- **is06a設計書**: `code/ESP32/is06a/design/DESIGN.md`
- **PWMController仕様**: `code/ESP32/global/design/PWMController_SPEC.md`
