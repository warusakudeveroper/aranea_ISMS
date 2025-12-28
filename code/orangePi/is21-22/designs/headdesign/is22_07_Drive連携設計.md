# is22 Drive連携設計

文書番号: is22_07
作成日: 2025-12-29
ステータス: Draft
参照: 基本設計書 Section 10, 特記仕様2 Section C

---

## 1. 概要

### 1.1 責務
- フル画像のGoogle Driveへの保存
- 保持区分（normal/quarantine/case）によるフォルダ分離
- 期限付き削除（TTL運用）
- drive_file_idのDB管理

### 1.2 保存方針
| 条件 | Drive保存 | フォルダ |
|-----|----------|---------|
| detected=false (no_event) | 保存しない | - |
| detected=true, normal | 保存する | camera-archive |
| retention_class=quarantine | 保存する | camera-quarantine |
| retention_class=case | 保存する | case |

---

## 2. フォルダ構成

```
Google Drive/
├── camera-archive/           # 通常保存（14日保持）
│   └── YYYY/
│       └── MM/
│           └── DD/
│               └── {camera_id}/
│                   └── {ts}_{frame_uuid}.jpg
│
├── camera-quarantine/        # 隔離保存（60日保持）
│   └── YYYY/
│       └── MM/
│           └── {camera_id}/
│               └── {ts}_{frame_uuid}.jpg
│
└── case/                     # 案件保存（手動管理）
    └── {case_id}/
        └── {camera_id}/
            └── {ts}_{frame_uuid}.jpg
```

---

## 3. 認証

### 3.1 サービスアカウント方式（推奨）
```json
// /opt/camserver/secrets/drive-service-account.json
{
  "type": "service_account",
  "project_id": "your-project",
  "private_key_id": "...",
  "private_key": "-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----\n",
  "client_email": "camserver@your-project.iam.gserviceaccount.com",
  "client_id": "...",
  "auth_uri": "https://accounts.google.com/o/oauth2/auth",
  "token_uri": "https://oauth2.googleapis.com/token"
}
```

### 3.2 スコープ
```
https://www.googleapis.com/auth/drive.file
```

### 3.3 共有設定
- サービスアカウントにDriveフォルダを共有
- 編集者権限を付与

---

## 4. アップロード処理

### 4.1 Rust実装例
```rust
use google_drive3::{DriveHub, hyper, hyper_rustls};

pub struct DriveUploader {
    hub: DriveHub<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    archive_folder_id: String,
    quarantine_folder_id: String,
}

impl DriveUploader {
    pub async fn upload_full(
        &self,
        camera_id: &str,
        captured_at: DateTime<Utc>,
        retention: RetentionClass,
        jpeg: Vec<u8>,
    ) -> Result<UploadResult> {
        let folder_id = match retention {
            RetentionClass::Normal => &self.archive_folder_id,
            RetentionClass::Quarantine => &self.quarantine_folder_id,
            RetentionClass::Case => panic!("case upload requires case_id"),
        };

        // サブフォルダ作成/取得
        let date_folder = self.ensure_date_folder(folder_id, captured_at).await?;
        let camera_folder = self.ensure_camera_folder(&date_folder, camera_id).await?;

        // ファイル名
        let file_name = format!("{}_{}.jpg",
            captured_at.format("%Y%m%d_%H%M%S"),
            Uuid::new_v4()
        );

        // アップロード
        let file = self.hub.files().create(
            google_drive3::api::File {
                name: Some(file_name),
                parents: Some(vec![camera_folder]),
                ..Default::default()
            }
        )
        .upload(std::io::Cursor::new(jpeg), "image/jpeg".parse().unwrap())
        .await?;

        Ok(UploadResult {
            drive_file_id: file.1.id.unwrap(),
            folder_class: retention,
        })
    }
}
```

### 4.2 リトライ設計
| エラー | 対応 |
|-------|------|
| 5xx | 指数バックオフで3回リトライ |
| 429 Rate Limit | 60秒待機後リトライ |
| 認証エラー | ログ出力、アラート |
| 容量超過 | ログ出力、アラート |

---

## 5. 削除処理

### 5.1 TTLバッチ
```sql
-- 期限切れファイル取得
SELECT d.drive_file_id, d.folder_class
FROM drive_objects d
WHERE d.expire_at IS NOT NULL
  AND d.expire_at < NOW()
LIMIT 100;
```

