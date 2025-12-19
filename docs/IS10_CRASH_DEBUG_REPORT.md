# IS10 (ar-is10) ESP32 クラッシュループ問題 調査報告書

**作成日**: 2025-12-19
**ステータス**: 未解決 - 外部知見求む

---

## 1. 問題の概要

IS10デバイス（ESP32ベースのOpenWrt/ASUSWRTルーターインスペクター）が起動後10〜20秒でクラッシュループに陥る。
リセット理由は `POWERON_RESET` で、Task Watchdog Timeout (WDT) が疑われる。

### 症状
- 起動後、setup()完了直後〜loop()数回でクラッシュ
- リセット理由: `rst:0x1 (POWERON_RESET)`
- 平均稼働時間: 約3秒
- 41回のリブートを観測（5分間）
- シリアル出力が途中から文字化け・重複

---

## 2. 環境情報

### ハードウェア
- **MCU**: ESP32-DevKitC（ESP32-WROOM-32）
- **Flash**: 4MB
- **PSRAM**: なし
- **電源**: USB給電（MacBook Pro経由）
- **接続デバイス**: I2C OLED (SSD1306 128x64)

### ソフトウェア
- **Arduino IDE**: CLI版（arduino-cli）
- **ESP32 Arduino Core**: 3.2.1
- **パーティションスキーム**: `huge_app`（3MB app / 1MB SPIFFS）
- **開発OS**: macOS Darwin 24.6.0

### 使用ライブラリ
| ライブラリ | バージョン | 用途 |
|-----------|-----------|------|
| **LibSSH-ESP32** | 5.7.0 | SSH接続（ルーター情報取得） |
| ArduinoJson | 7.4.2 | JSON処理 |
| Adafruit SSD1306 | 2.5.15 | OLED表示 |
| Adafruit GFX | 1.12.1 | グラフィックス |
| Adafruit BusIO | 1.17.2 | I2C抽象化 |
| NTPClient | 3.2.1 | 時刻同期 |
| NimBLE-Arduino | 2.3.2 | BLE（未使用だがインストール済み） |
| WebSockets | 2.6.1 | WebSocket（未使用） |

### カスタムライブラリ
- **AraneaGlobalGeneric**: 自作の共通モジュール群
  - WiFiManager, DisplayManager, SettingManager, NtpManager
  - OtaManager, HttpOtaManager, LacisIDGenerator, AraneaRegister
  - Operator, SshClient

---

## 3. IS10デバイスの機能

```
[IS10 Openwrt_RouterInspector]
    │
    ├── WiFi STA接続
    ├── AraneaGateway登録（CIC取得）
    ├── NTP時刻同期
    ├── HTTP設定UI（WebServer）
    │     └── 14KB HTML + 16KB JSON応答
    ├── OTA更新（HTTP OTA / ArduinoOTA）
    └── ルーターポーリング（メイン機能）
          ├── SSH接続（LibSSH-ESP32）
          ├── uci/nvramコマンド実行
          └── HTTPSでクラウドへPOST
```

---

## 4. デバイスヘルス情報（MCP取得）

```json
{
  "port": "/dev/cu.usbserial-0001",
  "status": "crash_loop",
  "rebootCount": 41,
  "avgUptimeSeconds": 3.08,
  "lastReboot": {
    "type": "reset",
    "resetCode": "1"
  },
  "loopDetection": {
    "detected": true,
    "pattern": "modules to stabilize...",
    "occurrences": 49,
    "confidence": 1
  },
  "suspectedPattern": "modules to stabilize...",
  "suggestion": "Loop detected (49x in 0ms). Check for race conditions or async timing issues."
}
```

### シリアルログ（クラッシュ直前）
```
rst:ets Jul 29 2019 12:21:46
rst:ets Jul 29 2019 12:21:46
rst:0x1 (POWERON_RESET),boot:0x13 (SPI_FAST_FLASH_BOOT)
configsip: 0, SPIWP:0xee
clk_drv:0x00,q_drv:0x00,d_drv:0x00,cs0_drv:0x00,hd_drv:0x00,wp_drv:0x00
```

---

## 5. 原因分析

### 5.1 初期仮説: WebServer (httpMgr.handle())

最初に`httpMgr.handle()`がクラッシュ原因と判明：
- `httpMgr.handle()` 無効化 → loop()が995回まで安定
- `httpMgr.handle()` 有効化 → 10〜20秒でクラッシュ

