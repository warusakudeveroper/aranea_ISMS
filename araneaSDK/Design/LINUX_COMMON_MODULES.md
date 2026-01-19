# Linux共通モジュール仕様書

> **Version**: 1.0.0
> **Last Updated**: 2026-01-20
> **Status**: Production
> **対象**: IS21/IS22およびLinux系araneaデバイス全般

---

## 1. Overview

### 1.1 目的

本仕様書は、Linux系araneaデバイスで共通利用するモジュール群の実装仕様を定義します。これらは**後続のLinuxデバイス開発でそのまま再利用可能**な共通コンポーネントです。

### 1.2 共通モジュール一覧

| モジュール | 責務 | 実装言語 |
|-----------|------|----------|
| AraneaRegister | araneaDeviceGate登録、LacisID/CIC管理 | Rust/Python |
| LacisOath | 認証ヘッダ生成・検証 | Rust/Python |
| ConfigStore | 設定永続化、SSoT管理 | Rust/Python |
| MACアドレス取得 | プライマリNIC MAC取得 | Rust/Python |

### 1.3 リファレンス実装

```
Rust版 (IS22): code/orangePi/is22/src/aranea_register/
Python版 (IS21): code/orangePi/is21/src/aranea_common/
ESP32版: code/ESP32/global/src/AraneaRegister.cpp
```

---

## 2. LacisID仕様

### 2.1 フォーマット

```
[Prefix][ProductType][MAC][ProductCode] = 20桁

位置: [0]   [1-3]       [4-15]    [16-19]
桁数:  1     3           12         4
```

| 要素 | 桁数 | 説明 |
|------|------|------|
| Prefix | 1 | `3` = araneaDevice, `4` = 特殊デバイス |
| ProductType | 3 | 製品種別コード（下記参照） |
| MAC | 12 | MACアドレス（コロンなし、大文字HEX） |
| ProductCode | 4 | 製品コード/追い番 |

### 2.2 ProductType一覧

| ProductType | デバイス | 説明 |
|-------------|----------|------|
| 021 | is21 | AI推論サーバー |
| 022 | is22 | RTSPカメラ管理サーバー |
| 023 | is23 | （予約） |
| 801 | is801 | ParaclateCamera（IS22管理下カメラ） |
| 802 | is802 | （予約） |

### 2.3 ProductCode定義

#### IS22デバイス自身
```
0001: 固定（追い番なし）
```

#### IS801 ParaclateCamera（カメラファミリー別）
```
0000: Generic（デフォルト）
0001: TP-Link Tapo
0002: Hikvision
0003: Dahua
0004: Reolink
0005: Axis
```

### 2.4 LacisID生成 - Rust実装

```rust
// types.rs
pub const PREFIX: &str = "3";
pub const PRODUCT_TYPE: &str = "022";  // デバイスごとに変更
pub const PRODUCT_CODE: &str = "0001";
pub const LACIS_ID_LENGTH: usize = 20;
pub const MAC_LENGTH: usize = 12;

// lacis_id.rs
/// MACアドレスからLacisIDを生成
pub fn try_generate_lacis_id(mac: &str) -> Result<String, String> {
    // MACアドレスからコロン/ハイフンを除去し大文字化
    let mac_clean: String = mac
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_uppercase();

    if mac_clean.len() != MAC_LENGTH {
        return Err(format!(
            "Invalid MAC address: expected {} hex digits, got {}",
            MAC_LENGTH,
            mac_clean.len()
        ));
    }

    Ok(format!(
        "{}{}{}{}",
        PREFIX, PRODUCT_TYPE, mac_clean, PRODUCT_CODE
    ))
}
```

### 2.5 LacisID生成 - Python実装

```python
PREFIX = "3"
PRODUCT_TYPE = "021"  # デバイスごとに変更
PRODUCT_CODE = "0001"

def generate_lacis_id(mac: str) -> str:
    """MACアドレスからLacisIDを生成"""
    # MACアドレスからコロン/ハイフンを除去し大文字化
    mac_clean = "".join(c for c in mac.upper() if c in "0123456789ABCDEF")

    if len(mac_clean) != 12:
        raise ValueError(f"Invalid MAC address: expected 12 hex digits, got {len(mac_clean)}")

    return f"{PREFIX}{PRODUCT_TYPE}{mac_clean}{PRODUCT_CODE}"
```

### 2.6 LacisIDバリデーション

