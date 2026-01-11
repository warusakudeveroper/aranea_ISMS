-- 026_camera_sync_extension.sql
-- Phase 8: カメラ双方向同期 (DD10_CameraBidirectionalSync.md)
-- T8-1: DBマイグレーション

-- カメラ同期状態テーブル
-- IS22とmobes2.0間のカメラメタデータ同期状態を管理
CREATE TABLE IF NOT EXISTS camera_sync_state (
    id INT AUTO_INCREMENT PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL COMMENT 'カメラID（cameras.camera_id参照）',
    lacis_id VARCHAR(24) NOT NULL COMMENT 'カメラのLacisID',
    last_sync_to_mobes DATETIME(3) NULL COMMENT 'mobes2.0への最終同期日時',
    last_sync_from_mobes DATETIME(3) NULL COMMENT 'mobes2.0からの最終同期日時',
    mobes_sync_status ENUM('synced', 'pending', 'failed', 'deleted') DEFAULT 'pending'
        COMMENT '同期状態: synced=同期済, pending=保留, failed=失敗, deleted=削除済',
    sync_error_message TEXT NULL COMMENT '同期エラーメッセージ',
    retry_count INT DEFAULT 0 COMMENT 'リトライ回数',
    created_at DATETIME(3) DEFAULT NOW(3),
    updated_at DATETIME(3) DEFAULT NOW(3) ON UPDATE NOW(3),

    UNIQUE KEY uk_camera_id (camera_id),
    UNIQUE KEY uk_lacis_id (lacis_id),
    INDEX idx_sync_status (mobes_sync_status),
    INDEX idx_last_sync (last_sync_to_mobes)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='カメラ同期状態管理テーブル (Phase 8)';

-- カメラ個別設定テーブル（mobes2.0からの同期用）
-- Paraclate APPで設定されたカメラ個別の感度等を保持
CREATE TABLE IF NOT EXISTS camera_paraclate_settings (
    id INT AUTO_INCREMENT PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL COMMENT 'カメラID',
    lacis_id VARCHAR(24) NOT NULL COMMENT 'カメラのLacisID',
    sensitivity DECIMAL(3,2) DEFAULT 0.60 COMMENT '検出感度 (0.00-1.00)',
    detection_zone VARCHAR(32) DEFAULT 'full' COMMENT '検出ゾーン設定',
    alert_threshold INT DEFAULT 3 COMMENT 'アラート閾値',
    custom_preset VARCHAR(64) NULL COMMENT 'カスタムプリセット名',
    settings_json JSON NULL COMMENT '追加設定（JSON形式）',
    synced_at DATETIME(3) NOT NULL COMMENT 'mobes2.0からの同期日時',
    created_at DATETIME(3) DEFAULT NOW(3),
    updated_at DATETIME(3) DEFAULT NOW(3) ON UPDATE NOW(3),

    UNIQUE KEY uk_camera_id (camera_id),
    UNIQUE KEY uk_lacis_id (lacis_id),
    INDEX idx_synced_at (synced_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='カメラ個別設定テーブル（mobes2.0同期用）(Phase 8)';

-- camerasテーブル拡張
-- mobes同期関連カラムを追加
ALTER TABLE cameras
    ADD COLUMN IF NOT EXISTS mobes_synced_at DATETIME(3) NULL
        COMMENT 'mobes2.0への最終メタデータ同期日時' AFTER updated_at,
    ADD COLUMN IF NOT EXISTS mobes_sync_version INT DEFAULT 0
        COMMENT '同期バージョン（楽観ロック用）' AFTER mobes_synced_at;

-- 同期バージョン用インデックス
CREATE INDEX IF NOT EXISTS idx_cameras_mobes_sync ON cameras(mobes_synced_at, mobes_sync_version);

-- カメラ同期ログテーブル
-- 同期操作の監査ログ
CREATE TABLE IF NOT EXISTS camera_sync_logs (
    id INT AUTO_INCREMENT PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL COMMENT 'カメラID',
    lacis_id VARCHAR(24) NULL COMMENT 'カメラのLacisID',
    sync_direction ENUM('to_mobes', 'from_mobes') NOT NULL COMMENT '同期方向',
    sync_type ENUM('full', 'partial', 'single', 'delete') NOT NULL COMMENT '同期タイプ',
    status ENUM('success', 'failed', 'skipped') NOT NULL COMMENT '処理結果',
    changed_fields JSON NULL COMMENT '変更されたフィールド一覧',
    error_message TEXT NULL COMMENT 'エラーメッセージ',
    request_payload JSON NULL COMMENT 'リクエストペイロード（デバッグ用）',
    response_payload JSON NULL COMMENT 'レスポンスペイロード（デバッグ用）',
    created_at DATETIME(3) DEFAULT NOW(3),

    INDEX idx_camera_id (camera_id),
    INDEX idx_created_at (created_at),
    INDEX idx_sync_direction (sync_direction, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='カメラ同期監査ログ (Phase 8)';

-- コメント: Phase 8 カメラ双方向同期
--
-- 本マイグレーションで追加されるテーブル:
-- 1. camera_sync_state: カメラの同期状態を管理
-- 2. camera_paraclate_settings: mobes2.0から同期されたカメラ個別設定
-- 3. camera_sync_logs: 同期操作の監査ログ
--
-- camerasテーブル拡張:
-- - mobes_synced_at: 最終同期日時
-- - mobes_sync_version: 楽観ロック用バージョン
--
-- 関連設計書: DD10_CameraBidirectionalSync.md
-- 関連Issue: #121
