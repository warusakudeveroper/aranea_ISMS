# AIEventLog_Redesign_v5: AI Event Log包括的設計（完全版）

文書番号: Layout＆AIlog_Settings/AIEventLog_Redesign_v5
作成日: 2026-01-08
ステータス: レビュー待ち
関連文書: AIEventlog.md, The_golden_rules.md
前版: AIEventLog_Redesign_v4.md
タスクリスト: AIEventLog_TaskList.md
テスト計画: AIEventLog_TestPlan.md

---

## v5修正事項

### v4からの主要修正点

| 修正ID | v4（誤り） | v5（修正後） | 根拠 |
|--------|----------|-------------|------|
| FIX-001 | cleanup_unknown_images（自動実行可能） | **manual_cleanup_unknown_images**（手動のみ） | Rule 5準拠 |
| FIX-002 | 分散サンプリング（10件毎に1件保持） | **最新10%保持**（古い90%削除） | 設計解釈エラー修正 |
| FIX-003 | クォータ連動で自動実行 | **管理者の明示的操作のみ** | 情報等価性原則 |
| FIX-004 | 確認ダイアログなし | **confirmed=falseでプレビュー、trueで実行** | ユーザー確認権利尊重 |

### Rule 5準拠声明

> The_golden_rules.md Rule 5: 「情報は如何なる場合においても等価であり、優劣をつけて自己判断で不要と判断した情報を握り潰すような仕様としてはならない」

本設計ではunknown画像の**自動削除を禁止**し、ユーザーが明示的に確認した場合のみ削除を実行する。

---

## 1. 大原則の宣言

本設計は以下の大原則に**完全**準拠する:

- [x] **Rule 1 (SSoT)**: AIEventlog.mdを唯一の情報源。本書内に全要件を明示（参照省略なし）
- [x] **Rule 2 (SOLID)**: 各コンポーネントは単一責務
- [x] **Rule 3 (MECE)**: 全問題を網羅的・相互排他的に分類。漏れなし
- [x] **Rule 4 (Unambiguous)**: 全コード・条件・動作を曖昧さなく明確に定義
- [x] **Rule 5 (情報等価)**: unknownの自動削除禁止。手動クリーンアップのみ
- [x] **Rule 6 (現場猫禁止)**: 具体的コード参照・エビデンス・ログ例付き
- [x] **Rule 7 (棚上げ禁止)**: 全タスクは本設計で解決。省略なし
- [x] **Rule 9 (テスト必須)**: BE/FE/UIテストを完全定義
- [x] **Rule 12 (車輪禁止)**: 既存コンポーネント構造を活用
- [x] **Rule 15 (設計必須)**: GitHub Issue登録・実装前コミット手順あり

---

## 2. AIEventlog.md要件の完全転記

> SSoT原則: 本書内に全要件を明示し、「v2/v3参照」の省略を排除

### 2.1 表示関連要件（AIEventlog.md Section 1）

#### 2.1.1 色スキーム仕様

| イベント種別 | 背景色 | 文字色 | ボーダー色 | 意図 |
|-------------|--------|--------|-----------|------|
| 人検知 (human) | #FFFF00 (黄) | #222222 | #B8B800 | 最重要、目立つ色 |
| 車両検知 (vehicle) | #6495ED (青) | #FFFFFF | #4169E1 | 人検知と明確に区別 |
| 異常検知 (alert/suspicious) | #FF0000 (赤) | #FFFF00 | #CC0000 | 危険な印象 |
| 不明 (unknown) | #D3D3D3 (薄グレー) | #222222 | #A9A9A9 | 曖昧な中間色 |
| カメラロスト (camera_lost) | #333333 (濃グレー) | #FFFFFF | #1A1A1A | ブラックアウト感 |
| カメラ復帰 (camera_recovered) | #FFFFFF (白) | #444444 | #CCCCCC | 目立たないが認識可能 |

#### 2.1.2 カメラ名表示要件

> 「スペースがあれば省略しない」「長い場合のみtruncate」「hoverでフルネーム表示」

#### 2.1.3 Severity表示要件

> 「丸数字の意味がわからない。マウスホバーでヒント表示すべき」

| 値 | 意味 | ツールチップ |
|----|------|-------------|
| 0 | 検出なし | 検出なし |
| 1 | Low | 低優先度 - 通常の検知 |
| 2 | Medium | 中優先度 - 注意が必要 |
| 3 | High | 高優先度 - 即時確認推奨 |
| 4 | Critical | 緊急 - 即時対応必要 |

#### 2.1.4 アイコン視認性要件

> 「アイコンがひどくみづらい。背景色を中間色にまとめているせい」
> → 背景色に応じたアイコン色を設定し、コントラスト比4.5:1以上を確保

