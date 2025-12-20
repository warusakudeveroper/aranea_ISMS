# IS10 Router Inspector 実装記録

## 概要

IS10はASUSWRTルーター16台をSSHでポーリングし、WAN/LAN/クライアント情報を取得する監視デバイス。

## 確認済み事項（2025-12-20）

### SSH 16台完走テスト結果

| 項目 | 結果 |
|------|------|
| SSHは繋がる | ✓ |
| ESP32で完走できる | ✓ |
| ハードの問題はない | ✓ |
| 到達性の問題はない | ✓ |
| 16台全て成功 | ✓ |

**ログ抜粋:**
```
Router 1/16: 101 (192.168.125.171) → WAN: 192.168.125.171 ✓
Router 2/16: 104 (192.168.125.172) → WAN: 192.168.125.172 ✓
...
Router 16/16: 306 (192.168.125.186) → WAN: 192.168.125.186 ✓

COMPLETE: 16/16 success
Heap: 240724 → 237140 bytes（約3.5KB減少のみ、安定）
所要時間: 約38秒
```

### 使用したミニマル構成

- WiFi接続のみ
- LibSSH-ESP32でSSH接続
- 余計なモジュールなし（Display, HTTP, OTA, SPIFFS設定なし）
- パーティション: min_spiffs（OTA対応）

## ルーター情報

| RID | IP | Port |
|-----|-----|------|
| 101 | 192.168.125.171 | 65305 |
| 104 | 192.168.125.172 | 65305 |
| 105 | 192.168.125.173 | 65305 |
| 106 | 192.168.125.174 | 65305 |
| 201 | 192.168.125.175 | 65305 |
| 202 | 192.168.125.176 | 65305 |
| 203 | 192.168.125.177 | 65305 |
| 204 | 192.168.125.178 | 65305 |
| 205 | 192.168.125.179 | 65305 |
| 206 | 192.168.125.180 | 65305 |
| 301 | 192.168.125.181 | 65305 |
| 302 | 192.168.125.182 | 65305 |
| 303 | 192.168.125.183 | 65305 |
| 304 | 192.168.125.184 | 65305 |
| 305 | 192.168.125.185 | 65305 |
| 306 | 192.168.125.186 | 65305 |

認証情報: user=HALE_admin, pass=Hale_tam_2020063

## 禁止事項

- huge_appパーティション使用禁止（OTA不可になる）
- WDT/セマフォへの介入禁止
- 余計な監視ロジック追加禁止
- 場当たり的な対策コード禁止
- SSHタスクをCore 0で実行禁止（WiFi/システムと競合）
- loop()をブロッキング待ちにする設計禁止

## 必須設計パターン（2025-12-20確定）

### SSHタスク生成
```cpp
// 必ずCore 1、優先度+1で生成
xTaskCreatePinnedToCore(
  sshTaskFunction, "ssh", 51200,  // 51KB stack
  NULL, tskIDLE_PRIORITY + 1, NULL, 1  // Core 1
);
```

### 同期方式
```cpp
// フラグベース（非ブロッキング）
volatile bool sshInProgress = false;
volatile bool sshDone = false;
volatile bool sshSuccess = false;

// loop()でフラグをポーリング
if (sshDone && sshInProgress) {
  // 完了処理
}
```

## 犯人探しの方針

完走したミニマル構成に順次モジュールを追加して犯人を特定:

1. ✓ SSHのみ → 16/16完走
2. ✓ HTTP + SPIFFS追加 → 16/16完走（Heap: 232KB→228KB、安定）
3. ✓ Display追加 → 16/16完走（Heap: 228KB→224KB、安定）
4. ✓ OTA追加 → 16/16完走（2周連続成功、Heap: 215KB→212KB、安定）
5. ✓ NTP + Reboot Scheduler追加 → 16/16完走（Heap: 215KB→212KB、安定）
6. ✓ lacisID + SettingManager + AraneaSettings追加 → 16/16完走（Heap: 214KB→211KB、安定）

### OTA追加テスト詳細（2025-12-20 13:10）

**結果: 成功 - OTAは犯人ではない**

```
Round 1: 16/16 success
Round 2: 16/16 success（60秒後自動再開）

Heap推移:
- 初期: 278,156 bytes
- 初期化後: 215,440 bytes（OTAで約10KB追加消費）
- 完走後: 212,016 bytes
- 2周目: 211,568〜211,792 bytes（安定）
```

### OTA書き込みテスト（2025-12-20 13:20）

