# IS22 プリセット詳細設定スライダー & カメライベント設計書

## 文書情報
- 作成日: 2026-01-04
- バージョン: 1.0.0
- ステータス: 設計確定

## 1. 概要

### 1.1 背景
IS21レスポンス分析の結果、以下の問題点が特定された：

1. **プリセット詳細設定UIの欠如**: `CameraContext`に定義された閾値調整パラメータ（conf_override, nms_threshold, par_threshold）がフロントエンドUIに露出していない
2. **カメラロスト/復帰イベントの未記録**: カメラ接続障害がAI Event Logに記録されず、ユーザーが障害履歴を確認できない

### 1.2 目的
- カメラごとの検知感度をスライダーUIで調整可能にする
- カメラのロスト/復帰をAI Event Logに記録し、ユーザー体験を向上させる

### 1.3 MECE確認
本設計は以下の観点でMECEである：
- **機能分類**: UI機能（スライダー）とシステムイベント機能（カメラ状態）が排他的
- **データフロー**: フロントエンド→API→DB→IS21 が一貫
- **イベント分類**: lost/recovered/normal が相互排他的かつ網羅的

---

## 2. 機能A: プリセット詳細設定スライダー

### 2.1 対象パラメータ

| パラメータ | 説明 | 範囲 | デフォルト | 単位 |
|-----------|------|------|-----------|------|
| `conf_override` | YOLO信頼度閾値 | 0.20 - 0.80 | null (プリセット依存) | - |
| `nms_threshold` | NMS (Non-Maximum Suppression) 閾値 | 0.30 - 0.60 | null (プリセット依存) | - |
| `par_threshold` | PAR属性分析閾値 | 0.30 - 0.80 | null (プリセット依存) | - |

### 2.2 データベース変更

#### 2.2.1 cameras テーブル拡張
```sql
-- Migration: 013_camera_threshold_overrides.sql
ALTER TABLE cameras
ADD COLUMN conf_override DECIMAL(3,2) DEFAULT NULL COMMENT 'YOLO confidence threshold override (0.20-0.80)',
ADD COLUMN nms_threshold DECIMAL(3,2) DEFAULT NULL COMMENT 'NMS threshold override (0.30-0.60)',
ADD COLUMN par_threshold DECIMAL(3,2) DEFAULT NULL COMMENT 'PAR threshold override (0.30-0.80)';
```

### 2.3 バックエンド変更

#### 2.3.1 config_store/types.rs
```rust
pub struct Camera {
    // ... 既存フィールド ...

    /// YOLO confidence threshold override (0.20-0.80)
    pub conf_override: Option<f32>,
    /// NMS threshold override (0.30-0.60)
    pub nms_threshold: Option<f32>,
    /// PAR threshold override (0.30-0.80)
    pub par_threshold: Option<f32>,
}
```

#### 2.3.2 config_store/repository.rs
- `find_all()`, `find_by_id()` のSELECT文に3カラム追加
- `update()` のUPDATE文に3カラム追加

#### 2.3.3 polling_orchestrator/mod.rs
`poll_camera_with_ai_log()` で `CameraContext` 構築時に閾値を注入：

```rust
// Apply threshold overrides from camera settings
if let Some(ref mut ctx) = request.camera_context {
    if camera.conf_override.is_some() {
        ctx.conf_override = camera.conf_override;
    }
    if camera.nms_threshold.is_some() {
        ctx.nms_threshold = camera.nms_threshold;
    }
    if camera.par_threshold.is_some() {
        ctx.par_threshold = camera.par_threshold;
    }
} else if camera.conf_override.is_some() || camera.nms_threshold.is_some() || camera.par_threshold.is_some() {
    // Create context if any override is set
    request.camera_context = Some(CameraContext {
        conf_override: camera.conf_override,
        nms_threshold: camera.nms_threshold,
        par_threshold: camera.par_threshold,
        ..Default::default()
    });
}
```

### 2.4 フロントエンド変更

#### 2.4.1 types/api.ts
```typescript
interface Camera {
  // ... 既存フィールド ...

  // Threshold overrides
  conf_override?: number | null
  nms_threshold?: number | null
  par_threshold?: number | null
}

interface UpdateCameraRequest {
  // ... 既存フィールド ...
  conf_override?: number | null
  nms_threshold?: number | null
  par_threshold?: number | null
}
```

#### 2.4.2 CameraDetailModal.tsx
「AI分析設定」セクション内に「詳細設定」サブセクションを追加：