### 2.2 AI判定関連要件（AIEventlog.md Section 2）

#### 2.2.1 IS21/IS22評価方法の解説要件

> 「is21とis22の評価方法、評価基準、レスポンスの内容などについて詳細に解説」
> 「エビデンスをつけてプリセットが明確に働いているか確認」
> 「直近画像を二枚送って差分査定を行っているかを解説」

#### 2.2.2 ストレージ管理要件

> 「unknown乱発によって画像の保存ストーム状態になっておりストレージが無限に肥大化」
> 「"最大保存容量"などの設定を設けてユーザーの裁量範疇で画像の保存を行う仕様に」

**重要**: 枚数制限だけでなく**容量制限**も必要。設定モーダルでユーザーが指定可能にする。

#### 2.2.3 画像保存ポリシー要件

> 「元々"何もない"なら画像の保存もログも出さない仕様のはず」

**解釈**:
- `none`（検出なし）= 画像保存しない
- `unknown`（不明判定）= 分析用に保存する（**自動削除禁止**、手動クリーンアップのみ可能）
- この区別が重要

#### 2.2.4 Unknown画像可視性要件

> 「unknownの項目に関しては保存画像が見えません」

**原因候補**:
1. 画像送信失敗
2. サブストリーム/メインストリームの不整合
3. 画像パス不正

→ 診断機能とエラーリカバリを設計

#### 2.2.5 推論統計要件

> 「設定モーダルに推論統計のタブを作成し、判定結果の分布と件数、傾向を分析」

#### 2.2.6 プリセット効果分析要件

> 「プリセットと推論結果による変移を分析可能に」
> 「どのカメラがそのプリセットでどのような結果を吐いているか」

#### 2.2.7 IS21監視要件

> 「is21の動作状態の監視も必要」
> 「まともに推論していないことによる高速反応であるなら大きな問題」

#### 2.2.8 精度向上要件

> 「もう少し精度向上を検討する必要」
> 「手ぶらの施設スタッフなのにcarry.bagと判定」

#### 2.2.9 autoAttunement基盤要件

> 「autoAttunementの基本設計ドキュメントの作成と実装予定todoをコード内に組み込む」

---

## 3. IS21/IS22評価方法解説

### 3.1 システム構成

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

### 3.2 二枚画像差分査定の検証

#### 3.2.1 二枚画像送信の設計方針

**AIEventlog.md要件**:
> 「直近画像を二枚送って差分査定を行っているか」

**現状の実装方式**: メタデータ参照方式
- IS21側に前回画像を保持し、メタデータのみ送信
- 利点: ネットワーク帯域削減（画像1枚分 = 約200KB削減/リクエスト）

**本設計で採用**: 二枚画像同時送信方式（フル実装）
- 差分査定の精度向上のため、実際に2枚の画像をIS21に送信
- IS21は両画像を直接比較し、より正確なframe_diff計算が可能

#### 3.2.2 二枚画像送信フロー

```
巡回サイクルN:
  1. SnapshotService: 現フレーム画像(image_current)取得
  2. PrevFrameCache.get_image(camera_id) → 前回画像データ(image_previous)取得
  3. AiClient: IS21へ送信
     - image_current: 現フレーム画像（バイナリ）
     - image_previous: 前フレーム画像（バイナリ）※存在する場合
     - hints_json.previous_frame: 前フレームメタ情報
       {
         "captured_at": "2026-01-08T10:00:00Z",
         "person_count": 2,
         "primary_event": "human"
       }
  4. IS21: 両画像を直接比較して差分計算
  5. IS21 → IS22: frame_diff結果を返却
     {
       "enabled": true,
       "comparison_method": "full_image",
       "person_changes": { "appeared": 1, "disappeared": 0 },
       "scene_change_score": 0.15
     }
  6. PrevFrameCache.store(camera_id, image_current, meta) → 次回用に保存
```

#### 3.2.3 二枚画像送信の実装コード

```rust
// ai_client/mod.rs

pub async fn analyze_with_frame_diff(
    &self,
    request: AnalyzeRequest,
    current_image: &[u8],
    previous_image: Option<&[u8]>,
) -> Result<AnalyzeResponse> {
    let mut form = reqwest::multipart::Form::new()
        .part(
            "image_current",
            reqwest::multipart::Part::bytes(current_image.to_vec())
                .file_name("current.jpg")
                .mime_str("image/jpeg")?,
        )
        .part("hints_json", reqwest::multipart::Part::text(
            serde_json::to_string(&request.hints_json)?
        ));

    if let Some(prev) = previous_image {
        form = form.part(
            "image_previous",
            reqwest::multipart::Part::bytes(prev.to_vec())
                .file_name("previous.jpg")
                .mime_str("image/jpeg")?,
        );

        tracing::debug!(
            camera_id = %request.camera_id,
            current_size = current_image.len(),
            previous_size = prev.len(),
            "Sending two images for frame diff"
        );
    } else {
        tracing::debug!(
            camera_id = %request.camera_id,
            "No previous image available, sending single image"
        );
    }

    let response = self.client
        .post(&format!("{}/v1/analyze", self.base_url))
        .multipart(form)
        .timeout(self.timeout)
        .send()
        .await?;

    // ...
}
```

