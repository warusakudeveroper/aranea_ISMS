# IS22 - mobes2.0 E2Eアクセステスト計画書

作成日: 2026-01-11
ステータス: **mobes2.0側エンドポイント情報待ち**
対象: IS22 Paraclate Client ↔ mobes2.0 Paraclate APP 連携確認

---

## 1. 概要

### 1.1 目的
IS22のParaclateClient実装が、mobes2.0のParaclate APP APIと正しく連携できることを検証する。

### 1.2 テスト環境

| 項目 | IS22側 | mobes2.0側 |
|------|--------|-----------|
| サーバー | 192.168.125.246:8080 | **要確認** |
| TID | T2025120621041161827 | 同左 |
| FID | 0150, 0151 | 同左 |
| 認証 | lacisOath (lacis_id, tid, cic) | 同左検証 |

### 1.3 前提条件
- IS22側: Phase 4 ParaclateClient実装完了 ✅
- IS22側: Issue #119 FID検証実装完了 ✅
- mobes2.0側: Paraclate APP Ingest API実装完了 **要確認**

---

## 2. mobes2.0側に必要なエンドポイント情報

### 2.1 必須エンドポイント

IS22から呼び出すエンドポイントの**完全なURL**を提供してください。

| # | エンドポイント | メソッド | 用途 | URL（要記入） |
|---|---------------|---------|------|--------------|
| E1 | `/api/paraclate/connect` | POST | 接続テスト | `https://________/api/paraclate/connect` |
| E2 | `/api/paraclate/ingest/summary` | POST | Summary送信 | `https://________/api/paraclate/ingest/summary` |
| E3 | `/api/paraclate/ingest/event` | POST | Event送信 | `https://________/api/paraclate/ingest/event` |
| E4 | `/api/paraclate/config/{tid}` | GET | 設定取得 | `https://________/api/paraclate/config/{tid}` |

### 2.2 環境情報

| 項目 | 値（要記入） |
|------|-------------|
| ベースURL | `https://________________` |
| 環境 | 本番 / ステージング / 開発 |
| 認証方式 | lacisOath ヘッダ検証 有効/無効 |
| IP制限 | あり(許可IP:______) / なし |
| CORS設定 | 設定済み / 未設定 |

### 2.3 テスト用認証情報

| 項目 | 値（要記入） |
|------|-------------|
| テスト用TID | T2025120621041161827 |
| テスト用FID | 0150 |
| テスト用lacis_id | 18217487937895888001 |
| テスト用CIC | 204965 |
| blessing | 不要 / 必要（値:______） |

---

## 3. テストケース

### 3.1 T-CONN: 接続テスト

**目的**: IS22からmobes2.0への基本接続確認

**リクエスト**:
```http
POST /api/paraclate/connect
Content-Type: application/json
X-Lacis-ID: 18217487937895888001
X-Lacis-TID: T2025120621041161827
X-Lacis-CIC: 204965

{
  "tid": "T2025120621041161827",
  "fid": "0150",
  "deviceType": "is22",
  "version": "0.1.0"
}
```

**期待レスポンス**:
```json
{
  "ok": true,
  "message": "Connection successful",
  "serverTime": "2026-01-11T12:00:00Z"
}
```

**合格基準**:
- [ ] HTTPステータス 200
- [ ] `ok: true` が返却される
- [ ] レスポンスタイム < 5秒

---

### 3.2 T-SUM: Summary送信テスト

**目的**: 検出サマリーをmobes2.0に送信できることを確認

**リクエスト**:
```http
POST /api/paraclate/ingest/summary
Content-Type: application/json
X-Lacis-ID: 18217487937895888001
X-Lacis-TID: T2025120621041161827
X-Lacis-CIC: 204965

{
  "tid": "T2025120621041161827",
  "fid": "0150",
  "summaryType": "hourly",
  "periodStart": "2026-01-11T11:00:00Z",
  "periodEnd": "2026-01-11T12:00:00Z",
  "totalDetections": 15,
  "byCamera": [
    {
      "cameraId": "cam-3f23af22-70c0-40d8-b9af-8eb8e272eb27",
      "cameraName": "エントランス",
      "detectionCount": 8,
      "topEvents": ["person_detected"]
    }
  ],
  "generatedAt": "2026-01-11T12:00:05Z"
}
```

**期待レスポンス**:
```json
{
  "ok": true,
  "summaryId": "sum_xxxxxxxx",
  "receivedAt": "2026-01-11T12:00:06Z"
}
```

**合格基準**:
- [ ] HTTPステータス 200
- [ ] `ok: true` が返却される
- [ ] `summaryId` が返却される
- [ ] mobes2.0側でサマリーが記録される

---

### 3.3 T-EVT: Event送信テスト（snapshot付き）

**目的**: 検出イベントとsnapshotをmobes2.0に送信できることを確認

**リクエスト**:
```http
POST /api/paraclate/ingest/event
Content-Type: multipart/form-data
X-Lacis-ID: 18217487937895888001
X-Lacis-TID: T2025120621041161827
X-Lacis-CIC: 204965

--boundary
Content-Disposition: form-data; name="event"
Content-Type: application/json

{
  "tid": "T2025120621041161827",
  "fid": "0150",
  "eventType": "person_detected",
  "cameraId": "cam-3f23af22-70c0-40d8-b9af-8eb8e272eb27",
  "cameraName": "エントランス",
  "detectedAt": "2026-01-11T12:05:30Z",
  "confidence": 0.92,
  "boundingBox": {"x": 100, "y": 150, "w": 80, "h": 200}
}
--boundary
Content-Disposition: form-data; name="snapshot"; filename="snapshot.jpg"
Content-Type: image/jpeg

<binary data>
--boundary--
```

