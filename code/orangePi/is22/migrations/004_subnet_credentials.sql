-- IS22 Migration: 004_subnet_credentials
-- Date: 2026-01-01
-- Purpose: Add credential trial support for IpcamScan

-- ========================================
-- Add facility_name and credentials to scan_subnets
-- ========================================
ALTER TABLE scan_subnets
ADD COLUMN facility_name VARCHAR(128) NULL AFTER fid,
ADD COLUMN credentials JSON NULL AFTER facility_name;

-- credentials format:
-- [
--   {"username": "admin", "password": "password123", "priority": 1},
--   {"username": "halecam", "password": "hale0083", "priority": 2}
-- ]

-- ========================================
-- Add credential_status to ipcamscan_devices
-- ========================================
ALTER TABLE ipcamscan_devices
ADD COLUMN credential_status ENUM('not_tried', 'success', 'failed') NOT NULL DEFAULT 'not_tried' AFTER verified,
ADD COLUMN credential_username VARCHAR(64) NULL AFTER credential_status;

-- credential_status values:
-- - not_tried: No credentials were tried during scan
-- - success: At least one credential worked, model info retrieved
-- - failed: All credentials were tried but none worked

-- ========================================
-- Index for faster credential lookups by fid
-- ========================================
CREATE INDEX idx_subnets_fid ON scan_subnets(fid);