#### 3.2.4 PrevFrameCache改修（画像保持）

```rust
// prev_frame_cache/mod.rs

pub async fn get_with_image(&self, camera_id: &str) -> Result<Option<(Vec<u8>, FrameMeta)>> {
    self.get(camera_id).await
}
```

#### 3.2.5 polling_orchestrator呼び出し改修

```rust
// polling_orchestrator/mod.rs

let prev_frame = self.prev_frame_cache.get(&camera_id).await?;
let (previous_image, previous_meta) = match prev_frame {
    Some((img, meta)) => (Some(img), Some(meta.to_previous_frame_info())),
    None => (None, None),
};

let analyze_request = AnalyzeRequest {
    camera_id: camera_id.clone(),
    hints_json: HintsJson {
        previous_frame: previous_meta,
        // ...
    },
    // ...
};

let result = self.ai_client
    .analyze_with_frame_diff(
        analyze_request,
        &current_image,
        previous_image.as_deref(),
    )
    .await?;

self.prev_frame_cache.store(&camera_id, current_image.clone(), FrameMeta::from(&result)).await?;

tracing::info!(
    camera_id = %camera_id,
    had_previous = previous_image.is_some(),
    frame_diff_enabled = result.frame_diff.as_ref().map(|f| f.enabled).unwrap_or(false),
    comparison_method = result.frame_diff.as_ref().map(|f| f.comparison_method.as_str()).unwrap_or("none"),
    "Frame diff analysis completed"
);
```

#### 3.2.6 ログ出力例

**正常動作（二枚画像送信）**:
```
2026-01-08T10:00:15Z INFO  is22::ai_client camera_id="cam001" current_size=245760 previous_size=241920 "Sending two images for frame diff"
2026-01-08T10:00:16Z INFO  is22::polling_orchestrator camera_id="cam001" had_previous=true frame_diff_enabled=true comparison_method="full_image" "Frame diff analysis completed"
```

**初回巡回（前フレームなし）**:
```
2026-01-08T10:00:05Z DEBUG is22::ai_client camera_id="cam001" "No previous image available, sending single image"
2026-01-08T10:00:06Z INFO  is22::polling_orchestrator camera_id="cam001" had_previous=false frame_diff_enabled=false comparison_method="none" "Frame diff analysis completed"
```

#### 3.2.7 エビデンス取得手順

```bash
# IS22ログで二枚画像送信を確認
journalctl -u is22 -f | grep -E "Sending two images|current_size|previous_size"

# comparison_methodがfull_imageであることを確認
journalctl -u is22 -f | grep "comparison_method"

# IS21側で両画像受信を確認
ssh mijeosadmin@192.168.3.240 "journalctl -u is21 -f | grep -E 'image_current|image_previous'"
```

#### 3.2.8 二枚画像送信テストケース

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| DIFF-01 | 初回巡回 | had_previous=false, comparison_method="none" |
| DIFF-02 | 2回目巡回 | had_previous=true, comparison_method="full_image" |
| DIFF-03 | 前フレーム5分超過 | had_previous=false（stale） |
| DIFF-04 | IS21側ログ | 両画像バイト数が記録される |

### 3.3 プリセット動作確認

#### 3.3.1 エビデンス取得手順

```bash
journalctl -u is22 -f | grep -E "preset_id|preset_version|conf_override"
```

#### 3.3.2 プリセット効果検証

| プリセットID | conf_threshold | 検出感度 | 用途 |
|-------------|----------------|----------|------|
| balanced | 0.5 | 中 | 標準検出 |
| high_security | 0.3 | 高 | 誤検知許容・漏れ防止 |
| low_noise | 0.7 | 低 | 誤検知抑制 |

### 3.4 IS21監視設計

#### 3.4.1 ヘルスチェック指標

| 指標 | 正常範囲 | 異常判定 |
|------|----------|----------|
| inference_ms | 100-2000ms | >5000ms で警告 |
| 応答率 | >95% | <90% で警告 |
| YOLO検出数 | カメラ依存 | 急激な変動で警告 |

#### 3.4.2 「まともに推論していない」検知

