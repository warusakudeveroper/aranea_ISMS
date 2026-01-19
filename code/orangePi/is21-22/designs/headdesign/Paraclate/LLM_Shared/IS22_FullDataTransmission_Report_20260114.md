# IS22 検出データ完全送信対応 完了報告

**報告日時**: 2026-01-14 03:20 UTC
**報告者**: IS22開発チーム
**対象文書**: `llm_designers_review(important).md`

---

## 1. 対応概要

`llm_designers_review(important).md` の以下要件に対応完了:

> - データフローとしてis22から必要な検出データをすべてParaclateに送信しているかを確認する。

**結論: IS22側の検出データ完全送信を実装完了**

---

## 2. 送信データ仕様

### 2.1 Event送信ペイロード構造

IS22からParaclate Ingest Event APIへ送信されるペイロード:

```json
{
  "fid": "0150",
  "payload": {
    "event": {
      "tid": "T2025120621041161827",
      "fid": "0150",
      "detection_log_id": 151860,
      "cameraLacisId": "38013460F99D4F8D0000",
      "capturedAt": "2026-01-14T02:41:22.766925461Z",
      "primaryEvent": "human",
      "severity": 3,
      "confidence": 0.5070011615753174,
      "tags": ["age.adult", "gender.female", "posture.crouching", ...],
      "details": {
        "bboxes": [...],
        "count_hint": 2,
        "unknown_flag": false,
        "schema_version": "2025-12-29.1",
        "person_details": [...],
        "vehicle_details": null,
        "suspicious": {
          "factors": ["appearance.mask_like (+40)", "crouching_posture (+10)"],
          "level": "high",
          "score": 50
        },
        "frame_diff": {
          "camera_status": "scene_change",
          "enabled": true,
          "person_changes": {
            "appeared": 1,
            "disappeared": 5,
            "moved": 0,
            "stationary": 1
          }
        },
        "loitering_detected": false,
        "is22_timings": {
          "snapshot_ms": 2356,
          "is21_roundtrip_ms": 566,
          "is21_inference_ms": 151,
          "is21_yolo_ms": 32,
          "is21_par_ms": 18,
          "total_ms": 2923
        },
        "preset_applied": "balanced",
        "preset_id": "balanced",
        "preset_version": null
      },
      "snapshotBase64": "<base64-encoded-jpeg>",
      "snapshotMimeType": "image/jpeg"
    }
  }
}
```

### 2.2 Summary送信ペイロード構造

```json
{
  "fid": "0150",
  "payload": {
    "summary": {
      "summaryId": "SUM-12345",
      "lacisOath": {...},
      "summaryOverview": {...},
      "cameraContext": {...},
      "cameraDetection": [
        {
          "timestamp": "2026-01-14T02:41:22Z",
          "cameraLacisId": "38013460F99D4F8D0000",
          "detectionDetail": {
            "detection_id": 151860,
            "camera_id": "cam-xxx",
            "captured_at": "2026-01-14T02:41:22Z",
            "primary_event": "human",
            "severity": 3,
            "confidence": 0.507,
            "count_hint": 2,
            "unknown_flag": false,
            "analyzed": true,
            "detected": true,
            "schema_version": "2025-12-29.1",
            "person_details": [...],
            "vehicle_details": null,
            "bboxes": [...],
            "suspicious": {...},
            "tags": [...],
            "frame_diff": {...},
            "loitering_detected": false
          }
        }
      ],
      "cameraStatusSummary": {...}
    },
    "periodStart": "2026-01-14T02:00:00Z",
    "periodEnd": "2026-01-14T03:00:00Z"
  }
}
```

---

## 3. フィールド対応表

### 3.1 llm_designers_review.md サンプルとの対応

| サンプルフィールド | IS22 Event | IS22 Summary | 備考 |
|-------------------|------------|--------------|------|
| `bboxes` | `details.bboxes` | `detectionDetail.bboxes` | 完全送信 |
| `bboxes[].conf` | ✅ | ✅ | |
| `bboxes[].details` | ✅ | ✅ | body_build, body_size, colors等 |
| `bboxes[].par.tags` | ✅ | ✅ | |
| `camera_id` | `cameraLacisId` | `camera_id` | LacisID形式 |
| `captured_at` | `capturedAt` | `captured_at` | ISO8601 |
| `confidence` | `confidence` | `confidence` | |
| `count_hint` | `details.count_hint` | `count_hint` | |
| `frame_diff` | `details.frame_diff` | `frame_diff` | |
| `frame_diff.camera_status` | ✅ | ✅ | |
| `frame_diff.person_changes` | ✅ | ✅ | appeared/disappeared/moved/stationary |
| `is22_timings` | `details.is22_timings` | N/A (Summary不要) | |
| `person_details` | `details.person_details` | `person_details` | DD19完全対応 |
| `primary_event` | `primaryEvent` | `primary_event` | |
| `schema_version` | `details.schema_version` | `schema_version` | |
| `severity` | `severity` | `severity` | |
| `suspicious` | `details.suspicious` | `suspicious` | factors/level/score |
| `tags` | `tags` | `tags` | |
| `unknown_flag` | `details.unknown_flag` | `unknown_flag` | |
| `vehicle_details` | `details.vehicle_details` | `vehicle_details` | DD19対応 |

