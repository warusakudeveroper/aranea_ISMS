-- Migration: 015_oui_rtsp_ssot.sql
-- Description: OUI/RTSP brand SSoT tables for camera brand management
-- Date: 2026-01-07
-- Design Reference: code/orangePi/is21-22/designs/headdesign/camscan/review_fixes_and_oui_rtsp_ssot_design.md
-- GitHub Issue: #80

-- ========================================
-- T1-1: Camera Brands Master Table
-- ========================================
CREATE TABLE IF NOT EXISTS camera_brands (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE COMMENT 'Internal name: TP-LINK, Hikvision',
    display_name VARCHAR(100) NOT NULL COMMENT 'Display name: TP-Link / Tapo',
    category ENUM('consumer', 'professional', 'enterprise', 'unknown') DEFAULT 'unknown',
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'System builtin (cannot be deleted)',
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    INDEX idx_camera_brands_category (category),
    INDEX idx_camera_brands_builtin (is_builtin)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- T1-2: OUI Entries Table (with status column)
-- ========================================
CREATE TABLE IF NOT EXISTS oui_entries (
    oui_prefix VARCHAR(8) NOT NULL PRIMARY KEY COMMENT 'XX:XX:XX format',
    brand_id INT NOT NULL,
    description VARCHAR(255) COMMENT 'e.g., Tapo C310',
    score_bonus INT NOT NULL DEFAULT 20 COMMENT 'Score addition for camera detection',
    status ENUM('confirmed', 'candidate', 'investigating') NOT NULL DEFAULT 'confirmed'
        COMMENT 'confirmed=verified, candidate=needs testing, investigating=unverified',
    verification_source VARCHAR(255) COMMENT 'e.g., maclookup.app, IEEE OUI, device test',
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    INDEX idx_oui_brand (brand_id),
    INDEX idx_oui_status (status),
    INDEX idx_oui_builtin (is_builtin),
    CONSTRAINT fk_oui_brand FOREIGN KEY (brand_id) REFERENCES camera_brands(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- T1-3: RTSP Templates Table
-- ========================================
CREATE TABLE IF NOT EXISTS rtsp_templates (
    id INT AUTO_INCREMENT PRIMARY KEY,
    brand_id INT NOT NULL,
    name VARCHAR(100) NOT NULL COMMENT 'Template name: Tapo Standard, Hikvision Main',
    main_path VARCHAR(255) NOT NULL COMMENT 'Main stream path: /stream1',
    sub_path VARCHAR(255) COMMENT 'Sub stream path: /stream2',
    default_port INT NOT NULL DEFAULT 554,
    is_default BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'Default template for this brand',
    priority INT NOT NULL DEFAULT 100 COMMENT 'Lower = higher priority',
    notes TEXT COMMENT 'Additional notes',
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    UNIQUE KEY uq_rtsp_template_brand_name (brand_id, name),
    INDEX idx_rtsp_template_brand (brand_id),
    INDEX idx_rtsp_template_default (is_default),
    INDEX idx_rtsp_template_priority (priority),
    CONSTRAINT fk_rtsp_template_brand FOREIGN KEY (brand_id) REFERENCES camera_brands(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- T1-4: Generic RTSP Paths Table (fallback)
-- ========================================
CREATE TABLE IF NOT EXISTS generic_rtsp_paths (
    id INT AUTO_INCREMENT PRIMARY KEY,
    main_path VARCHAR(255) NOT NULL,
    sub_path VARCHAR(255),
    description VARCHAR(255) COMMENT 'Description of the path',
    priority INT NOT NULL DEFAULT 100 COMMENT 'Lower = try first',
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    INDEX idx_generic_paths_priority (priority),
    INDEX idx_generic_paths_enabled (is_enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- T1-5: Cameras Table Extension
-- ========================================
-- Add rtsp_template_id column
ALTER TABLE cameras
ADD COLUMN rtsp_template_id INT DEFAULT NULL COMMENT 'FK to rtsp_templates',
ADD COLUMN rtsp_custom_main_path VARCHAR(255) DEFAULT NULL COMMENT 'Override main path',
ADD COLUMN rtsp_custom_sub_path VARCHAR(255) DEFAULT NULL COMMENT 'Override sub path',
ADD COLUMN status ENUM('active', 'inactive', 'pending_auth', 'maintenance') NOT NULL DEFAULT 'active'
    COMMENT 'Camera status: pending_auth for force-registered cameras',
ADD CONSTRAINT fk_cameras_rtsp_template FOREIGN KEY (rtsp_template_id) REFERENCES rtsp_templates(id) ON DELETE SET NULL;

-- Add index for status
CREATE INDEX idx_cameras_status ON cameras(status);

-- ========================================
-- T1-6: Initial Data - Camera Brands
-- ========================================
INSERT INTO camera_brands (name, display_name, category, is_builtin) VALUES
('TP-LINK', 'TP-Link / Tapo / VIGI', 'consumer', TRUE),
('Google', 'Google / Nest', 'consumer', TRUE),
('Hikvision', 'Hikvision', 'professional', TRUE),
('Dahua', 'Dahua', 'professional', TRUE),
('Axis', 'Axis Communications', 'enterprise', TRUE),
('Ring', 'Ring (Amazon)', 'consumer', TRUE),
('EZVIZ', 'EZVIZ', 'consumer', TRUE),
('Reolink', 'Reolink', 'consumer', TRUE),
('Amcrest', 'Amcrest', 'consumer', TRUE),
('Arlo', 'Arlo', 'consumer', TRUE),
('I-O-DATA', 'I.O.DATA', 'consumer', TRUE),
('SwitchBot', 'SwitchBot', 'consumer', TRUE),
('Panasonic', 'Panasonic / i-PRO', 'professional', TRUE);

-- ========================================
-- T1-6: Initial Data - OUI Entries
-- ========================================
-- TP-LINK OUIs
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, NULL, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT '70:5A:0F' as oui UNION ALL
    SELECT '54:AF:97' UNION ALL
    SELECT 'D8:07:B6' UNION ALL
    SELECT '98:DA:C4' UNION ALL
    SELECT 'B0:A7:B9' UNION ALL
    SELECT 'A8:42:E3' UNION ALL
    SELECT '60:A4:B7' UNION ALL
    SELECT '30:DE:4B' UNION ALL
    SELECT '5C:A6:E6' UNION ALL
    SELECT '14:EB:B6' UNION ALL
    SELECT 'AC:15:A2' UNION ALL
    SELECT '68:FF:7B' UNION ALL
    SELECT 'E8:48:B8'
) t, camera_brands b WHERE b.name = 'TP-LINK';

-- Google/Nest OUIs
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, NULL, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT '18:D6:C7' as oui UNION ALL
    SELECT '64:16:66' UNION ALL
    SELECT 'F8:0F:F9'
) t, camera_brands b WHERE b.name = 'Google';

-- Hikvision OUIs
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, NULL, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT 'C0:56:E3' as oui UNION ALL
    SELECT '54:C4:15' UNION ALL
    SELECT '44:19:B6'
) t, camera_brands b WHERE b.name = 'Hikvision';

-- Dahua OUIs
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, NULL, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT '3C:EF:8C' as oui UNION ALL
    SELECT 'E0:50:8B'
) t, camera_brands b WHERE b.name = 'Dahua';

-- Axis OUIs
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, NULL, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT '00:40:8C' as oui UNION ALL
    SELECT 'AC:CC:8E'
) t, camera_brands b WHERE b.name = 'Axis';

-- Ring OUIs
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, NULL, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT '34:3E:A4' as oui UNION ALL
    SELECT '0C:24:D6' UNION ALL
    SELECT '2C:72:FF' UNION ALL
    SELECT '3C:24:E0' UNION ALL
    SELECT '50:14:79' UNION ALL
    SELECT '70:3A:CB' UNION ALL
    SELECT '74:C1:4F' UNION ALL
    SELECT '88:57:EE' UNION ALL
    SELECT '90:6D:C5' UNION ALL
    SELECT '9C:76:14' UNION ALL
    SELECT 'B0:09:DA' UNION ALL
    SELECT 'F4:5F:D4'
) t, camera_brands b WHERE b.name = 'Ring';

-- EZVIZ OUIs
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, NULL, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT '50:28:73' as oui UNION ALL
    SELECT '58:73:D8' UNION ALL
    SELECT '60:3A:7C' UNION ALL
    SELECT '68:D8:4A' UNION ALL
    SELECT '80:E1:BF' UNION ALL
    SELECT '84:F7:03' UNION ALL
    SELECT 'A4:DA:22' UNION ALL
    SELECT 'AC:1A:3D' UNION ALL
    SELECT 'B0:C5:54' UNION ALL
    SELECT 'B4:A3:82' UNION ALL
    SELECT 'BC:94:0D' UNION ALL
    SELECT 'D0:60:8C' UNION ALL
    SELECT 'E4:13:D3'
) t, camera_brands b WHERE b.name = 'EZVIZ';

