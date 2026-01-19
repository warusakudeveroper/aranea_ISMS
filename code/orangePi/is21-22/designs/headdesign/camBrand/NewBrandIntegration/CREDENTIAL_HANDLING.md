# クレデンシャル管理詳細仕様

## 1. 概要

カメラブランドごとのRTSP認証クレデンシャル管理について詳細に記述する。

---

## 2. 現状分析

### 2.1 RTSP URL生成の実装

#### types.rs（ハードコード版）
ファイル: `is22/src/ipcam_scan/types.rs:111-132`

```rust
pub fn generate_urls(
    &self,
    ip: &str,
    port: Option<u16>,
    username: &str,
    password: &str,
) -> (String, String) {
    // 常にusername:password@を含む
    let main_url = format!(
        "rtsp://{}:{}@{}:{}{}",
        username, encoded_pass, ip, port, self.main_path
    );
    // ...
}
```

**問題**: usernameが必須

#### service.rs（DB版）
ファイル: `is22/src/camera_brand/service.rs:125-155`

```rust
pub fn generate_rtsp_url(
    template: &RtspTemplateInfo,
    ip: &str,
    port: Option<u16>,
    username: &str,
    password: &str,
) -> (String, Option<String>) {
    let main_url = if username.is_empty() {
        format!("rtsp://{}:{}{}", ip, actual_port, template.main_path)
    } else {
        format!(
            "rtsp://{}:{}@{}:{}{}",
            username, password, ip, actual_port, template.main_path
        )
    };
    // ...
}
```

**対応済み**: 空usernameの場合、認証部分を省略

---

## 3. ブランド別クレデンシャル特性

### 3.1 TP-Link / Tapo / VIGI

| 項目 | 値 |
|------|-----|
| ユーザー名 | 任意（設定で指定） |
| パスワード | 必須 |
| デフォルト | なし（初期設定必須） |
| 特記事項 | Tapoアプリで設定したRTSPパスワードを使用 |

### 3.2 Hikvision

| 項目 | 値 |
|------|-----|
| ユーザー名 | `admin`（初期値） |
| パスワード | 必須（初期設定時に指定） |
| デフォルト | admin / [設定したパスワード] |
| 特記事項 | ファームウェアによってはアクティベーション必須 |

### 3.3 Dahua

| 項目 | 値 |
|------|-----|
| ユーザー名 | `admin`（初期値） |
| パスワード | 必須 |
| デフォルト | admin / [設定したパスワード] |
| 特記事項 | Digest認証使用 |

### 3.4 NVT / JOOAN

| 項目 | 値 |
|------|-----|
| ユーザー名 | 空またはadmin |
| パスワード | 必須（機種により異なる） |
| デフォルト | 空/"admin"/"Admin"/"ADMIN" |
| 特記事項 | **ユーザー名空での接続が可能な場合あり** |

**NVT A6M-U実機確認結果**:
```
RTSP URL成功パターン:
- rtsp://:password@192.168.x.x:554/stream1  ← ユーザー名空
- rtsp://admin:password@192.168.x.x:554/stream1
```

### 3.5 Reolink

| 項目 | 値 |
|------|-----|
| ユーザー名 | `admin`（初期値） |
| パスワード | 必須 |
| デフォルト | admin / [設定したパスワード] |
| 特記事項 | H.265使用時は別パスの場合あり |

---

## 4. 提案: デフォルトクレデンシャル管理

### 4.1 DBスキーマ拡張

```sql
ALTER TABLE camera_brands
ADD COLUMN default_username VARCHAR(100) DEFAULT NULL
    COMMENT 'Default RTSP username (NULL = brand has no default)',
ADD COLUMN default_username_options JSON DEFAULT NULL
    COMMENT 'Alternative usernames to try: ["admin", "Admin", ""]',
ADD COLUMN requires_username BOOLEAN DEFAULT TRUE
    COMMENT 'Whether username is required (FALSE for NVT-like cameras)',
ADD COLUMN auth_type ENUM('basic', 'digest', 'none') DEFAULT 'basic'
    COMMENT 'Preferred authentication type';
```

### 4.2 初期データ

