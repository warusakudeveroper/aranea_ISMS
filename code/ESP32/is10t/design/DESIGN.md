# is10t - Tapo Camera Monitor 設計書

**正式名称**: Aranea Tapo Monitor
**製品コード**: AR-IS10T
**作成日**: 2025/12/23
**シリーズ**: is10系（ネットワーク機器監視デバイス）
**目的**: Tapoカメラの稼働状態・デバイス情報をONVIF+Discovery経由で取得

---

## 1. デバイス概要

### 1.1 機能概要

- **ONVIF経由監視**: Tapoカメラからデバイス情報・ネットワーク状態を取得
- **Discovery情報取得**: device_name（日本語名）・device_id取得
- **複数カメラ巡回**: 最大8台のTapoカメラを順次監視
- **CelestialGlobe連携**: is10m（Mercury AC）データとMAC照合でトポロジー構築

### 1.2 データ統合アーキテクチャ

```
┌─────────────┐                    ┌─────────────┐
│   is10t     │                    │   is10m     │
│ Tapo Monitor│                    │ Mercury AC  │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ カメラ情報                        │ クライアント情報
       │ - MAC (照合キー)                 │ - MAC
       │ - device_name                    │ - SSID, RSSI
       │ - Model, FW, IP...               │ - AP名
       ↓                                  ↓
┌─────────────────────────────────────────────────┐
│              CelestialGlobe                     │
│         MAC照合 → トポロジー統合表示             │
└─────────────────────────────────────────────────┘
```

### 1.3 is10系シリーズ

| デバイス | 役割 | プロトコル | 対象 |
|---------|------|-----------|------|
| is10 | SSHポーラー | SSH | ASUSWRT/OpenWrt |
| is10m | Mercury ACポーラー | HTTP JSON | Mercury MAC100 |
| is10D | ARPスキャナー | ARP | サブネット内デバイス |
| **is10t** | Tapoカメラモニター | **ONVIF SOAP** | Tapo C530WS等 |

---

## 2. ESP32 リソース制約と設計

### 2.1 メモリ制約

| リソース | 利用可能 | 使用見込み | 備考 |
|---------|---------|-----------|------|
| RAM総量 | 320KB | - | ESP32-WROOM-32 |
| ヒープ実効 | 160-200KB | - | フラグメンテーション考慮 |
| 1カメラ情報 | - | 約350バイト | TapoStatus構造体 |
| SOAPバッファ | - | 4KB | リクエスト/レスポンス用 |
| HTTPクライアント | - | 20-30KB | TLS無し（ポート2020） |
| 最大カメラ数 | - | **8台** | 350B × 8 = 2.8KB |

### 2.2 巡回可能数の算出

| パラメータ | 値 | 根拠 |
|-----------|-----|------|
| **最大カメラ数** | **8台** | メモリ・処理時間のバランス |
| 1カメラあたり処理時間 | 5-8秒 | ONVIF 3リクエスト + Discovery |
| カメラ間待機 | 2秒 | ネットワーク負荷軽減 |
| 全カメラ1周 | 56-80秒 | (8秒+2秒) × 8台 |
| ポーリング間隔 | 5分 | 1時間12回 |

### 2.3 処理負荷

| 処理 | 負荷 | 対策 |
|------|------|------|
| SHA1ダイジェスト | 低 | mbedTLS HW支援 |
| XMLパース | 中 | 軽量文字列検索方式 |
| HTTPS(TLS) | 高 | **使用しない**（ONVIF:2020はHTTP） |
| 複数同時接続 | 高 | **1台ずつ順次処理** |

---

## 3. 取得情報一覧

### 3.1 ONVIF経由（ポート2020）

| 情報 | API | ESP32実装 |
|------|-----|-----------|
| Model | GetDeviceInformation | ○ |
| FirmwareVersion | GetDeviceInformation | ○ |
| SerialNumber | GetDeviceInformation | ○ |
| HardwareId | GetDeviceInformation | ○ |
| Hostname | GetHostname | ○ |
| MAC | GetNetworkInterfaces | ○ |
| IP | GetNetworkInterfaces | ○ |
| Subnet | GetNetworkInterfaces | ○ |
| Gateway | GetNetworkDefaultGateway | ○ |
| DNS | GetDNS | △（オプション） |
| DHCP状態 | GetNetworkInterfaces | ○ |
| SystemDateTime | GetSystemDateAndTime | △（オプション） |
| Resolution | GetVideoSources | △（オプション） |

