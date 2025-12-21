# MercuryACScraper クラス設計計画書

## 概要

Mercury AC (MAC100) からHTTPスクレイピングでAP/クライアント情報を取得するモジュール。
SshPollerIs10のパターンを踏襲し、非ブロッキング・FreeRTOSタスク方式で実装する。

## 対応するis10との対比

| 項目 | is10 (SshPollerIs10) | is10m (MercuryACScraper) |
|------|----------------------|--------------------------|
| 通信方式 | SSH | HTTP |
| 対象台数 | 複数ルーター（最大20台） | 1台のAC（複数サブネット対応） |
| データ取得 | SSHコマンド実行 | HTMLスクレイピング |
| パース方式 | 文字列パース | ストリームHTMLパース |
| タスク方式 | FreeRTOS タスク | FreeRTOS タスク |

## アーキテクチャ

### マルチサブネット構成

```
Mercury AC (192.168.96.5)
    ├── サブネットA (192.168.97.0/24) → エンドポイントA (tidA/fidA/ridA)
    ├── サブネットB (192.168.98.0/24) → エンドポイントB (tidB/fidB/ridB)
    └── サブネットC (192.168.3.0/24)  → エンドポイントC (tidC/fidC/ridC)

is10mデバイス自体 → デバイス登録用エンドポイント (tidD/fidD/ridD)
```

### 認証概念の整理

| 認証種別 | 用途 | tid/fid/rid |
|----------|------|-------------|
| **デバイス認証** | is10m自体のAraneaDeviceGate登録 | デバイス固有 |
| **データ送信認証** | 各サブネットのクライアント情報送信 | サブネット毎に異なる |

## データ構造

### MercuryTypes.h

```cpp
#ifndef MERCURY_TYPES_H
#define MERCURY_TYPES_H

#include <Arduino.h>

// 最大数定義
#define MAX_MERCURY_CLIENTS 200    // 最大追跡クライアント数
#define MAX_MERCURY_APS 50         // 最大AP数
#define MAX_MERCURY_SUBNETS 10     // 最大サブネット設定数

// クライアント情報（取得データ）
struct MercuryClient {
    char mac[18];           // MACアドレス "XX-XX-XX-XX-XX-XX"
    char apName[48];        // 接続先AP名
    char ssid[33];          // SSID
    char ip[16];            // IPv4アドレス
    char band[12];          // "2.4GHz" / "5GHz"
    int8_t rssi;            // 信号強度(dBm)
    uint32_t connectedAt;   // 接続UNIX時刻
};

// AP情報（取得データ）
struct MercuryAP {
    char name[48];          // AP名
    char model[24];         // モデル
    char mac[18];           // MACアドレス
    char ip[16];            // IPアドレス
    bool online;            // オンライン状態
    uint8_t clients24;      // 2.4GHzクライアント数
    uint8_t clients50;      // 5GHzクライアント数
};

// サブネットエンドポイント設定
struct SubnetEndpoint {
    char subnet[20];        // "192.168.97" (第3オクテットまで)
    char endpointUrl[128];  // POST先URL
    char tid[32];           // テナントID
    char fid[16];           // 施設ID
    char rid[16];           // リソースID
    char lacisId[24];       // LacisID
    char cic[12];           // CIC
    bool enabled;           // 有効フラグ
};

// AC接続設定
struct MercuryACConfig {
    char host[32];          // ACのIPアドレス
    uint16_t port;          // HTTPポート（80）
    char username[32];      // ログインユーザー名
    char password[64];      // ログインパスワード（MD5暗号化前）
    bool enabled;           // 有効フラグ
};

#endif // MERCURY_TYPES_H
```

## MercuryACScraper クラス設計

### ヘッダファイル構成

