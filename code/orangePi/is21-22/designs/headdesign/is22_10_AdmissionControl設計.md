# is22 AdmissionControl設計

文書番号: is22_10
作成日: 2025-12-31
ステータス: Draft
参照:
- is22_Camserver仕様案 Section 5.6, 5.7
- DESIGN_GAP_ANALYSIS GAP-002, GAP-005

---

## 1. 概要

### 1.1 目的

AdmissionControllerは、Camserverの負荷を制御し、以下を実現する：
- 通常巡回・サジェスト・操作パフォーマンスを最優先で死守
- モーダル要求は拒否して良い（メッセージ表示で割り切る）
- 長時間運用でパンクしない

### 1.2 設計原則

- **予算モデル**: 固定上限から予約分を引いた残りがモーダル許容量
- **Lease/Heartbeat**: サーバーが発行・管理、タブ放置を自動回収
- **ヒステリシス**: フラップ防止のため復帰条件を厳しめに

---

## 2. 予算モデル

### 2.1 パラメータ定義

| パラメータ | 説明 | デフォルト値 |
|-----------|------|-------------|
| TOTAL_STREAM_UNITS | 機器能力に合わせた総ストリーム数 | 50 |
| MAX_UI_USERS | 最大同時UIユーザー数 | 10 |
| RESERVED_BASELINE_UNITS | 巡回・ログ等の予約分 | 5 |
| SUB_STREAM_COST | サブストリームのユニットコスト | 1 |
| MAIN_STREAM_COST | メインストリームのユニットコスト | 2 |

### 2.2 予算計算

```
RESERVED_SUGGEST_UNITS = MAX_UI_USERS * 1  // サジェスト最悪ケース
MODAL_BUDGET = TOTAL_STREAM_UNITS - RESERVED_BASELINE_UNITS - RESERVED_SUGGEST_UNITS

例：
MODAL_BUDGET = 50 - 5 - 10 = 35 units
```

### 2.3 現在使用量の計算

```rust
fn current_usage() -> u32 {
    let suggest_usage = if suggest_active { active_ui_users * SUB_STREAM_COST } else { 0 };
    let modal_usage = active_modals.iter()
        .map(|m| if m.quality == Main { MAIN_STREAM_COST } else { SUB_STREAM_COST })
        .sum();

    RESERVED_BASELINE_UNITS + suggest_usage + modal_usage
}
```

---

## 3. Lease管理

### 3.1 ModalLease構造

```rust
struct ModalLease {
    lease_id: Uuid,
    user_id: String,          // セッションID
    camera_id: String,
    quality: StreamQuality,   // Sub / Main
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    last_heartbeat: DateTime<Utc>,
}

enum StreamQuality {
    Sub,   // 1 unit
    Main,  // 2 units
}
```

### 3.2 Lease発行フロー

```
[クライアント] --POST /api/modal/lease--> [AdmissionController]
                                                 ↓
                                          予算チェック
                                                 ↓
                      ┌─────────────────────────┴─────────────────────────┐
                      ↓                                                     ↓
               許可（予算内）                                         拒否（予算超過）
                      ↓                                                     ↓
            Lease発行・保存                                          エラー応答
                      ↓                                                     ↓
            {leaseId, expiresAt}                           {errorCode: "OVER_CAPACITY"}
```

### 3.3 TTL設定

| 項目 | 値 | 説明 |
|-----|-----|------|
| MODAL_TTL_SEC | 300 | モーダル最大表示時間（5分） |
| HEARTBEAT_INTERVAL_SEC | 30 | クライアントのHeartbeat間隔 |
| HEARTBEAT_GRACE_SEC | 45 | Heartbeat猶予（これを超えるとLease失効） |

---

## 4. Heartbeat機構

### 4.1 クライアント側

```javascript
class ModalHeartbeat {
    constructor(leaseId) {
        this.leaseId = leaseId;
        this.interval = null;
    }

    start() {
        this.interval = setInterval(() => {
            fetch(`/api/modal/lease/${this.leaseId}/heartbeat`, {
                method: 'POST'
            }).catch(e => console.error('Heartbeat failed:', e));
        }, 30000); // 30秒間隔
    }

    stop() {
        if (this.interval) {
            clearInterval(this.interval);
            this.interval = null;
        }
    }
}
```