```rust
pub async fn cleanup_expired_files(pool: &MySqlPool, uploader: &DriveUploader) -> Result<()> {
    let expired = db::get_expired_drive_objects(pool, 100).await?;

    for obj in expired {
        match uploader.delete_file(&obj.drive_file_id).await {
            Ok(_) => {
                db::delete_drive_object(pool, &obj.drive_file_id).await?;
            }
            Err(e) => {
                warn!(file_id=%obj.drive_file_id, err=?e, "drive delete failed");
            }
        }
    }

    Ok(())
}
```

### 5.2 保持期間
| folder_class | 保持期間 | 自動削除 |
|-------------|---------|---------|
| normal | 14日 | ○ |
| quarantine | 60日 | ○ |
| case | 無期限 | × |

---

## 6. DB連携

### 6.1 アップロード時
```sql
-- frames更新
UPDATE frames
SET drive_file_id = ?
WHERE frame_id = ?;

-- drive_objects挿入
INSERT INTO drive_objects (
  drive_file_id, frame_id,
  folder_class, uploaded_at, expire_at,
  file_name, mime_type, size_bytes
) VALUES (?, ?, ?, NOW(3), DATE_ADD(NOW(), INTERVAL ? DAY), ?, 'image/jpeg', ?);
```

### 6.2 削除時
```sql
DELETE FROM drive_objects WHERE drive_file_id = ?;
```

---

## 7. Media Proxy連携

### 7.1 画像取得フロー
```
[ブラウザ] --GET /media/frame/{uuid}?sig=...--> [web]
                                                 ↓
                                            署名検証
                                                 ↓
                                            frames.drive_file_id取得
                                                 ↓
                                            Drive API files.get
                                                 ↓
                                            JPEG stream返却
```

### 7.2 キャッシュ（オプション）
```rust
// メモリキャッシュ（LRU）
let cache: Arc<Mutex<LruCache<String, Vec<u8>>>> = Arc::new(Mutex::new(
    LruCache::new(NonZeroUsize::new(100).unwrap())
));

async fn get_image(drive_file_id: &str) -> Result<Vec<u8>> {
    // キャッシュ確認
    if let Some(cached) = cache.lock().await.get(drive_file_id) {
        return Ok(cached.clone());
    }

    // Drive取得
    let jpeg = drive_client.download(drive_file_id).await?;

    // キャッシュ保存
    cache.lock().await.put(drive_file_id.to_string(), jpeg.clone());

    Ok(jpeg)
}
```

---

## 8. 設定

### 8.1 環境変数
| 変数 | 説明 |
|-----|------|
| DRIVE_SERVICE_ACCOUNT_FILE | サービスアカウントJSONパス |
| DRIVE_ARCHIVE_FOLDER_ID | camera-archiveフォルダID |
| DRIVE_QUARANTINE_FOLDER_ID | camera-quarantineフォルダID |
| DRIVE_NORMAL_EXPIRE_DAYS | normal保持日数 |
| DRIVE_QUARANTINE_EXPIRE_DAYS | quarantine保持日数 |

### 8.2 systemd設定追加
```ini
Environment=DRIVE_SERVICE_ACCOUNT_FILE=/opt/camserver/secrets/drive-service-account.json
Environment=DRIVE_ARCHIVE_FOLDER_ID=1ABCdef...
Environment=DRIVE_QUARANTINE_FOLDER_ID=1XYZabc...
Environment=DRIVE_NORMAL_EXPIRE_DAYS=14
Environment=DRIVE_QUARANTINE_EXPIRE_DAYS=60
```

---

## 9. 監視

### 9.1 メトリクス
| 項目 | 説明 |
|-----|------|
| drive_upload_total | アップロード総数 |
| drive_upload_errors | エラー数 |
| drive_upload_bytes | 転送バイト数 |
| drive_delete_total | 削除総数 |
| drive_quota_used | 使用容量 |

### 9.2 アラート
| 条件 | レベル |
|-----|-------|
| アップロード連続失敗 > 10 | ERROR |
| 容量 > 80% | WARNING |
| 認証エラー | CRITICAL |

---

## 10. テスト観点

### 10.1 正常系
- [ ] normal画像がcamera-archiveに保存
- [ ] quarantine画像がcamera-quarantineに保存
- [ ] drive_file_idがDBに保存
- [ ] expire_atが正しく設定

### 10.2 異常系
- [ ] 認証エラー時のリトライ
- [ ] Rate Limit時のバックオフ
- [ ] 容量超過時の通知

### 10.3 削除
- [ ] TTL超過ファイルが自動削除
- [ ] DBからも削除
- [ ] case画像は削除されない