**結果: 成功 - SSH中でもOTA受付可能**

```
OTA書き込み: 成功（24秒）
再起動後SSH: 16/16 success ✓
Heap: 215KB → 212KB（安定）
```

**重要な実装ポイント:**
```cpp
// 正しい実装: OTA.handle()は常に呼ぶ
ArduinoOTA.handle();
if (otaInProgress) return;  // OTA中はSSHしない

// 間違い: SSH中はOTAを呼ばない → OTAが受け付けられない
// if (!sshRunning) ArduinoOTA.handle();
```

**重要な発見:**
- ArduinoOTA（mDNS方式）はLibSSH-ESP32と競合しない
- HTTPS競合の疑いは晴れた
- OTA.handle()は常に呼び、OTA開始時にSSHを中断する設計が正解

### lacisID + SettingManager + AraneaSettings追加テスト（2025-12-20 13:40）

**結果: 成功 - これらは犯人ではない**

```
LacisID: 3010F8B3B7496DEC0001
MAC: F8B3B7496DEC
[SETTING] Initialized (ns: isms)
[ARANEA] Settings initialized

Router 1/16 → WAN: 192.168.125.171 ✓
...
Router 16/16 → WAN: 192.168.125.186 ✓

COMPLETE: 16/16 success
Heap: 277KB → 214KB → 211KB（安定）
所要時間: 約34秒
```

**含まれるモジュール:**
- LacisID生成（MACベース20桁ID）
- SettingManager（NVS/Preferencesラッパー）
- AraneaSettings（デフォルト設定初期化）

## 根本原因特定（2025-12-20 完了）

### 原因: FreeRTOSタスク設計の誤り

本体コード（is10.ino）がクラッシュしていた根本原因は、SSHタスクの設計がミニマル版と異なっていたこと。

| 項目 | 本体版（クラッシュ） | ミニマル版（成功） |
|------|---------------------|-------------------|
| Core | Core 0 | **Core 1** |
| 優先度 | tskIDLE_PRIORITY + 3 | **tskIDLE_PRIORITY + 1** |
| 同期方式 | セマフォブロッキング | **フラグベース非ブロッキング** |
| loop() | セマフォ待ちでブロック | **フラグチェックで継続** |

### なぜCore 0がダメなのか

- **Core 0**: WiFi/Bluetooth/システムタスクが動作
- **Core 1**: Arduino loop()が動作

SSHタスクをCore 0で高優先度実行すると、WiFiスタックやシステムタスクと競合してクラッシュ。

### 修正内容

**1. フラグベースSSH制御に変更:**
```cpp
// グローバルフラグ追加
volatile bool sshInProgress = false;
volatile bool sshDone = false;
volatile bool sshSuccess = false;
```

**2. タスク生成をCore 1、優先度+1に変更:**
```cpp
xTaskCreatePinnedToCore(
  sshTaskFunction, "ssh", SSH_TASK_STACK_SIZE,
  NULL, tskIDLE_PRIORITY + 1, NULL, 1  // Core 1, priority +1
);
```

**3. loop()を非ブロッキングに変更:**
```cpp
// 旧: セマフォ待ち（ブロック）
// if (xSemaphoreTake(sshSemaphore, portMAX_DELAY) == pdTRUE) { ... }

// 新: フラグチェック（非ブロッキング）
if (sshDone && sshInProgress) {
  sshInProgress = false;
  if (sshSuccess) { successCount++; }
  currentRouterIndex++;
  // ...
}
```

### 修正後のテスト結果

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
Heap: 218656 → 217072 bytes（約1.5KB使用、安定）
```

## 解決済み疑い

- ~~HTTPS系（LibSSHとTLS/SHA競合の可能性）~~ → OKと判明
- ~~OTA（mDNS/ArduinoOTA）~~ → OKと判明
- ~~NTP + Reboot Scheduler~~ → OKと判明
- ~~lacisID + SettingManager + AraneaSettings~~ → OKと判明
- ~~余計な監視・ハンドリングロジック~~ → **タスク設計が原因**
- ~~過剰なSerial出力~~ → **タスク設計が原因**

**真の原因: FreeRTOSタスクのCore/優先度/同期方式の設計ミス**

## ファイル

- ミニマル版: `/private/tmp/is10_minimal/is10_minimal.ino`
- 本体: `/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/aranea_ISMS/generic/ESP32/is10/is10.ino`

## ミニマル版の構成（全モジュール版）

```cpp
// 含まれるモジュール
#include <WiFi.h>
#include <SPIFFS.h>
#include <WebServer.h>
#include <ArduinoJson.h>
#include <Wire.h>
#include <Adafruit_GFX.h>
#include <Adafruit_SSD1306.h>
#include <ArduinoOTA.h>
#include <Preferences.h>       // SettingManager用
#include <time.h>              // NTP用
#include <esp_mac.h>           // LacisID用
#include "libssh_esp32.h"
#include <libssh/libssh.h>

