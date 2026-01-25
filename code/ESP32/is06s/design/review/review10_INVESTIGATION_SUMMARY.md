# IS06S GPIO18/GPIO5 電圧問題 - 調査報告書 v10

**作成日**: 2026-01-25
**報告者**: Claude Code
**ステータス**: **未解決 - 検証依頼**

---

## 1. 問題の概要

### 症状
- CH1 (GPIO18) および CH2 (GPIO5) から**出力がされない**
- CH3 (GPIO15), CH4 (GPIO27) は正常動作（リレー駆動確認済み）
- 以前は同じハードウェアで3.3V出力が確認されている → **コードの問題**

### 制約
- シリアル接続は使用不可（ユーザー指示）
- OTA経由での検証のみ

---

## 2. review8.md に沿った実施内容

### 2-1. GPIO Matrix Signal ID 確認 (Method B)
**実装**: `/api/debug/gpio` エンドポイント追加

```
結果:
CH1 GPIO18: sigOutId=256 (Simple GPIO) ✓
CH2 GPIO5:  sigOutId=256 (Simple GPIO) ✓
CH3 GPIO15: sigOutId=256 (Simple GPIO) ✓
CH4 GPIO27: sigOutId=256 (Simple GPIO) ✓
```

**結論**: ペリフェラル（LEDC/SPI）による干渉は**なし**

### 2-2. 高頻度サンプリング (Method A)
**実装**: 1000回の`gpio_get_level()`サンプリング

```
結果（CH1, CH2をONにした状態）:
CH1 GPIO18: highCount=1000, lowCount=0, transitions=0
CH2 GPIO5:  highCount=1000, lowCount=0, transitions=0
```

**結論**: PWMトグルなし、ソフトウェア的にはHIGH状態

### 2-3. レジスタ直接読み取り
**実装**: GPIO_OUT_REG, GPIO_ENABLE_REG, GPIO_IN_REG, GPIO_PINx_REG, IO_MUX 直接読み取り

```
結果（CH1をONにした状態）:
GPIO18:
- GPIO_OUT_REG bit: 1 (出力ラッチHIGH)
- GPIO_ENABLE_REG bit: 1 (出力有効)
- GPIO_IN_REG bit: 1 (入力読み取りHIGH)
- isOpenDrain: false (プッシュプル)
- iomuxFunSel: 2 (GPIO function)
```

**矛盾点**: ソフトウェア的にはすべて正常だが、**実際の電圧は出力されていない**

---

## 3. 実施した修正

### 3-1. gpio_pad_select_gpio() + gpio_matrix_out() 追加
**ファイル**: `Is06PinManager.cpp` - `applyDigitalOutput()`

```cpp
// IO_MUXでGPIO functionを選択
gpio_pad_select_gpio(pin);

// GPIO Matrixで出力をシンプルGPIOに設定
gpio_matrix_out(pin, 256, false, false);

// gpio_config()で完全な設定
gpio_config_t io_conf = {};
io_conf.mode = GPIO_MODE_INPUT_OUTPUT;
io_conf.pin_bit_mask = (1ULL << pin);
gpio_config(&io_conf);

gpio_set_drive_capability((gpio_num_t)pin, GPIO_DRIVE_CAP_3);
gpio_set_level((gpio_num_t)pin, state ? 1 : 0);
```

**結果**: 診断APIでIN=1を確認、しかし**実際の電圧変化なし**

### 3-2. initLedc(), applyLoadedPinTypes() への同様の修正適用
**結果**: 変化なし

---

## 4. 未実施項目

### ESP-IDF固有ツールでの検証
- [ ] `idf.py monitor` でのリアルタイムGPIOデバッグ（シリアル不可のため実施できず）
- [ ] `gpio_dump_io_configuration()` の出力確認（シリアル不可）
- [ ] ESP-IDF menuconfig でのGPIO設定確認
- [ ] eFuse読み取りによるGPIO制限確認

### ハードウェアレベル検証
- [ ] ESP32チップのGPIOパッド直接のオシロスコープ測定
- [ ] 別のESP32-DevKitCでの同一ファームウェアテスト
- [ ] 最小限スケッチ（WiFi/OLED無効）でのGPIO18単独テスト

---

## 5. 現在の状態

### コード変更箇所
1. `Is06PinManager.cpp`:
   - `applyDigitalOutput()`: gpio_pad_select_gpio + gpio_matrix_out + gpio_config 追加
   - `initLedc()`: 同様の完全GPIO設定追加
   - `applyLoadedPinTypes()`: DIGITAL_OUTPUT, DIGITAL_INPUT ケースに同様の修正
   - インクルード追加: `<rom/gpio.h>`, `<soc/io_mux_reg.h>`

