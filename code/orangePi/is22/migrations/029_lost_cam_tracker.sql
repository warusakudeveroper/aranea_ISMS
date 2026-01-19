-- Migration: 029_lost_cam_tracker.sql
-- LostCamTracker: DHCP追随によるカメラ自動復旧機能
--
-- 設計原則: ARPスキャンのみ使用（カメラに負荷をかけない）
-- - ONVIF/RTSP認証試行は行わない
-- - ポートスキャンは行わない
-- - クレデンシャル総当たりは行わない

-- cameras テーブルに追跡用カラムを追加
ALTER TABLE cameras ADD COLUMN arp_scan_attempted_at DATETIME NULL COMMENT 'ARPスキャン最終実行日時';
ALTER TABLE cameras ADD COLUMN arp_scan_result VARCHAR(50) NULL COMMENT 'ARPスキャン結果: found_updated, not_found, subnet_unreachable, arp_not_supported';
ALTER TABLE cameras ADD COLUMN last_healthy_at DATETIME NULL COMMENT 'カメラ正常動作の最終確認日時';
ALTER TABLE cameras ADD COLUMN ip_relocation_count INT DEFAULT 0 COMMENT 'IP自動更新の累計回数';

-- 設定テーブルにLostCamTracker設定を追加
INSERT INTO config (`key`, `value`, `description`) VALUES
  ('lost_cam_tracker_enabled', 'true', 'DHCPなどによる登録カメラロストに追随する機能の有効/無効'),
  ('lost_cam_tracker_threshold_minutes', '30', 'カメラ異常からARPスキャンまでの閾値（分）'),
  ('lost_cam_tracker_retry_minutes', '60', '見つからない場合の再試行間隔（分）')
ON DUPLICATE KEY UPDATE `description` = VALUES(`description`);

-- ARPスキャナデバイス登録テーブル（将来のis20s連携用）
CREATE TABLE IF NOT EXISTS arp_scanner_devices (
  id INT AUTO_INCREMENT PRIMARY KEY,
  name VARCHAR(100) NOT NULL COMMENT 'デバイス名（例: is20s-tam）',
  api_url VARCHAR(255) NOT NULL COMMENT 'ARP API URL（例: http://192.168.125.248:8080/api/arp-cache）',
  subnets TEXT NULL COMMENT '対象サブネット（JSON配列: ["192.168.125.0/24", "192.168.126.0/24"]）',
  enabled BOOLEAN DEFAULT TRUE,
  last_check_at DATETIME NULL COMMENT '最終疎通確認日時',
  last_check_status VARCHAR(50) NULL COMMENT '最終疎通確認結果',
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) COMMENT='ARPスキャナデバイス（is20s等）の登録';

-- IP変更追跡履歴テーブル
CREATE TABLE IF NOT EXISTS camera_ip_relocation_history (
  id INT AUTO_INCREMENT PRIMARY KEY,
  camera_id VARCHAR(100) NOT NULL,
  camera_name VARCHAR(255) NULL,
  old_ip VARCHAR(45) NOT NULL,
  new_ip VARCHAR(45) NOT NULL,
  detection_method VARCHAR(50) NOT NULL COMMENT 'arp_scan, arp_scanner_device, manual_scan',
  scanner_device_id INT NULL COMMENT 'ARPスキャナデバイスID（該当する場合）',
  relocated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  INDEX idx_camera_id (camera_id),
  INDEX idx_relocated_at (relocated_at),
  FOREIGN KEY (scanner_device_id) REFERENCES arp_scanner_devices(id) ON DELETE SET NULL
) COMMENT='カメラIP変更追跡履歴';
