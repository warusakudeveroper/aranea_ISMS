-- Migration 009: Camera Preset Columns
-- camerasテーブルにプリセット関連カラムを追加

-- ========================================
-- cameras テーブルにプリセットカラム追加
-- ========================================
-- MariaDB: IF NOT EXISTS使用可能なバージョン (10.0.2+)
-- プロシージャで安全に追加

DELIMITER //

DROP PROCEDURE IF EXISTS add_camera_preset_columns//

CREATE PROCEDURE add_camera_preset_columns()
BEGIN
    -- preset_id
    IF NOT EXISTS (
        SELECT * FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
        AND TABLE_NAME = 'cameras'
        AND COLUMN_NAME = 'preset_id'
    ) THEN
        ALTER TABLE cameras ADD COLUMN preset_id VARCHAR(32) NOT NULL DEFAULT 'balanced'
            COMMENT 'プリセットID (person_priority, balanced, parking, etc.)' AFTER camera_context;
    END IF;

    -- preset_version
    IF NOT EXISTS (
        SELECT * FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
        AND TABLE_NAME = 'cameras'
        AND COLUMN_NAME = 'preset_version'
    ) THEN
        ALTER TABLE cameras ADD COLUMN preset_version VARCHAR(16) NULL
            COMMENT 'プリセットバージョン (例: 1.0.0)' AFTER preset_id;
    END IF;

    -- ai_enabled
    IF NOT EXISTS (
        SELECT * FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
        AND TABLE_NAME = 'cameras'
        AND COLUMN_NAME = 'ai_enabled'
    ) THEN
        ALTER TABLE cameras ADD COLUMN ai_enabled BOOLEAN NOT NULL DEFAULT FALSE
            COMMENT 'AI解析有効フラグ' AFTER preset_version;
    END IF;

    -- ai_interval_sec
    IF NOT EXISTS (
        SELECT * FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
        AND TABLE_NAME = 'cameras'
        AND COLUMN_NAME = 'ai_interval_sec'
    ) THEN
        ALTER TABLE cameras ADD COLUMN ai_interval_sec INT NOT NULL DEFAULT 15
            COMMENT 'AI解析間隔（秒）' AFTER ai_enabled;
    END IF;
END//

DELIMITER ;

CALL add_camera_preset_columns();
DROP PROCEDURE IF EXISTS add_camera_preset_columns;

-- インデックス追加（存在しない場合のみ）
-- MariaDB 10.1.4+ supports CREATE INDEX IF NOT EXISTS
CREATE INDEX IF NOT EXISTS idx_cameras_preset ON cameras(preset_id);
CREATE INDEX IF NOT EXISTS idx_cameras_ai_enabled ON cameras(ai_enabled);

-- ========================================
-- NOTE: schema_versionsテーブルはスキーマ定義用のため
-- マイグレーションバージョン記録は別途管理
-- ========================================