2. `HttpManagerIs06s.cpp`:
   - `/api/debug/gpio` エンドポイント追加
   - GPIO Matrix, IO_MUX, レジスタ直接読み取り診断

### 診断API出力例
```json
{
  "gpio": 18,
  "channel": 1,
  "sigOutId": 256,
  "isSimpleGpio": true,
  "gpioOutBit": 1,
  "gpioEnableBit": 1,
  "gpioInBit": 1,
  "isOpenDrain": false,
  "iomuxFunSel": 2,
  "softwareState": 1,
  "highCount": 1000,
  "lowCount": 0,
  "diagnosis": "STABLE_HIGH"
}
```

**問題**: 診断はすべて正常を示すが、**物理的な出力電圧がない**

---

## 6. 仮説と検証状況

| 仮説 | 検証方法 | 結果 |
|------|----------|------|
| LEDCペリフェラル干渉 | sigOutId確認 | 256 (GPIO) - 干渉なし |
| SPI干渉 | コード検索 | SPI使用なし |
| IO_MUX function未設定 | iomuxFunSel確認 | 2 (GPIO) - 正常 |
| オープンドレイン設定 | GPIO_PINx_REG確認 | プッシュプル - 正常 |
| 出力有効化漏れ | GPIO_ENABLE_REG確認 | 1 - 有効 |
| PWMトグル | 高頻度サンプリング | トグルなし |
| gpio_set_direction不足 | gpio_config()使用 | 効果なし |
| gpio_pad_select_gpio不足 | 追加実装 | 効果なし |

---

## 7. 検証依頼

### 依頼内容
以下の追加検証をお願いします：

1. **ESP-IDFネイティブ環境での最小テスト**
   ```c
   // main.c (ESP-IDF純正)
   void app_main(void) {
       gpio_pad_select_gpio(18);
       gpio_set_direction(GPIO_NUM_18, GPIO_MODE_OUTPUT);
       gpio_set_level(GPIO_NUM_18, 1);
       // 電圧測定
   }
   ```

2. **別のESP32-DevKitCでの同一ファームウェアテスト**
   - 同じis06s.inoを別ボードにフラッシュ
   - GPIO18の電圧測定

3. **Arduino-ESP32のバージョン確認**
   - 現在: esp32:esp32 3.0.5
   - 既知の問題がないか確認

### 関連ドキュメント
- [review8.md](./review8.md) - GPIO Matrix調査指針
- [review9_ROOT_CAUSE_ANALYSIS.md](./review9_ROOT_CAUSE_ANALYSIS.md) - 初期仮説（無効化済み）
- [CH1_CH2_VOLTAGE_ISSUE_v3.md](./CH1_CH2_VOLTAGE_ISSUE_v3.md) - 問題詳細

---

## 8. 結論

**現状**: ソフトウェアレベルではすべて正常に設定されているが、物理的な出力がない

**可能性のある原因**:
1. Arduino-ESP32 3.x のGPIO APIに未知のバグ
2. ESP32チップ固有の問題（GPIO18/5特有）
3. ESP-IDFレイヤーでの設定競合

**推奨アクション**:
1. ESP-IDF純正環境での同一GPIOテスト
2. Arduino-ESP32 GitHubでの類似Issue検索
3. 別ハードウェアでの再現確認

---

## 9. 参考情報（Web検索結果）

### Arduino-ESP32 3.x 既知の問題
- [GitHub Issue #11343 - PHY and GPIO error after 3.1](https://github.com/espressif/arduino-esp32/issues/11343)
  - WiFi.hとの互換性問題が報告されている
  - 3.0.7では問題なし、3.1.3以降でエラー発生
- [GitHub Issue #9154 - GPIO pinMode error](https://github.com/espressif/arduino-esp32/issues/9154)
  - `GPIO can only be used as input mode` エラーの報告

### ESP32 GPIO5 特記事項
- **GPIO5はストラッピングピン**: ブートモード選択に使用
- **CANコントローラのデフォルトTXピン**: GPIO5
- 起動時の状態に影響を受ける可能性あり

### ESP-IDF GPIO ドキュメント
- [ESP-IDF GPIO API Reference](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/peripherals/gpio.html)

---

**連絡先**: GitHub Issue または CLAUDE.md 参照
