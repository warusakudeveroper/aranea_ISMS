# IS06S タスクリスト

**製品コード**: AR-IS06S
**作成日**: 2025/01/23
**最終更新**: 2025/01/23

---

## 1. タスク概要

### 1.1 フェーズ構成

| フェーズ | 内容 | 依存 |
|---------|------|------|
| P0 | 基盤構築（GPIO、WiFi、NVS） | なし |
| P1 | コア機能（PIN制御、Web UI） | P0完了 |
| P2 | 通信機能（MQTT、状態送信） | P1完了 |
| P3 | 拡張機能（OTA、表示、System PIN） | P2完了 |
| P4 | 統合テスト・調整 | P3完了 |

### 1.2 進捗サマリ

| フェーズ | タスク数 | 完了 | 進行中 | 未着手 |
|---------|---------|------|--------|--------|
| P0 | 8 | 7 | 0 | 1 |
| P1 | 10 | 7 | 0 | 3 |
| P2 | 6 | 0 | 0 | 6 |
| P3 | 5 | 0 | 0 | 5 |
| P4 | 4 | 0 | 0 | 4 |
| **合計** | **33** | **14** | **0** | **19** |

---

## 2. フェーズ0: 基盤構築

### 2.1 タスク一覧

| ID | タスク | 依存 | ステータス | 担当 |
|----|--------|------|----------|------|
| P0-1 | プロジェクトスケルトン作成 | - | ✅ 完了 | Claude |
| P0-2 | AraneaGlobalImporter.h作成 | P0-1 | ✅ 完了 | Claude |
| P0-3 | GPIO初期化実装 | P0-2 | ✅ 完了 | Claude |
| P0-4 | WiFiManager統合 | P0-2 | ✅ 完了 | Claude |
| P0-5 | SettingManager統合（NVSキー定義） | P0-2 | ✅ 完了 | Claude |
| P0-6 | 基盤動作確認（WiFi接続・NVS読み書き） | P0-3,4,5 | ⬜ 未着手 | - |
| **P0-7** | **AraneaSettingsDefaults実装** | P0-2 | ✅ 完了 | Claude |
| **P0-8** | **PINglobal NVS初期化実装** | P0-5,7 | ✅ 完了 | Claude |

### 2.2 詳細タスク

#### P0-1: プロジェクトスケルトン作成

**成果物**:
- `is06s/is06s.ino` (空のsetup/loop)
- `is06s/data/` ディレクトリ

**受け入れ条件**:
- [ ] コンパイルが通ること
- [ ] min_SPIFFSパーティション指定

#### P0-2: AraneaGlobalImporter.h作成

**成果物**:
- `is06s/AraneaGlobalImporter.h`

**内容**:
```cpp
#ifndef ARANEA_GLOBAL_IMPORTER_H
#define ARANEA_GLOBAL_IMPORTER_H

#include "../global/src/WiFiManager.h"
#include "../global/src/SettingManager.h"
#include "../global/src/DisplayManager.h"
#include "../global/src/NtpManager.h"
#include "../global/src/MqttManager.h"
#include "../global/src/AraneaWebUI.h"
#include "../global/src/HttpOtaManager.h"
#include "../global/src/Operator.h"
#include "../global/src/LacisIDGenerator.h"
#include "../global/src/AraneaRegister.h"

#endif
```

**受け入れ条件**:
- [ ] 全共通モジュールがインクルード可能
- [ ] コンパイルエラーなし

#### P0-3: GPIO初期化実装

**成果物**:
- `is06s.ino`にGPIO初期化コード追加

**GPIO設定**:
| GPIO | 機能 | モード |
|------|------|--------|
| 18 | CH1 | OUTPUT |
| 5 | CH2 | OUTPUT |
| 15 | CH3 | OUTPUT |
| 27 | CH4 | OUTPUT |
| 14 | CH5 | INPUT_PULLDOWN または OUTPUT |
| 12 | CH6 | INPUT_PULLDOWN または OUTPUT |
| 25 | Reconnect | INPUT_PULLDOWN |
| 26 | Reset | INPUT_PULLDOWN |
| 21 | I2C SDA | - |
| 22 | I2C SCL | - |

**受け入れ条件**:
- [ ] 全GPIO初期化完了
- [ ] シリアル出力で確認可能

#### P0-4: WiFiManager統合

**受け入れ条件**:
- [ ] 設定SSIDに接続成功
- [ ] APモード移行動作確認
- [ ] IPアドレス取得・表示

#### P0-5: SettingManager統合（NVSキー定義）

**定義するNVSキー**: IS06S_ModuleAdaptationPlan.md 2.2節参照

