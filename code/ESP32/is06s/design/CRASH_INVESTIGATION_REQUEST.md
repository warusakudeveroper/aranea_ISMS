# IS06S クラッシュ問題調査依頼

## 概要

IS06Sファームウェアにおいて、MQTT無効化状態でもデバイスがクラッシュループに陥る問題が発生しています。

- **発生日**: 2026-01-24
- **緊急度**: High
- **ステータス**: 調査待ち

---

## 症状

### クラッシュパターン

1. **シリアル出力の断片化・繰り返し**
   ```
   [NTP] Syncing...
   [NTP] Syncing...
   [NTP] Syncing...
   ... (数十回繰り返し)
   ets Jul 29 2019 12:21:46
   rst:0x1 (POWERON_RESET)
   ```

2. **別パターン**
   ```
   e: 1 success
   e: 1 success
   ... (数十回繰り返し)
   ets Jul 29 2019 12:21:46
   rst:0x1 (POWERON_RESET)
   ```

3. **デバイスヘルス情報**
   - 11回リブート/5分
   - 平均uptime: 2.5秒
   - ループ検出: "e: N success" パターン (48回/0ms)

### 重要な観察

- MQTTを`#if 0`で完全無効化してもクラッシュが継続
- フラッシュ消去後も同様の症状
- **blinkスケッチは正常動作** → ハードウェアは正常

---

## 検証済み事項

### ハードウェア検証
- [x] blinkスケッチで正常動作確認
- [x] シリアル通信正常（115200baud）
- [x] フラッシュ書き込み正常（低速115200baudで安定）

### MQTT問題（解決済み）
- **原因**: `onConnected`コールバック内で`publishAllPinStates()`を呼び出し
- **根本原因**: ESP-IDF MQTTイベントハンドラコンテキスト制約
- **修正**: フラグベースでloop()内で実行するよう変更
- **参考**: is04/is05aは同コールバック内でpublishを行っていない

### 現在のクラッシュ問題（未解決）
- MQTT無効化状態でも発生
- NtpManager, StateReporter等の共通モジュールはis04/is05aで正常動作
- **is06s固有コードが原因と推定**

---

## 疑わしいモジュール

### 1. Is06PinManager
- **ファイル**: `code/ESP32/is06s/Is06PinManager.cpp`
- **機能**: 6ch PIN制御（4ch D/P出力 + 2ch I/O）
- **懸念点**:
  - LEDC（PWM）初期化
  - NVS読み書き
  - update()内のロジック

### 2. HttpManagerIs06s
- **ファイル**: `code/ESP32/is06s/HttpManagerIs06s.cpp`
- **機能**: Web UI + REST API
- **懸念点**:
  - AraneaWebUIからの継承
  - PIN固有エンドポイント登録

### 3. StateReporterIs06s
- **ファイル**: `code/ESP32/is06s/StateReporterIs06s.cpp`
- **機能**: クラウドへの状態送信
- **懸念点**:
  - HTTPClient使用
  - JSON構築（メモリ使用量）
  - update()でのハートビート送信

---

## 推奨調査手順

### Phase 1: モジュール切り分け

1. **最小構成テスト**
   - WiFi + NTP + Display のみで動作確認
   - Is06PinManager無効化
   - HttpManager無効化
   - StateReporter無効化

2. **段階的有効化**
   - Is06PinManager追加 → 動作確認
   - HttpManager追加 → 動作確認
   - StateReporter追加 → 動作確認

### Phase 2: 詳細調査

1. **メモリ使用量確認**
   ```cpp
   Serial.printf("Free heap: %d\n", ESP.getFreeHeap());
   Serial.printf("Min free heap: %d\n", ESP.getMinFreeHeap());
   ```

2. **スタックサイズ確認**
   - デフォルトスタックサイズで不足している可能性

3. **NVS操作の確認**
   - Is06PinManagerのNVS読み書きが過剰でないか

### Phase 3: クラッシュダンプ解析

1. Core Dump有効化（CONFIG_ESP_COREDUMP_ENABLE）
2. addr2lineでスタックトレース解析

---

## 関連ファイル

```
code/ESP32/is06s/
├── is06s.ino                 # メインスケッチ
├── Is06PinManager.h/cpp      # PIN制御モジュール
├── HttpManagerIs06s.h/cpp    # HTTP/Web UIモジュール
├── StateReporterIs06s.h/cpp  # 状態送信モジュール
└── design/
    ├── IS06S_TaskList.md     # タスクリスト
    └── CRASH_INVESTIGATION_REQUEST.md  # 本ドキュメント
```

---

## 環境情報

- **ボード**: ESP32-DevKitC (ESP32-D0WD-V3 rev3.1)
- **Arduino-ESP32**: 3.0.5 (ESP-IDF 5.x)
- **パーティション**: min_spiffs
- **フラッシュ使用率**: 72% (1,424,925 bytes)
- **RAM使用率**: 16% (53,392 bytes)

---

## 参考情報

### 正常動作デバイス（比較用）
- `archive_ISMS/ESP32/is04/` - MQTT使用、正常動作
- `code/ESP32/is05a/` - MQTT使用、正常動作

### MQTT修正内容
- `is06s.ino` line 118: `mqttJustConnected`フラグ追加
- `is06s.ino` line 648-654: onConnectedコールバック修正
- `is06s.ino` line 368-372: loop()内でpublish実行

---

## 担当・期限

- **調査担当**: TBD
- **希望期限**: 2026-01-27
- **報告先**: 本ドキュメントに追記

---

## 更新履歴

| 日付 | 更新者 | 内容 |
|-----|-------|------|
| 2026-01-24 | Claude Code | 初版作成 |
