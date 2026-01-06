# is22 リグレッション問題修正設計書

## 概要

以前動作していた2つの機能がリファクタリング等の影響で動作しなくなっている問題の調査結果と修正計画。

**作成日**: 2026-01-06
**対象バージョン**: Phase 1 実装中
**ステータス**: 調査完了・修正計画策定中

---

## 問題一覧

| ID | 問題 | 深刻度 | 根本原因 |
|----|------|--------|----------|
| BUG-001 | サジェストビュー映像再生不具合 | 高 | lacis_id null化 |
| BUG-002 | 新規サブネットカメラ巡回対象外 | 高 | 静的サブネット管理 |

---

## BUG-001: サジェストビュー映像再生不具合

### 症状
- AI検知イベント発生時、サジェストビューで映像が再生されない
- イベントログには検知が記録される
- CameraGridのスナップショットは更新される

### 根本原因分析

#### データフロー図
```
バックエンド (PollingOrchestrator)
    ↓ WebSocket: EventLogMessage { camera_id, severity, ... }

App.tsx handleEventLog()
    ↓ cameras.find(c => c.camera_id === msg.camera_id)
    ↓ camera が見つからない場合 → lacis_id = null ← ★問題箇所

SuggestPane
    ↓ findCamera(event.lacis_id || "")
    ↓ camera 検索失敗 → return (映像再生スキップ) ← ★症状発生
```

#### 問題コード箇所

**App.tsx (lines 156-205)**
```typescript
const handleEventLog = useCallback((msg: EventLogMessage) => {
  // ★ cameras が古い、または該当カメラが未登録の場合 null
  const camera = cameras?.find(c => c.camera_id === msg.camera_id)
  const lacisId = camera?.lacis_id || null  // ← null になる

  const realtimeLog: DetectionLog = {
    // ...
    lacis_id: lacisId,  // ← null のまま渡される
  }
  setRealtimeEvent(realtimeLog)
}, [fetchLogs, cameras])  // ← cameras 依存
```

**SuggestPane.tsx (lines 79-84)**
```typescript
const addOrExtendCamera = useCallback((event: DetectionLog) => {
  const camera = findCamera(event.lacis_id || "")  // ← "" で検索失敗
  if (!camera?.rtsp_main) {
    console.log("[SuggestPane] No RTSP URL for camera:", event.lacis_id)
    return  // ★ ここで終了、映像再生されない
  }
  // ...
}, [findCamera])
```

#### 原因の可能性
1. **cameras キャッシュが古い**: 30秒ごとにリセット、新規カメラ登録直後は未反映
2. **camera_id 不一致**: EventLogMessage の camera_id と cameras テーブルの不整合
3. **依存配列問題**: cameras 変更時の useCallback 再生成タイミング

### 修正方針

#### 方針A: EventLogMessage に lacis_id を追加（推奨）

**バックエンド修正** (`realtime_hub/mod.rs`):
```rust
pub struct EventLogMessage {
    pub event_id: u64,
    pub camera_id: String,
    pub lacis_id: String,  // ← 追加
    pub primary_event: String,
    pub severity: i32,
    pub timestamp: String,
}
```

**PollingOrchestrator 修正** (`polling_orchestrator/mod.rs`):
```rust
realtime_hub.broadcast(HubMessage::EventLog(EventLogMessage {
    event_id,
    camera_id: camera.camera_id.clone(),
    lacis_id: camera.lacis_id.clone(),  // ← 追加
    primary_event: event.primary_event.clone(),
    severity: event.severity,
    timestamp: captured_at_str.clone(),
})).await;
```

**フロントエンド修正** (`App.tsx`):
```typescript
const handleEventLog = useCallback((msg: EventLogMessage) => {
  const realtimeLog: DetectionLog = {
    // ...
    lacis_id: msg.lacis_id,  // ← 直接使用、cameras 検索不要
  }
  setRealtimeEvent(realtimeLog)
}, [fetchLogs])  // cameras 依存を削除
```

#### 方針B: フロントエンドでのフォールバック

