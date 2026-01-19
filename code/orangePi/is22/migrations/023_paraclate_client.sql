-- Migration: 023_paraclate_client
-- Description: ParaclateClient用テーブル作成
-- Phase 4: ParaclateClient (Issue #117)
-- DD03_ParaclateClient.md準拠
--
-- ## 概要
-- Paraclate APPへの通信クライアント機能
-- - 設定同期（mobes2.0 → is22）
-- - Summary/Event送信キュー管理
-- - リトライ・オフライン対応

-- ============================================================
-- 1. paraclate_config テーブル作成
-- ============================================================
-- mobes2.0から同期される設定を保存

CREATE TABLE IF NOT EXISTS paraclate_config (
    config_id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,

    -- テナント・施設ID
    tid VARCHAR(32) NOT NULL COMMENT 'テナントID',
    fid VARCHAR(32) NOT NULL COMMENT '施設ID',

    -- 接続先エンドポイント
    endpoint VARCHAR(256) NOT NULL
    COMMENT 'Paraclate APP URL (e.g., https://api.mobes.app/paraclate)',

    -- 報告設定
    report_interval_minutes INT DEFAULT 60
    COMMENT 'Summary生成間隔（分）',
    grand_summary_times JSON DEFAULT '["09:00", "17:00", "21:00"]'
    COMMENT 'GrandSummary実行時刻リスト (Asia/Tokyo基準)',
    retention_days INT DEFAULT 60
    COMMENT 'ログ保持日数',

    -- AIアチューンメント設定
    attunement JSON DEFAULT '{}'
    COMMENT 'AIアチューンメント設定 (autoTuningEnabled, frequency, aggressiveness)',

    -- 同期状態管理
    sync_source_timestamp DATETIME(3)
    COMMENT 'mobes2.0側の設定更新タイムスタンプ（SSoT比較用）',
    last_sync_at DATETIME(3)
    COMMENT '最終同期日時（is22側）',

    -- 接続状態
    connection_status ENUM('connected', 'disconnected', 'error') DEFAULT 'disconnected'
    COMMENT '接続状態',
    last_error TEXT
    COMMENT '最終エラーメッセージ',

    -- タイムスタンプ
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    -- 制約
    UNIQUE KEY uk_tid_fid (tid, fid),
    INDEX idx_tid (tid),
    INDEX idx_connection_status (connection_status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Paraclate設定（mobes2.0同期） (DD03準拠)';

-- ============================================================
-- 2. paraclate_send_queue テーブル作成
-- ============================================================
-- Summary/Event送信キュー（指数バックオフリトライ対応）

CREATE TABLE IF NOT EXISTS paraclate_send_queue (
    queue_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,

    -- テナント・施設ID
    tid VARCHAR(32) NOT NULL COMMENT 'テナントID',
    fid VARCHAR(32) NOT NULL COMMENT '施設ID',

    -- ペイロード情報
    payload_type ENUM('summary', 'grand_summary', 'event', 'emergency') NOT NULL
    COMMENT 'ペイロード種別',
    payload JSON NOT NULL
    COMMENT '送信ペイロード（JSON形式）',

    -- 関連ID（Summary/Eventの場合）
    reference_id BIGINT UNSIGNED
    COMMENT '参照ID（summary_idなど）',

    -- 送信状態
    status ENUM('pending', 'sending', 'sent', 'failed', 'skipped') DEFAULT 'pending'
    COMMENT '送信状態',
    retry_count INT DEFAULT 0
    COMMENT 'リトライ回数',
    max_retries INT DEFAULT 5
    COMMENT '最大リトライ回数',
    next_retry_at DATETIME(3)
    COMMENT '次回リトライ日時（指数バックオフ）',

    -- エラー情報
    last_error TEXT
    COMMENT '最終エラーメッセージ',
    http_status_code INT
    COMMENT '最終HTTPステータスコード',

    -- タイムスタンプ
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    sent_at DATETIME(3)
    COMMENT '送信完了日時',

    -- 制約
    INDEX idx_status (status),
    INDEX idx_tid_fid (tid, fid),
    INDEX idx_created (created_at),
    INDEX idx_next_retry (next_retry_at, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Paraclate送信キュー (DD03準拠)';

-- ============================================================
-- 3. paraclate_connection_log テーブル作成
-- ============================================================
-- 接続履歴（トラブルシューティング用）

CREATE TABLE IF NOT EXISTS paraclate_connection_log (
    log_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,

    -- テナント・施設ID
    tid VARCHAR(32) NOT NULL COMMENT 'テナントID',
    fid VARCHAR(32) NOT NULL COMMENT '施設ID',

    -- イベント情報
    event_type ENUM('connect', 'disconnect', 'sync', 'error', 'retry') NOT NULL
    COMMENT 'イベント種別',
    event_detail TEXT
    COMMENT 'イベント詳細',

    -- エラー情報（error/retryの場合）
    error_code VARCHAR(64)
    COMMENT 'エラーコード',
    http_status_code INT
    COMMENT 'HTTPステータスコード',

    -- タイムスタンプ
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),

    -- 制約
    INDEX idx_tid_fid (tid, fid),
    INDEX idx_event_type (event_type),
    INDEX idx_created (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Paraclate接続ログ (DD03準拠)';
