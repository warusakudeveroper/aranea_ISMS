# is10D - ARP Scanner 設計書

**正式名称**: Aranea ARP Scanner
**製品コード**: AR-IS10D
**作成日**: 2025/12/22
**シリーズ**: is10系（ルーター連携デバイス）
**目的**: サブネット内ARPスキャンによるネットワーク監視・デバイス検出

---

## 1. デバイス概要

### 1.1 機能概要

- **ARPスキャン**: 接続サブネット内のデバイス検出
- **デバイス情報取得**: デバイス名、IP、MAC（取得可能な場合）
- **ネットワーク監視**: 機器数をCelestialGlobeに送信
- **疎通確認**: 自機の発信でネットワーク生存を通知
- **WiFi環境対応**: OpenWrt不可環境でのネットワーク状況把握

### 1.2 ユースケース

| 用途 | 説明 |
|------|------|
| ネットワーク監視 | サブネット内のデバイス数監視 |
| 疎通確認 | 重要機器の近傍に配置して監視 |
| 非管理ルーター環境 | OpenWrt不可のルーター配下での監視 |
| WiFi死活監視 | WiFi APの動作確認 |

### 1.3 is10系シリーズ

| デバイス | 役割 |
|---------|------|
| is10 | SSHポーラー（ASUSWRT/OpenWrt） |
| is10m | Mercury ACポーラー |
| **is10D** | ARPスキャナー |

---

## 2. 動作仕様

### 2.1 ARPスキャン

```
┌─────────────────────────────────────────────────────────────┐
│                    ARPスキャン実行                          │
│                                                             │
│  1. 自IPからサブネット計算（/24想定）                        │
│  2. 1-254までARP Request送信                                │
│  3. ARP Reply受信・デバイス情報記録                         │
│  4. 結果をCelestialGlobeに送信                              │
│  5. インターバル待機後に再スキャン                          │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 スキャン負荷対策

**重要**: サブネットをスキャンする際の負荷に注意が必要

```cpp
// 負荷軽減のための設定
struct ScanConfig {
    int batchSize = 10;      // 一度に送信するARP数
    int batchDelayMs = 100;  // バッチ間のディレイ
    int arpTimeoutMs = 500;  // ARP応答タイムアウト
    int scanIntervalSec = 300; // スキャン間隔（5分）
};
```

### 2.3 取得情報

| 項目 | 取得方法 | 備考 |
|------|---------|------|
| IPアドレス | ARP Reply | 必ず取得可能 |
| MACアドレス | ARP Reply | 必ず取得可能 |
| ホスト名 | mDNS/NetBIOS | 取得できない場合あり |

---

## 3. ハードウェア仕様

### 3.1 GPIO割り当て

| GPIO | 機能 | 説明 |
|------|------|------|
| 21 | I2C_SDA | OLED SDA |
| 22 | I2C_SCL | OLED SCL |
| 25 | BTN_WIFI | WiFi再接続（3秒長押し） |
| 26 | BTN_RESET | ファクトリーリセット（5秒長押し） |

### 3.2 ネットワーク要件

- **WiFi**: 2.4GHz対応
- **サブネット**: /24推奨（最大254ホスト）
- **特権**: 不要（ESPでのARPは特権不要）

---

## 4. ソフトウェア設計

### 4.1 設計原則（全デバイス共通）

```
重要: ESP32では以下を遵守
- セマフォとWDTの過剰制御を避ける
- 監査系関数を入れすぎない
- コードのシンプル化
- 可能な限りシングルタスクで実装
- パーティション: min_SPIFFS使用
```

### 4.2 ARPスキャナー

```cpp
class ArpScanner {
public:
    void begin();
    void setScanConfig(const ScanConfig& config);

    // スキャン実行
    void startScan();
    bool isScanning() const;
    void update();  // loop()で呼び出し

    // 結果取得
    int getDeviceCount() const;
    std::vector<DeviceInfo> getDevices() const;

private:
    ScanConfig config_;
    std::vector<DeviceInfo> devices_;
    bool scanning_;
    int currentIp_;

    void sendArpRequest(uint32_t targetIp);
    void processArpReply();
};

