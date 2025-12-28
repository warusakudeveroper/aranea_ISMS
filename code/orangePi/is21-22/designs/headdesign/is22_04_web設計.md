# is22 web設計

文書番号: is22_04
作成日: 2025-12-29
ステータス: Draft
参照: 基本設計書 Section 12, 特記仕様2 Section A, 特記仕様3 Section 6

---

## 1. 概要

### 1.1 責務
- Web UI提供（shadcn/ui）
- REST API提供
- 署名付きMedia Proxy
- SSE/WebSocketでリアルタイム更新

### 1.2 責務外
- RTSP取得（collector担当）
- 推論処理（dispatcher担当）
- HLSストリーミング（streamer担当）

---

## 2. 技術スタック

### 2.1 フロントエンド
| 技術 | バージョン | 用途 |
|-----|-----------|------|
| React | 18.x | UI |
| Next.js | 14.x | フレームワーク |
| shadcn/ui | latest | UIコンポーネント |
| TailwindCSS | 3.x | スタイリング |
| Lucide | - | アイコン |

### 2.2 バックエンド
| 技術 | バージョン | 用途 |
|-----|-----------|------|
| Rust | 1.92 | APIサーバ |
| axum | 0.7 | Webフレームワーク |
| sqlx | 0.8 | DBアクセス |
| tokio | 1.x | 非同期ランタイム |

---

## 3. URLルーティング

### 3.1 Apache設定
```apache
# /etc/apache2/sites-available/camserver.conf
<VirtualHost *:80>
    ServerName camserver.local

    # 静的ファイル
    DocumentRoot /opt/camserver/web/dist

    # API
    ProxyPass /api http://localhost:8080/api
    ProxyPassReverse /api http://localhost:8080/api

    # Media Proxy
    ProxyPass /media http://localhost:8080/media
    ProxyPassReverse /media http://localhost:8080/media

    # HLS
    ProxyPass /hls http://localhost:8082/hls
    ProxyPassReverse /hls http://localhost:8082/hls

    # WebSocket
    ProxyPass /ws ws://localhost:8080/ws
    ProxyPassReverse /ws ws://localhost:8080/ws
</VirtualHost>

# 管理UI（別ポート）
<VirtualHost *:8081>
    ServerName camserver.local

    # 認証必須
    AuthType Basic
    AuthName "Admin"
    AuthUserFile /etc/apache2/.htpasswd
    Require valid-user

    ProxyPass / http://localhost:8080/admin/
    ProxyPassReverse / http://localhost:8080/admin/
</VirtualHost>
```

---

## 4. API設計

### 4.1 エンドポイント一覧
| メソッド | パス | 用途 |
|---------|------|------|
| GET | /api/cameras | カメラ一覧 |
| GET | /api/cameras/{id} | カメラ詳細 |
| GET | /api/cameras/{id}/latest | 最新フレーム |
| GET | /api/events | イベント検索 |
| GET | /api/events/{id} | イベント詳細 |
| POST | /api/media/link | 署名付きURL発行 |
| GET | /media/frame/{uuid} | 画像配信 |
| GET | /api/admin/cameras | （管理）カメラ一覧 |
| POST | /api/admin/cameras | （管理）カメラ追加 |
| PUT | /api/admin/cameras/{id} | （管理）カメラ更新 |
| DELETE | /api/admin/cameras/{id} | （管理）カメラ削除 |
| GET | /api/admin/settings | （管理）設定取得 |
| PUT | /api/admin/settings | （管理）設定更新 |

### 4.2 レスポンス例

#### GET /api/cameras
```json
{
  "cameras": [
    {
      "camera_id": "cam_2f_19",
      "display_name": "2F 廊下",
      "location_label": "2F corridor",
      "network_zone": "local",
      "enabled": true,
      "latest_frame": {
        "frame_uuid": "6b7f1e2a-5a2b-4f33-8cc8-2a9a39f6d9d1",
        "captured_at": "2025-12-28T18:59:12Z",
        "detected": true,
        "primary_event": "human",
        "severity": 1
      }
    }
  ]
}
```

