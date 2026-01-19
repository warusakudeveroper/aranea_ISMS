# 新規ブランド統合 実装タスク一覧

## 概要

新規カメラブランド（NVT/JOOAN等）をIS22システムに統合するために必要な実装タスク。

---

## Phase 1: データベース拡張（優先度: P1）

### T1-1: NVTブランドのマイグレーション作成

**ファイル**: `migrations/030_nvt_brand.sql`

```sql
-- NVTブランド追加
INSERT INTO camera_brands (name, display_name, category, is_builtin) VALUES
('NVT', 'NVT / JOOAN', 'consumer', FALSE)
ON DUPLICATE KEY UPDATE display_name = VALUES(display_name);

-- NVT OUI追加
-- 実機から取得したMACアドレス: DC:62:79:8D:08:EA
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT 'DC:62:79', b.id, 'NVT/JOOAN camera', 20, 'confirmed', 'device test', FALSE
FROM camera_brands b WHERE b.name = 'NVT'
ON DUPLICATE KEY UPDATE description = VALUES(description);

-- NVT RTSPテンプレート
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'NVT Standard', '/stream1', '/stream2', 554, TRUE, 10, FALSE
FROM camera_brands b WHERE b.name = 'NVT'
ON DUPLICATE KEY UPDATE main_path = VALUES(main_path);

-- NVT用Genericパス（フォールバック）
INSERT INTO generic_rtsp_paths (main_path, sub_path, description, priority, is_enabled) VALUES
('/live/ch00_0', '/live/ch00_1', 'NVT/JOOAN alternative', 35, TRUE)
ON DUPLICATE KEY UPDATE description = VALUES(description);
```

**工数**: 0.5日

### T1-2: デフォルトクレデンシャルテーブル追加

**目的**: ブランドごとのデフォルトユーザー名を管理

```sql
-- ブランド別デフォルトクレデンシャル
ALTER TABLE camera_brands
ADD COLUMN default_username VARCHAR(100) DEFAULT NULL COMMENT 'Default RTSP username',
ADD COLUMN default_username_options JSON DEFAULT NULL COMMENT 'Alternative usernames: ["admin", "Admin", ""]';

-- NVTのデフォルト設定
UPDATE camera_brands
SET default_username = '',
    default_username_options = '["", "admin", "Admin", "ADMIN"]'
WHERE name = 'NVT';
```

**工数**: 0.5日

---

## Phase 2: バックエンド実装（優先度: P1）

### T2-1: CameraFamily enum 非推奨化

**現状**: `is22/src/ipcam_scan/types.rs` にハードコード

**対応**:
1. DB `camera_brands` からの動的ロードに切り替え
2. `CameraFamily` enumは互換性のため残す（deprecated）
3. `RtspTemplate::for_family()` は `CameraBrandService` を参照

**影響範囲**:
- `is22/src/ipcam_scan/types.rs`
- `is22/src/ipcam_scan/scanner/mod.rs`
- スキャン結果の表示

**工数**: 1日

### T2-2: クレデンシャル空ユーザー名対応

**ファイル**: `is22/src/camera_brand/service.rs`

```rust
impl CameraBrandService {
    /// Get default usernames for a brand
    pub async fn get_default_usernames(&self, brand_id: i32) -> Vec<String> {
        let brand = self.repo.get_brand(brand_id).await.ok().flatten();
        match brand {
            Some(b) => {
                // Parse default_username_options JSON
                if let Some(options) = b.default_username_options {
                    serde_json::from_str(&options).unwrap_or_default()
                } else if let Some(default) = b.default_username {
                    vec![default]
                } else {
                    vec![]
                }
            }
            None => vec![],
        }
    }
}
```

**工数**: 0.5日

### T2-3: スキャン時のクレデンシャル試行拡張

**ファイル**: `is22/src/ipcam_scan/scanner/credential_trial.rs`（新規作成）

ブランド検出後、そのブランドのデフォルトユーザー名リストを自動試行:

```rust
pub async fn trial_credentials_for_brand(
    brand_id: i32,
    ip: &str,
    brand_service: &CameraBrandService,
    known_passwords: &[String],
) -> Option<(String, String)> {
    let default_usernames = brand_service.get_default_usernames(brand_id).await;

    for username in default_usernames {
        for password in known_passwords {
            if try_rtsp_auth(ip, &username, password).await.is_ok() {
                return Some((username, password.clone()));
            }
        }
    }
    None
}
```

**工数**: 1日

---

## Phase 3: フロントエンド拡張（優先度: P2）

### T3-1: クレデンシャル入力UIの拡張

**ファイル**: `frontend/src/components/ScanModal.tsx`

