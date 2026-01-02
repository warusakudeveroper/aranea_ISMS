# IS22 RTSPポーリング ギャップ分析レポート

**作成日**: 2026-01-02
**対象**: RTSPスナップショットポーリング機能
**調査方式**: 設計書 vs 実装状態 vs ユーザー要件の3点比較

---

## 1. ユーザー要件（今回のミッション）

1. **巡回ポーリング**: 登録カメラを順次ポーリング
2. **画像表示**: CameraGridにスナップショットを表示
3. **アニメーション**: 更新時に新画像が上から下へスライドして被さる
4. **IS21連携**: スナップショットをIS21に送信、推論結果を取得
5. **設定機能**: ヘッダー設定ボタンで以下を設定可能に
   - 巡回速度（ポーリング間隔）
   - アニメーション有無
   - アニメーション方式

---

## 2. 設計書記載事項

### 2.1 is22_02_collector設計.md
- RTSP→JPEG取得（ffmpeg/go2rtc）
- 差分判定による早期リターン
- 推論必要時のみIS21送信
- タイムアウト: ローカル3秒、VPN経由5秒

### 2.2 is22_Camserver仕様案 Section 5.2
- ポーリング周期: 1巡 < 1分（30台カメラ目標）
- フロー: Snapshot → IS21 infer → Normalize → EventLog → Suggest

### 2.3 is22_12_長時間運用設計.md
- サムネキャッシュ: 100枚×30カメラ（LRU）
- リングバッファ設計

---

## 3. 現在の実装状態

### 3.1 バックエンド (Rust)

| コンポーネント | ファイル | 実装状態 |
|--------------|---------|---------|
| PollingOrchestrator | polling_orchestrator/mod.rs | **実装済み** |
| SnapshotService | snapshot_service/mod.rs | **実装済み** |
| AIClient | ai_client/mod.rs | **実装済み** |
| RealtimeHub | realtime_hub/mod.rs | **実装済み** |

**PollingOrchestrator詳細**:
```rust
// 現在のコード (polling_orchestrator/mod.rs:75)
let mut interval = interval(Duration::from_secs(5)); // Check interval
```

- ポーリング間隔: **ハードコード5秒**
- カメラごとに100msの遅延
- スナップショット→IS21推論→イベントログ記録の流れは動作

**問題点**:
1. ポーリング間隔がハードコード（設定不可）
2. スナップショット画像をフロントエンドに**通知しない**
3. EventLogMessage のみブロードキャスト、SnapshotUpdate メッセージ型なし

### 3.2 フロントエンド (React)

| コンポーネント | ファイル | 実装状態 |
|--------------|---------|---------|
| CameraGrid | CameraGrid.tsx | **実装済み** |
| CameraTile | CameraTile.tsx | **実装済み** |
| useApi | hooks/useApi.ts | **実装済み** |

**CameraTile詳細**:
```tsx
// CameraTile.tsx:69
<img
  src={snapshotUrl}
  alt={camera.name}
  className="h-full w-full object-cover"
  loading="lazy"
/>
```

**問題点**:
1. 静的な `<img src>` でスナップショット表示
2. 更新通知を受け取るWebSocket接続なし
3. アニメーション実装なし
4. 設定モーダル未実装

### 3.3 API Endpoints

| エンドポイント | 実装状態 | 用途 |
|--------------|---------|-----|
| GET /api/streams/:camera_id/snapshot | **実装済み** | go2rtc経由でJPEG取得 |
| GET /api/cameras | **実装済み** | カメラ一覧 |
| GET /api/system/status | **実装済み** | システム状態 |

---

## 4. ギャップ一覧

### P0（必須・コア機能）

| GAP ID | 項目 | 現状 | 必要な実装 |
|--------|-----|------|-----------|
| RTSP-001 | スナップショット更新通知 | なし | WebSocket HubMessage追加 |
| RTSP-002 | CameraTile リアルタイム更新 | 静的img | WebSocket受信→画像更新 |
| RTSP-003 | アニメーション | なし | CSS transition実装 |

### P1（重要・設定機能）

| GAP ID | 項目 | 現状 | 必要な実装 |
|--------|-----|------|-----------|
| RTSP-004 | ポーリング間隔設定 | ハードコード5秒 | ConfigStore + API |
| RTSP-005 | 設定モーダルUI | なし | Settings modal作成 |
| RTSP-006 | アニメーション設定 | なし | 有無/方式の設定保存 |

### P2（推奨・最適化）

| GAP ID | 項目 | 現状 | 必要な実装 |
|--------|-----|------|-----------|
| RTSP-007 | 差分判定スキップ | なし | 画像変化なし時のスキップ |
| RTSP-008 | サムネキャッシュ | なし | メモリ/ディスクキャッシュ |

---

## 5. HubMessage拡張設計

### 5.1 新規メッセージ型

```rust
// realtime_hub/mod.rs に追加
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotUpdateMessage {
    pub camera_id: String,
    pub snapshot_url: String,
    pub captured_at: String,
    pub infer_result: Option<InferSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferSummary {
    pub detected: bool,
    pub primary_event: String,
    pub severity: i32,
}

// HubMessage に追加
pub enum HubMessage {
    // 既存...
    SnapshotUpdate(SnapshotUpdateMessage),
}
```

### 5.2 PollingOrchestrator修正