-- Reolink OUI
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT 'EC:71:DB', b.id, NULL, 20, 'confirmed', 'maclookup.app', TRUE
FROM camera_brands b WHERE b.name = 'Reolink';

-- Amcrest OUIs
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, NULL, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT '9C:8E:CD' as oui UNION ALL
    SELECT 'E0:37:BF' UNION ALL
    SELECT 'FC:6F:B7' UNION ALL
    SELECT '24:4B:03'
) t, camera_brands b WHERE b.name = 'Amcrest';

-- Arlo OUIs
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, NULL, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT '28:80:23' as oui UNION ALL
    SELECT '9C:C1:73' UNION ALL
    SELECT 'E0:63:E5'
) t, camera_brands b WHERE b.name = 'Arlo';

-- I.O.DATA OUIs
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, NULL, 20, 'confirmed', 'maclookup.app', TRUE
FROM (
    SELECT '00:A0:B0' as oui UNION ALL
    SELECT '34:76:C5' UNION ALL
    SELECT 'DC:B4:C4'
) t, camera_brands b WHERE b.name = 'I-O-DATA';

-- SwitchBot OUI
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT '6C:DF:FB', b.id, 'Shenzhen Intellirocks Tech', 20, 'confirmed', 'maclookup.app', TRUE
FROM camera_brands b WHERE b.name = 'SwitchBot';

