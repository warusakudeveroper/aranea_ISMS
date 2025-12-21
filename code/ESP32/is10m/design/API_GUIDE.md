# Mercury AC (MAC100) API Guide

**検証日**: 2025/12/22
**検証環境**: curl (ESP32 HTTPClient互換)
**対象機器**: Mercury MAC100 v6.0 (Firmware 1.0.1 Build 250211)
**対象IP**: 192.168.96.5

---

## 1. 認証

### 1.1 パスワードエンコード

Mercury ACは **XORベースのsecurityEncode** を使用（MD5/RSAではない）

```cpp
// C++ Implementation for ESP32
String securityEncode(const String& password) {
    const char* KEY1 = "RDpbLfCPsJZ7fiv";
    const char* KEY2 = "yLwVl0zKqws7LgKPRQ84Mdt708T1qQ3Ha7xv3H7NyU84p21BriUWBU43odz3iP4rBL3cD02KZciXTysVXiV8ngg6vL48rPJyAUw0HurW20xqxv9aYb4M9wK1Ae0wlro510qXeU07kV57fQMc8L6aLgMLwygtc0F10a0Dg70TOoouyFhdysuRMO51yY5ZlOZZLEal1h0t9YQW0Ko7oBwmCAHoic4HYbUyVeU3sfQ1xtXcPcf1aT303wAQhv66qzW";

    String result = "";
    int g = password.length();
    int m = strlen(KEY1);
    int k = strlen(KEY2);
    int h = max(g, m);

    for (int f = 0; f < h; f++) {
        int l = 187, t = 187;
        if (f >= g) {
            t = KEY1[f];
        } else if (f >= m) {
            l = password[f];
        } else {
            l = password[f];
            t = KEY1[f];
        }
        result += KEY2[(l ^ t) % k];
    }
    return result;
}
```

**テスト例**: `"isms12345@"` → `"33QQrnYH2sefbwK"`

### 1.2 ログインAPI

```
POST http://{AC_IP}/
Content-Type: application/json; charset=UTF-8
```

**リクエスト**:
```json
{
    "method": "do",
    "login": {
        "username": "isms",
        "password": "33QQrnYH2sefbwK",
        "force": 1
    }
}
```

> **重要**: `force`パラメータは**必ず`1`を使用**すること。Mercury ACは1ユーザー1セッション制のため、`force:0`だと既存セッションと競合してトークンが無効になる可能性がある。

**レスポンス (成功)**:
```json
{
    "stok": "0bd1c4a5a22c4781cf507edb43f0a7b3",
    "role": "sys_admin",
    "error_code": 0
}
```

**トークン**: `stok` フィールド（32文字HEX）を後続API呼び出しに使用

---

## 2. クライアントリスト取得

### 2.1 APIエンドポイント

```
POST http://{AC_IP}/stok={TOKEN}/ds
Content-Type: application/json; charset=UTF-8
```

### 2.2 リクエスト

```json
{
    "method": "get",
    "apmng_client": {
        "table": "client_list",
        "filter": [{"group_id": ["-1"]}],
        "para": {"start": 0, "end": 99}
    }
}
```

**重要パラメータ**:
| パラメータ | 必須 | 説明 |
|-----------|------|------|
| `filter.group_id` | **必須** | `["-1"]` = 全グループ。省略すると空配列が返る |
| `para.start` | 推奨 | 取得開始位置（0から） |
| `para.end` | 推奨 | 取得終了位置（99で100件） |

