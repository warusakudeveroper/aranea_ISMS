# StrayChildScan（迷子カメラ検出）設計ドキュメント

## 1. 概要

### 1.1 目的
DHCPによりIPアドレスが変更された登録済みカメラ（迷子カメラ）を、MACアドレスを基に自動検出し、ユーザーに通知する機能を実装する。

### 1.2 対象ファイル
- バックエンド: `src/ipcam_scan/mod.rs` (新規ロジック追加)
- データベース: `cameras`テーブル, `scan_devices`テーブル

### 1.3 現状の問題点（Camscan_designers_review.md #5より）
- StrayChildScan機能は設計されていたが、**現在未実装**
- IPが変わったカメラは「登録済みだけど応答なし」として放置される
- 新しいIPで検出されても、登録済みカメラとの関連が認識されない

---

## 2. 設計

### 2.1 迷子カメラ検出ロジック

```
スキャン実行
    │
    ▼
ARP/ポートスキャンでデバイス発見
    │
    ▼
各デバイスのMACアドレスを取得
    │
    ▼
登録済みカメラのMACアドレスと照合
    │
    ├── MAC一致 かつ IP不一致 → 迷子カメラ検出！
    │
    └── MAC一致 かつ IP一致 → 正常
```

### 2.2 データ構造

```rust
#[derive(Debug, Clone, Serialize)]
pub struct StrayChildResult {
    /// 登録済みカメラID
    pub camera_id: i64,
    /// 登録済みカメラ名
    pub camera_name: String,
    /// 登録済みIPアドレス（旧）
    pub registered_ip: String,
    /// 検出されたIPアドレス（新）
    pub detected_ip: String,
    /// MACアドレス（一致）
    pub mac_address: String,
    /// 検出日時
    pub detected_at: DateTime<Utc>,
}
```

### 2.3 検出フロー

```rust
async fn check_stray_children(
    &self,
    scanned_devices: &[ScannedDevice],
    registered_cameras: &[Camera],
) -> Vec<StrayChildResult> {
    let mut results = Vec::new();

    // 登録済みカメラのMAC→カメラマッピング作成
    let mac_to_camera: HashMap<String, &Camera> = registered_cameras
        .iter()
        .filter_map(|c| c.mac_address.as_ref().map(|mac| (mac.to_uppercase(), c)))
        .collect();

    // スキャン結果をMACで照合
    for device in scanned_devices {
        if let Some(mac) = &device.mac_address {
            let mac_upper = mac.to_uppercase();
            if let Some(camera) = mac_to_camera.get(&mac_upper) {
                // MAC一致 → IP比較
                if camera.ip_address != device.ip_address {
                    // 迷子カメラ検出！
                    results.push(StrayChildResult {
                        camera_id: camera.id,
                        camera_name: camera.name.clone(),
                        registered_ip: camera.ip_address.clone(),
                        detected_ip: device.ip_address.clone(),
                        mac_address: mac_upper,
                        detected_at: Utc::now(),
                    });
                }
            }
        }
    }

    results
}
```

### 2.4 UI表示

```
┌─────────────────────────────────────────────────┐
│ ⚠ 迷子カメラを検出しました                      │
├─────────────────────────────────────────────────┤
│ 以下のカメラのIPアドレスが変更されています:     │
│                                                 │
│ 【ロビーカメラ1】                               │
│   登録IP: 192.168.125.12 → 検出IP: 192.168.125.99│
│   MAC: D8:07:B6:53:47:3F                        │
│                                                 │
│   [IPアドレスを更新] [無視]                      │
└─────────────────────────────────────────────────┘
```

### 2.5 IPアドレス更新API

```
POST /api/cameras/{camera_id}/update-ip
{
    "new_ip": "192.168.125.99",
    "reason": "stray_child_detected"
}

Response:
{
    "success": true,
    "data": {
        "camera_id": 123,
        "old_ip": "192.168.125.12",
        "new_ip": "192.168.125.99",
        "message": "IPアドレスを更新しました。巡回対象として再登録されます。"
    }
}
```

---

## 3. 実装優先度

| 項目 | 優先度 |
|------|--------|
| MAC照合ロジック | 高 |
| スキャン時の自動検出 | 高 |
| UI通知表示 | 中 |
| IPアドレス自動更新 | 低（ユーザー確認必須） |

---

## 4. テスト計画

### 4.1 単体テスト
1. MAC照合ロジックテスト
2. StrayChildResult生成テスト

### 4.2 結合テスト
1. スキャン→迷子検出→通知の流れ
2. IPアドレス更新APIテスト

### 4.3 UIテスト
1. 迷子カメラ通知表示確認
2. IPアドレス更新操作確認

---

## 5. MECE確認

- [x] 迷子カメラの定義が明確（MAC一致 かつ IP不一致）
- [x] 検出フローが網羅的
- [x] ユーザーアクション（更新/無視）が定義

---

**作成日**: 2026-01-07
**作成者**: Claude Code
**ステータス**: 設計完了・レビュー待ち（現在未実装）
