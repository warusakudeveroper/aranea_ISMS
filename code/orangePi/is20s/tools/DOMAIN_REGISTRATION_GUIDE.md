# ドメイン登録ガイド（AIエージェント向け）

このドキュメントは、不明ドメインを辞書に登録する際のワークフローを定義する。
AIエージェントはこのフローに従って安全かつ正確にドメイン分類を行うこと。

## 概要

```
不明ドメイン取得 → 調査（手動） → 辞書登録 → エミュレータテスト → 3デバイス同期
```

---

## 1. 不明ドメイン取得

### 1.1 各デバイスのAPIから未分類イベントを取得

```bash
# akb (192.168.3.250)
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no -o ConnectTimeout=15 mijeosadmin@192.168.3.250 "cat /var/lib/is20s/capture_logs/*.ndjson 2>/dev/null | python3 -c '
import json, sys
from collections import Counter
none_domains = []
for line in sys.stdin:
    try:
        e = json.loads(line.strip())
        svc = e.get(\"domain_service\") or \"\"
        cat = e.get(\"domain_category\") or \"\"
        if not svc or not cat:
            domain = e.get(\"tls_sni\") or e.get(\"dns_qry\") or e.get(\"resolved_domain\") or \"\"
            if domain:
                none_domains.append(domain)
    except: pass
print(\"=== akb unknown domains ===\")
for d, cnt in Counter(none_domains).most_common(50):
    print(f\"{d}\")
'"

# tam (192.168.125.248)
# 同様のコマンドでIPを変更

# to (192.168.126.249)
# 同様のコマンドでIPを変更
```

### 1.2 ローカルファイルに保存

```bash
# 例: /tmp/unknown_domains.txt に保存
echo "example.unknown.com" >> /tmp/unknown_domains.txt
```

---

## 2. ドメイン調査（手動）

### 2.1 調査すべき項目

| 項目 | 確認方法 |
|------|----------|
| サービス名 | whois、Webサイト確認 |
| 用途 | トラフィックの性質（ユーザー操作/バックグラウンド） |
| カテゴリ | 下記カテゴリ一覧から選択 |
| role | 補助的通信か否か |

### 2.2 カテゴリ一覧

```
# PRIMARY_CATEGORIES（重要：display_bufferフィルタをバイパスする）
SNS, Streaming, Gaming, Messenger, VideoConf, Search, EC, Adult, News, Finance

# SECONDARY_CATEGORIES（フィルタ対象）
Infrastructure, Cloud, CDN, IoT, Ad, Analytics, Media, Creative, Mail, Developer, Security, Unknown
```

### 2.3 role属性

| role | 用途 |
|------|------|
| primary | ユーザーの明示的なアクション（Webブラウジング等） |
| auxiliary | バックグラウンド通信（NCSI、telemetry等） |
| (未指定) | 通常のトラフィック |

**重要**: `auxiliary`を設定すると、そのトラフィックはフィルタリング対象になる。

### 2.4 よくある誤分類パターン

| ドメイン | 誤分類 | 正しい分類 | 理由 |
|----------|--------|------------|------|
| www.google.com | Search | Infrastructure/auxiliary | NCSI接続確認通信 |
| clients.l.google.com | Search | Infrastructure/auxiliary | Googleサービスバックグラウンド |
| connectivitycheck.gstatic.com | CDN | Infrastructure/auxiliary | 接続確認 |
| dns.msftncsi.com | Unknown | Infrastructure/auxiliary | Windows NCSI |

---

## 3. 辞書登録

### 3.1 辞書ファイルの場所

```
code/orangePi/is20s/data/default_domain_services.json
```

### 3.2 エントリフォーマット

```json
{
  "services": {
    "example.com": {
      "service": "サービス名",
      "category": "カテゴリ",
      "role": "auxiliary"  // オプション
    }
  }
}
```

### 3.3 パターンマッチングルール（2パス方式）

**Pass 1: Dotted Patterns（優先）**
- パターンに`.`が含まれる場合、厳密マッチ
- `www.google` → `www.google.com`, `www.google.co.jp` にマッチ
- prefix対応: `domain_lower.startswith(pattern + ".")`

**Pass 2: Substring Patterns**
- パターンに`.`が含まれない場合、部分一致
- `youtube` → `www.youtube.com`, `m.youtube.com` にマッチ

### 3.4 登録例

```json
// 接続確認通信（バックグラウンド）
"connectivitycheck.gstatic.com": {
  "service": "NCSI",
  "category": "Infrastructure",
  "role": "auxiliary"
},

// SNSサービス（プライマリ）
"instagram": {
  "service": "Instagram",
  "category": "SNS"
},

// CDN（バックグラウンド）
"akamaized.net": {
  "service": "Akamai",
  "category": "CDN"
}
```

---

## 4. エミュレータテスト

### 4.1 test_matching.py の使用方法

```bash
cd /path/to/code/orangePi/is20s/tools

# 単一ドメインテスト
./test_matching.py youtube.com

# 複数ドメインテスト
./test_matching.py youtube.com netflix.com amazon.co.jp

# 詳細モード（マッチしたパターンを表示）
./test_matching.py -v youtube.com

# バッチテスト（ファイルから）
./test_matching.py -f /tmp/unknown_domains.txt

# 辞書のパターン数を確認
./test_matching.py --count
```

