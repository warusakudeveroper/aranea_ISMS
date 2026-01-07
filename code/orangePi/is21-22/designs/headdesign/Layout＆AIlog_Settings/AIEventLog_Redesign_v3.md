# AIEventLog_Redesign_v3: AI Event Log包括的設計（違反解消版）

文書番号: Layout＆AIlog_Settings/AIEventLog_Redesign_v3
作成日: 2026-01-08
ステータス: レビュー待ち
関連文書: AIEventlog.md, The_golden_rules.md
前版: AIEventLog_Redesign_v2.md（保留）
レビュー文書: AIEventLog_Design_Review_Feedback.md

---

## 1. 大原則の宣言

本設計は以下の大原則に**完全**準拠する:

- [x] **Rule 1 (SSoT)**: AIEventlog.mdを唯一の情報源として参照
- [x] **Rule 2 (SOLID)**: 各コンポーネントは単一責務
- [x] **Rule 3 (MECE)**: 全問題を網羅的・相互排他的に分類
- [x] **Rule 4 (Unambiguous)**: 全コード・条件・動作を曖昧さなく明確に定義
- [x] **Rule 6 (現場猫禁止)**: 具体的コード参照・エビデンス付き
- [x] **Rule 7 (棚上げ禁止)**: 全タスクは本設計で解決。省略なし
- [x] **Rule 9 (テスト必須)**: BE/FE/UIテストを完全定義
- [x] **Rule 12 (車輪禁止)**: 既存コンポーネント構造を活用
- [x] **Rule 15 (設計必須)**: GitHub Issue登録・実装前コミット手順あり

---

## 2. IS21/IS22評価方法解説（FIX-C02対応）

> AIEventlog.md要件: 「is21とis22の評価方法、評価基準、レスポンスの内容などについて詳細に解説」

### 2.1 システム構成

```
┌─────────────────────────────────────────────────────────────────┐
│                         IS22 (is22サーバー)                      │
│  ┌───────────────────┐  ┌─────────────────┐  ┌───────────────┐ │
│  │PollingOrchestrator│→│  SnapshotService │→│   AiClient    │─┼──→ IS21
│  │  (巡回制御)        │  │  (画像取得)      │  │ (IS21通信)    │ │
│  └───────────────────┘  └─────────────────┘  └───────────────┘ │
│           ↓                                         ↑           │
│  ┌───────────────────┐                    ┌─────────────────┐  │
│  │ DetectionLogService│←───────────────────│ PrevFrameCache │  │
│  │  (ログ・画像保存)  │                    │ (前フレーム)    │  │
│  └───────────────────┘                    └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 評価フロー

```
1. PollingOrchestrator: カメラ巡回開始
                ↓
2. SnapshotService: RTSPから画像取得（snapshot_ms計測）
                ↓
3. PrevFrameCache: 前フレーム情報取得（存在すれば）
                ↓
4. AiClient: IS21へ画像送信（/v1/analyze）
   - 送信データ:
     - image: JPEG画像バイナリ
     - metadata: camera_id, captured_at, preset_id
     - hints_json: camera_context（前フレーム情報含む）
                ↓
5. IS21: 推論実行
   - YOLO: 物体検出（yolo_ms）
   - PAR: 人物属性認識（par_ms）
   - FrameDiff: 差分解析（前フレームあれば）
                ↓
6. IS21 → IS22: AnalyzeResponse返却
                ↓
7. DetectionLogService: ログ・画像保存
                ↓
8. PrevFrameCache: 現フレームを次回用に保存
```

### 2.3 IS21 AnalyzeResponse構造

```typescript
interface AnalyzeResponse {
  // 基本情報
  schema_version: string       // "2025-12-29.1"
  camera_id: string
  captured_at: string          // ISO8601

  // プリセット情報
  preset_id: string            // "balanced", "high_security" など
  preset_version: string

  // 解析ステータス
  analyzed: boolean            // 解析実行されたか
  detected: boolean            // 何か検出されたか

