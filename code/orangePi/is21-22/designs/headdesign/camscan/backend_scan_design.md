# バックエンドスキャン実行化 設計ドキュメント

## 1. 概要

### 1.1 目的
カメラスキャンプロセスをフロントエンド実行からバックエンド実行に移行し、以下を実現する：
- モーダルを閉じてもスキャンが継続される
- 単一実行の保証（並列実行防止）
- スキャン進捗のリアルタイム取得
- 早期終了機能の提供

### 1.2 対象ファイル
- バックエンド:
  - `src/ipcam_scan/mod.rs` (スキャンオーケストレーター)
  - `src/web_api/routes.rs` (API追加)
- フロントエンド:
  - `frontend/src/components/CameraScanModal.tsx`

### 1.3 現状の問題点（Camscan_designers_review.mdより）
1. スキャンがフロントエンド実行となっている
2. モーダルを閉じるとスキャンが終了する
3. 複数ブラウザ/タブからの並列実行が可能
4. スキャン中の早期終了機能がない

---

## 2. アーキテクチャ設計

### 2.1 システム構成図

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend                              │
│  ┌─────────────────┐     ┌─────────────────────────────┐   │
│  │ CameraScanModal │────▶│ WebSocket/SSE Connection    │   │
│  │   - 開始ボタン    │     │ - 進捗受信                  │   │
│  │   - 早期終了      │     │ - 結果受信                  │   │
│  │   - 結果表示      │     └─────────────────────────────┘   │
│  └─────────────────┘                                        │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                        Backend                               │
│  ┌─────────────────┐     ┌─────────────────────────────┐   │
│  │   Web API       │     │   ScanOrchestrator          │   │
│  │ POST /scan/start│────▶│   - 単一インスタンス保証     │   │
│  │ POST /scan/stop │     │   - 進捗ブロードキャスト     │   │
│  │ GET /scan/status│     │   - 結果保存               │   │
│  └─────────────────┘     └─────────────────────────────┘   │
│                                   │                         │
│                                   ▼                         │
│                          ┌─────────────────────────────┐   │
│                          │   RealtimeHub (既存)        │   │
│                          │   - WebSocket/SSE配信       │   │
│                          └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 状態遷移図

```
    ┌─────────┐
    │  Idle   │◀────────────────────────────────┐
    └────┬────┘                                  │
         │ start()                               │
         ▼                                       │
    ┌─────────┐   stop()    ┌──────────┐        │
    │ Running │───────────▶ │ Stopping │        │
    └────┬────┘             └────┬─────┘        │
         │ complete                │ stopped     │
         ▼                         ▼             │
    ┌─────────┐             ┌──────────┐        │
    │Complete │─────────────│ Stopped  │────────┘
    └─────────┘  (自動遷移)  └──────────┘
```

---

## 3. API設計

### 3.1 スキャン開始 API

```
POST /api/scan/start
Content-Type: application/json

Request:
{
    "subnets": ["192.168.125.0/24", "192.168.126.0/24"],
    "brute_force_mode": false
}

Response (200 OK):
{
    "success": true,
    "data": {
        "scan_id": "scan_20260107_142800_abc123",
        "status": "running",
        "started_at": "2026-01-07T14:28:00Z",
        "target_subnets": ["192.168.125.0/24", "192.168.126.0/24"],
        "estimated_total_hosts": 508
    }
}

Response (409 Conflict - スキャン実行中):
{
    "success": false,
    "error": "SCAN_ALREADY_RUNNING",
    "message": "スキャンが既に実行中です",
    "data": {
        "scan_id": "scan_20260107_140000_xyz789",
        "started_at": "2026-01-07T14:00:00Z",
        "progress_percent": 45
    }
}
```

### 3.2 スキャン停止 API

```
POST /api/scan/stop
Content-Type: application/json

Request:
{
    "scan_id": "scan_20260107_142800_abc123",
    "reason": "user_requested"  // optional
}

Response (200 OK):
{
    "success": true,
    "data": {
        "scan_id": "scan_20260107_142800_abc123",
        "status": "stopping",
        "message": "スキャン停止処理を開始しました。結果は部分的に保存されます。"
    }
}
```

### 3.3 スキャン状態取得 API

