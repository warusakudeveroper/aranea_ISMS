# AIEventLog_Redesign_v2: AI Event Log包括的設計

文書番号: Layout＆AIlog_Settings/AIEventLog_Redesign_v2
作成日: 2026-01-08
ステータス: 設計確定待ち
関連文書: AIEventlog.md, The_golden_rules.md
前版: AIEventLog_Redesign_v1.md（却下）

---

## 1. 大原則の宣言

本設計は以下の大原則に準拠する:

- [x] **SSoT**: AIEventlog.mdを唯一の情報源として参照
- [x] **SOLID-S**: 各コンポーネントは単一責務
- [x] **MECE**: 全問題を網羅的・相互排他的に分類
- [x] **アンアンビギュアス**: 色コード・条件・動作を曖昧さなく明確に定義
- [x] **車輪の再発明禁止**: 既存コンポーネント構造を維持
- [x] **テスト計画あり**: フロントエンド・バックエンド・Chrome UIテストを定義
- [x] **棚上げ禁止**: 全タスクは本設計で解決（The_golden_rules.md Rule 7準拠）

---

## 2. 現状評価

### 2.1 システム状態の評価

| 評価項目 | 状態 | 説明 |
|---------|------|------|
| AI推論動作 | 動作 | 人検知・車両検知自体は機能 |
| 表示仕様準拠 | **不合格** | 色スキーム・ラベル・ツールチップ全て仕様違反 |
| ストレージ安全性 | **危険** | 画像無制限保存でストレージ枯渇リスク |
| 運用可能性 | 制限付き | 「使えなくはないが意図した機能を果たしていない」 |

### 2.2 問題の重大度分類

| 重大度 | 問題カテゴリ | 影響 |
|--------|-------------|------|
| Critical | ストレージ枯渇 | システム停止リスク |
| High | 表示仕様違反 | 運用品質低下 |
| Medium | ツールチップなし | UX低下 |

---

## 3. 問題分析（MECE構造）

### 3.1 Category A: ストレージ管理問題（Critical）

#### A-1: 画像無制限保存

**現状コード** (`polling_orchestrator/mod.rs:1022-1034`):
```rust
// 全フレームで無条件にsave_detection呼び出し
let log_id = detection_log
    .save_detection(
        tid, fid, lacis_id,
        &result,
        &image_data,  // ← 常に保存
        camera_context.as_ref(),
        ...
    )
    .await?;
```

**問題**:
- `save_detection`が毎ポーリングサイクルで無条件実行
- `severity=0`（検出なし）でも画像保存
- `unknown`判定でも画像保存
- → 巡回毎に全カメラ分の画像が蓄積

#### A-2: クォータ未実装

**現状コード** (`detection_log_service/mod.rs:110`):
```rust
pub max_images_per_camera: usize,  // 1000に設定されているが...
```

**問題**:
- `max_images_per_camera: 1000`が定義されているが**一切使用されていない**
- `save_image()`関数にクォータチェック・ローテーション処理なし
- → ストレージが際限なく肥大化

#### A-3: 不要画像の削除ロジックなし

**仕様要件**（AIEventlog.mdより）:
> 「unknown乱発によって画像の保存ストーム状態になっておりストレージが無限に肥大化するループ」

**対応要件**:
- `unknown`/`none`判定の画像は最新10%のみ保持、残り90%削除
- カメラ毎にクォータ（1000枚）を強制
- 古い画像から自動削除

---

### 3.2 Category B: 表示仕様違反問題（High）

#### B-1: 色スキーム不準拠

**仕様** (AIEventlog.md):
| イベント種別 | 背景色 | 文字色 |
|-------------|--------|--------|
| 人検知 (human) | #FFFF00 (黄) | #222222 |
| 車両検知 (vehicle) | #6495ED (青) | #FFFFFF |
| 異常検知 (alert/suspicious) | #FF0000 (赤) | #FFFF00 |
| 不明 (unknown) | #D3D3D3 (薄グレー) | #222222 |
| カメラロスト (camera_lost) | #333333 (濃グレー) | #FFFFFF |
| カメラ復帰 (camera_recovered) | #FFFFFF (白) | #444444 |

