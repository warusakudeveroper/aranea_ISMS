# IS-20S CelestialGlobe連携 データ仕様書

## 概要

IS-20S（Network Capture Gateway）とCelestialGlobeのデータ連携における辞書データ、トラフィックデータ、未知ドメインキャッシュの仕様を定義する。

---

## 1. ドメインサービス辞書

### 1.1 保存形式

**ファイル**: `/var/lib/is20s/domain_services.json`

```json
{
  "version": "2024.12.30",
  "services": {
    "pattern": {
      "service": "ServiceName",
      "category": "Category"
    }
  }
}
```

### 1.2 フィールド定義

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| `version` | string | × | 辞書バージョン（YYYY.MM.DD形式推奨） |
| `services` | object | ○ | パターン→サービス情報のマッピング |
| `services[pattern]` | string | - | マッチングパターン（キー） |
| `services[pattern].service` | string | ○ | サービス名（例: "Gmail", "YouTube"） |
| `services[pattern].category` | string | ○ | カテゴリ（例: "Mail", "Streaming"） |

### 1.3 マッチングルール

```python
def get_service(domain: str) -> Tuple[Optional[str], Optional[str]]:
    for pattern, info in services.items():
        if "." in pattern:
            # ドット付きパターン: 厳密マッチ
            if (domain == pattern or
                domain.endswith("." + pattern) or
                ("." + pattern + ".") in domain):
                return (info["service"], info["category"])
        else:
            # ドットなしパターン: 部分マッチ
            if pattern in domain.lower():
                return (info["service"], info["category"])
    return (None, None)
```

**マッチング優先順位**:
1. 辞書の登録順（先にマッチしたものが採用）
2. **詳細パターン（例: `mail.google.com`）を先に登録**
3. **汎用パターン（例: `google.com`）を後に登録**

### 1.4 カテゴリ一覧

| カテゴリ | 説明 | 例 |
|---------|------|-----|
| `Ad` | 広告・トラッキング | GoogleAds, Criteo, AppLovin |
| `AI` | AI・機械学習サービス | ChatGPT, Copilot, Gemini |
| `Analytics` | 分析・計測 | Google Analytics, Amplitude |
| `App` | アプリストア・配信 | AppStore, GooglePlay |
| `Auth` | 認証・ID管理 | AppleID, OAuth |
| `Backend` | バックエンド基盤 | Firebase, AWS Lambda |
| `CDN` | コンテンツ配信 | Akamai, Cloudflare |
| `Cloud` | クラウドサービス | AWS, Azure, GCP |
| `EC` | Eコマース | Amazon, 楽天 |
| `Entertainment` | エンターテイメント | ReelShort, DramaWave |
| `Finance` | 金融 | Banking apps |
| `Game` | ゲーム | Steam, PlayStation |
| `IoT` | IoTデバイス | Eufy, HonorCloud |
| `Mail` | メール | Gmail, Outlook |
| `Maps` | 地図 | GoogleMaps, AppleMaps |
| `Messenger` | メッセージング | LINE, WhatsApp |
| `Music` | 音楽 | AppleMusic, Spotify |
| `News` | ニュース | BBC, NHK |
| `Payment` | 決済 | PayPay, Alipay |
| `Photo` | 写真 | GooglePhotos, iCloudPhotos |
| `Productivity` | 生産性ツール | GoogleDrive, SharePoint |
| `Search` | 検索エンジン | Google, Bing, Yahoo |
| `SNS` | ソーシャルメディア | Instagram, Twitter |
| `Streaming` | 動画配信 | Netflix, YouTube |
| `System` | システム・インフラ | NTP, DNS |
| `Telecom` | 通信キャリア | KDDI, SoftBank |
| `Tracker` | トラッキング SDK | Adjust, AppsFlyer |
| `Travel` | 旅行 | Skyscanner, Airbnb |
| `VPN` | VPN | NordVPN, ExpressVPN |
| `Video` | ビデオ会議 | Zoom, Teams |

### 1.5 API

#### 一覧取得
```http
GET /api/domain-services
```