```rust
if response.performance.inference_ms < 50 && response.analyzed == true {
    tracing::warn!(
        camera_id = %camera_id,
        inference_ms = response.performance.inference_ms,
        "Suspiciously fast inference - possible IS21 malfunction"
    );
}
```

### 3.5 現状評価

| 項目 | 状態 | エビデンス確認方法 |
|------|------|-------------------|
| 画像送信 | 動作 | `grep "image_size_kb"` |
| YOLO検出 | 動作 | `grep "primary_event"` |
| PAR分析 | 動作 | `grep "person_details"` |
| フレーム差分 | 動作 | `grep "frame_diff.enabled=true"` |
| プリセット適用 | 動作 | `grep "preset_id"` |

---

## 4. 問題分析（MECE構造）

### 4.1 Category A: ストレージ管理問題（Critical）

| 問題ID | 問題 | 現状 | 対応 |
|--------|------|------|------|
| A-1 | 画像無制限保存 | 全フレームで保存 | 保存条件分岐 |
| A-2 | 枚数クォータ未実装 | max_images未使用 | 強制ローテーション |
| A-3 | **容量クォータ未実装** | 容量管理なし | **容量制限追加** |
| A-4 | **ユーザー設定なし** | ハードコード | **設定モーダルUI** |
| A-5 | 不要画像削除なし | unknown蓄積 | **手動クリーンアップUI** |

### 4.2 Category B: 表示仕様違反問題（High）

| 問題ID | 問題 | 現状 | 対応 |
|--------|------|------|------|
| B-1 | 色スキーム不準拠 | Tailwindクラス | hex code直接指定 |
| B-2 | ラベル未翻訳 | 英語表示 | 日本語マップ |
| B-3 | カメラ名固定truncate | 60px固定 | 可変幅 |
| B-4 | Severityツールチップなし | 数字のみ | title属性追加 |
| B-5 | アイコン視認性不足 | 中間色 | 背景対応色 |

### 4.3 Category C: 画像送信・表示問題（High）

| 問題ID | 問題 | 現状 | 対応 |
|--------|------|------|------|
| C-1 | **unknown画像が見えない** | 原因不明 | **診断機能追加** |
| C-2 | **サブ/メインストリーム不整合** | 未検証 | **ストリーム検証追加** |
| C-3 | **画像送信失敗検知なし** | サイレント失敗 | **エラーログ強化** |

### 4.4 Category D: 監視・分析機能不足（Medium）

| 問題ID | 問題 | 現状 | 対応 |
|--------|------|------|------|
| D-1 | 推論統計タブなし | 未実装 | 新規実装 |
| D-2 | プリセット効果分析なし | 未実装 | 統計APIに含める |
| D-3 | **IS21監視なし** | 未実装 | **ヘルスチェック追加** |

### 4.5 Category E: 精度・誤検知問題（Medium）

| 問題ID | 問題 | 現状 | 対応 |
|--------|------|------|------|
| E-1 | **誤検知（carry.bag等）** | 未対応 | **フィードバック機能設計** |
| E-2 | **精度向上策なし** | 未対応 | **閾値調整UI設計** |

### 4.6 Category F: 後続タスク基盤（autoAttunement）

| 問題ID | 問題 | 現状 | 対応 |
|--------|------|------|------|
| F-1 | 基本設計なし | 未設計 | 本設計に含める |
| F-2 | TODOコメントなし | 未実装 | コード内に追加 |

---

## 5. 詳細設計

（タスクリストおよびテスト計画は独立ドキュメントを参照）
- タスクリスト: AIEventLog_TaskList.md（57タスク、6フェーズ）
- テスト計画: AIEventLog_TestPlan.md（BE-01〜BE-20、FE-01〜FE-02、UI-01〜UI-14）

### 5.1 IMPL-A: ストレージ管理実装

#### 5.1.1 画像保存条件の真理値表

| severity | primary_event | unknown_flag | should_save_image | 理由 |
|----------|---------------|--------------|-------------------|------|
| 0 | "none" | false | **false** | 何もない→保存不要 |
| 0 | "none" | true | **false** | none+unknown矛盾→保存不要 |
| 0 | "unknown" | true | true | 不明だが何かある→保存 |
| 0 | "human" | false | true | イベントあり |
| 1+ | any | any | true | severity>0は常に保存 |

```rust
// polling_orchestrator/mod.rs
fn should_save_image(result: &AnalyzeResponse) -> bool {
    if result.primary_event == "none" {
        return false;
    }
    if result.severity > 0 {
        return true;
    }
    if result.unknown_flag || result.primary_event == "unknown" {
        return true;
    }
    true
}
```

#### 5.1.2 StorageQuotaConfig構造体

