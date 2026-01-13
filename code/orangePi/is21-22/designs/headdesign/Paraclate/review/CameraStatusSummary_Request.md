# CameraStatusSummary 対応依頼

**日付**: 2026-01-13
**発信元**: IS22 (Paraclateクライアント)
**宛先**: mobes2.0 LLM Orchestrator チーム
**種別**: 機能拡張依頼

---

## 概要

IS22からParaclateへ送信するSummaryペイロードに `cameraStatusSummary` フィールドを追加しました。
LLMがカメラの接続状態を正確に判定できるよう、mobes側での対応をお願いします。

## 背景・問題

### 現象
- 複数カメラで「接続ロスト」イベントが発生しているにも関わらず
- AIが「すべてのカメラが正常に稼働しています」と回答

### 原因分析
- `camera_lost` / `connection_lost` イベントは `cameraDetection` 配列内に含まれる
- しかし、イベント数が多い場合や他のイベントと混在する場合、LLMが正確に集計できていない
- 明示的な「システム健全性」情報がなかった

## IS22側の対応（実装済み）

### 新規フィールド: `cameraStatusSummary`

```json
{
  "lacisOath": { ... },
  "summaryOverview": { ... },
  "cameraContext": { ... },
  "cameraDetection": [ ... ],
  "cameraStatusSummary": {
    "totalCameras": 8,
    "onlineCameras": 5,
    "offlineCameras": 3,
    "offlineCameraIds": [
      "3080AABBCCDD00010001",
      "3080AABBCCDD00010002",
      "3080AABBCCDD00010003"
    ],
    "connectionLostEvents": 12,
    "systemHealth": "degraded"
  }
}
```

### フィールド定義

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `totalCameras` | number | 登録済みカメラ総数 |
| `onlineCameras` | number | オンライン（正常稼働）カメラ数 |
| `offlineCameras` | number | オフライン（接続ロスト）カメラ数 |
| `offlineCameraIds` | string[] | 接続ロスト発生カメラのLacisIDリスト |
| `connectionLostEvents` | number | 期間内の接続ロストイベント総数 |
| `systemHealth` | string | システム健全性 (後述) |

### systemHealth 判定ロジック

```
healthy   : offlineCameras == 0
degraded  : offlineCameras > 0 && (offlineCameras / totalCameras) < 0.5
critical  : (offlineCameras / totalCameras) >= 0.5
```

## mobes側への依頼事項

### 1. llmOrchestrator.ts の更新

`cameraStatusSummary` を使用してシステム健全性を判定するようプロンプトを更新してください。

#### 推奨プロンプト追加例

```typescript
// summaryImageContext.ts または llmOrchestrator.ts

const systemHealthContext = `
## カメラシステム健全性

- システム状態: ${summary.cameraStatusSummary.systemHealth}
- 総カメラ数: ${summary.cameraStatusSummary.totalCameras}
- オンライン: ${summary.cameraStatusSummary.onlineCameras}
- オフライン: ${summary.cameraStatusSummary.offlineCameras}

${summary.cameraStatusSummary.systemHealth !== 'healthy' ? `
【重要】以下のカメラで接続問題が発生しています:
${summary.cameraStatusSummary.offlineCameraIds.map(id =>
  `- ${summary.cameraContext[id]?.cameraName || id}`
).join('\n')}
期間内の接続ロストイベント数: ${summary.cameraStatusSummary.connectionLostEvents}
` : ''}
`;
```

### 2. 回答生成時の考慮

| systemHealth | LLMへの指示 |
|--------------|------------|
| `healthy` | 「すべてのカメラが正常稼働」と回答可能 |
| `degraded` | 「一部カメラで接続問題あり」と明記必須 |
| `critical` | 「重大な接続問題」として警告を含める |

### 3. 型定義の更新

```typescript
// types/summary.ts

interface CameraStatusSummary {
  totalCameras: number;
  onlineCameras: number;
  offlineCameras: number;
  offlineCameraIds: string[];
  connectionLostEvents: number;
  systemHealth: 'healthy' | 'degraded' | 'critical';
}

interface SummaryPayload {
  lacisOath: LacisOath;
  summaryOverview: SummaryOverview;
  cameraContext: Record<string, CameraContextItem>;
  cameraDetection: CameraDetectionItem[];
  cameraStatusSummary: CameraStatusSummary;  // 追加
}
```

## 検証方法

### IS22側テスト済み

1. カメラ8台中3台でconnection_lostイベント発生
2. `cameraStatusSummary.systemHealth` = `"degraded"` で送信確認
3. `offlineCameraIds` に該当カメラのLacisID含まれることを確認

### mobes側検証依頼

1. 受信したペイロードに `cameraStatusSummary` が含まれることを確認
2. `systemHealth: "degraded"` または `"critical"` の場合、AIが接続問題を認識して回答することを確認
3. ユーザーへの回答に「○台のカメラで接続問題が発生」等の情報が含まれることを確認

## 関連ドキュメント

- `Paraclate_DesignOverview.md` - ペイロード仕様
- `SCHEMA_DEFINITIONS.md` - スキーマ定義
- `DD02_Summary_GrandSummary.md` - Summary詳細設計

## 備考

- 既存の `cameraDetection` 配列内の `camera_lost` イベントは引き続き送信されます
- `cameraStatusSummary` は集計済みサマリーとして追加されたものです
- 後方互換性のため、mobes側で `cameraStatusSummary` が存在しない場合のフォールバック処理も推奨します

---

**対応完了後、本ドキュメントへの回答または実装完了の連絡をお願いします。**
