# is05a 実装レビュー依頼

## 概要

**デバイス名**: ar-is05a（8ch Detector）
**目的**: 8チャンネルリードスイッチ/接点入力デバイス
**用途**: 窓・ドアの開閉状態検知、設備状態監視

## アーキテクチャ

### モジュール構成

```
is05a.ino (メインスケッチ)
    │
    ├── AraneaGlobalImporter.h ─┬── SettingManager (NVS設定管理)
    │                           ├── DisplayManager (OLED表示)
    │                           ├── WiFiManager (WiFi接続)
    │                           ├── NtpManager (時刻同期)
    │                           ├── LacisIDGenerator (ID生成)
    │                           ├── AraneaRegister (デバイス登録)
    │                           ├── OtaManager (OTA更新)
    │                           ├── HttpOtaManager (HTTP OTA)
    │                           ├── Operator (状態機械)
    │                           └── AraneaWebUI (Web UI基底)
    │
    ├── AraneaSettings.h/cpp ──── デバイス固有デフォルト設定
    ├── Is05aKeys.h ───────────── NVSキー定数
    │
    ├── ChannelManager.h/cpp ──── 8ch入力管理
    │       └── IOController ──── GPIO I/O制御（共通モジュール）
    │
    ├── WebhookManager.h/cpp ──── Webhook通知
    ├── HttpManagerIs05a.h/cpp ── Web UI（AraneaWebUI継承）
    └── StateReporterIs05a.h/cpp ─ 状態レポート送信
```

### データフロー

```
GPIO入力 → IOController → ChannelManager → onChannelChanged()
                                                  │
                    ┌─────────────────────────────┼─────────────────────────────┐
                    ↓                             ↓                             ↓
            StateReporter                  WebhookManager              HttpManager
            (ローカル/クラウド)              (Discord/Slack)            (状態更新)
                    │                             │
                    ↓                             ↓
            Zero3 (192.168.96.201/202)    外部Webhookサービス
```

---

## 新規モジュール詳細

### 1. IOController（共通モジュール）

**ファイル**: `code/ESP32/global/src/IOController.h/cpp`

**目的**: GPIO入出力の統一的な制御を提供

**主要機能**:
- 入力/出力モードの動的切替
- 可変デバウンス処理（5-10000ms）
- 非ブロッキングパルス出力
- 論理反転対応（アクティブLOW/HIGH）

**設計判断**:

```cpp
// enum値の命名（Arduinoマクロとの衝突回避）
enum class Mode {
    IO_IN,    // INPUT/OUTPUTはArduinoマクロで定義済み
    IO_OUT
};
```

**デバウンス実装**:
```cpp
void IOController::sample() {
    // 1msサンプリング間隔
    if (now - lastSampleMs_ < 1) return;

    if (current == rawState_) {
        stableCount_++;
        // stableCountがdebounceMs_以上になったら確定
        if (stableCount_ >= debounceMs_ && current != stableState_) {
            stableState_ = current;
            changed_ = true;
            if (onChangeCallback_) {
                onChangeCallback_(isActive());
            }
        }
    } else {
        rawState_ = current;
        stableCount_ = 0;  // 値が変わったらリセット
    }
}
```

**モード切替の安全チェック**:
```cpp
bool IOController::isSafeToSwitch(int pin) const {
    // ストラッピングピン（起動に影響）
    if (pin == 0 || pin == 2 || pin == 15) return false;
    // 入力専用ピン
    if (pin >= 34 && pin <= 39) return false;
    // フラッシュ使用ピン
    if (pin >= 6 && pin <= 11) return false;
    return true;
}
```

---

### 2. ChannelManager

**ファイル**: `code/ESP32/is05a/ChannelManager.h/cpp`

**目的**: 8チャンネルの入力を統合管理

**チャンネル構成**:
| チャンネル | GPIO | モード | 備考 |
|-----------|------|--------|------|
| ch1 | 4 | 入力専用 | |
| ch2 | 5 | 入力専用 | |
| ch3 | 13 | 入力専用 | |
| ch4 | 14 | 入力専用 | |
| ch5 | 16 | 入力専用 | |
| ch6 | 17 | 入力専用 | |
| ch7 | 18 | I/O切替可 | 出力モードでパルス出力可能 |
| ch8 | 19 | I/O切替可 | 出力モードでパルス出力可能 |