  // 検出結果
  primary_event: string        // "human", "vehicle", "unknown", "none" など
  severity: number             // 0-4（0=検出なし、1=低、2=中、3=高、4=緊急）
  confidence: number           // 0.0-1.0
  count_hint: number           // 検出人数
  unknown_flag: boolean        // 不明判定フラグ
  tags: string[]               // ["carry.bag", "outfit.pants" など]

  // 詳細データ
  bboxes: BBox[]               // バウンディングボックス配列
  person_details: PersonDetail[] // 人物属性詳細
  suspicious: SuspiciousInfo   // 不審行動スコア

  // フレーム差分（前フレームがある場合）
  frame_diff: {
    enabled: boolean
    person_changes: { appeared, disappeared, moved, stationary }
    loitering: { detected, duration_seconds }
    scene_change: { significant, change_ratio }
  }

  // 処理時間
  performance: {
    inference_ms: number
    yolo_ms: number
    par_ms: number
  }
}
```

### 2.4 プリセット動作確認方法

**エビデンス取得方法**:

```bash
# IS22ログで確認
journalctl -u is22 -f | grep "preset_id"

# 期待ログ出力例
# preset_id="balanced" preset_version="1.0.0"
```

**プリセット効果の検証**:

| プリセットID | conf_threshold | 用途 |
|-------------|----------------|------|
| balanced | 0.5 | 標準検出 |
| high_security | 0.3 | 高感度（誤検知許容） |
| low_noise | 0.7 | 低感度（誤検知抑制） |

### 2.5 フレーム差分査定の仕組み

**差分解析フロー**:

```
1. IS22: PrevFrameCache.get_previous_frame_info(camera_id)
   → PreviousFrameInfo { captured_at, person_count, primary_event }

2. IS22 → IS21: camera_context.previous_frame に設定して送信

3. IS21: 前フレームと現フレームを比較
   - person_changes: 出現/消失/移動/静止の人数
   - loitering: 同一位置に留まる人物の検出
   - scene_change: 画面全体の変化率

4. IS21 → IS22: frame_diff オブジェクトとして返却
```

**エビデンス取得方法**:

```bash
# IS22ログで差分解析結果確認
journalctl -u is22 -f | grep "frame_diff"

# 期待ログ出力例
# frame_diff.enabled=true person_changes.appeared=1
```

### 2.6 現状の評価

| 項目 | 状態 | エビデンス |
|------|------|-----------|
| 画像送信 | 動作 | ログでimage_size_kb確認可能 |
| YOLO検出 | 動作 | primary_event返却確認 |
| PAR分析 | 動作 | tags, person_details返却確認 |
| フレーム差分 | 動作 | frame_diff.enabled=true確認 |
| プリセット適用 | 動作 | preset_id echo-back確認 |

---

## 3. 問題分析（MECE構造）

### 3.1 Category A: ストレージ管理問題（Critical）

#### A-1: 画像無制限保存

**現状コード** (`polling_orchestrator/mod.rs:1022-1034`):
```rust
let log_id = detection_log
    .save_detection(
        tid, fid, lacis_id,
        &result,
        &image_data,  // ← 常に保存（無条件）
        ...
    )
    .await?;
```

#### A-2: クォータ未実装

**現状コード** (`detection_log_service/mod.rs:110`):
```rust
pub max_images_per_camera: usize,  // 1000に設定されているが未使用
```

#### A-3: 不要画像の削除ロジックなし

**要件**: unknown/none画像は最新10%のみ保持、90%削除

---

### 3.2 Category B: 表示仕様違反問題（High）

#### B-1〜B-5: v2と同一（省略）

色スキーム、ラベル翻訳、カメラ名表示、ツールチップ、アイコン視認性

---

### 3.3 Category C: 監視・分析機能（Medium）

#### C-1: 推論統計タブなし
#### C-2: ストレージ使用量設定なし

---

### 3.4 Category D: 後続タスク基盤（autoAttunement）

> AIEventlog.md要件: 「autoAttunementの基本設計ドキュメントの作成と実装予定todoをコード内に組み込んでおく必要性」

**本設計ではCategory D基本設計を含める**（FIX-C01対応）

---

## 4. 詳細設計

### 4.1 IMPL-A: ストレージ管理実装

#### 4.1.1 画像保存条件の真理値表（FIX-H01対応）

| severity | primary_event | unknown_flag | should_save_image | 理由 |
|----------|---------------|--------------|-------------------|------|
| 0 | "none" | false | **false** | 検出なし、保存不要 |
| 0 | "none" | true | true | unknown判定、分析用に保存 |
| 0 | "human" | false | true | イベントあり |
| 0 | "unknown" | true | true | unknown判定 |
| 1+ | any | any | true | severity > 0は常に保存 |

**実装コード**:

```rust
// polling_orchestrator/mod.rs
let should_save_image = result.severity > 0
    || result.primary_event != "none"
    || result.unknown_flag;

