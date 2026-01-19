# IS22 カメラ登録フロー詳細解析

## 1. 概要

本ドキュメントはIS22システムのカメラ登録フローを詳細に解析し、新規ブランド（NVT/JOOAN等）を追加する際の課題と対応策を明確化する。

---

## 2. 現行アーキテクチャ

### 2.1 主要コンポーネント

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Frontend      │────▶│    Web API      │────▶│    Services     │
│  (React/TS)     │     │   (routes.rs)   │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                        │
                        ┌───────────────────────────────┼───────────────────────────────┐
                        │                               │                               │
                ┌───────▼───────┐               ┌───────▼───────┐               ┌───────▼───────┐
                │ IpcamScan     │               │ CameraBrand   │               │ CameraRegistry│
                │ (scanner/)    │               │ Service       │               │ Service       │
                └───────────────┘               └───────────────┘               └───────────────┘
                        │                               │                               │
                        ▼                               ▼                               ▼
                ┌───────────────┐               ┌───────────────┐               ┌───────────────┐
                │ ARP Scanner   │               │ DB (camera_   │               │ araneaDevice  │
                │ Port Scanner  │               │ brands, oui,  │               │ Gate登録      │
                │ ONVIF/RTSP    │               │ rtsp_templates│               │               │
                └───────────────┘               └───────────────┘               └───────────────┘
```

### 2.2 データモデル

#### CameraFamily enum（ハードコード）
ファイル: `is22/src/ipcam_scan/types.rs:29-41`

```rust
pub enum CameraFamily {
    Tapo,
    Vigi,
    Nest,
    Axis,
    Hikvision,
    Dahua,
    Other,
    Unknown,
}
```

**問題点**: NVT/JOOANが含まれていない

#### DB: camera_brands テーブル
ファイル: `migrations/015_oui_rtsp_ssot.sql`

```sql
CREATE TABLE camera_brands (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    display_name VARCHAR(100) NOT NULL,
    category ENUM('consumer', 'professional', 'enterprise', 'unknown'),
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
    ...
);
```

初期データ（組み込み）:
- TP-LINK, Google, Hikvision, Dahua, Axis, Ring, EZVIZ, Reolink, Amcrest, Arlo, I-O-DATA, SwitchBot, Panasonic

**問題点**: NVTブランドが未登録

---

## 3. カメラ登録フロー詳細

### 3.1 Phase 1: 発見（Discovery）

```
[ARP Scan] → [Port Scan] → [OUI Lookup]
```

#### ARPスキャン
ファイル: `is22/src/lost_cam_tracker/arp_scanner.rs`

- `arp-scan`コマンドでL2スキャン
- フォールバック: `ip neigh show`（システムARPキャッシュ）
- sudo権限必要

#### ポートスキャン
対象ポート:
- 554 (RTSP標準)
- 80/8080/8899 (HTTP/ONVIF)
- 2020 (Tapo ONVIF)
- 5000/8000/8081/10080 (各社独自)

#### OUIルックアップ
ファイル: `is22/src/camera_brand/service.rs:95-99`

```rust
pub async fn lookup_oui(&self, mac_address: &str) -> Option<OuiBrandInfo> {
    let oui_prefix = Self::extract_oui_prefix(mac_address)?;
    let cache = self.cache.read().await;
    cache.oui_map.get(&oui_prefix).cloned()
}
```

### 3.2 Phase 2: 検証（Verification）

```
[ONVIF Probe] → [RTSP Probe] → [Credential Trial]
```

#### ONVIFプローブ
- GetDeviceInformation SOAP要求
- 認証不要/要認証の判別

#### RTSPプローブ
- OPTIONSメソッドで接続確認
- 認証応答確認

#### クレデンシャル試行
ファイル: `is22/src/ipcam_scan/types.rs:269-275`

```rust
pub struct TrialCredential {
    pub username: String,
    pub password: String,
    pub priority: u8,  // 1-10, 試行順序
}
```

### 3.3 Phase 3: 承認・登録（Approval）

```
[UI承認] → [RTSP URL生成] → [cameras登録] → [araneaDeviceGate登録]
```

#### RTSP URL生成
ファイル: `is22/src/ipcam_scan/types.rs:111-132`

```rust
pub fn generate_urls(
    &self,
    ip: &str,
    port: Option<u16>,
    username: &str,
    password: &str,
) -> (String, String) {
    let port = port.unwrap_or(self.default_port);
    let encoded_pass = password.replace("@", "%40");

    let main_url = format!(
        "rtsp://{}:{}@{}:{}{}",
        username, encoded_pass, ip, port, self.main_path
    );
    // ...
}
```

**問題点**: usernameが空の場合のハンドリングはservice.rsで実装されているが、types.rsのハードコード版では常にusername必須

---

## 4. 発見された課題

### 4.1 CameraFamily enumとDBの二重管理

| 場所 | 用途 | 問題 |
|------|------|------|
| `types.rs` CameraFamily | RtspTemplate生成 | ハードコード、拡張困難 |
| `camera_brands` テーブル | OUI/RTSPテンプレートSSoT | DB優先だが連携不完全 |

**解決策**: DB側を正とし、CameraFamilyは廃止または動的ロードに変更

### 4.2 NVT/JOOANブランド未対応

現状:
- `camera_brands`テーブルにNVT未登録
- `oui_entries`にNVT OUI未登録
- `rtsp_templates`にNVTパス未登録

必要な追加データ:
```sql
-- NVTブランド
INSERT INTO camera_brands (name, display_name, category, is_builtin) VALUES
('NVT', 'NVT / JOOAN', 'consumer', FALSE);

-- NVT OUI (DC:62:79 等)
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status)
SELECT 'DC:62:79', id, 'NVT/JOOAN camera', 20, 'confirmed'
FROM camera_brands WHERE name = 'NVT';

