-- Migration: 030_nvt_brand.sql
-- Description: NVT/JOOAN camera brand integration
-- Date: 2026-01-17
-- Design Reference: camBrand/NewBrandIntegration/IMPLEMENTATION_TASKS.md

-- ========================================
-- NVT Brand Registration
-- ========================================

-- Add NVT brand
INSERT INTO camera_brands (name, display_name, category, is_builtin)
VALUES ('NVT', 'NVT / JOOAN', 'consumer', FALSE)
ON DUPLICATE KEY UPDATE display_name = VALUES(display_name);

-- ========================================
-- NVT OUI Entries
-- ========================================

-- DC:62:79 - Verified from IS22 stress test (JOOAN A6M-U)
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT 'DC:62:79', b.id, 'NVT/JOOAN camera (A6M-U verified)', 20, 'confirmed', 'device stress test 2026-01-17', FALSE
FROM camera_brands b WHERE b.name = 'NVT'
ON DUPLICATE KEY UPDATE description = VALUES(description), verification_source = VALUES(verification_source);

-- Additional potential NVT OUIs (candidate - needs verification)
-- These are known OUI ranges used by Chinese camera manufacturers
-- INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
-- SELECT 'XX:XX:XX', b.id, 'NVT candidate', 15, 'candidate', 'online research', FALSE
-- FROM camera_brands b WHERE b.name = 'NVT'
-- ON DUPLICATE KEY UPDATE description = VALUES(description);

-- ========================================
-- NVT RTSP Templates
-- ========================================

-- Standard NVT template (same as Tapo: /stream1, /stream2)
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, notes, is_builtin)
SELECT b.id, 'NVT Standard', '/stream1', '/stream2', 554, TRUE, 10,
       'Verified with JOOAN A6M-U. Same path structure as Tapo.', FALSE
FROM camera_brands b WHERE b.name = 'NVT'
ON DUPLICATE KEY UPDATE notes = VALUES(notes);

-- Alternative NVT paths (some models may use different paths)
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, notes, is_builtin)
SELECT b.id, 'NVT Live Channel', '/live/ch00_0', '/live/ch00_1', 554, FALSE, 20,
       'Alternative path for some NVT models', FALSE
FROM camera_brands b WHERE b.name = 'NVT'
ON DUPLICATE KEY UPDATE notes = VALUES(notes);

-- ========================================
-- Generic RTSP Paths (fallback)
-- ========================================

-- Add NVT-compatible generic path if not exists
INSERT INTO generic_rtsp_paths (main_path, sub_path, description, priority, is_enabled)
VALUES ('/live/ch00_0', '/live/ch00_1', 'NVT/JOOAN alternative path', 35, TRUE)
ON DUPLICATE KEY UPDATE description = VALUES(description);

-- ========================================
-- Verification Query
-- ========================================

-- Run this after migration to verify:
-- SELECT
--     b.name AS brand_name,
--     COUNT(DISTINCT o.oui_prefix) AS oui_count,
--     COUNT(DISTINCT t.id) AS template_count
-- FROM camera_brands b
-- LEFT JOIN oui_entries o ON b.id = o.brand_id
-- LEFT JOIN rtsp_templates t ON b.id = t.brand_id
-- WHERE b.name = 'NVT'
-- GROUP BY b.id;
