-- Migration: 017_oui_expansion.sql
-- Description: Expand OUI entries for additional camera brands (RT-04)
-- Date: 2026-01-07
-- Design Reference: code/orangePi/is21-22/designs/headdesign/camscan/REMAINING_TASKS.md
-- GitHub Issue: #87

-- ========================================
-- New Camera Brands
-- ========================================
INSERT INTO camera_brands (name, display_name, category, is_builtin) VALUES
('Anker', 'Anker / Eufy', 'consumer', TRUE),
('Wyze', 'Wyze', 'consumer', TRUE),
('Blink', 'Amazon Blink', 'consumer', TRUE),
('Aqara', 'Aqara (Xiaomi)', 'consumer', TRUE),
('Imou', 'Imou (Dahua)', 'consumer', TRUE),
('Xiaomi', 'Xiaomi / Mi Home', 'consumer', TRUE),
('Ubiquiti', 'Ubiquiti UniFi', 'professional', TRUE)
ON DUPLICATE KEY UPDATE display_name = VALUES(display_name);

-- ========================================
-- Anker / Eufy OUIs
-- ========================================
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, desc_text, 20, 'candidate', 'maclookup.app', TRUE
FROM (
    SELECT '78:9C:85' as oui, 'Anker Innovations' as desc_text UNION ALL
    SELECT 'D4:25:CC', 'Eufy Security' UNION ALL
    SELECT 'AC:F1:08', 'Anker Innovations' UNION ALL
    SELECT '00:90:89', 'Anker'
) t, camera_brands b WHERE b.name = 'Anker'
ON DUPLICATE KEY UPDATE description = VALUES(description);

-- ========================================
-- Wyze OUIs
-- ========================================
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, desc_text, 20, 'candidate', 'maclookup.app', TRUE
FROM (
    SELECT '2C:AA:8E' as oui, 'Wyze Labs' as desc_text UNION ALL
    SELECT 'B4:D4:A4', 'Wyze Labs' UNION ALL
    SELECT 'E8:AB:FA', 'Wyze Labs'
) t, camera_brands b WHERE b.name = 'Wyze'
ON DUPLICATE KEY UPDATE description = VALUES(description);

-- ========================================
-- Amazon Blink OUIs
-- ========================================
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, desc_text, 20, 'candidate', 'maclookup.app', TRUE
FROM (
    SELECT '6C:56:97' as oui, 'Amazon Blink' as desc_text UNION ALL
    SELECT '74:DA:EA', 'Amazon Blink' UNION ALL
    SELECT '84:D6:D0', 'Amazon Technologies'
) t, camera_brands b WHERE b.name = 'Blink'
ON DUPLICATE KEY UPDATE description = VALUES(description);

-- ========================================
-- Aqara / Xiaomi OUIs
-- ========================================
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, desc_text, 20, 'candidate', 'maclookup.app', TRUE
FROM (
    SELECT '54:EF:44' as oui, 'Lumi/Aqara' as desc_text UNION ALL
    SELECT '04:CF:8C', 'Xiaomi/Aqara' UNION ALL
    SELECT '78:11:DC', 'Xiaomi/Aqara'
) t, camera_brands b WHERE b.name = 'Aqara'
ON DUPLICATE KEY UPDATE description = VALUES(description);

-- ========================================
-- Xiaomi / Mi Home OUIs
-- ========================================
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, desc_text, 20, 'candidate', 'maclookup.app', TRUE
FROM (
    SELECT '78:02:F8' as oui, 'Xiaomi Communications' as desc_text UNION ALL
    SELECT '64:09:80', 'Xiaomi Communications' UNION ALL
    SELECT '0C:1D:AF', 'Xiaomi Mi Home' UNION ALL
    SELECT '20:47:DA', 'Xiaomi Communications' UNION ALL
    SELECT '28:6C:07', 'Xiaomi Communications' UNION ALL
    SELECT '34:CE:00', 'Xiaomi Mi Home' UNION ALL
    SELECT '50:64:2B', 'Xiaomi Communications' UNION ALL
    SELECT '58:44:98', 'Xiaomi Mi Home' UNION ALL
    SELECT '64:B4:73', 'Xiaomi Communications' UNION ALL
    SELECT '74:23:44', 'Xiaomi Communications' UNION ALL
    SELECT '7C:49:EB', 'Xiaomi Mi' UNION ALL
    SELECT '8C:DE:F9', 'Xiaomi Communications' UNION ALL
    SELECT 'A4:77:33', 'Xiaomi Communications' UNION ALL
    SELECT 'B0:E2:35', 'Xiaomi Mi' UNION ALL
    SELECT 'C4:0B:CB', 'Xiaomi Communications' UNION ALL
    SELECT 'F8:A4:5F', 'Xiaomi Mi Home'
) t, camera_brands b WHERE b.name = 'Xiaomi'
ON DUPLICATE KEY UPDATE description = VALUES(description);

