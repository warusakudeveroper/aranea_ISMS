# is22 IpcamScan設計

文書番号: is22_09
作成日: 2025-12-31
ステータス: Draft
参照:
- is22_Camserver仕様案 Section 8
- DESIGN_GAP_ANALYSIS GAP-004
- is22_Phase2_カメラ疎通検証記録

---

## 1. 概要

### 1.1 目的

IpcamScanは、指定サブネット内からIPカメラ候補を効率的に絞り込み、最終的にメーカー/機種を特定するためのモジュールである。

### 1.2 設計原則

- **ブルートフォース禁止**: 段階的エビデンスを積み上げる
- **L3越し対応**: MACが取れない環境でも破綻しない
- **正規クレデンシャルのみ**: 推測・総当たり禁止

### 1.3 Phase 2検証結果の反映

| 検証結果 | IpcamScan設計への反映 |
|---------|---------------------|
| Tapo: 554+2020 open | Stage 3ポートセットに含む |
| Nest: 443+8443 open, RTSP/ONVIF無し | Stage 4 mDNS強化必要 |
| VIGI NVR: 80+443+8000+8443 | Stage 3拡張ポートセット |
| WS-Discovery全滅 | Stage 4でunsupported記録し継続 |
| L3越しMAC取得不可 | Stage 2 mac=null許容 |

---

## 2. アーキテクチャ

### 2.1 コンポーネント構成

```
[IpcamScan]
    ├── JobManager        # ジョブ管理（キュー、状態）
    ├── HostDiscovery     # Stage 1: 存在確認
    ├── MacResolver       # Stage 2: MAC/OUI（取得可能時のみ）
    ├── PortScanner       # Stage 3: ポート指紋
    ├── DiscoveryProber   # Stage 4: WS-Discovery/SSDP/mDNS
    ├── ScoringEngine     # Stage 5: スコアリング
    ├── Verifier          # Stage 6: 確証（正規クレデンシャル）
    └── ResultStore       # 結果保存
```

### 2.2 責務分離（SOLID）

| コンポーネント | 単一責任 |
|--------------|---------|
| JobManager | ジョブのライフサイクル管理 |
| HostDiscovery | IP存在確認のみ |
| MacResolver | MAC/OUI取得のみ |
| PortScanner | ポート状態取得のみ |
| DiscoveryProber | プロトコル応答取得のみ |
| ScoringEngine | スコア計算のみ |
| Verifier | 認証付き確認のみ |

---

## 3. 多段エビデンス方式（Stage 0〜6）

### Stage 0: 準備

| 項目 | 内容 |
|-----|------|
| 入力 | CIDR配列、モード、ポートセット、タイムアウト、並列度 |
| 処理 | CIDR正規化、重複排除、除外設定適用 |
| 出力 | 正規化済みIPリスト |

```rust
struct ScanJob {
    job_id: Uuid,
    targets: Vec<Cidr>,
    mode: ScanMode,           // Discovery / Deep
    ports: Vec<u16>,
    timeout_ms: u32,
    concurrency: u8,
    status: JobStatus,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    summary: Option<JobSummary>,
}
```

### Stage 1: Host Discovery

| 項目 | 内容 |
|-----|------|
| 優先順位 | 1. 既存インベントリ 2. ICMP 3. TCP軽量到達確認 |
| タイムアウト | 500ms |
| 出力 | 存在確認済みIPリスト |

```rust
async fn discover_hosts(targets: &[IpAddr]) -> Vec<HostResult> {
    // 優先順位に従って存在確認
    // 存在しないIPを落とす
}

struct HostResult {
    ip: IpAddr,
    alive: bool,
    method: DiscoveryMethod,  // Icmp / TcpConnect / Known
}
```

### Stage 2: MAC/OUI（取得可能時）

| 項目 | 内容 |
|-----|------|
| 取得方法 | ARP（L2内のみ） |
| L3越し | mac=null として継続 |
| OUI判定 | 粗くベンダ推定（TP-Link/Google/Other） |

```rust
struct MacInfo {
    ip: IpAddr,
    mac: Option<String>,          // L3越しはNone
    oui_vendor: Option<String>,   // TP-Link, Google, etc.
}
```

### Stage 3: ポート指紋

| ポート | 目的 | 重み |
|--------|------|-----|
| 554/tcp | RTSP | +30 |
| 2020/tcp | ONVIF | +30 |
| 80/tcp | 管理UI | +10 |
| 443/tcp | HTTPS管理UI | +10 |
| 8000/tcp | NVR/拡張 | +10 |
| 8080/tcp | 代替HTTP | +5 |
| 8443/tcp | 代替HTTPS | +5 |
| 8554/tcp | 代替RTSP | +5 |