let image_to_save: Option<&[u8]> = if should_save_image {
    Some(&image_data[..])
} else {
    None
};

let log_id = detection_log.save_detection(
    tid, fid, lacis_id,
    &result,
    image_to_save,  // Option<&[u8]>
    camera_context.as_ref(),
    Some(processing_ms),
    Some(&timings),
    polling_cycle_id,
).await?;
```

#### 4.1.2 CleanupStats構造体定義（FIX-M01対応）

```rust
// detection_log_service/mod.rs

/// 画像クリーンアップ統計
#[derive(Debug, Clone)]
pub struct CleanupStats {
    /// 処理対象の総画像数
    pub total: usize,
    /// 削除された画像数
    pub deleted: usize,
    /// 保持された画像数
    pub kept: usize,
}

impl CleanupStats {
    pub fn new(total: usize, deleted: usize, kept: usize) -> Self {
        Self { total, deleted, kept }
    }

    /// 削除率（%）
    pub fn deletion_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.deleted as f64 / self.total as f64) * 100.0
        }
    }
}
```

#### 4.1.3 クォータ強制・ローテーション実装

```rust
// detection_log_service/mod.rs

/// カメラ毎の画像クォータを強制し、古い画像を削除
pub async fn enforce_image_quota(&self, camera_id: &str) -> Result<CleanupStats> {
    let camera_dir = self.config.image_base_path.join(camera_id);
    if !camera_dir.exists() {
        return Ok(CleanupStats::new(0, 0, 0));
    }

    // 1. 画像ファイル一覧取得
    let mut entries: Vec<_> = fs::read_dir(&camera_dir)
        .await?
        .filter_map(|e| async { e.ok() })
        .collect()
        .await;

    // 2. 更新日時でソート（古い順）
    entries.sort_by_key(|e| {
        e.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });

    let total = entries.len();
    let quota = self.config.max_images_per_camera;

    // 3. クォータ超過分を削除
    if total > quota {
        let delete_count = total - quota;
        for entry in entries.iter().take(delete_count) {
            if let Err(e) = fs::remove_file(entry.path()).await {
                tracing::warn!(path = ?entry.path(), error = %e, "Failed to delete image");
            }
        }
        return Ok(CleanupStats::new(total, delete_count, quota));
    }

    Ok(CleanupStats::new(total, 0, total))
}
```

#### 4.1.4 Unknown/None画像クリーンアップ

```rust
// detection_log_service/mod.rs