```rust
/// mobes2.0側バリデーション準拠
/// 正規表現: ^[34]\d{3}[0-9A-F]{12}\d{4}$
pub fn validate_lacis_id(lacis_id: &str) -> bool {
    if lacis_id.len() != 20 {
        return false;
    }

    let chars: Vec<char> = lacis_id.chars().collect();

    // Prefix: 3 or 4
    if chars[0] != '3' && chars[0] != '4' {
        return false;
    }

    // ProductType: 3桁の数字
    if !chars[1..4].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // MAC: 12桁の16進数
    if !chars[4..16].iter().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }

    // ProductCode: 4桁の数字
    if !chars[16..20].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }

    true
}
```

---

## 3. MACアドレス取得

### 3.1 Linux環境でのMAC取得

```rust
/// システムのプライマリMACアドレスを取得
pub fn get_primary_mac_address() -> io::Result<String> {
    // 優先順位: eth0 > enp* > wlan0 > その他
    let interfaces = ["eth0", "enp0s3", "enp0s31f6", "wlan0"];

    for iface in &interfaces {
        let path = format!("/sys/class/net/{}/address", iface);
        if let Ok(mac) = std::fs::read_to_string(&path) {
            let mac = mac.trim().to_uppercase();
            if !mac.is_empty() && mac != "00:00:00:00:00:00" {
                return Ok(mac);
            }
        }
    }

    // フォールバック: ディレクトリスキャン
    let net_dir = std::fs::read_dir("/sys/class/net")?;
    for entry in net_dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // loopbackは除外
        if name_str == "lo" {
            continue;
        }

        let path = format!("/sys/class/net/{}/address", name_str);
        if let Ok(mac) = std::fs::read_to_string(&path) {
            let mac = mac.trim().to_uppercase();
            if !mac.is_empty() && mac != "00:00:00:00:00:00" {
                return Ok(mac);
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No valid network interface found",
    ))
}
```

### 3.2 Python版

```python
import os
import subprocess

def get_primary_mac_address() -> str:
    """プライマリMACアドレスを取得"""
    # 優先順位
    interfaces = ["eth0", "enp0s3", "enp0s31f6", "wlan0"]

    for iface in interfaces:
        path = f"/sys/class/net/{iface}/address"
        if os.path.exists(path):
            with open(path, "r") as f:
                mac = f.read().strip().upper()
                if mac and mac != "00:00:00:00:00:00":
                    return mac

    # フォールバック
    net_dir = "/sys/class/net"
    for iface in os.listdir(net_dir):
        if iface == "lo":
            continue
        path = f"{net_dir}/{iface}/address"
        if os.path.exists(path):
            with open(path, "r") as f:
                mac = f.read().strip().upper()
                if mac and mac != "00:00:00:00:00:00":
                    return mac

    raise RuntimeError("No valid network interface found")
```

---

## 4. LacisOath認証

### 4.1 認証ヘッダ形式

```
Authorization: LacisOath <base64-encoded-json>
```

### 4.2 JSONペイロード構造

```json
{
  "lacisId": "30221A2B3C4D5E6F0001",
  "tid": "T2025120621041161827",
  "cic": "204965",
  "timestamp": "2026-01-20T12:00:00Z"
}
```

| フィールド | 型 | 必須 | 説明 |
|-----------|------|------|------|
| lacisId | string | ✓ | 送信元デバイスのLacisID |
| tid | string | ✓ | テナントID |
| cic | string | ✓ | Client Identification Code |
| timestamp | string | ✓ | ISO8601形式のタイムスタンプ |

### 4.3 認証ヘッダ生成 - Rust実装

```rust
use base64::Engine;
use chrono::Utc;

pub struct LacisOathContext {
    pub lacis_id: String,
    pub tid: String,
    pub cic: String,
}

impl LacisOathContext {
    /// 認証ヘッダを生成
    pub fn to_headers(&self) -> Vec<(String, String)> {
        let auth_payload = serde_json::json!({
            "lacisId": self.lacis_id,
            "tid": self.tid,
            "cic": self.cic,
            "timestamp": Utc::now().to_rfc3339()
        });

        let json_str = serde_json::to_string(&auth_payload).unwrap();
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(json_str.as_bytes());

        vec![
            ("Authorization".to_string(), format!("LacisOath {}", encoded)),
            ("Content-Type".to_string(), "application/json".to_string()),
        ]
    }
}
```

### 4.4 認証ヘッダ生成 - Python実装

