# Phase 1: AraneaRegister 実装タスク

対応DD: DD01_AraneaRegister.md
依存関係: なし（基盤）

---

## 概要

is22（Paraclate Server）自身のaraneaDeviceGateへのデバイス登録機能を実装する。
登録によりlacisOath認証に必要なCIC（Client Identification Code）を取得し、後続のParaclate連携を可能にする。

---

## タスク一覧

### T1-1: ProductType定義確認・実装

**状態**: ⬜ NOT_STARTED
**優先度**: P0（ブロッカー）
**見積もり規模**: S

**内容**:
- is22のProductType=022を定数として定義
- Prefix=3, ProductCode=0000を定数化
- LacisID形式: `3022{MAC}{0000}` = 20桁

**成果物**:
- `src/aranea_register/types.rs` の定数定義
```rust
pub const PREFIX: &str = "3";
pub const PRODUCT_TYPE: &str = "022";
pub const PRODUCT_CODE: &str = "0000";
pub const DEVICE_TYPE: &str = "ar-is22CamServer";
pub const TYPE_DOMAIN: &str = "araneaDevice";
```

**検証方法**:
- ユニットテストでLacisID生成確認
- 定数値がmobes2.0バリデーション（`^[34]\d{3}[0-9A-F]{12}\d{4}$`）に準拠

---

### T1-2: データモデル設計・マイグレーション

**状態**: ⬜ NOT_STARTED
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- `aranea_registration`テーブル作成
- `config_store`拡張（aranea.*キー）

**成果物**:
- `migrations/018_aranea_registration.sql`
- config_storeキー設計

**マイグレーションSQL**:
```sql
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

**検証方法**:
- マイグレーション実行成功
- テーブル構造確認

---

### T1-3: registration.rs サービス実装

**状態**: ⬜ NOT_STARTED
**優先度**: P0（ブロッカー）
**見積もり規模**: L

**内容**:
- `AraneaRegisterService`クラス実装
- MAC取得→LacisID生成→araneaDeviceGate呼び出し→永続化

**主要メソッド**:
- `register_device()`
- `get_registration_status()`
- `clear_registration()`

**成果物**:
- `src/aranea_register/mod.rs`
- `src/aranea_register/service.rs`
- `src/aranea_register/repository.rs`
- `src/aranea_register/lacis_id.rs`

**検証方法**:
- 新規登録成功→CIC取得
- 再起動後再登録回避（config_store確認）
- 重複登録エラーハンドリング

---

### T1-4: lacis_oath.rs 認証情報管理

**状態**: ⬜ NOT_STARTED
**優先度**: P0（ブロッカー）
**見積もり規模**: M

**内容**:
- lacisOath認証情報の保存・取得
- TenantPrimaryAuth構造体
- CIC永続化

**serde対応**（CONSISTENCY_CHECK P0-6対応）:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantPrimaryAuth {
    pub lacis_id: String,
    pub user_id: String,
    pub cic: String,
}
```

**成果物**:
- `src/aranea_register/types.rs` 型定義
- config_store連携

**検証方法**:
- JSON camelCase変換テスト
- 認証情報保存・取得テスト

---

### T1-5: blessing.rs（越境アクセス用）

**状態**: ⬜ NOT_STARTED
**優先度**: P1（品質改善）
**見積もり規模**: M

**内容**:
- blessingフィールドサポート
- 越境アクセス時のみ使用（通常はnull）
- mobes2.0側systemBlessings連携準備

**成果物**:
- LacisOath構造体にblessing追加
- 越境判定ロジック（fid/tid境界チェック）

**検証方法**:
- 通常アクセス: blessing=null
- （将来）越境アクセス: blessing指定

---

### T1-6: 監査ログ出力

**状態**: ⬜ NOT_STARTED
**優先度**: P1（品質改善）
**見積もり規模**: S

**内容**:
- 登録成功/失敗のログ出力
- 認証情報変更の記録

**ログ形式**:
```
[INFO] AraneaRegister: Device registered lacis_id=3022AABBCCDDEEFF0000 tid=T2025...
[ERROR] AraneaRegister: Registration failed error=AuthenticationFailed
```

**成果物**:
- tracing::info/error呼び出し追加

**検証方法**:
- ログ出力確認
- 監査要件満足

---

### T1-7: 冗長化対応（is21/is22 2台構成）

**状態**: ⬜ NOT_STARTED
**優先度**: P1（品質改善）
**見積もり規模**: M

**内容**:
- 複数is22インスタンス対応
- lacisIDはMACベースなので自動的に分離
- config_storeは各インスタンスローカル

**成果物**:
- ドキュメント記載
- 冗長化テスト手順

**検証方法**:
- 2台構成で各々独立登録
- CIC競合なし確認

---

## API実装

### 内部API

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/register/device` | POST | デバイス登録実行 |
| `/api/register/status` | GET | 登録状態取得 |
| `/api/register` | DELETE | 登録情報クリア |

**成果物**:
- `src/web_api/register_routes.rs`

---

## 依存する外部API

| API | エンドポイント | 用途 |
|-----|---------------|------|
| araneaDeviceGate | `POST /api/araneaDeviceGate/register` | デバイス登録 |

---

## テストチェックリスト

- [ ] T1-1: LacisID生成テスト（MAC各形式）
- [ ] T1-2: マイグレーション実行確認
- [ ] T1-3: 新規登録E2Eテスト
- [ ] T1-3: 再起動後再登録回避テスト
- [ ] T1-4: JSONシリアライズテスト（camelCase）
- [ ] T1-6: ログ出力確認
- [ ] T1-7: 2台構成テスト

---

## E2E統合テスト（Phase完了時）

| テストID | 内容 | 確認項目 |
|---------|------|---------|
| E1 | デバイス登録→台帳反映 | Phase 1,2 |

---

## 完了条件

1. 全タスク（T1-1〜T1-7）が✅ COMPLETED
2. テストチェックリスト全項目パス
3. E1テスト実行可能（Phase 2完了後）

---

## Issue連携

**Phase Issue**: #114
**親Issue**: #113

全タスクは#114で一括管理。個別タスクのサブIssue化は必要に応じて実施。

---

## 更新履歴

| 日付 | 更新内容 |
|------|---------|
| 2026-01-10 | 初版作成 |
