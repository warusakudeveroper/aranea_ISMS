# IS21/IS22 修正仕様書（最終確定版）

**作成日時**: 2026-01-03
**ステータス**: 承認待ち
**対象**: is21 (AI推論サーバー) / is22 (Camserver)

---

## 1. 承認済み問題と修正方針

### 1.1 Issue 2.1: suspicious_score カラム型不整合【IS22修正】

| 項目 | 現状 | 修正後 |
|------|------|--------|
| カラム定義 | `DECIMAL(5,4)` | `INT` |
| 格納可能値 | 0.0000〜9.9999 | 0〜100 |

**修正ファイル**: `migrations/008_detection_logs.sql`

```sql
-- 修正前
suspicious_score DECIMAL(5,4) AS (JSON_VALUE(suspicious, '$.score')) STORED

-- 修正後
suspicious_score INT AS (JSON_VALUE(suspicious, '$.score')) STORED
```

**マイグレーション手順**:
```sql
ALTER TABLE detection_logs DROP COLUMN suspicious_score;
ALTER TABLE detection_logs ADD COLUMN suspicious_score INT AS (JSON_VALUE(suspicious, '$.score')) STORED;
-- インデックス再作成（必要に応じて）
```

---

### 1.2 Issue 2.2: プリセット機能【IS21修正 + IS22追加】

#### 1.2.1 IS21 プリセット管理アーキテクチャ

**設計原則**:
- 1 IS21 対 複数 IS22 の構成を考慮
- IS22毎に異なるプリセットを持つ可能性を考慮
- SDカードからの毎回読み込みは**禁止**（パフォーマンス・ライフサイクル問題）

**データ構造**:
```
┌─────────────────────────────────────────────────────────────┐
│  IS21 メモリ上のプリセットキャッシュ                        │
├─────────────────────────────────────────────────────────────┤
│  Key: lacis_id (IS22のlacisID)                              │
│  Value: PresetData                                          │
│    - preset_id: str                                         │
│    - preset_version: str                                    │
│    - detection_config: dict (閾値、有効/無効イベント等)     │
│    - updated_at: datetime                                   │
└─────────────────────────────────────────────────────────────┘
```

**ファイル永続化** (SD):
```
/opt/is21/data/presets/
  └── {lacis_id}.json   # 各IS22のプリセット
```

**起動時の動作**:
1. `/opt/is21/data/presets/*.json` を全て読み込み
2. メモリ上のキャッシュに展開
3. 以降はメモリから参照（SDアクセスなし）

#### 1.2.2 IS21 プリセット保存API

**エンドポイント**: `POST /v1/presets/{lacis_id}`

```python
@app.post("/v1/presets/{lacis_id}")
async def save_preset(
    lacis_id: str,
    preset_id: str = Form(...),
    preset_version: str = Form(...),
    detection_config: str = Form(...)  # JSON文字列
):
    """
    IS22からのプリセット登録を受け付け
    1. JSONファイルとしてSD保存
    2. メモリキャッシュを更新
    """
```

**レスポンス**:
```json
{
  "ok": true,
  "lacis_id": "30066CC8408C31B00001",
  "preset_id": "parking_lot",
  "preset_version": "1.2.0",
  "cached_at": "2026-01-03T15:30:00Z"
}
```

#### 1.2.3 IS21 analyzeエンドポイント拡張

```python
@app.post("/v1/analyze")
async def analyze(
    camera_id: str = Form(...),
    captured_at: str = Form(...),
    schema_version_req: str = Form(..., alias="schema_version"),
    infer_image: UploadFile = File(...),
    lacis_id: str = Form(...),           # 追加: IS22のlacisID
    preset_id: str = Form("balanced"),   # 追加: プリセットID
    preset_version: str = Form("1.0.0"), # 追加: プリセットバージョン
    profile: str = Form("standard"),
    return_bboxes: bool = Form(True),
    hints_json: Optional[str] = Form(None)
):
    # lacis_idからメモリキャッシュのプリセットを参照
    preset = preset_cache.get(lacis_id)
    if preset and preset.preset_id == preset_id:
        # プリセット設定を適用して推論実行
        ...
```

#### 1.2.4 IS22 プリセット同期UI

**設定画面に追加**:
- 「プリセット同期」ボタン
- 明示的にIS22→IS21への登録操作
- 同期ステータス表示（最終同期日時、成功/失敗）

**API**: `POST /api/cameras/{camera_id}/sync-preset`