### 3.2 Discovery経由（HTTPS:443）

| 情報 | 認証 | ESP32実装 |
|------|------|-----------|
| device_id | 不要 | ○ |
| device_name（日本語） | 不要 | ○ |
| device_type | 不要 | ○ |
| encrypt_type | 不要 | 情報のみ |

### 3.3 is10m経由（MAC照合でCelestialGlobeが統合）

| 情報 | 取得元 | 備考 |
|------|--------|------|
| SSID | Mercury AC | クライアントテーブル |
| RSSI | Mercury AC | 信号強度 |
| AP名 | Mercury AC | 接続先AP |
| 接続日時 | Mercury AC | - |

---

## 4. ONVIF認証実装

### 4.1 WS-Security UsernameToken (Digest)

```cpp
// 認証ヘッダ生成
String generateWsSecurityHeader(const char* username, const char* password) {
    // 1. nonce生成（16バイト乱数）
    uint8_t nonce[16];
    esp_fill_random(nonce, 16);
    String nonceB64 = base64Encode(nonce, 16);

    // 2. created生成（ISO8601形式）
    String created = getIso8601Time();  // "2025-12-23T00:00:00Z"

    // 3. PasswordDigest = Base64(SHA1(nonce + created + password))
    uint8_t digestInput[256];
    size_t len = 0;
    memcpy(digestInput + len, nonce, 16); len += 16;
    memcpy(digestInput + len, created.c_str(), created.length()); len += created.length();
    memcpy(digestInput + len, password, strlen(password)); len += strlen(password);

    uint8_t sha1Hash[20];
    mbedtls_sha1(digestInput, len, sha1Hash);
    String digestB64 = base64Encode(sha1Hash, 20);

    // 4. SOAPヘッダ構築
    return buildSecurityHeader(username, digestB64, nonceB64, created);
}
```

### 4.2 SOAPリクエスト構造

```xml
<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd"
            xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
  <s:Header>
    <wsse:Security>
      <wsse:UsernameToken>
        <wsse:Username>{username}</wsse:Username>
        <wsse:Password Type="...#PasswordDigest">{digest}</wsse:Password>
        <wsse:Nonce>{nonce}</wsse:Nonce>
        <wsu:Created>{created}</wsu:Created>
      </wsse:UsernameToken>
    </wsse:Security>
  </s:Header>
  <s:Body>
    <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>
```

---

## 5. モジュール構成

```
is10t/
├── is10t.ino                 # メインスケッチ
├── TapoPollerIs10t.h         # メインポーラー
├── TapoPollerIs10t.cpp
├── TapoTypes.h               # データ型定義
├── OnvifClient.h             # ONVIF SOAP通信
├── OnvifClient.cpp
├── OnvifAuth.h               # WS-Security認証
├── OnvifAuth.cpp
├── TapoDiscovery.h           # HTTPS Discovery
├── TapoDiscovery.cpp
├── HttpManagerIs10t.h        # Web管理UI
├── HttpManagerIs10t.cpp
├── CelestialSenderIs10t.h    # クラウド送信
├── CelestialSenderIs10t.cpp
└── design/
    └── DESIGN.md
```

### 5.1 TapoTypes.h

```cpp
#pragma once

#define MAX_TAPO_CAMERAS 8

// カメラ設定
struct TapoConfig {
    char host[16];           // IPアドレス
    uint16_t onvifPort;      // ONVIFポート（2020）
    char username[32];       // ONVIFユーザー
    char password[64];       // ONVIFパスワード
    bool enabled;            // 有効フラグ
};

// カメラステータス（約350バイト/台）
struct TapoStatus {
    // 識別情報
    char deviceId[40];       // Discovery: device_id
    char deviceName[64];     // Discovery: device_name（日本語対応）

    // デバイス情報（ONVIF）
    char model[32];          // Tapo C530WS
    char firmware[48];       // 1.3.3 Build...
    char serialNumber[16];   // 74614817
    char hostname[32];       // C530WS

    // ネットワーク情報（ONVIF）
    uint8_t mac[6];          // MACアドレス（照合キー）
    uint32_t ip;             // IPアドレス
    uint32_t gateway;        // ゲートウェイ
    uint8_t prefixLength;    // サブネット
    bool dhcp;               // DHCP有効

    // 状態
    bool online;             // オンライン状態
    unsigned long lastPollMs;// 最終ポーリング時刻
    uint8_t failCount;       // 連続失敗回数（3で offline）
};
```