**メカニズム**:
```
httpMgr.handle()
    └── WebServer::handleClient() [同期呼び出し]
          └── HTTPハンドラ実行
                ├── handleGetConfig() → 16KB DynamicJsonDocument
                ├── handleRoot() → 14KB HTML生成
                └── saveRouters() → 8KB JSON + SPIFFS書き込み
```

これらの重い処理が `loop()` をブロック → Task WDT タイムアウト → POWERON_RESET

### 5.2 さらなる調査: WebServer無効化でもクラッシュ

`httpMgr.begin()` と `httpMgr.handle()` を完全無効化してもクラッシュが継続。
次に疑われるのは **LibSSH-ESP32** ライブラリ。

### 5.3 LibSSH-ESP32の問題点

1. **メモリ消費**: libssh_begin() 呼び出しでヒープを大量消費
2. **mDNS競合**: ArduinoOTA（mDNS使用）との競合報告あり
3. **グローバル初期化**: `libssh_begin()` は静的/グローバル状態を持つ

---

## 6. これまでに試した対策と結果

| # | 対策 | 結果 |
|---|------|------|
| 1 | Brownout Detection無効化 | ❌ 効果なし |
| 2 | `huge_app` パーティション（3MB app） | ❌ 効果なし |
| 3 | ArduinoOTA無効化 | ❌ 効果なし |
| 4 | 5秒間の安定化待機ループ（yield()付き） | ❌ 効果なし |
| 5 | フラッシュ完全消去 | ❌ 効果なし |
| 6 | 巨大HTML簡略化（14KB→100B） | ❌ 効果なし |
| 7 | HTTPハンドラにyield()追加 | ❌ 効果なし |
| 8 | `httpMgr.begin()` 完全無効化 | ❌ **まだクラッシュ** |
| 9 | `libssh_begin()` 無効化 | ⏳ 未テスト（次のステップ） |

---

## 7. 現在のコード状態

### 7.1 is10.ino（主要部分）

```cpp
// Brownout disable
#include "soc/soc.h"
#include "soc/rtc_cntl_reg.h"

void setup() {
  // Disable brownout detection
  WRITE_PERI_REG(RTC_CNTL_BROWN_OUT_REG, 0);

  Serial.begin(115200);
  delay(100);
  Serial.println("\n[BOOT] is10 (ar-is10) starting...");

  // ... 各種初期化 ...

  // ArduinoOTA - DISABLED due to mDNS conflict with LibSSH
  /*
  ota.begin(myHostname, "");
  ...
  */

  // HttpManager - DISABLED for WDT debugging
  // httpMgr.begin(&settings, routers, &routerCount, 80);
  // httpMgr.setDeviceInfo(DEVICE_TYPE, myLacisId, myCic, FIRMWARE_VERSION);

  // 5秒間の安定化待機
  for (int i = 0; i < 50; i++) {
    delay(100);
    yield();
  }

  // libssh初期化 - DISABLED for crash debugging
  Serial.println("[LIBSSH] DISABLED for crash debugging");
  /*
  libssh_begin();
  */

  Serial.println("[BOOT] is10 ready!");
}

void loop() {
  static unsigned long loopCount = 0;
  loopCount++;

  // 10秒ごとにヒープ情報出力
  if (now - lastLoopLog >= 10000) {
    Serial.printf("[LOOP] count=%lu heap=%d\n", loopCount, ESP.getFreeHeap());
  }

  // OTA処理 - DISABLED
  // ota.handle();

  // HTTP処理 - DISABLED
  // httpMgr.handle();

  // ルーターポーリング（現在は実行されない: routerCount=0）
  if (!apModeActive && routerCount > 0 && ...) {
    pollRouter(currentRouterIndex);
  }

  delay(10);
}
```

### 7.2 HttpManagerIs10.cpp（yield()追加済み）

```cpp
void HttpManagerIs10::loadRouters() {
  if (!routers_ || !routerCount_) return;
  *routerCount_ = 0;

  yield();  // WDT対策: SPIFFS読み込み前
  File file = SPIFFS.open("/routers.json", "r");
  if (!file) return;

  String json = file.readString();
  file.close();
  yield();  // WDT対策: SPIFFS読み込み後

  DynamicJsonDocument doc(8192);
  DeserializationError err = deserializeJson(doc, json);
  yield();  // WDT対策: JSON parse後
  // ...
}

void HttpManagerIs10::handleGetConfig() {
  yield();  // WDT対策: 重い処理前
  DynamicJsonDocument doc(16384);
  // ... JSON構築 ...
  yield();  // WDT対策: JSON生成後
  String response;
  serializeJson(doc, response);
  yield();  // WDT対策: 送信前
  server_->send(200, "application/json", response);
}
```