### 2.3 レスポンス

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
                    "encode": "1",
                    "rf_entry": [
                        {
                            "freq_name": "2.4GHz",
                            "freq_unit": "1",
                            "rssi": "-51",
                            "nego_rate": "0"
                        }
                    ]
                }
            }
        ]
    }
}
```

### 2.4 クライアントフィールド

| フィールド | 型 | 説明 | URLデコード |
|-----------|-----|------|-------------|
| `mac` | string | MACアドレス（XX-XX-XX-XX-XX-XX形式） | 不要 |
| `ip` | string | IPv4アドレス | 不要 |
| `ipv6` | string | IPv6アドレス | 必要（%3a = :） |
| `ssid` | string | 接続SSID | 不要 |
| `ap_name` | string | 接続先AP名 | **必要**（%5b = [, %5d = ]） |
| `connect_date` | string | 接続日付 | 必要（%2f = /） |
| `connect_time` | string | 接続時刻 | 必要（%3a = :） |
| `rf_entry[].freq_name` | string | 周波数帯（2.4GHz/5GHz） | 不要 |
| `rf_entry[].rssi` | string | 信号強度（dBm） | 不要 |

---

## 3. APリスト取得

### 3.1 APIエンドポイント

```
POST http://{AC_IP}/stok={TOKEN}/ds
Content-Type: application/json; charset=UTF-8
```

### 3.2 リクエスト

```json
{
    "method": "get",
    "apmng_status": {
        "table": "ap_list",
        "filter": [{"group_id": ["-1"]}],
        "para": {"start": 0, "end": 99}
    }
}
```

### 3.3 レスポンス

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
                    "telnet_status": "0",
                    "online_time": "576576",
                    "entry_id": "0",
                    "rf_entry": [
                        {
                            "freq_name": "2.4GHz",
                            "freq_unit": "1",
                            "channel": "11",
                            "rf_client_num": "1"
                        },
                        {
                            "freq_name": "5GHz",
                            "freq_unit": "2",
                            "channel": "161",
                            "rf_client_num": "0"
                        }
                    ]
                }
            }
        ]
    }
}
```

### 3.4 APフィールド

| フィールド | 型 | 説明 | URLデコード |
|-----------|-----|------|-------------|
| `mac` | string | APのMACアドレス | 不要 |
| `entry_name` | string | AP名 | **必要** |
| `ip` | string | APのIPアドレス | 不要 |
| `model_name` | string | モデル名 | 不要 |
| `link_status` | string | 接続状態（1=オンライン） | 不要 |
| `online_time` | string | 稼働時間（秒） | 不要 |
| `rf_entry[].freq_name` | string | 周波数帯 | 不要 |
| `rf_entry[].channel` | string | チャンネル | 不要 |
| `rf_entry[].rf_client_num` | string | 接続クライアント数 | 不要 |

---

## 4. エラーコード

| コード | 意味 | 対処 |
|--------|------|------|
| 0 | 成功 | - |
| -40401 | 無効なトークン | 再ログイン（force=1） |
| -40209 | パラメータエラー | リクエスト形式確認 |

---

## 5. セッション管理とタイミング（重要）

### 5.1 検証結果

| 項目 | 結果 | 備考 |
|------|------|------|
| ログイン応答時間 | ~200ms | ESP32 HTTPClient互換 |
| データ取得応答時間 | ~300ms | 16クライアント時 |
| 無効トークン応答時間 | ~280ms | すぐにエラー返却 |
| ログイン後の待機 | **不要** | 即座にリクエスト可能 |
| 同一トークン連続利用 | **可能** | 複数リクエストOK |

### 5.2 セッション競合問題

Mercury ACは**1ユーザー1セッション制**：

```
Session A: login(force=0) → token_A取得
Session B: login(force=0) → token_B取得（token_Aが無効化される可能性）
```

**解決策**: 常に`force=1`でログインする。既存セッションを強制終了して新セッション確立。

### 5.3 ESP32推奨フロー

```
┌─────────────────────────────────────────────────────────────┐
│                    12分ごとのポーリング                      │
├─────────────────────────────────────────────────────────────┤
│  1. token_が空？                                            │
│     └─ Yes → login(force=1) → token_保存                   │
│                                                             │
│  2. fetchClients(token_)                                    │
│     ├─ error_code == 0 → 正常処理                          │
│     ├─ error_code == -40401 → login(force=1) → リトライ1回 │
│     └─ その他 → ログ出力、次サイクルへ                      │
│                                                             │
│  3. fetchAPs(token_)                                        │
│     └─ 同上のエラー処理                                     │
└─────────────────────────────────────────────────────────────┘
```