```python
import base64
import json
from datetime import datetime, timezone

class LacisOathContext:
    def __init__(self, lacis_id: str, tid: str, cic: str):
        self.lacis_id = lacis_id
        self.tid = tid
        self.cic = cic

    def get_auth_header(self) -> dict:
        """認証ヘッダを生成"""
        payload = {
            "lacisId": self.lacis_id,
            "tid": self.tid,
            "cic": self.cic,
            "timestamp": datetime.now(timezone.utc).isoformat()
        }

        json_str = json.dumps(payload)
        encoded = base64.b64encode(json_str.encode()).decode()

        return {
            "Authorization": f"LacisOath {encoded}",
            "Content-Type": "application/json"
        }
```

---

## 5. AraneaRegister - デバイス登録

### 5.1 概要

AraneaRegisterは、araneaデバイスを**araneaDeviceGate**に登録し、**CIC（Client Identification Code）**を取得するモジュールです。

### 5.2 登録フロー

```
┌─────────────────────────────────────────────────────────────────┐
│  1. テナントプライマリユーザーがWebUIから登録を実行              │
│     - lacis_id (テナントプライマリのLacisID)                    │
│     - user_id (メールアドレス)                                  │
│     - cic (テナントプライマリのCIC)                             │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. デバイスがMACアドレスからLacisIDを生成                      │
│     - get_primary_mac_address()                                 │
│     - generate_lacis_id(mac)                                    │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. araneaDeviceGate API呼び出し                                │
│     POST https://.../araneaDeviceGate                           │
│     Body: { lacisOath, userObject, deviceMeta }                 │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. 成功時: CIC・endpoint取得                                   │
│     - cic_code: デバイス固有のCIC                               │
│     - stateEndpoint: 状態報告用URL                              │
│     - mqttEndpoint: MQTT接続用URL（オプション）                 │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│  5. 永続化                                                      │
│     - config_store (メモリ) ← 即時参照用                        │
│     - DB/ファイル ← 監査・復旧用                                │
└─────────────────────────────────────────────────────────────────┘
```

### 5.3 araneaDeviceGate APIリクエスト

**エンドポイント:**
```
POST https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate
```

**リクエストBody:**
```json
{
  "lacisOath": {
    "lacisId": "12767487939173857894",  // テナントプライマリのlacisId
    "userId": "soejim@mijeos.com",       // テナントプライマリのemail
    "cic": "204965",                     // テナントプライマリのCIC
    "method": "register"
  },
  "userObject": {
    "lacisID": "3022AABBCCDDEEFF0001",   // 登録対象デバイスのlacisID
    "tid": "T2025120621041161827",       // テナントID
    "typeDomain": "araneaDevice",
    "type": "aranea_ar-is22"             // デバイスタイプ
  },
  "deviceMeta": {
    "macAddress": "AABBCCDDEEFF",        // 12桁HEX（コロンなし）
    "productType": "022",                // 3桁
    "productCode": "0001"                // 4桁
  }
}
```

### 5.4 araneaDeviceGate APIレスポンス

**成功時 (200/201):**
```json
{
  "ok": true,
  "userObject": {
    "lacisID": "3022AABBCCDDEEFF0001",
    "cic_code": "123456"                 // デバイス固有のCIC
  },
  "stateEndpoint": "https://...",        // 状態報告用URL
  "mqttEndpoint": "wss://..."            // MQTT接続用URL（オプション）
}
```

**エラー時:**
```json
{
  "ok": false,
  "error": "Error message"
}
```

**HTTPステータス:**
| Status | 意味 | 対応 |
|--------|------|------|
| 200/201 | 成功 | CIC永続化 |
| 409 | Conflict（既登録） | 既存CICを使用 |
| 401/403 | 認証エラー | テナントプライマリ認証確認 |
| 5xx | サーバーエラー | リトライ |

### 5.5 Rust実装

