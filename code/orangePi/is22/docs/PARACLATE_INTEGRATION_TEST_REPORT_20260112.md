# Paraclate統合テスト報告書

**報告日**: 2026-01-12
**テスト対象**: is22 CamServer - Paraclate APP (mobes2.0) 連携機能
**ブランチ**: `main`
**コミットID**: `ed7af39587cfa85855cba403f90875d1ecfb478b`
**テスト環境**: `http://192.168.125.246:3000` (HALE京都丹波口)

---

## 1. テスト結果サマリー

| カテゴリ | 機能 | is22側 | mobes2.0側 | 総合判定 |
|---------|------|--------|------------|----------|
| デバイス登録 | Aranea Registration | OK | OK | **OK** |
| Paraclate接続 | connect/disconnect | OK | OK | **OK** |
| 設定同期 | Config Sync | OK | 未確認 | **要確認** |
| Summary生成 | Hourly Summary | OK | - | **OK** |
| Summary生成 | Grand Summary | OK | - | **OK** |
| Summary送信 | Paraclate APP送信 | 未実装 | 未確認 | **要実装** |
| AI Chat | チャットAPI | OK | **404 Not Found** | **要mobes2.0実装** |
| カメラ不調検知 | Malfunction Reporter | 部分実装 | 未確認 | **要確認** |
| PubSub通知 | 設定変更通知受信 | OK | 未確認 | **要確認** |
| 送信キュー | Queue管理 | OK | - | **OK** |

---

## 2. 詳細テスト結果

### 2.1 デバイス登録 (Aranea Registration)

**エンドポイント**: `GET /api/register/status`

```json
{
  "registered": true,
  "lacisId": "3022E051D815448B0001",
  "tid": "T2025120621041161827",
  "fid": "0150",
  "cic": "605123",
  "registeredAt": "2026-01-10T09:53:31.354Z"
}
```

**結果**: OK - デバイス登録済み、lacisOath認証情報取得可能

---

### 2.2 Paraclate接続

**エンドポイント**: `POST /api/paraclate/connect`

```bash
curl -X POST 'http://192.168.125.246:3000/api/paraclate/connect?tid=T2025120621041161827' \
  -H 'Content-Type: application/json' \
  -d '{"fid":"0150","endpoint":"asia-northeast1"}'
```

```json
{
  "connected": true,
  "endpoint": "asia-northeast1",
  "configId": 2
}
```

**結果**: OK - Paraclate APPへの接続確立

---

### 2.3 Paraclate設定

**エンドポイント**: `GET /api/paraclate/config`

```json
{
  "attunement": {},
  "configId": 2,
  "endpoint": "https://asia-northeast1-mobesorder.cloudfunctions.net",
  "fid": "0150",
  "grandSummaryTimes": ["09:00", "17:00", "21:00"],
  "lastSyncAt": "2026-01-12T06:28:20.235Z",
  "reportIntervalMinutes": 60,
  "retentionDays": 60,
  "syncSourceTimestamp": null,
  "tid": "T2025120621041161827"
}
```

**結果**: OK - 設定保存・取得正常動作

---

### 2.4 送信キュー

**エンドポイント**: `GET /api/paraclate/queue`

```json
{
  "items": [],
  "total": 0,
  "stats": {
    "pending": 0,
    "sending": 0,
    "failed": 0,
    "sentToday": 0
  }
}
```

**結果**: OK - キュー管理正常動作

---

### 2.5 Summary生成 (Hourly)

**エンドポイント**: `POST /api/summary/force-hourly`

```json
{
  "summaryId": 24,
  "summaryType": "hourly",
  "periodStart": "2026-01-12T05:31:45.902917579Z",
  "periodEnd": "2026-01-12T06:31:45.902917579Z",
  "detectionCount": 0,
  "severityMax": 0,
  "cameraIds": [],
  "createdAt": "2026-01-12T06:31:46.375039507Z",
  "message": "直近1時間のサマリーを生成しました（検出: 0件）"
}
```

**結果**: OK - Hourly Summary強制生成正常動作

---

### 2.6 Grand Summary生成

**エンドポイント**: `POST /api/summary/force-grand`

```json
{
  "summaryId": 25,
  "summaryType": "grand_summary",
  "periodStart": "2026-01-12T05:31:51.760627869Z",
  "periodEnd": "2026-01-12T06:31:51.760627869Z",
  "detectionCount": 0,
  "severityMax": 0,
  "cameraIds": [],
  "createdAt": "2026-01-12T06:31:51.763562330Z",
  "message": "直近1時間のGrandSummaryを生成しました（検出: 0件, Hourly: 0件）"
}
```

**結果**: OK - GrandSummary強制生成正常動作

---

### 2.7 AI Chat API

**エンドポイント**: `POST /api/paraclate/chat`

```bash
curl -X POST 'http://192.168.125.246:3000/api/paraclate/chat' \
  -H 'Content-Type: application/json' \
  -d '{"fid":"0150","message":"最近の検出状況を教えて","autoContext":true}'
```

```json
{
  "ok": false,
  "processingTimeMs": 70,
  "error": "HTTP 404: ... Page not found ..."
}
```

**結果**: **NG (mobes2.0側エンドポイント未実装)**

