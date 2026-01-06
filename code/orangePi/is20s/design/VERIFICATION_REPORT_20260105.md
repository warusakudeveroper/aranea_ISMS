# is20s 実装検証レポート

**検証日時**: 2026/01/05 09:30
**検証対象**: tam (192.168.125.248), to (192.168.126.248)
**基準文書**: CORE_CONCEPT.md, CATEGORY_TAXONOMY.md

---

## 1. 検証サマリー

### 総合判定: ⚠️ 要改善

| 機能 | 部屋ヘルス監視 | マーケティング分析 |
|-----|--------------|------------------|
| 現状 | ✅ 機能している | ❌ 実質無効 |

---

## 2. 数値データ

### tam (192.168.125.248) Statistics

| カテゴリ | 件数 | 割合 |
|---------|------|------|
| IoT | 161 | 69.1% |
| Cloud | 35 | 15.0% |
| Infrastructure | 34 | 14.6% |
| Unknown | 3 | 1.3% |
| **合計** | **233** | 100% |

**Primary カテゴリ（マーケ分析対象）**: 0件

### to (192.168.126.248) Statistics

| カテゴリ | 件数 | 割合 |
|---------|------|------|
| IoT | 240+ | 80%+ |
| Infrastructure | 表示あり | - |
| Cloud | 表示あり | - |

**Primary カテゴリ（マーケ分析対象）**: 0件

---

## 3. CORE_CONCEPT.md との対比検証

### 3.1 インフラヘルス監視機能

| 要件 | 期待 | 現状 | 判定 |
|-----|------|------|------|
| IoT機器稼働確認 | Chromecast/linkS2等が表示 | Google Cast 161件表示 | ✅ |
| 部屋別表示 | 部屋番号順でソート | 101→306順で表示 | ✅ |
| DNS/DHCP確認 | Infrastructure表示 | 表示あり | ✅ |

**結論**: インフラヘルス監視は機能している

### 3.2 マーケティング分析機能

| 要件 | 期待 | 現状 | 判定 |
|-----|------|------|------|
| SNS利用傾向 | Instagram/TikTok等表示 | 0件 | ⚠️* |
| Streaming利用 | YouTube/Netflix等表示 | 0件 | ⚠️* |
| Gaming利用 | Steam/PS等表示 | 0件 | ⚠️* |
| IoT除外オプション | マーケ時に除外可能 | 除外不可・80%占有 | ❌ |
| NTPデフォルト除外 | 常時除外 | 34件表示（除外されず） | ❌ |

*⚠️ 環境要因（IoT機器中心でゲスト利用データなし）の可能性が高いが、IoT支配により仮にあっても埋もれる構造

**結論**: マーケティング分析は実質無効（無用の長物化）

### 3.3 表示モード要件

| 要件 | 期待 | 現状 | 判定 |
|-----|------|------|------|
| INFRA_HEALTH モード | IoT/Infrastructure表示 | - | 未実装 |
| MARKETING モード | IoT/Infrastructure除外 | - | 未実装 |
| NTP常時除外 | 全モードで除外 | 表示される | ❌ |

---

## 4. 根本原因の詳細分析（2026/01/05 追加調査）

### 4.0 重大発見: display_bufferフィルタによるデータ消失

**問題の本質**: ndjsonには全イベントが記録されているが、API/UI表示用の`display_buffer`に入る前にデフォルトフィルタで大量除外されている。

#### 証拠: 生データ vs API返却データ

| データソース | Photo | Search | EC | SNS/Streaming |
|-------------|-------|--------|-----|---------------|
| ndjson（生データ） | **14,960件** | **1,260件** | **127件** | 環境要因で0 |
| API返却 | 0件 | 0件 | 0件 | 0件 |

**乖離率: 100%** — ndjsonに存在するデータがAPIで全て消えている

#### capture.py:80-88 デフォルトフィルタ設定

