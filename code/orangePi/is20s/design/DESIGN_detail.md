# is20s 詳細設計書（Orange Pi Zero3 / Armbian 25.5.1）

作成日: 2025-12-24
最終更新: 2025-12-25 (v2)
対象ディスクイメージ: `code/orangePi/discimage/Armbian_25.5.1_Orangepizero3_noble_current_6.12.23.img`

## 1. 目的・要求

- **パケットキャプチャ**: ER605ルーターのミラーポートからtsharkでパケットキャプチャし、接続先メタデータを抽出
- **サービス識別**: DNS、TLS/SNI、ASN（自治システム番号）からアクセス先サービスを特定
- **脅威検知**: StevenBlack/hosts、Tor出口ノードリストによる脅威インテリジェンス
- **ローカルリレー**: ESP系デバイス（is0x/is1x 等）からのデータを受信し、ローカル/クラウドへ中継
- **エンドポイント投稿**: HTTP(S) で外部エンドポイントへ送信（プライマリ/セカンダリ両系統を想定）
- **MQTT遠隔操作**: MQTT でクラウド/ローカルからのコマンドを受信し、デバイス/リレーへ指示
- **統一系UI**: 状態・設定・ログ・キャプチャイベントの簡易Web UI を提供（FastAPI + 静的ページ）
- **共通モジュール分離**: OrangePi 系デバイス向けに `global/` とデバイス固有モジュールを分離

## 2. システム概要

### 2.1 役割
- is20s は拠点内の「ネットワーク監視 + ローカルリレー兼エッジゲート」
- ER605のミラーポートからパケットキャプチャし、部屋単位の通信先を可視化
- ESP系のローカルリレー機能を Armbian 上で強化し、遠隔操作とUIを一体化

### 2.2 主要機能
1. **tsharkパケットキャプチャ**: SYN-onlyモード（軽量）でTCP接続開始を検知
2. **DNS学習キャッシュ**: DNSクエリ/応答からIP→ドメインマッピングを自動学習
3. **ASNサービス判定**: Team Cymru DNSでASN検索し、YouTube/Netflix/TikTok等のサービスを特定
4. **脅威インテリジェンス**: マルウェア/アダルト/ギャンブル/フェイクニュース/Torを検知
5. **ローカルリレー**: ESP32デバイスからのHTTP POSTを受信・転送

### 2.3 想定入出力
- **入力**
  - ミラーポート（eth0）: ER605からのパケットコピー
  - HTTP POST: センサーデータ/状態報告（ESP系フォーマット互換）
  - MQTT Subscribe: クラウド/ローカルブローカーからのコマンド
- **出力**
  - HTTP POST: キャプチャイベントをバッチ送信（NDJSON、gzip圧縮）
  - HTTP POST: プライマリ/セカンダリエンドポイント
  - MQTT Publish: 応答・中継
  - Web UI / API: ステータス表示・イベント閲覧・設定更新

### 2.4 プロセス構成
```
systemd → uvicorn (FastAPI)
  ├─ FastAPI ルーター (api.py, ui.py)
  ├─ CaptureManager (capture.py)
  │   ├─ tshark subprocess (パケットキャプチャ)
  │   ├─ DnsCache (DNS学習キャッシュ)
  │   ├─ PTR Worker (逆引きワーカー)
  │   ├─ ThreatIntel (脅威インテリジェンス)
  │   ├─ ASNLookup (ASNサービス判定)
  │   └─ CaptureFileLogger (ファイルログ)
  ├─ MQTT クライアント (asyncio-mqtt)
  ├─ ローカルリレー Ingest → RingBuffer/SQLite → Forwarder
  ├─ AraneaRegister (CIC取得) & LacisID 生成
  └─ バックグラウンドタスク:
       - キャプチャイベントバッチ送信
       - キューflush
       - ヘルスチェック/再接続
```

## 3. ディレクトリ設計

