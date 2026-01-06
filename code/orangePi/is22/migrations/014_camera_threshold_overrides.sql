-- Migration: 014_camera_threshold_overrides.sql
-- Description: Add threshold override columns for per-camera detection sensitivity tuning
-- Date: 2026-01-04
-- Design Reference: docs/is22_preset_slider_and_camera_events_design.md

-- Add threshold override columns to cameras table
-- Note: Using FLOAT instead of DECIMAL for sqlx compatibility with Rust f32
ALTER TABLE cameras
ADD COLUMN conf_override FLOAT DEFAULT NULL COMMENT 'YOLO confidence threshold override (0.20-0.80), NULL = use preset default',
ADD COLUMN nms_threshold FLOAT DEFAULT NULL COMMENT 'NMS threshold override (0.30-0.60), NULL = use preset default',
ADD COLUMN par_threshold FLOAT DEFAULT NULL COMMENT 'PAR attribute threshold override (0.30-0.80), NULL = use preset default';

-- Add index for finding cameras with custom thresholds
CREATE INDEX idx_cameras_threshold_overrides ON cameras(conf_override, nms_threshold, par_threshold);
