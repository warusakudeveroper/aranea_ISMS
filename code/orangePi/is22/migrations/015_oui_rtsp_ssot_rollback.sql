-- Rollback: 015_oui_rtsp_ssot_rollback.sql
-- Description: Rollback OUI/RTSP SSoT tables
-- Date: 2026-01-07
-- Warning: This will delete all camera brand, OUI, and RTSP template data

-- ========================================
-- Step 1: Remove FK constraint from cameras
-- ========================================
ALTER TABLE cameras
DROP FOREIGN KEY fk_cameras_rtsp_template;

-- ========================================
-- Step 2: Remove added columns from cameras
-- ========================================
ALTER TABLE cameras
DROP COLUMN rtsp_template_id,
DROP COLUMN rtsp_custom_main_path,
DROP COLUMN rtsp_custom_sub_path,
DROP COLUMN status;

-- ========================================
-- Step 3: Drop indexes
-- ========================================
DROP INDEX idx_cameras_status ON cameras;

-- ========================================
-- Step 4: Drop tables in reverse order (respecting FK constraints)
-- ========================================
DROP TABLE IF EXISTS generic_rtsp_paths;
DROP TABLE IF EXISTS rtsp_templates;
DROP TABLE IF EXISTS oui_entries;
DROP TABLE IF EXISTS camera_brands;

-- ========================================
-- Verification
-- ========================================
-- Run these queries to verify rollback:
--
-- SHOW TABLES LIKE 'camera_brands';
-- SHOW TABLES LIKE 'oui_entries';
-- SHOW TABLES LIKE 'rtsp_templates';
-- SHOW TABLES LIKE 'generic_rtsp_paths';
-- DESCRIBE cameras;