```
code/orangePi/
├── global/                     # 共通モジュール (Python package)
│   ├── README.md
│   └── araneapi/               # import araneapi.*
│       ├── config.py           # 設定ロード(dataclass)
│       ├── logging.py          # ロガー初期化
│       ├── lacis_id.py         # LacisID/MAC取得
│       ├── device_register.py  # AraneaDeviceGate登録
│       ├── http_client.py      # HTTPクライアント共通
│       ├── mqtt_client.py      # MQTT接続・購読/発行ラッパー
│       ├── sqlite_store.py     # RingBuffer + SQLite 永続化
│       ├── system_info.py      # uptime/メモリ等取得
│       └── tasks.py            # 周期タスク/キャンセル管理
└── is20s/
    ├── design/
    │   ├── DESIGN.md           # 概要
    │   └── DESIGN_detail.md    # 本書
    ├── app/
    │   ├── main.py             # FastAPI エントリ
    │   ├── config.py           # is20s固有設定スキーマ
    │   ├── state.py            # 実行時ステート管理
    │   ├── ingest.py           # ローカルリレー受信処理
    │   ├── forwarder.py        # エンドポイント投稿/再送
    │   ├── mqtt_handlers.py    # MQTT購読/コマンド処理
    │   ├── api.py              # REST API ルーター
    │   ├── ui.py               # Web UI提供
    │   ├── capture.py          # ★ tsharkキャプチャモジュール
    │   ├── threat_intel.py     # ★ 脅威インテリジェンス
    │   ├── asn_lookup.py       # ★ ASN検索（Team Cymru DNS）
    │   ├── asn_services.py     # ★ ASN→サービス名マッピング
    │   ├── hardware_info.py    # ★ ハードウェア情報取得
    │   └── system_config.py    # ★ WiFi/NTP/同期設定
    ├── requirements.txt
    ├── is20s.service           # systemd ユニット雛形
    └── README.md               # セットアップ手順
```

## 4. 機能設計

### 4.1 tsharkパケットキャプチャ (capture.py)

#### 4.1.1 概要
- ER605ミラーポート（eth0）からパケットをキャプチャ
- SYN-onlyモード（デフォルト）: TCP接続開始のみを検知し低負荷で運用
- DNS、TLS/SNI、QUIC SNIからドメイン名を抽出

#### 4.1.2 CaptureManagerクラス
```python
class CaptureManager:
    """tsharkキャプチャ管理クラス"""

    # 主要属性
    process: asyncio.subprocess.Process  # tsharkプロセス
    running: bool                         # 稼働状態
    queue: List[Dict]                     # 送信用キュー
    display_buffer: List[Dict]            # 表示用リングバッファ（1000件）
    dns_cache: DnsCache                   # DNS学習キャッシュ
    threat_intel: ThreatIntel             # 脅威インテリジェンス
    asn_lookup: ASNLookup                 # ASN検索
    file_logger: CaptureFileLogger        # ファイルログ

    # 統計
    sent_count: int     # 送信済みイベント数
    drop_count: int     # ドロップ数（キュー溢れ）

    # メソッド
    async def start() -> bool       # キャプチャ開始
    async def stop() -> None        # キャプチャ停止
    async def restart() -> bool     # 再起動（設定反映）
    def get_status() -> Dict        # ステータス取得
    def reload_config() -> None     # 設定再読み込み
```

#### 4.1.3 BPFフィルタ生成
- 監視対象IPは`rooms`辞書（IP→部屋番号マッピング）で管理
- SYN-onlyモード（デフォルト）: DNS + TCP SYN のみキャプチャ
- DNSは送受信両方を捕捉（応答からIP→ドメインを学習するため）

```python
def build_bpf_filter(rooms: Dict[str, str], cfg: Dict) -> str:
    """
    BPFフィルタを構築
    syn_only=True:  DNS + TCP SYN のみ（軽量モード）
    syn_only=False: 全パケット詳細モード（高負荷）
    """
```

#### 4.1.4 セグメント指定機能

VLANで部屋を分けている環境向けに、セグメント全体（/24）を一括で監視対象にする機能。

