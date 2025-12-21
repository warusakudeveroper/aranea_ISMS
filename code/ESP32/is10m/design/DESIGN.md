# ar-is10m - Mercury AC Client Monitor 設計書

## 概要

Mercury AC（MAC100）からAPステータスとクライアント接続情報を取得するESP32デバイス。

| 項目 | 値 |
|-----|-----|
| デバイスタイプ | `ar-is10m` |
| モデル名 | `Mercury_AC_Monitor` |
| ProductType | `010` |
| ProductCode | `0002` |
| 対象ボード | ESP32-DevKitC (4MB Flash) |
| ベース | is10（Router Inspector） |

## 対象機器

- **Mercury MAC100** (無線ACコントローラ)
- IPアドレス: `192.168.96.5`
- ファームウェア: 1.0.1 Build 250211

## 取得対象データ

### 1. クライアント状態（客户端状态）

| フィールド | 説明 | 例 |
|-----------|------|-----|
| MAC地址 | クライアントMACアドレス | `6C-C8-40-8C-CD-4C` |
| AP名称 | 接続先AP名 | `MercuryMOAP1200D[ap02-10]` |
| 射频单元 | 周波数帯 | `1(2.4GHz)` / `2(5GHz)` |
| SSID | 接続SSID | `cluster1` |
| IPv4地址 | クライアントIP | `192.168.97.15` |
| 接入时间 | 接続日時 | `2025/12/15 15:12:28` |
| 信号强度 | 信号強度(RSSI) | `-57dBm` |

### 2. AP状態（AP状态）

| フィールド | 説明 | 例 |
|-----------|------|-----|
| AP名称 | AP識別名 | `MercuryMOAP1200D[ap02-10]` |
| 型号 | モデル | `MOAP1200D` |
| MAC地址 | APのMAC | `4C-B7-E0-59-FC-EF` |
| IP地址 | APのIP | `192.168.96.19` |
| 在线时长 | 稼働時間 | `6天14小时52分钟` |
| 链路状况 | 接続状態 | `在线` / `离线` |

## API調査結果（2025/12/22 最終検証済み）

### 認証方式

1. **ログインエンドポイント**: `POST http://192.168.96.5/`
2. **認証情報**:
   - ユーザー名: `isms`
   - パスワード: `isms12345@`
3. **トークン**: ログイン成功後レスポンスの `stok` フィールド
   - 形式: 32文字HEX (例: `5d4dded934606afefa3d23713a566aba`)
4. **パスワード暗号化**: XORベースのsecurityEncode（RSA/MD5ではない）

#### パスワードエンコード実装

```cpp
// C++実装
String securityEncode(const String& password, const String& key1, const String& key2) {
    String result = "";
    int g = password.length();
    int m = key1.length();
    int k = key2.length();
    int h = max(g, m);

    for (int f = 0; f < h; f++) {
        int l = 187, t = 187;
        if (f >= g) {
            t = key1[f];
        } else if (f >= m) {
            l = password[f];
        } else {
            l = password[f];
            t = key1[f];
        }
        result += key2[(l ^ t) % k];
    }
    return result;
}

// 固定キー
const char* KEY1 = "RDpbLfCPsJZ7fiv";
const char* KEY2 = "yLwVl0zKqws7LgKPRQ84Mdt708T1qQ3Ha7xv3H7NyU84p21BriUWBU43odz3iP4rBL3cD02KZciXTysVXiV8ngg6vL48rPJyAUw0HurW20xqxv9aYb4M9wK1Ae0wlro510qXeU07kV57fQMc8L6aLgMLwygtc0F10a0Dg70TOoouyFhdysuRMO51yY5ZlOZZLEal1h0t9YQW0Ko7oBwmCAHoic4HYbUyVeU3sfQ1xtXcPcf1aT303wAQhv66qzW";

// 例: "isms12345@" → "33QQrnYH2sefbwK"
```

### ログインAPI

```
POST http://192.168.96.5/
Content-Type: application/json; charset=UTF-8

{"method":"do","login":{"username":"isms","password":"33QQrnYH2sefbwK","force":0}}
```

**レスポンス**:
```json
{"stok":"5d4dded934606afefa3d23713a566aba","role":"sys_admin","error_code":0}
```