### 7.3 SshClient.cpp（LibSSH初期化）

```cpp
// libssh初期化フラグ（一度だけ初期化するため）
static bool _libsshInitialized = false;

SshClient::SshClient() : _connected(false), _lastError(""), _session(nullptr), _channel(nullptr) {
#if SSH_AVAILABLE
  // libssh初期化（一度だけ）
  if (!_libsshInitialized) {
    Serial.println("[SSH] Initializing libssh...");
    libssh_begin();  // ★ ここでクラッシュの可能性
    _libsshInitialized = true;
    Serial.println("[SSH] libssh initialized");
  }
#endif
}
```

**注意**: 現在の実装では、`SshClient` のコンストラクタ内で `libssh_begin()` が呼ばれる。
つまり、`globalSshClient = new SshClient()` 時点でlibsshが初期化される。

---

## 8. ヒープメモリ状況（参考値）

setup()直後の典型的なヒープ状態：
```
[MEM] heap=~180KB, largest_block=~100KB
```

libssh_begin()後（正常時）：
```
[MEM] heap=~120KB, largest_block=~60KB
```

→ libsshで約60KBのヒープを消費

---

## 9. 考えられる根本原因

### A. LibSSH-ESP32の初期化問題
- `libssh_begin()` 内部でFreeRTOSタスク生成や大量メモリ確保
- ESP32 Arduino Core 3.xとの互換性問題
- スタックオーバーフロー

### B. 複数モジュールの競合
- WiFi + mDNS + LibSSH + WebServer の同時使用
- ネットワークスタックのリソース枯渇

### C. Task Watchdog Timeout
- メインloop()のブロッキング
- バックグラウンドタスクの飢餓

### D. 電源問題（Brownout）
- USB給電の電流不足
- WiFi TX時の電圧降下

---

## 10. 求める助言

1. **LibSSH-ESP32とESP32 Arduino Core 3.xの既知の問題**はあるか？
2. **libssh_begin() の正しい呼び出しタイミング**は？
3. **ESP32でのSSH実装のベストプラクティス**は？
4. **Task WDTを回避しつつ重い処理を行う方法**は？
5. 他に試すべき**デバッグ手法**は？

---

## 11. 関連ファイル一覧

```
generic/ESP32/is10/
├── is10.ino              # メインスケッチ（1021行）
├── HttpManagerIs10.cpp   # HTTP設定UI実装（1066行）
├── HttpManagerIs10.h     # HTTPマネージャヘッダ
├── RouterTypes.h         # ルーター設定構造体
├── AraneaSettings.cpp/h  # 環境設定
├── AraneaRegister.cpp/h  # デバイス登録
└── README.md             # デバイス仕様

generic/ESP32/global/src/
├── SshClient.cpp/h       # SSHクライアントラッパー
├── WiFiManager.cpp/h     # WiFi管理
├── DisplayManager.cpp/h  # OLED表示
├── SettingManager.cpp/h  # NVS設定管理
├── NtpManager.cpp/h      # NTP時刻同期
├── OtaManager.cpp/h      # ArduinoOTA
├── HttpOtaManager.cpp/h  # HTTP OTA
├── Operator.cpp/h        # 状態管理
└── ...
```

---

## 12. 補足: 類似プロジェクトでの動作状況

同じ `AraneaGlobalGeneric` ライブラリを使用する他のデバイス：

| デバイス | LibSSH使用 | 状態 |
|---------|-----------|------|
| is01（BLEセンサー） | ❌ | 安定 |
| is02（ゲートウェイ） | ❌ | 安定 |
| is04（接点出力） | ❌ | 安定 |
| is05（リード入力） | ❌ | 安定 |
| **is10（ルーターインスペクター）** | ✅ | **クラッシュループ** |

→ LibSSH使用デバイスのみ問題発生

---

## 13. 次のステップ

1. `libssh_begin()` を完全無効化してテスト
2. LibSSH-ESP32の issue tracker 確認
3. ESP32 Arduino Core 3.x → 2.x へのダウングレード検討
4. FreeRTOSタスクでのSSH処理分離検討

---

**連絡先**: （外部相談時に追記）