**仕様**:
- IP末尾が`0`の場合（例: `192.168.125.0`）、そのセグメント全体（.1〜.254）を同一部屋として展開
- 例: `192.168.125.0=201` → `192.168.125.1`〜`192.168.125.254`をすべて部屋`201`として登録
- セグメント指定時はBPFフィルタを`net x.x.x.0/24`形式に最適化（254個の`host`条件を1つの`net`条件に集約）

```python
def expand_segment_ips(rooms: Dict[str, str]) -> Dict[str, str]:
    """
    IP末尾が0の場合、/24セグメント全体（1-254）に展開
    """
    expanded = {}
    for ip, room in rooms.items():
        if ip.endswith(".0"):
            prefix = ip[:-1]  # "192.168.125."
            for i in range(1, 255):
                expanded[f"{prefix}{i}"] = room
        else:
            expanded[ip] = room
    return expanded
```

**注意**: トラフィック量が膨大になるため、少量・小規模な場合以外は使用しないこと。

#### 4.1.5 tshark出力フィールド
| フィールド | 用途 |
|-----------|------|
| frame.time_epoch | タイムスタンプ |
| ip.src / ip.dst | 送信元/宛先IP |
| tcp.dstport / udp.dstport | 宛先ポート |
| ip.proto | プロトコル番号 |
| dns.qry.name | DNSクエリ名 |
| dns.a / dns.aaaa | DNS応答IP（キャッシュ用） |
| dns.cname | CNAME（別名解決用） |
| http.host | HTTPホストヘッダ |
| tls.handshake.extensions_server_name | TLS SNI |
| gquic.tag.sni | Google QUIC SNI |

#### 4.1.6 イベント構造
```json
{
  "id": "10120251225143005-0834",
  "epoch": 1735106405.123,
  "time": "2025-12-25T14:30:05+09:00",
  "room_no": "101",
  "src_ip": "192.168.3.101",
  "dst_ip": "142.250.196.110",
  "dst_port": "443",
  "protocol": "tcp",
  "dns_qry": null,
  "http_host": null,
  "tls_sni": "www.youtube.com",
  "resolved_domain": null,
  "threat": null,
  "asn": 15169,
  "asn_service": "YouTube/Google",
  "asn_category": "video"
}
```

### 4.2 DNS学習キャッシュ (DnsCacheクラス)

#### 4.2.1 概要
- DNSクエリ/応答からIP→ドメインのマッピングを学習
- TCP SYN時にSNIがない場合、キャッシュから逆引き
- LRU管理で最大50,000エントリを保持

#### 4.2.2 動作
1. DNS応答パケットを検知
2. `dns.a`（Aレコード応答IP）と`dns.qry.name`（クエリドメイン）を抽出
3. IP→ドメインのマッピングをキャッシュに追加
4. 複数IPの応答にも対応（カンマ区切り）

#### 4.2.3 永続化
- キャッシュファイル: `/var/lib/is20s/dns_cache.json`
- 100イベントごとに自動保存
- 停止時に保存

### 4.3 PTR逆引きワーカー

#### 4.3.1 概要
- DNSキャッシュにヒットしない場合、PTRレコードで逆引きを試行
- 非同期ワーカーがキューからIPを取得し、バックグラウンドで解決
- 上限付きキュー（100件）で溢れたら諦める

#### 4.3.2 設定
| 定数 | 値 | 説明 |
|------|-----|------|
| PTR_QUEUE_MAX_SIZE | 100 | キュー上限 |
| PTR_LOOKUP_TIMEOUT | 2.0秒 | タイムアウト |
| PTR_WORKER_INTERVAL | 0.1秒 | 処理間隔 |

### 4.4 ASNサービス判定 (asn_lookup.py, asn_services.py)

#### 4.4.1 概要
- Team Cymru DNSを使用してIPからASN（自治システム番号）を取得
- ASN→サービス名マッピングでYouTube、Netflix、TikTok等を特定
- DNSキャッシュ/SNIでドメインが不明な場合のフォールバック

#### 4.4.2 検索フロー
```
1. IP → Team Cymru DNS (dig +short {reversed-ip}.origin.asn.cymru.com TXT)
2. 応答からASN番号を抽出
3. ASN番号 → サービス名マッピング (ASN_SERVICE_MAP)
4. サービス名とカテゴリを返却
```