#### GET /api/events?from=2025-12-28&to=2025-12-29&tags=human
```json
{
  "events": [
    {
      "event_id": 123,
      "camera_id": "cam_2f_19",
      "start_at": "2025-12-28T18:55:00Z",
      "end_at": "2025-12-28T19:10:00Z",
      "primary_event": "human",
      "severity_max": 2,
      "tags_summary": ["count.single", "top_color.red", "behavior.loitering"],
      "best_frame_uuid": "6b7f1e2a-5a2b-4f33-8cc8-2a9a39f6d9d1"
    }
  ],
  "total": 1,
  "page": 1,
  "per_page": 20
}
```

---

## 5. 署名付きMedia Proxy

### 5.1 URL発行API
```
POST /api/media/link
```

#### リクエスト
```json
{
  "frame_uuid": "6b7f1e2a-5a2b-4f33-8cc8-2a9a39f6d9d1",
  "kind": "full",
  "ttl_sec": 900
}
```

#### レスポンス
```json
{
  "url": "http://camserver.local/media/frame/6b7f1e2a-5a2b-4f33-8cc8-2a9a39f6d9d1?exp=1735387152&sig=abc123...",
  "expires_at": "2025-12-28T19:19:12Z"
}
```

### 5.2 署名方式
```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

fn sign_media_url(frame_uuid: &str, drive_file_id: &str, exp_unix: i64, secret: &[u8]) -> String {
    let payload = format!("{}|{}|{}", frame_uuid, drive_file_id, exp_unix);

    let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
    mac.update(payload.as_bytes());
    let result = mac.finalize();

    URL_SAFE_NO_PAD.encode(result.into_bytes())
}
```

### 5.3 画像配信
```
GET /media/frame/{frame_uuid}?exp=1735387152&sig=abc123...
```

- 署名検証
- 有効期限チェック
- frames.drive_file_id取得
- DriveからJPEG取得（またはローカルキャッシュ）
- Cache-Control: private, max-age=60

---

## 6. UI設計

### 6.1 レイアウト
```
+------------------------------------------+
|                 Header                    |
+------------------------------------------+
|                    |                      |
|                    |    Info Panel        |
|   Camera Grid      |    +-----------+     |
|   (80%)            |    | Log       |     |
|                    |    | Search    |     |
|                    |    | Calendar  |     |
|                    |    +-----------+     |
|                    |    | Chat      |     |
|                    |    | (mobes)   |     |
|                    |    +-----------+     |
+------------------------------------------+
```

### 6.2 カメラグリッド
- 表示台数に応じて自動レイアウト（2x2, 3x3, 5x6）
- 各タイル: 最新スナップ + カメラ名 + 状態アイコン
- クリック: モーダル拡大 → RTSP映像（HLS）

### 6.3 異常時自動モーダル
- severity >= 閾値で自動表示
- 設定秒で自動クローズ
- 同一カメラはクールダウン

### 6.4 インフォパネル
- 上段: 最新ログ（no_event省略）、検索、カレンダー
- 下段: AIチャット（mobes連携）

---

## 7. リアルタイム更新

### 7.1 SSE/WebSocket
```typescript
// フロントエンド
const es = new EventSource("/api/events/stream");
es.onmessage = (e) => {
  const data = JSON.parse(e.data);
  updateCameraGrid(data);
};
```

### 7.2 イベント種別
| イベント | 内容 |
|---------|------|
| frame_update | 新規フレーム検知 |
| event_open | イベント開始 |
| event_close | イベント終了 |
| camera_offline | カメラオフライン |

---

## 8. systemdサービス

```ini
# /etc/systemd/system/cam-web.service
[Unit]
Description=camServer Web API
After=network.target mariadb.service

[Service]
Type=simple
User=camserver
WorkingDirectory=/opt/camserver
ExecStart=/opt/camserver/bin/cam_web
Restart=always
RestartSec=5

Environment=DATABASE_URL=mysql://camserver:pass@localhost/camserver
Environment=WEB_PORT=8080
Environment=MEDIA_HMAC_SECRET=your-secret-key

[Install]
WantedBy=multi-user.target
```

---

## 9. テスト観点

### 9.1 API
- [ ] /api/cameras が全カメラを返す
- [ ] /api/events 検索が正常動作
- [ ] 署名付きURL発行・検証

### 9.2 UI
- [ ] カメラグリッド表示
- [ ] モーダル拡大表示
- [ ] 異常時自動モーダル

### 9.3 セキュリティ
- [ ] 署名なしURLで403
- [ ] 期限切れURLで403
- [ ] 管理UIにBasic認証必須
