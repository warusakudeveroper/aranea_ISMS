# DD01: AraneaRegister 詳細設計

作成日: 2026-01-10
対象: is22 Paraclate Server
ステータス: 詳細設計

## 1. 目的と概要

### 1.1 目的
is22（Paraclate Server）のaraneaDeviceGateへのデバイス登録機能を実装する。
登録によりlacisOath認証に必要なCIC（Client Identification Code）を取得し、
Paraclate APP連携およびtid/fid権限境界に準拠した運用を可能にする。

### 1.2 スコープ
- is22自身（ar-is22CamServer）のデバイス登録
- 登録情報の永続化（config_store連携）
- 再起動時の再登録回避
- 登録状態のAPI公開

### 1.3 スコープ外
- カメラ（is801）の登録（DD05で設計）
- Paraclate APP側の登録受付処理

## 2. 依存関係

### 2.1 参照ドキュメント
| ドキュメント | 参照セクション |
|-------------|---------------|
| Paraclate_DesignOverview.md | TypeDomain/Type定義、lacisOath認証 |
| Paraclate_BasicDesign.md | アーキテクチャ、データ設計 |
| 03_authentication.md | 認証フロー、LacisID生成ルール |
| The_golden_rules.md | SSoT/SOLID/MECE準拠 |

### 2.2 既存実装参照
| 実装 | 用途 |
|------|------|
| `code/ESP32/global/src/AraneaRegister.cpp` | ESP32版登録実装（移植元） |
| `code/orangePi/is22/src/config_store/` | 設定永続化モジュール |

### 2.3 外部依存
| サービス | エンドポイント | 用途 |
|---------|---------------|------|
| araneaDeviceGate | `https://{host}/api/araneaDeviceGate/register` | デバイス登録API |

## 3. データ設計

### 3.1 LacisID生成ルール（is22用）

```
[Prefix][ProductType][MAC(12桁)][ProductCode]
   3        022        XXXXXXXXXXXX     0000
```

| 要素 | 桁数 | 値 | 備考 |
|------|------|-----|------|
| Prefix | 1 | 3 | araneaDevice共通 |
| ProductType | 3 | 022 | is22 Paraclate Server |
| MAC | 12 | (デバイスMAC) | コロン除去、大文字 |
| ProductCode | 4 | 0000 | 追い番なしのため固定 |

**例**: MAC=AA:BB:CC:DD:EE:FF → LacisID=`3022AABBCCDDEEFF0000`

### 3.2 新規テーブル: aranea_registration