#### 4.4.3 サービスマッピング (抜粋)
| ASN | サービス名 | カテゴリ |
|-----|-----------|---------|
| 15169 | YouTube/Google | video |
| 2906 | Netflix | video |
| 32934 | Meta/Instagram | sns |
| 138699 | TikTok | sns |
| 13414 | Twitter/X | sns |
| 49544 | Discord | messenger |
| 32590 | Steam | game |
| 13335 | Cloudflare | cdn |
| 45090 | Tencent/微信 | china |
| 45102 | Alibaba/淘宝 | china |

#### 4.4.4 カテゴリ
| カテゴリ | ラベル | 説明 |
|---------|--------|------|
| video | 動画 | YouTube, Netflix等 |
| sns | SNS | Meta, TikTok, Twitter等 |
| messenger | メッセ | LINE, Discord, Telegram等 |
| game | ゲーム | Steam, Nintendo, Epic Games等 |
| china | 中国系 | 騰訊, 阿里巴巴, 百度等 |
| ec | EC | 楽天等 |
| cloud | クラウド | AWS, Azure, iCloud等 |
| cdn | CDN | Cloudflare, Akamai, Fastly等 |
| carrier | 通信 | NTT, KDDI, SoftBank等 |

### 4.5 脅威インテリジェンス (threat_intel.py)

#### 4.5.1 概要
- 公開ドメインリスト、Tor出口ノードを活用した脅威検知
- 自動更新（24時間間隔）

#### 4.5.2 データソース
| ソース | カテゴリ | 説明 |
|--------|---------|------|
| StevenBlack/hosts (unified) | malware | マルウェア/広告 |
| StevenBlack/hosts (fakenews) | fakenews | フェイクニュース |
| StevenBlack/hosts (gambling) | gambling | ギャンブル |
| StevenBlack/hosts (porn) | adult | アダルト |
| dan.me.uk/torlist | tor | Tor出口ノード |

#### 4.5.3 検知フロー
1. ドメイン完全一致チェック
2. サブドメインチェック（親ドメインがリストにあるか）
3. IPアドレスチェック（Tor出口ノード）

#### 4.5.4 データ保存
- ディレクトリ: `/var/lib/is20s/threat_intel/`
- ファイル: `malware_domains.txt`, `adult_domains.txt`, `gambling_domains.txt`, `fakenews_domains.txt`, `tor_exit_ips.txt`
- メタデータ: `meta.json`（最終更新日時）

### 4.6 ファイルログ (CaptureFileLogger)

#### 4.6.1 概要
- キャプチャイベントをNDJSON形式でファイル保存
- 自動ローテーション（1MB/ファイル、合計10MB）

#### 4.6.2 設定
| 設定 | デフォルト | 説明 |
|------|-----------|------|
| log_dir | /var/lib/is20s/capture_logs | 保存ディレクトリ |
| max_file_size | 1MB | ファイルあたりの最大サイズ |
| max_total_size | 10MB | 合計最大サイズ |

### 4.7 ハードウェア監視 (hardware_info.py)

#### 4.7.1 概要
Orange Pi Zero3のハードウェア情報をリアルタイムで取得し、UI/APIで表示。

#### 4.7.2 取得項目
| 項目 | ソース | 説明 |
|------|--------|------|
| RAM使用量 | /proc/meminfo | total, used, free, available (MB単位) |
| CPU使用率 | /proc/stat | user+system/total (%) |
| CPU温度 | /sys/class/thermal/thermal_zone0/temp | 摂氏温度 |
| ロードアベレージ | os.getloadavg() | 1分/5分/15分 |
| ストレージ | os.statvfs() | /および/var/lib/is20sの使用量 |
| ネットワーク | /sys/class/net/, ip addr | eth0/end0, wlan0のIP/MAC/接続状態 |

