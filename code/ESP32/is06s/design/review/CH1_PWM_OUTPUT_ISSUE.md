# IS06S CH1 PWM/Digital出力問題 - 検証依頼書

**作成日**: 2026-01-25
**報告者**: Claude Code
**緊急度**: 高（基本機能の動作不良）

---

## 1. 問題概要

CH1 (GPIO18) からの出力電圧が異常に低い（1.9V程度）。
- **期待値**: 3.3V (HIGH時)
- **実測値**: 1.9V (Digital/PWM両モード)
- **以前の動作**: 正常に3.3V出力できていた

---

## 2. 発生経緯

### タイムライン

1. **変更前**: PWM周波数5kHz、正常動作
2. **変更**: LEDテープライト対応のため周波数を20kHzに変更
   - `AraneaSettingsDefaults.h`: `PWM_FREQUENCY = 20000`
3. **変更後**: CH1から3.3Vが出力されなくなった

### 関連する変更履歴

```
d5a75b3 feat(is06s): HTTPS/MQTT再有効化 - Serial出力削減で安定化確認
a824d34 fix(is06s): Serial出力削減で安定化 - OTA更新確認済み
```

---

## 3. 確認済み事項

### 3.1 ハードウェア正常性確認
- ✅ ESP32の3.3V端子から直接MOSFETにトリガー入力 → **100%発光**
- ✅ MOSFETおよび負荷（LEDテープ）は正常
- → **ハードウェア問題ではない**

### 3.2 API/ソフトウェア状態
```bash
# Digital Outputモード、state=1
curl "http://192.168.77.107/api/pin/1/state"
{"channel":1,"enabled":true,"state":1,"pwm":0,"type":"digitalOutput","actionMode":"Alt"}
# 電圧: 1.9V

# PWM Outputモード、pwm=100
curl "http://192.168.77.107/api/pin/1/state"
{"channel":1,"enabled":true,"state":1,"pwm":100,"type":"pwmOutput","actionMode":"Rapid"}
# 電圧: 1.9V
```

### 3.3 試行した修正

#### 修正1: initGpio()からpinMode(OUTPUT)を削除
**理由**: LEDCとpinModeの競合を回避

```cpp
// Before
void Is06PinManager::initGpio() {
  for (int i = 0; i < IS06_DP_CHANNELS; i++) {
    pinMode(IS06_PIN_MAP[i], OUTPUT);      // ← 削除
    digitalWrite(IS06_PIN_MAP[i], LOW);    // ← 削除
  }
}

// After
void Is06PinManager::initGpio() {
  // CH1-4 (D/P Type): LEDCで管理するためpinModeは呼ばない
  // CH5-6のみINPUT_PULLDOWN設定
}
```

**結果**: 改善せず

#### 修正2: ledcAttach()の戻り値確認ログ追加
```cpp
void Is06PinManager::initLedc() {
  for (int i = 0; i < IS06_DP_CHANNELS; i++) {
    bool ok = ledcAttach(IS06_PIN_MAP[i], PWM_FREQUENCY, PWM_RESOLUTION);
    if (ok) {
      Serial.printf("  CH%d (GPIO%d): LEDC attached OK\n", i + 1, IS06_PIN_MAP[i]);
    } else {
      Serial.printf("  CH%d (GPIO%d): LEDC attach FAILED!\n", i + 1, IS06_PIN_MAP[i]);
    }
  }
}
```

**結果**: シリアルログ未確認（USBケーブル未接続のためOTA更新のみ）

---

## 4. 技術的背景

### 4.1 PIN構成
| CH | GPIO | 用途 | LEDCチャンネル |
|----|------|------|----------------|
| 1 | 18 | D/P Type | 0 |
| 2 | 5 | D/P Type | 1 |
| 3 | 15 | D/P Type | 2 |
| 4 | 27 | D/P Type | 3 |
| 5 | 14 | I/O Type | - |
| 6 | 12 | I/O Type | - |