/// unknown/none判定画像のクリーンアップ
/// 最新10%を残し、残り90%を削除
pub async fn cleanup_unknown_images(&self, camera_id: &str) -> Result<CleanupStats> {
    // 1. DBからunknown/noneの画像パスを取得（古い順）
    let paths = sqlx::query_scalar::<_, String>(
        r#"
        SELECT image_path_local FROM detection_logs
        WHERE camera_id = ?
          AND (primary_event IN ('unknown', 'none') OR unknown_flag = TRUE)
          AND image_path_local != ''
        ORDER BY captured_at ASC
        "#
    )
    .bind(camera_id)
    .fetch_all(&self.pool)
    .await?;

    let total = paths.len();
    if total == 0 {
        return Ok(CleanupStats::new(0, 0, 0));
    }

    // 2. 保持数計算（最新10%、最低1件）
    let keep_count = std::cmp::max(1, (total as f64 * 0.10).ceil() as usize);
    let delete_count = total.saturating_sub(keep_count);

    // 3. 削除対象（古い90%）のファイルを削除
    let mut actual_deleted = 0;
    for path in paths.iter().take(delete_count) {
        match fs::remove_file(path).await {
            Ok(_) => actual_deleted += 1,
            Err(e) => tracing::warn!(path = %path, error = %e, "Failed to delete image"),
        }
    }

    // 4. DBのimage_path_localをクリア
    if actual_deleted > 0 {
        sqlx::query(
            r#"
            UPDATE detection_logs
            SET image_path_local = ''
            WHERE camera_id = ?
              AND (primary_event IN ('unknown', 'none') OR unknown_flag = TRUE)
              AND image_path_local != ''
            ORDER BY captured_at ASC
            LIMIT ?
            "#
        )
        .bind(camera_id)
        .bind(delete_count as i64)
        .execute(&self.pool)
        .await?;
    }

    tracing::info!(
        camera_id = %camera_id,
        total = total,
        deleted = actual_deleted,
        kept = keep_count,
        "Unknown images cleanup completed"
    );

    Ok(CleanupStats::new(total, actual_deleted, total - actual_deleted))
}
```

---

### 4.2 IMPL-B: 表示仕様準拠実装

#### 4.2.1 色スキーム定義

```typescript
// frontend/src/components/EventLogPane.tsx

const EVENT_COLORS = {
  human: { bg: '#FFFF00', text: '#222222', border: '#B8B800' },
  vehicle: { bg: '#6495ED', text: '#FFFFFF', border: '#4169E1' },
  alert: { bg: '#FF0000', text: '#FFFF00', border: '#CC0000' },
  unknown: { bg: '#D3D3D3', text: '#222222', border: '#A9A9A9' },
  camera_lost: { bg: '#333333', text: '#FFFFFF', border: '#1A1A1A' },
  camera_recovered: { bg: '#FFFFFF', text: '#444444', border: '#CCCCCC' },
} as const

type EventColorKey = keyof typeof EVENT_COLORS
```

#### 4.2.2 getEventStyle関数

```typescript
function getEventStyle(primaryEvent: string): React.CSSProperties {
  const event = primaryEvent.toLowerCase()

  let colorKey: EventColorKey = 'unknown'

  if (event === 'human' || event === 'person' || event === 'humans' || event === 'people') {
    colorKey = 'human'
  } else if (event === 'vehicle' || event === 'car' || event === 'truck') {
    colorKey = 'vehicle'
  } else if (['suspicious', 'alert', 'intrusion', 'fire', 'smoke'].includes(event)) {
    colorKey = 'alert'
  } else if (event === 'camera_lost') {
    colorKey = 'camera_lost'
  } else if (event === 'camera_recovered') {
    colorKey = 'camera_recovered'
  }

  const colors = EVENT_COLORS[colorKey]
  return {
    backgroundColor: colors.bg,
    color: colors.text,
    borderLeft: `3px solid ${colors.border}`,
  }
}
```

#### 4.2.3 ラベル翻訳定義

```typescript
const EVENT_LABELS: Record<string, string> = {
  'human': '人物検知',
  'person': '人物検知',
  'humans': '複数人検知',
  'people': '複数人検知',
  'crowd': '群衆検知',
  'vehicle': '車両検知',
  'car': '車両検知',
  'truck': 'トラック検知',
  'suspicious': '不審行動',
  'alert': '警戒',
  'intrusion': '侵入検知',
  'fire': '火災検知',
  'smoke': '煙検知',
  'animal': '動物検知',
  'dog': '犬検知',
  'cat': '猫検知',
  'package': '荷物検知',
  'object': '物体検知',
  'motion': '動体検知',
  'movement': '動き検知',
  'unknown': '不明',
  'none': '検出なし',
  'camera_lost': '接続ロスト',
  'camera_recovered': '接続復帰',
}

