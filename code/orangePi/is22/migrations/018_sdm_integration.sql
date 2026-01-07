-- IS22 Camserver SDM Integration
-- Version: 018
-- Purpose: Add Google Nest Doorbell SDM integration support

-- ========================================
-- Cameras SDM Extension
-- ========================================
-- Add SDM-specific columns for Nest Doorbell integration
-- SSoT: sdm_device_id is the unique identifier from Google SDM API

ALTER TABLE cameras
    ADD COLUMN sdm_device_id VARCHAR(128) NULL COMMENT 'Google SDM device identifier (enterprises/.../devices/xxx)',
    ADD COLUMN sdm_structure VARCHAR(128) NULL COMMENT 'SDM structure/home identifier',
    ADD COLUMN sdm_traits JSON NULL COMMENT 'SDM device traits summary (not for AI hints)';

-- Unique index on sdm_device_id (allows NULL for non-SDM cameras)
CREATE UNIQUE INDEX idx_cameras_sdm_device_id ON cameras(sdm_device_id);

-- Index for filtering SDM cameras
CREATE INDEX idx_cameras_sdm ON cameras(family, discovery_method)
    COMMENT 'For filtering Nest/SDM cameras (family=nest, discovery_method=sdm)';

-- ========================================
-- Rollback Instructions
-- ========================================
-- To rollback this migration:
--
-- DROP INDEX idx_cameras_sdm ON cameras;
-- DROP INDEX idx_cameras_sdm_device_id ON cameras;
-- ALTER TABLE cameras DROP COLUMN sdm_traits;
-- ALTER TABLE cameras DROP COLUMN sdm_structure;
-- ALTER TABLE cameras DROP COLUMN sdm_device_id;