#### 4.7.3 APIレスポンス例
```json
{
  "memory": {"total_mb": 976, "used_mb": 456, "usage_percent": 46.7},
  "cpu": {"usage_percent": 12.5, "temperature_celsius": 45.2, "load_average": {"1min": 0.15, "5min": 0.20, "15min": 0.18}},
  "storage": [{"mount": "/", "total_gb": 14.5, "used_gb": 3.2, "usage_percent": 22.1}],
  "network": [
    {"name": "end0", "ip": "192.168.125.50", "mac": "02:00:xx:xx:xx:xx", "is_up": true, "is_connected": true},
    {"name": "wlan0", "ip": "192.168.96.205", "mac": "02:00:yy:yy:yy:yy", "is_up": true, "is_connected": true}
  ]
}
```

### 4.8 WiFi/NTP設定 (system_config.py)

#### 4.8.1 WiFi設定
NetworkManager（nmcli）を使用したWiFi接続管理。

| 機能 | API | 説明 |
|------|-----|------|
| 接続状態取得 | GET /api/wifi/status | SSID、信号強度、IPアドレス |
| ネットワークスキャン | GET /api/wifi/networks | 利用可能なSSID一覧 |
| WiFi接続 | POST /api/wifi/connect | SSID/パスワードで接続 |

#### 4.8.2 NTP設定
systemd-timesyncdを使用した時刻同期管理。

| 機能 | API | 説明 |
|------|-----|------|
| 同期状態取得 | GET /api/ntp/status | 同期状態、サーバー、タイムゾーン |
| サーバー設定 | POST /api/ntp/server | NTPサーバー変更 |
| タイムゾーン設定 | POST /api/ntp/timezone | タイムゾーン変更 |

#### 4.8.3 設定ファイル
- NTPサーバー: `/etc/systemd/timesyncd.conf`に書き込み
- WiFi: nmcli経由で設定（NetworkManager管理）

### 4.9 キャッシュ同期機能 (system_config.py - CacheSyncManager)

#### 4.9.1 概要
複数のis20s機器間でDNSキャッシュ・ASNキャッシュを同期し、検知精度を向上。サイト間VPN環境でのデイジーチェーン同期に対応。

#### 4.9.2 同期対象ファイル
| ファイル | 内容 |
|----------|------|
| dns_cache.json | IP→ドメインマッピング |
| asn_cache.json | IP→ASN情報マッピング |

#### 4.9.3 認証方式
イニシエータ/レスポンダーのパスフレーズベース認証:
- **イニシエータパスフレーズ**: 同期を開始する側が使用
- **レスポンダーパスフレーズ**: 同期を受け付ける側が使用
- パスフレーズはSHA256ハッシュ（先頭32文字）で`X-Sync-Auth`ヘッダーに設定
- 両方の機器で同じパスフレーズを設定しておく

#### 4.9.4 同期フロー
```
1. イニシエータ → GET /api/sync/export (相手のキャッシュ取得)
2. イニシエータ: 自分のキャッシュとマージ
3. イニシエータ → POST /api/sync/import (マージ後のキャッシュを相手に送信)
4. レスポンダ: 受信したキャッシュをマージ
```

#### 4.9.5 デイジーチェーン
複数のピアを登録し、順次同期を実行:
- A ↔ B ↔ C の構成で、A-B同期後にB-C同期を行うとA-B-Cが同一キャッシュを共有
- 定期自動同期オプション（デフォルト無効）

#### 4.9.6 API
| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| /api/sync/status | GET | 同期ステータス |
| /api/sync/config | POST | パスフレーズ設定 |
| /api/sync/peer | POST | ピア追加/削除/即時同期 |
| /api/sync/all | POST | 全ピア一括同期 |
| /api/sync/export | GET | キャッシュエクスポート |
| /api/sync/import | POST | キャッシュインポート |

#### 4.9.7 設定ファイル
保存先: `/var/lib/is20s/sync_config.json`
```json
{
  "peers": [{"host": "192.168.x.x", "port": 8080}],
  "initiator_passphrase": "secret1",
  "responder_passphrase": "secret2",
  "last_sync": "2025-12-25T14:30:00+09:00",
  "auto_sync_enabled": false,
  "sync_interval_hours": 24
}
```

### 4.10 ローカルリレー (ingest.py, forwarder.py)