### データ取得API（JSON API方式 ✅ 動作確認済み）

**重要**: `filter` パラメータが必須。これがないと空配列が返る。

#### クライアントリストAPI

```
POST http://192.168.96.5/stok={token}/ds
Content-Type: application/json; charset=UTF-8

{
  "method": "get",
  "apmng_client": {
    "table": "client_list",
    "filter": [{"group_id": ["-1"]}],
    "para": {"start": 0, "end": 99}
  }
}
```

**レスポンス例**:
```json
{
  "error_code": 0,
  "apmng_client": {
    "count": {"client_list": 16},
    "client_list": [
      {
        "client_list_82": {
          "mac": "6C-C8-40-8C-CD-4C",
          "ip": "192.168.97.15",
          "ipv6": "%3a%3a",
          "ssid": "cluster1",
          "ap_name": "MercuryMOAP1200D%5bap02-10%5d",
          "entry_id": "82",
          "vlan_id": "0",
          "connect_date": "2025%2f12%2f15",
          "connect_time": "15%3a12%3a28",
          "rf_entry": [
            {"freq_name": "2.4GHz", "rssi": "-51", "freq_unit": "1", "nego_rate": "0"}
          ]
        }
      }
    ]
  }
}
```

#### APリストAPI

```
POST http://192.168.96.5/stok={token}/ds
Content-Type: application/json; charset=UTF-8

{
  "method": "get",
  "apmng_status": {
    "table": "ap_list",
    "filter": [{"group_id": ["-1"]}],
    "para": {"start": 0, "end": 99}
  }
}
```

**レスポンス例**:
```json
{
  "error_code": 0,
  "apmng_status": {
    "ap_list": [
      {
        "ap_list_0": {
          "mac": "4C-B7-E0-59-FC-EF",
          "entry_name": "MercuryMOAP1200D%5bap02-10%5d",
          "ip": "192.168.96.19",
          "model_name": "MOAP1200D",
          "link_status": "1",
          "led_status": "1",
          "online_time": "576576",
          "rf_entry": [
            {"freq_name": "2.4GHz", "rf_client_num": "1", "channel": "11"},
            {"freq_name": "5GHz", "rf_client_num": "0", "channel": "161"}
          ]
        }
      }
    ]
  }
}
```

### APIキーポイント

