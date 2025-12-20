# インシデント記録: IS10 SSHポーリング中クラッシュ

## 基本情報

| 項目 | 内容 |
|------|------|
| 発生日 | 2025-12-20 |
| 解決日 | 2025-12-20 |
| 対象デバイス | IS10 (Router Inspector) |
| 重要度 | 高（機能停止） |
| ステータス | **解決済み** |

## 現象

IS10がASUSWRTルーター16台へのSSHポーリング中に、8〜10台目付近でクラッシュ・再起動を繰り返す。

### 症状

- Router 8〜10でクラッシュ
- シリアル出力が途中で停止または文字化け
- ESP32が自動再起動
- 16台完走できない

### エラーログ例

```
[SSH] Router 8/16: 204 (192.168.125.178)
[SSH] WAN: 192.168.125.178
[L3200] heap=218156
[L3200] heap=218156  ← 同じ行が繰り返される（バッファ破損）
（クラッシュ・再起動）
```

## 原因調査

### 疑った箇所（すべてシロ）

1. ~~LibSSH-ESP32とHTTPS/TLSの競合~~ → OKと判明
2. ~~ArduinoOTA~~ → OKと判明
3. ~~NTP/Reboot Scheduler~~ → OKと判明
4. ~~SettingManager/AraneaSettings~~ → OKと判明
5. ~~ヒープ不足~~ → 217KB以上維持で安定
6. ~~ハードウェア問題~~ → ミニマル版は16/16成功

### 調査方法

ミニマル構成（WiFi + SSH のみ）から順次モジュールを追加し、クラッシュが発生するポイントを特定。

結果: **全モジュール追加しても16/16成功** → 本体コードとの差分が原因

## 根本原因

### FreeRTOSタスク設計の誤り

| 項目 | 本体版（クラッシュ） | ミニマル版（成功） |
|------|---------------------|-------------------|
| **Core** | Core 0 | Core 1 |
| **優先度** | tskIDLE_PRIORITY + 3 | tskIDLE_PRIORITY + 1 |
| **同期方式** | セマフォブロッキング | フラグベース非ブロッキング |
| **loop()動作** | セマフォ待ちでブロック | フラグチェックで継続 |

### なぜCore 0がダメなのか

```
ESP32 デュアルコア構成:
├── Core 0: WiFi/Bluetooth/システムタスク（FreeRTOS管理）
└── Core 1: Arduino loop()（ユーザーコード）
```

SSHタスクをCore 0で高優先度（+3）実行すると:
- WiFiスタックとCPU時間を奪い合う
- システムタスク（WDT feed等）が遅延
- 結果としてTask WDTまたはInt WDTでクラッシュ

### なぜセマフォブロッキングがダメなのか

```cpp
// 問題のあるコード
void loop() {
  if (xSemaphoreTake(sshSemaphore, portMAX_DELAY) == pdTRUE) {
    // SSHタスク完了まで loop() が完全停止
    // → ArduinoOTA.handle() も呼ばれない
    // → 他のハンドラも動かない
  }
}
```

loop()がブロックすると、ESP32の正常動作に必要な処理が滞る。

## 解決策

### 1. SSHタスクをCore 1で実行

```cpp
// 修正後
xTaskCreatePinnedToCore(
  sshTaskFunction,
  "ssh",
  51200,                    // 51KB stack
  NULL,
  tskIDLE_PRIORITY + 1,     // 低優先度に変更
  NULL,
  1                         // Core 1 に変更
);
```

### 2. フラグベース非ブロッキング同期

```cpp
// グローバルフラグ
volatile bool sshInProgress = false;
volatile bool sshDone = false;
volatile bool sshSuccess = false;

// SSHタスク終了時
void sshTaskFunction(void* pvParameters) {
  // ... SSH処理 ...
  sshSuccess = true;  // or false
  sshDone = true;
  vTaskDelete(NULL);
}

// loop()でポーリング
void loop() {
  ArduinoOTA.handle();  // 常に呼べる

  if (sshDone && sshInProgress) {
    sshInProgress = false;
    if (sshSuccess) successCount++;
    currentRouterIndex++;
    sshDone = false;
  }

  if (!sshInProgress) {
    startSshTask(currentRouterIndex);
  }
}
```

## 修正結果

```
[POLL] Starting SSH for Router 1/16
[SSH] Router 1/16: 101 (192.168.125.171)
[SSH] WAN: 192.168.125.171
[POLL] Router 1/16 SUCCESS
...
[SSH] Router 16/16: 306 (192.168.125.186)
[SSH] WAN: 192.168.125.186
[POLL] Router 16/16 SUCCESS

[COMPLETE] 16/16 success
[COMPLETE] heap=214832
```

- 16/16 完走
- Heap安定（214〜217KB維持）
- OTA動作確認済み
- 機能削減なし

## 教訓・禁止事項

### 禁止

1. **SSHタスクをCore 0で実行しない**
   - Core 0はWiFi/システム専用

2. **loop()をブロッキング待ちにしない**
   - セマフォの`portMAX_DELAY`待ちは禁止
   - フラグポーリングを使う

3. **高優先度タスクを乱用しない**
   - `tskIDLE_PRIORITY + 1` で十分
   - +3以上はシステムタスクと競合リスク

4. **WDT介入禁止**
   - `esp_task_wdt_delete()` などでWDTを無効化しない
   - WDTはシステム保護のために必要

### 必須パターン

```cpp
// ESP32でのバックグラウンドタスク正しい設計

// 1. Core 1、低優先度で生成
xTaskCreatePinnedToCore(taskFunc, "name", stackSize,
  NULL, tskIDLE_PRIORITY + 1, NULL, 1);

// 2. フラグで完了通知
volatile bool taskDone = false;
void taskFunc(void* p) {
  // 処理
  taskDone = true;
  vTaskDelete(NULL);
}

// 3. loop()は非ブロッキング
void loop() {
  if (taskDone) {
    // 完了処理
    taskDone = false;
  }
}
```

## 関連ファイル

- 本体: `generic/ESP32/is10/is10.ino`
- ミニマル版: `/private/tmp/is10_minimal/is10_minimal.ino`
- 実装記録: `docs/common/is10_implementation_record.md`

## 参考情報

- ESP32 FreeRTOS公式ドキュメント
- LibSSH-ESP32 GitHub リポジトリ
- Arduino-ESP32 Core割り当てガイドライン