```rust
struct PortScanResult {
    ip: IpAddr,
    ports: Vec<PortStatus>,
}

struct PortStatus {
    port: u16,
    status: PortState,  // Open / Closed / Filtered
}
```

### Stage 4: ディスカバリ

| プロトコル | ポート | 対象 | Phase 2結果 |
|-----------|-------|------|------------|
| WS-Discovery | UDP 3702 | ONVIF | 全滅（Tapoは非対応） |
| SSDP | UDP 1900 | UPnP | 応答なし |
| mDNS | UDP 5353 | Apple系/Nest | 応答なし |

```rust
struct DiscoveryResult {
    ip: IpAddr,
    onvif: Option<OnvifInfo>,
    ssdp: Option<SsdpInfo>,
    mdns: Option<MdnsInfo>,
}

enum DiscoveryStatus {
    Found(String),      // 応答あり
    NotSupported,       // プロトコル非対応
    Blocked,            // ネットワーク制約
    Timeout,            // タイムアウト
}
```

### Stage 5: スコアリング

| エビデンス | 重み |
|-----------|-----|
| 554 open | +30 |
| 2020 open | +30 |
| 80 open | +10 |
| 443 open | +10 |
| OUI=TP-Link | +20 |
| OUI=Google | +20 |
| WS-Discovery応答 | +50 |
| SSDP応答 | +20 |
| mDNS応答 | +20 |

```rust
fn calculate_score(evidence: &DeviceEvidence) -> u32 {
    let mut score = 0;

    // ポート
    if evidence.port_554_open { score += 30; }
    if evidence.port_2020_open { score += 30; }
    if evidence.port_80_open { score += 10; }
    if evidence.port_443_open { score += 10; }

    // OUI
    match &evidence.oui_vendor {
        Some(v) if v.contains("TP-LINK") => score += 20,
        Some(v) if v.contains("GOOGLE") => score += 20,
        _ => {}
    }

    // ディスカバリ
    if evidence.onvif_found { score += 50; }
    if evidence.ssdp_found { score += 20; }
    if evidence.mdns_found { score += 20; }

    score
}
```

### Stage 6: 確証（正規クレデンシャル）

| 項目 | 内容 |
|-----|------|
| 対象 | Stage 5でスコア閾値（例: 40）以上 |
| 試行 | ONVIF認証 → RTSP疎通確認 |
| 制限 | IPごと3回まで、ジョブ全体100回まで |

```rust
struct VerificationResult {
    ip: IpAddr,
    verified: bool,
    manufacturer: Option<String>,
    model: Option<String>,
    firmware: Option<String>,
    rtsp_uri: Option<String>,
    family: CameraFamily,  // Tapo / VIGI / Nest / Other
    confidence: u8,        // 0-100
}

enum CameraFamily {
    Tapo,
    Vigi,
    Nest,
    Other,
    Unknown,
}
```

---

## 4. データモデル

### 4.1 ipcamscan_jobs