**受け入れ条件**:
- [ ] 全NVSキーの読み書き動作確認
- [ ] デフォルト値が正しく適用されること

#### P0-6: 基盤動作確認

**確認項目**:
- [ ] WiFi接続成功（シリアル出力）
- [ ] NVS読み書き成功（シリアル出力）
- [ ] GPIO初期状態確認

#### P0-7: AraneaSettingsDefaults実装

**成果物**:
- `AraneaSettingsDefaults.h`

**内容**:
```cpp
namespace AraneaSettingsDefaults {
    const char* DIGITAL_VALIDITY = "3000";
    const char* DIGITAL_DEBOUNCE = "3000";
    const char* DIGITAL_ACTION_MODE = "Mom";
    const char* PWM_RATE_OF_CHANGE = "4000";
    const char* PWM_ACTION_MODE = "Slow";
    const char* DEFAULT_EXPIRY = "1";
    const char* DEFAULT_EXPIRY_ENABLED = "false";
}
```

**受け入れ条件**:
- [ ] ハードコード値が全てnamespaceに集約されていること
- [ ] コンパイル通過

#### P0-8: PINglobal NVS初期化実装

**成果物**:
- SettingManager初期化時にPINglobal値をNVSに書き込む処理

**受け入れ条件**:
- [ ] 初回起動時にAraneaSettingsDefaultsからNVSに書き込み
- [ ] 2回目以降は既存NVS値を維持
- [ ] NVSのpinglobe namespaceに全キー存在確認

---

## 3. フェーズ1: コア機能

### 3.1 タスク一覧

| ID | タスク | 依存 | ステータス | 担当 |
|----|--------|------|----------|------|
| P1-1 | Is06PinManager基本構造実装 | P0-8 | ✅ 完了 | Claude |
| **P1-1a** | **PIN enabled制御実装** | P1-1 | ✅ 完了 | Claude |
| **P1-1b** | **PINglobal参照チェーン実装** | P1-1 | ✅ 完了 | Claude |
| P1-2 | Digital Output実装（Momentary） | P1-1a,1b | ✅ 完了 | Claude |
| P1-3 | Digital Output実装（Alternate） | P1-2 | ✅ 完了 | Claude |
| P1-4 | PWM Output実装（LEDC） | P1-1a,1b | ✅ 完了 | Claude |
| P1-5 | Digital Input実装（連動） | P1-2,4 | ✅ 完了 | Claude |
| P1-6 | HttpManagerIs06s基本構造実装 | P0-8 | ⬜ 未着手 | - |
| P1-7 | PIN操作API実装 | P1-5,6 | ⬜ 未着手 | - |
| P1-8 | PIN操作Web UI実装 | P1-7 | ⬜ 未着手 | - |

### 3.2 詳細タスク

#### P1-1: Is06PinManager基本構造実装

**成果物**:
- `Is06PinManager.h`
- `Is06PinManager.cpp`

**実装内容**:
- PinType, ActionMode列挙
- PinSetting, PinState構造体
- begin(), update()メソッド
- コールバック登録

**受け入れ条件**:
- [ ] クラス定義完了
- [ ] コンパイル通過

#### P1-1a: PIN enabled制御実装

**実装内容**:
- isPinEnabled(), setPinEnabled()メソッド
- enabled=false時のコマンド拒否ロジック
- NVS ch{n}_enabledキー読み書き

**受け入れ条件**:
- [ ] enabled=false時にsetPinState()がfalseを返すこと
- [ ] enabled状態がNVSに保存されること
- [ ] 起動時にNVSからenabled状態を復元すること

#### P1-1b: PINglobal参照チェーン実装

**実装内容**:
- getEffectiveValidity(), getEffectiveDebounce()等のメソッド
- PINSettings → PINglobal → araneaSettings の参照チェーン
- resolveDefaultInt(), resolveDefaultString()ヘルパー

**受け入れ条件**:
- [ ] PINSettings値が優先されること
- [ ] PINSettings未設定時にPINglobal値が使用されること
- [ ] ハードコード値がIs06PinManager内に存在しないこと

#### P1-2: Digital Output実装（Momentary）

**実装内容**:
- setPinState() でパルス開始
- update() でパルス終了判定
- 状態変化コールバック呼び出し

**受け入れ条件**:
- [ ] CH1-4でモーメンタリ動作
- [ ] validity時間後に自動OFF
- [ ] デバウンス動作確認

#### P1-3: Digital Output実装（Alternate）

**実装内容**:
- setPinState() でトグル動作
- 状態保持

**受け入れ条件**:
- [ ] CH1-4でオルタネート動作
- [ ] ON/OFF切替確認

#### P1-4: PWM Output実装（LEDC）