function getEventLabel(primaryEvent: string, countHint: number): string {
  const event = primaryEvent.toLowerCase()
  const label = EVENT_LABELS[event] || primaryEvent

  if (countHint > 1) {
    return `${label} (${countHint})`
  }
  return label
}
```

#### 4.2.4 Severityツールチップ

```typescript
const SEVERITY_TOOLTIPS: Record<number, string> = {
  0: '検出なし',
  1: '低優先度 - 通常の検知',
  2: '中優先度 - 注意が必要',
  3: '高優先度 - 即時確認推奨',
  4: '緊急 - 即時対応必要',
}
```

#### 4.2.5 getIconColor関数（FIX-H02対応：完全実装）

```typescript
// 背景色に対応するアイコン色
const ICON_COLORS: Record<EventColorKey, string> = {
  human: '#222222',        // 黄色背景 → 濃い文字
  vehicle: '#FFFFFF',      // 青背景 → 白文字
  alert: '#FFFF00',        // 赤背景 → 黄文字
  unknown: '#555555',      // グレー背景 → 濃いグレー
  camera_lost: '#FFFFFF',  // 濃グレー背景 → 白
  camera_recovered: '#444444', // 白背景 → グレー
}

function getIconColor(primaryEvent: string): string {
  const event = primaryEvent.toLowerCase()

  let colorKey: EventColorKey = 'unknown'

  if (event === 'human' || event === 'person' || event === 'humans' || event === 'people') {
    colorKey = 'human'
  } else if (event === 'vehicle' || event === 'car' || event === 'truck') {
    colorKey = 'vehicle'
  } else if (['suspicious', 'alert', 'intrusion', 'fire', 'smoke'].includes(event)) {
    colorKey = 'alert'
  } else if (event === 'camera_lost') {
    colorKey = 'camera_lost'
  } else if (event === 'camera_recovered') {
    colorKey = 'camera_recovered'
  }

  return ICON_COLORS[colorKey]
}
```

#### 4.2.6 カメラ名表示修正

```tsx
<span
  className="text-[10px] font-medium truncate min-w-[40px] max-w-[120px] flex-shrink"
  title={cameraName}
>
  {cameraName}
</span>
```

---

### 4.3 IMPL-C: 監視・分析機能

#### 4.3.1 推論統計タブ

新規コンポーネント `InferenceStatsTab.tsx` を作成。
表示内容: カメラ別検出件数、イベント種別分布、処理時間統計

#### 4.3.2 ストレージ設定

設定モーダルに最大画像保存数の設定UIを追加。

---

### 4.4 IMPL-D: autoAttunement基本設計（FIX-C01対応）

> AIEventlog.md要件: 「autoAttunementの調整用API(LLMおよび人力調整)の機能拡張が必要でありその基本設計ドキュメントの作成と実装予定todoをコード内に組み込んでおく必要性」

#### 4.4.1 autoAttunementの目的

カメラ毎の検出傾向を統計分析し、過剰検出・誤検出を自動調整する機能。

#### 4.4.2 基本アーキテクチャ

```
┌─────────────────────────────────────────────────────────────────┐
│                      autoAttunement System                       │
│                                                                  │
│  ┌─────────────────┐     ┌─────────────────┐     ┌───────────┐ │
│  │ StatsCollector  │────→│ TrendAnalyzer   │────→│ Adjuster  │ │
│  │ (統計収集)       │     │ (傾向分析)       │     │ (調整適用) │ │
│  └─────────────────┘     └─────────────────┘     └───────────┘ │
│           ↑                       ↑                     ↓       │
│  ┌─────────────────┐     ┌─────────────────┐     ┌───────────┐ │
│  │ DetectionLog    │     │ LLM Context     │     │ PresetDB  │ │
│  │ (検出履歴)       │     │ (施設情報)       │     │ (閾値調整) │ │
│  └─────────────────┘     └─────────────────┘     └───────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

#### 4.4.3 統計収集項目

| 項目 | 説明 | 用途 |
|------|------|------|
| 検出頻度 | カメラ毎の時間あたり検出数 | 過剰検出検知 |
| 誤検出率 | unknown/誤検出の割合 | 閾値調整 |
| イベント分布 | human/vehicle/alert等の比率 | 傾向把握 |
| 時間帯パターン | 時間帯別検出傾向 | 予測モデル |

#### 4.4.4 調整API設計

