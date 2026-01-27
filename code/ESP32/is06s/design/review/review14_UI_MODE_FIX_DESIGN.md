# Review14: UI Mode/Allocation整合性修正 設計書

**作成日**: 2026-01-26
**ステータス**: 設計完了 → 実装待ち

---

## 1. 問題の概要

### 1.1 Mode ドロップダウン問題
スクリーンショットで確認された問題:
- **Digital Output** で `Slow (PWM)`, `Rapid (PWM)`, `Rotate (Input)` が表示される
- **PWM Output** で `Momentary`, `Alternate`, `Rotate (Input)` が表示される
- これにより不正な組み合わせが設定可能になっている

### 1.2 Allocation フィールド問題
- 現在: テキスト入力 (`CH1,CH2`)
- 問題: 任意の文字列が入力可能、CH5-CH6への連動も設定できてしまう
- 正しい仕様: Digital Input のみ有効、連動先はCH1-CH4の出力チャンネルに限定

### 1.3 正しい仕様（MECE）

| PinType | 有効なMode |
|---------|-----------|
| Digital Output | Momentary, Alternate |
| PWM Output | Slow, Rapid |
| Digital Input | Momentary, Alternate, Rotate |
| Disabled | (なし) |

| PinType | Allocation有効 | 連動先 |
|---------|---------------|--------|
| Digital Output | ❌ | - |
| PWM Output | ❌ | - |
| Digital Input | ✅ | CH1-CH4のみ |
| Disabled | ❌ | - |

---

## 2. 変更設計

### 2.1 変更箇所

**ファイル**: `HttpManagerIs06s.cpp` - `generateTypeSpecificJS()`

### 2.2 Mode ドロップダウン動的生成

現行 (L350-356):
```javascript
html += '<select id="mode-' + ch + '">';
html += '<option value="Mom">Momentary</option>';
html += '<option value="Alt">Alternate</option>';
html += '<option value="Slow">Slow (PWM)</option>';
html += '<option value="Rapid">Rapid (PWM)</option>';
html += '<option value="rotate">Rotate (Input)</option>';
html += '</select>';
```

変更後:
```javascript
html += '<select id="mode-' + ch + '">';
html += getModeOptionsForType(p.type, p.actionMode);
html += '</select>';
```

新規関数:
```javascript
function getModeOptionsForType(type, currentMode) {
  var opts = '';
  if (type === 'digitalOutput') {
    opts += '<option value="Mom"' + (currentMode === 'Mom' ? ' selected' : '') + '>Momentary</option>';
    opts += '<option value="Alt"' + (currentMode === 'Alt' ? ' selected' : '') + '>Alternate</option>';
  } else if (type === 'pwmOutput') {
    opts += '<option value="Slow"' + (currentMode === 'Slow' ? ' selected' : '') + '>Slow</option>';
    opts += '<option value="Rapid"' + (currentMode === 'Rapid' ? ' selected' : '') + '>Rapid</option>';
  } else if (type === 'digitalInput') {
    opts += '<option value="Mom"' + (currentMode === 'Mom' ? ' selected' : '') + '>Momentary</option>';
    opts += '<option value="Alt"' + (currentMode === 'Alt' ? ' selected' : '') + '>Alternate</option>';
    opts += '<option value="rotate"' + (currentMode === 'rotate' ? ' selected' : '') + '>Rotate</option>';
  }
  return opts;
}
```

### 2.3 Type変更時のMode再生成

`updateFieldVisibility(ch)` 関数を拡張:
```javascript
function updateFieldVisibility(ch) {
  var type = document.getElementById('type-' + ch).value;

  // Modeドロップダウンを再生成
  var modeSelect = document.getElementById('mode-' + ch);
  var currentMode = modeSelect.value;
  modeSelect.innerHTML = getModeOptionsForType(type, currentMode);

  // 既存のフィールド可視性制御...
}
```

### 2.4 Allocation チェックボックス化

現行 (L366):
```javascript
html += '<input type="text" id="allocation-' + ch + '" value="' + alloc.join(',') + '">';
```