// 機能:
// - WiFi接続
// - SPIFFS（ルーター設定読み込み）
// - HTTP Server（ステータス表示）
// - Display（OLED 128x64）
// - OTA（ArduinoOTA）
// - NTP（時刻同期）
// - Reboot Scheduler（毎日4:00再起動、1時間毎チェック）
// - LacisID生成
// - SettingManager（NVS）
// - AraneaSettings（デフォルト設定）
// - SSH（LibSSH-ESP32）16台ルーターポーリング

// ファームウェアサイズ: 1,231,477 bytes (62% of 1,966,080)
// RAM使用: 63,408 bytes (19%)
```

## 通信系実装（2025-12-20）

### 実装内容

| 機能 | ステータス |
|------|----------|
| type文字列分離 | ✓ 完了 |
| AraneaDeviceGate登録 | ✓ 完了 |
| deviceStateReport | ✓ 完了 |
| MQTT接続・config受信 | ✓ 完了 |
| CelestialGlobe SSOT送信 | ✓ 完了 |
| HTTP UI設定項目追加 | ✓ 完了 |
| LacisID Genタブ非推奨化 | ✓ 完了 |

### type文字列分離

用途別に3つの定数を定義:

```cpp
// AraneaDeviceGate / deviceStateReport用（canonical）
static const char* ARANEA_DEVICE_TYPE = "aranea_ar-is10";

// CelestialGlobe payload.source用
static const char* CELESTIAL_SOURCE = "ar-is10";

// AP SSID / hostname等 表示用
static const char* DEVICE_SHORT_NAME = "ar-is10";
```

### CelestialGlobe SSOT送信

全ルーターポーリング完了後、Universal Ingest形式でPOST:

```json
{
  "auth": {
    "tid": "T2025...",
    "lacisId": "3010...",
    "cic": "123456"
  },
  "source": "ar-is10",
  "observedAt": 1734681234567,
  "devices": [
    {
      "type": "Router",
      "label": "Room101",
      "status": "online",
      "lanIp": "192.168.1.1",
      "wanIp": "xxx.xxx.xxx.xxx",
      "ssid24": "...",
      "ssid50": "...",
      "clientCount": 5
    }
  ]
}
```

**注意事項:**
- observedAtはUnix ms（number型）
- ルーターのlacisIdは送信しない（サーバがMACから生成）
- parentMacにis10のMACを入れない

### MQTT設定同期

- 接続先: `araneaReg.getSavedMqttEndpoint()` (WSS)
- 認証: lacisId + cic
- トピック: `aranea/{tid}/{lacisId}/config`
- schemaVersion巻き戻し防止

### HTTP UI追加設定

| 項目 | NVSキー | 説明 |
|------|---------|------|
| CelestialGlobe Endpoint | is10_endpoint | POST先URL |
| X-Celestial-Secret | is10_celestial_secret | APIシークレット |
| Scan Interval | is10_scan_interval_sec | スキャン間隔(秒) |
| Report Clients | is10_report_clients | クライアント送信 |

### deviceStateReport

起動後1回（必須）+ 60秒毎heartbeat:

```json
{
  "auth": { "tid": "...", "lacisId": "...", "cic": "..." },
  "report": {
    "lacisId": "3010...",
    "type": "aranea_ar-is10",
    "observedAt": "2025-12-20T12:00:00Z",
    "state": {
      "IPAddress": "192.168.1.100",
      "routerCount": 16,
      "successCount": 16,
      "mqttConnected": true
    }
  }
}
```

## 最終ステータス（2025-12-20）

| 項目 | 状態 |
|------|------|
| 16台SSH完走 | **解決** |
| クラッシュ | **解決** |
| Heap安定性 | **安定**（217KB維持） |
| OTA対応 | **動作確認済み** |
| 通信系実装 | **完了** |

**結論**: is10.ino本体コードの修正完了。16/16 SSH完走、OTA可能、通信系統合済みで安定動作。ファームウェアバージョン: 1.2.0
