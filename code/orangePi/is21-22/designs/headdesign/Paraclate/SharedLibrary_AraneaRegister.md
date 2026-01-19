# AraneaRegister 共有ライブラリ設計

## 概要

is21/is22で共通利用するAraneaRegisterモジュールをRust crateとして切り出す設計。

## 背景

- is22: カメラ管理サーバー（ProductType=022）
- is21: AI推論サーバー（ProductType=021）

両者ともaraneaDeviceGateへのデバイス登録機能が必要。コードの重複を避け、メンテナンス性を向上させる。

---

## ディレクトリ構成

```
code/orangePi/is21-22/
├── aranea-register/          # 共有ライブラリ (新規crate)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # モジュールエクスポート
│       ├── types.rs          # 共通型定義
│       ├── service.rs        # AraneaRegisterService
│       ├── repository.rs     # DB永続化（trait + MySQL実装）
│       ├── lacis_id.rs       # LacisID生成
│       └── config.rs         # デバイス固有設定（ProductType等）
│
├── is21/                      # 推論サーバー
│   ├── Cargo.toml             # aranea-register依存追加
│   └── src/
│       └── main.rs
│
└── is22/                      # カメラ管理サーバー（既存）
    ├── Cargo.toml             # aranea-register依存追加
    └── src/
        └── aranea_register/   # 削除 → ライブラリに移行
```

---

## Cargo.toml設定

### aranea-register/Cargo.toml

```toml
[package]
name = "aranea-register"
version = "0.1.0"
edition = "2021"

[dependencies]
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sqlx = { version = "0.7", features = ["runtime-tokio", "mysql", "chrono"] }
thiserror = "1.0"
tracing = "0.1"

[features]
default = []
mysql = []  # MySQL repository実装を有効化
```

### is21/Cargo.toml

```toml
[dependencies]
aranea-register = { path = "../aranea-register" }
```

### is22/Cargo.toml

```toml
[dependencies]
aranea-register = { path = "../aranea-register" }
```

---

## APIインターフェース

### DeviceConfig（デバイス固有設定）

```rust
/// デバイス固有の設定
pub struct DeviceConfig {
    pub prefix: &'static str,        // "3"
    pub product_type: &'static str,  // "022" or "021"
    pub product_code: &'static str,  // "0001"
    pub device_type: &'static str,   // "aranea_ar-is22" or "aranea_ar-is21"
    pub type_domain: &'static str,   // "araneaDevice"
}

impl DeviceConfig {
    /// is22 CamServer用設定
    pub const IS22: DeviceConfig = DeviceConfig {
        prefix: "3",
        product_type: "022",
        product_code: "0001",
        device_type: "aranea_ar-is22",
        type_domain: "araneaDevice",
    };

    /// is21 Inference Server用設定
    pub const IS21: DeviceConfig = DeviceConfig {
        prefix: "3",
        product_type: "021",
        product_code: "0001",
        device_type: "aranea_ar-is21",
        type_domain: "araneaDevice",
    };
}
```

### AraneaRegisterService

```rust
pub struct AraneaRegisterService<R: RegistrationRepository, C: ConfigStore> {
    gate_url: String,
    http_client: Client,
    repository: R,
    config_store: C,
    device_config: DeviceConfig,
}

impl<R: RegistrationRepository, C: ConfigStore> AraneaRegisterService<R, C> {
    pub fn new(
        gate_url: String,
        repository: R,
        config_store: C,
        device_config: DeviceConfig,
    ) -> Self;

    pub async fn register_device(&self, request: RegisterRequest) -> Result<RegisterResult>;
    pub async fn get_registration_status(&self) -> Result<RegistrationStatus>;
    pub async fn clear_registration(&self) -> Result<ClearResult>;
    pub async fn is_registered(&self) -> bool;
    pub async fn get_lacis_id(&self) -> Option<String>;
    pub async fn get_cic(&self) -> Option<String>;
}
```

### Repository Trait

```rust
#[async_trait]
pub trait RegistrationRepository: Send + Sync {
    async fn insert(&self, registration: &AraneaRegistration) -> Result<()>;
    async fn get_latest(&self) -> Result<Option<AraneaRegistration>>;
    async fn delete_all(&self) -> Result<u64>;
    async fn get_registered_at(&self) -> Result<Option<DateTime<Utc>>>;
    async fn get_last_sync_at(&self) -> Result<Option<DateTime<Utc>>>;
    async fn update_last_sync_at(&self, lacis_id: &str) -> Result<()>;
}

/// MySQL実装
pub struct MySqlRegistrationRepository {
    pool: MySqlPool,
}
```

### ConfigStore Trait

```rust
#[async_trait]
pub trait ConfigStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>>;
    async fn set(&self, key: &str, value: &str) -> Result<()>;
    async fn remove(&self, key: &str) -> Result<()>;
}
```

---

## 使用例

### is22での使用

```rust
use aranea_register::{
    AraneaRegisterService, DeviceConfig,
    MySqlRegistrationRepository, // または既存のrepository
};

// サービス初期化
let service = AraneaRegisterService::new(
    "https://asia-northeast1-mobesorder.cloudfunctions.net/araneaDeviceGate".to_string(),
    MySqlRegistrationRepository::new(pool.clone()),
    config_store.clone(),
    DeviceConfig::IS22,  // is22用設定
);

// 登録
let result = service.register_device(request).await?;
```

### is21での使用

```rust
use aranea_register::{
    AraneaRegisterService, DeviceConfig,
    MySqlRegistrationRepository,
};

let service = AraneaRegisterService::new(
    "https://asia-northeast1-mobesorder.cloudfunctions.net/araneaDeviceGate".to_string(),
    MySqlRegistrationRepository::new(pool.clone()),
    config_store.clone(),
    DeviceConfig::IS21,  // is21用設定
);

let result = service.register_device(request).await?;
```

---

## 移行計画

### Phase A: ライブラリ作成（新規）

1. `aranea-register/` crateを作成
2. is22の`src/aranea_register/`からコードを移植
3. `DeviceConfig`でパラメータ化
4. Repository/ConfigStore traitを定義

### Phase B: is22移行

1. is22のCargo.tomlにaranea-register依存追加
2. `src/aranea_register/`を削除
3. ライブラリのサービスに切り替え
4. 動作確認

### Phase C: is21統合

1. is21のCargo.tomlにaranea-register依存追加
2. `DeviceConfig::IS21`で初期化
3. APIエンドポイント追加
4. 動作確認・登録テスト

---

## テスト戦略

### ユニットテスト（ライブラリ内）

- LacisID生成テスト
- 型シリアライズテスト
- モック Repository/ConfigStoreでのサービステスト

### 統合テスト（各プロジェクト）

- is22: 既存のE2Eテスト継続
- is21: 新規登録E2Eテスト追加

---

## スケジュール

| Phase | 内容 | 見積もり |
|-------|------|---------|
| Phase A | ライブラリ作成 | 2-3h |
| Phase B | is22移行 | 1-2h |
| Phase C | is21統合 | 2-3h |
| 合計 | | 5-8h |

---

## 関連ドキュメント

- [Phase1_AraneaRegister.md](./ImplementationTaskProgressManagement/Phase1_AraneaRegister.md)
- [DD01_AraneaRegister.md](./DetailedDesign/DD01_AraneaRegister.md)
- [SCHEMA_DEFINITIONS.md](./SCHEMA_DEFINITIONS.md)

---

## 更新履歴

| 日付 | 内容 |
|------|------|
| 2026-01-10 | 初版作成 |
