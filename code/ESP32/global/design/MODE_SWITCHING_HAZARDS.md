# モード切替における危険性と対策

**作成日**: 2025/12/22
**対象**: IOController I/O切替、TriggerController Digital↔PWM切替
**目的**: 事故防止のための設計指針と実装ルール

---

## 1. 危険度マトリクス

### 1.1 I/O切替（INPUT ↔ OUTPUT）

| 危険性 | 発生確率 | 影響度 | リスクレベル | 対策状態 |
|--------|---------|--------|-------------|---------|
| OUTPUT時に外部電圧印加→GPIO焼損 | 中 | **致命的** | **最高** | 要ハードウェア保護 |
| 切替時グリッチ→接続機器誤動作 | 高 | 高 | 高 | ソフトウェア対策 |
| 禁止ピン使用→起動失敗 | 低 | 高 | 中 | チェック実装済 |
| NVS設定不整合→再起動時誤動作 | 中 | 中 | 中 | 起動時検証 |

### 1.2 Digital↔PWM切替

| 危険性 | 発生確率 | 影響度 | リスクレベル | 対策状態 |
|--------|---------|--------|-------------|---------|
| LEDCチャンネル二重割当→フリーズ | 中 | **致命的** | **最高** | チャンネル管理必須 |
| フェード中モード切替→状態不定 | 高 | 高 | 高 | 禁止処理実装 |
| PWM→Digitalでリソースリーク | 中 | 中 | 中 | 解放処理必須 |
| 周波数変更中の出力異常 | 低 | 中 | 低 | デューティ保存 |

---

## 2. 絶対禁止事項

### 2.1 ハードウェア

```
🚫 絶対禁止:

1. OUTPUTモードのGPIOに外部から電圧を印加しない
   → GPIOドライバが破損する

2. 保護抵抗なしでI/O切替対応ピンを使用しない
   → 短絡時に過電流でGPIO焼損

3. 入力専用ピン(GPIO34-39)をOUTPUTに設定しない
   → ハードウェア的に不可能、動作不定

4. ストラッピングピン(GPIO0,2,15)でI/O切替しない
   → 起動に影響、ブートループの可能性
```

### 2.2 ソフトウェア

```
🚫 絶対禁止:

1. パルス実行中にモード切替しない
   → パルスが途中で切れる、状態不整合

2. フェード実行中にPWMを解除しない
   → 中間デューティで固定、期待と異なる動作

3. LEDCチャンネルを手動で割り当てない
   → チャンネル競合、他のPWMが異常動作

4. begin()前にモード切替/出力操作しない
   → 未初期化ピンへのアクセス、動作不定

5. 同一ピンに対してIOControllerとPWMControllerを
   両方使用しない
   → 状態管理が競合、予測不能な動作
```

---

## 3. 必須実装パターン

### 3.1 I/O切替の正しいパターン

```cpp
// ✅ 正しい実装
class IOController {
public:
    bool setMode(Mode newMode) {
        // 1. 同一モードは早期リターン
        if (mode_ == newMode) return true;

        // 2. パルス中は拒否
        if (pulseActive_) {
            logError("Cannot switch during pulse");
            return false;
        }

        // 3. 禁止ピンチェック
        if (!isSafePin(pin_)) {
            logError("Unsafe pin");
            return false;
        }

        // 4. 安全な遷移
        if (newMode == Mode::OUTPUT) {
            digitalWrite(pin_, LOW);  // LOWを先に
            pinMode(pin_, OUTPUT);
        } else {
            digitalWrite(pin_, LOW);
            pinMode(pin_, pullup_ ? INPUT_PULLUP : INPUT_PULLDOWN);
            delay(10);  // 安定化待ち
            rawState_ = digitalRead(pin_);
            stableState_ = rawState_;
        }

        mode_ = newMode;
        return true;
    }
};

// ❌ 間違った実装
void badSetMode(int pin, int mode) {
    pinMode(pin, mode);  // チェックなし
    // パルス中でも切り替えてしまう
    // 禁止ピンでも切り替えてしまう
}
```

### 3.2 Digital↔PWM切替の正しいパターン

