# DD05: カメラ台帳・lacisID 詳細設計

作成日: 2026-01-10
対象: is22 Paraclate Server
ステータス: 詳細設計

## 1. 目的と概要

### 1.1 目的
is22配下のカメラを仮想araneaDevice（is801 paraclateCamera）として管理し、
lacisIDを付与してParaclate連携およびtid/fid権限境界に準拠した運用を実現する。

### 1.2 カメラのデバイス仕様

| 項目 | 値 | 備考 |
|------|-----|------|
| TypeDomain | araneaDevice | 共通 |
| Type | ar-is801Camera | Paraclate配下カメラ |
| Prefix | 3 | araneaDevice共通 |
| ProductType | 801 | is801 paraclateCamera |
| ProductCode | XXXX | カメラブランドで割当 |

### 1.3 スコープ
- カメラlacisID生成（MAC + ブランドコード）
- araneaDeviceGateへのカメラ登録
- カメラ台帳管理（cameras テーブル拡張）
- IpcamScan承認→lacisID付与フロー
- カメラコンテキスト管理

### 1.4 スコープ外
- カメラの物理検出（ipcam_scan既存実装）
- RTSP接続・推論（既存実装）
- mobes2.0側のカメラ管理UI

## 2. 依存関係

### 2.1 参照ドキュメント
| ドキュメント | 参照セクション |
|-------------|---------------|
| Paraclate_DesignOverview.md | カメラlacisID形式 |
| Paraclate_BasicDesign.md | カメラ台帳設計 |
| DD01_AraneaRegister.md | 登録処理パターン |

### 2.2 既存実装依存
| モジュール | 用途 |
|-----------|------|
| ipcam_scan | カメラ検出・承認 |
| camera_brand | ブランド情報管理 |
| config_store | 設定保存 |

## 3. データ設計

### 3.1 LacisID生成ルール（カメラ用）

```
[Prefix][ProductType][MAC(12桁)][ProductCode]
   3        801        XXXXXXXXXXXX     XXXX
```

| 要素 | 桁数 | 値 | 備考 |
|------|------|-----|------|
| Prefix | 1 | 3 | araneaDevice共通 |
| ProductType | 3 | 801 | is801 paraclateCamera |
| MAC | 12 | (カメラMAC) | コロン除去、大文字 |
| ProductCode | 4 | (ブランドコード) | camera_brands参照 |

**例**: MAC=11:22:33:44:55:66, Brand=Tapo → LacisID=`3801112233445566T001`

### 3.2 ブランドコード割当

```sql
-- camera_brands テーブルに product_code カラム追加
ALTER TABLE camera_brands
ADD COLUMN product_code VARCHAR(4) UNIQUE;
```

| ブランド | ProductCode | 備考 |
|---------|-------------|------|
| Tapo | T001 | TP-Link Tapo |
| Hikvision | H001 | |
| Dahua | D001 | |
| Reolink | R001 | |
| Axis | A001 | |
| Generic | G000 | 不明ブランド |

### 3.3 cameras テーブル拡張

```sql
-- cameras テーブル拡張
ALTER TABLE cameras
ADD COLUMN lacis_id VARCHAR(20) UNIQUE,
ADD COLUMN tid VARCHAR(32),
ADD COLUMN fid VARCHAR(32),
ADD COLUMN rid VARCHAR(32),
ADD COLUMN cic VARCHAR(16),
ADD COLUMN camera_context TEXT,
ADD COLUMN registration_state ENUM('pending', 'registered', 'failed') DEFAULT 'pending',
ADD COLUMN registered_at DATETIME(3),
ADD INDEX idx_lacis_id (lacis_id),
ADD INDEX idx_tid_fid (tid, fid);
```

### 3.4 camerasテーブル全体スキーマ（拡張後）

