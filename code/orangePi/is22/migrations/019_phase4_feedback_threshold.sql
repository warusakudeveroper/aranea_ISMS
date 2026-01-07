-- Migration 019: AIEventLog Phase 4 - Misdetection Feedback & Threshold Tables
-- 誤検知報告と閾値管理のためのテーブル追加
-- Issue #104 (T4-7, T4-11, T4-12)

-- ========================================
-- T4-7: misdetection_feedbacks (誤検知報告)
-- ========================================
CREATE TABLE IF NOT EXISTS misdetection_feedbacks (
    feedback_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,

    -- 対象ログ
    log_id BIGINT UNSIGNED NOT NULL COMMENT '対象検知ログID',
    camera_id VARCHAR(64) NOT NULL COMMENT 'カメラID',

    -- 報告内容
    reported_label VARCHAR(32) NOT NULL COMMENT '報告されたラベル（AI判定結果）',
    correct_label VARCHAR(32) NOT NULL COMMENT '正しいラベル（ユーザー指定）',
    confidence DECIMAL(5,4) NULL COMMENT '元の信頼度（参考用）',
    comment TEXT NULL COMMENT 'ユーザーコメント',

    -- メタデータ
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    -- インデックス
    INDEX idx_feedback_camera (camera_id, created_at),
    INDEX idx_feedback_log (log_id),
    INDEX idx_feedback_labels (reported_label, correct_label),
    FOREIGN KEY (log_id) REFERENCES detection_logs(log_id) ON DELETE CASCADE,
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- T4-11: camera_thresholds (カメラ別閾値)
-- ========================================
CREATE TABLE IF NOT EXISTS camera_thresholds (
    camera_id VARCHAR(64) PRIMARY KEY,

    -- 閾値設定
    conf_threshold DECIMAL(5,4) NOT NULL DEFAULT 0.5 COMMENT '信頼度閾値（0.2-0.8）',
    min_threshold DECIMAL(5,4) NOT NULL DEFAULT 0.2 COMMENT '最小閾値',
    max_threshold DECIMAL(5,4) NOT NULL DEFAULT 0.8 COMMENT '最大閾値',

    -- 自動調整
    auto_adjust_enabled BOOLEAN NOT NULL DEFAULT FALSE COMMENT '自動調整有効',

    -- メタデータ
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- T4-12: threshold_change_history (閾値変更履歴)
-- ========================================
CREATE TABLE IF NOT EXISTS threshold_change_history (
    history_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,

    camera_id VARCHAR(64) NOT NULL COMMENT 'カメラID',
    old_threshold DECIMAL(5,4) NULL COMMENT '変更前閾値',
    new_threshold DECIMAL(5,4) NOT NULL COMMENT '変更後閾値',
    change_reason ENUM('manual', 'auto_adjust', 'feedback_based') NOT NULL DEFAULT 'manual' COMMENT '変更理由',

    -- メタデータ
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    -- インデックス
    INDEX idx_history_camera (camera_id, created_at),
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
