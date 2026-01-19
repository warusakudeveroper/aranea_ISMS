# IS22 - RTSPカメラ総合管理サーバー仕様書

> **Version**: 1.0.0
> **Last Updated**: 2026-01-20
> **Status**: Production
> **ProductType**: 022
> **DeviceType**: aranea_ar-is22

---

## 1. Overview

### 1.1 目的

IS22は、施設内のIPカメラ（RTSP/ONVIF対応）を統合管理し、AI推論サーバー（IS21）と連携して映像解析・検知を行うLinuxベースのエッジサーバーです。

### 1.2 主要機能

| 機能カテゴリ | 説明 |
|-------------|------|
| カメラ管理 | RTSP/ONVIFカメラの自動検出・登録・ストリーム管理 |
| AI連携 | IS21推論サーバーとの連携、検知ログ管理 |
| Paraclate連携 | mobes2.0クラウドへのSummary/Event送信 |
| WebUI | ブラウザベースの管理・モニタリングUI |
| PTZ制御 | ONVIF PTZ対応カメラの遠隔操作 |

### 1.3 システム要件

| 項目 | 仕様 |
|------|------|
| OS | Ubuntu 22.04 / Armbian 25.x |
| CPU | ARM64 / x86_64 |
| RAM | 4GB以上推奨 |
| Storage | 32GB以上 |
| Network | 固定IP推奨、RTSP/ONVIFポートアクセス可能 |

### 1.4 デプロイ情報

```
バイナリパス: /opt/is22/target/release/camserver
設定ファイル: /opt/is22/config/
データベース: MySQL/MariaDB (camserver)
サービス名: is22.service
ポート: 3000 (WebAPI/WebUI)
```

---

## 2. Architecture

### 2.1 システム構成図

```
┌─────────────────────────────────────────────────────────────────┐
│                         IS22 Server                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │  WebAPI     │  │  WebUI      │  │  RealtimeHub (WebSocket)│  │
│  │  (Axum)     │  │  (React+TS) │  │                         │  │
│  └──────┬──────┘  └──────┬──────┘  └────────────┬────────────┘  │
│         │                │                       │               │
│  ┌──────┴────────────────┴───────────────────────┴────────────┐ │
│  │                      AppState                               │ │
│  │  ┌───────────────┐  ┌───────────────┐  ┌────────────────┐  │ │
│  │  │ ConfigStore   │  │ CameraRegistry│  │ ParaclateClient│  │ │
│  │  │ (SSoT)        │  │               │  │                │  │ │
│  │  └───────────────┘  └───────────────┘  └────────────────┘  │ │
│  │  ┌───────────────┐  ┌───────────────┐  ┌────────────────┐  │ │
│  │  │ AraneaRegister│  │ IpcamScan     │  │ SummaryService │  │ │
│  │  │               │  │               │  │                │  │ │
│  │  └───────────────┘  └───────────────┘  └────────────────┘  │ │
│  │  ┌───────────────┐  ┌───────────────┐  ┌────────────────┐  │ │
│  │  │ PtzController │  │ AccessAbsorber│  │ PollingOrch.   │  │ │
│  │  │               │  │               │  │                │  │ │
│  │  └───────────────┘  └───────────────┘  └────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                       ┌──────┴──────┐                           │
│                       │   MySQL DB  │                           │
│                       └─────────────┘                           │
└─────────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────┐      ┌─────────────┐      ┌─────────────────┐
│  IP Cameras │      │  IS21       │      │  mobes2.0       │
│  (RTSP/ONVIF)│      │  (AI推論)   │      │  (Paraclate APP)│
└─────────────┘      └─────────────┘      └─────────────────┘
```

### 2.2 モジュール一覧

| モジュール | 責務 | 関連Phase |
|-----------|------|-----------|
| `aranea_register` | araneaDeviceGate登録、LacisID/CIC取得 | Phase 1 |
| `camera_registry` | カメラのaraneaDeviceGate登録管理 | Phase 2 |
| `ipcam_scan` | ネットワークスキャン、カメラ検出 | Phase 2 |
| `config_store` | 設定SSoT、DBアクセス | Core |
| `paraclate_client` | mobes2.0連携、キュー管理 | Phase 4 |
| `summary_service` | Summary/GrandSummary生成 | Phase 5 |
| `polling_orchestrator` | カメラポーリング統括 | Core |
| `ptz_controller` | ONVIF PTZ制御 | Extension |
| `access_absorber` | ストリーム同時接続制限 | Extension |
| `realtime_hub` | WebSocket通知 | Core |
| `web_api` | REST API | Core |

---

## 3. LacisID形式

### 3.1 IS22デバイス自身

```
フォーマット: [Prefix][ProductType][MAC][ProductCode]
            [3]     [022]        [12桁] [0001]
            = 20桁

例: 30221A2B3C4D5E6F0001

Prefix:      3 (araneaデバイス固定)
ProductType: 022 (IS22固有)
MAC:         プライマリNICのMACアドレス（コロンなし、大文字）
ProductCode: 0001 (固定)
```