```rust
// IS22 → IS21 への同期処理
async fn sync_preset_to_is21(camera: &Camera, preset: &Preset) -> Result<()> {
    let is21_url = format!("{}/v1/presets/{}", IS21_BASE_URL, camera.lacis_id);
    let form = multipart::Form::new()
        .text("preset_id", preset.preset_id.clone())
        .text("preset_version", preset.preset_version.clone())
        .text("detection_config", serde_json::to_string(&preset.config)?);

    client.post(&is21_url).multipart(form).send().await?;
    Ok(())
}
```

---

### 1.3 Issue 2.3: frame_diff機能【IS21修正】

#### 1.3.1 IS21 prev_imageパラメータ追加

```python
@app.post("/v1/analyze")
async def analyze(
    # ... 既存パラメータ ...
    prev_image: Optional[UploadFile] = File(None),  # 追加
    enable_frame_diff: bool = Form(True),           # 追加
):
```

#### 1.3.2 IS21 frame_diff応答追加

```json
{
  "primary_event": "person",
  "severity": 1,
  "suspicious": { "score": 28, "level": "low", "factors": [] },
  "frame_diff": {
    "motion_detected": true,
    "motion_area_percent": 12.5,
    "camera_status": "normal",
    "loitering_seconds": 0
  }
}
```

---

## 2. 新規追加要件: パフォーマンスログ記録

### 2.1 IS22側ログ記録

**記録項目**:

| 項目 | 説明 | 単位 |
|------|------|------|
| `camera_response_ms` | カメラからの画像取得応答時間 | ms |
| `image_size_bytes` | 取得画像サイズ | bytes |
| `is21_send_ms` | IS21への送信時間 | ms |
| `is21_response_ms` | IS21からのレスポンス応答時間 | ms |
| `is21_inference_ms` | IS21側の推論実行時間（レスポンスから取得） | ms |
| `is21_preset_id` | 使用されたプリセットID | string |
| `total_pipeline_ms` | 全体パイプライン処理時間 | ms |

**ログローテーション**:
- 最大1000件保持
- 1週間経過で削除

**DB定義** (新規Migration):
```sql
CREATE TABLE IF NOT EXISTS performance_logs (
    id INT AUTO_INCREMENT PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL,
    lacis_id VARCHAR(20) NOT NULL,
    captured_at DATETIME NOT NULL,

    -- カメラ側パフォーマンス
    camera_response_ms INT COMMENT 'カメラ応答時間(ms)',
    image_size_bytes INT COMMENT '画像サイズ(bytes)',

    -- IS21通信パフォーマンス
    is21_send_ms INT COMMENT 'IS21送信時間(ms)',
    is21_response_ms INT COMMENT 'IS21レスポンス時間(ms)',

    -- IS21側パフォーマンス（レスポンスから取得）
    is21_inference_ms INT COMMENT 'IS21推論時間(ms)',
    is21_preset_id VARCHAR(64) COMMENT '使用プリセットID',

    -- 全体
    total_pipeline_ms INT COMMENT '全体処理時間(ms)',

    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_camera_created (camera_id, created_at),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

**ローテーション処理** (Rust):
```rust
// 1000件超過時に古いレコードを削除
async fn rotate_performance_logs(pool: &MySqlPool) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM performance_logs
        WHERE id NOT IN (
            SELECT id FROM (
                SELECT id FROM performance_logs
                ORDER BY created_at DESC
                LIMIT 1000
            ) AS keep
        )
        "#
    ).execute(pool).await?;

    // 1週間以上前のログも削除
    sqlx::query!(
        "DELETE FROM performance_logs WHERE created_at < DATE_SUB(NOW(), INTERVAL 7 DAY)"
    ).execute(pool).await?;

    Ok(())
}
```

### 2.2 IS21側レスポンス拡張

**レスポンスに追加**:
```json
{
  "primary_event": "person",
  "severity": 1,
  "suspicious": { ... },
  "frame_diff": { ... },
  "performance": {
    "inference_ms": 245,
    "preset_id": "parking_lot",
    "preset_version": "1.2.0",
    "model_load_cached": true
  }
}
```

### 2.3 パフォーマンス統計UI【ISSUE登録】

**実装タイミング**: 完動後、実パフォーマンス検証完了後

**UI仕様案**:
- 設定ボタン → パフォーマンス統計
- グラフ表示（時系列）
  - カメラ応答時間
  - IS21応答時間
  - 全体パイプライン時間
- 異常検知（閾値超過ハイライト）

**ISSUE内容**:
```
タイトル: [is22] パフォーマンス統計UI実装
本文:
## 概要
performance_logsテーブルのデータを可視化するUIを実装