```tsx
{/* 詳細設定 (閾値オーバーライド) */}
<CollapsibleSection title="詳細設定（上級者向け）" icon={<Settings2 className="h-4 w-4" />}>
  <p className="text-xs text-muted-foreground mb-3">
    プリセットのデフォルト値を上書きします。空欄の場合はプリセットの値が使用されます。
  </p>

  <FormField label="検出感度" editable>
    <div className="flex items-center gap-2">
      <input
        type="range"
        min="0.20"
        max="0.80"
        step="0.05"
        value={getValue("conf_override") ?? 0.50}
        onChange={(e) => updateField("conf_override", parseFloat(e.target.value))}
        className="flex-1"
      />
      <span className="w-12 text-sm">{getValue("conf_override")?.toFixed(2) ?? "-"}</span>
      <Button size="sm" variant="ghost" onClick={() => updateField("conf_override", null)}>
        リセット
      </Button>
    </div>
    <p className="text-xs text-muted-foreground">低い=敏感（誤検知増）、高い=鈍感（見逃し増）</p>
  </FormField>

  <FormField label="NMS閾値" editable>
    <div className="flex items-center gap-2">
      <input
        type="range"
        min="0.30"
        max="0.60"
        step="0.05"
        value={getValue("nms_threshold") ?? 0.45}
        onChange={(e) => updateField("nms_threshold", parseFloat(e.target.value))}
        className="flex-1"
      />
      <span className="w-12 text-sm">{getValue("nms_threshold")?.toFixed(2) ?? "-"}</span>
      <Button size="sm" variant="ghost" onClick={() => updateField("nms_threshold", null)}>
        リセット
      </Button>
    </div>
    <p className="text-xs text-muted-foreground">重複検出除去の閾値</p>
  </FormField>

  <FormField label="属性分析閾値" editable>
    <div className="flex items-center gap-2">
      <input
        type="range"
        min="0.30"
        max="0.80"
        step="0.05"
        value={getValue("par_threshold") ?? 0.50}
        onChange={(e) => updateField("par_threshold", parseFloat(e.target.value))}
        className="flex-1"
      />
      <span className="w-12 text-sm">{getValue("par_threshold")?.toFixed(2) ?? "-"}</span>
      <Button size="sm" variant="ghost" onClick={() => updateField("par_threshold", null)}>
        リセット
      </Button>
    </div>
    <p className="text-xs text-muted-foreground">人物属性（服の色等）の確信度閾値</p>
  </FormField>
</CollapsibleSection>
```

---

## 3. 機能B: カメラロスト/復帰イベント

### 3.1 イベント定義

| イベント | primary_event | severity | 条件 |
|---------|--------------|----------|------|
| カメラロスト | `camera_lost` | 4 | 前回OK→今回NG |
| カメラ復帰 | `camera_recovered` | 2 | 前回NG→今回OK |
| 継続ロスト | (記録しない) | - | 前回NG→今回NG |

### 3.2 状態管理

#### 3.2.1 CameraStatusTracker
```rust
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub enum CameraConnectionStatus {
    Unknown,  // 初回（状態未確定）
    Online,   // 正常接続
    Offline,  // 接続失敗
}

pub struct CameraStatusTracker {
    statuses: RwLock<HashMap<String, CameraConnectionStatus>>,
}

impl CameraStatusTracker {
    pub fn new() -> Self {
        Self {
            statuses: RwLock::new(HashMap::new()),
        }
    }

    /// 状態更新して遷移イベントを返す
    pub async fn update_status(
        &self,
        camera_id: &str,
        is_online: bool,
    ) -> Option<CameraStatusEvent> {
        let mut statuses = self.statuses.write().await;
        let prev = statuses.get(camera_id).cloned().unwrap_or(CameraConnectionStatus::Unknown);
        let new = if is_online { CameraConnectionStatus::Online } else { CameraConnectionStatus::Offline };

        statuses.insert(camera_id.to_string(), new.clone());

        match (prev, new) {
            (CameraConnectionStatus::Online, CameraConnectionStatus::Offline) => {
                Some(CameraStatusEvent::Lost)
            }
            (CameraConnectionStatus::Offline, CameraConnectionStatus::Online) => {
                Some(CameraStatusEvent::Recovered)
            }
            (CameraConnectionStatus::Unknown, CameraConnectionStatus::Offline) => {
                Some(CameraStatusEvent::Lost)  // 初回ロストも記録
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CameraStatusEvent {
    Lost,
    Recovered,
}
```

### 3.3 PollingOrchestrator への統合

```rust
// poll_camera_with_ai_log の結果に応じて状態更新
match result {
    Ok(processing_ms) => {
        if let Some(event) = camera_status_tracker.update_status(&camera.camera_id, true).await {
            if matches!(event, CameraStatusEvent::Recovered) {
                // detection_logs に camera_recovered イベントを記録
                save_camera_event(&detection_log, camera, "camera_recovered", 2).await;
            }
        }
        successful += 1;
    }
    Err(e) => {
        if let Some(event) = camera_status_tracker.update_status(&camera.camera_id, false).await {
            if matches!(event, CameraStatusEvent::Lost) {
                // detection_logs に camera_lost イベントを記録
                save_camera_event(&detection_log, camera, "camera_lost", 4).await;
            }
        }
        failed += 1;
    }
}
```

### 3.4 detection_logs への記録

カメライベントはAI分析結果ではないが、同じテーブルに記録してAI Event Logで一覧表示可能にする：

