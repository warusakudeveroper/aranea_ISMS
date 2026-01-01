# is22 Suggest設計

文書番号: is22_11
作成日: 2025-12-31
ステータス: Draft
参照:
- is22_Camserver仕様案 Section 5.4
- DESIGN_GAP_ANALYSIS GAP-006

---

## 1. 概要

### 1.1 目的

SuggestEngineは、検知イベントに基づいて「今見るべきカメラ」を決定し、全ユーザーに同一のサジェスト状態を配信する。

### 1.2 設計原則

- **サーバー主導**: サジェスト対象はサーバーで決定
- **全員共通**: 全ユーザーに同じ状態を配信（GlobalSuggestState）
- **TTL制限**: 無限に固定せず、設定されたTTLで見直す
- **優先度ベース**: 検知内容に応じたスコアリングで最優先1件に集約

---

## 2. GlobalSuggestState

### 2.1 状態構造

```rust
struct GlobalSuggestState {
    active: bool,                    // サジェスト表示中か
    camera_id: Option<String>,       // 対象カメラID
    stream_profile: StreamProfile,   // 原則: Sub
    reason_event_id: Option<u64>,    // トリガーとなったイベントID
    started_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    policy: SuggestPolicy,           // どのルールで選ばれたか
}

enum StreamProfile {
    Sub,
    Main,
}

enum SuggestPolicy {
    HighSeverity,      // severity >= 閾値
    HazardDetected,    // hazard.* タグ
    UnknownFlag,       // unknown_flag=true
    ScheduledRound,    // 定期巡回サジェスト（オプション）
    Manual,            // 手動固定
}
```

### 2.2 状態遷移

```
[idle] ─── 検知イベント発生 ───> [active]
                                    │
                                    ├── TTL到達 ───> [idle]
                                    ├── 高優先度イベント ───> [active（更新）]
                                    └── 手動解除 ───> [idle]
```

---

## 3. スコアリングエンジン

### 3.1 スコア計算

| 要素 | 重み | 説明 |
|-----|------|------|
| severity | ×20 | 重要度（0-3） |
| hazard.* | +100 | 危険タグ |
| unknown_flag | +50 | 不明物検知 |
| human検知 | +30 | 人物検知 |
| animal検知 | +20 | 動物検知 |
| camera_issue | +80 | カメラ異常 |
| suggestPolicyWeight | ×1 | カメラごとの重み（0-10） |
| 最終検知からの経過時間 | -1/秒 | 時間減衰 |

### 3.2 スコア計算ロジック

```rust
fn calculate_suggest_score(
    event: &DetectionEvent,
    camera: &Camera,
    now: DateTime<Utc>,
) -> i32 {
    let mut score = 0;

    // 重要度
    score += event.severity as i32 * 20;

    // タグによるボーナス
    if event.tags.iter().any(|t| t.starts_with("hazard.")) {
        score += 100;
    }
    if event.unknown_flag {
        score += 50;
    }
    if event.primary_event == "human" {
        score += 30;
    }
    if event.primary_event == "animal" {
        score += 20;
    }
    if event.primary_event == "camera_issue" {
        score += 80;
    }

    // カメラ重み
    score = score * camera.suggest_policy_weight / 5;

    // 時間減衰
    let elapsed_sec = (now - event.captured_at).num_seconds() as i32;
    score -= elapsed_sec;

    score.max(0)
}
```

---

## 4. サジェスト決定ロジック

### 4.1 決定フロー

```
[新規イベント受信]
       ↓
スコア計算
       ↓
現在のサジェストと比較
       ↓
┌────────────────────────────────────────┐
│ 新スコア > 現スコア × 1.2（ヒステリシス）│
└────────────────────────────────────────┘
       ↓                    ↓
      YES                   NO
       ↓                    ↓
サジェスト更新         維持（変更なし）
       ↓
全クライアントに配信
```

### 4.2 切替ポリシー

```rust
struct SuggestConfig {
    min_score_threshold: i32,          // サジェスト発動最低スコア
    hysteresis_ratio: f32,             // 切替ヒステリシス（1.2 = 20%上回らないと切替）
    ttl_sec: u64,                      // サジェストTTL
    sticky_policies: Vec<SuggestPolicy>, // 固定貼り付け条件
    cooldown_after_clear_sec: u64,     // サジェスト終了後のクールダウン
}

impl Default for SuggestConfig {
    fn default() -> Self {
        Self {
            min_score_threshold: 50,
            hysteresis_ratio: 1.2,
            ttl_sec: 60,
            sticky_policies: vec![SuggestPolicy::HazardDetected],
            cooldown_after_clear_sec: 10,
        }
    }
}
```

### 4.3 Sticky（固定貼り付け）

```rust
fn should_extend_sticky(state: &GlobalSuggestState, event: &DetectionEvent) -> bool {
    // sticky条件に該当するイベントは同一カメラで継続検知中は延長
    match state.policy {
        SuggestPolicy::HazardDetected => {
            event.tags.iter().any(|t| t.starts_with("hazard."))
                && event.camera_id == state.camera_id.as_ref().unwrap()
        }
        _ => false,
    }
}
```

---

## 5. RealtimeHub連携

### 5.1 状態配信

```
[SuggestEngine]
       ↓ 状態変更
[RealtimeHub]
       ↓ WebSocket/SSE
[全クライアント]
```