### 4.2 サーバー側

```rust
async fn heartbeat(lease_id: Uuid) -> Result<(), Error> {
    let lease = leases.get_mut(&lease_id)?;
    lease.last_heartbeat = Utc::now();
    Ok(())
}

// 定期クリーンアップ（10秒ごと）
async fn cleanup_expired_leases() {
    let now = Utc::now();
    leases.retain(|_, lease| {
        let heartbeat_expired = now - lease.last_heartbeat > Duration::seconds(HEARTBEAT_GRACE_SEC);
        let ttl_expired = now > lease.expires_at;
        !heartbeat_expired && !ttl_expired
    });
}
```

---

## 5. CPU/メモリ監視（最終安全弁）

### 5.1 監視メトリクス

| メトリクス | 警告閾値 | 拒否閾値 |
|-----------|---------|---------|
| CPU使用率 | 70% | 85% |
| メモリ使用率 | 75% | 90% |
| Swap使用量 | 50MB | 100MB |

### 5.2 ヒステリシス

```rust
struct SystemHealth {
    overloaded: bool,
    last_overload_at: Option<DateTime<Utc>>,
}

impl SystemHealth {
    fn check(&mut self, cpu: f32, memory: f32) {
        if cpu > 85.0 || memory > 90.0 {
            self.overloaded = true;
            self.last_overload_at = Some(Utc::now());
        } else if self.overloaded {
            // 復帰条件：過負荷解消から60秒経過 + 現在値が低め
            if let Some(last) = self.last_overload_at {
                let elapsed = Utc::now() - last;
                if elapsed > Duration::seconds(60) && cpu < 60.0 && memory < 70.0 {
                    self.overloaded = false;
                }
            }
        }
    }
}
```

---

## 6. Admission判定ロジック

### 6.1 判定フロー

```rust
fn can_admit(request: &LeaseRequest) -> AdmissionResult {
    // 1. システム過負荷チェック
    if system_health.overloaded {
        return AdmissionResult::Rejected(RejectReason::SystemOverload);
    }

    // 2. 同一ユーザー既存モーダルチェック
    if has_active_modal(&request.user_id) {
        return AdmissionResult::Rejected(RejectReason::UserAlreadyHasModal);
    }

    // 3. 予算チェック
    let cost = match request.quality {
        Main => MAIN_STREAM_COST,
        Sub => SUB_STREAM_COST,
    };
    let available = MODAL_BUDGET - current_modal_usage();
    if cost > available {
        return AdmissionResult::Rejected(RejectReason::OverCapacity);
    }

    // 4. 許可
    AdmissionResult::Admitted(cost)
}
```

### 6.2 拒否理由

| 理由コード | メッセージ | 対応 |
|-----------|-----------|------|
| SYSTEM_OVERLOAD | サーバーが高負荷のためしばらく経ってからお試しください | 待機 |
| USER_ALREADY_HAS_MODAL | 既に別のカメラを表示中です | 既存モーダルを閉じる |
| OVER_CAPACITY | 現在多数のアクセスがありストリーム数が上限に達しています | 待機 |

---

## 7. データモデル

### 7.1 modal_leases