```typescript
const handleEventLog = useCallback((msg: EventLogMessage) => {
  let camera = cameras?.find(c => c.camera_id === msg.camera_id)

  // フォールバック: キャッシュミス時に再取得
  if (!camera) {
    console.warn("[App] Camera not found, refetching...")
    refetchCameras()
    return  // 次のイベントで処理
  }
  // ...
}, [fetchLogs, cameras, refetchCameras])
```

### 推奨修正: 方針A

**理由**:
- SSoT原則: lacis_id はバックエンドが持つ正確な情報
- 依存関係解消: フロントエンドの cameras 依存を排除
- パフォーマンス: 追加API呼び出し不要

---

## BUG-002: 新規サブネットカメラ巡回対象外

### 症状
- 新規サブネットでスキャン → カメラ発見 → カメラタイルに追加される
- しかし巡回対象にならない（既存2サブネットのみ巡回）
- アプリ再起動まで新サブネットは巡回されない

### 根本原因分析

#### 現在のアーキテクチャ
```
アプリ起動
    ↓
PollingOrchestrator::start()
    ↓ cameras キャッシュから既存カメラ取得
    ↓ サブネット抽出: [125.x, 126.x]
    ↓ 各サブネットごとに run_subnet_loop() を spawn
    ↓
125.x ループ ←─┐
126.x ループ ←─┴─ ★ここで固定、新サブネットは追加されない

新規スキャン (192.168.3.x)
    ↓ cameras テーブルに登録
    ↓ ConfigStore キャッシュ更新
    ↓ CameraGrid に表示 ✓
    ↓
192.168.3.x ループ = 存在しない ← ★問題
```

#### 問題コード箇所

**polling_orchestrator/mod.rs (lines 214-288)**
```rust
pub async fn start(&self) {
    let cameras = self.config_store.get_cached_cameras().await;
    let enabled: Vec<_> = cameras
        .into_iter()
        .filter(|c| c.enabled && c.polling_enabled)
        .collect();

    // ★ ここで確定したサブネットは二度と更新されない
    let subnets = Self::group_cameras_by_subnet(&enabled);

    for (subnet, cameras) in subnets {
        // ★ この時点で存在するサブネットのループのみ生成
        tokio::spawn(async move {
            Self::run_subnet_loop(pool, subnet, config_store, ...)
        });
    }
}
```

### 修正方針

#### 方針A: サブネット差分検出と動的ループ生成（推奨）

**新規メソッド追加**:
```rust
impl PollingOrchestrator {
    /// 現在のサブネットループ状態を保持
    active_subnets: Arc<RwLock<HashSet<String>>>,

    /// 定期的にサブネット差分をチェックし、新サブネットループを生成
    pub async fn check_and_spawn_new_subnets(&self) {
        let cameras = self.config_store.get_cached_cameras().await;
        let enabled: Vec<_> = cameras
            .into_iter()
            .filter(|c| c.enabled && c.polling_enabled)
            .collect();

        let current_subnets = Self::group_cameras_by_subnet(&enabled);
        let active = self.active_subnets.read().await;

        for (subnet, cameras) in current_subnets {
            if !active.contains(&subnet) {
                // 新サブネット検出
                info!("New subnet detected: {}, spawning loop", subnet);
                self.spawn_subnet_loop(subnet, cameras).await;
            }
        }
    }
}
```

**統合ポイント**:
1. スキャン完了時に `check_and_spawn_new_subnets()` を呼び出す
2. または定期的（60秒ごと）にチェック

#### 方針B: 簡易対応 - スキャン完了時に全ループ再起動

```rust
/// スキャン完了後に呼び出される
pub async fn restart_all_loops(&self) {
    // 既存ループに停止シグナル送信
    self.stop_signal.store(true, Ordering::SeqCst);

    // 少し待機
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 再起動
    self.stop_signal.store(false, Ordering::SeqCst);
    self.start().await;
}
```

#### 方針C: APIエンドポイント追加

