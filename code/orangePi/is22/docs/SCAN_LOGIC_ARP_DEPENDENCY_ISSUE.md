# IS22 スキャンロジック問題分析: ARP依存による検出漏れ

## 1. 問題概要

IS22のIPカメラスキャン機能において、ARPスキャンの結果がホスト発見の唯一のゲートとなっており、ARP応答を返さないデバイス（ネットワーク不安定、パケットロス、遅延応答など）が完全にスキップされる問題が発生している。

**重要度**: 高（カメラ検出の根幹に関わる問題）

## 2. 現行の動作フロー

```
Stage 0: 初期化
    ↓
Stage 1-2: ホスト発見 + MAC/OUI取得
    ├── L2サブネット → arp-scan 実行
    │   └── 結果を alive_hosts リストに追加
    └── L3サブネット → TCP connect discovery
        └── 結果を alive_hosts リストに追加
    ↓
Stage 3: ポートスキャン
    └── alive_hosts のみを対象にポートスキャン ← ★問題点
    ↓
Stage 4-5: ONVIF/RTSPプローブ
    └── ポートが開いているホストのみ対象
    ↓
Stage 6-7: カメラ判定・認証
    └── スコアベースでカメラ判定、認証試行
```

### 問題のコード箇所

**mod.rs (lines 261-297)**:
```rust
// Stage 1 & 2: Host Discovery + MAC/OUI
// For L2 subnets: Use ARP scan (gets MAC addresses)
for target in &l2_targets {
    let results = arp_scan_subnet(target, None).await;
    for result in results {
        arp_results.insert(result.ip, (result.mac.clone(), result.vendor.clone()));
        if !alive_hosts.contains(&result.ip) {
            alive_hosts.push(result.ip);  // ★ ARPで検出されたホストのみ追加
        }
    }
}
```

**Stage 3以降は `alive_hosts` のみを走査**:
```rust
// Stage 3: Port Scanning
for ip in &alive_hosts {
    // ARPで検出されなかったホストはここで完全にスキップ
    let port_results = port_scan_host(*ip, &CAMERA_PORTS, timeout_ms).await;
    // ...
}
```

## 3. 検証結果（2025-01-02実施）

### 3.1 対象カメラの状態

| IP | 直接テスト結果 | スキャン結果 | 問題 |
|----|--------------|------------|------|
| 192.168.125.78 | Ping 50%損失、RTSP/ONVIF動作OK | **スキャン結果に含まれず** | ARP応答漏れ |
| 192.168.125.12 | Ping OK、ポートOpen、RTSP 400 Bad Request | credential_status=failed | カメラ側問題 |
| 192.168.125.13 | 100%パケットロス | スキャン結果に含まれず | オフライン（正常動作） |

### 3.2 192.168.125.78 の詳細検証

**直接アクセステスト**:
```bash
# Ping (50% packet loss - 不安定)
$ ping -c 4 192.168.125.78
4 packets transmitted, 2 packets received, 50.0% packet loss

# ポートチェック (Open確認)
192.168.125.78:554 (RTSP) - Open
192.168.125.78:2020 (Onvif) - Open

# スナップショット取得 (成功)
$ ffmpeg -rtsp_transport tcp -i 'rtsp://halecam:hale0083%40@192.168.125.78:554/stream1' -frames:v 1 -f image2 /tmp/test_78.jpg
→ 135KB の画像取得成功
```

**スキャン結果**: デバイスリストに存在せず

**原因分析**:
- ARPスキャン実行時にこのカメラが応答を返さなかった（50%パケットロスの不安定なネットワーク状態）
- `alive_hosts` リストに追加されなかった
- Stage 3以降のポートスキャンが一切実行されなかった
- RTSPもONVIFも正常に動作するカメラが完全にスキップされた

### 3.3 ARPスキャン時のログ

スキャン中にUIで確認されたARP結果:
```
192.168.125.78 | A8:42:A1:B9:53:23 | Hikvision
```
→ ARPには一瞬現れたが、不安定なためalive_hostsへの追加処理で漏れた可能性

## 4. 問題の本質

**ARPは「参照情報」であるべきだが、現在は「発見のゲート」になっている**