**現状** (`EventLogPane.tsx:55-86`):
```typescript
// Tailwind classで指定 ← 仕様違反
if (event === "human"...) {
  return "bg-yellow-100 dark:bg-yellow-900/60 text-yellow-900..."
}
```

**問題**:
- Tailwindクラス使用で仕様の色コードと一致しない
- `bg-yellow-100`は`#FEF9C3`であり`#FFFF00`ではない

#### B-2: ラベル未翻訳

**仕様要件**（AIEventlog.mdフィードバック）:
> 「ラベルを人間にとって意味のある検知内容特徴を示す用語に変換」

**現状** (`EventLogPane.tsx:137-150`):
```typescript
function getEventLabel(primaryEvent: string, countHint: number) {
  // camera_lost/camera_recovered以外は生の英語ラベルをそのまま表示
  return primaryEvent  // ← "human"がそのまま表示される
}
```

**対応要件**:
| IS21ラベル | 表示ラベル |
|-----------|-----------|
| human/person | 人物検知 |
| humans/people | 複数人検知 |
| vehicle/car | 車両検知 |
| suspicious/alert | 不審行動 |
| fire/smoke | 火災/煙検知 |
| unknown | 不明 |
| none | 検出なし |

#### B-3: カメラ名固定truncate

**仕様** (AIEventlog.md):
> 「スペースがあれば省略しない」

**現状** (`EventLogPane.tsx:201`):
```tsx
<span className="text-[10px] font-medium truncate max-w-[60px]">
  {cameraName}
</span>
```

**問題**:
- `max-w-[60px]`固定で常にtruncate
- スペースに余裕があっても省略される

#### B-4: Severityツールチップなし

**仕様** (AIEventlog.md):
> 「丸数字の意味がわからない。マウスホバーでヒント表示すべき」

**現状** (`EventLogPane.tsx:214-216`):
```tsx
<Badge variant={badgeVariant}>
  {log.severity}  // ← title属性なし
</Badge>
```

#### B-5: アイコン視認性不足

**仕様** (AIEventlog.md):
> 「アイコンがひどくみづらい。背景色を中間色にまとめているせい」

**現状**: アイコン色がTailwindクラスで指定され、仕様背景色との対比が不十分

---

### 3.3 Category C: 監視・分析機能不足（Medium）

#### C-1: 推論統計タブなし

**仕様要件** (AIEventlog.md):
> 「設定モーダルに推論統計のタブを作成し、判定結果の分布と件数、傾向を分析」

#### C-2: ストレージ使用量設定なし

**仕様要件** (AIEventlog.md):
> 「"最大保存容量"などの設定を設けてユーザーの裁量範疇で画像保存」

---

## 4. 詳細設計

### 4.1 IMPL-A: ストレージ管理実装（Critical）

#### 4.1.1 画像保存条件の導入

**ファイル**: `is22/src/polling_orchestrator/mod.rs`

**変更**: 検出結果に基づいて画像保存を条件分岐

```rust
// Before: 無条件保存
let log_id = detection_log.save_detection(..., &image_data, ...).await?;

// After: 条件付き保存
let should_save_image = result.severity > 0
    || result.primary_event != "none"
    || result.unknown_flag;

let image_to_save = if should_save_image {
    Some(&image_data[..])
} else {
    None
};

let log_id = detection_log.save_detection(
    ...,
    image_to_save,  // Option<&[u8]>に変更
    ...
).await?;
```

#### 4.1.2 クォータ強制・ローテーション実装

**ファイル**: `is22/src/detection_log_service/mod.rs`

**新規関数**:
```rust
/// カメラ毎の画像クォータを強制し、古い画像を削除
async fn enforce_image_quota(&self, camera_id: &str) -> Result<()> {
    let camera_dir = self.config.image_base_path.join(camera_id);
    if !camera_dir.exists() {
        return Ok(());
    }

    // 1. 画像ファイル一覧取得（更新日時順ソート）
    let mut entries: Vec<_> = fs::read_dir(&camera_dir)
        .await?
        .filter_map(|e| e.ok())
        .collect();

    // 2. クォータ超過分を削除
    if entries.len() > self.config.max_images_per_camera {
        let excess = entries.len() - self.config.max_images_per_camera;
        // 古い順にexcess件削除
        for entry in entries.iter().take(excess) {
            fs::remove_file(entry.path()).await?;
        }
    }
    Ok(())
}
```