struct DeviceInfo {
    uint32_t ip;
    uint8_t mac[6];
    String hostname;  // 取得できない場合は空
    unsigned long lastSeenMs;
};
```

### 4.3 メインループ

```cpp
void loop() {
    // 1. スキャン更新
    arpScanner.update();

    // 2. スキャン完了時に送信
    if (arpScanner.isComplete()) {
        sendToCelestialGlobe();
        arpScanner.scheduleNextScan();
    }

    // 3. Web UI処理
    webServer.handleClient();

    // 4. MQTT処理
    mqtt.loop();
}
```

---

## 5. 通信仕様

### 5.1 CelestialGlobe送信

```json
POST /api/device/{lacisId}/telemetry
{
    "type": "arp_scan",
    "timestamp": "2025-12-22T08:00:00Z",
    "network": {
        "self_ip": "192.168.1.100",
        "subnet": "192.168.1.0/24",
        "gateway": "192.168.1.1"
    },
    "scan_result": {
        "device_count": 15,
        "scan_duration_ms": 12500,
        "devices": [
            {
                "ip": "192.168.1.1",
                "mac": "AA:BB:CC:DD:EE:FF",
                "hostname": "router.local"
            },
            {
                "ip": "192.168.1.50",
                "mac": "11:22:33:44:55:66",
                "hostname": ""
            }
        ]
    }
}
```

### 5.2 疎通通知（心拍）

```json
POST /api/device/{lacisId}/heartbeat
{
    "type": "is10d_heartbeat",
    "timestamp": "2025-12-22T08:00:00Z",
    "network": {
        "self_ip": "192.168.1.100",
        "rssi": -55,
        "gateway_reachable": true
    },
    "last_scan": {
        "device_count": 15,
        "scanned_at": "2025-12-22T07:55:00Z"
    }
}
```

---

## 6. NVS設定項目

### 6.1 必須設定（AraneaDeviceGate用）

| キー | 型 | 説明 |
|------|-----|------|
| `gate_url` | string | AraneaDeviceGate URL |
| `tid` | string | テナントID |
| `tenant_lacisid` | string | テナントプライマリのlacisID |
| `tenant_email` | string | テナントプライマリのEmail |
| `tenant_cic` | string | テナントプライマリのCIC |
| `cic` | string | 自デバイスのCIC |

### 6.2 スキャン設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `scan_interval_sec` | int | 300 | スキャン間隔（秒） |
| `batch_size` | int | 10 | バッチサイズ |
| `batch_delay_ms` | int | 100 | バッチ間ディレイ |
| `arp_timeout_ms` | int | 500 | ARPタイムアウト |
| `heartbeat_interval_sec` | int | 60 | 心拍送信間隔 |

### 6.3 送信設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `celestial_url` | string | "" | CelestialGlobe URL |
| `send_full_list` | bool | true | 全デバイスリスト送信 |
| `send_count_only` | bool | false | デバイス数のみ送信 |

---

## 7. Web UI

### 7.1 エンドポイント

| パス | メソッド | 説明 |
|------|---------|------|
| `/` | GET | ダッシュボード |
| `/config` | GET | 設定画面 |
| `/api/status` | GET | 現在の状態 |
| `/api/config` | GET/POST | 設定取得/更新 |
| `/api/scan` | POST | 手動スキャン実行 |
| `/api/devices` | GET | 検出デバイス一覧 |
| `/api/reboot` | POST | 再起動 |

### 7.2 ダッシュボード

```
=== is10D ARP Scanner ===
IP: 192.168.1.100 | RSSI: -55 dBm
Subnet: 192.168.1.0/24

[最新スキャン] 2025-12-22 08:00:00
検出デバイス: 15台

[デバイス一覧]
192.168.1.1   AA:BB:CC:DD:EE:FF  router.local
192.168.1.50  11:22:33:44:55:66  -
192.168.1.51  22:33:44:55:66:77  printer.local
...

次回スキャン: 4分32秒後
```

---

## 8. 共通コンポーネント使用

| モジュール | 使用 | 備考 |
|-----------|------|------|
| WiFiManager | ○ | APモード/STA切替対応 |
| SettingManager | ○ | NVS永続化 |
| DisplayManager | ○ | I2C OLED表示 |
| NtpManager | ○ | 時刻同期 |
| MqttManager | ○ | MQTT接続 |
| LacisIDGenerator | **○必須** | lacisID生成 |
| AraneaRegister | **○必須** | CIC取得 |
| AraneaWebUI | ○ | Web UI基底クラス |
| HttpOtaManager | ○ | OTA更新 |
| Operator | ○ | 状態機械 |

---

## 9. 注意事項

### 9.1 スキャン負荷

- 一度に大量のARP送信はネットワーク負荷増大
- `batch_size` と `batch_delay_ms` で調整
- 本番環境では `scan_interval_sec` を5分以上推奨

### 9.2 ホスト名取得

- mDNS対応デバイスのみホスト名取得可能
- Windows PCはNetBIOS名を取得可能な場合あり
- 取得できない場合は空文字

### 9.3 セキュリティ

- ARPスキャンは受動的なネットワーク調査
- 攻撃的な行為ではないが、企業ネットワークでは事前許可推奨

---

## 10. 参照

- **is10 (SSHポーラー)**: `code/ESP32/is10/`
- **is10m (Mercury AC)**: `code/ESP32/is10m/`
- **global モジュール**: `code/ESP32/global/src/`
- **CelestialGlobe API**: 別途ドキュメント参照