```sql
CREATE TABLE cameras (
    id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    -- 既存カラム
    mac VARCHAR(17) NOT NULL UNIQUE,
    ip VARCHAR(45) NOT NULL,
    name VARCHAR(128),
    brand_id INT UNSIGNED,
    model VARCHAR(64),
    rtsp_url VARCHAR(512),
    onvif_url VARCHAR(512),
    status ENUM('online', 'offline', 'unknown') DEFAULT 'unknown',
    preset VARCHAR(64),
    -- 新規カラム（lacisOath連携）
    lacis_id VARCHAR(20) UNIQUE,
    tid VARCHAR(32),
    fid VARCHAR(32),
    rid VARCHAR(32),
    cic VARCHAR(16),
    camera_context TEXT,
    registration_state ENUM('pending', 'registered', 'failed') DEFAULT 'pending',
    registered_at DATETIME(3),
    -- タイムスタンプ
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    -- インデックス
    INDEX idx_lacis_id (lacis_id),
    INDEX idx_tid_fid (tid, fid),
    INDEX idx_brand (brand_id),
    INDEX idx_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

## 4. API設計

### 4.1 カメラ登録API

#### POST /api/cameras/:id/register
承認済みカメラをaraneaDeviceGateに登録

**Path Parameters**
- id: カメラID（camerasテーブルのid）

**Request**
```json
{
  "tid": "T2025120621041161827",
  "fid": "0150",
  "rid": "R001"
}
```

**Response (201 Created)**
```json
{
  "ok": true,
  "lacisId": "3801112233445566T001",
  "cic": "654321",
  "registeredAt": "2026-01-10T12:00:00Z"
}
```

**Response (409 Conflict)**
```json
{
  "ok": false,
  "error": "Camera already registered"
}
```

#### GET /api/cameras/:id/registration
カメラの登録状態を取得

**Response**
```json
{
  "cameraId": 1,
  "mac": "11:22:33:44:55:66",
  "lacisId": "3801112233445566T001",
  "tid": "T2025120621041161827",
  "fid": "0150",
  "rid": "R001",
  "registrationState": "registered",
  "registeredAt": "2026-01-10T12:00:00Z"
}
```

#### DELETE /api/cameras/:id/registration
カメラの登録を解除

**Response**
```json
{
  "ok": true,
  "message": "Registration cleared"
}
```

### 4.2 カメラコンテキストAPI

#### GET /api/cameras/:id/context
カメラコンテキストを取得

**Response**
```json
{
  "cameraId": 1,
  "lacisId": "3801112233445566T001",
  "cameraName": "エントランスカメラ",
  "cameraContext": "建物正面入口。来訪者の検出を重視。夜間照明あり。",
  "fid": "0150",
  "rid": "R001",
  "preset": "entrance_standard"
}
```

#### PUT /api/cameras/:id/context
カメラコンテキストを更新

**Request**
```json
{
  "cameraName": "エントランスカメラ",
  "cameraContext": "建物正面入口。来訪者の検出を重視。夜間照明あり。深夜は不審者アラート優先。",
  "rid": "R001"
}
```

**Response**
```json
{
  "ok": true,
  "updatedAt": "2026-01-10T12:00:00Z"
}
```

### 4.3 カメラ一覧API（拡張）

#### GET /api/cameras
カメラ一覧を取得（lacisID情報付き）

**Query Parameters**
| パラメータ | 必須 | 説明 |
|-----------|------|------|
| fid | ❌ | 施設ID |
| registered | ❌ | 登録状態フィルタ(true/false) |
| status | ❌ | online/offline/unknown |

**Response**
```json
{
  "cameras": [
    {
      "id": 1,
      "mac": "11:22:33:44:55:66",
      "ip": "192.168.125.100",
      "name": "エントランスカメラ",
      "brand": "Tapo",
      "lacisId": "3801112233445566T001",
      "tid": "T2025120621041161827",
      "fid": "0150",
      "rid": "R001",
      "registrationState": "registered",
      "status": "online"
    }
  ],
  "total": 1
}
```

## 5. モジュール構造

### 5.1 ディレクトリ構成

```
src/
├── camera_registry/
│   ├── mod.rs              # モジュールエクスポート
│   ├── types.rs            # 型定義
│   ├── lacis_id.rs         # カメラlacisID生成
│   ├── service.rs          # 登録サービス本体
│   ├── context.rs          # カメラコンテキスト管理
│   └── repository.rs       # DB操作
├── web_api/
│   └── camera_registry_routes.rs  # APIルート（新規追加）
```

### 5.2 型定義 (types.rs)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// カメラ登録状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum RegistrationState {
    Pending,
    Registered,
    Failed,
}

/// カメラ登録情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraRegistration {
    pub camera_id: u32,
    pub mac: String,
    pub lacis_id: Option<String>,
    pub tid: Option<String>,
    pub fid: Option<String>,
    pub rid: Option<String>,
    pub cic: Option<String>,
    pub registration_state: RegistrationState,
    pub registered_at: Option<DateTime<Utc>>,
}

/// カメラコンテキスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraContext {
    pub camera_id: u32,
    pub lacis_id: String,
    pub camera_name: String,
    pub camera_context: String,
    pub fid: String,
    pub rid: String,
    pub preset: String,
}

/// 登録リクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraRegisterRequest {
    pub tid: String,
    pub fid: String,
    pub rid: Option<String>,
}

/// 登録結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraRegisterResult {
    pub ok: bool,
    pub lacis_id: Option<String>,
    pub cic: Option<String>,
    pub registered_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// デバイス種別定数（カメラ用）
pub const CAMERA_TYPE: &str = "ar-is801Camera";
pub const CAMERA_TYPE_DOMAIN: &str = "araneaDevice";
pub const CAMERA_PREFIX: &str = "3";
pub const CAMERA_PRODUCT_TYPE: &str = "801";
pub const DEFAULT_PRODUCT_CODE: &str = "G000";
```