- エンドポイント: `POST /api/ingest`
- 受信後:
  1. RingBuffer（メモリ最新2000件）へ格納
  2. SQLite へバッチ書き込み
  3. Forwarder キューへ投入
- Forwarder: 指数バックオフ、プライマリ→セカンダリの順に試行

### 4.11 MQTT 遠隔操作 (mqtt_handlers.py)

- 購読トピック: `aranea/{tid}/device/{lacisId}/command/#`
- 発行トピック: `aranea/{tid}/device/{lacisId}/response`, `.../status`
- コマンド: `ping`, `set_config`, `relay_now`

## 5. API設計

### 5.1 ステータス・共通
| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| /health | GET | ヘルスチェック |
| /api/status | GET | 総合ステータス（キャプチャ状態含む） |
| /api/events | GET | リングバッファ最新N件 |
| /api/logs | GET | アプリログ最新N行 |
| /api/config | GET/POST | 設定取得/更新 |

### 5.2 キャプチャ制御
| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| /api/capture/status | GET | キャプチャステータス |
| /api/capture/start | POST | キャプチャ開始 |
| /api/capture/stop | POST | キャプチャ停止 |
| /api/capture/restart | POST | キャプチャ再起動 |
| /api/capture/config | GET/POST | キャプチャ設定取得/更新 |
| /api/capture/rooms | POST | rooms辞書更新（IP→部屋番号） |
| /api/capture/events | GET | キャプチャイベント取得（表示用バッファ） |

### 5.3 脅威インテリジェンス
| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| /api/threat/status | GET | 脅威インテリジェンス統計 |
| /api/threat/update | POST | 脅威データ強制更新 |

### 5.4 ハードウェア情報
| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| /api/hardware | GET | ハードウェアステータス（RAM, CPU, 温度, ストレージ, ネットワーク） |

### 5.5 WiFi/NTP設定
| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| /api/wifi/status | GET | WiFi接続状態 |
| /api/wifi/networks | GET | 利用可能なWiFiネットワーク一覧 |
| /api/wifi/connect | POST | WiFi接続（ssid, password） |
| /api/ntp/status | GET | NTP同期状態 |
| /api/ntp/server | POST | NTPサーバー設定 |
| /api/ntp/timezone | POST | タイムゾーン設定 |

### 5.6 キャッシュ同期
| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| /api/sync/status | GET | 同期ステータス |
| /api/sync/config | POST | パスフレーズ設定 |
| /api/sync/peer | POST | ピア管理（action: add/remove/sync） |
| /api/sync/all | POST | 全ピア一括同期 |
| /api/sync/export | GET | キャッシュエクスポート（JSON） |
| /api/sync/import | POST | キャッシュインポート（マージ） |

## 6. 設定設計

### 6.1 メイン設定 (/etc/is20s/config.yaml)

```yaml
device:
  name: is20s
  product_type: "020"
  product_code: "0096"
  data_dir: /var/lib/is20s
  log_dir: /var/log/is20s
  interface: end0  # メインNIC（wlan0経由で通信）

relay:
  primary_url: "https://example.com/api/ingest"
  secondary_url: ""
  timeout_sec: 8
  max_retry: 3
  max_queue: 5000
  backoff_base_sec: 2
  backoff_max_sec: 60

mqtt:
  host: "mqtt.example.com"
  port: 8883
  tls: true
  user: ""
  pass: ""
  base_topic: "aranea"

ui:
  http_port: 8080

register:
  enabled: true
  gate_url: "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate"
  tid: "T2025XXXXX"

access:
  allowed_sources:
    - "192.168.0.0/24"
    - "10.0.0.5"

store:
  ring_size: 2000
  max_db_size: 20000
  flush_interval_sec: 10
  flush_batch: 50
  sqlite_path: /var/lib/is20s/relay.sqlite
```

### 6.2 キャプチャ設定 (/var/lib/is20s/watch_config.json)

