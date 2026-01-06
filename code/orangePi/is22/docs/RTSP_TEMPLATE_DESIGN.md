# RTSP Template設計書

## 概要

カメラブランド・モデルごとにRTSP URLの構成が異なるため、テンプレート方式で柔軟に対応する機能を実装する。

## 背景・問題

### 現状の問題

1. **go2rtc自動登録の欠落** (BUG-002)
   - カメラ登録時にgo2rtcへのストリーム登録が行われていない
   - 17台中5台のみgo2rtcに登録されている状態
   - SuggestPaneで動画再生不可

2. **RTSP URL構成の多様性** (BUG-003)
   - ブランドごとにURL構成が異なる
   - 現在はハードコードで対応 → スケールしない

### 対応が必要なブランド（例）

| ブランド | メインストリームURL例 | サブストリームURL例 |
|----------|------------------------|----------------------|
| TP-Link Tapo | `rtsp://{user}:{pass}@{ip}:554/stream1` | `rtsp://{user}:{pass}@{ip}:554/stream2` |
| TP-Link VIGI | `rtsp://{user}:{pass}@{ip}:554/stream1` | `rtsp://{user}:{pass}@{ip}:554/stream2` |
| Anker Eufy | `rtsp://{ip}:554/live0` | `rtsp://{ip}:554/live1` |
| BOFUN | `rtsp://{user}:{pass}@{ip}:554/h264_stream` | - |
| Cooau | `rtsp://{user}:{pass}@{ip}:554/1` | `rtsp://{user}:{pass}@{ip}:554/2` |
| WTW | `rtsp://{user}:{pass}@{ip}:554/cam/realmonitor?channel=1&subtype=0` | `...&subtype=1` |
| Hiseeu | `rtsp://{user}:{pass}@{ip}:554/11` | `rtsp://{user}:{pass}@{ip}:554/12` |
| Atom (Wyze) | `rtsp://{user}:{pass}@{ip}:8554/unicast` | - |
| Reolink | `rtsp://{user}:{pass}@{ip}:554/h264Preview_01_main` | `..._sub` |
| Hikvision | `rtsp://{user}:{pass}@{ip}:554/Streaming/Channels/101` | `.../102` |
| Dahua | `rtsp://{user}:{pass}@{ip}:554/cam/realmonitor?channel=1&subtype=0` | `...&subtype=1` |

## 設計

### 1. データモデル

#### RTSPテンプレート定義

