# IS20S ドメイン辞書管理ツール

## 概要

3デバイス（akb, tam, to）のドメイン→サービス辞書を管理するツールセット。

## ディレクトリ構成

```
is20s/
├── tools/
│   ├── collect_unknown.sh  # 不明ドメイン収集
│   ├── add_pattern.sh      # パターン追加
│   ├── sync_dict.sh        # 3デバイス同期＆バックアップ
│   └── README.md           # このファイル
└── dict_backups/           # バックアップ保存先（git管理）
    └── domain_services_YYYYMMDDHHMMSS.json
```

## デバイス情報

| 名前 | IP | 用途 |
|------|-----|------|
| akb | 192.168.3.250 | マスター（ここでパターン追加） |
| tam | 192.168.125.248 | スレーブ |
| to | 192.168.126.248 | スレーブ |

## 作業フロー

### 1. 不明ドメイン収集

```bash
./collect_unknown.sh
```

3デバイスから不明ドメインを収集・マージして表示。

### 2. 手動分類（重要）

出力された不明ドメインを**手動で**分類する。

分類基準：
- **サービス名**: 具体的なサービス名（例: Kuro Games, Yostar, 美団）
- **カテゴリ**: 以下から選択

| カテゴリ | 説明 | 例 |
|---------|------|-----|
| Streaming | 動画配信 | YouTube, Netflix, TikTok |
| SNS | ソーシャル | Instagram, Twitter, 小紅書 |
| Messenger | メッセージ | LINE, WeChat, Discord |
| Game | ゲーム | Fortnite, アークナイツ, NIKKE |
| EC | 通販 | Amazon, Taobao, 楽天, 美団 |
| Cloud | クラウド | AWS, Azure, GCP |
| CDN | 配信網 | Akamai, Cloudflare, CDN77 |
| Tracker | 解析/広告 | Google Analytics, OneTrust |
| Payment | 決済 | Stripe, PayPal |
| Auth | 認証 | Microsoft Auth, Google認証 |
| IoT | IoTデバイス | TP-Link, Alexa, Tapo |
| Development | 開発ツール | Unity, GitHub |
| Hosting | VPS/ホスティング | Vultr, Lolipop |
| Adult | アダルト | - |
| Infrastructure | インフラ | NTP, DNS |

**注意**:
- 一括自動分類は禁止（バルク化の原因）
- 各ドメインの実態を確認して分類
- 不明な場合は調査する

### 3. パターン追加

```bash
./add_pattern.sh <pattern> <service> <category>
```

例:
```bash
./add_pattern.sh kurogames "Kuro Games" Game
./add_pattern.sh meituan "美団" EC
./add_pattern.sh phncdn "Pornhub" Adult
```

パターン設計のポイント：
- 汎用的なキーワードを抽出（例: `kurogames`）
- サブドメインに対応できるよう短めに
- 誤検知しないよう注意

### 4. 辞書同期＆バックアップ

```bash
./sync_dict.sh
```

実行内容：
1. akbから辞書取得
2. `dict_backups/`にバックアップ保存
3. tam/toに同期＆再起動
4. 各デバイスのパターン数確認

### 5. GitHubにpush（必須）

```bash
cd /path/to/aranea_ISMS
git add code/orangePi/is20s/dict_backups/
git commit -m "dict: update domain_services (XXX patterns)"
git push
```

## トラブルシューティング

### パターンが反映されない

```bash
# 辞書キャッシュをクリアして再起動
ssh mijeosadmin@192.168.3.250
echo 'mijeos12345@' | sudo -S rm -f /var/lib/is20s/domain_services.json
echo 'mijeos12345@' | sudo -S systemctl restart is20s
```

### 判定確認

```bash
curl "http://192.168.3.250:8080/api/domain-services/lookup?domain=example.com"
```

## 禁止事項

- 一括自動分類スクリプトの使用
- 確認なしのパターン追加
- バックアップなしの辞書更新
- git pushしないでの作業終了