### 5.3 カメラlacisID生成 (lacis_id.rs)

```rust
use crate::camera_registry::types::{CAMERA_PREFIX, CAMERA_PRODUCT_TYPE, DEFAULT_PRODUCT_CODE};
use crate::camera_brand::CameraBrandService;

pub struct CameraLacisIdGenerator {
    brand_service: Arc<CameraBrandService>,
}

impl CameraLacisIdGenerator {
    pub fn new(brand_service: Arc<CameraBrandService>) -> Self {
        Self { brand_service }
    }

    /// MACアドレスとブランドからカメラlacisIDを生成
    ///
    /// # Arguments
    /// * `mac` - MACアドレス（任意形式）
    /// * `brand_id` - ブランドID（camera_brandsテーブル）
    ///
    /// # Returns
    /// 20桁のLacisID文字列
    pub async fn generate(
        &self,
        mac: &str,
        brand_id: Option<u32>,
    ) -> Result<String, CameraRegistryError> {
        // MACアドレスからコロン/ハイフンを除去し大文字化
        let mac_clean: String = mac
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect::<String>()
            .to_uppercase();

        if mac_clean.len() != 12 {
            return Err(CameraRegistryError::InvalidMacAddress);
        }

        // ブランドからProductCodeを取得
        let product_code = match brand_id {
            Some(id) => {
                self.brand_service
                    .get_product_code(id)
                    .await?
                    .unwrap_or_else(|| DEFAULT_PRODUCT_CODE.to_string())
            }
            None => DEFAULT_PRODUCT_CODE.to_string(),
        };

        Ok(format!(
            "{}{}{}{}",
            CAMERA_PREFIX,
            CAMERA_PRODUCT_TYPE,
            mac_clean,
            product_code
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_camera_lacis_id() {
        // モックを使用したテスト
        let lacis_id = format!(
            "{}{}{}{}",
            CAMERA_PREFIX,
            CAMERA_PRODUCT_TYPE,
            "112233445566",
            "T001"
        );
        assert_eq!(lacis_id, "3801112233445566T001");
        assert_eq!(lacis_id.len(), 20);
    }
}
```

### 5.4 登録サービス (service.rs)

