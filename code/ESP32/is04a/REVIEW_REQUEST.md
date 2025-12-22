# is04a 実装レビュー依頼

## 概要

**デバイス名**: ar-is04a（2ch Trigger Output）
**目的**: 2チャンネル接点出力デバイス
**用途**: 自動ドア・電気錠制御、設備ON/OFF

## アーキテクチャ

### モジュール構成

```
is04a.ino (メインスケッチ)
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
    │                           ├── IOController (共通I/O制御)
    │                           └── AraneaWebUI (Web UI基底)
    │
    ├── AraneaSettings.h/cpp ──── デバイス固有デフォルト設定
    ├── Is04aKeys.h ───────────── NVSキー定数
    │
    ├── TriggerManager.h/cpp ──── 2ch入力 + 2ch出力連動管理
    │       └── IOController ──── GPIO I/O制御（共通モジュール）
    │
    ├── HttpManagerIs04a.h/cpp ── Web UI（AraneaWebUI継承）
    └── StateReporterIs04a.h/cpp ─ 状態レポート送信
```

### データフロー

```
物理入力 → IOController → TriggerManager → onInputChange()
   (GPIO5,18)      ↓               │
              デバウンス           ↓ トリガーアサイン
                             startPulse()
                                   │
    ┌─────────────────────────────┼─────────────────────────────┐
    ↓                             ↓                             ↓
IOController              StateReporter              HttpManager
(GPIO12,14)               (ローカル/クラウド)         (状態表示/API)
    │                             │
    ↓                             ↓
接点出力                  Zero3 (192.168.96.201/202)
```

---

## 新規モジュール詳細

### 1. TriggerManager

**ファイル**: `code/ESP32/is04a/TriggerManager.h/cpp`

**目的**: 2ch入力 + 2ch出力の連動管理

**GPIO割り当て**:
| GPIO | 機能 | モード | 備考 |
|------|------|--------|------|
| 5 | PHYS_IN1 | INPUT_PULLDOWN | 物理入力1 |
| 18 | PHYS_IN2 | INPUT_PULLDOWN | 物理入力2 |
| 12 | TRG_OUT1 | OUTPUT | 接点出力1 (OPEN) |
| 14 | TRG_OUT2 | OUTPUT | 接点出力2 (CLOSE) |

**パルスソース定義**:
```cpp
enum class PulseSource {
    NONE,
    CLOUD,    // クラウド（MQTT）からの命令
    MANUAL,   // 物理入力
    HTTP      // Web API
};
```

**トリガーアサイン**:
```cpp
// 入力→出力のマッピング（NVS保存）
int triggerAssign_[2];  // triggerAssign_[0]=1 → IN1→OUT1
                        // triggerAssign_[1]=2 → IN2→OUT2
```

**インターロック機構**:
```cpp
bool TriggerManager::startPulse(int outputNum, int durationMs, PulseSource source) {
    // パルス実行中チェック
    if (isPulseActive()) {
        return false;
    }

    // インターロックチェック（連続パルス防止）
    unsigned long now = millis();
    if (lastPulseEndMs_ > 0 && (now - lastPulseEndMs_) < (unsigned long)interlockMs_) {
        return false;  // インターロック期間中
    }

    // パルス開始
    outputs_[idx].pulse(durationMs);
    return true;
}
```

**エッジ検出**:
```cpp
void TriggerManager::handleInputEdges() {
    bool current = inputs_[0].isActive();
    if (current && !lastInputStable_[0]) {
        // 立ち上がりエッジ → トリガー実行
        int target = triggerAssign_[0];
        if (!isPulseActive()) {
            startPulse(target, pulseMs_, PulseSource::MANUAL);
        }
    }
    lastInputStable_[0] = current;
}
```

---

### 2. HttpManagerIs04a

**ファイル**: `code/ESP32/is04a/HttpManagerIs04a.h/cpp`

**目的**: Web UI提供（AraneaWebUI継承）

**追加APIエンドポイント**:
| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/status` | GET | 入出力状態取得 |
| `/api/pulse` | POST | パルス実行 |
| `/api/config` | GET/POST | 設定取得/変更 |
| `/api/trigger` | GET/POST | トリガーアサイン取得/変更 |

**パルスAPI仕様**:
```json
// POST /api/pulse
{
  "output": 1,      // 出力番号 (1 or 2)
  "duration": 3000  // パルス幅(ms) [10-10000]
}

