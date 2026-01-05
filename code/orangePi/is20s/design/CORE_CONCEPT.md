# is20s Network Capture Gateway - コアコンセプト設計書

**文書バージョン**: 1.0
**作成日**: 2026/01/05
**最終更新**: 2026/01/05

---

## 1. システム概要

is20sは宿泊施設向けネットワークキャプチャゲートウェイである。
ルーターのミラーポートからトラフィックを受動的に監視し、以下の2つの主要機能を提供する。

---

## 2. 主要機能

### 2.1 インフラヘルス監視（Infrastructure Health Monitoring）

#### 目的
- ゲストに提供しているIoT機器・システムの**正常動作確認**
- 異常発生時の**動作状況・通信状況のデバッグ**
- ゲストがWiFi利用開始時に「**無事に通信できている**」ことの確認

#### 対象トラフィック
| カテゴリ | 例 | 監視目的 |
|---------|-----|---------|
| **IoT** | Chromecast, iwasaki linkS2, Alexa | 機器稼働確認 |
| **Infrastructure** | DNS, DHCP | ネットワーク基盤確認 |
| **Cloud** | AWS IoT, Google Home | バックエンド接続確認 |

#### ユースケース
1. **Chromecastが反応しない** → Google Cast通信の有無を確認
2. **照明スイッチが動かない** → linkS2の通信パターンを確認
3. **ゲストがネット使えない** → DNS/DHCP通信の正常性確認
4. **特定の部屋だけ不調** → 部屋別トラフィック比較

#### 除外すべきトラフィック
- **NTP**: 時刻同期のみ、デバッグ価値なし

---

### 2.2 マーケティング・施設改善分析（Marketing & Facility Improvement Analysis）

#### 目的
- **非公開データ**としてゲスト利用動向を収集
- 施設改善・マーケティング戦略立案の基礎データ提供
- セキュリティインシデント対応・**法的エビデンス**

#### 制約事項（重要）
```
ルーター単位での抽象化
├─ 取得可能: ドメイン/SNI（どのサービスにアクセスしたか）
├─ 取得不可: 具体的なクエリ内容、投稿内容、視聴コンテンツ詳細
└─ プライバシー: 「YouTubeを見た」は分かるが「何を見た」は分からない
```

#### 分析対象と施設改善への活用

| 分析カテゴリ | 取得データ例 | 施設改善アクション |
|-------------|-------------|-------------------|
| **Streaming** | YouTube, Netflix, Amazon Prime | 帯域確保、4K対応検討 |
| **Gaming** | Steam, PlayStation Network, Nintendo | **レイテンシ改善**、ゲーミングプラン検討 |
| **SNS** | Instagram, TikTok, X, LINE | **マーケティングチャネル選定** |
| **Messenger** | LINE, WhatsApp, Discord | コミュニケーション傾向把握 |
| **VideoConf** | Zoom, Teams, Google Meet | ビジネス利用需要把握 |
| **Adult** | 成人向けサイト | 需要把握（施設ポリシー検討） |
| **EC** | Amazon, 楽天, Taobao | ゲスト属性推定 |
| **Search** | Google, Yahoo, Bing | 情報探索傾向 |

#### 除外すべきトラフィック
- **IoT**: 機器通信であり、ゲスト行動ではない
- **Infrastructure**: DNS/NTP等、分析価値なし
- **NTP**: 完全に不要

---

## 3. セキュリティ・法的エビデンス機能

### 3.1 背景
過去に**違法アクセスによる警察案件**が発生した実績があり、is20sは法的エビデンス収集の役割も担う。

### 3.2 エビデンス要件

| 要件 | 内容 |
|------|------|
| **タイムスタンプ精度** | NTP同期による正確な時刻記録 |
| **送信元特定** | 部屋番号（MACアドレス→部屋マッピング） |
| **アクセス先記録** | ドメイン/SNI、IPアドレス |
| **データ保全** | 改ざん防止、一定期間の保存 |

### 3.3 検出対象

| カテゴリ | 例 | 対応 |
|---------|-----|------|
| **Malware** | ボットネット通信、C2サーバー | アラート・記録 |
| **Tor/VPN** | 匿名化通信 | 記録（合法だが注視） |
| **P2P** | BitTorrent等 | 記録（著作権侵害の可能性） |
| **DarkWeb** | .onionアクセス | アラート・記録 |
| **Attack** | ポートスキャン、攻撃パターン | アラート・即時対応 |

