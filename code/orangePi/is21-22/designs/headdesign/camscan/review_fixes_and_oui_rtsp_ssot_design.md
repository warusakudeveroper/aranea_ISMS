# 設計レビュー修正 + OUI/RTSPパスSSoT統合設計

## 概要

本ドキュメントは以下の2つを統合した設計である：
1. `Camscan_design_review_report.md`で指摘されたHigh/Medium/Low計8項目の修正
2. OUI情報とカメラブランドRTSPパスのSSoT化・DB統合・UI管理機能

---

# Part 1: レビュー指摘事項の修正

## High #1: クレデンシャル表示方針修正

### 変更内容
デザイナー要求「隠す必要なし」に基づき、試行クレデンシャルをマスクなしで表示する。

**修正前 (category_display_design.md):**
```rust
pub struct TriedCredential {
    pub username: String,
    pub password_masked: String,  // "***"でマスク
}
```

**修正後:**
```rust
pub struct TriedCredential {
    pub username: String,
    pub password: String,  // マスクなし（平文表示）
    pub result: CredentialResult,
}
```

**表示例:**
```
🔐 試行済みクレデンシャル:
  × halecam / halecam12345@  → 認証失敗
  × admin / admin12345@      → タイムアウト
```

**セキュリティ考慮:**
- スキャン結果画面はローカルネットワーク管理者向け
- クレデンシャルは設定で既に登録済みのもの
- トラブルシューティングに平文表示が必要

### 1.2 クレデンシャル監査ポリシー（追加）

#### 保存ポリシー

| データ | 保存場所 | 保存期間 | 暗号化 |
|--------|---------|---------|--------|
| 設定クレデンシャル（マスター） | `credentials`テーブル | 永続 | AES-256暗号化 |
| スキャン時試行結果 | `scan_devices.tried_credentials` | スキャン完了後24時間 | なし（メモリ/一時保存） |
| ログ出力 | `app.log` | 7日間ローテーション | クレデンシャルは出力しない |
| APIレスポンス | HTTP通信 | 一時的 | TLS必須（本番環境） |

#### データフロー

```
設定画面で登録
    │
    ▼
credentials テーブル（AES-256暗号化で永続保存）
    │
    ▼
スキャン実行時に復号化してメモリロード
    │
    ▼
各カメラに対して認証試行
    │
    ├── 成功: camera.username/password に保存（暗号化）
    │
    └── 失敗: tried_credentials[] に記録（平文・24時間後自動削除）
              │
              ▼
         スキャン結果APIで返却（管理者向け画面のみ）
```

#### 自動クリーンアップ

```sql
-- 24時間経過したスキャン結果の試行クレデンシャルをクリア
-- 毎時実行のバッチジョブ
UPDATE scan_devices
SET tried_credentials = NULL
WHERE last_scanned_at < NOW() - INTERVAL 24 HOUR
  AND tried_credentials IS NOT NULL;
```

#### アクセス制御

| エンドポイント | 認証要件 | クレデンシャル返却 |
|--------------|---------|------------------|
| `GET /api/scan/results` | 必須（管理者権限） | 平文で返却 |
| `GET /api/cameras` | 必須（閲覧権限） | マスク（`***`） |
| WebSocket進捗通知 | 必須（管理者権限） | 含まない |

#### マスキング切り替えオプション（将来実装）

```typescript
// フロントエンド設定（将来対応）
interface DisplaySettings {
  maskCredentials: boolean;  // true: 常時マスク, false: 平文表示
}

// デフォルト: false（平文表示、デザイナー要求に準拠）
// 将来的にユーザー設定で切り替え可能に
```

#### ログ出力ルール

```rust
// ログ出力時のクレデンシャル除去
impl TriedCredential {
    fn log_safe(&self) -> String {
        format!("user={}, result={:?}", self.username, self.result)
        // password は絶対にログに出さない
    }
}

// 正: tracing::info!("Auth attempt: {}", cred.log_safe());
// 誤: tracing::info!("Auth attempt: {:?}", cred);  // パスワード露出
```

---

## High #2: ScannedDevice拡張フィールドSSoT統一

### 問題
`category_display_design.md`と`camera_name_display_design.md`が別々に`ScannedDevice`を拡張定義。

### 解決: 統一型定義

**SSoT型定義 (`src/ipcam_scan/types.rs` に一本化):**

```rust
/// スキャン結果デバイス情報（SSoT）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedDevice {
    // === 基本情報 ===
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub vendor: Option<String>,  // OUIから判定
    pub score: u32,
    pub open_ports: Vec<u16>,

    // === スキャン情報 ===
    pub last_scanned_at: DateTime<Utc>,
    pub is_current_scan_target: bool,
    pub subnet: String,

    // === 登録済みカメラ情報 ===
    pub registered_camera_id: Option<i64>,
    pub registered_camera_name: Option<String>,
    pub ip_changed: bool,  // StrayChild検出: MACは一致するがIPが違う

    // === 認証情報 ===
    pub tried_credentials: Vec<TriedCredential>,
    pub auth_status: AuthStatus,

    // === カテゴリ分類 ===
    pub category: DeviceCategory,
    pub category_detail: DeviceCategoryDetail,

    // === プロトコル情報 ===
    pub rtsp_available: bool,
    pub onvif_available: bool,
    pub camera_model: Option<String>,
    pub firmware_version: Option<String>,
    pub camera_family: Option<CameraFamily>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AuthStatus {
    NotTried,       // 認証未試行
    Success,        // 認証成功
    Failed,         // 全クレデンシャル失敗
    Partial,        // 一部成功
    Timeout,        // タイムアウト
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeviceCategory {
    A,  // 登録済み
    B,  // 登録可能
    C,  // 認証待ち
    D,  // その他ネットワークデバイス
    E,  // 非対応デバイス
    F,  // 通信断（登録済みだが応答なし）
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeviceCategoryDetail {
    RegisteredAuthenticated,   // A: 登録済み・認証OK
    RegisteredAuthIssue,       // A: 登録済み・認証要確認
    Registrable,               // B: 登録可能
    AuthRequired,              // C: 認証待ち
    PossibleCamera,            // D: カメラ可能性あり（OUI一致）
    NetworkEquipment,          // D: ネットワーク機器
    IoTDevice,                 // D: IoTデバイス
    UnknownDevice,             // D: 不明
    NonCamera,                 // E: 非カメラ
    LostConnection,            // F: 通信断
    StrayChild,                // F: 迷子カメラ（IP変更検出）
}
```

---

## Medium #3: カテゴリFとStrayChild優先順位

### 判定アルゴリズム

```rust
fn determine_category(device: &ScannedDevice, registered: &[Camera]) -> (DeviceCategory, DeviceCategoryDetail) {
    // 1. 登録済みカメラとのIP照合
    if let Some(camera) = registered.iter().find(|c| c.ip_address == device.ip_address) {
        // 登録済みIPが発見された
        if device.auth_status == AuthStatus::Success {
            return (DeviceCategory::A, DeviceCategoryDetail::RegisteredAuthenticated);
        } else {
            return (DeviceCategory::A, DeviceCategoryDetail::RegisteredAuthIssue);
        }
    }

    // 2. 登録済みカメラとのMAC照合（IP変更検出 = StrayChild）
    if let Some(mac) = &device.mac_address {
        if let Some(camera) = registered.iter().find(|c|
            c.mac_address.as_ref().map(|m| m.eq_ignore_ascii_case(mac)).unwrap_or(false)
        ) {
            // MACは一致するがIPが違う → 迷子カメラ
            return (DeviceCategory::F, DeviceCategoryDetail::StrayChild);
        }
    }

    // 3. 登録済みカメラで今回未応答（通信断）
    // これは登録済みカメラリストから、今回スキャンで発見されなかったものをチェック
    // （外部で判定し、DeviceCategory::F, LostConnectionを付与）

    // 4. RTSP/ONVIF応答による分類
    if device.rtsp_available || device.onvif_available {
        if device.auth_status == AuthStatus::Success {
            return (DeviceCategory::B, DeviceCategoryDetail::Registrable);
        } else {
            return (DeviceCategory::C, DeviceCategoryDetail::AuthRequired);
        }
    }

    // 5. OUI一致チェック
    if device.vendor.is_some() && is_camera_vendor(&device.vendor) {
        return (DeviceCategory::D, DeviceCategoryDetail::PossibleCamera);
    }

    // 6. ポートパターンによる推定
    if is_network_equipment_ports(&device.open_ports) {
        return (DeviceCategory::D, DeviceCategoryDetail::NetworkEquipment);
    }
    if is_iot_device_ports(&device.open_ports) {
        return (DeviceCategory::D, DeviceCategoryDetail::IoTDevice);
    }

    // 7. 何も該当しない
    if device.open_ports.is_empty() {
        return (DeviceCategory::E, DeviceCategoryDetail::NonCamera);
    }

    (DeviceCategory::D, DeviceCategoryDetail::UnknownDevice)
}
```