**NVS設定項目**（チャンネルごと）:
```cpp
// Is05aChannelKeys による動的キー生成
static String getChannelKey(int ch, const char* suffix) {
    return "ch" + String(ch) + "_" + String(suffix);
}
// 例: "ch1_pin", "ch1_name", "ch1_mean", "ch1_deb", "ch1_inv"
```

**状態変化通知**:
```cpp
void ChannelManager::initChannels() {
    for (int i = 0; i < NUM_CHANNELS; i++) {
        channels_[i].onStateChange([this, ch](bool active) {
            lastUpdatedAt_[idx] = ntp_->getIso8601();
            changed_ = true;
            lastChangedCh_ = ch;
            if (onChangeCallback_) {
                onChangeCallback_(ch, active);
            }
        });
    }
}
```

---

### 3. WebhookManager

**ファイル**: `code/ESP32/is05a/WebhookManager.h/cpp`

**目的**: 外部サービスへのWebhook通知

**対応プラットフォーム**:
- Discord（embeds形式）
- Slack（attachments形式）
- Generic（JSON POST）

**Discordペイロード例**:
```json
{
  "embeds": [{
    "title": "is05a State Change",
    "color": 65280,
    "fields": [
      {"name": "Device", "value": "ar-is05a-AABBCC", "inline": true},
      {"name": "Channel", "value": "ch1 (窓センサー)", "inline": true},
      {"name": "State", "value": "open", "inline": true}
    ],
    "timestamp": "2025-01-15T10:30:00Z"
  }]
}
```

**Slackペイロード例**:
```json
{
  "attachments": [{
    "color": "#00ff00",
    "title": "is05a State Change",
    "fields": [
      {"title": "Device", "value": "ar-is05a-AABBCC", "short": true},
      {"title": "Channel", "value": "ch1", "short": true},
      {"title": "State", "value": "open", "short": true}
    ],
    "ts": 1705315800
  }]
}
```

---

### 4. HttpManagerIs05a

**ファイル**: `code/ESP32/is05a/HttpManagerIs05a.h/cpp`

**目的**: Web UI提供（AraneaWebUI継承）

**追加タブ**:
- Channels: 8ch状態表示、ch7/ch8モード切替、パルス出力
- Webhook: Discord/Slack/Generic URL設定、テスト送信

**追加APIエンドポイント**:
| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/channels` | GET | 全チャンネル状態取得 |
| `/api/channel` | GET/POST | 個別チャンネル設定 |
| `/api/pulse` | POST | パルス出力（ch7/ch8） |
| `/api/webhook/config` | GET/POST | Webhook設定 |
| `/api/webhook/test` | POST | テスト送信 |

---

### 5. StateReporterIs05a

**ファイル**: `code/ESP32/is05a/StateReporterIs05a.h/cpp`

**目的**: 状態レポートの構築と送信

**送信先**:
1. ローカルリレー Primary (192.168.96.201)
2. ローカルリレー Secondary (192.168.96.202)
3. クラウド (AraneaSettings::getCloudUrl())

**ローカルペイロード構造**:
```json
{
  "observedAt": "2025-01-15T10:30:00Z",
  "sensor": {
    "lacisId": "300500AABBCCDDEEFF0001",
    "mac": "AABBCCDDEEFF",
    "productType": "005",
    "productCode": "0001"
  },
  "state": {
    "ch1": "open",
    "ch1_lastUpdatedAt": "2025-01-15T10:29:55Z",
    "ch2": "close",
    ...
    "rssi": "-65",
    "ipaddr": "192.168.96.50",
    "SSID": "cluster1"
  },
  "meta": {
    "observedAt": "2025-01-15T10:30:00Z",
    "direct": true
  },
  "gateway": {
    "lacisId": "300500AABBCCDDEEFF0001",
    "ip": "192.168.96.50",
    "rssi": -65
  }
}
```

**クラウドペイロード構造**:
```json
{
  "auth": {
    "tid": "T2025120608261484221",
    "lacisId": "300500AABBCCDDEEFF0001",
    "cic": "123456"
  },
  "report": {
    "lacisId": "300500AABBCCDDEEFF0001",
    "type": "aranea_ar-is05a",
    "observedAt": "2025-01-15T10:30:00Z",
    "state": {
      "ch1": "open",
      "ch2": "close",
      ...
    }
  }
}
```

---

## 設計上の判断事項

### 1. シングルタスク設計

**理由**: ESP32のマルチタスクによる複雑性とデバッグ困難性を回避

```cpp
void loop() {
    ota.handle();           // OTA処理（最優先）
    httpMgr.handle();       // HTTP処理
    channels.sample();      // 入力サンプリング
    channels.update();      // パルス終了チェック
    // ... 他の処理
    delay(10);
}
```

### 2. NVSキー15文字制限

**理由**: ESP32 NVSの制限に対応

```cpp
// Is05aKeys.h
static constexpr const char* kHeartbeat = "is05a_hb";  // 8文字
static constexpr const char* kBootGrace = "is05a_bg";  // 8文字
static_assert(sizeof("is05a_hb") <= 16, "NVS key too long");
```

### 3. F()マクロによるHTML/JS文字列

**理由**: Raw string literal (`R"(...)"`) でエンコーディング問題が発生したため

```cpp
// Before (問題あり)
return R"(<button class="tab-btn">Channels</button>)";