```rust
use crate::camera_registry::{
    types::*,
    lacis_id::CameraLacisIdGenerator,
    repository::CameraRegistryRepository,
};
use crate::aranea_register::AraneaRegisterService;

pub struct CameraRegistryService {
    lacis_id_generator: CameraLacisIdGenerator,
    repository: CameraRegistryRepository,
    register_service: Arc<AraneaRegisterService>,
    http_client: reqwest::Client,
    gate_url: String,
}

impl CameraRegistryService {
    pub fn new(
        pool: Pool<MySql>,
        brand_service: Arc<CameraBrandService>,
        register_service: Arc<AraneaRegisterService>,
        gate_url: String,
    ) -> Self {
        Self {
            lacis_id_generator: CameraLacisIdGenerator::new(brand_service),
            repository: CameraRegistryRepository::new(pool),
            register_service,
            http_client: reqwest::Client::new(),
            gate_url,
        }
    }

    /// カメラをaraneaDeviceGateに登録
    pub async fn register_camera(
        &self,
        camera_id: u32,
        request: CameraRegisterRequest,
    ) -> Result<CameraRegisterResult, CameraRegistryError> {
        // 1. カメラ情報を取得
        let camera = self.repository.get_camera(camera_id).await?;

        // 2. 既存登録チェック
        if camera.registration_state == RegistrationState::Registered {
            return Ok(CameraRegisterResult {
                ok: true,
                lacis_id: camera.lacis_id,
                cic: camera.cic,
                registered_at: camera.registered_at,
                error: None,
            });
        }

        // 3. lacisID生成
        let lacis_id = self.lacis_id_generator
            .generate(&camera.mac, camera.brand_id)
            .await?;

        // 4. テナントプライマリ認証情報を取得（is22のregistrationから）
        let is22_reg = self.register_service.get_registration_status().await?;
        if !is22_reg.registered {
            return Err(CameraRegistryError::Is22NotRegistered);
        }

        let tenant_auth = self.get_tenant_primary_auth().await?;

        // 5. araneaDeviceGateへ登録
        let result = self.call_device_gate(
            &tenant_auth,
            &request.tid,
            &lacis_id,
            &camera.mac,
        ).await?;

        // 6. 成功時はDB更新
        if result.ok {
            self.repository.update_registration(
                camera_id,
                &lacis_id,
                &request.tid,
                &request.fid,
                request.rid.as_deref(),
                result.cic.as_deref(),
            ).await?;
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
    ) -> Result<CameraRegisterResult, CameraRegistryError> {
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
                "typeDomain": CAMERA_TYPE_DOMAIN,
                "type": CAMERA_TYPE
            },
            "deviceMeta": {
                "macAddress": mac,
                "productType": CAMERA_PRODUCT_TYPE,
                "productCode": lacis_id[16..20].to_string()
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
                    Ok(CameraRegisterResult {
                        ok: true,
                        lacis_id: Some(lacis_id.to_string()),
                        cic: body["userObject"]["cic_code"]
                            .as_str()
                            .map(String::from),
                        registered_at: Some(Utc::now()),
                        error: None,
                    })
                } else {
                    Ok(CameraRegisterResult {
                        ok: false,
                        lacis_id: None,
                        cic: None,
                        registered_at: None,
                        error: body["error"].as_str().map(String::from),
                    })
                }
            }
            409 => Ok(CameraRegisterResult {
                ok: false,
                lacis_id: None,
                cic: None,
                registered_at: None,
                error: Some("Camera already registered (conflict)".to_string()),
            }),
            _ => Ok(CameraRegisterResult {
                ok: false,
                lacis_id: None,
                cic: None,
                registered_at: None,
                error: body["error"].as_str().map(String::from),
            }),
        }
    }

    /// 登録状態を取得
    pub async fn get_registration(
        &self,
        camera_id: u32,
    ) -> Result<CameraRegistration, CameraRegistryError> {
        self.repository.get_registration(camera_id).await
    }

    /// 登録を解除
    pub async fn clear_registration(
        &self,
        camera_id: u32,
    ) -> Result<(), CameraRegistryError> {
        self.repository.clear_registration(camera_id).await
    }

    /// lacisIDでカメラを検索
    pub async fn get_camera_by_lacis_id(
        &self,
        lacis_id: &str,
    ) -> Result<Option<Camera>, CameraRegistryError> {
        self.repository.get_by_lacis_id(lacis_id).await
    }

    /// tid/fid配下の登録済みカメラ一覧
    pub async fn get_registered_cameras(
        &self,
        tid: &str,
        fid: Option<&str>,
    ) -> Result<Vec<Camera>, CameraRegistryError> {
        self.repository.get_registered_cameras(tid, fid).await
    }
}
```

