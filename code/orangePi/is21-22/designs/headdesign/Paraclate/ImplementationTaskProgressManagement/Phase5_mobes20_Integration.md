# Phase 5: mobes2.0回答に基づく統合実装

作成日: 2026-01-11
基準: SPEC_CONFIRMATION_RESPONSE_2026-01-11.md

---

## 1. 概要

mobes2.0チームからの仕様確認回答に基づき、IS22側で実装すべき項目を定義する。

### 回答サマリ

| カテゴリ | 質問No | 回答要点 | IS22側対応 |
|----------|--------|----------|-----------|
| IS21アクティベート | IS21-1〜4 | araneaDeviceGate API使用 | 既存AraneaRegister流用 |
| LacisFiles | LF-1〜4 | Event APIにBase64 snapshot送信 | EventPayload拡張 |
| カメラ不調 | CM-1〜3 | IngestEvent APIにmalfunction_type送信 | EventPayload拡張 |
| BQ同期 | BQ-1〜4 | mobes2.0側で自動処理 | **IS22側対応不要** |

---

## 2. タスク一覧

### T5-1: IS22→IS21アクティベート（araneaDeviceGate経由）

**目的**: IS22がis21推論サーバーをアクティベートする

**mobes2.0回答**:
- araneaDeviceGate APIを使用
- LacisOath認証
- is21のdeviceTypeは既に定義済み

**実装方針**:
```rust
// src/is21_activation/mod.rs (新規)
pub struct Is21ActivationService {
    gate_url: String,
    http_client: Client,
}

impl Is21ActivationService {
    /// is21をアクティベート
    /// AraneaRegister::register_device()と同様のロジック
    pub async fn activate_is21(
        &self,
        is21_endpoint: &str,  // http://192.168.3.240:9000
        auth: &LacisOath,
    ) -> Result<ActivationResult, Error>
}
```

**ファイル**:
- `src/is21_activation/mod.rs` (新規)
- `src/is21_activation/types.rs` (新規)
- `src/web_api/is21_routes.rs` (新規)

---

### T5-2: LacisFiles対応（snapshot Base64送信）

**目的**: 検出画像をLacisFilesに保存

**mobes2.0回答**:
- IngestEvent APIにsnapshot（Base64）を送信するのみ
- mobes2.0側でLacisFilesへ自動保存
- レスポンスでstoragePath返却

**実装方針**:
```rust
// src/paraclate_client/types.rs 修正

/// Event送信ペイロード（拡張）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPayload {
    // ... 既存フィールド ...

    /// スナップショット画像（Base64エンコード）
    /// multipart/form-dataではなくJSON bodyに含める
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_base64: Option<String>,

    /// スナップショットのMIMEタイプ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_mime_type: Option<String>,
}
```

**変更点**:
- `send_event_with_snapshot()`: multipart → JSON Base64
- EventPayload: snapshot_base64フィールド追加

---

### T5-3: カメラ不調報告（IngestEvent拡張）

**目的**: カメラの不調状態をParaclateに報告

**mobes2.0回答**:
- IngestEvent APIにmalfunction_type含めて送信
- 不調種別: offline, stream_error, high_latency, low_fps

**実装方針**:
```rust
// src/paraclate_client/types.rs 修正

/// カメラ不調種別
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraMalfunctionType {
    Offline,
    StreamError,
    HighLatency,
    LowFps,
    NoFrames,
}

/// Event送信ペイロード（拡張）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPayload {
    // ... 既存フィールド ...

    /// カメラ不調種別（不調報告時のみ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub malfunction_type: Option<CameraMalfunctionType>,

    /// 不調詳細情報
    #[serde(skip_serializing_if = "Option::is_none")]
    pub malfunction_details: Option<serde_json::Value>,
}

// src/camera_malfunction_reporter.rs (新規)
pub struct CameraMalfunctionReporter {
    paraclate_client: Arc<ParaclateClient>,
}

impl CameraMalfunctionReporter {
    /// カメラ不調をParaclateに報告
    pub async fn report_malfunction(
        &self,
        camera_lacis_id: &str,
        malfunction_type: CameraMalfunctionType,
        details: Option<serde_json::Value>,
    ) -> Result<(), Error>
}
```

**統合点**:
- `CameraStatusTracker`: 不調検出時にreportを呼び出し
- polling_orchestrator: ステータス変化時にトリガー

---

## 3. 修正対象ファイル一覧

| ファイル | 変更種別 | 内容 |
|----------|----------|------|
| `src/paraclate_client/types.rs` | 修正 | EventPayload拡張 |
| `src/paraclate_client/client.rs` | 修正 | Base64送信対応 |
| `src/is21_activation/mod.rs` | 新規 | IS21アクティベートサービス |
| `src/is21_activation/types.rs` | 新規 | アクティベート用型定義 |
| `src/camera_malfunction_reporter.rs` | 新規 | 不調報告サービス |
| `src/web_api/is21_routes.rs` | 新規 | IS21管理API |
| `src/lib.rs` | 修正 | モジュール登録 |
| `src/main.rs` | 修正 | サービス初期化 |

---

## 4. 実装順序

```
T5-2 (LacisFiles Base64)
    ↓
T5-3 (カメラ不調報告)
    ↓
T5-1 (IS21アクティベート)
```

**理由**:
1. T5-2: 既存EventPayloadの拡張で完結、影響範囲小
2. T5-3: EventPayload拡張の延長、T5-2と同時実装可能
3. T5-1: 新規モジュール作成、他への依存なし

---

## 5. テスト計画

### E2Eテスト

| テスト項目 | 期待結果 |
|------------|----------|
| snapshot付きEvent送信 | storagePathがレスポンスに含まれる |
| カメラofflineイベント送信 | malfunction_type=offline受理 |
| IS21アクティベート | CICがレスポンスに含まれる |

### 確認環境

- IS22: 192.168.125.246:3000
- IS21: 192.168.3.240:9000
- Paraclate Connect: paraclateconnect-vm44u3kpua-an.a.run.app
- Paraclate IngestEvent: paraclateingestevent-vm44u3kpua-an.a.run.app

---

## 6. 完了条件

- [ ] T5-1: IS21アクティベートAPIが動作し、CICが取得できる
- [ ] T5-2: snapshot付きEventでstoragePath返却される
- [ ] T5-3: malfunction_type付きEventが正常受理される
- [ ] ビルド成功（cargo build --release）
- [ ] Chrome実機確認

---

## 7. BQ同期について

**mobes2.0回答**: IS22側対応不要（mobes2.0側で自動処理）

現在IS22に実装されている `bq_sync_service` は不要となるが、
ローカルDBへの検出ログ保存・履歴参照機能は引き続き有効。

BQへの同期はParaclateへのEvent/Summary送信で自動的に行われる。