Response:
```json
{
  "ok": true,
  "count": 1026,
  "services": { ... }
}
```

#### 追加
```http
POST /api/domain-services
Content-Type: application/json

{
  "pattern": "example.com",
  "service": "Example",
  "category": "Cloud"
}
```

#### 更新
```http
PUT /api/domain-services/{pattern}
Content-Type: application/json

{
  "service": "NewService",
  "category": "NewCategory"
}
```

#### 削除
```http
DELETE /api/domain-services/{pattern}
```

---

## 2. 未知ドメインキャッシュ

### 2.1 概要

辞書にマッチしなかったドメインをFIFOキャッシュ（最大100件）で保持。
学習データ収集やセキュリティ監視に使用。

### 2.2 データ構造

```json
{
  "domain": "unknown.example.com",
  "count": 15,
  "first_seen": "2024-12-30T10:00:00.000000",
  "last_seen": "2024-12-30T12:30:00.000000",
  "sample_ips": ["192.168.1.100", "192.168.1.101"],
  "rooms": ["306", "UNK"]
}
```

### 2.3 フィールド定義

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `domain` | string | 未知ドメイン名 |
| `count` | int | 検出回数 |
| `first_seen` | string | 初回検出時刻（ISO8601） |
| `last_seen` | string | 最終検出時刻（ISO8601） |
| `sample_ips` | string[] | 送信元IPサンプル（最大5件） |
| `rooms` | string[] | 検出された部屋ID |

### 2.4 API

#### 一覧取得
```http
GET /api/unknown-domains
```

Response:
```json
{
  "ok": true,
  "count": 42,
  "total_hits": 1318,
  "domains": [ ... ]
}
```

#### クリア
```http
DELETE /api/unknown-domains
```

---

## 3. トラフィックパケット

### 3.1 パケット構造

```json
{
  "ts": "2024-12-30T06:42:00.000000+00:00",
  "src_ip": "192.168.125.100",
  "dst_ip": "142.250.196.142",
  "port": 443,
  "protocol": "TCP",
  "rid": "306",
  "domain": "www.google.com",
  "bytes": 1500,
  "direction": "outbound",
  "dns_type": null,
  "summary": {
    "service_id": "GoogleSearch",
    "service_category": "Search",
    "threat_level": 0,
    "dict_version": "2024.12.30"
  }
}
```

### 3.2 フィールド定義

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| `ts` | string | ○ | タイムスタンプ（ISO8601 UTC） |
| `src_ip` | string | ○ | 送信元IPアドレス |
| `dst_ip` | string | ○ | 宛先IPアドレス |
| `port` | int | ○ | 宛先ポート |
| `protocol` | string | ○ | プロトコル（TCP/UDP） |
| `rid` | string | ○ | 部屋ID（lacisOath userObject準拠） |
| `domain` | string | × | ドメイン名（DNS解決済み） |
| `bytes` | int | × | バイト数 |
| `direction` | string | × | 通信方向（outbound/inbound） |
| `dns_type` | string | × | DNSレコードタイプ（A/AAAA/CNAME等） |
| `summary` | object | × | サービス解決結果 |

### 3.3 ridフィールド

`rid`（Room ID）はlacisOathのuserObjectスキーマ共通仕様に準拠。
IP→部屋IDマッピングファイルから解決。

**マッピングファイル**: `/var/lib/is20s/ip_room_mapping.json`

```json
{
  "mappings": {
    "192.168.125.100": "306",
    "192.168.125.101": "307"
  },
  "ranges": {
    "192.168.125.150-192.168.125.160": "3F_LOBBY"
  }
}
```

---

## 4. CelestialGlobe連携

### 4.1 Ingestエンドポイント

```
POST https://us-central1-mobesorder.cloudfunctions.net/celestialGlobe_ingest
  ?fid={facility_id}&source=araneaDevice
```

### 4.2 認証ヘッダー

