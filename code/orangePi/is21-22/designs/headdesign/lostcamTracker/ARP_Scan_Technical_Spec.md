# ARPスキャン技術仕様

## 概要

LostCamTrackerで使用するARPスキャンの技術詳細。

---

## ARPプロトコル基礎

### ARP (Address Resolution Protocol)

- OSI Layer 2 (データリンク層) で動作
- IPアドレス → MACアドレスの解決
- ブロードキャストで問い合わせ、ユニキャストで応答

```
┌──────────────┐                      ┌──────────────┐
│   is22       │  ARP Request (BC)    │   Camera     │
│              │─────────────────────▶│              │
│ 192.168.125.246                     │ 192.168.125.45
│              │  ARP Reply (UC)      │              │
│              │◀─────────────────────│              │
└──────────────┘                      └──────────────┘
        │                                    │
        │  Who has 192.168.125.45?           │
        │  Tell 192.168.125.246              │
        │                                    │
        │  192.168.125.45 is at              │
        │  A8:42:A1:B9:53:23                 │
```

---

## 実装方法

### 方法1: arp-scan コマンド (推奨)

**インストール:**
```bash
# Debian/Ubuntu (Armbian)
sudo apt install arp-scan
```

**実行例:**
```bash
# サブネット全体スキャン
sudo arp-scan --interface=eth0 192.168.125.0/24

# 出力例:
# 192.168.125.1    00:1a:2b:3c:4d:5e    TP-LINK TECHNOLOGIES
# 192.168.125.45   a8:42:a1:b9:53:23    TP-LINK TECHNOLOGIES
# 192.168.125.62   dc:62:79:8d:08:ea    TP-LINK TECHNOLOGIES
```

**パース:**
```rust
// Rust実装例
use std::process::Command;

pub struct ArpEntry {
    pub ip: String,
    pub mac: String,
    pub vendor: Option<String>,
}

pub fn arp_scan(interface: &str, subnet: &str) -> Result<Vec<ArpEntry>, Error> {
    let output = Command::new("sudo")
        .args(["arp-scan", "--interface", interface, subnet])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries: Vec<ArpEntry> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0].contains('.') {
                Some(ArpEntry {
                    ip: parts[0].to_string(),
                    mac: parts[1].to_uppercase().replace(":", "").replace("-", ""),
                    vendor: parts.get(2).map(|s| s.to_string()),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(entries)
}
```

### 方法2: arping (単一ホスト)

```bash
# 特定IPのMAC取得
arping -c 1 192.168.125.45
```

### 方法3: Python scapy (プログラム内実行)

```python
from scapy.all import ARP, Ether, srp

def arp_scan(subnet):
    arp = ARP(pdst=subnet)
    ether = Ether(dst="ff:ff:ff:ff:ff:ff")
    packet = ether/arp

    result = srp(packet, timeout=3, verbose=0)[0]

    devices = []
    for sent, received in result:
        devices.append({
            'ip': received.psrc,
            'mac': received.hwsrc.upper().replace(':', '')
        })

    return devices

# 使用例
devices = arp_scan("192.168.125.0/24")
```

### 方法4: システムARPキャッシュ参照

```bash
# 既存のARPキャッシュ確認 (スキャンなし)
arp -a
cat /proc/net/arp
ip neigh show
```

**注意:** キャッシュは古い可能性あり。能動的スキャンが推奨。

---

## 権限要件

### sudoが必要な理由

ARPスキャンはraw socketを使用するため、root権限が必要：

```bash
# sudoersに追加 (パスワードなし実行)
echo "mijeosadmin ALL=(ALL) NOPASSWD: /usr/bin/arp-scan" | sudo tee /etc/sudoers.d/arp-scan
```

### Rustプロセスからの実行

```rust
// camserver実行時にsudo権限を継承しない場合
Command::new("sudo")
    .args(["arp-scan", ...])
    .output()
```

---

## パフォーマンス

### ベンチマーク (/24サブネット)

