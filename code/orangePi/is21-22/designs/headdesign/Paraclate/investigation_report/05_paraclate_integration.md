# Paraclate APP連携要件

作成日: 2026-01-10

## 1. 概要

is22からmobes2.0 Paraclate APPへの連携要件を整理する。
現時点でParaclate APPは未実装のため、is22側のインターフェース準備が主目的。

---

## 2. Paraclate APPの機能（設計上）

### 2.1 テナント管理層機能

| 機能 | 説明 |
|------|------|
| カメラ管理 | 傘下カメラのLacisID・設定管理 |
| サマリー閲覧 | is22から受信したSummaryの要約表示 |
| カメラ設定 | リモートでのカメラパラメータ変更 |
| サーバー設定 | is22の設定をmobes側から管理 |

### 2.2 AI機能

| 機能 | 説明 |
|------|------|
| 定時報告 | Summary/GrandSummaryの受信・表示 |
| 対話クエリ | 「今日は何かあった？」等の質問応答 |
| 人物追跡 | カメラ間での検出特徴の移動把握 |
| カメラ不調検知 | 傾向分析によるアラート |

---

## 3. is22 → Paraclate APP連携

### 3.1 エンドポイント（プレースホルダー）

```
https://www.example_endpoint.com/api/paraclate
```

**注意**: 実際のURLはmobes2.0実装後に確定

### 3.2 認証方式（lacisOath）

**認証方式**: lacisOath（lacisID/tid/cic）

```http
POST /api/paraclate/summary
Content-Type: application/json
```

リクエストボディ内の`lacisOath`オブジェクトで認証を行う：

| フィールド | 説明 |
|-----------|------|
| lacisID | デバイス固有識別子（3022...形式） |
| tid | テナントID |
| cic | Client Identification Code（アクティベーション時に発行） |
| blessing | 越境アクセス時のみ使用（permission91+による発行） |

**注意**: JWT/Bearerトークン認証ではなく、lacisOath方式を使用する。
これはaraneaDevice全体で統一された認証方式である。

### 3.3 Summary送信

```json
POST /api/paraclate/summary
{
  "lacisOath": {
    "lacisID": "3022C0742BFCCF950000",
    "tid": "T2025120621041161827",
    "cic": "204965"
  },
  "summaryOverview": {
    "summaryID": "SUM-20260110-001",
    "firstDetectAt": "2026-01-10T00:00:00Z",
    "lastDetectAt": "2026-01-10T01:00:00Z",
    "detectedEvents": 15
  },
  "cameraContext": {
    "3801C0742BFCCF950001": {
      "cameraName": "Entrance",
      "cameraContext": "入口監視",
      "fid": "0150",
      "rid": "R001",
      "preset": "balanced"
    }
  },
  "cameraDetection": [
    {
      "timestamp": "2026-01-10T00:15:32Z",
      "cameraLacisId": "3801C0742BFCCF950001",
      "detectionDetail": "human:2,vehicle:0,severity:2"
    }
  ]
}
```

**フォーマット仕様**（Paraclate_DesignOverview.mdと統一）:
- `lastDetectAt`: 最終検出時刻（旧: fendDetectAt）
- `cameraContext`: オブジェクト形式（キー=lacisID、値=カメラ情報オブジェクト）
- `cameraDetection`: オブジェクト配列形式（timestamp/cameraLacisId/detectionDetail）

---

## 4. Paraclate APP → is22連携

### 4.1 設定同期

```json
POST /api/paraclate/config/sync
{
  "cameras": {
    "3801C0742BFCCF950001": {
      "preset_id": "person_focus",
      "conf_override": 0.4,
      "excluded_objects": ["cat", "dog"]
    }
  },
  "server": {
    "polling_interval_ms": 5000,
    "summary_interval_minutes": 60
  }
}
```

### 4.2 ステータス取得

```json
GET /api/paraclate/status
Response:
{
  "is22_lacis_id": "3022C0742BFCCF950000",
  "uptime_seconds": 86400,
  "camera_count": 18,
  "last_summary_at": "2026-01-10T00:00:00Z",
  "connection_status": "healthy"
}
```

---

## 5. PUB/SUB連携

### 5.1 トピック構成

```
projects/mobesorder/topics/paraclate-{tid}
  └── subscriptions/
      ├── is22-{lacis_id}-config   # 設定変更通知
      └── is22-{lacis_id}-status   # ステータス更新
```

### 5.2 メッセージ形式

```json
{
  "type": "config_update",
  "timestamp": "2026-01-10T00:00:00Z",
  "payload": {
    "camera_id": "3801C0742BFCCF950001",
    "changes": {
      "preset_id": "person_focus"
    }
  }
}
```

---

## 6. AIチャット連携

### 6.1 質問送信

```json
POST /api/paraclate/chat
{
  "lacisOath": { ... },
  "message": "今日は何かあった？",
  "context": {
    "fid": "0150",
    "time_range": "today"
  }
}
```

### 6.2 応答受信

```json
{
  "response": "本日は昨日と比べて来訪者が多い日でした。...",
  "references": [
    {
      "camera_id": "3801C0742BFCCF950001",
      "timestamp": "2026-01-10T15:32:00Z",
      "event_type": "human"
    }
  ],
  "model_used": "gemini-flash"
}
```

---

## 7. is22フロントエンド実装状態

### 7.1 実装済（今回）

| 項目 | ファイル |
|------|---------|
| AIタブ | SettingsModal.tsx |
| エンドポイント設定入力 | SettingsModal.tsx (disabled) |
| 報告間隔設定 | SettingsModal.tsx (disabled) |
| GrandSummary時刻設定 | SettingsModal.tsx (disabled) |
| 報告詳細度設定 | SettingsModal.tsx (disabled) |
| AIアチューンメント設定 | SettingsModal.tsx (disabled) |
| 接続状態表示 | SettingsModal.tsx ("未接続") |

### 7.2 未実装

| 項目 | 理由 |
|------|------|
| 実際の接続処理 | Paraclate APP未実装 |
| Summary送信 | API未実装 |
| 設定同期 | API未実装 |

---

## 8. is22バックエンド実装要件

### 8.1 新規モジュール

```
src/paraclate_client/
├── mod.rs           # モジュール定義
├── types.rs         # リクエスト・レスポンス型
├── http_client.rs   # Paraclate API通信
├── pubsub_client.rs # PUB/SUB通信
└── sync_service.rs  # 設定同期サービス
```

### 8.2 新規APIエンドポイント

| エンドポイント | メソッド | 機能 |
|---------------|---------|------|
| GET /api/paraclate/config | GET | Paraclate連携設定取得 |
| PUT /api/paraclate/config | PUT | Paraclate連携設定更新 |
| POST /api/paraclate/connect | POST | 接続テスト |
| GET /api/paraclate/status | GET | 接続状態取得 |

---

## 9. 依存関係

```
1. AraneaRegister実装
   ↓ (lacisOath取得が必要)
2. TID/FID権限管理
   ↓ (テナント境界が必要)
3. Summary API実装
   ↓ (送信データが必要)
4. Paraclate APP実装 (mobes2.0側)
   ↓
5. is22 Paraclate連携実装
```

---

## 10. MECE確認

- **網羅性**: 双方向連携（送信・受信）、認証、PUB/SUB、チャットを全調査
- **重複排除**: is22側実装とmobes2.0側実装を明確に分離
- **未カバー領域**: Paraclate APP内部実装の詳細はmobes2.0開発ラインの範囲