```json
{
  "timezone": "Asia/Tokyo",
  "rooms": {
    "192.168.3.101": "101",
    "192.168.3.102": "102",
    "192.168.3.103": "103"
  },
  "capture": {
    "iface": "eth0",
    "snaplen": 2048,
    "buffer_mb": 16,
    "ports": [53, 80, 443],
    "include_quic_udp_443": true,
    "include_tcp_syn_any_port": true,
    "syn_only": true
  },
  "post": {
    "url": "https://example.com/api/capture",
    "headers": {},
    "batch_max_events": 500,
    "batch_max_seconds": 30,
    "gzip": true,
    "max_queue_events": 20000
  },
  "mode": {
    "dry_run": false,
    "enabled": true
  },
  "logging": {
    "enabled": true,
    "log_dir": "/var/lib/is20s/capture_logs",
    "max_file_size": 1048576,
    "max_total_size": 10485760
  }
}
```

#### 6.2.1 設定項目詳細

| カテゴリ | キー | デフォルト | 説明 |
|----------|------|-----------|------|
| rooms | {IP: room_no} | {} | 監視対象IPと部屋番号のマッピング |
| capture | iface | eth0 | キャプチャインターフェース（ミラーポート） |
| capture | snaplen | 2048 | パケット取得サイズ |
| capture | buffer_mb | 16 | tsharkバッファサイズ(MB) |
| capture | syn_only | true | SYN-onlyモード（軽量） |
| post | url | "" | 送信先URL |
| post | batch_max_events | 500 | バッチ送信最大件数 |
| post | batch_max_seconds | 30 | バッチ送信最大間隔 |
| post | gzip | true | gzip圧縮 |
| post | max_queue_events | 20000 | キュー上限 |
| mode | enabled | false | キャプチャ有効化 |
| mode | dry_run | false | ドライラン（送信しない） |

## 7. データ保存先

| パス | 内容 |
|------|------|
| /etc/is20s/config.yaml | メイン設定 |
| /var/lib/is20s/ | データディレクトリ |
| /var/lib/is20s/watch_config.json | キャプチャ設定 |
| /var/lib/is20s/relay.sqlite | リレーイベントDB |
| /var/lib/is20s/dns_cache.json | DNS学習キャッシュ |
| /var/lib/is20s/asn_cache.json | ASN検索キャッシュ |
| /var/lib/is20s/capture_logs/ | キャプチャログ（NDJSON） |
| /var/lib/is20s/threat_intel/ | 脅威インテリジェンスデータ |
| /var/lib/is20s/register.json | デバイス登録キャッシュ |
| /var/lib/is20s/config_override.yaml | UI設定オーバーライド |
| /var/lib/is20s/sync_config.json | キャッシュ同期設定（ピア、パスフレーズ） |
| /var/log/is20s/ | アプリログ |

## 8. 前提条件・運用注意

### 8.1 ネットワーク構成
- **eth0**: ミラーポート専用（IPアドレスなし）
- **wlan0**: 通信用（インターネット接続、API受付）
- デフォルトルートはwlan0経由であること

### 8.2 起動前チェック
- `check_prerequisites()`で以下を確認:
  - eth0にIPv4アドレスが設定されていないこと
  - デフォルトルートがwlan0経由であること

### 8.3 tshark権限
- tsharkの実行にはroot権限またはcapability設定が必要
- `sudo setcap cap_net_raw,cap_net_admin=eip /usr/bin/dumpcap`

## 9. 非機能要件

- **起動制御**: systemd unit `is20s.service`
- **データ保持**: SQLite 2万件程度でローテート、キャプチャログ10MB
- **監視**: `/api/status` をヘルスチェック
- **ロギング**: JSON/テキスト両用フォーマット

## 10. テスト戦略

- ユニット: config ロード、lacisId 生成、BPFフィルタ生成
- 手動: 各サービス（YouTube、Netflix、TikTok等）へのアクセスでサービス検知確認
- 疎通: MQTT ブローカーに対し `ping` コマンドで応答を確認

## 11. リスク/懸念

- Armbian での高トラフィック時のCPU負荷 → syn_onlyモードで軽減
- Team Cymru DNSの応答遅延 → キャッシュで軽減、タイムアウト設定
- 脅威インテリジェンスデータの更新失敗 → キャッシュ利用で継続動作
