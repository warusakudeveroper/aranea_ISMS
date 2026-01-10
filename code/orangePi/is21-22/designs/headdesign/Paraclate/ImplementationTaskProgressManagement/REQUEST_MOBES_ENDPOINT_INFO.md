# mobes2.0チームへ: Paraclate E2Eテスト用エンドポイント情報リクエスト

**日付**: 2026-01-11
**依頼元**: IS22開発チーム
**回答期限**: 2026-01-13（月）

---

## 依頼内容

IS22側のParaclate Client実装が完了しました。mobes2.0との連携E2Eテストを実施するため、以下の情報提供をお願いします。

---

## 1. 必要なエンドポイント情報

### 1.1 ベースURL

```
Paraclate APP ベースURL: https://________________________
環境: □本番  □ステージング  □開発
```

### 1.2 エンドポイント一覧（URLを記入してください）

| # | パス | メソッド | 完全URL |
|---|------|---------|---------|
| E1 | `/api/paraclate/connect` | POST | |
| E2 | `/api/paraclate/ingest/summary` | POST | |
| E3 | `/api/paraclate/ingest/event` | POST | |
| E4 | `/api/paraclate/config/{tid}` | GET | |

---

## 2. 認証設定

```
lacisOathヘッダ検証: □有効  □無効（テスト環境のみ）
IP制限: □あり（許可IP:_______________）  □なし
```

---

## 3. テスト用認証情報（確認）

以下の情報でテスト可能か確認してください：

| 項目 | 値 |
|------|-----|
| TID | T2025120621041161827 (mijeo.inc) |
| FID | 0150 (HALE京都丹波口) |
| lacis_id | 18217487937895888001 |
| CIC | 204965 |

```
上記情報でテスト: □可能  □不可（理由:_______________）
```

---

## 4. 実装状態確認

```
Paraclate APP Ingest API実装:
□ 完了・テスト可能
□ 実装中（完了予定:_______________）
□ 未着手
```

---

## 5. 関連ドキュメント

### IS22側（aranea_ISMS）

| ドキュメント | リンク |
|-------------|-------|
| **E2Eテスト計画書** | [E2E_TEST_PLAN_IS22_MOBES.md](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is21-22/designs/headdesign/Paraclate/ImplementationTaskProgressManagement/E2E_TEST_PLAN_IS22_MOBES.md) |
| Phase 4 実装タスク | [Phase4_ParaclateClient.md](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is21-22/designs/headdesign/Paraclate/ImplementationTaskProgressManagement/Phase4_ParaclateClient.md) |
| 実装レポート | [IMPLEMENTATION_REPORT_Phase1-4.md](https://github.com/warusakudeveroper/aranea_ISMS/blob/main/code/orangePi/is21-22/designs/headdesign/Paraclate/ImplementationTaskProgressManagement/IMPLEMENTATION_REPORT_Phase1-4.md) |

### mobes2.0側

| ドキュメント | リンク |
|-------------|-------|
| IS22テスト依頼 | [IS22_TEST_REQUEST.md](https://github.com/warusakudeveroper/mobes2.0/blob/feature/drawboards-facility-access-ui/doc/APPS/Paraclate/IS22_TEST_REQUEST.md) |

---

## 6. 回答方法

以下のいずれかで回答をお願いします：

1. このドキュメントの空欄を埋めてコミット
2. Slackで直接連絡
3. GitHub Issueで回答

---

## 7. IS22側の準備状態

| 項目 | 状態 |
|------|------|
| ParaclateClient実装 | ✅ 完了 |
| lacisOath認証ヘッダ | ✅ 実装済み |
| Summary送信機能 | ✅ 実装済み |
| Event送信機能 | ✅ 実装済み |
| 設定同期機能 | ✅ 実装済み |
| Pub/Sub受信機能 | ✅ 実装済み |
| FID検証 (Issue #119) | ✅ 実装済み |
| テストサーバー | ✅ 192.168.125.246:8080 稼働中 |

**エンドポイント情報をいただき次第、即座にE2Eテストを実行可能です。**

---

## 連絡先

IS22開発チーム