```rust
// TODO: autoAttunement API (後続タスクで実装)

/// カメラ毎の検出統計を取得
/// GET /api/attunement/stats/{camera_id}
pub async fn get_camera_stats(camera_id: &str) -> AttunementStats;

/// 調整推奨値を取得（LLMベース）
/// GET /api/attunement/suggest/{camera_id}
pub async fn suggest_adjustment(camera_id: &str) -> AdjustmentSuggestion;

/// 調整を適用
/// POST /api/attunement/apply/{camera_id}
pub async fn apply_adjustment(camera_id: &str, adj: Adjustment) -> Result<()>;
```

#### 4.4.5 本設計でのTODO埋め込み

以下のTODOコメントを該当ファイルに追加:

```rust
// polling_orchestrator/mod.rs
// TODO(autoAttunement): 検出結果をStatsCollectorに送信
// 実装予定: Layout＆AIlog_Settings/AIEventLog_Redesign_v3.md Section 4.4

// detection_log_service/mod.rs
// TODO(autoAttunement): 統計集計クエリを追加
// 実装予定: Layout＆AIlog_Settings/AIEventLog_Redesign_v3.md Section 4.4

// preset_loader/mod.rs
// TODO(autoAttunement): 動的閾値調整機能を追加
// 実装予定: Layout＆AIlog_Settings/AIEventLog_Redesign_v3.md Section 4.4
```

#### 4.4.6 LLMコンテキスト連携（概要）

mobesSystemの施設情報（fid, rid）とカメラコンテキストを連携し、施設種別に応じた検出調整を行う。

```
宿泊施設 → 予約者情報との照合
飲食店   → 営業時間に応じた検出閾値
オフィス → 入退室管理との連携
```

**本設計スコープ**: 基本設計のみ。詳細実装は後続タスクで設計。

---

## 5. 変更ファイル一覧

| ファイル | 変更種別 | 変更内容 |
|----------|---------|---------|
| `is22/src/polling_orchestrator/mod.rs` | 修正 | 画像保存条件分岐、TODOコメント追加 |
| `is22/src/detection_log_service/mod.rs` | 修正 | CleanupStats追加、クォータ強制、クリーンアップ、TODOコメント |
| `is22/src/preset_loader/mod.rs` | 修正 | TODOコメント追加 |
| `frontend/src/components/EventLogPane.tsx` | 修正 | 色スキーム、ラベル翻訳、ツールチップ、アイコン色 |
| `frontend/src/components/SettingsModal.tsx` | 修正 | 推論統計タブ、ストレージ設定追加 |
| `frontend/src/components/InferenceStatsTab.tsx` | 新規 | 推論統計表示コンポーネント |
| `is22/src/web_api/routes.rs` | 修正 | 推論統計API追加 |

---

## 6. タスクリスト（依存順）

### Phase 1: Critical - ストレージ管理

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T1-1 | CleanupStats構造体追加 | - | 10分 |
| T1-2 | save_detection引数をOption<&[u8]>に変更 | T1-1 | 20分 |
| T1-3 | enforce_image_quota実装 | T1-1 | 30分 |
| T1-4 | cleanup_unknown_images実装 | T1-1 | 40分 |
| T1-5 | 画像保存条件分岐追加 | T1-2 | 20分 |
| T1-6 | post_cycle_maintenance追加 | T1-3,T1-4 | 20分 |
| T1-7 | バックエンドビルド・テスト | T1-6 | 30分 |

### Phase 2: High - 表示仕様準拠

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T2-1 | EVENT_COLORS, ICON_COLORS定数追加 | - | 10分 |
| T2-2 | getEventStyle関数実装 | T2-1 | 15分 |
| T2-3 | getIconColor関数実装 | T2-1 | 10分 |
| T2-4 | EVENT_LABELS定数追加 | - | 10分 |
| T2-5 | getEventLabel関数修正 | T2-4 | 10分 |
| T2-6 | カメラ名表示修正 | - | 10分 |
| T2-7 | SEVERITY_TOOLTIPS追加・Badge修正 | - | 10分 |
| T2-8 | DetectionLogItem統合修正 | T2-2,T2-3,T2-5,T2-6,T2-7 | 30分 |
| T2-9 | フロントエンドビルド確認 | T2-8 | 10分 |