| 項目 | 値 |
|------|-----|
| `group_id: ["-1"]` | 全グループ（必須パラメータ） |
| `para.start` | 取得開始位置（0から） |
| `para.end` | 取得終了位置（99で100件） |
| 値のエンコード | URLエンコード済み（%5b = [, %2f = /） |

### クライアントテーブル構造（検証済み）

| Index | カラム名 | 説明 |
|-------|---------|------|
| 0 | (checkbox) | 選択用 |
| 1 | 序号 | 連番 |
| 2 | 客户端名称 | クライアント名（通常"---"） |
| 3 | MAC地址 | MACアドレス |
| 4 | AP名称 | 接続先AP名 |
| 5 | 射频单元 | 周波数帯 |
| 6 | (duplicate) | - |
| 7 | SSID | 接続SSID |
| 8 | IPv4/IPv6地址 | IPアドレス |
| 9 | VLAN ID | VLAN（通常"---"） |
| 10 | 接入时间 | 接続日時 |
| 11-12 | 协商速率 | 通信速度 |
| 13 | 信号强度 | RSSI |

### APテーブル構造（検証済み）

| Index | カラム名 | 説明 |
|-------|---------|------|
| 2 | AP名称 | AP識別名 |
| 3 | 型号 | モデル |
| 4 | MAC地址 | MACアドレス |
| 5 | IP地址 | IPアドレス |

### 検証結果サマリ

- **接続確認済みAP数**: 18台
- **接続確認済みクライアント数**: 12台
- **APモデル**: MOAP1200D（MercuryMOAP1200D）
- **クライアントMAC例**: `6C-C8-40-8C-CD-4C`
- **AP MAC例**: `4C-B7-E0-59-FC-EF`

## システム設計

### アーキテクチャ

```
┌─────────────┐     HTTP      ┌─────────────┐
│   ESP32     │ ───────────→ │  Mercury AC │
│  (is10m)    │               │  (MAC100)   │
└─────────────┘               └─────────────┘
       │
       │ POST JSON
       ↓
┌─────────────┐
│   Cloud     │
│ (is03/API)  │
└─────────────┘
```

### is10との差分

| 項目 | is10 | is10m |
|------|------|-------|
| 対象機器 | OpenWrt/AsusWRTルーター | Mercury AC |
| 接続方式 | SSH | HTTP |
| 対象台数 | 最大20台（マルチルーター） | 1台（AC経由で複数AP） |
| 取得情報 | ルーター状態 | クライアント接続先AP |
| ポーリング間隔 | 30秒 | 12分（1時間5回） |

### 主要モジュール

```
is10m/
├── is10m.ino                 # メインスケッチ
├── MercuryPoller.h           # Mercury AC HTTP通信
├── MercuryPoller.cpp
├── MercuryTypes.h            # データ型定義
├── HttpManagerIs10m.h        # Web管理UI
├── HttpManagerIs10m.cpp
├── CelestialSenderIs10m.h    # クラウド送信
├── CelestialSenderIs10m.cpp
└── design/
    └── DESIGN.md             # この設計書
```

### MercuryTypes.h 案

```cpp
// クライアント情報
struct MercuryClient {
    String mac;           // MACアドレス
    String apName;        // 接続先AP名
    String ssid;          // SSID
    String ipv4;          // IPv4アドレス
    String band;          // 2.4GHz / 5GHz
    int rssi;             // 信号強度(dBm)
    String connectedAt;   // 接続日時
};

// AP情報
struct MercuryAP {
    String name;          // AP名
    String model;         // モデル
    String mac;           // MACアドレス
    String ip;            // IPアドレス
    String uptime;        // 稼働時間
    bool online;          // オンライン状態
    int clients24;        // 2.4GHzクライアント数
    int clients50;        // 5GHzクライアント数
};

// AC設定
struct MercuryConfig {
    String host;          // ACのIPアドレス
    int port;             // HTTPポート（80）
    String username;      // ログインユーザー名
    String password;      // ログインパスワード
    bool enabled;         // 有効フラグ
};
```

### MercuryPoller 処理フロー（JSON API方式）

```
1. login()
   ├── POST / (ルートURL)
   ├── securityEncode()でパスワードエンコード
   ├── {"method":"do","login":{...}} 送信
   └── レスポンスから stok トークン抽出・保存

2. fetchClients()
   ├── POST /stok={token}/ds
   ├── {"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],...}}
   ├── JSONレスポンスパース
   └── client_list配列からMercuryClient構造体へ変換
       └── URLデコード処理（ap_name等）

3. fetchAPs()
   ├── POST /stok={token}/ds
   ├── {"method":"get","apmng_status":{"table":"ap_list","filter":[{"group_id":["-1"]}],...}}
   ├── JSONレスポンスパース
   └── ap_list配列からMercuryAP構造体へ変換

4. handle() - メインループ
   ├── 12分間隔でポーリング
   ├── トークン期限切れ時（error_code != 0）は再ログイン
   └── 取得データをコールバックで通知
```

### JSON APIの利点

HTMLスクレイピングと比較した利点:
- **構造化データ**: パース処理が簡単・確実
- **軽量レスポンス**: HTMLより小さいデータサイズ
- **安定性**: UIレイアウト変更の影響を受けない
- **ArduinoJson対応**: ESP32での標準的なJSONライブラリが使用可能

## タイミング設計

| パラメータ | 値 | 説明 |
|-----------|-----|------|
| ポーリング間隔 | 720000ms (12分) | 1時間に5回 |
| HTTPタイムアウト | 30000ms (30秒) | リクエストタイムアウト |
| リトライ回数 | 3 | 失敗時のリトライ |
| トークン更新 | 3600000ms (1時間) | 定期的な再ログイン |

## 送信データフォーマット案

### CelestialGlobe送信

```json
{
  "auth": {
    "tid": "T2025...",
    "lacisId": "30100...",
    "cic": "123456"
  },
  "payload": {
    "observedAt": "2025-12-22T06:00:00Z",
    "source": "ar-is10m",
    "mercuryAc": {
      "host": "192.168.96.5",
      "model": "MAC100",
      "apCount": 20,
      "clientCount": 18
    },
    "clients": [
      {
        "mac": "6C-C8-40-8C-CD-4C",
        "apName": "ap02-10",
        "ssid": "cluster1",
        "ip": "192.168.97.15",
        "band": "2.4GHz",
        "rssi": -57
      }
    ],
    "aps": [
      {
        "name": "ap02-10",
        "mac": "4C-B7-E0-59-FC-EF",
        "ip": "192.168.96.19",
        "online": true,
        "clients": 1
      }
    ]
  }
}
```

## 次のステップ

### Phase 1: API検証 ✅ 完了（2025/12/22 最終更新）

1. [x] ログインAPIのリクエスト方式特定 → `POST /` JSON API
2. [x] パスワード暗号化方式の確認 → **XORベースsecurityEncode**（MD5/RSAではない）
3. [x] クライアント/AP一覧取得方法特定 → **JSON API方式**（HTMLスクレイピング不要）
4. [x] APIパラメータ特定 → `filter:[{group_id:["-1"]}]` が必須
5. [x] レスポンス構造解析完了

### Phase 2: 基本実装（次フェーズ）

1. [ ] MercuryACScraper基本実装（JSON APIクライアント）
2. [ ] securityEncode()パスワードエンコード実装
3. [ ] ログイン・トークン管理
4. [ ] クライアント/AP情報取得・URLデコード処理
5. [ ] Web管理UI（AC設定画面）
6. [ ] ESP32 HTTPClientでの接続テスト

### Phase 3: 統合

1. [ ] CelestialSenderIs10m実装
2. [ ] StateReporter対応
3. [ ] OTA対応

## ネットワーク構成

- Mercury ACはLAN内からのみアクセス可能（192.168.96.5）
- **is10mはVPN経由でアクセス**（192.168.3.0/24 → 192.168.96.0/24）
- mDNS/ブロードキャスト系機能は使用不可
- セキュリティ・プライバシー考慮不要（自社施設・機器）

## 技術的考慮事項

### ページネーション対応

Mercury ACはページネーション機能あり:
- **ページサイズオプション**: 10, 20, 50, 100, 200, 500, 1000
- **対策**: `slt_item_num=1000` パラメータで最大1000件/ページ取得
- 1000件を超える場合は複数ページ取得が必要（現実的には発生しない想定）

### ESP32メモリ制約

#### JSON API方式のメモリ使用量（HTML比較）

| 項目 | HTML方式 | JSON API方式 |
|------|----------|--------------|
| フルレスポンス | ~230KB | ~15KB (100クライアント) |
| 1クライアント | ~675バイト | ~150バイト |
| パース用バッファ | 要ストリーム処理 | ArduinoJson DynamicJsonDocument |
| ESP32利用可能RAM | ~160KB | ~160KB |

**JSON API方式の利点**: メモリに余裕があり、ArduinoJsonで安全にパース可能

#### 推奨実装

```cpp
// ArduinoJsonでパース（DynamicJsonDocument使用）
DynamicJsonDocument doc(16384);  // 16KB（100クライアント対応）

HTTPClient http;
http.begin(url);
http.addHeader("Content-Type", "application/json; charset=UTF-8");
int httpCode = http.POST(requestJson);

if (httpCode == 200) {
    String response = http.getString();
    DeserializationError error = deserializeJson(doc, response);
    if (!error) {
        JsonArray clients = doc["apmng_client"]["client_list"];
        for (JsonObject client : clients) {
            // クライアント情報抽出
        }
    }
}
```

**実装方針**:
1. HTTPリクエストでJSON API呼び出し
2. ArduinoJsonでレスポンスパース
3. URLデコード処理（%5b→[等）
4. MercuryClient/MercuryAP構造体に変換
5. コールバックで通知

### 最大クライアント数設計

```cpp
#define MAX_MERCURY_CLIENTS 200  // 最大追跡クライアント数
#define MAX_MERCURY_APS 50       // 最大AP数

// 1クライアント: MAC(18) + IP(16) + AP名(32) + SSID(32) + RSSI(4) ≈ 102バイト
// 200クライアント × 102バイト ≈ 20KB（許容範囲内）
```