現状:
```tsx
<TextField label="ユーザー名" required />
<TextField label="パスワード" required />
```

変更後:
```tsx
<FormControlLabel
    control={<Checkbox checked={useDefaultUsername} />}
    label="デフォルトユーザー名を使用（空またはadmin）"
/>
{!useDefaultUsername && (
    <TextField label="ユーザー名" />
)}
<TextField label="パスワード" required />
```

**工数**: 0.5日

### T3-2: ブランド設定UI追加

**ファイル**: `frontend/src/components/SettingsModal.tsx`

設定画面に「カメラブランド管理」セクションを追加:
- ブランド一覧表示
- OUIエントリ追加
- RTSPテンプレート編集
- デフォルトユーザー名設定

**工数**: 1日

---

## Phase 4: Access Absorber実装（優先度: P3）

### T4-1: 接続制限テーブル追加

**ファイル**: `migrations/031_access_absorber.sql`

参照: `camBrand/accessAbsorber/DB_SCHEMA.md`

```sql
CREATE TABLE camera_access_limits (
    id INT AUTO_INCREMENT PRIMARY KEY,
    brand_id INT NOT NULL,
    model_pattern VARCHAR(100) DEFAULT NULL COMMENT 'Regex for model matching',
    max_concurrent_streams INT NOT NULL DEFAULT 2,
    min_reconnect_interval_ms INT NOT NULL DEFAULT 0,
    require_exclusive_lock BOOLEAN NOT NULL DEFAULT FALSE,
    connection_timeout_ms INT NOT NULL DEFAULT 10000,
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    FOREIGN KEY (brand_id) REFERENCES camera_brands(id) ON DELETE CASCADE
);

-- 初期データ
INSERT INTO camera_access_limits (brand_id, model_pattern, max_concurrent_streams, min_reconnect_interval_ms, require_exclusive_lock, is_builtin)
SELECT b.id, 'C2[0-9]{2}', 1, 0, TRUE, TRUE
FROM camera_brands b WHERE b.name = 'TP-LINK'; -- Tapo PTZ

INSERT INTO camera_access_limits (brand_id, model_pattern, max_concurrent_streams, min_reconnect_interval_ms, require_exclusive_lock, is_builtin)
SELECT b.id, 'C1[0-9]{2}', 2, 0, FALSE, TRUE
FROM camera_brands b WHERE b.name = 'TP-LINK'; -- Tapo Fixed

INSERT INTO camera_access_limits (brand_id, model_pattern, max_concurrent_streams, min_reconnect_interval_ms, require_exclusive_lock, is_builtin)
SELECT b.id, NULL, 2, 2000, FALSE, TRUE
FROM camera_brands b WHERE b.name = 'NVT';
```

**工数**: 0.5日

### T4-2: AccessAbsorberサービス実装

**ファイル**: `is22/src/access_absorber/mod.rs`（新規）

参照: `camBrand/accessAbsorber/SPECIFICATION.md`

**工数**: 2日

---

## Phase 5: テスト・検証（優先度: P1）

### T5-1: NVT実機テスト

**手順**:
1. NVT/JOOANカメラをネットワーク接続
2. ARPスキャンでMAC検出確認
3. OUIルックアップでブランド判定確認
4. RTSPテンプレートで接続確認
5. クレデンシャル試行確認

**使用スクリプト**:
- `NewBrandValidation/scripts/onvif_discovery.sh`
- `NewBrandValidation/scripts/rtsp_path_discovery.sh`
- `NewBrandValidation/scripts/camera_stress_test.sh`

**工数**: 0.5日

### T5-2: 回帰テスト

既存のTapo/VIGI/Hikvision等が影響を受けないことを確認。

**工数**: 0.5日

---

## 工数サマリー

| Phase | 内容 | 工数 |
|-------|------|------|
| P1 | DB拡張 | 1日 |
| P2 | バックエンド | 2.5日 |
| P3 | フロントエンド | 1.5日 |
| P4 | Access Absorber | 2.5日 |
| P5 | テスト | 1日 |
| **合計** | | **8.5日** |

---

## 優先順位付き実装順序

1. **即座に対応（P1）**
   - T1-1: NVTブランドのマイグレーション
   - T5-1: 実機テスト

2. **短期対応（P1-P2）**
   - T1-2: デフォルトクレデンシャルテーブル
   - T2-2: 空ユーザー名対応
   - T3-1: UI拡張

3. **中期対応（P2-P3）**
   - T2-1: CameraFamily非推奨化
   - T2-3: クレデンシャル試行拡張
   - T3-2: 設定UI追加

4. **長期対応（P3）**
   - T4-1, T4-2: Access Absorber実装