```sql
UPDATE camera_brands SET
    default_username = NULL,
    default_username_options = NULL,
    requires_username = TRUE,
    auth_type = 'basic'
WHERE name = 'TP-LINK';

UPDATE camera_brands SET
    default_username = 'admin',
    default_username_options = '["admin", "Admin"]',
    requires_username = TRUE,
    auth_type = 'digest'
WHERE name = 'Hikvision';

UPDATE camera_brands SET
    default_username = '',
    default_username_options = '["", "admin", "Admin", "ADMIN"]',
    requires_username = FALSE,
    auth_type = 'basic'
WHERE name = 'NVT';
```

### 4.3 クレデンシャル試行ロジック

```rust
pub struct CredentialTrialConfig {
    /// ブランドのデフォルトユーザー名リスト
    pub default_usernames: Vec<String>,
    /// ユーザー名必須か
    pub requires_username: bool,
    /// 施設クレデンシャル
    pub facility_credentials: Vec<TrialCredential>,
}

impl CredentialTrialConfig {
    /// クレデンシャル試行順序を生成
    pub fn generate_trial_order(&self) -> Vec<(String, String)> {
        let mut trials = Vec::new();

        // 1. 施設クレデンシャル（優先度順）
        for cred in &self.facility_credentials {
            trials.push((cred.username.clone(), cred.password.clone()));
        }

        // 2. デフォルトユーザー名 × 施設パスワード
        for username in &self.default_usernames {
            for cred in &self.facility_credentials {
                if &cred.username != username {
                    trials.push((username.clone(), cred.password.clone()));
                }
            }
        }

        // 3. ユーザー名不要の場合、空ユーザー名を試行
        if !self.requires_username {
            for cred in &self.facility_credentials {
                trials.push(("".to_string(), cred.password.clone()));
            }
        }

        trials
    }
}
```

---

## 5. UIフロー提案

### 5.1 スキャン結果画面

```
┌─────────────────────────────────────────────────────┐
│ カメラ検出: 192.168.125.62                          │
│ ブランド: NVT / JOOAN                               │
│ 認証状態: ⚠ 認証必要                               │
├─────────────────────────────────────────────────────┤
│ クレデンシャル設定                                  │
│                                                     │
│ [✓] デフォルトユーザー名を使用                      │
│     └── このブランドは空ユーザー名をサポートします │
│                                                     │
│ ユーザー名: [グレーアウト: (デフォルト: 空)]       │
│                                                     │
│ パスワード: [________________] *                    │
│                                                     │
│ [認証テスト] [登録]                                │
└─────────────────────────────────────────────────────┘
```

### 5.2 認証テスト結果

```
┌─────────────────────────────────────────────────────┐
│ 認証テスト結果                                      │
├─────────────────────────────────────────────────────┤
│ 試行1: ユーザー名「」 → ✅ 成功                    │
│   └── RTSP接続確認済み                             │
│                                                     │
│ 推奨設定:                                          │
│   ユーザー名: (空)                                 │
│   パスワード: ********                             │
│                                                     │
│ [この設定で登録]                                   │
└─────────────────────────────────────────────────────┘
```

---

## 6. API設計

### 6.1 クレデンシャルテスト

```
POST /api/ipcamscan/devices/{device_id}/test-credentials

Request:
{
    "username": "",  // 空も許可
    "password": "mypassword",
    "use_default": true  // ブランドデフォルトを使用
}

Response:
{
    "success": true,
    "tested_credentials": [
        {"username": "", "password": "****", "result": "success"},
        {"username": "admin", "password": "****", "result": "auth_failed"}
    ],
    "recommended": {
        "username": "",
        "password": "mypassword"
    },
    "rtsp_main": "rtsp://:****@192.168.x.x:554/stream1",
    "rtsp_sub": "rtsp://:****@192.168.x.x:554/stream2"
}
```

### 6.2 ブランドデフォルト取得

```
GET /api/settings/camera-brands/{brand_id}/credential-defaults

Response:
{
    "brand_id": 15,
    "brand_name": "NVT",
    "default_username": "",
    "default_username_options": ["", "admin", "Admin", "ADMIN"],
    "requires_username": false,
    "auth_type": "basic"
}
```

---

## 7. セキュリティ考慮事項

1. **パスワードの保存**
   - DBには暗号化して保存
   - ログにパスワードを出力しない

2. **試行制限**
   - 短時間での大量試行を制限（DoS防止）
   - 試行失敗カウント記録

3. **ネットワーク分離**
   - クレデンシャル試行はローカルネットワーク内のみ
   - 外部への送信禁止

---

## 8. 変更履歴

| 日付 | 変更内容 |
|------|----------|
| 2026-01-17 | 初版作成 |
