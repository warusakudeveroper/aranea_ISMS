# ドメイン分類解析レポート

**日時**: 2026/01/05 07:33 (更新: 10:45)
**収集元**: tam (192.168.125.248), to (192.168.126.248), akb (192.168.3.250)
**収集件数**: 各500件、計1500イベント（追加検証: 各1000件）

---

## 1. 現状サマリー

### カテゴリ別イベント数

| カテゴリ | 件数 | 備考 |
|---------|------|------|
| IoT | 688 | Google Cast, AWS IoT, Chromecast等 |
| Media | 282 | gstatic系 |
| Cloud | 140 | AWS WAF, Google, Azure等 |
| Infrastructure | 130 | PTR, NTP |
| CDN | 84 | Google Fonts, Cloudflare |
| Mail | 51 | SmartMail |
| EC | 43 | Amazon, Taobao |
| AI | 37 | Cursor, DeepL |
| Tracker | 23 | Intercom |
| Messenger | 11 | Discord |
| Unknown | 8 | 2ドメインのみ |

### 未分類ドメイン（2件）

- `ntp.nict.jp` (x7) → NTP (Infrastructure) に分類すべき
- `mosaic-nova.apis.mcafee.com` (x1) → McAfee (Security) に分類すべき

---

## 2. SNS系の状況

**収集データにSNS通信なし**

収集した1500イベント中、Instagram/Facebook/TikTok/Twitter等のSNS関連ドメインは0件。
これは環境的にSNS利用がないためと思われる（宿泊施設のIoT機器中心）。

**ただしエミュレータでのSNSパターン検証は正常**:
```
instagram.com -> Instagram (SNS) ✓
facebook.com -> Facebook (SNS) ✓
tiktok.com -> TikTok (SNS) ✓
twitter.com -> X (SNS) ✓
xiaohongshu.com -> 小紅書 (SNS) ✓
```

---

## 3. バルク化問題（重大）

### 発見された問題パターン

| パターン | 意図 | 誤マッチ例 | 影響度 |
|---------|------|-----------|--------|
| `scdn` | Spotify | line-scdn.net → Spotify | 高 |
| `line` | LINE | timeline.com, offline.com, pipeline.io → LINE | 高 |
| `nest` | Nest | honest.com, earnest.io → Nest | 中 |
| `tor` | Tor | motor.com, doctor.io → Tor | 中 |

### 詳細

#### `scdn` パターン（最重要）
```
期待: line-scdn.net -> LINE (Messenger)
実際: line-scdn.net -> Spotify (Streaming)  ❌

原因: 'scdn'が'line-scdn'より先にマッチ
```

#### `line` パターン
```
期待: timeline.com -> (unknown)
実際: timeline.com -> LINE (Messenger)  ❌

期待: offline.com -> (unknown)
実際: offline.com -> LINE (Messenger)  ❌
```

#### `www.google` パターン（Search除外の問題）
```
パターン: www.google → Google検索 (Search)

問題点:
- www.google.com は検索以外にも多用途で使用される
  - OAuth/SSO認証フロー（Gmail, Drive, YouTube等へのログイン）
  - Gmail/Driveへのリダイレクト
  - Google Workspace認証

- Search除外フィルタを有効にすると、これらの有意なデータも除外される恐れ

現状: tam/toではwww.google*へのアクセスが0件（IoT中心のため）
      ゲスト利用時には問題が顕在化する可能性あり
```

---

## 4. データ有意性の問題と用途別要件

### 現状の分布

| デバイス | IoT | Infrastructure | 合計 | 残り |
|---------|-----|----------------|------|------|
| tam | 43.3% | 29.7% | **73.0%** | 27.0% |
| to | 37.3% | 14.0% | **51.3%** | 48.7% |

### 用途別カテゴリ要件

| カテゴリ | 部屋ヘルス監視 | マーケティング分析 | 備考 |
|---------|--------------|------------------|------|
| **SNS/Streaming** | ○ | ◎ | 主要分析対象 |
| **Search** | ○ | ○ | バルク化解消が前提 |
| **IoT** | ◎ | × | Chromecast, linkS2等の稼働確認 |
| **Infrastructure** | △ | × | DNS/DHCPは△、NTPは完全不要 |
| **NTP** | × | × | 分離してデフォルト除外すべき |

### 主なIoT通信（部屋ヘルス監視で有用）

1. **Google Cast/Chromecast**
   - `www3.l.google.com`
   - `cast.google.com`
   - `tools.l.google.com`

2. **iwasaki linkS2スイッチ** （要パターン追加）
   - 現在未分類の可能性

3. **Alexa/Echo**
   - `*.amazon.com`, `*.amazonaws.com`

### 完全不要なトラフィック

- **NTPサーバー** → 新カテゴリ「NTP」に分離してデフォルト除外
  - `ntp.nict.jp`
  - `*.pool.ntp.org`

### 改善案