```rust
/// AraneaRegister Service
pub struct AraneaRegisterService {
    gate_url: String,
    http_client: Client,
    repository: AraneaRegistrationRepository,
    config_store: Arc<ConfigStore>,
}

impl AraneaRegisterService {
    /// デバイス登録
    pub async fn register_device(
        &self,
        request: RegisterRequest,
    ) -> Result<RegisterResult> {
        // 1. 既存登録チェック
        let status = self.get_registration_status().await?;
        if status.registered {
            return Ok(RegisterResult {
                ok: true,
                lacis_id: status.lacis_id,
                cic: status.cic,
                ..Default::default()
            });
        }

        // 2. MACアドレス取得
        let mac = get_primary_mac_address()?;

        // 3. LacisID生成
        let lacis_id = try_generate_lacis_id(&mac)?;

        // 4. API呼び出し
        let result = self.call_device_gate(
            &request.tenant_primary_auth,
            &request.tid,
            &lacis_id,
            &mac,
        ).await?;

        // 5. 成功時: 永続化
        if result.ok {
            self.save_registration(&result, &request.tid, request.fid.as_deref()).await?;
        }

        Ok(result)
    }
}
```

### 5.6 Python実装

```python
class AraneaRegister:
    DEFAULT_GATE_URL = "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate"

    def register_device(
        self,
        tid: str,
        device_type: str,
        lacis_id: str,
        mac_address: str,
        product_type: str,
        product_code: str,
    ) -> RegisterResult:
        result = RegisterResult()

        # 既に登録済みの場合は保存データを返す
        if self.is_registered():
            saved_cic = self.get_saved_cic()
            if saved_cic:
                result.ok = True
                result.cic_code = saved_cic
                result.state_endpoint = self.get_saved_state_endpoint()
                return result

        # JSON構築
        payload = {
            "lacisOath": {
                "lacisId": self.tenant_auth.lacis_id,
                "userId": self.tenant_auth.user_id,
                "cic": self.tenant_auth.cic,
                "method": "register"
            },
            "userObject": {
                "lacisID": lacis_id,
                "tid": tid,
                "typeDomain": "araneaDevice",
                "type": device_type
            },
            "deviceMeta": {
                "macAddress": mac_address,
                "productType": product_type,
                "productCode": product_code
            }
        }

        # HTTP POST
        response = httpx.post(self.gate_url, json=payload)

        if response.status_code in (200, 201):
            data = response.json()
            if data.get("ok"):
                result.ok = True
                result.cic_code = data["userObject"]["cic_code"]
                result.state_endpoint = data.get("stateEndpoint", "")
                self._save_registration(result)

        return result
```

---

## 6. ConfigStore - 設定永続化

### 6.1 SSoT原則

```
config_store (メモリキャッシュ) = 即時参照用（高速）
DB/ファイル (永続化) = 履歴・監査・復旧用

起動時:
  1. DB/ファイルから設定読み込み
  2. config_storeに展開

更新時:
  1. config_store更新（即時反映）
  2. DB/ファイル更新（永続化）
```

### 6.2 config_storeキー定義

```rust
pub mod config_keys {
    pub const LACIS_ID: &str = "aranea.lacis_id";
    pub const TID: &str = "aranea.tid";
    pub const FID: &str = "aranea.fid";
    pub const CIC: &str = "aranea.cic";
    pub const REGISTERED: &str = "aranea.registered";
    pub const STATE_ENDPOINT: &str = "aranea.state_endpoint";
    pub const MQTT_ENDPOINT: &str = "aranea.mqtt_endpoint";
}
```

### 6.3 Rust実装パターン

```rust
/// 設定取得
async fn get_config_value(&self, key: &str) -> Option<String> {
    self.config_store
        .service()
        .get_setting(key)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.as_str().map(String::from))
}

/// 設定保存
async fn set_config_value(&self, key: &str, value: &str) -> Result<()> {
    self.config_store
        .service()
        .set_setting(key, serde_json::json!(value))
        .await
}
```

### 6.4 Python実装パターン

```python
class ConfigStore:
    def __init__(self, config_file: str):
        self.config_file = Path(config_file)
        self._data: dict = {}
        self._load()

    def _load(self) -> None:
        if self.config_file.exists():
            with open(self.config_file, "r") as f:
                self._data = json.load(f)

    def _save(self) -> None:
        self.config_file.parent.mkdir(parents=True, exist_ok=True)
        with open(self.config_file, "w") as f:
            json.dump(self._data, f, indent=2)

    def get(self, key: str, default=None):
        return self._data.get(key, default)

    def set(self, key: str, value) -> None:
        self._data[key] = value
        self._save()
```

---

## 7. DB永続化

### 7.1 aranea_registrationテーブル

