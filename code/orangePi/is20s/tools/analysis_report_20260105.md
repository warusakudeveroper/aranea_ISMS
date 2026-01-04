# ドメイン分類解析レポート

**日時**: 2026/01/05 07:33
**収集元**: tam (192.168.125.248), to (192.168.126.248), akb (192.168.3.250)
**収集件数**: 各500件、計1500イベント

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

---

## 4. 推奨対応

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

## 5. 収集データ保存先

- `/tmp/tam_events.json`
- `/tmp/to_events.json`
- `/tmp/akb_events.json`
- `/tmp/all_domains.txt`

---

## 6. エミュレータ検証結果

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