```
GET /api/scan/status

Response (200 OK - スキャン実行中):
{
    "success": true,
    "data": {
        "scan_id": "scan_20260107_142800_abc123",
        "status": "running",
        "started_at": "2026-01-07T14:28:00Z",
        "progress": {
            "percent": 45,
            "current_stage": "port_scan",
            "current_stage_name": "ポートスキャン",
            "hosts_scanned": 127,
            "hosts_total": 254,
            "stages_completed": ["arp_scan"],
            "stages_remaining": ["protocol_probe", "auth_try"]
        },
        "interim_results": {
            "hosts_found": 56,
            "cameras_detected": 12,
            "auth_successful": 4
        }
    }
}

Response (200 OK - スキャン未実行):
{
    "success": true,
    "data": {
        "status": "idle",
        "last_scan": {
            "scan_id": "scan_20260107_120000_def456",
            "completed_at": "2026-01-07T12:15:30Z",
            "results_available": true
        }
    }
}
```

### 3.4 スキャン結果取得 API（既存拡張）

```
GET /api/scan/results?scan_id=scan_20260107_142800_abc123

Response (200 OK):
{
    "success": true,
    "data": {
        "scan_id": "scan_20260107_142800_abc123",
        "status": "completed",  // or "stopped"
        "started_at": "2026-01-07T14:28:00Z",
        "completed_at": "2026-01-07T14:45:30Z",
        "duration_seconds": 1050,
        "target_subnets": ["192.168.125.0/24"],
        "devices": [...]  // 既存のデバイス配列
    }
}
```

---

## 4. WebSocket/SSE進捗配信

### 4.1 イベント種別

```typescript
// 進捗イベント
interface ScanProgressEvent {
    type: "scan_progress";
    scan_id: string;
    progress: {
        percent: number;
        current_stage: string;
        current_stage_name: string;
        hosts_scanned: number;
        hosts_total: number;
    };
    timestamp: string;
}

// ログイベント
interface ScanLogEvent {
    type: "scan_log";
    scan_id: string;
    level: "info" | "warn" | "error";
    ip?: string;
    message: string;
    timestamp: string;
}

// ステータス変更イベント
interface ScanStatusEvent {
    type: "scan_status";
    scan_id: string;
    status: "running" | "stopping" | "completed" | "stopped" | "error";
    message?: string;
    timestamp: string;
}

// 完了イベント
interface ScanCompleteEvent {
    type: "scan_complete";
    scan_id: string;
    summary: {
        hosts_found: number;
        cameras_detected: number;
        auth_successful: number;
        duration_seconds: number;
    };
    timestamp: string;
}
```

### 4.2 既存RealtimeHubへの統合

`RealtimeHub`の既存機能を活用し、新しいイベントタイプ`scan_progress`、`scan_log`、`scan_status`、`scan_complete`を追加。

---

## 5. バックエンド実装

### 5.1 ScanOrchestrator構造

```rust
pub struct ScanOrchestrator {
    /// 現在のスキャン状態
    state: Arc<RwLock<ScanState>>,
    /// スキャン実行用のJoinHandle
    scan_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// 停止シグナル
    stop_signal: Arc<RwLock<bool>>,
    /// DB pool
    pool: MySqlPool,
    /// RealtimeHub for broadcasting
    realtime: Arc<RealtimeHub>,
}

#[derive(Debug, Clone)]
pub enum ScanState {
    Idle {
        last_scan_id: Option<String>,
        last_completed_at: Option<DateTime<Utc>>,
    },
    Running {
        scan_id: String,
        started_at: DateTime<Utc>,
        progress: ScanProgress,
    },
    Stopping {
        scan_id: String,
        requested_at: DateTime<Utc>,
    },
}

impl ScanOrchestrator {
    /// スキャン開始
    pub async fn start(&self, request: StartScanRequest) -> Result<StartScanResponse, ScanError>;

    /// スキャン停止
    pub async fn stop(&self, scan_id: &str) -> Result<StopScanResponse, ScanError>;

    /// 状態取得
    pub async fn status(&self) -> ScanStatusResponse;

    /// 進捗更新（内部用）
    async fn update_progress(&self, progress: ScanProgress);

    /// 完了処理（内部用）
    async fn complete(&self, results: ScanResults);
}
```