```sql
CREATE TABLE rtsp_templates (
    template_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,                    -- "TP-Link Tapo", "Hikvision", etc.
    brand TEXT NOT NULL,                   -- ブランド名
    main_stream_pattern TEXT NOT NULL,     -- "rtsp://{user}:{pass}@{ip}:{port}/stream1"
    sub_stream_pattern TEXT,               -- サブストリーム（オプション）
    default_port INTEGER DEFAULT 554,
    default_user TEXT,                     -- デフォルトユーザー名
    requires_auth BOOLEAN DEFAULT TRUE,
    notes TEXT,                            -- 備考
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

#### camerasテーブル拡張

```sql
ALTER TABLE cameras ADD COLUMN rtsp_template_id TEXT REFERENCES rtsp_templates(template_id);
```

### 2. プリセットテンプレート

システム起動時に以下のプリセットを自動登録：

```json
[
  {
    "template_id": "tplink-tapo",
    "name": "TP-Link Tapo",
    "brand": "TP-Link",
    "main_stream_pattern": "rtsp://{user}:{pass}@{ip}:554/stream1",
    "sub_stream_pattern": "rtsp://{user}:{pass}@{ip}:554/stream2",
    "default_user": "admin",
    "requires_auth": true
  },
  {
    "template_id": "tplink-vigi",
    "name": "TP-Link VIGI",
    "brand": "TP-Link",
    "main_stream_pattern": "rtsp://{user}:{pass}@{ip}:554/stream1",
    "sub_stream_pattern": "rtsp://{user}:{pass}@{ip}:554/stream2",
    "default_user": "admin",
    "requires_auth": true
  },
  {
    "template_id": "hikvision",
    "name": "Hikvision",
    "brand": "Hikvision",
    "main_stream_pattern": "rtsp://{user}:{pass}@{ip}:554/Streaming/Channels/101",
    "sub_stream_pattern": "rtsp://{user}:{pass}@{ip}:554/Streaming/Channels/102",
    "default_user": "admin",
    "requires_auth": true
  },
  {
    "template_id": "anker-eufy",
    "name": "Anker Eufy",
    "brand": "Anker",
    "main_stream_pattern": "rtsp://{ip}:554/live0",
    "sub_stream_pattern": "rtsp://{ip}:554/live1",
    "requires_auth": false
  },
  {
    "template_id": "custom",
    "name": "カスタム",
    "brand": "Other",
    "main_stream_pattern": "",
    "requires_auth": true,
    "notes": "ユーザーが直接URLを入力"
  }
]
```

### 3. UI設計

#### 3.1 設定モーダル（SettingsModal）

新規タブ「RTSPテンプレート」を追加：

```
┌─────────────────────────────────────────────┐
│ 設定                                    [×] │
├─────────────────────────────────────────────┤
│ [一般] [RTSPテンプレート] [詳細設定]        │
├─────────────────────────────────────────────┤
│                                             │
│ プリセットテンプレート:                     │
│ ┌─────────────────────────────────────────┐│
│ │ TP-Link Tapo    stream1/stream2     [編]││
│ │ TP-Link VIGI    stream1/stream2     [編]││
│ │ Hikvision       Channels/101        [編]││
│ │ Anker Eufy      live0/live1         [編]││
│ └─────────────────────────────────────────┘│
│                                             │
│ [+ カスタムテンプレート追加]                │
│                                             │
└─────────────────────────────────────────────┘
```

#### 3.2 カメラ登録時（ScanModal）

検出されたカメラに対してテンプレート選択：

```
┌─────────────────────────────────────────────┐
│ 検出されたカメラ                            │
├─────────────────────────────────────────────┤
│ ☑ 192.168.125.79                           │
│   名前: [Tapo C100        ]                 │
│   テンプレート: [TP-Link Tapo     ▼]        │
│   ユーザー: [halecam  ] パス: [********]    │
│                                             │
│ ☑ 192.168.126.124                          │
│   名前: [VIGI C400HP-4    ]                 │
│   テンプレート: [TP-Link VIGI     ▼]        │
│   ユーザー: [admin    ] パス: [********]    │
└─────────────────────────────────────────────┘
```

#### 3.3 カメラ設定モーダル（CameraDetailModal）

既存カメラのテンプレート変更：

```
┌─────────────────────────────────────────────┐
│ カメラ設定: Tapo C100                   [×] │
├─────────────────────────────────────────────┤
│ RTSPストリーム設定:                         │
│                                             │
│ テンプレート: [TP-Link Tapo     ▼]          │
│                                             │
│ メインストリーム:                           │
│ [rtsp://halecam:****@192.168.125.79/stream1]│
│ [接続テスト]                                │
│                                             │
│ サブストリーム:                             │
│ [rtsp://halecam:****@192.168.125.79/stream2]│
│ [接続テスト]                                │
│                                             │
└─────────────────────────────────────────────┘
```

### 4. go2rtc自動登録フロー

カメラ登録・更新時に以下を実行：

```
1. cameras テーブルに INSERT/UPDATE
2. rtsp_main / rtsp_sub URLを生成（テンプレートから）
3. go2rtc API呼び出し:
   PUT /api/streams/{camera_id}
   {
     "producers": [
       { "url": "{rtsp_main}" }
     ]
   }
4. 登録成功を確認
5. 失敗時はエラーログ + UI通知
```

### 5. 実装タスク

| # | タスク | 優先度 |
|---|--------|--------|
| 1 | rtsp_templates テーブル作成 | 高 |
| 2 | プリセットテンプレート初期データ投入 | 高 |
| 3 | cameras テーブルに rtsp_template_id 追加 | 高 |
| 4 | カメラ登録時のgo2rtc自動登録実装 | **最高** |
| 5 | SettingsModal RTSPテンプレートタブ | 中 |
| 6 | ScanModal テンプレート選択UI | 中 |
| 7 | CameraDetailModal テンプレート変更UI | 中 |
| 8 | 接続テスト機能 | 中 |
| 9 | 既存カメラのgo2rtc一括登録マイグレーション | 高 |

### 6. 影響範囲

- `config_store/types.rs` - Camera構造体拡張
- `config_store/repository.rs` - テンプレート管理
- `ipcam_scan/` - 登録時のgo2rtc連携
- `web_api/routes.rs` - テンプレートAPI
- フロントエンド: SettingsModal, ScanModal, CameraDetailModal

## ONVIF統合設計

### 背景

TP-Link VIGIなどのプロフェッショナルカメラはONVIF対応。
ONVIFプロファイルSのGetStreamUriを使えば、テンプレートに頼らず
**実際のRTSP URLを直接取得**できる。

### ONVIFからRTSP URL取得フロー

```
1. ONVIF GetCapabilities でMedia URLを取得
2. ONVIF GetProfiles でプロファイル一覧取得
3. ONVIF GetStreamUri でRTSP URLを直接取得
   - ProfileToken: MainStream用 / SubStream用
   - StreamSetup: RTP-Unicast / RTSP
```

### VIGIカメラの特性

| 項目 | 値 |
|------|-----|
| ONVIF対応 | ○ (Profile S) |
| メインストリーム | `/stream1` (H.264/H.265) |
| サブストリーム | `/stream2` (H.264) |
| 認証 | Digest認証 |
| デフォルトポート | 554 (RTSP), 80 (HTTP), 8080 (ONVIF) |

### RTSP URL取得の優先順位

```
1. ONVIF GetStreamUri (最も信頼性が高い)
   ↓ 失敗時
2. RTSPテンプレート (ブランド/モデルから推定)
   ↓ 失敗時
3. カスタム手動入力
```

### 実装: ONVIFからのURL取得

```rust
/// ONVIFからRTSP URLを取得（既存のonvif_client活用）
async fn get_rtsp_url_from_onvif(
    ip: &str,
    port: u16,
    username: &str,
    password: &str,
) -> Result<(String, Option<String>), OnvifError> {
    let client = OnvifClient::new(ip, port, username, password)?;

    // プロファイル取得
    let profiles = client.get_profiles().await?;

    // メインストリーム（通常は最初のプロファイル）
    let main_profile = profiles.first()
        .ok_or(OnvifError::NoProfiles)?;
    let main_url = client.get_stream_uri(&main_profile.token).await?;

    // サブストリーム（2番目のプロファイルがあれば）
    let sub_url = if profiles.len() > 1 {
        Some(client.get_stream_uri(&profiles[1].token).await?)
    } else {
        None
    };

    Ok((main_url, sub_url))
}
```

### camerasテーブル拡張（ONVIF対応）

```sql
ALTER TABLE cameras ADD COLUMN onvif_profile_main TEXT;  -- MainStreamのProfileToken
ALTER TABLE cameras ADD COLUMN onvif_profile_sub TEXT;   -- SubStreamのProfileToken
ALTER TABLE cameras ADD COLUMN rtsp_source TEXT DEFAULT 'template';  -- 'onvif' | 'template' | 'manual'
```

### カメラ登録フロー（ONVIF対応版）

```
1. ONVIFスキャンでカメラ検出
2. GetDeviceInformation で Manufacturer/Model 取得
3. ONVIF GetStreamUri でRTSP URL取得を試行
   - 成功 → rtsp_source = 'onvif', URL保存
   - 失敗 → テンプレート推定にフォールバック
4. camerasテーブルに保存
5. go2rtcに自動登録
```

### UI表示

ScanModalでのカメラ登録時：

```
┌─────────────────────────────────────────────┐
│ 検出されたカメラ                            │
├─────────────────────────────────────────────┤
│ ☑ 192.168.126.124 (VIGI C400HP-4)          │
│   🟢 ONVIF検出: stream1/stream2            │
│   ユーザー: [admin    ] パス: [********]    │
│                                             │
│ ☑ 192.168.125.79 (Tapo C100)               │
│   🟡 テンプレート: TP-Link Tapo            │
│   ユーザー: [halecam  ] パス: [********]    │
└─────────────────────────────────────────────┘
```

## 補足

### ONVIF検出時の自動テンプレート推定

ONVIFからURL取得できない場合のフォールバック。
GetDeviceInformation で取得できる Manufacturer/Model から
適切なテンプレートを自動推定。

```rust
fn guess_template(manufacturer: &str, model: &str) -> &str {
    match manufacturer.to_lowercase().as_str() {
        "tp-link" if model.contains("Tapo") => "tplink-tapo",
        "tp-link" if model.contains("VIGI") => "tplink-vigi",
        "hikvision" => "hikvision",
        "dahua" => "dahua",
        _ => "custom"
    }
}
```

### go2rtc登録時のURL形式

VIGIなどONVIF対応カメラの場合、認証情報を含めたURL形式：

```
rtsp://admin:password@192.168.126.124:554/stream1
```

go2rtcは認証情報をURLに含める形式を推奨。