### 3.2 管理下カメラ（is801 ParaclateCamera）

```
フォーマット: [Prefix][ProductType][MAC][ProductCode]
            [3]     [801]        [12桁] [0000-0005]
            = 20桁

例: 38011A2B3C4D5E6F0001

ProductCode定義:
  0000: Generic（デフォルト）
  0001: TP-Link Tapo
  0002: Hikvision
  0003: Dahua
  0004: Reolink
  0005: Axis
```

---

## 4. 認証・認可

### 4.1 LacisOath認証

IS22からmobes2.0への全通信はLacisOath認証を使用します。

```
Authorization: LacisOath <base64-encoded-json>

JSON構造:
{
  "lacisId": "30221A2B3C4D5E6F0001",  // IS22のLacisID
  "tid": "T2025120621041161827",       // テナントID
  "cic": "204965",                     // Client Identification Code
  "timestamp": "2026-01-20T12:00:00Z"  // ISO8601
}
```

### 4.2 認証情報の取得フロー

```
1. テナントプライマリユーザーが登録を実行
   - lacis_id (テナントプライマリのLacisID)
   - user_id
   - cic (テナントプライマリのCIC)

2. IS22がaraneaDeviceGateにPOST
   - Authorization: LacisOath <tenant-primary-auth>
   - Body: { userObject, deviceMeta }

3. 成功時、IS22固有のCICを取得
   - config_storeに永続化
   - 以降の通信でこのCICを使用
```

### 4.3 Config Store キー定義

```rust
pub const LACIS_ID: &str = "aranea.lacis_id";
pub const TID: &str = "aranea.tid";
pub const FID: &str = "aranea.fid";
pub const CIC: &str = "aranea.cic";
pub const REGISTERED: &str = "aranea.registered";
pub const STATE_ENDPOINT: &str = "aranea.state_endpoint";
pub const MQTT_ENDPOINT: &str = "aranea.mqtt_endpoint";
```

---

## 5. カメラ管理

### 5.1 カメラカテゴリ分類

| Category | 説明 | Detail |
|----------|------|--------|
| A | 登録済みカメラ | IP一致で識別 |
| B | 登録可能カメラ | RTSP/ONVIF認証成功 |
| C | 認証待ちカメラ | RTSP/ONVIF応答あり、認証必要 |
| D | カメラ可能性あり | OUI一致等 |
| E | 非カメラ | 応答なし |
| F | 通信断・迷子 | MAC一致だがIP変更 |

### 5.2 カメラファミリー

| Family | ONVIF Port | RTSP Main | RTSP Sub |
|--------|------------|-----------|----------|
| Tapo | 2020 | /stream1 | /stream2 |
| Vigi | 2020 | /stream1 | /stream2 |
| Hikvision | 80 | /Streaming/Channels/101 | /102 |
| Dahua | 80 | /cam/realmonitor?channel=1&subtype=0 | &subtype=1 |
| Axis | 80 | /axis-media/media.amp | - |
| Nest | - | /live | - |

### 5.3 登録フロー

```
1. ネットワークスキャン実行 (/api/scan)
2. 検出デバイス一覧取得 (/api/scan/results)
3. Category B/Cデバイスを選択
4. 承認リクエスト (/api/scan/approve-batch)
   - Category B: そのまま登録
   - Category C: force_register=true で pending_auth状態で登録
5. カメラ登録完了 → araneaDeviceGate登録
6. ポーリング開始
```

---

## 6. Paraclate連携

### 6.1 エンドポイント

| API | URL | 用途 |
|-----|-----|------|
| Connect | paraclateConnect | 接続・設定同期 |
| GetConfig | paraclateGetConfig | 設定取得 |
| IngestSummary | paraclateIngestSummary | Summary送信 |
| IngestEvent | paraclateIngestEvent | Event送信 |
| AIChat | paraclateAIChat | AIチャット |
| CameraMetadata | paraclateCameraMetadata | カメラメタ同期 |

### 6.2 送信キュー管理

```rust
pub enum PayloadType {
    Summary,        // 定時サマリー
    GrandSummary,   // 日次グランドサマリー
    Event,          // 検知イベント
    Emergency,      // 緊急イベント（即時送信）
}

pub enum QueueStatus {
    Pending,   // 送信待ち
    Sending,   // 送信中
    Sent,      // 送信完了
    Failed,    // 送信失敗（リトライ対象）
    Skipped,   // スキップ（データ不備等）
}
```

### 6.3 リトライ戦略

```
指数バックオフ:
  retry_delay = min(30s × 2^retry_count, 3600s)

  0回目: 30秒
  1回目: 60秒
  2回目: 120秒
  3回目: 240秒
  4回目: 480秒
  5回目: 960秒
  ...
  最大: 3600秒 (1時間)

最大リトライ回数: 5回（デフォルト）
```

