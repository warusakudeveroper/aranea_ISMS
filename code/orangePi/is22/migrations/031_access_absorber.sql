-- Migration: 031_access_absorber.sql
-- Description: Access Absorber - Camera brand-specific connection limit management
-- Date: 2026-01-17
-- Design Reference: camBrand/accessAbsorber/SPECIFICATION.md

-- ========================================
-- 1. ブランド別接続制限テーブル
-- ========================================
CREATE TABLE IF NOT EXISTS camera_access_limits (
    id INT AUTO_INCREMENT PRIMARY KEY,
    family VARCHAR(32) NOT NULL UNIQUE COMMENT 'Camera family identifier (tapo_ptz, tapo_fixed, vigi, nvt, etc.)',
    display_name VARCHAR(64) NOT NULL COMMENT 'Display name for UI',
    max_concurrent_streams TINYINT UNSIGNED NOT NULL DEFAULT 2 COMMENT 'Maximum concurrent RTSP streams',
    min_reconnect_interval_ms INT UNSIGNED NOT NULL DEFAULT 0 COMMENT 'Minimum interval between disconnection and reconnection',
    require_exclusive_lock BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'Whether exclusive lock is required',
    connection_timeout_ms INT UNSIGNED NOT NULL DEFAULT 10000 COMMENT 'Connection timeout in milliseconds',
    model_pattern VARCHAR(128) DEFAULT NULL COMMENT 'Regex pattern for model matching (e.g., C2[0-9]{2} for Tapo PTZ)',
    notes TEXT COMMENT 'Additional notes',
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'System builtin (cannot be deleted)',
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    INDEX idx_family (family),
    INDEX idx_builtin (is_builtin)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- 2. 初期データ投入（ストレステスト結果に基づく）
-- ========================================
INSERT INTO camera_access_limits
(family, display_name, max_concurrent_streams, min_reconnect_interval_ms, require_exclusive_lock, model_pattern, notes, is_builtin)
VALUES
-- Tapo PTZ系: 放熱・処理負荷問題により1セッション推奨
('tapo_ptz', 'TP-Link Tapo (PTZ)', 1, 0, TRUE, 'C2[0-9]{2}|C5[0-9]{2}', 'C200/C210/C220/C500等PTZ系。放熱問題あり。並列接続不可', TRUE),

-- Tapo 固定系: 2並列まで安定
('tapo_fixed', 'TP-Link Tapo (Fixed)', 2, 0, FALSE, 'C1[0-9]{2}', 'C100/C110/C120等固定系。2並列安定', TRUE),

-- VIGI: プロ向け、3並列まで安定
('vigi', 'TP-Link VIGI', 3, 0, FALSE, NULL, 'VIGI C3xx/C4xx等。3並列まで安定', TRUE),

-- NVT/JOOAN: 再接続に2秒間隔必須
('nvt', 'NVT / JOOAN', 2, 2000, FALSE, NULL, '2秒間隔必須。急速再接続5%成功率。Main+Sub同時可', TRUE),

-- Hikvision: 推定値
('hikvision', 'Hikvision', 4, 0, FALSE, NULL, '推定値。実機テスト推奨', TRUE),

-- Dahua: 推定値
('dahua', 'Dahua', 4, 0, FALSE, NULL, '推定値。実機テスト推奨', TRUE),

-- Axis: エンタープライズ向け、多並列可
('axis', 'Axis Communications', 8, 0, FALSE, NULL, '推定値。エンタープライズ品質', TRUE),

-- 不明: 安全側デフォルト
('unknown', '不明', 1, 1000, TRUE, NULL, '安全側デフォルト。1接続、1秒間隔', TRUE)

ON DUPLICATE KEY UPDATE
    display_name = VALUES(display_name),
    max_concurrent_streams = VALUES(max_concurrent_streams),
    min_reconnect_interval_ms = VALUES(min_reconnect_interval_ms),
    require_exclusive_lock = VALUES(require_exclusive_lock),
    model_pattern = VALUES(model_pattern),
    notes = VALUES(notes);

-- ========================================
-- 3. アクティブストリームセッションテーブル
-- ========================================
CREATE TABLE IF NOT EXISTS camera_stream_sessions (
    session_id VARCHAR(64) PRIMARY KEY COMMENT 'Unique session identifier',
    camera_id VARCHAR(64) NOT NULL COMMENT 'Camera identifier',
    stream_type ENUM('main', 'sub') NOT NULL DEFAULT 'main' COMMENT 'Stream type',
    purpose ENUM('click_modal', 'suggest_play', 'polling', 'snapshot', 'health_check') NOT NULL COMMENT 'Stream purpose for priority control',
    client_id VARCHAR(128) NOT NULL COMMENT 'Client identifier (browser session, polling job, etc.)',
    started_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) COMMENT 'Session start time',
    expires_at DATETIME(3) COMMENT 'Session expiration time',
    last_heartbeat_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) COMMENT 'Last heartbeat time for timeout detection',
    status ENUM('active', 'releasing', 'expired') NOT NULL DEFAULT 'active' COMMENT 'Session status',

    INDEX idx_camera_id (camera_id),
    INDEX idx_status (status),
    INDEX idx_expires_at (expires_at),
    INDEX idx_camera_status (camera_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- 4. 接続イベントログテーブル（メトリクス・デバッグ用）
-- ========================================
CREATE TABLE IF NOT EXISTS camera_connection_events (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL COMMENT 'Camera identifier',
    event_type ENUM(
        'connect_success',
        'connect_blocked_concurrent',
        'connect_blocked_interval',
        'connect_timeout',
        'connect_preempted',
        'disconnect_normal',
        'disconnect_preempted',
        'disconnect_timeout',
        'disconnect_error'
    ) NOT NULL COMMENT 'Event type',
    purpose ENUM('click_modal', 'suggest_play', 'polling', 'snapshot', 'health_check') COMMENT 'Stream purpose',
    client_id VARCHAR(128) COMMENT 'Client identifier',
    blocked_reason TEXT COMMENT 'Reason for blocking (if applicable)',
    wait_time_ms INT UNSIGNED COMMENT 'Wait time in milliseconds (for interval blocking)',
    preempted_by VARCHAR(128) COMMENT 'Client that caused preemption',
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX idx_camera_created (camera_id, created_at),
    INDEX idx_event_type (event_type),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- 5. 施設別オーバーライドテーブル
-- ========================================
CREATE TABLE IF NOT EXISTS facility_camera_limits (
    id INT AUTO_INCREMENT PRIMARY KEY,
    fid VARCHAR(4) NOT NULL COMMENT 'Facility ID',
    family VARCHAR(32) NOT NULL COMMENT 'Camera family',
    max_concurrent_streams TINYINT UNSIGNED COMMENT 'Override max concurrent streams (NULL = use default)',
    min_reconnect_interval_ms INT UNSIGNED COMMENT 'Override min reconnect interval (NULL = use default)',
    require_exclusive_lock BOOLEAN COMMENT 'Override exclusive lock requirement (NULL = use default)',
    notes TEXT COMMENT 'Reason for override',
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    UNIQUE KEY uk_fid_family (fid, family),
    INDEX idx_fid (fid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- 6. camerasテーブル拡張（個別オーバーライド）
-- ========================================
-- Check if column exists before adding
SET @dbname = DATABASE();
SET @tablename = 'cameras';
SET @columnname = 'access_limit_override';
SET @preparedStatement = (SELECT IF(
    (SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS
     WHERE TABLE_SCHEMA = @dbname AND TABLE_NAME = @tablename AND COLUMN_NAME = @columnname) > 0,
    'SELECT 1',
    CONCAT('ALTER TABLE ', @tablename, ' ADD COLUMN ', @columnname, ' JSON DEFAULT NULL COMMENT \'Per-camera access limit override: {"max_concurrent_streams": 2, "min_reconnect_interval_ms": 1000}\'')
));
PREPARE alterIfNotExists FROM @preparedStatement;
EXECUTE alterIfNotExists;
DEALLOCATE PREPARE alterIfNotExists;

-- ========================================
-- 7. カメラ最終切断時刻追跡カラム追加
-- ========================================
SET @columnname2 = 'last_disconnect_at';
SET @preparedStatement2 = (SELECT IF(
    (SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS
     WHERE TABLE_SCHEMA = @dbname AND TABLE_NAME = @tablename AND COLUMN_NAME = @columnname2) > 0,
    'SELECT 1',
    CONCAT('ALTER TABLE ', @tablename, ' ADD COLUMN ', @columnname2, ' DATETIME(3) DEFAULT NULL COMMENT \'Last RTSP disconnection time for interval checking\'')
));
PREPARE alterIfNotExists2 FROM @preparedStatement2;
EXECUTE alterIfNotExists2;
DEALLOCATE PREPARE alterIfNotExists2;

-- ========================================
-- 8. カメラファミリー分類カラム追加
-- ========================================
SET @columnname3 = 'access_family';
SET @preparedStatement3 = (SELECT IF(
    (SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS
     WHERE TABLE_SCHEMA = @dbname AND TABLE_NAME = @tablename AND COLUMN_NAME = @columnname3) > 0,
    'SELECT 1',
    CONCAT('ALTER TABLE ', @tablename, ' ADD COLUMN ', @columnname3, ' VARCHAR(32) DEFAULT NULL COMMENT \'Access absorber family classification (tapo_ptz, tapo_fixed, etc.)\'')
));
PREPARE alterIfNotExists3 FROM @preparedStatement3;
EXECUTE alterIfNotExists3;
DEALLOCATE PREPARE alterIfNotExists3;

-- ========================================
-- Verification
-- ========================================
-- SELECT * FROM camera_access_limits;
-- DESCRIBE cameras;