// Response
{
  "ok": true,
  "output": 1,
  "duration": 3000
}
// または
{
  "ok": false,
  "output": 1,
  "duration": 3000,
  "error": "Pulse already active"  // or "Interlock active"
}
```

---

### 3. StateReporterIs04a

**ファイル**: `code/ESP32/is04a/StateReporterIs04a.h/cpp`

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
    "lacisId": "300400AABBCCDDEEFF0001",
    "mac": "AABBCCDDEEFF",
    "productType": "004",
    "productCode": "0001"
  },
  "state": {
    "Trigger1": false,
    "Trigger2": false,
    "Trigger1_Name": "OPEN",
    "Trigger2_Name": "CLOSE",
    "Input1_Physical": false,
    "Input2_Physical": false,
    "Input1_lastUpdatedAt": "2025-01-15T10:29:55Z",
    "Input2_lastUpdatedAt": "2025-01-15T10:29:55Z",
    "rssi": "-65",
    "ipaddr": "192.168.96.50",
    "SSID": "cluster1"
  },
  "meta": {
    "observedAt": "2025-01-15T10:30:00Z",
    "direct": true
  },
  "gateway": {
    "lacisId": "300400AABBCCDDEEFF0001",
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
    "lacisId": "300400AABBCCDDEEFF0001",
    "cic": "123456"
  },
  "report": {
    "lacisId": "300400AABBCCDDEEFF0001",
    "type": "aranea_ar-is04a",
    "observedAt": "2025-01-15T10:30:00Z",
    "state": {
      "Trigger1": false,
      "Trigger2": false,
      "Input1_Physical": false,
      "Input2_Physical": false
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
    trigger.sample();       // 入力サンプリング
    trigger.update();       // 出力状態更新（パルス終了チェック）
    // ... 他の処理
    delay(10);
}
```

### 2. NVSキー15文字制限

**理由**: ESP32 NVSの制限に対応

```cpp
// Is04aKeys.h
#define NVS_KEY(name, value) \
  static constexpr const char name[] = value; \
  static_assert(sizeof(value) - 1 <= 15, "NVS key '" value "' exceeds 15 chars")

namespace Is04aKeys {
  NVS_KEY(kPulseMs,     "is04_pls_ms");   // 11文字
  NVS_KEY(kInterlockMs, "is04_intrlk");   // 11文字
  // ...
}
```

### 3. INPUT_PULLDOWNの使用

**理由**: 物理スイッチ入力でHIGHをアクティブ状態とするため

```cpp
inputs_[0].begin(in1Pin, IOController::Mode::IO_IN);
inputs_[0].setPullup(false);  // INPUT_PULLDOWN（HIGHでアクティブ）
```

### 4. インターロック機構

**理由**: 自動ドア・電気錠の連続動作防止

```cpp
// デフォルト200ms
// パルス終了後、次のパルスまで最低200ms待機
if (lastPulseEndMs_ > 0 && (now - lastPulseEndMs_) < (unsigned long)interlockMs_) {
    return false;
}
```

---

## is05aとの差異

| 項目 | is04a | is05a |
|------|-------|-------|
| 入力チャンネル数 | 2ch | 8ch |
| 出力チャンネル数 | 2ch | 2ch (ch7/ch8のみ) |
| I/Oモード切替 | 固定 | ch7/ch8は動的切替可 |
| トリガーアサイン | あり（入力→出力マッピング） | なし |
| インターロック | あり | なし |
| Webhook | なし | あり（Discord/Slack/Generic） |

---

## ビルド情報

```
パーティション: min_spiffs（OTA対応）
Flash使用量: 1,285,113 / 1,966,080 bytes (65%)
RAM使用量:   53,584 / 327,680 bytes (16%)
```

---

## レビュー観点

### 必須確認事項

1. **TriggerManagerのインターロック安全性**
   - パルス実行中の排他制御は適切か
   - インターロック時間の計算は正しいか

2. **エッジ検出の正確性**
   - 立ち上がりエッジのみでトリガー発火は適切か
   - チャタリング対策（デバウンス）は十分か

3. **入力→出力アサインのNVS永続化**
   - setTriggerAssign()でのNVS保存は正しいか
   - 起動時のloadConfig()で正しく復元されるか

4. **NVSキー長の検証**
   - 全キーが15文字以内か
   - static_assertによるコンパイル時チェックは十分か

### 推奨確認事項

5. **パルス終了コールバックの競合**
   - 2つの出力が同時にパルス終了した場合の動作

6. **Web UIのセキュリティ**
   - Basic認証は正しく機能するか
   - CORS設定

7. **起動時の出力状態**
   - 電源投入時に出力がLOWで初期化されるか
   - 意図しない動作を防げるか

---

## テスト項目

### 単体テスト

- [ ] TriggerManager: 入力デバウンス動作
- [ ] TriggerManager: パルス出力動作
- [ ] TriggerManager: インターロック動作
- [ ] TriggerManager: トリガーアサイン変更
- [ ] StateReporter: ローカル/クラウド送信

### 統合テスト

- [ ] 起動シーケンス（WiFi→NTP→登録→初期化）
- [ ] 物理入力→トリガー→パルス出力→状態送信
- [ ] Web UI経由のパルス実行
- [ ] Web UI経由の設定変更
- [ ] OTA更新

### 耐久テスト

- [ ] 連続稼働（24時間）
- [ ] 頻繁なパルス実行
- [ ] WiFi切断→再接続

---

## 関連ドキュメント

- `code/ESP32/is04a/design/DESIGN.md` - 設計書
- `code/ESP32/is04a/design/MODULE_ADAPTATION_PLAN.md` - モジュール適応計画
- `docs/is04a/PENDING_TEST.md` - テスト待ち事項

---

**作成日**: 2025-12-22
**作成者**: Claude Code
**コミット**: e1a3a2e
