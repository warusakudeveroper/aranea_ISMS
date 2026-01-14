# IS22側調査報告書

**作成日**: 2026-01-14
**作成元**: IS22開発チーム
**対象**: llm_designers_review(important).md に対するIS22側確認結果

---

## 1. 調査概要

`llm_designers_review(important).md` で指摘されたサマリー精度向上に関して、IS22側のデータフローを調査し、Paraclateへのデータ送信状況を確認した。

---

## 2. 確認結果サマリー

| 項目 | IS22の状態 | Paraclate送信状況 |
|------|-----------|------------------|
| 基本検出データ (primary_event, severity, confidence, count_hint) | ✅ 保存済み | ✅ 送信済み |
| person_details (人物属性詳細) | ✅ 保存済み | ❌ **未送信** |
| bboxes (バウンディングボックス・PAR tags) | ✅ 保存済み | ❌ **未送信** |
| suspicious (不審要因・スコア) | ✅ 保存済み | ❌ **未送信** |
| tags (全検出タグ) | ✅ 保存済み | ❌ **未送信** |
| frame_diff (シーン変化情報) | ✅ 保存済み | ❌ **未送信** |
| loitering_detected (徘徊フラグ) | ✅ 保存済み | ❌ **未送信** |

---

## 3. 問題の根本原因

### 3.1 現在のペイロードビルダー実装

**ファイル**: `src/summary_service/payload_builder.rs`
**関数**: `format_detection_detail()` (197-206行目)

```rust
fn format_detection_detail(&self, log: &DetectionLog) -> String {
    serde_json::json!({
        "detection_id": log.log_id,
        "camera_id": log.camera_id,
        "primary_event": log.primary_event,
        "severity": log.severity,
        "confidence": log.confidence,
        "count_hint": log.count_hint
    })
    .to_string()
}
```

**問題点**: 6つの基本フィールドのみを送信しており、IS22のdetection_logsテーブルに保存されている豊富な詳細データが送信されていない。

### 3.2 IS22に保存されているが未送信のデータ

**ファイル**: `src/detection_log_service/mod.rs`
**構造体**: `DetectionLog` (29-89行目)

IS22の`detection_logs`テーブルには以下のデータが完全に保存されている:

```rust
pub struct DetectionLog {
    // 基本情報 (送信済み)
    pub log_id: Option<u64>,
    pub camera_id: String,
    pub primary_event: String,
    pub severity: i32,
    pub confidence: f32,
    pub count_hint: i32,

    // 詳細データ (未送信)
    pub tags: Vec<String>,                        // 全検出タグ
    pub person_details: Option<serde_json::Value>, // 人物属性詳細
    pub bboxes: Option<serde_json::Value>,        // バウンディングボックス
    pub suspicious: Option<serde_json::Value>,    // 不審要因
    pub frame_diff: Option<serde_json::Value>,    // シーン変化情報
    pub loitering_detected: bool,                 // 徘徊検出フラグ
    // ...
}
```

---

## 4. 各データの詳細

### 4.1 person_details (人物属性詳細)

IS21から返却される人物ごとの詳細属性データ。レビュー文書で期待されているサマリー内容の生成に**必須**。

含まれるデータ:
- `body_build`: 体格 (heavy/medium/light)
- `body_size`: サイズ (small/medium/large)
- `height_category`: 身長カテゴリ (short/average/tall)
- `meta`: 年齢層、性別、服装タイプ
- `posture`: 姿勢 (standing/crouching/etc.)
- `top_color` / `bottom_color`: 服の色
- `uniform_like`: 制服風判定
- `mask_like`: マスク着用判定

**サンプル**:
```json
{
  "body_build": "heavy",
  "body_size": "small",
  "height_category": "short",
  "meta": {
    "age_group": "adult",
    "gender": "female",
    "lower_type": "trousers",
    "upper_sleeve": "long"
  },
  "posture": { "type": "crouching", "confidence": 0.6 },
  "top_color": { "color": "blue", "confidence": 0.47 },
  "bottom_color": { "color": "white", "confidence": 0.90 },
  "uniform_like": true
}
```