### 5.4 トークン保存

| 方式 | メリット | デメリット |
|------|----------|------------|
| メモリのみ | 実装シンプル | リセット時に再ログイン |
| SPIFFS | 再起動後も有効 | トークン期限切れの可能性 |

**推奨**: メモリのみ。12分間隔なら再ログインのオーバーヘッドは無視できる。

---

## 6. ESP32実装例

```cpp
#include <HTTPClient.h>
#include <ArduinoJson.h>

class MercuryACClient {
private:
    String host_;
    String token_;
    String username_;
    String password_;  // エンコード済み

    String securityEncode(const String& password);

public:
    MercuryACClient(const String& host, const String& user, const String& pass)
        : host_(host), username_(user) {
        password_ = securityEncode(pass);
    }

    // 常にforce=1でログイン（セッション競合回避）
    bool login() {
        HTTPClient http;
        http.begin("http://" + host_ + "/");
        http.addHeader("Content-Type", "application/json; charset=UTF-8");
        http.setTimeout(10000);  // 10秒タイムアウト

        String payload = "{\"method\":\"do\",\"login\":{\"username\":\"" + username_ +
                        "\",\"password\":\"" + password_ + "\",\"force\":1}}";

        int httpCode = http.POST(payload);
        if (httpCode == 200) {
            JsonDocument doc;
            deserializeJson(doc, http.getString());
            if (doc["error_code"] == 0) {
                token_ = doc["stok"].as<String>();
                http.end();
                return true;
            }
        }
        http.end();
        return false;
    }

    // クライアント取得（エラー時は自動再ログイン＋リトライ）
    int fetchClients(/* callback */) {
        int result = fetchClientsInternal();
        if (result == -40401) {
            // トークン無効 → 再ログインしてリトライ
            if (login()) {
                result = fetchClientsInternal();
            }
        }
        return result;
    }

private:
    int fetchClientsInternal() {
        if (token_.isEmpty()) return -1;

        HTTPClient http;
        http.begin("http://" + host_ + "/stok=" + token_ + "/ds");
        http.addHeader("Content-Type", "application/json; charset=UTF-8");
        http.setTimeout(15000);

        String payload = "{\"method\":\"get\",\"apmng_client\":{\"table\":\"client_list\","
                        "\"filter\":[{\"group_id\":[\"-1\"]}],\"para\":{\"start\":0,\"end\":99}}}";

        int httpCode = http.POST(payload);
        if (httpCode == 200) {
            JsonDocument doc;
            deserializeJson(doc, http.getString());
            int errorCode = doc["error_code"] | -1;
            if (errorCode == 0) {
                int count = doc["apmng_client"]["count"]["client_list"];
                // TODO: コールバックでクライアント情報を処理
                http.end();
                return count;
            }
            http.end();
            return errorCode;  // -40401など
        }
        http.end();
        return -1;  // HTTP error
    }
};
```

---

## 7. 検証エビデンス

以下のファイルに生のレスポンスを保存:
- `evidence/01_login_response.json` - ログイン成功レスポンス
- `evidence/02_client_list_response.json` - クライアントリスト（16件）
- `evidence/03_ap_list_response.json` - APリスト（18件）
- `evidence/04_invalid_token_response.json` - 無効トークン時のエラー
- `evidence/05_no_filter_response.json` - フィルター省略時（エラー-40209）
- `evidence/test_log.txt` - 全テストログ
- `evidence/timing_test_results.txt` - タイミング計測結果
- `evidence/session_conflict_results.txt` - セッション競合検証
- `evidence/token_expiry_results.txt` - トークン有効性検証