**is22側実装状況**:
- コンテキスト自動構築: 実装済み
- lacisOath認証ヘッダ付与: 実装済み
- Paraclate APP呼び出し: 実装済み
- フォールバック処理: 実装済み

**mobes2.0側必要対応**:
```
エンドポイント: https://asia-northeast1-mobesorder.cloudfunctions.net/paraclateAIChat
メソッド: POST
Content-Type: application/json

リクエスト例:
{
  "tid": "T2025120621041161827",
  "fid": "0150",
  "message": "最近の検出状況を教えて",
  "context": {
    "cameras": [...],
    "recentDetections": {...}
  },
  "conversationHistory": [...]
}

期待レスポンス:
{
  "ok": true,
  "message": "AIからの回答テキスト",
  "relatedData": {...},
  "processingTimeMs": 1234
}
```

---

### 2.8 Paraclate Status

**エンドポイント**: `GET /api/paraclate/status`

```json
{
  "error_code": "FORBIDDEN",
  "message": "FID 0000 does not belong to TID T2025120621041161827. Actual TID: unknown"
}
```

**結果**: **要確認** - FIDパラメータなしの場合のデフォルト処理に問題あり

---

## 3. mobes2.0側への依頼事項

### 3.1 【緊急】AI Chat エンドポイント実装

**優先度**: 高

is22側の実装は完了しています。mobes2.0側に以下のCloud Functionの実装が必要です。

```
関数名: paraclateAIChat
リージョン: asia-northeast1
URL: https://asia-northeast1-mobesorder.cloudfunctions.net/paraclateAIChat
```

**認証**:
- `X-Lacis-ID`: is22のlacisID
- `X-Lacis-TID`: テナントID
- `X-Lacis-CIC`: Client Identification Code
- `Authorization`: lacisOathトークン (Base64エンコード)

**リクエストBody**:
```typescript
interface AIChatRequest {
  tid: string;
  fid: string;
  message: string;
  conversationHistory?: ChatMessage[];
  context?: {
    cameras: CameraContextInfo[];
    recentDetections: {
      totalCount: number;
      humanCount: number;
      vehicleCount: number;
      byCamera: CameraDetectionCount[];
    };
  };
}
```

**レスポンス**:
```typescript
interface AIChatResponse {
  ok: boolean;
  message?: string;
  relatedData?: {
    cameraIds?: string[];
    detectionIds?: number[];
    timeRange?: { start: string; end: string };
  };
  processingTimeMs?: number;
  error?: string;
}
```

---

### 3.2 Summary/GrandSummary送信先エンドポイント確認

**優先度**: 中

Summary/GrandSummaryのParaclate APP送信機能の実装にあたり、以下の確認が必要です:

1. 送信先エンドポイントURL
2. 認証方式（lacisOath同様か）
3. リクエスト/レスポンス仕様
4. エラーハンドリング仕様

---

### 3.3 カメラ不調検知通知エンドポイント確認

**優先度**: 中

カメラの不調（オフライン、接続エラー等）をParaclate APPに通知する機能について:

1. 通知先エンドポイント
2. 通知トリガー条件
3. 通知内容フォーマット

---

### 3.4 PubSub通知の動作確認

**優先度**: 低

設定変更時のPubSub通知受信機能は実装済みですが、実際の通知送信テストが必要です。

---

## 4. is22側の修正履歴

### 今回のセッションで修正した項目

1. **AI Chat統合実装**
   - `types.rs`: AI Chat関連型追加
   - `client.rs`: `send_ai_chat`メソッド追加
   - `paraclate_routes.rs`: `/api/paraclate/chat`ルート追加
   - `EventLogPane.tsx`: processAIQueryをAPI呼び出しに置換

2. **DBカラム名修正**
   - `context` → `camera_context` (cameras テーブル)

3. **MySQL型互換性修正**
   - SUM/COUNT結果のDECIMAL→SIGNED CAST追加

---

## 5. 未テスト項目

| 項目 | 理由 | 対応予定 |
|------|------|----------|
| Config Sync実行結果 | mobes2.0側の設定変更が必要 | mobes2.0側で設定変更後に検証 |
| Summary自動送信 | 送信先エンドポイント未確定 | mobes2.0側仕様確定後に実装・検証 |
| PubSub通知受信 | 実通知の発行が必要 | mobes2.0側から通知発行後に検証 |

---

## 6. 参照リンク

- **リポジトリ**: aranea_ISMS (プライベート)
- **ブランチ**: `main`
- **コミット**: `ed7af39587cfa85855cba403f90875d1ecfb478b`
- **テスト環境**: http://192.168.125.246:3000

### 関連ドキュメント

- [Paraclate設計概要](../../../is21-22/designs/headdesign/Paraclate/Paraclate_DesignOverview.md)
- [DD03_ParaclateClient](../../../is21-22/designs/headdesign/Paraclate/DetailedDesign/DD03_ParaclateClient.md)
- [スキーマ定義](../../../is21-22/designs/headdesign/Paraclate/SCHEMA_DEFINITIONS.md)

---

## 7. 連絡先

**is22開発担当**: Claude Code (AI Assistant)
**テスト実施日時**: 2026-01-12 15:30 JST

---

*このドキュメントはis22 Paraclate統合テストの結果報告です。mobes2.0側の対応完了後、再テストを実施します。*
