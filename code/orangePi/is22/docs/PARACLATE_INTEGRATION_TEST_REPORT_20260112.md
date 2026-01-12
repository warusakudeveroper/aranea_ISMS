# Paraclate統合テスト報告書 兼 mobes2.0対応依頼書

**報告日**: 2026-01-12 (最終更新: 17:45 JST)
**テスト対象**: is22 CamServer - Paraclate APP (mobes2.0) 連携機能
**リポジトリ**: [aranea_ISMS](https://github.com/anthropics/aranea_ISMS) (プライベート)
**ブランチ**: `main`
**テスト環境**: http://192.168.125.246:3000 (HALE京都丹波口)

---

## 目次

1. [テスト結果サマリー](#1-テスト結果サマリー)
2. [is22側 実装完了項目](#2-is22側-実装完了項目)
3. [mobes2.0側 対応依頼事項](#3-mobes20側-対応依頼事項)
4. [詳細テスト結果](#4-詳細テスト結果)
5. [未テスト項目](#5-未テスト項目)
6. [参照リンク](#6-参照リンク)

---

## 1. テスト結果サマリー

| カテゴリ | 機能 | is22側 | mobes2.0側 | 総合判定 | 備考 |
|---------|------|--------|------------|----------|------|
| デバイス登録 | Aranea Registration | OK | OK | **OK** | FID:0150で登録済み |
| Paraclate接続 | connect/disconnect | OK | OK | **OK** | asia-northeast1 |
| 設定同期 | Config Sync | OK | 未確認 | **要確認** | mobes2.0側設定変更待ち |
| Summary生成 | Hourly Summary | OK | - | **OK** | 強制生成API実装済み |
| Summary生成 | Grand Summary | OK | - | **OK** | 強制生成API実装済み |
| Summary送信 | Paraclate APP送信 | 未実装 | 未確認 | **要仕様確認** | エンドポイント未確定 |
| AI Chat | チャットAPI | OK | **404** | **要mobes2.0実装** | 最優先依頼 |
| AI Chat | フロントエンドUI | OK | - | **OK** | モーダル実装完了 |
| カメラ不調検知 | Malfunction Reporter | 部分実装 | 未確認 | **要仕様確認** | |
| PubSub通知 | 設定変更通知受信 | OK | 未確認 | **要確認** | 実通知待ち |
| 送信キュー | Queue管理 | OK | - | **OK** | |

---

## 2. is22側 実装完了項目

### 2.1 Backend (Rust)

| ファイル | 実装内容 |
|---------|---------|
| `src/paraclate_client/types.rs` | AI Chat関連型定義 |
| `src/paraclate_client/client.rs` | `send_ai_chat`メソッド |
| `src/web_api/paraclate_routes.rs` | `/api/paraclate/chat`ルート |
| `src/web_api/summary_routes.rs` | 強制生成API (`force-hourly`, `force-grand`) |

### 2.2 Frontend (React/TypeScript)

| ファイル | 実装内容 |
|---------|---------|
| `src/components/EventLogPane.tsx` | AI Chat API呼び出し統合 |
| `src/components/ChatExpandModal.tsx` | 拡大チャットモーダル（白背景、タイピングアニメーション） |

### 2.3 DB修正

- `cameras.context` → `cameras.camera_context` カラム名修正
- MySQL互換: SUM/COUNT結果のDECIMAL→SIGNED CAST追加

---

## 3. mobes2.0側 対応依頼事項

### 3.1 【最優先】AI Chat エンドポイント実装

**現状**: is22からのAPI呼び出しで **HTTP 404** が返却される

**必要なCloud Function**:

```
関数名: paraclateAIChat
リージョン: asia-northeast1
URL: https://asia-northeast1-mobesorder.cloudfunctions.net/paraclateAIChat
メソッド: POST
Content-Type: application/json
```

**認証ヘッダ** (is22が送信):
```
X-Lacis-ID: 3022E051D815448B0001
X-Lacis-TID: T2025120621041161827
X-Lacis-CIC: 605123
Authorization: Basic <lacisOath Base64>
```

**リクエストBody**:
```typescript
interface AIChatRequest {
  tid: string;                    // "T2025120621041161827"
  fid: string;                    // "0150"
  message: string;                // ユーザーからの質問
  conversationHistory?: Array<{
    role: "user" | "assistant";
    content: string;
  }>;
  context?: {
    cameras: Array<{
      cameraId: string;
      name: string;
      preset: string;
      status: string;
    }>;
    recentDetections: {
      totalCount: number;
      humanCount: number;
      vehicleCount: number;
      byCamera: Array<{
        cameraId: string;
        count: number;
      }>;
    };
  };
}
```

**期待するレスポンス**:
```typescript
interface AIChatResponse {
  ok: boolean;
  message?: string;              // AIからの回答テキスト
  relatedData?: {
    cameraIds?: string[];        // 関連カメラID
    detectionIds?: number[];     // 関連検出ID
    timeRange?: {
      start: string;             // ISO8601
      end: string;
    };
  };
  processingTimeMs?: number;
  error?: string;                // エラー時のみ
}
```

**テスト用curlコマンド** (is22経由):
```bash
curl -X POST 'http://192.168.125.246:3000/api/paraclate/chat' \
  -H 'Content-Type: application/json' \
  -d '{"fid":"0150","message":"最近の検出状況を教えて","autoContext":true}'
```

---

### 3.2 【中優先】Summary送信先エンドポイント仕様確認

is22側でSummary/GrandSummary送信機能を実装するため、以下の情報が必要です:

1. **送信先エンドポイントURL** (例: `paraclateReceiveSummary`)
2. **認証方式** (lacisOath同様か)
3. **リクエストBody仕様**
4. **レスポンス仕様**
5. **エラーハンドリング仕様** (リトライ条件等)

---

### 3.3 【中優先】カメラ不調通知エンドポイント仕様確認

カメラの不調（オフライン、接続エラー等）をParaclate APPに通知する機能について:

1. **通知先エンドポイントURL**
2. **通知トリガー条件** (何秒オフラインで通知等)
3. **通知内容フォーマット**

---

### 3.4 【低優先】PubSub通知の動作確認

設定変更時のPubSub通知受信機能はis22側で実装済みです。
mobes2.0側から実際に通知を発行してテストを行いたいです。

---

## 4. 詳細テスト結果

### 4.1 デバイス登録 (Aranea Registration)

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

**結果**: OK

---

### 4.2 Paraclate接続

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

**結果**: OK

---

### 4.3 Summary強制生成

**Hourly Summary**: `POST /api/summary/force-hourly`

```json
{
  "summaryId": 24,
  "summaryType": "hourly",
  "periodStart": "2026-01-12T05:31:45.902917579Z",
  "periodEnd": "2026-01-12T06:31:45.902917579Z",
  "detectionCount": 0,
  "severityMax": 0,
  "cameraIds": [],
  "createdAt": "2026-01-12T06:31:46.375039507Z"
}
```

**Grand Summary**: `POST /api/summary/force-grand`

```json
{
  "summaryId": 25,
  "summaryType": "grand_summary",
  "periodStart": "2026-01-12T05:31:51.760627869Z",
  "periodEnd": "2026-01-12T06:31:51.760627869Z",
  "detectionCount": 0,
  "severityMax": 0
}
```

**結果**: OK

---

### 4.4 AI Chat API

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

**結果**: **NG** - mobes2.0側エンドポイント未実装

**現在の動作**: is22側フォールバック応答を表示

---

## 5. 未テスト項目

| 項目 | 理由 | 対応 |
|------|------|------|
| AI Chat本番応答 | mobes2.0側エンドポイント未実装 | mobes2.0実装後に再テスト |
| Config Sync | mobes2.0側設定変更が必要 | mobes2.0側で設定変更後に検証 |
| Summary自動送信 | 送信先エンドポイント未確定 | mobes2.0側仕様確定後に実装 |
| PubSub通知受信 | 実通知の発行が必要 | mobes2.0側から通知発行後に検証 |

---

## 6. 参照リンク

### リポジトリ情報

- **リポジトリ**: aranea_ISMS (プライベート)
- **ブランチ**: `main`
- **テスト環境**: http://192.168.125.246:3000

### 関連ドキュメント

- [Paraclate設計概要](../../../is21-22/designs/headdesign/Paraclate/Paraclate_DesignOverview.md)
- [DD03_ParaclateClient](../../../is21-22/designs/headdesign/Paraclate/DetailedDesign/DD03_ParaclateClient.md)
- [スキーマ定義](../../../is21-22/designs/headdesign/Paraclate/SCHEMA_DEFINITIONS.md)

### テナント情報

```
テナント: mijeo.inc
TID: T2025120621041161827
FID: 0150 (HALE京都丹波口ホテル)
```

---

## 連絡先

**is22開発担当**: Claude Code (AI Assistant)
**テスト実施日時**: 2026-01-12 15:30-17:45 JST

---

*このドキュメントはis22 Paraclate統合テストの結果報告およびmobes2.0側への対応依頼書です。*
*mobes2.0側の対応完了後、再テストを実施し本ドキュメントを更新します。*
