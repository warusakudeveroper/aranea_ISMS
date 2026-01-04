# IS-20S ドメイン分類ロジック検証報告書

**作成日**: 2026-01-04
**対象システム**: IS-20S Network Capture Gateway
**報告者**: Claude Code

---

## 1. 問題概要

tam デバイス (192.168.125.248) において、ユーザーがYouTubeやInstagramを利用していると推測される状況で、統計の大部分が「CDN」カテゴリに分類されている問題。

### 観測データ

| カテゴリ | 件数 | 割合 |
|---------|------|------|
| CDN | 87 | 87% |
| Cloud | 13 | 13% |

**CDN内訳**: 全87件が `www.gstatic.com` → "Google Static" / "CDN"

---

## 2. サンプルデータ

### イベント例

```json
{
  "id": "20520260104095846-0783",
  "time": "2026-01-04T09:58:46.041831+09:00",
  "room_no": "205",
  "src_ip": "192.168.125.179",
  "dst_ip": "142.250.76.131",
  "dst_port": "443",
  "protocol": "udp",
  "tls_sni": "www.gstatic.com",
  "domain_service": "Google Static",
  "domain_category": "CDN"
}
```

### 観測ドメイン一覧

| ドメイン | サービス | カテゴリ | 件数 |
|---------|---------|---------|------|
| www.gstatic.com | Google Static | CDN | 87 |

---

## 3. 現在の判定ロジック

### ファイル位置
- [domain_services.py](../../code/orangePi/is20s/app/domain_services.py)

### マッチングアルゴリズム (L646-L662)

```python
for pattern, info in self._data.items():
    if "." in pattern:
        # パターンにドットがある場合は厳密マッチ
        if (
            domain_lower == pattern
            or domain_lower.endswith("." + pattern)
            or ("." + pattern + ".") in domain_lower
        ):
            result = (info.get("service"), info.get("category"))
            break
    else:
        # パターンにドットがない場合は部分マッチ
        if pattern in domain_lower:
            result = (info.get("service"), info.get("category"))
            break
```

### 関連パターン定義 (L301-L317)

```python
# gstatic細分化（汎用より先に）
"fonts.gstatic": {"service": "Google Fonts", "category": "CDN"},
"maps.gstatic": {"service": "Google Maps", "category": "Cloud"},
"youtube.gstatic": {"service": "YouTube", "category": "Streaming"},
...
# 汎用（最後にマッチ）
"gstatic": {"service": "Google Static", "category": "CDN"},
```

### マッチング優先順位の問題

Python の `dict` は挿入順序を保持するが、パターンマッチングは **最初にマッチしたものが採用** される。

`www.gstatic.com` に対して:
1. `fonts.gstatic` → マッチしない（`.fonts.gstatic.` が含まれない）
2. `maps.gstatic` → マッチしない
3. `youtube.gstatic` → マッチしない
4. ...
5. `gstatic` → **マッチ**（部分文字列として含まれる）

結果: "Google Static" / "CDN" に分類

---

## 4. 技術的制約

### TLS/SNI ベース分析の限界

IS-20S は TLS Client Hello の SNI (Server Name Indication) フィールドを解析してドメイン名を取得している。

**取得可能な情報**:
- ドメイン名 (例: `www.gstatic.com`)
- 宛先IP/ポート
- プロトコル

**取得不可能な情報**:
- HTTPリクエストパス (例: `/youtube/thumbnail/xxx.jpg`)
- Content-Type
- リファラー
- 実際のコンテンツ

### www.gstatic.com の特性

`www.gstatic.com` は Google の汎用 CDN エンドポイントで、以下のサービスの静的リソースを配信:

| サービス | 用途例 |
|---------|--------|
| YouTube | サムネイル、プレイヤーUI |
| Google検索 | 検索結果の画像 |
| Gmail | UI画像、アイコン |
| Google Maps | UI要素 |
| Google Fonts | フォントファイル |

**SNI 情報だけでは、どのサービス向けのリソースかを判別することは原理的に不可能**

---

## 5. 想定される原因

### シナリオ A: YouTubeのバックグラウンド通信
- YouTube動画再生中、動画ストリーム本体は `googlevideo.com` に行く
- しかしサムネイルやUI要素は `www.gstatic.com` から取得
- バックグラウンドフィルタが `googlevideo.com` を除外している可能性

### シナリオ B: 実際の利用がCDNアクセス中心
- ユーザーがYouTubeを開いていても、実際にはサムネイル閲覧のみ
- 動画再生をしていない場合、`googlevideo.com` 通信は発生しない

### シナリオ C: パターン順序の問題
- より具体的なパターン（`youtube.gstatic`）が先にマッチすべきだが、
- `www.gstatic.com` は `youtube.gstatic` パターンにマッチしない

---

## 6. 改善案

### 案1: www.gstatic.com 専用パターン追加

```python
"www.gstatic": {"service": "Google CDN", "category": "CDN"},
```

→ 分類は変わらないが、サービス名がより明確になる

### 案2: gstatic を Streaming に変更

```python
"gstatic": {"service": "Google Media", "category": "Streaming"},
```

→ YouTube利用が多い環境では合理的だが、Gmail等での利用も含まれてしまう

### 案3: バックグラウンドフィルタの見直し

`googlevideo.com` がフィルタされていないか確認し、YouTube動画ストリームが正しくキャプチャされているか検証

### 案4: 複合判定の導入

同一IPアドレスへの他のドメインアクセスを参照し、
`www.gstatic.com` と `youtube.com` が同一IPから来ている場合は
YouTube関連と推定する（実装複雑度: 高）

---

## 7. 参照ファイル

- **判定ロジック**: [domain_services.py](../../code/orangePi/is20s/app/domain_services.py)
- **パケットキャプチャ**: [capture.py](../../code/orangePi/is20s/app/capture.py)
- **UI実装**: [ui.py](../../code/orangePi/is20s/app/ui.py)

---

## 8. 結論

現在の判定ロジックは **技術的に正しく動作している**。

`www.gstatic.com` が「Google Static / CDN」に分類されるのは、パターンマッチングの設計通りの動作である。

しかし、**ユーザー体験の観点** から、YouTubeやInstagramの利用を検知したい場合、以下の検討が必要:

1. `gstatic` パターンのカテゴリ見直し
2. `googlevideo.com` 通信がキャプチャされているかの確認
3. Instagram (`cdninstagram.com`) 通信の有無確認

**推奨アクション**: まず `googlevideo.com` や `instagram.com` 関連の通信がフィルタされていないか確認する

---

## 9. 追加検証結果 (2026-01-04 10:00)

### フィルタ設定確認

`capture.py` のバックグラウンドフィルタを確認した結果:

- `googlevideo` → **フィルタ対象外**
- `youtube` → **フィルタ対象外**
- `instagram` → **フィルタ対象外**

### 実トラフィック検索結果

tam の直近500件のイベントで YouTube/Instagram 関連ドメインを検索:

```
=== YouTube/Instagram関連 (0件) ===
  (該当なし - フィルタされているか、通信がない可能性)
```

### 結論

**YouTube/Instagram トラフィックはフィルタされていない**が、**実際にキャプチャされたトラフィックに含まれていない**。

考えられる原因:
1. 対象room (205) のデバイスがYouTube動画を再生していない（サムネイル閲覧のみ）
2. 動画再生はあるがサンプリング期間外
3. `www.gstatic.com` アクセスは Google検索 や Gmail など別サービスによるもの

---

## 10. 参照ドキュメント

- [ドメインパターン一覧](./domain_patterns_list.md)
- [辞書ファイルスナップショット](./domain_services_snapshot_20260104.py)
