# FIX-004 詳細設計: rtsp_sub自動登録（テンプレートベースURL生成）

**作成日**: 2026-01-05 13:30 JST
**親ドキュメント**: [FIX_DESIGN_ffmpeg_leak_and_frontend_boundary.md](./FIX_DESIGN_ffmpeg_leak_and_frontend_boundary.md)
**優先度**: P0（致命的）
**難易度**: 中

---

# 絶対ルール（大原則の宣言）

**以下を必ず復唱してから作業を開始すること**

1. SSoTの大原則を遵守します
2. Solidの原則を意識し特に単一責任、開放閉鎖、置換原則、依存逆転に注意します
3. 必ずMECEを意識し、このコードはMECEであるか、この報告はMECEであるかを報告の中に加えます。
4. アンアンビギュアスな報告回答を義務化しアンアンビギュアスに到達できていない場合はアンアンビギュアスであると宣言できるまで明確な検証を行います。
5. 根拠なき自己解釈で情報に優劣を持たせる差別コード、レイシストコードは絶対の禁止事項です。情報の公平性原則
6. 実施手順と大原則に違反した場合は直ちに作業を止め再度現在位置を確認の上でリカバーしてから再度実装に戻ります
7. チェック、テストのない完了報告はただのやった振りのためこれは行いません
8. 依存関係と既存の実装を尊重し、確認し、車輪の再発明行為は絶対に行いません
9. 作業のフェーズ開始前とフェーズ終了時に必ずこの実施手順と大原則を全文省略なく復唱します
10. 必須参照の改変はゴールを動かすことにつながるため禁止です。
11. このドキュメントを全てのフェーズ前に鑑として再読し基本的な思想や概念的齟齬が存在しないかを確認します。
12. ルール無視はルールを無視した時点からやり直すこと。

---

# 1. 問題定義

## 1.1 確定的事実

| 事実 | 確認方法 | 結果 |
|------|----------|------|
| 全17台のカメラで `rtsp_sub = NULL` | MySQL直接確認 | 全件NULL |
| `approve_device()`が`rtsp_sub`を生成していない | ソースコード分析 | 確認済 |
| main→subフォールバックが機能しない | ログ確認 | subストリーム未設定のため不可能 |

## 1.2 根本原因（コード特定済み）

**ファイル**: `src/ipcam_scan/mod.rs` (Lines 1430-1472)

```rust
// Lines 1430-1441: stream1のみ生成（stream2なし）
let rtsp_url = if let (Some(ref user), Some(ref pass)) = (&use_username, &use_password) {
    let encoded_pass = pass.replace("@", "%40");
    Some(format!(
        "rtsp://{}:{}@{}:554/stream1",  // ← HARDCODED stream1 only!
        user, encoded_pass, device_ip
    ))
} else {
    device.rtsp_uri.clone()
};

// Lines 1444-1449: INSERT文に rtsp_sub が存在しない
let result = sqlx::query(
    "INSERT INTO cameras \
     (camera_id, name, location, ip_address, mac_address, rtsp_main, rtsp_username, rtsp_password, family, \
      source_device_id, fid, lacis_id, enabled, polling_enabled) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 1)"  // ← NO rtsp_sub!
)
```

## 1.3 影響

- Tapoカメラは1-2同時接続制限あり
- メインストリーム（高解像度）がタイムアウトしてもサブストリーム（低解像度）にフォールバック不可
- 結果：スナップショット取得成功率低下

---

# 2. 設計要件

## 2.1 機能要件

| ID | 要件 |
|----|------|
| REQ-001 | `approve_device()`で`rtsp_main`と`rtsp_sub`を両方生成する |
| REQ-002 | CameraFamilyごとにストリームパスパターンをテンプレート化する |
| REQ-003 | 既存カメラのrtsp_sub未設定を検出し補完可能にする |
| REQ-004 | 新規カメラ登録時に自動でrtsp_subを設定する |

## 2.2 非機能要件

| ID | 要件 |
|----|------|
| NFR-001 | 既存カメラの設定を破壊しない（後方互換性） |
| NFR-002 | テンプレートパターンは将来の拡張に対応できる構造にする |

---

# 3. CameraFamily別ストリームパターン

## 3.1 テンプレート定義

| CameraFamily | Main Stream Path | Sub Stream Path | Port | Notes |
|--------------|------------------|-----------------|------|-------|
| Tapo | `/stream1` | `/stream2` | 554 | 確認済み |
| Vigi | `/stream1` | `/stream2` | 554 | Tapo互換（TP-Link製） |
| Hikvision | `/Streaming/Channels/101` | `/Streaming/Channels/102` | 554 | ch1 main/sub |
| Dahua | `/cam/realmonitor?channel=1&subtype=0` | `/cam/realmonitor?channel=1&subtype=1` | 554 | subtype=0:main, 1:sub |
| Axis | `/axis-media/media.amp?videocodec=h264&resolution=1920x1080` | `/axis-media/media.amp?videocodec=h264&resolution=640x480` | 554 | resolution指定 |
| Nest | N/A | N/A | N/A | RTSPサポートなし |
| Other | `/stream1` | `/stream2` | 554 | デフォルト（Tapo互換） |
| Unknown | `/stream1` | `/stream2` | 554 | デフォルト（Tapo互換） |

