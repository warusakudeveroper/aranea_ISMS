-- Migration 005: Add credentials to cameras table
-- カメラ個別のクレデンシャル保存（サブネット設定よりも優先される）

ALTER TABLE cameras
ADD COLUMN rtsp_username VARCHAR(64) NULL AFTER rtsp_sub,
ADD COLUMN rtsp_password VARCHAR(128) NULL AFTER rtsp_username;

-- ipcamscan_devicesにも成功したパスワードを記録（暗号化なし、内部ネットワーク用）
ALTER TABLE ipcamscan_devices
ADD COLUMN credential_password VARCHAR(128) NULL AFTER credential_username;
