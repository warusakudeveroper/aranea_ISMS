# IS22 回答: 代表画像選択機能仕様への対応

**作成日**: 2026-01-13
**宛先**: mobes2.0開発チーム
**件名**: `is22_representative_image_spec.md` への回答

---

## 1. 概要

仕様書を確認しました。IS22側の現状実装と仕様書の差異について報告します。

**結論**: 一部フィールド名と構造に差異があります。確認・調整が必要です。

---

## 2. 現状実装 vs 仕様書の差異

### 2.1 フィールド名の差異

| 項目 | mobes仕様書 | IS22現状実装 | 対応要否 |
|------|-------------|--------------|----------|
| イベントID | `eventId: "det_12345"` (string) | `log_id: 12345` (u64) | ⚠️ 要確認 |
| カメラID | `cameraId: "cam-001"` | `camera_lacis_id: "..."` | ⚠️ 要確認 |
| タイムスタンプ | `timestamp` | `captured_at` | ⚠️ 要確認 |
| イベント種別 | `eventType: "detection"` | `primary_event: "person"` 等 | ⚠️ 要確認 |

### 2.2 Severity範囲の差異

| 項目 | mobes仕様書 | IS22現状実装 |
|------|-------------|--------------|
| severity | 1-5 | 0-3 |

**IS22のseverity定義**:
- 0: Normal（検出なし/無視可能）
- 1: Low（軽微な検出）
- 2: Medium（注意が必要）
- 3: High（重要な検出）

**確認事項**: mobes側で0-3を1-5にマッピングするか、IS22側で変換するか？

### 2.3 snapshot構造の差異

**mobes仕様書**:
```json
{
  "snapshot": {
    "data": "<Base64画像データ>",
    "mimeType": "image/jpeg",
    "filename": "snapshot_12345.jpg"
  }
}
```

**IS22現状実装**:
```json
{
  "snapshotBase64": "<Base64画像データ>",
  "snapshotMimeType": "image/jpeg"
}
```

**差異**:
1. IS22はネストせずフラットに配置
2. `filename` フィールドがない

### 2.4 ペイロード構造の差異

**mobes仕様書** (フラット構造):
```json
{
  "fid": "0150",
  "eventId": "det_12345",
  "cameraId": "cam-001",
  "timestamp": "2026-01-13T10:00:00Z",
  "severity": 4,
  "confidence": 0.9,
  "eventType": "detection",
  "snapshot": { ... }
}
```

**IS22現状実装** (ネスト構造):
```json
{
  "fid": "0150",
  "payload": {
    "event": {
      "tid": "T123...",
      "fid": "0150",
      "logId": 12345,
      "cameraLacisId": "...",
      "capturedAt": "2026-01-13T10:00:00.000Z",
      "primaryEvent": "person",
      "severity": 3,
      "confidence": 0.9,
      "tags": ["indoor", "daytime"],
      "snapshotBase64": "...",
      "snapshotMimeType": "image/jpeg"
    }
  }
}
```

---

## 3. IS22側の確認結果

### 3.1 eventId形式

**現状**: `log_id: 12345` (数値)

**仕様要求**: `eventId: "det_12345"` (プレフィックス付き文字列)

→ IS22側で `det_` プレフィックスを付与して送信可能。**mobes側でどちらの形式を期待しているか確認希望**。

### 3.2 timestamp形式

**現状**: `captured_at: "2026-01-13T10:00:00.000000Z"` (ISO8601、マイクロ秒精度)

**仕様要求**: `timestamp: "2026-01-13T10:00:00Z"` (ISO8601、秒精度)

→ IS22はISO8601形式で送信中。精度の違いは問題ないはず。フィールド名 `captured_at` → `timestamp` への変更が必要か確認希望。

### 3.3 cameraId一貫性

**現状**: カメラのLacisID (`camera_lacis_id`) を使用

**仕様要求**: `cameraId` フィールド

→ IS22はカメラごとにユニークなLacisIDを割り当て済み。同一カメラは常に同じIDで送信。フィールド名の変更が必要か確認希望。

---

## 4. 質問事項

### Q1: ペイロード構造

現状IS22は `{ fid, payload: { event: {...} } }` のネスト構造で送信しています。

仕様書はフラット構造 `{ fid, eventId, cameraId, ... }` を記載しています。

**どちらの構造が正しいですか？**

a) IS22をフラット構造に変更する
b) mobes側で両方の構造を受け入れる（既に対応済み？）
c) 仕様書の記載が簡略化されており、現状IS22の構造で問題ない

### Q2: フィールド名の統一

以下のフィールド名をmobes仕様に合わせて変更する必要がありますか？

| IS22現状 | mobes仕様 | 変更要否 |
|----------|-----------|----------|
| `logId` | `eventId` | ? |
| `cameraLacisId` | `cameraId` | ? |
| `capturedAt` | `timestamp` | ? |
| `primaryEvent` | `eventType` | ? |
| `snapshotBase64` | `snapshot.data` | ? |
| `snapshotMimeType` | `snapshot.mimeType` | ? |

### Q3: eventIdプレフィックス

`eventId: "det_12345"` の `det_` プレフィックスは必須ですか？

IS22側で付与する場合、以下のようになります：
```rust
eventId: format!("det_{}", log_id)  // "det_12345"
```

### Q4: severity範囲

IS22は0-3、mobes仕様は1-5です。

a) IS22側で +1 して送信する（0→1, 1→2, 2→3, 3→4）
b) IS22側で範囲を拡張する（0-5に対応）
c) mobes側で変換する
d) 現状のまま（代表画像選択に影響しない）

---

## 5. IS22側の対応方針（案）

mobes側の回答次第で以下のいずれかを実施：

### 案A: フィールド名のみ変更（最小変更）

```rust
// EventPayload の serde rename を変更
#[serde(rename = "eventId")]
pub log_id: u64,  // → "eventId": 12345

#[serde(rename = "cameraId")]
pub camera_lacis_id: String,  // → "cameraId": "..."

#[serde(rename = "timestamp")]
pub captured_at: DateTime<Utc>,  // → "timestamp": "..."
```

### 案B: 構造も含めて完全準拠

```rust
// 新しいペイロード構造
{
  "fid": "0150",
  "eventId": "det_12345",
  "cameraId": "cam-001",
  "timestamp": "2026-01-13T10:00:00Z",
  "severity": 4,
  "confidence": 0.9,
  "eventType": "detection",
  "snapshot": {
    "data": "<Base64>",
    "mimeType": "image/jpeg",
    "filename": "det_12345.jpg"
  }
}
```

### 案C: 現状維持

mobes側が現状のIS22形式を受け入れ可能な場合、変更不要。

---

## 6. 次のステップ

1. mobes側からQ1〜Q4への回答を待つ
2. 回答に基づきIS22側の実装を調整
3. E2Eテストで動作確認

---

**連絡先**: IS22開発チーム

---

**更新日**: 2026-01-13 09:30 JST