### 5.2 配信メッセージ

```json
{
    "type": "suggest_update",
    "data": {
        "active": true,
        "camera_id": "cam_2f_19",
        "stream_profile": "sub",
        "reason_event_id": 12345,
        "started_at": "2025-12-31T10:00:00Z",
        "expires_at": "2025-12-31T10:01:00Z",
        "policy": "high_severity"
    }
}
```

### 5.3 サジェスト終了

```json
{
    "type": "suggest_update",
    "data": {
        "active": false,
        "camera_id": null,
        "stream_profile": null,
        "reason_event_id": null,
        "started_at": null,
        "expires_at": null,
        "policy": null
    }
}
```

---

## 6. プリウォーム

### 6.1 目的

サジェスト切替時に「速く見せる」ため、事前にストリーム接続を準備する。

### 6.2 実装

```rust
async fn prewarm_stream(camera_id: &str) {
    // StreamGatewayAdapterに事前接続要求
    stream_gateway.prepare_stream(camera_id, StreamProfile::Sub).await;
}
```

### 6.3 トリガー

- 現在のサジェストTTL残り10秒以下で次候補が存在する場合
- 高スコアイベント検知時（切替前に準備）

---

## 7. UI連携

### 7.1 左ペイン表示

```
サジェストactive=true時：
┌─────────────────────────────────────┐
│ [ライブ映像]                         │
│                                     │
│ ────────────────────────────────────│
│ 📍 2F 廊下                          │
│ 人物検知 18:59                       │
└─────────────────────────────────────┘

サジェストactive=false時：
┌─────────────────────────────────────┐
│                                     │
│     （検知イベントなし）             │
│                                     │
└─────────────────────────────────────┘
```

### 7.2 中央ペインハイライト

```javascript
function updateCameraHighlight(suggestState) {
    document.querySelectorAll('.camera-thumb').forEach(el => {
        el.classList.remove('suggest-active');
    });
    if (suggestState.active && suggestState.camera_id) {
        const target = document.querySelector(`[data-camera-id="${suggestState.camera_id}"]`);
        if (target) {
            target.classList.add('suggest-active');
        }
    }
}
```

### 7.3 CSS

```css
.camera-thumb.suggest-active {
    border: 3px solid #ff6b00;
    box-shadow: 0 0 10px rgba(255, 107, 0, 0.5);
    animation: pulse 1s infinite;
}

@keyframes pulse {
    0% { box-shadow: 0 0 10px rgba(255, 107, 0, 0.5); }
    50% { box-shadow: 0 0 20px rgba(255, 107, 0, 0.8); }
    100% { box-shadow: 0 0 10px rgba(255, 107, 0, 0.5); }
}
```

---

## 8. API設計

### 8.1 現在のサジェスト状態

```
GET /api/suggest
```

Response:
```json
{
    "active": true,
    "camera_id": "cam_2f_19",
    "camera_name": "2F 廊下",
    "stream_url": "wss://camserver/stream/cam_2f_19",
    "reason_event_id": 12345,
    "reason_summary": "人物を検知（単独 / 上衣:赤系）",
    "started_at": "2025-12-31T10:00:00Z",
    "expires_at": "2025-12-31T10:01:00Z",
    "remaining_sec": 45
}
```

### 8.2 手動サジェスト設定（管理用）

```
POST /api/suggest/manual
```

Request:
```json
{
    "camera_id": "cam_entrance",
    "duration_sec": 300
}
```

### 8.3 サジェスト解除（管理用）

```
DELETE /api/suggest
```

---

## 9. 設定

### 9.1 環境変数

| 変数 | 説明 | デフォルト |
|-----|------|-----------|
| SUGGEST_MIN_SCORE | サジェスト発動最低スコア | 50 |
| SUGGEST_TTL_SEC | サジェストTTL | 60 |
| SUGGEST_HYSTERESIS | 切替ヒステリシス | 1.2 |
| SUGGEST_COOLDOWN_SEC | 終了後クールダウン | 10 |

### 9.2 settingsテーブル

```sql
INSERT INTO settings (setting_key, setting_json) VALUES
('suggest', JSON_OBJECT(
    'min_score_threshold', 50,
    'ttl_sec', 60,
    'hysteresis_ratio', 1.2,
    'cooldown_after_clear_sec', 10,
    'sticky_policies', JSON_ARRAY('hazard_detected')
));
```

---

## 10. テスト観点

### 10.1 スコアリング

- [ ] severity=3でスコア60以上
- [ ] hazard.*で+100
- [ ] unknown_flagで+50
- [ ] 時間減衰が正しく動作

### 10.2 サジェスト決定

- [ ] 閾値以上でサジェスト発動
- [ ] ヒステリシスで頻繁な切替を抑制
- [ ] TTL到達でサジェスト終了
- [ ] sticky条件で延長

### 10.3 配信

- [ ] 状態変更が全クライアントに配信
- [ ] 中央ペインハイライト連動
- [ ] 左ペイン映像切替

### 10.4 多発時

- [ ] 最優先1件に集約
- [ ] 他はログ表示のみ

---

## 更新履歴

| 日付 | バージョン | 内容 |
|-----|-----------|------|
| 2025-12-31 | 1.0 | 初版作成（Phase 4） |