```sql
CREATE TABLE ipcamscan_jobs (
    job_id CHAR(36) PRIMARY KEY,
    requested_by VARCHAR(64) NOT NULL,
    targets JSON NOT NULL,              -- ["192.168.96.0/24", ...]
    mode ENUM('discovery', 'deep') NOT NULL DEFAULT 'discovery',
    ports JSON NOT NULL,                -- [554, 2020, 80, 443, ...]
    timeout_ms INT UNSIGNED NOT NULL DEFAULT 500,
    concurrency TINYINT UNSIGNED NOT NULL DEFAULT 10,
    rate_limit_pps INT UNSIGNED NOT NULL DEFAULT 100,
    status ENUM('queued', 'running', 'partial', 'success', 'failed', 'canceled') NOT NULL DEFAULT 'queued',
    started_at DATETIME(3),
    ended_at DATETIME(3),
    summary_json JSON,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX idx_jobs_status (status),
    INDEX idx_jobs_created (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

### 4.2 ipcamscan_devices

```sql
CREATE TABLE ipcamscan_devices (
    device_id CHAR(36) PRIMARY KEY,
    ip VARCHAR(45) NOT NULL,
    subnet VARCHAR(45) NOT NULL,
    mac VARCHAR(17),                    -- L3越しはNULL
    oui_vendor VARCHAR(64),
    hostnames JSON,                     -- ["cam1.local", ...]
    open_ports JSON NOT NULL,           -- [{port: 554, status: "open"}, ...]
    discovery_onvif JSON,
    discovery_ssdp JSON,
    discovery_mdns JSON,
    score INT UNSIGNED NOT NULL DEFAULT 0,
    verified TINYINT(1) NOT NULL DEFAULT 0,
    manufacturer VARCHAR(64),
    model VARCHAR(64),
    firmware VARCHAR(128),
    family ENUM('tapo', 'vigi', 'nest', 'other', 'unknown') NOT NULL DEFAULT 'unknown',
    confidence TINYINT UNSIGNED NOT NULL DEFAULT 0,
    rtsp_uri VARCHAR(512),
    first_seen DATETIME(3) NOT NULL,
    last_seen DATETIME(3) NOT NULL,

    UNIQUE KEY uq_devices_ip (ip),
    INDEX idx_devices_family (family),
    INDEX idx_devices_score (score),
    INDEX idx_devices_verified (verified)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

### 4.3 ipcamscan_evidence

```sql
CREATE TABLE ipcamscan_evidence (
    evidence_id CHAR(36) PRIMARY KEY,
    device_id CHAR(36) NOT NULL,
    job_id CHAR(36) NOT NULL,
    evidence_type ENUM('arp', 'portscan', 'wsd', 'ssdp', 'mdns', 'onvif_auth', 'rtsp_auth') NOT NULL,
    raw_summary TEXT,                   -- マスキング済み要約
    collected_at DATETIME(3) NOT NULL,
    source VARCHAR(64) NOT NULL,        -- 'local', 'agent', 'is20s'

    INDEX idx_evidence_device (device_id),
    INDEX idx_evidence_job (job_id),
    FOREIGN KEY (device_id) REFERENCES ipcamscan_devices(device_id) ON DELETE CASCADE,
    FOREIGN KEY (job_id) REFERENCES ipcamscan_jobs(job_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

---

## 5. API設計

### 5.1 ジョブ管理

```
POST /api/ipcamscan/jobs
```

Request:
```json
{
    "targets": ["192.168.96.0/24", "192.168.125.0/24"],
    "mode": "deep",
    "ports": [554, 2020, 80, 443, 8000, 8443],
    "timeout_ms": 500,
    "concurrency": 10
}
```

Response:
```json
{
    "job_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "queued",
    "created_at": "2025-12-31T10:00:00Z"
}
```

```
GET /api/ipcamscan/jobs/{job_id}
```

Response:
```json
{
    "job_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "success",
    "started_at": "2025-12-31T10:00:01Z",
    "ended_at": "2025-12-31T10:02:30Z",
    "summary": {
        "total_ips": 512,
        "hosts_alive": 45,
        "cameras_found": 12,
        "cameras_verified": 8
    }
}
```

### 5.2 デバイス管理

```
GET /api/ipcamscan/devices?subnet=192.168.96.0/24&family=tapo&verified=true
```

Response:
```json
{
    "devices": [
        {
            "device_id": "...",
            "ip": "192.168.96.85",
            "mac": null,
            "family": "tapo",
            "model": "C310",
            "verified": true,
            "confidence": 95,
            "rtsp_uri": "rtsp://192.168.96.85:554/stream2"
        }
    ],
    "total": 8
}
```

```
POST /api/ipcamscan/devices/{device_id}/verify
```

Request:
```json
{
    "credentials": {
        "username": "ismscam",
        "password": "isms12345@"
    }
}
```

---

## 6. Camserver統合

### 6.1 CameraInventory連携

```
IpcamScan.devices (verified=true)
         ↓
   承認フロー（管理者が選択）
         ↓
   ConfigStore.cameras (SSoT)
```

### 6.2 自動登録禁止

- 誤検出リスク回避のため自動登録しない
- 管理者による承認フローを経由

---

## 7. セキュリティ

### 7.1 スキャン制限

| 項目 | 制限値 |
|-----|-------|
| 許可CIDR | ホワイトリスト運用 |
| 同時接続数 | 10 |
| パケット/秒 | 100 |
| IPごと試行上限 | 3回 |
| ジョブ試行上限 | 100回 |

### 7.2 認証情報保護

- 平文保存禁止（暗号化ストア）
- ログにマスキング
- アクセス監査

---

## 8. テスト観点

### 8.1 Stage別検証

- [ ] Stage 1: ICMP不可環境でもTCPで拾える
- [ ] Stage 2: L2/L3差で動作変化
- [ ] Stage 3: open/closed/filtered判定
- [ ] Stage 4: blocked/unsupported記録
- [ ] Stage 5: スコアが候補を適切に上位へ
- [ ] Stage 6: 認証成功/失敗時の挙動

### 8.2 環境差分

- [ ] AP isolation有無
- [ ] ICMP遮断有無
- [ ] マルチキャスト制限有無
- [ ] VPN越し（L3）

### 8.3 負荷/安全性

- [ ] /24での実行時間
- [ ] 並列度変更時の影響
- [ ] 失敗回数上限/クールダウン動作

---

## 更新履歴

| 日付 | バージョン | 内容 |
|-----|-----------|------|
| 2025-12-31 | 1.0 | 初版作成（Phase 4） |
