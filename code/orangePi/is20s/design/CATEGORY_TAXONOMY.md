# is20s カテゴリ分類体系設計書

**文書バージョン**: 1.0
**作成日**: 2026/01/05

---

## 1. カテゴリ階層

```
Root
├─ Primary（ゲスト行動分析）
│   ├─ SNS
│   ├─ Streaming
│   ├─ Gaming
│   ├─ Messenger
│   ├─ VideoConf
│   ├─ Search
│   ├─ EC
│   ├─ Adult
│   ├─ News
│   └─ Finance
│
├─ Auxiliary（インフラ監視）
│   ├─ IoT
│   ├─ Infrastructure
│   ├─ Cloud
│   └─ CDN
│
├─ Excluded（除外）
│   ├─ NTP
│   └─ Background
│
└─ Security（セキュリティ）
    ├─ Tor
    ├─ VPN
    ├─ P2P
    ├─ Malware
    └─ Suspicious
```

---

## 2. Primary カテゴリ詳細

### 2.1 SNS（ソーシャルネットワーク）

**目的**: ゲストのSNS利用傾向把握、マーケティングチャネル選定

| サービス | ドメインパターン | 備考 |
|---------|-----------------|------|
| Instagram | `instagram.com`, `cdninstagram.com` | Meta系 |
| Facebook | `facebook.com`, `fbcdn.net` | Meta系 |
| TikTok | `tiktok.com`, `tiktokcdn.com` | ByteDance系 |
| X (Twitter) | `twitter.com`, `x.com`, `twimg.com` | |
| LINE Timeline | `timeline.line.me` | LINEのSNS機能 |
| 小紅書 | `xiaohongshu.com` | 中国系 |
| Threads | `threads.net` | Meta系 |
| Weibo | `weibo.com` | 中国系 |

**注意**: LINE本体はMessengerカテゴリ

### 2.2 Streaming（動画・音楽配信）

**目的**: 帯域需要把握、4K対応検討

| サービス | ドメインパターン | 備考 |
|---------|-----------------|------|
| YouTube | `youtube.com`, `googlevideo.com`, `ytimg.com` | |
| Netflix | `netflix.com`, `nflxvideo.net` | |
| Amazon Prime | `primevideo.com`, `aiv-cdn.net` | |
| Disney+ | `disneyplus.com`, `disney-plus.net` | |
| Hulu | `hulu.com`, `hulustream.com` | |
| Twitch | `twitch.tv`, `jtvnw.net` | ゲーム実況 |
| Spotify | `spotify.com`, `scdn.co` | 音楽 |
| Apple Music | `music.apple.com` | 音楽 |
| DAZN | `dazn.com` | スポーツ |
| AbemaTV | `abema.tv` | 国内 |
| TVer | `tver.jp` | 国内 |

### 2.3 Gaming（ゲーム）

**目的**: レイテンシ要件把握、ゲーミング需要分析

| サービス | ドメインパターン | 備考 |
|---------|-----------------|------|
| Steam | `steampowered.com`, `steamcommunity.com` | PC |
| PlayStation | `playstation.com`, `playstation.net` | Sony |
| Nintendo | `nintendo.com`, `nintendo.net` | Nintendo |
| Xbox | `xbox.com`, `xboxlive.com` | Microsoft |
| Epic Games | `epicgames.com`, `unrealengine.com` | Fortnite等 |
| Riot Games | `riotgames.com` | LoL, Valorant |
| Blizzard | `blizzard.com`, `battle.net` | |
| miHoYo | `mihoyo.com`, `hoyoverse.com` | 原神等 |

### 2.4 Messenger（メッセンジャー）

**目的**: コミュニケーション傾向把握

| サービス | ドメインパターン | 備考 |
|---------|-----------------|------|
| LINE | `line.me`, `line-scdn.net`, `linecorp.com` | **line-scdn優先** |
| WhatsApp | `whatsapp.com`, `whatsapp.net` | Meta系 |
| Discord | `discord.com`, `discordapp.com` | ゲーマー |
| Telegram | `telegram.org`, `t.me` | |
| WeChat | `wechat.com`, `weixin.qq.com` | 中国系 |
| Slack | `slack.com` | ビジネス |
| Facebook Messenger | `messenger.com` | Meta系 |

### 2.5 VideoConf（ビデオ会議）

**目的**: ビジネス利用需要把握

| サービス | ドメインパターン | 備考 |
|---------|-----------------|------|
| Zoom | `zoom.us`, `zoomgov.com` | |
| Microsoft Teams | `teams.microsoft.com` | |
| Google Meet | `meet.google.com` | |
| Webex | `webex.com` | Cisco |
| Skype | `skype.com` | Microsoft |

### 2.6 Search（検索）

**目的**: 情報探索傾向把握

| サービス | ドメインパターン | 備考 |
|---------|-----------------|------|
| Google検索 | `www.google.*/search` | **URLパス必要** |
| Yahoo | `search.yahoo.com` | |
| Bing | `bing.com` | |
| DuckDuckGo | `duckduckgo.com` | プライバシー重視 |
| Baidu | `baidu.com` | 中国系 |

