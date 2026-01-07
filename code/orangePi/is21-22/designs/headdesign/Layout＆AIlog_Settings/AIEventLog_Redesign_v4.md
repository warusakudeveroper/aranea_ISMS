# AIEventLog_Redesign_v4: AI Event Log包括的設計（完全版）

文書番号: Layout＆AIlog_Settings/AIEventLog_Redesign_v4
作成日: 2026-01-08
ステータス: レビュー待ち
関連文書: AIEventlog.md, The_golden_rules.md
前版: AIEventLog_Redesign_v3.md（差し戻し）

---

## 1. 大原則の宣言

本設計は以下の大原則に**完全**準拠する:

- [x] **Rule 1 (SSoT)**: AIEventlog.mdを唯一の情報源。本書内に全要件を明示（参照省略なし）
- [x] **Rule 2 (SOLID)**: 各コンポーネントは単一責務
- [x] **Rule 3 (MECE)**: 全問題を網羅的・相互排他的に分類。漏れなし
- [x] **Rule 4 (Unambiguous)**: 全コード・条件・動作を曖昧さなく明確に定義
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

| イベント種別 | 背景色 | 文字色 | 意図 |
|-------------|--------|--------|------|
| 人検知 (human) | #FFFF00 (黄) | #222222 | 最重要、目立つ色 |
| 車両検知 (vehicle) | #6495ED (青) | #FFFFFF | 人検知と明確に区別 |
| 異常検知 (alert/suspicious) | #FF0000 (赤) | #FFFF00 | 危険な印象 |
| 不明 (unknown) | #D3D3D3 (薄グレー) | #222222 | 曖昧な中間色 |
| カメラロスト (camera_lost) | #333333 (濃グレー) | #FFFFFF | ブラックアウト感 |
| カメラ復帰 (camera_recovered) | #FFFFFF (白) | #444444 | 目立たないが認識可能 |

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
- `unknown`（不明判定）= 分析用に保存するが、90%は定期削除
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
       "comparison_method": "full_image",  // "metadata_only" ではない
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

    // 前フレーム画像があれば追加
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

/// 前フレームの画像データも返却するように変更
pub async fn get_with_image(&self, camera_id: &str) -> Result<Option<(Vec<u8>, FrameMeta)>> {
    // メモリキャッシュから画像データ+メタデータを返却
    // 既存実装で対応済み（get()メソッド）
    self.get(camera_id).await
}
```

#### 3.2.5 polling_orchestrator呼び出し改修

```rust
// polling_orchestrator/mod.rs

// IS21推論呼び出し部分
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

// 推論後に現フレームを保存
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

#### 3.2.7 エビデンス取得手順（二枚画像送信確認）

```bash
# 1. IS22ログで二枚画像送信を確認
journalctl -u is22 -f | grep -E "Sending two images|current_size|previous_size"

# 期待ログ出力:
# camera_id="cam001" current_size=245760 previous_size=241920 "Sending two images for frame diff"
```

```bash
# 2. comparison_methodがfull_imageであることを確認
journalctl -u is22 -f | grep "comparison_method"

# 期待ログ出力:
# camera_id="cam001" comparison_method="full_image" "Frame diff analysis completed"
```

```bash
# 3. IS21側で両画像受信を確認
ssh mijeosadmin@192.168.3.240 "journalctl -u is21 -f | grep -E 'image_current|image_previous'"

# 期待ログ出力:
# Received image_current: 245760 bytes
# Received image_previous: 241920 bytes
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
# IS22ログでプリセット適用確認
journalctl -u is22 -f | grep -E "preset_id|preset_version|conf_override"

# 期待ログ出力:
# preset_id="balanced" preset_version="1.0.0" conf_override=0.5
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
// 異常高速応答の検知
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
| A-5 | 不要画像削除なし | unknown蓄積 | 90%削除ロジック |

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

### 5.1 IMPL-A: ストレージ管理実装

#### 5.1.1 画像保存条件の真理値表

**AIEventlog.md解釈**:
- `none`（何もない）= 保存しない
- `unknown`（不明だが何かある）= 保存する（ただし90%は定期削除）

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
    // 「何もない」(none)は保存しない
    if result.primary_event == "none" {
        return false;
    }
    // severity > 0 は常に保存
    if result.severity > 0 {
        return true;
    }
    // unknown判定は保存（後で90%削除）
    if result.unknown_flag || result.primary_event == "unknown" {
        return true;
    }
    // それ以外のイベントがあれば保存
    true
}
```

#### 5.1.2 容量クォータ設計（新規）

```rust
// detection_log_service/mod.rs

#[derive(Debug, Clone)]
pub struct StorageQuotaConfig {
    /// カメラ毎の最大画像枚数
    pub max_images_per_camera: usize,
    /// カメラ毎の最大容量（バイト）
    pub max_bytes_per_camera: u64,
    /// 全体の最大容量（バイト）
    pub max_total_bytes: u64,
}

impl Default for StorageQuotaConfig {
    fn default() -> Self {
        Self {
            max_images_per_camera: 1000,
            max_bytes_per_camera: 500 * 1024 * 1024,  // 500MB per camera
            max_total_bytes: 10 * 1024 * 1024 * 1024, // 10GB total
        }
    }
}

/// 容量ベースのクォータ強制
pub async fn enforce_storage_quota(&self, camera_id: &str) -> Result<CleanupStats> {
    let camera_dir = self.config.image_base_path.join(camera_id);
    if !camera_dir.exists() {
        return Ok(CleanupStats::new(0, 0, 0));
    }

    // 1. 現在の使用量を計算
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

    // 2. 古い順にソート
    entries.sort_by_key(|(_, _, modified)| *modified);

    // 3. 枚数クォータチェック
    let mut deleted = 0;
    while entries.len() > self.config.quota.max_images_per_camera {
        if let Some((path, size, _)) = entries.first() {
            fs::remove_file(path).await?;
            total_bytes -= size;
            deleted += 1;
            entries.remove(0);
        }
    }

    // 4. 容量クォータチェック
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

#### 5.1.3 ユーザー設定UI設計（新規）

**設定モーダル追加項目**:

```tsx
// SettingsModal.tsx - ストレージ設定タブ