---

## 6. タイミング設計

| パラメータ | 値 | 説明 |
|-----------|-----|------|
| POLL_INTERVAL_MS | 300000 (5分) | 全カメラポーリング間隔 |
| CAMERA_GAP_MS | 2000 (2秒) | カメラ間待機時間 |
| HTTP_TIMEOUT_MS | 8000 (8秒) | HTTPタイムアウト |
| FAIL_THRESHOLD | 3 | オフライン判定閾値 |
| CELESTIAL_INTERVAL_MS | 300000 (5分) | クラウド送信間隔 |

### 6.1 処理フロー

```
[起動]
   │
   ├─→ WiFi接続
   ├─→ NTP同期（ONVIF認証に必須）
   ├─→ 設定読み込み（NVS）
   │
[メインループ]
   │
   ├─→ カメラ0: ONVIF取得 → Discovery取得 → ステータス更新
   │         ↓ (2秒待機)
   ├─→ カメラ1: ONVIF取得 → Discovery取得 → ステータス更新
   │         ↓ (2秒待機)
   │        ...
   ├─→ カメラ7: ONVIF取得 → Discovery取得 → ステータス更新
   │
   ├─→ CelestialGlobe送信（5分間隔）
   │
   └─→ 5分待機 → ループ
```

---

## 7. CelestialGlobe送信フォーマット

```json
{
  "auth": {
    "tid": "T2025120608261484221",
    "lacisId": "30100...",
    "cic": "123456"
  },
  "payload": {
    "observedAt": "2025-12-23T09:00:00Z",
    "source": "ar-is10t",
    "cameras": [
      {
        "mac": "BC:07:1D:B9:48:17",
        "online": true,
        "discovery": {
          "deviceId": "DE8E8A1FD448303DA2973B0EAB21C64E",
          "deviceName": "1F入口-冷風乾燥"
        },
        "onvif": {
          "model": "Tapo C530WS",
          "firmware": "1.3.3 Build 250923",
          "serial": "74614817",
          "hostname": "C530WS"
        },
        "network": {
          "ip": "192.168.96.85",
          "gateway": "192.168.96.1",
          "dhcp": true
        },
        "lastSeen": "2025-12-23T08:59:55Z"
      }
    ],
    "summary": {
      "total": 1,
      "online": 1,
      "offline": 0
    }
  }
}
```

**CelestialGlobe側でMAC照合**:
- is10tの `mac: "BC:07:1D:B9:48:17"`
- is10mの `client.mac: "BC-07-1D-B9-48-17"`
- → SSID, RSSI, AP名を統合表示

---

## 8. NVS設定項目

### 8.1 必須設定（AraneaDeviceGate用）

| キー | 型 | 説明 |
|------|-----|------|
| `gate_url` | string | AraneaDeviceGate URL |
| `tid` | string | テナントID |
| `cic` | string | 自デバイスのCIC |

### 8.2 カメラ設定

| キー | 型 | 説明 |
|------|-----|------|
| `tapo_count` | int | 監視カメラ数（1-8） |
| `tapo_0_host` | string | カメラ0のIPアドレス |
| `tapo_0_port` | int | カメラ0のONVIFポート（デフォルト2020） |
| `tapo_0_user` | string | カメラ0のユーザー名 |
| `tapo_0_pass` | string | カメラ0のパスワード |
| `tapo_0_enabled` | bool | カメラ0の有効フラグ |

---

## 9. Web UI

### 9.1 エンドポイント

| パス | メソッド | 説明 |
|------|---------|------|
| `/` | GET | ダッシュボード |
| `/config` | GET | 設定画面 |
| `/api/status` | GET | 全カメラ状態 |
| `/api/cameras` | GET/POST | カメラ一覧・追加 |
| `/api/cameras/{id}` | GET/PUT/DELETE | 個別カメラ操作 |
| `/api/poll` | POST | 手動ポーリング |

### 9.2 ダッシュボード表示

```
=== is10t Tapo Monitor ===
IP: 192.168.96.100 | RSSI: -55 dBm

[カメラ一覧] (1/8台)
┌──────────────────────────────────────────┐
│ #1 [ONLINE] 1F入口-冷風乾燥              │
│    IP: 192.168.96.85                     │
│    MAC: BC:07:1D:B9:48:17                │
│    Model: Tapo C530WS                    │
│    FW: 1.3.3 Build 250923                │
│    Last: 2025-12-23 09:14:55             │
└──────────────────────────────────────────┘

次回ポーリング: 4分32秒後
CelestialGlobe: 4分32秒後
```