```sql
CREATE TABLE aranea_registration (
    id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    lacis_id VARCHAR(20) NOT NULL UNIQUE,
    tid VARCHAR(32) NOT NULL,
    cic VARCHAR(16) NOT NULL,
    device_type VARCHAR(32) NOT NULL DEFAULT 'ar-is22CamServer',
    state_endpoint VARCHAR(256),
    mqtt_endpoint VARCHAR(256),
    registered_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    last_sync_at DATETIME(3),
    INDEX idx_tid (tid),
    INDEX idx_lacis_id (lacis_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

### 3.3 config_store拡張

既存の`config_store`に以下のキーを追加:

| キー | 型 | 説明 |
|-----|-----|------|
| `aranea.lacis_id` | String | 生成済みLacisID |
| `aranea.tid` | String | 所属テナントID |
| `aranea.cic` | String | 取得済みCIC |
| `aranea.registered` | Boolean | 登録完了フラグ |
| `aranea.state_endpoint` | String | 状態報告先URL |

## 4. API設計

### 4.1 内部API（is22管理画面用）

#### POST /api/register/device
デバイス登録を実行（手動トリガー用）

**Request**
```json
{
  "tenantPrimaryAuth": {
    "lacisId": "18217487937895888001",
    "userId": "soejim@mijeos.com",
    "cic": "204965"
  },
  "tid": "T2025120621041161827"
}
```

**Response (201 Created)**
```json
{
  "ok": true,
  "lacisId": "3022AABBCCDDEEFF0000",
  "cic": "123456",
  "stateEndpoint": "https://..."
}
```

**Response (409 Conflict)**
```json
{
  "ok": false,
  "error": "Device already registered"
}
```

#### GET /api/register/status
登録状態を取得

**Response**
```json
{
  "registered": true,
  "lacisId": "3022AABBCCDDEEFF0000",
  "tid": "T2025120621041161827",
  "registeredAt": "2026-01-10T10:00:00.000Z",
  "lastSyncAt": "2026-01-10T12:00:00.000Z"
}
```

#### DELETE /api/register
登録情報をクリア（再登録用）

**Response (200 OK)**
```json
{
  "ok": true,
  "message": "Registration cleared"
}
```

### 4.2 araneaDeviceGate連携

#### POST /api/araneaDeviceGate/register
外部APIへのリクエスト形式

**Request**
```json
{
  "lacisOath": {
    "lacisId": "18217487937895888001",
    "userId": "soejim@mijeos.com",
    "cic": "204965",
    "method": "register"
  },
  "userObject": {
    "lacisID": "3022AABBCCDDEEFF0000",
    "tid": "T2025120621041161827",
    "typeDomain": "araneaDevice",
    "type": "ar-is22CamServer"
  },
  "deviceMeta": {
    "macAddress": "AA:BB:CC:DD:EE:FF",
    "productType": "022",
    "productCode": "0000"
  }
}
```

**Response (201 Created)**
```json
{
  "ok": true,
  "userObject": {
    "lacisID": "3022AABBCCDDEEFF0000",
    "cic_code": "123456"
  },
  "stateEndpoint": "https://..."
}
```

## 5. モジュール構造

### 5.1 ディレクトリ構成

```
src/
├── aranea_register/
│   ├── mod.rs          # モジュールエクスポート
│   ├── types.rs        # 型定義
│   ├── lacis_id.rs     # LacisID生成ロジック
│   ├── service.rs      # 登録サービス本体
│   └── repository.rs   # DB永続化
├── web_api/
│   └── register_routes.rs  # APIルート（新規追加）
```

### 5.2 型定義 (types.rs)

```rust
/// テナントプライマリ認証情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantPrimaryAuth {
    pub lacis_id: String,
    pub user_id: String,
    pub cic: String,
}

/// 登録リクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub tenant_primary_auth: TenantPrimaryAuth,
    pub tid: String,
}

/// 登録結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResult {
    pub ok: bool,
    pub lacis_id: Option<String>,
    pub cic: Option<String>,
    pub state_endpoint: Option<String>,
    pub mqtt_endpoint: Option<String>,
    pub error: Option<String>,
}

/// 登録状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationStatus {
    pub registered: bool,
    pub lacis_id: Option<String>,
    pub tid: Option<String>,
    pub registered_at: Option<DateTime<Utc>>,
    pub last_sync_at: Option<DateTime<Utc>>,
}

/// デバイス種別定数
pub const DEVICE_TYPE: &str = "ar-is22CamServer";
pub const TYPE_DOMAIN: &str = "araneaDevice";
pub const PREFIX: &str = "3";
pub const PRODUCT_TYPE: &str = "022";
pub const PRODUCT_CODE: &str = "0000";
```

### 5.3 LacisID生成 (lacis_id.rs)

```rust
use crate::aranea_register::types::{PREFIX, PRODUCT_TYPE, PRODUCT_CODE};