### 4.2 bboxes (バウンディングボックス)

検出された物体の位置情報とPAR(Pedestrian Attribute Recognition)タグ。

含まれるデータ:
- 位置座標 (x1, y1, x2, y2)
- confidence値
- PAR tags (帽子、眼鏡、バッグ等)
- 人物属性の冗長コピー

### 4.3 suspicious (不審要因)

suspicious判定の根拠となる要因とスコア。

**サンプル**:
```json
{
  "factors": [
    "appearance.mask_like (+40)",
    "crouching_posture (+10)"
  ],
  "level": "high",
  "score": 50
}
```

### 4.4 tags (全検出タグ)

検出結果から抽出されたすべてのタグ。

**サンプル**:
```json
[
  "hair.covered",
  "top_color.blue",
  "carry.backpack",
  "appearance.uniform_like",
  "appearance.mask_like",
  "posture.crouching",
  "age.adult",
  "gender.female"
]
```

### 4.5 frame_diff (シーン変化情報)

フレーム差分解析結果。

**サンプル**:
```json
{
  "camera_status": "scene_change",
  "enabled": true,
  "person_changes": {
    "appeared": 1,
    "disappeared": 5,
    "moved": 0,
    "stationary": 1
  }
}
```

---

## 5. 修正提案

### 5.1 IS22側での修正

`format_detection_detail()` を拡張して、全詳細データを送信する。

```rust
fn format_detection_detail(&self, log: &DetectionLog) -> String {
    serde_json::json!({
        // 基本情報
        "detection_id": log.log_id,
        "camera_id": log.camera_id,
        "primary_event": log.primary_event,
        "severity": log.severity,
        "confidence": log.confidence,
        "count_hint": log.count_hint,

        // 詳細情報 (追加)
        "tags": log.tags,
        "person_details": log.person_details,
        "bboxes": log.bboxes,
        "suspicious": log.suspicious,
        "frame_diff": log.frame_diff,
        "loitering_detected": log.loitering_detected
    })
    .to_string()
}
```

### 5.2 ペイロードサイズの考慮

詳細データを全て送信するとペイロードサイズが増加する。以下のオプションを検討:

1. **全件送信**: LLMでの精度向上を最優先
2. **圧縮送信**: gzip等で圧縮
3. **サマリー化**: IS22側でperson_detailsを事前に要約

**推奨**: まず全件送信で精度を確認し、必要に応じて最適化。

---

## 6. レビュー文書への回答

### 6.1 「サマリーがis22から検出があった、無かったのみを受けている」

**回答**: 正確。現在IS22は基本6フィールドのみ送信しており、詳細な人物属性は送信していない。IS22側の`detection_logs`テーブルには完全なデータが保存されているが、`format_detection_detail()`関数の実装が簡素化されている。

### 6.2 人物特徴レジストリについて

**回答**: これはParaclate側の新規機能であり、IS22側の実装は不要。ただし、IS22からperson_detailsを送信することが前提条件となる。

### 6.3 チャット継続会話について

**回答**: これはParaclate側（mobes）の問題であり、IS22側は関与しない。

---

## 7. 次のアクション

### IS22側 (このリポジトリ)

1. `format_detection_detail()` の拡張実装
2. ペイロードサイズテスト
3. Paraclate側との連携確認

### Paraclate側 (mobes2.0)への依頼

1. 拡張されたdetection_detailの受信・保存対応
2. LLMプロンプトでの詳細データ活用
3. 人物特徴レジストリの設計・実装

---

## 8. 関連ファイル

| ファイル | パス | 役割 |
|---------|-----|------|
| PayloadBuilder | `src/summary_service/payload_builder.rs` | ペイロード構築 |
| DetectionLogService | `src/detection_log_service/mod.rs` | 検出ログ保存・取得 |
| SummaryGenerator | `src/summary_service/generator.rs` | サマリー生成 |

---

**以上**

IS22開発チーム
2026-01-14