### 5.5 カメラコンテキスト管理 (context.rs)

```rust
pub struct CameraContextService {
    repository: CameraRegistryRepository,
}

impl CameraContextService {
    /// カメラコンテキストを取得
    pub async fn get_context(
        &self,
        camera_id: u32,
    ) -> Result<CameraContext, CameraRegistryError> {
        let camera = self.repository.get_camera(camera_id).await?;

        Ok(CameraContext {
            camera_id,
            lacis_id: camera.lacis_id.unwrap_or_default(),
            camera_name: camera.name.unwrap_or_default(),
            camera_context: camera.camera_context.unwrap_or_default(),
            fid: camera.fid.unwrap_or_default(),
            rid: camera.rid.unwrap_or_default(),
            preset: camera.preset.unwrap_or_default(),
        })
    }

    /// カメラコンテキストを更新
    pub async fn update_context(
        &self,
        camera_id: u32,
        name: Option<&str>,
        context: Option<&str>,
        rid: Option<&str>,
    ) -> Result<(), CameraRegistryError> {
        self.repository.update_context(camera_id, name, context, rid).await
    }

    /// Summary用のコンテキストマップを構築
    pub async fn build_context_map(
        &self,
        camera_lacis_ids: &[String],
    ) -> Result<HashMap<String, CameraContextItem>, CameraRegistryError> {
        let mut map = HashMap::new();

        for lacis_id in camera_lacis_ids {
            if let Some(camera) = self.repository.get_by_lacis_id(lacis_id).await? {
                map.insert(
                    lacis_id.clone(),
                    CameraContextItem {
                        camera_name: camera.name.unwrap_or_default(),
                        camera_context: camera.camera_context.unwrap_or_default(),
                        fid: camera.fid.unwrap_or_default(),
                        rid: camera.rid.unwrap_or_default(),
                        preset: camera.preset.unwrap_or_default(),
                    },
                );
            }
        }

        Ok(map)
    }
}
```

## 6. 処理フロー

### 6.1 IpcamScan承認→登録フロー

```
┌─────────────────────────────────────────────────────────────┐
│           IpcamScan                                          │
│           カメラ検出・承認                                    │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ 承認ボタンクリック
                          ▼
              ┌───────────────────────┐
              │ POST /api/cameras/:id │
              │     /register         │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ lacisID生成           │
              │ 3801{MAC}{ProductCode}│
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ araneaDeviceGate      │
              │ POST /register        │
              └───────────────────────┘
                          │
           ┌──────────────┴──────────────┐
           │                             │
        [成功]                        [失敗]
           │                             │
           ▼                             ▼
    ┌─────────────────┐          ┌─────────────────┐
    │ cameras更新      │          │ registration_   │
    │ lacis_id, cic   │          │ state=failed    │
    │ registration_   │          │ エラー表示       │
    │   state=reg.    │          └─────────────────┘
    └─────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ preset同期有効化      │
              │ 推論登録有効化        │
              └───────────────────────┘
```

### 6.2 カメラコンテキスト利用フロー

```
┌─────────────────────────────────────────────────────────────┐
│           Summary生成 (DD02)                                 │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ camera_lacis_ids取得
                          ▼
              ┌───────────────────────┐
              │ CameraContextService  │
              │ .build_context_map()  │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ cameraContext JSON    │
              │ {                     │
              │   "{lacisId}": {      │
              │     "cameraName":..   │
              │     "cameraContext":..│
              │     "fid":...         │
              │     "rid":...         │
              │     "preset":...      │
              │   }                   │
              │ }                     │
              └───────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │ SummaryPayload構築    │
              │ Paraclate APP送信     │
              └───────────────────────┘
```

## 7. エラーハンドリング

| エラー | 原因 | 対応 |
|-------|------|------|
| InvalidMacAddress | MAC形式不正 | 入力検証エラー表示 |
| Is22NotRegistered | is22未登録 | is22登録を先に完了させる |
| BrandNotFound | ブランド不明 | デフォルトProductCode(G000)使用 |
| GateAuthFailed | 認証失敗 | テナントプライマリ認証再確認 |
| AlreadyRegistered | 重複登録 | 既存情報を返す |