```cpp
// MercuryACScraper.h

#ifndef MERCURY_AC_SCRAPER_H
#define MERCURY_AC_SCRAPER_H

#include <Arduino.h>
#include <functional>
#include <HTTPClient.h>
#include <WiFiClient.h>
#include "MercuryTypes.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/semphr.h"

class MercuryACScraper {
public:
    MercuryACScraper();
    ~MercuryACScraper();

    // ========================================
    // 初期化・設定
    // ========================================

    void begin(MercuryACConfig* config);

    // サブネットエンドポイント設定
    void setSubnetEndpoints(SubnetEndpoint* endpoints, int count);

    // タイミング設定
    void setPollInterval(unsigned long ms);     // ポーリング間隔（デフォルト12分）
    void setHttpTimeout(unsigned long ms);      // HTTPタイムアウト（デフォルト30秒）

    // ========================================
    // メインループ
    // ========================================

    void handle();  // 非ブロッキング、main loop()で毎回呼び出す

    // ========================================
    // 状態取得
    // ========================================

    bool isEnabled() const;
    void setEnabled(bool enabled);

    bool isScraping() const;            // スクレイピング中か
    bool isLoggedIn() const;            // ログイン済みか

    int getClientCount() const;         // 取得クライアント数
    int getAPCount() const;             // 取得AP数
    int getLastErrorCode() const;       // 最後のエラーコード
    String getLastErrorMessage() const; // 最後のエラーメッセージ

    unsigned long getLastScrapeTime() const;  // 最終スクレイピング時刻

    // ========================================
    // データ取得
    // ========================================

    MercuryClient* getClients();
    int getClientsForSubnet(const char* subnet, MercuryClient* out, int maxOut);

    MercuryAP* getAPs();

    // ========================================
    // コールバック
    // ========================================

    // スクレイピング完了時
    using ScrapeCompleteCallback = std::function<void(int clientCount, int apCount, bool success)>;
    void onScrapeComplete(ScrapeCompleteCallback cb);

    // サブネット別データ準備完了時（送信トリガー用）
    using SubnetDataReadyCallback = std::function<void(const SubnetEndpoint& endpoint,
                                                        MercuryClient* clients, int count)>;
    void onSubnetDataReady(SubnetDataReadyCallback cb);

    // ========================================
    // 復旧
    // ========================================

    bool needsRestart() const;
    String getRestartReason() const;

private:
    // 設定
    MercuryACConfig* config_ = nullptr;
    SubnetEndpoint* subnetEndpoints_ = nullptr;
    int subnetEndpointCount_ = 0;

    unsigned long pollIntervalMs_ = 720000;   // 12分
    unsigned long httpTimeoutMs_ = 30000;     // 30秒

    // 状態
    bool enabled_ = true;
    volatile bool scrapeInProgress_ = false;
    volatile bool scrapeDone_ = false;
    volatile bool scrapeSuccess_ = false;
    bool loggedIn_ = false;
    char token_[64] = {0};                    // 認証トークン

    // タイミング（非ブロッキング）
    unsigned long nextScrapeAt_ = 0;
    unsigned long lastScrapeTime_ = 0;
    unsigned long scrapeTaskStartTime_ = 0;

    // エラー情報
    int lastErrorCode_ = 0;
    String lastErrorMessage_;

    // 復旧フラグ
    volatile bool needsRestart_ = false;
    String restartReason_;

    // 収集データ
    MercuryClient clients_[MAX_MERCURY_CLIENTS];
    int clientCount_ = 0;
    MercuryAP aps_[MAX_MERCURY_APS];
    int apCount_ = 0;

    // FreeRTOSタスク
    static const uint32_t SCRAPE_TASK_STACK_SIZE = 16384;  // 16KB
    static const unsigned long SCRAPE_TASK_WATCHDOG_MS = 120000;  // 2分
    TaskHandle_t scrapeTaskHandle_ = nullptr;
    SemaphoreHandle_t dataMutex_ = nullptr;

    // コールバック
    ScrapeCompleteCallback scrapeCompleteCallback_ = nullptr;
    SubnetDataReadyCallback subnetDataReadyCallback_ = nullptr;

    // ========================================
    // 内部メソッド
    // ========================================

    // タスク関数
    static void scrapeTaskFunction(void* params);
    void executeScrape();

    // HTTP通信
    bool login();
    bool fetchClientPage(WiFiClient& stream);
    bool fetchAPPage(WiFiClient& stream);

    // ストリームパース
    void parseClientStream(WiFiClient& stream);
    void parseAPStream(WiFiClient& stream);
    bool parseClientRow(const String& rowHtml, MercuryClient& out);
    bool parseAPRow(const String& rowHtml, MercuryAP& out);

    // ユーティリティ
    String md5Encrypt(const String& password);
    String extractBetween(const String& html, const String& start, const String& end);
    bool isValidMac(const char* mac);
    const char* getSubnetFromIP(const char* ip);

    // タイミング・ウォッチドッグ
    void startScrapeTask();
    void processCompletedScrape();
    void handleWatchdog();
    void notifySubnetData();
};

#endif // MERCURY_AC_SCRAPER_H
```

