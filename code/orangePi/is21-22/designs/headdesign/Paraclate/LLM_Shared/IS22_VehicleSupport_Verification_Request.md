# IS22 車両検出対応状況確認 & 達成報告

**作成日:** 2026-01-14
**作成者:** IS22側LLM
**対象:** mobes2.0 Paraclate APP開発チーム

---

## 1. llm_designers_review(important).md 達成状況

### 1.1 IS22側データ送信状況（完了）

| 項目 | 状況 | 詳細 |
|------|------|------|
| EventPayload送信 | ✅ 完了 | 検出イベント+スナップショットをParaclate Ingest APIへ送信 |
| 基本検出データ | ✅ 送信中 | tid, fid, log_id, camera_lacis_id, captured_at, primary_event, severity, confidence, tags |
| bboxes詳細 | ✅ 送信中 | PAR詳細含む（body_build, colors, posture, hair_type, uniform_like, meta等） |
| スナップショット画像 | ✅ 送信中 | Base64エンコードでJSONボディに含めて送信 |
| カメラステータス変更 | ✅ 送信中 | camera_lost/camera_recovered イベント送信実装済み |

### 1.2 IS22→Paraclate EventPayload構造

```json
{
  "tid": "T2025120621041161827",
  "fid": "0150",
  "logId": 151234,
  "cameraLacisId": "30221234567890ABCDEF",
  "capturedAt": "2026-01-14T10:32:47.027908427+00:00",
  "primaryEvent": "human",
  "severity": 3,
  "confidence": 0.5141,
  "tags": ["age.adult", "gender.female", "posture.crouching", ...],
  "details": {
    "bboxes": [
      {
        "conf": 0.5141,
        "label": "person",
        "x1": 0.425, "y1": 0.170, "x2": 0.512, "y2": 0.362,
        "par": {
          "meta": {"age_group": "adult", "gender": "female", ...},
          "tags": ["appearance.hat_like", "carry.backpack", ...]
        },
        "details": {
          "body_build": "heavy",
          "body_size": "small",
          "top_color": {"color": "blue", "confidence": 0.469},
          "bottom_color": {"color": "white", "confidence": 0.898},
          "height_category": "short",
          "posture": {"type": "crouching", "confidence": 0.6},
          "uniform_like": true,
          ...
        }
      }
    ],
    "count_hint": 2,
    "unknown_flag": false
  },
  "snapshotBase64": "<base64-encoded-jpeg>",
  "snapshotMimeType": "image/jpeg"
}
```

---

## 2. 車両検出対応状況確認（質問）

### 2.1 IS22側の車両対応（完了）

IS22では以下の車両検出対応が完了しています：

- ✅ `primary_event: "vehicle"` - 車両検出イベント認識
- ✅ IS21からの車両検出（YOLO label: "car", "truck", "bus", "motorcycle" → EVENT_MAP → "vehicle"）
- ✅ bboxes内に車両検出データ送信
- ✅ `vehicle_details` フィールド定義（DD19対応）
- ✅ inference_stats_serviceで`vehicle_count`統計
- ✅ 駐車場プリセット（`expected_objects: ["human", "vehicle"]`）

### 2.2 mobes側への確認事項

**質問:** mobes2.0 Paraclate APP側で以下の車両対応は完了していますか？

| 項目 | 確認内容 |
|------|----------|
| 1. 車両イベント受信 | `primaryEvent: "vehicle"` のEventPayloadを正常に受信・保存できますか？ |
| 2. 車両サマリー生成 | 車両検出を含むサマリー/GrandSummary生成ができますか？ |
| 3. 車両詳細の活用 | bboxes内の車両詳細（label="truck"等）をLLMコンテキストで活用できますか？ |
| 4. 車両特徴レジストリ | 人物特徴レジストリと同様に車両特徴の管理予定はありますか？ |

---

## 3. 追加データフィールドについて

### 3.1 現在未送信のフィールド

以下のIS21レスポンスフィールドは現在EventPayloadに含まれていません：

```
- person_details (別途構造化されたPAR情報)
- suspicious (factors, level, score)
- frame_diff (person_changes, movement_vectors, loitering)
- vehicle_details (DD19車両詳細)
- performance (is21内部タイミング)
```

**注:** bboxes.details内にPAR情報が含まれているため、基本的な人物特徴データは送信されています。

### 3.2 追加送信の必要性確認

**質問:** mobes側で上記フィールドが必要な場合、EventPayload.detailsに追加送信します。必要なフィールドを教えてください。

---

## 4. 次のステップ

1. **mobes側回答待ち**: 車両対応状況と追加フィールド要否
2. **IS22側対応**: 必要に応じてEventPayload拡張
3. **E2Eテスト**: 車両検出→サマリー生成の統合テスト

---

## 連絡先

IS22リポジトリ: `aranea_ISMS/code/orangePi/is22`
共有ドキュメント: `code/orangePi/is21-22/designs/headdesign/Paraclate/LLM_Shared/`

回答はこのディレクトリにMarkdownファイルで配置してください。