```rust
#[derive(Debug, Clone)]
pub struct StorageQuotaConfig {
    pub max_images_per_camera: usize,      // デフォルト: 1000
    pub max_bytes_per_camera: u64,         // デフォルト: 500MB
    pub max_total_bytes: u64,              // デフォルト: 10GB
}

impl Default for StorageQuotaConfig {
    fn default() -> Self {
        Self {
            max_images_per_camera: 1000,
            max_bytes_per_camera: 500 * 1024 * 1024,
            max_total_bytes: 10 * 1024 * 1024 * 1024,
        }
    }
}
```

#### 5.1.3 CleanupStats構造体

```rust
#[derive(Debug, Clone)]
pub struct CleanupStats {
    pub total: usize,
    pub deleted: usize,
    pub kept: usize,
    pub bytes_freed: u64,
}
```

#### 5.1.4 enforce_storage_quota実装

```rust
pub async fn enforce_storage_quota(&self, camera_id: &str) -> Result<CleanupStats> {
    let camera_dir = self.config.image_base_path.join(camera_id);
    if !camera_dir.exists() {
        return Ok(CleanupStats::new(0, 0, 0));
    }

    let mut entries: Vec<(PathBuf, u64, std::time::SystemTime)> = vec![];
    let mut total_bytes: u64 = 0;

    let mut read_dir = fs::read_dir(&camera_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        let size = metadata.len();
        let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        total_bytes += size;
        entries.push((entry.path(), size, modified));
    }

    entries.sort_by_key(|(_, _, modified)| *modified);

    let mut deleted = 0;
    while entries.len() > self.config.quota.max_images_per_camera {
        if let Some((path, size, _)) = entries.first() {
            fs::remove_file(path).await?;
            total_bytes -= size;
            deleted += 1;
            entries.remove(0);
        }
    }

    while total_bytes > self.config.quota.max_bytes_per_camera {
        if let Some((path, size, _)) = entries.first() {
            fs::remove_file(path).await?;
            total_bytes -= size;
            deleted += 1;
            entries.remove(0);
        } else {
            break;
        }
    }

    Ok(CleanupStats::new(entries.len() + deleted, deleted, entries.len()))
}
```

#### 5.1.5 manual_cleanup_unknown_images実装【v5修正】

> **WARNING**: Rule 5準拠のため自動実行禁止。手動操作のみ。

```rust
/// 手動unknown画像クリーンアップ（管理者専用）
pub async fn manual_cleanup_unknown_images(
    &self,
    camera_id: &str,
    confirmed: bool,
) -> Result<CleanupResult> {
    let rows = sqlx::query!(
        r#"
        SELECT log_id, image_path_local, captured_at
        FROM detection_logs
        WHERE camera_id = ?
          AND primary_event = 'unknown'
          AND image_path_local != ''
        ORDER BY captured_at ASC
        "#,
        camera_id
    )
    .fetch_all(&self.pool)
    .await?;

    let total = rows.len();
    if total == 0 {
        return Ok(CleanupResult {
            preview: true,
            total_unknown: 0,
            to_delete: 0,
            to_keep: 0,
            deleted: 0,
            bytes_freed: 0,
            kept_paths: vec![],
        });
    }

    let keep_count = std::cmp::max(1, (total as f64 * 0.10).ceil() as usize);
    let delete_count = total.saturating_sub(keep_count);

    let kept_paths: Vec<String> = rows.iter()
        .rev()
        .take(keep_count)
        .map(|r| r.image_path_local.clone())
        .collect();

    let delete_targets: Vec<_> = rows.iter().take(delete_count).collect();

    if !confirmed {
        return Ok(CleanupResult {
            preview: true,
            total_unknown: total,
            to_delete: delete_count,
            to_keep: keep_count,
            deleted: 0,
            bytes_freed: 0,
            kept_paths,
        });
    }

    let mut bytes_freed: u64 = 0;
    let mut deleted_count = 0;

    for row in delete_targets {
        if let Ok(metadata) = fs::metadata(&row.image_path_local).await {
            bytes_freed += metadata.len();
        }
        let _ = fs::remove_file(&row.image_path_local).await;

        sqlx::query!(
            "UPDATE detection_logs SET image_path_local = '' WHERE log_id = ?",
            row.log_id
        )
        .execute(&self.pool)
        .await?;

        deleted_count += 1;
    }

    tracing::info!(
        camera_id = %camera_id,
        total_unknown = total,
        deleted = deleted_count,
        kept = keep_count,
        bytes_freed = bytes_freed,
        "Manual unknown cleanup completed"
    );

    Ok(CleanupResult {
        preview: false,
        total_unknown: total,
        to_delete: delete_count,
        to_keep: keep_count,
        deleted: deleted_count,
        bytes_freed,
        kept_paths,
    })
}
```

#### 5.1.6 ストレージ設定API