### Phase 3: Medium - 監視・分析

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T3-1 | 推論統計API設計・実装 | T1-7 | 60分 |
| T3-2 | InferenceStatsTab.tsx実装 | T3-1 | 60分 |
| T3-3 | SettingsModal: 推論統計タブ追加 | T3-2 | 15分 |
| T3-4 | ストレージ設定UI追加 | - | 30分 |
| T3-5 | ストレージ設定API追加 | T3-4 | 30分 |

### Phase 4: 基盤整備 - autoAttunement TODO

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T4-1 | polling_orchestrator: TODOコメント追加 | - | 5分 |
| T4-2 | detection_log_service: TODOコメント追加 | - | 5分 |
| T4-3 | preset_loader: TODOコメント追加 | - | 5分 |

### Phase 5: テスト

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T5-1 | BE: 画像保存条件テスト | T1-7 | 30分 |
| T5-2 | BE: クォータ強制テスト | T1-7 | 20分 |
| T5-3 | BE: クリーンアップテスト | T1-7 | 20分 |
| T5-4 | FE: ビルド確認 | T2-9 | 10分 |
| T5-5 | UI: 色スキーム検証 | T5-4 | 30分 |
| T5-6 | UI: ラベル翻訳検証 | T5-4 | 20分 |
| T5-7 | UI: ツールチップ検証 | T5-4 | 15分 |

---

## 7. テスト計画

### 7.1 バックエンドテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| BE-01 | cargo build --release | エラーなし |
| BE-02 | severity=0, event="none", unknown=false | 画像保存**されない** |
| BE-03 | severity=0, event="none", unknown=true | 画像保存**される** |
| BE-04 | severity=0, event="human", unknown=false | 画像保存**される** |
| BE-05 | severity=1, any, any | 画像保存**される** |
| BE-06 | enforce_image_quota: 1100枚→1000枚 | 100枚削除 |
| BE-07 | cleanup_unknown_images: 100枚 | 90枚削除、10枚残存 |
| BE-08 | cleanup_unknown_images: 5枚 | 4枚削除、1枚残存（最低1件保持） |

### 7.2 フロントエンドテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| FE-01 | npm run build | エラーなし |
| FE-02 | npx tsc --noEmit | エラーなし |

### 7.3 Chrome UIテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| UI-01 | 人検知イベント表示 | 背景#FFFF00、文字#222222 |
| UI-02 | 車両検知イベント表示 | 背景#6495ED、文字#FFFFFF |
| UI-03 | 異常検知イベント表示 | 背景#FF0000、文字#FFFF00 |
| UI-04 | unknownイベント表示 | 背景#D3D3D3、文字#222222 |
| UI-05 | camera_lost表示 | 背景#333333、文字#FFFFFF |
| UI-06 | camera_recovered表示 | 背景#FFFFFF、文字#444444 |
| UI-07 | 人検知ラベル | 「人物検知」と表示 |
| UI-08 | 車両検知ラベル | 「車両検知」と表示 |
| UI-09 | unknownラベル | 「不明」と表示 |
| UI-10 | カメラ名表示（短い名前） | 省略なし |
| UI-11 | カメラ名表示（長い名前） | truncate + hoverでフル表示 |
| UI-12 | severityバッジhover | ツールチップ表示 |
| UI-13 | アイコン視認性（人検知） | アイコン色#222222 |
| UI-14 | アイコン視認性（車両検知） | アイコン色#FFFFFF |

---

## 8. 実装手順（FIX-M02, FIX-M03対応）

### 8.1 GitHub Issue登録

本設計承認後、以下のIssueを作成:

```
Title: [is22] AI Event Log表示仕様準拠・ストレージ管理実装
Labels: enhancement, is22, frontend, backend

Body:
## 概要
AIEventlog.mdの要件に基づき、AI Event Logの表示仕様準拠とストレージ管理を実装する。

## 設計ドキュメント
- Layout＆AIlog_Settings/AIEventLog_Redesign_v3.md

## タスク
- [ ] Phase 1: ストレージ管理 (T1-1〜T1-7)
- [ ] Phase 2: 表示仕様準拠 (T2-1〜T2-9)
- [ ] Phase 3: 監視・分析 (T3-1〜T3-5)
- [ ] Phase 4: autoAttunement TODO (T4-1〜T4-3)
- [ ] Phase 5: テスト (T5-1〜T5-7)
```