```rust
async fn save_camera_event(
    detection_log: &DetectionLogService,
    camera: &Camera,
    event_type: &str,  // "camera_lost" or "camera_recovered"
    severity: i32,
) -> Result<()> {
    // 簡易レコードを作成（image_dataは空、is21_logは最小限）
    let now = Utc::now();
    let is21_log = serde_json::json!({
        "analyzed": false,
        "detected": false,
        "primary_event": event_type,
        "severity": severity,
        "camera_id": camera.camera_id,
        "captured_at": now.to_rfc3339(),
        "schema_version": "2025-12-29.1",
        "event_type": "camera_status",
        "message": if event_type == "camera_lost" { "カメラ接続ロスト" } else { "カメラ接続復帰" },
    });

    // DetectionLogService に専用メソッドを追加
    detection_log.save_camera_event(camera, event_type, severity, is21_log).await
}
```

### 3.5 DetectionLogService への追加

```rust
/// カメラ状態イベントを記録（画像なし）
pub async fn save_camera_event(
    &self,
    camera: &Camera,
    event_type: &str,
    severity: i32,
    is21_log: serde_json::Value,
) -> Result<u64> {
    let now = Utc::now();
    let tid = camera.tid.as_deref().unwrap_or(&self.default_tid);
    let fid = camera.fid.as_deref().unwrap_or(&self.default_fid);

    let result = sqlx::query(
        r#"
        INSERT INTO detection_logs (
            tid, fid, camera_id, lacis_id,
            captured_at, analyzed_at,
            primary_event, severity, confidence, count_hint, unknown_flag,
            tags, preset_id, schema_version,
            is21_log, image_path_local, synced_to_bq
        ) VALUES (
            ?, ?, ?, ?,
            ?, ?,
            ?, ?, 1.0, 0, 0,
            '[]', 'system', '2025-12-29.1',
            ?, '/dev/null', FALSE
        )
        "#,
    )
    .bind(tid)
    .bind(fid)
    .bind(&camera.camera_id)
    .bind(&camera.lacis_id)
    .bind(now)
    .bind(now)
    .bind(event_type)
    .bind(severity)
    .bind(is21_log.to_string())
    .execute(&self.pool)
    .await?;

    Ok(result.last_insert_id())
}
```

---

## 4. テスト計画

### 4.1 スライダーUI テスト

| # | テスト項目 | 手順 | 期待結果 |
|---|-----------|------|----------|
| 1 | スライダー表示 | カメラ設定モーダルを開く | 詳細設定セクションにスライダー3つが表示される |
| 2 | 値変更 | 検出感度スライダーを0.30に変更 | 数値表示が0.30に更新される |
| 3 | 保存 | 保存ボタンをクリック | DBに値が保存される |
| 4 | リセット | リセットボタンをクリック | 値がnullになりプリセット依存に戻る |
| 5 | IS21反映 | ポーリングサイクルを待つ | hints_jsonに閾値が含まれる |

### 4.2 カメライベント テスト

| # | テスト項目 | 手順 | 期待結果 |
|---|-----------|------|----------|
| 1 | ロスト検知 | カメラを物理的に切断 | AI Event Logに`camera_lost`イベントが表示される |
| 2 | 復帰検知 | カメラを再接続 | AI Event Logに`camera_recovered`イベントが表示される |
| 3 | 継続ロスト | 切断状態を維持 | 追加イベントが記録されない（スパム防止確認） |
| 4 | UI表示 | AI Event Logを確認 | severity=4（ロスト）が赤、severity=2（復帰）が黄色で表示 |

---

## 5. 実装順序

1. DBマイグレーション作成・適用
2. バックエンド: config_store拡張（Camera構造体、リポジトリ）
3. バックエンド: CameraStatusTracker実装
4. バックエンド: DetectionLogService.save_camera_event()追加
5. バックエンド: polling_orchestrator統合
6. フロントエンド: types/api.ts拡張
7. フロントエンド: CameraDetailModal.tsxスライダー追加
8. ビルド・デプロイ
9. テスト実施

---

## 6. 整合性確認

### 6.1 is22_Camserver仕様案との整合性
- カメラ設定機能の拡張であり、基本設計の範囲内
- AI Event Logへの記録は設計方針に合致

### 6.2 既存実装との整合性
- `CameraContext`の`conf_override`等は既にai_client/mod.rsに定義済み
- DetectionLogServiceの拡張は既存メソッドと整合

### 6.3 SSoT確認
- 閾値パラメータの定義: ai_client/mod.rs（既存）
- DB格納: cameras テーブル（新規カラム）
- UI: CameraDetailModal.tsx（新規セクション）

---

## 7. アンアンビギュアス宣言

本設計書の内容は以下の点で曖昧さがない：
- パラメータ範囲が数値で明確（0.20-0.80等）
- イベント種別と条件が排他的に定義
- 実装箇所が具体的なファイル名・関数名で特定
- テスト項目が具体的な手順と期待結果を持つ

---

**設計完了日**: 2026-01-04
**レビュー待ち**: 実装開始前に本設計の妥当性を確認すること