**実装内容**:
- LEDC初期化（4ch, 5kHz, 8bit）
- Slow変化（rateOfChange考慮）
- Rapid変化（即座に変更）

**受け入れ条件**:
- [ ] CH1-4でPWM出力
- [ ] 0-100%の調光動作
- [ ] Slow/Rapid切替

#### P1-5: Digital Input実装（連動）

**実装内容**:
- CH5-6入力検知
- デバウンス処理
- allocation先への連動トリガー

**受け入れ条件**:
- [ ] CH5-6で入力検知
- [ ] 指定PINへの連動動作
- [ ] rotate動作（PWM時）

#### P1-6: HttpManagerIs06s基本構造実装

**成果物**:
- `HttpManagerIs06s.h`
- `HttpManagerIs06s.cpp`

**実装内容**:
- AraneaWebUI継承
- begin(), setupRoutes()
- PIN Manager連携

**受け入れ条件**:
- [ ] クラス定義完了
- [ ] Web UI基本ページ表示

#### P1-7: PIN操作API実装

**実装エンドポイント**:
- GET /api/status
- GET/POST /api/pin/{ch}/state
- GET/POST /api/pin/{ch}/setting
- GET/POST /api/settings

**受け入れ条件**:
- [ ] 全エンドポイント動作
- [ ] JSON形式正常
- [ ] エラーハンドリング

#### P1-8: PIN操作Web UI実装

**成果物**:
- `data/pincontrol.html`
- `data/pinsetting.html`

**受け入れ条件**:
- [ ] PIN状態表示
- [ ] PIN操作ボタン動作
- [ ] PIN設定変更UI

---

## 4. フェーズ2: 通信機能

### 4.1 タスク一覧

| ID | タスク | 依存 | ステータス | 担当 |
|----|--------|------|----------|------|
| P2-1 | StateReporterIs06s実装 | P1-7 | ⬜ 未着手 | - |
| P2-2 | LacisIDGenerator統合 | P0-8 | ⬜ 未着手 | - |
| P2-3 | AraneaRegister統合（CIC取得） | P2-2 | ⬜ 未着手 | - |
| P2-4 | MqttManager統合 | P2-3 | ⬜ 未着手 | - |
| P2-5 | MQTT経由PIN制御実装 | P2-4 | ⬜ 未着手 | - |
| **P2-6** | **Firestore typeSettings登録依頼** | P2-1 | ⬜ 未着手 | - |

### 4.2 詳細タスク

#### P2-1: StateReporterIs06s実装

**成果物**:
- `StateReporterIs06s.h`
- `StateReporterIs06s.cpp`

**受け入れ条件**:
- [ ] ペイロード構築正常
- [ ] HTTP POST送信成功
- [ ] 状態変化時自動送信

#### P2-2: LacisIDGenerator統合

**受け入れ条件**:
- [ ] lacisID生成成功（ProductType=006）
- [ ] NVS保存

#### P2-3: AraneaRegister統合（CIC取得）

**受け入れ条件**:
- [ ] デバイス登録成功
- [ ] CIC取得・保存
- [ ] フォールバック動作（オフライン時）

#### P2-4: MqttManager統合

**受け入れ条件**:
- [ ] MQTT接続成功
- [ ] Subscribe成功
- [ ] Publish成功

#### P2-5: MQTT経由PIN制御実装

**受け入れ条件**:
- [ ] コマンド受信→PIN制御
- [ ] ACK送信
- [ ] 状態変化Publish

#### P2-6: Firestore typeSettings登録依頼

**内容**:
- mobes2.0チームへar-is06sのtypeSettings登録を依頼
- スキーマファイル: `araneaSDK/schemas/types/aranea_ar-is06s.json`

**依頼情報**:
- Type名: ar-is06s
- ProductType: 006
- ProductCode: 0200
- displayName: Relay & Switch Controller

**受け入れ条件**:
- [ ] typeSettings/araneaDevice/ar-is06sが作成されていること
- [ ] StateReportの検証が通ること

---

## 5. フェーズ3: 拡張機能

### 5.1 タスク一覧

| ID | タスク | 依存 | ステータス | 担当 |
|----|--------|------|----------|------|
| P3-1 | DisplayManager統合（OLED表示） | P2-5 | ⬜ 未着手 | - |
| P3-2 | HttpOtaManager統合 | P2-5 | ⬜ 未着手 | - |
| P3-3 | NtpManager統合 | P2-5 | ⬜ 未着手 | - |
| P3-4 | System PIN実装（Reconnect/Reset） | P3-1 | ⬜ 未着手 | - |
| P3-5 | expiryDate判定実装 | P3-3 | ⬜ 未着手 | - |