```cpp
// ✅ 正しい実装
class TriggerController {
public:
    bool enablePWM() {
        if (mode_ == Mode::PWM) return true;

        // 1. パルス/フェード中チェック
        if (pulseActive_ || fading_) {
            logError("Cannot enable PWM during operation");
            return false;
        }

        // 2. チャンネル確保
        channel_ = LEDCManager::allocate(pin_);
        if (channel_ < 0) {
            logError("No LEDC channel");
            return false;
        }

        // 3. 現状態を保存
        bool wasHigh = digitalRead(pin_) == HIGH;

        // 4. PWM初期化（0から開始）
        ledcSetup(channel_, freq_, res_);
        ledcAttachPin(pin_, channel_);
        ledcWrite(channel_, 0);

        // 5. 必要なら状態復元
        if (wasHigh) {
            ledcWrite(channel_, getMaxDuty());
        }

        mode_ = Mode::PWM;
        return true;
    }

    bool disablePWM() {
        if (mode_ != Mode::PWM) return true;

        // 1. フェード中チェック
        if (fading_) {
            logError("Cannot disable during fade");
            return false;
        }

        // 2. 現在値でDigital状態決定
        bool shouldBeHigh = currentDuty_ > getMaxDuty() / 2;

        // 3. LEDC解放（順序重要）
        ledcDetachPin(pin_);
        LEDCManager::release(pin_);
        channel_ = -1;

        // 4. Digital出力に戻す
        pinMode(pin_, OUTPUT);
        digitalWrite(pin_, shouldBeHigh ? HIGH : LOW);

        mode_ = Mode::DIGITAL;
        return true;
    }
};

// ❌ 間違った実装
void badEnablePWM(int pin) {
    ledcSetup(0, 5000, 8);  // チャンネル固定 → 競合
    ledcAttachPin(pin, 0);
    // チャンネル管理なし
    // 状態保存なし
}
```

---

## 4. 推奨ハードウェア構成

### 4.1 I/O切替対応ピンの保護回路

```
ESP32 GPIO ──[330Ω]──┬── 外部機器
                     │
                    [保護ダイオード]
                     │
                    GND
```

### 4.2 高電流負荷制御

```
ESP32 GPIO ──[1kΩ]──┤ フォトカプラ├── AC/DC負荷
                    └─────────────┘
```

### 4.3 モーター/LED駆動（PWM）

```
ESP32 GPIO ──[1kΩ]──┤ MOSFETドライバ├── モーター/LED
                    └───────────────┘
```

---

## 5. デバッグ・トラブルシューティング

### 5.1 I/O切替の問題

| 症状 | 原因 | 対処 |
|------|------|------|
| モード切替がfalse返却 | パルス実行中 | パルス完了を待つ |
| モード切替がfalse返却 | 禁止ピン | 別のGPIOを使用 |
| 切替後に入力が不安定 | プルアップ未設定 | setPullup(true) |
| OUTPUT時にGPIO焼損 | 外部電圧印加 | 保護抵抗追加 |

### 5.2 PWM切替の問題

| 症状 | 原因 | 対処 |
|------|------|------|
| enablePWMがfalse返却 | チャンネル枯渇 | 他のPWM解除 |
| PWM周波数がおかしい | チャンネル共有 | チャンネル確認 |
| フェードが途中で止まる | update()未呼出 | loop()で呼出 |
| PWM→Digital後もPWM | detach忘れ | ledcDetachPin()確認 |

### 5.3 デバッグログ

```cpp
// デバッグモード有効化
#define IO_DEBUG 1

#if IO_DEBUG
    #define IO_LOG(fmt, ...) Serial.printf("[IO] " fmt "\n", ##__VA_ARGS__)
#else
    #define IO_LOG(fmt, ...)
#endif

// 使用例
IO_LOG("GPIO%d: mode changed to %s", pin_, modeStr);
```

---

## 6. テスト実施チェックリスト

### 6.1 開発時テスト

- [ ] 各ピンで INPUT→OUTPUT→INPUT サイクル確認
- [ ] 各ピンで DIGITAL→PWM→DIGITAL サイクル確認
- [ ] パルス中のモード切替が拒否されることを確認
- [ ] フェード中のPWM解除が拒否されることを確認
- [ ] 4ch同時PWM動作確認
- [ ] オシロスコープでグリッチ確認

### 6.2 出荷前テスト

- [ ] 電源OFF/ONで設定復元確認
- [ ] 24時間連続動作でメモリリークなし
- [ ] 高温環境(40℃)での動作確認
- [ ] 外部ノイズ環境での動作確認

### 6.3 運用開始後

- [ ] 週次でログ確認（エラーなし）
- [ ] 月次でチャンネル使用状況確認

---

## 7. 参照

- **IOController検証**: `IOController_VERIFICATION.md`
- **PWMController検証**: `PWMController_VERIFICATION.md`
- **ESP32 GPIO制約**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