```python
"display_filter": {
    "exclude_local_ip": True,      # ← P0: 全DNS除外の原因
    "exclude_ptr": True,
    "exclude_photo_sync": True,    # ← P0: Photo 14,960件消失の原因
    "exclude_os_check": True,
    "exclude_ad_tracker": True,
    "exclude_dns": True,           # ← コメントと動作が不一致
    "exclude_background": True,
    "exclude_categories": [],
}
```

### 4.1 最優先（P0）: `exclude_local_ip` によるDNS全除外

**問題**: capture.py:704-708

```python
if filter_cfg.get("exclude_local_ip", True):
    dst_ip = event.get("dst_ip", "")
    if dst_ip.startswith("192.168.") or dst_ip.startswith("10."):
        return False  # ← ローカルIP宛を全て除外
```

**影響**:
- DNSクエリ: `dst_ip: 192.168.126.1`（ローカルDNS）→ **全除外**
- DNS応答: `dst_ip: 192.168.126.167`（ローカルクライアント）→ **全除外**

**生データサンプル**:
```json
{"dst_ip": "192.168.126.1", "dst_port": "53", "dns_qry": "www.google.com", "domain_category": "Search"}
```
→ `dst_ip`がローカルなので除外、Searchカテゴリは0件になる

**対策**: DNSクエリはカテゴリ分類済みなら除外しない

### 4.2 最優先（P0）: `exclude_photo_sync` による広範な除外

**問題**: capture.py:739-747

```python
if filter_cfg.get("exclude_photo_sync", True):
    photo_patterns = [
        "googleusercontent.com",  # ← 14,960件消失の原因
        "photos.google.com", "lh3.google.com", ...
    ]
```

**影響**: `googleusercontent.com`パターンでGoogle Photos通信が全て除外

**対策**: カテゴリがPhotoの場合のみ適用、または明示的なPhoto除外オプションに変更

### 4.3 高優先（P1）: モード切替UI実装

**問題**: IoT/Infrastructure が常時表示され、マーケ分析が実質不可能

**対策**:
```javascript
const DISPLAY_MODES = {
  INFRA_HEALTH: {
    name: "部屋ヘルス監視",
    exclude: ["NTP"],
    highlight: ["IoT", "Infrastructure"]
  },
  MARKETING: {
    name: "マーケティング分析",
    exclude: ["IoT", "Infrastructure", "NTP", "Cloud"],
    highlight: ["SNS", "Streaming", "Gaming"]
  }
};
```

### 4.4 中優先（P2）: NTP分離・デフォルト除外

**問題**: NTPが Infrastructure に含まれ、除外されていない

**対策**: NTPカテゴリを分離してデフォルト除外

### 4.5 中優先（P2）: バルク化パターン修正

**問題**: `line` → `line.me` 等、誤マッチ防止

**対策**: CATEGORY_TAXONOMY.md のルールに従いパターン修正

---

## 5. 次のアクション

| 優先度 | アクション | 担当 | 期限 |
|--------|-----------|------|------|
| P0 | NTPカテゴリ分離・デフォルト除外 | 開発 | 即座 |
| P1 | モード切替トグル追加 | 開発 | 1週間 |
| P1 | IoT/Infrastructure除外オプション | 開発 | 1週間 |
| P2 | バルク化パターン修正 | 開発 | 2週間 |

---

## 6. 結論

is20sは**インフラヘルス監視としては機能している**が、**マーケティング分析としては無用の長物化**している。

根本原因:
1. NTPがデフォルト除外されていない
2. IoT/Infrastructureの除外オプションがない
3. モード切替概念が未実装

CORE_CONCEPT.md で定義した2つの主要機能（インフラヘルス + マーケティング分析）のうち、後者が実質的に機能していない状態である。

---

## 改訂履歴

| 日付 | バージョン | 内容 |
|------|-----------|------|
| 2026/01/05 | 1.0 | 初版作成 |
