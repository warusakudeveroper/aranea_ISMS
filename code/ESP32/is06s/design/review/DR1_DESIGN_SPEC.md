# デザイナーレビュー#1 詳細設計仕様書

**作成日**: 2026-01-25
**対象**: IS06S aranea_ar-is06s
**準拠**: The_golden_rules.md

---

## 大原則宣言

本設計は以下の原則に従う:
1. SSoT遵守 - PIN状態の唯一のソースはIs06PinManager
2. SOLID原則 - 単一責任（UI表示/API処理/PIN制御の分離）
3. MECE - 14項目を漏れなく重複なく対応
4. アンアンビギュアス - 全項目に明確な完了条件を定義
5. 情報等価 - disabled状態でも情報は等価に表示
6. 現場猫禁止 - 全項目にテストを実施
7. 棚上げ禁止 - 14項目全て本フェーズで対応

---

## MECE分析: 14項目の分類

### A. WebUI改修 (フロントエンド)
| # | 項目 | 修正ファイル |
|---|------|-------------|
| 3 | モーメンタリUI復帰 | HttpManagerIs06s.cpp (JS) |
| 5 | ON/OFFカード半透明化見直し | HttpManagerIs06s.cpp (JS) |
| 6 | disabled操作禁止 | HttpManagerIs06s.cpp (JS) |
| 7 | Low=ONトグル修正 | HttpManagerIs06s.cpp (JS) |
| 8 | PIN Controlレイアウト改善 | HttpManagerIs06s.cpp (JS/CSS) |
| 9 | PIN Settings条件付き入力 | HttpManagerIs06s.cpp (JS) |

### B. ファームウェア改修 (バックエンド)
| # | 項目 | 修正ファイル |
|---|------|-------------|
| 1 | リブート時PIN状態 | is06s.ino, Is06PinManager |
| 2 | オフライン時Input→Output | Is06PinManager (確認のみ) |
| 4 | デバイス操作フィードバック | HttpManagerIs06s (SSE検討) |
| 10 | PINglobalデフォルト拡張 | HttpManagerIs06s.cpp, is06s.ino |
| 11 | デバウンス・同期検証 | Is06PinManager, HttpManagerIs06s |
| 12 | WDT確認 | is06s.ino |
| 13 | RSSI閾値再接続 | is06s.ino |
| 14 | ヒープ監視再起動 | is06s.ino |

---

## 詳細設計

### DR1-01: リブート時PIN状態

**要件**: リブート後のPIN状態をどう復元するか

**設計判断**:
- セキュリティデバイス（電気錠）: リブート後は**安全状態（OFF/施錠）**
- 照明: mobes2.0からの同期を待つ
- **結論**: リブート後はPIN状態をOFFで初期化し、mobes2.0/MQTTからの同期を待つ

**実装**:
```cpp
// Is06PinManager::begin() - PIN状態は常にOFFで開始
for (int i = 1; i <= 6; i++) {
  pinStates_[i-1] = 0;  // 常にOFF
  // 設定のみNVSから復元
}
```

**完了条件**: リブート後、全PINがOFF状態で起動することをAPI確認

---

### DR1-02: オフライン時Input→Output連動

**要件**: MQTT未接続でもallocation連動が動作すること

**現状確認**: Is06PinManager::handleInputChange()で実装済み

**確認項目**:
```cpp
// Is06PinManager.cpp
void Is06PinManager::handleInputChange(int inputChannel) {
  // WiFi/MQTT状態に依存せずローカルで連動
}
```

**完了条件**: WiFi切断状態でCH5物理入力→CH1-4出力連動を確認

---

### DR1-03: モーメンタリUI復帰

**要件**: Momモード操作後、validity時間経過でUIが自動OFF表示

**設計**:
```javascript
function togglePin(ch) {
  fetch('/api/pin/' + ch + '/toggle', {method: 'POST'})
    .then(r => r.json())
    .then(data => {
      if (data.ok) {
        // 楽観的更新: 即座に表示変更
        updatePinDisplay(ch, data.state);

        // Momモードの場合、validity後に再取得
        if (data.actionMode === 'Mom' && data.state === 1) {
          setTimeout(() => loadPinStates(), data.validity || 3000);
        }
      }
    });
}
```

**完了条件**: Chrome UIでMomピンON→validity秒後に自動OFF表示

---

### DR1-04: デバイス操作フィードバック

**要件**: 他クライアントの操作をリアルタイム反映

**設計**: ポーリング方式（SSEは将来検討）
```javascript
// PIN Controlタブがアクティブな間、3秒ごとに状態取得
let pinPollInterval = null;

function startPinPolling() {
  pinPollInterval = setInterval(loadPinStates, 3000);
}

function stopPinPolling() {
  if (pinPollInterval) {
    clearInterval(pinPollInterval);
    pinPollInterval = null;
  }
}
```

**完了条件**: 2つのブラウザで同時接続し、一方の操作が他方に3秒以内に反映

---

### DR1-05: ON/OFFカード半透明化見直し