## 処理フロー

### メインフロー

```
┌─────────────────────────────────────────────────────────────┐
│                    handle() - メインループ                    │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              │ 非ブロッキングチェック         │
              │ - millis() >= nextScrapeAt_? │
              │ - scrapeInProgress_?         │
              │ - scrapeDone_?              │
              └───────────────┬───────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
   時刻到達              タスク実行中           タスク完了
        │                     │                     │
   startScrapeTask()    handleWatchdog()    processCompletedScrape()
        │                                           │
   FreeRTOS Task起動                          notifySubnetData()
                                                    │
                                           各サブネット別に
                                           コールバック発火
```

### スクレイピングタスクフロー

```
executeScrape() [FreeRTOS Task]
        │
        ├── 1. login()
        │       ├── POST http://192.168.96.5/
        │       ├── MD5暗号化パスワード送信
        │       └── トークン抽出・保存
        │
        ├── 2. fetchClientPage()
        │       ├── POST /stok={token}/userrpm/apmng_wstation.htm
        │       │   └── パラメータ: slt_item_num=1000
        │       └── parseClientStream()
        │               └── 行単位でストリームパース
        │                   └── MACアドレス検出で有効行判定
        │
        └── 3. fetchAPPage()
                ├── POST /stok={token}/userrpm/apmng_status.htm
                │   └── パラメータ: slt_item_num=100
                └── parseAPStream()
                        └── 行単位でストリームパース
```

### ストリームパース詳細

```cpp
void MercuryACScraper::parseClientStream(WiFiClient& stream) {
    String buffer;
    bool inDataRow = false;
    int tdIndex = 0;
    MercuryClient current;

    while (stream.available() || stream.connected()) {
        if (!stream.available()) {
            delay(1);  // データ待ち
            continue;
        }

        char c = stream.read();
        buffer += c;

        // <tr> 検出で行開始
        if (buffer.endsWith("<tr")) {
            inDataRow = true;
            tdIndex = 0;
            memset(&current, 0, sizeof(current));
        }

        // </tr> 検出で行終了
        if (buffer.endsWith("</tr>")) {
            if (inDataRow && isValidMac(current.mac)) {
                // 有効なクライアント行
                if (clientCount_ < MAX_MERCURY_CLIENTS) {
                    clients_[clientCount_++] = current;
                }
            }
            inDataRow = false;
            buffer = "";  // バッファクリア（メモリ節約）
        }

        // <td>...</td> 検出でセル値抽出
        if (inDataRow && buffer.endsWith("</td>")) {
            String value = extractTdValue(buffer);

            switch (tdIndex) {
                case 3: strncpy(current.mac, value.c_str(), 17); break;
                case 4: strncpy(current.apName, value.c_str(), 47); break;
                case 5: strncpy(current.band, value.c_str(), 11); break;
                case 7: strncpy(current.ssid, value.c_str(), 32); break;
                case 8: strncpy(current.ip, extractIpFromValue(value).c_str(), 15); break;
                case 13: current.rssi = extractRssi(value); break;
            }
            tdIndex++;

            // tdバッファのみクリア（行バッファは維持）
            int lastTdStart = buffer.lastIndexOf("<td");
            if (lastTdStart > 0) {
                buffer = buffer.substring(0, lastTdStart);
            }
        }

        // バッファサイズ制限（メモリ保護）
        if (buffer.length() > 4096) {
            buffer = buffer.substring(buffer.length() - 1024);
        }
    }
}
```

