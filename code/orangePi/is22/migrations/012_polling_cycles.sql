-- Migration 012: Polling Cycles Table
-- 巡回サイクルのログを保存するテーブル
-- ポーリングIDで検知ログと紐付け可能

-- ========================================
-- Polling Cycles (巡回サイクルログ)
-- ========================================
CREATE TABLE IF NOT EXISTS polling_cycles (
    cycle_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,

    -- ========================================
    -- ポーリングID (一意識別子)
    -- フォーマット: {subnet_octet3}-{YYMMDD}-{HHmmss}-{rand4}
    -- 例: 125-250103-143052-7a3f
    -- ========================================
    polling_id VARCHAR(32) NOT NULL UNIQUE COMMENT 'ポーリングID (125-250103-143052-7a3f)',

    -- ========================================
    -- サブネット情報
    -- ========================================
    subnet VARCHAR(24) NOT NULL COMMENT 'サブネット (例: 192.168.125)',
    subnet_octet3 INT NOT NULL COMMENT 'サブネット第3オクテット (例: 125)',

    -- ========================================
    -- タイムスタンプ
    -- ========================================
    started_at DATETIME(3) NOT NULL COMMENT '巡回開始時刻',
    completed_at DATETIME(3) NULL COMMENT '巡回完了時刻',

    -- ========================================
    -- サイクル統計
    -- ========================================
    cycle_number BIGINT UNSIGNED NOT NULL COMMENT 'サイクル番号 (サブネット内)',
    camera_count INT NOT NULL COMMENT '対象カメラ台数',
    success_count INT NOT NULL DEFAULT 0 COMMENT '成功件数',
    failed_count INT NOT NULL DEFAULT 0 COMMENT '失敗件数',
    timeout_count INT NOT NULL DEFAULT 0 COMMENT 'タイムアウト件数',

    -- ========================================
    -- パフォーマンス
    -- ========================================
    duration_ms INT NULL COMMENT '巡回所要時間 (ms)',
    avg_processing_ms INT NULL COMMENT '平均処理時間 (ms)',

    -- ========================================
    -- ステータス
    -- ========================================
    status ENUM('running', 'completed', 'interrupted') NOT NULL DEFAULT 'running',

    -- ========================================
    -- メタデータ
    -- ========================================
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    -- ========================================
    -- インデックス
    -- ========================================
    INDEX idx_cycles_polling_id (polling_id),
    INDEX idx_cycles_subnet (subnet, started_at),
    INDEX idx_cycles_started (started_at),
    INDEX idx_cycles_status (status, started_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- detection_logs にポーリングID追加
-- ========================================
ALTER TABLE detection_logs
    ADD COLUMN polling_cycle_id VARCHAR(32) NULL COMMENT 'ポーリングサイクルID' AFTER processing_ms,
    ADD INDEX idx_logs_polling_cycle (polling_cycle_id);