### 3.2 キー命名規則

| 項目 | IS22送信キー名 | 備考 |
|------|---------------|------|
| 検出ログID | `detection_log_id` | mobes要求に合わせて変換済み |
| カメラLacisID | `cameraLacisId` | camelCase |
| 検出時刻 | `capturedAt` | camelCase, ISO8601 |
| プライマリイベント | `primaryEvent` | camelCase |
| スナップショット | `snapshotBase64` | Base64エンコード |
| MIME | `snapshotMimeType` | image/jpeg |

---

## 4. 送信状況確認結果

### 4.1 送信キュー統計 (2026-01-14 03:17 UTC時点)

| payload_type | status | count |
|--------------|--------|-------|
| summary | sent | 31 |
| summary | failed | 1 |
| grand_summary | sent | 5 |
| grand_summary | skipped | 3 |
| event | sent | 173 |

**全173件のイベントが正常送信完了**

### 4.2 修正履歴

1. **Event payload wrapping** - `{ fid, payload: { event: {...} } }` 形式に修正
2. **detection_log_id フィールド名** - `logId` → `detection_log_id` に変換
3. **details完全送信** - 以下フィールドを追加:
   - `person_details`
   - `vehicle_details`
   - `suspicious`
   - `frame_diff`
   - `loitering_detected`
   - `is22_timings`
   - `schema_version`
   - `preset_applied/id/version`

---

## 5. mobes側確認依頼事項

### 5.1 キー名齟齬確認

以下のキー名がmobes側の期待と一致しているか確認をお願いします:

| IS22送信キー | 確認状況 |
|-------------|---------|
| `detection_log_id` | 要確認 |
| `cameraLacisId` | 要確認 |
| `capturedAt` | 要確認 |
| `primaryEvent` | 要確認 |
| `snapshotBase64` | 要確認 |
| `snapshotMimeType` | 要確認 |
| `details.person_details` | 要確認 |
| `details.vehicle_details` | 要確認 |
| `details.suspicious` | 要確認 |
| `details.frame_diff` | 要確認 |
| `details.is22_timings` | 要確認 |

### 5.2 動作確認依頼

1. **Ingest Event API受信確認**
   - IS22からのeventペイロードが正常にパースされているか
   - 全フィールドがFirestore/BigQueryに保存されているか

2. **LLM参照確認**
   - `person_details`がLLMコンテキストで参照可能か
   - `suspicious.factors`がLLMサマリー生成で使用されているか
   - `frame_diff.person_changes`が状況判定に利用されているか

3. **Summary受信確認**
   - `cameraDetection[].detectionDetail`の全フィールドが利用可能か

### 5.3 回答フォーマット

以下の形式で回答をお願いします:

```markdown
## mobes側確認結果

### キー名確認
| キー名 | 状況 | 備考 |
|--------|------|------|
| detection_log_id | OK/NG | |
| ... | | |

### 動作確認
- [ ] Ingest Event API: 正常受信
- [ ] Firestore保存: 全フィールド保存確認
- [ ] LLM参照: person_details利用可能
- [ ] Summary受信: detectionDetail完全取得

### 問題点・修正依頼
(あれば記載)
```

---

## 6. IS22側完了宣言

`llm_designers_review(important).md` に記載の以下要件について、IS22側の実装は完了しています:

> - データフローとしてis22から必要な検出データをすべてParaclateに送信しているかを確認する。

**達成状況: 100%完了**

残りの要件（人物特徴レジストリ、チャット継続会話改善など）はmobes/Paraclate側の実装となります。

---

## 7. 関連ファイル

| ファイル | 修正内容 |
|---------|---------|
| `src/polling_orchestrator/mod.rs:1241-1295` | EventPayload details完全送信 |
| `src/paraclate_client/client.rs:970-987` | logId→detection_log_id変換 |
| `src/paraclate_client/types.rs:536` | EventPayload serde rename |
| `src/summary_service/payload_builder.rs:196-237` | Summary detectionDetail完全送信 |

---

**以上**