## サブネット別送信フロー

```
onScrapeComplete()
        │
        ▼
notifySubnetData()
        │
        ├── サブネットA (192.168.97.x)
        │       ├── クライアントをIPでフィルタ
        │       └── onSubnetDataReady(endpointA, filteredClients)
        │                   │
        │                   ▼
        │           CelestialSenderIs10m::send(endpointA, clients)
        │
        ├── サブネットB (192.168.98.x)
        │       └── ...
        │
        └── サブネットC (192.168.3.x)
                └── ...
```

## 設定保存構造（SPIFFS）

### /mercury_config.json

```json
{
  "ac": {
    "host": "192.168.96.5",
    "port": 80,
    "username": "isms",
    "password": "isms12345@",
    "enabled": true
  },
  "polling": {
    "intervalMs": 720000,
    "httpTimeoutMs": 30000
  },
  "subnets": [
    {
      "subnet": "192.168.97",
      "endpointUrl": "https://api.example.com/ingest",
      "tid": "T2025...",
      "fid": "9000",
      "rid": "R001",
      "lacisId": "30100...",
      "cic": "123456",
      "enabled": true
    },
    {
      "subnet": "192.168.98",
      "endpointUrl": "https://api.example.com/ingest",
      "tid": "T2025...",
      "fid": "9001",
      "rid": "R002",
      "lacisId": "30100...",
      "cic": "789012",
      "enabled": true
    }
  ]
}
```

## ファイル構成

```
is10m/
├── is10m.ino                   # メインスケッチ
├── MercuryTypes.h              # データ型定義
├── MercuryACScraper.h          # スクレイパーヘッダ
├── MercuryACScraper.cpp        # スクレイパー実装
├── HttpManagerIs10m.h          # Web管理UI（is10から流用・改修）
├── HttpManagerIs10m.cpp
├── CelestialSenderIs10m.h      # クラウド送信（is10から流用・改修）
├── CelestialSenderIs10m.cpp
├── AraneaSettings.h            # デフォルト設定（is10から流用・改修）
├── AraneaSettings.cpp
├── Is10mKeys.h                 # NVSキー定義
├── AraneaGlobalImporter.h      # 共通ライブラリインポート
└── design/
    ├── DESIGN.md               # 全体設計書
    └── MercuryACScraper_PLAN.md  # このファイル
```

## 実装順序

### Step 1: 基礎構造 (MercuryTypes.h)
- データ構造定義
- 定数定義

### Step 2: コア実装 (MercuryACScraper)
1. ログイン機能（MD5暗号化含む）
2. HTTPストリーム取得
3. ストリームパーサー（クライアント）
4. ストリームパーサー（AP）
5. FreeRTOSタスク統合
6. 非ブロッキングhandle()

### Step 3: サブネット振り分け
1. IPアドレスからサブネット判定
2. サブネット別コールバック発火
3. CelestialSender連携

### Step 4: 統合テスト
1. 単体テスト（ログイン→パース）
2. 12分間隔ポーリングテスト
3. メモリ使用量監視
4. 長時間稼働テスト

## 注意事項

### メモリ管理
- ストリームパース必須（フルHTML読み込み禁止）
- バッファサイズ上限設定（4KB推奨）
- 固定配列使用（動的確保最小限）

### エラーハンドリング
- HTTPタイムアウト→次サイクルでリトライ
- ログイン失敗→エラーログ出力、次サイクルで再試行
- パースエラー→行スキップ、継続処理

### VPN環境考慮
- mDNS使用不可
- IPアドレス直接指定
- 接続タイムアウト長めに設定（VPN遅延考慮）
