# Access Absorber 機能仕様書

## 1. 概要

### 1.1 目的
カメラブランド/シリーズごとの接続制限を管理し、制限到達時にユーザーへ明確なフィードバックを提供する。

### 1.2 スコープ
- RTSPストリーム接続の排他制御
- 再接続インターバル管理
- ユーザー向けエラーメッセージ生成
- 接続状態の可視化

---

## 2. ブランド別制限定義

### 2.1 制限パラメータ

| パラメータ | 説明 | 単位 |
|-----------|------|------|
| `max_concurrent_streams` | 最大同時ストリーム数 | 接続数 |
| `min_reconnect_interval_ms` | 最小再接続間隔 | ミリ秒 |
| `require_exclusive_lock` | 排他ロック必要 | bool |
| `connection_timeout_ms` | 接続タイムアウト | ミリ秒 |

### 2.2 ブランド別設定値

| ブランド | max_concurrent | min_interval | exclusive_lock | 備考 |
|----------|----------------|--------------|----------------|------|
| `tapo_ptz` | **1** | 0 | **true** | C200/C210等PTZ系。放熱・処理負荷問題 |
| `tapo_fixed` | **2** | 0 | false | C100/C110等固定系。2並列安定 |
| `vigi` | 3 | 0 | false | 3並列まで安定 |
| `nvt` | 2 | **2000** | false | 2秒間隔必須 |
| `hikvision` | 4 | 0 | false | 推定値 |
| `dahua` | 4 | 0 | false | 推定値 |
| `unknown` | 1 | 1000 | true | 安全側デフォルト |

### 2.3 Tapoモデル分類（重要）

**Tapoは「PTZ / 固定」で分類が必要**（ストレステスト結果より）

```
tapo_ptz:   C200, C210, C220, C225, C500, C510W, C520WS（PTZ対応モデル）
tapo_fixed: C100, C110, C120, C125（固定カメラ）
```

#### 分類理由

| 要因 | PTZ (C200等) | 固定 (C110等) |
|------|-------------|---------------|
| モーター制御 | 常時待機 → CPU消費 | なし |
| 放熱設計 | 不足（コミュニティ報告） | 比較的余裕 |
| 実測2並列 | ❌ 0% | ✅ 100% |
| Main+Sub同時 | ❌ 0% | ✅ 100% |

#### ストリームアーキテクチャ（公式情報）

```
Tapo共通:
- ネイティブストリーム: 2本（stream1=HQ, stream2=LQ）
- 公称最大: 4セッション（2ストリーム × 各2接続）
- 実運用安全値: 3以下（公式FAQ「通常3台まで」）
- PTZモデル: 放熱・処理負荷により実質1セッション推奨
```