```rust
// POST /api/polling/refresh-subnets
async fn refresh_subnets(State(state): State<AppState>) -> impl IntoResponse {
    state.polling_orchestrator.check_and_spawn_new_subnets().await;
    Json(json!({"ok": true, "message": "Subnet check completed"}))
}
```

### 推奨修正: 方針A + 方針C

**理由**:
- 自動検出: ユーザー操作不要で新サブネット対応
- 既存ループ維持: 再起動による断絶なし
- 明示的制御: 必要時にAPIで強制リフレッシュ可能

---

## 修正計画

### Phase 1: BUG-001 (サジェストビュー) - 優先度高

| 順序 | 作業 | ファイル | 見積もり |
|------|------|----------|----------|
| 1.1 | EventLogMessage に lacis_id 追加 | realtime_hub/mod.rs | 10分 |
| 1.2 | broadcast 時に lacis_id セット | polling_orchestrator/mod.rs | 10分 |
| 1.3 | フロントエンド型定義更新 | useWebSocket.ts | 5分 |
| 1.4 | handleEventLog 修正 | App.tsx | 15分 |
| 1.5 | 単体テスト | - | 20分 |
| 1.6 | E2Eテスト（Chrome） | - | 20分 |

### Phase 2: BUG-002 (サブネット巡回) - 優先度高

| 順序 | 作業 | ファイル | 見積もり |
|------|------|----------|----------|
| 2.1 | active_subnets フィールド追加 | polling_orchestrator/mod.rs | 15分 |
| 2.2 | check_and_spawn_new_subnets 実装 | polling_orchestrator/mod.rs | 30分 |
| 2.3 | スキャン完了時フック追加 | ipcam_scan/mod.rs | 15分 |
| 2.4 | API エンドポイント追加 | web_api/routes.rs | 15分 |
| 2.5 | 単体テスト | - | 20分 |
| 2.6 | E2Eテスト（Chrome） | - | 30分 |

---

## テスト計画

### BUG-001 テスト項目

1. **バックエンド単体テスト**
   - EventLogMessage に lacis_id が含まれることを確認
   - WebSocket broadcast で lacis_id が送信されることを確認

2. **フロントエンド単体テスト**
   - handleEventLog が lacis_id を正しく処理することを確認
   - SuggestPane が lacis_id から camera を正しく検索することを確認

3. **E2Eテスト（Chrome）**
   - AI検知イベント発生 → サジェストビューで映像再生を確認
   - 複数カメラでの連続イベント処理を確認

### BUG-002 テスト項目

1. **バックエンド単体テスト**
   - check_and_spawn_new_subnets が新サブネットを検出することを確認
   - 既存サブネットに対して重複ループが生成されないことを確認

2. **統合テスト**
   - 新サブネットスキャン → カメラ登録 → 巡回開始を確認
   - API /api/polling/refresh-subnets の動作確認

3. **E2Eテスト（Chrome）**
   - 新サブネットでスキャン実行
   - カメラ承認
   - カメラタイルに表示されることを確認
   - 数分後、巡回対象として処理されていることを確認（パフォーマンスログ）

---

## リスクと考慮事項

### BUG-001
- **リスク**: WebSocket メッセージ形式変更による互換性
- **対策**: フロントエンドとバックエンドを同時デプロイ

### BUG-002
- **リスク**: 動的ループ生成によるリソース消費
- **対策**: active_subnets で重複チェック、最大サブネット数制限

---

## MECE確認

- **問題分類**: 2つの独立した問題を特定（相互依存なし）
- **原因分析**: 各問題に対して単一の根本原因を特定
- **修正方針**: 各問題に対して複数の選択肢を提示し、推奨を決定
- **テスト計画**: バックエンド/フロントエンド/E2Eの3層でカバー

## アンアンビギュアス確認

- **BUG-001**: EventLogMessage に lacis_id を追加することで解決（明確）
- **BUG-002**: check_and_spawn_new_subnets メソッド追加で解決（明確）
- **テスト基準**: Chrome実機で映像再生/巡回実行を目視確認（明確）