#### 4.1.3 Unknown/None画像クリーンアップ

**ファイル**: `is22/src/detection_log_service/mod.rs`

**新規関数**:
```rust
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
    let keep_count = (total as f64 * 0.10).ceil() as usize; // 最新10%
    let delete_count = total.saturating_sub(keep_count);

    // 2. 削除対象（古い90%）のファイルを削除
    for path in paths.iter().take(delete_count) {
        if let Err(e) = fs::remove_file(path).await {
            tracing::warn!(path = %path, error = %e, "Failed to delete image");
        }
    }

    // 3. DBのimage_path_localをクリア（ファイル削除済み）
    sqlx::query(
        r#"
        UPDATE detection_logs
        SET image_path_local = ''
        WHERE camera_id = ?
          AND (primary_event IN ('unknown', 'none') OR unknown_flag = TRUE)
          AND captured_at < (
            SELECT MIN(captured_at) FROM (
              SELECT captured_at FROM detection_logs
              WHERE camera_id = ?
                AND (primary_event IN ('unknown', 'none') OR unknown_flag = TRUE)
              ORDER BY captured_at DESC
              LIMIT ?
            ) AS keep_set
          )
        "#
    )
    .bind(camera_id)
    .bind(camera_id)
    .bind(keep_count as i64)
    .execute(&self.pool)
    .await?;

    Ok(CleanupStats { total, deleted: delete_count, kept: keep_count })
}
```

#### 4.1.4 定期クリーンアップタスク

**ファイル**: `is22/src/polling_orchestrator/mod.rs`

```rust
// ポーリングサイクル完了後に呼び出し
async fn post_cycle_maintenance(&self) {
    // 100サイクルに1回クリーンアップ実行
    if self.cycle_count % 100 == 0 {
        for camera_id in self.active_cameras.keys() {
            let _ = self.detection_log.enforce_image_quota(camera_id).await;
            let _ = self.detection_log.cleanup_unknown_images(camera_id).await;
        }
    }
}
```

---

### 4.2 IMPL-B: 表示仕様準拠実装（High）

#### 4.2.1 色スキーム定義

**ファイル**: `frontend/src/components/EventLogPane.tsx`

```typescript
// 仕様準拠の色定義
const EVENT_COLORS = {
  human: { bg: '#FFFF00', text: '#222222', border: '#B8B800' },
  vehicle: { bg: '#6495ED', text: '#FFFFFF', border: '#4169E1' },
  alert: { bg: '#FF0000', text: '#FFFF00', border: '#CC0000' },
  unknown: { bg: '#D3D3D3', text: '#222222', border: '#A9A9A9' },
  camera_lost: { bg: '#333333', text: '#FFFFFF', border: '#1A1A1A' },
  camera_recovered: { bg: '#FFFFFF', text: '#444444', border: '#CCCCCC' },
} as const

type EventColorKey = keyof typeof EVENT_COLORS

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

#### 4.2.2 ラベル翻訳定義

**ファイル**: `frontend/src/components/EventLogPane.tsx`

```typescript
// 日本語ラベル翻訳マップ
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

#### 4.2.3 カメラ名表示改善

**ファイル**: `frontend/src/components/EventLogPane.tsx`

```tsx
// Before
<span className="text-[10px] font-medium truncate max-w-[60px]">

// After
<span
  className="text-[10px] font-medium truncate min-w-[40px] max-w-[120px] flex-shrink"
  title={cameraName}
>
```

#### 4.2.4 Severityツールチップ追加

**ファイル**: `frontend/src/components/EventLogPane.tsx`

```typescript
const SEVERITY_TOOLTIPS: Record<number, string> = {
  0: '検出なし',
  1: '低優先度 - 通常の検知',
  2: '中優先度 - 注意が必要',
  3: '高優先度 - 即時確認推奨',
  4: '緊急 - 即時対応必要',
}

// Badge適用
<Badge
  variant={badgeVariant}
  className="text-[9px] px-1 py-0 h-4 min-w-[18px] justify-center cursor-help"
  title={SEVERITY_TOOLTIPS[log.severity] ?? `重要度: ${log.severity}`}
>
  {log.severity}
</Badge>
```