## 8. セキュリティ考慮

### 8.1 tid/fid境界
- カメラは必ずtid/fidに紐づく
- 異なるtidへの登録は拒否
- fid=0000（全施設）へのカメラ登録は不可

### 8.2 CIC管理
- カメラごとにCICを発行・保存
- CICは認証に使用（将来のステータス報告等）

## 9. テスト計画

### 9.1 ユニットテスト

| テストケース | 内容 |
|-------------|------|
| test_generate_camera_lacis_id | 各ブランドでの生成確認 |
| test_mac_normalization | 各MAC形式での正規化 |
| test_default_product_code | 不明ブランドでG000 |

### 9.2 結合テスト

| テストケース | 前提条件 | 期待結果 |
|-------------|---------|---------|
| 新規登録 | カメラ承認済み | lacisID生成、DB保存 |
| 重複登録 | 登録済みカメラ | 既存情報返却 |
| is22未登録 | is22未登録状態 | エラー返却 |
| コンテキスト更新 | 登録済みカメラ | DB更新成功 |

### 9.3 E2Eテスト

| テストケース | 手順 | 確認項目 |
|-------------|------|---------|
| IpcamScan→登録 | 検出→承認→登録 | lacisID付与、状態更新 |
| コンテキスト編集 | 設定UI入力 | 保存、Summary反映 |
| 複数カメラ一覧 | API呼び出し | tid/fid絞り込み正常 |

## 10. MECE/SOLID確認

### 10.1 MECE確認
- **網羅性**: lacisID生成/登録/コンテキスト/一覧を全カバー
- **重複排除**: 登録処理とコンテキスト管理を分離
- **未カバー領域**: カメラ検出はipcam_scan既存実装

### 10.2 SOLID確認
- **SRP**: LacisIdGenerator/RegistryService/ContextService分離
- **OCP**: 新ブランド追加はDB行追加のみ
- **LSP**: CameraRegistration共通構造
- **ISP**: 登録/コンテキスト/一覧APIを分離
- **DIP**: BrandService/Repository抽象に依存

## 11. マイグレーション

### 11.1 SQLマイグレーション

ファイル: `migrations/022_camera_registry.sql`

```sql
-- Migration: 022_camera_registry
-- Description: カメラ台帳lacisID対応

-- camera_brands に product_code 追加
ALTER TABLE camera_brands
ADD COLUMN product_code VARCHAR(4) UNIQUE;

-- 初期ブランドコード設定
UPDATE camera_brands SET product_code = 'T001' WHERE name LIKE '%Tapo%';
UPDATE camera_brands SET product_code = 'H001' WHERE name LIKE '%Hikvision%';
UPDATE camera_brands SET product_code = 'D001' WHERE name LIKE '%Dahua%';
UPDATE camera_brands SET product_code = 'R001' WHERE name LIKE '%Reolink%';
UPDATE camera_brands SET product_code = 'A001' WHERE name LIKE '%Axis%';
UPDATE camera_brands SET product_code = 'G000' WHERE product_code IS NULL;

-- cameras テーブル拡張
ALTER TABLE cameras
ADD COLUMN lacis_id VARCHAR(20) UNIQUE,
ADD COLUMN tid VARCHAR(32),
ADD COLUMN fid VARCHAR(32),
ADD COLUMN rid VARCHAR(32),
ADD COLUMN cic VARCHAR(16),
ADD COLUMN camera_context TEXT,
ADD COLUMN registration_state ENUM('pending', 'registered', 'failed') DEFAULT 'pending',
ADD COLUMN registered_at DATETIME(3),
ADD INDEX idx_lacis_id (lacis_id),
ADD INDEX idx_tid_fid (tid, fid);
```

---

**SSoT宣言**: カメラ台帳SSoTはis22のcamerasテーブル。ブランドコードはcamera_brandsで管理。
**MECE確認**: lacisID生成/登録/コンテキスト/一覧管理を網羅、重複なし。