- ARPスキャンは高速でMAC/OUI情報を取得できるメリットがある
- しかし、ARP応答は保証されない（パケットロス、遅延、ファイアウォール設定など）
- IPカメラの検出は「カメラポート（554, 2020）が開いているか」が本質的な判断基準
- ARP漏れ = カメラ検出漏れ という現状は設計上の欠陥

## 5. 修正案

### 5.1 案A: ARP-lessポートスキャン（推奨）

**概要**: カメラ専用ポート（554, 2020）についてはARP結果に依存せず、サブネット全体をスキャン

```rust
// Stage 3改良案
// ARP結果に関係なく、カメラ専用ポートは全IPをスキャン
let camera_ports = vec![554, 2020];

for ip_suffix in 1..255 {
    let ip = format!("{}.{}", subnet_prefix, ip_suffix).parse::<Ipv4Addr>()?;

    // カメラポートのみ高速スキャン（短いタイムアウト）
    let port_results = port_scan_host(ip, &camera_ports, 500).await;

    if !port_results.is_empty() {
        // ポートが開いていればalive_hostsに追加
        if !alive_hosts.contains(&ip) {
            alive_hosts.push(ip);
        }
    }
}

// その後、alive_hostsに対して詳細スキャン
```

**メリット**:
- ARP漏れによる検出漏れを完全に防止
- カメラ専用ポートに限定することで速度低下を最小化
- /24サブネットで254IP × 2ポート = 約2分以内

**デメリット**:
- 若干のスキャン時間増加

### 5.2 案B: 登録済みカメラの強制スキャン

**概要**: 既にDB登録されているカメラIPは、ARP結果に関係なく強制的にスキャン対象に追加

```rust
// スキャン開始前に登録済みカメラIPを取得
let registered_cameras = db.get_registered_camera_ips(&target_subnet).await?;

// ARP結果に追加
for ip in registered_cameras {
    if !alive_hosts.contains(&ip) {
        alive_hosts.push(ip);
    }
}
```

**メリット**:
- 既知のカメラは確実にチェック
- 最小限の変更で実装可能

**デメリット**:
- 新規カメラの発見漏れは解決しない

### 5.3 案C: ARPリトライ + 拡張タイムアウト

**概要**: ARP応答が不安定なデバイス向けにリトライと長めのタイムアウトを設定

```rust
async fn arp_scan_with_retry(subnet: &str, retries: u32) -> Vec<ArpResult> {
    let mut all_results = HashMap::new();

    for i in 0..retries {
        let results = arp_scan_subnet(subnet, Some(Duration::from_secs(3))).await;
        for result in results {
            all_results.entry(result.ip).or_insert(result);
        }
        // リトライ間隔
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    all_results.into_values().collect()
}
```

**メリット**:
- 一時的なネットワーク不安定に対応

**デメリット**:
- スキャン時間が大幅増加
- 根本解決にならない

### 5.4 推奨: 案A + 案B のハイブリッド

1. **登録済みカメラ**は無条件で`alive_hosts`に追加
2. **新規発見**はカメラポート（554, 2020）の直接スキャンで補完
3. **ARP結果**はMAC/OUI情報の取得にのみ使用（ゲートとしては使用しない）

## 6. 実装優先度

| 優先度 | 項目 | 理由 |
|-------|------|------|
| P0 | 案B: 登録済みカメラ強制スキャン | 既存ユーザー影響回避、最小変更 |
| P1 | 案A: カメラポート直接スキャン | 新規発見の確実性向上 |
| P2 | 案C: ARPリトライ | 補助的な改善 |

## 7. テスト計画

1. **修正前**: 192.168.125.78 がスキャン結果に含まれないことを確認
2. **修正後**: 同じカメラがスキャン結果に含まれることを確認
3. **回帰テスト**: 正常なカメラ（.45, .62, .17, .19, .79）が引き続き検出されることを確認

## 8. 関連ファイル

- `code/orangePi/is22/src/ipcam_scan/mod.rs` - メインスキャンフロー
- `code/orangePi/is22/src/ipcam_scan/scanner.rs` - スキャン関数群

## 9. 参考情報

- 検証日時: 2025-01-02
- 検証環境: HALE玉造 192.168.125.0/24サブネット
- 検証ツール: is22 Camserver v0.1.0, Chrome UI, ffmpeg