### 優先順位まとめ

| 優先度 | 条件 | カテゴリ |
|-------|------|---------|
| 1 | IP一致（登録済み） | A |
| 2 | MAC一致・IP不一致（迷子） | F (StrayChild) |
| 3 | 登録済みで今回未応答 | F (LostConnection) |
| 4 | RTSP/ONVIF応答 + 認証成功 | B |
| 5 | RTSP/ONVIF応答 + 認証失敗 | C |
| 6 | OUIカメラベンダー一致 | D (PossibleCamera) |
| 7 | ネットワーク機器推定 | D (NetworkEquipment) |
| 8 | IoTデバイス推定 | D (IoTDevice) |
| 9 | ポート応答あり | D (UnknownDevice) |
| 10 | ポート応答なし | E |

### 3.2 カテゴリF (LostConnection) スキャン結果流し込み設計

#### 問題
「登録済みカメラで今回未応答（通信断）」の判定は、スキャン完了後に登録済みカメラリストとの差分で行う必要があるが、その流し込み方法が未定義だった。

#### 解決: 2段階カテゴリ判定

```
スキャン実行
    │
    ▼
【Phase 1: デバイス発見】
ARP/ポートスキャン → 発見デバイスリスト生成
    │
    ▼
【Phase 2: カテゴリ判定（発見デバイス）】
発見デバイスに対してカテゴリA～Eを判定
（優先順位1～10を適用）
    │
    ▼
【Phase 3: 通信断カメラ判定（LostConnection）】★追加
登録済みカメラリストから、Phase 1で発見されなかったカメラを抽出
→ カテゴリF (LostConnection) として結果に追加
    │
    ▼
スキャン結果統合・返却
```

#### Phase 3 実装詳細

```rust
/// 通信断カメラの検出と結果への追加
fn inject_lost_connection_cameras(
    scan_results: &mut Vec<ScannedDevice>,
    registered_cameras: &[Camera],
    target_subnet: &str,
) {
    // 発見されたIPアドレスのセット
    let discovered_ips: HashSet<&str> = scan_results
        .iter()
        .map(|d| d.ip_address.as_str())
        .collect();

    // 発見されたMACアドレスのセット（StrayChild判定用）
    let discovered_macs: HashSet<String> = scan_results
        .iter()
        .filter_map(|d| d.mac_address.as_ref())
        .map(|m| m.to_uppercase())
        .collect();

    // 対象サブネット内の登録済みカメラをチェック
    for camera in registered_cameras {
        // サブネット外のカメラはスキップ
        if !is_ip_in_subnet(&camera.ip_address, target_subnet) {
            continue;
        }

        // IP発見済み → すでにカテゴリ判定済み
        if discovered_ips.contains(camera.ip_address.as_str()) {
            continue;
        }

        // MAC発見済み → StrayChild（Phase 2でカテゴリF/StrayChildとして判定済み）
        if let Some(mac) = &camera.mac_address {
            if discovered_macs.contains(&mac.to_uppercase()) {
                continue;
            }
        }

        // ★ここに到達 = 登録済みだが未発見 = 通信断
        let lost_device = ScannedDevice {
            ip_address: camera.ip_address.clone(),
            mac_address: camera.mac_address.clone(),
            vendor: None,
            score: 0,
            open_ports: vec![],
            last_scanned_at: Utc::now(),
            is_current_scan_target: true,
            subnet: target_subnet.to_string(),
            registered_camera_id: Some(camera.id),
            registered_camera_name: Some(camera.name.clone()),
            ip_changed: false,
            tried_credentials: vec![],
            auth_status: AuthStatus::NotTried,
            category: DeviceCategory::F,
            category_detail: DeviceCategoryDetail::LostConnection,
            rtsp_available: false,
            onvif_available: false,
            camera_model: camera.model.clone(),
            firmware_version: camera.firmware_version.clone(),
            camera_family: camera.family.clone(),
        };

        scan_results.push(lost_device);
    }
}
```

#### UI表示（カテゴリF）