| ヘッダー | 説明 |
|---------|------|
| `X-Aranea-LacisId` | デバイスのlacisId |
| `X-Aranea-Mac` | MACアドレス（12桁HEX） |
| `X-Aranea-CIC` | araneaDeviceGateから取得したCIC |
| `X-Aranea-SourceType` | `ar-is20s` |
| `X-Aranea-Timestamp` | 送信時刻（ISO8601 UTC） |
| `X-Aranea-Nonce` | UUID（リプレイ攻撃防止） |

### 4.3 ペイロード形式

**Content-Type**: `application/x-ndjson`
**Content-Encoding**: `gzip`

```
{"ts":"...","src_ip":"...","dst_ip":"...","port":443,"protocol":"TCP","rid":"306","domain":"www.google.com","summary":{...}}
{"ts":"...","src_ip":"...","dst_ip":"...","port":443,"protocol":"TCP","rid":"307","domain":"youtube.com","summary":{...}}
...
```

### 4.4 バッチ設定

| パラメータ | デフォルト値 | 説明 |
|-----------|-------------|------|
| `batch_interval_sec` | 300 | バッチ送信間隔（5分） |
| `max_packets_per_batch` | 10,000 | 1バッチの最大パケット数 |
| `max_payload_bytes` | 5MB | 1バッチの最大サイズ |

### 4.5 リトライ戦略

```
失敗時: 指数バックオフ（1秒 → 2秒 → 4秒 → ... → 最大30秒）
最大リトライ: 5回
429 Rate Limit: Retry-Afterヘッダーに従う
400/401/403: リトライしない
```

---

## 5. ファイル一覧

| パス | 説明 |
|-----|------|
| `/var/lib/is20s/domain_services.json` | ドメインサービス辞書 |
| `/var/lib/is20s/dns_cache.json` | DNSキャッシュ（IP→ドメイン） |
| `/var/lib/is20s/ip_room_mapping.json` | IP→部屋IDマッピング |
| `/var/lib/is20s/ingest_config.json` | Ingestクライアント設定 |
| `/var/lib/is20s/aranea_register.json` | デバイス登録情報（CIC等） |
| `/var/lib/is20s/capture_logs/` | キャプチャログ（NDJSON） |
| `/etc/is20s/watch_config.json` | キャプチャ設定 |

---

## 6. 辞書同期

### 6.1 デバイス間同期

複数のIS-20Sデバイス間で辞書を同期する場合：

```bash
# デバイス1から全辞書を取得
curl -s http://device1:8080/api/domain-services > services.json

# デバイス2に一括登録
jq -r '.services | to_entries[] | @json' services.json | while read entry; do
  curl -s -X POST http://device2:8080/api/domain-services \
    -H "Content-Type: application/json" \
    -d "$entry"
done
```

### 6.2 CelestialGlobeとの同期

CelestialGlobeから最新辞書を取得する機能は今後実装予定。
`dict_version`フィールドでバージョン管理。

---

## 7. デプロイ済みデバイス

詳細は `code/orangePi/is20s/README.md` を参照。

| デバイス | ホスト名 | WiFi IP | lacisId |
|----------|----------|---------|---------|
| is20s-tam | is20s-tam | 192.168.125.248 | 302002004F750DAA0096 |
| is20s-to | is20s-to | 192.168.126.248 | 30200200C7FDB5550096 |

---

## 8. SDK ナレッジ参照

CelestialGlobe開発班は以下のナレッジを参照：

| タイトル | 内容 |
|----------|------|
| SSoT辞書 差分同期設計仕様 | 辞書同期プロトコル詳細 |
| Metatron AI ドメイン判定ルール | AI自動分類時の必須ルール |
| IS-20S ドメインサービス辞書 ベストプラクティス | マッチング順序の注意事項 |

**重要**: Metatron AIでの自動判定時、「ドットなしパターン禁止」「catch-all末尾配置」を厳守すること。

---

## 変更履歴

| 日付 | 変更内容 |
|-----|---------|
| 2024-12-31 | デプロイ済みデバイス情報追加、SDKナレッジ参照追加 |
| 2024-12-30 | 初版作成 |
