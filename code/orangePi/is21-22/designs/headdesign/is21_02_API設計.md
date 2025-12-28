# is21 API設計

文書番号: is21_02
作成日: 2025-12-29
ステータス: Draft
参照: 特記仕様1 - OpenAPI定義

---

## 1. API概要

### 1.1 基本情報
| 項目 | 値 |
|-----|-----|
| ベースURL | http://is21:9000 |
| プロトコル | HTTP/1.1 |
| 認証 | なし（VPN/LAN内限定運用） |
| Content-Type | application/json, multipart/form-data |

### 1.2 エンドポイント一覧
| メソッド | パス | 用途 |
|---------|------|------|
| GET | /healthz | ヘルスチェック |
| GET | /v1/capabilities | ランタイム・モデル情報 |
| GET | /v1/schema | 現在のスキーマ取得 |
| PUT | /v1/schema | スキーマ更新 |
| POST | /v1/analyze | 画像推論 |

---

## 2. /healthz

### 2.1 リクエスト
```http
GET /healthz HTTP/1.1
Host: is21:9000
```

### 2.2 レスポンス（200 OK）
```json
{
  "status": "ok",
  "uptime_sec": 3600
}
```

### 2.3 エラー時
- 500: 内部エラー
- 推論サービス未起動時は接続拒否

---

## 3. /v1/capabilities

### 3.1 用途
- is22起動時にis21の能力を確認
- サポートするprimary_events・推奨サイズを取得

### 3.2 レスポンス（200 OK）
```json
{
  "api_version": "1.0.0",
  "runtime": {
    "npu": "rknpu2",
    "os": "Armbian Ubuntu 24.04"
  },
  "model": {
    "name": "yolo-person-lite",
    "version": "2025.12.28"
  },
  "supported_primary_events": [
    "none", "human", "animal", "vehicle",
    "hazard", "camera_issue", "object_missing", "unknown"
  ],
  "max_image_bytes": 5000000,
  "recommended_infer_widths": [640, 960]
}
```

---

## 4. /v1/schema

### 4.1 GET（現在のスキーマ取得）

#### リクエスト
```http
GET /v1/schema HTTP/1.1
Host: is21:9000
```

#### レスポンス（200 OK）
```json
{
  "schema_version": "2025-12-28.1",
  "schema": { /* schema.json本体 */ },
  "received_at": "2025-12-28T12:00:00Z"
}
```

#### エラー
- 404: スキーマ未設定

### 4.2 PUT（スキーマ更新）

#### リクエスト
```http
PUT /v1/schema HTTP/1.1
Host: is21:9000
Content-Type: application/json

{
  "schema_version": "2025-12-28.1",
  "schema": { /* schema.json本体 */ }
}
```

#### レスポンス（200 OK）
```json
{
  "schema_version": "2025-12-28.1",
  "status": "accepted"
}
```

#### エラー
- 400: スキーマ形式不正

---

## 5. /v1/analyze

### 5.1 リクエスト
```http
POST /v1/analyze HTTP/1.1
Host: is21:9000
Content-Type: multipart/form-data; boundary=----boundary

------boundary
Content-Disposition: form-data; name="camera_id"

cam_2f_19
------boundary
Content-Disposition: form-data; name="captured_at"

2025-12-28T18:59:12+09:00
------boundary
Content-Disposition: form-data; name="schema_version"

2025-12-28.1
------boundary
Content-Disposition: form-data; name="profile"

standard
------boundary
Content-Disposition: form-data; name="return_bboxes"

true
------boundary
Content-Disposition: form-data; name="infer_image"; filename="infer.jpg"
Content-Type: image/jpeg

<JPEG binary data>
------boundary--
```

### 5.2 リクエストパラメータ
| パラメータ | 必須 | 型 | 説明 |
|-----------|-----|-----|------|
| camera_id | ○ | string | カメラ識別子 |
| captured_at | ○ | datetime | キャプチャ時刻（ISO8601） |
| schema_version | ○ | string | 期待するスキーマバージョン |
| infer_image | ○ | file | 推論用JPEG/PNG |
| request_id | - | string | トレーシング用ID |
| profile | - | enum | fast/standard（デフォルト: standard） |
| return_bboxes | - | bool | bbox返却（デフォルト: true） |
| hints_json | - | string | ヒントJSON（time_band等） |
| prev_infer_image | - | file | 差分用前回画像（オプション） |

### 5.3 レスポンス（200 OK）
```json
{
  "schema_version": "2025-12-28.1",
  "camera_id": "cam_2f_19",
  "captured_at": "2025-12-28T18:59:12+09:00",
  "analyzed": true,
  "detected": true,
  "primary_event": "human",
  "tags": [
    "count.single",
    "top_color.red",
    "behavior.loitering"
  ],
  "confidence": 0.82,
  "severity": 2,
  "unknown_flag": false,
  "count_hint": 1,
  "bboxes": [
    {
      "x1": 0.31, "y1": 0.12,
      "x2": 0.62, "y2": 0.88,
      "label": "person",
      "conf": 0.82
    }
  ],
  "processing_ms": 127,
  "model_info": {
    "name": "yolo-person-lite",
    "version": "2025.12.28"
  }
}
```

### 5.4 レスポンスフィールド
| フィールド | 型 | 説明 |
|-----------|-----|------|
| analyzed | bool | 推論実施フラグ |
| detected | bool | 何か検知（none以外） |
| primary_event | string | 主イベント（schema.primary_eventsの値） |
| tags | string[] | タグID配列（schema.tags[].id） |
| confidence | number | 信頼度（0.0-1.0） |
| severity | int | 重要度（0-3） |
| unknown_flag | bool | 不明/要確認フラグ |
| count_hint | int | 検知数（任意） |
| bboxes | array | バウンディングボックス（正規化座標） |
| processing_ms | int | 処理時間（ms） |

### 5.5 エラーレスポンス
| ステータス | error | 説明 |
|-----------|-------|------|
| 400 | bad_request | 画像形式不正・パラメータ不足 |
| 409 | schema_mismatch | スキーマバージョン不一致 |
| 429 | overloaded | 処理過多（バックプレッシャ） |
| 503 | unavailable | サービス一時停止 |

#### 409 schema_mismatch 時の対応
1. is22は `PUT /v1/schema` でスキーマを再プッシュ
2. 再度 `POST /v1/analyze` を送信

---

## 6. タイムアウト設計

### 6.1 is22→is21通信
| 操作 | タイムアウト |
|-----|------------|
| healthz | 3秒 |
| capabilities | 5秒 |
| schema GET/PUT | 5秒 |
| analyze | 10秒 |

### 6.2 is21内部処理
| 処理 | 目標時間 |
|-----|---------|
| 640px画像推論 | 300ms |
| 960px画像推論 | 500ms |
| bbox描画（オプション） | 50ms |

---

## 7. テスト観点

### 7.1 正常系
- [ ] 各エンドポイントが期待通りのレスポンスを返す
- [ ] schema_version一致時にanalyzeが成功
- [ ] bboxesが正規化座標（0-1）で返る

### 7.2 異常系
- [ ] 不正JPEG（破損）で400
- [ ] schema未設定でanalyze時に409
- [ ] 巨大画像（>5MB）で400
- [ ] 同時多数リクエストで429

### 7.3 境界値
- [ ] 最小画像（1x1px）
- [ ] 最大画像（4096px幅）
- [ ] 空のtags配列
- [ ] 検知なし（detected=false）