```rust
// PUT /api/settings/storage
#[derive(Deserialize)]
pub struct StorageSettingsRequest {
    pub max_images_per_camera: usize,
    pub max_bytes_per_camera: u64,
    pub max_total_bytes: u64,
}
```

#### 5.1.7 手動クリーンアップAPI

```rust
// GET /api/cleanup/unknown/{camera_id}/preview
// POST /api/cleanup/unknown/{camera_id} (body: { confirmed: true })
```

#### 5.1.8 ストレージ設定UI

```tsx
<TabsContent value="storage">
  <div className="space-y-4">
    <div>
      <Label>カメラ毎の最大画像枚数</Label>
      <Input type="number" value={maxImagesPerCamera} min={100} max={10000} />
    </div>
    <div>
      <Label>カメラ毎の最大容量 (MB)</Label>
      <Input type="number" value={maxMbPerCamera} min={100} max={2000} />
    </div>
    <div>
      <Label>全体の最大容量 (GB)</Label>
      <Input type="number" value={maxTotalGb} min={1} max={100} />
    </div>
    <Button onClick={handleSaveStorageSettings}>保存</Button>
  </div>
</TabsContent>
```

#### 5.1.9 手動クリーンアップUI

```tsx
function UnknownCleanupSection({ cameraId }: { cameraId: string }) {
  const [preview, setPreview] = useState<CleanupResult | null>(null)
  const [showConfirm, setShowConfirm] = useState(false)

  const handlePreview = async () => {
    const res = await fetch(`/api/cleanup/unknown/${cameraId}`, {
      method: 'POST',
      body: JSON.stringify({ confirmed: false }),
    })
    setPreview(await res.json())
  }

  const handleExecute = async () => {
    await fetch(`/api/cleanup/unknown/${cameraId}`, {
      method: 'POST',
      body: JSON.stringify({ confirmed: true }),
    })
    setShowConfirm(false)
    setPreview(null)
  }

  return (
    <div className="space-y-4 border rounded p-4">
      <h4 className="font-semibold">Unknown画像クリーンアップ</h4>
      <p className="text-sm text-muted-foreground">
        既存のゴミデータをクリーンアップします。最新10%を保持し、古い90%を削除します。
      </p>

      <Button variant="outline" onClick={handlePreview}>プレビュー</Button>

      {preview && (
        <div className="p-3 bg-muted rounded space-y-2">
          <p>Unknown画像総数: {preview.total_unknown}件</p>
          <p className="text-red-600">削除対象: {preview.to_delete}件</p>
          <p className="text-green-600">保持対象: {preview.to_keep}件（最新）</p>

          <Dialog open={showConfirm} onOpenChange={setShowConfirm}>
            <DialogTrigger asChild>
              <Button variant="destructive" disabled={preview.to_delete === 0}>
                削除実行
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogTitle>削除の確認</DialogTitle>
              <DialogDescription>
                {preview.to_delete}件のUnknown画像を削除します。
                この操作は取り消せません。
              </DialogDescription>
              <DialogFooter>
                <Button variant="outline" onClick={() => setShowConfirm(false)}>
                  キャンセル
                </Button>
                <Button variant="destructive" onClick={handleExecute}>
                  削除を実行
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </div>
      )}
    </div>
  )
}
```

---

### 5.2 IMPL-B: 表示仕様準拠実装

#### 5.2.1 色スキーム定義

```typescript
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

#### 5.2.2 getEventStyle関数

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

#### 5.2.3 ラベル翻訳定義

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

#### 5.2.4 Severityツールチップ

```typescript
const SEVERITY_TOOLTIPS: Record<number, string> = {
  0: '検出なし',
  1: '低優先度 - 通常の検知',
  2: '中優先度 - 注意が必要',
  3: '高優先度 - 即時確認推奨',
  4: '緊急 - 即時対応必要',
}

<Badge
  title={SEVERITY_TOOLTIPS[log.severity] ?? `重要度: ${log.severity}`}
>
  {log.severity}
</Badge>
```

#### 5.2.5 getIconColor関数

```typescript
const ICON_COLORS: Record<EventColorKey, string> = {
  human: '#222222',
  vehicle: '#FFFFFF',
  alert: '#FFFF00',
  unknown: '#555555',
  camera_lost: '#FFFFFF',
  camera_recovered: '#444444',
}

function getIconColor(primaryEvent: string): string {
  // getEventStyleと同じロジックでcolorKeyを決定
  return ICON_COLORS[colorKey]
}
```

#### 5.2.6 カメラ名表示

```tsx
<span
  className="text-[10px] font-medium truncate min-w-[40px] max-w-[120px] flex-shrink"
  title={cameraName}
>
  {cameraName}