-- ========================================
-- Imou (Dahua subsidiary) OUIs
-- ========================================
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, desc_text, 20, 'candidate', 'maclookup.app', TRUE
FROM (
    SELECT 'E0:50:8B' as oui, 'Imou/Dahua' as desc_text UNION ALL
    SELECT 'A0:BD:1D', 'Imou Technology' UNION ALL
    SELECT '20:4A:E1', 'Imou Technology'
) t, camera_brands b WHERE b.name = 'Imou'
ON DUPLICATE KEY UPDATE description = VALUES(description);

-- ========================================
-- Ubiquiti UniFi OUIs
-- ========================================
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, desc_text, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT '00:27:22' as oui, 'Ubiquiti Networks' as desc_text UNION ALL
    SELECT '24:A4:3C', 'Ubiquiti Inc' UNION ALL
    SELECT '44:D9:E7', 'Ubiquiti Inc' UNION ALL
    SELECT '68:72:51', 'Ubiquiti Inc' UNION ALL
    SELECT '74:83:C2', 'Ubiquiti Inc' UNION ALL
    SELECT '78:8A:20', 'Ubiquiti Inc' UNION ALL
    SELECT '80:2A:A8', 'Ubiquiti Inc' UNION ALL
    SELECT 'AC:8B:A9', 'Ubiquiti Inc' UNION ALL
    SELECT 'B4:FB:E4', 'Ubiquiti Inc' UNION ALL
    SELECT 'DC:9F:DB', 'Ubiquiti Inc' UNION ALL
    SELECT 'E0:63:DA', 'Ubiquiti Inc' UNION ALL
    SELECT 'F0:9F:C2', 'Ubiquiti Inc' UNION ALL
    SELECT 'FC:EC:DA', 'Ubiquiti Inc'
) t, camera_brands b WHERE b.name = 'Ubiquiti'
ON DUPLICATE KEY UPDATE description = VALUES(description);

-- ========================================
-- Extended TP-LINK / Tapo OUIs (additional)
-- ========================================
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, desc_text, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT '6C:5A:B0' as oui, 'TP-LINK Tapo' as desc_text UNION ALL
    SELECT '6C:C8:40', 'TP-LINK Tapo' UNION ALL
    SELECT '08:A6:F7', 'TP-LINK' UNION ALL
    SELECT '78:20:51', 'TP-LINK' UNION ALL
    SELECT 'BC:07:1D', 'TP-LINK' UNION ALL
    SELECT '3C:84:6A', 'TP-LINK Tapo C500/C310' UNION ALL
    SELECT 'D8:44:89', 'TP-LINK Tapo' UNION ALL
    SELECT '9C:53:22', 'TP-LINK Tapo C310' UNION ALL
    SELECT '64:64:4A', 'TP-LINK' UNION ALL
    SELECT '84:D8:1B', 'TP-LINK' UNION ALL
    SELECT '90:9A:4A', 'TP-LINK' UNION ALL
    SELECT '98:25:4A', 'TP-LINK' UNION ALL
    SELECT 'A8:57:4E', 'TP-LINK' UNION ALL
    SELECT 'AC:84:C6', 'TP-LINK' UNION ALL
    SELECT 'B0:4E:26', 'TP-LINK' UNION ALL
    SELECT 'B8:27:EB', 'TP-LINK' UNION ALL
    SELECT 'C0:25:E9', 'TP-LINK' UNION ALL
    SELECT 'C4:6E:1F', 'TP-LINK' UNION ALL
    SELECT 'D4:6E:0E', 'TP-LINK' UNION ALL
    SELECT 'E4:6F:13', 'TP-LINK' UNION ALL
    SELECT 'EC:08:6B', 'TP-LINK' UNION ALL
    SELECT 'F0:F3:36', 'TP-LINK' UNION ALL
    SELECT 'F4:EC:38', 'TP-LINK'
) t, camera_brands b WHERE b.name = 'TP-LINK'
ON DUPLICATE KEY UPDATE description = VALUES(description);

-- ========================================
-- RTSP Templates for new brands
-- ========================================

-- Wyze Standard
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'Wyze Standard', '/live', NULL, 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'Wyze'
ON DUPLICATE KEY UPDATE main_path = VALUES(main_path);

-- Anker/Eufy Standard
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'Eufy Standard', '/live0', '/live1', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'Anker'
ON DUPLICATE KEY UPDATE main_path = VALUES(main_path);

-- Imou Standard (Dahua compatible)
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'Imou Standard', '/cam/realmonitor?channel=1&subtype=0', '/cam/realmonitor?channel=1&subtype=1', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'Imou'
ON DUPLICATE KEY UPDATE main_path = VALUES(main_path);

-- Xiaomi Standard
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'Xiaomi Standard', '/live', NULL, 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'Xiaomi'
ON DUPLICATE KEY UPDATE main_path = VALUES(main_path);

-- Ubiquiti UniFi Standard
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'UniFi Protect', '/proxy/protect/ws/api', NULL, 443, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'Ubiquiti'
ON DUPLICATE KEY UPDATE main_path = VALUES(main_path);