// After (修正版)
html += F("<button class=\"tab-btn\">Channels</button>");
```

### 4. IOController::Mode enum命名

**理由**: ArduinoのINPUT/OUTPUTマクロとの衝突回避

```cpp
// esp32-hal-gpio.h で定義済み
#define INPUT  0x01
#define OUTPUT 0x03

// 衝突を避けるため
enum class Mode { IO_IN, IO_OUT };
```

---

## ビルド情報

```
パーティション: min_spiffs（OTA対応）
Flash使用量: 1,290,877 / 1,966,080 bytes (65%)
RAM使用量:   54,384 / 327,680 bytes (16%)
```

---

## レビュー観点

### 必須確認事項

1. **IOControllerのモード切替安全性**
   - transitionToInput()/transitionToOutput()の遷移順序は適切か
   - パルス実行中のモード切替禁止は正しく機能するか

2. **デバウンス実装の妥当性**
   - 1msサンプリング間隔は適切か
   - stableCountによる判定ロジックは正しいか

3. **Webhook送信のエラーハンドリング**
   - タイムアウト設定は適切か
   - 連続失敗時の挙動は問題ないか

4. **NVSキー長の検証**
   - 全キーが15文字以内か
   - static_assertによるコンパイル時チェックは十分か

### 推奨確認事項

5. **メモリ使用量の最適化余地**
   - String連結による断片化リスク
   - F()マクロの使用箇所は適切か

6. **Web UIのセキュリティ**
   - API認証の有無
   - CORS設定

7. **起動猶予期間(bootGrace)の妥当性**
   - デフォルト5000msは適切か
   - 猶予期間中の動作は期待通りか

---

## テスト項目

### 単体テスト

- [ ] IOController: 入力デバウンス動作
- [ ] IOController: 出力パルス動作
- [ ] IOController: モード切替（ch7/ch8）
- [ ] ChannelManager: 8ch同時サンプリング
- [ ] WebhookManager: Discord/Slack/Generic送信

### 統合テスト

- [ ] 起動シーケンス（WiFi→NTP→登録→チャンネル初期化）
- [ ] 状態変化→レポート送信→Webhook通知
- [ ] Web UI経由の設定変更
- [ ] OTA更新

### 耐久テスト

- [ ] 連続稼働（24時間）
- [ ] 頻繁な状態変化（チャタリング耐性）
- [ ] WiFi切断→再接続

---

## 関連ドキュメント

- `docs/is05a/DESIGN.md` - 設計書
- `docs/is05a/MODULE_ADAPTATION_PLAN.md` - モジュール適応計画
- `docs/global/IOController_VERIFICATION.md` - I/O切替検証設計

---

**作成日**: 2025-01-15
**作成者**: Claude Code
**コミット**: e7f2063
