-- IS22 Camserver Migration 002
-- Camera Registration Flow Support
-- Version: 002

-- ========================================
-- Add status column to ipcamscan_devices
-- ========================================
ALTER TABLE ipcamscan_devices
ADD COLUMN status ENUM('discovered', 'verifying', 'verified', 'rejected', 'approved')
    NOT NULL DEFAULT 'discovered' AFTER confidence;

ALTER TABLE ipcamscan_devices
ADD INDEX idx_devices_status (status);

-- ========================================
-- Add source_device_id to cameras (for traceability)
-- ========================================
ALTER TABLE cameras
ADD COLUMN source_device_id CHAR(36) NULL AFTER lacis_id;

ALTER TABLE cameras
ADD COLUMN fid VARCHAR(4) NULL AFTER source_device_id;

-- ========================================
-- Facility Credentials (for batch verification)
-- ========================================
CREATE TABLE IF NOT EXISTS facility_credentials (
    fid VARCHAR(4) PRIMARY KEY,
    facility_name VARCHAR(128) NOT NULL,
    username VARCHAR(64) NOT NULL,
    password_encrypted VARBINARY(256) NOT NULL,
    encryption_iv VARBINARY(16) NOT NULL,
    notes TEXT,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- Camera Registration Audit Log
-- ========================================
CREATE TABLE IF NOT EXISTS camera_registration_log (
    log_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    device_id CHAR(36) NOT NULL,
    camera_id VARCHAR(64),
    action ENUM('discovered', 'verify_started', 'verify_success', 'verify_failed', 'approved', 'rejected') NOT NULL,
    performed_by VARCHAR(64) NOT NULL DEFAULT 'system',
    details JSON,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX idx_reglog_device (device_id),
    INDEX idx_reglog_action (action, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- Subnet Configuration (for scan targets)
-- ========================================
CREATE TABLE IF NOT EXISTS scan_subnets (
    subnet_id CHAR(36) PRIMARY KEY,
    cidr VARCHAR(45) NOT NULL,
    fid VARCHAR(4),
    description VARCHAR(256),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_scanned_at DATETIME(3),
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    UNIQUE KEY uq_subnets_cidr (cidr),
    INDEX idx_subnets_fid (fid),
    INDEX idx_subnets_enabled (enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
