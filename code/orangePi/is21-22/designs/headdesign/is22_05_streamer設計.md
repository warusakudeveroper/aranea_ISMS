# is22 streamer設計

文書番号: is22_05
作成日: 2025-12-29
ステータス: Draft
参照: 基本設計書 Section 14, 特記仕様3 Section 7

---

## 1. 概要

### 1.1 責務
- RTSP→HLS変換（オンデマンド）
- WebRTC変換（オプション・go2rtc利用時）
- セッション管理（TTLで自動停止）
- HLSファイル配信

### 1.2 責務外
- スナップショット取得（collector担当）
- 推論処理（dispatcher担当）

---

## 2. アーキテクチャ

### 2.1 オンデマンド方式
```
[ブラウザ] --POST /v1/streams/start--> [streamer]
                                            ↓
                                      ffmpeg spawn
                                            ↓
                                      RTSP → HLS
                                            ↓
[ブラウザ] <--GET /hls/{sid}/index.m3u8-- [streamer]
```

### 2.2 リソース節約
- モーダルを開いた時だけストリーム生成
- TTLで自動停止（デフォルト5分）
- 同時視聴時は同一セッション共有

---

## 3. API設計

### 3.1 エンドポイント
| メソッド | パス | 用途 |
|---------|------|------|
| POST | /v1/streams/start | ストリーム開始 |
| POST | /v1/streams/stop | ストリーム停止 |
| GET | /v1/streams | アクティブセッション一覧 |
| GET | /hls/{session_id}/* | HLSファイル配信 |

### 3.2 POST /v1/streams/start

#### リクエスト
```json
{
  "camera_id": "cam_2f_19"
}
```

#### レスポンス
```json
{
  "session_id": "abc123",
  "hls_url": "/hls/abc123/index.m3u8",
  "expires_at": "2025-12-28T19:05:00Z"
}
```

### 3.3 POST /v1/streams/stop

#### リクエスト
```json
{
  "session_id": "abc123"
}
```

#### レスポンス
```json
{
  "status": "stopped"
}
```

---

## 4. ffmpeg HLS変換

### 4.1 コマンド
```bash
ffmpeg -nostdin -loglevel error \
  -rtsp_transport tcp \
  -i "rtsp://user:pass@camera_ip:554/stream" \
  -c:v copy \
  -an \
  -f hls \
  -hls_time 1 \
  -hls_list_size 3 \
  -hls_flags delete_segments+omit_endlist \
  /var/lib/camserver/hls/{session_id}/index.m3u8
```

### 4.2 パラメータ解説
| パラメータ | 値 | 説明 |
|-----------|-----|------|
| -rtsp_transport tcp | - | TCP転送（安定性向上） |
| -c:v copy | - | 再エンコードなし |
| -an | - | 音声なし |
| -hls_time 1 | 1秒 | セグメント長 |
| -hls_list_size 3 | 3個 | プレイリスト保持数 |
| -hls_flags | - | 古いセグメント削除 |

### 4.3 再エンコード時（コピー不可の場合）
```bash
ffmpeg -nostdin -loglevel error \
  -rtsp_transport tcp \
  -i "rtsp://user:pass@camera_ip:554/stream" \
  -c:v libx264 \
  -preset veryfast \
  -tune zerolatency \
  -an \
  -f hls \
  -hls_time 1 \
  -hls_list_size 3 \
  -hls_flags delete_segments+omit_endlist \
  /var/lib/camserver/hls/{session_id}/index.m3u8
```

---

## 5. セッション管理

### 5.1 セッション構造体
```rust
struct StreamSession {
    session_id: String,
    camera_id: String,
    process: tokio::process::Child,
    started_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    last_access: DateTime<Utc>,
}
```

### 5.2 TTL管理
```rust
async fn session_cleanup_loop(sessions: Arc<Mutex<HashMap<String, StreamSession>>>) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let now = Utc::now();
        let mut sessions = sessions.lock().await;

        let expired: Vec<String> = sessions.iter()
            .filter(|(_, s)| s.expires_at < now || s.last_access < now - Duration::minutes(2))
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired {
            if let Some(mut session) = sessions.remove(&id) {
                let _ = session.process.kill().await;
                // HLSディレクトリ削除
                let _ = tokio::fs::remove_dir_all(format!("/var/lib/camserver/hls/{}", id)).await;
            }
        }
    }
}
```

### 5.3 同一カメラ共有
```rust
async fn start_stream(camera_id: &str, sessions: &mut HashMap<String, StreamSession>) -> String {
    // 既存セッション確認
    for (id, session) in sessions.iter_mut() {
        if session.camera_id == camera_id {
            session.last_access = Utc::now();
            session.expires_at = Utc::now() + Duration::minutes(5);
            return id.clone();
        }
    }

    // 新規セッション作成
    let session_id = Uuid::new_v4().to_string();
    // ... ffmpeg spawn
    session_id
}
```

---

## 6. ディレクトリ構成

```
/var/lib/camserver/hls/
├── {session_id}/
│   ├── index.m3u8
│   ├── segment_0.ts
│   ├── segment_1.ts
│   └── segment_2.ts
└── ...
```

---

## 7. VPNカメラ対応

### 7.1 タイムアウト設定
| カメラ種別 | 接続タイムアウト | 読み取りタイムアウト |
|-----------|----------------|-------------------|
| ローカル | 5秒 | 3秒 |
| VPN経由 | 10秒 | 5秒 |

### 7.2 ffmpegオプション追加
```bash
ffmpeg \
  -rtsp_transport tcp \
  -stimeout 10000000 \  # 10秒（マイクロ秒）
  -i "rtsp://..." \
  ...
```

---

## 8. WebRTC対応（オプション）

### 8.1 go2rtc利用時
```yaml
# /opt/go2rtc/config.yaml
streams:
  cam_2f_19:
    - rtsp://user:pass@camera_ip:554/stream
webrtc:
  candidates:
    - 192.168.125.10:8555
```

### 8.2 メリット・デメリット
| 方式 | 遅延 | 実装難易度 |
|-----|------|-----------|
| HLS | 3-5秒 | 低 |
| WebRTC | < 1秒 | 高（go2rtc依存） |

---

## 9. systemdサービス

```ini
# /etc/systemd/system/cam-streamer.service
[Unit]
Description=camServer Streamer
After=network.target

[Service]
Type=simple
User=camserver
WorkingDirectory=/opt/camserver
ExecStart=/opt/camserver/bin/cam_streamer
Restart=always
RestartSec=5

Environment=DATABASE_URL=mysql://camserver:pass@localhost/camserver
Environment=STREAMER_PORT=8082
Environment=HLS_DIR=/var/lib/camserver/hls
Environment=SESSION_TTL_SEC=300

[Install]
WantedBy=multi-user.target
```

---

## 10. テスト観点

### 10.1 正常系
- [ ] POST /v1/streams/start でHLS URL取得
- [ ] HLSファイルが生成される
- [ ] ブラウザでHLS再生可能

### 10.2 異常系
- [ ] カメラ接続失敗時のエラー応答
- [ ] 無効camera_idで404

### 10.3 リソース管理
- [ ] TTL超過でセッション自動終了
- [ ] ffmpegプロセス正常終了
- [ ] HLSディレクトリ削除

### 10.4 VPNカメラ
- [ ] VPN越しカメラでHLS生成可能
- [ ] 遅延許容範囲内