### 5.2 詳細タスク

#### P3-1: DisplayManager統合（OLED表示）

**表示内容**:
- CIC
- IPアドレス/APモード
- PIN状態サマリ
- RSSI/Uptime

**受け入れ条件**:
- [ ] 起動時表示
- [ ] 状態変化時更新
- [ ] I2Cエラー時スキップ

#### P3-2: HttpOtaManager統合

**受け入れ条件**:
- [ ] OTAページ表示
- [ ] ファームウェア更新成功
- [ ] 更新中表示

#### P3-3: NtpManager統合

**受け入れ条件**:
- [ ] NTP同期成功
- [ ] タイムスタンプ取得

#### P3-4: System PIN実装（Reconnect/Reset）

**受け入れ条件**:
- [ ] GPIO25 5秒長押し→WiFi再接続
- [ ] GPIO26 15秒長押し→ファクトリーリセット
- [ ] OLED表示（カウントダウン）

#### P3-5: expiryDate判定実装

**受け入れ条件**:
- [ ] 有効期限超過時PIN無効化
- [ ] 警告表示

---

## 6. フェーズ4: 統合テスト・調整

### 6.1 タスク一覧

| ID | タスク | 依存 | ステータス | 担当 |
|----|--------|------|----------|------|
| P4-1 | 単体テスト実施 | P3-5 | ⬜ 未着手 | - |
| P4-2 | 統合テスト実施 | P4-1 | ⬜ 未着手 | - |
| P4-3 | 実機テスト（テスト環境） | P4-2 | ⬜ 未着手 | - |
| P4-4 | ドキュメント最終更新 | P4-3 | ⬜ 未着手 | - |

### 6.2 詳細タスク

#### P4-1: 単体テスト実施

**テスト対象**: IS06S_TestPlan.md参照

#### P4-2: 統合テスト実施

**テスト対象**: IS06S_TestPlan.md参照

#### P4-3: 実機テスト（テスト環境）

**環境**:
- SSID: sorapia_facility_wifi
- TID: T2025120621041161827

#### P4-4: ドキュメント最終更新

**更新対象**:
- 全設計ドキュメントのステータス更新
- 変更履歴追記

---

## 7. 依存関係図

```
P0-1 ─────► P0-2 ─────┬────► P0-3
                      │
                      ├────► P0-4
                      │
                      └────► P0-5
                                │
                      ┌─────────┴─────────┐
                      │                   │
                      ▼                   ▼
                    P0-6 ◄───────────── P0-3,4,5
                      │
        ┌─────────────┼─────────────┐
        │             │             │
        ▼             ▼             ▼
      P1-1          P1-6          P2-2
        │             │             │
   ┌────┼────┐        │             ▼
   │    │    │        │           P2-3
   ▼    ▼    ▼        │             │
 P1-2  P1-4  │        │             ▼
   │    │    │        │           P2-4
   ▼    │    │        │             │
 P1-3   │    │        │             ▼
   │    │    │        │           P2-5
   └────┼────┘        │             │
        │             │      ┌──────┼──────┐
        ▼             ▼      │      │      │
      P1-5 ─────► P1-7 ◄────┘      │      │
        │             │             │      │
        │             ▼             │      │
        │           P1-8            │      │
        │             │             │      │
        └─────────────┼─────────────┘      │
                      │                    │
                      ▼                    ▼
                    P2-1              P3-1,2,3
                                          │
                                          ▼
                                      P3-4,5
                                          │
                                          ▼
                                        P4-1
                                          │
                                          ▼
                                        P4-2
                                          │
                                          ▼
                                        P4-3
                                          │
                                          ▼
                                        P4-4
```

---

## 8. MECE確認

### 8.1 タスク網羅確認

| 機能領域 | タスク数 | 漏れ確認 |
|---------|---------|---------|
| 基盤構築 | 6 | ✅ |
| PIN制御 | 5 | ✅ |
| Web UI/API | 3 | ✅ |
| 通信（認証） | 2 | ✅ |
| 通信（MQTT） | 2 | ✅ |
| 通信（状態送信） | 1 | ✅ |
| 拡張機能 | 5 | ✅ |
| テスト | 4 | ✅ |

### 8.2 依存関係確認

- 循環依存なし
- 全タスクに明確な依存先

---

## 9. 変更履歴

| 日付 | バージョン | 変更内容 | 担当 |
|------|-----------|----------|------|
| 2025/01/23 | 1.0 | 初版作成 | Claude |
| 2025/01/23 | 1.1 | AraneaSettingsDefaults, PINglobal, enabled制御, Firestore登録タスク追加 | Claude |