```rust
// poll_camera 関数の末尾に追加
// Always broadcast snapshot (not just on detection)
realtime_hub.broadcast(HubMessage::SnapshotUpdate(SnapshotUpdateMessage {
    camera_id: camera.camera_id.clone(),
    snapshot_url: format!("/api/streams/{}/snapshot?t={}", camera.camera_id, Utc::now().timestamp_millis()),
    captured_at: Utc::now().to_rfc3339(),
    infer_result: if result.detected {
        Some(InferSummary {
            detected: true,
            primary_event: result.primary_event.clone(),
            severity: result.severity,
        })
    } else {
        None
    },
})).await;
```

---

## 6. フロントエンド実装設計

### 6.1 useWebSocket Hook

```typescript
// hooks/useWebSocket.ts
export function useWebSocket(onMessage: (msg: HubMessage) => void) {
  useEffect(() => {
    const ws = new WebSocket(`${WS_BASE_URL}/api/ws`);
    ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      onMessage(msg);
    };
    return () => ws.close();
  }, [onMessage]);
}
```

### 6.2 CameraTile アニメーション

```tsx
// CameraTile.tsx
function CameraTile({ camera, snapshotUrl, onUpdate }) {
  const [currentUrl, setCurrentUrl] = useState(snapshotUrl);
  const [nextUrl, setNextUrl] = useState<string | null>(null);
  const [animating, setAnimating] = useState(false);

  useEffect(() => {
    if (onUpdate && onUpdate !== currentUrl) {
      setNextUrl(onUpdate);
      setAnimating(true);
      // アニメーション完了後に切り替え
      setTimeout(() => {
        setCurrentUrl(onUpdate);
        setNextUrl(null);
        setAnimating(false);
      }, 500); // 500ms transition
    }
  }, [onUpdate]);

  return (
    <div className="relative overflow-hidden">
      <img src={currentUrl} className="..." />
      {animating && nextUrl && (
        <img
          src={nextUrl}
          className="absolute inset-0 animate-slide-down"
        />
      )}
    </div>
  );
}

// tailwind.config.js
// animation: { 'slide-down': 'slideDown 0.5s ease-out forwards' }
// keyframes: { slideDown: { '0%': { transform: 'translateY(-100%)' }, '100%': { transform: 'translateY(0)' } } }
```

### 6.3 Settings Modal

```tsx
// components/SettingsModal.tsx
interface PollingSettings {
  pollingIntervalSec: number;  // 5-60
  animationEnabled: boolean;
  animationStyle: 'slide-down' | 'fade' | 'none';
}

function SettingsModal({ open, onOpenChange, onSave }) {
  const [settings, setSettings] = useState<PollingSettings>(defaultSettings);
  // ... 設定フォーム
}
```

---

## 7. 実装優先順位

### Phase 1: コア機能（RTSP-001, RTSP-002, RTSP-003）
1. RealtimeHub に SnapshotUpdateMessage 追加
2. PollingOrchestrator でブロードキャスト追加
3. WebSocket接続（axum-tungstenite）
4. useWebSocket Hook 作成
5. CameraTile アニメーション実装

### Phase 2: 設定機能（RTSP-004, RTSP-005, RTSP-006）
1. ConfigStore にポーリング設定追加
2. API: GET/PUT /api/settings/polling
3. Settings Modal UI作成
4. localStorage によるクライアント設定保存

### Phase 3: 最適化（RTSP-007, RTSP-008）
1. 差分判定ロジック追加
2. サムネキャッシュ実装

---

## 8. テスト計画

### 8.1 E2E テスト

| テストID | 内容 | 期待結果 |
|---------|------|---------|
| RTSP-E2E-001 | カメラ登録→ポーリング開始 | 画像が5秒ごとに更新 |
| RTSP-E2E-002 | アニメーション動作 | 新画像が上から下へスライド |
| RTSP-E2E-003 | 設定変更→即時反映 | ポーリング間隔が変更される |

### 8.2 負荷テスト

| テストID | 内容 | 期待結果 |
|---------|------|---------|
| RTSP-LOAD-001 | 13台カメラ1時間連続 | メモリリークなし |
| RTSP-LOAD-002 | 30台カメラシミュレーション | 1巡 < 1分 |

---

## 9. IS21連携確認

**疎通確認結果（2026-01-02）**:
```json
// GET http://192.168.3.240:9000/v1/schema
{
  "schema_version": "2025-12-29.1",
  "schema": {
    "primary_events": ["none", "human", "animal", "vehicle"],
    "tags": [
      {"id": "count.single", "severity_default": 1},
      {"id": "count.multiple", "severity_default": 1}
    ]
  }
}
```

**結論**: IS21 API疎通OK。推論リクエスト `/v1/analyze` への画像POST可能。

---

## 10. アンビギュイティ宣言

本分析において以下は**未確定**（実装時に判断必要）:

1. WebSocket接続のエンドポイントパス（`/api/ws` or `/ws`）
2. アニメーション持続時間（300ms? 500ms?）
3. ポーリング間隔の範囲（5-60秒? 1-120秒?）

これらはPhase 1実装時にユーザーと確認の上決定。

---

## 更新履歴

| 日付 | バージョン | 内容 |
|-----|-----------|------|
| 2026-01-02 | 1.0 | 初版作成 |
