# Review13: GPIO18/GPIO5 gpio_reset_pin追加 設計書

**作成日**: 2026-01-26
**ステータス**: 設計完了 → 実装待ち
**対象Issue**: GPIO18/GPIO5出力不可問題

---

## 大原則確認（タスク開始時）

- [x] SSoT: 本設計書を唯一の変更定義とする
- [x] SOLID: 単一責任（Is06PinManager内で完結）
- [x] MECE: 全初期化パスを漏れなく列挙
- [x] アンアンビギュアス: 変更箇所・行番号を明示
- [x] 確証バイアス排除: review10の実測データを尊重、SPI発振説を仮説扱いに
- [x] 設計ドキュメントあり: 本書

---

## 1. 問題の概要

### 1.1 現象
- GPIO18 (CH1), GPIO5 (CH2) から3.3V出力が得られない
- 実測値: 約1.25〜1.37V（中間電位）
- GPIO15, GPIO27, GPIO14, GPIO12 は正常動作

### 1.2 実測証拠（review10より）
| 項目 | 測定値 | 解釈 |
|------|--------|------|
| sigOutId | 256 | Simple GPIO（ペリフェラル干渉なし） |
| highCount | 1000 | 安定HIGH |
| transitions | 0 | トグルなし（発振なし） |
| GPIO_OUT_REG | 1 | 出力ラッチHIGH |
| GPIO_ENABLE_REG | 1 | 出力有効 |
| GPIO_IN_REG | 1 | 読み返しHIGH |

### 1.3 原因分析

**レビュー1〜5の統合結論**:
1. ソフトウェアレジスタは正常（sigOutId=256, レジスタ整合）
2. SPI発振説は実測データと矛盾（transitions=0）
3. `gpio_reset_pin`が通常初期化パスで呼ばれていない
4. gpiotestでは毎回`gpio_reset_pin`を呼んでいる

**優先仮説**（review4より）:
- A) 外部回路によるクランプ/負荷
- B) 出力段の残留設定（`gpio_reset_pin`未実行による）
- C) 測定点がGPIOパッド直でない

---

## 2. 変更設計（MECE）

### 2.1 変更対象関数一覧

| 関数名 | ファイル | 行番号 | 現状 | 変更内容 |
|--------|----------|--------|------|----------|
| `initLedc()` | Is06PinManager.cpp | L53-92 | gpio_reset_pinなし | 追加 |
| `applyLoadedPinTypes()` | Is06PinManager.cpp | L97-176 | gpio_reset_pinなし | 追加 |
| `applyDigitalOutput()` | Is06PinManager.cpp | L1259-1298 | gpio_reset_pinなし | 追加 |

### 2.2 変更対象外（既に対応済み）

| 関数名 | 行番号 | 状況 |
|--------|--------|------|
| `setPinType()` | L520-580 | ✅ 既にgpio_reset_pinあり |

### 2.3 変更詳細

#### 2.3.1 initLedc() 変更

**現行コード (L58-84)**:
```cpp
for (int i = 0; i < IS06_DP_CHANNELS; i++) {
    int pin = IS06_PIN_MAP[i];
    ledcDetach(pin);
    ledcChannel_[i] = -1;
    gpio_pad_select_gpio(pin);  // ← ここから
    gpio_matrix_out(pin, 256, false, false);
    // ...
}
```

**変更後**:
```cpp
for (int i = 0; i < IS06_DP_CHANNELS; i++) {
    int pin = IS06_PIN_MAP[i];
    ledcDetach(pin);
    ledcChannel_[i] = -1;

    // 【修正】周辺機能の強制停止（review12対応）
    gpio_reset_pin((gpio_num_t)pin);

    gpio_pad_select_gpio(pin);
    gpio_matrix_out(pin, 256, false, false);
    // ...
}
```

#### 2.3.2 applyLoadedPinTypes() 変更

**DIGITAL_OUTPUT経路 (L131-149)**:
```cpp
} else if (type == ::PinType::DIGITAL_OUTPUT) {
    ledcDetach(pin);
    if (idx < IS06_DP_CHANNELS) ledcChannel_[idx] = -1;

    // 【修正】追加
    gpio_reset_pin((gpio_num_t)pin);

    gpio_pad_select_gpio(pin);
    // ...
}
```

**PWM_OUTPUT経路 (L107-130)** - フォールバック時:
```cpp
} else {
    ledcChannel_[idx] = -1;
    // 【修正】追加
    gpio_reset_pin((gpio_num_t)pin);

    gpio_pad_select_gpio(pin);
    // ...
}
```

**DIGITAL_INPUT経路 (L150-164)**:
```cpp
} else if (type == ::PinType::DIGITAL_INPUT) {
    ledcDetach(pin);
    if (idx < IS06_DP_CHANNELS) ledcChannel_[idx] = -1;

    // 【修正】追加
    gpio_reset_pin((gpio_num_t)pin);

    gpio_pad_select_gpio(pin);
    // ...
}
```

#### 2.3.3 applyDigitalOutput() 変更

**現行コード (L1264-1277)**:
```cpp
if (idx < IS06_DP_CHANNELS && ledcChannel_[idx] >= 0) {
    ledcDetach(pin);
    ledcChannel_[idx] = -1;
}

// 【修正】追加位置
gpio_reset_pin((gpio_num_t)pin);

gpio_pad_select_gpio(pin);
gpio_matrix_out(pin, 256, false, false);
```

---

## 3. タスクリスト