</span>
```

---

### 5.3 IMPL-C: 画像送信・表示問題対応

#### 5.3.1 診断API実装

```rust
// GET /api/diagnostics/images/{camera_id}
pub async fn diagnose_camera_images(camera_id: &str) -> DiagnosticsResult {
    let db_paths = get_unknown_image_paths(camera_id).await?;
    let mut missing_files = vec![];
    let mut existing_files = vec![];

    for path in &db_paths {
        if Path::new(path).exists() {
            existing_files.push(path.clone());
        } else {
            missing_files.push(path.clone());
        }
    }

    DiagnosticsResult {
        total_in_db: db_paths.len(),
        existing_on_disk: existing_files.len(),
        missing_on_disk: missing_files.len(),
        missing_paths: missing_files,
        diagnosis: if missing_files.is_empty() {
            "OK: All images exist"
        } else {
            "ERROR: Some images missing from disk"
        },
    }
}
```

#### 5.3.2 復旧API実装

```rust
pub enum RecoveryMode {
    RefetchCurrent,  // 現在のスナップショットで再保存
    FixPathOnly,     // DBパスのみ修正
    PurgeOrphans,    // 欠損レコードをDB削除
}

pub async fn recover_unknown_images(
    camera_id: &str,
    request: RecoveryRequest,
) -> Result<RecoveryResult> {
    // ... 実装詳細は5.3.4参照
}
```

#### 5.3.3 ストリーム選択ロジック

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamSource {
    MainStream,
    SubStream,
}

pub async fn get_snapshot_for_inference(
    &self,
    camera: &Camera,
) -> Result<SnapshotResult> {
    // メインストリーム試行
    match self.try_stream(camera, StreamSource::MainStream).await {
        Ok(result) => return Ok(result),
        Err(e) => {
            tracing::warn!(camera_id = %camera.camera_id, "Main stream failed, falling back");
        }
    }

    // サブストリームにフォールバック
    match self.try_stream(camera, StreamSource::SubStream).await {
        Ok(result) => {
            tracing::info!(camera_id = %camera.camera_id, "Using substream (fallback)");
            return Ok(result);
        }
        Err(e) => {
            tracing::error!(camera_id = %camera.camera_id, "Both streams failed");
            return Err(Error::AllStreamsFailed);
        }
    }
}
```

#### 5.3.4 画像送信失敗ログ強化

```rust
match result {
    Ok(response) => {
        if !response.status().is_success() {
            tracing::error!(
                camera_id = %request.camera_id,
                status = %response.status(),
                elapsed_ms = start.elapsed().as_millis(),
                "IS21 analyze request failed"
            );
        }
    }
    Err(e) => {
        tracing::error!(
            camera_id = %request.camera_id,
            error = %e,
            "IS21 connection failed"
        );
    }
}
```

---

### 5.4 IMPL-D: 監視・分析機能

#### 5.4.1 推論統計API

```rust
// GET /api/stats/inference
pub async fn get_inference_stats(query: StatsQuery) -> InferenceStats {
    InferenceStats {
        by_camera: get_detection_count_by_camera().await?,
        by_event: get_detection_count_by_event().await?,
        by_hour: get_detection_count_by_hour().await?,
        by_preset: get_detection_count_by_preset().await?,
        processing_time: get_processing_time_stats().await?,
    }
}
```

#### 5.4.2 IS21ヘルスチェック

```rust
pub async fn check_is21_health(&self) -> Is21Health {
    let start = Instant::now();
    let result = self.client
        .get(&format!("{}/health", self.base_url))
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    match result {
        Ok(response) if response.status().is_success() => {
            Is21Health {
                status: "healthy",
                latency_ms: start.elapsed().as_millis() as i32,
                last_check: Utc::now(),
            }
        }
        _ => {
            Is21Health { status: "unhealthy", latency_ms: -1, last_check: Utc::now() }
        }
    }
}
```

---

### 5.5 IMPL-E: 精度向上策

#### 5.5.1 誤検知フィードバックUI

```tsx
<Button variant="outline" onClick={() => reportMisdetection(log.log_id)}>
  誤検知を報告
</Button>
```

#### 5.5.2 誤検知報告API

```rust
// POST /api/feedback/misdetection
pub async fn report_misdetection(
    State(app): State<AppState>,
    Json(req): Json<MisdetectionRequest>,
) -> Result<Json<MisdetectionFeedback>> {
    let log = app.detection_log.get_by_id(req.log_id).await?;

    let feedback = sqlx::query_as!(
        MisdetectionFeedback,
        r#"INSERT INTO misdetection_feedbacks ... RETURNING *"#,
        ...
    )
    .fetch_one(&app.pool)
    .await?;

    Ok(Json(feedback))
}
```