### 4.2 期待される出力

```
# 成功例
youtube.com -> YouTube (Streaming)
www.google.com -> Google Background (Infrastructure) [auxiliary]

# 失敗例（未登録）
unknown-domain.com -> (unknown)
```

### 4.3 テスト必須項目

1. **登録したドメインが正しくマッチすること**
2. **既存のパターンに影響がないこと**
3. **Unknown: 0 になること**（バッチテスト時）

---

## 5. 3デバイス同期

### 5.1 SSH認証情報

```
User: mijeosadmin
Password: mijeos12345@
```

### 5.2 デプロイ対象

| デバイス | IPアドレス |
|----------|------------|
| akb | 192.168.3.250 |
| tam | 192.168.125.248 |
| to | 192.168.126.249 |

### 5.3 デプロイコマンド

```bash
# 辞書ファイルパス（ローカル）
DICT_PATH="/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/aranea_ISMS/code/orangePi/is20s/data/default_domain_services.json"

# akb
SSHPASS='mijeos12345@' sshpass -e scp -o StrictHostKeyChecking=no -o ConnectTimeout=15 "$DICT_PATH" mijeosadmin@192.168.3.250:/opt/is20s/data/

# tam
SSHPASS='mijeos12345@' sshpass -e scp -o StrictHostKeyChecking=no -o ConnectTimeout=15 "$DICT_PATH" mijeosadmin@192.168.125.248:/opt/is20s/data/

# to
SSHPASS='mijeos12345@' sshpass -e scp -o StrictHostKeyChecking=no -o ConnectTimeout=20 "$DICT_PATH" mijeosadmin@192.168.126.249:/opt/is20s/data/
```

### 5.4 サービス再起動（ランタイム辞書リセット付き）

```bash
# akb
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no -o ConnectTimeout=10 mijeosadmin@192.168.3.250 "echo 'mijeos12345@' | sudo -S rm -f /var/lib/is20s/domain_services.json && echo 'mijeos12345@' | sudo -S systemctl restart is20s && echo 'akb: restarted'"

# tam
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no -o ConnectTimeout=15 mijeosadmin@192.168.125.248 "echo 'mijeos12345@' | sudo -S rm -f /var/lib/is20s/domain_services.json && echo 'mijeos12345@' | sudo -S systemctl restart is20s && echo 'tam: restarted'"

# to
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no -o ConnectTimeout=20 mijeosadmin@192.168.126.249 "echo 'mijeos12345@' | sudo -S rm -f /var/lib/is20s/domain_services.json && echo 'mijeos12345@' | sudo -S systemctl restart is20s && echo 'to: restarted'"
```

**重要**: `/var/lib/is20s/domain_services.json`（ランタイム辞書）を削除しないと、新しいパターンが反映されない。

### 5.5 デプロイ後の確認

```bash
# API経由でカテゴリ分布を確認
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no -o ConnectTimeout=15 mijeosadmin@192.168.125.248 "curl -s 'http://localhost:8080/api/capture/events?limit=200' | python3 -c '
import json, sys
from collections import Counter
d = json.load(sys.stdin)
events = d.get(\"events\", [])
cats = Counter(e.get(\"domain_category\") or \"(none)\" for e in events)
print(f\"=== カテゴリ分布 (計{len(events)}件) ===\")
for cat, cnt in cats.most_common():
    print(f\"  {cat}: {cnt}\")
'"
```

---

## 6. チェックリスト

デプロイ前に以下を確認すること:

- [ ] ドメインの用途を調査した
- [ ] 適切なカテゴリを選択した（PRIMARY/SECONDARY）
- [ ] バックグラウンド通信には`role: "auxiliary"`を設定した
- [ ] `test_matching.py`でマッチを確認した
- [ ] 既存パターンへの影響がないことを確認した
- [ ] 3デバイス全てにSCPでデプロイした
- [ ] ランタイム辞書を削除してサービス再起動した
- [ ] API経由でカテゴリ分布を確認した

---

## 7. トラブルシューティング

### 7.1 パターンがマッチしない

1. **パターンの優先順位を確認**: dotted patterns (Pass 1) が先に評価される
2. **大文字小文字**: マッチングは小文字に正規化される
3. **prefix対応**: `www.google` は `www.google.com` にマッチするが、`google` にはマッチしない

### 7.2 デプロイ後に反映されない

1. **ランタイム辞書を削除**: `/var/lib/is20s/domain_services.json`
2. **サービス再起動**: `sudo systemctl restart is20s`
3. **イベントバッファのクリア**: 既存のイベントは再分類されない。新規イベントを待つ

### 7.3 PRIMARY_CATEGORIESの誤用

**問題**: バックグラウンド通信をSearchやStreamingに分類すると、フィルタをバイパスしてUI大量表示になる

**解決**: 接続確認やテレメトリは必ず`Infrastructure`カテゴリ + `role: "auxiliary"`を使用

---

## 8. 参考資料

- `test_matching.py`: デバイスと同一の2パスマッチングロジック実装
- `app/domain_services.py`: 実際のマッチング処理
- `app/capture.py`: `_should_include_in_display()`でフィルタリング処理
- `data/default_domain_services.json`: デフォルト辞書