参考: [Tapo RTSP/ONVIF FAQ](https://www.tapo.com/us/faq/724/)

---

## 3. コアロジック

### 3.1 接続管理構造

```rust
/// カメラごとの接続状態
pub struct CameraConnectionState {
    pub camera_id: String,
    pub family: CameraFamily,
    pub active_streams: Vec<ActiveStream>,
    pub last_disconnect_at: Option<Instant>,
    pub limits: ConnectionLimits,
}

/// アクティブストリーム情報
pub struct ActiveStream {
    pub stream_id: String,
    pub stream_type: StreamType, // Main or Sub
    pub client_id: String,       // 接続元識別子
    pub started_at: Instant,
    pub purpose: StreamPurpose,  // ClickModal, Suggest, Polling, Snapshot
}

/// ストリーム種別
pub enum StreamPurpose {
    ClickModal,     // クリックモーダル再生
    SuggestPlay,    // サジェスト再生
    Polling,        // ポーリング
    Snapshot,       // スナップショット取得
    HealthCheck,    // ヘルスチェック
}
```

### 3.2 接続取得フロー

```rust
impl AccessAbsorber {
    /// ストリーム接続を要求
    pub async fn acquire_stream(
        &self,
        camera_id: &str,
        purpose: StreamPurpose,
        client_id: &str,
    ) -> Result<StreamToken, AbsorberError> {
        let state = self.get_camera_state(camera_id).await?;
        let limits = &state.limits;

        // 1. 同時接続数チェック
        if state.active_streams.len() >= limits.max_concurrent_streams as usize {
            return Err(AbsorberError::ConcurrentLimitReached {
                camera_id: camera_id.to_string(),
                family: state.family.clone(),
                max_streams: limits.max_concurrent_streams,
                current_streams: state.active_streams.clone(),
            });
        }

        // 2. 再接続インターバルチェック
        if let Some(last_disconnect) = state.last_disconnect_at {
            let elapsed = last_disconnect.elapsed();
            let required = Duration::from_millis(limits.min_reconnect_interval_ms as u64);

            if elapsed < required {
                let wait_time = required - elapsed;

                // 短い待機なら待つ
                if wait_time < Duration::from_secs(1) {
                    tokio::time::sleep(wait_time).await;
                } else {
                    return Err(AbsorberError::ReconnectIntervalNotMet {
                        camera_id: camera_id.to_string(),
                        family: state.family.clone(),
                        required_interval_ms: limits.min_reconnect_interval_ms,
                        remaining_ms: wait_time.as_millis() as u32,
                    });
                }
            }
        }

        // 3. 排他ロック取得（必要な場合）
        if limits.require_exclusive_lock {
            self.acquire_exclusive_lock(camera_id).await?;
        }

        // 4. ストリームトークン発行
        let token = self.issue_stream_token(camera_id, purpose, client_id).await?;

        Ok(token)
    }

    /// ストリーム解放
    pub async fn release_stream(&self, token: StreamToken) -> Result<()> {
        let camera_id = &token.camera_id;

        // アクティブストリームから削除
        self.remove_active_stream(camera_id, &token.stream_id).await?;

        // 最終切断時刻を記録
        self.update_last_disconnect(camera_id).await?;

        // 排他ロック解放（必要な場合）
        if self.has_exclusive_lock(camera_id) {
            self.release_exclusive_lock(camera_id).await?;
        }

        Ok(())
    }
}
```

### 3.3 エラー型定義

```rust
#[derive(Debug, Clone)]
pub enum AbsorberError {
    /// 同時接続数上限到達
    ConcurrentLimitReached {
        camera_id: String,
        family: CameraFamily,
        max_streams: u8,
        current_streams: Vec<ActiveStream>,
    },

    /// 再接続インターバル未達
    ReconnectIntervalNotMet {
        camera_id: String,
        family: CameraFamily,
        required_interval_ms: u32,
        remaining_ms: u32,
    },

    /// 排他ロック取得失敗
    ExclusiveLockFailed {
        camera_id: String,
        held_by: String,
    },

    /// タイムアウト
    ConnectionTimeout {
        camera_id: String,
        timeout_ms: u32,
    },
}
```

---

## 4. ユーザーメッセージ生成

### 4.1 メッセージテンプレート

```rust
impl AbsorberError {
    /// ユーザー向けメッセージ生成
    pub fn to_user_message(&self) -> UserMessage {
        match self {
            Self::ConcurrentLimitReached { family, max_streams, current_streams, .. } => {
                let purpose_text = current_streams.first()
                    .map(|s| s.purpose.to_japanese())
                    .unwrap_or("他のプロセス");

                UserMessage {
                    title: "ストリーミング制限".to_string(),
                    message: format!(
                        "このカメラは並列再生数が{}です。現在{}で再生中のためストリーミングできません。",
                        max_streams,
                        purpose_text
                    ),
                    severity: Severity::Warning,
                    show_duration_sec: None, // 手動で閉じるまで表示
                    action_hint: Some("しばらく待ってから再度お試しください".to_string()),
                }
            },

            Self::ReconnectIntervalNotMet { family, required_interval_ms, remaining_ms, .. } => {
                UserMessage {
                    title: "接続間隔制限".to_string(),
                    message: format!(
                        "{}カメラは再接続に{}秒の間隔が必要です。{}秒後に再試行してください。",
                        family.display_name(),
                        required_interval_ms / 1000,
                        (remaining_ms + 999) / 1000
                    ),
                    severity: Severity::Info,
                    show_duration_sec: Some(*remaining_ms / 1000 + 1),
                    action_hint: None,
                }
            },

            _ => UserMessage::generic_error(),
        }
    }
}

impl StreamPurpose {
    pub fn to_japanese(&self) -> &'static str {
        match self {
            Self::ClickModal => "モーダル再生",
            Self::SuggestPlay => "サジェスト再生",
            Self::Polling => "ポーリング",
            Self::Snapshot => "スナップショット取得",
            Self::HealthCheck => "ヘルスチェック",
        }
    }
}
```

---

## 5. 接続状態の可視化

### 5.1 ステータスAPI

```
GET /api/cameras/{camera_id}/stream-status
```

**レスポンス:**
```json
{
  "camera_id": "cam-xxx",
  "family": "tapo",
  "limits": {
    "max_concurrent_streams": 1,
    "min_reconnect_interval_ms": 0,
    "require_exclusive_lock": true
  },
  "current_state": {
    "active_streams": [
      {
        "stream_id": "stream-001",
        "stream_type": "main",
        "purpose": "click_modal",
        "client_id": "browser-session-abc",
        "started_at": "2026-01-17T12:00:00Z",
        "duration_sec": 45
      }
    ],
    "available_slots": 0,
    "can_connect": false,
    "next_available_at": null
  },
  "user_message": {
    "title": "ストリーミング制限",
    "message": "このカメラは並列再生数が1です。現在モーダル再生で再生中のためストリーミングできません。",
    "severity": "warning"
  }
}
```

---

## 6. 優先度制御

### 6.1 ストリーム優先度

| 優先度 | 用途 | 説明 |
|--------|------|------|
| 1 (最高) | ClickModal | ユーザー明示的操作 |
| 2 | Snapshot | スナップショット取得 |
| 3 | Polling | 定期ポーリング |
| 4 | SuggestPlay | サジェスト再生 |
| 5 (最低) | HealthCheck | ヘルスチェック |

### 6.2 プリエンプション（横取り）

高優先度のリクエストが来た場合、低優先度のストリームを切断できる。

```rust
impl AccessAbsorber {
    /// 優先度による横取り判定
    pub fn can_preempt(
        current: &StreamPurpose,
        requested: &StreamPurpose,
    ) -> bool {
        // ClickModalは常に優先
        if matches!(requested, StreamPurpose::ClickModal) {
            return !matches!(current, StreamPurpose::ClickModal);
        }

        // ポーリング/ヘルスチェックはサジェストに譲る
        if matches!(current, StreamPurpose::Polling | StreamPurpose::HealthCheck) {
            return matches!(requested, StreamPurpose::Snapshot | StreamPurpose::SuggestPlay);
        }

        false
    }
}
```

---

## 7. 設定の永続化

### 7.1 デフォルト設定（組み込み）

ソースコードにハードコードされたデフォルト値。

### 7.2 DB設定（オーバーライド可能）

`camera_access_limits` テーブルで施設/テナント単位でオーバーライド可能。

### 7.3 設定の優先順位

```
1. 個別カメラ設定（cameras.access_limit_override）
2. 施設別設定（facility_camera_limits）
3. ブランド別デフォルト（camera_access_limits）
4. ソースコードデフォルト
```

---

## 8. モニタリング

### 8.1 メトリクス

| メトリクス | 説明 |
|-----------|------|
| `absorber_concurrent_limit_hits` | 同時接続制限到達回数 |
| `absorber_interval_limit_hits` | インターバル制限到達回数 |
| `absorber_preemptions` | プリエンプション発生回数 |
| `absorber_average_wait_time_ms` | 平均待機時間 |

### 8.2 ログ

```
[ABSORBER] camera=cam-xxx family=tapo action=acquire purpose=click_modal result=blocked reason=concurrent_limit current=1 max=1
[ABSORBER] camera=cam-yyy family=nvt action=acquire purpose=polling result=delayed wait_ms=1500
```

---

## 9. 実装優先度

| 優先度 | 機能 | 工数 |
|--------|------|------|
| P1 | コアロジック（acquire/release） | 1日 |
| P1 | エラーメッセージ生成 | 0.5日 |
| P2 | UIフィードバック統合 | 1日 |
| P2 | DBスキーマ | 0.5日 |
| P3 | 優先度制御/プリエンプション | 1日 |
| P3 | モニタリング | 0.5日 |

**合計: 約4.5日**