**期待レスポンス**:
```json
{
  "ok": true,
  "eventId": "evt_xxxxxxxx",
  "snapshot": {
    "tid": "T2025120621041161827",
    "fileId": "snapshot_evt_xxxxxxxx",
    "storagePath": "lacisFiles/T2025120621041161827/snapshot_evt_xxxxxxxx.jpg"
  }
}
```

**合格基準**:
- [ ] HTTPステータス 200
- [ ] `ok: true` が返却される
- [ ] `eventId` が返却される
- [ ] `snapshot.storagePath` が返却される（署名URLではなく恒久参照子）
- [ ] mobes2.0側でイベントが記録される
- [ ] snapshotがCloud Storageにアップロードされる

---

### 3.4 T-CFG: 設定取得テスト

**目的**: mobes2.0から設定を取得できることを確認

**リクエスト**:
```http
GET /api/paraclate/config/T2025120621041161827?fid=0150
X-Lacis-ID: 18217487937895888001
X-Lacis-TID: T2025120621041161827
X-Lacis-CIC: 204965
```

**期待レスポンス**:
```json
{
  "tid": "T2025120621041161827",
  "fid": "0150",
  "reportIntervalMinutes": 60,
  "grandSummaryTimes": ["09:00", "17:00", "21:00"],
  "retentionDays": 60,
  "attunement": {
    "autoTuningEnabled": true,
    "tuningFrequency": "weekly",
    "tuningAggressiveness": 50
  },
  "updatedAt": "2026-01-10T10:00:00Z"
}
```

**合格基準**:
- [ ] HTTPステータス 200
- [ ] 設定JSONが返却される
- [ ] `reportIntervalMinutes`, `grandSummaryTimes` が含まれる

---

### 3.5 T-ERR: エラーハンドリングテスト

**目的**: 不正リクエスト時のエラーレスポンスを確認

| テストID | 条件 | 期待HTTPステータス | 期待エラーコード |
|---------|------|-------------------|-----------------|
| T-ERR-1 | 認証ヘッダなし | 401 | `UNAUTHORIZED` |
| T-ERR-2 | 無効なTID | 403 | `FORBIDDEN` |
| T-ERR-3 | 無効なCIC | 403 | `INVALID_CIC` |
| T-ERR-4 | 不正なJSON | 400 | `BAD_REQUEST` |

---

## 4. 合格基準サマリー

### 4.1 必須テスト（全パス必須）

| テストID | テスト名 | 合格条件 |
|---------|---------|---------|
| T-CONN | 接続テスト | HTTP 200, ok:true |
| T-SUM | Summary送信 | HTTP 200, summaryId返却 |
| T-CFG | 設定取得 | HTTP 200, 設定JSON返却 |

### 4.2 推奨テスト

| テストID | テスト名 | 合格条件 |
|---------|---------|---------|
| T-EVT | Event送信 | HTTP 200, storagePath返却 |
| T-ERR-* | エラーハンドリング | 適切なエラーコード |

### 4.3 全体合格基準

- **必須テスト全パス**: 3/3 (100%)
- **推奨テスト**: 4/5 以上 (80%以上)
- **レスポンスタイム**: 平均 < 3秒、最大 < 10秒

---

## 5. IS22側テスト実行コマンド

mobes2.0エンドポイント情報取得後、以下のコマンドでテスト実行可能：

### 5.1 接続テスト
```bash
# IS22 APIから接続テスト実行
curl -X POST "http://192.168.125.246:8080/api/paraclate/connect?fid=0150" \
  -H "Content-Type: application/json" \
  -d '{"fid": "0150", "endpoint": "https://<MOBES_URL>"}'
```

### 5.2 手動キュー処理（Summary送信）
```bash
# キューに溜まったSummaryを送信
curl -X POST "http://192.168.125.246:8080/api/paraclate/queue/process?fid=0150"
```

### 5.3 ステータス確認
```bash
curl "http://192.168.125.246:8080/api/paraclate/status?fid=0150"
```

---

## 6. mobes2.0側への依頼事項

### 6.1 必須提供情報

1. **エンドポイントURL**（セクション2.1）
2. **環境情報**（セクション2.2）
3. **認証設定状態**（lacisOathヘッダ検証の有効/無効）

### 6.2 確認事項

1. Paraclate APP Ingest APIの実装状態
2. テスト環境へのアクセス可否
3. IP制限がある場合、192.168.125.246 の許可

### 6.3 回答期限

**2026-01-13（月）** までに上記情報の提供をお願いします。

---

## 7. 関連ドキュメント

| ドキュメント | リンク |
|-------------|-------|
| IS22テスト依頼（mobes2.0側作成） | [IS22_TEST_REQUEST.md](https://github.com/warusakudeveroper/mobes2.0/blob/feature/drawboards-facility-access-ui/doc/APPS/Paraclate/IS22_TEST_REQUEST.md) |
| Phase 4 実装タスク | [Phase4_ParaclateClient.md](./Phase4_ParaclateClient.md) |
| 詳細設計 DD03 | [DD03_ParaclateClient.md](../DetailedDesign/DD03_ParaclateClient.md) |
| 実装レポート | [IMPLEMENTATION_REPORT_Phase1-4.md](./IMPLEMENTATION_REPORT_Phase1-4.md) |

---

## 更新履歴

| 日付 | 更新内容 |
|------|---------|
| 2026-01-11 | 初版作成 |