変更後:
```javascript
html += '<div class="allocation-checkboxes" id="allocation-' + ch + '">';
for (var j = 1; j <= 4; j++) {
  var checked = alloc.indexOf('CH' + j) >= 0 ? ' checked' : '';
  html += '<label class="alloc-cb"><input type="checkbox" value="CH' + j + '"' + checked + '> CH' + j + '</label>';
}
html += '</div>';
```

### 2.5 保存時のAllocation収集変更

`savePinSettings()` 内:
```javascript
// 現行
var allocStr = document.getElementById('allocation-' + ch).value;
var allocArr = allocStr.split(',');

// 変更後
var allocArr = [];
var allocCbs = document.querySelectorAll('#allocation-' + ch + ' input[type=checkbox]:checked');
allocCbs.forEach(function(cb) { allocArr.push(cb.value); });
```

---

## 3. タスクリスト

| # | タスク | 状態 |
|---|--------|------|
| T1 | getModeOptionsForType()関数追加 | ✅ |
| T2 | renderPinSettings()でMode動的生成 | ✅ |
| T3 | updateFieldVisibility()でMode再生成 | ✅ |
| T4 | Allocationチェックボックス化 | ✅ |
| T5 | savePinSettings()でAllocation収集変更 | ✅ |
| T6 | CSS追加（チェックボックスレイアウト） | ✅ |
| T7 | コンパイル・OTA | ✅ (74% Flash, OTA成功) |
| T8 | UI動作確認 | ⬜ 要ユーザー確認 |

---

## 4. テスト計画

| テストID | 内容 | 期待値 |
|----------|------|--------|
| UT-01 | Digital Output選択時のMode | Momentary, Alternate のみ |
| UT-02 | PWM Output選択時のMode | Slow, Rapid のみ |
| UT-03 | Digital Input選択時のMode | Momentary, Alternate, Rotate |
| UT-04 | Type変更時のMode再生成 | 適切なオプションに変わる |
| UT-05 | Digital Input以外でAllocation | 無効化（グレーアウト） |
| UT-06 | Digital InputでAllocation | CH1-CH4チェックボックス |
| UT-07 | 保存後の設定反映 | 正しく保存・読み込み |

---

## 5. GPIO18/5 電圧問題 - 最終調査結果

### 5.1 SPI干渉調査結果 (2026-01-26)

UI修正後に実施した詳細調査結果:

**コード検査:**
- is06sコード内にSPI使用は確認されない
- OLEDはI2C (Wire.begin)を使用、SPIではない
- Adafruit SSD1306ライブラリもI2Cモードで初期化

**GPIO診断API結果（全CH ON時）:**
```
CH1 GPIO18: sigOutId=256, gpioOutBit=1, gpioInBit=1, STABLE_HIGH
CH2 GPIO5:  sigOutId=256, gpioOutBit=1, gpioInBit=1, STABLE_HIGH
CH3 GPIO15: sigOutId=256, gpioOutBit=1, gpioInBit=1, STABLE_HIGH
CH4 GPIO27: sigOutId=256, gpioOutBit=1, gpioInBit=1, STABLE_HIGH
```

**結論:**
- sigOutId=256 = Simple GPIO（SPI干渉なし確定）
- gpioOutBit=1, gpioInBit=1 = レジスタ完全正常
- transitions=0 = トグル/発振なし
- **ソフトウェア側は完全に正常**

### 5.2 根本原因

**GPIO18/5の物理電圧問題（CH1=1.7V, CH2=1.3V）はハードウェア問題と結論**

考えられる原因:
1. **外部回路による負荷** - GPIO18/5に接続されたMOSFET/リレー回路がGPIO出力をプルダウン
2. **PCB配線問題** - パターンの抵抗や分圧
3. **測定点の問題** - GPIOパッド直接ではなく中間点を測定

### 5.3 推奨対応

1. GPIO18/5のピンをテスターで**パッド直接**測定
2. 外部回路を切り離した状態でESP32単体の出力電圧を測定
3. オシロスコープで波形確認（テスターの応答速度問題の排除）
4. 回路図を確認し、GPIO18/5出力段の負荷を検証