## 3.2 URL生成式

```
rtsp://{username}:{encoded_password}@{ip_address}:{port}{stream_path}
```

- `{encoded_password}`: `@` → `%40` エンコード必須
- `{port}`: デフォルト 554
- `{stream_path}`: CameraFamily別テンプレート

---

# 4. 実装設計

## 4.1 新規構造体: RtspTemplate

**ファイル**: `src/ipcam_scan/types.rs`（追加）

```rust
/// RTSP URL template per camera family
pub struct RtspTemplate {
    pub main_path: &'static str,
    pub sub_path: &'static str,
    pub default_port: u16,
}

impl RtspTemplate {
    pub fn for_family(family: &CameraFamily) -> Self {
        match family {
            CameraFamily::Tapo => Self {
                main_path: "/stream1",
                sub_path: "/stream2",
                default_port: 554,
            },
            CameraFamily::Vigi => Self {
                main_path: "/stream1",
                sub_path: "/stream2",
                default_port: 554,
            },
            CameraFamily::Hikvision => Self {
                main_path: "/Streaming/Channels/101",
                sub_path: "/Streaming/Channels/102",
                default_port: 554,
            },
            CameraFamily::Dahua => Self {
                main_path: "/cam/realmonitor?channel=1&subtype=0",
                sub_path: "/cam/realmonitor?channel=1&subtype=1",
                default_port: 554,
            },
            CameraFamily::Axis => Self {
                main_path: "/axis-media/media.amp",
                sub_path: "/axis-media/media.amp?resolution=640x480",
                default_port: 554,
            },
            // Nest はRTSPサポートなし
            CameraFamily::Nest => Self {
                main_path: "",
                sub_path: "",
                default_port: 0,
            },
            // デフォルト: Tapo互換
            CameraFamily::Other | CameraFamily::Unknown => Self {
                main_path: "/stream1",
                sub_path: "/stream2",
                default_port: 554,
            },
        }
    }
}
```

## 4.2 URL生成関数

**ファイル**: `src/ipcam_scan/mod.rs`（追加）

```rust
/// Generate RTSP URLs for main and sub streams
fn generate_rtsp_urls(
    family: &CameraFamily,
    ip_address: &str,
    username: Option<&str>,
    password: Option<&str>,
    port: Option<u16>,
) -> (Option<String>, Option<String>) {
    let template = RtspTemplate::for_family(family);

    // Nest or invalid family
    if template.default_port == 0 {
        return (None, None);
    }

    let port = port.unwrap_or(template.default_port);

    let base_url = match (username, password) {
        (Some(user), Some(pass)) => {
            let encoded_pass = pass.replace("@", "%40");
            format!("rtsp://{}:{}@{}:{}", user, encoded_pass, ip_address, port)
        }
        _ => format!("rtsp://{}:{}", ip_address, port),
    };

    let rtsp_main = Some(format!("{}{}", base_url, template.main_path));
    let rtsp_sub = Some(format!("{}{}", base_url, template.sub_path));

    (rtsp_main, rtsp_sub)
}
```

## 4.3 approve_device() 修正

**ファイル**: `src/ipcam_scan/mod.rs` (Lines 1430-1472)

### 修正前（現行コード）:
```rust
let rtsp_url = if let (Some(ref user), Some(ref pass)) = (&use_username, &use_password) {
    let encoded_pass = pass.replace("@", "%40");
    Some(format!(
        "rtsp://{}:{}@{}:554/stream1",
        user, encoded_pass, device_ip
    ))
} else {
    device.rtsp_uri.clone()
};
```

### 修正後:
```rust
// Determine camera family
let family: CameraFamily = device.family.as_deref().unwrap_or("unknown").into();

// Generate both main and sub stream URLs using template
let (rtsp_main, rtsp_sub) = generate_rtsp_urls(
    &family,
    &device_ip,
    use_username.as_deref(),
    use_password.as_deref(),
    device.rtsp_port.map(|p| p as u16),
);
```

## 4.4 INSERT文修正

### 修正前:
```rust
let result = sqlx::query(
    "INSERT INTO cameras \
     (camera_id, name, location, ip_address, mac_address, rtsp_main, rtsp_username, rtsp_password, family, \
      source_device_id, fid, lacis_id, enabled, polling_enabled) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 1)"
)
.bind(&camera_id)
.bind(&device.name)
.bind(&location)
.bind(&device_ip)
.bind(&device.mac_address)
.bind(&rtsp_url)  // ← rtsp_main only
```