-- NVT RTSPテンプレート
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default)
SELECT id, 'NVT Standard', '/stream1', '/stream2', 554, TRUE
FROM camera_brands WHERE name = 'NVT';
```

### 4.3 クレデンシャル空ユーザー名問題

**NVT系の特徴**:
- ユーザー名なしでパスワードのみの場合がある
- または `admin`/`Admin`/`ADMIN` がデフォルト

**現状の動作**:
```rust
// camera_brand/service.rs:134-141
let main_url = if username.is_empty() {
    format!("rtsp://{}:{}{}", ip, actual_port, template.main_path)
} else {
    format!(
        "rtsp://{}:{}@{}:{}{}",
        username, password, ip, actual_port, template.main_path
    )
};
```

サービス層では空ユーザー名をサポートしているが、UIから空ユーザー名を送信できない可能性がある。

**解決策**:
1. UI側でユーザー名空を許可
2. または「デフォルトユーザー名を使用」チェックボックス追加
3. ブランドごとのデフォルトユーザー名をDBに追加

### 4.4 Access Absorber未実装

現在の実装には接続数制限機能がない。

参照: `camBrand/accessAbsorber/SPECIFICATION.md`

必要な機能:
- ブランド/モデルごとの同時接続数制限
- 再接続インターバル制御
- 排他ロック機構

---

## 5. 関連ファイル一覧

### Rust実装
| ファイル | 役割 |
|----------|------|
| `src/ipcam_scan/types.rs` | CameraFamily enum, RtspTemplate |
| `src/ipcam_scan/job.rs` | ScanJob, 進捗管理 |
| `src/camera_brand/service.rs` | ブランドサービス、OUIキャッシュ |
| `src/camera_brand/types.rs` | DB型定義 |
| `src/camera_registry/service.rs` | araneaDeviceGate登録 |
| `src/lost_cam_tracker/arp_scanner.rs` | ARPスキャン |
| `src/web_api/routes.rs` | APIエンドポイント |

### マイグレーション
| ファイル | 内容 |
|----------|------|
| `migrations/015_oui_rtsp_ssot.sql` | camera_brands, oui_entries, rtsp_templates |
| `migrations/016_oui_rtsp_backfill.sql` | 既存データバックフィル |
| `migrations/017_oui_expansion.sql` | OUI追加 |

### フロントエンド
| ファイル | 役割 |
|----------|------|
| `frontend/src/components/ScanModal.tsx` | スキャンUI |
| `frontend/src/components/SettingsModal.tsx` | 設定UI |

---

## 6. 次のアクション

参照: `NewBrandIntegration/IMPLEMENTATION_TASKS.md`