```
┌─────────────────────────────────────────────────────────────┐
│ ⚠ カテゴリF: 通信断・迷子カメラ                              │
├─────────────────────────────────────────────────────────────┤
│ 【ロビーカメラ3】                                           │
│ 192.168.125.88                                              │
│ ❌ 通信断 - スキャンで応答がありませんでした                  │
│ D8:07:B6:53:00:00 (TP-LINK)                                │
│                                                             │
│ 考えられる原因:                                             │
│ ・電源が切れている                                          │
│ ・ネットワークケーブルが抜けている                           │
│ ・IPアドレスが変更された（DHCPリース切れ）                   │
│                                                             │
│ [カメラ設定を確認] [再スキャン]                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Medium #4: サブネット削除CIDR汎用化

**修正前:**
```rust
sqlx::query("DELETE FROM scan_devices WHERE subnet = ?")
```

**修正後:**
```rust
/// サブネット削除時のスキャン結果クリーンアップ
/// CIDRを正確にパースして該当IPをすべて削除
async fn delete_scan_devices_for_subnet(pool: &MySqlPool, cidr: &str) -> Result<u64> {
    use ipnetwork::IpNetwork;

    let network: IpNetwork = cidr.parse()?;

    // 計算されたネットワークアドレスとブロードキャストで範囲削除
    let network_addr = network.network().to_string();
    let broadcast_addr = network.broadcast().to_string();

    let result = sqlx::query(r#"
        DELETE FROM scan_devices
        WHERE INET_ATON(ip_address) >= INET_ATON(?)
          AND INET_ATON(ip_address) <= INET_ATON(?)
    "#)
    .bind(&network_addr)
    .bind(&broadcast_addr)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
```

### 4.1 IPv4/IPv6対応方針

#### 現行仕様: IPv4専用

本システムは**IPv4専用**として設計する。理由:

1. **対象機器の制約**: IPカメラ（Tapo, Hikvision, Dahua等）の大多数はIPv4のみサポート
2. **ローカルネットワーク運用**: 192.168.x.x/10.x.x.xのプライベートアドレス運用が前提
3. **RTSP/ONVIFプロトコル**: IPv6対応が不完全なベンダーが多い
4. **ARPスキャン**: ARPはIPv4専用プロトコル（IPv6はNDP）

#### スキーマ・クエリの前提

| 項目 | 仕様 |
|-----|-----|
| `ip_address`カラム | VARCHAR(15)（IPv4最大長: 255.255.255.255 = 15文字） |
| サブネットCIDR | IPv4形式のみ（例: 192.168.125.0/24） |
| 範囲クエリ | `INET_ATON()` / `INET_NTOA()` 使用（IPv4専用MySQL関数） |
| Rust側パース | `ipnetwork::Ipv4Network` で明示的にIPv4を要求 |

#### バリデーション追加

```rust
/// サブネット削除時のスキャン結果クリーンアップ（IPv4専用）
async fn delete_scan_devices_for_subnet(pool: &MySqlPool, cidr: &str) -> Result<u64> {
    use ipnetwork::Ipv4Network;  // ★ Ipv4Network を明示的に使用

    // IPv4 CIDR のみ許可（IPv6はエラー）
    let network: Ipv4Network = cidr.parse()
        .map_err(|_| anyhow!("IPv4 CIDR形式で指定してください（例: 192.168.1.0/24）"))?;

    let network_addr = network.network().to_string();
    let broadcast_addr = network.broadcast().to_string();

    let result = sqlx::query(r#"
        DELETE FROM scan_devices
        WHERE INET_ATON(ip_address) >= INET_ATON(?)
          AND INET_ATON(ip_address) <= INET_ATON(?)
    "#)
    .bind(&network_addr)
    .bind(&broadcast_addr)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
```

#### 将来のIPv6対応（参考）

IPv6対応が必要になった場合の拡張ポイント:

```sql
-- MySQL 5.6.3+ / MariaDB 10.0.1+ でIPv6対応
-- INET6_ATON() / INET6_NTOA() を使用

-- スキーマ変更
ALTER TABLE scan_devices
MODIFY ip_address VARCHAR(45);  -- IPv6最大長

-- クエリ変更（IPv4/IPv6両対応）
DELETE FROM scan_devices
WHERE INET6_ATON(ip_address) >= INET6_ATON(?)
  AND INET6_ATON(ip_address) <= INET6_ATON(?);
```

**現時点ではIPv6対応は実装しない。** 将来必要になった場合は別途設計を行う。

---

## Medium #5: OUI追加候補/確定の明確化

**修正後の分類:**

| ステータス | 根拠 | 追加タイミング |
|-----------|------|---------------|
| **確定追加** | IEEE OUIデータベースで確認済み | 即座に実装 |
| **追加候補** | 実機確認が必要 / 複数OUI使用の可能性 | 実機確認後に追加 |
| **調査継続** | OUI未特定 | 情報収集中 |

```rust
/// OUI追加ステータス
pub enum OuiAdditionStatus {
    /// 確定: IEEE確認済み、即座に追加
    Confirmed,
    /// 候補: 実機確認後に追加
    Candidate,
    /// 調査中: OUI未特定
    Investigating,
}
```

**OUI_expansion_design.md への反映:**

| ベンダー | ステータス | 根拠 |
|---------|----------|------|
| Ring (12件) | 確定 | maclookup.app確認済み |
| EZVIZ (13件) | 確定 | maclookup.app確認済み |
| Reolink (1件) | 確定 | maclookup.app確認済み |
| Amcrest (4件) | 確定 | maclookup.app確認済み |
| Arlo (3件) | 確定 | maclookup.app確認済み |
| I.O.DATA (3件) | 確定 | maclookup.app確認済み |
| SwitchBot (1件) | 確定 | Shenzhen Intellirocks確認 |
| Panasonic (10件) | **候補** | 多数あり、カメラ関連を選定要 |
| Anker/Eufy (1件) | **候補** | OEMで異なるOUI使用の可能性 |
| F.R.C | **調査中** | OUI未特定 |
| Anpviz | **調査中** | OEM使用の可能性 |

---

## Medium #6: 進捗計算動的算出（完全版）

### 6.1 スキャンステージ定義

```rust
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum ScanStage {
    /// ARP/Ping スキャン（ホスト発見）
    HostDiscovery,
    /// ポートスキャン（554, 80, 8080, 443）
    PortScan,
    /// OUI判定
    OuiLookup,
    /// ONVIF検出
    OnvifProbe,
    /// RTSP認証試行
    RtspAuth,
    /// 登録済みカメラ照合
    CameraMatching,
}
```

### 6.2 ステージ別重み定義

| ステージ | 重み | 根拠 |
|---------|------|------|
| HostDiscovery | 15% | ARP/Ping は高速（~1ms/host） |
| PortScan | 25% | TCP接続待機あり（~50ms/port/host） |
| OuiLookup | 5% | ローカルDB参照のみ |
| OnvifProbe | 20% | HTTPリクエスト（~500ms/host） |
| RtspAuth | 30% | 複数クレデンシャル試行（~1s/credential） |
| CameraMatching | 5% | DB照合のみ |

```rust
const STAGE_WEIGHTS: &[(ScanStage, u32)] = &[
    (ScanStage::HostDiscovery, 15),
    (ScanStage::PortScan, 25),
    (ScanStage::OuiLookup, 5),
    (ScanStage::OnvifProbe, 20),
    (ScanStage::RtspAuth, 30),
    (ScanStage::CameraMatching, 5),
];
// 合計: 100%
```

### 6.3 進捗計算器（完全版）

```rust
/// 動的進捗計算器
pub struct DynamicProgressCalculator {
    /// サブネット別ホスト数（理論値）
    subnet_host_counts: HashMap<String, u32>,
    /// ステージごとの対象ホスト数（実測）
    stage_actual_hosts: HashMap<ScanStage, u32>,
    /// ステージ別完了ホスト数
    stage_completed_hosts: HashMap<ScanStage, u32>,
    /// ARPバイパス追加数（登録済みカメラ）
    arp_bypass_count: u32,
    /// 現在アクティブなステージ
    current_stage: Option<ScanStage>,
}

impl DynamicProgressCalculator {
    /// 初期化（スキャン開始時）
    pub fn new(subnets: &[SubnetConfig], registered_cameras: &[Camera]) -> Self {
        let mut subnet_host_counts = HashMap::new();

        for subnet in subnets {
            if let Ok(network) = subnet.cidr.parse::<IpNetwork>() {
                // ネットワーク/ブロードキャストアドレスを除外
                let host_count = network.size().saturating_sub(2) as u32;
                subnet_host_counts.insert(subnet.cidr.clone(), host_count);
            }
        }

        Self {
            subnet_host_counts,
            stage_actual_hosts: HashMap::new(),
            stage_completed_hosts: HashMap::new(),
            arp_bypass_count: registered_cameras.len() as u32,
            current_stage: None,
        }
    }

    /// 理論上の総ホスト数
    fn theoretical_total_hosts(&self) -> u32 {
        self.subnet_host_counts.values().sum::<u32>() + self.arp_bypass_count
    }

    /// ステージ開始（対象ホスト数を記録）
    pub fn start_stage(&mut self, stage: ScanStage, target_hosts: u32) {
        self.current_stage = Some(stage);
        self.stage_actual_hosts.insert(stage, target_hosts);
        self.stage_completed_hosts.insert(stage, 0);
    }

    /// ホスト処理完了通知（1ホスト単位）
    pub fn complete_host(&mut self, stage: ScanStage) {
        if let Some(count) = self.stage_completed_hosts.get_mut(&stage) {
            *count += 1;
        }
    }

    /// ステージ完了
    pub fn finish_stage(&mut self, stage: ScanStage) {
        if let Some(actual) = self.stage_actual_hosts.get(&stage) {
            self.stage_completed_hosts.insert(stage, *actual);
        }
        self.current_stage = None;
    }

    /// 現在の進捗%を算出
    pub fn calculate_percent(&self) -> f32 {
        let mut total_progress = 0.0f32;

        for (stage, weight) in STAGE_WEIGHTS {
            let actual_hosts = self.stage_actual_hosts.get(stage).copied().unwrap_or(0);
            let completed_hosts = self.stage_completed_hosts.get(stage).copied().unwrap_or(0);

            if actual_hosts == 0 {
                // 未開始ステージはスキップ（重みはそのまま残す）
                continue;
            }

            let stage_percent = completed_hosts as f32 / actual_hosts as f32;
            total_progress += (*weight as f32) * stage_percent;
        }

        total_progress.clamp(0.0, 100.0)
    }

    /// 進捗イベント生成（フロントエンド通知用）
    pub fn to_progress_event(&self) -> ScanProgressEvent {
        ScanProgressEvent {
            percent: self.calculate_percent(),
            current_stage: self.current_stage,
            stage_details: STAGE_WEIGHTS
                .iter()
                .map(|(stage, weight)| StageProgress {
                    stage: *stage,
                    weight: *weight,
                    actual_hosts: self.stage_actual_hosts.get(stage).copied(),
                    completed_hosts: self.stage_completed_hosts.get(stage).copied().unwrap_or(0),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgressEvent {
    pub percent: f32,
    pub current_stage: Option<ScanStage>,
    pub stage_details: Vec<StageProgress>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StageProgress {
    pub stage: ScanStage,
    pub weight: u32,
    pub actual_hosts: Option<u32>,
    pub completed_hosts: u32,
}
```

### 6.4 消費更新タイミング

| イベント | 更新メソッド | タイミング |
|---------|------------|-----------|
| ステージ開始 | `start_stage(stage, n)` | 各ステージ開始直前 |
| ホスト処理完了 | `complete_host(stage)` | 各ホスト処理完了時 |
| ステージ完了 | `finish_stage(stage)` | 各ステージ完了時 |

### 6.5 実装例（Scanner内）

```rust
impl Scanner {
    async fn execute_scan(&self) -> Result<ScanResult> {
        let mut progress = DynamicProgressCalculator::new(&self.subnets, &self.cameras);

        // Stage 1: Host Discovery
        let hosts = self.discover_hosts().await?;
        progress.start_stage(ScanStage::HostDiscovery, self.theoretical_hosts());
        for host in &hosts {
            // ... discover logic ...
            progress.complete_host(ScanStage::HostDiscovery);
            self.emit_progress(progress.to_progress_event()).await;
        }
        progress.finish_stage(ScanStage::HostDiscovery);

        // Stage 2: Port Scan
        progress.start_stage(ScanStage::PortScan, hosts.len() as u32);
        for host in &hosts {
            // ... port scan logic ...
            progress.complete_host(ScanStage::PortScan);
            self.emit_progress(progress.to_progress_event()).await;
        }
        progress.finish_stage(ScanStage::PortScan);

        // ... 以下同様 ...
    }
}
```

---

## Medium #7: 強制登録デフォルト値検証

**既存実装との整合確認:**

| 項目 | 強制登録時デフォルト | 既存Tapo/VIGI | 整合 |
|-----|-------------------|--------------|------|
| rtsp_main | `rtsp://<ip>:554/stream1` | 認証付きURL | ✓ (認証なしパターン) |
| rtsp_sub | NULL | 認証付きURL | ✓ |
| polling_enabled | false | true | ✓ (接続失敗防止) |
| status | `pending_auth` | - | **要追加** |

**状態遷移への追加:**
```rust
pub enum CameraStatus {
    Active,        // 正常動作中
    Inactive,      // 無効化
    PendingAuth,   // 認証待ち（強制登録時）★新規
    Maintenance,   // メンテナンス中
}
```

### 7.1 PendingAuthステータス DBマイグレーション

#### 現行スキーマ確認

```sql
-- 現行: cameras.status の定義を確認
SHOW COLUMNS FROM cameras LIKE 'status';
-- 想定: ENUM('active', 'inactive', 'maintenance') または VARCHAR
```

#### マイグレーション方針

| 項目 | 方針 |
|-----|-----|
| ENUM拡張方式 | `ALTER TABLE ... MODIFY COLUMN` でENUM値追加 |
| デフォルト値 | 既存レコードは変更なし（`active`のまま） |
| NULL許容 | NOT NULL維持（デフォルト`active`） |
| 新規強制登録時 | `pending_auth`を明示的に設定 |

#### Step 1: ENUM値追加

```sql
-- cameras.status に 'pending_auth' を追加
-- MySQLのENUM追加は既存データに影響しない
ALTER TABLE cameras
MODIFY COLUMN status ENUM(
    'active',
    'inactive',
    'pending_auth',   -- ★追加
    'maintenance'
) NOT NULL DEFAULT 'active';
```

#### Step 2: バックフィル（不要）

既存レコードは現在のステータスを維持するため、バックフィルは**不要**。

```sql
-- 確認クエリ: 既存データの分布
SELECT status, COUNT(*) as count
FROM cameras
GROUP BY status;

-- 期待結果:
-- | status   | count |
-- |----------|-------|
-- | active   | 45    |
-- | inactive | 3     |
-- 'pending_auth' は新規強制登録時のみ使用
```

#### Step 3: アプリケーション側対応

```rust
// 強制登録API: PendingAuthステータスで登録
async fn force_register_camera(
    pool: &MySqlPool,
    device: &ScannedDevice,
) -> Result<Camera> {
    let camera = sqlx::query_as::<_, Camera>(r#"
        INSERT INTO cameras (
            ip_address, mac_address, name,
            rtsp_main_url, polling_enabled, status
        ) VALUES (?, ?, ?, ?, ?, ?)
    "#)
    .bind(&device.ip_address)
    .bind(&device.mac_address)
    .bind(format!("Camera_{}", device.ip_address))
    .bind(format!("rtsp://{}:554/stream1", device.ip_address))
    .bind(false)  // polling_enabled = false
    .bind("pending_auth")  // ★ PendingAuthステータス
    .fetch_one(pool)
    .await?;

    Ok(camera)
}

// ステータス更新: 認証成功時にActiveへ
async fn activate_camera(pool: &MySqlPool, camera_id: i64) -> Result<()> {
    sqlx::query(r#"
        UPDATE cameras
        SET status = 'active', polling_enabled = true
        WHERE id = ? AND status = 'pending_auth'
    "#)
    .bind(camera_id)
    .execute(pool)
    .await?;

    Ok(())
}
```

#### Step 4: ロールバック手順

```sql
-- ロールバック: pending_auth を削除
-- 注意: pending_auth のカメラがある場合は事前に inactive に変更

-- 1. pending_auth のカメラを inactive に変更
UPDATE cameras
SET status = 'inactive'
WHERE status = 'pending_auth';

-- 2. ENUM から pending_auth を削除
ALTER TABLE cameras
MODIFY COLUMN status ENUM(
    'active',
    'inactive',
    'maintenance'
) NOT NULL DEFAULT 'active';

-- 確認
SHOW COLUMNS FROM cameras LIKE 'status';
```

#### 検証クエリ

```sql
-- マイグレーション後の確認
-- 1. ENUM値が追加されていることを確認
SHOW COLUMNS FROM cameras LIKE 'status';
-- Expected: enum('active','inactive','pending_auth','maintenance')

-- 2. 既存データに影響がないことを確認
SELECT
    status,
    COUNT(*) as count,
    GROUP_CONCAT(name SEPARATOR ', ') as cameras
FROM cameras
GROUP BY status;

-- 3. 新規強制登録テスト後の確認
SELECT * FROM cameras WHERE status = 'pending_auth';
```

#### UI表示対応

```typescript
// ステータスバッジ表示
const StatusBadge: React.FC<{ status: CameraStatus }> = ({ status }) => {
  const config = {
    active: { label: '正常', color: 'green', icon: '✓' },
    inactive: { label: '無効', color: 'gray', icon: '−' },
    pending_auth: { label: '認証待ち', color: 'orange', icon: '🔐' },  // ★追加
    maintenance: { label: 'メンテナンス', color: 'blue', icon: '🔧' },
  };

  const { label, color, icon } = config[status];

  return (
    <span className={`status-badge status-${color}`}>
      {icon} {label}
    </span>
  );
};
```

---

## Low #8: テスト計画網羅化

**全設計ドキュメントに追加するテスト計画テンプレート:**

```markdown
## テスト計画

### バックエンドテスト
1. API単体テスト（Rust #[test]）
2. 統合テスト（データベース込み）
3. エラーケーステスト

### フロントエンドテスト
1. コンポーネント単体テスト（Jest/React Testing Library）
2. スナップショットテスト
3. E2Eテスト（Playwright）

### Chrome実UIテスト
1. 設定モーダルからの操作確認
2. レスポンシブ表示確認
3. 既存デザインとの整合確認
4. ブラウザ互換性確認（Chrome最新版）
```

---

# Part 2: OUI + RTSPパス SSoT統合設計

## 1. 概要

### 1.1 目的
OUI情報、カメラブランド、RTSPパステンプレートをSSoT化し、以下を実現：
- データベース一元管理
- 設定モーダルからの閲覧・編集
- ユーザーによる新規登録
- 汎用フォールバックパス

### 1.2 現状の問題
- OUIデータが`oui_data.rs`にハードコード
- RTSPテンプレートが`types.rs`の`RtspTemplate`にハードコード
- ユーザーが独自カメラを追加できない
- 新ベンダー対応にコード変更が必要

---

## 2. データベース設計

### 2.1 ER図

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  camera_brands  │     │   oui_entries   │     │ rtsp_templates  │
├─────────────────┤     ├─────────────────┤     ├─────────────────┤
│ id (PK)         │◀───┤ brand_id (FK)   │     │ id (PK)         │
│ name            │     │ oui_prefix (PK) │     │ brand_id (FK)   │◀┐
│ display_name    │     │ description     │     │ name            │ │
│ category        │     │ score_bonus     │     │ main_path       │ │
│ is_builtin      │     │ is_builtin      │     │ sub_path        │ │
│ created_at      │     │ created_at      │     │ default_port    │ │
│ updated_at      │     │ updated_at      │     │ is_default      │ │
└─────────────────┘     └─────────────────┘     │ priority        │ │
                                                │ is_builtin      │ │
                                                │ created_at      │ │
                                                │ updated_at      │ │
                                                └─────────────────┘ │
                                                         │          │
┌─────────────────────────────────────────────────────────┘          │
│                                                                    │
│  ┌────────────────────┐                                           │
│  │ generic_rtsp_paths │                                           │
│  ├────────────────────┤                                           │
└─▶│ id (PK)            │                                           │
   │ main_path          │                                           │
   │ sub_path           │                                           │
   │ description        │                                           │
   │ priority           │  (低いほど優先)                            │
   │ is_enabled         │                                           │
   └────────────────────┘                                           │
                                                                    │
   ┌────────────────────┐                                           │
   │   cameras (既存)    │───────────────────────────────────────────┘
   ├────────────────────┤     rtsp_template_id で紐付け
   │ ...                │
   │ rtsp_template_id   │
   │ ...                │
   └────────────────────┘
```

### 2.2 テーブル定義

```sql
-- カメラブランドマスタ
CREATE TABLE camera_brands (
    id INT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(100) NOT NULL UNIQUE,          -- 'TP-LINK', 'Hikvision'
    display_name VARCHAR(100) NOT NULL,         -- 'TP-Link / Tapo'
    category ENUM('consumer', 'professional', 'enterprise', 'unknown') DEFAULT 'unknown',
    is_builtin BOOLEAN DEFAULT FALSE,           -- システム組み込み（削除不可）
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

-- OUIエントリ
CREATE TABLE oui_entries (
    oui_prefix VARCHAR(8) NOT NULL,             -- 'XX:XX:XX' 形式
    brand_id INT NOT NULL,
    description VARCHAR(255),                   -- 'Tapo C310用'
    score_bonus INT DEFAULT 20,                 -- スコア加算値
    status ENUM('confirmed', 'candidate', 'investigating') DEFAULT 'confirmed',  -- ★追加: OUI検証ステータス
    verification_source VARCHAR(255),           -- ★追加: 検証ソース（'maclookup.app', 'IEEE OUI', '実機確認'等）
    is_builtin BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (oui_prefix),
    FOREIGN KEY (brand_id) REFERENCES camera_brands(id) ON DELETE CASCADE
);

-- OUIステータス定義:
--   'confirmed'    : IEEE OUI/maclookup.app等で確認済み。即座にスキャンで使用
--   'candidate'    : 実機確認待ち。スキャン対象だが信頼度低として扱う
--   'investigating': 調査中。スキャン対象外（手動でconfirmedへ昇格が必要）

-- RTSPテンプレート
CREATE TABLE rtsp_templates (
    id INT PRIMARY KEY AUTO_INCREMENT,
    brand_id INT NOT NULL,
    name VARCHAR(100) NOT NULL,                 -- 'Tapo標準', 'Hikvision標準'
    main_path VARCHAR(255) NOT NULL,            -- '/stream1'
    sub_path VARCHAR(255),                      -- '/stream2'
    default_port INT DEFAULT 554,
    is_default BOOLEAN DEFAULT FALSE,           -- このブランドのデフォルト
    priority INT DEFAULT 100,                   -- 低いほど優先
    notes TEXT,                                 -- メモ/備考
    is_builtin BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (brand_id) REFERENCES camera_brands(id) ON DELETE CASCADE,
    UNIQUE KEY (brand_id, name)
);

-- 汎用RTSPパス（ブランド不明時のフォールバック）
CREATE TABLE generic_rtsp_paths (
    id INT PRIMARY KEY AUTO_INCREMENT,
    main_path VARCHAR(255) NOT NULL,
    sub_path VARCHAR(255),
    description VARCHAR(255),                   -- '一般的なストリームパス'
    priority INT DEFAULT 100,                   -- 低いほど優先（試行順序）
    is_enabled BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

-- 初期データ: 汎用パス
INSERT INTO generic_rtsp_paths (main_path, sub_path, description, priority) VALUES
('/stream1', '/stream2', '一般的なストリームパス', 10),
('/live', '/live', 'ライブストリーム', 20),
('/h264_stream', '/h264_stream', 'H.264ストリーム', 30),
('/video1', '/video2', 'ビデオストリーム', 40),
('/cam/realmonitor?channel=1&subtype=0', '/cam/realmonitor?channel=1&subtype=1', 'Dahua互換', 50),
('/Streaming/Channels/101', '/Streaming/Channels/102', 'Hikvision互換', 60);
```

### 2.3 マイグレーション（既存データ移行）

```sql
-- 既存ハードコードデータをDBに移行
-- camera_brands
INSERT INTO camera_brands (name, display_name, category, is_builtin) VALUES
('TP-LINK', 'TP-Link / Tapo', 'consumer', TRUE),
('Google', 'Google / Nest', 'consumer', TRUE),
('Hikvision', 'Hikvision', 'professional', TRUE),
('Dahua', 'Dahua', 'professional', TRUE),
('Axis', 'Axis', 'enterprise', TRUE),
('Ring', 'Ring', 'consumer', TRUE),
('EZVIZ', 'EZVIZ', 'consumer', TRUE),
('Reolink', 'Reolink', 'consumer', TRUE),
('Amcrest', 'Amcrest', 'consumer', TRUE),
('Arlo', 'Arlo', 'consumer', TRUE),
('I-O-DATA', 'I.O.DATA', 'consumer', TRUE),
('SwitchBot', 'SwitchBot', 'consumer', TRUE),
('Panasonic', 'Panasonic / i-PRO', 'professional', TRUE);

-- oui_entries (既存oui_data.rsから移行)
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, is_builtin)
SELECT '70:5A:0F', id, NULL, 20, TRUE FROM camera_brands WHERE name = 'TP-LINK'
UNION ALL
SELECT '54:AF:97', id, NULL, 20, TRUE FROM camera_brands WHERE name = 'TP-LINK'
-- ... (既存23件 + 新規48件)
;

-- rtsp_templates
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, is_builtin)
SELECT id, 'Tapo/VIGI標準', '/stream1', '/stream2', 554, TRUE, TRUE
FROM camera_brands WHERE name = 'TP-LINK'
UNION ALL
SELECT id, 'Hikvision標準', '/Streaming/Channels/101', '/Streaming/Channels/102', 554, TRUE, TRUE
FROM camera_brands WHERE name = 'Hikvision'
-- ... (各ブランド)
;
```

### 2.4 既存camerasテーブルマイグレーション手順

#### 問題
既存の`cameras`テーブルには`rtsp_main_url`, `rtsp_sub_url`が直接格納されている。新しい`rtsp_templates`システムへの移行が必要。

#### マイグレーション戦略

1. **カラム追加（下位互換維持）**
2. **バックフィル（既存データ変換）**
3. **検証期間（並行運用）**
4. **旧カラム削除（オプション）**

#### Step 1: カラム追加

```sql
-- cameras テーブルに rtsp_template_id を追加（NULL許可で下位互換維持）
ALTER TABLE cameras
ADD COLUMN rtsp_template_id INT DEFAULT NULL,
ADD COLUMN rtsp_custom_main_path VARCHAR(255) DEFAULT NULL,  -- テンプレート上書き用
ADD COLUMN rtsp_custom_sub_path VARCHAR(255) DEFAULT NULL,
ADD CONSTRAINT fk_cameras_rtsp_template
    FOREIGN KEY (rtsp_template_id) REFERENCES rtsp_templates(id)
    ON DELETE SET NULL;
```

#### Step 2: バックフィルSQL

```sql
-- 既存カメラのRTSPパスからテンプレートを推定してマッピング
-- パターン1: Tapo/VIGI形式 (/stream1, /stream2)
UPDATE cameras c
JOIN camera_brands b ON b.name = 'TP-LINK'
JOIN rtsp_templates t ON t.brand_id = b.id AND t.is_default = TRUE
SET c.rtsp_template_id = t.id
WHERE c.rtsp_main_url LIKE '%/stream1%'
  AND c.rtsp_template_id IS NULL;

-- パターン2: Hikvision形式 (/Streaming/Channels/)
UPDATE cameras c
JOIN camera_brands b ON b.name = 'Hikvision'
JOIN rtsp_templates t ON t.brand_id = b.id AND t.is_default = TRUE
SET c.rtsp_template_id = t.id
WHERE c.rtsp_main_url LIKE '%/Streaming/Channels/%'
  AND c.rtsp_template_id IS NULL;

-- パターン3: Dahua形式 (/cam/realmonitor)
UPDATE cameras c
JOIN camera_brands b ON b.name = 'Dahua'
JOIN rtsp_templates t ON t.brand_id = b.id AND t.is_default = TRUE
SET c.rtsp_template_id = t.id
WHERE c.rtsp_main_url LIKE '%/cam/realmonitor%'
  AND c.rtsp_template_id IS NULL;

-- パターン4: マッチしなかった場合はカスタムパスを抽出
UPDATE cameras c
SET c.rtsp_custom_main_path = REGEXP_SUBSTR(c.rtsp_main_url, '/[^?]+'),
    c.rtsp_custom_sub_path = REGEXP_SUBSTR(c.rtsp_sub_url, '/[^?]+')
WHERE c.rtsp_template_id IS NULL
  AND c.rtsp_main_url IS NOT NULL;
```

#### Step 3: バックフィル検証クエリ

```sql
-- マイグレーション結果確認
SELECT
    CASE
        WHEN rtsp_template_id IS NOT NULL THEN 'テンプレート適用'
        WHEN rtsp_custom_main_path IS NOT NULL THEN 'カスタムパス'
        ELSE '未マイグレーション'
    END AS migration_status,
    COUNT(*) AS camera_count
FROM cameras
GROUP BY migration_status;

-- テンプレート別カウント
SELECT
    b.display_name AS brand,
    t.name AS template_name,
    COUNT(c.id) AS camera_count
FROM cameras c
JOIN rtsp_templates t ON c.rtsp_template_id = t.id
JOIN camera_brands b ON t.brand_id = b.id
GROUP BY b.display_name, t.name
ORDER BY camera_count DESC;
```

#### Step 4: ロールバック手順

```sql
-- ロールバック: 追加カラムを削除（必要に応じて）
ALTER TABLE cameras
DROP CONSTRAINT fk_cameras_rtsp_template,
DROP COLUMN rtsp_template_id,
DROP COLUMN rtsp_custom_main_path,
DROP COLUMN rtsp_custom_sub_path;

-- 注意: 旧rtsp_main_url, rtsp_sub_urlは維持されているため、
-- ロールバック後も既存機能は動作する
```

#### RTSP URL生成ロジック変更

```rust
impl Camera {
    /// RTSP URLを生成（新旧両対応）
    pub fn get_rtsp_url(&self, brand_service: &CameraBrandService) -> (String, String) {
        // 新方式: テンプレート使用
        if let Some(template_id) = self.rtsp_template_id {
            if let Some(template) = brand_service.get_template_by_id(template_id) {
                let main_path = self.rtsp_custom_main_path.as_ref()
                    .unwrap_or(&template.main_path);
                let sub_path = self.rtsp_custom_sub_path.as_ref()
                    .or(template.sub_path.as_ref());

                return (
                    format!("rtsp://{}:{}{}@{}:{}{}",
                        self.username, self.password, self.ip_address,
                        template.default_port, main_path),
                    sub_path.map(|p| format!("rtsp://{}:{}@{}:{}{}",
                        self.username, self.password, self.ip_address,
                        template.default_port, p)),
                );
            }
        }

        // 旧方式: 直接URL使用（フォールバック）
        (
            self.rtsp_main_url.clone().unwrap_or_default(),
            self.rtsp_sub_url.clone(),
        )
    }
}
```

---

## 3. バックエンド実装

### 3.1 新規サービス: CameraBrandService

```rust
/// カメラブランド・OUI・RTSPテンプレート管理サービス
pub struct CameraBrandService {
    pool: MySqlPool,
    /// キャッシュ（起動時ロード、変更時リフレッシュ）
    cache: Arc<RwLock<BrandCache>>,
}

struct BrandCache {
    /// OUIプレフィックス → ブランド情報
    oui_map: HashMap<String, OuiBrandInfo>,
    /// ブランドID → RTSPテンプレート
    templates: HashMap<i64, Vec<RtspTemplateInfo>>,
    /// 汎用パス（優先度順）
    generic_paths: Vec<GenericRtspPath>,
    /// 最終更新
    last_updated: DateTime<Utc>,
}

impl CameraBrandService {
    /// OUIからブランド情報を取得
    pub async fn lookup_oui(&self, mac: &str) -> Option<OuiBrandInfo>;

    /// ブランドのRTSPテンプレートを取得
    pub async fn get_rtsp_template(&self, brand_id: i64) -> Option<RtspTemplateInfo>;

    /// 汎用パスを優先度順に取得
    pub async fn get_generic_paths(&self) -> Vec<GenericRtspPath>;

    /// RTSP URLを生成（ブランド判明時）
    pub fn generate_rtsp_url(
        &self,
        template: &RtspTemplateInfo,
        ip: &str,
        port: Option<u16>,
        username: &str,
        password: &str,
    ) -> (String, String);

    /// RTSP URLを生成（ブランド不明時 - 汎用パス使用）
    pub fn generate_generic_rtsp_url(
        &self,
        path: &GenericRtspPath,
        ip: &str,
        port: Option<u16>,
        username: &str,
        password: &str,
    ) -> (String, String);

    // === 管理API ===

    /// 全ブランド一覧取得
    pub async fn list_brands(&self) -> Result<Vec<CameraBrand>>;

    /// ブランド新規作成
    pub async fn create_brand(&self, req: CreateBrandRequest) -> Result<CameraBrand>;

    /// ブランド更新
    pub async fn update_brand(&self, id: i64, req: UpdateBrandRequest) -> Result<CameraBrand>;

    /// ブランド削除（is_builtin=falseのみ）
    pub async fn delete_brand(&self, id: i64) -> Result<()>;

    /// OUIエントリ追加
    pub async fn add_oui_entry(&self, req: AddOuiRequest) -> Result<OuiEntry>;

    /// OUIエントリ削除
    pub async fn delete_oui_entry(&self, oui_prefix: &str) -> Result<()>;

    /// RTSPテンプレート追加
    pub async fn add_rtsp_template(&self, req: AddTemplateRequest) -> Result<RtspTemplate>;

    /// RTSPテンプレート更新
    pub async fn update_rtsp_template(&self, id: i64, req: UpdateTemplateRequest) -> Result<RtspTemplate>;

    /// RTSPテンプレート削除
    pub async fn delete_rtsp_template(&self, id: i64) -> Result<()>;

    /// 汎用パス追加
    pub async fn add_generic_path(&self, req: AddGenericPathRequest) -> Result<GenericRtspPath>;

    /// キャッシュリフレッシュ
    pub async fn refresh_cache(&self) -> Result<()>;
}
```

### 3.2 API設計

```
# カメラブランド
GET    /api/settings/camera-brands              # 一覧取得
POST   /api/settings/camera-brands              # 新規作成
PUT    /api/settings/camera-brands/{id}         # 更新
DELETE /api/settings/camera-brands/{id}         # 削除

# OUIエントリ
GET    /api/settings/camera-brands/{id}/oui     # ブランドのOUI一覧
POST   /api/settings/camera-brands/{id}/oui     # OUI追加
DELETE /api/settings/oui/{prefix}               # OUI削除

# RTSPテンプレート
GET    /api/settings/camera-brands/{id}/rtsp-templates   # ブランドのテンプレート一覧
POST   /api/settings/camera-brands/{id}/rtsp-templates   # テンプレート追加
PUT    /api/settings/rtsp-templates/{id}                 # テンプレート更新
DELETE /api/settings/rtsp-templates/{id}                 # テンプレート削除

# 汎用パス
GET    /api/settings/generic-rtsp-paths         # 一覧取得
POST   /api/settings/generic-rtsp-paths         # 追加
PUT    /api/settings/generic-rtsp-paths/{id}    # 更新
DELETE /api/settings/generic-rtsp-paths/{id}    # 削除
```

### 3.3 API認可・エラーハンドリング（追加）

#### 認可ポリシー

| 操作 | 必要権限 | is_builtin=true時 |
|-----|---------|------------------|
| 一覧取得 (GET) | 閲覧権限 | 許可 |
| 新規作成 (POST) | 管理者権限 | 許可（新規はis_builtin=false） |
| 更新 (PUT) | 管理者権限 | **禁止**（エラー返却） |
| 削除 (DELETE) | 管理者権限 | **禁止**（エラー返却） |

#### is_builtin制約のエラーレスポンス

```rust
/// 組込リソース編集エラー
#[derive(Debug, Serialize)]
pub struct BuiltinResourceError {
    pub error: String,
    pub code: String,
    pub resource_type: String,
    pub resource_id: String,
}

impl BuiltinResourceError {
    pub fn new(resource_type: &str, resource_id: &str) -> Self {
        Self {
            error: format!(
                "{}「{}」はシステム組込のため編集・削除できません",
                resource_type, resource_id
            ),
            code: "BUILTIN_RESOURCE_READONLY".to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
        }
    }
}
```

#### HTTPステータスコード定義

| ケース | HTTPステータス | エラーコード | メッセージ例 |
|-------|--------------|-------------|-------------|
| is_builtin更新試行 | 403 Forbidden | BUILTIN_RESOURCE_READONLY | "ブランド「TP-LINK」はシステム組込のため編集できません" |
| is_builtin削除試行 | 403 Forbidden | BUILTIN_RESOURCE_READONLY | "OUI「70:5A:0F」はシステム組込のため削除できません" |
| 存在しないリソース | 404 Not Found | RESOURCE_NOT_FOUND | "ブランドID 999 は存在しません" |
| 外部キー制約違反 | 409 Conflict | FK_CONSTRAINT_VIOLATION | "ブランドにOUIまたはテンプレートが紐付いているため削除できません" |
| バリデーションエラー | 400 Bad Request | VALIDATION_ERROR | "OUIプレフィックスは XX:XX:XX 形式で指定してください" |
| 認証エラー | 401 Unauthorized | AUTHENTICATION_REQUIRED | "認証が必要です" |
| 権限不足 | 403 Forbidden | INSUFFICIENT_PERMISSIONS | "管理者権限が必要です" |

#### 実装例

```rust
async fn update_brand(
    State(state): State<AppState>,
    Path(brand_id): Path<i64>,
    Json(req): Json<UpdateBrandRequest>,
) -> Result<Json<CameraBrand>, AppError> {
    // 1. リソース存在確認
    let brand = state.brand_service
        .get_brand(brand_id)
        .await?
        .ok_or_else(|| AppError::not_found(
            "RESOURCE_NOT_FOUND",
            format!("ブランドID {} は存在しません", brand_id)
        ))?;

    // 2. is_builtin制約チェック
    if brand.is_builtin {
        return Err(AppError::forbidden(
            "BUILTIN_RESOURCE_READONLY",
            format!("ブランド「{}」はシステム組込のため編集できません", brand.display_name)
        ));
    }

    // 3. バリデーション
    req.validate()?;

    // 4. 更新実行
    let updated = state.brand_service
        .update_brand(brand_id, req)
        .await?;

    Ok(Json(updated))
}

async fn delete_oui_entry(
    State(state): State<AppState>,
    Path(oui_prefix): Path<String>,
) -> Result<StatusCode, AppError> {
    // 1. リソース存在確認
    let entry = state.brand_service
        .get_oui_entry(&oui_prefix)
        .await?
        .ok_or_else(|| AppError::not_found(
            "RESOURCE_NOT_FOUND",
            format!("OUI「{}」は存在しません", oui_prefix)
        ))?;

    // 2. is_builtin制約チェック
    if entry.is_builtin {
        return Err(AppError::forbidden(
            "BUILTIN_RESOURCE_READONLY",
            format!("OUI「{}」はシステム組込のため削除できません", oui_prefix)
        ));
    }

    // 3. 削除実行
    state.brand_service.delete_oui_entry(&oui_prefix).await?;

    Ok(StatusCode::NO_CONTENT)
}
```

#### フロントエンドでのエラー表示

```typescript
// API呼び出し時のエラーハンドリング
async function updateBrand(id: number, data: UpdateBrandRequest) {
  try {
    await api.put(`/api/settings/camera-brands/${id}`, data);
    showSuccess('ブランドを更新しました');
  } catch (error) {
    if (error.response?.status === 403) {
      if (error.response.data.code === 'BUILTIN_RESOURCE_READONLY') {
        showError('システム組込のブランドは編集できません');
      } else {
        showError('権限がありません');
      }
    } else if (error.response?.status === 404) {
      showError('ブランドが見つかりません');
    } else {
      showError('更新に失敗しました');
    }
  }
}

// UI上での編集制限表示
const BrandCard: React.FC<{ brand: CameraBrand }> = ({ brand }) => (
  <div className={`brand-card ${brand.isBuiltin ? 'readonly' : ''}`}>
    <h4>
      {brand.isBuiltin && <span className="lock-icon">🔒</span>}
      {brand.displayName}
    </h4>
    {brand.isBuiltin && (
      <span className="readonly-badge">システム組込</span>
    )}
    {!brand.isBuiltin && (
      <div className="actions">
        <button onClick={() => editBrand(brand)}>編集</button>
        <button onClick={() => deleteBrand(brand)}>削除</button>
      </div>
    )}
  </div>
);
```

---

## 4. フロントエンド実装

### 4.1 設定モーダル拡張

```
設定モーダル
├─ タイムアウト設定 (既存)
├─ クレデンシャル設定 (既存)
└─ カメラブランド設定 (新規)
   ├─ ブランド一覧
   │   ├─ [TP-Link / Tapo] (システム組込)
   │   │   ├─ OUI一覧 (70:5A:0F, 54:AF:97, ...)
   │   │   └─ RTSPテンプレート
   │   │       ├─ Tapo標準: /stream1, /stream2
   │   │       └─ [+ テンプレート追加]
   │   ├─ [Hikvision] (システム組込)
   │   │   └─ ...
   │   └─ [+ ブランド追加]
   │
   └─ 汎用RTSPパス
       ├─ /stream1, /stream2 (優先度10)
       ├─ /live (優先度20)
       └─ [+ パス追加]
```

### 4.2 UIコンポーネント

```typescript
// ブランド一覧表示
const CameraBrandSettings: React.FC = () => {
    const [brands, setBrands] = useState<CameraBrand[]>([]);
    const [selectedBrand, setSelectedBrand] = useState<CameraBrand | null>(null);

    return (
        <div className="camera-brand-settings">
            <h3>📷 カメラブランド設定</h3>

            {/* ブランド一覧 */}
            <div className="brand-list">
                {brands.map(brand => (
                    <BrandCard
                        key={brand.id}
                        brand={brand}
                        onSelect={() => setSelectedBrand(brand)}
                        isBuiltin={brand.isBuiltin}
                    />
                ))}
                <AddBrandButton onClick={openAddBrandModal} />
            </div>

            {/* 選択中ブランドの詳細 */}
            {selectedBrand && (
                <BrandDetail
                    brand={selectedBrand}
                    onOuiAdd={handleOuiAdd}
                    onTemplateAdd={handleTemplateAdd}
                    onTemplateEdit={handleTemplateEdit}
                />
            )}

            {/* 汎用パス */}
            <GenericPathsSection />
        </div>
    );
};

// ブランド追加モーダル
const AddBrandModal: React.FC = () => {
    return (
        <Modal title="カメラブランドを追加">
            <Form>
                <Input label="ブランド名" name="name" required />
                <Input label="表示名" name="displayName" required />
                <Select
                    label="カテゴリ"
                    name="category"
                    options={[
                        { value: 'consumer', label: '家庭用' },
                        { value: 'professional', label: '業務用' },
                        { value: 'enterprise', label: 'エンタープライズ' },
                    ]}
                />

                <h4>OUIプレフィックス</h4>
                <OuiInputList name="ouiPrefixes" />

                <h4>RTSPテンプレート</h4>
                <Input label="メインストリームパス" name="mainPath" placeholder="/stream1" />
                <Input label="サブストリームパス" name="subPath" placeholder="/stream2" />
                <Input label="デフォルトポート" name="defaultPort" type="number" defaultValue={554} />
            </Form>
        </Modal>
    );
};
```

### 4.3 表示例

```
┌─────────────────────────────────────────────────────────────────┐
│ 設定                                                    [✕]    │
├─────────────────────────────────────────────────────────────────┤
│ ┌─────────┬─────────────────────────┬───────────────────────┐  │
│ │タイムアウト│ クレデンシャル           │ カメラブランド▼        │  │
│ └─────────┴─────────────────────────┴───────────────────────┘  │
│                                                                 │
│ 📷 カメラブランド設定                                           │
│ ─────────────────────────────────────────────────────────────  │
│                                                                 │
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐   │
│ │ 🔒 TP-Link/Tapo │ │ 🔒 Hikvision    │ │ 🔒 Dahua        │   │
│ │ OUI: 13件       │ │ OUI: 3件        │ │ OUI: 2件        │   │
│ │ テンプレート: 1  │ │ テンプレート: 1  │ │ テンプレート: 1  │   │
│ └─────────────────┘ └─────────────────┘ └─────────────────┘   │
│                                                                 │
│ ┌─────────────────┐ ┌─────────────────┐                       │
│ │ ✏️ カスタム1     │ │ ➕ ブランド追加  │                       │
│ │ OUI: 2件        │ │                  │                       │
│ │ テンプレート: 1  │ │                  │                       │
│ └─────────────────┘ └─────────────────┘                       │
│                                                                 │
│ 📋 汎用RTSPパス（ブランド不明時に順番に試行）                    │
│ ─────────────────────────────────────────────────────────────  │
│ 優先度   メインパス                    サブパス                  │
│ 10       /stream1                     /stream2        [編集][削除]│
│ 20       /live                        /live           [編集][削除]│
│ 30       /h264_stream                 /h264_stream    [編集][削除]│
│                                                 [➕ パス追加]    │
│                                                                 │
│                                            [キャンセル] [保存]   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. スキャン時のRTSPパス解決フロー

```
スキャンでデバイス発見
    │
    ▼
MACアドレスからOUIを抽出
    │
    ▼
oui_entries検索
    │
    ├── OUI一致 → brand_id取得
    │       │
    │       ▼
    │   rtsp_templates検索 (is_default=true or priority順)
    │       │
    │       ▼
    │   ブランド専用パスでRTSP URL生成
    │
    └── OUI不一致
            │
            ▼
        generic_rtsp_paths取得 (priority順)
            │
            ▼
        汎用パスを順番に試行
            │
            ├── 応答あり → そのパスを使用
            │
            └── 全て失敗 → RTSP利用不可として記録
```

---

## 6. テスト計画

### バックエンドテスト
1. CameraBrandService CRUD操作テスト
2. OUI検索パフォーマンステスト（1000件OUI時）
3. キャッシュリフレッシュテスト
4. is_builtin制約テスト（削除拒否）

### フロントエンドテスト
1. ブランド一覧表示テスト
2. ブランド追加フォームバリデーションテスト
3. OUI入力形式バリデーション（XX:XX:XX）
4. 組込ブランドの編集制限テスト

### Chrome実UIテスト
1. 設定モーダルからのブランド管理操作
2. 新規ブランド追加→スキャン→認識確認
3. 汎用パス順序変更→スキャン確認

---

## 7. MECE確認

- [x] OUI、ブランド、RTSPテンプレート、汎用パスが明確に分離
- [x] 組込/ユーザー定義が`is_builtin`フラグで区別
- [x] ブランド不明時のフォールバックが汎用パスで定義
- [x] 全APIがCRUD完備
- [x] キャッシュ機構でパフォーマンス確保

---

## 8. 承認・実装フロー

1. [ ] 本設計レビュー
2. [ ] GitHub Issue登録（レビュー修正8件 + SSoT統合1件）
3. [ ] データベースマイグレーション作成
4. [ ] CameraBrandService実装
5. [ ] API実装
6. [ ] フロントエンド実装
7. [ ] 既存oui_data.rs/RtspTemplate削除
8. [ ] テスト実行
9. [ ] 完了報告

---

**作成日**: 2026-01-07
**更新日**: 2026-01-07
**作成者**: Claude Code
**ステータス**: 設計完了・実装開始可能

---

## 更新履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-07 | 初版作成 |
| 2026-01-07 | レビュー指摘対応（6件） |
| 2026-01-07 | 追加レビュー指摘対応（2件）：IPv4方針明記、PendingAuthマイグレーション |

### レビュー指摘対応詳細

| 優先度 | 指摘 | 対応セクション |
|-------|------|--------------|
| High | 進捗計算DynamicProgressCalculator未完成 | Medium #6: 6.1-6.5で完全定義 |
| High | oui_entriesにstatus列未反映 | 2.2テーブル定義に追加 |
| High | カテゴリF (LostConnection) 流し込み未定義 | Medium #3: 3.2で3段階フロー定義 |
| High | 既存cameras RTSPマイグレーション未記載 | 2.4で4ステップ手順定義 |
| Medium | クレデンシャル監査ポリシー未定義 | High #1: 1.2で完全定義 |
| Low | API認可・エラーハンドリング不足 | 3.3で完全定義 |

### 追加レビュー対応

| 指摘 | 対応セクション |
|------|--------------|
| サブネット削除クエリIPv4固定問題 | Medium #4: 4.1でIPv4専用方針を明記、将来IPv6拡張ポイント記載 |
| PendingAuthステータスDBマイグレーション未記載 | Medium #7: 7.1でENUM追加・ロールバック・検証手順を完全定義 |