#### 4.2.5 アイコン色対応

**ファイル**: `frontend/src/components/EventLogPane.tsx`

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
  // getEventStyleと同じロジックでcolorKeyを決定
  // ICON_COLORS[colorKey]を返す
}

// Iconコンポーネントにstyle適用
<Icon style={{ color: getIconColor(log.primary_event) }} />
```

---

### 4.3 IMPL-C: 監視・分析機能（Medium）

#### 4.3.1 推論統計タブ追加

**ファイル**: `frontend/src/components/SettingsModal.tsx`

新規タブ「推論統計」を追加:

```tsx
<TabsTrigger value="inference-stats">推論統計</TabsTrigger>

<TabsContent value="inference-stats">
  <InferenceStatsTab />
</TabsContent>
```

**新規コンポーネント**: `frontend/src/components/InferenceStatsTab.tsx`

表示内容:
- カメラ別検出件数
- イベント種別分布
- 時間帯別検出傾向
- 処理時間統計

#### 4.3.2 ストレージ設定追加

**ファイル**: `frontend/src/components/SettingsModal.tsx`（システム設定タブ）

```tsx
<div className="space-y-2">
  <Label>最大画像保存数（カメラ毎）</Label>
  <Input
    type="number"
    value={maxImagesPerCamera}
    onChange={(e) => setMaxImagesPerCamera(Number(e.target.value))}
    min={100}
    max={10000}
  />
  <p className="text-xs text-muted-foreground">
    各カメラの保存画像数上限。超過分は古いものから削除されます。
  </p>