---

## 4. データモデル設計指針

### 4.1 カテゴリ体系

```
Primary Categories（ゲスト行動分析対象）
├─ SNS: Instagram, Facebook, TikTok, X, LINE（タイムライン）
├─ Streaming: YouTube, Netflix, Amazon Prime, Twitch
├─ Gaming: Steam, PlayStation, Nintendo, Xbox
├─ Messenger: LINE, WhatsApp, Discord, Telegram
├─ VideoConf: Zoom, Teams, Meet, Webex
├─ Search: Google, Yahoo, Bing, DuckDuckGo
├─ EC: Amazon, 楽天, Taobao, AliExpress
├─ Adult: 成人向けサイト
├─ News: ニュースサイト
└─ Finance: 銀行、証券、暗号資産

Auxiliary Categories（インフラ監視対象）
├─ IoT: Chromecast, Alexa, スマートスイッチ等
├─ Infrastructure: DNS, DHCP
├─ Cloud: AWS, GCP, Azure（IoT連携）
└─ CDN: コンテンツ配信（分析対象外だが記録）

Excluded Categories（除外対象）
├─ NTP: 時刻同期（常時除外）
└─ Background: OS自動更新、テレメトリ（オプション除外）

Security Categories（セキュリティ監視）
├─ Tor: Torネットワーク
├─ VPN: VPNサービス
├─ P2P: ファイル共有
├─ Malware: 既知のマルウェア通信
└─ Suspicious: 不審なパターン
```

### 4.2 表示モード

```javascript
// モード定義
const DISPLAY_MODES = {
  INFRA_HEALTH: {
    name: "インフラヘルス監視",
    show: ["IoT", "Infrastructure", "Cloud"],
    hide: ["NTP"],
    focus: "部屋別機器稼働状況"
  },
  MARKETING: {
    name: "マーケティング分析",
    show: ["SNS", "Streaming", "Gaming", "Messenger", "VideoConf", "Search", "EC", "Adult"],
    hide: ["IoT", "Infrastructure", "NTP", "Background"],
    focus: "ゲスト利用動向"
  },
  SECURITY: {
    name: "セキュリティ監視",
    show: ["Tor", "VPN", "P2P", "Malware", "Suspicious"],
    hide: [],
    focus: "不正アクセス検出"
  },
  FULL: {
    name: "全件表示",
    show: ["*"],
    hide: ["NTP"],
    focus: "デバッグ用"
  }
};
```

---

## 5. 実装検証チェックリスト

### 5.1 カテゴリ分類精度

- [ ] SNS系ドメインが正しくSNSに分類されるか
- [ ] Streaming系ドメインが正しくStreamingに分類されるか
- [ ] Gaming系ドメインが正しくGamingに分類されるか
- [ ] IoT機器通信が正しくIoTに分類されるか
- [ ] **バルク化問題がないか**（www.google→Search等の誤分類）

### 5.2 モード切替

- [ ] インフラヘルス監視モードでIoT/Infraが表示されるか
- [ ] マーケティングモードでIoT/Infraが除外されるか
- [ ] NTPが全モードで除外されるか

### 5.3 部屋別表示

- [ ] MACアドレス→部屋番号マッピングが正しいか
- [ ] 部屋番号順でソートされるか
- [ ] 未登録MACが「UNK」表示されるか

### 5.4 エビデンス要件

- [ ] タイムスタンプがNTP同期されているか
- [ ] ログが一定期間保存されるか
- [ ] セキュリティカテゴリがアラート表示されるか

---

## 6. 今後の拡張検討

### 6.1 短期（1-2週間）
- NTPカテゴリ分離・デフォルト除外
- モード切替UI実装
- バルク化パターン修正

### 6.2 中期（1-2ヶ月）
- Gamingカテゴリ詳細化（タイトル別分類）
- セキュリティアラート機能
- 統計レポート自動生成

### 6.3 長期（3ヶ月以上）
- 機械学習による異常検知
- 外部SIEM連携
- マルチ施設ダッシュボード

---

## 7. 改訂履歴

| 日付 | バージョン | 内容 |
|------|-----------|------|
| 2026/01/05 | 1.0 | 初版作成 |