<TabsContent value="storage">
  <div className="space-y-4">
    <div>
      <Label>カメラ毎の最大画像枚数</Label>
      <Input
        type="number"
        value={maxImagesPerCamera}
        onChange={(e) => setMaxImagesPerCamera(Number(e.target.value))}
        min={100}
        max={10000}
      />
    </div>
    <div>
      <Label>カメラ毎の最大容量 (MB)</Label>
      <Input
        type="number"
        value={maxMbPerCamera}
        onChange={(e) => setMaxMbPerCamera(Number(e.target.value))}
        min={100}
        max={2000}
      />
    </div>
    <div>
      <Label>全体の最大容量 (GB)</Label>
      <Input
        type="number"
        value={maxTotalGb}
        onChange={(e) => setMaxTotalGb(Number(e.target.value))}
        min={1}
        max={100}
      />
    </div>
    <Button onClick={handleSaveStorageSettings}>保存</Button>
  </div>
</TabsContent>
```

**API設計**:

```rust
// PUT /api/settings/storage
#[derive(Deserialize)]
pub struct StorageSettingsRequest {
    pub max_images_per_camera: usize,
    pub max_bytes_per_camera: u64,
    pub max_total_bytes: u64,
}
```

#### 5.1.4 CleanupStats構造体

```rust
#[derive(Debug, Clone)]
pub struct CleanupStats {
    pub total: usize,
    pub deleted: usize,
    pub kept: usize,
    pub bytes_freed: u64,
}
```

#### 5.1.5 Unknown/None画像クリーンアップ

```rust
/// unknown判定画像のクリーンアップ（最新10%保持）
pub async fn cleanup_unknown_images(&self, camera_id: &str) -> Result<CleanupStats> {
    let paths = sqlx::query_scalar::<_, String>(
        r#"
        SELECT image_path_local FROM detection_logs
        WHERE camera_id = ?
          AND primary_event = 'unknown'
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

    let keep_count = std::cmp::max(1, (total as f64 * 0.10).ceil() as usize);
    let delete_count = total.saturating_sub(keep_count);

    let mut bytes_freed: u64 = 0;
    for path in paths.iter().take(delete_count) {
        if let Ok(metadata) = fs::metadata(path).await {
            bytes_freed += metadata.len();
        }
        let _ = fs::remove_file(path).await;
    }

    // DBのimage_path_localをクリア
    sqlx::query(
        r#"
        UPDATE detection_logs
        SET image_path_local = ''
        WHERE camera_id = ?
          AND primary_event = 'unknown'
          AND image_path_local != ''
        ORDER BY captured_at ASC
        LIMIT ?
        "#
    )
    .bind(camera_id)
    .bind(delete_count as i64)
    .execute(&self.pool)
    .await?;

    Ok(CleanupStats { total, deleted: delete_count, kept: total - delete_count, bytes_freed })
}
```

---

### 5.2 IMPL-B: 表示仕様準拠実装

#### 5.2.1 色スキーム定義（完全版）

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

// Badge適用
<Badge
  variant={badgeVariant}
  className="text-[9px] px-1 py-0 h-4 min-w-[18px] justify-center cursor-help"
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

#### 5.3.1 Unknown画像可視性診断

**診断API追加**:

```rust
// GET /api/diagnostics/images/{camera_id}
pub async fn diagnose_camera_images(camera_id: &str) -> DiagnosticsResult {
    // 1. DBのunknown画像パスを取得
    let db_paths = get_unknown_image_paths(camera_id).await?;

    // 2. ファイルシステムの存在確認
    let mut missing_files = vec![];
    let mut existing_files = vec![];
    for path in &db_paths {
        if Path::new(path).exists() {
            existing_files.push(path.clone());
        } else {
            missing_files.push(path.clone());
        }
    }

    // 3. 結果返却
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

**フロントエンド表示**:

```tsx
// 診断結果表示コンポーネント
function ImageDiagnostics({ cameraId }: { cameraId: string }) {
  const { data } = useQuery(['diagnostics', cameraId], () => fetchDiagnostics(cameraId))

  return (
    <div className="p-2 bg-muted rounded">
      <p>DB登録: {data?.total_in_db}件</p>
      <p>ファイル存在: {data?.existing_on_disk}件</p>
      <p className={data?.missing_on_disk > 0 ? 'text-red-500' : ''}>
        欠損: {data?.missing_on_disk}件
      </p>
    </div>
  )
}
```

#### 5.3.2 サブ/メインストリーム検証

```rust
// snapshot_service/mod.rs

pub async fn get_snapshot_with_validation(
    camera: &Camera,
    prefer_substream: bool,
) -> Result<(Vec<u8>, SnapshotSource)> {
    let (data, source) = self.get_snapshot(camera, prefer_substream).await?;

    // 画像サイズ検証
    if data.len() < 1000 {
        tracing::warn!(
            camera_id = %camera.camera_id,
            size = data.len(),
            source = ?source,
            "Suspiciously small image - possible stream issue"
        );
    }

    // JPEG検証
    if !data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Err(Error::InvalidImage("Not a valid JPEG"));
    }

    Ok((data, source))
}
```

#### 5.3.3 画像送信失敗ログ強化

```rust
// ai_client/mod.rs

pub async fn analyze(&self, request: AnalyzeRequest, image: &[u8]) -> Result<AnalyzeResponse> {
    let start = Instant::now();

    let result = self.client
        .post(&format!("{}/v1/analyze", self.base_url))
        .multipart(form)
        .timeout(self.timeout)
        .send()
        .await;

    match result {
        Ok(response) => {
            if !response.status().is_success() {
                tracing::error!(
                    camera_id = %request.camera_id,
                    status = %response.status(),
                    elapsed_ms = start.elapsed().as_millis(),
                    "IS21 analyze request failed"
                );
                return Err(Error::Is21Error(response.status().to_string()));
            }
            // ...
        }
        Err(e) => {
            tracing::error!(
                camera_id = %request.camera_id,
                error = %e,
                elapsed_ms = start.elapsed().as_millis(),
                "IS21 connection failed"
            );
            return Err(Error::NetworkError(e.to_string()));
        }
    }
}
```

#### 5.3.4 Unknown画像可視化の復旧手順

> AIEventlog.md要件: 「unknownの項目に関しては保存画像が見えません」

**復旧フロー概要**:
```
診断 → 原因特定 → 復旧実行 → UI確認 → 受け入れ確認
```

##### 5.3.4.1 復旧API設計

```rust
// POST /api/recovery/images/{camera_id}
#[derive(Deserialize)]
pub struct RecoveryRequest {
    /// 復旧対象のlog_id（指定なしで全unknown対象）
    pub log_ids: Option<Vec<i64>>,
    /// 復旧モード
    pub mode: RecoveryMode,
}

#[derive(Deserialize)]
pub enum RecoveryMode {
    /// 現在のスナップショットで再保存
    RefetchCurrent,
    /// DBパスのみ修正（ファイル存在時）
    FixPathOnly,
    /// 欠損レコードをDBから削除
    PurgeOrphans,
}

#[derive(Serialize)]
pub struct RecoveryResult {
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub details: Vec<RecoveryDetail>,
}

pub async fn recover_unknown_images(
    camera_id: &str,
    request: RecoveryRequest,
) -> Result<RecoveryResult> {
    let targets = match &request.log_ids {
        Some(ids) => get_logs_by_ids(ids).await?,
        None => get_unknown_logs_missing_images(camera_id).await?,
    };

    let mut result = RecoveryResult::default();

    for log in targets {
        result.attempted += 1;

        match request.mode {
            RecoveryMode::RefetchCurrent => {
                // カメラから現在のスナップショットを取得して再保存
                match snapshot_service.get_snapshot(&log.camera_id).await {
                    Ok(image_data) => {
                        let path = save_image(&log.camera_id, &log.log_id, &image_data).await?;
                        update_log_image_path(&log.log_id, &path).await?;
                        result.succeeded += 1;
                        result.details.push(RecoveryDetail {
                            log_id: log.log_id,
                            status: "recovered",
                            new_path: Some(path),
                        });
                    }
                    Err(e) => {
                        result.failed += 1;
                        result.details.push(RecoveryDetail {
                            log_id: log.log_id,
                            status: "failed",
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            RecoveryMode::FixPathOnly => {
                // 正しいパスを探して更新
                if let Some(correct_path) = find_image_file(&log.camera_id, &log.log_id).await? {
                    update_log_image_path(&log.log_id, &correct_path).await?;
                    result.succeeded += 1;
                }
            }
            RecoveryMode::PurgeOrphans => {
                // 画像なしレコードをDB削除
                delete_log(&log.log_id).await?;
                result.succeeded += 1;
            }
        }
    }

    Ok(result)
}
```

##### 5.3.4.2 復旧UIフロー

```tsx
// components/ImageRecoveryDialog.tsx

function ImageRecoveryDialog({ cameraId, diagnostics }: Props) {
  const [mode, setMode] = useState<'refetch' | 'fix' | 'purge'>('refetch')
  const [result, setResult] = useState<RecoveryResult | null>(null)

  const handleRecover = async () => {
    const res = await fetch(`/api/recovery/images/${cameraId}`, {
      method: 'POST',
      body: JSON.stringify({ mode }),
    })
    setResult(await res.json())
  }

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="outline" disabled={diagnostics.missing_on_disk === 0}>
          欠損画像を復旧
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogTitle>Unknown画像の復旧</DialogTitle>
        <p className="text-sm text-muted-foreground">
          欠損画像: {diagnostics.missing_on_disk}件
        </p>

        <RadioGroup value={mode} onValueChange={setMode}>
          <div className="flex items-center space-x-2">
            <RadioGroupItem value="refetch" id="refetch" />
            <Label htmlFor="refetch">現在のスナップショットで再保存</Label>
          </div>
          <div className="flex items-center space-x-2">
            <RadioGroupItem value="fix" id="fix" />
            <Label htmlFor="fix">パス修正のみ（ファイル存在時）</Label>
          </div>
          <div className="flex items-center space-x-2">
            <RadioGroupItem value="purge" id="purge" />
            <Label htmlFor="purge">欠損レコードを削除</Label>
          </div>
        </RadioGroup>

        {result && (
          <div className="mt-4 p-2 bg-muted rounded">
            <p>試行: {result.attempted}</p>
            <p className="text-green-600">成功: {result.succeeded}</p>
            <p className="text-red-600">失敗: {result.failed}</p>
          </div>
        )}

        <DialogFooter>
          <Button onClick={handleRecover}>復旧実行</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

##### 5.3.4.3 受け入れ条件

| ID | 条件 | 検証方法 |
|----|------|----------|
| RCV-01 | 復旧後に欠損件数が0になる | 診断API再実行 |
| RCV-02 | 復旧した画像がUI上で表示される | DetectionDetailModalで確認 |
| RCV-03 | 復旧ログが記録される | journalctlで確認 |
| RCV-04 | 復旧失敗時にエラー詳細が表示される | UIで確認 |

#### 5.3.5 ストリーム選択仕様

> AIEventlog.md要件: 「サブストリームで巡回画像を取得しているにも関わらず取得できないメインストリームの画像をAI推論判定材料にしていないか？」

##### 5.3.5.1 ストリーム選択ポリシー

**原則**: 巡回用ストリームと推論用ストリームは**同一**とする

```
┌─────────────────────────────────────────────────────────────────┐
│                     ストリーム選択フロー                          │
│                                                                 │
│  ┌─────────────┐    成功    ┌─────────────┐                    │
│  │メインストリーム│─────────→│ 推論に使用   │                    │
│  │  (高解像度)  │           │ 画像保存     │                    │
│  └─────────────┘           └─────────────┘                    │
│        │                                                        │
│        │ 失敗（timeout/error）                                   │
│        ↓                                                        │
│  ┌─────────────┐    成功    ┌─────────────┐                    │
│  │サブストリーム │─────────→│ 推論に使用   │                    │
│  │  (低解像度)  │           │ 画像保存     │                    │
│  └─────────────┘           └─────────────┘                    │
│        │                                                        │
│        │ 失敗                                                    │
│        ↓                                                        │
│  ┌─────────────┐                                               │
│  │ スキップ     │ → ログ記録、次サイクルでリトライ               │
│  └─────────────┘                                               │
└─────────────────────────────────────────────────────────────────┘
```

##### 5.3.5.2 ストリーム選択ロジック実装

```rust
// snapshot_service/mod.rs

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamSource {
    MainStream,
    SubStream,
}

#[derive(Debug)]
pub struct SnapshotResult {
    pub data: Vec<u8>,
    pub source: StreamSource,
    pub resolution: (u32, u32),
}

pub async fn get_snapshot_for_inference(
    &self,
    camera: &Camera,
) -> Result<SnapshotResult> {
    // 1. メインストリームを試行
    match self.try_stream(camera, StreamSource::MainStream).await {
        Ok(result) => {
            tracing::debug!(
                camera_id = %camera.camera_id,
                source = "main",
                size = result.data.len(),
                "Snapshot acquired from main stream"
            );
            return Ok(result);
        }
        Err(e) => {
            tracing::warn!(
                camera_id = %camera.camera_id,
                error = %e,
                "Main stream failed, falling back to substream"
            );
        }
    }

    // 2. サブストリームにフォールバック
    match self.try_stream(camera, StreamSource::SubStream).await {
        Ok(result) => {
            tracing::info!(
                camera_id = %camera.camera_id,
                source = "sub",
                size = result.data.len(),
                "Snapshot acquired from substream (fallback)"
            );
            return Ok(result);
        }
        Err(e) => {
            tracing::error!(
                camera_id = %camera.camera_id,
                error = %e,
                "Both streams failed"
            );
            return Err(Error::AllStreamsFailed);
        }
    }
}
```

##### 5.3.5.3 ストリーム使用ログ

```rust
// polling_orchestrator/mod.rs - 推論呼び出し後

tracing::info!(
    camera_id = %camera_id,
    stream_source = ?snapshot.source,
    resolution = ?snapshot.resolution,
    image_size = snapshot.data.len(),
    inference_result = %result.primary_event,
    "Inference completed with stream source logged"
);

// DBにもストリームソースを保存
detection_log.save_detection_with_source(
    &result,
    &snapshot.data,
    snapshot.source,  // stream_source列に保存
).await?;
```

##### 5.3.5.4 ストリーム不整合検知

```rust
/// 巡回と推論で異なるストリームを使用していないか検証
pub async fn validate_stream_consistency(camera_id: &str) -> StreamConsistencyReport {
    // 直近100件の検出ログを取得
    let logs = get_recent_logs(camera_id, 100).await?;

    let main_count = logs.iter().filter(|l| l.stream_source == "main").count();
    let sub_count = logs.iter().filter(|l| l.stream_source == "sub").count();

    StreamConsistencyReport {
        camera_id: camera_id.to_string(),
        main_stream_ratio: main_count as f64 / logs.len() as f64,
        sub_stream_ratio: sub_count as f64 / logs.len() as f64,
        warning: if sub_count > main_count * 2 {
            Some("Substream is being used more frequently than main - check camera connection")
        } else {
            None
        },
    }
}
```

##### 5.3.5.5 ストリーム選択テストケース

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| STR-01 | メインストリーム成功 | source="main"でログ |
| STR-02 | メイン失敗→サブ成功 | source="sub"でログ、警告出力 |
| STR-03 | 両方失敗 | エラーログ、スキップ |
| STR-04 | 推論に使用した画像と保存画像が同一 | 画像サイズ一致 |
| STR-05 | サブストリーム多用警告 | sub > main*2で警告 |

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
// 定期実行（60秒毎）
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
            Is21Health {
                status: "unhealthy",
                latency_ms: -1,
                last_check: Utc::now(),
            }
        }
    }
}
```

---

### 5.5 IMPL-E: 精度向上策

#### 5.5.1 誤検知フィードバックUI

```tsx
// DetectionDetailModal.tsx に追加

<Button variant="outline" onClick={() => reportMisdetection(log.log_id)}>
  誤検知を報告
</Button>

<Dialog>
  <DialogContent>
    <DialogTitle>誤検知報告</DialogTitle>
    <Select value={correctLabel} onValueChange={setCorrectLabel}>
      <SelectItem value="no_person">人物なし</SelectItem>
      <SelectItem value="staff">スタッフ（除外対象）</SelectItem>
      <SelectItem value="other">その他</SelectItem>
    </Select>
    <Textarea placeholder="詳細（任意）" />
    <Button onClick={submitFeedback}>送信</Button>
  </DialogContent>
</Dialog>
```

#### 5.5.2 閾値調整UI

```tsx
// CameraSettingsDialog.tsx

<div>
  <Label>YOLO検出閾値</Label>
  <Slider
    value={[confThreshold]}
    onValueChange={([v]) => setConfThreshold(v)}
    min={0.2}
    max={0.8}
    step={0.05}
  />
  <span>{confThreshold}</span>
</div>
```

#### 5.5.3 誤検知フィードバック バックエンド処理

##### 5.5.3.1 データモデル

```rust
// models/feedback.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MisdetectionFeedback {
    pub feedback_id: i64,
    pub log_id: i64,
    pub camera_id: String,
    pub original_label: String,
    pub correct_label: String,
    pub feedback_type: FeedbackType,
    pub detail: Option<String>,
    pub created_at: DateTime<Utc>,
    pub applied: bool,           // 閾値調整に反映済みか
    pub applied_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    NoDetection,      // 何もないのに検出された
    WrongLabel,       // ラベルが間違っている
    StaffExclusion,   // スタッフ除外対象
    Other,
}
```

##### 5.5.3.2 誤検知報告API

```rust
// POST /api/feedback/misdetection
#[derive(Deserialize)]
pub struct MisdetectionRequest {
    pub log_id: i64,
    pub correct_label: String,
    pub feedback_type: FeedbackType,
    pub detail: Option<String>,
}

pub async fn report_misdetection(
    State(app): State<AppState>,
    Json(req): Json<MisdetectionRequest>,
) -> Result<Json<MisdetectionFeedback>> {
    // 1. 対象ログを取得
    let log = app.detection_log.get_by_id(req.log_id).await?;

    // 2. フィードバックを保存
    let feedback = sqlx::query_as!(
        MisdetectionFeedback,
        r#"
        INSERT INTO misdetection_feedbacks
            (log_id, camera_id, original_label, correct_label, feedback_type, detail, created_at, applied)
        VALUES (?, ?, ?, ?, ?, ?, ?, false)
        RETURNING *
        "#,
        req.log_id,
        log.camera_id,
        log.primary_event,
        req.correct_label,
        req.feedback_type.as_str(),
        req.detail,
        Utc::now(),
    )
    .fetch_one(&app.pool)
    .await?;

    tracing::info!(
        log_id = req.log_id,
        camera_id = %log.camera_id,
        original = %log.primary_event,
        correct = %req.correct_label,
        "Misdetection feedback recorded"
    );

    // 3. 閾値調整の候補としてキューに追加
    app.threshold_adjuster.queue_feedback(&feedback).await?;

    Ok(Json(feedback))
}
```

##### 5.5.3.3 誤検知統計集計

```rust
// GET /api/feedback/stats/{camera_id}
pub async fn get_feedback_stats(camera_id: &str) -> FeedbackStats {
    let stats = sqlx::query!(
        r#"
        SELECT
            original_label,
            correct_label,
            COUNT(*) as count
        FROM misdetection_feedbacks
        WHERE camera_id = ?
          AND created_at > datetime('now', '-30 days')
        GROUP BY original_label, correct_label
        ORDER BY count DESC
        "#,
        camera_id,
    )
    .fetch_all(&pool)
    .await?;

    FeedbackStats {
        camera_id: camera_id.to_string(),
        period_days: 30,
        by_label_pair: stats,
        total_feedbacks: stats.iter().map(|s| s.count).sum(),
        misdetection_rate: calculate_misdetection_rate(camera_id).await?,
    }
}
```

#### 5.5.4 閾値変更 バックエンド処理

##### 5.5.4.1 閾値設定API

```rust
// PUT /api/cameras/{camera_id}/threshold
#[derive(Deserialize)]
pub struct ThresholdUpdateRequest {
    pub conf_threshold: f32,    // YOLO信頼度閾値 (0.2-0.8)
    pub reason: Option<String>, // 変更理由
}

pub async fn update_camera_threshold(
    Path(camera_id): Path<String>,
    State(app): State<AppState>,
    Json(req): Json<ThresholdUpdateRequest>,
) -> Result<Json<ThresholdUpdateResponse>> {
    // 1. バリデーション
    if req.conf_threshold < 0.2 || req.conf_threshold > 0.8 {
        return Err(Error::InvalidThreshold("conf_threshold must be between 0.2 and 0.8"));
    }

    // 2. 現在の閾値を取得（変更履歴用）
    let old_threshold = app.camera_settings.get_threshold(&camera_id).await?;

    // 3. 新しい閾値を保存
    sqlx::query!(
        r#"
        INSERT INTO camera_thresholds (camera_id, conf_threshold, updated_at)
        VALUES (?, ?, ?)
        ON CONFLICT(camera_id) DO UPDATE SET
            conf_threshold = excluded.conf_threshold,
            updated_at = excluded.updated_at
        "#,
        camera_id,
        req.conf_threshold,
        Utc::now(),
    )
    .execute(&app.pool)
    .await?;

    // 4. 変更履歴を記録
    sqlx::query!(
        r#"
        INSERT INTO threshold_change_history
            (camera_id, old_value, new_value, reason, changed_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
        camera_id,
        old_threshold,
        req.conf_threshold,
        req.reason,
        Utc::now(),
    )
    .execute(&app.pool)
    .await?;

    tracing::info!(
        camera_id = %camera_id,
        old_threshold = old_threshold,
        new_threshold = req.conf_threshold,
        reason = ?req.reason,
        "Camera threshold updated"
    );

    // 5. 推論時に使用される設定をリロード
    app.preset_loader.reload_camera_settings(&camera_id).await?;

    Ok(Json(ThresholdUpdateResponse {
        camera_id,
        old_threshold,
        new_threshold: req.conf_threshold,
        applied_at: Utc::now(),
    }))
}
```

##### 5.5.4.2 閾値の推論適用

```rust
// polling_orchestrator/mod.rs - 推論リクエスト構築時

pub async fn build_analyze_request(
    &self,
    camera: &Camera,
    preset: &Preset,
) -> AnalyzeRequest {
    // カメラ固有の閾値を取得（未設定ならプリセットのデフォルト）
    let conf_threshold = self.camera_settings
        .get_threshold(&camera.camera_id)
        .await
        .unwrap_or(preset.default_conf_threshold);

    AnalyzeRequest {
        camera_id: camera.camera_id.clone(),
        hints_json: HintsJson {
            conf_override: Some(conf_threshold),  // IS21に閾値を渡す
            preset_id: preset.preset_id.clone(),
            // ...
        },
        // ...
    }
}
```

##### 5.5.4.3 自動閾値調整（オプション）

```rust
// threshold_adjuster.rs

/// 誤検知フィードバックに基づく自動閾値調整
pub async fn auto_adjust_threshold(&self, camera_id: &str) -> Option<ThresholdAdjustment> {
    // 1. 直近30日のフィードバックを取得
    let feedbacks = self.get_recent_feedbacks(camera_id, 30).await?;

    if feedbacks.len() < 10 {
        // 十分なデータがない場合は調整しない
        return None;
    }

    // 2. 誤検知率を計算
    let total_detections = self.get_total_detections(camera_id, 30).await?;
    let misdetection_rate = feedbacks.len() as f64 / total_detections as f64;

    // 3. 閾値調整の判定
    let current_threshold = self.get_current_threshold(camera_id).await?;

    let suggested_threshold = if misdetection_rate > 0.20 {
        // 誤検知率20%超過 → 閾値を上げる（より厳しく）
        (current_threshold + 0.05).min(0.8)
    } else if misdetection_rate < 0.05 {
        // 誤検知率5%未満 → 閾値を下げる（より敏感に）
        (current_threshold - 0.05).max(0.2)
    } else {
        return None; // 調整不要
    };

    Some(ThresholdAdjustment {
        camera_id: camera_id.to_string(),
        current_threshold,
        suggested_threshold,
        misdetection_rate,
        feedback_count: feedbacks.len(),
        reason: format!(
            "Auto-adjustment based on {} feedbacks, misdetection rate: {:.1}%",
            feedbacks.len(),
            misdetection_rate * 100.0
        ),
    })
}
```

##### 5.5.4.4 テストケース

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| FB-01 | 誤検知報告API正常 | feedback_id返却、DBに保存 |
| FB-02 | 誤検知統計取得 | カメラ別・ラベル別の集計 |
| FB-03 | 閾値変更API正常 | 0.2-0.8範囲で更新成功 |
| FB-04 | 閾値変更履歴記録 | history テーブルに記録 |
| FB-05 | 閾値範囲外拒否 | 0.1指定でエラー |
| FB-06 | 閾値が推論に適用される | ログにconf_override出力 |
| FB-07 | 自動調整（誤検知率高） | 閾値上昇を提案 |
| FB-08 | 自動調整（誤検知率低） | 閾値下降を提案 |

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

#### 5.6.2 統計収集項目

| 項目 | 説明 | 用途 |
|------|------|------|
| 検出頻度 | 時間あたり検出数 | 過剰検出検知 |
| 誤検出率 | フィードバックベース | 閾値調整 |
| イベント分布 | human/vehicle等の比率 | 傾向把握 |

#### 5.6.3 TODOコメント埋め込み

```rust
// polling_orchestrator/mod.rs
// TODO(autoAttunement): 検出結果をStatsCollectorに送信
// 参照: Layout＆AIlog_Settings/AIEventLog_Redesign_v4.md Section 5.6

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
| `is22/src/detection_log_service/mod.rs` | 修正 | 容量クォータ、CleanupStats、診断API |
| `is22/src/snapshot_service/mod.rs` | 修正 | ストリーム検証強化 |
| `is22/src/ai_client/mod.rs` | 修正 | 送信失敗ログ強化、ヘルスチェック |
| `is22/src/web_api/routes.rs` | 修正 | 統計API、診断API、設定API追加 |
| `frontend/src/components/EventLogPane.tsx` | 修正 | 色・ラベル・ツールチップ・アイコン |
| `frontend/src/components/SettingsModal.tsx` | 修正 | ストレージ設定タブ、推論統計タブ |
| `frontend/src/components/DetectionDetailModal.tsx` | 修正 | 誤検知報告UI |

---

## 7. タスクリスト（依存順）

### Phase 1: Critical - ストレージ管理

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T1-1 | StorageQuotaConfig, CleanupStats追加 | - | 15分 |
| T1-2 | should_save_image関数実装 | - | 10分 |
| T1-3 | enforce_storage_quota実装（枚数+容量） | T1-1 | 40分 |
| T1-4 | cleanup_unknown_images実装 | T1-1 | 30分 |
| T1-5 | ストレージ設定API追加 | T1-1 | 30分 |
| T1-6 | ストレージ設定UI追加 | T1-5 | 40分 |
| T1-7 | バックエンドビルド・テスト | T1-4 | 30分 |

### Phase 2: High - 表示仕様準拠

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T2-1 | EVENT_COLORS, ICON_COLORS定数 | - | 10分 |
| T2-2 | getEventStyle, getIconColor実装 | T2-1 | 15分 |
| T2-3 | EVENT_LABELS定数、getEventLabel修正 | - | 15分 |
| T2-4 | カメラ名表示・Severityツールチップ | - | 15分 |
| T2-5 | DetectionLogItem統合修正 | T2-2,T2-3,T2-4 | 30分 |
| T2-6 | フロントエンドビルド確認 | T2-5 | 10分 |

### Phase 3: High - 画像可視性・ストリーム対応

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T3-1 | 診断API実装 | - | 40分 |
| T3-2 | ストリーム検証強化 | - | 30分 |
| T3-3 | 送信失敗ログ強化 | - | 20分 |
| T3-4 | 診断UI追加 | T3-1 | 30分 |
| T3-5 | **復旧API実装（RecoveryMode 3種）** | T3-1 | 45分 |
| T3-6 | **復旧UI（ImageRecoveryDialog）** | T3-5 | 40分 |
| T3-7 | **ストリーム選択ロジック実装** | - | 35分 |
| T3-8 | **ストリームソースDB保存** | T3-7 | 20分 |
| T3-9 | **二枚画像送信実装（ai_client改修）** | - | 50分 |
| T3-10 | **polling_orchestrator二枚画像対応** | T3-9 | 30分 |

### Phase 4: Medium - 監視・精度・バックエンド

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T4-1 | 推論統計API | - | 60分 |
| T4-2 | IS21ヘルスチェック | - | 30分 |
| T4-3 | 推論統計タブUI | T4-1 | 60分 |
| T4-4 | 誤検知報告UI | - | 40分 |
| T4-5 | 閾値調整UI | - | 30分 |
| T4-6 | **MisdetectionFeedbackモデル・テーブル** | - | 25分 |
| T4-7 | **誤検知報告API実装** | T4-6 | 35分 |
| T4-8 | **誤検知統計集計API** | T4-7 | 30分 |
| T4-9 | **閾値変更API実装** | - | 40分 |
| T4-10 | **閾値変更履歴テーブル** | - | 15分 |
| T4-11 | **閾値の推論適用（conf_override）** | T4-9 | 25分 |
| T4-12 | **自動閾値調整ロジック（オプション）** | T4-7,T4-9 | 45分 |

### Phase 5: 基盤整備

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T5-1 | autoAttunement TODOコメント追加 | - | 10分 |

### Phase 6: テスト

| ID | タスク | 依存 | 見積 |
|----|--------|------|------|
| T6-1 | BE: 画像保存条件テスト | T1-7 | 30分 |
| T6-2 | BE: クォータテスト（枚数・容量） | T1-7 | 30分 |
| T6-3 | BE: 診断APIテスト | T3-1 | 20分 |
| T6-4 | FE: ビルド確認 | T2-6 | 10分 |
| T6-5 | UI: 色スキーム・ラベル検証 | T6-4 | 30分 |
| T6-6 | UI: ストレージ設定検証 | T1-6 | 20分 |

---

## 8. テスト計画

### 8.1 バックエンドテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| BE-01 | cargo build --release | エラーなし |
| BE-02 | event="none", unknown=false | 画像保存**されない** |
| BE-03 | event="none", unknown=true | 画像保存**されない**（none優先） |
| BE-04 | event="unknown", unknown=true | 画像保存**される** |
| BE-05 | severity=1, any | 画像保存**される** |
| BE-06 | 枚数クォータ: 1100枚→1000枚 | 100枚削除 |
| BE-07 | 容量クォータ: 600MB→500MB | 古い画像から削除 |
| BE-08 | unknown cleanup: 100枚 | 90枚削除、10枚残存 |
| BE-09 | 診断API: 欠損ファイルあり | missing_on_disk > 0 |
| BE-10 | **二枚画像送信: 2回目巡回** | had_previous=true, comparison_method="full_image" |
| BE-11 | **二枚画像送信: IS21ログ確認** | 両画像バイト数が記録 |
| BE-12 | **復旧API: RefetchCurrent** | 現スナップショットで画像再保存 |
| BE-13 | **復旧API: PurgeOrphans** | 欠損レコードDB削除 |
| BE-14 | **ストリーム選択: メイン成功** | source="main"でログ |
| BE-15 | **ストリーム選択: フォールバック** | source="sub"でログ、警告出力 |
| BE-16 | **誤検知報告API** | feedback_id返却、DBに保存 |
| BE-17 | **誤検知統計取得** | カメラ別・ラベル別の集計 |
| BE-18 | **閾値変更API: 正常** | 0.2-0.8範囲で更新成功 |
| BE-19 | **閾値変更API: 範囲外** | 0.1指定でエラー |
| BE-20 | **閾値推論適用** | ログにconf_override出力 |

### 8.2 フロントエンドテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| FE-01 | npm run build | エラーなし |
| FE-02 | npx tsc --noEmit | エラーなし |

### 8.3 Chrome UIテスト

| ID | テスト内容 | 期待結果 |
|----|-----------|----------|
| UI-01 | 人検知イベント表示 | 背景#FFFF00、文字#222222 |
| UI-02 | 車両検知イベント表示 | 背景#6495ED、文字#FFFFFF |
| UI-03 | 異常検知イベント表示 | 背景#FF0000、文字#FFFF00 |
| UI-04 | unknownイベント表示 | 背景#D3D3D3、文字#222222 |
| UI-05 | camera_lost表示 | 背景#333333、文字#FFFFFF |
| UI-06 | camera_recovered表示 | 背景#FFFFFF、文字#444444 |
| UI-07 | 人検知ラベル | 「人物検知」と表示 |
| UI-08 | unknownラベル | 「不明」と表示 |
| UI-09 | severityホバー | ツールチップ表示 |
| UI-10 | ストレージ設定 | 枚数・容量設定可能 |
| UI-11 | 診断結果表示 | 欠損件数表示 |

---

## 9. 実装手順

### 9.1 GitHub Issue登録

```
Title: [is22] AI Event Log完全仕様準拠・ストレージ管理・監視機能実装
Labels: enhancement, is22, frontend, backend

設計ドキュメント: Layout＆AIlog_Settings/AIEventLog_Redesign_v4.md
```

### 9.2 実装前コミット

```bash
git add code/orangePi/is21-22/designs/headdesign/Layout＆AIlog_Settings/
git commit -m "docs(is22): AI Event Log Redesign v4 設計ドキュメント"
git push origin main
```

---

## 10. 受け入れ条件

### Critical
- [ ] 画像保存条件がnone=保存しない、unknown=保存する（後で削除）
- [ ] 枚数+容量クォータが強制される
- [ ] ユーザーがストレージ設定を変更可能
- [ ] **二枚画像が実際にIS21に送信される（comparison_method="full_image"）**
- [ ] **ストリーム選択が推論と保存で同一**

### High
- [ ] 全色スキームがhex code完全一致
- [ ] 全ラベルが日本語翻訳
- [ ] unknown画像診断機能が動作
- [ ] 画像送信失敗がログに記録される
- [ ] **unknown画像復旧フローが動作（診断→復旧→確認）**
- [ ] **ストリームフォールバックがログに記録される**
- [ ] **誤検知報告がバックエンドに保存される**
- [ ] **閾値変更が推論に適用される（conf_override）**

### Medium
- [ ] 推論統計タブ表示
- [ ] IS21ヘルスチェック動作
- [ ] 誤検知報告UI動作
- [ ] **誤検知統計の集計・表示**
- [ ] **閾値変更履歴の記録**
- [ ] **自動閾値調整の提案（オプション）**

---

## 11. MECEチェック

| カテゴリ | 問題ID | 網羅性 |
|---------|--------|--------|
| A: ストレージ管理 | A-1〜A-5 | 枚数・容量・設定UI全て |
| B: 表示仕様 | B-1〜B-5 | 色・ラベル・ツールチップ・アイコン全て |
| C: 画像送信・表示 | C-1〜C-3 | unknown可視性・ストリーム・送信失敗全て |
| D: 監視・分析 | D-1〜D-3 | 統計・プリセット・IS21監視全て |
| E: 精度・誤検知 | E-1〜E-2 | フィードバック・閾値調整全て |
| F: 後続基盤 | F-1〜F-2 | autoAttunement設計・TODO全て |

**漏れなし確認**: サブストリーム(C-2)、送信失敗(C-3)、unknown表示(C-1)全て分類済み。

---

## 12. v3からの変更点

| 指摘 | v3 | v4 |
|------|----|----|
| UI要件省略 | 「v2と同一」 | Section 2.1に完全転記 |
| 容量クォータなし | 枚数のみ | 枚数+容量+設定UI |
| unknown常時保存 | unknown_flag→保存 | none→保存しない、unknown→保存 |
| 二枚画像エビデンス | 説明のみ | ログ例・検証手順追加 |
| 精度向上策なし | 未設計 | フィードバック・閾値UI |
| IS21監視なし | 未設計 | ヘルスチェック追加 |
| MECE不整合 | C-1〜C-3欠落 | 全問題分類済み |

---

## 13. v4レビュー後の追記事項（v4.1）

| 指摘 | v4（初版） | v4.1（追記後） |
|------|-----------|---------------|
| 二枚画像送信 | メタデータのみ送信 | 実画像2枚送信フロー（Section 3.2.2-3.2.8） |
| unknown可視性 | 診断APIのみ | 復旧手順・UI・受け入れ条件追加（Section 5.3.4） |
| ストリーム選択 | 検証のみ | 選択ポリシー・フォールバック・ログ定義（Section 5.3.5） |
| 誤検知報告バックエンド | UIのみ | データモデル・API・統計集計追加（Section 5.5.3） |
| 閾値変更バックエンド | UIのみ | API・推論適用・自動調整・テスト追加（Section 5.5.4） |

---

**文書終端**