</div>
```

---

## 5. 変更ファイル一覧

| ファイル | 変更種別 | 変更内容 |
|----------|---------|---------|
| `is22/src/polling_orchestrator/mod.rs` | 修正 | 画像保存条件分岐、定期クリーンアップ呼び出し |
| `is22/src/detection_log_service/mod.rs` | 修正 | save_detection引数変更、クォータ強制、クリーンアップ関数追加 |
| `frontend/src/components/EventLogPane.tsx` | 修正 | 色スキーム、ラベル翻訳、カメラ名表示、ツールチップ |
| `frontend/src/components/SettingsModal.tsx` | 修正 | 推論統計タブ、ストレージ設定追加 |
| `frontend/src/components/InferenceStatsTab.tsx` | 新規 | 推論統計表示コンポーネント |
| `is22/src/web_api/routes.rs` | 修正 | 推論統計API追加 |

---

## 6. タスクリスト（依存順）

### Phase 1: Critical - ストレージ管理

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T1-1 | DetectionLogService: save_detection引数をOption<&[u8]>に変更 | - | 20分 |
| T1-2 | DetectionLogService: enforce_image_quota実装 | T1-1 | 30分 |
| T1-3 | DetectionLogService: cleanup_unknown_images実装 | T1-1 | 40分 |
| T1-4 | PollingOrchestrator: 画像保存条件分岐追加 | T1-1 | 20分 |
| T1-5 | PollingOrchestrator: post_cycle_maintenance追加 | T1-2,T1-3 | 20分 |
| T1-6 | バックエンドテスト実行 | T1-5 | 30分 |

### Phase 2: High - 表示仕様準拠

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T2-1 | EventLogPane: EVENT_COLORS定数追加 | - | 10分 |
| T2-2 | EventLogPane: getEventStyle関数実装 | T2-1 | 15分 |
| T2-3 | EventLogPane: EVENT_LABELS定数追加 | - | 10分 |
| T2-4 | EventLogPane: getEventLabel関数修正 | T2-3 | 10分 |
| T2-5 | EventLogPane: カメラ名表示修正 | - | 10分 |
| T2-6 | EventLogPane: SEVERITY_TOOLTIPS追加・Badge修正 | - | 10分 |
| T2-7 | EventLogPane: ICON_COLORS追加・getIconColor実装 | T2-1 | 15分 |
| T2-8 | EventLogPane: DetectionLogItemコンポーネント統合修正 | T2-2,T2-4,T2-5,T2-6,T2-7 | 30分 |
| T2-9 | フロントエンドビルド確認 | T2-8 | 10分 |

### Phase 3: Medium - 監視・分析

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T3-1 | 推論統計API設計・実装 | T1-6 | 60分 |
| T3-2 | InferenceStatsTab.tsx実装 | T3-1 | 60分 |
| T3-3 | SettingsModal: 推論統計タブ追加 | T3-2 | 15分 |
| T3-4 | SettingsModal: ストレージ設定追加 | - | 30分 |
| T3-5 | ストレージ設定API追加 | T3-4 | 30分 |

### Phase 4: テスト

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T4-1 | Chrome UIテスト: 色スキーム検証 | T2-9 | 30分 |
| T4-2 | Chrome UIテスト: ラベル翻訳検証 | T2-9 | 20分 |
| T4-3 | Chrome UIテスト: ツールチップ検証 | T2-9 | 15分 |
| T4-4 | E2Eテスト: ストレージクリーンアップ動作確認 | T1-6 | 30分 |

---

## 7. テスト計画

### 7.1 バックエンドテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| BE-01 | cargo build --release | エラーなし |
| BE-02 | severity=0でsave_detection呼び出し | 画像保存されない |
| BE-03 | unknown判定でsave_detection呼び出し | 画像保存される |
| BE-04 | enforce_image_quota: 1100枚→1000枚 | 100枚削除 |
| BE-05 | cleanup_unknown_images: 100枚unknown | 90枚削除、10枚残存 |

### 7.2 フロントエンドテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| FE-01 | npm run build | エラーなし |
| FE-02 | TypeScript型チェック | エラーなし |

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
| UI-09 | カメラ名表示（短い名前） | 省略なし |
| UI-10 | カメラ名表示（長い名前） | 120px超でtruncate、hoverでフル表示 |
| UI-11 | severityバッジhover | ツールチップ表示（「低優先度」等） |
| UI-12 | アイコン視認性（全種別） | 背景との対比が明確 |

### 7.4 テスト実行環境

- バックエンド: is22サーバー（192.168.125.246）
- フロントエンド: http://192.168.125.246:3000/
- 確認方法: スクリーンショット + 開発者ツールCSS確認

---

## 8. 受け入れ条件

### Critical（必須）
- [ ] 画像保存がseverity>0またはunknown_flagの場合のみに限定
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

---

## 9. MECEチェック

| 観点 | 分類 | 網羅性確認 |
|------|------|-----------||
| 問題カテゴリ | A:ストレージ/B:表示/C:監視 | 全問題カバー |
| イベント種別 | human/vehicle/alert/unknown/camera_lost/camera_recovered | 全種別定義 |
| 色指定 | 背景/文字/ボーダー | 各種別で定義 |
| ラベル翻訳 | 20種以上のIS21ラベル | 主要ラベル全て |
| 表示要素 | アイコン/カメラ名/ラベル/時刻/severity/confidence/tags | 全要素対応 |
| ストレージ操作 | 保存条件/クォータ/クリーンアップ | 全操作定義 |

---

## 10. 補足: v1からの変更点

| 項目 | v1 | v2 |
|------|----|----|
| 「将来拡張」記述 | あり（Phase 2として後回し） | なし（全タスク本設計で解決） |
| ラベル翻訳 | なし | あり（20種以上定義） |
| ストレージ管理 | 言及のみ | 詳細実装設計あり |
| クリーンアップ | なし | unknown/none 90%削除実装 |
| 粒度 | 低（概要レベル） | 高（コードスニペット付き） |

---

## 付録A: 関連コード参照

### 問題箇所
- `polling_orchestrator/mod.rs:1022-1034` - 無条件画像保存
- `detection_log_service/mod.rs:110` - 未使用のmax_images_per_camera
- `detection_log_service/mod.rs:334-356` - クォータなしsave_image
- `EventLogPane.tsx:55-86` - Tailwind色クラス使用
- `EventLogPane.tsx:137-150` - ラベル未翻訳
- `EventLogPane.tsx:201` - 固定truncate
- `EventLogPane.tsx:214-216` - ツールチップなし

---

**文書終端**
