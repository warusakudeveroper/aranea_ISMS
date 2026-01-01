-- Migration 003: Add detection_json for user-friendly device classification
-- This stores DetectionReason struct as JSON for UI feedback

-- Add detection_json column to ipcamscan_devices
ALTER TABLE ipcamscan_devices
ADD COLUMN IF NOT EXISTS detection_json JSON DEFAULT NULL
COMMENT 'JSON: DetectionReason with user_message, device_type, suggested_action';

-- Index for filtering by device type (extracted from JSON)
-- Note: MySQL 8.0+ supports functional indexes
-- ALTER TABLE ipcamscan_devices
-- ADD INDEX idx_device_type ((CAST(detection_json->>'$.device_type' AS CHAR(50))));
