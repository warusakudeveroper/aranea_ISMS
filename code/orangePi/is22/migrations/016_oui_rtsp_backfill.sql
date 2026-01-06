-- Migration: 016_oui_rtsp_backfill.sql
-- Description: Backfill existing cameras with RTSP template mappings
-- Date: 2026-01-07
-- Design Reference: code/orangePi/is21-22/designs/headdesign/camscan/review_fixes_and_oui_rtsp_ssot_design.md
-- GitHub Issue: #80
-- Note: Run AFTER 015_oui_rtsp_ssot.sql

-- ========================================
-- T1-7: Backfill Existing Cameras
-- ========================================

-- Pattern 1: Tapo/VIGI format (/stream1, /stream2)
UPDATE cameras c
JOIN camera_brands b ON b.name = 'TP-LINK'
JOIN rtsp_templates t ON t.brand_id = b.id AND t.is_default = TRUE
SET c.rtsp_template_id = t.id
WHERE c.rtsp_main LIKE '%/stream1%'
  AND c.rtsp_template_id IS NULL;

-- Pattern 2: Hikvision format (/Streaming/Channels/)
UPDATE cameras c
JOIN camera_brands b ON b.name = 'Hikvision'
JOIN rtsp_templates t ON t.brand_id = b.id AND t.is_default = TRUE
SET c.rtsp_template_id = t.id
WHERE c.rtsp_main LIKE '%/Streaming/Channels/%'
  AND c.rtsp_template_id IS NULL;

-- Pattern 3: Dahua format (/cam/realmonitor)
UPDATE cameras c
JOIN camera_brands b ON b.name = 'Dahua'
JOIN rtsp_templates t ON t.brand_id = b.id AND t.is_default = TRUE
SET c.rtsp_template_id = t.id
WHERE c.rtsp_main LIKE '%/cam/realmonitor%'
  AND c.rtsp_template_id IS NULL;

-- Pattern 4: Axis format (/axis-media/)
UPDATE cameras c
JOIN camera_brands b ON b.name = 'Axis'
JOIN rtsp_templates t ON t.brand_id = b.id AND t.is_default = TRUE
SET c.rtsp_template_id = t.id
WHERE c.rtsp_main LIKE '%/axis-media/%'
  AND c.rtsp_template_id IS NULL;

-- Pattern 5: EZVIZ format (/H.264/)
UPDATE cameras c
JOIN camera_brands b ON b.name = 'EZVIZ'
JOIN rtsp_templates t ON t.brand_id = b.id AND t.is_default = TRUE
SET c.rtsp_template_id = t.id
WHERE c.rtsp_main LIKE '%/H.264/%'
  AND c.rtsp_template_id IS NULL;

-- Pattern 6: Reolink format (/h264Preview_)
UPDATE cameras c
JOIN camera_brands b ON b.name = 'Reolink'
JOIN rtsp_templates t ON t.brand_id = b.id AND t.is_default = TRUE
SET c.rtsp_template_id = t.id
WHERE c.rtsp_main LIKE '%/h264Preview_%'
  AND c.rtsp_template_id IS NULL;

-- Pattern 7: Panasonic format (/MediaInput/)
UPDATE cameras c
JOIN camera_brands b ON b.name = 'Panasonic'
JOIN rtsp_templates t ON t.brand_id = b.id AND t.is_default = TRUE
SET c.rtsp_template_id = t.id
WHERE c.rtsp_main LIKE '%/MediaInput/%'
  AND c.rtsp_template_id IS NULL;

-- Pattern 8: Extract custom paths for unmatched cameras
-- Use SUBSTRING_INDEX to extract path from RTSP URL
UPDATE cameras c
SET c.rtsp_custom_main_path =
    CASE
        WHEN c.rtsp_main LIKE 'rtsp://%@%' THEN
            SUBSTRING(c.rtsp_main,
                LOCATE('/', c.rtsp_main, LOCATE('@', c.rtsp_main)))
        WHEN c.rtsp_main LIKE 'rtsp://%' THEN
            SUBSTRING(c.rtsp_main,
                LOCATE('/', c.rtsp_main, 8))
        ELSE NULL
    END,
    c.rtsp_custom_sub_path =
    CASE
        WHEN c.rtsp_sub LIKE 'rtsp://%@%' THEN
            SUBSTRING(c.rtsp_sub,
                LOCATE('/', c.rtsp_sub, LOCATE('@', c.rtsp_sub)))
        WHEN c.rtsp_sub LIKE 'rtsp://%' THEN
            SUBSTRING(c.rtsp_sub,
                LOCATE('/', c.rtsp_sub, 8))
        ELSE NULL
    END
WHERE c.rtsp_template_id IS NULL
  AND c.rtsp_main IS NOT NULL
  AND c.rtsp_custom_main_path IS NULL;

-- ========================================
-- Verification Queries (for manual check)
-- ========================================
-- Run these queries to verify backfill results:
--
-- SELECT
--     CASE
--         WHEN rtsp_template_id IS NOT NULL THEN 'Template Applied'
--         WHEN rtsp_custom_main_path IS NOT NULL THEN 'Custom Path'
--         ELSE 'Not Migrated'
--     END AS migration_status,
--     COUNT(*) AS camera_count
-- FROM cameras
-- GROUP BY migration_status;
--
-- SELECT
--     b.display_name AS brand,
--     t.name AS template_name,
--     COUNT(c.camera_id) AS camera_count
-- FROM cameras c
-- JOIN rtsp_templates t ON c.rtsp_template_id = t.id
-- JOIN camera_brands b ON t.brand_id = b.id
-- GROUP BY b.display_name, t.name
-- ORDER BY camera_count DESC;