### 5.2 単一実行保証

```rust
impl ScanOrchestrator {
    pub async fn start(&self, request: StartScanRequest) -> Result<StartScanResponse, ScanError> {
        let mut state = self.state.write().await;

        match &*state {
            ScanState::Running { scan_id, .. } => {
                return Err(ScanError::AlreadyRunning {
                    scan_id: scan_id.clone()
                });
            }
            ScanState::Stopping { .. } => {
                return Err(ScanError::StoppingInProgress);
            }
            ScanState::Idle { .. } => {
                // 新規スキャン開始
                let scan_id = generate_scan_id();
                *state = ScanState::Running {
                    scan_id: scan_id.clone(),
                    started_at: Utc::now(),
                    progress: ScanProgress::default(),
                };

                // バックグラウンドタスク起動
                self.spawn_scan_task(scan_id, request).await;

                Ok(StartScanResponse { scan_id, status: "running" })
            }
        }
    }
}
```

---

## 6. フロントエンド実装

### 6.1 スキャンモーダル状態管理

```typescript
interface ScanModalState {
    // UI状態
    isOpen: boolean;

    // スキャン状態
    scanStatus: 'idle' | 'running' | 'stopping' | 'completed' | 'stopped';
    scanId: string | null;

    // 進捗
    progress: {
        percent: number;
        currentStage: string;
        currentStageName: string;
        hostsScanned: number;
        hostsTotal: number;
    } | null;

    // ログ
    logs: ScanLogEntry[];

    // 結果
    results: ScanResults | null;
}
```

### 6.2 モーダル開閉とスキャン継続

```typescript
const CameraScanModal: React.FC = () => {
    const [state, setState] = useState<ScanModalState>();

    // モーダルを開いた時、既存のスキャン状態をチェック
    useEffect(() => {
        if (isOpen) {
            checkExistingScan();
        }
    }, [isOpen]);

    const checkExistingScan = async () => {
        const response = await fetch('/api/scan/status');
        const data = await response.json();

        if (data.data.status === 'running') {
            // 実行中のスキャンに再接続
            setState(prev => ({
                ...prev,
                scanStatus: 'running',
                scanId: data.data.scan_id,
                progress: data.data.progress,
            }));
            subscribeToProgress(data.data.scan_id);
        }
    };

    // モーダルを閉じてもスキャンは継続
    const handleClose = () => {
        setIsOpen(false);
        // スキャンは停止しない
    };

    // 明示的な早期終了
    const handleEarlyStop = async () => {
        await fetch('/api/scan/stop', {
            method: 'POST',
            body: JSON.stringify({ scan_id: scanId }),
        });
    };
};
```

---

## 7. テスト計画

### 7.1 単体テスト
1. ScanOrchestrator状態遷移テスト
2. 単一実行保証テスト（並列開始リクエスト）
3. 停止シグナル処理テスト
4. 進捗計算テスト

### 7.2 結合テスト
1. API→ScanOrchestrator→DB連携テスト
2. WebSocket進捗配信テスト
3. モーダル再接続テスト

### 7.3 UIテスト
1. モーダル閉じてもスキャン継続確認
2. 別タブからのスキャン開始拒否確認
3. 早期終了ボタン動作確認
4. 進捗バーのリアルタイム更新確認

---

## 8. MECE確認

- [x] スキャン状態が相互排他的に定義されている（Idle/Running/Stopping）
- [x] API設計が網羅的（start/stop/status/results）
- [x] フロントエンド・バックエンド両方の変更が設計されている
- [x] エラーケース（並列実行、停止中の開始等）が考慮されている

---

## 9. 承認・実装フロー

1. [ ] 設計レビュー完了
2. [ ] GitHub Issue登録
3. [ ] 実装前コミット
4. [ ] バックエンド API実装
5. [ ] ScanOrchestrator実装
6. [ ] WebSocket統合
7. [ ] フロントエンド実装
8. [ ] 単体テスト実行
9. [ ] 結合テスト実行
10. [ ] UIテスト実行
11. [ ] 完了報告

---

**作成日**: 2026-01-07
**作成者**: Claude Code
**ステータス**: 設計完了・レビュー待ち
