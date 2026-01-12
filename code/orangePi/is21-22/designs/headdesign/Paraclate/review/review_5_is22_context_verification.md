# is22 → mobes2.0 AI Chat コンテキスト連携確認書

**作成日**: 2026-01-12
**作成者**: is22開発班 (Claude Code)
**宛先**: mobes2.0開発チーム
**件名**: paraclateAIChat APIのコンテキスト参照に関する確認依頼

---

## 1. 問題の概要

is22からparaclateAIChat APIを呼び出した際、**どのようなコンテキスト（カメラ情報、検出データ等）を送信しても、LLMが同一の汎用応答を返す**状況が発生しています。

### 再現手順

```bash
# FID 0150 (コンテキストあり: カメラ8台、camera_context設定済み、検出データあり)
curl -s -X POST 'http://192.168.125.246:3000/api/paraclate/chat' \
  -H 'Content-Type: application/json' \
  -d '{"fid":"0150","message":"このホテルのカメラについて教えてください","autoContext":true}'

# 応答: {"ok":true,"message":"ご質問いただいた内容について、現在お答えできる情報がございません。"}

# FID 0151 (コンテキスト空: camera_context未設定、検出データなし)
curl -s -X POST 'http://192.168.125.246:3000/api/paraclate/chat' \
  -H 'Content-Type: application/json' \
  -d '{"fid":"0151","message":"このホテルのカメラについて教えてください","autoContext":true}'

# 応答: {"ok":true,"message":"ご質問いただいた内容について、現在お答えできる情報がございません。"}
```

**結果**: 両FIDで全く同じ応答。コンテキストの有無が応答に影響していない。

---

## 2. is22側の送信内容

### 2.1 リクエストペイロード構造

```typescript
interface AIChatRequest {
  tid: string;                    // "T2025120621041161827"
  fid: string;                    // "0150" or "0151"
  message: string;                // ユーザーの質問
  conversationHistory?: ChatMessage[];  // 会話履歴
  context?: AIChatContext;        // ★ コンテキスト情報
}

interface AIChatContext {
  cameras?: CameraContextInfo[];       // カメラ一覧
  recentDetections?: RecentDetectionsSummary;  // 検出サマリー
  facilityContext?: string;            // 施設コンテキスト
}

interface CameraContextInfo {
  cameraId: string;
  name: string;
  location?: string;           // camera_context (カメラ設置場所・用途の説明)
  status: string;              // "online" | "offline"
  lastDetectionAt?: string;    // ISO8601
}

interface RecentDetectionsSummary {
  total24h: number;            // 24時間以内の総検出数
  humanCount: number;          // 人物検知数
  vehicleCount: number;        // 車両検知数
  byCamera?: CameraDetectionCount[];  // カメラ別検出数
}
```

### 2.2 実際に送信されるcontextの例 (FID 0150)

```json
{
  "cameras": [
    {
      "cameraId": "cam-3f23af22-70c0-40d8-b9af-8eb8e272eb27",
      "name": "Tam-Entrance-E2E-Test",
      "location": "HALE京都丹波口ホテルのエントランス上部のカメラです。施設の入り口はここのみなのですべてのゲスト、スタッフはここを通って施設に入ります。右側には画角的に見えませんがQRコードリーダーとインターフォンがあります。",
      "status": "online",
      "lastDetectionAt": "2026-01-12T12:30:00Z"
    },
    // ... 他7台のカメラ
  ],
  "recentDetections": {
    "total24h": 51,
    "humanCount": 45,
    "vehicleCount": 2,
    "byCamera": [
      {"cameraId": "cam-xxx", "cameraName": "Tam-Entrance", "count": 20},
      // ...
    ]
  },
  "facilityContext": null  // attunement.facilityContextから取得（未設定時はnull）
}
```

### 2.3 送信ヘッダ

```
POST https://asia-northeast1-mobesorder.cloudfunctions.net/paraclateAIChat
Content-Type: application/json
Authorization: LacisOath <base64-encoded-json>
```

Authorization ペイロード:
```json
{
  "lacisId": "3022E051D815448B0001",
  "tid": "T2025120621041161827",
  "cic": "605123",
  "timestamp": "2026-01-12T12:34:56Z"
}
```

---

## 3. 確認事項

### 3.1 【最重要】コンテキスト参照方式の確認

is22は**リクエストボディにcontextを含めて送信**しています。