| # | タスク | 状態 |
|---|--------|------|
| T1 | initLedc()にgpio_reset_pin追加 | ✅ |
| T2 | applyLoadedPinTypes() DIGITAL_OUTPUT経路修正 | ✅ |
| T3 | applyLoadedPinTypes() PWM_OUTPUTフォールバック経路修正 | ✅ |
| T4 | applyLoadedPinTypes() DIGITAL_INPUT経路修正 | ✅ |
| T5 | applyDigitalOutput()にgpio_reset_pin追加 | ✅ |
| T6 | コンパイル確認 | ✅ (74% Flash使用) |
| T7 | OTAアップロード | ✅ (192.168.77.108) |
| T8 | テスト実行 | ✅ |

---

## 4. テスト計画

### 4.1 コンパイルテスト
- [ ] `mcp__mcp-arduino-esp32__compile`でエラーなし

### 4.2 機能テスト（OTA後）

| テストID | 内容 | 期待値 | 実測 |
|----------|------|--------|------|
| FT-01 | CH1 (GPIO18) Digital ON | 3.3V | ⬜ 要物理測定 |
| FT-02 | CH2 (GPIO5) Digital ON | 3.3V | ⬜ 要物理測定 |
| FT-03 | CH3 (GPIO15) Digital ON | 3.3V | ⬜ 要物理測定 |
| FT-04 | CH4 (GPIO27) Digital ON | 3.3V | ⬜ 要物理測定 |
| FT-05 | CH1 Digital OFF | 0V | ⬜ 要物理測定 |
| FT-06 | CH2 Digital OFF | 0V | ⬜ 要物理測定 |
| FT-07 | /api/debug/gpio CH1 sigOutId | 256 | ✅ 256 (STABLE_HIGH) |
| FT-08 | /api/debug/gpio CH2 sigOutId | 256 | ✅ 256 (STABLE_HIGH) |

### 4.2.1 APIテスト結果 (2026-01-26)

全チャンネルをONにした状態でのGPIOデバッグ結果:

```
CH | GPIO | sigOutId   | gpioOutBit | gpioInBit | diagnosis
---|------|------------|------------|-----------|-------------------
1  | 18   | SimpleGPIO | 1          | 1         | STABLE_HIGH
2  | 5    | SimpleGPIO | 1          | 1         | STABLE_HIGH
3  | 15   | SimpleGPIO | 1          | 1         | STABLE_HIGH
4  | 27   | SimpleGPIO | 1          | 1         | STABLE_HIGH
```

**ソフトウェアレベル**: 全チャンネル正常（sigOutId=256, gpioOutBit=1, gpioInBit=1）
**物理電圧**: 要実測（テスターで3.3V確認必要）

### 4.3 回帰テスト

| テストID | 内容 | 期待値 |
|----------|------|--------|
| RT-01 | Web UI アクセス | 正常表示 |
| RT-02 | /api/status | JSON応答 |
| RT-03 | OLED表示 | CIC + Ready |
| RT-04 | WiFi接続維持 | 切断なし |

---

## 5. ロールバック計画

変更がさらなる問題を引き起こした場合:
1. 現行ファームウェア (`is06s.ino.bin`) をバックアップ
2. OTA経由で以前のバージョンに戻す

---

## 6. MECE確認

### 6.1 GPIO初期化パス網羅性

```
begin()
├── initGpio() → CH5-6のみ（CH1-4はinitLedcで）
├── initLedc() → CH1-4 ✅ 修正対象
├── loadFromNvs()
└── applyLoadedPinTypes() ✅ 修正対象
    ├── PWM_OUTPUT → ledcAttach or フォールバック ✅
    ├── DIGITAL_OUTPUT ✅ 修正対象
    ├── DIGITAL_INPUT ✅ 修正対象
    └── PIN_DISABLED → 既にgpio_reset_pinあり

setPinState()
└── applyDigitalOutput() ✅ 修正対象

setPwmValue()
└── applyPwmOutput() → ledcWriteChannel（GPIO設定なし、対象外）

setPinType()
└── gpio_reset_pin ✅ 既存

```

**結論**: 全パス網羅済み（MECE）

---

## 7. 調査依頼書との整合

本修正後も問題が解決しない場合、review4の優先仮説に基づき以下を検討:
- A) 外部回路調査（ハードウェア側）
- C) 測定点確認（GPIOパッド直測定）

本修正は「B) 出力段の残留設定」仮説への対応。

---

## 大原則確認チェックリスト（実装完了時に記入）

- [x] SSoT: 本設計書通りに実装
- [x] テスト実行済み（ソフトウェアレベル）
- [x] 回帰テストパス（Web UI, /api/status正常）
- [ ] コミット・プッシュ完了
- [ ] 物理電圧測定（要ユーザー確認）

---

## 8. 実装履歴

### 2026-01-26 実装完了

**変更ファイル**: `Is06PinManager.cpp`

**追加箇所**:
1. L68: `initLedc()` - `gpio_reset_pin((gpio_num_t)pin);`
2. L138: `applyLoadedPinTypes()` DIGITAL_OUTPUT - `gpio_reset_pin((gpio_num_t)pin);`
3. L123: `applyLoadedPinTypes()` PWMフォールバック - `gpio_reset_pin((gpio_num_t)pin);`
4. L165: `applyLoadedPinTypes()` DIGITAL_INPUT - `gpio_reset_pin((gpio_num_t)pin);`
5. L1282: `applyDigitalOutput()` - `gpio_reset_pin((gpio_num_t)pin);`

**ビルド結果**: Flash 74%使用、エラーなし
**OTA結果**: 192.168.77.108 正常稼働

**重要な発見**:
- CH1がNVSで`pwmOutput`設定だったため、LEDCがアタッチされていた（sigOutId=71）
- `digitalOutput`に変更後、sigOutId=256（Simple GPIO）となり正常動作
- 全チャンネルでソフトウェアレジスタは正常（gpioOutBit=1, gpioInBit=1）