### 4.2 PWM設定
```cpp
// AraneaSettingsDefaults.h
constexpr int PWM_FREQUENCY = 20000;  // 20kHz（変更後）
constexpr int PWM_RESOLUTION = 8;     // 8bit (0-255)
```

### 4.3 Arduino-ESP32 3.x API
```cpp
// 新API（3.x）
ledcAttach(pin, freq, resolution);  // チャンネル自動割り当て
ledcWrite(pin, duty);               // ピン指定で書き込み

// 旧API（2.x）との違い
// - ledcSetup() + ledcAttachPin() → ledcAttach() に統合
// - チャンネル番号の明示指定が不要に
```

---

## 5. 仮説と検証項目

### 仮説A: 20kHz周波数がGPIO18で問題を起こしている
- ESP32のLEDCは周波数と解像度に制限がある
- 理論上: 80MHz / 20kHz = 4000 → 最大約12bit解像度（8bitは余裕のはず）
- **検証**: 周波数を5kHzに戻して動作確認

### 仮説B: ledcAttach()が失敗している
- 戻り値がfalseの場合、ピンはLEDCにアタッチされていない
- **検証**: シリアルログで「LEDC attached OK」または「FAILED」を確認

### 仮説C: 複数回のledcAttach()呼び出しによる競合
- `initLedc()`と`setPinType()`の両方で`ledcAttach()`を呼んでいる
- **検証**: 初期化シーケンスのログを詳細確認

### 仮説D: GPIO18特有の問題
- GPIO18はVSPI SCKとしても使用可能
- 他の機能との競合の可能性
- **検証**: CH2-4 (GPIO5, 15, 27) の動作確認

### 仮説E: PWM duty値の問題
- `ledcWrite(pin, 255)`が正しく動作していない
- **検証**: `applyPwmOutput()`のログで`duty=255`が出力されているか確認

---

## 6. 検証依頼事項

### 優先度: 高
1. **シリアルログの取得**
   - デバイスをUSB接続してシリアルモニターでログを確認
   - 特に起動時の「LEDC attached OK/FAILED」メッセージ

2. **周波数を5kHzに戻して検証**
   ```cpp
   // AraneaSettingsDefaults.h
   constexpr int PWM_FREQUENCY = 5000;  // 5kHzに戻す
   ```

### 優先度: 中
3. **他のチャンネル (CH2-4) の動作確認**
   - 同様の問題が発生するか
   - GPIO依存の問題か全体的な問題かの切り分け

4. **ledcDetach()後の再アタッチテスト**
   ```cpp
   ledcDetach(IS06_PIN_MAP[0]);
   delay(100);
   ledcAttach(IS06_PIN_MAP[0], 5000, 8);
   ledcWrite(IS06_PIN_MAP[0], 255);
   ```

### 優先度: 低
5. **Arduino-ESP32のバージョン確認**
   - 現在: 3.0.5
   - 3.1.x へのアップデートで解決する可能性

---

## 7. 関連ファイル

- `Is06PinManager.cpp` - PIN制御ロジック
- `Is06PinManager.h` - PIN定義
- `AraneaSettingsDefaults.h` - PWM設定定数
- `HttpManagerIs06s.cpp` - HTTP API

---

## 8. 補足情報

### 1.9V出力の意味
- 3.3V × (約58%) ≈ 1.9V
- PWM duty約148/255 に相当
- なぜこの値になるのか不明

### ESP32 GPIO18の特性
- 出力可能
- VSPI SCK（SPI Clock）としても使用可能
- プルアップ/プルダウン内蔵

---

## 9. 次のアクション

1. シリアルログを取得して原因特定
2. 周波数5kHzでの動作確認
3. 必要に応じてGPIO18を他のGPIOに変更検討

---

**連絡先**: GitHub Issue または CLAUDE.md 参照