**質問**: mobes2.0のparaclateAIChat実装において、以下のどちらの方式を想定していますか？

| 方式 | 説明 | is22の対応状況 |
|------|------|---------------|
| **A: リクエスト内包方式** | is22がcontextをリクエストに含めて送信し、mobes側がそれを読む | ✅ 実装済み |
| **B: mobes側参照方式** | mobes側がFIDからFirestore等を参照してコンテキストを取得 | ❌ 未対応（Firestoreにカメラ情報等を同期していない） |
| **C: ハイブリッド方式** | リクエストのcontextと、mobes側DBの両方を参照 | △ 部分対応 |

### 3.2 【要確認】LLMプロンプトへのコンテキスト注入

mobes2.0のparaclateAIChat Cloud Function内で、以下のいずれかが実装されていますか？

1. **リクエストのcontextフィールドをLLMプロンプトに含める処理**
2. **FID/TIDからFirestoreを参照してコンテキストを構築する処理**
3. **検出ログ（IngestEvent経由で受信したもの）を参照する処理**

### 3.3 【要確認】mobes側が持つデータとis22側が持つデータの整理

| データ種別 | is22側 | mobes2.0側 | 現在の同期状況 |
|-----------|--------|------------|---------------|
| カメラ一覧 | ✅ cameras テーブル | ？ | ❓ |
| カメラ名・設置場所 | ✅ name, camera_context | ？ | ❓ |
| 検出ログ | ✅ detection_logs テーブル | ？ IngestEventで受信？ | ❓ |
| カメラ画像 | ✅ snapshot_url | ？ | ❓ |
| 施設情報 | △ fid のみ | ✅ FIDから施設マスタ参照 | - |
| 予約情報 | ❌ | ✅ (未連携) | - |
| 部屋情報 (RID) | △ rid フィールドあり | ✅ | ❓ |

---

## 4. is22側の実装状況

### 4.1 実装完了

| 機能 | 状況 | 備考 |
|------|------|------|
| AI Chat API呼び出し | ✅ 完了 | response.text形式に対応済み |
| コンテキスト自動収集 | ✅ 完了 | cameras, recentDetections をリクエストに含める |
| カメラ不調通知 | ✅ 完了 | IngestEvent経由でmobes送信 |
| Summary生成 | ✅ 完了 | force-hourly, force-grand API |

### 4.2 未実装・要確認

| 機能 | 状況 | 備考 |
|------|------|------|
| Summary → Paraclate送信 | ❌ TODO | スケジューラからの自動送信未実装 |
| GrandSummary → Paraclate送信 | ❌ TODO | 同上 |
| カメラメタデータ同期 | ❓ 要確認 | paraclateCameraMetadata エンドポイントへの同期 |

---

## 5. テスト用データ

### FID 0150 (HALE京都丹波口)
- カメラ: 8台登録、camera_context設定済み
- 検出データ: 24時間以内に51件
- is22サーバー: http://192.168.125.246:3000

### FID 0151 (HALE京都東寺)
- カメラ: 登録あり、**camera_context意図的に空**
- 検出データ: なし（比較用）
- is22サーバー: http://192.168.126.xxx:3000 (未稼働)

---

## 6. 期待する回答

1. **AI Chatがコンテキストを読む方式の明確化**
   - リクエストのcontextを参照する実装が入っているか
   - 入っていない場合、いつ実装予定か

2. **必要なデータ同期の明確化**
   - is22からmobes側DBへカメラ情報を同期する必要があるか
   - 検出ログは現在どう保持されているか

3. **Summary送信エンドポイントの仕様確認**
   - paraclateIngestSummary に送信すればAI Chatで参照されるか
   - 送信フォーマットの確認

---

## 7. 関連リンク

- is22 リポジトリ: aranea_ISMS (プライベート)
- テスト環境: http://192.168.125.246:3000
- 前回の確認書: [review_4_response.md](https://github.com/warusakudeveroper/mobes2.0/blob/main/doc/APPS/Paraclate/review/review_4_response.md)

---

## 8. 連絡先

**is22開発班**: Claude Code (AI Assistant)
**確認日時**: 2026-01-12 21:40 JST

---

*本確認書はis22側の実装状況の共有とmobes2.0側の仕様確認を目的としています。*
*コンテキスト連携の齟齬解消のため、早急なご回答をお願いいたします。*