```sql
CREATE TABLE modal_leases (
    lease_id CHAR(36) PRIMARY KEY,
    user_id VARCHAR(128) NOT NULL,      -- セッションID
    camera_id VARCHAR(64) NOT NULL,
    quality ENUM('sub', 'main') NOT NULL DEFAULT 'sub',
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    expires_at DATETIME(3) NOT NULL,
    last_heartbeat DATETIME(3) NOT NULL,

    UNIQUE KEY uq_leases_user (user_id),  -- ユーザーごと1モーダル
    INDEX idx_leases_expires (expires_at),
    INDEX idx_leases_heartbeat (last_heartbeat)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

### 7.2 admission_metrics（監視用）

```sql
CREATE TABLE admission_metrics (
    metric_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    recorded_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    active_modals INT UNSIGNED NOT NULL,
    active_suggest TINYINT(1) NOT NULL,
    cpu_percent DECIMAL(5,2) NOT NULL,
    memory_percent DECIMAL(5,2) NOT NULL,
    requests_total INT UNSIGNED NOT NULL,
    rejections_total INT UNSIGNED NOT NULL,

    INDEX idx_metrics_time (recorded_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

---

## 8. API設計

### 8.1 Lease発行

```
POST /api/modal/lease
```

Request:
```json
{
    "camera_id": "cam_2f_19",
    "quality": "sub"
}
```

Response（成功）:
```json
{
    "lease_id": "550e8400-e29b-41d4-a716-446655440000",
    "allowed_quality": "sub",
    "expires_at": "2025-12-31T10:05:00Z",
    "stream_url": "wss://camserver/stream/cam_2f_19"
}
```

Response（拒否）:
```json
{
    "error_code": "OVER_CAPACITY",
    "message": "現在多数のアクセスがありストリーム数が上限に達しています。しばらく経ってからアクセスしてください"
}
```

### 8.2 Heartbeat

```
POST /api/modal/lease/{lease_id}/heartbeat
```

Response:
```json
{
    "ok": true,
    "remaining_sec": 240
}
```

### 8.3 Lease解放

```
DELETE /api/modal/lease/{lease_id}
```

Response:
```json
{
    "ok": true
}
```

### 8.4 システム状態

```
GET /api/system/status
```

Response:
```json
{
    "healthy": true,
    "cpu_percent": 45.2,
    "memory_percent": 62.1,
    "active_modals": 3,
    "modal_budget_remaining": 32,
    "suggest_active": true
}
```

---

## 9. UI連携

### 9.1 モーダル表示フロー

```
[サムネクリック]
       ↓
  Lease要求（POST /api/modal/lease）
       ↓
  ┌────┴────┐
  ↓          ↓
許可       拒否
  ↓          ↓
モーダル表示  拒否メッセージ表示
  ↓
Heartbeat開始
  ↓
TTL到達 or ユーザー終了
  ↓
Lease解放 + ストリーム停止
```

### 9.2 TTL到達時のUI

```javascript
function onTTLExpired() {
    stopStream();
    showDialog({
        title: "表示時間が終了しました",
        message: "続けて表示しますか？",
        buttons: [
            { label: "再接続", action: () => requestNewLease() },
            { label: "閉じる", action: () => closeModal() }
        ]
    });
}
```

### 9.3 バックグラウンド時の処理

```javascript
document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
        // バックグラウンド時はモーダル停止
        if (activeModal) {
            releaseModal();
        }
    }
});
```

---

## 10. 長時間運用設計

### 10.1 リソースクリーンアップ

| 対象 | 間隔 | 処理 |
|-----|------|------|
| 期限切れLease | 10秒 | 自動削除 |
| Heartbeat停止Lease | 10秒 | 自動削除 |
| admission_metrics | 1時間 | 24時間以上古いレコード削除 |

### 10.2 UI側クリーンアップ

- WebRTC PeerConnection close
- track stop
- srcObject解除
- イベントリスナ解除

---

## 11. テスト観点

### 11.1 Admission判定

- [ ] 予算内でLease発行成功
- [ ] 予算超過でOVER_CAPACITY拒否
- [ ] 過負荷時でSYSTEM_OVERLOAD拒否
- [ ] 同一ユーザー2つ目でUSER_ALREADY_HAS_MODAL拒否

### 11.2 Lease管理

- [ ] TTL到達でLease自動失効
- [ ] Heartbeat停止でLease自動失効
- [ ] 明示的DELETE成功

### 11.3 長時間運用

- [ ] 24時間放置でメモリリークなし
- [ ] Lease蓄積なし
- [ ] クリーンアップ正常動作

---

## 更新履歴

| 日付 | バージョン | 内容 |
|-----|-----------|------|
| 2025-12-31 | 1.0 | 初版作成（Phase 4） |