/// MACアドレスからLacisIDを生成
///
/// # Arguments
/// * `mac` - MACアドレス（任意形式: AA:BB:CC:DD:EE:FF or AABBCCDDEEFF）
///
/// # Returns
/// 20桁のLacisID文字列
pub fn generate_lacis_id(mac: &str) -> String {
    // MACアドレスからコロン/ハイフンを除去し大文字化
    let mac_clean: String = mac
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_uppercase();

    assert_eq!(mac_clean.len(), 12, "Invalid MAC address length");

    format!(
        "{}{}{}{}",
        PREFIX,
        PRODUCT_TYPE,
        mac_clean,
        PRODUCT_CODE
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_lacis_id() {
        assert_eq!(
            generate_lacis_id("AA:BB:CC:DD:EE:FF"),
            "3022AABBCCDDEEFF0000"
        );
        assert_eq!(
            generate_lacis_id("aabbccddeeff"),
            "3022AABBCCDDEEFF0000"
        );
    }
}
```

### 5.4 登録サービス (service.rs)

```rust
use crate::aranea_register::{
    types::*,
    lacis_id::generate_lacis_id,
    repository::AraneaRegistrationRepository,
};
use crate::config_store::ConfigStore;

pub struct AraneaRegisterService {
    gate_url: String,
    http_client: reqwest::Client,
    repository: AraneaRegistrationRepository,
    config_store: Arc<ConfigStore>,
}

impl AraneaRegisterService {
    pub fn new(
        gate_url: String,
        pool: Pool<MySql>,
        config_store: Arc<ConfigStore>,
    ) -> Self {
        Self {
            gate_url,
            http_client: reqwest::Client::new(),
            repository: AraneaRegistrationRepository::new(pool),
            config_store,
        }
    }

    /// デバイス登録を実行
    pub async fn register_device(
        &self,
        request: RegisterRequest,
    ) -> Result<RegisterResult, AraneaRegisterError> {
        // 1. 既存登録チェック
        if let Some(existing) = self.get_registration_status().await? {
            if existing.registered {
                return Ok(RegisterResult {
                    ok: true,
                    lacis_id: existing.lacis_id,
                    cic: self.config_store.get("aranea.cic").await.ok(),
                    state_endpoint: self.config_store.get("aranea.state_endpoint").await.ok(),
                    mqtt_endpoint: None,
                    error: None,
                });
            }
        }

        // 2. MACアドレス取得
        let mac = get_primary_mac_address()?;

        // 3. LacisID生成
        let lacis_id = generate_lacis_id(&mac);

        // 4. araneaDeviceGateへ登録
        let result = self.call_device_gate(
            &request.tenant_primary_auth,
            &request.tid,
            &lacis_id,
            &mac,
        ).await?;

        // 5. 成功時は永続化
        if result.ok {
            self.save_registration(&result, &request.tid).await?;
        }

        Ok(result)
    }

    /// araneaDeviceGate API呼び出し
    async fn call_device_gate(
        &self,
        auth: &TenantPrimaryAuth,
        tid: &str,
        lacis_id: &str,
        mac: &str,
    ) -> Result<RegisterResult, AraneaRegisterError> {
        let payload = json!({
            "lacisOath": {
                "lacisId": auth.lacis_id,
                "userId": auth.user_id,
                "cic": auth.cic,
                "method": "register"
            },
            "userObject": {
                "lacisID": lacis_id,
                "tid": tid,
                "typeDomain": TYPE_DOMAIN,
                "type": DEVICE_TYPE
            },
            "deviceMeta": {
                "macAddress": mac,
                "productType": PRODUCT_TYPE,
                "productCode": PRODUCT_CODE
            }
        });

        let response = self.http_client
            .post(&format!("{}/register", self.gate_url))
            .json(&payload)
            .timeout(Duration::from_secs(15))
            .send()
            .await?;

        let status = response.status();
        let body: serde_json::Value = response.json().await?;

        match status.as_u16() {
            200 | 201 => {
                let ok = body["ok"].as_bool().unwrap_or(false);
                if ok {
                    Ok(RegisterResult {
                        ok: true,
                        lacis_id: Some(lacis_id.to_string()),
                        cic: body["userObject"]["cic_code"]
                            .as_str()
                            .map(String::from),
                        state_endpoint: body["stateEndpoint"]
                            .as_str()
                            .map(String::from),
                        mqtt_endpoint: body["mqttEndpoint"]
                            .as_str()
                            .map(String::from),
                        error: None,
                    })
                } else {
                    Ok(RegisterResult {
                        ok: false,
                        error: body["error"].as_str().map(String::from),
                        ..Default::default()
                    })
                }
            }
            409 => Ok(RegisterResult {
                ok: false,
                error: Some("Device already registered (conflict)".to_string()),
                ..Default::default()
            }),
            _ => Ok(RegisterResult {
                ok: false,
                error: body["error"].as_str().map(String::from),
                ..Default::default()
            }),
        }
    }

    /// 登録情報を永続化
    async fn save_registration(
        &self,
        result: &RegisterResult,
        tid: &str,
    ) -> Result<(), AraneaRegisterError> {
        // config_storeに保存
        if let Some(ref lacis_id) = result.lacis_id {
            self.config_store.set("aranea.lacis_id", lacis_id).await?;
        }
        if let Some(ref cic) = result.cic {
            self.config_store.set("aranea.cic", cic).await?;
        }
        if let Some(ref endpoint) = result.state_endpoint {
            self.config_store.set("aranea.state_endpoint", endpoint).await?;
        }
        self.config_store.set("aranea.tid", tid).await?;
        self.config_store.set("aranea.registered", "true").await?;

        // DBにも保存
        self.repository.insert(AraneaRegistration {
            lacis_id: result.lacis_id.clone().unwrap_or_default(),
            tid: tid.to_string(),
            cic: result.cic.clone().unwrap_or_default(),
            device_type: DEVICE_TYPE.to_string(),
            state_endpoint: result.state_endpoint.clone(),
            mqtt_endpoint: result.mqtt_endpoint.clone(),
            registered_at: Utc::now(),
            last_sync_at: None,
        }).await?;

        Ok(())
    }

    /// 登録状態を取得
    pub async fn get_registration_status(&self) -> Result<RegistrationStatus, AraneaRegisterError> {
        let registered = self.config_store
            .get("aranea.registered")
            .await
            .map(|v| v == "true")
            .unwrap_or(false);

        if !registered {
            return Ok(RegistrationStatus {
                registered: false,
                lacis_id: None,
                tid: None,
                registered_at: None,
                last_sync_at: None,
            });
        }

        Ok(RegistrationStatus {
            registered: true,
            lacis_id: self.config_store.get("aranea.lacis_id").await.ok(),
            tid: self.config_store.get("aranea.tid").await.ok(),
            registered_at: self.repository.get_registered_at().await.ok(),
            last_sync_at: self.repository.get_last_sync_at().await.ok(),
        })
    }

    /// 登録情報をクリア
    pub async fn clear_registration(&self) -> Result<(), AraneaRegisterError> {
        self.config_store.remove("aranea.lacis_id").await?;
        self.config_store.remove("aranea.tid").await?;
        self.config_store.remove("aranea.cic").await?;
        self.config_store.remove("aranea.state_endpoint").await?;
        self.config_store.remove("aranea.registered").await?;

        self.repository.delete_all().await?;

        Ok(())
    }
}
```

## 6. 処理フロー

### 6.1 起動時自動登録フロー

```
┌─────────────────────────────────────────────────────────────┐
│                    is22起動                                  │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ config_store確認       │
              │ aranea.registered?    │
              └───────────────────────┘
                          │
           ┌──────────────┴──────────────┐
           │                             │
      [registered=true]           [registered=false]
           │                             │
           ▼                             ▼
    ┌─────────────┐            ┌──────────────────┐
    │ CIC取得      │            │ 設定ファイル確認  │
    │ 既存情報利用  │            │ tenantPrimaryAuth│
    └─────────────┘            └──────────────────┘
           │                             │
           │                   ┌─────────┴─────────┐
           │                   │                   │
           │              [設定あり]          [設定なし]
           │                   │                   │
           │                   ▼                   ▼
           │         ┌──────────────────┐   ┌──────────┐
           │         │ MAC取得           │   │ 待機     │
           │         │ LacisID生成       │   │ (UI要求) │
           │         └──────────────────┘   └──────────┘
           │                   │
           │                   ▼
           │         ┌──────────────────┐
           │         │ araneaDeviceGate │
           │         │ POST /register   │
           │         └──────────────────┘
           │                   │
           │         ┌────────┴────────┐
           │         │                 │
           │      [成功]            [失敗]
           │         │                 │
           │         ▼                 ▼
           │   ┌───────────┐    ┌───────────┐
           │   │ CIC保存    │    │ エラー通知 │
           │   │ DB保存     │    │ 再試行待機 │
           │   └───────────┘    └───────────┘
           │         │
           └─────────┤
                     ▼
          ┌────────────────────┐
          │ Paraclate連携有効化 │
          │ Summary/BQ同期開始  │
          └────────────────────┘
```

### 6.2 エラーハンドリングフロー

| エラー | 原因 | 対応 |
|-------|------|------|
| HTTP 409 | MAC重複（既登録） | config_store確認、再登録不要 |
| HTTP 401/403 | 認証失敗 | tenantPrimaryAuthの再確認を促す |
| HTTP 5xx | サーバーエラー | 指数バックオフでリトライ |
| Timeout | ネットワーク問題 | 3回まで再試行後、手動トリガー待機 |
| MAC取得失敗 | ネットワークI/F問題 | エラーログ出力、手動入力オプション |

## 7. セキュリティ考慮

### 7.1 認証情報の保護

| 情報 | 保護方法 |
|------|---------|
| TenantPrimaryAuth | 設定ファイルに暗号化保存（環境変数優先） |
| CIC | config_storeに平文保存（ローカルのみ参照） |
| stateEndpoint | config_storeに保存（HTTPSのみ許可） |

### 7.2 通信セキュリティ

- araneaDeviceGateへの通信はHTTPS必須
- TLS 1.2以上を強制
- 証明書検証を有効化

## 8. テスト計画

### 8.1 ユニットテスト

| テストケース | 内容 |
|-------------|------|
| test_generate_lacis_id_colon | AA:BB:CC:DD:EE:FF → 正常変換 |
| test_generate_lacis_id_lowercase | aabbccddeeff → 大文字化変換 |
| test_generate_lacis_id_invalid | 不正MAC → panic |

### 8.2 結合テスト

| テストケース | 前提条件 | 期待結果 |
|-------------|---------|---------|
| 新規登録成功 | 未登録状態 | CIC取得、DB保存、config_store更新 |
| 再起動後再登録回避 | 登録済み | 既存CIC利用、API呼び出しなし |
| 重複登録エラー | MAC重複 | 409エラー、既存情報維持 |
| 認証エラー | 不正な認証情報 | 401/403エラー、未登録維持 |

### 8.3 E2Eテスト

| テストケース | 手順 | 確認項目 |
|-------------|------|---------|
| 初回起動登録 | is22起動→設定入力→登録 | UI表示、API応答、DB確認 |
| 登録状態表示 | /api/register/status GET | 登録情報が正しく返る |
| 登録クリア | /api/register DELETE→再起動 | 再登録が実行される |

## 9. MECE/SOLID確認

### 9.1 MECE確認
- **網羅性**: LacisID生成、API呼び出し、永続化、状態管理、エラー処理を全カバー
- **重複排除**: config_storeとDBの役割を明確化（即時参照=config_store、履歴/監査=DB）
- **未カバー領域**: カメラ登録はDD05で設計

### 9.2 SOLID確認
- **SRP**: AraneaRegisterServiceは登録のみ担当、永続化はrepository分離
- **OCP**: 新デバイスタイプ追加時は定数追加のみで対応可能
- **LSP**: RegisterResultインターフェースで結果を統一
- **ISP**: 登録・状態取得・クリアをメソッド分離
- **DIP**: ConfigStore/Repository抽象に依存、具象実装は注入

## 10. マイグレーション

### 10.1 SQLマイグレーション

ファイル: `migrations/018_aranea_registration.sql`

```sql
-- Migration: 018_aranea_registration
-- Description: AraneaRegister用テーブル作成

CREATE TABLE IF NOT EXISTS aranea_registration (
    id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    lacis_id VARCHAR(20) NOT NULL UNIQUE,
    tid VARCHAR(32) NOT NULL,
    cic VARCHAR(16) NOT NULL,
    device_type VARCHAR(32) NOT NULL DEFAULT 'ar-is22CamServer',
    state_endpoint VARCHAR(256),
    mqtt_endpoint VARCHAR(256),
    registered_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    last_sync_at DATETIME(3),
    INDEX idx_tid (tid),
    INDEX idx_lacis_id (lacis_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

---

**SSoT宣言**: 本詳細設計は`Paraclate_BasicDesign.md`を正とし、ESP32版`AraneaRegister.cpp`の実装パターンを移植する。
**MECE確認**: 登録ライフサイクル全体（生成→送信→保存→参照→クリア）を網羅し、重複なし。
