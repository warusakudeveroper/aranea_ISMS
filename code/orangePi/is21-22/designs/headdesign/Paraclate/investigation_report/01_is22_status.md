# IS22 (Paraclate Server) 実装状態調査

作成日: 2026-01-10

## 1. 概要

IS22はRTSPカメラ総合管理サーバーであり、Paraclate設計においてはParaclate Serverとして位置付けられる。

- **IP**: 192.168.125.246
- **言語**: Rust (Axum + Tokio)
- **フロントエンド**: React + TypeScript + Vite

---

## 2. モジュール構成

### 2.1 コアモジュール（11個）

| # | モジュール | 責務 | 実装状況 |
|---|-----------|------|---------|
| 1 | config_store | SSoT カメラ・ポリシー管理 | ✅ |
| 2 | snapshot_service | カメラ画像キャプチャ | ✅ |
| 3 | ai_client | is21通信アダプタ | ✅ |
| 4 | polling_orchestrator | カメラポーリング制御 | ✅ |
| 5 | event_log_service | イベント記録 | ✅ |
| 6 | suggest_engine | 推奨カメラ決定 | ✅ |
| 7 | stream_gateway | go2rtc統合 | ✅ |
| 8 | admission_controller | 負荷制御 | ✅ |
| 9 | realtime_hub | WebSocket/SSE配信 | ✅ |
| 10 | web_api | REST API | ✅ |
| 11 | ipcam_scan | カメラ自動発見 | ✅ |

### 2.2 追加モジュール（17個）

| モジュール | 機能 |
|-----------|------|
| camera_brand | OUI/RTSPテンプレートSSoT |
| detection_log_service | is21検知ログDB永続化 |
| inference_stats_service | 推論統計・分析 |
| auto_attunement | 自動チューニング |
| overdetection_analyzer | 過剰検出判定 |
| sdm_integration | Google SDM (Nest対応) |
| prev_frame_cache | フレーム差分キャッシュ |
| preset_loader | プリセット管理 |
| rtsp_manager | RTSPアクセス制御 |

---

## 3. APIエンドポイント（117個）

### 3.1 カテゴリ別集計

| カテゴリ | 数 | 主な機能 |
|---------|---|---------|
| カメラ管理 | 10 | CRUD・認証テスト・リスキャン |
| 推奨カメラ | 3 | Suggest API |
| イベント・ログ | 9 | 検知ログ・画像取得 |
| システム設定 | 10 | IS21設定・タイムアウト・ストレージ |
| 推論統計 | 8 | 分布・傾向・プリセット効果 |
| 自動チューニング | 4 | 計算・適用・ステータス |
| カメラブランドSSoT | 15 | OUI・RTSPテンプレート |
| IpcamScan | 18 | スキャン・認可・登録 |
| SDM (Google Nest) | 18 | OAuth・デバイス同期 |

### 3.2 Paraclate関連で不足しているAPI

| API | 目的 | 状態 |
|-----|------|------|
| POST /api/summary/generate | Summary生成 | ❌ 未実装 |
| GET /api/summary/latest | 最新Summary取得 | ❌ 未実装 |
| GET /api/grand-summary/:date | GrandSummary取得 | ❌ 未実装 |
| POST /api/paraclate/sync | Paraclate APP同期 | ❌ 未実装 |
| POST /api/register/device | AraneaRegister | ❌ 未実装 |

---

## 4. データベーススキーマ（19マイグレーション）

### 4.1 Paraclate関連テーブル

```sql
-- migration 008: ai_summary_cache
CREATE TABLE ai_summary_cache (
    summary_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,           -- テナントID
    fid VARCHAR(32) NOT NULL,           -- 施設ID
    summary_type ENUM('hourly', 'daily', 'emergency') NOT NULL,
    period_start DATETIME(3) NOT NULL,
    period_end DATETIME(3) NOT NULL,
    summary_text TEXT NOT NULL,
    detection_count INT NOT NULL DEFAULT 0,
    severity_max INT NOT NULL DEFAULT 0,
    camera_ids JSON NOT NULL,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    expires_at DATETIME(3) NOT NULL
);

-- ai_chat_history
CREATE TABLE ai_chat_history (
    chat_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL,
    fid VARCHAR(32) NOT NULL,
    user_message TEXT NOT NULL,
    assistant_response TEXT NOT NULL,
    context_summary_ids JSON,
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3)
);
```

### 4.2 不足しているスキーマ

| テーブル | 用途 | 状態 |
|---------|------|------|
| aranea_registration | デバイス登録情報 | ❌ 未定義 |
| paraclate_config | Paraclate連携設定 | ❌ 未定義 |
| scheduled_reports | 定時報告スケジュール | ❌ 未定義 |

---

## 5. フロントエンド実装状態

### 5.1 AIアシスタント関連（今回実装済）

| 機能 | ファイル | 状態 |
|------|---------|------|
| AIタブ | SettingsModal.tsx | ✅ 実装済 |
| サジェスト頻度スライダー | SettingsModal.tsx | ✅ 実装済 |
| Paraclateプレースホルダー | SettingsModal.tsx | ✅ 実装済 |
| メッセージ削除機能 | EventLogPane.tsx | ✅ 実装済 |

### 5.2 未実装フロントエンド

| 機能 | 状態 |
|------|------|
| Summary表示ビュー | ❌ |
| GrandSummary閲覧 | ❌ |
| Paraclate接続設定入力 | ❌ |

---

## 6. MECE確認

- **網羅性**: 全28モジュール、117 API、19マイグレーションを調査
- **重複排除**: 各項目は単一責務で分類
- **未カバー領域**: AraneaRegister、Summary API、Paraclate連携が明確に未実装として識別
