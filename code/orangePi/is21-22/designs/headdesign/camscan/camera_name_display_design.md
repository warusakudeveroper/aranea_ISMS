# 登録済みカメラ名表示機能 設計ドキュメント

## 1. 概要

### 1.1 目的
スキャン結果において、登録済みカメラのカメラ名（表示名）を明確に表示し、ユーザーがどのカメラかを即座に識別できるようにする。

### 1.2 対象ファイル
- バックエンド: `src/ipcam_scan/mod.rs`
- フロントエンド: `frontend/src/components/CameraScanModal.tsx`

### 1.3 現状の問題点（Camscan_designers_review.md #2, #6より）
- 登録済みカメラがIPアドレスのみで表示される
- どのカメラかを識別するために別画面を確認する必要がある
- 認証失敗時にカメラ名がわからず問題箇所の特定が困難

---

## 2. 設計

### 2.1 カメラ名取得ロジック

```rust
async fn enrich_with_camera_names(
    &self,
    devices: &mut [ScannedDevice],
    pool: &MySqlPool,
) -> Result<()> {
    // 登録済みカメラをIPアドレスでインデックス化
    let cameras = sqlx::query_as::<_, CameraBasic>(
        "SELECT id, name, ip_address, mac_address FROM cameras"
    )
    .fetch_all(pool)
    .await?;

    let ip_to_camera: HashMap<String, &CameraBasic> = cameras
        .iter()
        .map(|c| (c.ip_address.clone(), c))
        .collect();

    let mac_to_camera: HashMap<String, &CameraBasic> = cameras
        .iter()
        .filter_map(|c| c.mac_address.as_ref().map(|m| (m.to_uppercase(), c)))
        .collect();

    // 各デバイスにカメラ名を付与
    for device in devices.iter_mut() {
        // IP一致でカメラ名取得
        if let Some(camera) = ip_to_camera.get(&device.ip_address) {
            device.registered_camera_name = Some(camera.name.clone());
            device.registered_camera_id = Some(camera.id);
            continue;
        }

        // MAC一致でカメラ名取得（IP変更されたカメラ）
        if let Some(mac) = &device.mac_address {
            if let Some(camera) = mac_to_camera.get(&mac.to_uppercase()) {
                device.registered_camera_name = Some(camera.name.clone());
                device.registered_camera_id = Some(camera.id);
                device.ip_changed = true;  // 迷子カメラフラグ
            }
        }
    }

    Ok(())
}
```

### 2.2 データ構造拡張

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedDevice {
    // 既存フィールド
    pub ip_address: String,
    pub mac_address: Option<String>,
    // ...

    // 新規フィールド
    pub registered_camera_id: Option<i64>,
    pub registered_camera_name: Option<String>,
    pub ip_changed: bool,  // MACで検出されたがIPが違う場合
}
```

### 2.3 UI表示

#### カテゴリA: 登録済みカメラ

```
【ロビーカメラ1】                    ← 太字、大きめフォント
192.168.125.12
✓ 登録済み ✓ RTSP応答あり
D8:07:B6:53:47:3F (TP-LINK)
tp-link Tapo C100 (1.3.9)
```

#### カテゴリC: 認証待ち（登録済みカメラの場合）

```
【ロビーカメラ2】                    ← 太字
192.168.125.14
✓ 登録済み ⚠ 認証失敗
D8:07:B6:53:40:8E (TP-LINK)

🔐 試行済みクレデンシャル:
  × halecam / ***
  × admin / ***
```

#### 迷子カメラ（IP変更検出時）

```
【バックヤードカメラ】                ← 太字
⚠ IPアドレスが変更されています
  旧: 192.168.125.88 → 新: 192.168.125.99
D8:07:B6:53:XX:XX (TP-LINK)
```

### 2.4 表示スタイル

```typescript
const CameraNameDisplay: React.FC<{ name: string; ipChanged?: boolean }> =
    ({ name, ipChanged }) => (
    <div style={{
        fontSize: '16px',
        fontWeight: 'bold',
        marginBottom: '8px',
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
    }}>
        【{name}】
        {ipChanged && (
            <span style={{
                color: '#FF9800',
                fontSize: '12px',
                fontWeight: 'normal'
            }}>
                ⚠ IP変更
            </span>
        )}
    </div>
);
```

---

## 3. テスト計画

### 3.1 単体テスト
1. IP照合によるカメラ名取得テスト
2. MAC照合によるカメラ名取得テスト
3. 未登録デバイスの処理テスト

### 3.2 UIテスト
1. カメラ名表示の確認
2. 太字・フォントサイズの確認
3. 迷子カメラフラグ表示の確認

---

## 4. MECE確認

- [x] 登録済みカメラの全パターンをカバー（IP一致/MAC一致/両方一致）
- [x] 未登録デバイスはカメラ名なしで表示
- [x] 表示スタイルが明確に定義

---

**作成日**: 2026-01-07
**作成者**: Claude Code
**ステータス**: 設計完了・レビュー待ち