**要件**: PIN ON/OFFとenabled/disabledの意味を明確に分離

**設計**:
- **enabled/disabled**: カード全体のopacity制御
- **state ON/OFF**: ボタン/スライダーの表示状態のみ

```css
.pin-card.disabled {
  opacity: 0.5;
  pointer-events: none;
}
.pin-card .btn-toggle.on { background: #4CAF50; }
.pin-card .btn-toggle.off { background: #9E9E9E; }
```

**完了条件**: disabled PINはグレーアウト、enabled PINの状態変化はボタン色のみ変更

---

### DR1-06: disabled操作禁止

**要件**: enabled=falseのPINはUI操作不可

**設計**:
```javascript
function renderPinControls(pins) {
  // ...
  if (!p.enabled) {
    ctrl = '<button disabled class="btn-toggle off">' + label + '</button>';
  }
  // カード全体にdisabledクラス追加
  html += '<div class="pin-card' + (en ? '' : ' disabled') + '">';
}
```

```css
.pin-card.disabled input,
.pin-card.disabled button,
.pin-card.disabled .switch {
  pointer-events: none;
  cursor: not-allowed;
}
```

**完了条件**: disabled PINのトグル/スライダー操作が無効であることをChrome確認

---

### DR1-07: Low=ONトグル修正

**要件**: digitalInputの状態表示を適切にする

**設計**:
- digitalInputのトグルは**読み取り専用表示**
- 状態ラベル: `stateName`に従う、なければ"HIGH"/"LOW"

```javascript
if (tp === 'digitalInput') {
  var inputLabel = getStateLabel(stn, st, 'HIGH', 'LOW');
  ctrl = '<span class="input-state ' + (st ? 'high' : 'low') + '">' + inputLabel + '</span>';
  // トグルスイッチは表示せず、状態ラベルのみ
}
```

**完了条件**: digitalInputにトグルスイッチが表示されず、状態ラベルのみ表示

---

### DR1-08: PIN Controlレイアウト改善

**要件**:
- CH番号、name、type、mode情報を整理表示
- モバイル対応
- 改行なしのレイアウト

**設計**: カード型からテーブル/リスト型へ変更
```html
<div class="pin-row">
  <div class="pin-info">
    <span class="pin-ch">CH1</span>
    <span class="pin-name">メインリレー</span>
  </div>
  <div class="pin-meta">
    <span class="pin-type">DIGITAL</span>
    <span class="pin-mode">Mom 3000ms</span>
  </div>
  <div class="pin-control">
    <button class="btn-state">施錠</button>
  </div>
  <div class="pin-enable">
    <label class="switch">...</label>
  </div>
</div>
```

**レイアウト定義**:
```
| CH | Name        | Type/Mode      | Control    | Enable |
|----|-------------|----------------|------------|--------|
| 1  | メインリレー | DIGITAL Mom 3s | [施錠]     | [○━━] |
| 2  | 調光LED     | PWM Slow       | [===50%==] | [○━━] |
```

**完了条件**: Chrome/モバイルで表示崩れなし、全情報が1行に収まる

---

### DR1-09: PIN Settings条件付き入力

**要件**: PINタイプによって不要な項目をグレーアウト

**条件マトリクス**:
| 項目 | digitalOutput | pwmOutput | digitalInput | disabled |
|------|--------------|-----------|--------------|----------|
| validity | Mom時のみ | - | Mom時のみ | - |
| debounce | ✓ | - | ✓ | - |
| rateOfChange | - | ✓ | - | - |
| allocation | - | - | ✓ | - |
| stateName | ✓ | ✓ | - | - |
| expiryDate | ✓ | ✓ | - | - |

**実装**:
```javascript
function updateFieldVisibility(ch) {
  var type = document.getElementById('type-' + ch).value;
  var mode = document.getElementById('mode-' + ch).value;

  // Validity: digitalOutput/Input の Mom モードのみ
  var validityEnabled = (type === 'digitalOutput' || type === 'digitalInput') && mode === 'Mom';
  document.getElementById('validity-' + ch).disabled = !validityEnabled;

  // RateOfChange: pwmOutput のみ
  document.getElementById('rateOfChange-' + ch).disabled = (type !== 'pwmOutput');

  // Allocation: digitalInput のみ
  document.getElementById('allocation-' + ch).disabled = (type !== 'digitalInput');

  // stateName: digitalOutput/pwmOutput のみ
  document.getElementById('stateName-' + ch).disabled = (type === 'digitalInput' || type === 'disabled');
}
```

**完了条件**: タイプ変更時に不要フィールドがグレーアウトされることをChrome確認

---

### DR1-10: PINglobalデフォルト値拡張

**要件**: DesignerInstructions.mdの全項目をPINglobalに追加

**追加項目**:
- defaultExpiry (日単位)
- expiryEnabled デフォルト

**実装**:
```cpp
// AraneaSettingsDefaults.h に追加
const char* DEFAULT_EXPIRY_DAYS = "1";       // デフォルト有効期限（日）

// handleSettingsGet() に追加
pinGlobal["expiryDays"] = settings_->getInt("g_expiryDays", 1);
```