-- Panasonic OUIs (candidate - needs device verification)
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT oui, b.id, NULL, 15, 'candidate', 'maclookup.app - camera related subset', TRUE
FROM (
    SELECT '00:80:45' as oui UNION ALL
    SELECT '04:20:9A' UNION ALL
    SELECT '08:60:6E' UNION ALL
    SELECT '10:A5:D0' UNION ALL
    SELECT '34:E6:D7' UNION ALL
    SELECT '50:E5:49' UNION ALL
    SELECT '64:B5:C6' UNION ALL
    SELECT '8C:09:F4' UNION ALL
    SELECT 'AC:D1:B8' UNION ALL
    SELECT 'D0:17:C2'
) t, camera_brands b WHERE b.name = 'Panasonic';

-- ========================================
-- T1-6: Initial Data - RTSP Templates
-- ========================================
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'Tapo/VIGI Standard', '/stream1', '/stream2', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'TP-LINK';

INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'Hikvision Standard', '/Streaming/Channels/101', '/Streaming/Channels/102', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'Hikvision';

INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'Dahua Standard', '/cam/realmonitor?channel=1&subtype=0', '/cam/realmonitor?channel=1&subtype=1', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'Dahua';

INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'Axis Standard', '/axis-media/media.amp', '/axis-media/media.amp?videocodec=h264', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'Axis';

INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'EZVIZ Standard', '/H.264/ch1/main/av_stream', '/H.264/ch1/sub/av_stream', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'EZVIZ';

INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'Reolink Standard', '/h264Preview_01_main', '/h264Preview_01_sub', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'Reolink';

INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'Amcrest Standard', '/cam/realmonitor?channel=1&subtype=0', '/cam/realmonitor?channel=1&subtype=1', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'Amcrest';

INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'Panasonic i-PRO Standard', '/MediaInput/h264/stream_1', '/MediaInput/h264/stream_2', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'Panasonic';

-- ========================================
-- T1-6: Initial Data - Generic RTSP Paths
-- ========================================
INSERT INTO generic_rtsp_paths (main_path, sub_path, description, priority, is_enabled) VALUES
('/stream1', '/stream2', 'Common stream path (Tapo compatible)', 10, TRUE),
('/live', '/live', 'Generic live stream', 20, TRUE),
('/h264_stream', '/h264_stream', 'H.264 stream', 30, TRUE),
('/video1', '/video2', 'Video stream', 40, TRUE),
('/cam/realmonitor?channel=1&subtype=0', '/cam/realmonitor?channel=1&subtype=1', 'Dahua compatible', 50, TRUE),
('/Streaming/Channels/101', '/Streaming/Channels/102', 'Hikvision compatible', 60, TRUE);