---

## 7. WebAPI エンドポイント

### 7.1 登録・管理系

```
POST   /api/register              - IS22デバイス登録
GET    /api/register/status       - 登録状態確認
POST   /api/register/clear        - 登録クリア

POST   /api/cameras               - カメラ作成
GET    /api/cameras               - カメラ一覧
GET    /api/cameras/:id           - カメラ詳細
PUT    /api/cameras/:id           - カメラ更新
DELETE /api/cameras/:id           - カメラ削除
```

### 7.2 スキャン・検出系

```
POST   /api/scan                  - ネットワークスキャン開始
GET    /api/scan/:job_id          - スキャンジョブ確認
GET    /api/scan/results          - スキャン結果取得
POST   /api/scan/approve-batch    - バッチ承認
```

### 7.3 Paraclate連携系

```
POST   /api/paraclate/connect     - 接続
GET    /api/paraclate/status      - ステータス
POST   /api/paraclate/disconnect  - 切断
GET    /api/paraclate/config      - 設定取得
PUT    /api/paraclate/config      - 設定更新
```

### 7.4 PTZ制御

```
POST   /api/cameras/:id/ptz/move  - 方向指定移動
POST   /api/cameras/:id/ptz/stop  - 停止
POST   /api/cameras/:id/ptz/home  - ホームポジション
GET    /api/cameras/:id/ptz/status - 状態取得
```

---

## 8. データベース構造

### 8.1 主要テーブル

| テーブル | 用途 | Migration |
|---------|------|-----------|
| cameras | カメラマスタ | 001 |
| subnets | サブネット定義 | 001 |
| detection_logs | 検知ログ | 008 |
| aranea_registration | IS22登録情報 | 020 |
| camera_registry | カメラ登録状態 | 021 |
| summary_data | Summaryデータ | 022 |
| paraclate_config | Paraclate設定 | 023 |
| paraclate_send_queue | 送信キュー | 023 |
| access_absorber_state | アクセス制限状態 | 031 |

### 8.2 camerasテーブル（抜粋）

```sql
CREATE TABLE cameras (
    camera_id VARCHAR(64) PRIMARY KEY,
    name VARCHAR(128) NOT NULL,
    ip_address VARCHAR(45),
    mac_address VARCHAR(17),
    lacis_id VARCHAR(20),
    cic VARCHAR(16),

    -- ストリーム
    rtsp_main VARCHAR(512),
    rtsp_sub VARCHAR(512),
    rtsp_username VARCHAR(64),
    rtsp_password VARCHAR(128),

    -- デバイス情報
    family VARCHAR(32),
    manufacturer VARCHAR(128),
    model VARCHAR(128),

    -- ONVIF
    onvif_endpoint VARCHAR(256),
    ptz_supported BOOLEAN DEFAULT FALSE,

    -- 所属
    fid VARCHAR(32),
    tid VARCHAR(32),

    -- ポリシー
    enabled BOOLEAN DEFAULT TRUE,
    polling_enabled BOOLEAN DEFAULT TRUE,
    polling_interval_sec INT DEFAULT 30,

    created_at DATETIME(3),
    updated_at DATETIME(3)
);
```

---

## 9. 実装ガイド

### 9.1 Rust実装パターン

```rust
// LacisOath認証ヘッダ生成
pub fn to_headers(&self) -> Vec<(String, String)> {
    use base64::Engine;

    let auth_payload = serde_json::json!({
        "lacisId": self.lacis_id,
        "tid": self.tid,
        "cic": self.cic,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let json_str = serde_json::to_string(&auth_payload).unwrap();
    let encoded = base64::engine::general_purpose::STANDARD
        .encode(json_str.as_bytes());

    vec![
        ("Authorization".to_string(), format!("LacisOath {}", encoded)),
    ]
}
```

### 9.2 SSoT原則

```
config_store (メモリキャッシュ) = 即時参照用（高速）
DB (永続化) = 履歴・監査・復旧用

起動時:
  1. DBから設定読み込み
  2. config_storeに展開

更新時:
  1. config_store更新（即時反映）
  2. DB更新（永続化）
```

---

## 10. 関連ドキュメント

- [AUTH_SPEC.md](./AUTH_SPEC.md) - LacisOath認証仕様
- [LINUX_COMMON_MODULES.md](./LINUX_COMMON_MODULES.md) - Linux共通モジュール
- [DD01_AraneaRegister.md](../../code/orangePi/is21-22/designs/headdesign/Paraclate/DetailedDesign/DD01_AraneaRegister.md)
- [DD03_ParaclateClient.md](../../code/orangePi/is21-22/designs/headdesign/Paraclate/DetailedDesign/DD03_ParaclateClient.md)

---

## 11. Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-20 | 初版作成 - IS22仕様を体系的に文書化 |
