# IOController I/O切替 検証設計書

**作成日**: 2025/12/22
**対象**: IOController の INPUT ↔ OUTPUT モード切替
**リスクレベル**: 高（ハードウェア損傷・誤動作の可能性）

---

## 1. 危険性分析

### 1.1 想定される事故シナリオ

| # | シナリオ | 影響 | 深刻度 |
|---|---------|------|--------|
| 1 | OUTPUT状態で外部からHIGH入力 | GPIO焼損 | **致命的** |
| 2 | モード切替中のグリッチパルス | 接続機器誤動作 | 高 |
| 3 | 切替後の初期状態不定 | 意図しない出力 | 高 |
| 4 | 設定変更のレースコンディション | 状態不整合 | 中 |
| 5 | NVS設定とピン状態の不一致 | 再起動時の誤動作 | 中 |

### 1.2 ハードウェア保護要件

```
重要: GPIO損傷防止のためのハードウェア設計要件

1. OUTPUT使用ピンには必ず直列抵抗（330Ω〜1kΩ）を入れる
2. 外部機器接続時はレベルシフタまたはフォトカプラを推奨
3. I/O切替対応ピン(GPIO05, 18)は保護回路必須
```

---

## 2. 状態遷移設計

### 2.1 モード状態図

```
                    ┌─────────────────────────────────────────┐
                    │           UNINITIALIZED                 │
                    │  pin未設定、どの操作も無効              │
                    └─────────────────────────────────────────┘
                                        │
                                        │ begin(pin, mode)
                                        ▼
        ┌───────────────────────────────────────────────────────────────┐
        │                                                               │
        ▼                                                               ▼
┌─────────────────────────┐                         ┌─────────────────────────┐
│       INPUT_MODE        │                         │      OUTPUT_MODE        │
│                         │                         │                         │
│ - sample()有効          │   setMode(OUTPUT)       │ - setOutput()有効       │
│ - hasChanged()有効      │ ───────────────────────▶│ - pulse()有効           │
│ - isActive()有効        │                         │ - update()有効          │
│                         │   setMode(INPUT)        │                         │
│ - setOutput()無効       │ ◀───────────────────────│ - sample()無効          │
│ - pulse()無効           │                         │ - hasChanged()無効      │
└─────────────────────────┘                         └─────────────────────────┘
```

### 2.2 モード遷移時の処理シーケンス

```cpp
// INPUT → OUTPUT への遷移
void transitionToOutput() {
    // 1. 現在のサンプリングを停止
    sampling_enabled_ = false;

    // 2. ピンモードを変更（LOWで初期化）
    digitalWrite(pin_, LOW);  // 先にLOW設定
    pinMode(pin_, OUTPUT);    // その後OUTPUTに変更

    // 3. 状態変数をリセット
    pulseActive_ = false;
    currentOutput_ = false;

    // 4. モードフラグを更新
    mode_ = Mode::OUTPUT;
}

// OUTPUT → INPUT への遷移
void transitionToInput() {
    // 1. 出力を停止
    digitalWrite(pin_, LOW);
    pulseActive_ = false;

    // 2. ピンモードを変更
    if (pullup_) {
        pinMode(pin_, INPUT_PULLUP);
    } else {
        pinMode(pin_, INPUT_PULLDOWN);
    }

    // 3. 安定化待ち（重要）
    delay(10);  // プルアップ/ダウン安定化

    // 4. 状態変数を初期化
    rawState_ = digitalRead(pin_);
    stableState_ = rawState_;
    stableCount_ = 0;
    changed_ = false;

    // 5. モードフラグを更新
    mode_ = Mode::INPUT;

    // 6. サンプリングを有効化
    sampling_enabled_ = true;
}
```

---

## 3. 安全機構

### 3.1 モード切替前チェック

```cpp
bool IOController::setMode(Mode newMode) {
    // 1. 同一モードへの遷移は無視
    if (mode_ == newMode) {
        return true;
    }

    // 2. 未初期化チェック
    if (pin_ < 0) {
        Serial.println("[IOController] ERROR: pin not initialized");
        return false;
    }

    // 3. パルス実行中は切替禁止
    if (pulseActive_) {
        Serial.println("[IOController] ERROR: cannot switch mode during pulse");
        return false;
    }

    // 4. 禁止ピンチェック（ストラッピングピン等）
    if (!isSafeToSwitch(pin_)) {
        Serial.printf("[IOController] ERROR: GPIO%d is not safe for I/O switch\n", pin_);
        return false;
    }

    // 5. 遷移実行
    if (newMode == Mode::INPUT) {
        transitionToInput();
    } else {
        transitionToOutput();
    }

    return true;
}
```

### 3.2 禁止ピン定義

