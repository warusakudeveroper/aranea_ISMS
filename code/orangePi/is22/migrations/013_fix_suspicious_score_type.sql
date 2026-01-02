-- Migration 013: Fix suspicious_score column type
-- Issue: suspicious_score was DECIMAL(5,4) (max 9.9999) but IS21 returns integer 0-100
-- Fix: Change to INT to match IS21's calculate_suspicious_score output

-- ========================================
-- Step 1: Drop the existing stored generated column
-- ========================================
-- Note: Cannot ALTER a generated column's expression, must drop and recreate
ALTER TABLE detection_logs DROP COLUMN suspicious_score;

-- ========================================
-- Step 2: Recreate with correct INT type
-- ========================================
ALTER TABLE detection_logs
ADD COLUMN suspicious_score INT AS (JSON_VALUE(suspicious, '$.score')) STORED
COMMENT '不審スコア（仮想カラム、0-100整数）';

-- ========================================
-- Step 3: Recreate index
-- ========================================
-- Note: Index on generated column is automatically dropped when column is dropped
CREATE INDEX idx_logs_suspicious_score ON detection_logs (suspicious_score, captured_at);

-- ========================================
-- Migration verification
-- ========================================
-- After running, verify with:
-- DESCRIBE detection_logs;
-- Should show: suspicious_score INT GENERATED
