-- IS22 Camserver Initial Schema
-- Database: camserver
-- Version: 001

-- ========================================
-- Cameras (SSoT)
-- ========================================
CREATE TABLE IF NOT EXISTS cameras (
    camera_id VARCHAR(64) PRIMARY KEY,
    name VARCHAR(128) NOT NULL,
    location VARCHAR(128) NOT NULL,
    floor VARCHAR(32),
    rtsp_main VARCHAR(512),
    rtsp_sub VARCHAR(512),
    snapshot_url VARCHAR(512),
    family ENUM('tapo', 'vigi', 'nest', 'axis', 'hikvision', 'dahua', 'other', 'unknown') NOT NULL DEFAULT 'unknown',
    manufacturer VARCHAR(64),
    model VARCHAR(64),
    ip_address VARCHAR(45),
    mac_address VARCHAR(17),
    lacis_id VARCHAR(32),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    polling_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    polling_interval_sec INT NOT NULL DEFAULT 60,
    suggest_policy_weight INT NOT NULL DEFAULT 5,
    camera_context JSON,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    INDEX idx_cameras_location (location),
    INDEX idx_cameras_enabled (enabled, polling_enabled),
    INDEX idx_cameras_family (family)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- Schema Versions
-- ========================================
CREATE TABLE IF NOT EXISTS schema_versions (
    version_id VARCHAR(32) PRIMARY KEY,
    schema_json JSON NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX idx_schema_active (is_active)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- Settings
-- ========================================
CREATE TABLE IF NOT EXISTS settings (
    setting_key VARCHAR(64) PRIMARY KEY,
    setting_json JSON NOT NULL,
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- Modal Leases
-- ========================================
CREATE TABLE IF NOT EXISTS modal_leases (
    lease_id CHAR(36) PRIMARY KEY,
    user_id VARCHAR(128) NOT NULL,
    camera_id VARCHAR(64) NOT NULL,
    quality ENUM('sub', 'main') NOT NULL DEFAULT 'sub',
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    expires_at DATETIME(3) NOT NULL,
    last_heartbeat DATETIME(3) NOT NULL,

    UNIQUE KEY uq_leases_user (user_id),
    INDEX idx_leases_expires (expires_at),
    INDEX idx_leases_heartbeat (last_heartbeat),
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- Admission Metrics
-- ========================================
CREATE TABLE IF NOT EXISTS admission_metrics (
    metric_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    recorded_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    active_modals INT UNSIGNED NOT NULL,
    active_suggest TINYINT(1) NOT NULL,
    cpu_percent DECIMAL(5,2) NOT NULL,
    memory_percent DECIMAL(5,2) NOT NULL,
    requests_total INT UNSIGNED NOT NULL,
    rejections_total INT UNSIGNED NOT NULL,

    INDEX idx_metrics_time (recorded_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- Frames (detection snapshots)
-- ========================================
CREATE TABLE IF NOT EXISTS frames (
    frame_id CHAR(36) PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL,
    captured_at DATETIME(3) NOT NULL,
    detected BOOLEAN NOT NULL DEFAULT FALSE,
    thumbnail_path VARCHAR(512),
    full_path VARCHAR(512),
    drive_file_id VARCHAR(128),
    processing_ms INT,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX idx_frames_camera (camera_id, captured_at),
    INDEX idx_frames_detected (detected, captured_at),
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- Events (detection events)
-- ========================================
CREATE TABLE IF NOT EXISTS events (
    event_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    frame_id CHAR(36) NOT NULL,
    camera_id VARCHAR(64) NOT NULL,
    captured_at DATETIME(3) NOT NULL,
    primary_event VARCHAR(32) NOT NULL,
    severity INT NOT NULL DEFAULT 0,
    tags JSON NOT NULL,
    unknown_flag BOOLEAN NOT NULL DEFAULT FALSE,
    attributes JSON,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX idx_events_camera (camera_id, captured_at),
    INDEX idx_events_severity (severity, captured_at),
    INDEX idx_events_primary (primary_event, captured_at),
    FOREIGN KEY (frame_id) REFERENCES frames(frame_id) ON DELETE CASCADE,
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- Notifications
-- ========================================
CREATE TABLE IF NOT EXISTS notifications (
    notification_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    event_id BIGINT UNSIGNED,
    notification_type ENUM('detection', 'summary', 'alert') NOT NULL,
    recipient VARCHAR(256),
    sent_at DATETIME(3),
    status ENUM('pending', 'sent', 'failed') NOT NULL DEFAULT 'pending',
    error_message TEXT,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX idx_notifications_event (event_id),
    INDEX idx_notifications_status (status, created_at),
    FOREIGN KEY (event_id) REFERENCES events(event_id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- IpcamScan Jobs
-- ========================================
CREATE TABLE IF NOT EXISTS ipcamscan_jobs (
    job_id CHAR(36) PRIMARY KEY,
    requested_by VARCHAR(64) NOT NULL,
    targets JSON NOT NULL,
    mode ENUM('discovery', 'deep') NOT NULL DEFAULT 'discovery',
    ports JSON NOT NULL,
    timeout_ms INT UNSIGNED NOT NULL DEFAULT 500,
    concurrency TINYINT UNSIGNED NOT NULL DEFAULT 10,
    rate_limit_pps INT UNSIGNED NOT NULL DEFAULT 100,
    status ENUM('queued', 'running', 'partial', 'success', 'failed', 'canceled') NOT NULL DEFAULT 'queued',
    started_at DATETIME(3),
    ended_at DATETIME(3),
    summary_json JSON,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX idx_jobs_status (status),
    INDEX idx_jobs_created (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- IpcamScan Devices
-- ========================================
CREATE TABLE IF NOT EXISTS ipcamscan_devices (
    device_id CHAR(36) PRIMARY KEY,
    ip VARCHAR(45) NOT NULL,
    subnet VARCHAR(45) NOT NULL,
    mac VARCHAR(17),
    oui_vendor VARCHAR(64),
    hostnames JSON,
    open_ports JSON NOT NULL,
    discovery_onvif JSON,
    discovery_ssdp JSON,
    discovery_mdns JSON,
    score INT UNSIGNED NOT NULL DEFAULT 0,
    verified TINYINT(1) NOT NULL DEFAULT 0,
    manufacturer VARCHAR(64),
    model VARCHAR(64),
    firmware VARCHAR(128),
    family ENUM('tapo', 'vigi', 'nest', 'other', 'unknown') NOT NULL DEFAULT 'unknown',
    confidence TINYINT UNSIGNED NOT NULL DEFAULT 0,
    rtsp_uri VARCHAR(512),
    first_seen DATETIME(3) NOT NULL,
    last_seen DATETIME(3) NOT NULL,

    UNIQUE KEY uq_devices_ip (ip),
    INDEX idx_devices_family (family),
    INDEX idx_devices_score (score),
    INDEX idx_devices_verified (verified)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- IpcamScan Evidence
-- ========================================
CREATE TABLE IF NOT EXISTS ipcamscan_evidence (
    evidence_id CHAR(36) PRIMARY KEY,
    device_id CHAR(36) NOT NULL,
    job_id CHAR(36) NOT NULL,
    evidence_type ENUM('arp', 'portscan', 'wsd', 'ssdp', 'mdns', 'onvif_auth', 'rtsp_auth') NOT NULL,
    raw_summary TEXT,
    collected_at DATETIME(3) NOT NULL,
    source VARCHAR(64) NOT NULL,

    INDEX idx_evidence_device (device_id),
    INDEX idx_evidence_job (job_id),
    FOREIGN KEY (device_id) REFERENCES ipcamscan_devices(device_id) ON DELETE CASCADE,
    FOREIGN KEY (job_id) REFERENCES ipcamscan_jobs(job_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ========================================
-- Default Settings
-- ========================================
INSERT INTO settings (setting_key, setting_json) VALUES
('polling', JSON_OBJECT(
    'base_interval_sec', 60,
    'priority_interval_sec', 15,
    'max_concurrent', 5,
    'timeout_ms', 10000
)),
('suggest', JSON_OBJECT(
    'min_score_threshold', 50,
    'ttl_sec', 60,
    'hysteresis_ratio', 1.2,
    'cooldown_after_clear_sec', 10,
    'sticky_policies', JSON_ARRAY('hazard_detected')
)),
('admission', JSON_OBJECT(
    'total_stream_units', 50,
    'max_ui_users', 10,
    'reserved_baseline_units', 5,
    'sub_stream_cost', 1,
    'main_stream_cost', 2,
    'modal_ttl_sec', 300,
    'heartbeat_interval_sec', 30,
    'heartbeat_grace_sec', 45
)),
('notification', JSON_OBJECT(
    'cooldown_sec', 300,
    'daily_summary_enabled', true,
    'daily_summary_time', '08:00',
    'webhook_url', null
))
ON DUPLICATE KEY UPDATE setting_json = VALUES(setting_json);

-- ========================================
-- Default Schema Version
-- ========================================
INSERT INTO schema_versions (version_id, schema_json, is_active) VALUES
('2025-12-29.1', '{"version":"2025-12-29.1","supported_events":["human","animal","vehicle","unknown","camera_issue"],"severity_levels":[0,1,2,3]}', TRUE)
ON DUPLICATE KEY UPDATE is_active = TRUE;