**注意**: `www.google`単体はバルク化問題あり。OAuthやリダイレクトも含むため、より厳密なパターン必要。

### 2.7 EC（電子商取引）

**目的**: ゲスト属性推定、購買傾向

| サービス | ドメインパターン | 備考 |
|---------|-----------------|------|
| Amazon | `amazon.co.jp`, `amazon.com` | **IoTと区別必要** |
| 楽天 | `rakuten.co.jp` | |
| Yahoo!ショッピング | `shopping.yahoo.co.jp` | |
| Taobao | `taobao.com` | 中国系 |
| AliExpress | `aliexpress.com` | 中国系 |
| メルカリ | `mercari.com` | フリマ |

### 2.8 Adult（成人向け）

**目的**: 需要把握（施設ポリシー検討）

具体的なドメインリストは別途管理。

### 2.9 News（ニュース）

**目的**: 情報収集傾向

| サービス | ドメインパターン |
|---------|-----------------|
| Yahoo!ニュース | `news.yahoo.co.jp` |
| NHK | `nhk.or.jp` |
| 朝日新聞 | `asahi.com` |
| 読売新聞 | `yomiuri.co.jp` |
| CNN | `cnn.com` |
| BBC | `bbc.com` |

### 2.10 Finance（金融）

**目的**: セキュリティ監視対象としても重要

| サービス | ドメインパターン |
|---------|-----------------|
| 銀行各社 | 個別登録 |
| 証券各社 | 個別登録 |
| 暗号資産 | `coinbase.com`, `binance.com` 等 |

---

## 3. Auxiliary カテゴリ詳細

### 3.1 IoT（IoTデバイス）

**目的**: 部屋機器の稼働確認

| デバイス | ドメインパターン | 備考 |
|---------|-----------------|------|
| Google Cast | `cast.google.com`, `www3.l.google.com` | Chromecast |
| Alexa | `alexa.amazon.com`, `avs-alexa-*.amazon.com` | Echo |
| iwasaki linkS2 | **要調査** | 照明スイッチ |
| TP-Link | `tplinkcloud.com` | ネットワーク機器 |
| Fire TV | `firetvcaptiveportal.com` | Amazon |

### 3.2 Infrastructure（インフラ）

| 種別 | ドメインパターン | 備考 |
|------|-----------------|------|
| DNS | `dns.google`, 各種DNSサーバー | |
| PTR | `*.in-addr.arpa` | 逆引き |
| DHCP | （ドメインなし） | |

### 3.3 Cloud（クラウド基盤）

| サービス | ドメインパターン | 備考 |
|---------|-----------------|------|
| AWS | `amazonaws.com` | IoT連携含む |
| GCP | `googleapis.com` | |
| Azure | `azure.com`, `windows.net` | |

---

## 4. Excluded カテゴリ詳細

### 4.1 NTP（時刻同期）

**常時除外**: 分析価値なし

| パターン | 備考 |
|---------|------|
| `ntp.nict.jp` | 国内NTP |
| `*.pool.ntp.org` | 公開NTPプール |
| `time.google.com` | Google NTP |
| `time.windows.com` | Microsoft NTP |
| `time.apple.com` | Apple NTP |

### 4.2 Background（バックグラウンド通信）

**オプション除外**: OS自動更新等

| パターン | 備考 |
|---------|------|
| `windowsupdate.com` | Windows Update |
| `apple.com/software` | macOS Update |
| テレメトリ各種 | |

---

## 5. Security カテゴリ詳細

### 5.1 Tor

| パターン | 備考 |
|---------|------|
| `*.onion` | Torドメイン |
| Tor Exit Node IP | リスト管理 |

### 5.2 VPN

| サービス | ドメインパターン |
|---------|-----------------|
| NordVPN | `nordvpn.com` |
| ExpressVPN | `expressvpn.com` |
| 各種VPN | 個別登録 |

### 5.3 P2P

| プロトコル | 検出方法 |
|-----------|---------|
| BitTorrent | トラッカードメイン、ポート |

---

## 6. パターン設計ルール

### 6.1 禁止事項

1. **3文字以下のパターン禁止**: `tor`, `msn` 等は誤マッチ多発
2. **一般英単語の部分マッチ禁止**: `line` → `line.me` に変更
3. **複数サービスで使用されるパターン禁止**: 個別登録必須

### 6.2 推奨事項

1. **ドット付きパターン使用**: `line.me`, `instagram.com`
2. **サブドメイン個別登録**: `line-scdn.net`, `fb-scdn.net`
3. **CDNは配信元サービスに分類**: `googlevideo.com` → Streaming (YouTube)

### 6.3 優先順位

より具体的なパターンが先にマッチするよう、パターン長でソートするか、明示的な優先順位を設定。

```python
# 例: line-scdn が scdn より先にマッチすべき
patterns = [
    ("line-scdn", "LINE", "Messenger"),  # 優先
    ("scdn", "Spotify", "Streaming"),     # 後
]
```

---

## 7. 改訂履歴

| 日付 | バージョン | 内容 |
|------|-----------|------|
| 2026/01/05 | 1.0 | 初版作成 |
