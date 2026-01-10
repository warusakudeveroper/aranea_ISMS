-- Migration: 021_camera_registry
-- Description: CameraRegistry用カラム追加 (Phase 2: Issue #115)
-- DD05_CameraRegistry.md準拠
--
-- ## 目的
-- is22配下のカメラを仮想araneaDevice（is801 paraclateCamera）として管理
-- lacisIDを付与してParaclate連携を実現
--
-- ## 変更内容
-- 1. camera_brands に product_code 追加
-- 2. cameras に Paraclate連携カラム追加
-- 3. detection_logs に camera_lacis_id 追加

-- ============================================================
-- 1. camera_brands に product_code 追加
-- ============================================================
-- ProductCode割当表:
--   Tapo       = 0001
--   Hikvision  = 0002
--   Dahua      = 0003
--   Reolink    = 0004
--   Axis       = 0005
--   Generic    = 0000

ALTER TABLE camera_brands
ADD COLUMN IF NOT EXISTS product_code VARCHAR(4) DEFAULT '0000'
COMMENT 'Paraclate用ProductCode (4桁)';

-- 既存ブランドにProductCode設定
UPDATE camera_brands SET product_code = '0001' WHERE name = 'TP-LINK' OR name LIKE '%Tapo%';
UPDATE camera_brands SET product_code = '0002' WHERE name = 'Hikvision' OR name LIKE '%HIKVISION%';
UPDATE camera_brands SET product_code = '0003' WHERE name = 'Dahua' OR name LIKE '%DAHUA%';
UPDATE camera_brands SET product_code = '0004' WHERE name = 'Reolink' OR name LIKE '%REOLINK%';
UPDATE camera_brands SET product_code = '0005' WHERE name = 'Axis' OR name LIKE '%AXIS%';

-- ============================================================
-- 2. cameras テーブル拡張
-- ============================================================
-- Paraclate連携用カラム追加

-- テナントID (is22のtidを継承)
ALTER TABLE cameras
ADD COLUMN IF NOT EXISTS tid VARCHAR(32)
COMMENT 'テナントID (is22から継承)';

-- 施設ID
ALTER TABLE cameras
ADD COLUMN IF NOT EXISTS fid VARCHAR(32)
COMMENT '施設ID (Paraclateエンティティ)';

-- 部屋/エリアID
ALTER TABLE cameras
ADD COLUMN IF NOT EXISTS rid VARCHAR(32)
COMMENT '部屋/エリアID (Paraclateエンティティ)';

-- CIC (Client Identification Code)
ALTER TABLE cameras
ADD COLUMN IF NOT EXISTS cic VARCHAR(16)
COMMENT 'CIC (araneaDeviceGateから取得)';

-- 登録状態
ALTER TABLE cameras
ADD COLUMN IF NOT EXISTS registration_state ENUM('pending', 'registered', 'failed') DEFAULT 'pending'
COMMENT 'Paraclate登録状態';

-- 登録日時
ALTER TABLE cameras
ADD COLUMN IF NOT EXISTS registered_at DATETIME(3)
COMMENT 'araneaDeviceGate登録日時';

-- インデックス追加
ALTER TABLE cameras
ADD INDEX IF NOT EXISTS idx_cameras_lacis_id (lacis_id);

ALTER TABLE cameras
ADD INDEX IF NOT EXISTS idx_cameras_tid_fid (tid, fid);

ALTER TABLE cameras
ADD INDEX IF NOT EXISTS idx_cameras_registration_state (registration_state);

-- ============================================================
-- 3. detection_logs に camera_lacis_id 追加
-- ============================================================
-- 検出ログにカメラlacisIDを紐付け（tid/fid境界でのログ分離用）

ALTER TABLE detection_logs
ADD COLUMN IF NOT EXISTS camera_lacis_id VARCHAR(20)
COMMENT 'カメラのLacisID (Paraclate連携用)';

ALTER TABLE detection_logs
ADD INDEX IF NOT EXISTS idx_detection_logs_camera_lacis_id (camera_lacis_id);

-- ============================================================
-- 4. camera_registration テーブル（監査用）
-- ============================================================
-- カメラ登録の履歴・監査用テーブル

CREATE TABLE IF NOT EXISTS camera_registration (
    id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,

    -- カメラID (cameras.camera_id)
    camera_id VARCHAR(64) NOT NULL,

    -- LacisID (20桁): [Prefix=3][ProductType=801][MAC=12桁][ProductCode=4桁]
    lacis_id VARCHAR(20) NOT NULL,

    -- テナントID
    tid VARCHAR(32) NOT NULL,

    -- 施設ID
    fid VARCHAR(32),

    -- 部屋ID
    rid VARCHAR(32),

    -- CIC
    cic VARCHAR(16) NOT NULL,

    -- 状態
    state ENUM('pending', 'registered', 'failed', 'cleared') NOT NULL DEFAULT 'pending',

    -- 登録日時
    registered_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),

    -- 最終更新
    updated_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    -- エラーメッセージ（失敗時）
    error_message TEXT,

    -- インデックス
    INDEX idx_camera_registration_camera_id (camera_id),
    INDEX idx_camera_registration_lacis_id (lacis_id),
    INDEX idx_camera_registration_tid (tid),
    INDEX idx_camera_registration_state (state),

    -- 外部キー
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='カメラParaclate登録履歴 (DD05準拠)';