```cpp
// I/O切替禁止ピン
bool IOController::isSafeToSwitch(int pin) {
    // ストラッピングピン
    if (pin == 0 || pin == 2 || pin == 15) {
        return false;
    }
    // 入力専用ピン
    if (pin >= 34 && pin <= 39) {
        return false;
    }
    // フラッシュ使用ピン（4MB WROOM）
    if (pin >= 6 && pin <= 11) {
        return false;
    }
    return true;
}
```

### 3.3 グリッチ防止

```cpp
// OUTPUT設定時はLOWを先に設定
void safeSetOutputMode(int pin) {
    // 重要: ピンモード変更前にLOWを確定
    GPIO.out_w1tc = (1 << pin);  // 直接レジスタでLOW設定
    GPIO.enable_w1ts = (1 << pin);  // 出力有効化
    pinMode(pin, OUTPUT);  // Arduinoの設定も更新
}
```

---

## 4. 検証テストケース

### 4.1 単体テスト

| # | テスト名 | 手順 | 期待結果 | 判定 |
|---|---------|------|---------|------|
| T1 | INPUT→OUTPUT基本 | begin(18, INPUT)→setMode(OUTPUT)→setOutput(HIGH) | LED点灯 | |
| T2 | OUTPUT→INPUT基本 | begin(18, OUTPUT)→setMode(INPUT)→接点短絡 | isActive()=true | |
| T3 | 未初期化切替 | setMode(OUTPUT) (begin未実行) | false返却、変化なし | |
| T4 | 同一モード切替 | begin(18, INPUT)→setMode(INPUT) | true返却、変化なし | |
| T5 | パルス中切替拒否 | pulse(3000)→setMode(INPUT) | false返却、パルス継続 | |
| T6 | 禁止ピン切替拒否 | begin(2, INPUT)→setMode(OUTPUT) | false返却 | |

### 4.2 グリッチ検証（オシロスコープ使用）

| # | テスト名 | 手順 | 期待結果 | 判定 |
|---|---------|------|---------|------|
| G1 | INPUT→OUTPUT遷移波形 | モード切替をオシロで観測 | グリッチ<1ms、LOWに遷移 | |
| G2 | OUTPUT→INPUT遷移波形 | モード切替をオシロで観測 | グリッチ<1ms | |
| G3 | 連続切替 | 100回連続でモード切替 | 全て正常遷移 | |

### 4.3 統合テスト

| # | テスト名 | 手順 | 期待結果 | 判定 |
|---|---------|------|---------|------|
| I1 | NVS保存→再起動 | モード変更→保存→再起動 | 保存したモードで起動 | |
| I2 | Web UIからモード変更 | /api/config で mode変更 | 即座に反映 | |
| I3 | 入力検出→出力パルス | ch1入力検出→ch7出力 | 正常動作 | |

### 4.4 異常系テスト

| # | テスト名 | 手順 | 期待結果 | 判定 |
|---|---------|------|---------|------|
| E1 | OUTPUT時に外部HIGH | GPIO18をOUTPUT、外部から3.3V印加 | 保護抵抗で電流制限 | |
| E2 | 高速連続切替 | 1ms間隔で100回切替 | エラーなく完了 | |
| E3 | 割り込み中の切替 | タイマー割り込み中にsetMode | 正常完了 | |

---

## 5. 実装チェックリスト

### 5.1 コードレビュー項目

- [ ] setMode()でパルス実行中チェックがある
- [ ] transitionToOutput()でLOWを先に設定している
- [ ] transitionToInput()で安定化delayがある
- [ ] 禁止ピンチェックがある
- [ ] エラー時にログ出力がある
- [ ] 戻り値で成功/失敗を返している

### 5.2 ハードウェアチェック項目

- [ ] I/O切替対応ピンに保護抵抗がある
- [ ] 外部機器との接続にフォトカプラ/レベルシフタがある
- [ ] テスト用LEDで動作確認可能

---

## 6. 運用時の注意事項

### 6.1 禁止操作

```
絶対禁止:
1. OUTPUTモードのピンに外部から電圧を印加しない
2. パルス実行中にモード切替しない
3. ストラッピングピン(GPIO0,2,15)でI/O切替しない
4. 入力専用ピン(GPIO34-39)をOUTPUTにしない
```

### 6.2 推奨運用

```
推奨:
1. 起動時にモードを確定し、動作中は切替を最小限に
2. モード切替前に必ずログ出力
3. 設定変更はWeb UI経由で行い、NVSに保存
4. 外部機器は電気的に絶縁
```

---

## 7. 参照

- **ESP32 GPIO制約**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
- **is05a設計書**: `code/ESP32/is05a/design/DESIGN.md`
- **is06a設計書**: `code/ESP32/is06a/design/DESIGN.md`