### 修正後:
```rust
let result = sqlx::query(
    "INSERT INTO cameras \
     (camera_id, name, location, ip_address, mac_address, rtsp_main, rtsp_sub, rtsp_username, rtsp_password, family, \
      source_device_id, fid, lacis_id, enabled, polling_enabled) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 1)"
)
.bind(&camera_id)
.bind(&device.name)
.bind(&location)
.bind(&device_ip)
.bind(&device.mac_address)
.bind(&rtsp_main)  // ← rtsp_main
.bind(&rtsp_sub)   // ← rtsp_sub (NEW!)
```

---

# 5. 既存カメラへのバックフィル

## 5.1 一括更新SQL（管理者手動実行）

```sql
-- Tapo/Vigi family cameras: /stream1 → /stream2
UPDATE cameras
SET rtsp_sub = REPLACE(rtsp_main, '/stream1', '/stream2')
WHERE family IN ('tapo', 'vigi', 'other', 'unknown')
  AND rtsp_main IS NOT NULL
  AND rtsp_sub IS NULL;

-- Verify update
SELECT camera_id, name, family, rtsp_main, rtsp_sub
FROM cameras
WHERE deleted_at IS NULL;
```

## 5.2 APIエンドポイント追加（オプション）

将来的に`POST /api/cameras/backfill-rtsp-sub`エンドポイントを追加し、管理UIから実行可能にする。

---

# 6. テスト計画

## 6.1 ユニットテスト

### TC-004-1: RtspTemplate生成確認

| 項目 | 内容 |
|------|------|
| **前提条件** | CameraFamily::Tapo |
| **操作** | `RtspTemplate::for_family(&CameraFamily::Tapo)` |
| **期待結果** | main_path="/stream1", sub_path="/stream2", port=554 |

### TC-004-2: URL生成確認

| 項目 | 内容 |
|------|------|
| **前提条件** | family=Tapo, ip=192.168.125.78, user=halecam, pass=hale0083@ |
| **操作** | `generate_rtsp_urls(...)` |
| **期待結果** | main="rtsp://halecam:hale0083%40@192.168.125.78:554/stream1", sub="...stream2" |

### TC-004-3: approve_device()新規カメラ登録

| 項目 | 内容 |
|------|------|
| **前提条件** | スキャンで新規Tapoカメラ検出 |
| **操作** | `approve_device()` 実行 |
| **期待結果** | cameras テーブルに rtsp_main と rtsp_sub の両方が登録される |

## 6.2 統合テスト

### TC-004-4: main→subフォールバック動作

| 項目 | 内容 |
|------|------|
| **前提条件** | rtsp_sub が設定されたカメラ |
| **操作** | メインストリームを10秒タイムアウトさせる |
| **期待結果** | サブストリームへのフォールバック成功 |
| **確認コマンド** | `journalctl -u is22 | grep "trying sub stream"` |

---

# 7. 受け入れ基準

| ID | 基準 | 検証方法 |
|----|------|----------|
| AC-004-1 | 新規カメラ登録時にrtsp_subが自動設定されること | TC-004-3 |
| AC-004-2 | 既存カメラのrtsp_subがバックフィル可能なこと | 5.1のSQL実行 |
| AC-004-3 | main→subフォールバックが動作すること | TC-004-4 |
| AC-004-4 | 全CameraFamilyでテンプレートが定義されていること | TC-004-1 |

---

# 8. 実装チェックリスト

- [ ] 大原則を復唱した
- [ ] `RtspTemplate`構造体を追加した
- [ ] `generate_rtsp_urls()`関数を追加した
- [ ] `approve_device()`を修正した
- [ ] INSERT文に`rtsp_sub`を追加した
- [ ] Rustコンパイルが通ることを確認した
- [ ] TC-004-1〜TC-004-3を実行した
- [ ] 既存カメラのバックフィルSQLを実行した
- [ ] TC-004-4（フォールバック動作）を確認した
- [ ] 大原則を再度復唱した

---

# 9. ロールバック計画

## 9.1 ロールバック条件

- 新規カメラ登録が失敗する場合
- rtsp_sub設定が既存カメラの動作に影響を与える場合

## 9.2 ロールバック手順

1. Gitで変更をリバート: `git revert <commit-hash>`
2. is22サービス再起動: `sudo systemctl restart is22`
3. 必要に応じてrtsp_subをNULLに戻す:
   ```sql
   UPDATE cameras SET rtsp_sub = NULL WHERE rtsp_sub IS NOT NULL;
   ```

---

# 10. 関連ドキュメント

| ドキュメント | 関連性 |
|--------------|--------|
| FIX_DETAIL_001_ffmpeg_kill.md | FIX-001完了後に本修正を適用 |
| TEST_PLAN_integrated.md | 統合テスト計画に追加 |

---

# 更新履歴

| 日時 (JST) | 内容 |
|------------|------|
| 2026-01-05 13:30 | 初版作成 |

---

**作成者**: Claude Code
**ステータス**: 実装待ち