**完了条件**: /api/settings に expiryDays が含まれること

---

### DR1-11: デバウンス・同期検証

**要件**: 連打、同時操作、リブート中操作の耐性

**テスト項目**:
1. ボタン連打（100ms間隔で10回）→ デバウンスで1回のみ処理
2. 同時API呼び出し（5並列）→ 全て処理完了
3. リブート中操作 → 適切にエラー応答

**確認ポイント**:
```cpp
// Is06PinManager::setPinState()
if (now - lastCommandTime_[ch] < setting.debounce) {
  return false;  // デバウンス中は拒否
}
```

**完了条件**: テストスクリプトで連打テストpass

---

### DR1-12: WDT確認

**要件**: パニック/ハング時にWDTが正常動作

**確認ポイント**:
```cpp
// loop() 内で yield() または delay() が呼ばれているか
void loop() {
  // ... 処理 ...
  delay(1);  // ✓ WDT feed
}
```

**完了条件**: loop()内にdelay()またはyield()があることをコード確認

---

### DR1-13: RSSI閾値再接続

**要件**: WiFi信号弱化時に自動再接続

**設計**:
```cpp
// is06s.ino に追加
const int RSSI_RECONNECT_THRESHOLD = -85;  // dBm
const unsigned long RSSI_CHECK_INTERVAL = 30000;  // 30秒
unsigned long lastRssiCheck = 0;

void checkWiFiQuality() {
  if (WiFi.status() == WL_CONNECTED) {
    int rssi = WiFi.RSSI();
    if (rssi < RSSI_RECONNECT_THRESHOLD) {
      Serial.printf("RSSI weak (%d dBm), reconnecting...\n", rssi);
      wifi.disconnect();
      wifi.connectWithSettings(&settings);
    }
  }
}

// loop() に追加
if (now - lastRssiCheck >= RSSI_CHECK_INTERVAL) {
  lastRssiCheck = now;
  checkWiFiQuality();
}
```

**完了条件**: RSSI < -85dBm で再接続が発生することをログ確認

---

### DR1-14: ヒープ監視再起動

**要件**: ヒープ枯渇時に自動再起動

**設計**:
```cpp
// is06s.ino に追加
const int HEAP_CRITICAL_THRESHOLD = 20000;  // 20KB
const unsigned long HEAP_CHECK_INTERVAL = 10000;  // 10秒
unsigned long lastHeapCheck = 0;

void checkHeapHealth() {
  int freeHeap = ESP.getFreeHeap();
  if (freeHeap < HEAP_CRITICAL_THRESHOLD) {
    Serial.printf("CRITICAL: Heap low (%d bytes), rebooting...\n", freeHeap);
    ESP.restart();
  }
}

// loop() に追加
if (now - lastHeapCheck >= HEAP_CHECK_INTERVAL) {
  lastHeapCheck = now;
  checkHeapHealth();
}
```

**完了条件**: ヒープ監視ログが10秒ごとに出力されることを確認

---

## タスクリスト

| ID | タスク | 依存 | 優先度 |
|----|--------|------|--------|
| T01 | DR1-01 リブート状態確認・修正 | - | P1 |
| T02 | DR1-02 オフライン連動確認 | - | P1 |
| T03 | DR1-12 WDT確認 | - | P1 |
| T04 | DR1-13 RSSI監視実装 | - | P1 |
| T05 | DR1-14 ヒープ監視実装 | - | P1 |
| T06 | DR1-06 disabled操作禁止 | - | P1 |
| T07 | DR1-08 レイアウト改善 | T06 | P1 |
| T08 | DR1-07 Input表示修正 | T07 | P2 |
| T09 | DR1-05 透明化見直し | T07 | P2 |
| T10 | DR1-03 Mom自動復帰 | T07 | P2 |
| T11 | DR1-04 ポーリング実装 | T07 | P2 |
| T12 | DR1-09 条件付き入力 | - | P2 |
| T13 | DR1-10 PINglobal拡張 | - | P3 |
| T14 | DR1-11 デバウンステスト | T01-T13 | P3 |

---

## テスト計画

### 自動テスト (test_dr1.sh)
1. API応答確認（全エンドポイント）
2. デバウンステスト（連打）
3. 同時アクセステスト
4. PIN状態整合性テスト

### Chrome UIテスト (手動)
1. PIN Controlレイアウト確認
2. disabled PIN操作不可確認
3. Momモード自動復帰確認
4. ポーリング反映確認
5. PIN Settings条件付き入力確認
6. モバイル表示確認

### ファームウェアテスト
1. リブート後PIN状態確認
2. オフライン連動確認（WiFi切断）
3. RSSI監視ログ確認
4. ヒープ監視ログ確認
5. WDT動作確認

---

## MECE確認

**漏れ**: なし（14項目全てに設計・実装・テストを定義）
**重複**: なし（各項目が独立した責務を持つ）

本設計はMECEである。