#### 案1: モード切替（推奨）
```
[部屋ヘルス監視モード]     [マーケティング分析モード]
├─ SNS/Streaming ○         ├─ SNS/Streaming ◎
├─ Search ○                 ├─ Search ○
├─ IoT ◎                    ├─ IoT ×（除外）
├─ Infrastructure △         ├─ Infrastructure ×（除外）
└─ NTP ×（常時除外）        └─ NTP ×（常時除外）
```

UIにモード切替トグルを追加し、プリセットで除外カテゴリを変更。

#### 案2: NTP分離 + カテゴリ除外チェックボックス
- NTPをInfrastructureから分離して独立カテゴリ化
- IoT/Infrastructure/NTPの個別除外チェックボックス追加
- NTPはデフォルトON（除外）

#### 案3: Search除外廃止
- 現在のSearch除外チェックボックスを廃止
- バルク化問題の根本解決（パターン精緻化）を優先
- 「何でもSearchに入る」問題を解消してから再検討

---

## 5. 推奨対応

### 即座に修正すべき項目

1. **`line-scdn`パターン追加**
   - `line-scdn` → LINE (Messenger) を `scdn` より優先させる

2. **`line`パターン修正**
   - `line.me` (ドット付き) に変更、または
   - `line-apps`, `line-scdn`, `linecorp` 等の具体パターンに分解

3. **未分類ドメイン追加**
   - `ntp.nict.jp` → NTP (Infrastructure)
   - `mcafee` → McAfee (Security)

### パターン設計原則（今後）

1. **3文字以下のパターンは禁止** （tor, msn等）
2. **一般英単語を含むパターンは`.`付きで登録** （line → line.me）
3. **CDNサブドメインは個別登録** （line-scdn, fb-scdn等）

---

## 6. Stats表示順検証

### 現状
- tam: room登録16件、順序 `['101', '104', '105', '106', '201', ...]`
- to: room登録17件、順序 `['101', '103', '104', '105', '106', '201', ...]`

### 実装確認（ui.py）

```javascript
// lines 798-802: configuredRoomsは数値順でソート済み
const roomSet=new Set(Object.values(rooms));
configuredRooms=[...roomSet].sort((a,b)=>{
  const na=parseInt(a),nb=parseInt(b);
  if(!isNaN(na)&&!isNaN(nb))return na-nb;
  return a.localeCompare(b);
});

// lines 1599-1600: Statsデータにorderをセット
configuredRooms.forEach(r=>{
  rooms[r]={total:0,cats:{},order:configuredRooms.indexOf(r)};
});

// lines 1654-1658: Stats表示はorder順
const rooms=Object.entries(statsData.rooms).sort((a,b)=>{
  const orderA=a[1].order??9999;
  const orderB=b[1].order??9999;
  return orderA-orderB;
});
```

### 結論
**Stats表示順は登録リスト順（数値昇順）で固定されている**
- `order`属性は`configuredRooms.indexOf(r)`から取得
- `configuredRooms`は数値ソート済み
- 動的にイベント量で並び替わることはない

---

## 7. 収集データ保存先

- `/tmp/tam_events.json`
- `/tmp/to_events.json`
- `/tmp/akb_events.json`
- `/tmp/all_domains.txt`
- `/tmp/tam_full.json` (1000件)
- `/tmp/to_full.json` (1000件)

---

## 8. エミュレータ検証結果

```
Total domains: 80
Correctly classified: 67 (84%)
Unknown: 13 (16%)
```

Unknown 13件の内訳:
- NTP関連: 4件 (0.pool.ntp.org等)
- PTR逆引き: 3件 (*.in-addr.arpa)
- AWS内部: 2件 (awswaf.com, devices.a2z.com)
- その他: 4件 (mmstat.com, ipv4only.arpa等)

---

## 9. 総括と次のステップ

### 発見された問題

| 問題 | 深刻度 | 状態 |
|-----|-------|------|
| バルク化（scdn, line, nest, tor） | 高 | 要修正 |
| バルク化（www.google → Search） | 中 | パターン精緻化で対応 |
| IoT/Infrastructure支配（73%） | 高 | モード切替で対応 |
| NTP完全不要 | 中 | 分離・デフォルト除外 |
| SNS未検出 | 低 | 環境要因（パターンは正常） |
| Stats表示順 | - | 正常動作確認済 |

### 推奨アクション優先順位

1. **バルク化パターン修正**（即座に）
   - `line-scdn` 追加（LINE CDN）
   - `line` → `line.me` に変更
   - `www.google` → より厳密なパターンに分解
   - `ntp.nict.jp`, `mcafee` 追加

2. **NTP分離**（短期）
   - InfrastructureからNTPを分離して独立カテゴリ化
   - `ntp`, `pool.ntp.org`, `ntp.nict.jp` → NTP (デフォルト除外)

3. **モード切替UI追加**（短期）
   - 「部屋ヘルス監視」「マーケティング分析」切替
   - IoT/Infrastructure除外プリセット

4. **Search除外チェックボックス廃止検討**（中期）
   - バルク化解消後に再評価
   - 現状の実装は誤解を招く可能性

5. **iwasaki linkS2パターン追加**（要調査）
   - 実際の通信ドメインを特定してIoT分類に追加