| 項目 | 値 |
|------|-----|
| 送信パケット数 | 254 |
| 受信パケット数 | 応答デバイス数 |
| 所要時間 | 1-3秒 |
| メモリ使用 | 〜1MB |
| CPU使用 | 無視できるレベル |
| ネットワーク帯域 | 〜10KB |

### 同時実行制限

```rust
// 複数サブネットを同時スキャンする場合
use tokio::sync::Semaphore;

static ARP_SCAN_SEMAPHORE: Semaphore = Semaphore::const_new(2); // 最大2並列

async fn arp_scan_with_limit(subnet: &str) -> Result<Vec<ArpEntry>> {
    let _permit = ARP_SCAN_SEMAPHORE.acquire().await?;
    // スキャン実行
}
```

---

## MAC照合ロジック

### 正規化比較

```rust
/// MAC正規化: 大文字化 + セパレータ除去
fn normalize_mac(mac: &str) -> String {
    mac.to_uppercase()
       .replace(":", "")
       .replace("-", "")
       .replace(".", "")
}

/// lacis_idからMAC抽出 (位置4-15)
fn extract_mac_from_lacis_id(lacis_id: &str) -> Option<String> {
    if lacis_id.len() >= 16 {
        Some(lacis_id[4..16].to_uppercase())
    } else {
        None
    }
}

/// MAC照合
fn find_camera_by_mac(
    arp_results: &[ArpEntry],
    cameras: &[Camera]
) -> Vec<(Camera, ArpEntry)> {
    let mut matches = Vec::new();

    for camera in cameras {
        if let Some(camera_mac) = extract_mac_from_lacis_id(&camera.lacis_id) {
            for entry in arp_results {
                let arp_mac = normalize_mac(&entry.mac);
                if camera_mac == arp_mac {
                    matches.push((camera.clone(), entry.clone()));
                }
            }
        }
    }

    matches
}
```

---

## エラーハンドリング

### 想定されるエラー

| エラー | 原因 | 対処 |
|--------|------|------|
| `arp-scan: command not found` | 未インストール | apt install arp-scan |
| `Operation not permitted` | 権限不足 | sudoers設定 |
| `No such device` | インターフェース名誤り | ip link で確認 |
| `Network is unreachable` | サブネット到達不可 | ルーティング確認 |

### タイムアウト処理

```rust
use tokio::time::timeout;
use std::time::Duration;

async fn arp_scan_with_timeout(subnet: &str) -> Result<Vec<ArpEntry>> {
    timeout(Duration::from_secs(10), arp_scan(subnet))
        .await
        .map_err(|_| Error::Timeout("ARP scan timed out"))?
}
```

---

## セキュリティ考慮

### ARP Spoofing検知との共存

- ARPスキャンはネットワーク監視システムに検知される可能性
- 正当な目的であることをドキュメント化
- スキャン頻度を制限（1時間に1回程度）

### ログ記録

```rust
tracing::info!(
    subnet = %subnet,
    device_count = %results.len(),
    "ARP scan completed"
);
```

---

## is20s連携（将来）

### is20sへのAPI追加案

```
GET /api/arp-scan?subnet=192.168.125.0/24
```

```json
{
  "ok": true,
  "data": {
    "subnet": "192.168.125.0/24",
    "scan_time": "2026-01-16T10:00:00Z",
    "entries": [
      {"ip": "192.168.125.45", "mac": "A842A1B95323"},
      {"ip": "192.168.125.62", "mac": "DC62798D08EA"}
    ]
  }
}
```

### is22からの呼び出し

```rust
async fn arp_scan_via_is20s(is20s_url: &str, subnet: &str) -> Result<Vec<ArpEntry>> {
    let url = format!("{}/api/arp-scan?subnet={}", is20s_url, subnet);
    let resp: ArpScanResponse = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.entries)
}
```

---

## 変更履歴

| 日付 | 版 | 内容 |
|------|-----|------|
| 2026-01-16 | 1.0 | 初版作成 |