## 前提条件
- performance_logsへの記録が正常動作すること
- 実運用で十分なデータが蓄積されていること
- 実際のパフォーマンス値の範囲が判明していること

## 実装内容
- 設定画面にパフォーマンス統計リンク追加
- 時系列グラフ（折れ線）
- カメラ別フィルタ
- 異常値ハイライト

## 優先度
低（完動・検証後）
```

---

## 3. 実装順序

```
Phase 1: 致命的問題の解消
├── [IS22] suspicious_score INT修正
└── [IS22] マイグレーション実行

Phase 2: プリセット機能
├── [IS21] プリセット保存API実装
├── [IS21] メモリキャッシュ機構実装
├── [IS21] analyzeエンドポイント拡張
├── [IS22] プリセット同期API実装
└── [IS22] 設定画面にプリセット同期UI追加

Phase 3: frame_diff機能
├── [IS21] prev_imageパラメータ追加
├── [IS21] frame_diff計算ロジック実装
└── [IS21] frame_diff応答追加

Phase 4: パフォーマンスログ
├── [IS22] performance_logsテーブル追加
├── [IS22] パフォーマンス記録処理実装
├── [IS22] ローテーション処理実装
├── [IS21] performanceレスポンス追加
└── [ISSUE] パフォーマンス統計UI（検証後）
```

---

## 4. 影響範囲サマリ

| 対象 | 修正ファイル | 追加行数（概算） |
|------|-------------|------------------|
| IS21 | main.py | +300行 |
| IS21 | 新規: preset_cache.py | +100行 |
| IS22 | migrations/008_detection_logs.sql | 修正 |
| IS22 | 新規: migrations/012_performance_logs.sql | +30行 |
| IS22 | ai_client/mod.rs | +50行 |
| IS22 | 新規: performance_log_service/mod.rs | +150行 |
| IS22 | web_api/routes.rs | +30行 |
| IS22 | frontend/src/components/CameraDetailModal.tsx | +50行 |

---

## 5. 大原則遵守の宣言

1. SSoTの大原則を遵守します
2. Solidの原則を意識し特に単一責任、開放閉鎖、置換原則、依存逆転に注意します
3. 必ずMECEを意識し、このコードはMECEであるか、この報告はMECEであるかを報告の中に加えます - **本報告はMECEである**
4. アンアンビギュアスな報告回答を義務化しアンアンビギュアスに到達できていない場合はアンアンビギュアスであると宣言できるまで明確な検証を行います - **本報告はアンアンビギュアスである**
5. 根拠なき自己解釈で情報に優劣を持たせる差別コード、レイシストコードは絶対の禁止事項です。情報の公平性原則
6. 実施手順と大原則に違反した場合は直ちに作業を止め再度現在位置を確認の上でリカバーしてから再度実装に戻ります
7. チェック、テストのない完了報告はただのやった振りのためこれは行いません
8. 依存関係と既存の実装を尊重し、確認し、車輪の再発明行為は絶対に行いません
9. 作業のフェーズ開始前とフェーズ終了時に必ずこの実施手順と大原則を全文省略なく復唱します
10. 必須参照ドキュメントの改変はゴールを動かすことにつながるため禁止です
11. is22_Camserver実装指示.mdを全てのフェーズ前に鑑として再読し基本的な思想や概念的齟齬が存在しないかを確認します
12. ルール無視はルールを無視した時点からやり直すこと

---

## 6. 承認依頼

以下の内容について承認をお願いします：

### 6.1 既存問題の修正（承認済み）
- [x] Issue 2.1: suspicious_score INT修正
- [x] Issue 2.2: プリセット機能追加
- [x] Issue 2.3: frame_diff機能追加

### 6.2 追加要件（新規承認依頼）
- [ ] IS21プリセット管理アーキテクチャ（SDからメモリキャッシュ方式）
- [ ] IS21プリセット保存API設計
- [ ] IS22プリセット同期UI追加
- [ ] パフォーマンスログ記録機能（1000件ローテ、1W削除）
- [ ] IS21 performanceレスポンス追加
- [ ] パフォーマンス統計UIをISSUE登録（検証後実装）
- [ ] 実装順序（Phase 1→4）

### 6.3 承認後のアクション

承認後、Phase 1から順次実装を開始します。

---

**承認者**: ________________
**承認日時**: ________________