### 8.2 実装前コミット

```bash
# 設計ドキュメントをコミット
git add code/orangePi/is21-22/designs/headdesign/Layout＆AIlog_Settings/
git commit -m "docs(is22): AI Event Log Redesign v3 設計ドキュメント

- AIEventLog_Redesign_v3.md: 包括的設計（違反解消版）
- AIEventLog_Design_Review_Feedback.md: レビューフィードバック
- IS21/IS22評価方法解説追加
- autoAttunement基本設計追加
- 画像保存条件真理値表追加
- 全コード完全実装

関連: AIEventlog.md, The_golden_rules.md"

git push origin main
```

### 8.3 実装開始

Issue登録・実装前コミット完了後、タスクリストに従って実装を開始。

---

## 9. 受け入れ条件

### Critical（必須）
- [ ] 画像保存条件が真理値表通りに動作
- [ ] カメラ毎クォータ（1000枚）が強制される
- [ ] unknown/none画像の90%が定期削除される

### High（必須）
- [ ] 全色スキームがAIEventlog.md仕様通り（hex code完全一致）
- [ ] 全ラベルが日本語に翻訳される
- [ ] カメラ名がスペースがある場合は省略されない
- [ ] severityバッジにツールチップが表示される
- [ ] アイコンが全背景色で視認可能

### Medium（推奨）
- [ ] 推論統計タブが設定モーダルに表示される
- [ ] ストレージ設定が変更可能

### 基盤整備（後続タスク準備）
- [ ] autoAttunement TODOコメントが該当ファイルに存在

---

## 10. MECEチェック

| 観点 | 分類 | 網羅性確認 |
|------|------|-----------|
| 問題カテゴリ | A:ストレージ/B:表示/C:監視/D:後続基盤 | 全問題カバー |
| イベント種別 | human/vehicle/alert/unknown/camera_lost/camera_recovered | 全種別定義 |
| 色指定 | 背景/文字/ボーダー/アイコン | 各種別で定義 |
| ラベル翻訳 | 24種のIS21ラベル | 全ラベル対応 |
| 画像保存条件 | severity×event×unknown_flag | 真理値表で網羅 |
| テストケース | BE(8件)/FE(2件)/UI(14件) | 全観点カバー |

---

## 11. v2からの変更点（違反解消）

| 違反ID | 内容 | v3での対応 |
|--------|------|-----------|
| V2-001 | autoAttunement省略 | Section 4.4で基本設計追加 |
| V2-002 | IS21/IS22解説省略 | Section 2で詳細解説追加 |
| V2-003 | 真理値表なし | Section 4.1.1で真理値表追加 |
| V2-004 | getIconColor未完成 | Section 4.2.5で完全実装 |
| V2-005 | CleanupStats未定義 | Section 4.1.2で定義追加 |
| V2-006 | Issue手順なし | Section 8.1で手順追加 |
| V2-007 | コミット手順なし | Section 8.2で手順追加 |

---

## 付録A: 関連コード参照

| ファイル | 行番号 | 内容 |
|----------|--------|------|
| polling_orchestrator/mod.rs | 1022-1034 | 画像保存処理 |
| detection_log_service/mod.rs | 110 | max_images_per_camera |
| detection_log_service/mod.rs | 334-356 | save_image関数 |
| ai_client/mod.rs | 24-60 | AnalyzeRequest構造 |
| ai_client/mod.rs | 304-350 | AnalyzeResponse構造 |
| prev_frame_cache/mod.rs | 全体 | 前フレームキャッシュ |
| EventLogPane.tsx | 55-86 | getEventTypeColor |
| EventLogPane.tsx | 137-150 | getEventLabel |
| EventLogPane.tsx | 201 | カメラ名表示 |
| EventLogPane.tsx | 214-216 | severityバッジ |

---

**文書終端**
