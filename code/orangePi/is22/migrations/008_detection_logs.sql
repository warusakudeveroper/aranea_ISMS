-- Migration 008: AI Event Log - Detection Logs Table
-- is21からの検知結果を保存するテーブル
-- BQカラムとローカルDBカラムを完全一致させる

-- ========================================
-- Detection Logs (is21検知ログ)
-- ========================================
-- NOTE: 検知ありのログのみ保存（none/未検知は非保存）
CREATE TABLE IF NOT EXISTS detection_logs (
    log_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,

    -- ========================================
    -- 必須識別子 (BQ同期対象)
    -- ========================================
    tid VARCHAR(32) NOT NULL COMMENT 'テナントID',
    fid VARCHAR(32) NOT NULL COMMENT '施設ID',
    camera_id VARCHAR(64) NOT NULL COMMENT 'カメラID',
    lacis_id VARCHAR(32) NULL COMMENT 'カメラのlacisID',

    -- ========================================
    -- タイムスタンプ
    -- ========================================
    captured_at DATETIME(3) NOT NULL COMMENT '画像キャプチャ日時',
    analyzed_at DATETIME(3) NOT NULL COMMENT 'is21解析完了日時',

    -- ========================================
    -- is21解析結果
    -- ========================================
    primary_event VARCHAR(32) NOT NULL COMMENT 'human/animal/vehicle/unknown',
    severity INT NOT NULL DEFAULT 1 COMMENT '重要度 (0-3)',
    confidence DECIMAL(5,4) NOT NULL COMMENT '信頼度 (0.0-1.0)',
    count_hint INT NOT NULL DEFAULT 0 COMMENT '検出数ヒント',
    unknown_flag BOOLEAN NOT NULL DEFAULT FALSE COMMENT '未知フラグ',

    -- ========================================
    -- 詳細データ (JSON)
    -- ========================================
    tags JSON NOT NULL COMMENT 'タグ配列 ["outfit.dress","gender.female",...]',
    person_details JSON NULL COMMENT 'PAR詳細 [{index,top_color,bottom_color,...}]',
    bboxes JSON NULL COMMENT '検出ボックス [{x1,y1,x2,y2,label,conf,...}]',
    suspicious JSON NULL COMMENT '不審スコア {score,level,factors}',

    -- ========================================
    -- フレーム差分分析 (frame_diff)
    -- ========================================
    frame_diff JSON NULL COMMENT '差分分析結果 {enabled,person_changes,movement_vectors,loitering,scene_change,camera_status}',
    loitering_detected BOOLEAN NOT NULL DEFAULT FALSE COMMENT '滞在検知フラグ（frame_diff.loitering.detectedの展開）',

    -- ========================================
    -- プリセット情報 (BQ同期対象・トレーサビリティ用)
    -- ========================================
    preset_id VARCHAR(32) NOT NULL DEFAULT 'balanced' COMMENT '使用プリセットID',
    preset_version VARCHAR(16) NULL COMMENT 'プリセットバージョン (例: 1.0.0)',
    output_schema VARCHAR(32) NULL COMMENT '出力スキーマ名 (例: parking, person_detailed)',

    -- ========================================
    -- コンテキスト情報
    -- ========================================
    context_applied BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'camera_context適用有無',
    camera_context JSON NULL COMMENT '送信したcamera_context',

    -- ========================================
    -- 生データ (BQ同期対象)
    -- ========================================
    is21_log JSON NOT NULL COMMENT 'is21からの完全レスポンス',
    camera_response JSON NULL COMMENT 'カメラからのレスポンス（将来用）',

    -- ========================================
    -- 画像パス (BQ同期対象)
    -- ========================================
    image_path_local VARCHAR(512) NOT NULL COMMENT 'ローカルファイルパス',
    image_path_cloud VARCHAR(512) NULL COMMENT 'クラウドパス (GoogleDrive API対応時)',

    -- ========================================
    -- 処理情報
    -- ========================================
    processing_ms INT NULL COMMENT '処理時間 (ms)',
    schema_version VARCHAR(32) NOT NULL COMMENT 'is21スキーマバージョン',

    -- ========================================
    -- メタデータ
    -- ========================================
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    synced_to_bq BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'BQ同期済みフラグ',
    synced_at DATETIME(3) NULL COMMENT 'BQ同期日時',

    -- ========================================
    -- 仮想カラム（JSONインデックス用）
    -- ========================================
    suspicious_score DECIMAL(5,4) AS (JSON_VALUE(suspicious, '$.score')) STORED COMMENT '不審スコア（仮想カラム）',

    -- ========================================
    -- インデックス
    -- ========================================
    INDEX idx_logs_tid_fid (tid, fid),
    INDEX idx_logs_camera (camera_id, captured_at),
    INDEX idx_logs_captured (captured_at),
    INDEX idx_logs_primary_event (primary_event, captured_at),
    INDEX idx_logs_severity (severity, captured_at),
    INDEX idx_logs_suspicious_score (suspicious_score, captured_at),
    INDEX idx_logs_sync (synced_to_bq, created_at),
    INDEX idx_logs_preset (preset_id, captured_at),
    INDEX idx_logs_output_schema (output_schema, captured_at),
    INDEX idx_logs_loitering (loitering_detected, captured_at),
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- BQ Sync Queue (BQ同期キュー)
-- ========================================
CREATE TABLE IF NOT EXISTS bq_sync_queue (
    queue_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    table_name VARCHAR(64) NOT NULL COMMENT '対象テーブル名',
    record_id BIGINT UNSIGNED NOT NULL COMMENT '対象レコードID',
    operation ENUM('INSERT', 'UPDATE', 'DELETE') NOT NULL DEFAULT 'INSERT',
    payload JSON NOT NULL COMMENT 'BQ送信用ペイロード',
    retry_count INT NOT NULL DEFAULT 0,
    last_error TEXT NULL,
    status ENUM('pending', 'processing', 'success', 'failed') NOT NULL DEFAULT 'pending',
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    processed_at DATETIME(3) NULL,

    INDEX idx_queue_status (status, created_at),
    INDEX idx_queue_table (table_name, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- AI Chat History (AIチャット履歴)
-- ========================================
CREATE TABLE IF NOT EXISTS ai_chat_history (
    chat_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    session_id VARCHAR(64) NOT NULL COMMENT 'セッションID',
    tid VARCHAR(32) NOT NULL COMMENT 'テナントID',
    fid VARCHAR(32) NOT NULL COMMENT '施設ID',
    user_query TEXT NOT NULL COMMENT 'ユーザークエリ',
    ai_response TEXT NOT NULL COMMENT 'AI応答',
    referenced_logs JSON NULL COMMENT '参照した検知ログID配列',
    referenced_images JSON NULL COMMENT '参照した画像パス配列',
    query_to_bq TEXT NULL COMMENT 'BQに送信したクエリ',
    response_time_ms INT NULL COMMENT '応答時間',
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX idx_chat_session (session_id, created_at),
    INDEX idx_chat_tid_fid (tid, fid, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- AI Summary Cache (AIサマリーキャッシュ)
-- ========================================
CREATE TABLE IF NOT EXISTS ai_summary_cache (
    summary_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    tid VARCHAR(32) NOT NULL COMMENT 'テナントID',
    fid VARCHAR(32) NOT NULL COMMENT '施設ID',
    summary_type ENUM('hourly', 'daily', 'emergency') NOT NULL,
    period_start DATETIME(3) NOT NULL COMMENT '対象期間開始',
    period_end DATETIME(3) NOT NULL COMMENT '対象期間終了',
    summary_text TEXT NOT NULL COMMENT 'サマリーテキスト',
    detection_count INT NOT NULL DEFAULT 0,
    severity_max INT NOT NULL DEFAULT 0,
    camera_ids JSON NOT NULL COMMENT '対象カメラID配列',
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    expires_at DATETIME(3) NOT NULL COMMENT 'キャッシュ有効期限',

    UNIQUE KEY uq_summary_period (tid, fid, summary_type, period_start),
    INDEX idx_summary_expires (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