---

## 10. 共通コンポーネント使用

| モジュール | 使用 | 備考 |
|-----------|------|------|
| WiFiManager | ○ | APモード/STA切替 |
| SettingManager | ○ | NVS永続化 |
| DisplayManager | ○ | I2C OLED表示 |
| **NtpManager** | **○必須** | ONVIF認証に時刻必須 |
| LacisIDGenerator | ○ | lacisID生成 |
| AraneaRegister | ○ | CIC取得 |
| AraneaWebUI | ○ | Web UI基底 |
| HttpOtaManager | ○ | OTA更新 |
| Operator | ○ | 状態機械 |

---

## 11. 実装上の注意

### 11.1 NTP同期必須

ONVIF WS-Security認証は `wsu:Created` タイムスタンプを使用。
サーバー時刻と大きくずれると認証失敗するため、**起動時に必ずNTP同期完了を待つ**。

```cpp
void setup() {
    // ...
    ntpManager.begin();
    while (!ntpManager.isSynced()) {
        delay(100);
    }
    tapoPoller.begin();  // NTP同期後に開始
}
```

### 11.2 軽量XMLパース

ESP32ではDOM/SAXパーサーは重いため、文字列検索方式を使用：

```cpp
String extractElement(const String& xml, const String& tag) {
    String startTag = "<" + tag + ">";
    String endTag = "</" + tag + ">";
    int start = xml.indexOf(startTag);
    if (start < 0) {
        // 名前空間付きタグも検索 (例: <tds:Model>)
        startTag = ":" + tag + ">";
        start = xml.indexOf(startTag);
        if (start < 0) return "";
        start = xml.lastIndexOf('<', start);
    }
    int end = xml.indexOf(endTag, start);
    if (end < 0) return "";
    return xml.substring(start + startTag.length(), end);
}
```

### 11.3 メモリ管理

SOAPリクエスト/レスポンスは動的確保し、使用後すぐ解放：

```cpp
bool OnvifClient::getDeviceInfo(TapoStatus* status) {
    char* buffer = (char*)malloc(SOAP_BUFFER_SIZE);  // 4KB
    if (!buffer) return false;

    // ... SOAP通信 ...

    free(buffer);
    return true;
}
```

---

## 12. 検証済み情報（2025/12/23）

### 12.1 対象カメラ

| 項目 | 値 |
|------|-----|
| モデル | Tapo C530WS |
| ファームウェア | 1.3.3 Build 250923 |
| ONVIFポート | 2020 (HTTP) |
| 認証方式 | WS-Security Digest (SHA1) |
| 認証情報 | ismscam / isms12345@ |

### 12.2 ONVIF API動作確認

| API | 状態 |
|-----|------|
| GetDeviceInformation | ○ |
| GetNetworkInterfaces | ○ |
| GetHostname | ○ |
| GetNetworkDefaultGateway | ○ |
| GetDNS | ○ |
| GetSystemDateAndTime | ○ |

### 12.3 Discovery動作確認

| 方式 | 状態 | 備考 |
|------|------|------|
| python-kasa | ○ | device_name取得可 |
| UDP Broadcast | × | VPN経由では不可 |
| HTTPS直接 | △ | 認証エラー（情報取得は可能） |

---

## 13. 次のステップ

### Phase 1: 基本実装

1. [ ] OnvifAuth実装（WS-Security Digest）
2. [ ] OnvifClient実装（SOAP通信）
3. [ ] TapoDiscovery実装（HTTPS経由）
4. [ ] TapoPollerIs10t統合
5. [ ] 単体接続テスト

### Phase 2: 巡回・UI

1. [ ] 複数カメラ巡回ロジック
2. [ ] Web UI実装
3. [ ] 設定画面（カメラ追加/編集）

### Phase 3: クラウド連携

1. [ ] CelestialSenderIs10t実装
2. [ ] is10mとのMAC照合ロジック（CelestialGlobe側）
3. [ ] OTA対応

---

## 14. 参照

- **is10 (SSHポーラー)**: `code/ESP32/is10/`
- **is10m (Mercury AC)**: `code/ESP32/is10m/`
- **is10D (ARPスキャナー)**: `code/ESP32/is10D/`
- **global モジュール**: `code/ESP32/global/src/`
- **ONVIF仕様**: https://www.onvif.org/specs/core/ONVIF-Core-Specification.pdf
