-- Migration 007: Camera extended fields for CameraDetailModal
-- カメラ詳細モーダル用の拡張フィールド + ONVIF Discovery情報

-- ========================================
-- 1. 管理用フィールド
-- ========================================
ALTER TABLE cameras
ADD COLUMN rid VARCHAR(32) NULL AFTER location COMMENT '部屋ID/設置場所識別子',
ADD COLUMN cic CHAR(6) NULL AFTER lacis_id COMMENT '6桁登録コード（将来自動取得）',
ADD COLUMN rotation INT NOT NULL DEFAULT 0 COMMENT '表示回転角度 (0/90/180/270)',
ADD COLUMN fid VARCHAR(32) NULL COMMENT 'ファシリティID（サブネットから継承）',
ADD COLUMN tid VARCHAR(32) NULL COMMENT 'テナントID（サブネットから継承）',
ADD COLUMN sort_order INT NOT NULL DEFAULT 0 COMMENT 'ドラッグ並べ替え用順序',
ADD COLUMN deleted_at DATETIME(3) NULL COMMENT 'ソフトデリート日時（NULLは有効）';

-- ========================================
-- 2. ONVIF デバイス情報
-- ========================================
ALTER TABLE cameras
ADD COLUMN serial_number VARCHAR(64) NULL COMMENT 'シリアル番号',
ADD COLUMN hardware_id VARCHAR(64) NULL COMMENT 'ハードウェアID',
ADD COLUMN firmware_version VARCHAR(64) NULL COMMENT 'ファームウェアバージョン',
ADD COLUMN onvif_endpoint VARCHAR(256) NULL COMMENT 'ONVIF WSエンドポイントURL';

-- ========================================
-- 3. ネットワーク情報
-- ========================================
ALTER TABLE cameras
ADD COLUMN rtsp_port INT NULL DEFAULT 554 COMMENT 'RTSPポート番号',
ADD COLUMN http_port INT NULL DEFAULT 80 COMMENT 'HTTPポート番号',
ADD COLUMN onvif_port INT NULL DEFAULT 80 COMMENT 'ONVIFポート番号';

-- ========================================
-- 4. ビデオ能力（メインストリーム）
-- ========================================
ALTER TABLE cameras
ADD COLUMN resolution_main VARCHAR(16) NULL COMMENT 'メイン解像度 (例: 1920x1080)',
ADD COLUMN codec_main VARCHAR(16) NULL COMMENT 'メインコーデック (H.264/H.265/MJPEG)',
ADD COLUMN fps_main INT NULL COMMENT 'メインフレームレート',
ADD COLUMN bitrate_main INT NULL COMMENT 'メインビットレート (kbps)';

-- ========================================
-- 5. ビデオ能力（サブストリーム）
-- ========================================
ALTER TABLE cameras
ADD COLUMN resolution_sub VARCHAR(16) NULL COMMENT 'サブ解像度 (例: 640x480)',
ADD COLUMN codec_sub VARCHAR(16) NULL COMMENT 'サブコーデック',
ADD COLUMN fps_sub INT NULL COMMENT 'サブフレームレート',
ADD COLUMN bitrate_sub INT NULL COMMENT 'サブビットレート (kbps)';

-- ========================================
-- 6. PTZ能力
-- ========================================
ALTER TABLE cameras
ADD COLUMN ptz_supported BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'PTZ対応フラグ',
ADD COLUMN ptz_continuous BOOLEAN NOT NULL DEFAULT FALSE COMMENT '連続移動対応',
ADD COLUMN ptz_absolute BOOLEAN NOT NULL DEFAULT FALSE COMMENT '絶対位置移動対応',
ADD COLUMN ptz_relative BOOLEAN NOT NULL DEFAULT FALSE COMMENT '相対移動対応',
ADD COLUMN ptz_pan_range JSON NULL COMMENT 'パン範囲 {"min":-180,"max":180}',
ADD COLUMN ptz_tilt_range JSON NULL COMMENT 'チルト範囲 {"min":-90,"max":90}',
ADD COLUMN ptz_zoom_range JSON NULL COMMENT 'ズーム範囲 {"min":1,"max":10}',
ADD COLUMN ptz_presets JSON NULL COMMENT 'プリセット一覧 [{"token":"1","name":"玄関"}]',
ADD COLUMN ptz_home_supported BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'ホームポジション対応';

-- ========================================
-- 7. 音声能力
-- ========================================
ALTER TABLE cameras
ADD COLUMN audio_input_supported BOOLEAN NOT NULL DEFAULT FALSE COMMENT '音声入力対応（マイク）',
ADD COLUMN audio_output_supported BOOLEAN NOT NULL DEFAULT FALSE COMMENT '音声出力対応（スピーカー）',
ADD COLUMN audio_codec VARCHAR(16) NULL COMMENT '音声コーデック (G.711/AAC/PCM)';

-- ========================================
-- 8. ONVIFプロファイル
-- ========================================
ALTER TABLE cameras
ADD COLUMN onvif_profiles JSON NULL COMMENT 'ONVIFプロファイル一覧 [{"token":"Profile_1","name":"mainStream"}]';

-- ========================================
-- 9. 検出メタ情報
-- ========================================
ALTER TABLE cameras
ADD COLUMN discovery_method VARCHAR(32) NULL COMMENT '検出方法 (onvif/rtsp/manual)',
ADD COLUMN last_verified_at DATETIME(3) NULL COMMENT '最終疎通確認日時',
ADD COLUMN last_rescan_at DATETIME(3) NULL COMMENT '最終再スキャン日時';

-- ========================================
-- インデックス
-- ========================================
CREATE INDEX idx_cameras_rid ON cameras(rid);
CREATE INDEX idx_cameras_fid ON cameras(fid);
CREATE INDEX idx_cameras_tid ON cameras(tid);
CREATE INDEX idx_cameras_deleted ON cameras(deleted_at);
CREATE INDEX idx_cameras_sort ON cameras(sort_order);
CREATE INDEX idx_cameras_ptz ON cameras(ptz_supported);
CREATE INDEX idx_cameras_serial ON cameras(serial_number);