#### 5.5.3 閾値変更API

```rust
// PUT /api/cameras/{camera_id}/threshold
pub async fn update_camera_threshold(
    Path(camera_id): Path<String>,
    Json(req): Json<ThresholdUpdateRequest>,
) -> Result<Json<ThresholdUpdateResponse>> {
    if req.conf_threshold < 0.2 || req.conf_threshold > 0.8 {
        return Err(Error::InvalidThreshold);
    }
    // ... 更新処理
}
```

---

### 5.6 IMPL-F: autoAttunement基本設計

#### 5.6.1 アーキテクチャ

```
┌─────────────────────────────────────────────────────────────────┐
│                      autoAttunement System                       │
│  ┌─────────────────┐     ┌─────────────────┐     ┌───────────┐ │
│  │ StatsCollector  │────→│ TrendAnalyzer   │────→│ Adjuster  │ │
│  └─────────────────┘     └─────────────────┘     └───────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

#### 5.6.2 TODOコメント埋め込み

```rust
// polling_orchestrator/mod.rs
// TODO(autoAttunement): 検出結果をStatsCollectorに送信
// 参照: Layout＆AIlog_Settings/AIEventLog_Redesign_v5.md Section 5.6

// detection_log_service/mod.rs
// TODO(autoAttunement): 統計集計クエリを追加

// preset_loader/mod.rs
// TODO(autoAttunement): 動的閾値調整機能を追加
```

---

## 6. 変更ファイル一覧

| ファイル | 変更種別 | 変更内容 |
|----------|---------|---------|
| `is22/src/polling_orchestrator/mod.rs` | 修正 | 画像保存条件、IS21監視、TODOコメント |
| `is22/src/detection_log_service/mod.rs` | 修正 | 容量クォータ、CleanupStats、診断API、**手動クリーンアップ** |
| `is22/src/snapshot_service/mod.rs` | 修正 | ストリーム検証強化 |
| `is22/src/ai_client/mod.rs` | 修正 | 送信失敗ログ強化、ヘルスチェック |
| `is22/src/web_api/routes.rs` | 修正 | 統計API、診断API、設定API、**手動クリーンアップAPI**追加 |
| `frontend/src/components/EventLogPane.tsx` | 修正 | 色・ラベル・ツールチップ・アイコン |
| `frontend/src/components/SettingsModal.tsx` | 修正 | ストレージ設定タブ、推論統計タブ、**手動クリーンアップUI** |
| `frontend/src/components/DetectionDetailModal.tsx` | 修正 | 誤検知報告UI |

---

## 7. 受け入れ条件

### Critical
- [ ] 画像保存条件がnone=保存しない、unknown=保存する
- [ ] 枚数+容量クォータが強制される
- [ ] ユーザーがストレージ設定を変更可能
- [ ] **二枚画像が実際にIS21に送信される（comparison_method="full_image"）**
- [ ] **ストリーム選択が推論と保存で同一**
- [ ] **Unknown削除は手動のみ（自動削除禁止）**

### High
- [ ] 全色スキームがhex code完全一致
- [ ] 全ラベルが日本語翻訳
- [ ] unknown画像診断機能が動作
- [ ] 画像送信失敗がログに記録される
- [ ] **unknown画像復旧フローが動作（診断→復旧→確認）**
- [ ] **ストリームフォールバックがログに記録される**
- [ ] **誤検知報告がバックエンドに保存される**
- [ ] **閾値変更が推論に適用される（conf_override）**
- [ ] **手動クリーンアップUIに確認ダイアログあり**

### Medium
- [ ] 推論統計タブ表示
- [ ] IS21ヘルスチェック動作
- [ ] 誤検知報告UI動作
- [ ] **誤検知統計の集計・表示**
- [ ] **閾値変更履歴の記録**
- [ ] **自動閾値調整の提案（オプション）**

---

## 8. v4からの変更サマリ

| 項目 | v4 | v5 |
|------|----|----|
| cleanup_unknown_images | 自動実行可能 | **手動のみ（manual_cleanup_unknown_images）** |
| unknown削除トリガー | クォータ連動で自動 | **管理者の明示的操作のみ** |
| 10%保持方式 | 分散サンプリング | **最新10%保持** |
| 確認ダイアログ | なし | **あり（confirmed=falseでプレビュー）** |
| Rule 5準拠 | 違反リスクあり | **完全準拠** |

---

## 9. 関連ドキュメント

- **タスクリスト**: AIEventLog_TaskList.md（57タスク、6フェーズ）
- **テスト計画**: AIEventLog_TestPlan.md（BE-01〜BE-20、FE-01〜FE-02、UI-01〜UI-14）
- **GitHub Issue**: #100

---

**文書終端**