```sql
CREATE TABLE aranea_registration (
    id INT AUTO_INCREMENT PRIMARY KEY,
    lacis_id VARCHAR(20) NOT NULL UNIQUE,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32),
    cic VARCHAR(16) NOT NULL,
    device_type VARCHAR(32) NOT NULL,
    state_endpoint VARCHAR(512),
    mqtt_endpoint VARCHAR(512),
    registered_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    last_sync_at DATETIME(3),

    INDEX idx_tid (tid),
    INDEX idx_fid (fid)
);
```

### 7.2 Repository実装パターン

```rust
pub struct AraneaRegistrationRepository {
    pool: MySqlPool,
}

impl AraneaRegistrationRepository {
    /// 登録情報を挿入
    pub async fn insert(&self, reg: &AraneaRegistration) -> Result<u64> {
        let result = sqlx::query(
            r#"
            INSERT INTO aranea_registration
            (lacis_id, tid, fid, cic, device_type, state_endpoint, mqtt_endpoint, registered_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                tid = VALUES(tid),
                fid = VALUES(fid),
                cic = VALUES(cic),
                state_endpoint = VALUES(state_endpoint),
                mqtt_endpoint = VALUES(mqtt_endpoint)
            "#,
        )
        .bind(&reg.lacis_id)
        .bind(&reg.tid)
        .bind(&reg.fid)
        .bind(&reg.cic)
        .bind(&reg.device_type)
        .bind(&reg.state_endpoint)
        .bind(&reg.mqtt_endpoint)
        .bind(reg.registered_at)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id())
    }

    /// 最終同期時刻を更新
    pub async fn update_last_sync_at(&self, lacis_id: &str) -> Result<()> {
        sqlx::query("UPDATE aranea_registration SET last_sync_at = NOW(3) WHERE lacis_id = ?")
            .bind(lacis_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
```

---

## 8. エラーハンドリング

### 8.1 エラー種別

| エラー | 原因 | 対応 |
|--------|------|------|
| MACアドレス取得失敗 | ネットワークインターフェースなし | 環境確認 |
| HTTP接続タイムアウト | ネットワーク障害 | リトライ |
| HTTP 409 Conflict | 既に登録済み | 既存CICを使用 |
| HTTP 401/403 | 認証失敗 | テナントプライマリ確認 |
| HTTP 5xx | サーバーエラー | リトライ |

### 8.2 リトライ戦略

```rust
/// 指数バックオフリトライ
async fn with_retry<F, T, E>(f: F, max_retries: u32) -> Result<T, E>
where
    F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>>,
{
    let mut retry_count = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if retry_count < max_retries => {
                let delay = std::cmp::min(30 * (1 << retry_count), 3600);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                retry_count += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```

---

## 9. 新規Linuxデバイス追加手順

### 9.1 チェックリスト

1. **ProductType決定**
   - 新規3桁コードを割り当て
   - 本ドキュメントに追記

2. **定数設定**
   ```rust
   pub const PRODUCT_TYPE: &str = "0XX";  // 新規コード
   pub const DEVICE_TYPE: &str = "aranea_ar-isXX";
   ```

3. **AraneaRegisterモジュール追加**
   - `types.rs`: 定数定義
   - `lacis_id.rs`: LacisID生成（共通コード流用）
   - `service.rs`: 登録ロジック（共通コード流用）
   - `repository.rs`: DB永続化（共通コード流用）

4. **config_storeキー定義**
   - 既存キープレフィックス（`aranea.`）を使用

5. **WebAPI追加**
   ```
   POST /api/register
   GET  /api/register/status
   POST /api/register/clear
   ```

6. **DBマイグレーション**
   - `aranea_registration`テーブル作成

### 9.2 共通コード再利用

```
code/orangePi/is22/src/aranea_register/
├── mod.rs           # モジュール定義
├── types.rs         # 定数・型定義（ProductType変更のみ）
├── lacis_id.rs      # LacisID生成（そのまま流用）
├── service.rs       # 登録サービス（ほぼ流用）
└── repository.rs    # DB操作（そのまま流用）
```

---

## 10. 関連ドキュメント

- [IS22_SPECIFICATION.md](./IS22_SPECIFICATION.md) - IS22詳細仕様
- [AUTH_SPEC.md](./AUTH_SPEC.md) - LacisOath認証仕様
- [DD01_AraneaRegister.md](../../code/orangePi/is21-22/designs/headdesign/Paraclate/DetailedDesign/DD01_AraneaRegister.md)

---

## 11. Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-20 | 初版作成 - Linux共通モジュール仕様を体系的に文書化 |
