IS22側からのリクエストに回答します。

  Paraclate E2Eテスト用エンドポイント情報

  1. 本番エンドポイント一覧
  ┌───────────────┬──────────┬────────────────────────────────────────────────────────────────────────┐
  │      API      │ メソッド │                                完全URL                                 │
  ├───────────────┼──────────┼────────────────────────────────────────────────────────────────────────┤
  │ Connect       │ POST     │ https://paraclateconnect-vm44u3kpua-an.a.run.app                       │
  ├───────────────┼──────────┼────────────────────────────────────────────────────────────────────────┤
  │ IngestSummary │ POST     │ https://paraclateingestsummary-vm44u3kpua-an.a.run.app                 │
  ├───────────────┼──────────┼────────────────────────────────────────────────────────────────────────┤
  │ IngestEvent   │ POST     │ https://paraclateingestevent-vm44u3kpua-an.a.run.app                   │
  ├───────────────┼──────────┼────────────────────────────────────────────────────────────────────────┤
  │ GetConfig     │ GET      │ https://paraclategetconfig-vm44u3kpua-an.a.run.app?tid={tid}&fid={fid} │
  └───────────────┴──────────┴────────────────────────────────────────────────────────────────────────┘
  2. 認証設定
  ┌───────────────┬────────────────────────────────────────┐
  │     項目      │                   値                   │
  ├───────────────┼────────────────────────────────────────┤
  │ LacisOath認証 │ 有効                                   │
  ├───────────────┼────────────────────────────────────────┤
  │ IP制限        │ なし（invoker: 'public'）              │
  ├───────────────┼────────────────────────────────────────┤
  │ ヘッダー形式  │ Authorization: LacisOath <base64-json> │
  └───────────────┴────────────────────────────────────────┘
  3. 認証ペイロード形式

  {
    "lacisId": "<IS22デバイスのLacisID>",
    "tid": "<テナントID>",
    "cic": "<CICコード>",
    "timestamp": "<ISO8601>"
  }

  4. リクエストボディ形式（重要）

  ネスト構造必須:
  {
    "fid": "<施設ID>",
    "payload": {
      "summary": { ... },
      "periodStart": "...",
      "periodEnd": "..."
    }
  }

  5. テスト済み情報
  ┌─────────────────┬──────────────────────┐
  │      項目       │          値          │
  ├─────────────────┼──────────────────────┤
  │ テスト用LacisID │ 3022E051D815448B0001 │
  ├─────────────────┼──────────────────────┤
  │ テスト用TID     │ T2025120621041161827 │
  ├─────────────────┼──────────────────────┤
  │ テスト用CIC     │ 605123               │
  ├─────────────────┼──────────────────────┤
  │ テスト用FID     │ 0099                 │
  └─────────────────┴──────────────────────┘
  6. mobes側テスト依頼ドキュメント

  https://github.com/warusakudeveroper/mobes2.0/blob/feature/drawboards-facility-access-ui/doc/APPS/Paraclate/IS22_TEST_REQUEST.md

  ---
  IS22側のE2Eテスト計画書とリクエストドキュメントを確認しました。上記情報でE2Eテストを実施可能です。


  # 追記1

  GetConfig APIも正常動作確認。正確なスキーマ情報をまとめます。

  ---
  IS22向け 正確なAPIスキーマ回答

  1. IngestSummary API

  POST https://paraclateingestsummary-vm44u3kpua-an.a.run.app

  ペイロード構造:
  {
    "fid": "0099",
    "payload": {
      "summary": {
        "summaryId": "sum_is22_xxx"
      },
      "periodStart": "2026-01-11T00:00:00Z",
      "periodEnd": "2026-01-11T23:59:59Z"
    }
  }

  summaryIdの位置: payload.summary.summaryId

  ---
  2. GetConfig API

  GET https://paraclategetconfig-vm44u3kpua-an.a.run.app/config/{tid}?fid={fid}
  ┌────────────┬──────────────────┬──────────────────────────────┐
  │ パラメータ │       位置       │             形式             │
  ├────────────┼──────────────────┼──────────────────────────────┤
  │ tid        │ パスパラメータ   │ /config/T2025120621041161827 │
  ├────────────┼──────────────────┼──────────────────────────────┤
  │ fid        │ クエリパラメータ │ ?fid=0099                    │
  └────────────┴──────────────────┴──────────────────────────────┘
  テスト確認済みレスポンス:
  {
    "success": true,
    "config": {
      "schedule": {"summaryInterval": 60, "eventBatchSize": 10},
      "retention": {"imagesDays": 60, "summaryDays": 60},
      "chatApi": {"enabled": true, "maxTokens": 4096}
    },
    "resolvedFrom": "hardcoded",
    "etag": "ab9d5bc4..."
  }

  ---
  3. IngestEvent API

  POST https://paraclateingestevent-vm44u3kpua-an.a.run.app

  ペイロード構造:
  {
    "fid": "0099",
    "payload": {
      "event": {
        "detection_log_id": "is22_event_xxx"
      }
    }
  }

  ---
  全API動作確認済みです。